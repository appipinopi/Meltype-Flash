// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Discord のステータス (Rich Presence) に打鍵数を出す。
//!
//! Discord のローカル IPC (Windows は `\\.\pipe\discord-ipc-N`) へ直接つなぎ、
//! 外のサーバーは通さない。送るのは今日と合計の打鍵数・1 分あたりの回数・経過時間だけで、
//! 打った文字・変換候補・入力先のアプリ名は送らない。表示する文言は設定画面で変えられる。
use meltype_core::{
    keycount::{self, KeyCount},
    storage::Store,
};
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Discord の開発者ポータルで作ったアプリの ID。空なら何も送らない。
const CLIENT_ID: &str = "1568799577841344653";
const UPDATE_INTERVAL: Duration = Duration::from_secs(20);

/// 送る打鍵数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub today: u64,
    pub total: u64,
}

/// 経過時間 (起動からの 時:分:秒)。
pub fn elapsed_text(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// 1 分あたりの打鍵数。
pub fn keys_per_minute(keys: u64, elapsed: Duration) -> u64 {
    let minutes = elapsed.as_secs_f64() / 60.;
    if minutes <= 0. {
        return 0;
    }
    (keys as f64 / minutes).round() as u64
}

/// 表示する内容 (activity)。打った文字やアプリ名は入れない。
pub fn activity_json(count: &KeyCount, stats: Stats, kpm: u64, elapsed: &str, ai: bool) -> String {
    let ai = ai && count.ai;
    let details = keycount::render(&count.details, stats.today, stats.total, kpm, elapsed, ai);
    let state = keycount::render(&count.state, stats.today, stats.total, kpm, elapsed, ai);
    let mut activity = serde_json::json!({
        "details": details,
        "state": state,
        "timestamps": {"start": now_ms().saturating_sub(elapsed_ms(elapsed))},
    });
    if !count.button.is_empty() && !count.url.is_empty() {
        activity["buttons"] = serde_json::json!([{"label": count.button, "url": count.url}]);
    }
    activity.to_string()
}

/// 経過時間の文字列からミリ秒に戻す (タイムスタンプの起点を求めるため)。
fn elapsed_ms(text: &str) -> u64 {
    let mut total = 0u64;
    for part in text.split(':') {
        total = total * 60 + part.parse::<u64>().unwrap_or(0);
    }
    total * 1000
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// IPC の 1 通 (opcode と長さを little endian の 4 バイトずつ、続けて UTF-8 の JSON)。
pub fn frame(opcode: i32, json: &str) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + json.len());
    frame.extend_from_slice(&opcode.to_le_bytes());
    frame.extend_from_slice(&(json.len() as u32).to_le_bytes());
    frame.extend_from_slice(json.as_bytes());
    frame
}

pub fn handshake_json(client_id: &str) -> String {
    serde_json::json!({"v": 1, "client_id": client_id}).to_string()
}

pub fn set_activity_json(activity: Option<&str>) -> String {
    let activity: serde_json::Value = activity
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "args": {"pid": std::process::id(), "activity": activity},
        "nonce": nonce(),
    })
    .to_string()
}

fn nonce() -> String {
    let time = now_ms();
    let random = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()))
        .unwrap_or(0);
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        time as u32,
        random as u16,
        (random >> 16) as u16,
        std::process::id() as u16,
        time & 0xffff_ffff_ffff
    )
}

/// Discord の応答を検証する。ERROR を成功扱いしない。
pub fn verify_response(opcode: i32, json: &str) -> Result<(), String> {
    if opcode == 2 {
        return Err("Discord が接続を閉じました。".into());
    }
    if opcode != 0 && opcode != 1 {
        return Err(format!(
            "Discord から不明な opcode が返されました: {opcode}"
        ));
    }
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| "Discord の返事が JSON ではありません。".to_string())?;
    if value.get("evt").and_then(|evt| evt.as_str()) == Some("ERROR") {
        let data = value.get("data").cloned().unwrap_or_default();
        let code = data.get("code").map(|v| v.to_string()).unwrap_or_default();
        let message = data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("不明なエラー");
        return Err(format!(
            "Discord の ERROR 応答: code={code}, message={message}"
        ));
    }
    Ok(())
}

/// IPC の読み書き。Windows は名前付きパイプ、macOS/Linux は Unix ドメインソケット。
trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

#[cfg(windows)]
fn connect() -> Option<Box<dyn ReadWrite>> {
    use std::fs::OpenOptions;
    for index in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{index}");
        if let Ok(file) = OpenOptions::new().read(true).write(true).open(path) {
            return Some(Box::new(file));
        }
    }
    None
}

#[cfg(unix)]
fn connect() -> Option<Box<dyn ReadWrite>> {
    use std::os::unix::net::UnixStream;
    let mut directories = Vec::new();
    if let Some(directory) = std::env::var_os("XDG_RUNTIME_DIR") {
        directories.push(PathBuf::from(directory));
    }
    if let Some(directory) = std::env::var_os("TMPDIR") {
        directories.push(PathBuf::from(directory));
    }
    directories.push(PathBuf::from("/tmp"));
    for directory in directories {
        for index in 0..10 {
            let path = directory.join(format!("discord-ipc-{index}"));
            if let Ok(socket) = UnixStream::connect(path) {
                return Some(Box::new(socket));
            }
        }
    }
    None
}

fn send(file: &mut dyn ReadWrite, opcode: i32, json: &str) -> io::Result<()> {
    file.write_all(&frame(opcode, json))?;
    file.flush()
}

fn receive(file: &mut dyn ReadWrite) -> io::Result<String> {
    let mut header = [0u8; 8];
    file.read_exact(&mut header)?;
    let opcode = i32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    if length > 1 << 20 {
        return Err(io::Error::other("Discord から大きすぎる返事が来ました。"));
    }
    let mut body = vec![0u8; length];
    file.read_exact(&mut body)?;
    let json = String::from_utf8_lossy(&body).into_owned();
    verify_response(opcode, &json).map_err(io::Error::other)?;
    Ok(json)
}

/// 送信の状態。つないでいなければ次回つなぎ直す。
struct Session {
    file: Box<dyn ReadWrite>,
    sent: Option<String>,
    last: Option<(u64, Instant)>,
}

/// 設定画面の裏で動く送信スレッド。止めるときは [Presence] を落とす。
pub struct Presence {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Presence {
    /// Discord のステータスに打鍵数を出すかどうかはファイル (keycount.json) を見る。
    /// AI 対応中かどうかは保存した設定から読む。
    pub fn start(directory: PathBuf, store: Store) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            let started = Instant::now();
            let mut session: Option<Session> = None;
            while !flag.load(Ordering::Relaxed) {
                let count = keycount::load(&directory);
                if count.enabled && count.discord && !CLIENT_ID.is_empty() {
                    let total = count.total;
                    let kpm = session
                        .as_ref()
                        .and_then(|s| s.last)
                        .map(|(last_total, at)| {
                            keys_per_minute(total.saturating_sub(last_total), at.elapsed())
                        })
                        .unwrap_or(0);
                    let elapsed = elapsed_text(started.elapsed().as_secs());
                    let ai = store
                        .settings()
                        .map(|settings| ai_active(&settings))
                        .unwrap_or(false);
                    let activity = activity_json(
                        &count,
                        Stats {
                            today: count.days.get(&keycount::today()).copied().unwrap_or(0),
                            total,
                        },
                        kpm,
                        &elapsed,
                        ai,
                    );
                    match send_activity(&mut session, &activity) {
                        Ok(()) => log_presence(&directory, "Discord にステータスを送信しました"),
                        Err(error) => {
                            log_presence(&directory, &format!("送信できませんでした: {error}"));
                            // 次回つなぎ直す (壊れた接続を残さない)。
                            session = None;
                        }
                    }
                    if let Some(session) = &mut session {
                        session.last = Some((total, Instant::now()));
                    }
                } else if session.is_some() {
                    // OFF にしたら表示を消して切る。
                    if let Some(current) = session.as_mut() {
                        let _ = send(current.file.as_mut(), 1, &set_activity_json(None));
                    }
                    session = None;
                }
                // 停止の指示を待ちながら 20 秒待つ (終了時に長く待たない)。
                let mut waited = Duration::ZERO;
                while waited < UPDATE_INTERVAL && !flag.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(200));
                    waited += Duration::from_millis(200);
                }
            }
            if let Some(mut session) = session {
                let _ = send(session.file.as_mut(), 1, &set_activity_json(None));
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 送信の結果を `presence.log` に残す (最後の結果だけ上書き)。
/// 接続できないときの原因を追うため。打った文字は書かない。
fn log_presence(directory: &Path, message: &str) {
    let _ = std::fs::write(
        directory.join("presence.log"),
        format!(
            "{} {}
",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            message
        ),
    );
}

/// つないでいなければつなぎ直し、内容が変わっていれば送る。
/// つなげない・送れないときは理由を返す (呼び出し側が presence.log に残す)。
fn send_activity(session: &mut Option<Session>, activity: &str) -> Result<(), String> {
    if session.is_none() {
        let Some(mut file) = connect() else {
            return Err("Discord が見つかりません (起動していないか、IPC が開いていません)".into());
        };
        send(&mut file, 0, &handshake_json(CLIENT_ID)).map_err(|e| e.to_string())?;
        receive(&mut file).map_err(|e| e.to_string())?;
        *session = Some(Session {
            file,
            sent: None,
            last: None,
        });
    }
    let Some(current) = session.as_mut() else {
        return Ok(());
    };
    if current.sent.as_deref() == Some(activity) {
        return Ok(());
    }
    send(current.file.as_mut(), 1, &set_activity_json(Some(activity)))
        .map_err(|e| e.to_string())?;
    receive(current.file.as_mut()).map_err(|e| e.to_string())?;
    current.sent = Some(activity.to_owned());
    Ok(())
}

/// AI (予測変換) が有効か。設定画面の保存内容から読む。
pub fn ai_active(settings: &meltype_core::settings::Settings) -> bool {
    settings.boolean("Enabled")
        && (settings.boolean("UseZenzai")
            || settings.boolean("UseShunti")
            || settings.text("AiModel") != "None")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_opcode_and_length_in_little_endian() {
        let frame = frame(1, "{\"a\":\"打\"}");
        assert_eq!(i32::from_le_bytes(frame[..4].try_into().unwrap()), 1);
        assert_eq!(
            i32::from_le_bytes(frame[4..8].try_into().unwrap()) as usize,
            frame.len() - 8
        );
        assert_eq!(&frame[8..], "{\"a\":\"打\"}".as_bytes());
    }

    #[test]
    fn activity_shows_only_counts_and_elapsed() {
        let count = KeyCount {
            enabled: true,
            discord: true,
            ..KeyCount::default()
        };
        let activity = activity_json(
            &count,
            Stats {
                today: 4528,
                total: 69450,
            },
            141,
            "0:15",
            false,
        );
        let value: serde_json::Value = serde_json::from_str(&activity).unwrap();
        assert_eq!(value["details"], "今日 4,528 打");
        assert_eq!(value["state"], "合計 69,450 打 · 141 打/分");
        assert_eq!(value["buttons"][0]["label"], "Meltype");
        assert!(value["timestamps"]["start"].as_u64().is_some());
        let names = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name == "app" || name == "text"));
    }

    #[test]
    fn ai_label_is_appended_when_active_and_enabled() {
        let mut count = KeyCount {
            ai: true,
            state: "合計 {total} 打{ai}".into(),
            ..KeyCount::default()
        };
        let activity = activity_json(&count, Stats { today: 1, total: 2 }, 3, "0:15", true);
        assert!(activity.contains("AI対応中"));
        count.ai = false;
        let activity = activity_json(&count, Stats { today: 1, total: 2 }, 3, "0:15", true);
        assert!(!activity.contains("AI対応中"));
    }

    #[test]
    fn set_activity_null_clears() {
        let value: serde_json::Value = serde_json::from_str(&set_activity_json(None)).unwrap();
        assert_eq!(value["cmd"], "SET_ACTIVITY");
        assert!(value["args"]["activity"].is_null());
        assert!(value["nonce"].as_str().is_some());
    }

    #[test]
    fn verify_response_rejects_errors_and_unknown_opcodes() {
        assert!(
            verify_response(1, r#"{"evt":"ERROR","data":{"code":4000,"message":"bad"}}"#).is_err()
        );
        assert!(verify_response(2, "{}").is_err());
        assert!(verify_response(7, "{}").is_err());
        assert!(verify_response(0, r#"{"evt":"READY"}"#).is_ok());
        assert!(verify_response(1, r#"{"cmd":"SET_ACTIVITY","data":{}}"#).is_ok());
    }

    #[test]
    fn keys_per_minute_and_elapsed_text() {
        assert_eq!(keys_per_minute(120, Duration::from_secs(60)), 120);
        assert_eq!(keys_per_minute(0, Duration::from_secs(0)), 0);
        assert_eq!(elapsed_text(15), "0:15");
        assert_eq!(elapsed_text(3723), "1:02:03");
    }
}
