// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
use serde_json::Value;
fn data() -> &'static Value {
    meltype_core::settings_schema::metadata()
}
pub fn fields() -> &'static [Value] {
    data()["fields"].as_array().unwrap()
}
pub fn field(key: &str) -> Option<&'static Value> {
    fields().iter().find(|f| f["key"] == key)
}
pub fn label(key: &str) -> &str {
    if key == "UseZenzai" && cfg!(target_os = "linux") {
        return crate::i18n::label(key, "Zenzaiを使う (Linux・実験)");
    }
    let fallback = field(key).and_then(|f| f["label"].as_str()).unwrap_or(key);
    crate::i18n::label(key, fallback)
}
pub fn description(key: &str) -> &str {
    if key == "UseZenzai" && cfg!(any(windows, target_os = "linux")) {
        return if crate::i18n::english() {
            "Opens a confirmation dialog. Say yes to download a CPU-only AI runtime and model separately; save the settings after it finishes. Normal conversion is used while starting or when the model fails."
        } else {
            "有効にすると確認画面を開きます。「はい」でAI専用のCPU実行環境とモデルを別にダウンロードします。完了後に設定を保存してください。起動中や応答失敗時は通常の変換を使います。"
        };
    }
    if key == "AutoUpdate" && cfg!(windows) {
        return if crate::i18n::english() {
            "Automatic checks for and downloads a new version at startup; install it from the update button. Manual checks only when the check button is pressed."
        } else {
            "自動は起動時に新しい版を確認してダウンロードし、導入は更新ボタンから行います。手動では確認ボタンを押したときだけ公開版を確認します。"
        };
    }
    if key == "AutoUpdate" && cfg!(target_os = "linux") {
        return if crate::i18n::english() {
            "Automatic checks daily even after closing settings (at desktop login without systemd). Package installation is approved in the system dialog."
        } else {
            "自動を選んで保存すると、設定画面を閉じても毎日確認します（systemd非対応ではデスクトップのログイン時）。新しいパッケージの導入はシステム画面で承認します。手動ではボタンで確認します。"
        };
    }
    if cfg!(target_os = "linux") && key == "ConversionEngine" {
        return if crate::i18n::english() {
            "Linux uses Mozc for kana-kanji conversion."
        } else {
            "LinuxではMozcでかな漢字変換します。"
        };
    }
    let fallback = field(key)
        .and_then(|f| f["description"].as_str())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| label(key));
    crate::i18n::description(key, fallback)
}
pub fn choice<'a>(key: &str, value: &'a str) -> &'a str {
    if cfg!(target_os = "linux") && key == "ConversionEngine" {
        return "Mozc";
    }
    let kind = field(key).and_then(|f| f["type"].as_str()).unwrap_or("");
    let fallback = data()["enums"][kind][value].as_str().unwrap_or(value);
    crate::i18n::choice(kind, value, fallback)
}
