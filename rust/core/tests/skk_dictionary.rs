// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(feature = "filesystem")]
use meltype_core::{storage::Store, Session};
use std::fs;

#[test]
fn session_loads_skk_dictionaries_and_persists_registration() {
    let dir = tempfile::tempdir().unwrap();
    let dictionaries = dir.path().join("dictionaries");
    fs::create_dir_all(&dictionaries).unwrap();
    fs::write(dictionaries.join("skk-jisyo.txt"), "あ /亜/阿/\n").unwrap();
    fs::write(dictionaries.join("skk-user-jisyo.txt"), "あ /唖/\n").unwrap();

    let mut session = Session::open(Store::new(dir.path())).unwrap();
    assert_eq!(session.skk_lookup("あ")[0], "唖");
    assert!(session.skk_register("てすと", "テスト"));

    let saved = fs::read_to_string(dictionaries.join("skk-user-jisyo.txt")).unwrap();
    assert!(saved.contains("てすと"));
    assert!(saved.contains("テスト"));

    let reopened = Session::open(Store::new(dir.path())).unwrap();
    assert_eq!(reopened.skk_lookup("てすと"), &["テスト".to_string()]);
}
