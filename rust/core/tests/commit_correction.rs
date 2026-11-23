// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use meltype_core::{session::SessionResult, Session};
fn apply(text: &mut String, result: SessionResult) {
    for edit in result.commits {
        if let Some(expected) = edit.expect {
            assert!(text.ends_with(&expected), "{text:?} / {expected:?}");
            assert_eq!(expected.encode_utf16().count(), edit.delete_before);
            text.truncate(text.len() - expected.len());
        } else {
            assert_eq!(edit.delete_before, 0);
        }
        text.push_str(&edit.text);
    }
}
fn type_text(session: &mut Session, text: &mut String, keys: &str) {
    for c in keys.chars() {
        let result = session.key(if c == ' ' { 32 } else { 0 }, c as i32, 0);
        if !result.consumed {
            text.push(c);
        }
        apply(text, result);
    }
}
#[test]
fn following_english_repairs_ambiguous_commits_and_preserves_suru_and_spaces() {
    for (keys, expected) in [
        ("site PR", "して PR"),
        ("i want", "I want"),
        ("make sure you have", "make sure you have"),
    ] {
        let mut s = Session::default();
        s.can_delete = true;
        if keys.starts_with("i ") {
            s.set_context(Some("前"), None);
        }
        let mut text = String::new();
        type_text(&mut s, &mut text, keys);
        apply(&mut text, s.commit());
        assert_eq!(text, expected, "{keys}");
    }
}
#[test]
fn correction_requires_capability_setting_matching_context_and_unchosen_input() {
    for mode in 0..5 {
        let mut s = Session::default();
        s.can_delete = mode != 0;
        if mode == 1 {
            s.settings
                .set("AutoCorrectAfterCommit", false.into())
                .unwrap();
        }
        let mut text = String::new();
        type_text(&mut s, &mut text, "site ");
        apply(&mut text, s.commit());
        if mode == 2 {
            s.caret_moved(0, false);
        }
        if mode == 3 {
            s.set_context(Some("elsewhere"), None);
        }
        if mode == 4 {
            s.preferences.english.insert("site".into(), false);
        }
        type_text(&mut s, &mut text, "PR");
        let result = s.commit();
        assert!(
            result.commits.iter().all(|e| e.expect.is_none()),
            "mode {mode}"
        );
    }
}
#[test]
fn explicit_language_choice_and_reset_disable_retroactive_replacement() {
    let mut s = Session::default();
    s.can_delete = true;
    let mut text = String::new();
    type_text(&mut s, &mut text, "site");
    s.key(0x75, 0, 0);
    apply(&mut text, s.commit());
    type_text(&mut s, &mut text, "PR");
    assert!(s.commit().commits.iter().all(|e| e.expect.is_none()));
    s.reset();
    type_text(&mut s, &mut text, "site ");
    apply(&mut text, s.commit());
    s.reset();
    type_text(&mut s, &mut text, "PR");
    assert!(s.commit().commits.iter().all(|e| e.expect.is_none()));
    let mut s = Session::default();
    s.can_delete = true;
    let mut text = String::new();
    type_text(&mut s, &mut text, "site ");
    apply(&mut text, s.select(0));
    type_text(&mut s, &mut text, "PR");
    assert!(s.commit().commits.iter().all(|e| e.expect.is_none()));
}

#[test]
fn following_japanese_repairs_only_known_readings_and_preserves_english_context() {
    for (keys, expected) in [
        ("sushi nihongo", "すしにほんご"),
        ("issue nihongo", "issue にほんご"),
        ("Are you nihongo", "Are you にほんご"),
    ] {
        let mut s = Session::default();
        s.can_delete = true;
        let mut text = String::new();
        type_text(&mut s, &mut text, keys);
        apply(&mut text, s.commit());
        assert_eq!(text, expected, "{keys}");
    }
}

#[test]
fn issue_834_shifted_english_after_japanese_survives_space() {
    let mut session = Session::default();
    session.can_delete = true;
    session.set_context(Some("ああああ"), Some(""));
    let mut document = String::from("ああああ");
    type_text(&mut session, &mut document, "are");
    apply(&mut document, session.key(13, 0, 0));
    assert_eq!(document, "あああああれ");
    for ch in "ON".chars() {
        apply(&mut document, session.key(ch as i32, ch as i32, 1));
    }
    apply(&mut document, session.key(32, 32, 0));
    assert_eq!(document, "あああああれON ");
}
