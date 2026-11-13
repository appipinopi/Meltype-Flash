// SPDX-License-Identifier: GPL-3.0-or-later
//! Practical accuracy corpus for ReadingLattice (Japanese, English, mixed).
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
fn accuracy_corpus_matches_expected() {
    for (text, expected) in [
        // Pure Japanese
        ("konnnichiha", "こんにちは"),
        ("arigatou", "ありがとう"),
        ("ohayou", "おはよう"),
        ("gakkou", "がっこう"),
        ("toukyou", "とうきょう"),
        ("pasokon", "ぱそこん"),
        ("shigoto", "しごと"),
        ("soudesu", "そうです"),
        ("wakarimashita", "わかりました"),
        ("onegaishimasu", "おねがいします"),
        // English words / acronyms
        ("github", "github"),
        ("GitHub", "GitHub"),
        ("google", "google"),
        ("Google", "Google"),
        ("AI", "AI"),
        ("ChatGPT", "ChatGPT"),
        ("OpenAI", "OpenAI"),
        ("SNS", "SNS"),
        ("URL", "URL"),
        ("API", "API"),
        ("PDF", "PDF"),
        ("JavaScript", "JavaScript"),
        ("Python", "Python"),
        ("Docker", "Docker"),
        ("iPhone15", "iPhone15"),
        ("YouTube", "YouTube"),
        ("Wi-Fi", "Wi-Fi"),
        ("email", "email"),
        ("LINE", "LINE"),
        ("IT", "IT"),
        ("DX", "DX"),
        ("UI", "UI"),
        ("UX", "UX"),
        ("ML", "ML"),
        ("VR", "VR"),
        ("AR", "AR"),
        ("IoT", "IoT"),
        ("RPA", "RPA"),
        ("SEO", "SEO"),
        ("CTO", "CTO"),
        ("CEO", "CEO"),
        ("GDP", "GDP"),
        ("SDGs", "SDGs"),
        ("M1", "M1"),
        ("M2", "M2"),
        ("5G", "5G"),
        // Mixed sentences
        ("sorehaAI", "それはAI"),
        ("ashitahaAIgahairimasu", "あしたはAIがはいります"),
        (
            "watashihagithubwotsukatteimasu",
            "わたしはgithubをつかっています",
        ),
        (
            "kaigihadockerdeokonaimashita",
            "かいぎはdockerでおこないました",
        ),
        ("PDFwookurimasu", "PDFをおくります"),
        ("URLwoshirasetekudasai", "URLをしらせてください"),
        // Domains, files, versions, tokens
        ("example.com", "example.com"),
        ("meltype.moe", "meltype.moe"),
        ("llama.cpp", "llama.cpp"),
        ("MeltypeTip.dll", "MeltypeTip.dll"),
        ("user@example.com", "user@example.com"),
        ("state-of-the-art", "state-of-the-art"),
        ("i18n", "i18n"),
        ("v1.2.3", "v1.2.3"),
        ("1.5.0-preview.3", "1.5.0-preview.3"),
        ("abc123", "abc123"),
        ("konnnichihaexample.com", "こんにちはexample.com"),
        (
            "kyouhaiitenkidesuiPhone15",
            "きょうはいいてんきですiPhone15",
        ),
    ] {
        assert_eq!(run(text), expected, "input: {text}");
    }
}
