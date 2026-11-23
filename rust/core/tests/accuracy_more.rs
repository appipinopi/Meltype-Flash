// SPDX-License-Identifier: GPL-3.0-or-later
//! Additional accuracy corpus: contractions, conjugation, numbers, paths.
use meltype_core::Session;
use serde_json::json;

fn run(text: &str) -> String {
    let mut session = Session::default();
    session.settings.set("ReadingLattice", json!(true)).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session
        .settings
        .set("PredictiveCandidates", json!(false))
        .unwrap();
    session
        .settings
        .set("LearningEnabled", json!(false))
        .unwrap();
    session
        .settings
        .set("SpaceAroundEnglish", json!(false))
        .unwrap();
    for c in text.chars() {
        session.key(0, c as i32, 0);
    }
    session
        .commit()
        .commits
        .iter()
        .map(|edit| edit.text.as_str())
        .collect::<String>()
}

#[test]
fn contracts_and_conjugations_match_expected() {
    for (text, expected) in [
        ("don't", "don't"),
        ("it's", "it's"),
        ("I'm", "I'm"),
        ("you're", "you're"),
        ("we'll", "we'll"),
        ("I've", "I've"),
        ("can't", "can't"),
        ("won't", "won't"),
        ("rockin'", "rockin'"),
        ("tabete", "たべて"),
        ("tabemashita", "たべました"),
        ("ikanai", "いかない"),
        ("ikimashita", "いきました"),
        ("kakimasu", "かきます"),
        ("kakimashita", "かきました"),
        ("yomimasu", "よみます"),
        ("yomimashita", "よみました"),
        ("shiteimasu", "しています"),
        ("mattemasu", "まってます"),
        ("matte", "まって"),
        ("mattara", "まったら"),
        ("irekata", "いれかた"),
        ("tsukaikata", "つかいかた"),
        ("yarikata", "やりかた"),
        ("hanashikata", "はなしかた"),
        ("kangaekata", "かんがえかた"),
        ("kakikata", "かきかた"),
        ("yomikata", "よみかた"),
        ("okonaikata", "おこないかた"),
        ("tsukurikata", "つくりかた"),
        ("mainnipushsuru", "mainにpushする"),
        ("pa-thisyon", "ぱーてぃしょん"),
        (
            "sonodokyumentowoA4saizudeinsatushitekudasai",
            "そのどきゅめんとをA4さいずでいんさつしてください",
        ),
        ("tanniGithubde", "たんいGithubで"),
        ("kwa", "くぁ"),
    ] {
        assert_eq!(run(text), expected, "input: {text}");
    }
}

#[test]
fn numbers_and_counters_convert() {
    for (text, expected) in [
        ("3gatsu", "3月"),
        ("4gatsu", "4月"),
        ("2026nen", "2026年"),
        ("1nin", "1人"),
        ("2nin", "2人"),
        ("10%", "10％"),
        ("1000en", "1000円"),
        ("3tsu", "3つ"),
        ("2ko", "2個"),
        ("10ji", "10時"),
        ("5fun", "5分"),
        ("2kai", "2回"),
        ("3nichi", "3日"),
        ("5cm", "5cm"),
        ("100MB", "100MB"),
        ("1.5kg", "1.5kg"),
        ("3.14", "3.14"),
        ("1,000en", "1,000円"),
    ] {
        assert_eq!(run(text), expected, "input: {text}");
    }
}

#[test]
fn urls_emails_paths_and_files_are_preserved() {
    for text in [
        "taro.yamada@example.co.jp",
        "user+tag@example.com",
        "https://github.com/yksr-melt/Meltype",
        "http://example.com/path?q=1#section",
        "/usr/local/bin/meltype",
        r"C:\Users\test\file.txt",
        "README.md",
        "package.json",
        "Cargo.toml",
        "main.rs",
        "styles.css",
        "index.html",
        "file_name_v2.tar.gz",
        ".env.local",
        "sub.example.co.jp",
    ] {
        assert_eq!(run(text), text, "input: {text}");
    }
}

#[test]
fn learned_and_user_dictionary_weights() {
    let mut session = Session::default();
    session.settings.set("ReadingLattice", json!(true)).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session
        .preferences
        .user_dictionary
        .insert("かんじ".into(), vec!["漢字".into()]);
    for c in "kanji".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.commit().commits[0].text, "漢字");

    let mut session = Session::default();
    session.settings.set("ReadingLattice", json!(true)).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session.preferences.english.insert("github".into(), false);
    for c in "github".chars() {
        session.key(0, c as i32, 0);
    }
    assert_ne!(session.commit().commits[0].text, "github");
}
