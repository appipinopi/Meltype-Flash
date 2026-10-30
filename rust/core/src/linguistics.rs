// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
mod normalization_marks;
use crate::{romaji, storage::State};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::OnceLock,
};
use unicode_normalization::{char::canonical_combining_class, UnicodeNormalization};

#[derive(Clone)]
pub struct Linguistics {
    meanings: HashMap<String, Vec<(String, String)>>,
    stems: HashMap<String, String>,
    translations: HashMap<String, Vec<(String, Vec<String>)>>,
    contexts: HashMap<String, Vec<(String, Vec<String>)>>,
    misspellings: Vec<(String, String)>,
    correct: HashSet<String>,
    shapes: HashMap<String, Vec<String>>,
    ascii_misspellings: bool,
    pub readings: BTreeMap<String, Vec<String>>,
}
pub struct Misspelling {
    pub start: usize,
    pub end: usize,
    pub right: String,
}
pub fn kanji(c: char) -> bool {
    ('㐀'..='䶿').contains(&c) || ('一'..='鿿').contains(&c) || c == '々'
}
pub fn kana(c: char) -> bool {
    ('ぁ'..='ゖ').contains(&c) || ('ァ'..='ヶ').contains(&c) || c == 'ー'
}
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
}
fn ends(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.char_indices().map(|(i, c)| i + c.len_utf8()).rev()
}
impl Linguistics {
    #[cfg(feature = "filesystem")]
    pub(crate) fn profile_misspellings(directory: &std::path::Path) -> Option<Self> {
        let bytes = std::fs::read(directory.join("misspellings.txt")).ok()?;
        let (text, _) = crate::user_dictionary::decode(&bytes).ok()?;
        let mut dictionary = Self::bundled().clone();
        for line in text.lines() {
            let fields: Vec<_> = line
                .split('#')
                .next()
                .unwrap_or("")
                .split('\t')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            let Some(right) = fields.last().map(|s| romaji::katakana(s)) else {
                continue;
            };
            if fields.len() >= 2 {
                let wrong = romaji::katakana(fields[0]);
                let right = romaji::katakana(fields[1]);
                if wrong != right {
                    if let Some(entry) = dictionary
                        .misspellings
                        .iter_mut()
                        .find(|(word, _)| word == &wrong)
                    {
                        entry.1 = right;
                    } else {
                        dictionary.misspellings.push((wrong, right));
                    }
                }
            }
            if dictionary.correct.insert(right.clone()) && right.encode_utf16().count() >= 5 {
                dictionary
                    .shapes
                    .entry(shape(&right))
                    .or_default()
                    .push(right);
            }
        }
        dictionary
            .misspellings
            .sort_by_key(|(word, _)| std::cmp::Reverse(word.len()));
        dictionary.ascii_misspellings = dictionary
            .misspellings
            .iter()
            .any(|(word, _)| word.is_ascii())
            || dictionary.shapes.keys().any(|word| word.is_ascii());
        Some(dictionary)
    }
    pub fn bundled() -> &'static Self {
        static DATA: OnceLock<Linguistics> = OnceLock::new();
        DATA.get_or_init(|| {
            let mut d = Self {
                meanings: HashMap::new(),
                stems: HashMap::new(),
                translations: HashMap::new(),
                contexts: HashMap::new(),
                misspellings: vec![],
                correct: HashSet::new(),
                shapes: HashMap::new(),
                ascii_misspellings: false,
                readings: BTreeMap::new(),
            };
            for line in lines(crate::system_data::text("meanings.txt")) {
                let fields: Vec<_> = line.split('\t').collect();
                if fields.len() < 3 {
                    continue;
                }
                let (word, reading, gloss) = (fields[0], fields[1], fields[2]);
                d.meanings
                    .entry(word.into())
                    .or_default()
                    .push((reading.into(), gloss.into()));
                if let Some((i, last)) = word.char_indices().next_back() {
                    if i > 0
                        && "うくぐすつぬぶむるい".contains(last)
                        && word[..i].chars().any(kanji)
                    {
                        d.stems.entry(word[..i].into()).or_insert(word.into());
                    }
                }
                if reading.chars().count() >= 2
                    && reading.chars().all(kana)
                    && word.chars().any(kanji)
                {
                    let entry = d
                        .readings
                        .entry(crate::user_dictionary::hiragana(reading))
                        .or_default();
                    if !entry.iter().any(|w| w == word) {
                        entry.push(word.into());
                    }
                }
            }
            for line in lines(crate::system_data::text("translations.txt")) {
                let p: Vec<_> = line.split('\t').collect();
                if p.len() >= 3 {
                    d.translations.entry(p[0].into()).or_default().push((
                        p[1].into(),
                        p[2].split(',')
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect(),
                    ));
                }
            }
            for line in lines(crate::system_data::text("contexts.txt")) {
                let line = line.split('#').next().unwrap_or("");
                if let Some((head, cues)) = line.split_once(':') {
                    let head: Vec<_> = head.split_whitespace().collect();
                    if head.len() == 2 {
                        d.contexts.entry(head[0].into()).or_default().push((
                            head[1].into(),
                            cues.split_whitespace().map(str::to_owned).collect(),
                        ));
                    }
                }
            }
            for line in lines(crate::system_data::text("misspellings.txt")) {
                let p: Vec<_> = line.split('\t').collect();
                let right = romaji::katakana(p[p.len() - 1]);
                if p.len() > 1 {
                    d.misspellings.push((romaji::katakana(p[0]), right.clone()));
                }
                d.correct.insert(right.clone());
                if right.chars().count() >= 5 {
                    d.shapes.entry(shape(&right)).or_default().push(right);
                }
            }
            // ASCII readings cannot match non-ASCII spellings or normalized
            // shapes. Derive this from the loaded dictionary, including empty
            // shapes, so adding an ASCII entry keeps the normal lookup path.
            d.ascii_misspellings = d.misspellings.iter().any(|(wrong, _)| wrong.is_ascii())
                || d.shapes.keys().any(|key| key.is_ascii());
            d.misspellings
                .sort_by_key(|(wrong, _)| std::cmp::Reverse(wrong.len()));
            d
        })
    }
    pub fn context(&self, reading: &str, surrounding: &str) -> Option<String> {
        for end in ends(reading) {
            if let Some(rules) = self.contexts.get(&reading[..end]) {
                return rules
                    .iter()
                    .map(|(candidate, cues)| {
                        (
                            candidate,
                            cues.iter()
                                .filter(|s| surrounding.contains(s.as_str()))
                                .count(),
                        )
                    })
                    .filter(|(_, hits)| *hits > 0)
                    .max_by_key(|(_, hits)| *hits)
                    .map(|(candidate, _)| candidate.clone() + &reading[end..]);
            }
        }
        None
    }
    pub fn meaning(&self, text: &str, reading: &str) -> Option<String> {
        if !text.chars().any(|c| kanji(c) || ('ァ'..='ヶ').contains(&c)) {
            return None;
        }
        for end in ends(text) {
            let word = &text[..end];
            let senses = self
                .meanings
                .get(word)
                .or_else(|| self.stems.get(word).and_then(|w| self.meanings.get(w)));
            if let Some(senses) = senses {
                if text[end..]
                    .chars()
                    .any(|c| kanji(c) || ('ァ'..='ヶ').contains(&c))
                {
                    return None;
                }
                let mut selected: Vec<_> = senses
                    .iter()
                    .filter(|(r, _)| !r.is_empty() && reading.starts_with(r))
                    .collect();
                selected.extend(senses.iter().filter(|(r, _)| r.is_empty()));
                if selected.is_empty() {
                    selected = senses.iter().collect();
                }
                let selected: Vec<_> = selected
                    .into_iter()
                    .take(2)
                    .map(|(_, g)| g.as_str())
                    .collect();
                return Some(if selected.len() == 1 {
                    selected[0].into()
                } else {
                    selected
                        .iter()
                        .enumerate()
                        .map(|(i, g)| {
                            format!("{} {g}", char::from_u32('①' as u32 + i as u32).unwrap())
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                });
            }
        }
        None
    }
    pub fn translation_meaning(&self, text: &str) -> Option<String> {
        if !text.chars().any(|c| kanji(c) || ('ァ'..='ヶ').contains(&c)) {
            return None;
        }
        for end in ends(text) {
            if let Some(entries) = self.translations.get(&text[..end]) {
                if text[end..]
                    .chars()
                    .any(|c| kanji(c) || ('ァ'..='ヶ').contains(&c))
                {
                    return None;
                }
                let mut words = vec![];
                for (_, values) in entries {
                    for word in values {
                        if words.len() < 4 && !words.contains(word) {
                            words.push(word.clone());
                        }
                    }
                }
                return (!words.is_empty()).then(|| words.join(", "));
            }
        }
        None
    }
    pub fn translations(&self, text: &str, reading: &str) -> Vec<String> {
        for key in [text, reading] {
            for end in ends(key) {
                if let Some(entries) = self.translations.get(&key[..end]) {
                    let suffix = &key[end..];
                    if ![
                        "", "な", "に", "だ", "です", "の", "を", "が", "は", "も", "と", "で",
                        "へ", "や", "から", "まで", "より", "って", "とか", "さ",
                    ]
                    .contains(&suffix)
                    {
                        break;
                    }
                    let order = match suffix {
                        "な" => ["na", "i", "n", "adv", "v"],
                        "に" => ["adv", "na", "n", "i", "v"],
                        _ => ["n", "na", "i", "v", "adv"],
                    };
                    let mut words = vec![];
                    for kind in order {
                        for (entry_kind, translations) in entries {
                            if entry_kind == kind {
                                let tail = if suffix == "な"
                                    || suffix == "に" && ["adv", "na"].contains(&kind)
                                {
                                    ""
                                } else {
                                    suffix
                                };
                                for word in translations {
                                    let word = format!("{word}{tail}");
                                    if words.len() < 5 && !words.contains(&word) {
                                        words.push(word);
                                    }
                                }
                            }
                        }
                    }
                    return words;
                }
            }
        }
        vec![]
    }
    pub fn misspelling(&self, reading: &str) -> Option<String> {
        let found = self.find_misspelling(reading)?;
        let text = romaji::katakana(reading);
        Some(text[..found.start].to_owned() + &found.right + &text[found.end..])
    }
    /// Byte boundaries in the katakana reading, for replacing only the typo.
    pub fn find_misspelling(&self, reading: &str) -> Option<Misspelling> {
        if reading.is_ascii() && !self.ascii_misspellings {
            return None;
        }
        let text = romaji::katakana(reading);
        let inside_correct = |start: usize, len: usize| {
            self.correct.iter().any(|word| {
                word.len() > len
                    && text
                        .match_indices(word)
                        .any(|(i, _)| i <= start && i + word.len() >= start + len)
            })
        };
        for (wrong, right) in &self.misspellings {
            if let Some(i) = text.find(wrong) {
                if !inside_correct(i, wrong.len()) {
                    return Some(Misspelling {
                        start: i,
                        end: i + wrong.len(),
                        right: right.clone(),
                    });
                }
            }
        }
        let bounds: Vec<_> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        // Removing combining marks leaves stable starters for the bundled
        // shapes. Normalize each scalar once instead of every overlapping
        // substring. Retain the full normalization path if any retained scalar
        // has a combining class, since canonical reordering can cross a cut.
        let units: Vec<_> = text.chars().map(|c| shape(&c.to_string())).collect();
        let incremental = units
            .iter()
            .all(|s| s.chars().all(|c| canonical_combining_class(c) == 0));
        let mut normalized = String::new();
        let mut matches = Vec::new();
        for start in 0..units.len() {
            normalized.clear();
            matches.clear();
            for end in start + 1..=usize::min(start + 20, units.len()) {
                if incremental {
                    normalized.push_str(&units[end - 1]);
                }
                if end < start + 4 {
                    continue;
                }
                let part = &text[bounds[start]..bounds[end]];
                if self.correct.contains(part) {
                    // The original searches from longest to shortest and stops
                    // at a correct spelling. Discard all shorter candidates.
                    matches.clear();
                    continue;
                }
                if !incremental {
                    normalized = shape(part);
                }
                if let Some(right) = self.shapes.get(&normalized).and_then(|s| s.first()) {
                    matches.push((end, right));
                }
            }
            for (end, right) in matches.iter().rev() {
                let end = bounds[*end];
                if !inside_correct(bounds[start], end - bounds[start]) {
                    return Some(Misspelling {
                        start: bounds[start],
                        end,
                        right: (*right).clone(),
                    });
                }
            }
        }
        None
    }
    pub fn predict_english_names(&self, reading: &str) -> Vec<String> {
        crate::candidate_sources::Sources::english_names(reading)
    }
    pub fn predict(&self, reading: &str, state: &State) -> Vec<String> {
        self.predict_with_preferences(reading, &state.phrases, &state.preferences)
    }
    pub(crate) fn predict_with_preferences(
        &self,
        reading: &str,
        phrases: &[crate::storage::Phrase],
        preferences: &crate::session::Preferences,
    ) -> Vec<String> {
        use std::ops::Bound::{Excluded, Unbounded};
        if reading.chars().take(2).count() < 2 {
            return vec![];
        }
        let mut phrases: Vec<_> = phrases
            .iter()
            .filter(|p| p.reading.starts_with(reading) && p.reading != reading)
            .collect();
        phrases.sort_by_key(|p| (std::cmp::Reverse(p.count), std::cmp::Reverse(p.used)));
        let mut result = vec![];
        for word in phrases
            .into_iter()
            .map(|p| p.text.clone())
            .chain(
                preferences
                    .user_dictionary
                    .range::<str, _>((Excluded(reading), Unbounded))
                    .take_while(|(r, _)| r.starts_with(reading))
                    // Conversion stores newest registrations first; upstream
                    // prediction enumerates the original registration order.
                    .flat_map(|(_, v)| v.iter().rev().cloned()),
            )
            .chain(
                preferences
                    .selected
                    .range::<str, _>((Excluded(reading), Unbounded))
                    .take_while(|(r, _)| r.starts_with(reading))
                    .map(|(_, v)| v.clone()),
            )
        {
            if !result.contains(&word) {
                result.push(word);
                if result.len() == 5 {
                    break;
                }
            }
        }
        result
    }
    pub fn words(&self, reading: &str) -> Vec<String> {
        self.readings.get(reading).cloned().unwrap_or_default()
    }
}
fn shape(text: &str) -> String {
    text.nfd()
        .filter(|c| !normalization_marks::is_non_spacing_mark(*c) && !"ッー".contains(*c))
        .map(|c| match c {
            'ァ' => 'ア',
            'ィ' => 'イ',
            'ゥ' => 'ウ',
            'ェ' => 'エ',
            'ォ' => 'オ',
            'ャ' => 'ヤ',
            'ュ' => 'ユ',
            'ョ' => 'ヨ',
            'ヮ' => 'ワ',
            _ => c,
        })
        .collect()
}
pub fn distance(a: &str, b: &str, max: usize) -> usize {
    let a: Vec<_> = a.chars().collect();
    let b: Vec<_> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let mut prev: Vec<_> = (0..=b.len()).collect();
    for (i, a) in a.iter().enumerate() {
        let mut row = vec![i + 1; b.len() + 1];
        for (j, b) in b.iter().enumerate() {
            row[j + 1] = (prev[j + 1] + 1)
                .min(row[j] + 1)
                .min(prev[j] + usize::from(a != b));
        }
        if *row.iter().min().unwrap() > max {
            return max + 1;
        }
        prev = row;
    }
    prev[b.len()]
}

#[cfg(test)]
mod normalization_tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    #[test]
    fn original_character_categories_and_shapes_match() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/data/upstream-unicode-marks.json"))
                .unwrap();
        let bitmap = STANDARD
            .decode(fixture["categories"]["bitmap"].as_str().unwrap())
            .unwrap();
        assert_eq!(bitmap.len(), 8192);
        for value in 0..65536u32 {
            let expected = bitmap[value as usize / 8] & (1 << (value % 8)) != 0;
            if let Some(c) = char::from_u32(value) {
                assert_eq!(
                    normalization_marks::is_non_spacing_mark(c),
                    expected,
                    "U+{value:04X}"
                );
            } else {
                assert!(!expected);
            }
        }
        let cases = fixture["shapes"].as_array().unwrap();
        assert_eq!(cases.len(), 112);
        for case in cases {
            let reading = case["reading"].as_str().unwrap();
            assert_eq!(
                shape(reading),
                case["expected"].as_str().unwrap(),
                "{reading:?}"
            );
        }
    }
}
