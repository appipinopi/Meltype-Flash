// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::settings::Settings;
use serde_json::{json, Value};

fn additions() -> Vec<String> {
    let inventory: Value =
        serde_json::from_str(include_str!("../../../tools/parity/pre1/inventory.json")).unwrap();
    inventory["missing_settings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["first_present_in_requested_release"] != "v1.3.1-pre.1")
        .map(|v| match v["key"].as_str().unwrap() {
            "ToggleKeyboardShortcuts" => "ToggleShortcuts".to_owned(),
            key => key.to_owned(),
        })
        .collect()
}
fn reference_default(reference: &Value, key: &str) -> Value {
    if key == "ToggleShortcuts" {
        json!([{"Key":25,"Modifiers":0},{"Key":192,"Modifiers":4}])
    } else {
        reference["defaults"][key].clone()
    }
}
#[test]
fn all_28_defaults_types_profiles_and_import_export_match_target() {
    let reference: Value =
        serde_json::from_str(include_str!("data/upstream-v130-defaults.json")).unwrap();
    let fields = additions();
    assert_eq!(fields.len(), 28);
    let mut s = Settings::parse("{}").unwrap();
    let original = s.text("ActiveProfile").to_owned();
    let mut changed = serde_json::Map::new();
    for key in &fields {
        let value = &reference_default(&reference, key);
        assert_eq!(&s.0[key], value, "default {key}");
        let alternate = match value {
            Value::Bool(b) => json!(!b),
            Value::String(_) => json!("test-value"),
            Value::Array(_) if key == "ToggleShortcuts" => json!([{"Key":75,"Modifiers":3}]),
            _ => panic!("unexpected addition type: {key}"),
        };
        assert!(s.set(key, json!(null)).is_err(), "reject wrong type {key}");
        s.set(key, alternate.clone()).unwrap();
        changed.insert(key.clone(), alternate);
    }
    s.add_profile("new defaults").unwrap();
    for key in &fields {
        s.set(key, reference_default(&reference, key)).unwrap();
    }
    s.switch_profile(&original).unwrap();
    for key in &fields {
        assert_eq!(s.0[key], changed[key], "profile {key}");
    }
    let export = s.export_profile().to_string();
    let mut restored = Settings::parse(&serde_json::to_string(&s).unwrap()).unwrap();
    let imported = restored.import_profile(&export).unwrap();
    restored.switch_profile(&imported).unwrap();
    for key in &fields {
        assert_eq!(restored.0[key], changed[key], "import {key}");
    }
    restored.switch_profile("new defaults").unwrap();
    for key in &fields {
        assert_eq!(
            restored.0[key],
            reference_default(&reference, key),
            "default profile {key}"
        );
    }
}
#[test]
fn removed_choices_migrate_at_top_level_profiles_and_import() {
    for mode in ["AutoSwitch", "autoswitch", "AUTOSWITCH"] {
        let mut s = Settings::parse(&json!({"SettingsVersion":5,"Mode":mode,"InputStyle":"bOtH","TsfIntroduced":true,
            "future":{"keep":true}, "Profiles":[{"Name":"old","Values":{"InputStyle":"Both","Mode":"AutoSwitch","ClipboardCandidates":"bad"}}]}).to_string()).unwrap();
        assert_eq!(s.text("Mode"), "Tsf");
        assert_eq!(s.text("InputStyle"), "Romaji");
        assert!(!s.boolean("TsfIntroduced"));
        assert_eq!(s.0["future"], json!({"keep":true}));
        s.switch_profile("old").unwrap();
        assert_eq!(s.text("InputStyle"), "Romaji");
        assert_eq!(
            s.0["ClipboardCandidates"],
            Settings::default().0["ClipboardCandidates"]
        );
        let before = s.0.clone();
        s.normalize();
        assert_eq!(s.0, before, "migration must be idempotent");
        let name = s.import_profile(&json!({"format":"meltype-profile","name":"imported","values":{"InputStyle":"Both"}}).to_string()).unwrap();
        s.switch_profile(&name).unwrap();
        assert_eq!(s.text("InputStyle"), "Romaji");
    }
    let s = Settings::parse(r#"{"Mode":"Keyboard","TsfIntroduced":true,"Profiles":[{"Name":"legacy","Values":{"Mode":"AutoSwitch"}}]}"#).unwrap();
    assert_eq!(s.text("Mode"), "Keyboard");
    assert!(
        s.boolean("TsfIntroduced"),
        "nested Mode must not restart introduction"
    );
}

#[test]
fn migration_is_persisted_once_and_does_not_lose_later_edits() {
    use meltype_core::storage::Store;
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    let file = dir.path().join("config.json");
    std::fs::write(&file, r#"{"SettingsVersion":5,"Mode":"AutoSwitch","InputStyle":"Both","GitHubRepository":"owner/repo"}"#).unwrap();
    let loaded = store.settings().unwrap();
    assert_eq!(loaded.text("Mode"), "Tsf");
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved["Mode"], "Tsf");
    assert_eq!(saved["InputStyle"], "Romaji");
    assert_eq!(saved["GitHubRepository"], "owner/repo");
    store
        .change_settings(|s| s.set("Enabled", json!(false)))
        .unwrap();
    assert!(!store.settings().unwrap().boolean("Enabled"));
}
#[test]
fn bracket_caret_request_is_opt_in_and_exact_pair_only() {
    use meltype_core::Session;
    for (enabled, capable) in [(false, false), (false, true), (true, false), (true, true)] {
        for text in ["()", "[]", "{}", "(abc)", "hello"] {
            let mut s = Session::default();
            s.can_move_caret = capable;
            s.settings
                .set("CursorIntoBrackets", json!(enabled))
                .unwrap();
            s.settings.set("LiveConversion", json!(false)).unwrap();
            s.settings
                .set("FullWidthParentheses", json!(false))
                .unwrap();
            for ch in text.chars() {
                s.key(0, ch as i32, 0);
            }
            let result = s.commit();
            let edit = result.commits.last().unwrap();
            assert_eq!(
                edit.caret_back,
                u32::from(
                    enabled && capable && meltype_core::shortcuts::is_bracket_pair(&edit.text)
                ),
                "{text}: {}",
                edit.text
            );
            let serialized = serde_json::to_value(edit).unwrap();
            assert_eq!(serialized.get("caretBack").is_some(), edit.caret_back != 0);
        }
    }
}
