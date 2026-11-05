// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use eframe::egui;
use meltype_core::settings::Settings;
use serde_json::{json, Value};

pub fn editor(ui: &mut egui::Ui, key: &str, value: &mut Value, settings: &Settings) -> bool {
    let Some(rows) = value.as_array_mut() else {
        return false;
    };
    let old = rows.clone();
    if cfg!(target_os = "linux") && key == "AppRules" {
        ui.label("LinuxのアプリID（例: code、org.gnome.Terminal）を指定してください。Windows用ルールは非表示です。");
    }
    let kinds = key == "AppKinds";
    if cfg!(target_os = "linux") && !kinds && ui.button("Linuxの標準アプリを追加").clicked()
    {
        for name in [
            "code",
            "code-oss",
            "codium",
            "code-insiders",
            "cursor",
            "windsurf",
            "zed",
            "sublime_text",
            "gnome-terminal",
            "konsole",
            "kitty",
            "foot",
            "wezterm",
            "alacritty",
        ] {
            if rows.iter().any(|row| row["Process"] == name) {
                continue;
            }
            let legacy = meltype_core::app_identity::legacy_rule(
                name,
                meltype_core::app_identity::Platform::Linux,
            );
            let mut rule = rows
                .iter()
                .rev()
                .find(|row| Some(row["Process"].as_str().unwrap_or("")) == legacy)
                .cloned()
                .unwrap_or_else(|| json!({"Enabled":true,"Profile":"Code"}));
            rule["Process"] = json!(name);
            rows.push(rule);
        }
    }
    let mut remove = None;
    let mut names = vec![
        ("General".to_owned(), "一般".to_owned()),
        ("Code".into(), "コード".into()),
        ("Game".into(), "ゲーム".into()),
    ];
    if !kinds {
        if let Some(custom) = settings.0.get("AppKinds").and_then(Value::as_array) {
            for kind in custom {
                if let Some(name) = kind["Name"].as_str() {
                    names.push((name.to_owned(), name.to_owned()));
                }
            }
        }
    }
    egui::ScrollArea::horizontal().id_salt(key).show(ui, |ui| {
        egui::Grid::new(key).striped(true).show(ui, |ui| {
            ui.label(if kinds {
                "名前"
            } else {
                if cfg!(windows) {
                    "プロセス名 (例: Code.exe)"
                } else if cfg!(target_os = "macos") {
                    "アプリID (例: com.microsoft.VSCode)"
                } else {
                    "アプリ名 (例: code)"
                }
            });
            ui.label(if kinds { "最初は英数" } else { "有効" });
            ui.label(if kinds { "基本の種類" } else { "種類" });
            if kinds {
                ui.label("判定の強さ");
                ui.label("ライブ変換");
            }
            ui.end_row();
            for (index, row) in rows.iter_mut().enumerate() {
                if cfg!(target_os = "linux")
                    && !kinds
                    && row["Process"]
                        .as_str()
                        .is_some_and(|n| n.to_ascii_lowercase().ends_with(".exe"))
                {
                    continue;
                }
                ui.push_id(index, |ui| {
                    let mut name = row[if kinds { "Name" } else { "Process" }]
                        .as_str()
                        .unwrap_or("")
                        .to_owned();
                    if ui
                        .add(egui::TextEdit::singleline(&mut name).desired_width(170.))
                        .changed()
                    {
                        row[if kinds { "Name" } else { "Process" }] = json!(name);
                    }
                    let flag = if kinds { "StartInEnglish" } else { "Enabled" };
                    let mut enabled = row[flag].as_bool().unwrap_or(!kinds);
                    if ui.checkbox(&mut enabled, "").changed() {
                        row[flag] = json!(enabled);
                    }
                    let base = if kinds { "Base" } else { "Profile" };
                    let mut selected = row["Kind"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .or_else(|| row[base].as_str())
                        .unwrap_or("General")
                        .to_owned();
                    let prior = selected.clone();
                    let label = names
                        .iter()
                        .find(|n| n.0 == selected)
                        .map_or(selected.as_str(), |n| n.1.as_str());
                    egui::ComboBox::from_id_salt("kind")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (name, label) in &names {
                                ui.selectable_value(&mut selected, name.clone(), label);
                            }
                        });
                    if selected != prior {
                        if ["General", "Code", "Game"].contains(&selected.as_str()) {
                            row[base] = json!(selected);
                            row["Kind"] = Value::Null;
                        } else {
                            row["Kind"] = json!(selected);
                        }
                    }
                    if kinds {
                        nullable_choice(
                            ui,
                            row,
                            "DetectionLevel",
                            &[
                                ("Aggressive", "積極的"),
                                ("Balanced", "標準"),
                                ("Conservative", "慎重"),
                                ("Manual", "手動"),
                            ],
                        );
                        nullable_choice(
                            ui,
                            row,
                            "LiveConversion",
                            &[("true", "ON"), ("false", "OFF")],
                        );
                    }
                    if ui.button("削除").clicked() {
                        remove = Some(index);
                    }
                });
                ui.end_row();
            }
        });
    });
    if let Some(index) = remove {
        rows.remove(index);
    }
    if ui
        .button(if kinds {
            "種類を追加"
        } else {
            "アプリを追加"
        })
        .clicked()
    {
        rows.push(if kinds {
            json!({"Name":"新しい種類","Base":"General","StartInEnglish":false})
        } else {
            json!({"Process":if cfg!(windows) { "new.exe" } else if cfg!(target_os = "macos") { "com.example.app" } else { "new-app" },"Enabled":true,"Profile":"General"})
        });
    }
    *rows != old
}
fn nullable_choice(ui: &mut egui::Ui, row: &mut Value, key: &str, choices: &[(&str, &str)]) {
    let mut selected = if row[key].is_null() {
        String::new()
    } else if let Some(s) = row[key].as_str() {
        s.to_owned()
    } else {
        row[key].to_string()
    };
    let prior = selected.clone();
    let label = choices
        .iter()
        .find(|c| c.0 == selected)
        .map_or("全体と同じ", |c| c.1);
    egui::ComboBox::from_id_salt(key)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, String::new(), "全体と同じ");
            for (value, label) in choices {
                ui.selectable_value(&mut selected, (*value).to_owned(), *label);
            }
        });
    if selected != prior {
        row[key] = match selected.as_str() {
            "" => Value::Null,
            "true" => json!(true),
            "false" => json!(false),
            _ => json!(selected),
        };
    }
}
