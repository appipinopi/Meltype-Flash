// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 利用状況の集計 (Aptabase)。送るのは出来事の名前と端末の種類・OSの版・アプリの版だけで、
//! 入力内容・入力履歴・辞書・設定値は送らない。設定画面で入れた人だけ送る。既定は無効。
use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_KEY: &str = "A-US-4882313783";
const API_URL: &str = "https://us.aptabase.com/api/v0/events";
const SDK_VERSION: &str = "meltype-flash-http@1";
const SETTINGS_FILE: &str = "analytics.json";

/// 設定画面で利用状況の送信を入れたか。ファイルが無い・読めないときは無効。
pub fn enabled(directory: &Path) -> bool {
    std::fs::read(directory.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value["enabled"].as_bool())
        .unwrap_or(false)
}

/// 設定を保存する。入れたときはすぐ送り、切ったときは次の起動から送らない。
pub fn set_enabled(directory: &Path, enabled: bool) -> std::io::Result<()> {
    let bytes =
        serde_json::to_vec_pretty(&json!({"enabled": enabled})).map_err(std::io::Error::other)?;
    meltype_core::storage::atomic_write(&directory.join(SETTINGS_FILE), &bytes)
}

/// 背景スレッドで1件だけ送る。待たない・失敗しても本体の動作に影響させない。
pub fn track(directory: &Path, event: &'static str) {
    if !enabled(directory) {
        return;
    }
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        let _ = runtime.block_on(send(event));
    });
}

fn payload(event: &str) -> Value {
    json!([{
        "timestamp": timestamp(),
        "sessionId": session_id(),
        "eventName": event,
        "systemProps": {
            "isDebug": cfg!(debug_assertions),
            "locale": locale(),
            "osName": "Windows",
            "osVersion": os_version(),
            "appVersion": env!("CARGO_PKG_VERSION"),
            "appBuildNumber": env!("CARGO_PKG_VERSION"),
            "sdkVersion": SDK_VERSION,
            "deviceModel": ""
        }
    }])
}

async fn send(event: &str) -> Result<(), Box<dyn std::error::Error>> {
    let os_version = os_version();
    let locale = locale();
    let payload = payload(event);
    let response = reqwest::Client::new()
        .post(API_URL)
        .header("App-Key", APP_KEY)
        .header("User-Agent", format!("Windows/{os_version} {locale}"))
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("Aptabase responded with {}", response.status()).into());
    }
    Ok(())
}

fn timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// SDKと同じ形 (エポック秒 * 10^8 + 乱数) の匿名セッションID。
fn session_id() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let random = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos()))
        .unwrap_or(0)
        % 100_000_000;
    format!("{}", seconds * 100_000_000 + random)
}

fn os_version() -> String {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    unsafe { RtlGetVersion(&mut info) };
    format!(
        "{}.{}.{}",
        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
    )
}

fn locale() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    use windows_sys::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    let length =
        unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH as i32) };
    if length <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(length as usize - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_roundtrip_and_payload_shape() {
        let directory = tempfile::tempdir().unwrap();
        assert!(!enabled(directory.path()));
        set_enabled(directory.path(), true).unwrap();
        assert!(enabled(directory.path()));
        set_enabled(directory.path(), false).unwrap();
        assert!(!enabled(directory.path()));

        let value = payload("app_open");
        assert_eq!(value[0]["eventName"], "app_open");
        assert_eq!(value[0]["systemProps"]["osName"], "Windows");
        assert!(value[0]["systemProps"]["appVersion"].as_str().is_some());
        assert!(timestamp().ends_with('Z'));
        assert!(session_id().parse::<u64>().is_ok());
    }
}
