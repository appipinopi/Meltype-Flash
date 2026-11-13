// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! PackedWordSet / CommonWordCost ported from upstream PackedWordSet.cs and
//! RomajiTypoCorrector.cs.
use std::sync::OnceLock;

/// FNV-1a over UTF-16 code units, matching upstream `HashIndex.Of`.
fn hash_of(text: &str) -> i32 {
    let mut hash = 2_166_136_261u32;
    for unit in text.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    (hash ^ (hash >> 15)) as i32
}

/// Open-addressed table of word numbers + 1. Matches upstream `HashIndex<T>`
/// (power-of-two slots, linear probing, empty slot ends the probe).
#[derive(Debug)]
struct HashIndex {
    slots: Vec<u32>,
}

impl HashIndex {
    fn new(count: usize) -> Self {
        let wanted = count + count / 3 + 1;
        let size = wanted.max(4).next_power_of_two();
        Self {
            slots: vec![0; size],
        }
    }

    fn add(&mut self, hash: i32, value: u32) {
        let mask = self.slots.len() - 1;
        let mut slot = (hash as u32 as usize) & mask;
        while self.slots[slot] != 0 {
            slot = (slot + 1) & mask;
        }
        self.slots[slot] = value;
    }

    fn candidates(&self, hash: i32) -> Candidates<'_> {
        let mask = self.slots.len() - 1;
        Candidates {
            slots: &self.slots,
            mask,
            slot: (hash as u32 as usize).wrapping_sub(1) & mask,
        }
    }
}

struct Candidates<'a> {
    slots: &'a [u32],
    mask: usize,
    slot: usize,
}

impl Iterator for Candidates<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        loop {
            self.slot = (self.slot + 1) & self.mask;
            let value = self.slots[self.slot];
            if value == 0 {
                return None;
            }
            return Some(value - 1);
        }
    }
}

/// Sorted words packed into one string plus start offsets. No per-word String
/// allocation; `contains` uses the upstream hash index and `has_prefix` uses
/// ordinal binary search.
#[derive(Debug)]
pub(crate) struct PackedWordSet {
    chars: String,
    starts: Vec<usize>,
    index: HashIndex,
}

impl PackedWordSet {
    pub(crate) fn from_lines(text: &str, first_field: bool) -> Self {
        let mut words: Vec<String> = text
            .lines()
            .filter_map(|line| {
                let line = if first_field {
                    line.split('\t').next().unwrap_or("")
                } else {
                    line
                };
                let trimmed = line.trim();
                (!trimmed.is_empty() && !trimmed.starts_with('#')).then(|| trimmed.to_owned())
            })
            .collect();
        words.sort_unstable();
        words.dedup();
        Self::from_sorted(words)
    }

    pub(crate) fn from_words(mut words: Vec<String>) -> Self {
        words.sort_unstable();
        words.dedup();
        Self::from_sorted(words)
    }

    fn from_sorted(words: Vec<String>) -> Self {
        let total: usize = words.iter().map(String::len).sum();
        let mut chars = String::with_capacity(total);
        let mut starts = Vec::with_capacity(words.len() + 1);
        let mut index = HashIndex::new(words.len());
        starts.push(0);
        for (position, word) in words.iter().enumerate() {
            index.add(hash_of(word), (position + 1) as u32);
            chars.push_str(word);
            starts.push(chars.len());
        }
        Self {
            chars,
            starts,
            index,
        }
    }

    fn word(&self, index: usize) -> &str {
        &self.chars[self.starts[index]..self.starts[index + 1]]
    }

    pub(crate) fn len(&self) -> usize {
        self.starts.len().saturating_sub(1)
    }

    pub(crate) fn contains(&self, value: &str) -> bool {
        for candidate in self.index.candidates(hash_of(value)) {
            if self.word(candidate as usize) == value {
                return true;
            }
        }
        false
    }

    fn lower_bound(&self, value: &str) -> usize {
        let mut low = 0;
        let mut high = self.len();
        while low < high {
            let middle = (low + high) / 2;
            if self.word(middle) < value {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    pub(crate) fn has_prefix(&self, prefix: &str) -> bool {
        let index = self.lower_bound(prefix);
        index < self.len() && self.word(index).starts_with(prefix)
    }
}

const FUNCTION_WORDS: &[&str] = &[
    "は", "が", "を", "に", "で", "と", "も", "へ", "の", "や", "か", "ね", "よ", "な", "わ",
    "さ", "ぞ", "ぜ", "だ", "です", "でし", "でしょ", "ます", "まし", "ませ", "ましょ", "た",
    "て", "で", "だっ", "ない", "なかっ", "なく", "なけれ", "たい", "たく", "たかっ", "れる",
    "れ", "られ", "られる", "せる", "せ", "させ", "させる", "ば", "う", "よう", "ん", "し",
    "する", "さ", "しろ", "すれ", "して", "した", "しない", "こと", "もの", "ので", "から",
    "まで", "より", "けど", "けれど", "ながら", "って", "という", "でも", "ください", "くださ",
    "ちゃ", "じゃ", "じゃない", "ちゃう", "いる", "い", "いた", "いて", "ある", "あり", "あっ",
    "おり", "おる", "ござい", "ございます", "だけ", "しか", "ほど", "くらい", "ぐらい", "など",
    "なら", "たら", "だら", "たり", "だり", "ずつ", "そう", "らしい", "みたい", "ほう", "とき",
    "ところ", "ため", "はず", "わけ", "ました", "ません", "ましょう", "ませんでした", "でした",
    "でしょう", "ている", "ていた", "てい", "てる", "てた", "ちゃった", "なかった", "たかった",
    "ないで", "なくて", "られた", "れた", "ください", "でしょうか", "ですか", "ますか", "ですね",
    "ますね", "ですよ", "ますよ",
];

const ROWS: &[(char, &str)] = &[
    ('う', "わいえお"),
    ('く', "かきけこ"),
    ('ぐ', "がぎげご"),
    ('す', "さしせそ"),
    ('つ', "たちてと"),
    ('ぬ', "なにねの"),
    ('ぶ', "ばびべぼ"),
    ('む', "まみめも"),
    ('る', "らりれろ"),
];

struct Common {
    readings: PackedWordSet,
    forms: PackedWordSet,
}

fn common() -> &'static Common {
    static COMMON: OnceLock<Common> = OnceLock::new();
    COMMON.get_or_init(|| {
        let text = format!(
            "{}\n{}",
            crate::system_data::text("readings.txt"),
            crate::system_data::text("readings-extra.txt")
        );
        let readings = PackedWordSet::from_lines(&text, true);
        let forms = build_forms(&text, &readings);
        Common { readings, forms }
    })
}

pub(crate) fn common_word_cost(kana: &str) -> Option<f64> {
    let common = common();
    (common.readings.contains(kana) || common.forms.contains(kana)).then(|| word_cost(kana))
}

pub(crate) fn common_has_prefix(kana: &str) -> bool {
    let common = common();
    common.readings.has_prefix(kana) || common.forms.has_prefix(kana)
}

fn word_cost(word: &str) -> f64 {
    if FUNCTION_WORDS.contains(&word) {
        0.8
    } else if word.chars().count() >= 2 {
        1.0
    } else {
        1.5
    }
}

fn build_forms(text: &str, readings: &PackedWordSet) -> PackedWordSet {
    let mut forms: Vec<String> = FUNCTION_WORDS.iter().map(|word| (*word).to_owned()).collect();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split('\t');
        let Some(word) = parts.next() else {
            continue;
        };
        let Some(kinds) = parts.next() else {
            continue;
        };
        for kind in kinds.split(',') {
            add_conjugations(word.trim(), kind.trim(), &mut forms);
        }
    }
    forms.retain(|form| !readings.contains(form));
    PackedWordSet::from_words(forms)
}

fn push_te_forms(base: &str, forms: &mut Vec<String>) {
    for suffix in [
        "て",
        "た",
        "ています",
        "てます",
        "ていました",
        "てました",
        "ている",
        "てる",
        "てた",
        "てください",
        "ちゃった",
        "ちゃう",
        "とく",
        "といた",
    ] {
        forms.push(format!("{base}{suffix}"));
    }
}

fn add_conjugations(word: &str, kind: &str, forms: &mut Vec<String>) {
    let mut chars = word.char_indices();
    let Some((last_at, last)) = chars.next_back() else {
        return;
    };
    if word[..last_at].is_empty() {
        return;
    }
    let stem = &word[..last_at];
    match kind {
        "v1" => {
            if last != 'る' {
                return;
            }
            for ending in ["", "て", "た"] {
                forms.push(format!("{stem}{ending}"));
            }
            push_te_forms(stem, forms);
        }
        "i" => {
            if last != 'い' {
                return;
            }
            for ending in ["く", "かっ", "けれ", "さ", "そう"] {
                forms.push(format!("{stem}{ending}"));
            }
            if word == "いい" {
                for ending in ["よく", "よかっ", "よけれ"] {
                    forms.push(ending.to_owned());
                }
            }
        }
        "vk" => {
            if let Some(base) = word.strip_suffix("くる") {
                for form in ["き", "きて", "きた", "こ", "く", "くれ"] {
                    forms.push(format!("{base}{form}"));
                }
                push_te_forms(&format!("{base}き"), forms);
            }
        }
        "vs" => {
            if let Some(base) = word.strip_suffix("する") {
                for form in ["し", "さ", "せ", "すれ", "しろ"] {
                    forms.push(format!("{base}{form}"));
                }
                push_te_forms(&format!("{base}し"), forms);
            }
        }
        _ => {
            if !kind.starts_with("v5") {
                return;
            }
            let Some((_, row)) = ROWS.iter().find(|(ending, _)| *ending == last) else {
                return;
            };
            for kana in row.chars() {
                forms.push(format!("{stem}{kana}"));
            }
            let onbin = match kind {
                "v5k" | "v5g" => Some("い"),
                "v5k-s" | "v5t" | "v5r" | "v5u" | "v5aru" | "v5r-i" | "v5u-s" => Some("っ"),
                "v5b" | "v5m" | "v5n" => Some("ん"),
                _ => None,
            };
            if let Some(onbin) = onbin {
                forms.push(format!("{stem}{onbin}"));
                push_te_forms(&format!("{stem}{onbin}"), forms);
            }
            if kind == "v5aru" {
                forms.push(format!("{stem}い"));
                for suffix in ["ます", "ました", "ません", "ませんでした"] {
                    forms.push(format!("{stem}い{suffix}"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_word_set_contains_and_prefix() {
        let set = PackedWordSet::from_lines("あ\nかき\nかきく\n# comment\n\n", false);
        assert!(set.contains("かき"));
        assert!(!set.contains("か"));
        assert!(set.has_prefix("か"));
        assert!(set.has_prefix("かき"));
        assert!(!set.has_prefix("さ"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn common_word_cost_and_prefix_match_upstream_rules() {
        assert_eq!(common_word_cost("は"), Some(0.8));
        assert!(common_word_cost("ありがとう").is_some());
        assert!(common_has_prefix("ありが"));
        if common().readings.contains("あ") {
            assert_eq!(common_word_cost("あ"), Some(1.5));
        }
    }

    #[test]
    fn conjugation_forms_are_generated() {
        let mut forms = Vec::new();
        add_conjugations("たべる", "v1", &mut forms);
        assert!(forms.contains(&"たべ".to_string()));
        add_conjugations("かく", "v5k", &mut forms);
        assert!(forms.contains(&"かか".to_string()));
        assert!(forms.contains(&"かい".to_string()));
        add_conjugations("くる", "vk", &mut forms);
        assert!(forms.contains(&"き".to_string()));
    }
}
