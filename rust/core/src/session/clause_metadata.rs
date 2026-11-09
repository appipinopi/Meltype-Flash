// SPDX-License-Identifier: GPL-3.0-or-later
//! Provenance of a converter clause within the retained composition units.
use super::*;
impl Session {
    pub(super) fn single_conversion_clause(&self, reading: &str, text: &str) -> Clause {
        let english = !self.uses_kana()
            && self.reconversion_origin.is_none()
            && self.raw.bytes().all(|b| b.is_ascii_alphabetic())
            && self.mixed_reading(true) == self.raw;
        let mut clause = Clause::new(
            if english {
                self.raw.clone()
            } else {
                reading.to_owned()
            },
            vec![text.to_owned()],
        );
        clause.english = english;
        clause.raw =
            (!self.uses_kana() && self.reconversion_origin.is_none()).then(|| self.raw.clone());
        clause.changed = self.chosen_conversion;
        clause.translations = self.translation_candidates.clone();
        clause
    }

    pub(super) fn clause_history_change(clause: &Clause) -> Option<Change> {
        let text = clause.text();
        let translation = clause.translations.iter().any(|t| t == text);
        if translation
            || !clause.english
                && clause.changed
                && clause.reading.encode_utf16().count() >= 2
                && text != clause.reading
                && text != crate::romaji::katakana(&clause.reading)
                && clause.raw.as_deref() != Some(text)
                && !Linguistics::is_version_text(text)
                && !crate::time_candidates::is_dynamic_choice(&clause.reading, text)
        {
            Some(Change::Conversion {
                reading: clause.reading.clone(),
                text: text.into(),
                translation,
            })
        } else {
            None
        }
    }
    pub(super) fn clause_history_changes(&self) -> Vec<Change> {
        self.clauses
            .iter()
            .filter_map(Self::clause_history_change)
            .collect()
    }

    pub(super) fn clause_language_change(clause: &Clause) -> Option<Change> {
        if !clause.english && clause.changed {
            if let Some(raw) = clause
                .raw
                .as_ref()
                .filter(|raw| raw.as_str() == clause.text())
            {
                return Some(Change::Language {
                    raw: raw.clone(),
                    english: true,
                    explicit: false,
                });
            }
        }
        if clause.english
            && clause.reading.bytes().all(|b| b.is_ascii_alphabetic())
            && clause.text().chars().any(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'))
        {
            return Some(Change::Language { raw: clause.reading.clone(), english: false, explicit: false });
        }
        None
    }
    pub(super) fn clause_language_changes(&self) -> Vec<Change> {
        self.clauses
            .iter()
            .filter_map(Self::clause_language_change)
            .collect()
    }
    pub(super) fn clause_original_inputs(&self, clauses: &[Clause]) -> Vec<Option<String>> {
        if self.uses_kana() || self.reconversion_origin.is_some() {
            return vec![None; clauses.len()];
        }
        let reading: String = clauses.iter().map(|c| c.reading.as_str()).collect();
        let captured;
        let units = if let Some(units) = self.edited_units.as_ref().filter(|u| u.raw() == self.raw)
        {
            units
        } else {
            captured = crate::composition_units::Units::capture(&self.raw, &reading);
            &captured
        };
        let (settled, pending) = units.parts();
        let mut pieces: Vec<_> = settled
            .iter()
            .map(|u| (u.kana.clone(), u.spelling.clone()))
            .collect();
        if !pending.is_empty() {
            pieces.push((romaji::hiragana(pending, true), pending.into()));
        }
        let mut remaining = reading.as_str();
        let mut spans = Vec::new();
        let mut offset = 0;
        for (kana, raw) in pieces {
            let punctuated = self.punctuate(romaji::hiragana(&kana, true));
            let Some(output) = [kana.as_str(), raw.as_str(), punctuated.as_str()]
                .into_iter()
                .find(|s| !s.is_empty() && remaining.starts_with(s))
            else {
                return vec![None; clauses.len()];
            };
            let end = offset + output.encode_utf16().count();
            let consumed = output.len();
            spans.push((offset, end, raw));
            offset = end;
            remaining = &remaining[consumed..];
        }
        if !remaining.is_empty() {
            return vec![None; clauses.len()];
        }
        let mut start = 0;
        clauses
            .iter()
            .map(|clause| {
                let end = start + clause.reading.encode_utf16().count();
                let mut value = String::new();
                let mut partial = false;
                for (from, to, raw) in &spans {
                    if *to > start && *from < end {
                        partial |= *from < start || *to > end;
                        value.push_str(raw);
                    }
                }
                start = end;
                (!clause.english && !partial && !value.is_empty()).then_some(value)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_number_key_page_selection_and_clause_advance_match() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/data/upstream-number-keys.json"))
                .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 39);
        for row in cases {
            let c = &row["case"];
            let clauses = c["clauses"].as_u64().unwrap() as usize;
            let active = c["active"].as_u64().unwrap() as usize;
            let mut s = Session {
                raw: if clauses == 1 {
                    "hashi".into()
                } else {
                    "hashikami".into()
                },
                ..Default::default()
            };
            for name in ["LiveConversion", "PredictiveCandidates", "LearningEnabled"] {
                s.settings.set(name, serde_json::json!(false)).unwrap();
            }
            s.settings
                .set("DetectionLevel", serde_json::json!("Manual"))
                .unwrap();
            if c["kana"] == true {
                s.settings
                    .set("InputStyle", serde_json::json!("Kana"))
                    .unwrap();
            }
            s.clauses = (0..clauses)
                .map(|i| {
                    let mut clause = Clause::new(
                        if i == 0 {
                            "はし".into()
                        } else {
                            "かみ".into()
                        },
                        (0..c["count"].as_u64().unwrap())
                            .map(|n| format!("候補{i}_{n}"))
                            .collect(),
                    );
                    if i == active {
                        clause.selected = c["start"].as_u64().unwrap() as usize;
                    }
                    clause
                })
                .collect();
            s.selected_clause = active;
            s.sync_clause();
            let digit = c["row"].as_i64().unwrap() as i32 + 1;
            let result = s.key(
                if c["keypad"] == true {
                    0x60 + digit
                } else {
                    0x30 + digit
                },
                '0' as i32 + digit,
                0,
            );
            let commits: Vec<_> = result.commits.iter().map(|e| serde_json::json!({"deleteBefore":e.delete_before,"text":e.text,"expect":e.expect})).collect();
            let view = result.view.as_ref();
            let mut expected = row["expected"].clone();
            // Selection positions outside conversion are not used by candidate
            // navigation (the original reports -1 while this ABI uses 0).
            // Preserve those original values in the fixture, and compare
            // selection positions whenever the candidate UI is active.
            if !view.is_some_and(|v| v.converting) {
                expected["selectedClause"] = serde_json::json!(0);
                expected["selectedIndex"] = serde_json::json!(0);
            }
            assert_eq!(
                serde_json::json!({"consumed":result.consumed,"commits":commits,"text":view.map(|v|v.text.as_str()),"converting":view.is_some_and(|v|v.converting),"selectedClause":view.map_or(0,|v|v.selected_clause),"selectedIndex":view.map_or(0,|v|v.selected_index)}),
                expected,
                "{}",
                c["name"]
            );
        }
    }
    #[test]
    fn original_single_conversion_and_go_preview_learning_match() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/data/upstream-single-choice.json"))
                .unwrap();
        let cases = golden["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 16);
        for row in cases {
            let c = &row["case"];
            let raw = c["raw"].as_str().unwrap();
            let mut s = Session {
                no_learning: c["private"].as_bool().unwrap(),
                ..Default::default()
            };
            s.settings
                .set("DetectionLevel", serde_json::json!("Manual"))
                .unwrap();
            s.settings
                .set("LiveConversion", serde_json::json!(false))
                .unwrap();
            s.settings
                .set("PredictiveCandidates", serde_json::json!(false))
                .unwrap();
            s.settings
                .set("LearningEnabled", c["learning"].clone())
                .unwrap();
            s.collect_engine_learning(true);
            let mut commits = Vec::new();
            for _ in 0..c["repeats"].as_u64().unwrap() {
                for ch in raw.chars() {
                    s.key(
                        ch.to_ascii_uppercase() as i32,
                        ch as i32,
                        i32::from(ch.is_ascii_uppercase()),
                    );
                }
                if c["forceKana"] == true {
                    s.key(0x75, 0, 0);
                }
                if c["preview"] != true {
                    s.key(0x20, 32, 0);
                    // Keep exercising the flat API after automatic fallback
                    // conversion began returning a structured clause in 1.3.0.
                    s.clauses.clear();
                }
                let view = s.result(false).view.unwrap();
                let index = view
                    .candidates
                    .iter()
                    .position(|candidate| candidate == c["target"].as_str().unwrap())
                    .unwrap_or_else(|| {
                        panic!(
                            "missing target: {} candidates {:?}",
                            c["name"], view.candidates
                        )
                    });
                assert!(s.select_composing(index).consumed);
                let result = s.key(13, 0, 0);
                assert!(s.take_engine_learning().is_empty(), "{}", c["name"]);
                commits.push(
                    result
                        .commits
                        .iter()
                        .map(|e| e.text.as_str())
                        .collect::<String>(),
                );
            }
            assert_eq!(serde_json::json!(commits), row["commits"], "{}", c["name"]);
            let actual: Vec<_> = s.state.language_counts.iter().map(|(word, entry)| serde_json::json!({"word":word,"english":entry.english,"count":entry.count,"active":entry.active(word)})).collect();
            assert_eq!(serde_json::json!(actual), row["expected"], "{}", c["name"]);
        }
    }
    #[test]
    fn original_controller_clause_history_choices_match() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/data/upstream-clause-history.json"
        ))
        .unwrap();
        let cases = golden["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 25);
        for row in cases {
            let c = &row["case"];
            let mut s = Session {
                raw: "fixture".into(),
                private: c["private"].as_bool().unwrap(),
                ..Default::default()
            };
            s.settings
                .set("LearningEnabled", c["learning"].clone())
                .unwrap();
            let reading = c["reading"].as_str().unwrap();
            let mut clause = Clause::new(reading.into(), vec![c["text"].as_str().unwrap().into()]);
            clause.raw = c["raw"].as_str().map(str::to_owned);
            clause.english = c["english"].as_bool().unwrap();
            clause.changed = c["changed"].as_bool().unwrap();
            clause.translations = c["translations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t.as_str().unwrap().into())
                .collect();
            s.clauses = vec![clause];
            for _ in 0..c["repeats"].as_u64().unwrap() {
                s.learn(s.clause_history_changes());
            }
            let translations = s
                .state
                .translations
                .get(reading)
                .cloned()
                .unwrap_or_default();
            assert_eq!(
                serde_json::json!({"selected":s.preferences.selected,"translations":translations}),
                row["expected"],
                "{}",
                c["name"]
            );
        }
    }
    #[test]
    fn only_explicit_clause_choices_enter_conversion_history() {
        let mut s = Session {
            raw: "hashi".into(),
            ..Default::default()
        };
        s.settings
            .set("LearningEnabled", serde_json::json!(true))
            .unwrap();
        assert!(s.begin_clauses(vec![Clause::new("はし".into(), vec!["橋".into()])]));
        s.commit();
        assert!(s.preferences.selected.is_empty());
        s.raw = "hashi".into();
        assert!(s.begin_clauses(vec![Clause::new("はし".into(), vec!["橋".into()])]));
        assert!(s.select_composing(0).consumed);
        s.commit();
        assert_eq!(
            s.preferences.selected.get("はし").map(String::as_str),
            Some("橋")
        );
    }
    #[test]
    fn original_controller_clause_language_choices_match() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/data/upstream-clause-language.json"
        ))
        .unwrap();
        let cases = golden["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 16);
        for row in cases {
            let c = &row["case"];
            let mut s = Session {
                raw: "fixture".into(),
                private: c["private"].as_bool().unwrap(),
                ..Default::default()
            };
            s.settings
                .set("LearningEnabled", c["learning"].clone())
                .unwrap();
            let mut clause = Clause::new(
                c["reading"].as_str().unwrap().into(),
                vec![c["text"].as_str().unwrap().into()],
            );
            clause.raw = c["raw"].as_str().map(str::to_owned);
            clause.english = c["english"].as_bool().unwrap();
            clause.changed = c["changed"].as_bool().unwrap();
            s.clauses = vec![clause];
            for _ in 0..c["repeats"].as_u64().unwrap() {
                s.learn(s.clause_language_changes());
            }
            let actual: Vec<_> = s.state.language_counts.iter().map(|(word, entry)| serde_json::json!({"word":word,"english":entry.english,"count":entry.count,"active":entry.active(word)})).collect();
            assert_eq!(serde_json::json!(actual), row["expected"], "{}", c["name"]);
        }
    }
    #[test]
    fn candidate_selection_marks_only_the_selected_clause() {
        let mut s = Session {
            raw: "hashikami".into(),
            ..Default::default()
        };
        assert!(s.begin_clauses(vec![
            Clause::new("はし".into(), vec!["橋".into(), "hashi".into()]),
            Clause::new("かみ".into(), vec!["紙".into()])
        ]));
        assert!(s.clauses.iter().all(|c| !c.changed));
        let index = s.candidates.iter().position(|c| c == "hashi").unwrap();
        assert!(s.select_composing(index).consumed);
        assert!(s.clauses[0].changed);
        assert!(!s.clauses[1].changed);
        s.settings
            .set("LearningEnabled", serde_json::json!(true))
            .unwrap();
        s.commit();
        assert!(s.state.language_counts["hashi"].english);
        assert!(!s.state.language_counts["hashi"].explicit);
        s.raw = "hashikami".into();
        assert!(s.begin_clauses(vec![
            Clause::new("はし".into(), vec!["橋".into()]),
            Clause::new("かみ".into(), vec!["紙".into()])
        ]));
        s.cycle(1);
        assert!(s.clauses[0].changed);
        s.resize_clause(true);
        assert!(s.clauses.iter().all(|c| !c.changed));
    }
    #[test]
    fn ordinary_input_has_clause_local_raw_and_boundary_edits_clear_it() {
        let mut s = Session {
            raw: "hashikami".into(),
            // Isolate provenance from context ranking: the bundled rules now
            // correctly prefer 端 when another clause contains 紙.
            candidate_sources: std::sync::Arc::new(Default::default()),
            ..Default::default()
        };
        s.settings
            .set("LearningEnabled", serde_json::json!(true))
            .unwrap();
        s.collect_engine_learning(true);
        assert!(s.begin_clauses(vec![
            Clause::new("はし".into(), vec!["hashi".into()]),
            Clause::new("かみ".into(), vec!["紙".into()])
        ]));
        assert_eq!(s.clauses[0].raw.as_deref(), Some("hashi"));
        assert_eq!(s.clauses[1].raw.as_deref(), Some("kami"));
        s.record_engine_learning();
        let learned = s.take_engine_learning();
        assert_eq!(learned.len(), 1);
        assert_eq!(learned[0].clauses[0].text, "紙");
        s.commit();
        assert!(!s.preferences.selected.contains_key("はし"));
        assert!(!s.preferences.selected.contains_key("はしかみ"));
        assert!(!s.preferences.selected.contains_key("かみ"));
        s.raw = "hashikami".into();
        assert!(s.begin_clauses(vec![
            Clause::new("はし".into(), vec!["hashi".into()]),
            Clause::new("かみ".into(), vec!["紙".into()])
        ]));
        s.resize_clause(true);
        assert!(s.clauses.iter().all(|c| c.raw.is_none()));
    }
    #[test]
    fn translations_are_recorded_per_clause_and_not_as_engine_learning() {
        let mut s = Session {
            raw: "hashikami".into(),
            ..Default::default()
        };
        s.settings
            .set("LearningEnabled", serde_json::json!(true))
            .unwrap();
        s.settings
            .set("TranslationCandidates", serde_json::json!(true))
            .unwrap();
        s.collect_engine_learning(true);
        assert!(s.begin_clauses(vec![
            Clause::new("はし".into(), vec!["橋".into()]),
            Clause::new("かみ".into(), vec!["紙".into()])
        ]));
        let word = s.clauses[0]
            .translations
            .first()
            .expect("bundled translation for 橋")
            .clone();
        s.clauses[0].selected = s.clauses[0]
            .candidates
            .iter()
            .position(|t| t == &word)
            .unwrap();
        s.sync_clause();
        s.commit();
        assert!(s.state.translations["はし"][&word] > 0);
        assert_ne!(s.preferences.selected.get("はし"), Some(&word));
        let learned = s.take_engine_learning();
        assert_eq!(learned.len(), 1);
        assert_eq!(learned[0].clauses[0].text, "紙");
    }
    #[test]
    fn original_sound_unit_raw_ranges_match() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/data/upstream-clause-raw.json"))
                .unwrap();
        assert_eq!(golden["cases"].as_array().unwrap().len(), 36);
        let mut checked = 0;
        for row in golden["cases"].as_array().unwrap() {
            let s = Session {
                raw: row["case"]["raw"].as_str().unwrap().into(),
                ..Default::default()
            };
            let segments = row["segments"].as_array().unwrap();
            for (index, segment) in segments.iter().enumerate() {
                let units: Vec<_> = segment["reading"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect();
                for span in segment["spans"].as_array().unwrap() {
                    let start = span["start"].as_u64().unwrap() as usize;
                    let end = start + span["length"].as_u64().unwrap() as usize;
                    // Clause reading is a Rust string; surrogate-half cuts
                    // cannot be represented by this API. Keep those original
                    // rows in the fixture, and test all scalar boundaries.
                    let (Ok(prefix), Ok(target), Ok(suffix)) = (
                        String::from_utf16(&units[..start]),
                        String::from_utf16(&units[start..end]),
                        String::from_utf16(&units[end..]),
                    ) else {
                        continue;
                    };
                    let mut clauses = Vec::new();
                    let mut selected = 0;
                    for (i, part) in segments.iter().enumerate() {
                        if i == index {
                            if !prefix.is_empty() {
                                clauses.push(Clause::new(prefix.clone(), vec![]));
                            }
                            selected = clauses.len();
                            clauses.push(Clause::new(target.clone(), vec![]));
                            if !suffix.is_empty() {
                                clauses.push(Clause::new(suffix.clone(), vec![]));
                            }
                        } else {
                            let reading = part["reading"].as_str().unwrap().to_owned();
                            clauses.push(if part["english"] == true {
                                Clause::literal(reading, true)
                            } else {
                                Clause::new(reading, vec![])
                            });
                        }
                    }
                    let raw = s.clause_original_inputs(&clauses);
                    assert_eq!(
                        serde_json::json!(raw[selected]),
                        span["raw"],
                        "input {:?}, segment {index}, {start}..{end}",
                        row["case"]
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 334);
    }
}
