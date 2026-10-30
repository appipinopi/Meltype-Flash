// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
use meltype_core::Session;
use serde_json::json;

fn key(s: &mut Session, c: char) -> meltype_core::SessionResult {
    s.key(
        match c {
            ' ' => 32,
            '\n' => 13,
            _ => 0,
        },
        c as i32,
        0,
    )
}
fn passes(s: &mut Session, raw: &str) {
    for c in raw.chars() {
        let r = key(s, c);
        assert!(
            !r.consumed && r.view.is_none() && r.commits.is_empty(),
            "{raw}: {c}"
        );
    }
    assert!(s.raw().is_empty());
}
// v1.4.0 reserves an initial slash in prose for ・. Commands still pass in
// code/terminal fields, and in prose after explicitly choosing / with Space.
fn command(s: &mut Session, raw: &str) {
    s.code_input = true;
    passes(s, raw);
    s.code_input = false;
}
#[test]
fn v140_initial_slash_is_middle_dot_in_prose_comments_and_strings() {
    for (code, before) in [
        (false, ""),
        (false, "文章 "),
        (true, "// "),
        (true, "let s = \" "),
    ] {
        let mut s = Session::default();
        s.code_input = code;
        s.set_context(Some(before), None);
        for c in "/ka".chars() {
            assert!(key(&mut s, c).consumed);
        }
        assert_eq!(s.display(false), "・か", "{before:?}");
        assert_eq!(s.commit().commits[0].text, "・か");
    }
    for before in ["", "> ", "let path = "] {
        let mut s = Session::default();
        s.code_input = true;
        s.set_context(Some(before), None);
        passes(&mut s, "/help");
    }
    let mut s = Session::default();
    key(&mut s, '/');
    let result = key(&mut s, ' ');
    assert_eq!(result.view.unwrap().candidates, ["/", "・", "･", "／"]);
    let result = key(&mut s, 'h');
    assert!(!result.consumed);
    assert_eq!(result.commits[0].text, "/");
    passes(&mut s, "elp");
}
#[test]
fn v140_initial_slash_retains_kana_width_and_backspace_state() {
    for (vk, expected) in [
        (0x75, "・か"),
        (0x76, "・カ"),
        (0x77, "･ｶ"),
        (0x78, "／ｋａ"),
        (0x79, "/ka"),
    ] {
        let mut s = Session::default();
        for c in "/ka".chars() {
            key(&mut s, c);
        }
        s.key(vk, 0, 0);
        assert_eq!(s.display(false), expected);
    }
    let mut s = Session::default();
    key(&mut s, '/');
    s.key(8, 0, 0);
    assert!(s.raw().is_empty());
    for c in "ka".chars() {
        key(&mut s, c);
    }
    assert_eq!(s.commit().commits[0].text, "か");
    s.code_input = true;
    passes(&mut s, "/help");
}
#[test]
fn upstream_sigil_words_pass_each_key_until_space_then_resume_japanese() {
    // CompositionTests.SigilWord_AtStartPassesNameToTheApp and
    // SessionFacadeTests.SigilWord_PassesToTheAppUntilSpace.
    for word in [
        "/review",
        "$skill-name",
        "@src/app.ts",
        "/kensaku",
        "/plugin:cmd",
        "@file.rs",
    ] {
        let mut s = Session::default();
        if word.starts_with('/') {
            command(&mut s, word);
        } else {
            passes(&mut s, word);
        }
        passes(&mut s, " ");
        for c in "kyou".chars() {
            assert!(key(&mut s, c).consumed);
        }
        assert_eq!(s.display(false), "きょう");
    }
    let mut s = Session::default();
    for c in "google ".chars() {
        key(&mut s, c);
    }
    passes(&mut s, "@file");
}
#[test]
fn host_context_ends_a_sigil_when_space_or_newline_was_not_reported_as_a_key() {
    for before in ["/xyz ", "/xyz\r\n", "/xyz\n"] {
        let mut s = Session::default();
        s.set_context(Some(""), None);
        command(&mut s, "/xyz");
        s.set_context(Some(before), None);
        for c in "kyou".chars() {
            key(&mut s, c);
        }
        assert_eq!(s.display(false), "きょう", "{before:?}");
    }
    let mut s = Session::default();
    command(&mut s, "/xy");
    s.set_context(Some("/xy"), None);
    passes(&mut s, "z");
    s.set_context(Some("/xyz "), None);
    command(&mut s, "/next");
}
#[test]
fn boundaries_use_nonempty_context_but_empty_context_does_not_override_observed_letters() {
    let mut s = Session::default();
    s.set_context(Some("taro"), None);
    assert!(key(&mut s, '@').consumed);
    let mut s = Session::default();
    s.caret_moved(37, false);
    s.set_context(Some("> "), None);
    command(&mut s, "/review");
    let mut s = Session::default();
    s.direct = true;
    passes(&mut s, "taro");
    s.direct = false;
    s.set_context(Some(""), None);
    assert!(key(&mut s, '@').consumed);
}
#[test]
fn escape_keeps_completion_active_backspace_restores_boundary_and_tab_requires_new_context() {
    let mut s = Session::default();
    passes(&mut s, "@file");
    assert!(!s.key(27, 0, 0).consumed);
    passes(&mut s, ".rs");
    for _ in 0..8 {
        assert!(!s.key(8, 0, 0).consumed);
    }
    command(&mut s, "/next");
    assert!(!s.key(9, 0, 0).consumed);
    s.set_context(Some("/next "), None);
    for c in "nihon".chars() {
        key(&mut s, c);
    }
    assert_eq!(s.display(true), "にほん");
    s.reset();
    passes(&mut s, "$skill");
    s.caret_moved(37, true);
    s.set_context(Some("word"), None);
    assert!(key(&mut s, '@').consumed);
}
#[test]
fn direct_detection_never_steals_sigil_names_and_settings_or_kana_can_disable_passthrough() {
    let mut s = Session::default();
    s.direct = true;
    s.direct_detection = true;
    s.can_delete = true;
    passes(&mut s, "/");
    let mut before = String::from("/");
    for c in "kensaku".chars() {
        s.set_context(Some(&before), None);
        passes(&mut s, &c.to_string());
        before.push(c);
    }
    for setting in ["SigilWordsDirect", "InputStyle"] {
        let mut s = Session::default();
        s.settings
            .set(
                setting,
                if setting == "InputStyle" {
                    json!("Kana")
                } else {
                    json!(false)
                },
            )
            .unwrap();
        assert!(key(&mut s, '/').consumed, "{setting}");
    }
    let mut s = Session::default();
    command(&mut s, "/review");
    assert!(!s.key(67, 0, 2).consumed);
    for c in "kyou".chars() {
        key(&mut s, c);
    }
    assert_eq!(s.display(false), "きょう");
}
