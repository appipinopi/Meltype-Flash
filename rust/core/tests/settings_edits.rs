// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
use meltype_core::{control, settings::Settings, storage::Store};
use serde_json::json;
fn settings() -> Settings {
    let mut s = Settings::default();
    s.normalize();
    s
}
#[test]
fn upstream_named_toggle_modifiers_and_numeric_exports_are_interoperable() {
    let settings = Settings::parse(
        r#"{"SettingsVersion":7,"ToggleShortcuts":[
        {"Key":25,"Modifiers":"None"},{"Key":192,"Modifiers":"Alt"},
        {"Key":74,"Modifiers":"Control, Alt"},{"Key":75,"Modifiers":3}]}"#,
    )
    .unwrap();
    let bindings = settings.toggle_shortcuts();
    assert_eq!(
        bindings
            .iter()
            .map(|s| (s.key, s.modifiers))
            .collect::<Vec<_>>(),
        [(25, 0), (192, 4), (74, 6), (75, 3)]
    );
    assert_eq!(
        Settings::parse(&serde_json::to_string(&settings).unwrap())
            .unwrap()
            .toggle_shortcuts(),
        bindings
    );
    assert!(
        serde_json::from_value::<meltype_core::shortcuts::ToggleShortcut>(
            json!({"Key":74,"Modifiers":"FutureModifier"})
        )
        .is_err()
    );
}
#[test]
fn v140_shortcuts_migrate_legacy_settings_profiles_and_import_once() {
    use meltype_core::shortcuts::ToggleShortcut;
    let mut settings = Settings::parse(
        &json!({
            "SettingsVersion":5, "HankakuTogglesKeyboard":true,
            "ToggleKeyboardShortcuts":"", "FutureSetting":{"keep":true},
            "Profiles":[{"Name":"旧設定", "Values":{
                "HankakuTogglesKeyboard":false, "ToggleKeyboardShortcuts":"Ctrl+Shift+K"
            }}]
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(settings.integer("SettingsVersion"), 7);
    assert_eq!(
        settings.toggle_shortcuts(),
        vec![
            ToggleShortcut {
                key: 25,
                modifiers: 0
            },
            ToggleShortcut {
                key: 192,
                modifiers: 4
            },
            ToggleShortcut {
                key: 74,
                modifiers: 6
            }
        ]
    );
    assert_eq!(settings.0["FutureSetting"], json!({"keep":true}));
    let before = settings.0.clone();
    settings.normalize();
    assert_eq!(settings.0, before);
    for legacy in ["HankakuTogglesKeyboard", "ToggleKeyboardShortcuts"] {
        assert!(!settings.0.contains_key(legacy));
        assert!(settings.0["Profiles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["Values"].get(legacy).is_none()));
    }
    settings.switch_profile("旧設定").unwrap();
    assert_eq!(
        settings.toggle_shortcuts(),
        vec![ToggleShortcut {
            key: 75,
            modifiers: 3
        }]
    );
    let name = settings
        .import_profile(
            &json!({"format":"meltype-profile","name":"移行",
        "values":{"HankakuTogglesKeyboard":false,"ToggleKeyboardShortcuts":"Alt+英数"}})
            .to_string(),
        )
        .unwrap();
    settings.switch_profile(&name).unwrap();
    assert_eq!(
        settings.toggle_shortcuts(),
        vec![ToggleShortcut {
            key: 240,
            modifiers: 4
        }]
    );
    settings.set("ToggleShortcuts", json!([])).unwrap();
    assert!(Settings::parse(&serde_json::to_string(&settings).unwrap())
        .unwrap()
        .toggle_shortcuts()
        .is_empty());
    // A current-version file may still carry a legacy inactive profile.
    // The migrated profile must be saved before it is ever activated.
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("config.json");
    std::fs::write(&file, json!({"SettingsVersion":7,"Profiles":[{
        "Name":"未選択", "Values":{"HankakuTogglesKeyboard":false,"ToggleKeyboardShortcuts":"Ctrl+Alt+K"}
    }]}).to_string()).unwrap();
    Store::new(directory.path()).settings().unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let values = &saved["Profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["Name"] == "未選択")
        .unwrap()["Values"];
    assert!(values.get("ToggleKeyboardShortcuts").is_none());
    assert_eq!(values["ToggleShortcuts"], json!([{"Key":75,"Modifiers":6}]));
}
#[test]
fn v140_shortcuts_validate_conflicts_and_report_excluded_legacy_keys() {
    use meltype_core::shortcuts::ToggleShortcut;
    let mut settings = settings();
    settings.set("ControlKeyBindings", json!("K=Left")).unwrap();
    for text in [
        "CapsLock",
        "Shift+A",
        "Ctrl+半角/全角",
        "Ctrl+K",
        "Ctrl+Shift+K",
        "Ctrl+U",
        "F9",
    ] {
        let shortcut = ToggleShortcut::parse(text).unwrap();
        assert!(shortcut.validate(&settings, &[]).is_err(), "{text}");
    }
    for text in [
        "Ctrl+CapsLock",
        "Alt+英数",
        "Ctrl+Alt+K",
        "半角/全角",
        "Alt+Backquote",
    ] {
        let shortcut = ToggleShortcut::parse(text).unwrap();
        assert!(shortcut.validate(&settings, &[]).is_ok(), "{text}");
        assert_eq!(ToggleShortcut::parse(&shortcut.label()).unwrap(), shortcut);
    }
    settings
        .set("ConvertKeysSwitchKeyboard", json!(true))
        .unwrap();
    assert!(ToggleShortcut::parse("Henkan")
        .unwrap()
        .validate(&settings, &[])
        .is_err());
    let warnings = Settings::shortcut_migration_warnings(&json!({
        "ControlKeyBindings":"K=Left", "ToggleKeyboardShortcuts":"Ctrl+K;Ctrl+Unknown;Ctrl+CapsLock"
    }).to_string());
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().any(|warning| warning.contains("Control")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("Ctrl+Unknown")));
    let before = settings.0.clone();
    assert!(settings
        .set(
            "ToggleShortcuts",
            json!([{"Key":25,"Modifiers":0},{"Key":244,"Modifiers":0}])
        )
        .is_err());
    assert_eq!(
        settings.0, before,
        "invalid lists must not mutate saved settings"
    );
}
#[test]
fn v140_toggle_detector_pairs_jis_keys_repeats_layout_and_modifier_release() {
    use meltype_core::shortcuts::{ToggleDetector, ToggleShortcut};
    let bindings = ToggleShortcut::defaults();
    let mut detector = ToggleDetector::default();
    let first = detector.press(0xf4, 0x29, false, 0x19, 0, &bindings);
    assert!(first.consume && first.toggle);
    let repeat = detector.press(0xf4, 0x29, false, 0x19, 0, &bindings);
    assert!(repeat.consume && !repeat.toggle);
    assert!(detector.release(0xf3, 0x29, false));
    // VK_KANJI on a US Backquote scan identifies the Alt+Backquote binding.
    assert!(detector.press(0x19, 0x29, false, 0xc0, 4, &bindings).toggle);
    // A changed setting/modifier while held must still consume the repeat.
    let repeat = detector.press(0x19, 0x29, false, 0xc0, 0, &[]);
    assert!(repeat.consume && !repeat.toggle);
    assert!(detector.release(0xc0, 0x29, false));
    // JIS VK_KANJI can omit its own keyup; releasing Alt re-arms the physical key.
    let jis = [ToggleShortcut {
        key: 25,
        modifiers: 4,
    }];
    assert!(detector.press(0x19, 0x29, false, 0x19, 4, &jis).toggle);
    assert!(!detector.release(0xa4, 0x38, false));
    assert!(detector.press(0x19, 0x29, false, 0x19, 4, &jis).toggle);
    assert!(detector.release(0x19, 0x29, false));
    assert!(
        !detector
            .press(0xc0, 0x29, false, 0xc0, 0, &bindings)
            .consume
    );
    // Adding a modifier after a physical key was already pressed is not a new press.
    assert!(
        !detector
            .press(0xc0, 0x29, false, 0xc0, 4, &bindings)
            .consume
    );
}
#[test]
fn hosts_without_ai_remove_only_model_settings_from_all_profiles() {
    let mut settings = Settings::parse(&json!({
        "UseZenzai": true, "ZenzaiServerPath": "server", "ZenzaiModelPath": "model",
        "PredictiveCandidates": true, "LearningEnabled": true,
        "FutureSetting": {"keep":true},
        "Profiles": [{"Name":"デスクトップから移行", "Values":{
            "UseZenzai":true, "ZenzaiServerPath":"other-server", "ZenzaiModelPath":"other-model",
            "PredictiveCandidates":false, "FutureSetting":{"keep":true}
        }}]
    }).to_string()).unwrap();
    assert!(settings.remove_neural_settings());
    assert!(!settings.remove_neural_settings());
    for key in ["UseZenzai", "ZenzaiServerPath", "ZenzaiModelPath"] {
        assert!(!settings.0.contains_key(key));
        for profile in settings.0["Profiles"].as_array().unwrap() {
            assert!(profile["Values"].get(key).is_none());
        }
    }
    assert!(settings.boolean("PredictiveCandidates"));
    assert!(settings.boolean("LearningEnabled"));
    assert_eq!(settings.0["FutureSetting"], json!({"keep":true}));
    let values = &settings.0["Profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["Name"] == "デスクトップから移行")
        .unwrap()["Values"];
    assert_eq!(values["PredictiveCandidates"], false);
    assert_eq!(values["FutureSetting"], json!({"keep":true}));
}
#[test]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn desktop_settings_keep_optional_neural_configuration() {
    let settings = Settings::parse(
        r#"{"UseZenzai":true,"ZenzaiServerPath":"server","ZenzaiModelPath":"model"}"#,
    )
    .unwrap();
    assert!(settings.boolean("UseZenzai"));
    assert_eq!(settings.text("ZenzaiServerPath"), "server");
    assert_eq!(settings.text("ZenzaiModelPath"), "model");
}
#[test]
fn unchanged_window_preserves_current_settings_and_unknown_values() {
    let opened = settings();
    let mut current = opened.clone();
    current.set("Enabled", json!(false)).unwrap();
    current.set("FutureSetting", json!({"keep":true})).unwrap();
    assert_eq!(
        Settings::merge_edits(&opened, &opened, &current).0,
        current.0
    );
}
#[test]
fn window_edits_merge_into_original_profile_after_external_switch() {
    let mut opened = settings();
    opened.add_profile("仕事").unwrap();
    // Upstream AddProfile selects the new profile. This scenario opens the
    // settings window on 標準 before a separate window switches to 仕事.
    opened.switch_profile("標準").unwrap();
    let mut edited = opened.clone();
    edited.set("LiveConversion", json!(false)).unwrap();
    let mut current = opened.clone();
    current.set("Enabled", json!(false)).unwrap();
    current.switch_profile("仕事").unwrap();
    current.set("JapaneseThreshold", json!(9)).unwrap();
    let mut merged = Settings::merge_edits(&opened, &edited, &current);
    assert!(!merged.boolean("Enabled"));
    assert_eq!(merged.text("ActiveProfile"), "仕事");
    assert_eq!(merged.integer("JapaneseThreshold"), 9);
    assert!(merged.boolean("LiveConversion"));
    merged.switch_profile("標準").unwrap();
    assert!(!merged.boolean("LiveConversion"));
    assert_eq!(merged.integer("JapaneseThreshold"), 4);
}
#[test]
fn edited_profile_switch_and_addition_preserve_external_edits_in_each_profile() {
    let mut opened = settings();
    opened.add_profile("仕事").unwrap();
    opened.switch_profile("標準").unwrap();
    let mut edited = opened.clone();
    edited.switch_profile("仕事").unwrap();
    edited.set("LiveConversion", json!(false)).unwrap();
    edited.add_profile("趣味").unwrap();
    edited.switch_profile("仕事").unwrap();
    let mut current = opened.clone();
    current.set("PredictiveCandidates", json!(false)).unwrap();
    current.switch_profile("仕事").unwrap();
    current.set("JapaneseThreshold", json!(8)).unwrap();
    let mut merged = Settings::merge_edits(&opened, &edited, &current);
    assert_eq!(merged.text("ActiveProfile"), "仕事");
    assert!(!merged.boolean("LiveConversion"));
    assert_eq!(merged.integer("JapaneseThreshold"), 8);
    merged.switch_profile("標準").unwrap();
    assert!(!merged.boolean("PredictiveCandidates"));
    merged.switch_profile("趣味").unwrap();
    assert!(!merged.boolean("LiveConversion"));
}
#[test]
fn merge_management_api_is_atomic_and_preserves_other_process_changes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    let opened = store.settings().unwrap();
    let mut edited = opened.clone();
    edited.set("JapaneseThreshold", json!(7)).unwrap();
    store
        .change_settings(|s| s.set("Enabled", json!(false)))
        .unwrap();
    control::execute(
        &store,
        &json!({"op":"settings_merge","opened":opened,"edited":edited}),
    )
    .unwrap();
    let actual = store.settings().unwrap();
    assert_eq!(actual.integer("JapaneseThreshold"), 7);
    assert!(!actual.boolean("Enabled"));
    let bytes = std::fs::read(dir.path().join("config.json")).unwrap();
    assert!(control::execute(
        &store,
        &json!({"op":"settings_merge","opened":actual,"edited":"invalid"})
    )
    .is_err());
    assert_eq!(
        std::fs::read(dir.path().join("config.json")).unwrap(),
        bytes
    );
}

#[test]
fn upstream_profile_files_round_trip_and_cannot_overwrite_shared_settings() {
    let mut source = settings();
    source.set("LiveConversion", json!(false)).unwrap();
    source.set("JapaneseThreshold", json!(8)).unwrap();
    source.set("Enabled", json!(false)).unwrap();
    let file = source.export_profile();
    assert_eq!(file["format"], "meltype-profile");
    assert!(file["values"].get("Enabled").is_none());
    let mut target = settings();
    let name = target.import_profile(&file.to_string()).unwrap();
    assert_eq!(name, "標準 (2)");
    assert_eq!(target.text("ActiveProfile"), name);
    assert!(!target.boolean("LiveConversion"));
    assert!(target.boolean("Enabled"));
    assert_eq!(target.integer("JapaneseThreshold"), 8);
    let name = target.import_profile(r#"{"format":"meltype-profile","name":"仕事","values":{"Enabled":false,"JapaneseThreshold":999,"Unknown":1}}"#).unwrap();
    assert_eq!(name, "仕事");
    assert!(target.boolean("Enabled"));
    assert_eq!(target.integer("JapaneseThreshold"), 20);
    assert!(!target.0.contains_key("Unknown"));
    let before = target.0.clone();
    for file in [
        r#"{"format":"something-else","values":{}}"#,
        r#"{"format":"meltype-profile","values":{"LiveConversion":"yes"}}"#,
        r#"{"format":"meltype-profile","values":[]}"#,
    ] {
        assert!(target.import_profile(file).is_err());
        assert_eq!(target.0, before);
    }
}
#[test]
fn rename_and_remove_current_profile_use_upstream_rules_and_merge_on_save() {
    let opened = settings();
    let mut edited = opened.clone();
    edited.rename_profile("標準", "仕事").unwrap();
    assert_eq!(edited.text("ActiveProfile"), "仕事");
    assert!(edited.rename_profile("仕事", " ").is_err());
    assert!(edited.remove_profile("仕事").is_err());
    edited.add_profile("趣味").unwrap();
    edited.switch_profile("仕事").unwrap();
    assert!(edited.rename_profile("仕事", "趣味").is_err());
    let mut merged = Settings::merge_edits(&opened, &edited, &opened);
    assert_eq!(merged.text("ActiveProfile"), "仕事");
    merged.remove_profile("仕事").unwrap();
    assert_eq!(merged.text("ActiveProfile"), "趣味");
    assert!(merged.remove_profile("unknown").is_err());
}
