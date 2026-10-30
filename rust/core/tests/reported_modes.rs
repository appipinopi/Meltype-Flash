// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 残っていたモード切替・文節操作の報告を再現する。
use meltype_core::{session::Clause, Session};
use serde_json::json;

fn type_text(session: &mut Session, text: &str) {
    for c in text.chars() {
        session.key(0, c as i32, 0);
    }
}

#[test]
fn issue_1095_function_keys_on_english_change_only_the_selected_clause() {
    for (vk, expected) in [
        (0x75, "みぢの橋"),
        (0x76, "ミヂの橋"),
        (0x77, "ﾐﾁﾞの橋"),
        (0x78, "ｍｉｄｉの橋"),
        (0x79, "midiの橋"),
    ] {
        let mut session = Session::default();
        session
            .settings
            .set("LiveConversion", json!(false))
            .unwrap();
        session
            .settings
            .set("FunctionKeysAffectSelectedClause", json!(true))
            .unwrap();
        type_text(&mut session, "midinohashi");
        let mut english = Clause::new("midi".into(), vec!["midi".into()]);
        english.english = true;
        english.kana = Some("みぢ".into());
        assert!(session.begin_clauses(vec![
            english,
            Clause::new("の".into(), vec!["の".into()]),
            Clause::new("はし".into(), vec!["橋".into()])
        ]));
        let result = session.key(vk, 0, 0);
        let view = result.view.unwrap();
        assert!(view.converting, "F key must retain the other clauses");
        assert_eq!(view.text, expected, "key {vk:x}");
        assert_eq!(session.commit().commits[0].text, expected);
    }
}

#[test]
fn issue_1033_configured_shift_space_switches_without_adding_a_space() {
    let mut session = Session::default();
    session
        .settings
        .set("ToggleKeyboardShortcuts", json!("Shift+Space; Ctrl+Space"))
        .unwrap();
    assert!(!session.settings.0.contains_key("ToggleKeyboardShortcuts"));
    // 旧形式から1.4形式へ移行し、保存・再読込後もShift+Spaceを保持する。
    session.settings =
        meltype_core::settings::Settings::parse(&serde_json::to_string(&session.settings).unwrap())
            .unwrap();
    type_text(&mut session, "kyou");
    let displayed = session.display(false);
    let result = session.mode_key(32, 1).unwrap();
    assert_eq!(result.commits[0].text, displayed);
    assert!(session.input_is_direct());
    assert!(!session.key(0, 'k' as i32, 0).consumed);
    let result = session.mode_key(32, 2).unwrap();
    assert!(result.commits.is_empty());
    assert!(!session.input_is_direct());
    assert!(session.mode_key(32, 4).is_none());
    let mut defaults = Session::default();
    assert!(defaults.mode_key(32, 1).is_none());
    assert_eq!(defaults.key(32, 32, 1).commits[0].text, "　");
}

#[test]
fn issue_1022_terminal_toggles_japanese_and_caret_movement_returns_to_code() {
    let mut session = Session::default();
    session.code_input = true;
    assert!(session.input_is_direct());
    assert!(!session.key(0, 'n' as i32, 0).consumed);
    session.mode_key(0x19, 0).unwrap();
    assert!(!session.input_is_direct());
    type_text(&mut session, "nihongo");
    assert_eq!(session.display(false), "にほんご");
    let result = session.mode_key(0x19, 0).unwrap();
    assert_eq!(result.commits[0].text, "にほんご");
    assert!(session.input_is_direct());
    session.mode_key(0x19, 0).unwrap();
    session.caret_moved(0x25, false);
    assert!(session.input_is_direct());
}

#[test]
fn issue_925_fixed_mode_keys_are_idempotent_and_commit_once() {
    let mut session = Session::default();
    type_text(&mut session, "kyou");
    let result = session.mode_key(0x16, 0).unwrap();
    assert!(result.commits.is_empty());
    assert_eq!(session.raw(), "kyou");
    let result = session.mode_key(0x1a, 0).unwrap();
    assert_eq!(result.commits[0].text, "きょう");
    assert!(session.input_is_direct());
    assert!(session.mode_key(0x1a, 0).unwrap().commits.is_empty());
    session.mode_key(0x16, 0).unwrap();
    assert!(!session.input_is_direct());
    type_text(&mut session, "a");
    assert_eq!(session.display(false), "あ");
}

#[test]
fn v140_typed_mode_shortcuts_obey_custom_bindings_and_disabled_defaults() {
    let mut session = Session::default();
    session.settings.set("ToggleShortcuts", json!([])).unwrap();
    assert!(session.mode_key(0x19, 0).is_none());
    session
        .settings
        .set("ToggleShortcuts", json!([{"Key":75,"Modifiers":6}]))
        .unwrap();
    assert!(session.mode_key(0x19, 0).is_none());
    assert!(session.mode_key(0x4b, 6).is_some());
    assert!(session.input_is_direct());
    assert!(session.mode_key(0x4b, 6).is_some());
    assert!(!session.input_is_direct());
}
