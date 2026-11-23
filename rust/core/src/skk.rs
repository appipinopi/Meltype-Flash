// SPDX-License-Identifier: GPL-3.0-or-later
//! SKK-like input: reading + okurigana + dictionary registration.
//! Core provides state machine and dictionary lookup; hosts provide candidate UI.

use std::collections::BTreeMap;
#[cfg(feature = "filesystem")]
use std::io;
#[cfg(feature = "filesystem")]
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dictionary {
    /// reading (hiragana, possibly with okurigana marker suffix) -> candidates
    entries: BTreeMap<String, Vec<String>>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse SKK jisyo format (UTF-8). Lines: `reading /cand1/cand2;ann/`
    pub fn parse(text: &str) -> Self {
        let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let Some(space) = line.find(' ') else {
                continue;
            };
            let (reading, rest) = line.split_at(space);
            let reading = reading.trim();
            let rest = rest.trim();
            if reading.is_empty() || !rest.starts_with('/') {
                continue;
            }
            let mut candidates = Vec::new();
            for part in rest.split('/') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                // strip annotation after ;
                let cand = part.split(';').next().unwrap_or("").trim();
                if cand.is_empty() {
                    continue;
                }
                // handle # for number conversion - keep as-is for now
                candidates.push(cand.to_string());
            }
            if !candidates.is_empty() {
                entries
                    .entry(reading.to_string())
                    .or_default()
                    .extend(candidates);
            }
        }
        // dedupe per reading
        for cands in entries.values_mut() {
            let mut seen = std::collections::HashSet::new();
            cands.retain(|c| seen.insert(c.clone()));
        }
        Self { entries }
    }

    pub fn lookup(&self, reading: &str) -> &[String] {
        self.entries.get(reading).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Register a new entry (user dictionary).
    pub fn register(&mut self, reading: &str, candidate: &str) {
        if reading.is_empty() || candidate.is_empty() {
            return;
        }
        let list = self.entries.entry(reading.to_string()).or_default();
        if !list.iter().any(|c| c == candidate) {
            list.insert(0, candidate.to_string());
        }
    }

    /// Append entries from another dictionary. Base dictionaries keep their
    /// own candidate order and only add candidates that are still absent.
    pub fn merge(&mut self, other: &Self) {
        for (reading, candidates) in &other.entries {
            let list = self.entries.entry(reading.clone()).or_default();
            for candidate in candidates {
                if !list.iter().any(|c| c == candidate) {
                    list.push(candidate.clone());
                }
            }
        }
    }

    /// Put another dictionary's candidates first (user dictionary precedence).
    pub fn merge_front(&mut self, other: &Self) {
        for (reading, candidates) in &other.entries {
            let list = self.entries.entry(reading.clone()).or_default();
            for candidate in candidates.iter().rev() {
                if let Some(index) = list.iter().position(|c| c == candidate) {
                    list.remove(index);
                }
                list.insert(0, candidate.clone());
            }
        }
    }

    /// Load an SKK JISYO file as UTF-8.
    #[cfg(feature = "filesystem")]
    pub fn load_file(path: &Path) -> io::Result<Self> {
        Ok(Self::parse(&std::fs::read_to_string(path)?))
    }

    /// Load bundled SKK dictionaries from a directory. The user dictionary is
    /// not included here; call `load_user` and `merge_front` for that.
    #[cfg(feature = "filesystem")]
    pub fn load_dir(dir: &Path) -> Self {
        let mut dictionary = Self::new();
        for name in ["skk-jisyo.txt", "skk-jisyo.utf8", "SKK-JISYO.L", "skk.txt"] {
            if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
                dictionary.merge(&Self::parse(&text));
            }
        }
        dictionary
    }

    /// Load the user dictionary (`skk-user-jisyo.txt`) if present.
    #[cfg(feature = "filesystem")]
    pub fn load_user(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join("skk-user-jisyo.txt"))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// Atomically save the dictionary as an SKK JISYO file.
    #[cfg(feature = "filesystem")]
    pub fn save(&self, path: &Path) -> io::Result<()> {
        crate::storage::atomic_write(path, self.to_jisyo().as_bytes())
    }

    /// Serialize to SKK jisyo text.
    pub fn to_jisyo(&self) -> String {
        let mut out = String::new();
        for (reading, cands) in &self.entries {
            out.push_str(reading);
            out.push_str(" /");
            for c in cands {
                out.push_str(c);
                out.push('/');
            }
            out.push('\n');
        }
        out
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Normal input (pass-through to underlying romaji/kana handling)
    #[default]
    Direct,
    /// ▽ preedit: reading being composed for conversion
    Preedit,
    /// ▽ preedit with okurigana marker set (e.g. after uppercase consonant)
    Okurigana,
    /// ▼ converting: showing candidates
    Convert,
    /// Dictionary registration: prompting user to enter word for unknown reading
    Register,
}

/// Result of one SKK key event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Not an SKK key; let the normal romaji/kana path handle it.
    Pass,
    /// Consumed by SKK; the host should refresh the preedit display.
    Consumed,
    /// Commit this text and leave SKK mode.
    Commit(String),
    /// Register this reading/word, then commit the word.
    Register { reading: String, text: String },
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub mode: Mode,
    /// Hiragana reading accumulated in preedit (without okurigana consonant marker display char)
    pub reading: String,
    /// Okurigana part in hiragana (the kana after the marker, e.g. く in 書く)
    pub okurigana: String,
    /// The okurigana consonant key for dictionary lookup (e.g. 'k' for か*く)
    pub okuri_key: Option<char>,
    /// Raw romaji being typed for current kana unit (to detect uppercase start)
    pub pending_romaji: String,
    /// Conversion candidates
    pub candidates: Vec<String>,
    pub selected: usize,
    /// For registration: the reading being registered
    pub register_reading: String,
    /// For registration: text typed so far as the new word
    pub register_text: String,
}

impl State {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_active(&self) -> bool {
        self.mode != Mode::Direct
    }

    /// Display string for the SKK preedit/convert state.
    pub fn display(&self) -> String {
        match self.mode {
            Mode::Direct => String::new(),
            Mode::Preedit => format!("▽{}", self.reading),
            Mode::Okurigana => format!("▽{}*{}", self.reading, self.okurigana),
            Mode::Convert => {
                if let Some(c) = self.candidates.get(self.selected) {
                    format!("▼{}", c)
                } else {
                    format!("▼{}", self.reading)
                }
            }
            Mode::Register => {
                format!(
                    "[辞書登録] {}: {}",
                    self.register_reading, self.register_text
                )
            }
        }
    }

    /// Process one key while SKK mode is enabled. Lowercase input in Direct
    /// mode passes through so SKK can coexist with the normal romaji/kana path.
    pub fn handle_key(&mut self, ch: i32, dictionary: &Dictionary) -> Action {
        let Some(c) = char::from_u32(ch as u32) else {
            return Action::Pass;
        };
        match self.mode {
            Mode::Direct => {
                if c.is_ascii_uppercase() {
                    self.mode = Mode::Preedit;
                    self.pending_romaji.clear();
                    self.reading.clear();
                    self.pending_romaji.push(c.to_ascii_lowercase());
                    self.refresh_reading();
                    Action::Consumed
                } else {
                    Action::Pass
                }
            }
            Mode::Preedit => match c {
                ' ' => {
                    self.start_convert(dictionary);
                    Action::Consumed
                }
                '\u{1b}' => {
                    self.reset();
                    Action::Consumed
                }
                '\u{8}' => {
                    self.backspace();
                    Action::Consumed
                }
                '\r' => {
                    let text = self.reading.clone();
                    self.reset();
                    Action::Commit(text)
                }
                c if c.is_ascii_uppercase() => {
                    self.start_okurigana(c);
                    Action::Consumed
                }
                c if c.is_ascii_lowercase() => {
                    self.pending_romaji.push(c);
                    self.refresh_reading();
                    Action::Consumed
                }
                _ => Action::Pass,
            },
            Mode::Okurigana => match c {
                ' ' => {
                    self.start_convert(dictionary);
                    Action::Consumed
                }
                '\u{1b}' => {
                    self.mode = Mode::Preedit;
                    self.okurigana.clear();
                    self.okuri_key = None;
                    Action::Consumed
                }
                '\u{8}' => {
                    self.backspace();
                    Action::Consumed
                }
                '\r' => {
                    let text = format!("{}{}", self.reading, self.okurigana);
                    self.reset();
                    Action::Commit(text)
                }
                c if c.is_ascii_lowercase() => {
                    self.pending_romaji.push(c);
                    self.refresh_okurigana();
                    Action::Consumed
                }
                _ => Action::Pass,
            },
            Mode::Convert => match c {
                ' ' => {
                    if !self.candidates.is_empty() {
                        self.selected = (self.selected + 1) % self.candidates.len();
                    }
                    Action::Consumed
                }
                '\u{1b}' => {
                    self.mode = Mode::Preedit;
                    Action::Consumed
                }
                '\r' => {
                    let text = self
                        .candidates
                        .get(self.selected)
                        .cloned()
                        .unwrap_or_else(|| self.reading.clone());
                    self.reset();
                    Action::Commit(text)
                }
                _ => Action::Pass,
            },
            Mode::Register => match c {
                '\r' => {
                    let reading = self.register_reading.clone();
                    let text = self.register_text.clone();
                    self.reset();
                    if reading.is_empty() || text.is_empty() {
                        Action::Consumed
                    } else {
                        Action::Register { reading, text }
                    }
                }
                '\u{1b}' => {
                    self.reset();
                    Action::Consumed
                }
                '\u{8}' => {
                    self.register_text.pop();
                    Action::Consumed
                }
                c if !c.is_control() => {
                    self.register_text.push(c);
                    Action::Consumed
                }
                _ => Action::Pass,
            },
        }
    }

    fn refresh_reading(&mut self) {
        self.reading = crate::romaji::hiragana(&self.pending_romaji, false);
    }

    fn refresh_okurigana(&mut self) {
        self.okurigana = crate::romaji::hiragana(&self.pending_romaji, false);
    }

    fn start_okurigana(&mut self, c: char) {
        self.mode = Mode::Okurigana;
        self.okuri_key = Some(c.to_ascii_lowercase());
        self.pending_romaji.clear();
        self.pending_romaji.push(c.to_ascii_lowercase());
        self.okurigana.clear();
        self.refresh_okurigana();
    }

    fn start_convert(&mut self, dictionary: &Dictionary) {
        self.mode = Mode::Convert;
        self.candidates = dictionary.lookup(&self.lookup_key()).to_vec();
        self.selected = 0;
        if self.candidates.is_empty() {
            self.mode = Mode::Register;
            self.register_reading = self.lookup_key();
            self.register_text.clear();
        }
    }

    fn backspace(&mut self) {
        if self.pending_romaji.pop().is_some() {
            if self.mode == Mode::Okurigana {
                self.refresh_okurigana();
            } else {
                self.refresh_reading();
            }
        } else if self.mode == Mode::Okurigana {
            self.okurigana.pop();
        } else {
            self.reading.pop();
        }
    }

    /// Lookup key for dictionary (with okurigana marker if present).
    pub fn lookup_key(&self) -> String {
        if let Some(k) = self.okuri_key {
            format!("{}{}", self.reading, k)
        } else {
            self.reading.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_jisyo() {
        let dict = Dictionary::parse("あ /亜/阿/\nかk /書/掛/\n; comment\n");
        assert_eq!(dict.lookup("あ"), &["亜".to_string(), "阿".to_string()]);
        assert_eq!(dict.lookup("かk"), &["書".to_string(), "掛".to_string()]);
    }

    #[test]
    fn merge_keeps_base_order_and_puts_user_first() {
        let mut base = Dictionary::parse(
            "あ /亜/阿/
",
        );
        let user = Dictionary::parse(
            "あ /唖/
かk /書/
",
        );
        base.merge(&user);
        assert_eq!(
            base.lookup("あ"),
            &["亜".to_string(), "阿".to_string(), "唖".to_string()]
        );
        base.merge_front(&user);
        assert_eq!(
            base.lookup("あ"),
            &["唖".to_string(), "亜".to_string(), "阿".to_string()]
        );
    }

    #[test]
    fn skk_state_machine_converts_and_registers() {
        let dictionary = Dictionary::parse("かんじ /漢字/感じ/\nかんじk /感じ/\n");
        let mut state = State::default();
        // Lowercase passes through in Direct mode.
        assert_eq!(state.handle_key('a' as i32, &dictionary), Action::Pass);
        // Uppercase starts a reading.
        assert_eq!(state.handle_key('K' as i32, &dictionary), Action::Consumed);
        for c in "anji".chars() {
            assert_eq!(state.handle_key(c as i32, &dictionary), Action::Consumed);
        }
        assert_eq!(state.reading, "かんじ");
        assert_eq!(state.handle_key(' ' as i32, &dictionary), Action::Consumed);
        assert_eq!(state.mode, Mode::Convert);
        assert_eq!(state.candidates[0], "漢字");
        assert_eq!(
            state.handle_key('\r' as i32, &dictionary),
            Action::Commit("漢字".into())
        );
        assert_eq!(state.mode, Mode::Direct);
    }

    #[test]
    fn skk_okurigana_and_registration() {
        let dictionary = Dictionary::parse("かんじk /感じ/\n");
        let mut state = State::default();
        for c in "Kanji".chars() {
            state.handle_key(c as i32, &dictionary);
        }
        // Uppercase K starts okurigana; lowercase u completes く.
        state.handle_key('K' as i32, &dictionary);
        state.handle_key('u' as i32, &dictionary);
        assert_eq!(state.lookup_key(), "かんじk");
        assert_eq!(state.okurigana, "く");
        state.handle_key(' ' as i32, &dictionary);
        assert_eq!(state.candidates[0], "感じ");

        let mut state = State::default();
        state.handle_key('X' as i32, &dictionary);
        state.handle_key(' ' as i32, &dictionary);
        assert_eq!(state.mode, Mode::Register);
        for c in "新語".chars() {
            state.handle_key(c as i32, &dictionary);
        }
        assert_eq!(
            state.handle_key('\r' as i32, &dictionary),
            Action::Register {
                reading: "x".into(),
                text: "新語".into()
            }
        );
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn load_dir_user_save_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("skk-jisyo.txt"),
            "あ /亜/阿/
",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("skk-user-jisyo.txt"),
            "あ /唖/
",
        )
        .unwrap();
        let mut dictionary = Dictionary::load_dir(dir.path());
        let user = Dictionary::load_user(dir.path());
        dictionary.merge_front(&user);
        assert_eq!(dictionary.lookup("あ")[0], "唖");
        dictionary.register("てすと", "テスト");
        let path = dir.path().join("saved.txt");
        dictionary.save(&path).unwrap();
        let reloaded = Dictionary::load_file(&path).unwrap();
        assert_eq!(reloaded.lookup("あ")[0], "唖");
        assert_eq!(reloaded.lookup("てすと"), &["テスト".to_string()]);
    }

    #[test]
    fn register_and_serialize() {
        let mut dict = Dictionary::new();
        dict.register("てすと", "テスト");
        dict.register("てすと", "試験");
        assert_eq!(dict.lookup("てすと")[0], "試験");
        let text = dict.to_jisyo();
        assert!(text.contains("てすと /"));
    }
}
