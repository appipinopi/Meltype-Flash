// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Level-dependent English spans, following CompositionDetector's evidence rules.
use super::Dictionary;
use crate::{protection, romaji};

#[derive(Clone, Copy)]
struct Evidence<'a> {
    level: &'a str,
    before: i32,
    after: i32,
}
const PARTICLES: &[&str] = &[
    "no", "ga", "wo", "ni", "de", "to", "ha", "mo", "wa", "he", "ya",
];

impl Dictionary {
    pub(super) fn english_particle_pair(&self, raw: &str) -> Option<String> {
        let lower = raw.to_ascii_lowercase();
        for index in 2..raw.len() {
            for particle in PARTICLES {
                if let Some(right) = lower[index..].strip_prefix(particle) {
                    if right.len() >= 2
                        && self.english_atom(&lower[..index])
                        && self.english_atom(right)
                        && (!self.is_japanese(&lower[..index])
                            || self.proper.contains(&lower[..index]))
                        && (!self.is_japanese(right) || self.proper.contains(right))
                    {
                        return Some(format!(
                            "{}{}{}",
                            &raw[..index],
                            romaji::hiragana(particle, true),
                            &raw[index + particle.len()..]
                        ));
                    }
                }
            }
        }
        None
    }
    // Upstream #377/#394: keep game+over together, while excluding common
    // Japanese heads, already-known whole words and arbitrary syllable splits.
    pub(crate) fn english_compound_end(&self, raw: &str) -> Option<usize> {
        if raw.len() < 7 || !raw.bytes().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        let lower = raw.to_ascii_lowercase();
        let common = |word: &str| crate::typo::Corrector::bundled().is_common_japanese(word);
        if self.known_english(&lower)
            || common(&lower)
            || crate::language_memory::get(&lower) == Some(false)
        {
            return None;
        }
        for split in 4..=lower.len() - 3 {
            let head = &lower[..split];
            if !self.english_index.contains(head) {
                break;
            }
            let reading = romaji::analyze(head, false);
            if !self.is_english(head)
                || common(head)
                || self.is_japanese(head)
                || !reading.valid
                || !reading.partial.is_empty()
            {
                continue;
            }
            for end in (split + 3..=lower.len()).rev() {
                if end < lower.len() && !PARTICLES.iter().any(|p| lower[end..].starts_with(p)) {
                    continue;
                }
                let word = &lower[split..end];
                let reading = romaji::analyze(word, false);
                if self.is_english(word)
                    && !(reading.valid && ["", "n"].contains(&reading.partial.as_str()))
                {
                    return Some(end);
                }
            }
        }
        None
    }
    pub(crate) fn composition_kana(
        &self,
        raw: &str,
        kana: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
    ) -> String {
        let raw_bounds: Vec<_> = raw
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(raw.len()))
            .collect();
        let kana_bounds: Vec<_> = kana
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(kana.len()))
            .collect();
        if raw_bounds.len() != kana_bounds.len() {
            return kana.into();
        }
        static JAPANESE: std::sync::OnceLock<super::PrefixIndex> = std::sync::OnceLock::new();
        let japanese = self.custom_kana.as_ref().unwrap_or_else(|| {
            JAPANESE.get_or_init(|| {
                let _builtins = romaji::Scope::new(None);
                super::PrefixIndex::new(
                    &self
                        .japanese
                        .iter()
                        .map(|word| romaji::hiragana(word, true))
                        .collect(),
                )
            })
        });
        let mut out = String::new();
        let mut i = 0;
        let mut previous = self.before_evidence(before);
        while i + 1 < raw_bounds.len() {
            let chosen = (i + 1..raw_bounds.len()).rev().find(|j| {
                let span = &raw[raw_bounds[i]..raw_bounds[*j]];
                if !span.bytes().all(|c| c.is_ascii_alphabetic()) {
                    return false;
                }
                let lower = span.to_ascii_lowercase();
                let at_end = *j + 1 == raw_bounds.len();
                let listed = self.common_english.contains(&lower)
                    || lower.len() >= 4 && self.proper.contains(&lower);
                let word = listed || lower.len() >= 4 && self.spell_word(&lower);
                let prefix = at_end
                    && !final_input
                    && level == "Aggressive"
                    && lower.len() >= 4
                    && self.common_index.contains(&lower);
                if span.starts_with(|c: char| c.is_ascii_uppercase())
                    && ((word || prefix) && span.len() >= 2 || at_end)
                {
                    return true;
                }
                if level == "Manual" {
                    return false;
                }
                if let Some(choice) = crate::language_memory::get(&lower) {
                    return choice;
                }
                if !(word || prefix) || lower.len() < 2 {
                    return false;
                }
                let following = if at_end {
                    self.after_evidence(after)
                } else {
                    -1
                };
                let context = if following < 0 {
                    previous.min(1)
                } else {
                    previous
                } + following;
                if context >= if level == "Conservative" { 2 } else { 1 } {
                    return true;
                }
                let reading = &kana[kana_bounds[i]..kana_bounds[*j]];
                if japanese.contains(reading) {
                    return false;
                }
                if context < 0 {
                    return (listed && lower.len() >= 4 || word && lower.len() >= 6)
                        && !(0..reading.len())
                            .filter(|at| reading.is_char_boundary(*at))
                            .any(|at| {
                                reading[at..].chars().count() >= 3
                                    && japanese.contains(&reading[at..])
                            });
                }
                lower.len()
                    >= match level {
                        "Aggressive" => 2,
                        "Conservative" => 4,
                        _ => 3,
                    }
            });
            if let Some(j) = chosen {
                out.push_str(&raw[raw_bounds[i]..raw_bounds[j]]);
                i = j;
                previous = 1;
            } else {
                out.push_str(&kana[kana_bounds[i]..kana_bounds[i + 1]]);
                i += 1;
                previous = -1;
            }
        }
        out
    }
    fn side_character(c: char) -> Option<i32> {
        if c.is_ascii_alphanumeric() {
            Some(1)
        } else if c >= '\u{2000}' && !c.is_whitespace() {
            Some(-1)
        } else {
            None
        }
    }
    fn english_sentence(before: &str) -> bool {
        if !before.ends_with(' ') {
            return false;
        }
        let start = before
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_ascii() || matches!(c, '\r' | '\n'))
            .map_or(0, |(i, c)| i + c.len_utf8());
        let line_start = start == 0 || before[..start].ends_with(['\r', '\n']);
        let words: Vec<_> = before[start..]
            .split(' ')
            .filter(|w| !w.is_empty())
            .collect();
        if let [word] = words.as_slice() {
            let lower = word.trim_end_matches([',', '!', '.']).to_ascii_lowercase();
            if line_start
                && [
                    "hey", "hi", "hello", "oh", "wow", "yeah", "yes", "well", "so", "hmm", "ah",
                    "ooh", "oops", "thanks", "sorry", "please", "dear", "yay", "whoa", "nope",
                    "yep",
                ]
                .contains(&lower.as_str())
                || [
                    "oh", "wow", "yeah", "ooh", "oops", "whoa", "woah", "yay", "hmm", "hey",
                ]
                .contains(&lower.as_str())
            {
                return true;
            }
            if line_start
                && word.split_once('\'').is_some_and(|(a, b)| {
                    !a.is_empty()
                        && a.bytes().all(|c| c.is_ascii_alphabetic())
                        && (1..=2).contains(&b.len())
                        && b.bytes().all(|c| c.is_ascii_alphabetic())
                })
            {
                return true;
            }
        }
        words.len() >= 2
            && words.iter().all(|w| {
                w.bytes().any(|c| c.is_ascii_alphabetic())
                    && w.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b",.'-!?:;".contains(&c))
            })
    }
    pub(crate) fn before_evidence(&self, before: &str) -> i32 {
        for c in before.chars().rev() {
            if let Some(score) = Self::side_character(c) {
                return if score == 1 && Self::english_sentence(before) {
                    2
                } else {
                    score
                };
            }
            if !c.is_ascii() && !c.is_whitespace() {
                break;
            }
        }
        0
    }
    pub(crate) fn after_evidence(&self, after: &str) -> i32 {
        for c in after.chars() {
            if matches!(c, '\r' | '\n') {
                break;
            }
            if let Some(score) = Self::side_character(c) {
                return score;
            }
            if !c.is_ascii() && !c.is_whitespace() {
                break;
            }
        }
        0
    }
    pub fn composition(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
    ) -> String {
        let compute = || self.composition_uncached(raw, final_input, before, after, level);
        match romaji::probes::Cache::current() {
            Some(cache) => cache.composition(raw, final_input, before, after, level, compute),
            None => compute(),
        }
    }
    fn composition_uncached(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
    ) -> String {
        if level != "Manual" {
            if let Some(end) = self.english_compound_end(raw) {
                return format!(
                    "{}{}",
                    &raw[..end],
                    self.composition(&raw[end..], final_input, &raw[..end], after, level)
                );
            }
        }
        if !["Aggressive", "Conservative", "Manual"].contains(&level)
            && !crate::language_memory::relevant(raw)
            && self.custom_kana.is_none()
        {
            // IsEnglishSpan requires two sides for a two-letter spelling
            // which also reads as Japanese. One preceding Latin word is not
            // an English sentence (k|ai after a direct-mode change).
            if raw.len() == 2 && raw.bytes().all(|b| b.is_ascii_lowercase()) {
                let analysis = romaji::analyze(raw, false);
                if analysis.valid
                    && ["", "n"].contains(&analysis.partial.as_str())
                    && self.before_evidence(before) + self.after_evidence(after) < 2
                    && !(before.ends_with(char::is_whitespace) && self.english_context(before))
                {
                    return self.mixed_context(raw, final_input, false);
                }
            }
            return self.mixed_context_evidence(
                raw,
                final_input,
                self.before_evidence(before),
                self.after_evidence(after),
                level,
            );
        }
        if raw.starts_with('$') && raw.bytes().all(|c| c.is_ascii_graphic()) {
            return raw.into();
        }
        protection::transform(raw, |part| {
            let mut result = String::new();
            let mut cursor = 0;
            while cursor < part.len() {
                let ch = part[cursor..].chars().next().unwrap();
                if ch.is_ascii_alphabetic() {
                    let end = part[cursor..]
                        .find(|c: char| !c.is_ascii_alphabetic())
                        .map_or(part.len(), |n| cursor + n);
                    let history = format!("{before}{result}");
                    let next = if end == part.len() {
                        after
                    } else {
                        &part[end..]
                    };
                    result.push_str(&self.composition_word(
                        &part[cursor..end],
                        final_input && end == part.len(),
                        Evidence {
                            level,
                            before: self.before_evidence(&history),
                            after: self.after_evidence(next),
                        },
                    ));
                    cursor = end;
                } else {
                    result.push_str(&romaji::hiragana(&ch.to_string(), true));
                    cursor += ch.len_utf8();
                }
            }
            result
        })
    }
    fn allows_english(
        &self,
        raw: &str,
        at_end: bool,
        final_input: bool,
        evidence: Evidence<'_>,
        start: bool,
        next: &str,
    ) -> bool {
        let analysis = romaji::analyze(&raw.to_ascii_lowercase(), false);
        let unreadable = !analysis.valid
            || !at_end
                && !analysis.partial.is_empty()
                && !["n", "nn"].contains(&analysis.partial.as_str());
        self.scored_english(
            raw,
            at_end,
            final_input,
            evidence.before,
            evidence.after,
            start,
            unreadable,
            next,
            evidence.level,
        )
    }
    fn composition_word(&self, raw: &str, final_input: bool, evidence: Evidence<'_>) -> String {
        let lower = raw.to_ascii_lowercase();
        // Preserve an explicitly capitalized word, separating an attached particle.
        if raw.starts_with(|c: char| c.is_ascii_uppercase()) {
            for end in (2..raw.len()).rev() {
                let tail = &lower[end..];
                if self.is_english(&raw[..end])
                    && PARTICLES.iter().any(|p| tail.starts_with(p))
                    && romaji::analyze(tail, true).valid
                {
                    return format!(
                        "{}{}",
                        &raw[..end],
                        romaji::hiragana(&raw[end..], final_input)
                    );
                }
            }
            return raw.into();
        }
        if evidence.level == "Manual" {
            if let Some(index) = raw.find(|c: char| c.is_ascii_uppercase()) {
                return format!(
                    "{}{}",
                    romaji::hiragana(&raw[..index], true),
                    self.composition_word(&raw[index..], final_input, evidence)
                );
            }
            return romaji::hiragana(raw, final_input);
        }
        if self.allows_english(raw, true, final_input, evidence, true, "") {
            return raw.into();
        }
        if (self.is_english(raw) || self.is_japanese(raw))
            && crate::language_memory::get(&lower) != Some(false)
        {
            return romaji::hiragana(raw, final_input);
        }
        // Keep Japanese spans together while allowing confident English islands
        // (googleで検索, 日本語の後の unreadable words) in Conservative.
        let n = raw.len();
        let (bounds, pending_start) = romaji::unit_boundaries(
            &lower,
            |prefix| {
                (0..prefix.len().saturating_sub(2)).any(|start| {
                    let word = &prefix[start..];
                    self.common_english.contains(word)
                        || self.proper.contains(word)
                        || self.readable_english.contains(word)
                        || word.len() >= 4 && self.known_english(word)
                })
            },
            |word| self.known_english(word),
        );
        let mut costs = vec![f64::INFINITY; n + 1];
        let mut paths = vec![(0, false); n + 1];
        let after_scores: Vec<_> = (0..=n)
            .map(|end| if end == n { evidence.after } else { -1 })
            .collect();
        costs[0] = 0.;
        for start in 0..n {
            for end in start + 1..=n.min(start + 40) {
                let piece = &raw[start..end];
                let before = if start == 0 {
                    evidence.before
                } else if paths[start].1 {
                    1
                } else {
                    -1
                };
                let english = bounds[start]
                    && bounds[end]
                    && (pending_start != Some(end) || end == n)
                    && self.allows_english(
                        piece,
                        end == n,
                        final_input,
                        Evidence {
                            before,
                            after: after_scores[end],
                            ..evidence
                        },
                        start == 0,
                        &raw[end..],
                    );
                let cost = if english {
                    1.
                } else if self.is_japanese(piece) || PARTICLES.contains(&piece) {
                    0.3
                } else if end == start + 1 {
                    2.
                } else {
                    continue;
                };
                if costs[start] + cost < costs[end] {
                    costs[end] = costs[start] + cost;
                    paths[end] = (start, english);
                }
            }
        }
        let mut pieces = vec![];
        let mut end = n;
        while end > 0 {
            let (start, english) = paths[end];
            pieces.push((start, end, english));
            end = start;
        }
        let mut out = String::new();
        let mut japanese = String::new();
        for (start, end, english) in pieces.into_iter().rev() {
            if english {
                out.push_str(&romaji::hiragana(&japanese, true));
                japanese.clear();
                out.push_str(&raw[start..end]);
            } else {
                japanese.push_str(&raw[start..end]);
            }
        }
        out.push_str(&romaji::hiragana(&japanese, final_input));
        out
    }
}
