// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 配布リビジョンとハッシュを固定した任意導入モデルの一覧。
use crate::{ai_model::AiModel, settings::Settings};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
pub const JSON: &str = include_str!("../../ai/model-catalog.json");
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelFile {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub id: String,
    pub family: String,
    pub label: String,
    pub directory: String,
    pub protocol: String,
    pub license: String,
    pub source: String,
    pub available: bool,
    pub files: Vec<ModelFile>,
}
pub fn models() -> &'static [Model] {
    #[derive(Deserialize)]
    struct Catalog {
        models: Vec<Model>,
    }
    static DATA: OnceLock<Vec<Model>> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str::<Catalog>(JSON)
            .expect("model catalog")
            .models
    })
}
pub fn find(id: &str) -> Option<&'static Model> {
    models().iter().find(|m| m.id == id)
}
pub fn key(family: AiModel) -> &'static str {
    match family {
        AiModel::Zenz => "ZenzModelVariant",
        AiModel::Jinen => "JinenModelVariant",
        AiModel::Kanakanji => "KanakanjiModelVariant",
        AiModel::Shunti => "ShuntiModelVariant",
        AiModel::Gamma => "GammaModelVariant",
        AiModel::Qwen => "QwenModelVariant",
        AiModel::Shion => "ShionModelVariant",
        AiModel::None => "",
    }
}
pub fn selected(settings: &Settings, family: AiModel) -> Option<&'static Model> {
    let default = match family {
        AiModel::Zenz => "zenz-v3.1-small-q5_k_m",
        AiModel::Jinen => "jinen-v2-small-q5_k_m",
        AiModel::Kanakanji => "kanakanji-edge-33m-q8_0",
        AiModel::Shunti => "shunti-v7-s8p",
        AiModel::Gamma => "gamma-3-1b-q4_k_m",
        AiModel::Qwen => "qwen2.5-0.5b-instruct-q4_k_m",
        AiModel::Shion => "shion-qwen2.5-0.5b-q4_k_m",
        AiModel::None => return None,
    };
    find(settings.text(key(family)))
        .filter(|m| m.family == family.value())
        .or_else(|| find(default))
}
/// 各世代で異なる制御文字の順序と Jinen の正規化をまとめて扱う。
pub fn prompt(protocol: &str, reading: &str, context: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let kana = crate::romaji::katakana(reading);
    match protocol {
        "zenz-v1" => format!("\u{ee00}{kana}\u{ee01}"),
        "zenz-v2" => format!("\u{ee00}{kana}\u{ee02}{context}\u{ee01}"),
        "jinen" => format!("\u{ee02}{context}\u{ee00}{kana}\u{ee01}")
            .nfkc()
            .collect(),
        _ => format!("\u{ee02}{context}\u{ee00}{kana}\u{ee01}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_unique_pinned_variants_and_valid_defaults() {
        let mut ids = std::collections::HashSet::new();
        for m in models() {
            assert!(ids.insert(&m.id));
            assert!(m
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c)));
            assert!(!m.available || !m.license.is_empty());
            for f in &m.files {
                assert_eq!(f.sha256.len(), 64);
                assert!(f.sha256.bytes().all(|c| c.is_ascii_hexdigit()));
                assert!(f.url.starts_with("https://"));
                assert!(f.size > 0);
                assert!(matches!(
                    f.name.as_str(),
                    "model.gguf" | "kkc_lex.bin" | "kkc_model.bin"
                ));
            }
        }
        for family in AiModel::ALL.into_iter().filter(|m| *m != AiModel::None) {
            assert_eq!(
                selected(&Settings::default(), family).unwrap().family,
                family.value()
            );
        }
    }
    #[test]
    fn prompt_uses_each_generation_and_nfkc() {
        assert_eq!(
            prompt("zenz-v1", "きょう", "昨日"),
            "\u{ee00}キョウ\u{ee01}"
        );
        assert_eq!(
            prompt("zenz-v2", "きょう", "昨日"),
            "\u{ee00}キョウ\u{ee02}昨日\u{ee01}"
        );
        assert_eq!(
            prompt("jinen", "きょう", "ＡＢＣ①"),
            "\u{ee02}ABC1\u{ee00}キョウ\u{ee01}"
        );
        assert_eq!(
            prompt("zenz-v3", "きょう", ""),
            "\u{ee02}\u{ee00}キョウ\u{ee01}"
        );
    }
    #[test]
    fn versions_survive_switching_and_cannot_cross_families() {
        let mut settings = Settings::default();
        settings
            .set(key(AiModel::Zenz), serde_json::json!("zenz-v1-q8_0"))
            .unwrap();
        settings
            .set(
                key(AiModel::Jinen),
                serde_json::json!("jinen-v2-xsmall-q8_0"),
            )
            .unwrap();
        AiModel::Jinen.select(&mut settings, true);
        assert!(settings.boolean("UseZenzai") && !settings.boolean("UseShunti"));
        AiModel::Zenz.select(&mut settings, false);
        assert_eq!(
            selected(&settings, AiModel::Zenz).unwrap().id,
            "zenz-v1-q8_0"
        );
        assert_eq!(
            selected(&settings, AiModel::Jinen).unwrap().id,
            "jinen-v2-xsmall-q8_0"
        );
        assert!(settings
            .set(
                key(AiModel::Zenz),
                serde_json::json!("jinen-v2-xsmall-q8_0")
            )
            .is_err());
        AiModel::Kanakanji.select(&mut settings, true);
        settings.remove_neural_settings();
        assert_eq!(AiModel::selected(&settings), AiModel::None);
    }
}
