// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 同梱したリリースノートと、更新後に一度だけ出すための状態。
//!
//! ノートは `rust/release-notes/<版>.<言語>.md` を実行ファイルへ埋め込む。
//! 「最後に起動した版」と「表示した版」はプロファイルの外 (release-notes.json) に置き、
//! 設定のバックアップや別プロファイルへ混ぜない。
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FILE: &str = "release-notes.json";
const JA_1_1_3: &str = include_str!("../../release-notes/1.1.3.ja.md");
const EN_1_1_3: &str = include_str!("../../release-notes/1.1.3.en.md");
const JA_1_1_2: &str = include_str!("../../release-notes/1.1.2.ja.md");
const EN_1_1_2: &str = include_str!("../../release-notes/1.1.2.en.md");

/// その版のリリースノート。同梱していなければ None。
pub fn notes(version: &str, english: bool) -> Option<&'static str> {
    match version {
        "1.1.3" => Some(if english { EN_1_1_3 } else { JA_1_1_3 }),
        "1.1.2" => Some(if english { EN_1_1_2 } else { JA_1_1_2 }),
        _ => None,
    }
}

/// GitHub のその版のリリースページ。
pub fn release_url(version: &str) -> String {
    format!("https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v{version}")
}

fn path(directory: &Path) -> PathBuf {
    directory.join(FILE)
}

fn read(directory: &Path) -> Value {
    std::fs::read(path(directory))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn write(directory: &Path, state: &Value) {
    if let Ok(bytes) = serde_json::to_vec_pretty(state) {
        let _ = meltype_core::storage::atomic_write(&path(directory), &bytes);
    }
}

/// 更新後の初回起動かを判定し、リリースノートを一度だけ出す。
///
/// `first_run` は新しく入れた直後 (config.json が無い) を true にする。
/// 新規導入では出さず、前の版から更新したときだけ出す。状態は必ず更新する。
pub fn take_update_notice(directory: &Path, first_run: bool) -> bool {
    let mut state = read(directory);
    let previous = state["lastVersion"].as_str();
    let shown = state["shownVersion"].as_str();
    let should_show = !first_run && previous != Some(VERSION) && shown != Some(VERSION);
    if should_show {
        state["shownVersion"] = json!(VERSION);
    }
    state["lastVersion"] = json!(VERSION);
    write(directory, &state);
    should_show
}

/// 利用者がリリースノートを開いたことを記録する。
pub fn mark_shown(directory: &Path) {
    let mut state = read(directory);
    state["shownVersion"] = json!(VERSION);
    state["lastVersion"] = json!(VERSION);
    write(directory, &state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_exist_in_both_languages() {
        let japanese = notes("1.1.3", false).unwrap();
        let english = notes("1.1.3", true).unwrap();
        assert!(japanese.contains("同意"));
        assert!(english.contains("consent"));
        assert!(notes("0.0.0", false).is_none());
        assert!(release_url("1.1.3").ends_with("/flash-v1.1.3"));
    }

    #[test]
    fn update_notice_is_shown_once_and_never_on_a_fresh_install() {
        let directory = tempfile::tempdir().unwrap();
        // 新規導入では出さず、その版を記録する。
        assert!(!take_update_notice(directory.path(), true));
        assert!(!take_update_notice(directory.path(), false));

        // 前の版から更新したときは一度だけ出す。
        std::fs::write(path(directory.path()), r#"{"lastVersion":"1.1.1"}"#).unwrap();
        assert!(take_update_notice(directory.path(), false));
        assert!(!take_update_notice(directory.path(), false));

        // 明示的に開いたときも、次からは自動では出さない。
        std::fs::write(path(directory.path()), r#"{"lastVersion":"1.0.0"}"#).unwrap();
        mark_shown(directory.path());
        assert!(!take_update_notice(directory.path(), false));
    }
}
