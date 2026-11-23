// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Android InputType/EditorInfo and KeyEvent policy. Constants are the public
//! Android SDK values; no Android/JVM objects are retained by the engine.
pub(super) fn field_flags(input_type: i32, ime_options: i32) -> i32 {
    let class = input_type & 0xf;
    let variation = input_type & 0xff0;
    let password =
        class == 1 && matches!(variation, 0x80 | 0x90 | 0xe0) || class == 2 && variation == 0x10;
    let private = password || ime_options & 0x0100_0000 != 0;
    let direct = password || matches!(class, 2..=4);
    i32::from(direct) | (i32::from(private) << 1)
}
pub(super) fn modifier_key(key: i32) -> bool {
    // KeyEvent.isModifierKey excludes Caps/Num/Scroll Lock.
    matches!(key, 57..=60 | 63 | 78 | 113 | 114 | 117..=119)
}
pub(super) fn virtual_key(key: i32) -> i32 {
    match key {
        67 => 8,
        66 => 13,
        62 => 32,
        111 => 27,
        19 => 38,
        20 => 40,
        21 => 37,
        22 => 39,
        61 => 9,
        136..=140 => 0x75 + key - 136,
        69 => 0xbd,
        70 => 0xbb,
        71 => 0xdb,
        72 => 0xdd,
        73 => 0xdc,
        74 => 0xba,
        75 => 0xde,
        68 => 0xc0,
        55 => 0xbc,
        56 => 0xbe,
        76 => 0xbf,
        29..=54 => 0x41 + key - 29,
        7..=16 => 0x30 + key - 7,
        _ => 0,
    }
}
pub(super) fn modifiers(meta: i32) -> i32 {
    i32::from(meta & 1 != 0)
        | (i32::from(meta & 0x1000 != 0) << 1)
        | (i32::from(meta & 2 != 0) << 2)
        | (i32::from(meta & 0x10000 != 0) << 3)
}
pub(super) fn kana_variant(c: char) -> Option<char> {
    [
        "かが",
        "きぎ",
        "くぐ",
        "けげ",
        "こご",
        "さざ",
        "しじ",
        "すず",
        "せぜ",
        "そぞ",
        "ただ",
        "ちぢ",
        "つっづ",
        "てで",
        "とど",
        "はばぱ",
        "ひびぴ",
        "ふぶぷ",
        "へべぺ",
        "ほぼぽ",
        "あぁ",
        "いぃ",
        "うぅゔ",
        "えぇ",
        "おぉ",
        "やゃ",
        "ゆゅ",
        "よょ",
        "わゎ",
    ]
    .iter()
    .find_map(|group| {
        let chars: Vec<_> = group.chars().collect();
        chars
            .iter()
            .position(|candidate| *candidate == c)
            .map(|index| chars[(index + 1) % chars.len()])
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flick_variants_cycle_voicing_and_small_kana() {
        for (a, b) in [
            ('か', 'が'),
            ('が', 'か'),
            ('は', 'ば'),
            ('ば', 'ぱ'),
            ('ぱ', 'は'),
            ('つ', 'っ'),
            ('っ', 'づ'),
            ('づ', 'つ'),
            ('う', 'ぅ'),
            ('ぅ', 'ゔ'),
            ('ゔ', 'う'),
            ('や', 'ゃ'),
            ('ゃ', 'や'),
        ] {
            assert_eq!(kana_variant(a), Some(b));
        }
        assert_eq!(kana_variant('ん'), None);
    }
    #[test]
    fn privacy_and_direct_are_independent_and_class_scoped() {
        for class in 0..16 {
            for variation in (0..=0xff0).step_by(0x10) {
                let field = class | variation;
                let password = class == 1 && [0x80, 0x90, 0xe0].contains(&variation)
                    || class == 2 && variation == 0x10;
                let expected =
                    i32::from(password || [2, 3, 4].contains(&class)) | (i32::from(password) << 1);
                assert_eq!(field_flags(field, 0), expected);
                assert_eq!(field_flags(field | 0x80000, 0x0100_0000), expected | 2);
            }
        }
    }
    #[test]
    fn caps_lock_is_not_a_modifier_event_or_engine_modifier() {
        for key in [115, 116, 143] {
            assert!(!modifier_key(key));
        }
        assert_eq!(modifiers(0x0010_0000 | 0x0020_0000 | 0x0040_0000), 0);
        assert_eq!(modifiers(1 | 0x1000 | 2 | 0x10000), 15);
    }
}
