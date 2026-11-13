// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 既存の生成メタデータとFlash追加設定から、UIとJSON Schemaを組み立てる。
use crate::settings::Settings;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub const OPTIONAL_BOOLEANS: &[&str] = &[
    "DisableZShortcuts",
    "NumpadDirect",
    "MuhenkanKatakanaCycle",
    "SpaceAroundNumbers",
    "SkkMode",
    "AsyncConversion",
];

pub fn metadata() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data: Value = serde_json::from_str(
            &include_str!("../../settings-app/settings-ui.json")
                .replace("Meltype", "Meltype-FLASH"),
        )
        .expect("settings UI metadata");
        for (key, label, description) in [
            ("SpaceAroundNumbers", "日本語と半角数字の間に空白を入れる", "確定時に日本語と半角数字の境界へ半角空白を追加します。英数字の識別子・URL・ファイル名は対象外です。既定はオフです。"),
            ("DisableZShortcuts", "z記号変換を無効にする", "z.・z/・zhなどによる記号入力を無効にします。既定はオフです。"),
            ("NumpadDirect", "テンキーを直接確定する", "テンキーの数字と演算記号を直接確定します。入力中の文は先に確定します。Windows用。既定はオフです。"),
            ("MuhenkanKatakanaCycle", "無変換でカタカナを切り替える", "入力中の無変換キーで全角カタカナ→半角カタカナ→ひらがなを切り替えます。キーの役割を分ける設定より優先します。既定はオフです。"),
        ] {
            let fields = data["fields"].as_array_mut().unwrap();
            // 本家の設定へ追加された項目は、生成済みの表示定義を使う。
            if !fields.iter().any(|field| field["key"] == key) {
                fields.push(json!({
                    "key":key, "type":"Boolean", "category":"1. 全般",
                    "label":label, "description":description,
                }));
            }
        }
        data["enums"]["AiModel"] = json!({"None":"使用しない","Zenz":"Zenz","Shunti":"shuntelligence","Jinen":"Jinen","Kanakanji":"kanakanji-edge","Gamma":"Gamma","Qwen":"Qwen","Shion":"Shion"});
        data["fields"].as_array_mut().unwrap().push(json!({"key":"AiModel","type":"AiModel","category":"1a. AI","label":"AIモデル","description":"使用しない・Zenz・Jinen・kanakanji-edge・shuntelligence・Gamma・Qwen・Shionから選択します。導入とパスは選択したモデルだけ表示します。"}));
        data
    })
}

pub fn schema() -> Value {
    let data = metadata();
    let mut defaults = Settings::default().0;
    for key in OPTIONAL_BOOLEANS {
        defaults.insert((*key).into(), json!(false));
    }
    let mut properties = serde_json::Map::new();
    for (key, default) in defaults {
        let typ = match &default {
            Value::Bool(_) => "boolean",
            Value::Number(_) => "integer",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Null => continue,
        };
        let mut field = json!({"type":typ,"default":default});
        if let Some(meta) = data["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["key"] == key)
        {
            field["title"] = meta["label"].clone();
            field["description"] = meta["description"].clone();
            if let Some(choices) = meta["type"]
                .as_str()
                .and_then(|t| data["enums"][t].as_object())
            {
                field["enum"] = json!(choices.keys().collect::<Vec<_>>());
            }
        }
        properties.insert(key, field);
    }
    properties.insert(
        "AiModel".into(),
        json!({"type":"string","enum":["None","Zenz","Shunti","Jinen","Kanakanji","Gamma","Qwen","Shion"],"title":"AIモデル"}),
    );
    properties.insert(
        "UseShunti".into(),
        json!({"type":"boolean","default":false}),
    );
    properties.insert("ShuntiHelperPath".into(), json!({"type":"string"}));
    properties.insert("$schema".into(), json!({"type":"string"}));
    for family in crate::ai_model::AiModel::ALL
        .into_iter()
        .filter(|m| *m != crate::ai_model::AiModel::None)
    {
        properties.insert(crate::ai_catalog::key(family).into(), json!({"type":"string", "enum":crate::ai_catalog::models().iter().filter(|m| m.family == family.value()).map(|m| &m.id).collect::<Vec<_>>()}));
    }
    json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "title":"Meltype-FLASH config.json",
        "type":"object", "properties":properties, "additionalProperties":false,
    })
}
