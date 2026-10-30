// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use meltype_core::{dictionary::Dictionary, Session};
fn type_text(s: &mut Session, text: &str) {
    for c in text.chars() {
        assert!(s.key(0, c as i32, 0).consumed);
    }
}
#[test]
fn closed_issue_338_z_shortcuts_keep_raw_keys_and_do_not_rewrite_puzzle() {
    for scored in [false, true] {
        for (raw, expected) in [
            ("zh", "←"),
            ("zj", "↓"),
            ("zk", "↑"),
            ("zl", "→"),
            ("ZH", "←"),
            ("ｚｈ", "←"),
            ("ｚｊ", "↓"),
            ("ｚｋ", "↑"),
            ("ｚｌ", "→"),
            ("z/", "・"),
            ("z.", "…"),
            ("z,", "‥"),
            ("z-", "～"),
            ("z[", "『"),
            ("z]", "』"),
            ("kazh", "か←"),
        ] {
            let mut s = Session::default();
            s.settings
                .set("LiveConversion", serde_json::json!(false))
                .unwrap();
            s.settings
                .set("ScoredSegmentation", serde_json::json!(scored))
                .unwrap();
            type_text(&mut s, raw);
            assert_eq!(s.display(false), expected, "{raw}: preedit");
            assert_eq!(s.commit().commits[0].text, expected, "{raw}: commit");
        }
    }
    let mut s = Session::default();
    type_text(&mut s, "zh");
    s.key(0x79, 0, 0);
    assert_eq!(
        s.commit().commits[0].text,
        "zh",
        "F10 retains the original keys"
    );
    type_text(&mut s, "puzzle");
    assert!(
        !s.display(false).contains('→'),
        "zz is not a pending single z"
    );
    for raw in ["./zh", "./z]", "https://example.com/zl"] {
        let mut s = Session::default();
        s.settings
            .set("SigilWordsDirect", serde_json::json!(false))
            .unwrap();
        s.settings
            .set("LiveConversion", serde_json::json!(false))
            .unwrap();
        type_text(&mut s, raw);
        assert!(
            !s.display(false).chars().any(|c| "←↓↑→『』".contains(c)),
            "{raw}"
        );
    }
}

#[test]
fn closed_issue_mixed_word_regressions_remain_fixed() {
    assert_eq!(Dictionary::bundled().mixed("gameover", true), "gameover");
    let mut failures = Vec::new();
    for (issue, raw, expected) in [
        (12, "feature", "feature"),
        (378, "memoirs", "memoirs"),
        (377, "gameover", "gameover"),
        (379, "faceni", "faceに"),
        (376, "passwordnishite", "passwordにして"),
        (129, "AInituite", "AIについて"),
        (835, "docs/", "docs/"),
        (77, "reflectsareta", "reflectされた"),
        (69, "inviteshimashita", "inviteしました"),
    ] {
        let mut s = Session::default();
        s.settings
            .set("LiveConversion", serde_json::json!(false))
            .unwrap();
        type_text(&mut s, raw);
        let actual = s.commit().commits[0].text.clone();
        if actual != expected {
            failures.push(format!("#{issue}: {raw}: {actual:?} != {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn closed_issue_833_uppercase_ten_reconverts_as_japanese_with_final_n() {
    let mut s = Session::default();
    s.settings
        .set("LiveConversion", serde_json::json!(false))
        .unwrap();
    type_text(&mut s, "TEN");
    let reply = s.key(0x1c, 0, 0);
    let view = reply.view.unwrap();
    assert!(
        view.candidates.contains(&"てん".to_owned()),
        "{:?}",
        view.candidates
    );
}

#[test]
fn closed_issue_599_compatible_shortcuts_resize_only_when_enabled() {
    use meltype_core::session::Clause;
    for enabled in [false, true] {
        let mut s = Session::default();
        s.settings
            .set("CompatibleShortcuts", serde_json::json!(enabled))
            .unwrap();
        type_text(&mut s, "hashiwotoru");
        assert!(s.begin_clauses(vec![
            Clause::new("はしを".into(), vec!["橋を".into()]),
            Clause::new("とる".into(), vec!["取る".into()]),
        ]));
        let reply = s.key(0x4b, 0, 2);
        assert_eq!(reply.consumed, enabled);
        if enabled {
            assert!(reply.commits.is_empty());
            assert_eq!(s.clause_readings(), ["はし", "をとる"]);
            assert!(s.key(0x4c, 0, 2).consumed);
            assert_eq!(s.clause_readings(), ["はしを", "とる"]);
        } else {
            assert_eq!(reply.commits[0].text, "橋を取る");
            assert!(s.raw().is_empty());
        }
    }
}

#[test]
fn closed_issue_490_typing_after_function_keys_commits_only_the_explicit_word() {
    for (function, count, expected) in [(0x76, 1, "テスト"), (0x79, 2, "TESUTO")] {
        let mut s = Session::default();
        s.settings
            .set("LiveConversion", serde_json::json!(false))
            .unwrap();
        type_text(&mut s, "tesuto");
        for _ in 0..count {
            s.key(function, 0, 0);
        }
        assert_eq!(s.display(false), expected);
        let first = s.key(0x44, 'd' as i32, 0);
        assert_eq!(first.commits[0].text, expected);
        type_text(&mut s, "esu");
        assert_eq!(s.display(false), "です");
        assert_eq!(s.commit().commits[0].text, "です");
    }
}

#[test]
fn mixed_language_and_protected_spans() {
    let d = Dictionary::bundled();
    for (raw, expected) in [
        ("kyouhagoogledekensaku", "きょうはgoogleでけんさく"),
        ("ashitanomeetingwotsuika", "あしたのmeetingをついか"),
        ("https://example.com/a?b=1", "https://example.com/a?b=1"),
        ("/command $skill @file.rs", "/command $skill @file.rs"),
        ("I want to go to the park", "I want to go to the park"),
        ("日本語😀", "日本語😀"),
    ] {
        assert_eq!(d.mixed(raw, true), expected, "{raw}");
    }
}
#[test]
fn conversion_candidate_and_learning() {
    let mut s = Session::default();
    type_text(&mut s, "hashi");
    let result = s.key(0x20, 32, 0);
    let view = result.view.unwrap();
    assert!(view.converting);
    assert!(view.candidates.contains(&"橋".to_owned()));
    let index = view.candidates.iter().position(|c| c == "箸").unwrap();
    assert_eq!(s.select(index).commits[0].text, "箸");
    type_text(&mut s, "hashi");
    assert_eq!(s.key(0x20, 32, 0).view.unwrap().text, "箸");
}
#[test]
fn shortcuts_commit_once_and_pass_through() {
    let mut s = Session::default();
    type_text(&mut s, "nihongo");
    let result = s.key(0x43, 99, 2);
    assert!(!result.consumed);
    assert_eq!(result.commits.len(), 1);
    assert_eq!(result.commits[0].text, "にほんご");
    assert!(s.commit().commits.is_empty());
}
#[test]
fn edit_cancel_and_function_keys() {
    let mut s = Session::default();
    type_text(&mut s, "kya");
    s.key(8, 0, 0);
    assert_eq!(s.display(false), "");
    type_text(&mut s, "kya");
    assert_eq!(s.key(0x76, 0, 0).view.unwrap().text, "キャ");
    assert_eq!(s.key(13, 0, 0).commits[0].text, "キャ");
    type_text(&mut s, "test");
    s.key(27, 0, 0);
    assert!(s.commit().commits.is_empty());
}
#[test]
fn invalid_unicode_leaves_composition_intact() {
    let mut s = Session::default();
    type_text(&mut s, "ka");
    for scalar in [-1, 0xd800, 0x110000] {
        assert!(!s.key(0, scalar, 0).consumed);
        assert_eq!(s.raw(), "ka");
    }
}
#[test]
fn private_input_never_learns_or_captures() {
    let mut s = Session::default();
    s.private = true;
    assert!(!s.key(0, 97, 0).consumed);
    assert!(s.preferences.english.is_empty());
}
#[test]
fn user_dictionary_is_used() {
    let mut s = Session::default();
    s.preferences
        .user_dictionary
        .insert("めると".into(), vec!["Meltype".into()]);
    type_text(&mut s, "meruto");
    assert_eq!(s.key(32, 32, 0).view.unwrap().text, "Meltype");
}
#[test]
fn typing_after_conversion_keeps_commit() {
    let mut s = Session::default();
    type_text(&mut s, "hashi");
    s.key(32, 32, 0);
    let r = s.key(0, 97, 0);
    assert_eq!(r.commits.len(), 1);
    assert_eq!(r.view.unwrap().text, "あ");
}

#[test]
fn segmented_conversion_selection_resize_cancel_and_learning() {
    use meltype_core::session::Clause;
    let mut s = Session::default();
    type_text(&mut s, "hashiwotoru");
    assert!(!s.begin_clauses(vec![Clause::new("ちがう".into(), vec!["違う".into()])]));
    assert!(s.begin_clauses(vec![
        Clause::new("はしを".into(), vec!["橋を".into(), "箸を".into()]),
        Clause::new("とる".into(), vec!["取る".into(), "撮る".into()]),
    ]));
    assert_eq!(s.key(0x28, 0, 0).view.unwrap().text, "箸を取る");
    let view = s.key(0x27, 0, 0).view.unwrap();
    assert_eq!(view.selected_clause, 1);
    assert_eq!(view.clauses, ["箸を", "取る"]);
    assert!(s.select(1).commits.is_empty());
    assert_eq!(s.display(false), "箸を撮る");
    assert_eq!(s.commit().commits[0].text, "箸を撮る");
    assert_eq!(s.preferences.selected["はしを"], "箸を");
    type_text(&mut s, "hashi");
    s.key(0x20, 32, 0);
    let view = s.key(0x25, 0, 1).view.unwrap();
    assert_eq!(view.clauses.len(), 2);
    assert_eq!(view.clauses.concat(), view.text);
    let view = s.key(0x27, 0, 1).view.unwrap();
    assert_eq!(view.clauses.len(), 1);
    let view = s.key(0x1b, 0, 0).view.unwrap();
    assert!(!view.converting);
    assert!(view.clauses.is_empty());
    assert_eq!(view.text, "はし");
}

#[test]
fn undo_requires_capability_and_matching_surrounding_text() {
    let mut s = Session::default();
    type_text(&mut s, "hashi");
    s.key(0x20, 32, 0);
    let text = s.commit().commits[0].text.clone();
    assert!(!s.undo_commit().consumed);
    s.can_delete = true;
    s.set_context(Some(&format!("prefix{text}")), None);
    let result = s.undo_commit();
    assert_eq!(result.commits[0].expect.as_deref(), Some(text.as_str()));
    assert_eq!(result.commits[0].delete_before, text.encode_utf16().count());
    assert_eq!(result.view.unwrap().text, "はし");
    s.commit();
    s.set_context(Some("different"), None);
    assert!(!s.undo_commit().consumed);
    assert!(!s.undo_commit().consumed);
    type_text(&mut s, "😀");
    s.commit();
    assert_eq!(s.undo_commit().commits[0].delete_before, 2);
    s.reset();
    assert!(!s.undo_commit().consumed);
}

#[test]
fn live_preview_retains_reading_promotes_to_clauses_and_respects_settings() {
    use meltype_core::session::Clause;
    let mut s = Session::default();
    type_text(&mut s, "hashi");
    assert!(
        s.live_reading().is_none(),
        "upstream requires four UTF-16 units"
    );
    s.reset();
    type_text(&mut s, "nihongo");
    assert_eq!(s.live_reading().as_deref(), Some("にほんご"));
    let clause = || Clause::new("にほんご".into(), vec!["日本語".into(), "日本語訳".into()]);
    assert!(s.update_live_conversion(vec![clause()]));
    assert!(!s.is_converting());
    assert_eq!(s.display(false), "日本語");
    assert_eq!(s.reading(), "にほんご");
    s.begin_conversion(vec![]);
    assert!(s.is_converting());
    assert_eq!(s.result(false).view.unwrap().clauses, ["日本語"]);
    s.select(1);
    type_text(&mut s, "nihongo");
    s.update_live_conversion(vec![clause()]);
    assert_eq!(s.display(false), "日本語訳");
    s.key(8, 0, 0);
    assert_eq!(s.display(false), "にほん");
    s.reset();
    type_text(&mut s, "hello");
    assert!(s.live_reading().is_none());
    s.reset();
    type_text(&mut s, "nihongo");
    s.settings
        .set("LiveConversion", serde_json::json!(false))
        .unwrap();
    assert!(!s.update_live_conversion(vec![clause()]));
    assert_eq!(s.display(false), "にほんご");
    s.settings
        .set("LiveConversion", serde_json::json!(true))
        .unwrap();
    s.private = true;
    assert!(s.live_reading().is_none());
}

#[test]
fn code_line_toggle_survives_enter_and_resets_on_caret_move() {
    let mut s = Session::default();
    s.code_input = true;
    s.set_context(Some("let value = "), None);
    assert!(s.code_english());
    assert!(!s.key(0, 'a' as i32, 0).consumed);
    assert!(s.set_code_line(true));
    type_text(&mut s, "nihongo");
    assert_eq!(s.commit().commits[0].text, "にほんご");
    s.caret_moved(13, false);
    assert!(!s.code_english());
    s.caret_moved(0x25, false);
    assert!(s.code_english());
    s.set_context(Some("// comment "), None);
    assert!(!s.code_english());
    assert!(!s.set_code_line(true));
    type_text(&mut s, "nihongo");
    assert_eq!(s.commit().commits[0].text, "にほんご");
}

#[test]
fn selection_reconversion_infers_reading_commits_and_cancels_without_learning() {
    let mut s = Session::default();
    let result = s.reconvert_selection("橋", None);
    assert_eq!(result.selection_expect.as_deref(), Some("橋"));
    let view = result.view.unwrap();
    assert_eq!(view.text, "橋");
    let index = view.candidates.iter().position(|c| c == "箸").unwrap();
    assert_eq!(s.select(index).commits[0].text, "箸");
    s.reconvert_selection("橋", Some("はし"));
    let state = s.state.phrases.len();
    let cancel = s.key(0x1b, 0, 0);
    assert_eq!(cancel.commits[0].text, "橋");
    assert!(cancel.view.is_none());
    assert_eq!(s.state.phrases.len(), state);
    assert!(!s.reconvert_selection("未登録の漢字列", None).consumed);
    assert!(!s.reconvert_selection("橋", Some("not a reading")).consumed);
    s.private = true;
    assert!(!s.reconvert_selection("橋", Some("はし")).consumed);
    assert!(s.raw().is_empty());
}

#[test]
fn composition_control_shortcuts_and_shift_space_match_native_key_roles() {
    let mut s = Session::default();
    type_text(&mut s, "kya");
    assert_eq!(s.key(0x49, 0, 2).view.unwrap().text, "キャ");
    assert_eq!(s.key(0x55, 0, 2).view.unwrap().text, "きゃ");
    assert_eq!(s.key(0x4f, 0, 2).view.unwrap().text, "kya");
    assert_eq!(s.key(0x50, 0, 2).view.unwrap().text, "ｋｙａ");
    assert!(s.key(0x48, 0, 2).commits.is_empty());
    assert_eq!(s.raw(), "ky");
    s.key(0, 'a' as i32, 0);
    let result = s.key(32, 32, 1);
    assert!(result.commits.is_empty());
    assert!(result.view.unwrap().converting);
    let count = s.result(false).view.unwrap().candidates.len();
    assert_eq!(s.key(32, 32, 1).view.unwrap().selected_index, count - 1);
    s.commit();
    assert_eq!(s.key(32, 32, 1).commits[0].text, "　");
    assert!(!s.key(32, 32, 0).consumed);
}
#[test]
fn host_committed_preedit_preserves_display_without_learning_unfinished_kana() {
    let mut s = Session::default();
    s.can_delete = true;
    type_text(&mut s, "nihon");
    assert_eq!(s.display(false), "にほn");
    let result = s.commit_preedit();
    assert_eq!(result.commits[0].text, "にほn");
    assert_eq!(result.commits[0].delete_before, 0);
    assert!(result.commits[0].expect.is_none());
    assert!(s.raw().is_empty());
    assert!(s.can_delete);
    // RememberPhrase requires a Japanese result containing kanji. A host's
    // unfinished kana preedit must not become a future phrase prediction.
    assert!(s.state.phrases.is_empty());
    assert!(s.commit_preedit().commits.is_empty());
}

#[test]
fn issue_301_tool_names_remain_english_as_whole_words() {
    for raw in [
        "opencode",
        "openrouter",
        "tokenrouter",
        "OpenRouter",
        "TOKENROUTER",
    ] {
        let mut session = Session::default();
        for ch in raw.chars() {
            session.key(0, ch as i32, 0);
        }
        assert_eq!(session.commit().commits[0].text, raw, "{raw}");
    }
}

#[test]
fn chatgpt_mixed_conversion_keeps_english_and_japanese_clauses() {
    for raw in [
        "chatgptwotsukau",
        "chatgptwotsukatteimasu",
        "googledekensaku",
        "githubwotsukau",
    ] {
        let mut s = Session::default();
        s.settings
            .set("LiveConversion", serde_json::json!(false))
            .unwrap();
        type_text(&mut s, raw);
        let reading = s.conversion_reading();
        let reply = s.key(32, 32, 0);
        assert!(reply.consumed);
        assert!(s.is_converting());
        assert_eq!(s.clause_readings().concat(), reading, "{raw}");
        assert!(!reply.view.unwrap().clauses.is_empty(), "{raw}");
    }
}

#[test]
fn ctrl_enter_confirms_chat_preedit_before_allowing_a_send_shortcut() {
    for converting in [false, true] {
        let mut s = Session::default();
        s.can_delete = false;
        type_text(&mut s, "nihongo");
        if converting {
            s.key(32, 32, 0);
        }
        let shown = s.display(true);
        let reply = s.key(13, 0, 2);
        assert!(
            reply.consumed,
            "the first Ctrl+Enter must not submit the chat"
        );
        assert_eq!(reply.commits.len(), 1);
        assert_eq!(reply.commits[0].text, shown);
        assert!(reply.view.is_none());
        let send = s.key(13, 0, 2);
        assert!(!send.consumed);
        assert!(send.commits.is_empty());
    }
}

#[test]
fn upstream_1048_english_final_d_or_t_stays_before_japanese_ha() {
    for (raw, expected) in [
        ("Androidha", "Androidは"),
        ("androidha", "androidは"),
        ("Redditha", "Redditは"),
        ("redditha", "redditは"),
        ("dha", "でゃ"),
        ("tha", "てゃ"),
    ] {
        let mut s = Session::default();
        type_text(&mut s, raw);
        assert_eq!(s.display(false), expected, "{raw}");
    }
}

#[test]
fn v140_detection_regressions_keep_japanese_boundaries_and_product_names() {
    let mut mismatches = Vec::new();
    for scored in [false, true] {
        for (raw, expected) in [
            ("Androidwa", "Androidわ"),
            ("Androidha", "Androidは"),
            ("kamiwoA4", "かみをA4"),
            ("huusoku6me-toru", "ふうそく6めーとる"),
            ("nottotte", "のっとって"),
            ("imagennzai", "いまげんざい"),
            ("imagenzai", "いまげんざい"),
            ("galaxyS24", "galaxyS24"),
            ("e2e", "e2e"),
            ("exceltoppt", "excelとppt"),
            ("prozac", "prozac"),
            ("usbtunaide", "usbつないで"),
            ("kyouhaGitHubni", "きょうはGitHubに"),
        ] {
            let mut s = Session::default();
            s.settings
                .set("ScoredSegmentation", serde_json::json!(scored))
                .unwrap();
            type_text(&mut s, raw);
            let actual = s.display(true);
            if actual != expected {
                mismatches.push(format!(
                    "{raw}, scored={scored}: {actual:?} != {expected:?}"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn v140_scored_capitalized_prefix_uses_scores_without_splitting_acronyms() {
    for (raw, expected) in [("Goo", "Goお"), ("Googlede", "Googleで"), ("IME", "IME")] {
        let mut session = Session::default();
        session.settings.set("ScoredSegmentation", serde_json::json!(true)).unwrap();
        type_text(&mut session, raw);
        assert_eq!(session.display(false), expected, "{raw}");
    }
}
