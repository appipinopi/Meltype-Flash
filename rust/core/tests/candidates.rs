// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
use chrono::NaiveDate;
use meltype_core::{
    dictionary::Dictionary,
    session::Clause,
    storage::{Change, State, Store},
    time_candidates as time, Session,
};

#[test]
fn upstream_date_and_clock_formats_validate_calendar_and_fullwidth_digits() {
    let now = NaiveDate::from_ymd_opt(2026, 10, 9)
        .unwrap()
        .and_hms_opt(17, 22, 0)
        .unwrap();
    assert_eq!(
        time::candidates("きょう", now),
        [
            "2026年10月9日",
            "2026年10月9日(金)",
            "10月9日",
            "10月9日(金)",
            "2026/10/09",
            "2026/10/09(金)",
            "10/9(金)",
            "2026-10-09",
            "金曜日"
        ]
    );
    assert_eq!(
        time::candidates("いま", now),
        [
            "17:22",
            "17時22分",
            "午後5時22分",
            "2026年10月9日(金) 17時22分",
            "10月9日(金) 17時22分",
            "2026年10月9日(金) 17:22",
            "2026/10/09 17:22"
        ]
    );
    assert_eq!(
        time::typed("１０がつ９にち", 2026).unwrap(),
        ["10/9", "10/09", "10/9(金)", "2026/10/09"]
    );
    assert_eq!(time::typed("３じ５ぷん", 2026).unwrap(), ["3:05", "03:05"]);
    assert!(time::typed("2がつ29にち", 2026).unwrap().is_empty());
    assert!(!time::typed("2がつ29にち", 2024).unwrap().is_empty());
    for raw in ["13がつ1にち", "2がつ30にち", "25じ0ふん", "23じ60ふん"] {
        assert!(time::typed(raw, 2026).unwrap().is_empty(), "{raw}");
    }
    for raw in ["100がつ1にち", "3じ15", "abcじ5ふん"] {
        assert!(time::typed(raw, 2026).is_none(), "{raw}");
    }
}

#[test]
fn date_segments_join_and_dynamic_choices_do_not_become_stale_learning() {
    let mut s = Session::default();
    for c in "10gatsu9nichi".chars() {
        s.key(0, c as i32, 0);
    }
    assert!(s.begin_clauses(vec![
        Clause::new("10がつ".into(), vec!["10月".into()]),
        Clause::new("9にち".into(), vec!["9日".into()])
    ]));
    let view = s.result(false).view.unwrap();
    assert_eq!(view.clauses, ["10月9日"]);
    let index = view.candidates.iter().position(|s| s == "10/9").unwrap();
    assert_eq!(s.select(index).commits[0].text, "10/9");
    assert!(s.state.phrases.is_empty());
    assert!(!s.preferences.selected.contains_key("10がつ9にち"));
    let mut state = State::default();
    state.apply(Change::Phrase {
        reading: "きょう".into(),
        text: "2026年10月9日".into(),
    });
    state.apply(Change::Conversion {
        reading: "いま".into(),
        text: "17:22".into(),
        translation: false,
    });
    assert!(state.phrases.is_empty());
    assert!(state.preferences.selected.is_empty());
}

#[test]
fn emoji_and_ascii_supplementary_words_follow_normal_kana_candidates() {
    let d = Dictionary::bundled();
    assert_eq!(d.convert("すし"), "すし");
    assert_eq!(d.convert("ぺん"), "ぺん");
    let candidates = d.candidates("すし");
    let emoji = d.emoji("すし");
    assert!(!emoji.is_empty());
    assert_eq!(&candidates[candidates.len() - emoji.len()..], emoji);
    assert!(!emoji.contains(&candidates[0]));
    let candidates = d.candidates("ぺん");
    assert!(
        candidates.iter().position(|s| s == "ぺん").unwrap()
            < candidates.iter().position(|s| s == "PEN").unwrap()
    );
}

#[test]
fn language_memory_requires_two_english_choices_for_common_japanese_and_survives_reload() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    let language = |raw: &str, english, explicit| Change::Language {
        raw: raw.into(),
        english,
        explicit,
    };
    let state = store.record(vec![language("kyouha", true, true)]).unwrap();
    assert!(!state.preferences.english.contains_key("kyouha"));
    assert!(!store.load().unwrap().language_counts["kyouha"].active("kyouha"));
    let state = store.record(vec![language("kyouha", true, true)]).unwrap();
    assert_eq!(state.preferences.english.get("kyouha"), Some(&true));
    let state = store
        .record(vec![
            language("kyouha", false, true),
            language("api", true, false),
            language("go", true, false),
        ])
        .unwrap();
    assert_eq!(state.preferences.english.get("kyouha"), Some(&false));
    assert_eq!(state.preferences.english.get("api"), Some(&true));
    assert!(!state.preferences.english.contains_key("go"));
    let state = store.record(vec![language("go", true, false)]).unwrap();
    assert_eq!(state.preferences.english.get("go"), Some(&true));
    assert_eq!(
        store.load().unwrap().preferences.english,
        state.preferences.english
    );
}

#[test]
fn arithmetic_candidates_require_numbers_preserve_width_and_never_learn() {
    use meltype_core::math_candidates as math;
    assert_eq!(math::candidate("3かける4", ""), Some(("3×4".into(), true)));
    assert_eq!(math::candidate("わる", "12"), Some(("÷".into(), true)));
    assert_eq!(
        math::candidate("たす１", "２"),
        Some(("＋１".into(), false))
    );
    assert_eq!(math::candidate("1ひく1", ""), Some(("1-1".into(), false)));
    assert_eq!(math::candidate("かける", "電話を"), None);
    assert_eq!(math::candidate("たす", ""), None);
    assert_eq!(math::candidate("3かけるもの", ""), None);
    let mut s = Session::default();
    for c in "3kakeru4".chars() {
        s.key(0, c as i32, 0);
    }
    let view = s.key(32, 32, 0).view.unwrap();
    assert_eq!(view.candidates[0], "3×4");
    assert_eq!(s.commit().commits[0].text, "3×4");
    assert!(s.state.phrases.is_empty());
    assert!(s.preferences.selected.is_empty());
    let mut s = Session::default();
    s.set_context(Some("２"), None);
    for c in "tasu".chars() {
        s.key(0, c as i32, 0);
    }
    let view = s.key(32, 32, 0).view.unwrap();
    assert_eq!(view.candidates[1], "＋");
    let mut state = State::default();
    state.apply(Change::Conversion {
        reading: "かける".into(),
        text: "×".into(),
        translation: false,
    });
    assert!(state.preferences.selected.is_empty());
}

#[test]
fn japanese_counters_product_context_and_punctuation_keep_language_boundaries() {
    let d = Dictionary::bundled();
    assert_eq!(d.mixed("10ji", true), "10じ");
    assert_eq!(d.mixed("kaigiha3jikaradesu", true), "かいぎは3じからです");
    assert_eq!(d.mixed("RTX4090", true), "RTX4090");
    assert!(!d.english_context("Google "));
    assert!(d.english_context("I use Google "));
    assert!(!d.english_context("日本語 Google "));
    assert_eq!(d.mixed("sakewonomu", true), "さけをのむ");
    assert_eq!(
        d.mixed("ashitanomeetingwotsuika", true),
        "あしたのmeetingをついか"
    );
    let mut s = Session::default();
    for c in "nihongo!?".chars() {
        s.key(0, c as i32, 0);
    }
    assert_eq!(s.commit().commits[0].text, "にほんご！？");
    let mut s = Session::default();
    for c in "hello!?".chars() {
        s.key(0, c as i32, 0);
    }
    assert_eq!(s.commit().commits[0].text, "hello!?");
}
