// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! AIの選択と実行許可を分ける。選択だけでは未導入のモデルを起動しない。
use crate::settings::Settings;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiModel {
    None,
    Zenz,
    Shunti,
    Jinen,
    Kanakanji,
}
impl AiModel {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Zenz,
        Self::Jinen,
        Self::Kanakanji,
        Self::Shunti,
    ];
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Zenz" => Some(Self::Zenz),
            "Shunti" => Some(Self::Shunti),
            "Jinen" => Some(Self::Jinen),
            "Kanakanji" => Some(Self::Kanakanji),
            _ => None,
        }
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Zenz => "Zenz",
            Self::Shunti => "Shunti",
            Self::Jinen => "Jinen",
            Self::Kanakanji => "Kanakanji",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "使用しない",
            Self::Zenz => "Zenz",
            Self::Shunti => "shuntelligence",
            Self::Jinen => "Jinen",
            Self::Kanakanji => "kanakanji-edge",
        }
    }
    pub fn is_gguf(self) -> bool {
        matches!(self, Self::Zenz | Self::Jinen | Self::Kanakanji)
    }
    pub fn selected(settings: &Settings) -> Self {
        Self::parse(settings.text("AiModel")).unwrap_or_else(|| {
            if settings.boolean("UseShunti") {
                Self::Shunti
            } else if settings.boolean("UseZenzai") {
                Self::Zenz
            } else {
                Self::None
            }
        })
    }
    /// 「はい」で環境を作るまで実行許可は与えない。パス・既存環境は保持する。
    pub fn select(self, settings: &mut Settings, ready: bool) {
        settings.set("AiModel", json!(self.value())).unwrap();
        settings
            .set("UseZenzai", json!(ready && self.is_gguf()))
            .unwrap();
        settings
            .set("UseShunti", json!(ready && self == Self::Shunti))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_preserves_paths_and_requires_verified_environment() {
        let mut s = Settings::default();
        s.set("ZenzaiModelPath", json!("model.gguf")).unwrap();
        s.set("ShuntiHelperPath", json!("helper")).unwrap();
        AiModel::Shunti.select(&mut s, false);
        assert_eq!(AiModel::selected(&s), AiModel::Shunti);
        assert!(!s.boolean("UseShunti") && !s.boolean("UseZenzai"));
        AiModel::Shunti.select(&mut s, true);
        assert!(s.boolean("UseShunti"));
        AiModel::Zenz.select(&mut s, true);
        assert!(s.boolean("UseZenzai") && !s.boolean("UseShunti"));
        AiModel::None.select(&mut s, true);
        assert!(!s.boolean("UseShunti") && !s.boolean("UseZenzai"));
        assert_eq!(s.text("ZenzaiModelPath"), "model.gguf");
        assert_eq!(s.text("ShuntiHelperPath"), "helper");
        assert!(s.set("AiModel", json!("unknown")).is_err());
    }
    #[test]
    fn existing_enabled_flags_are_read_without_requiring_a_new_setting() {
        let mut s = Settings::default();
        s.set("UseZenzai", json!(true)).unwrap();
        assert_eq!(AiModel::selected(&s), AiModel::Zenz);
        s.set("UseShunti", json!(true)).unwrap();
        assert_eq!(AiModel::selected(&s), AiModel::Shunti);
    }
}
