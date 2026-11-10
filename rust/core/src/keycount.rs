// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! 打鍵数の集計 (keycount.json)。打った文字は残さず、合計と日ごとの回数だけを持つ。
//!
//! 入力は IME のプロセス (TSF worker) が数え、設定画面が読む。入力側は数えた分を
//! まとめて [`KeyCounter::flush`] で書き、設定画面は [`save_settings`] でフラグと
//! 表示の文言だけを書く。どちらもファイルロックを取ってから読み直して足すので、
//! 複数の入力プロセスが同時に数えても回数を落とさない。
use crate::storage::atomic_write;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub const FILE: &str = "keycount.json";
pub const LOCK: &str = "keycount.lock";

/// Discord の表示に使う文言の既定値。`{today}` `{total}` `{kpm}` `{elapsed}` `{ai}` を置き換える。
pub const DEFAULT_DETAILS: &str = "今日 {today} 打";
pub const DEFAULT_STATE: &str = "合計 {total} 打 · {kpm} 打/分{ai}";
pub const DEFAULT_BUTTON: &str = "Meltype";
pub const DEFAULT_URL: &str = "https://github.com/appipinopi/Meltype-Flash";

/// 保存する内容。足りない項目は既定値で補う。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyCount {
    pub total: u64,
    /// 日 (yyyy-MM-dd) ごとの打鍵数。
    pub days: BTreeMap<String, u64>,
    /// 打鍵を数えるか。既定は有効 (設定画面で OFF にできる)。
    pub enabled: bool,
    /// Discord のステータスに出すか。既定は有効 (設定画面で OFF にできる)。
    pub discord: bool,
    /// Discord の詳細行。
    pub details: String,
    /// Discord の状態行。
    pub state: String,
    /// Discord のボタンの文字。
    pub button: String,
    /// Discord のボタンのリンク先。
    pub url: String,
    /// AI (予測変換) が動いている間に「AI対応中」を出すか。
    pub ai: bool,
}

impl Default for KeyCount {
    fn default() -> Self {
        Self {
            total: 0,
            days: BTreeMap::new(),
            enabled: false,
            discord: false,
            details: DEFAULT_DETAILS.into(),
            state: DEFAULT_STATE.into(),
            button: DEFAULT_BUTTON.into(),
            url: DEFAULT_URL.into(),
            ai: true,
        }
    }
}

pub fn path(directory: &Path) -> PathBuf {
    directory.join(FILE)
}

/// 今日の日付 (yyyy-MM-dd)。打鍵数の日ごとの集計に使う。
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 保存してある打鍵数を読む。無い・壊れているときは既定値。
pub fn load(directory: &Path) -> KeyCount {
    fs::read(path(directory))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn lock(directory: &Path) -> io::Result<File> {
    fs::create_dir_all(directory)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join(LOCK))?;
    fs2::FileExt::lock_exclusive(&file)?;
    Ok(file)
}

fn write(directory: &Path, count: &KeyCount) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(count).map_err(io::Error::other)?;
    atomic_write(&path(directory), &bytes)
}

/// 設定画面からフラグと表示の文言を保存する。数えた回数はファイルの値をそのまま残す。
pub fn save_settings(directory: &Path, settings: &KeyCount) -> io::Result<()> {
    let _lock = lock(directory)?;
    let mut count = load(directory);
    count.enabled = settings.enabled;
    count.discord = settings.discord;
    count.details = settings.details.clone();
    count.state = settings.state.clone();
    count.button = settings.button.clone();
    count.url = settings.url.clone();
    count.ai = settings.ai;
    write(directory, &count)
}

/// 数える側。押した回数をメモリに溜め、[`flush`](Self::flush) でファイルに足す。
pub struct KeyCounter {
    directory: PathBuf,
    count: KeyCount,
    pending: BTreeMap<String, u64>,
    /// 最後に読んだファイルの更新時刻。設定画面が書き換えたかどうかを打鍵ごとに見る。
    mtime: Option<SystemTime>,
}

/// keycount.json の更新時刻。無い・読めないときは None。
fn modified(directory: &Path) -> Option<SystemTime> {
    fs::metadata(path(directory))
        .and_then(|metadata| metadata.modified())
        .ok()
}

impl KeyCounter {
    pub fn load(directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        let count = load(&directory);
        let mtime = modified(&directory);
        Self {
            directory,
            count,
            pending: BTreeMap::new(),
            mtime,
        }
    }

    /// 数える設定になっているか (ファイルの値を読む)。
    pub fn enabled(&self) -> bool {
        self.count.enabled
    }

    pub fn total(&self) -> u64 {
        self.count.total + self.pending.values().sum::<u64>()
    }

    pub fn on_day(&self, day: &str) -> u64 {
        self.count.days.get(day).copied().unwrap_or(0) + self.pending.get(day).copied().unwrap_or(0)
    }

    /// 1 打鍵を数える。数えない設定のときは何もしない。
    pub fn count_key(&mut self, day: &str) {
        if !self.count.enabled {
            return;
        }
        *self.pending.entry(day.to_owned()).or_default() += 1;
    }

    /// 設定 (数えるか・Discord に出すか・文言) だけをファイルから読み直す。
    /// 入力プロセスは動いたままなので、設定画面での変更を次の打鍵から反映させる。
    pub fn refresh(&mut self) {
        self.mtime = modified(&self.directory);
        let count = load(&self.directory);
        self.count.enabled = count.enabled;
        self.count.discord = count.discord;
        self.count.details = count.details;
        self.count.state = count.state;
        self.count.button = count.button;
        self.count.url = count.url;
        self.count.ai = count.ai;
    }

    /// ファイルが設定画面で書き換わっていれば設定を読み直す。
    /// 更新時刻が同じときは何もしない (打鍵のたびに数えるだけなら安い)。
    pub fn refresh_if_changed(&mut self) {
        if modified(&self.directory) == self.mtime {
            return;
        }
        self.refresh();
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// 溜めた分をファイルに足す。ほかのプロセスが足した分も取り込む。
    pub fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let _lock = lock(&self.directory)?;
        let mut count = load(&self.directory);
        count.total = count
            .total
            .saturating_add(self.pending.values().sum::<u64>());
        for (day, added) in &self.pending {
            let slot = count.days.entry(day.clone()).or_default();
            *slot = slot.saturating_add(*added);
        }
        write(&self.directory, &count)?;
        self.mtime = modified(&self.directory);
        self.count = count;
        self.pending.clear();
        Ok(())
    }
}

/// 数えた分を 0 に戻す。数えるかどうか・Discord に出すかの設定は残す。
pub fn reset(directory: &Path) -> io::Result<()> {
    let _lock = lock(directory)?;
    let mut count = load(directory);
    count.total = 0;
    count.days.clear();
    write(directory, &count)
}

/// 表示する文言の `{today}` `{total}` `{kpm}` `{elapsed}` `{ai}` を置き換える。
/// 3 桁区切りは `,`。`{ai}` は AI が動いているときだけ ` · AI対応中` になる。
pub fn render(template: &str, today: u64, total: u64, kpm: u64, elapsed: &str, ai: bool) -> String {
    template
        .replace("{today}", &group(today))
        .replace("{total}", &group(total))
        .replace("{kpm}", &group(kpm))
        .replace("{elapsed}", elapsed)
        .replace("{ai}", if ai { " · AI対応中" } else { "" })
}

/// 3 桁区切り (12,345) の文字列。
pub fn group(value: u64) -> String {
    let digits = value.to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_match_the_reference_text() {
        let count = KeyCount::default();
        assert!(!count.enabled);
        assert!(!count.discord);
        assert!(count.ai);
        assert_eq!(count.details, "今日 {today} 打");
        assert_eq!(count.state, "合計 {total} 打 · {kpm} 打/分{ai}");
    }

    #[test]
    fn counts_merge_across_writers_and_keep_settings() {
        let directory = tempfile::tempdir().unwrap();
        let mut settings = load(directory.path());
        settings.enabled = true;
        settings.discord = true;
        settings.details = "きょう {today}".into();
        save_settings(directory.path(), &settings).unwrap();

        let mut first = KeyCounter::load(directory.path());
        first.count_key("2026-02-14");
        first.count_key("2026-02-14");
        first.flush().unwrap();
        assert_eq!(first.total(), 2);

        let mut second = KeyCounter::load(directory.path());
        second.count_key("2026-02-14");
        second.count_key("2026-02-15");
        second.flush().unwrap();

        let merged = load(directory.path());
        assert_eq!(merged.total, 4);
        assert_eq!(merged.days["2026-02-14"], 3);
        assert_eq!(merged.days["2026-02-15"], 1);
        // 数える側はフラグや文言を書き換えない。
        assert!(merged.enabled);
        assert!(merged.discord);
        assert_eq!(merged.details, "きょう {today}");
    }

    #[test]
    fn disabled_counter_counts_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let mut settings = load(directory.path());
        settings.enabled = false;
        save_settings(directory.path(), &settings).unwrap();
        let mut counter = KeyCounter::load(directory.path());
        counter.count_key("2026-02-14");
        assert!(!counter.has_pending());
        counter.flush().unwrap();
        assert_eq!(load(directory.path()).total, 0);
    }

    #[test]
    fn render_replaces_placeholders_and_groups_digits() {
        assert_eq!(
            render("今日 {today} 打", 4528, 69450, 141, "0:15", false),
            "今日 4,528 打"
        );
        assert_eq!(
            render(
                "合計 {total} 打 · {kpm} 打/分{ai}",
                4528,
                69450,
                141,
                "0:15",
                true
            ),
            "合計 69,450 打 · 141 打/分 · AI対応中"
        );
        assert_eq!(
            render("{elapsed} / {kpm}", 0, 0, 0, "1:02:03", false),
            "1:02:03 / 0"
        );
        assert_eq!(render("{today}", 0, 0, 0, "", false), "0");
        assert_eq!(render("{today}", 1234567, 0, 0, "", false), "1,234,567");
    }

    #[test]
    fn refresh_if_changed_follows_settings_written_by_another_process() {
        let directory = tempfile::tempdir().unwrap();
        let mut settings = load(directory.path());
        settings.enabled = false;
        save_settings(directory.path(), &settings).unwrap();

        // 入力プロセスは数えない設定で起動する。
        let mut counter = KeyCounter::load(directory.path());
        assert!(!counter.enabled());
        counter.count_key("2026-02-14");
        assert!(!counter.has_pending());

        // 設定画面がファイルを書き換えたら、次の打鍵で読み直して数え始める。
        settings.enabled = true;
        save_settings(directory.path(), &settings).unwrap();
        counter.refresh_if_changed();
        assert!(counter.enabled());
        counter.count_key("2026-02-14");
        assert!(counter.has_pending());
        counter.flush().unwrap();
        assert_eq!(load(directory.path()).total, 1);

        // 文言の変更も次の打鍵で反映される。
        settings.details = "きょう {today}".into();
        save_settings(directory.path(), &settings).unwrap();
        counter.refresh_if_changed();
        let count = load(directory.path());
        assert_eq!(count.details, "きょう {today}");
        assert!(count.enabled);
    }
}
