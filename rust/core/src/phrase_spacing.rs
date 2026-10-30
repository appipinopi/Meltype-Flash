// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 hrmcngs, appipinopi
//! The upstream EnglishPhraseSpacing token rules and embedded word boundaries.
use std::{collections::HashMap, sync::OnceLock};
fn boundaries() -> &'static HashMap<String, Vec<usize>> {
    static BOUNDARIES: OnceLock<HashMap<String, Vec<usize>>> = OnceLock::new();
    BOUNDARIES.get_or_init(|| {
        crate::system_data::text("english-phrases.txt")
            .lines()
            .filter_map(|line| {
                if line.starts_with('#') {
                    return None;
                }
                let (key, phrase) = line.trim_end_matches('\r').split_once('\t')?;
                if phrase.contains('\t') {
                    return None;
                }
                let words: Vec<_> = phrase.split(' ').filter(|part| !part.is_empty()).collect();
                (words.concat() == key).then(|| {
                    (
                        key.to_ascii_lowercase(),
                        words.iter().map(|word| word.len()).collect(),
                    )
                })
            })
            .collect()
    })
}
fn token(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_-+#.'@/:".contains(c)
}
pub(crate) fn is_phrase(lower: &str) -> bool {
    boundaries().contains_key(lower)
}
pub fn format(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        let ch = text[at..].chars().next().unwrap();
        if !token(ch) {
            result.push(ch);
            at += ch.len_utf8();
            continue;
        }
        let end = text[at..]
            .find(|c: char| !token(c))
            .map_or(text.len(), |n| at + n);
        let word = &text[at..end];
        if let Some(lengths) = boundaries().get(&word.to_ascii_lowercase()) {
            let mut offset = 0;
            for &length in lengths {
                if offset != 0 {
                    result.push(' ');
                }
                result.push_str(&word[offset..offset + length]);
                offset += length;
            }
        } else {
            result.push_str(word);
        }
        at = end;
    }
    result
}
/// 数値だけのトークンと日本語の境界を空ける。識別子や構造化入力は保つ。
pub fn space_numbers(text: &str, before: &str, after: &str) -> String {
    fn japanese(c: char) -> bool {
        crate::linguistics::kana(c) || crate::linguistics::kanji(c)
    }
    fn numeric(text: &str) -> bool {
        text.bytes().any(|b| b.is_ascii_digit())
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || b".,+-/%".contains(&b))
    }
    fn token(c: char) -> bool {
        c.is_ascii_graphic()
    }
    let protected = crate::protection::scan(text);
    let mut output = String::with_capacity(text.len());
    let mut offset = 0;
    let mut previous = before.chars().next_back();
    let before_number = before
        .rsplit(|c: char| !token(c))
        .next()
        .is_some_and(numeric);
    if text.chars().next().is_some_and(japanese) && before_number {
        output.push(' ');
    }
    while offset < text.len() {
        let c = text[offset..].chars().next().unwrap();
        if !token(c) {
            output.push(c);
            previous = Some(c);
            offset += c.len_utf8();
            continue;
        }
        let end = text[offset..]
            .find(|c: char| !token(c))
            .map_or(text.len(), |i| offset + i);
        let word = &text[offset..end];
        let number = numeric(word)
            && !protected
                .iter()
                .any(|s| s.kind != "number" && s.start < end && offset < s.end);
        if number && previous.is_some_and(japanese) {
            output.push(' ');
        }
        output.push_str(word);
        let next = text[end..].chars().next().or_else(|| after.chars().next());
        if number && next.is_some_and(japanese) {
            output.push(' ');
        }
        previous = word.chars().next_back();
        offset = end;
    }
    let after_number = after.split(|c: char| !token(c)).next().is_some_and(numeric);
    if text.chars().next_back().is_some_and(japanese) && after_number {
        output.push(' ');
    }
    output
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_case_preserving_boundaries_and_structural_tokens() {
        for (input, expected) in [
            ("seeyouagain", "see you again"),
            ("SeeYouAgain!", "See You Again!"),
            ("日本語THANKYOUまたね", "日本語THANK YOUまたね"),
            (
                "@seeyouagain https://thankyou seeyouagain.",
                "@seeyouagain https://thankyou seeyouagain.",
            ),
        ] {
            assert_eq!(format(input), expected);
        }
    }
}
