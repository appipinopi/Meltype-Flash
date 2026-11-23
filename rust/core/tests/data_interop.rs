// SPDX-License-Identifier: GPL-3.0-or-later
use base64::{engine::general_purpose::STANDARD, Engine};
use meltype_core::{
    control,
    storage::{Change, Store},
};
use serde_json::{json, Value};
use std::fs;
fn backup(files: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"format":"meltype-backup","version":1,"files":files})).unwrap()
}
#[test]
fn upstream_backup_replaces_stale_state_and_preserves_unmentioned_categories() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    control::execute(
        &store,
        &json!({"op":"snippet_set","name":"keep","text":"既存"}),
    )
    .unwrap();
    store
        .record(vec![Change::Conversion {
            reading: "はし".into(),
            text: "旧候補".into(),
            translation: false,
        }])
        .unwrap();
    let bytes = backup(
        json!({"conversions.json":STANDARD.encode(r#"{"はし":{"Text":"新候補","Used":"2026-10-19T00:00:00Z"},"き":{"Text":"木"}}"#),"userdict.txt":STANDARD.encode("はし\t箸\nはし\t橋\n")}),
    );
    assert_eq!(store.restore(&bytes).unwrap(), 2);
    let state = store.load().unwrap();
    assert_eq!(state.preferences.selected["はし"], "新候補");
    assert!(!state.preferences.selected.contains_key("き"));
    assert_eq!(state.snippets["keep"], "既存");
    assert_eq!(state.preferences.user_dictionary["はし"], ["橋", "箸"]);
    assert!(dir.path().join("state.json.before-restore").exists());
    let restart = Store::new(dir.path()).load().unwrap();
    assert_eq!(restart.preferences.selected["はし"], "新候補");
}
#[test]
fn backup_exports_updated_upstream_files_and_deletions_not_stale_copies() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    fs::write(dir.path().join("snippets.json"), r#"{"old":"old"}"#).unwrap();
    control::execute(&store, &json!({"op":"snippet_remove","name":"old"})).unwrap();
    control::execute(
        &store,
        &json!({"op":"snippet_set","name":"sig","text":"新しい\n署名 😀"}),
    )
    .unwrap();
    store
        .record(vec![Change::Language {
            raw: "review".into(),
            english: true,
            explicit: true,
        }])
        .unwrap();
    let value: Value = serde_json::from_slice(&store.backup().unwrap()).unwrap();
    for name in [
        "state.json",
        "model.json",
        "languages.json",
        "conversions.json",
        "translations.json",
        "snippets.json",
        "phrases.txt",
        "userdict.txt",
    ] {
        assert!(value["files"][name].is_string(), "{name}");
    }
    let bytes = STANDARD
        .decode(value["files"]["snippets.json"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        json!({"sig":"新しい\n署名 😀"})
    );
    store
        .update(|state| {
            state.clear_learning();
            Ok(())
        })
        .unwrap();
    let value: Value = serde_json::from_slice(&store.backup().unwrap()).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(
            &STANDARD
                .decode(value["files"]["languages.json"].as_str().unwrap())
                .unwrap()
        )
        .unwrap(),
        json!({})
    );
}
#[test]
fn restore_preflights_all_targets_and_malformed_legacy_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    fs::write(dir.path().join("config.json"), b"{}").unwrap();
    fs::create_dir(dir.path().join("snippets.json")).unwrap();
    let bytes = backup(
        json!({"config.json":STANDARD.encode("{\"Enabled\":false}"),"snippets.json":STANDARD.encode("{}"),"state.json":STANDARD.encode("{}")}),
    );
    assert!(store.restore(&bytes).is_err());
    assert_eq!(fs::read(dir.path().join("config.json")).unwrap(), b"{}");
    assert!(!dir.path().join("state.json").exists());
    let bytes = backup(
        json!({"config.json":STANDARD.encode("{\"Enabled\":false}"),"languages.json":STANDARD.encode("broken")}),
    );
    assert!(store.restore(&bytes).is_err());
    assert_eq!(fs::read(dir.path().join("config.json")).unwrap(), b"{}");
}
#[test]
fn bom_phrase_duplicates_and_upstream_dictionary_priority_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.json"),
        b"\xef\xbb\xbf{\"Enabled\":false}",
    )
    .unwrap();
    fs::write(dir.path().join("languages.json"), b"\xef\xbb\xbf{}").unwrap();
    fs::write(
        dir.path().join("userdict.txt"),
        "\u{feff}はし\t箸\nはし\t橋\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("phrases.txt"),
        "\u{feff}きょう\t今日\t2\t638965152001234567\nきょう\t今日\t0\t638965152001234567\n",
    )
    .unwrap();
    let store = Store::new(dir.path());
    assert!(!store.settings().unwrap().boolean("Enabled"));
    let s = store.load().unwrap();
    assert_eq!(s.phrases.len(), 1);
    assert_eq!(s.phrases[0].count, 1);
    assert_eq!(s.phrases[0].reading, "きょう");
    assert_eq!(s.preferences.user_dictionary["はし"], ["橋", "箸"]);
    let file = dir.path().join("import.txt");
    fs::write(&file, "はし\t橋\nはし\t端\nはし\t端\n").unwrap();
    let result = control::execute(&store, &json!({"op":"dictionary_import","path":file})).unwrap();
    assert_eq!(result["imported"], 1);
    assert_eq!(
        store.load().unwrap().preferences.user_dictionary["はし"],
        ["端", "橋", "箸"]
    );
}

#[test]
fn partial_restore_keeps_translation_timestamps_and_export_ignores_stale_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    store
        .record(vec![Change::Conversion {
            reading: "き".into(),
            text: "tree".into(),
            translation: true,
        }])
        .unwrap();
    let before = store.load().unwrap();
    let bytes = backup(json!({"snippets.json":STANDARD.encode(r#"{"sig":"署名"}"#)}));
    store.restore(&bytes).unwrap();
    let after = Store::new(dir.path()).load().unwrap();
    assert_eq!(after.used, before.used);
    assert_eq!(after.translations, before.translations);
    fs::write(dir.path().join("languages.json"), "broken").unwrap();
    fs::write(dir.path().join("conversions.json"), [0xff]).unwrap();
    fs::write(dir.path().join("phrases.txt"), [0xff]).unwrap();
    assert!(store.backup().is_ok());
}

#[test]
fn upstream_naive_and_fractional_dates_survive_export() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    let date = "2026-10-19T01:02:03.1234567";
    let mut value: Value = serde_json::from_slice(&backup(json!({"conversions.json": STANDARD.encode(serde_json::to_vec(&json!({"はし":{"Text":"橋","Used":date}})).unwrap())}))).unwrap();
    value["created"] = json!(date);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(store.preview_backup(&bytes).unwrap()["created"], date);
    store.restore(&bytes).unwrap();
    assert!(store.load().unwrap().used["conversion:はし"] > 0);
    let exported: Value = serde_json::from_slice(&store.backup().unwrap()).unwrap();
    let conversions: Value = serde_json::from_slice(
        &STANDARD
            .decode(exported["files"]["conversions.json"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(conversions["はし"]["Used"], date);
}

#[test]
fn direct_legacy_snippets_use_the_same_name_and_newline_rules_as_import() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("snippets.json"), serde_json::to_vec(&json!({
        "sig":"one\r\ntwo\rthree\u{0085}four\u{2028}five", "bad name":"skip", ";bad":"skip", "日本語":"skip", "empty":"", "null":null
    })).unwrap()).unwrap();
    let state = Store::new(dir.path()).load().unwrap();
    assert_eq!(state.snippets.len(), 1);
    assert_eq!(state.snippets["sig"], "one\ntwo\nthree\nfour\nfive");
}

#[test]
fn issue_151_json_round_trip_and_failed_reads_preserve_previous_data() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path());
    store
        .change_settings(|s| s.set("FileLog", json!(true)))
        .unwrap();
    store
        .record(vec![
            Change::Conversion {
                reading: "ごかん".into(),
                text: "互換".into(),
                translation: false,
            },
            Change::Language {
                raw: "emoji".into(),
                english: true,
                explicit: true,
            },
        ])
        .unwrap();
    let restarted = Store::new(directory.path());
    assert!(restarted.settings().unwrap().boolean("FileLog"));
    assert_eq!(
        restarted.load().unwrap().preferences.selected["ごかん"],
        "互換"
    );
    assert!(restarted.load().unwrap().language_counts["emoji"].english);
    restarted
        .record(vec![Change::Conversion {
            reading: "はし".into(),
            text: "橋".into(),
            translation: false,
        }])
        .unwrap();
    assert_eq!(store.load().unwrap().preferences.selected["ごかん"], "互換");
    for (name, bytes) in [
        ("config.json", b"{invalid".as_slice()),
        ("state.json", b"{broken".as_slice()),
    ] {
        let path = directory.path().join(name);
        fs::write(&path, bytes).unwrap();
        if name == "config.json" {
            assert!(store
                .change_settings(|s| s.set("FileLog", json!(false)))
                .is_err());
        } else {
            assert!(store.record(vec![]).is_err());
        }
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
