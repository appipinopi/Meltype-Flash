// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use meltype_core::{
    app_identity::{ibus_application, Platform},
    settings::Settings,
};
use serde_json::json;
#[test]
fn native_apps_use_existing_profiles_without_exe_suffix_guessing() {
    let settings = Settings::default();
    for (platform, identities) in [
        (
            Platform::Linux,
            vec![
                "code",
                "code-insiders",
                "cursor",
                "zed",
                "gnome-terminal",
                "konsole",
            ],
        ),
        (
            Platform::Mac,
            vec![
                "com.microsoft.VSCode",
                "com.microsoft.VSCodeInsiders",
                "com.apple.Terminal",
                "com.googlecode.iterm2",
            ],
        ),
    ] {
        for identity in identities {
            assert_eq!(
                settings.for_app_on(identity, platform).text("AppProfile"),
                "Code",
                "{identity}"
            );
        }
    }
    assert_eq!(
        settings
            .for_app_on("code", Platform::Windows)
            .text("AppProfile"),
        "General"
    );
    assert_eq!(
        settings
            .for_app_on("unknown", Platform::Linux)
            .text("AppProfile"),
        "General"
    );
    assert_eq!(
        settings
            .for_app_on("Code.exe", Platform::Windows)
            .text("AppProfile"),
        "Code"
    );
}
#[test]
fn native_overrides_win_and_unrelated_executables_do_not_match() {
    let mut settings = Settings::default();
    let rules = settings.0["AppRules"].as_array_mut().unwrap();
    rules.push(json!({"Process":"code","Enabled":false,"Profile":"General"}));
    rules.push(json!({"Process":"com.microsoft.VSCode","Enabled":true,"Profile":"General"}));
    rules.push(json!({"Process":"custom.exe","Enabled":false,"Profile":"Code"}));
    assert!(!settings
        .for_app_on("code", Platform::Linux)
        .boolean("Enabled"));
    assert_eq!(
        settings
            .for_app_on("com.microsoft.VSCode", Platform::Mac)
            .text("AppProfile"),
        "General"
    );
    assert!(settings
        .for_app_on("custom", Platform::Linux)
        .boolean("Enabled"));
    assert_eq!(
        settings
            .for_app_on("com.microsoft.VSCode.evil", Platform::Mac)
            .text("AppProfile"),
        "General"
    );
}
#[test]
fn ibus_toolkit_names_are_not_mistaken_for_applications() {
    for client in ["gtk3-im:code", "gtk4-im:code", "gtk-im:code"] {
        assert_eq!(ibus_application(client), "code");
    }
    for client in [
        "gnome-shell",
        "xim",
        "QIBusInputContext",
        "fake",
        "gtk3-im:",
    ] {
        assert_eq!(ibus_application(client), "");
    }
}
