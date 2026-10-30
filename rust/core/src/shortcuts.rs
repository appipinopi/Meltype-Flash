// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Shared configured shortcut parsing; modifier bits match the input ABI.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub vk: i32,
    pub modifiers: i32,
}

/// 1.4.0の入力モード切替設定。キーコードの表記は本家と共通にする。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleShortcut {
    #[serde(rename = "Key")]
    pub key: i32,
    #[serde(rename = "Modifiers", default)]
    #[serde(deserialize_with = "deserialize_toggle_modifiers")]
    pub modifiers: i32,
}
fn deserialize_toggle_modifiers<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    if let Some(number) = value.as_i64().and_then(|n| i32::try_from(n).ok()) {
        return Ok(number);
    }
    let text = value
        .as_str()
        .ok_or_else(|| serde::de::Error::custom("invalid shortcut modifiers"))?;
    let mut flags = 0;
    for name in text.split(',').map(str::trim) {
        flags |= match name.to_ascii_lowercase().as_str() {
            "none" => 0,
            "shift" => 1,
            "control" => 2,
            "alt" => 4,
            "win" => 8,
            _ => return Err(serde::de::Error::custom("unknown shortcut modifier")),
        };
    }
    Ok(flags)
}
impl ToggleShortcut {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                key: 0x19,
                modifiers: 0,
            },
            Self {
                key: 0xc0,
                modifiers: 4,
            },
        ]
    }
    pub fn normalize_key(key: i32) -> i32 {
        if matches!(key, 0xf3 | 0xf4) {
            0x19
        } else {
            key
        }
    }
    pub fn matches(&self, key: i32, modifiers: i32) -> bool {
        self.modifiers == modifiers && Self::normalize_key(self.key) == Self::normalize_key(key)
    }
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.replace("半角/全角", "Kanji").replace("英数", "Eisu");
        let parsed = parse(&text)?;
        Some(Self {
            key: Self::normalize_key(parsed.vk),
            modifiers: parsed.modifiers,
        })
    }
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        for (bit, name) in [(2, "Ctrl"), (4, "Alt"), (1, "Shift"), (8, "Win")] {
            if self.modifiers & bit != 0 {
                parts.push(name.to_owned());
            }
        }
        parts.push(match Self::normalize_key(self.key) {
            0x19 => "半角/全角".into(),
            0xc0 => "Backquote".into(),
            0x14 => "CapsLock".into(),
            0xf0 => "英数".into(),
            0x1c => "Henkan".into(),
            0x1d => "Muhenkan".into(),
            0x20 => "Space".into(),
            9 => "Tab".into(),
            13 => "Enter".into(),
            27 => "Esc".into(),
            0x30..=0x39 | 0x41..=0x5a => char::from_u32(self.key as u32).unwrap().to_string(),
            0x70..=0x7b => format!("F{}", self.key - 0x6f),
            key => format!("VK{key:02X}"),
        });
        parts.join("+")
    }
    pub fn validate(
        &self,
        settings: &crate::settings::Settings,
        others: &[Self],
    ) -> Result<(), &'static str> {
        let key = Self::normalize_key(self.key);
        if !(1..256).contains(&key)
            || matches!(key, 0xe5 | 0xe7 | 0xe8 | 0xff | 0x10..=0x12 | 0xa0..=0xa5 | 0x5b | 0x5c)
            || self.modifiers & !15 != 0
            || key == 0x19 && self.modifiers & 2 != 0
        {
            return Err("このキーは切替に登録できません");
        }
        if others
            .iter()
            .any(|other| other.matches(key, self.modifiers))
        {
            return Err("同じ切替キーが登録されています");
        }
        if self.modifiers & 14 == 2 {
            if control_bindings(settings.text("ControlKeyBindings"))
                .iter()
                .any(|(letter, _)| *letter == key)
            {
                return Err("Controlキーの割り当てと競合しています");
            }
            if matches!(key, 0x48 | 0x55 | 0x49 | 0x4f | 0x50) {
                return Err("変換操作で使うControlキーです");
            }
        }
        if (0x75..=0x79).contains(&key) && self.modifiers & !1 == 0 {
            return Err("F6〜F10は字種変換に使います");
        }
        if self.modifiers & !1 == 0 && !matches!(key, 0x19 | 0x1c | 0x1d) {
            return Err("このキーにはCtrl・Alt・Winのいずれかが必要です");
        }
        if self.modifiers == 0
            && settings.boolean("ConvertKeysSwitchKeyboard")
            && matches!(key, 0x1c | 0x1d)
        {
            return Err("無変換・変換キーのモード切替と競合しています");
        }
        Ok(())
    }
}

/// フックが引き取った物理キーの繰り返しと解放を追跡する。
#[derive(Default)]
pub struct ToggleDetector {
    held: std::collections::HashSet<u32>,
    consumed: std::collections::HashSet<u32>,
    jis_modifiers: std::collections::HashMap<u32, i32>,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ToggleResult {
    pub consume: bool,
    pub toggle: bool,
}
impl ToggleDetector {
    fn physical(key: i32, scan: u32, extended: bool) -> u32 {
        if scan == 0 {
            0x20000 | ToggleShortcut::normalize_key(key) as u32
        } else {
            scan | (u32::from(extended) << 16)
        }
    }
    pub fn release(&mut self, key: i32, scan: u32, extended: bool) -> bool {
        let modifier = match key {
            0x10 | 0xa0 | 0xa1 => 1,
            0x11 | 0xa2 | 0xa3 => 2,
            0x12 | 0xa4 | 0xa5 => 4,
            0x5b | 0x5c => 8,
            _ => 0,
        };
        if modifier != 0 {
            for (&physical, &modifiers) in &self.jis_modifiers {
                if modifiers & modifier != 0 {
                    self.held.remove(&physical);
                }
            }
        }
        let physical = Self::physical(key, scan, extended);
        self.jis_modifiers.remove(&physical);
        self.held.remove(&physical);
        self.consumed.remove(&physical)
    }
    pub fn press(
        &mut self,
        key: i32,
        scan: u32,
        extended: bool,
        unmodified: i32,
        modifiers: i32,
        bindings: &[ToggleShortcut],
    ) -> ToggleResult {
        let physical = Self::physical(key, scan, extended);
        let first = self.held.insert(physical);
        if first && self.jis_modifiers.remove(&physical).is_some() {
            self.consumed.remove(&physical);
        }
        if self.consumed.contains(&physical) {
            return ToggleResult {
                consume: true,
                toggle: false,
            };
        }
        if !first {
            return ToggleResult::default();
        }
        // US配列のAlt+`もVK_KANJIになるため、配列上のキーへ戻す。
        let shortcut_key = if scan == 0x29
            && !extended
            && unmodified == 0xc0
            && ToggleShortcut::normalize_key(key) == 0x19
        {
            0xc0
        } else {
            ToggleShortcut::normalize_key(key)
        };
        if shortcut_key == 0x19 && key == 0x19 && scan == 0x29 && !extended && modifiers != 0 {
            self.jis_modifiers.insert(physical, modifiers);
        }
        if !bindings
            .iter()
            .any(|binding| binding.matches(shortcut_key, modifiers))
        {
            return ToggleResult::default();
        }
        self.consumed.insert(physical);
        ToggleResult {
            consume: true,
            toggle: true,
        }
    }
}
pub fn key_name(token: &str) -> Option<i32> {
    let token = token.to_ascii_lowercase();
    Some(match token.as_str() {
        "space" => 0x20,
        "tab" => 9,
        "enter" => 13,
        "esc" => 27,
        "backquote" | "grave" => 0xc0,
        "muhenkan" => 0x1d,
        "henkan" => 0x1c,
        "kanji" => 0x19,
        "capslock" => 0x14,
        "eisu" | "alphanumeric" => 0xf0,
        _ if token.len() > 2 && token.starts_with("vk") => i32::from_str_radix(&token[2..], 16)
            .ok()
            .filter(|key| (1..256).contains(key))?,
        _ if token.len() == 1 && token.bytes().all(|b| b.is_ascii_alphanumeric()) => {
            token.as_bytes()[0].to_ascii_uppercase() as i32
        }
        _ if (2..=3).contains(&token.len()) && token.starts_with('f') => {
            let n = token[1..].parse::<i32>().ok()?;
            if !(1..=12).contains(&n) {
                return None;
            }
            0x70 + n - 1
        }
        _ => return None,
    })
}
pub fn parse(text: &str) -> Option<Shortcut> {
    let mut modifiers = 0;
    let mut vk = None;
    for token in text.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= 2,
            "alt" => modifiers |= 4,
            "shift" => modifiers |= 1,
            "win" | "windows" => modifiers |= 8,
            _ => {
                if vk.is_some() {
                    return None;
                }
                vk = Some(key_name(token)?);
            }
        }
    }
    Some(Shortcut { vk: vk?, modifiers })
}
pub fn parse_list(text: &str) -> Vec<Shortcut> {
    text.split([';', ',', '\n']).filter_map(parse).collect()
}
/// User Control mappings take priority over built-in assignments.
pub fn control_bindings(text: &str) -> Vec<(i32, i32)> {
    let mut result = vec![];
    for part in text.split([';', ',', '\n']) {
        let Some((letter, key)) = part.split_once('=') else {
            continue;
        };
        let letter = letter.trim();
        if letter.len() != 1 || !letter.as_bytes()[0].is_ascii_alphabetic() {
            continue;
        }
        let key = match key.trim().to_ascii_lowercase().as_str() {
            "left" => 0x25,
            "right" => 0x27,
            "up" => 0x26,
            "down" => 0x28,
            "home" => 0x24,
            "end" => 0x23,
            "delete" => 0x2e,
            "back" => 8,
            key => match key_name(key) {
                Some(v) if matches!(v, 9 | 13 | 27 | 0x75..=0x79) => v,
                _ => continue,
            },
        };
        let letter = letter.as_bytes()[0].to_ascii_uppercase() as i32;
        if let Some((_, old)) = result.iter_mut().find(|(k, _)| *k == letter) {
            *old = key;
        } else {
            result.push((letter, key));
        }
    }
    result
}

/// The exact two-character pair set used by CompositionController 1.3.0.
pub fn is_bracket_pair(text: &str) -> bool {
    [
        "「」", "『』", "（）", "()", "［］", "[]", "｛｝", "{}", "【】", "〈〉", "《》", "〔〕",
        "\"\"", "“”",
    ]
    .contains(&text)
}
