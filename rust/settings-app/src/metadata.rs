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
    field(key).and_then(|f| f["label"].as_str()).unwrap_or(key)
}
pub fn description(key: &str) -> &str {
    field(key)
        .and_then(|f| f["description"].as_str())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| label(key))
}
pub fn choice<'a>(key: &str, value: &'a str) -> &'a str {
    field(key)
        .and_then(|f| f["type"].as_str())
        .and_then(|typ| data()["enums"][typ][value].as_str())
        .unwrap_or(value)
}
