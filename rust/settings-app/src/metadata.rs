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
        return "Zenzaiを使う (Linux・実験)";
    }
    field(key).and_then(|f| f["label"].as_str()).unwrap_or(key)
}
pub fn description(key: &str) -> &str {
    if key == "UseZenzai" && cfg!(any(windows, target_os = "linux")) {
        return "有効にすると確認画面を開きます。「はい」でAI専用のCPU実行環境とモデルを別にダウンロードします。完了後に設定を保存してください。起動中や応答失敗時は通常の変換を使います。";
    }
    if key == "AutoUpdate" && cfg!(windows) {
        return "自動は起動時に新しい版を確認してダウンロードし、導入は更新ボタンから行います。手動では確認ボタンを押したときだけ公開版を確認します。";
    }
    if key == "AutoUpdate" && cfg!(target_os = "linux") {
        return "自動を選ぶと起動時に更新を確認し、新しいパッケージを取得してインストール画面を開きます。手動ではボタンを押した時だけ確認します。導入時の確認はシステムで行います。";
    }
    if cfg!(target_os = "linux") && key == "ConversionEngine" {
        return "LinuxではMozcでかな漢字変換します。";
    }
    field(key)
        .and_then(|f| f["description"].as_str())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| label(key))
}
pub fn choice<'a>(key: &str, value: &'a str) -> &'a str {
    if cfg!(target_os = "linux") && key == "ConversionEngine" {
        return "Mozc";
    }
    field(key)
        .and_then(|f| f["type"].as_str())
        .and_then(|typ| data()["enums"][typ][value].as_str())
        .unwrap_or(value)
}
