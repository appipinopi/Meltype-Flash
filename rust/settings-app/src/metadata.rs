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
    if key == "UseZenzai" && cfg!(target_os = "linux") {
        return "別途ダウンロードしたllama-serverとZenzaiモデルをローカルで使用します。未導入・起動中・応答失敗の場合はMozcで変換します。";
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
