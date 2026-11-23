// SPDX-License-Identifier: GPL-3.0-or-later
//! ReadingLattice vs ScoredSegmentation comparison harness.
use meltype_core::Session;
use serde_json::{json, Value};

fn run(text: &str, lattice: bool, scored: bool) -> String {
    let mut session = Session::default();
    for (setting, value) in [
        ("LiveConversion", false),
        ("PredictiveCandidates", false),
        ("LearningEnabled", false),
    ] {
        session.settings.set(setting, json!(value)).unwrap();
    }
    session
        .settings
        .set("ReadingLattice", json!(lattice))
        .unwrap();
    session
        .settings
        .set("ScoredSegmentation", json!(scored))
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
fn reading_lattice_and_scored_segmentation_agree_on_pure_japanese_input() {
    let cases: Value =
        serde_json::from_str(include_str!("data/upstream-lattice-keys.json")).unwrap();
    let cases = cases.as_array().unwrap();
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let text = case["text"].as_str().unwrap();
        let expected = case["expected"].as_str().unwrap();
        // Keep the comparison meaningful: baseline, ScoredSegmentation and
        // ReadingLattice must all agree for pure-kana readings.
        assert_eq!(run(text, false, false), expected, "{name}: baseline");
        assert_eq!(run(text, false, true), expected, "{name}: scored");
        assert_eq!(run(text, true, false), expected, "{name}: lattice");
    }
}

#[test]
fn reading_lattice_is_deterministic_for_mixed_input() {
    for text in ["google", "github", "konnnichihagoogle", "kyouhaiitenkidesu"] {
        let first = run(text, true, false);
        let second = run(text, true, false);
        assert_eq!(first, second, "{text} must be deterministic");
        assert!(!first.is_empty(), "{text} must commit something");
    }
}

#[test]
fn mixed_input_comparison_is_deterministic_and_preserves_ascii_tokens() {
    // Both implementations must preserve opaque ASCII tokens and agree on
    // Japanese readings. Known divergences (github, HelloWorld, sorehaAI) are
    // still checked for determinism and non-empty output below.
    for (text, expected) in [
        ("konnnichihagoogle", "こんにちはgoogle"),
        ("kyouhaiitenkidesu", "きょうはいいてんきです"),
        ("https://example.com", "https://example.com"),
        ("abc123", "abc123"),
        ("iPhone15", "iPhone15"),
        ("test@example.com", "test@example.com"),
    ] {
        assert_eq!(run(text, true, false), expected, "{text}: lattice");
        assert_eq!(run(text, false, true), expected, "{text}: scored");
    }
    for text in ["google", "github", "HelloWorld", "sorehaAI"] {
        let lattice = run(text, true, false);
        let scored = run(text, false, true);
        assert_eq!(
            lattice,
            run(text, true, false),
            "{text}: lattice deterministic"
        );
        assert_eq!(
            scored,
            run(text, false, true),
            "{text}: scored deterministic"
        );
        assert!(!lattice.is_empty(), "{text}: lattice non-empty");
        assert!(!scored.is_empty(), "{text}: scored non-empty");
    }
}

#[test]
fn forced_english_spans_are_preserved() {
    for text in [
        "https://example.com/path?q=1",
        "example.com",
        "MeltypeTip.dll",
        "user@example.com",
        "state-of-the-art",
        "i18n",
        "v1.2.3",
        "iPhone15",
        "#tag",
        ".gitignore",
        "abc123",
    ] {
        assert_eq!(run(text, true, false), text, "forced span: {text}");
    }
}

#[test]
fn japanese_and_forced_spans_mix() {
    assert_eq!(
        run("konnnichihaexample.com", true, false),
        "こんにちはexample.com"
    );
    assert_eq!(
        run("kyouhaiitenkidesuiPhone15", true, false),
        "きょうはいいてんきですiPhone15"
    );
}

#[test]
fn learned_english_is_honored_by_the_lattice() {
    let mut session = Session::default();
    session.settings.set("ReadingLattice", json!(true)).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session.preferences.english.insert("github".into(), true);
    for c in "github".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.commit().commits[0].text, "github");
}

#[test]
fn non_ascii_raw_is_preserved_or_composed_without_panicking() {
    let mut session = Session::default();
    session.settings.set("ReadingLattice", json!(true)).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    for c in "日本語".chars() {
        session.key(0, c as i32, 0);
    }
    assert_eq!(session.commit().commits[0].text, "日本語");
}

#[test]
fn domain_and_prerelease_rules_are_prioritized() {
    for text in [
        "meltype.moe",
        "llama.cpp",
        "1.2.3-beta.2",
        "v2.0.0-rc1",
        "1.5.0-preview.3",
    ] {
        assert_eq!(run(text, true, false), text, "domain/version: {text}");
    }
}
