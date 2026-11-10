// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! かな入力の配列。上流 v1.5.0 の KanaLayout.cs の表を使う。
//!
//! 月・花は前置シフト、NICOLA・中指NICOLA・TRON は親指シフト。
//! 共通Coreはキーアップを受け取らないため、いまは文字キーの後に親指キーを押す
//! 同時打鍵には未対応。親指キーを先に押してから文字キーを押す形で使う。
use crate::kana;
use crate::kana_layout_data;
use std::collections::HashMap;
use std::sync::OnceLock;

/// SKY配列: QWERTYの位置のキーを、ローマ字の綴りにする。
pub fn sky(vk: i32) -> Option<&'static str> {
    Some(match vk {
        0x51 => ",",
        0x57 => "w",
        0x45 => "r",
        0x52 => "m",
        0x54 => "h",
        0x59 => "uu",
        0x55 => "ai",
        0x49 => "ou",
        0x4F => ".",
        0x50 => "ei",
        0x41 => "n",
        0x53 => "t",
        0x44 => "s",
        0x46 => "k",
        0x47 => "y",
        0x48 => "u",
        0x4A => "a",
        0x4B => "o",
        0x4C => "i",
        0xBB => "e",
        0x5A => "p",
        0x58 => "d",
        0x43 => "z",
        0x56 => "g",
        0x42 => "b",
        0x4E => "unn",
        0x4D => "ann",
        0xBC => "onn",
        0xBE => "inn",
        0xBF => "enn",
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Jis,
    Tsuki,
    Hana,
    Nicola,
    NakayubiNicola,
    Tron,
}

impl Layout {
    pub fn parse(value: &str) -> Self {
        match value {
            "Tsuki" => Self::Tsuki,
            "Hana" => Self::Hana,
            "Nicola" => Self::Nicola,
            "NakayubiNicola" => Self::NakayubiNicola,
            "Tron" => Self::Tron,
            _ => Self::Jis,
        }
    }
    fn is_prefix(self) -> bool {
        matches!(self, Self::Tsuki | Self::Hana)
    }
    fn is_thumb(self) -> bool {
        matches!(self, Self::Nicola | Self::NakayubiNicola | Self::Tron)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stroke {
    pub consumed: bool,
    pub remove_last: usize,
    pub kana: String,
}

impl Stroke {
    fn consumed() -> Self {
        Self {
            consumed: true,
            ..Self::default()
        }
    }
    fn emit(kana: &str) -> Self {
        Self {
            consumed: true,
            kana: kana.to_owned(),
            ..Self::default()
        }
    }
}

const ROWS: [&[i32]; 4] = [
    &[
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0xBD, 0xDE, 0xDC,
    ],
    &[
        0x51, 0x57, 0x45, 0x52, 0x54, 0x59, 0x55, 0x49, 0x4F, 0x50, 0xC0, 0xDB,
    ],
    &[
        0x41, 0x53, 0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C, 0xBB, 0xBA, 0xDD,
    ],
    &[
        0x5A, 0x58, 0x43, 0x56, 0x42, 0x4E, 0x4D, 0xBC, 0xBE, 0xBF, 0xE2,
    ],
];

const LEFT_THUMB: i32 = 0x1D;
const RIGHT_THUMB: i32 = 0x1C;
const KEY_D: i32 = 0x44;
const KEY_K: i32 = 0x4B;

fn faces(layout: Layout) -> &'static [HashMap<i32, &'static str>] {
    static CACHE: OnceLock<HashMap<&'static str, Vec<HashMap<i32, &'static str>>>> =
        OnceLock::new();
    let all = CACHE.get_or_init(|| {
        let mut all = HashMap::new();
        for (name, table) in [
            ("Tsuki", kana_layout_data::TSUKI),
            ("Hana", kana_layout_data::HANA),
            ("Nicola", kana_layout_data::NICOLA),
            ("NakayubiNicola", kana_layout_data::NAKAYUBINICOLA),
            ("Tron", kana_layout_data::TRON),
        ] {
            let mut faces = Vec::new();
            for face in table {
                let mut map = HashMap::new();
                for (row, keys) in ROWS.iter().enumerate() {
                    let cells: Vec<_> = face[row].split(',').collect();
                    for (column, key) in keys.iter().enumerate() {
                        if let Some(cell) = cells.get(column).filter(|cell| !cell.is_empty()) {
                            map.insert(*key, *cell);
                        }
                    }
                }
                faces.push(map);
            }
            all.insert(name, faces);
        }
        all
    });
    let name = match layout {
        Layout::Tsuki => "Tsuki",
        Layout::Hana => "Hana",
        Layout::Nicola => "Nicola",
        Layout::NakayubiNicola => "NakayubiNicola",
        Layout::Tron => "Tron",
        Layout::Jis => "Tsuki",
    };
    all.get(name).map(Vec::as_slice).unwrap_or(&[])
}

fn is_left_hand(vk: i32) -> bool {
    ROWS.iter().any(|row| {
        row.iter()
            .position(|key| *key == vk)
            .is_some_and(|index| index < 5)
    })
}

#[derive(Default)]
pub struct Engine {
    layout: Layout,
    prefix: usize,
    thumb: i32,
    middle: i32,
    last_key: i32,
    last_time: u64,
    last_shifted: bool,
}

impl Engine {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            ..Self::default()
        }
    }

    /// 1 打鍵。None は配列のキーではない (通常の文字・記号として扱う)。
    pub fn down(&mut self, vk: i32, shift: bool, time: u64) -> Option<Stroke> {
        if self.layout == Layout::Jis {
            return kana::key(vk, shift).map(|kana| {
                let stroke = Stroke::emit(&kana.to_string());
                self.last_key = vk;
                self.last_time = time;
                self.last_shifted = false;
                stroke
            });
        }
        if self.layout.is_thumb() && matches!(vk, LEFT_THUMB | RIGHT_THUMB) {
            // 文字キーの直後 (100ms以内) に親指キーを押したら、その文字をシフト面で打ち直す。
            let face = if vk == LEFT_THUMB { 1 } else { 2 };
            if !self.last_shifted
                && self.last_key != 0
                && time.saturating_sub(self.last_time) <= 100
            {
                if let Some(shifted) = faces(self.layout)
                    .get(face)
                    .and_then(|face| face.get(&self.last_key))
                    .copied()
                {
                    self.thumb = vk;
                    self.last_shifted = true;
                    return Some(Stroke {
                        consumed: true,
                        remove_last: 1,
                        kana: shifted.to_owned(),
                    });
                }
            }
            if self.thumb != vk {
                self.thumb = vk;
            }
            return Some(Stroke::consumed());
        }
        if shift {
            return None;
        }
        let table = faces(self.layout);
        let plain = table.first().and_then(|face| face.get(&vk)).copied();
        if plain.is_none()
            && !(self.prefix > 0 && table.get(self.prefix).is_some_and(|f| f.contains_key(&vk)))
        {
            return None;
        }
        if self.layout.is_prefix() {
            if self.prefix > 0 {
                let face = &table[self.prefix];
                self.prefix = 0;
                return Some(match face.get(&vk) {
                    Some(kana) if !matches!(*kana, "1" | "2") => self.emit(vk, kana, true, time),
                    _ => Stroke::consumed(),
                });
            }
            if matches!(plain, Some("1" | "2")) {
                self.prefix = plain.unwrap().as_bytes()[0] as usize - b'0' as usize;
                return Some(Stroke::consumed());
            }
            return plain.map(|kana| self.emit(vk, kana, false, time));
        }
        if self.thumb != 0 {
            let face = if self.thumb == LEFT_THUMB { 1 } else { 2 };
            let kana = table.get(face).and_then(|face| face.get(&vk)).copied();
            self.thumb = 0;
            return Some(
                kana.map_or_else(Stroke::consumed, |kana| self.emit(vk, kana, true, time)),
            );
        }
        if self.layout == Layout::NakayubiNicola && vk == self.middle {
            return Some(Stroke::consumed());
        }
        if self.layout == Layout::NakayubiNicola && vk == KEY_D && self.middle == 0 {
            self.middle = KEY_D;
            return Some(Stroke::consumed());
        }
        if self.layout == Layout::NakayubiNicola && vk == KEY_K && self.middle == 0 {
            self.middle = KEY_K;
            return Some(Stroke::consumed());
        }
        if self.layout == Layout::NakayubiNicola
            && self.middle != 0
            && is_left_hand(vk) == (self.middle == KEY_K)
        {
            let face = if self.middle == KEY_K { 1 } else { 2 };
            self.middle = 0;
            if let Some(kana) = table.get(face).and_then(|face| face.get(&vk)) {
                return Some(self.emit(vk, kana, true, time));
            }
        }
        plain.map(|kana| self.emit(vk, kana, false, time))
    }

    fn emit(&mut self, vk: i32, kana: &str, shifted: bool, time: u64) -> Stroke {
        self.last_key = vk;
        self.last_time = time;
        self.last_shifted = shifted;
        Stroke::emit(kana)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_layout_maps_a_known_key() {
        assert_eq!(
            Engine::new(Layout::Jis).down(0x51, false, 0).unwrap().kana,
            "た"
        );
        assert_eq!(
            Engine::new(Layout::Tsuki)
                .down(0x51, false, 0)
                .unwrap()
                .kana,
            "ゅ"
        );
        assert_eq!(
            Engine::new(Layout::Hana).down(0x51, false, 0).unwrap().kana,
            "ょ"
        );
        assert_eq!(
            Engine::new(Layout::Nicola)
                .down(0x51, false, 0)
                .unwrap()
                .kana,
            "。"
        );
        assert_eq!(
            Engine::new(Layout::Tron).down(0x51, false, 0).unwrap().kana,
            "ら"
        );
    }

    #[test]
    fn tsuki_and_hana_use_prefix_shift() {
        let mut tsuki = Engine::new(Layout::Tsuki);
        assert!(tsuki.down(0x44, false, 0).unwrap().kana.is_empty());
        assert_eq!(tsuki.down(0x46, false, 0).unwrap().kana, "あ");
        let mut hana = Engine::new(Layout::Hana);
        assert!(hana.down(0x44, false, 0).unwrap().kana.is_empty());
        assert_eq!(hana.down(0x46, false, 0).unwrap().kana, "や");
    }

    #[test]
    fn nicola_uses_the_thumb_before_the_character() {
        let mut nicola = Engine::new(Layout::Nicola);
        assert!(nicola.down(LEFT_THUMB, false, 0).unwrap().kana.is_empty());
        assert_eq!(nicola.down(0x51, false, 0).unwrap().kana, "ぁ");
    }

    #[test]
    fn nicola_restrokes_a_character_when_the_thumb_follows_within_100ms() {
        let mut nicola = Engine::new(Layout::Nicola);
        assert_eq!(nicola.down(0x51, false, 0).unwrap().kana, "。");
        let stroke = nicola.down(LEFT_THUMB, false, 50).unwrap();
        assert_eq!(stroke.remove_last, 1);
        assert_eq!(stroke.kana, "ぁ");
    }
}
