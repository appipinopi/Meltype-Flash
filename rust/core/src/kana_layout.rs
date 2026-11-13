// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! かな入力の配列。上流 v1.5.0 の KanaLayout.cs の表を使う。
//!
//! 月・花は前置シフト、NICOLA・中指NICOLA・TRON は親指シフト。
//! NICOLA系は押下・解放順で同時打鍵を確定する（キーアップAPI対応）。
//! 親指キーのタップ（同時打鍵に使われなかった解放）は無変換/変換として扱える。
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

pub const LEFT_THUMB: i32 = 0x1D; // 無変換
pub const RIGHT_THUMB: i32 = 0x1C; // 変換
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

#[derive(Default, Clone)]
pub struct Engine {
    layout: Layout,
    prefix: usize,
    thumb: i32,
    thumb_used: bool,
    middle: i32,
    middle_used: bool,
    /// 文字キーが押されたまま（未解放）で、親指シフト面への打ち直しが可能な状態。
    pending_upgrade_key: Option<i32>,
}

impl Engine {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            ..Self::default()
        }
    }

    /// 1 打鍵。None は配列のキーではない (通常の文字・記号として扱う)。
    pub fn down(&mut self, vk: i32, shift: bool, _time: u64) -> Option<Stroke> {
        if self.layout == Layout::Jis {
            return kana::key(vk, shift).map(|kana| Stroke::emit(&kana.to_string()));
        }
        if self.layout.is_thumb() && matches!(vk, LEFT_THUMB | RIGHT_THUMB) {
            let face = if vk == LEFT_THUMB { 1 } else { 2 };
            // 押下・解放順: 文字キーがまだ解放されていなければシフト面で打ち直す。
            if let Some(pending) = self.pending_upgrade_key {
                if let Some(shifted) = faces(self.layout)
                    .get(face)
                    .and_then(|f| f.get(&pending))
                    .copied()
                {
                    self.thumb = vk;
                    self.thumb_used = true;
                    self.pending_upgrade_key = None;
                    return Some(Stroke {
                        consumed: true,
                        remove_last: 1,
                        kana: shifted.to_owned(),
                    });
                }
            }
            self.thumb = vk;
            self.thumb_used = false;
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
                    Some(kana) if !matches!(*kana, "1" | "2") => {
                        self.pending_upgrade_key = None;
                        Stroke::emit(kana)
                    }
                    _ => Stroke::consumed(),
                });
            }
            if matches!(plain, Some("1" | "2")) {
                self.prefix = plain.unwrap().as_bytes()[0] as usize - b'0' as usize;
                return Some(Stroke::consumed());
            }
            self.pending_upgrade_key = Some(vk);
            return plain.map(Stroke::emit);
        }
        if self.thumb != 0 {
            self.thumb_used = true;
            let face = if self.thumb == LEFT_THUMB { 1 } else { 2 };
            let kana = table.get(face).and_then(|face| face.get(&vk)).copied();
            self.thumb = 0;
            self.thumb_used = false;
            self.pending_upgrade_key = None;
            return Some(kana.map_or_else(Stroke::consumed, Stroke::emit));
        }
        if self.layout == Layout::NakayubiNicola {
            if vk == self.middle {
                return Some(Stroke::consumed());
            }
            if vk == KEY_D && self.middle == 0 {
                self.middle = KEY_D;
                self.middle_used = false;
                return Some(Stroke::consumed());
            }
            if vk == KEY_K && self.middle == 0 {
                self.middle = KEY_K;
                self.middle_used = false;
                return Some(Stroke::consumed());
            }
            if self.middle != 0 && is_left_hand(vk) == (self.middle == KEY_K) {
                let face = if self.middle == KEY_K { 1 } else { 2 };
                self.middle_used = true;
                let kana = table.get(face).and_then(|face| face.get(&vk)).copied();
                self.middle = 0;
                self.middle_used = false;
                self.pending_upgrade_key = None;
                if let Some(kana) = kana {
                    return Some(Stroke::emit(kana));
                }
                return Some(Stroke::consumed());
            }
        }
        self.pending_upgrade_key = Some(vk);
        plain.map(Stroke::emit)
    }

    /// キー解放。親指キーが同時打鍵に使われなかった場合、タップ(無変換/変換)として VK を返す。
    pub fn up(&mut self, vk: i32) -> Option<i32> {
        if Some(vk) == self.pending_upgrade_key {
            self.pending_upgrade_key = None;
        }
        if self.layout.is_thumb() && matches!(vk, LEFT_THUMB | RIGHT_THUMB) && self.thumb == vk {
            let tap = if self.thumb_used { None } else { Some(vk) };
            self.thumb = 0;
            self.thumb_used = false;
            return tap;
        }
        if self.layout == Layout::NakayubiNicola && matches!(vk, KEY_D | KEY_K) && self.middle == vk
        {
            self.middle = 0;
            self.middle_used = false;
        }
        None
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
    fn nicola_restrokes_a_character_when_the_thumb_follows_before_release() {
        let mut nicola = Engine::new(Layout::Nicola);
        assert_eq!(nicola.down(0x51, false, 0).unwrap().kana, "。");
        let stroke = nicola.down(LEFT_THUMB, false, 500).unwrap();
        assert_eq!(stroke.remove_last, 1);
        assert_eq!(stroke.kana, "ぁ");
        assert_eq!(nicola.up(LEFT_THUMB), None);
        nicola.up(0x51);
    }

    #[test]
    fn nicola_does_not_restroke_after_char_release() {
        let mut nicola = Engine::new(Layout::Nicola);
        assert_eq!(nicola.down(0x51, false, 0).unwrap().kana, "。");
        nicola.up(0x51);
        let stroke = nicola.down(LEFT_THUMB, false, 50).unwrap();
        assert!(stroke.kana.is_empty());
        assert_eq!(nicola.up(LEFT_THUMB), Some(LEFT_THUMB));
    }

    #[test]
    fn nicola_thumb_tap_detected_on_release() {
        let mut nicola = Engine::new(Layout::Nicola);
        assert!(nicola.down(RIGHT_THUMB, false, 0).unwrap().kana.is_empty());
        assert_eq!(nicola.up(RIGHT_THUMB), Some(RIGHT_THUMB));
        assert!(nicola.down(LEFT_THUMB, false, 0).unwrap().kana.is_empty());
        assert_eq!(nicola.up(LEFT_THUMB), Some(LEFT_THUMB));
    }
}
