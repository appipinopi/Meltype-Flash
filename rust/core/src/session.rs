// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
mod clause_metadata;
mod direct;
mod engine_learning;
mod misspelling;
mod mode;
mod reconversion;
mod v130;
#[cfg(feature = "filesystem")]
use crate::storage::Store;
use crate::{
    dictionary::Dictionary,
    linguistics::Linguistics,
    romaji,
    settings::Settings,
    storage::{Change, State},
};
pub use engine_learning::Batch as EngineLearning;
pub use reconversion::ReconversionPart;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    /// UTF-16 units to move inside a committed bracket pair (capable hosts only).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub caret_back: u32,
    pub delete_before: usize,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub text: String,
    pub converting: bool,
    pub selected_index: usize,
    pub selected_clause: usize,
    pub hint: String,
    pub candidates: Vec<String>,
    pub clauses: Vec<String>,
    pub meaning: Option<String>,
    pub suggestion: Option<String>,
    pub predictions: Vec<String>,
    pub selected_prediction: i32,
    pub typed: Option<String>,
    pub notes: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub consumed: bool,
    pub commits: Vec<TextEdit>,
    pub view: Option<View>,
    #[serde(rename = "selectionExpect", skip_serializing_if = "Option::is_none")]
    pub selection_expect: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub english: BTreeMap<String, bool>,
    pub selected: BTreeMap<String, String>,
    pub user_dictionary: BTreeMap<String, Vec<String>>,
}

/// One converter segment. Reading boundaries are Unicode scalar boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub reading: String,
    pub candidates: Vec<String>,
    pub selected: usize,
    /// English is preserved until the user explicitly moves its boundary.
    pub english: bool,
    /// None for protected text, Some for English sound-unit readings.
    pub kana: Option<String>,
    /// Original input for this exact reading range; cleared by boundary edits.
    pub raw: Option<String>,
    /// True only after an explicit candidate selection; reset by reconversion.
    pub changed: bool,
    /// Translation membership belongs to this clause, not the whole sentence.
    pub translations: Vec<String>,
    /// Separate engine calls retain their place before auxiliary words.
    pub standalone: Option<String>,
    /// MoreCandidates is expanded after base candidates and preferences.
    pub expansion: Option<Vec<String>>,
}
impl Clause {
    pub fn new(reading: String, mut candidates: Vec<String>) -> Self {
        candidates.retain(|c| !c.is_empty());
        let mut unique = Vec::new();
        for candidate in candidates {
            if !unique.contains(&candidate) {
                unique.push(candidate);
            }
        }
        // Converter replies retain their order. The session adds kana/raw
        // alternatives when building the final list; injecting a reading here
        // would place it ahead of dictionary words and shift numbered choices.
        if unique.is_empty() {
            unique.push(reading.clone());
        }
        Self {
            reading,
            candidates: unique,
            selected: 0,
            english: false,
            kana: None,
            raw: None,
            changed: false,
            translations: Vec::new(),
            standalone: None,
            expansion: None,
        }
    }
    pub fn literal(reading: String, ordinary_english: bool) -> Self {
        let mut clause = Self::new(reading.clone(), vec![reading]);
        clause.english = true;
        if ordinary_english {
            clause.kana = Some(String::new());
        }
        clause
    }
    pub fn text(&self) -> &str {
        &self.candidates[self.selected]
    }
}

struct CommittedInput {
    raw: String,
    text: String,
    keys: Vec<(i32, char, bool)>,
    units: Option<crate::composition_units::Units>,
    romaji: Option<std::sync::Arc<romaji::CustomTable>>,
}

pub struct Session {
    v130: v130::State,
    profile_dictionary: Option<Dictionary>,
    candidate_sources: std::sync::Arc<crate::candidate_sources::Sources>,
    profile_misspellings: Option<Linguistics>,
    probe_cache: std::sync::Arc<romaji::probes::Cache>,
    raw: String,
    candidates: Vec<String>,
    selected: usize,
    clauses: Vec<Clause>,
    live_clauses: Vec<Clause>,
    selected_clause: usize,
    explicit: Option<String>,
    pub direct: bool,
    pub external_converter_only: bool,
    pub automatic_english_spacing: bool,
    pub direct_detection: bool,
    pub direct_english_word: bool,
    pub code_input: bool,
    code_japanese: bool,
    pub private: bool,
    /// InputScope.PRIVATE permits conversion while suppressing learning.
    pub no_learning: bool,
    engine_learning: Option<Vec<engine_learning::Batch>>,
    pub preferences: Preferences,
    pub settings: Settings,
    pub state: State,
    #[cfg(feature = "filesystem")]
    store: Option<Store>,
    pub storage_error: Option<String>,
    before: String,
    after: String,
    selected_prediction: i32,
    translation_candidates: Vec<String>,
    function_key: i32,
    balanced_override: bool,
    case_cycle: u8,
    kana_text: String,
    process: String,
    pub can_delete: bool,
    /// Host can place the caret inside a newly committed bracket pair.
    pub can_move_caret: bool,
    last_commit: Option<CommittedInput>,
    typed_keys: Vec<(i32, char, bool)>,
    reconversion_origin: Option<String>,
    context_is_fresh: bool,
    correctable: Vec<crate::commit_correction::Record>,
    space_started_conversion: bool,
    tab_started_conversion: bool,
    conversion_origin_display: Option<String>,
    chosen_conversion: bool,
    sigil: crate::sigil::Sigil,
    edited_units: Option<crate::composition_units::Units>,
    custom_romaji: Option<std::sync::Arc<romaji::CustomTable>>,
    clock: web_time::Instant,
    key_time_ms: u64,
    interval_last_ms: Option<u64>,
    key_intervals: std::collections::VecDeque<serde_json::Value>,
    direct_pending: String,
    direct_last_key_ms: Option<u64>,
    #[cfg(feature = "filesystem")]
    hook_session: crate::input_session::InputSession,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            v130: Default::default(),
            profile_dictionary: None,
            candidate_sources: crate::candidate_sources::Sources::bundled(),
            profile_misspellings: None,
            probe_cache: Default::default(),
            raw: String::new(),
            candidates: vec![],
            selected: 0,
            clauses: vec![],
            live_clauses: vec![],
            selected_clause: 0,
            explicit: None,
            direct: false,
            external_converter_only: false,
            automatic_english_spacing: false,
            direct_detection: false,
            direct_english_word: false,
            code_input: false,
            code_japanese: false,
            private: false,
            no_learning: false,
            engine_learning: None,
            preferences: Preferences::default(),
            settings: Settings::default(),
            state: State::default(),
            #[cfg(feature = "filesystem")]
            store: None,
            storage_error: None,
            before: String::new(),
            after: String::new(),
            selected_prediction: -1,
            translation_candidates: vec![],
            function_key: 0,
            balanced_override: false,
            case_cycle: 0,
            kana_text: String::new(),
            process: String::new(),
            can_delete: false,
            can_move_caret: true,
            last_commit: None,
            typed_keys: vec![],
            reconversion_origin: None,
            context_is_fresh: false,
            correctable: vec![],
            space_started_conversion: false,
            tab_started_conversion: false,
            conversion_origin_display: None,
            chosen_conversion: false,
            sigil: Default::default(),
            edited_units: None,
            custom_romaji: None,
            clock: web_time::Instant::now(),
            key_time_ms: 0,
            interval_last_ms: None,
            key_intervals: Default::default(),
            direct_pending: String::new(),
            direct_last_key_ms: None,
            #[cfg(feature = "filesystem")]
            hook_session: Default::default(),
        }
    }
}

impl Session {
    fn dictionary(&self) -> &Dictionary {
        self.profile_dictionary
            .as_ref()
            .unwrap_or_else(|| Dictionary::bundled())
    }
    #[cfg(feature = "filesystem")]
    pub fn open(store: Store) -> std::io::Result<Self> {
        let candidate_sources =
            crate::candidate_sources::Sources::load(&store.directory.join("dictionaries"))?;
        let settings = store.settings()?;
        let state = store.load()?;
        let (custom_romaji, storage_error) =
            match romaji::CustomTable::load(&store.directory.join("dictionaries")) {
                Ok(table) => (table, None),
                Err(error) => (None, Some(format!("romaji.txt: {error}"))),
            };
        let mut hook_session = crate::input_session::InputSession::default();
        let profile_dictionary =
            Dictionary::profile(&store.directory.join("dictionaries"), custom_romaji.clone());
        hook_session.set_model(state.prefix_model.clone());
        Ok(Self {
            profile_dictionary,
            candidate_sources,
            profile_misspellings: Linguistics::profile_misspellings(
                &store.directory.join("dictionaries"),
            ),
            hook_session,
            preferences: state.preferences.clone(),
            state,
            settings,
            store: Some(store),
            selected_prediction: -1,
            custom_romaji,
            storage_error,
            ..Default::default()
        })
    }
    pub fn set_context(&mut self, before: Option<&str>, after: Option<&str>) {
        if self.private {
            self.before.clear();
            self.after.clear();
            self.context_is_fresh = false;
            return;
        }
        self.context_is_fresh = before.is_some();
        if let Some(before) = before {
            self.sigil.context(before);
            let expected: String = self.correctable.iter().map(|r| r.text.as_str()).collect();
            if !before.ends_with(&expected) {
                self.correctable.clear();
            }
            self.before = before
                .chars()
                .rev()
                .take(4000)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
        }
        if let Some(after) = after {
            self.after = after.chars().take(4000).collect();
        }
    }
    #[cfg(feature = "filesystem")]
    pub fn reload(&mut self) -> std::io::Result<()> {
        if let Some(store) = &self.store {
            self.settings = store.settings()?.for_app(&self.process);
            self.state = store.load()?;
            self.hook_session.set_model(self.state.prefix_model.clone());
            self.preferences = self.state.preferences.clone();
            if self.raw.is_empty() {
                self.candidate_sources =
                    crate::candidate_sources::Sources::load(&store.directory.join("dictionaries"))?;
                self.profile_misspellings =
                    Linguistics::profile_misspellings(&store.directory.join("dictionaries"));
                match romaji::CustomTable::load(&store.directory.join("dictionaries")) {
                    Ok(table) => self.custom_romaji = table,
                    Err(error) => self.storage_error = Some(format!("romaji.txt: {error}")),
                }
                self.profile_dictionary = Dictionary::profile(
                    &store.directory.join("dictionaries"),
                    self.custom_romaji.clone(),
                );
            }
        }
        Ok(())
    }
    #[cfg(feature = "filesystem")]
    pub fn control(&mut self, request: &serde_json::Value) -> std::io::Result<serde_json::Value> {
        if request["op"] == "prefix_learn" && (self.private || self.no_learning) {
            return Ok(serde_json::json!({"saved":false}));
        }
        if request["op"] == "hook_session" {
            let command = serde_json::from_value(request["command"].clone())?;
            let _romaji = romaji::Scope::new(self.custom_romaji.clone());
            return Ok(serde_json::to_value(
                self.hook_session.execute(command, &self.settings),
            )?);
        }
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| std::io::Error::other("session has no persistent store"))?;
        let result = crate::control::execute(store, request)?;
        self.reload()?;
        Ok(result)
    }
    pub fn set_process(&mut self, process: &str) {
        if self.process != process {
            self.forget_direct_input();
            self.sigil = Default::default();
            self.correctable.clear();
            self.code_japanese = false;
            self.last_commit = None;
        }
        self.process = process.into();
        #[cfg(feature = "filesystem")]
        let settings = self
            .store
            .as_ref()
            .and_then(|s| s.settings().ok())
            .unwrap_or_else(|| self.settings.clone());
        #[cfg(not(feature = "filesystem"))]
        let settings = self.settings.clone();
        self.settings = settings.for_app(process);
        self.code_input = self.settings.text("AppProfile") == "Code";
    }
    fn learn(&mut self, changes: Vec<Change>) {
        if self.private || self.no_learning || !self.settings.boolean("LearningEnabled") {
            return;
        }
        let context = self.memory_context();
        let changes = changes
            .into_iter()
            .map(|change| match (change, context) {
                (
                    Change::Language {
                        raw,
                        english,
                        explicit,
                    },
                    Some(context),
                ) => Change::ContextualLanguage {
                    raw,
                    english,
                    explicit,
                    context,
                },
                (change, _) => change,
            })
            .collect::<Vec<_>>();
        self.state.preferences = self.preferences.clone();
        #[cfg(feature = "filesystem")]
        if let Some(store) = &self.store {
            match store.record(changes) {
                Ok(state) => {
                    self.state = state;
                    self.preferences = self.state.preferences.clone();
                }
                Err(e) => self.storage_error = Some(e.to_string()),
            }
            return;
        }
        for change in changes {
            self.state.apply(change);
        }
        self.preferences = self.state.preferences.clone();
    }
    pub fn predictions(&self) -> Vec<String> {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.private
            || !self.settings.boolean("PredictiveCandidates")
            || self.is_converting()
            || self.raw.is_empty()
            || self.function_key != 0
            || self.explicit.is_some()
            || self.language_suggestion().is_some()
            || self.composition_misspelling().is_some()
        {
            return vec![];
        }
        let displayed = self.display(false);
        let english = displayed == self.raw && self.raw.bytes().all(|c| c.is_ascii_alphabetic());
        let reading = if self.uses_kana() {
            self.kana_text.clone()
        } else if let Some(units) = self.edited_units.as_ref().filter(|u| u.raw() == self.raw) {
            units.kana(false)
        } else {
            romaji::hiragana(&self.raw, false)
        };
        let mut predictions = if !english
            && reading
                .chars()
                .all(|c| ('ぁ'..='ゖ').contains(&c) || c == 'ー')
        {
            Linguistics::bundled().predict_with_preferences(
                &reading,
                &self.state.phrases,
                &self.preferences,
            )
        } else {
            vec![]
        };
        if !english && self.settings.boolean("EnglishNamePredictions") {
            for word in crate::candidate_sources::Sources::english_names(&reading) {
                if predictions.len() < 5 && !predictions.contains(&word) {
                    predictions.push(word);
                }
            }
        }
        for word in self
            .dictionary()
            .predict_english(&self.raw)
            .into_iter()
            .filter(|_| english || self.raw.len() >= 4)
        {
            if predictions.len() < 5 && !predictions.contains(&word) {
                predictions.push(word);
            }
        }
        predictions
    }
    pub fn select_prediction(&mut self, index: usize) -> SessionResult {
        let predictions = self.predictions();
        if let Some(text) = predictions.get(index) {
            self.explicit = Some(text.clone());
            self.selected_prediction = index as i32;
            self.commit()
        } else {
            self.result(false)
        }
    }
    pub fn code_english(&self) -> bool {
        self.code_input
            && !self.code_japanese
            && !self.direct
            && crate::context::classify(&self.before) == crate::context::LineKind::Code
    }
    pub fn set_code_line(&mut self, japanese: bool) -> bool {
        if !self.code_input || self.direct || !self.raw.is_empty() {
            return false;
        }
        if japanese {
            if self.code_japanese
                || crate::context::classify(&self.before) != crate::context::LineKind::Code
            {
                return false;
            }
            self.code_japanese = true;
        } else {
            if !self.code_japanese {
                return false;
            }
            self.code_japanese = false;
        }
        true
    }
    pub fn caret_moved(&mut self, vk: i32, control: bool) {
        self.sigil.lose();
        self.correctable.clear();
        self.last_commit = None;
        self.forget_direct_input();
        if control || matches!(vk, 0 | 9 | 0x21..=0x28 | 0x2e) {
            self.code_japanese = false;
        }
    }
    pub fn spelling_probe(
        &self,
        vk: i32,
        ch: i32,
        modifiers: i32,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Option<crate::spelling::Probe> {
        if self.private
            || self.direct
            || self.code_english()
            || self.uses_kana()
            || !self.settings.boolean("Enabled")
            || self.settings.text("DetectionLevel") == "Manual"
        {
            return None;
        }
        let mut words = vec![self.raw.clone()];
        if vk == -1 {
            words.extend(self.last_commit.iter().map(|commit| commit.raw.clone()));
        }
        if modifiers & 14 == 0 {
            if vk == 8 {
                let mut raw = self.raw.clone();
                raw.pop();
                words.push(raw);
            } else if let Some(c) = u32::try_from(ch)
                .ok()
                .and_then(char::from_u32)
                .filter(|c| *c >= ' ' && *c != '\u{7f}')
            {
                words.push(format!("{}{c}", self.raw));
            }
        }
        Some(crate::spelling::Probe {
            romaji: self.custom_romaji.clone(),
            words,
            before: before
                .unwrap_or(&self.before)
                .chars()
                .rev()
                .take(4000)
                .collect::<String>()
                .chars()
                .rev()
                .collect(),
            after: after.unwrap_or(&self.after).chars().take(4000).collect(),
            level: self.settings.text("DetectionLevel").to_owned(),
            scored: Dictionary::uses_scored(self.settings.boolean("ScoredSegmentation")),
        })
    }
    pub fn raw(&self) -> &str {
        &self.raw
    }
    pub fn reading(&self) -> String {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.v130.slash_at_start && self.raw.starts_with('/') {
            return format!("・{}", romaji::hiragana(&self.raw[1..], true));
        }
        if self.reconversion_origin.is_some() {
            self.raw.clone()
        } else if self.uses_kana() {
            self.kana_text.clone()
        } else if let Some(units) = self.edited_units.as_ref().filter(|u| u.raw() == self.raw) {
            if self.external_converter_only {
                if Dictionary::uses_scored(self.settings.boolean("ScoredSegmentation")) {
                    self.mixed_reading(true)
                } else {
                    units.render(&self.mixed_reading(true), true)
                }
            } else {
                units.kana(true)
            }
        } else if let Some(fix) = self.romaji_fix() {
            fix.kana
        } else if self.external_converter_only
            && self.function_key == 0
            && self.settings.text("DetectionLevel") != "Manual"
            && self.preferences.english.get(&self.raw.to_ascii_lowercase()) != Some(&false)
        {
            // Classify only unprotected gaps; a mention must not make the
            // surrounding Japanese sentence look like one protected token.
            self.mixed_reading(true)
        } else {
            crate::protection::transform(&self.raw, |part| romaji::hiragana(part, true))
        }
    }
    /// Conversion engines receive Japanese readings and literal English spans.
    /// This is separate from the reading used by edit and repair operations.
    pub fn conversion_reading(&self) -> String {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.v130.slash_at_start && self.raw.starts_with('/') {
            return self.display(true);
        }
        if self.reconversion_origin.is_some() {
            self.reading()
        } else if self.uses_kana() {
            self.kana_display(true)
        } else {
            self.mixed_reading(true)
        }
    }
    fn mixed_reading(&self, final_input: bool) -> String {
        crate::protection::transform(&self.raw, |part| self.detect_composition(part, final_input))
    }
    fn detect_composition(&self, raw: &str, final_input: bool) -> String {
        if self.settings.boolean("LaughterAfterKana") {
            if let Some(prefix) = raw.strip_suffix('w') {
                let kana = romaji::hiragana(prefix, true);
                if kana.chars().count() == 1 && kana.chars().all(|c| ('ぁ'..='ん').contains(&c)) {
                    return kana + "w";
                }
            }
        }
        let _language = self.language_scope();
        let _probes = romaji::probes::Scope::new(self.probe_cache.clone());
        let dictionary = self.dictionary();
        let level = if self.balanced_override {
            "Balanced"
        } else {
            self.settings.text("DetectionLevel")
        };
        let words: Vec<_> = self
            .before
            .trim_end()
            .split(' ')
            .filter(|w| !w.is_empty())
            .collect();
        let last = words
            .last()
            .map(|w| w.to_ascii_lowercase())
            .unwrap_or_default();
        let name_context = dictionary.before_evidence(&self.before) == 2
            && (["called", "named"].contains(&last.as_str())
                || last == "is"
                    && words.len() >= 2
                    && ["name", "surname", "nickname"]
                        .contains(&words[words.len() - 2].to_ascii_lowercase().as_str()));
        if !self.uses_kana()
            && name_context
            && dictionary.after_evidence(&self.after) >= 0
            && level != "Manual"
            && !raw.is_empty()
            && raw.bytes().all(|c| c.is_ascii_alphabetic())
            && self.preferences.english.get(&raw.to_ascii_lowercase()) != Some(&false)
        {
            return raw.into();
        }
        if Dictionary::uses_scored(self.settings.boolean("ScoredSegmentation")) {
            dictionary.composition_scored_units(
                raw,
                final_input,
                &self.before,
                &self.after,
                level,
                self.edited_units.as_ref().filter(|u| u.raw() == raw),
            )
        } else {
            dictionary.composition(raw, final_input, &self.before, &self.after, level)
        }
    }
    fn refresh_kana(&mut self) {
        if self.settings.text("InputStyle") != "Kana" {
            return;
        }
        self.kana_text.clear();
        for (vk, ch, shift) in &self.typed_keys {
            // Non-JIS keys still pass through CompositionText.Symbol in the
            // original kana input path (e.g. a character-only '-' event).
            let kana = crate::kana::key(*vk, *shift).unwrap_or(match ch {
                '-' => 'ー',
                ',' => '、',
                '.' => '。',
                '[' => '「',
                ']' => '」',
                _ => *ch,
            });
            if "゛゜".contains(kana) {
                if let Some(combined) = self
                    .kana_text
                    .chars()
                    .next_back()
                    .and_then(|previous| crate::kana::combine(previous, kana))
                {
                    self.kana_text.pop();
                    self.kana_text.push(combined);
                    continue;
                }
            }
            self.kana_text.push(kana);
        }
    }
    fn kana_display(&self, final_input: bool) -> String {
        let _memory = self.language_scope();
        self.dictionary().composition_kana(
            &self.raw,
            &self.kana_text,
            final_input,
            &self.before,
            &self.after,
            if self.balanced_override {
                "Balanced"
            } else {
                self.settings.text("DetectionLevel")
            },
        )
    }
    fn uses_kana(&self) -> bool {
        self.settings.text("InputStyle") == "Kana"
    }
    fn romaji_fix(&self) -> Option<crate::typo::Fix> {
        if self.v130.enter_as_shown
            || self.settings.boolean("LaughterAfterKana")
                && self.raw.strip_suffix('w').is_some_and(|prefix| {
                    let kana = romaji::hiragana(prefix, true);
                    kana.chars().count() == 1 && kana.chars().all(|c| ('ぁ'..='ん').contains(&c))
                })
            || !self.settings.boolean("CorrectTypos")
            || self.preferences.english.get(&self.raw.to_ascii_lowercase()) == Some(&true)
            || !self.raw.bytes().all(|c| c.is_ascii_lowercase())
        {
            return None;
        }
        let corrector = crate::typo::Corrector::bundled();
        let fix = corrector.fix(&self.raw, true)?;
        // English word + Japanese particle/suffix is intentional mixed input.
        // Do not reinterpret that word while repairing a following syllable.
        let dictionary = self.dictionary();
        let mixed = dictionary.mixed(&self.raw, true);
        if mixed == self.raw && (self.raw.len() >= 4 || dictionary.is_common_english(&self.raw))
            || mixed.split(|c: char| !c.is_ascii_alphabetic()).any(|word| {
                word.len() >= 4 || word.len() >= 3 && dictionary.is_common_english(word)
            })
        {
            return None;
        }
        for end in 2..self.raw.len() {
            let (word, rest) = self.raw.split_at(end);
            if dictionary.is_english(word)
                && (dictionary.is_japanese(rest)
                    || [
                        "ha", "ga", "wo", "ni", "de", "to", "no", "mo", "ya", "wa", "he",
                    ]
                    .contains(&rest)
                    || corrector.is_common_japanese(rest))
            {
                return None;
            }
        }
        Some(fix)
    }
    /// Explicit Latin display commits on Space unless separate key roles request conversion.
    pub fn space_commits_alphanumeric(&self) -> bool {
        !self.is_converting()
            && !self.settings.boolean("SeparateKeyRoles")
            && matches!(self.function_key, 0x78 | 0x79)
    }
    pub fn is_converting(&self) -> bool {
        !self.candidates.is_empty()
    }
    pub fn display(&self, final_input: bool) -> String {
        let _probes = romaji::probes::Scope::new(self.probe_cache.clone());
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if let Some(text) = &self.explicit {
            return text.clone();
        }
        if !self.clauses.is_empty() {
            return self.clauses.iter().map(Clause::text).collect();
        }
        if let Some(text) = self.candidates.get(self.selected) {
            return text.clone();
        }
        if self.v130.slash_at_start && self.raw.starts_with('/') && self.function_key == 0 {
            return self
                .punctuate(self.detect_composition(&format!("・{}", &self.raw[1..]), final_input));
        }
        if self.english_first() {
            return self.raw.clone();
        }
        if !self.live_clauses.is_empty() {
            return self.live_clauses.iter().map(Clause::text).collect();
        }
        if self.uses_kana() {
            return self.punctuate(self.kana_display(final_input));
        }
        if final_input {
            if let Some(fix) = self.romaji_fix() {
                // FixTypos changes retained spelling before the original
                // detector renders it again (camer -> came is English).
                return self.punctuate(self.detect_composition(&fix.right, true));
            }
        }
        let dictionary = self.dictionary();
        let detect_whole = || self.detect_composition(&self.raw, final_input);
        let detected = detect_whole;
        let text = crate::protection::transform(&self.raw, |part| {
            if part == self.raw {
                detected()
            } else if self.settings.text("DetectionLevel") == "Balanced"
                && !Dictionary::uses_scored(self.settings.boolean("ScoredSegmentation"))
            {
                dictionary.mixed(part, final_input)
            } else {
                self.detect_composition(part, final_input)
            }
        });
        let text = if self.raw.len() > 1
            && self.raw.starts_with('-')
            && self.raw[1..]
                .bytes()
                .all(|c| c.is_ascii_digit() || c == b'.')
            && text.starts_with('ー')
        {
            format!("-{}", &text['ー'.len_utf8()..])
        } else {
            text
        };
        let text = self
            .edited_units
            .as_ref()
            .filter(|u| u.raw() == self.raw)
            .filter(|u| {
                !Dictionary::uses_scored(self.settings.boolean("ScoredSegmentation"))
                    || u.has_z_symbols()
            })
            .filter(|_| {
                !crate::protection::scan(&self.raw)
                    .iter()
                    .any(|span| span.start == 0 && span.end == self.raw.len())
            })
            .map_or(text.clone(), |u| u.render(&text, final_input));
        self.punctuate(text)
    }
    fn language_suggestion(&self) -> Option<String> {
        if self.settings.text("DetectionLevel") != "Manual"
            || self.balanced_override
            || self.function_key != 0
            || self.explicit.is_some()
            || self.is_converting()
        {
            return None;
        }
        let suggested = if self.uses_kana() {
            self.dictionary().composition_kana(
                &self.raw,
                &self.kana_text,
                false,
                &self.before,
                &self.after,
                "Balanced",
            )
        } else {
            self.dictionary()
                .composition(&self.raw, false, &self.before, &self.after, "Balanced")
        };
        if suggested == self.display(false) {
            return None;
        }
        let words: Vec<_> = suggested
            .split(|c: char| !c.is_ascii_alphabetic())
            .filter(|s| !s.is_empty())
            .collect();
        (!words.is_empty()).then(|| words.join(" "))
    }
    /// Auto-mode Space commits a final English segment, including mixed text.
    pub fn ends_in_english(&self) -> bool {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        self.function_key == 0
            && self.explicit.is_none()
            && !self.uses_kana()
            && self.dictionary().rendered_english_tail(
                &self.detect_composition(&self.raw, true),
                self.settings.text("DetectionLevel"),
            )
    }
    fn english_at_word_end(&self) -> bool {
        if self.settings.text("DetectionLevel") == "Manual"
            || self.raw.len() < 2
            || !self.raw.bytes().all(|c| c.is_ascii_alphabetic())
            || !self.dictionary().known_english(&self.raw)
        {
            return false;
        }
        let analysis = romaji::analyze(&self.raw.to_ascii_lowercase(), false);
        analysis.valid
            && !analysis.partial.is_empty()
            && !["n", "nn"].contains(&analysis.partial.as_str())
    }
    fn punctuate(&self, text: String) -> String {
        self.punctuate_with_context(text, self.dictionary().english_context(&self.before))
    }
    fn punctuate_with_context(&self, text: String, mut english: bool) -> String {
        if self.raw.starts_with('$') && self.raw.bytes().all(|c| c.is_ascii_graphic()) {
            return text;
        }
        let (comma, period) = match self.settings.text("Punctuation") {
            "FullWidthCommaPeriod" => ('，', '．'),
            "FullWidthCommaKuten" => ('，', '。'),
            "ToutenFullWidthPeriod" => ('、', '．'),
            _ => ('、', '。'),
        };
        let mut previous = self.before.chars().next_back().unwrap_or('\0');
        crate::protection::transform(&text, |part| {
            part.chars()
                .map(|c| {
                    let latin_punctuation =
                        english && !self.settings.boolean("JapanesePunctuationAfterEnglish");
                    let output = match c {
                        '、' => {
                            if latin_punctuation {
                                ','
                            } else {
                                comma
                            }
                        }
                        '。' => {
                            if latin_punctuation {
                                '.'
                            } else {
                                period
                            }
                        }
                        '!' if !latin_punctuation => '！',
                        '?' if !latin_punctuation => '？',
                        '~' if !latin_punctuation => '～',
                        '\\' => '￥',
                        '/' if self.settings.boolean("SlashAsMiddleDot") => '・',
                        '/' if self.settings.boolean("FullWidthSlash")
                            && crate::linguistics::kana(previous) =>
                        {
                            '／'
                        }
                        '(' if self.settings.boolean("FullWidthParentheses") => '（',
                        ')' if self.settings.boolean("FullWidthParentheses") => '）',
                        '「' if self.settings.boolean("SquareBrackets")
                            && self.raw.contains('[') =>
                        {
                            '［'
                        }
                        '」' if self.settings.boolean("SquareBrackets")
                            && self.raw.contains(']') =>
                        {
                            '］'
                        }
                        '+' | '*' | '=' | '%' | '&' | '$' | '^' | '<' | '>'
                            if !latin_punctuation =>
                        {
                            char::from_u32(c as u32 + 0xfee0).unwrap()
                        }
                        _ => c,
                    };
                    if c.is_ascii_alphabetic() {
                        english = true;
                    } else if crate::linguistics::kana(c) || crate::linguistics::kanji(c) {
                        english = false;
                    }
                    previous = c;
                    output
                })
                .collect()
        })
    }
    fn preview_candidates(&self) -> Option<Vec<String>> {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.is_converting() || self.function_key != 0 || self.explicit.is_some() {
            return None;
        }
        if self.raw.eq_ignore_ascii_case("go") {
            return Some(vec![self.raw.clone(), "ご".into()]);
        }
        if self.settings.boolean("ShowCandidatesWhileTyping")
            && !self.uses_kana()
            && self.display(false) != self.raw
        {
            let reading = romaji::hiragana(&self.raw, false);
            if reading.chars().count() >= 2
                && reading
                    .chars()
                    .all(|c| ('ぁ'..='ゖ').contains(&c) || c == 'ー')
            {
                let mut values = self
                    .preferences
                    .user_dictionary
                    .get(&reading)
                    .cloned()
                    .unwrap_or_default();
                values.extend(self.candidate_sources.candidates(&reading));
                let mut unique = Vec::new();
                for value in values {
                    if !unique.contains(&value) {
                        unique.push(value);
                        if unique.len() == 5 {
                            break;
                        }
                    }
                }
                if unique.len() > 1 {
                    return Some(unique);
                }
            }
        }
        None
    }
    pub fn result(&self, consumed: bool) -> SessionResult {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        let preview = self.preview_candidates();
        let language_suggestion = self.language_suggestion();
        let predictions = self.predictions();
        SessionResult {
            consumed,
            selection_expect: None,
            commits: vec![],
            view: if self.raw.is_empty() {
                None
            } else {
                Some(View {
                    text: predictions
                        .get(self.selected_prediction as usize)
                        .cloned()
                        .unwrap_or_else(|| self.display(false)),
                    converting: self.is_converting(),
                    selected_index: preview
                        .as_ref()
                        .and_then(|values| {
                            values
                                .iter()
                                .position(|value| value == &self.display(false))
                        })
                        .unwrap_or(self.selected),
                    selected_clause: self.selected_clause,
                    hint: language_suggestion
                        .as_ref()
                        .map(|s| format!("Tab → {s}"))
                        .unwrap_or_default(),
                    clauses: self.clauses.iter().map(|c| c.text().to_owned()).collect(),
                    candidates: preview.clone().unwrap_or_else(|| self.candidates.clone()),
                    meaning: if self.settings.boolean("ShowCandidateMeanings")
                        && self.is_converting()
                    {
                        let text = self
                            .candidates
                            .get(self.selected)
                            .cloned()
                            .unwrap_or_else(|| self.display(false));
                        let reading = self
                            .clauses
                            .get(self.selected_clause)
                            .map(|c| c.reading.clone())
                            .unwrap_or_else(|| self.reading());
                        Linguistics::bundled()
                            .meaning(&text, &reading)
                            .or_else(|| Linguistics::bundled().translation_meaning(&text))
                    } else {
                        None
                    },
                    suggestion: self.misspelling_suggestion(),
                    predictions,
                    selected_prediction: if self.selected_prediction >= 0 {
                        self.selected_prediction
                    } else {
                        -1
                    },
                    typed: if self.settings.boolean("ShowTypedKeys") {
                        Some(self.raw.clone())
                    } else {
                        None
                    },
                    notes: preview
                        .as_ref()
                        .unwrap_or(&self.candidates)
                        .iter()
                        .map(|c| {
                            if self
                                .clauses
                                .get(self.selected_clause)
                                .map(|clause| &clause.translations)
                                .unwrap_or(&self.translation_candidates)
                                .contains(c)
                            {
                                Some("英訳".into())
                            } else {
                                let values = preview.as_ref().unwrap_or(&self.candidates);
                                let half: String = c
                                    .chars()
                                    .map(|ch| {
                                        if ('！'..='～').contains(&ch) {
                                            char::from_u32(ch as u32 - 0xfee0).unwrap()
                                        } else if ch == '　' {
                                            ' '
                                        } else {
                                            ch
                                        }
                                    })
                                    .collect();
                                let full = crate::kana::fullwidth(c);
                                if self.is_converting() && half != *c && values.contains(&half) {
                                    Some("全角".into())
                                } else if self.is_converting()
                                    && full != *c
                                    && values.contains(&full)
                                {
                                    Some("半角".into())
                                } else {
                                    None
                                }
                            }
                        })
                        .collect(),
                })
            },
        }
    }
    pub fn reset(&mut self) {
        self.v130 = Default::default();
        self.probe_cache.clear();
        self.forget_direct_input();
        self.sigil = Default::default();
        self.correctable.clear();
        self.space_started_conversion = false;
        self.tab_started_conversion = false;
        self.conversion_origin_display = None;
        self.chosen_conversion = false;
        self.last_commit = None;
        self.reconversion_origin = None;
        self.context_is_fresh = false;
        self.raw.clear();
        self.edited_units = None;
        self.typed_keys.clear();
        self.candidates.clear();
        self.clauses.clear();
        self.live_clauses.clear();
        self.selected_clause = 0;
        self.explicit = None;
        self.selected = 0;
        self.selected_prediction = -1;
        self.translation_candidates.clear();
        self.function_key = 0;
        self.balanced_override = false;
        self.case_cycle = 0;
        self.kana_text.clear();
    }
    /// A host such as IBus has already committed its displayed preedit on
    /// focus loss. Record that exact text, without finalizing pending letters,
    /// accepting an uncommitted prediction, or changing older document text.
    pub fn commit_preedit(&mut self) -> SessionResult {
        self.explicit = Some(self.display(false));
        self.selected_prediction = -1;
        let can_delete = std::mem::replace(&mut self.can_delete, false);
        let result = self.commit();
        self.can_delete = can_delete;
        result
    }
    /// Preserve the legacy macOS C ABI's external-scalar boundary. Combining
    /// marks attach to committed text; the OS receives the original event once.
    pub fn protects_tail(&self) -> bool {
        !self.is_converting()
            && self.function_key == 0
            && !self.uses_kana()
            && crate::protection::scan(&self.raw)
                .last()
                .is_some_and(|span| span.end == self.raw.len())
    }
    pub fn commit_external(&mut self, latin: bool, preserve_text: bool) -> SessionResult {
        let raw_latin = latin
            && !self.is_converting()
            && self.reconversion_origin.is_none()
            && !self.uses_kana()
            && matches!(self.function_key, 0 | 0x79)
            && !self.raw.is_empty()
            && self.raw.bytes().all(|b| b.is_ascii_alphabetic());
        let mut result = if raw_latin {
            let text = self.raw.clone();
            self.reset();
            SessionResult {
                consumed: false,
                commits: vec![TextEdit {
                    caret_back: 0,
                    delete_before: 0,
                    text,
                    expect: None,
                }],
                view: None,
                selection_expect: None,
            }
        } else {
            self.commit_with_options(
                !preserve_text && self.settings.boolean("SpaceAroundEnglish"),
                !preserve_text,
            )
        };
        self.before.clear();
        self.after.clear();
        self.correctable.clear();
        self.last_commit = None;
        self.sigil = Default::default();
        result.consumed = false;
        result
    }
    pub fn commit(&mut self) -> SessionResult {
        self.commit_with_spacing(self.settings.boolean("SpaceAroundEnglish"))
    }
    fn commit_with_spacing(&mut self, space_around: bool) -> SessionResult {
        self.commit_with_options(space_around, true)
    }
    fn commit_with_options(&mut self, space_around: bool, format_phrases: bool) -> SessionResult {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        let mut text = self.display(true);
        if self.selected_prediction >= 0 {
            if let Some(prediction) = self.predictions().get(self.selected_prediction as usize) {
                text = prediction.clone();
            }
        }
        let reading = self.reading();
        let prediction = self.selected_prediction >= 0;
        let phrase_reading = if prediction {
            self.state
                .phrases
                .iter()
                .find(|p| p.text == text && p.reading.starts_with(&reading))
                .map_or_else(|| reading.clone(), |p| p.reading.clone())
        } else if self.is_converting() && !self.clauses.is_empty() {
            self.clauses
                .iter()
                .filter(|c| !c.english)
                .map(|c| c.reading.as_str())
                .collect()
        } else {
            reading.clone()
        };
        #[cfg(all(windows, feature = "filesystem"))]
        self.record_report_correction(&text);
        self.record_engine_learning();
        if !self.private
            && !self.no_learning
            && self.settings.boolean("LearningEnabled")
            && !self.v130.esc_reverted
            && self
                .v130
                .clipboard
                .as_ref()
                .is_none_or(|clip| !text.contains(clip))
            && !self.raw.is_empty()
        {
            let dynamic = crate::time_candidates::is_dynamic_choice(&reading, &text)
                || self
                    .clauses
                    .iter()
                    .chain(&self.live_clauses)
                    .any(|c| crate::time_candidates::is_dynamic_choice(&c.reading, c.text()));
            let remember_phrase = self.settings.boolean("PredictiveCandidates")
                && if prediction {
                    !text.is_ascii()
                } else {
                    text.chars().any(crate::linguistics::kanji)
                        && !phrase_reading.chars().any(|c| c.is_ascii_alphabetic())
                };
            let mut changes = if dynamic || !remember_phrase {
                vec![]
            } else {
                vec![Change::Phrase {
                    reading: phrase_reading,
                    text: text.clone(),
                }]
            };
            changes.extend(self.clause_language_changes());
            changes.extend(self.clause_history_changes());
            if self.is_converting() && self.clauses.is_empty() {
                let clause = self.single_conversion_clause(&reading, &text);
                if let Some(change) = Self::clause_language_change(&clause) {
                    changes.push(change);
                }
                if let Some(change) = Self::clause_history_change(&clause) {
                    changes.push(change);
                }
            }
            if self.function_key != 0 && !self.is_converting() {
                // Like LearnLanguage, remember an explicit mode only when it
                // changes the automatic choice. Retained sound units matter:
                // deleting kou from gakkou leaves がっ, not an English gak.
                let detected = self.detect_composition(&self.raw, true);
                let automatic = self
                    .edited_units
                    .as_ref()
                    .filter(|u| u.raw() == self.raw)
                    .map_or(detected.clone(), |u| u.render(&detected, true));
                let english = self.function_key >= 0x78;
                if if english {
                    !automatic.bytes().all(|c| c.is_ascii_alphabetic())
                } else {
                    automatic.chars().any(|c| c.is_ascii_alphabetic())
                } {
                    changes.push(Change::Language {
                        raw: self.raw.clone(),
                        english,
                        explicit: true,
                    });
                }
            }
            self.learn(changes);
        }
        if format_phrases && self.automatic_english_spacing && self.function_key == 0 {
            text = crate::phrase_spacing::format(&text);
        }
        if space_around {
            text = space_words(&text, &self.before, &self.after, true, false);
        }
        if format_phrases
            && self.settings.boolean("SpaceAroundNumbers")
            && !self.direct
            && !self.private
            && !self.code_english()
        {
            text = crate::phrase_spacing::space_numbers(&text, &self.before, &self.after);
        }
        let english =
            !text.trim().is_empty() && text.trim().bytes().all(|b| b.is_ascii_alphabetic());
        let after_english = self.dictionary().english_context(&self.before);
        let chosen = self.function_key != 0
            || self.chosen_conversion
            || self.selected_prediction >= 0
            || self.is_converting() && self.selected != 0
            || self.reconversion_origin.is_some();
        let correction = if self.can_delete
            && !self.private
            && !self.direct
            && !self.code_english()
            && self.settings.boolean("AutoCorrectAfterCommit")
            && self.settings.text("DetectionLevel") != "Manual"
            && !chosen
        {
            crate::commit_correction::replacement(
                &self.correctable,
                &self.raw,
                english,
                &self.before,
                &self.preferences,
            )
        } else {
            None
        };
        let mut records = std::mem::take(&mut self.correctable);
        if let Some((original, replacement)) = &correction {
            self.before.truncate(self.before.len() - original.len());
            self.before.push_str(replacement);
            records.clear();
        }
        let dictionary = self.dictionary();
        if !self.private
            && !chosen
            && self.settings.text("DetectionLevel") != "Manual"
            && self.reconversion_origin.is_none()
            && dictionary.is_english(&self.raw)
            && dictionary.is_japanese(&self.raw)
        {
            if records.last().is_some_and(|r| !r.space) {
                records.clear();
            }
            records.push(crate::commit_correction::Record {
                raw: self.raw.clone(),
                text: text.clone(),
                english,
                space: self.space_started_conversion,
                after_english,
            });
            if records.len() > 4 {
                records.remove(0);
            }
        } else if !text.is_empty() {
            records.clear();
        }
        self.before.push_str(&text);
        if self.before.chars().count() > 4000 {
            self.before = self
                .before
                .chars()
                .rev()
                .take(4000)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
        }
        let raw = self.raw.clone();
        let keys = self.typed_keys.clone();
        let units = self.edited_units.take();
        let romaji = self.custom_romaji.clone();
        self.reset();
        self.correctable = records;
        if !self.private {
            self.sigil.append(&text);
        }
        if !self.private && !text.is_empty() {
            self.last_commit = Some(CommittedInput {
                raw,
                text: text.clone(),
                keys,
                units,
                romaji,
            });
        }
        let caret_back = u32::from(
            self.can_move_caret
                && self.settings.boolean("CursorIntoBrackets")
                && correction.is_none()
                && crate::shortcuts::is_bracket_pair(&text),
        );
        if caret_back != 0 {
            // The host moves left; cached context and correction history must follow.
            self.before.pop();
            self.after
                .insert_str(0, &text[text.char_indices().nth(1).unwrap().0..]);
            self.correctable.clear();
        }
        SessionResult {
            consumed: !text.is_empty(),
            commits: if text.is_empty() {
                vec![]
            } else {
                vec![TextEdit {
                    caret_back,
                    delete_before: correction
                        .as_ref()
                        .map_or(0, |(original, _)| original.encode_utf16().count()),
                    text: correction
                        .as_ref()
                        .map_or(text.clone(), |(_, replacement)| replacement.clone() + &text),
                    expect: correction.map(|(original, _)| original),
                }]
            },
            view: None,
            selection_expect: None,
        }
    }
    /// Start reconversion without deleting text. Hosts must verify the selected
    /// text against selection_expect before replacing it with the preedit.
    pub fn reconvert_selection(
        &mut self,
        text: &str,
        supplied_reading: Option<&str>,
    ) -> SessionResult {
        if !self.raw.is_empty()
            || self.private
            || !self.settings.boolean("Enabled")
            || text.is_empty()
            || text.chars().count() > 1024
        {
            return self.result(false);
        }
        let reading = supplied_reading
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .or_else(|| {
                self.state
                    .phrases
                    .iter()
                    .filter(|p| p.text == text)
                    .max_by_key(|p| (p.used, p.count))
                    .map(|p| p.reading.clone())
            })
            .or_else(|| {
                self.preferences
                    .user_dictionary
                    .iter()
                    .find(|(_, words)| words.iter().any(|w| w == text))
                    .map(|(r, _)| r.clone())
            })
            .or_else(|| {
                self.preferences
                    .selected
                    .iter()
                    .find(|(_, word)| word.as_str() == text)
                    .map(|(r, _)| r.clone())
            })
            .or_else(|| self.dictionary().reading_for(text))
            .unwrap_or_else(|| romaji::hiragana(&crate::user_dictionary::hiragana(text), true));
        let reading = crate::user_dictionary::hiragana(&reading);
        let literal =
            supplied_reading == Some(text) && self.romaji_reconversion_parts(text).is_some();
        if reading.chars().count() > 1024
            || !literal && reading.chars().any(|c| !crate::linguistics::kana(c))
        {
            return self.result(false);
        }
        self.forget_direct_input();
        self.correctable.clear();
        self.reconversion_origin = Some(text.to_owned());
        self.raw = reading.clone();
        self.typed_keys = reading.chars().map(|c| (0, c, false)).collect();
        self.kana_text = reading;
        self.begin_conversion(vec![text.to_owned()]);
        if let Some(index) = self.candidates.iter().position(|c| c == text) {
            self.selected = index;
        }
        self.reconversion_origin = Some(text.to_owned());
        let mut result = self.result(true);
        result.selection_expect = Some(text.to_owned());
        result
    }
    /// Restore the last committed input as a composition. The host must recheck
    /// `expect` immediately before deleting; `delete_before` counts UTF-16 units.
    pub fn undo_commit(&mut self) -> SessionResult {
        if !self.can_delete || self.private || !self.raw.is_empty() {
            return self.result(false);
        }
        let Some(CommittedInput {
            raw,
            text,
            keys,
            units,
            romaji,
        }) = self.last_commit.take()
        else {
            return self.result(false);
        };
        self.correctable.clear();
        if !self.before.ends_with(&text) {
            return self.result(false);
        }
        self.before.truncate(self.before.len() - text.len());
        self.raw = raw;
        self.typed_keys = keys;
        self.edited_units = units;
        self.custom_romaji = romaji;
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        self.refresh_kana();
        let mut result = self.result(true);
        result.commits.push(TextEdit {
            caret_back: 0,
            delete_before: text.encode_utf16().count(),
            text: String::new(),
            expect: Some(text),
        });
        result
    }
    /// macOS candidate clicks select a composition; Enter commits afterward.
    pub fn select_composing(&mut self, index: usize) -> SessionResult {
        if let Some(preview) = self.preview_candidates() {
            if index >= preview.len() {
                return self.result(false);
            }
            self.candidates = preview;
            let mut clause = Clause::new(self.reading(), self.candidates.clone());
            clause.raw = Some(self.raw.clone());
            self.clauses = vec![clause];
            self.live_clauses.clear();
        }
        if index >= self.candidates.len() {
            return self.result(false);
        }
        self.chosen_conversion = true;
        self.selected = index;
        if let Some(clause) = self.clauses.get_mut(self.selected_clause) {
            clause.selected = index;
            clause.changed = true;
        }
        self.result(true)
    }
    pub fn select(&mut self, index: usize) -> SessionResult {
        if self.preview_candidates().is_some() {
            if self.settings.boolean("ShowCandidatesWhileTyping")
                && !self.raw.eq_ignore_ascii_case("go")
            {
                return self.select_composing(index);
            }
            let result = self.select_composing(index);
            return if result.consumed {
                self.commit()
            } else {
                result
            };
        }
        if index >= self.candidates.len() {
            return self.result(false);
        }
        self.chosen_conversion = true;
        self.selected = index;
        if let Some(clause) = self.clauses.get_mut(self.selected_clause) {
            clause.selected = index;
            clause.changed = true;
            if self.clauses.len() > 1 {
                return self.result(true);
            }
        }
        self.commit()
    }
    fn select_number(&mut self, row: usize) -> Option<SessionResult> {
        let index = self.selected / 9 * 9 + row;
        if index >= self.candidates.len() {
            return None;
        }
        self.chosen_conversion = true;
        self.selected = index;
        if let Some(clause) = self.clauses.get_mut(self.selected_clause) {
            clause.selected = index;
            clause.changed = true;
            if self.selected_clause + 1 < self.clauses.len() {
                self.selected_clause += 1;
                self.sync_clause();
                return Some(self.result(true));
            }
        }
        Some(self.commit())
    }
    /// True after Core accepted Tab as a conversion start. Native adapters
    /// can then fetch clauses without preempting predictions or suggestions.
    pub fn tab_conversion_started(&self) -> bool {
        self.tab_started_conversion && self.is_converting()
    }
    pub fn begin_conversion(&mut self, external: Vec<String>) {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.raw.is_empty() || self.is_converting() {
            return;
        }
        self.tab_started_conversion = false;
        self.conversion_origin_display = self.explicit.clone();
        if self.v130_conversion() {
            return;
        }
        if !self.live_clauses.is_empty() {
            let clauses = std::mem::take(&mut self.live_clauses);
            self.begin_clauses(clauses);
            return;
        }
        let mixed = self.conversion_reading();
        // Mixed prose must keep English islands when matching converter clauses.
        // Explicit conversion of an English-only word still uses its kana reading.
        let reading = if mixed.chars().any(crate::linguistics::kana)
            && mixed.chars().any(|c| c.is_ascii_alphabetic())
        {
            mixed
        } else {
            self.reading()
        };
        // A residual alphabetic reading with no engine response still has a
        // Japanese clause and width alternatives. This also applies when the
        // platform adapter declines converter calls because Enabled is false
        // while an already-open composition is being edited.
        if external.is_empty() && !self.external_converter_only {
            let mut runs: Vec<(bool, String)> = Vec::new();
            for c in reading.chars() {
                let english = c.is_ascii_alphabetic();
                if let Some((kind, text)) = runs.last_mut().filter(|(kind, _)| *kind == english) {
                    let _ = kind;
                    text.push(c);
                } else {
                    runs.push((english, c.to_string()));
                }
            }
            let mixed = runs.len() > 1
                && runs
                    .iter()
                    .any(|(_, text)| text.chars().any(crate::linguistics::kana));
            let clauses = runs
                .into_iter()
                .map(|(english, text)| {
                    let mut clause = Clause::new(text.clone(), vec![text.clone()]);
                    if mixed && english {
                        clause.english = true;
                        clause.kana = Some(romaji::hiragana(&text, true));
                    }
                    clause
                })
                .collect();
            if self.begin_clauses(clauses) {
                return;
            }
        }
        let mut candidates = self
            .preferences
            .user_dictionary
            .get(&reading)
            .cloned()
            .unwrap_or_default();
        candidates.extend(external);
        if self.external_converter_only {
            // The macOS frontend delegates conversion to its registered azooKey
            // callbacks. With no converter result, preserve the complete reading.
            candidates.push(reading.clone());
        } else {
            candidates.extend(self.dictionary().candidates(&self.display(true)));
            candidates.extend(self.dictionary().candidates(&reading));
            candidates.extend(Linguistics::bundled().words(&reading));
            candidates.extend(self.clipboard_candidates(&reading));
            if let Some(context) =
                Linguistics::bundled().context(&reading, &format!("{}{}", self.before, self.after))
            {
                candidates.insert(0, context);
            }
        }
        if self.settings.boolean("TranslationCandidates") {
            self.translation_candidates = Linguistics::bundled().translations(
                candidates.first().map(String::as_str).unwrap_or(&reading),
                &reading,
            );
            candidates.extend(self.translation_candidates.clone());
        }
        candidates.push(self.raw.clone());
        let mut unique = Vec::new();
        for candidate in candidates {
            if !candidate.is_empty() && !unique.contains(&candidate) {
                unique.push(candidate);
            }
        }
        let emoji = self.dictionary().emoji(&reading);
        unique.retain(|candidate| !emoji.contains(candidate));
        unique.extend(emoji);
        unique.extend(crate::number_candidates::candidates(&self.raw));
        crate::math_candidates::augment(&reading, &self.before, &mut unique);
        if let Some(learned) = self.preferences.selected.get(&reading) {
            if let Some(i) = unique.iter().position(|s| s == learned) {
                let value = unique.remove(i);
                unique.insert(0, value);
            }
        }
        self.candidates = unique;
        self.selected = 0;
        self.explicit = None;
        self.selected_prediction = -1;
    }
    /// Request live conversion only for settled Japanese input. English islands
    /// and protected tokens retain their original text.
    pub fn live_reading(&self) -> Option<String> {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if !self.settings.boolean("LiveConversion")
            || self.is_converting()
            || self.explicit.is_some()
            || self.direct
            || self.private
            || self.code_english()
            || !self.settings.boolean("Enabled")
            || self.raw.is_empty()
        {
            return None;
        }
        let reading = self.reading();
        if self.external_converter_only {
            let pending = self.mixed_reading(false);
            return (reading.encode_utf16().count() >= 4
                && pending == reading
                && reading.chars().any(crate::linguistics::kana))
            .then_some(reading);
        }
        if reading.encode_utf16().count() < 4
            || romaji::hiragana(&self.raw, false) != reading
            || self
                .dictionary()
                .mixed(&self.raw, false)
                .chars()
                .any(|c| c.is_ascii_alphabetic())
            || reading.chars().any(|c| !crate::linguistics::kana(c))
        {
            return None;
        }
        Some(reading)
    }
    pub fn update_live_conversion(&mut self, mut clauses: Vec<Clause>) -> bool {
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        let Some(reading) = self.live_reading() else {
            return false;
        };
        if clauses.is_empty()
            || clauses.iter().any(|c| {
                c.reading.is_empty() || c.candidates.is_empty() || c.selected >= c.candidates.len()
            })
            || clauses
                .iter()
                .map(|c| c.reading.as_str())
                .collect::<String>()
                != reading
        {
            return false;
        }
        self.rank_clauses(&mut clauses);
        self.live_clauses = clauses;
        true
    }
    /// Accept segments only when they cover the entire current reading.
    pub fn registered_conversion_plan(
        &self,
        reading: &str,
        engine: &[Clause],
    ) -> Option<Vec<(String, Option<String>)>> {
        let mut words = self.candidate_sources.registrations();
        for (reading, entries) in &self.preferences.user_dictionary {
            if let Some(word) = entries.first() {
                words.insert(reading.clone(), (word.clone(), true));
            }
        }
        let mut boundaries = std::collections::BTreeMap::new();
        let mut at = 0;
        for clause in engine {
            boundaries.insert(at, clause);
            at += clause.reading.len();
        }
        let mut pieces = vec![];
        let mut cursor = 0;
        let mut plain = String::new();
        let mut found = false;
        while cursor < reading.len() {
            let matched = words
                .iter()
                .filter(|(key, (_, user))| {
                    reading[cursor..].starts_with(key.as_str())
                        && (!self.candidate_sources.head_only(key)
                            || *user
                            || engine.is_empty() && cursor == 0
                            || boundaries.contains_key(&cursor))
                        && (!*user
                            || (engine.is_empty() || boundaries.contains_key(&cursor))
                                && !self
                                    .dictionary()
                                    .splits_known_kana_word(&reading[cursor..], key.len()))
                })
                .max_by_key(|(key, _)| key.len());
            if let Some((key, (word, _))) = matched.filter(|(_, (word, _))| {
                !boundaries
                    .get(&cursor)
                    .is_some_and(|clause| clause.text().starts_with(word))
            }) {
                if !plain.is_empty() {
                    pieces.push((std::mem::take(&mut plain), None));
                }
                if let Some(parts) = self.candidate_sources.phrase_parts(key, word) {
                    pieces.extend(parts.iter().map(|(r, w)| (r.clone(), Some(w.clone()))));
                } else {
                    pieces.push((key.clone(), Some(word.clone())));
                }
                cursor += key.len();
                found = true;
            } else {
                let ch = reading[cursor..].chars().next().unwrap();
                plain.push(ch);
                cursor += ch.len_utf8();
            }
        }
        if !plain.is_empty() {
            pieces.push((plain, None));
        }
        (found && pieces.len() > 1).then_some(pieces)
    }
    /// Adapters must retain Space provenance when they fetch engine clauses
    /// before Core handles the key; post-commit correction uses that separator.
    pub fn begin_key_clauses(&mut self, clauses: Vec<Clause>, space: bool) -> bool {
        let starting = !self.is_converting();
        if !self.begin_clauses(clauses) {
            return false;
        }
        if starting && space {
            self.space_started_conversion = true;
        }
        true
    }
    pub fn begin_clauses(&mut self, mut clauses: Vec<Clause>) -> bool {
        if !self.raw.is_empty() && self.v130_conversion() {
            return true;
        }
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if self.raw.is_empty()
            || clauses.is_empty()
            || clauses.iter().any(|c| {
                c.reading.is_empty() || c.candidates.is_empty() || c.selected >= c.candidates.len()
            })
            || clauses
                .iter()
                .map(|c| c.reading.as_str())
                .collect::<String>()
                != self.conversion_reading()
        {
            return false;
        }
        let mut divided = Vec::new();
        let mut phrase_parts = Vec::new();
        let joined = crate::time_candidates::join(clauses);
        let prescribed = self.fixed_phrase_choices(&joined);
        for (clause, prescribed) in joined.into_iter().zip(prescribed) {
            let registered = self.candidate_sources.registered(&clause.reading);
            let parts = (!clause.english)
                .then(|| {
                    self.preferences
                        .user_dictionary
                        .get(&clause.reading)
                        .and_then(|words| words.first())
                        .or_else(|| registered.first())
                        .and_then(|word| self.candidate_sources.phrase_parts(&clause.reading, word))
                })
                .flatten();
            if let Some(parts) = parts {
                for (reading, text) in parts {
                    phrase_parts.push(Some(text.clone()));
                    divided.push(Clause::new(reading.clone(), vec![text.clone()]));
                }
            } else {
                phrase_parts.push(prescribed);
                divided.push(clause);
            }
        }
        clauses = divided;
        for clause in &mut clauses {
            if clause.english {
                continue;
            }
            let digits = clause.reading.chars().any(|c| c.is_ascii_digit());
            let letters = clause.reading.chars().any(|c| c.is_ascii_alphabetic());
            for candidate in clause
                .candidates
                .iter_mut()
                .chain(clause.expansion.iter_mut().flatten())
                .chain(clause.standalone.iter_mut())
            {
                *candidate = candidate
                    .chars()
                    .map(|c| {
                        if digits && ('０'..='９').contains(&c)
                            || letters && (('Ａ'..='Ｚ').contains(&c) || ('ａ'..='ｚ').contains(&c))
                        {
                            char::from_u32(c as u32 - 0xfee0).unwrap()
                        } else {
                            c
                        }
                    })
                    .collect();
            }
        }
        if !self.is_converting() {
            self.tab_started_conversion = false;
            self.conversion_origin_display = self.explicit.clone();
        }
        let original_inputs = self.clause_original_inputs(&clauses);
        let surroundings: Vec<String> = clauses
            .iter()
            .enumerate()
            .map(|(index, _)| {
                format!(
                    "{}{}{}",
                    self.before,
                    clauses
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != index)
                        .map(|(_, clause)| clause.text())
                        .collect::<String>(),
                    self.after
                )
            })
            .collect();
        self.translation_candidates.clear();
        let mut preceding = self.before.clone();
        let mut previous_english = false;
        for (((clause, original_input), surrounding), phrase_part) in clauses
            .iter_mut()
            .zip(original_inputs)
            .zip(surroundings)
            .zip(phrase_parts)
        {
            clause.raw = original_input;
            if clause.english {
                if clause.kana.is_some() {
                    clause.kana = Some(romaji::hiragana(&clause.reading, true));
                    // A literal English island needs case choices. Explicit
                    // Shift+Space/reconversion may carry Japanese converter
                    // choices while still excluding this clause from learning.
                    if clause.candidates.len() == 1 && clause.candidates[0] == clause.reading {
                        clause.candidates = self.dictionary().english_candidates(&clause.reading);
                        clause.selected = 0;
                    }
                }
                preceding.push_str(clause.text());
                previous_english = true;
                continue;
            }
            let more = clause
                .expansion
                .clone()
                .unwrap_or_else(|| clause.candidates.clone());
            let mut values = vec![clause.text().to_owned()];
            values.extend(
                self.preferences
                    .user_dictionary
                    .get(&clause.reading)
                    .cloned()
                    .unwrap_or_default(),
            );
            let registered = self.candidate_sources.registered(&clause.reading);
            values.extend(registered.clone());
            if let Some(standalone) = clause.standalone.as_ref() {
                values.push(standalone.clone());
            }
            if !self.external_converter_only {
                values.extend(self.candidate_sources.candidates(&clause.reading));
                values.extend(self.clipboard_candidates(&clause.reading));
                // MeaningDictionary supplies annotations in the original, not
                // extra conversion candidates. Adding its reverse readings
                // changes both candidate numbering and the final-page size.
                if let Some(user) = self
                    .preferences
                    .user_dictionary
                    .get(&clause.reading)
                    .and_then(|words| words.first())
                    .or_else(|| registered.first())
                {
                    values.insert(0, user.clone());
                }
                if let Some(learned) = self.preferences.selected.get(&clause.reading) {
                    values.insert(0, learned.clone());
                }
                if let Some(context) = self
                    .candidate_sources
                    .context(&clause.reading, &surrounding)
                {
                    values.insert(0, context);
                }
                if let Some(preferred) = &phrase_part {
                    values.insert(0, preferred.clone());
                }
                if !self.preferences.selected.contains_key(&clause.reading)
                    && self
                        .candidate_sources
                        .context(&clause.reading, &surrounding)
                        .is_none()
                    && values.first().is_some_and(|text| {
                        !text.is_empty() && !text.chars().any(char::is_alphanumeric)
                    })
                    && clause.reading.chars().all(|c| ('ぁ'..='ゖ').contains(&c))
                {
                    values.insert(0, clause.reading.clone());
                }
            } else if !values.contains(&clause.reading) {
                values.push(clause.reading.clone());
            }
            let translations = if self.settings.boolean("TranslationCandidates") {
                let mut translations = Linguistics::bundled().translations(
                    values.first().map(String::as_str).unwrap_or(""),
                    &clause.reading,
                );
                translations.retain(|word| !values.contains(word));
                if let Some(learned) = self.state.translations.get(&clause.reading) {
                    translations.sort_by_key(|word| {
                        std::cmp::Reverse(learned.get(word).copied().unwrap_or(0))
                    });
                }
                self.translation_candidates.extend(translations.clone());
                let often = translations.iter().find(|word| {
                    self.state
                        .translations
                        .get(&clause.reading)
                        .and_then(|learned| learned.get(*word))
                        .is_some_and(|count| *count >= 2)
                });
                if let Some(word) = often {
                    values.insert(1.min(values.len()), word.clone());
                }
                values.extend(
                    translations
                        .iter()
                        .filter(|word| Some(*word) != often)
                        .cloned(),
                );
                translations
            } else {
                Vec::new()
            };
            crate::math_candidates::augment(&clause.reading, &preceding, &mut values);
            // AddRawCandidates in the original controller places the typed
            // spelling and its fullwidth form after the Japanese alternatives;
            // MoveEmojiLast then moves emoji behind those spellings.
            if let Some(raw) = clause
                .raw
                .as_ref()
                .filter(|raw| raw.chars().any(|c| ('!'..='~').contains(&c)))
            {
                values.extend([raw.clone(), crate::kana::fullwidth(raw)]);
                values.extend(crate::number_candidates::candidates(raw));
            }
            let math = crate::math_candidates::candidate(&clause.reading, &preceding)
                .map(|(symbol, _)| symbol);
            let emoji: Vec<_> = self
                .candidate_sources
                .emoji(&clause.reading)
                .into_iter()
                .filter(|text| math.as_ref() != Some(text))
                .collect();
            let primary = values.first().cloned();
            values.retain(|candidate| {
                !emoji.contains(candidate) || primary.as_ref() == Some(candidate)
            });
            values.extend(emoji);
            let has_preference = self.preferences.selected.contains_key(&clause.reading)
                || !self.external_converter_only
                    && self
                        .candidate_sources
                        .context(&clause.reading, &surrounding)
                        .is_some();
            let particle = [
                "は", "が", "を", "に", "で", "と", "も", "へ", "の", "や", "か", "から", "まで",
                "より",
            ]
            .contains(&clause.reading.as_str());
            let suru = previous_english
                && [
                    "する",
                    "すれ",
                    "した",
                    "して",
                    "しま",
                    "しな",
                    "しよ",
                    "しと",
                    "しちゃ",
                    "しろ",
                    "され",
                    "させ",
                    "せず",
                ]
                .iter()
                .any(|stem| clause.reading.starts_with(stem))
                && values
                    .first()
                    .and_then(|text| text.chars().next())
                    .is_some_and(|ch| !crate::linguistics::kana(ch));
            if phrase_part.is_none() && (!has_preference && particle || suru) {
                values.insert(0, clause.reading.clone());
            }
            // MoreCandidates is expanded after the base list is complete;
            // its emoji and engine ordering must not be moved with the block.
            if !values.is_empty() {
                values.splice(1..1, more);
            }
            let raw = clause.raw.take();
            *clause = Clause::new(clause.reading.clone(), values);
            clause.raw = raw;
            clause.translations = translations;
            preceding.push_str(clause.text());
            previous_english = false;
        }
        self.rank_clauses(&mut clauses);
        self.live_clauses.clear();
        self.clauses = clauses;
        self.selected_clause = 0;
        self.explicit = None;
        self.selected_prediction = -1;
        self.sync_clause();
        true
    }
    fn fixed_phrase_choices(&self, clauses: &[Clause]) -> Vec<Option<String>> {
        let mut choices = vec![None; clauses.len()];
        for parts in self.candidate_sources.phrase_groups() {
            for (start, group) in clauses.windows(parts.len()).enumerate() {
                if group.iter().zip(parts).all(|(clause, (reading, word))| {
                    &clause.reading == reading && clause.candidates.first() == Some(word)
                }) {
                    for (slot, (_, word)) in
                        choices[start..start + parts.len()].iter_mut().zip(parts)
                    {
                        *slot = Some(word.clone());
                    }
                }
            }
        }
        choices
    }
    fn rank_clauses(&self, clauses: &mut [Clause]) {
        // A bundled split phrase keeps its own word choices. Learning 市 for
        // いち must not change 一|学期 into 市|学期, including live previews.
        let choices = self.fixed_phrase_choices(clauses);
        for (clause, phrase_part) in clauses.iter_mut().zip(choices) {
            if phrase_part.is_some() {
                continue;
            }
            if clause.english {
                continue;
            }
            if let Some(preferred) = self.preferences.selected.get(&clause.reading) {
                if let Some(index) = clause.candidates.iter().position(|c| c == preferred) {
                    clause.selected = index;
                }
            }
        }
    }
    fn sync_clause(&mut self) {
        if let Some(clause) = self.clauses.get(self.selected_clause) {
            self.candidates = clause.candidates.clone();
            self.selected = clause.selected;
        }
    }
    /// Snapshot converter readings before a native host releases its registry
    /// lock to obtain fresh candidates. These are conversion clauses only.
    pub fn clause_readings(&self) -> Vec<String> {
        if self.clauses.is_empty() && self.is_converting() {
            vec![self.reading()]
        } else {
            self.clauses.iter().map(|c| c.reading.clone()).collect()
        }
    }
    /// Apply converter replies only to the unchanged composition snapshot.
    /// Callbacks run outside the native registry lock and may return no result.
    pub fn refresh_clause_candidates(
        &mut self,
        expected: &[String],
        external: Vec<(usize, Vec<String>)>,
    ) -> bool {
        if self.clause_readings() != expected || !self.is_converting() {
            return false;
        }
        for (index, candidates) in external {
            if candidates.is_empty() || index >= self.clauses.len() || self.clauses[index].english {
                continue;
            }
            let reading = self.clauses[index].reading.clone();
            self.clauses[index] = self.local_clause_with_external(reading, candidates);
        }
        self.sync_clause();
        true
    }
    fn local_clause(&self, reading: String) -> Clause {
        self.local_clause_with_external(reading, vec![])
    }
    fn local_clause_with_external(&self, reading: String, external: Vec<String>) -> Clause {
        let mut candidates = self
            .preferences
            .user_dictionary
            .get(&reading)
            .cloned()
            .unwrap_or_default();
        candidates.extend(external);
        if !self.external_converter_only {
            candidates.extend(self.dictionary().candidates(&reading));
            candidates.extend(Linguistics::bundled().words(&reading));
        }
        if let Some(learned) = self.preferences.selected.get(&reading) {
            candidates.insert(0, learned.clone());
        }
        Clause::new(reading, candidates)
    }
    fn resize_clause(&mut self, grow: bool) {
        let index = self.selected_clause;
        let Some(current) = self.clauses.get(index) else {
            return;
        };
        let as_japanese = |clause: &Clause| -> Option<String> {
            if clause.english {
                clause
                    .kana
                    .as_ref()
                    .filter(|kana| kana.chars().any(crate::linguistics::kana))
                    .cloned()
            } else {
                Some(clause.reading.clone())
            }
        };
        let Some(mut reading) = as_japanese(current) else {
            return;
        };
        // Validate the move before replacing an English clause. A failed
        // resize must preserve its display and selected candidate verbatim.
        if grow {
            let Some(mut next) = self.clauses.get(index + 1).and_then(as_japanese) else {
                return;
            };
            let c = next.remove(0);
            reading.push(c);
            if next.is_empty() {
                self.clauses.remove(index + 1);
            } else {
                self.clauses[index + 1] = self.local_clause(next);
            }
        } else {
            if reading.chars().count() <= 1 {
                return;
            }
            let c = reading.pop().unwrap();
            // Shrinking before English inserts a new Japanese clause instead
            // of absorbing/reinterpreting that untouched English clause.
            let next = if self.clauses.get(index + 1).is_some_and(|c| !c.english) {
                format!("{c}{}", self.clauses.remove(index + 1).reading)
            } else {
                c.to_string()
            };
            let clause = self.local_clause(next);
            self.clauses.insert(index + 1, clause);
        }
        self.clauses[index] = self.local_clause(reading);
        self.sync_clause();
    }
    /// Virtual keys follow the existing native ABI; modifiers: shift=1, ctrl=2,
    /// alt=4, command=8. Platform shortcuts commit first and pass through.
    pub fn key(&mut self, vk: i32, ch: i32, modifiers: i32) -> SessionResult {
        let now_ms = self.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
        self.key_at(vk, ch, modifiers, now_ms)
    }
    /// Monotonic milliseconds, for hosts with an event clock and deterministic
    /// replay tests. Use one clock consistently for a session.
    pub fn key_at(&mut self, vk: i32, ch: i32, modifiers: i32, now_ms: u64) -> SessionResult {
        // Upstream #630: normalize only full-width Latin letters, preserving
        // punctuation, kana and non-Latin Unicode input.
        let ch = if (0xff21..=0xff3a).contains(&ch) || (0xff41..=0xff5a).contains(&ch) {
            ch - 0xfee0
        } else {
            ch
        };
        let _probes = romaji::probes::Scope::new(self.probe_cache.clone());
        let _romaji = romaji::Scope::new(self.custom_romaji.clone());
        if ch < 0 || char::from_u32(ch as u32).is_none() {
            return self.result(false);
        }
        self.record_key_interval(vk, now_ms);
        self.key_time_ms = now_ms;
        if self.direct && modifiers & 15 != 0 && !self.direct_pending.is_empty() {
            self.direct_pending.clear();
            self.direct_last_key_ms = None;
            self.direct_english_word = true;
        }
        let timeout_edit = self.direct_timeout(now_ms);
        let c = char::from_u32(ch as u32).unwrap();
        if self.private {
            self.sigil = Default::default();
        }
        if self.raw.is_empty()
            && !self.private
            && self.settings.boolean("Enabled")
            && self.settings.boolean("SigilWordsDirect")
            && !self.uses_kana()
            && modifiers & 14 == 0
            && self.sigil.passes(c)
        {
            let passes_slash = self.code_input
                && !self.code_japanese
                && matches!(
                    crate::context::classify(&self.before),
                    crate::context::LineKind::Code | crate::context::LineKind::Prompt
                );
            if c == '/' && !self.direct && !passes_slash && !self.sigil.is_active() {
                self.v130.slash_at_start = true;
            } else {
                self.correctable.clear();
                self.last_commit = None;
                self.forget_direct_input();
                self.sigil.passed(vk, c, modifiers);
                return self.result(false);
            }
        }
        let mut result = self.key_inner(vk, ch, modifiers);
        if let Some(edit) = timeout_edit {
            result.commits.insert(0, edit);
        }
        if !result.consumed && !self.private {
            self.sigil.passed(vk, c, modifiers);
        }
        result
    }
    fn key_inner(&mut self, vk: i32, ch: i32, modifiers: i32) -> SessionResult {
        if let Some(result) = self.detect_direct(vk, ch, modifiers) {
            return result;
        }
        if let Some(result) = self.v130_key(vk, ch, modifiers) {
            return result;
        }
        let reconverting = self.reconversion_origin.is_some();
        let code_direct = self.code_english() && !reconverting;
        if modifiers & 14 == 2
            && !self.raw.is_empty()
            && (!self.direct || reconverting)
            && !self.private
            && !code_direct
            && self.settings.boolean("Enabled")
        {
            let mapped = match vk {
                0x48 => Some(8),
                0x55 => Some(0x75),
                0x49 => Some(0x76),
                0x4f => Some(0x79),
                0x50 => Some(0x78),
                _ => None,
            };
            if let Some(mapped) = mapped {
                return self.key(mapped, 0, modifiers & 1);
            }
        }
        // Upstream PR #802: a send shortcut confirms preedit first. Passing
        // Ctrl+Enter through can submit before the committed text reaches a chat.
        if modifiers & 14 == 2
            && vk == 0x0d
            && !self.raw.is_empty()
            && (!self.direct || reconverting)
            && !self.private
            && !code_direct
            && self.settings.boolean("Enabled")
        {
            return self.commit();
        }
        if modifiers & 14 != 0
            || self.direct && !reconverting
            || self.private
            || code_direct
            || !self.settings.boolean("Enabled")
        {
            self.correctable.clear();
            if self.raw.is_empty() {
                return self.result(false);
            }
            let mut result = self.commit();
            self.correctable.clear();
            result.consumed = false;
            return result;
        }
        if self.settings.boolean("NumpadDirect")
            && matches!(vk, 0x60..=0x69 | 0x6a | 0x6b | 0x6d..=0x6f)
            && matches!(
                char::from_u32(ch as u32),
                Some('0'..='9' | '*' | '+' | '-' | '.' | '/')
            )
        {
            let mut result = self.commit();
            let text = char::from_u32(ch as u32).unwrap().to_string();
            self.before.push_str(&text);
            result.commits.push(TextEdit {
                text,
                delete_before: 0,
                caret_back: 0,
                expect: None,
            });
            result.consumed = true;
            return result;
        }
        if self.is_converting()
            && modifiers & 1 == 0
            && (matches!(vk, 0x31..=0x39)
                && !self.uses_kana()
                && self.settings.boolean("NumberKeysSelectCandidates")
                || matches!(vk, 0x61..=0x69))
        {
            let row = if vk >= 0x61 { vk - 0x61 } else { vk - 0x31 };
            if let Some(result) = self.select_number(row as usize) {
                return result;
            }
        }
        match vk {
            0x09 if self.composition_misspelling().is_some() => {
                let converting = self.is_converting();
                let edit = self.composition_misspelling().expect("checked spelling");
                self.repair_misspelling(edit);
                if converting {
                    self.begin_conversion(vec![]);
                }
                self.result(true)
            }
            0x08 if self.is_converting() => {
                // The first Backspace cancels conversion without editing the
                // retained input. A subsequent Backspace removes a sound unit.
                self.candidates.clear();
                self.clauses.clear();
                self.live_clauses.clear();
                self.selected_clause = 0;
                self.selected = 0;
                self.explicit = self.conversion_origin_display.take();
                self.tab_started_conversion = false;
                self.result(true)
            }
            0x1c if self.is_converting() => {
                self.cycle(1);
                self.result(true)
            }
            0x09 if self.is_converting() && !self.tab_started_conversion => {
                let mut result = self.commit();
                result.consumed = false;
                result
            }
            0x20 if modifiers & 1 != 0 && !self.raw.is_empty() => {
                if self.is_converting() {
                    self.cycle(-1);
                } else {
                    self.space_started_conversion = true;
                    self.begin_conversion(vec![]);
                }
                self.result(true)
            }
            0x20 if modifiers & 1 != 0 => {
                let mut result = self.commit();
                self.correctable.clear();
                result.commits.push(TextEdit {
                    caret_back: 0,
                    delete_before: 0,
                    text: "　".into(),
                    expect: None,
                });
                self.before.push('　');
                result.consumed = true;
                result
            }
            0x20 if self.raw.is_empty() => {
                self.correctable.clear();
                self.result(false)
            }
            0x10..=0x12 | 0xa0..=0xa5 | 0x5b..=0x5c => self.result(false),
            0x1b if self.selected_prediction >= 0 && !self.is_converting() => {
                self.selected_prediction = -1;
                self.result(true)
            }
            0x1b => {
                if let Some(original) = self.reconversion_origin.take() {
                    self.reset();
                    return SessionResult {
                        consumed: true,
                        commits: vec![TextEdit {
                            caret_back: 0,
                            delete_before: 0,
                            text: original,
                            expect: None,
                        }],
                        view: None,
                        selection_expect: None,
                    };
                }
                if self.raw.is_empty() {
                    return self.result(false);
                }
                if self.is_converting() {
                    self.candidates.clear();
                    self.clauses.clear();
                    self.live_clauses.clear();
                    self.selected_clause = 0;
                    self.explicit = self.conversion_origin_display.take();
                    self.tab_started_conversion = false;
                } else {
                    self.reset();
                }
                self.result(true)
            }
            0x08 => {
                if self.raw.is_empty() {
                    self.correctable.clear();
                    return self.result(false);
                }
                let displayed = self.display(false);
                let english_tail = matches!(self.function_key, 0x78 | 0x79)
                    || self.function_key == 0
                        && displayed
                            .chars()
                            .next_back()
                            .is_some_and(|c| c.is_ascii_alphabetic());
                let previous_keys = self.raw.chars().count();
                if self.uses_kana() || self.protects_tail() {
                    self.raw.pop();
                    self.edited_units = None;
                } else {
                    if self
                        .edited_units
                        .as_ref()
                        .is_none_or(|u| u.raw() != self.raw)
                    {
                        self.edited_units = Some(crate::composition_units::Units::capture(
                            &self.raw, &displayed,
                        ));
                    }
                    let units = self.edited_units.as_mut().expect("captured composition");
                    units.remove_last(english_tail);
                    self.raw = units.raw().to_owned();
                }
                self.candidates.clear();
                self.clauses.clear();
                self.live_clauses.clear();
                self.selected_clause = 0;
                self.explicit = None;
                self.selected = 0;
                self.selected_prediction = -1;
                self.function_key = 0;
                self.case_cycle = 0;
                let removed_keys = previous_keys - self.raw.chars().count();
                if self.raw.is_empty() {
                    self.balanced_override = false;
                    self.v130.slash_at_start = false;
                }
                self.typed_keys
                    .truncate(self.typed_keys.len().saturating_sub(removed_keys));
                self.refresh_kana();
                self.result(true)
            }
            0x0d => {
                if self.raw.is_empty() {
                    self.correctable.clear();
                }
                self.commit()
            }
            0x20 | 0x09
                if self.raw.starts_with(self.settings.text("SnippetMark"))
                    && !self.settings.text("SnippetMark").is_empty()
                    && self
                        .state
                        .snippets
                        .contains_key(&self.raw[self.settings.text("SnippetMark").len()..]) =>
            {
                self.explicit = self
                    .state
                    .snippets
                    .get(&self.raw[self.settings.text("SnippetMark").len()..])
                    .cloned();
                self.commit()
            }
            0x20 | 0x09 if self.protects_tail() => {
                if vk == 0x09
                    && self.settings.text("DetectionLevel") == "Manual"
                    && !self.balanced_override
                {
                    // Protected text renders literally in both levels, but
                    // Manual's first Tab still accepts its English proposal.
                    self.balanced_override = true;
                    return self.result(true);
                }
                let suffix = if vk == 0x20 { " " } else { "" };
                self.explicit = Some(self.display(true) + suffix);
                let mut result = self.commit_with_spacing(false);
                result.consumed = vk == 0x20;
                result
            }
            0x09 if !self.is_converting()
                && self.function_key == 0
                && !crate::protection::scan(&self.raw).is_empty() =>
            {
                let mut result = self.commit();
                result.consumed = false;
                result
            }
            0x09 if !self.raw.is_empty() && !self.is_converting() => {
                if self.language_suggestion().is_some() {
                    // Manual's Tab suggestion switches detection strength;
                    // it does not choose an explicit F10 display mode.
                    self.balanced_override = true;
                    self.explicit = None;
                    self.selected_prediction = -1;
                    return self.result(true);
                }
                let predictions = self.predictions();
                if !predictions.is_empty() {
                    let direction = if modifiers & 1 != 0 { -1 } else { 1 };
                    self.selected_prediction = (self.selected_prediction + 1 + direction)
                        .rem_euclid(predictions.len() as i32 + 1)
                        - 1;
                    return self.result(true);
                }
                if self.settings.boolean("TabConversion")
                    && modifiers & 1 == 0
                    && !self.space_commits_alphanumeric()
                    && !self.ends_in_english()
                {
                    self.begin_conversion(vec![]);
                    self.tab_started_conversion = self.is_converting();
                    return self.result(true);
                }
                let mut result = self.commit();
                result.consumed = false;
                result
            }
            0x1c if !self.raw.is_empty() => {
                self.begin_conversion(vec![]);
                self.result(true)
            }
            0x1d if self.settings.boolean("MuhenkanKatakanaCycle") && !self.raw.is_empty() => self
                .key(
                    match self.function_key {
                        0x76 => 0x77,
                        0x77 => 0x75,
                        _ => 0x76,
                    },
                    0,
                    modifiers,
                ),
            0x1d if self.settings.boolean("SeparateKeyRoles") && !self.raw.is_empty() => {
                self.key(0x79, 0, modifiers)
            }
            0x15 if self.settings.boolean("SeparateKeyRoles") && !self.raw.is_empty() => self.key(
                if self.function_key == 0x75 {
                    0x76
                } else {
                    0x75
                },
                0,
                modifiers,
            ),
            0x20 if !self.raw.is_empty() => {
                if self.space_commits_alphanumeric() {
                    self.space_started_conversion = true;
                    self.explicit = Some(self.display(true) + " ");
                    return self.commit();
                }
                if !self.is_converting()
                    && !self.settings.boolean("SeparateKeyRoles")
                    && self.ends_in_english()
                {
                    self.space_started_conversion = true;
                    self.explicit = Some(self.display(true) + " ");
                    return self.commit();
                }
                if !self.is_converting()
                    && !self.settings.boolean("SeparateKeyRoles")
                    && self.function_key == 0
                    && (self.display(true) == self.raw
                        || self.english_at_word_end()
                        || self.before.is_empty()
                            && !["Manual", "Conservative"]
                                .contains(&self.settings.text("DetectionLevel"))
                            && self.dictionary().is_common_english(&self.raw)
                            && (!self.dictionary().is_japanese(&self.raw)
                                || self.raw.starts_with(|c: char| c.is_ascii_uppercase())
                                || [
                                    "no", "i", "we", "you", "they", "he", "she", "it", "the",
                                    "this", "that", "please", "make", "do", "can", "will", "let",
                                    "would", "should", "have", "has", "get", "take", "look",
                                    "open", "run",
                                ]
                                .contains(&self.raw.to_ascii_lowercase().as_str())))
                    && (self.dictionary().known_english(&self.raw)
                        || self.dictionary().english_context(&self.before)
                            && self.raw.bytes().all(|c| c.is_ascii_alphabetic()))
                {
                    self.space_started_conversion = true;
                    self.explicit = Some(self.raw.clone() + " ");
                    return self.commit();
                }
                if self.is_converting() {
                    self.cycle(if modifiers & 1 != 0 { -1 } else { 1 });
                } else {
                    self.space_started_conversion = true;
                    self.begin_conversion(vec![]);
                }
                self.result(true)
            }
            0x25 | 0x27 if self.is_converting() => {
                if self.clauses.is_empty() {
                    self.clauses.push(Clause {
                        reading: self.reading(),
                        candidates: self.candidates.clone(),
                        selected: self.selected,
                        english: false,
                        kana: None,
                        raw: if self.uses_kana() || self.reconversion_origin.is_some() {
                            None
                        } else {
                            Some(self.raw.clone())
                        },
                        changed: self.chosen_conversion,
                        translations: self.translation_candidates.clone(),
                        standalone: None,
                        expansion: None,
                    });
                }
                if modifiers & 1 != 0 {
                    self.resize_clause(vk == 0x27);
                } else {
                    self.selected_clause = if vk == 0x25 {
                        self.selected_clause.saturating_sub(1)
                    } else {
                        (self.selected_clause + 1).min(self.clauses.len() - 1)
                    };
                    self.sync_clause();
                }
                self.result(true)
            }
            0x21 | 0x22 if self.is_converting() => {
                let pages = self.candidates.len().div_ceil(9);
                if pages > 1 {
                    let step = if vk == 0x21 { -1 } else { 1 };
                    let page = (self.selected as i32 / 9 + step).rem_euclid(pages as i32);
                    self.selected = page as usize * 9;
                    self.chosen_conversion = true;
                    if let Some(clause) = self.clauses.get_mut(self.selected_clause) {
                        clause.selected = self.selected;
                        clause.changed = true;
                    }
                }
                self.result(true)
            }
            0x26 | 0x28 | 0x09 if self.is_converting() => {
                self.cycle(if vk == 0x26 || vk == 0x09 && modifiers & 1 != 0 {
                    -1
                } else {
                    1
                });
                self.result(true)
            }
            0x75..=0x79 if !self.raw.is_empty() => {
                self.tab_started_conversion = false;
                self.selected_prediction = -1;
                let before = (!self.is_converting()).then(|| self.display(false));
                let same_mode = !self.is_converting() && self.function_key == vk;
                self.candidates.clear();
                self.clauses.clear();
                self.live_clauses.clear();
                self.selected_clause = 0;
                self.case_cycle = if self.function_key == vk {
                    (self.case_cycle + 1) % 3
                } else {
                    0
                };
                self.function_key = vk;
                if vk >= 0x78 {
                    let render = |case: u8| {
                        let raw = match case {
                            1 => self.raw.to_ascii_uppercase(),
                            2 => {
                                let mut raw = self.raw.to_ascii_lowercase();
                                if let Some(first) = raw.get_mut(..1) {
                                    first.make_ascii_uppercase();
                                }
                                raw
                            }
                            _ => self.raw.clone(),
                        };
                        if vk == 0x78 {
                            crate::kana::fullwidth(&raw)
                        } else {
                            raw
                        }
                    };
                    if !same_mode {
                        self.case_cycle = 0;
                    }
                    let mut output = render(self.case_cycle);
                    if !(self.settings.boolean("AlphanumericKeyStartsAsTyped") && !same_mode)
                        && (same_mode || before.as_deref() == Some(output.as_str()))
                    {
                        // Try all three cases: one-letter/uppercase/numeric
                        // inputs can have identical visible representations.
                        // same_mode already advanced the cycle above.
                        for _ in 0..3 {
                            if before.as_deref() != Some(output.as_str()) {
                                break;
                            }
                            self.case_cycle = (self.case_cycle + 1) % 3;
                            output = render(self.case_cycle);
                        }
                    }
                    self.explicit = Some(output);
                    return self.result(true);
                }
                let raw = match self.case_cycle {
                    1 => self.raw.to_ascii_uppercase(),
                    2 => {
                        let mut raw = self.raw.to_ascii_lowercase();
                        if let Some(first) = raw.get_mut(..1) {
                            first.make_ascii_uppercase();
                        }
                        raw
                    }
                    _ => self.raw.clone(),
                };
                let reading = self.reading();
                self.explicit = Some(
                    if !self.uses_kana()
                        && self.raw.len() >= 2
                        && self.raw.bytes().all(|c| c.is_ascii_uppercase())
                        && vk <= 0x77
                    {
                        // Consecutive uppercase keys are retained acronym
                        // units by CompositionText, even in a kana mode.
                        self.raw.clone()
                    } else if (self.uses_kana()
                        || self.v130.slash_at_start && self.raw.starts_with('/')
                        || self
                            .edited_units
                            .as_ref()
                            .is_some_and(|u| u.raw() == self.raw))
                        && vk <= 0x77
                    {
                        match vk {
                            0x75 => reading,
                            0x76 => romaji::katakana(&reading),
                            _ => crate::kana::halfwidth(&romaji::katakana(&reading)),
                        }
                    } else if vk == 0x78 {
                        crate::kana::fullwidth(&raw)
                    } else if vk == 0x79 {
                        raw.clone()
                    } else {
                        crate::protection::transform(&raw, |part| match vk {
                            0x75 => romaji::hiragana(part, true),
                            0x76 => romaji::katakana(&romaji::hiragana(part, true)),
                            0x77 => crate::kana::halfwidth(&romaji::katakana(&romaji::hiragana(
                                part, true,
                            ))),
                            0x78 => crate::kana::fullwidth(part),
                            _ => part.into(),
                        })
                    },
                );
                self.result(true)
            }
            _ if ch >= 0x20 && ch != 0x7f => {
                self.tab_started_conversion = false;
                // Retain the old commit when a new character starts after conversion.
                let commits = if self.function_key != 0 && !self.is_converting() {
                    // #490/#585: an explicit F6-F10 choice applies to the
                    // current text; the next character starts fresh input.
                    self.commit().commits
                } else if self.is_converting() && !self.settings.boolean("ContinueAfterConversion")
                {
                    // A non-deleting host cannot later recover the typed
                    // separator when an uppercase English word follows
                    // a Japanese conversion (site Space PR).
                    if !self.can_delete
                        && self.space_started_conversion
                        && char::from_u32(ch as u32).is_some_and(|c| c.is_ascii_uppercase())
                        && ![
                            "は", "が", "を", "に", "で", "と", "も", "へ", "の", "や", "か",
                            "から", "まで", "より",
                        ]
                        .contains(&self.reading().as_str())
                        && self.display(true).chars().any(crate::linguistics::kana)
                    {
                        self.explicit = Some(self.display(true) + " ");
                    }
                    self.commit().commits
                } else {
                    self.candidates.clear();
                    self.clauses.clear();
                    self.live_clauses.clear();
                    self.selected_clause = 0;
                    vec![]
                };
                self.explicit = None;
                self.selected_prediction = -1;
                self.function_key = 0;
                if self.raw.chars().count() >= 1024 {
                    let mut result = self.commit();
                    result.consumed = false;
                    return result;
                }
                let character = char::from_u32(ch as u32).unwrap();
                if self.edited_units.is_none()
                    && !self.uses_kana()
                    && self.settings.boolean("ZSymbols")
                    && !self.settings.boolean("DisableZShortcuts")
                    && crate::protection::scan(&self.raw).is_empty()
                    && romaji::z_symbol(character).is_some()
                    && !self.raw.to_ascii_lowercase().ends_with("zz")
                    && romaji::analyze(&self.raw.to_ascii_lowercase(), true).partial == "z"
                {
                    self.edited_units = Some(crate::composition_units::Units::capture(
                        &self.raw,
                        &self.display(false),
                    ));
                }
                if self.raw.is_empty() && self.custom_romaji.is_some() && !self.uses_kana() {
                    self.edited_units = Some(Default::default());
                }
                if let Some(units) = self.edited_units.as_mut() {
                    if units.raw() == self.raw {
                        units.push_with_z_symbols(
                            &character.to_string(),
                            self.settings.boolean("ZSymbols")
                                && !self.settings.boolean("DisableZShortcuts"),
                        );
                    } else {
                        self.edited_units = None;
                    }
                }
                self.raw.push(character);
                self.typed_keys
                    .push((vk, char::from_u32(ch as u32).unwrap(), modifiers & 1 != 0));
                self.refresh_kana();
                let mut result = self.result(true);
                result.commits = commits;
                result
            }
            _ => {
                self.correctable.clear();
                let mut result = self.commit();
                self.correctable.clear();
                result.consumed = false;
                result
            }
        }
    }
    fn cycle(&mut self, direction: i32) {
        self.chosen_conversion = true;
        self.selected =
            (self.selected as i32 + direction).rem_euclid(self.candidates.len() as i32) as usize;
        if let Some(clause) = self.clauses.get_mut(self.selected_clause) {
            clause.selected = self.selected;
            clause.changed = true;
        }
    }
}

fn space_words(text: &str, before: &str, after: &str, english: bool, numbers: bool) -> String {
    let japanese = |c: char| matches!(c,'\u{3040}'..='\u{30ff}'|'\u{3400}'..='\u{4dbf}'|'\u{4e00}'..='\u{9fff}'|'\u{f900}'..='\u{faff}');
    let word_char = |c: char| c.is_ascii_alphanumeric() || "_-+#.'@".contains(c);
    let chars: Vec<_> = text.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    let previous = before.chars().next_back().unwrap_or('\0');
    let next = after.chars().next().unwrap_or('\0');
    let spaced_word = |word: &str| {
        if word.chars().any(|c| c.is_ascii_alphabetic()) {
            english
        } else {
            numbers && word.chars().any(|c| c.is_ascii_digit())
        }
    };
    let previous_word: String = before
        .chars()
        .rev()
        .take_while(|c| word_char(*c))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let next_word: String = after.chars().take_while(|c| word_char(*c)).collect();
    let mut output = String::new();
    if japanese(chars[0]) && previous.is_ascii_alphanumeric() && spaced_word(&previous_word) {
        output.push(' ');
    }
    let mut index = 0;
    while index < chars.len() {
        if !word_char(chars[index]) {
            output.push(chars[index]);
            index += 1;
            continue;
        }
        let mut end = index;
        while end < chars.len() && word_char(chars[end]) {
            end += 1;
        }
        let mut stop = end;
        while stop > index && ".'-@_".contains(chars[stop - 1]) {
            stop -= 1;
        }
        let word = &chars[index..stop];
        let left = if index > 0 {
            chars[index - 1]
        } else {
            previous
        };
        let right = chars.get(stop).copied().unwrap_or(next);
        let spaced = spaced_word(&word.iter().collect::<String>());
        if spaced && japanese(left) && !output.ends_with(' ') {
            output.push(' ');
        }
        output.extend(word);
        if spaced && japanese(right) {
            output.push(' ');
        }
        output.extend(&chars[stop..end]);
        index = end;
    }
    if japanese(*chars.last().unwrap()) && next.is_ascii_alphanumeric() && spaced_word(&next_word) {
        output.push(' ');
    }
    output
}

// Pagefile-backed history shared by the tray and per-application TIP workers.
// No input history is written to the filesystem or included in HTTP requests.
#[cfg(all(windows, feature = "filesystem"))]
pub mod report_corrections {
    use serde::{Deserialize, Serialize};
    use std::{
        collections::HashMap,
        ffi::c_void,
        path::Path,
        sync::{Mutex, OnceLock},
    };
    const SIZE: usize = 128 * 1024;
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Entry {
        pub typed: String,
        pub shown: String,
        pub expected: String,
        pub before: String,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileMappingW(
            file: *mut c_void,
            security: *const c_void,
            protection: u32,
            high: u32,
            low: u32,
            name: *const u16,
        ) -> *mut c_void;
        fn MapViewOfFile(
            handle: *mut c_void,
            access: u32,
            high: u32,
            low: u32,
            length: usize,
        ) -> *mut c_void;
        fn UnmapViewOfFile(view: *const c_void) -> i32;
        fn CreateMutexW(security: *const c_void, owned: i32, name: *const u16) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, timeout: u32) -> u32;
        fn ReleaseMutex(handle: *mut c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    struct Memory {
        map: *mut c_void,
        gate: *mut c_void,
        view: *mut u8,
    }
    // The process-local Mutex protects handles; the named mutex protects all
    // mapped bytes between processes. No pointer or Rust layout crosses the ABI.
    unsafe impl Send for Memory {}
    impl Drop for Memory {
        fn drop(&mut self) {
            unsafe {
                if !self.view.is_null() {
                    UnmapViewOfFile(self.view.cast());
                }
                if !self.map.is_null() {
                    CloseHandle(self.map);
                }
                if !self.gate.is_null() {
                    CloseHandle(self.gate);
                }
            }
        }
    }
    impl Memory {
        fn open(name: &str) -> Option<Self> {
            unsafe {
                let wide = |s: String| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
                let mut result = Self {
                    map: std::ptr::null_mut(),
                    gate: std::ptr::null_mut(),
                    view: std::ptr::null_mut(),
                };
                result.gate =
                    CreateMutexW(std::ptr::null(), 0, wide(format!("{name}-lock")).as_ptr());
                if result.gate.is_null() {
                    return None;
                }
                result.map = CreateFileMappingW(
                    (-1isize) as *mut c_void,
                    std::ptr::null(),
                    4,
                    0,
                    SIZE as u32,
                    wide(name.to_owned()).as_ptr(),
                );
                if result.map.is_null() {
                    return None;
                }
                result.view = MapViewOfFile(result.map, 6, 0, 0, SIZE).cast();
                (!result.view.is_null()).then_some(result)
            }
        }
        fn access(&self, update: impl FnOnce(&mut Vec<Entry>) -> bool) -> Vec<Entry> {
            unsafe {
                // Input must never wait for the report dialog or another worker.
                let wait = WaitForSingleObject(self.gate, 0);
                if wait != 0 && wait != 0x80 {
                    return Vec::new();
                }
                struct Unlock(*mut c_void);
                impl Drop for Unlock {
                    fn drop(&mut self) {
                        unsafe {
                            ReleaseMutex(self.0);
                        }
                    }
                }
                let _unlock = Unlock(self.gate);
                let len = u32::from_le_bytes(std::ptr::read_unaligned(self.view.cast())) as usize;
                let mut entries: Vec<Entry> = if (1..=SIZE - 4).contains(&len) {
                    serde_json::from_slice(std::slice::from_raw_parts(self.view.add(4), len))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                if entries.len() > 20 {
                    entries.clear();
                }
                if update(&mut entries) {
                    if let Ok(bytes) = serde_json::to_vec(&entries) {
                        if bytes.len() <= SIZE - 4 {
                            // Clear removed content as well as its length.
                            std::ptr::write_bytes(self.view, 0, SIZE);
                            std::ptr::copy_nonoverlapping(
                                bytes.as_ptr(),
                                self.view.add(4),
                                bytes.len(),
                            );
                            std::ptr::write_unaligned(
                                self.view.cast::<[u8; 4]>(),
                                (bytes.len() as u32).to_le_bytes(),
                            );
                        }
                    }
                }
                entries
            }
        }
    }
    fn access(directory: &Path, update: impl FnOnce(&mut Vec<Entry>) -> bool) -> Vec<Entry> {
        static ALL: OnceLock<Mutex<HashMap<String, Memory>>> = OnceLock::new();
        let path = directory
            .canonicalize()
            .unwrap_or_else(|_| directory.to_path_buf());
        let identity = path.to_string_lossy().to_lowercase();
        let hash = identity
            .as_bytes()
            .iter()
            .fold(0xcbf29ce484222325_u64, |h, b| {
                (h ^ *b as u64).wrapping_mul(0x100000001b3)
            });
        let name = format!("Local\\Meltype-flash-corrections-v1-{hash:016x}");
        let Ok(mut all) = ALL.get_or_init(Default::default).try_lock() else {
            return Vec::new();
        };
        if !all.contains_key(&name) {
            let Some(memory) = Memory::open(&name) else {
                return Vec::new();
            };
            all.insert(name.clone(), memory);
        }
        all[&name].access(update)
    }
    pub fn recent(directory: &Path) -> Vec<Entry> {
        access(directory, |_| false)
    }
    pub fn clear(directory: &Path) {
        access(directory, |entries| {
            entries.clear();
            true
        });
    }
    pub fn record(directory: &Path, mut entry: Entry) {
        for value in [&mut entry.typed, &mut entry.shown, &mut entry.expected] {
            *value = value.chars().take(256).collect();
        }
        entry.before = entry
            .before
            .chars()
            .rev()
            .take(20)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        access(directory, |entries| {
            if entries.last() == Some(&entry) {
                return false;
            }
            entries.push(entry);
            if entries.len() > 20 {
                entries.remove(0);
            }
            true
        });
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        // Both scenarios exercise the process-wide best-effort registry.
        // Keep their full sequences uncontended; production deliberately drops
        // a report when another report operation owns that registry.
        static HISTORY_TEST: Mutex<()> = Mutex::new(());
        #[test]
        fn independent_handles_share_bounded_memory_without_files() {
            let _test = HISTORY_TEST.lock().unwrap();
            let directory = tempfile::tempdir().unwrap();
            let name = format!(
                "Local\\Meltype-flash-history-test-{}",
                directory.path().file_name().unwrap().to_string_lossy()
            );
            let a = Memory::open(&name).unwrap();
            let b = Memory::open(&name).unwrap();
            let entry = Entry {
                typed: "nihongo".into(),
                shown: "にほんご".into(),
                expected: "nihongo".into(),
                before: String::new(),
            };
            a.access(|entries| {
                entries.push(entry.clone());
                true
            });
            assert_eq!(b.access(|_| false), vec![entry]);
            b.access(|entries| {
                entries.clear();
                true
            });
            assert!(a.access(|_| false).is_empty());
            a.access(|entries| {
                entries.push(Entry {
                    typed: "temporary".into(),
                    shown: "仮".into(),
                    expected: "temporary".into(),
                    before: String::new(),
                });
                true
            });
            drop(a);
            drop(b);
            assert!(Memory::open(&name).unwrap().access(|_| false).is_empty());
            for i in 0..25 {
                record(
                    directory.path(),
                    Entry {
                        typed: i.to_string(),
                        shown: "日".repeat(300),
                        expected: "a".into(),
                        before: "前".repeat(50),
                    },
                );
            }
            let entries = recent(directory.path());
            assert_eq!(entries.len(), 20);
            assert_eq!(entries[0].typed, "5");
            assert_eq!(entries[0].shown.chars().count(), 256);
            assert_eq!(entries[0].before.chars().count(), 20);
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
            clear(directory.path());
            assert!(recent(directory.path()).is_empty());
        }
        #[test]
        fn only_explicit_nonprivate_corrections_are_reported() {
            let _test = HISTORY_TEST.lock().unwrap();
            let directory = tempfile::tempdir().unwrap();
            let mut session =
                super::super::Session::open(crate::storage::Store::new(directory.path())).unwrap();
            session
                .settings
                .set("LiveConversion", serde_json::json!(false))
                .unwrap();
            session
                .settings
                .set("PredictiveCandidates", serde_json::json!(false))
                .unwrap();
            for suppressed in [false, true] {
                for ch in "nihongo".chars() {
                    session.key(ch.to_ascii_uppercase() as i32, ch as i32, 0);
                }
                session.key(0x79, 0, 0);
                session.no_learning = suppressed;
                session.commit();
                if !suppressed {
                    let history = recent(directory.path());
                    assert_eq!(history.len(), 1);
                    assert_eq!(history[0].shown, "にほんご");
                    assert_eq!(history[0].expected, "nihongo");
                    clear(directory.path());
                } else {
                    assert!(recent(directory.path()).is_empty());
                }
            }
            session.no_learning = false;
            for ch in "nihongo".chars() {
                session.key(ch.to_ascii_uppercase() as i32, ch as i32, 0);
            }
            session.key(0x75, 0, 0);
            session.private = true;
            session.commit();
            assert!(recent(directory.path()).is_empty());
        }
    }
}
#[cfg(all(windows, feature = "filesystem"))]
impl Session {
    fn record_report_correction(&mut self, text: &str) {
        if self.private
            || self.no_learning
            || self.reconversion_origin.is_some()
            || self.raw.is_empty()
            || !self.raw.bytes().any(|c| c.is_ascii_alphabetic())
            || self.v130.clipboard.is_some()
            || self.selected_prediction >= 0
            || self.store.is_none()
        {
            return;
        }
        let shown = if self.is_converting() {
            let switched = self.clauses.iter().any(|clause| {
                clause.changed
                    && (clause.english && clause.text().chars().any(crate::linguistics::kana)
                        || !clause.english && clause.raw.as_deref() == Some(clause.text()))
            });
            if !switched {
                return;
            }
            self.clauses
                .iter()
                .filter_map(|clause| clause.candidates.first())
                .cloned()
                .collect::<String>()
        } else {
            if self.function_key == 0 {
                return;
            }
            let explicit = self.explicit.take();
            let key = std::mem::replace(&mut self.function_key, 0);
            let shown = self.display(true);
            self.function_key = key;
            self.explicit = explicit;
            shown
        };
        if shown == text || text.is_empty() {
            return;
        }
        report_corrections::record(
            &self.store.as_ref().unwrap().directory,
            report_corrections::Entry {
                typed: self.raw.clone(),
                shown,
                expected: text.to_owned(),
                before: self.before.clone(),
            },
        );
    }
}
