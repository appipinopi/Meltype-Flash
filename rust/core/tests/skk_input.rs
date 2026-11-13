// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::Session;
use serde_json::json;

fn skk() -> Session {
    let mut session = Session::default();
    session
        .settings
        .set("SkkMode", json!(true))
        .unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session
}

#[test]
fn lowercase_passes_through_to_normal_input() {
    let mut session = skk();
    session.key(0, 'a' as i32, 0);
    assert_eq!(session.display(false), "あ");
}

#[test]
fn registered_word_converts_and_commits() {
    let mut session = skk();
    session.skk_register("かんじ", "漢字");
    for c in "Kanji".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.display(false), "▽かんじ");
    session.key(0, ' ' as i32, 0);
    assert_eq!(session.display(false), "▼漢字");
    let result = session.key(0, '\r' as i32, 0);
    assert_eq!(result.commits[0].text, "漢字");
}

#[test]
fn okurigana_uses_the_marked_reading() {
    let mut session = skk();
    session.skk_register("かんじk", "感じ");
    for c in "Kanji".chars() {
        session.key(0, c as i32, 0);
    }
    session.key(0, 'K' as i32, 0);
    session.key(0, 'u' as i32, 0);
    assert_eq!(session.display(false), "▽かんじ*く");
    session.key(0, ' ' as i32, 0);
    assert_eq!(session.display(false), "▼感じ");
    let result = session.key(0, '\r' as i32, 0);
    assert_eq!(result.commits[0].text, "感じ");
}

#[test]
fn unknown_reading_registers_a_new_word() {
    let mut session = skk();
    session.key(0, 'K' as i32, 0);
    session.key(0, 'a' as i32, 0);
    session.key(0, ' ' as i32, 0);
    assert_eq!(session.display(false), "[辞書登録] か: ");
    for c in "新語".chars() {
        session.key(0, c as i32, 0);
    }
    let result = session.key(0, '\r' as i32, 0);
    assert_eq!(result.commits[0].text, "新語");
    assert_eq!(session.skk_lookup("か"), &["新語".to_string()]);
}

#[test]
fn skk_view_exposes_candidates_for_host_ui() {
    let mut session = skk();
    session.skk_register("かんじ", "漢字");
    session.skk_register("かんじ", "感じ");
    for c in "Kanji".chars() {
        session.key(0, c as i32, 0);
    }
    session.key(0, ' ' as i32, 0);
    let view = session.result(true).view.unwrap();
    assert!(view.converting);
    assert_eq!(view.text, "▼感じ");
    assert_eq!(view.candidates, vec!["感じ", "漢字"]);
    assert_eq!(view.selected_index, 0);

    session.key(0, ' ' as i32, 0);
    let view = session.result(true).view.unwrap();
    assert_eq!(view.text, "▼漢字");
    assert_eq!(view.selected_index, 1);
}
