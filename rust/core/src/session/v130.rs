// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Optional 1.3.0 operations, kept outside the ordinary key path.
use super::*;
#[derive(Default)]
pub(super) struct State {
    pub slash_at_start: bool,
    pub english_conversion: bool,
    pub esc_reverted: bool,
    pub enter_as_shown: bool,
    pub clipboard: Option<String>,
    pub github_title: Option<String>,
}
impl Session {
    /// Only explicit conversion may request this key; never include surrounding input.
    pub fn github_title_request(&self) -> Option<(String, u32)> {
        if !self.settings.boolean("GitHubIssueCandidates")
            || self.private
            || self.no_learning
            || self.direct
            || self.is_converting()
            || !self.settings.boolean("Enabled")
        {
            return None;
        }
        let digits = self.raw.strip_prefix('#')?;
        if digits.is_empty() || digits.len() > 7 || !digits.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let repository = crate::github_titles::normalize(self.settings.text("GitHubRepository"));
        if crate::github_titles::valid(&repository) {
            Some((repository, digits.parse::<u32>().ok().filter(|n| *n > 0)?))
        } else {
            None
        }
    }
    pub fn set_github_title(&mut self, title: Option<String>) {
        self.v130.github_title = if self.github_title_request().is_some() {
            title.filter(|title| !title.is_empty())
        } else {
            None
        };
    }
    /// The host reads the clipboard only when this returns true at explicit conversion.
    pub fn wants_clipboard_candidate(&self) -> bool {
        self.settings.boolean("ClipboardCandidates")
            && !self.private
            && !self.no_learning
            && !self.direct
            && self.settings.boolean("Enabled")
            && !self.is_converting()
            && ["くりっぷぼーど", "ぺーすと"].contains(&self.conversion_reading().as_str())
    }
    /// Supply one conversion's clipboard content. Never persist or log it.
    pub fn set_clipboard_candidate(&mut self, text: Option<&str>) {
        self.v130.clipboard = if self.wants_clipboard_candidate() {
            text.and_then(crate::number_candidates::clipboard_line)
        } else {
            None
        };
    }
    pub(super) fn clipboard_candidates(&self, reading: &str) -> Vec<String> {
        if self.settings.boolean("ClipboardCandidates")
            && ["くりっぷぼーど", "ぺーすと"].contains(&reading)
        {
            self.v130.clipboard.iter().cloned().collect()
        } else {
            vec![]
        }
    }
    /// Bounded diagnostic events contain elapsed time and composing state only.
    pub fn key_intervals(&self) -> &std::collections::VecDeque<serde_json::Value> {
        &self.key_intervals
    }
    pub(super) fn record_key_interval(&mut self, vk: i32, now_ms: u64) {
        if !self.settings.boolean("LogKeyIntervals") || self.private || self.no_learning {
            self.interval_last_ms = None;
            return;
        }
        if matches!(vk,0x10..=0x12|0x5b..=0x5c|0xa0..=0xa5) {
            return;
        }
        if let Some(interval) = self
            .interval_last_ms
            .and_then(|last| now_ms.checked_sub(last))
        {
            let event = serde_json::json!({"intervalMs":interval,"composing":!self.raw.is_empty()});
            self.key_intervals.push_back(event.clone());
            if self.key_intervals.len() > 128 {
                self.key_intervals.pop_front();
            }
            #[cfg(feature = "filesystem")]
            if self.settings.boolean("FileLog") {
                if let Some(store) = &self.store {
                    use std::io::Write;
                    let path = store.directory.join("key-intervals.jsonl");
                    if let Err(error) = (|| -> std::io::Result<()> {
                        if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024) {
                            std::fs::remove_file(&path)?;
                        }
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)?;
                        writeln!(file, "{event}")
                    })() {
                        self.storage_error = Some(error.to_string());
                    }
                }
            }
        }
        self.interval_last_ms = Some(now_ms);
    }
    pub(super) fn memory_context(&self) -> Option<bool> {
        if !self.settings.boolean("ContextualLearning") {
            return None;
        }
        let score = self.dictionary().before_evidence(&self.before);
        if score == 0 {
            None
        } else {
            Some(score > 0)
        }
    }
    pub(super) fn language_scope(&self) -> crate::language_memory::Scope {
        let Some(english) = self.memory_context() else {
            return crate::language_memory::Scope::new(&self.preferences.english);
        };
        let suffix = if english { "@e" } else { "@j" };
        let mut values = self.preferences.english.clone();
        for (key, entry) in &self.state.language_counts {
            if let Some(raw) = key.strip_suffix(suffix) {
                values.remove(raw);
                if entry.active(raw) {
                    values.insert(raw.to_owned(), entry.english);
                }
            }
        }
        crate::language_memory::Scope::new(&values)
    }
    pub(super) fn english_first(&self) -> bool {
        self.settings.boolean("EnglishFirstInput")
            && !self.uses_kana()
            && !self.v130.english_conversion
            && self.function_key == 0
            && !self.protects_tail()
    }
    pub(super) fn v130_key(&mut self, vk: i32, ch: i32, modifiers: i32) -> Option<SessionResult> {
        if self.private || self.direct || self.code_english() || !self.settings.boolean("Enabled") {
            return None;
        }
        if self.v130.slash_at_start
            && self.raw == "/"
            && self.is_converting()
            && self.display(true) == "/"
            && modifiers & 14 == 0
            && char::from_u32(ch as u32).is_some_and(|c| c.is_ascii_graphic())
        {
            let mut result = self.commit();
            result.consumed = false;
            return Some(result);
        }
        if vk == 0x2e && self.selected_prediction >= 0 && !self.is_converting() {
            if let Some(word) = self
                .predictions()
                .get(self.selected_prediction as usize)
                .cloned()
            {
                #[cfg(feature = "filesystem")]
                if let Some(store) = &self.store {
                    match store.update(|state| {
                        state.phrases.retain(|p| p.text != word);
                        Ok(state.clone())
                    }) {
                        Ok(state) => self.state = state,
                        Err(error) => self.storage_error = Some(error.to_string()),
                    }
                } else {
                    self.state.phrases.retain(|p| p.text != word);
                }
                #[cfg(not(feature = "filesystem"))]
                self.state.phrases.retain(|p| p.text != word);
                self.selected_prediction = -1;
                return Some(self.result(true));
            }
        }
        if modifiers & 14 == 2 && !self.raw.is_empty() {
            if let Some((_, mapped)) =
                crate::shortcuts::control_bindings(self.settings.text("ControlKeyBindings"))
                    .into_iter()
                    .find(|(key, _)| *key == vk)
            {
                return Some(self.key(mapped, 0, modifiers & 1));
            }
            if self.settings.boolean("MacControlKeys") {
                if self.is_converting() && matches!(vk, 0x49 | 0x4f) {
                    self.resize_clause(vk == 0x4f);
                    return Some(self.result(true));
                }
                let mapped = match vk {
                    0x4a => Some(0x75),
                    0x4b => Some(0x76),
                    0x4c => Some(0x78),
                    0x48 => Some(8),
                    0x44 => Some(0x2e),
                    0x4e => Some(0x28),
                    0x50 => Some(0x26),
                    0x42 => Some(0x25),
                    0x46 => Some(0x27),
                    0x41 => Some(0x24),
                    0x45 => Some(0x23),
                    _ => None,
                };
                if let Some(key) = mapped {
                    return Some(self.key(key, 0, modifiers & 1));
                }
            }
            if self.settings.boolean("CompatibleShortcuts")
                && self.is_converting()
                && matches!(vk, 0x4b | 0x4c)
            {
                self.resize_clause(vk == 0x4c);
                return Some(self.result(true));
            }
        }
        if modifiers & 14 != 0 {
            return None;
        }
        if self.raw.is_empty() {
            if vk == 0x20
                && (modifiers & 1 != 0 || self.settings.boolean("FullWidthSpaceByDefault"))
            {
                let half = if self.settings.boolean("FullWidthSpaceByDefault") {
                    modifiers & 1 != 0
                } else {
                    self.settings.boolean("ShiftSpaceHalfWidth")
                };
                return Some(SessionResult {
                    consumed: true,
                    commits: vec![TextEdit {
                        caret_back: 0,
                        delete_before: 0,
                        text: if half { " " } else { "　" }.into(),
                        expect: None,
                    }],
                    view: None,
                    selection_expect: None,
                });
            }
            return None;
        }
        if self.english_first() && !self.is_converting() {
            if matches!(vk, 0x1c | 0x28) {
                self.v130.english_conversion = true;
                self.explicit = None;
                self.begin_conversion(vec![]);
                return Some(self.result(true));
            }
            if vk == 0x20 && modifiers & 1 == 0 {
                self.raw.push(' ');
                self.typed_keys.push((vk, ' ', false));
                return Some(self.result(true));
            }
        }
        if matches!(vk, 0x25..=0x28) && !self.is_converting() && self.display(false) != self.raw {
            self.begin_conversion(vec![]);
            if !self.clauses.is_empty() {
                self.selected_clause = if vk == 0x25 {
                    self.clauses.len() - 1
                } else {
                    0
                };
                if vk == 0x26 {
                    self.cycle(-1);
                }
                self.sync_clause();
            }
            return Some(self.result(true));
        }
        if vk == 32
            && !self.is_converting()
            && self
                .raw
                .bytes()
                .all(|c| c.is_ascii_digit() || b"/-:.".contains(&c))
            && self.raw.bytes().any(|c| c.is_ascii_digit())
        {
            self.explicit = Some(self.raw.clone() + " ");
            return Some(self.commit());
        }
        if vk == 13 && self.settings.boolean("EnterCommitsAsShown") && !self.is_converting() {
            self.v130.enter_as_shown = true;
            return Some(self.commit());
        }
        if self.is_converting() && self.clauses.len() > 1 {
            let clause = &self.clauses[self.selected_clause];
            let value = if vk == 27
                && self.settings.boolean("EscRevertsSelectedClause")
                && !clause.english
                && clause.text() != clause.reading
            {
                Some(clause.reading.clone())
            } else if (0x75..=0x79).contains(&vk)
                && self.settings.boolean("FunctionKeysAffectSelectedClause")
            {
                let reading = if clause.english {
                    clause.kana.as_deref()
                } else {
                    Some(clause.reading.as_str())
                };
                let raw = if clause.english {
                    Some(clause.reading.as_str())
                } else {
                    clause.raw.as_deref()
                };
                match vk {
                    0x75 => reading.map(str::to_owned),
                    0x76 => reading.map(romaji::katakana),
                    0x77 => reading.map(|s| crate::kana::halfwidth(&romaji::katakana(s))),
                    // 元の入力がない文節では、選択中の文字と文節の区切りを保つ (#1095)。
                    0x78 => {
                        Some(raw.map_or_else(|| clause.text().to_owned(), crate::kana::fullwidth))
                    }
                    _ => Some(raw.unwrap_or_else(|| clause.text()).to_owned()),
                }
            } else {
                None
            };
            if let Some(value) = value {
                let clause = &mut self.clauses[self.selected_clause];
                let i = clause
                    .candidates
                    .iter()
                    .position(|s| s == &value)
                    .unwrap_or_else(|| {
                        clause.candidates.push(value);
                        clause.candidates.len() - 1
                    });
                clause.selected = i;
                self.sync_clause();
                return Some(self.result(true));
            }
            if (0x75..=0x79).contains(&vk)
                && self.settings.boolean("FunctionKeysAffectSelectedClause")
            {
                // A clause without retained spelling cannot use F9/F10. Keep
                // the conversion intact instead of changing the whole text.
                return Some(self.result(true));
            }
            if vk == 13
                && modifiers & 1 != 0
                && self.settings.boolean("PartialCommit")
                && self.selected_clause + 1 < self.clauses.len()
            {
                let rest = self.clauses.split_off(self.selected_clause + 1);
                let remaining: String = rest
                    .iter()
                    .map(|c| {
                        if c.english {
                            c.raw.as_deref().unwrap_or(&c.reading)
                        } else {
                            &c.reading
                        }
                    })
                    .collect();
                self.raw = self
                    .clauses
                    .iter()
                    .map(|c| c.raw.as_deref().unwrap_or(&c.reading))
                    .collect();
                self.typed_keys = self.raw.chars().map(|c| (0, c, false)).collect();
                let mut result = self.commit();
                self.raw = remaining;
                self.typed_keys = self.raw.chars().map(|c| (0, c, false)).collect();
                result.view = self.result(true).view;
                return Some(result);
            }
        }
        if vk == 27 && self.settings.boolean("EscStepsBack") && !self.is_converting() {
            let before = self.display(false);
            if !(self.function_key == 0 && before == self.raw) {
                for (mode, text) in [
                    (0x75, romaji::hiragana(&self.raw, true)),
                    (0x79, self.raw.clone()),
                ] {
                    if mode == 0x75 && self.function_key == 0x79 {
                        continue;
                    }
                    self.function_key = mode;
                    self.explicit = Some(text.clone());
                    self.v130.esc_reverted = true;
                    if text != before {
                        return Some(self.result(true));
                    }
                }
            }
        }
        let _ = ch;
        None
    }
    pub(super) fn v130_conversion(&mut self) -> bool {
        if self.v130.slash_at_start && self.raw == "/" && self.function_key == 0 {
            self.candidates = ["/", "・", "･", "／"].map(str::to_owned).to_vec();
            self.clauses = vec![Clause::new("・".into(), self.candidates.clone())];
            self.selected = 0;
            self.selected_clause = 0;
            return true;
        }
        if self.uses_kana() || self.function_key != 0 {
            return false;
        }
        let words = if self.settings.boolean("ConsonantAbbreviations")
            && self.raw.len() >= 2
            && self.raw.bytes().all(|c| c.is_ascii_alphabetic())
        {
            self.candidate_sources
                .abbreviations(&self.raw.to_ascii_lowercase())
        } else {
            vec![]
        };
        let words_were_calculator = words.is_empty();
        let mut values = if !words.is_empty() {
            words
        } else if let Some(value) = self
            .settings
            .boolean("CalculatorCandidates")
            .then(|| crate::calculator::evaluate(&self.raw))
            .flatten()
        {
            vec![
                self.raw.clone(),
                value.clone(),
                format!("{}={value}", self.raw.trim_end_matches('=')),
            ]
        } else if self.raw.strip_prefix('#').is_some_and(|digits| {
            !digits.is_empty() && digits.len() <= 7 && digits.bytes().all(|c| c.is_ascii_digit())
        }) {
            let mut values = vec![self.raw.clone()];
            if self.settings.boolean("GitHubIssueCandidates") {
                if let Some(title) = &self.v130.github_title {
                    values.extend([
                        format!("{} {title}", self.raw),
                        format!("{title} ({})", self.raw),
                        format!(
                            "{}{}",
                            crate::github_titles::normalize(self.settings.text("GitHubRepository")),
                            self.raw
                        ),
                    ]);
                }
            }
            values.push(crate::kana::fullwidth(&self.raw));
            values
        } else {
            return false;
        };
        if !words_were_calculator {
            values.push(self.raw.clone());
            values.extend(self.dictionary().english_candidates(&self.raw));
        }
        let mut clause = Clause::new(self.raw.clone(), values);
        clause.english = true;
        clause.raw = Some(self.raw.clone());
        self.clauses = vec![clause];
        self.live_clauses.clear();
        self.selected_clause = 0;
        self.explicit = None;
        self.sync_clause();
        true
    }
}
