// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 初回起動時の同意。
//!
//! 入力を使い始める前に、ライセンスと利用状況の扱いを表示し、同意した人だけ
//! 設定画面へ進める。同意した版を保存し、文面を変えたときは version を上げる。
use serde_json::{json, Value};
use std::path::Path;

/// 同意の文面の版。文面を変えたら上げる (前の版に同意した人にも、もう一度同意してもらう)。
pub const VERSION: u64 = 1;
const FILE: &str = "consent.json";

/// 同意済みか。ファイルが無い・読めない・古い版のときは false。
pub fn accepted(directory: &Path) -> bool {
    std::fs::read(directory.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value["version"].as_u64())
        .is_some_and(|version| version >= VERSION)
}

/// 同意を保存する。
pub fn accept(directory: &Path) -> std::io::Result<()> {
    let bytes =
        serde_json::to_vec_pretty(&json!({ "version": VERSION })).map_err(std::io::Error::other)?;
    meltype_core::storage::atomic_write(&directory.join(FILE), &bytes)
}

/// スタート画面に表示する文面。
pub fn text() -> &'static str {
    if cfg!(any(windows, target_os = "linux")) {
        "Meltype-FLASH を使うには、次の内容に同意してください。\n\n\
        ・本ソフトは GNU GPL v3 以降で提供され、無保証です。\n\
        ・より良い入力のために、利用状況（アプリの版・OSの版・起動回数・画面を開いた回数）を匿名で送信します。この送信は設定で停止できません。\n\
        ・入力内容・入力履歴・辞書・設定値・入力欄の文字列は送信しません。\n\
        ・送信先は Aptabase です。\n\n\
        同意しない場合は、この画面を閉じてアプリを終了してください。"
    } else {
        "Meltype-FLASH を使うには、次の内容に同意してください。\n\n\
        ・本ソフトは GNU GPL v3 以降で提供され、無保証です。\n\
        ・この版は利用状況を送信しません。\n\
        ・入力内容・入力履歴・辞書・設定値・入力欄の文字列を外部へ送信しません。\n\n\
        同意しない場合は、この画面を閉じてアプリを終了してください。"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptance_is_stored_with_the_current_version() {
        let directory = tempfile::tempdir().unwrap();
        assert!(!accepted(directory.path()));
        accept(directory.path()).unwrap();
        assert!(accepted(directory.path()));
        std::fs::write(directory.path().join(FILE), r#"{"version":0}"#).unwrap();
        assert!(!accepted(directory.path()));
        std::fs::write(directory.path().join(FILE), "not json").unwrap();
        assert!(!accepted(directory.path()));
    }
}
