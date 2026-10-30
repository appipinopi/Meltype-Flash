// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use meltype_core::{
    context::{self, LineKind},
    control,
    linguistics::Linguistics,
    protection,
    settings::Settings,
    storage::{Change, Store},
    user_dictionary, Session,
};
use serde_json::json;
fn type_text(s: &mut Session, text: &str) {
    for c in text.chars() {
        s.key(0, c as i32, 0);
    }
}

#[test]
fn v140_candidate_pages_wrap_at_the_first_candidate_of_each_page() {
    let mut s = Session::default();
    type_text(&mut s, "hashi");
    let candidates: Vec<_> = (0..20).map(|i| format!("候補{i}")).collect();
    s.begin_clauses(vec![meltype_core::session::Clause::new(
        "はし".into(),
        candidates,
    )]);
    let pages = s.result(false).view.unwrap().candidates.len().div_ceil(9);
    for (key, expected) in (1..=pages)
        .map(|page| (0x22, page % pages * 9))
        .chain([(0x21, (pages - 1) * 9)])
    {
        let result = s.key(key, 0, 0);
        assert!(result.consumed);
        assert!(result.commits.is_empty());
        assert_eq!(result.view.unwrap().selected_index, expected);
    }
}

#[test]
fn v140_number_selection_can_be_disabled_without_changing_numpad_selection() {
    for (enabled, key) in [(true, 0x31), (false, 0x31), (false, 0x61)] {
        let mut s = Session::default();
        s.settings
            .set("NumberKeysSelectCandidates", json!(enabled))
            .unwrap();
        type_text(&mut s, "hashi");
        s.begin_clauses(vec![meltype_core::session::Clause::new(
            "はし".into(),
            vec!["橋".into(), "箸".into()],
        )]);
        let result = s.key(key, '1' as i32, 0);
        assert_eq!(result.commits[0].text, "橋");
        if !enabled && key == 0x31 {
            assert_eq!(result.view.unwrap().text, "1");
        } else {
            assert!(result.view.is_none());
        }
    }
}

#[test]
fn v140_numbers_and_english_have_independent_spacing_and_context() {
    for (english, numbers, expected) in [
        (false, false, "日本語3回iPhone15台"),
        (true, false, "日本語3回 iPhone15 台"),
        (false, true, "日本語 3 回iPhone15台"),
        (true, true, "日本語 3 回 iPhone15 台"),
    ] {
        let mut s = Session::default();
        s.settings
            .set("SpaceAroundEnglish", json!(english))
            .unwrap();
        s.settings
            .set("SpaceAroundNumbers", json!(numbers))
            .unwrap();
        type_text(&mut s, "日本語3回iPhone15台");
        assert_eq!(s.commit().commits[0].text, expected);
    }
    for (before, after, expected) in [
        ("3", "4", " 回 "),
        ("iPhone15", "iPhone16", "回"),
        ("3 ", " 4", "回"),
    ] {
        let mut s = Session::default();
        s.settings.set("SpaceAroundNumbers", json!(true)).unwrap();
        s.set_context(Some(before), Some(after));
        type_text(&mut s, "回");
        assert_eq!(s.commit().commits[0].text, expected);
    }
}

#[test]
fn v140_z_symbols_can_be_disabled_in_regular_and_edited_input() {
    for enabled in [false, true] {
        for edited in [false, true] {
            let mut s = Session::default();
            s.settings.set("ZSymbols", json!(enabled)).unwrap();
            if edited {
                type_text(&mut s, "ka");
                s.key(8, 0, 0);
            }
            type_text(&mut s, "zj");
            assert_eq!(s.display(false).contains('↓'), enabled);
        }
    }
}

#[test]
fn v140_bundled_phrase_parts_preserve_boundaries_and_ignore_unrelated_history() {
    let mut s = Session::default();
    s.preferences.selected.insert("いち".into(), "市".into());
    type_text(&mut s, "ichigakki");
    let result = s.key(32, 32, 0);
    assert_eq!(result.view.unwrap().clauses, ["一", "学期"]);
    assert_eq!(s.clause_readings(), ["いち", "がっき"]);
    let mut s = Session::default();
    type_text(&mut s, "ichigakki");
    let plan = s.registered_conversion_plan("いちがっき", &[]).unwrap();
    assert_eq!(
        plan,
        [
            ("いち".into(), Some("一".into())),
            ("がっき".into(), Some("学期".into()))
        ]
    );
    let mut s = Session::default();
    type_text(&mut s, "hakkenritsu");
    assert!(s
        .registered_conversion_plan(
            "はっけんりつ",
            &[meltype_core::session::Clause::new(
                "はっけんりつ".into(),
                vec!["発見率".into()]
            )]
        )
        .is_none());
}

#[test]
fn v140_head_only_phrases_do_not_match_inside_an_unknown_reading() {
    let dictionary = meltype_core::dictionary::Dictionary::bundled();
    assert_eq!(dictionary.convert("けんりつ"), "県立");
    assert!(!dictionary.convert("はっけんりつ").contains("県立"));
    let session = Session::default();
    assert!(session
        .registered_conversion_plan("はっけんりつ", &[])
        .is_none());
    let engine = vec![
        meltype_core::session::Clause::new("とうきょう".into(), vec!["東京".into()]),
        meltype_core::session::Clause::new("けんりつ".into(), vec!["県率".into()]),
    ];
    assert_eq!(
        session.registered_conversion_plan("とうきょうけんりつ", &engine),
        Some(vec![
            ("とうきょう".into(), None),
            ("けんりつ".into(), Some("県立".into()))
        ])
    );
}

#[test]
fn v140_phrase_boundaries_respect_user_words_with_the_same_reading() {
    for (word, expected) in [("一学期", vec!["一", "学期"]), ("一楽器", vec!["一楽器"])]
    {
        let mut session = Session::default();
        session
            .preferences
            .user_dictionary
            .insert("いちがっき".into(), vec![word.into()]);
        type_text(&mut session, "ichigakki");
        let reply = session.key(32, 32, 0);
        assert_eq!(reply.view.unwrap().clauses, expected);
    }
    let mut session = Session::default();
    session
        .preferences
        .selected
        .insert("いち".into(), "市".into());
    type_text(&mut session, "ichigakki");
    assert!(session.update_live_conversion(vec![
        meltype_core::session::Clause::new("いち".into(), vec!["一".into(), "市".into()]),
        meltype_core::session::Clause::new("がっき".into(), vec!["学期".into()]),
    ]));
    assert_eq!(session.display(false), "一学期");
    let mut session = Session::default();
    session
        .preferences
        .selected
        .insert("に".into(), "荷".into());
    session
        .preferences
        .selected
        .insert("がっき".into(), "楽器".into());
    type_text(&mut session, "nigakki");
    assert!(session.begin_clauses(vec![
        meltype_core::session::Clause::new("に".into(), vec!["二".into()]),
        meltype_core::session::Clause::new("がっき".into(), vec!["学期".into()]),
    ]));
    assert_eq!(session.result(false).view.unwrap().clauses, ["二", "学期"]);
}

#[test]
fn v140_clause_function_keys_preserve_other_clauses_when_raw_is_unavailable() {
    let mut s = Session::default();
    s.settings
        .set("FunctionKeysAffectSelectedClause", json!(true))
        .unwrap();
    type_text(&mut s, "nihongonohon");
    let mut clause = meltype_core::session::Clause::new("にほんご".into(), vec!["日本語".into()]);
    clause.raw = None;
    s.begin_clauses(vec![
        clause,
        meltype_core::session::Clause::new("のほん".into(), vec!["の本".into()]),
    ]);
    // The boundary edit removes the retained original spelling.
    s.key(0x27, 0, 1);
    let before = s.result(false).view.unwrap().clauses;
    let result = s.key(0x79, 0, 0);
    assert_eq!(result.view.unwrap().clauses, before);
    assert!(s.is_converting());
}
#[test]
fn settings_profiles_preserve_shared_and_future_values() {
    let mut s = Settings::parse(
        r#"{// comment
 "LiveConversion":false,"FutureSetting":{"a":1},"JapaneseThreshold":999,"SettingsVersion":5,}"#,
    )
    .unwrap();
    assert_eq!(s.integer("JapaneseThreshold"), 20);
    assert_eq!(s.0["FutureSetting"]["a"], 1);
    s.add_profile("仕事").unwrap();
    s.switch_profile("仕事").unwrap();
    s.set("SpaceAroundEnglish", json!(true)).unwrap();
    s.set("AutoUpdate", json!(false)).unwrap();
    s.switch_profile("標準").unwrap();
    assert!(!s.boolean("SpaceAroundEnglish"));
    assert!(!s.boolean("AutoUpdate"));
    s.switch_profile("仕事").unwrap();
    assert!(s.boolean("SpaceAroundEnglish"));
    assert!(s.delete_profile("仕事").is_err());
}
#[test]
fn app_rules_last_override_and_custom_kind() {
    let s=Settings::parse(r#"{"SettingsVersion":5,"AppRules":[{"Process":"Code.exe","Enabled":true},{"Process":"code.exe","Enabled":false}],"AppKinds":[]}"#).unwrap();
    assert!(!s.for_app("CODE.EXE").boolean("Enabled"));
    assert!(s.for_app("CODE").boolean("Enabled"));
    let s=Settings::parse(r#"{"SettingsVersion":5,"AppRules":[{"Process":"chat","Kind":"会話"}],"AppKinds":[{"Name":"会話","Base":"General","DetectionLevel":"Conservative","LiveConversion":false}]}"#).unwrap();
    assert_eq!(s.for_app("chat").text("DetectionLevel"), "Conservative");
    assert!(!s.for_app("chat").boolean("LiveConversion"));
}
#[test]
fn durable_learning_and_concurrent_updates() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path());
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let store = store.clone();
            std::thread::spawn(move || {
                store
                    .record(vec![Change::Conversion {
                        reading: format!("よみ{i}"),
                        text: format!("語{i}"),
                        translation: false,
                    }])
                    .unwrap()
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(store.load().unwrap().preferences.selected.len(), 8);
    let mut s = Session::open(store.clone()).unwrap();
    type_text(&mut s, "hashi");
    s.key(32, 32, 0);
    let index = s
        .result(false)
        .view
        .unwrap()
        .candidates
        .iter()
        .position(|c| c == "箸")
        .unwrap();
    s.select(index);
    let mut s = Session::open(store).unwrap();
    type_text(&mut s, "hashi");
    assert_eq!(s.key(32, 32, 0).view.unwrap().text, "箸");
}
#[test]
fn malformed_data_is_reported_and_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    std::fs::write(&path, b"broken").unwrap();
    let store = Store::new(dir.path());
    assert!(store
        .update(|s| {
            s.clear_learning();
            Ok(())
        })
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
}
#[test]
fn backup_round_trip_rejects_escapes_and_invalid_entries_before_writes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    control::execute(
        &store,
        &json!({"op":"snippet_set","name":"sig","text":"署名"}),
    )
    .unwrap();
    let bytes = store.backup().unwrap();
    store
        .update(|s| {
            s.snippets.clear();
            Ok(())
        })
        .unwrap();
    // state.json plus the seven upstream data files used by C#.
    assert_eq!(store.restore(&bytes).unwrap(), 8);
    assert_eq!(store.load().unwrap().snippets["sig"], "署名");
    let bad=serde_json::to_vec(&json!({"format":"meltype-backup","version":1,"files":{"config.json":"e30=","state.json":"!!!","../escape":"e30="}})).unwrap();
    assert!(store.restore(&bad).is_err());
    assert!(store.preview_backup(&bad).is_err());
    assert!(!dir.path().join("config.json").exists());
    for name in [
        "../escape",
        "dictionaries/../x.txt",
        "dictionaries/CON.txt",
        "dictionaries/a:b.txt",
        "C:/x",
        "dictionaries/x/y.txt",
    ] {
        assert!(!meltype_core::storage::restorable_name(name), "{name}");
    }
}
#[test]
fn backup_preview_describes_valid_targets_without_data_or_writes() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    std::fs::write(dir.path().join("config.json"), b"original settings").unwrap();
    let mut files = serde_json::Map::new();
    for (name, text) in [
        ("config.json", "{}"),
        ("state.json", "private learned content"),
        ("snippets.json", "private snippet"),
        ("userdict.txt", "private dictionary"),
        ("dictionaries/words.txt", "custom dictionary"),
    ] {
        files.insert(name.into(), json!(STANDARD.encode(text)));
    }
    files.insert("../outside".into(), json!("ignored unsupported bytes"));
    let mut backup = json!({"format":"meltype-backup","version":1,"created":"2026-10-17T12:00:00Z","files":files});
    let preview = store
        .preview_backup(&serde_json::to_vec(&backup).unwrap())
        .unwrap();
    assert_eq!(preview["fileCount"], 5);
    assert_eq!(preview["ignoredFiles"], 1);
    assert_eq!(
        preview["categories"],
        json!(["設定", "学習データ", "ユーザー辞書", "定型文"])
    );
    assert_eq!(preview["created"], "2026-10-17T12:00:00+00:00");
    assert!(preview["bytes"].as_u64().unwrap() > 0);
    assert!(preview.get("files").is_none());
    assert!(!preview.to_string().contains("private"));
    assert_eq!(
        std::fs::read(dir.path().join("config.json")).unwrap(),
        b"original settings"
    );
    assert!(!dir.path().join("state.json").exists());
    assert!(!dir.path().join("config.json.before-restore").exists());
    let combined = json!({"format":"meltype-backup","version":1,"files":{"state.json":STANDARD.encode("combined user data")}});
    let combined = store
        .preview_backup(&serde_json::to_vec(&combined).unwrap())
        .unwrap();
    assert_eq!(
        combined["categories"],
        json!(["学習データ", "ユーザー辞書", "定型文"])
    );
    backup["created"] = json!("invalid date with arbitrary content");
    assert!(store
        .preview_backup(&serde_json::to_vec(&backup).unwrap())
        .unwrap()["created"]
        .is_null());
    backup["files"]["state.json"] = json!("!!!");
    assert!(store
        .preview_backup(&serde_json::to_vec(&backup).unwrap())
        .is_err());
    assert_eq!(
        std::fs::read(dir.path().join("config.json")).unwrap(),
        b"original settings"
    );
}
#[test]
fn legacy_import_and_dictionary_encodings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("languages.json"),
        r#"{"review":{"English":true,"Explicit":true,"Count":2,"Used":"2026-10-16T00:00:00Z"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("conversions.json"),
        r#"{"はし":{"Text":"箸","Used":"2026-10-16T00:00:00Z"}}"#,
    )
    .unwrap();
    let state = Store::new(dir.path()).load().unwrap();
    assert_eq!(state.preferences.english.get("review"), Some(&true));
    assert_eq!(state.preferences.selected["はし"], "箸");
    let bytes = user_dictionary::export(vec![("めると".into(), "Meltype".into())].into_iter());
    let parsed = user_dictionary::parse(&bytes).unwrap();
    assert_eq!(parsed.words[0], ("めると".into(), "Meltype".into()));
    assert_eq!(parsed.encoding, "UTF-16");
    let plist=br#"<?xml version="1.0"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist><array><dict><key>shortcut</key><string>test</string><key>phrase</key><string>Test</string></dict></array></plist>"#;
    assert_eq!(
        user_dictionary::parse(plist).unwrap().words[0],
        ("test".into(), "Test".into())
    );
}
#[test]
fn context_classifier_respects_literals_blocks_and_prompts() {
    for (input, expected) in [
        ("let x = 1; // ", LineKind::Comment),
        ("let x = \"", LineKind::String),
        ("/* a\nb", LineKind::Comment),
        ("\"\"\"a\nb", LineKind::String),
        ("/* done */ x", LineKind::Code),
        ("#include", LineKind::Code),
        ("# comment", LineKind::Comment),
        ("│ › ", LineKind::Prompt),
        ("PS C:\\> ", LineKind::Code),
        ("don't", LineKind::Code),
    ] {
        assert_eq!(context::classify(input), expected, "{input}");
    }
}
#[test]
fn dictionaries_supply_context_translation_gloss_and_correction() {
    let d = Linguistics::bundled();
    assert_eq!(d.context("はしを", "ご飯を食べる"), Some("箸を".into()));
    assert!(d.meaning("箸", "はし").is_some());
    assert!(d
        .translations("複雑な", "ふくざつな")
        .contains(&"complex".into()));
    assert_eq!(d.misspelling("ぶれすれっど"), Some("ブレスレット".into()));
    assert_eq!(d.misspelling("シミュレーション"), None);
}
#[test]
fn predictions_commit_only_after_selection() {
    let mut s = Session::default();
    s.preferences
        .user_dictionary
        .insert("めるたいぷ".into(), vec!["Meltype".into()]);
    type_text(&mut s, "meru");
    assert!(s.predictions().contains(&"Meltype".into()));
    assert_eq!(s.commit().commits[0].text, "める");
    type_text(&mut s, "meru");
    s.key(9, 0, 0);
    assert_eq!(s.commit().commits[0].text, "Meltype");
}
#[test]
fn halfwidth_case_and_jis_kana() {
    let mut s = Session::default();
    type_text(&mut s, "gakkou");
    s.key(0x77, 0, 0);
    assert_eq!(s.commit().commits[0].text, "ｶﾞｯｺｳ");
    type_text(&mut s, "review");
    s.key(0x79, 0, 0);
    assert_eq!(s.display(false), "REVIEW");
    s.key(0x79, 0, 0);
    assert_eq!(s.display(false), "Review");
    s.key(0x79, 0, 0);
    assert_eq!(s.commit().commits[0].text, "review");
    s.settings.set("InputStyle", json!("Kana")).unwrap();
    for (vk, ch) in [(0x54, 't'), (0xc0, '@'), (0x51, 'q')] {
        s.key(vk, ch as i32, 0);
    }
    assert_eq!(s.commit().commits[0].text, "がた");
}
#[test]
fn snippets_tab_and_private_learning() {
    let mut s = Session::default();
    s.state
        .snippets
        .insert("sig".into(), "署名\nMeltype".into());
    type_text(&mut s, ";sig");
    assert_eq!(s.key(9, 0, 0).commits[0].text, "署名\nMeltype");
    s.private = true;
    assert!(!s.key(0, 'a' as i32, 0).consumed);
    assert!(s.raw().is_empty());
}
#[test]
fn protected_spans_preserve_automatic_text_and_honor_explicit_width() {
    let raw = "https://example.com/a 日本語 @name C:\\foo\\bar";
    let spans = protection::scan(raw);
    assert_eq!(spans.len(), 3);
    let mut automatic = Session::default();
    type_text(&mut automatic, "https://example.com/a");
    assert_eq!(automatic.commit().commits[0].text, "https://example.com/a");
    // Upstream CompositionText.Display explicitly applies width/case to Raw;
    // protection is an automatic-conversion policy, not a veto of F9/F10.
    let mut explicit = Session::default();
    type_text(&mut explicit, "https://example.com/a");
    explicit.key(0x78, 0, 0);
    assert_eq!(
        explicit.commit().commits[0].text,
        "ｈｔｔｐｓ：／／ｅｘａｍｐｌｅ．ｃｏｍ／ａ"
    );
    let mut uppercase = Session::default();
    uppercase
        .settings
        .set("SigilWordsDirect", json!(false))
        .unwrap();
    type_text(&mut uppercase, "@name");
    uppercase.key(0x79, 0, 0);
    assert_eq!(uppercase.display(false), "@NAME");
    uppercase.key(0x79, 0, 0);
    assert_eq!(uppercase.commit().commits[0].text, "@name");
}

#[test]
#[ignore = "manual prediction benchmark; run with --release --ignored --nocapture"]
fn prediction_growth_timing() {
    use std::{hint::black_box, time::Instant};
    let dictionary = meltype_core::dictionary::Dictionary::bundled();
    for round in 0..3 {
        let start = Instant::now();
        for _ in 0..200 {
            black_box(dictionary.predict_english("hel"));
        }
        println!(
            "ENGLISH_BENCH round={round} us={:.2}",
            start.elapsed().as_secs_f64() * 1e6 / 200.0
        );
    }
    for size in [0, 5000] {
        let mut session = Session::default();
        session
            .settings
            .set("LiveConversion", json!(false))
            .unwrap();
        session
            .settings
            .set("PredictiveCandidates", json!(true))
            .unwrap();
        session
            .preferences
            .user_dictionary
            .insert("めるたいぷ".into(), vec!["Meltype".into()]);
        type_text(&mut session, "meru");
        for i in 0..size {
            session
                .state
                .snippets
                .insert(format!("snippet{i}"), "x".repeat(128));
            session
                .preferences
                .selected
                .insert(format!("よみ{i:05}"), format!("単語{i}"));
        }
        session.state.preferences = session.preferences.clone();
        assert!(!session.predictions().is_empty());
        for round in 0..3 {
            let start = Instant::now();
            for _ in 0..200 {
                black_box(session.predictions());
            }
            println!(
                "PREDICTION_BENCH size={size} round={round} us={:.2}",
                start.elapsed().as_secs_f64() * 1e6 / 200.0
            );
        }
    }
}

#[test]
fn predictions_use_current_preferences_and_preserve_learning_order() {
    use meltype_core::storage::{Phrase, State};
    let mut state = State::default();
    for (reading, text, count, used) in [
        ("める", "完全一致は除外", 100, 100),
        ("めるたいぷ", "学習", 3, 10),
        ("めるたいぷ", "同順位の先", 2, 10),
        ("めるたいぷ", "同順位の後", 2, 10),
        ("めるたいぷ", "学習", 1, 20),
    ] {
        state.phrases.push(Phrase {
            reading: reading.into(),
            text: text.into(),
            count,
            used,
        });
    }
    state.preferences.user_dictionary.insert(
        "めるたいぷ".into(),
        vec!["新しい登録".into(), "古い登録".into(), "学習".into()],
    );
    state
        .preferences
        .selected
        .insert("めるたいぷ".into(), "選択履歴".into());
    assert_eq!(
        Linguistics::bundled().predict("める", &state),
        ["学習", "同順位の先", "同順位の後", "古い登録", "新しい登録"]
    );
    state.phrases.clear();
    assert_eq!(
        Linguistics::bundled().predict("める", &state),
        ["学習", "古い登録", "新しい登録", "選択履歴"]
    );
    assert!(Linguistics::bundled().predict("め", &state).is_empty());
    let mut session = Session::default();
    session.state = state;
    session
        .preferences
        .user_dictionary
        .insert("めるたいぷ".into(), vec!["Meltype".into()]);
    session
        .preferences
        .selected
        .insert("めるたいぷ".into(), "最新の選択".into());
    type_text(&mut session, "meru");
    let predictions = session.predictions();
    assert!(predictions.contains(&"Meltype".into()));
    assert!(predictions.contains(&"最新の選択".into()));
    assert!(!predictions.contains(&"古い登録".into()));
    assert!(!predictions.contains(&"選択履歴".into()));
}
