// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 公開Issueの例。旧版の誤動作をgoldenとして固定せず、報告された期待を検査する。
use meltype_core::Session;
use serde_json::json;

fn type_and_commit(raw: &str, scored: bool) -> String {
    let mut session = Session::default();
    session
        .settings
        .set("ScoredSegmentation", json!(scored))
        .unwrap();
    let mut text = String::new();
    for c in raw.chars() {
        let result = session.key(if c == ' ' { 32 } else { 0 }, c as i32, 0);
        for commit in result.commits {
            text.push_str(&commit.text);
        }
        if !result.consumed {
            text.push(c);
        }
    }
    for commit in session.commit().commits {
        text.push_str(&commit.text);
    }
    text
}

#[test]
fn reported_english_words_and_commands_preserve_spelling() {
    let mut failures = Vec::new();
    for scored in [false, true] {
        for (issue, raw) in [
            (1125, "dive into summer"),
            (1123, "ost"),
            (1122, "midi"),
            (1121, "vocaloid"),
            (1117, "nohup"),
            (918, "pure"),
            (915, "printenv"),
            (1003, "meta"),
            (1001, "claudecode"),
            (876, "doomscroll"),
            (852, "image"),
            (347, "deepseek"),
            (1013, "yachi8000"),
        ] {
            let actual = type_and_commit(raw, scored);
            if actual != raw {
                failures.push(format!("#{issue}, scored={scored}: {raw} -> {actual}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn standalone_english_does_not_capture_following_japanese_syllables() {
    let dictionary = meltype_core::dictionary::Dictionary::bundled();
    for level in ["Balanced", "Conservative", "Manual"] {
        assert_eq!(
            dictionary.composition("puretuma", true, "", "", level),
            "ぷれつま"
        );
    }
    assert_eq!(
        dictionary.composition("pure", true, "", "", "Manual"),
        "ぷれ"
    );
    assert_eq!(
        dictionary.composition("metaru", true, "", "", "Balanced"),
        "めたる"
    );
}

#[test]
fn issue_914_english_final_n_is_not_consumed_by_japanese_ni() {
    let d = meltype_core::dictionary::Dictionary::bundled();
    assert_eq!(
        d.composition("mainnipushsuru", true, "", "", "Balanced"),
        "mainにpushする",
        "dictionary"
    );
    assert_eq!(
        type_and_commit("mainnipushsuru", false),
        "mainにpushする",
        "session"
    );
}

#[test]
fn reported_loanword_and_document_readings_stay_japanese() {
    assert_eq!(type_and_commit("pa-thisyon", false), "ぱーてぃしょん");
    assert_eq!(
        type_and_commit("sonodokyumentowoA4saizudeinsatushitekudasai", false),
        "そのどきゅめんとをA4さいずでいんさつしてください"
    );
}

#[test]
fn requested_dictionary_candidates_are_available_without_fixing_one_homophone() {
    let d = meltype_core::dictionary::Dictionary::bundled();
    for (reading, expected) in [
        ("ご", "語"),
        ("ご", "後"),
        ("ぶるあか", "ブルーアーカイブ"),
        ("ぶるあか", "ブルアカ"),
        ("いんでぃあな", "インディアナ"),
        ("かれんだー", "カレンダー"),
        ("ふうそく", "風速"),
        ("めーとる", "メートル"),
        ("あいちけんりつ", "愛知県立"),
        ("にがっきちゅうかん", "二学期中間"),
        ("ぶいあーるちゃっと", "VRChat"),
        ("いしゅー", "issues"),
    ] {
        assert!(
            d.candidates(reading).iter().any(|word| word == expected),
            "{reading}: {expected}"
        );
    }
}

#[cfg(feature = "filesystem")]
#[test]
fn issue_1036_one_character_reading_round_trips_registration_export_and_import() {
    use meltype_core::{control, storage::Store, user_dictionary};
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path());
    control::execute(
        &store,
        &json!({"op":"dictionary_set", "reading":"あ", "word":"亜"}),
    )
    .unwrap();
    let path = directory.path().join("export.txt");
    control::execute(&store, &json!({"op":"dictionary_export", "path":path})).unwrap();
    let parsed = user_dictionary::parse(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(parsed.words, [("あ".into(), "亜".into())]);
    control::execute(&store, &json!({"op":"dictionary_remove", "reading":"あ"})).unwrap();
    let result = control::execute(&store, &json!({"op":"dictionary_import", "path":path})).unwrap();
    assert_eq!(result["imported"], 1);
    let session = Session::open(Store::new(directory.path())).unwrap();
    assert_eq!(session.preferences.user_dictionary["あ"], ["亜"]);
    assert!(!user_dictionary::valid("", "亜"));
    assert!(!user_dictionary::valid("あ", ""));
    assert!(!user_dictionary::valid("あ", "亜\n悪い行"));
}

#[test]
fn dotfiles_and_leading_decimal_keep_their_dot() {
    for scored in [false, true] {
        for raw in [".env", ".gitignore", ".env.local", ".2", ".25"] {
            assert_eq!(type_and_commit(raw, scored), raw, "{raw}, scored={scored}");
        }
        assert_eq!(type_and_commit("nihongo.", scored), "にほんご。");
    }
}

#[test]
fn issue_960_foreign_kw_syllables_do_not_split_before_wo() {
    for (raw, expected) in [
        ("kwa", "くぁ"),
        ("kwi", "くぃ"),
        ("kwe", "くぇ"),
        ("kwo", "くぉ"),
    ] {
        assert_eq!(meltype_core::romaji::hiragana(raw, true), expected);
        assert_eq!(type_and_commit(raw, false), expected);
    }
}

#[test]
fn trailing_capital_keeps_an_established_japanese_reading_while_typing() {
    let mut session = Session::default();
    for c in "tanni".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.display(false), "たんい");
    session.key(0x47, 'G' as i32, 1);
    assert_eq!(session.display(false), "たんいG");
    for c in "ithubde".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.commit().commits[0].text, "たんいGithubで");
}
