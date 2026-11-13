// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! ReadingLattice: 日本語の語・英単語・未知語をノードにした Viterbi。
//!
//! 上流 v1.5.0 `ReadingLattice.cs` のコスト・状態・連接規則を Rust の
//! `Dictionary` / ローマ字単位へ接続した実装。記号・ドメイン・ユーザー学習の
//! 一部は既存の `composition` 経路へフォールバックする。
use super::packed_words;
use super::Dictionary;
use crate::romaji;
use std::collections::BTreeMap;

const JAPANESE_WORD_COST: f64 = 1.0;
const KATAKANA_WORD_COST: f64 = 1.1;
const UNKNOWN_KANA_COST: f64 = 3.0;
const LIGHT_KANA_COST: f64 = 0.5;
const DANGLING_SOKUON_COST: f64 = UNKNOWN_KANA_COST - LIGHT_KANA_COST;
const KNOWN_ENGLISH_COST: f64 = 1.0;
const UNKNOWN_LETTER_COST: f64 = 1.5;
const NAME_LETTER_COST: f64 = 0.5;
const TECH_ENGLISH_BOOST: f64 = 0.8;

// Technical words that are commonly mixed into Japanese sentences. They are
// often also romaji-readable, so prefer the English spelling in context.
const TECH_ENGLISH: &[&str] = &[
    "main", "push", "pull", "commit", "merge", "branch", "issue", "repo", "release", "deploy",
    "test", "build", "server", "client", "app", "api", "bug", "fix", "log", "file", "user",
    "code", "line", "page", "link", "site", "tool", "mode", "type", "size", "style", "image",
    "video", "music", "data", "date", "time", "cache", "token", "query", "table", "index",
];
const SWITCH_COST: f64 = 1.0;
const SENTENCE_SWITCH_COST: f64 = 2.0;
const ADJACENT_ENGLISH_COST: f64 = 1.0;
const SPELLED_SOKUON_COST: f64 = 2.0;
const VOWEL_AFTER_ENGLISH_COST: f64 = 1.0;
const C_ROW_COST: f64 = 0.5;
const ENGLISH_TIE_BREAK: f64 = 1e-6;
const MAX_SPAN: usize = 48;

const PARTICLES: &[&str] = &[
    "が", "を", "に", "で", "と", "の", "は", "も", "へ", "や", "から", "まで", "より", "って",
    "だ", "です",
];

const SURU_FORMS: &[&str] = &[
    "する", "すれ", "した", "して", "しま", "しな", "しよ", "しと", "しちゃ", "しろ", "され",
    "させ", "せず",
];

fn is_noun_particle(kana: &str) -> bool {
    PARTICLES.contains(&kana)
        || SURU_FORMS.iter().any(|form| kana.starts_with(form))
        || matches!(kana, "し" | "さ" | "せ")
}

fn is_kana_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| ('ぁ'..='ゖ').contains(&c) || c == 'ー')
}

fn is_small_kana(c: char) -> bool {
    matches!(c, 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ')
}

fn unknown_kana_cost(kana: &str) -> f64 {
    kana.chars()
        .enumerate()
        .map(|(index, c)| {
            if c == 'っ' || c == 'ー' {
                LIGHT_KANA_COST
            } else if index > 0 && is_small_kana(c) {
                0.0
            } else {
                UNKNOWN_KANA_COST
            }
        })
        .sum()
}

const HYPHEN_PREFIXES: &[&str] = &[
    "e", "x", "re", "co", "ex", "non", "anti", "semi", "multi", "pre", "sub", "post", "mid",
    "self", "well",
];

const HYPHENATED_WORDS: &[&str] = &[
    "bio-break", "co-op", "re-do", "x-ray", "t-shirt", "wi-fi", "hi-fi", "lo-fi", "sci-fi",
    "e-book", "e-sports", "k-pop", "j-pop", "j-rock", "j-core", "p-hub", "talk-admin", "r-18",
    "sub-6", "gpt-6.7", "a-z", "u-turn", "check-in", "log-in", "sign-in", "add-on", "plug-in",
    "built-in", "follow-up", "set-up", "pop-up", "drop-down", "blu-ray", "binge-watch",
    "ad-blocker",
];

const TOP_LEVEL_DOMAINS: &[&str] = &[
    "ai", "app", "au", "biz", "ca", "cn", "co", "com", "de", "dev", "edu", "eu", "fr", "gg",
    "gov", "info", "in", "io", "jp", "kr", "me", "moe", "net", "org", "uk", "us", "xyz",
];

const FILE_EXTENSIONS: &[&str] = &[
    "a", "ace", "ada", "ado", "ann", "ape", "ase", "dae", "di", "do", "fifo", "fon", "geo",
    "go", "ifo", "ii", "ini", "ino", "ipa", "iso", "jade", "ko", "ma", "metadata", "midi",
    "mime", "mo", "mobi", "o", "oga", "ora", "oso", "otio", "po", "pyo", "ra", "re", "rego",
    "rei", "resi", "ru", "sami", "so", "suo", "tese", "ufo", "vue", "dll", "json", "txt",
    "md", "html", "css", "js", "ts", "rs", "py", "exe", "zip", "png", "jpg", "gif", "pdf",
];

fn ascii_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn known_domain_suffix(value: &str) -> bool {
    TOP_LEVEL_DOMAINS.contains(&value.to_ascii_lowercase().as_str())
}

fn known_file_extension(value: &str) -> bool {
    FILE_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str())
}

fn consume_while(raw: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    for (offset, c) in raw[start..].char_indices() {
        if !predicate(c) {
            break;
        }
        end = start + offset + c.len_utf8();
    }
    end
}

fn url_end(raw: &str, start: usize) -> Option<usize> {
    let rest = &raw[start..];
    if !(rest.starts_with("http://") || rest.starts_with("https://")) {
        return None;
    }
    Some(
        start
            + rest
                .char_indices()
                .find(|(_, c)| c.is_whitespace() || *c == '。' || *c == '、')
                .map_or(rest.len(), |(offset, _)| offset),
    )
}

fn email_end(raw: &str, start: usize) -> Option<usize> {
    let local_end = consume_while(raw, start, |c| ascii_word(c) || c == '.');
    if local_end == start || raw.as_bytes().get(local_end) != Some(&b'@') {
        return None;
    }
    let domain_start = local_end + 1;
    let domain_end = consume_while(raw, domain_start, |c| ascii_alnum(c) || c == '.' || c == '-');
    let domain = &raw[domain_start..domain_end];
    let dot = domain.rfind('.')?;
    known_domain_suffix(&domain[dot + 1..]).then_some(domain_end)
}

fn user_name_end(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'@') {
        return None;
    }
    let end = consume_while(raw, start + 1, |c| ascii_word(c) || c == '.');
    (end > start + 1).then_some(end)
}

fn tag_end(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'#') {
        return None;
    }
    let end = consume_while(raw, start + 1, |c| ascii_word(c));
    (end > start + 1).then_some(end)
}

fn dotfile_end(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'.') {
        return None;
    }
    let name_end = consume_while(raw, start + 1, |c| ascii_word(c));
    (name_end > start + 1).then_some(name_end)
}

fn file_end(raw: &str, start: usize) -> Option<usize> {
    let stem_end = consume_while(raw, start, |c| c.is_ascii_alphabetic());
    if stem_end == start || raw.as_bytes().get(stem_end) != Some(&b'.') {
        return None;
    }
    let ext_start = stem_end + 1;
    let ext_end = consume_while(raw, ext_start, |c| c.is_ascii_alphabetic());
    let ext = &raw[ext_start..ext_end];
    known_file_extension(ext).then_some(ext_end)
}

fn domain_end(raw: &str, start: usize, known_stem: &impl Fn(&str) -> bool) -> Option<usize> {
    let stem_end = consume_while(raw, start, |c| c.is_ascii_alphabetic());
    if stem_end == start || raw.as_bytes().get(stem_end) != Some(&b'.') {
        return None;
    }
    if !known_stem(&raw[start..stem_end]) {
        return None;
    }
    let suffix_start = stem_end + 1;
    let suffix_end = consume_while(raw, suffix_start, |c| c.is_ascii_alphabetic());
    known_domain_suffix(&raw[suffix_start..suffix_end]).then_some(suffix_end)
}

fn hyphenated_end(
    raw: &str,
    start: usize,
    known_stem: &impl Fn(&str) -> bool,
) -> Option<usize> {
    let end = consume_while(raw, start, |c| ascii_word(c));
    if end == start {
        return None;
    }
    let word = &raw[start..end];
    let lower = word.to_ascii_lowercase();
    if HYPHENATED_WORDS.contains(&lower.as_str()) {
        return Some(end);
    }
    let dash = lower.find('-')?;
    if lower[dash + 1..].contains('-') {
        return None;
    }
    let prefix = &lower[..dash];
    let rest = &lower[dash + 1..];
    (HYPHEN_PREFIXES.contains(&prefix)
        && rest.chars().count() >= 3
        && rest.chars().all(|c| c.is_ascii_lowercase())
        && known_stem(rest))
    .then_some(end)
}

fn numeronym_end(raw: &str, start: usize) -> Option<usize> {
    let head_end = consume_while(raw, start, |c| c.is_ascii_alphabetic());
    let head = &raw[start..head_end];
    if head.len() < 1 || head.len() > 2 {
        return None;
    }
    let digit_end = consume_while(raw, head_end, |c| c.is_ascii_digit());
    let digits = &raw[head_end..digit_end];
    if digits.len() < 1 || digits.len() > 2 {
        return None;
    }
    let tail_end = consume_while(raw, digit_end, |c| c.is_ascii_alphabetic());
    let tail = &raw[digit_end..tail_end];
    if tail.len() < 1 || tail.len() > 3 {
        return None;
    }
    let word = &raw[start..tail_end];
    if word.len() < 3 || word.len() > 6 {
        return None;
    }
    // A number between two romaji-readable kana readings is Japanese,
    // not a numeronym (huusoku 6 me-toru).
    let head_kana = crate::romaji::hiragana(&head.to_ascii_lowercase(), true);
    let tail_kana = crate::romaji::hiragana(&tail.to_ascii_lowercase(), true);
    if head.len() >= 2
        && tail.len() >= 2
        && !head_kana.is_empty()
        && !tail_kana.is_empty()
        && is_kana_text(&head_kana)
        && is_kana_text(&tail_kana)
    {
        return None;
    }
    Some(tail_end)
}

fn proper_noun_with_digits_end(raw: &str, start: usize) -> Option<usize> {
    let letters = consume_while(raw, start, |c| c.is_ascii_alphabetic());
    if letters == start {
        return None;
    }
    let stem = &raw[start..letters];
    let digit_end = consume_while(raw, letters, |c| c.is_ascii_digit());
    if digit_end == letters {
        return None;
    }
    // Exact upstream rule: propernouns.txt entries such as yachi8000.
    let candidate = &raw[start..digit_end];
    let listed = proper_nouns().contains(&candidate.to_ascii_lowercase())
        && candidate.chars().any(|c| c.is_ascii_digit())
        && candidate.chars().any(|c| c.is_ascii_alphabetic());
    // Name-like fallback for names not in the bundled list (iPhone15, MP3,
    // Windows11, abc123). A long lowercase romaji reading before the digits is
    // left to the lattice so Japanese + English can be chosen normally.
    let upper_at = stem
        .char_indices()
        .find(|(_, c)| c.is_ascii_uppercase())
        .map(|(byte, _)| stem[..byte].chars().count());
    let stem_len = stem.chars().count();
    let name_like = stem_len >= 3 && (upper_at == Some(0) || upper_at == Some(1));
    let short_token = stem_len <= 5 && stem.chars().all(|c| c.is_ascii_lowercase());
    if !(listed || name_like || short_token) {
        return None;
    }
    let digits = consume_while(raw, letters, |c| c.is_ascii_digit());
    (digits > letters).then_some(digits)
}

const PRERELEASE_NAMES: &[&str] = &[
    "alpha", "beta", "canary", "dev", "edge", "experimental", "final", "hotfix", "insiders",
    "lts", "next", "nightly", "pre", "preview", "rc", "release", "snapshot", "stable", "test",
    "twilight", "unstable",
];

fn prerelease_end(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'-') {
        return None;
    }
    let rest = raw[start + 1..].to_ascii_lowercase();
    let name = PRERELEASE_NAMES
        .iter()
        .filter(|name| rest.starts_with(**name))
        .max_by_key(|name| name.len())?;
    let mut end = start + 1 + name.len();
    end = consume_while(raw, end, |c| c.is_ascii_digit());
    while end < raw.len() && raw.as_bytes()[end] == b'.' {
        let next = consume_while(raw, end + 1, |c| c.is_ascii_digit());
        if next == end + 1 {
            break;
        }
        end = next;
    }
    Some(end)
}

fn version_end(raw: &str, start: usize) -> Option<usize> {
    let mut at = start;
    if raw.as_bytes().get(at) == Some(&b'v') || raw.as_bytes().get(at) == Some(&b'V') {
        at += 1;
    }
    let first = consume_while(raw, at, |c| c.is_ascii_digit());
    if first == at {
        return None;
    }
    let mut end = first;
    let mut parts = 0;
    while end < raw.len() && raw.as_bytes()[end] == b'.' {
        let next = consume_while(raw, end + 1, |c| c.is_ascii_digit());
        if next == end + 1 {
            break;
        }
        end = next;
        parts += 1;
    }
    if parts == 0 {
        return None;
    }
    if let Some(prerelease) = prerelease_end(raw, end) {
        end = prerelease;
    }
    Some(end)
}

fn proper_nouns() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        crate::system_data::text("propernouns.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_ascii_lowercase)
            .collect()
    })
}

fn dot_suffix_end(raw: &str, start: usize, known_stem: &impl Fn(&str) -> bool) -> Option<usize> {
    fn part(c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-')
    }
    let mut end = consume_while(raw, start, part);
    while end + 1 < raw.len()
        && matches!(raw.as_bytes()[end], b'.' | b'/' | b'-')
        && raw[end + 1..].chars().next().is_some_and(part)
    {
        end += 1;
        end = consume_while(raw, end, part);
    }
    let name = &raw[start..end];
    if !name.chars().any(|c| c.is_ascii_alphabetic())
        || (!name.contains('.') && !name.contains('/'))
    {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let last = lower
        .rsplit(['.', '/'])
        .next()
        .unwrap_or("");
    let known = |label: &str| known_domain_suffix(label) || known_file_extension(label);
    let separators = lower.matches(['.', '/', '_', '+', '-']).count();
    let stem = lower.split(['.', '/']).next().unwrap_or("");
    // A bare `stem.tld` is a domain only when the stem is a known/readable
    // English word (avoid turning a Japanese reading + example.com into one).
    if separators == 1 && lower.matches('.').count() == 1 && !known_stem(stem) {
        return None;
    }
    if known(&lower) || known(last) {
        return Some(end);
    }
    if lower
        .chars()
        .any(|c| c.is_ascii_digit() || matches!(c, '.' | '/' | '_' | '+'))
    {
        return Some(end);
    }
    None
}

fn user_name_end_with_underscore(raw: &str, start: usize) -> Option<usize> {
    let end = consume_while(raw, start, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '.'
    });
    let name = &raw[start..end];
    (end > start && name.contains('_') && name.chars().any(|c| c.is_ascii_alphabetic()))
        .then_some(end)
}

fn contraction_end(raw: &str, start: usize) -> Option<usize> {
    let stem_end = consume_while(raw, start, |c| c.is_ascii_alphabetic());
    if stem_end == start || raw.as_bytes().get(stem_end) != Some(&b'\'') {
        return None;
    }
    let stem = raw[start..stem_end].to_ascii_lowercase();
    if stem.is_empty() || !stem.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let suffix_end = consume_while(raw, stem_end + 1, |c| c.is_ascii_alphabetic());
    let suffix = raw[stem_end + 1..suffix_end].to_ascii_lowercase();
    let valid = match suffix.as_str() {
        "" => stem.chars().count() >= 3 && stem.ends_with("in"),
        "t" => stem.ends_with('n') || stem == "can" || stem == "won",
        "s" | "re" | "ve" | "ll" | "d" | "m" => true,
        _ => false,
    };
    valid.then_some(suffix_end)
}

fn forced_english_end(raw: &str, start: usize, known_stem: &impl Fn(&str) -> bool) -> Option<usize> {
    url_end(raw, start)
        .or_else(|| email_end(raw, start))
        .or_else(|| user_name_end(raw, start))
        .or_else(|| user_name_end_with_underscore(raw, start))
        .or_else(|| tag_end(raw, start))
        .or_else(|| contraction_end(raw, start))
        .or_else(|| dotfile_end(raw, start))
        .or_else(|| dot_suffix_end(raw, start, known_stem))
        .or_else(|| file_end(raw, start))
        .or_else(|| domain_end(raw, start, known_stem))
        .or_else(|| hyphenated_end(raw, start, known_stem))
        .or_else(|| numeronym_end(raw, start))
        .or_else(|| proper_noun_with_digits_end(raw, start))
        .or_else(|| version_end(raw, start))
}

fn katakana_readings() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        crate::system_data::try_text("katakana.txt")
            .unwrap_or("")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect()
    })
}

fn brand_readings() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        crate::system_data::text("brands.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect()
    })
}

fn phrase_readings() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        crate::system_data::text("phrases.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split('\t').next())
            .map(|reading| reading.trim_start_matches('^').to_owned())
            .collect()
    })
}

fn sorted_katakana() -> &'static Vec<String> {
    static WORDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        let mut words: Vec<String> = crate::system_data::try_text("katakana.txt")
            .unwrap_or("")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        words.sort_unstable();
        words.dedup();
        words
    })
}

fn sorted_brands() -> &'static Vec<String> {
    static WORDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        let mut words: Vec<String> = crate::system_data::text("brands.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        words.sort_unstable();
        words.dedup();
        words
    })
}

fn sorted_phrases() -> &'static Vec<String> {
    static WORDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        let mut words: Vec<String> = crate::system_data::text("phrases.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split('\t').next())
            .map(|reading| reading.trim_start_matches('^').to_owned())
            .collect();
        words.sort_unstable();
        words.dedup();
        words
    })
}

fn has_prefix(words: &[String], value: &str) -> bool {
    let index = words.partition_point(|word| word.as_str() < value);
    words
        .get(index)
        .is_some_and(|word| word.starts_with(value))
}

fn common_has_prefix(value: &str) -> bool {
    packed_words::common_has_prefix(value)
        || has_prefix(sorted_katakana(), value)
        || has_prefix(sorted_brands(), value)
        || has_prefix(sorted_phrases(), value)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    None,
    Japanese,
    English,
    Sentence,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct State {
    side: Side,
    particle: bool,
    all_english: bool,
    previous_english: bool,
    sokuon: bool,
    context_switch: bool,
}

impl State {
    fn key(self) -> usize {
        self.side as usize
            | usize::from(self.particle) << 2
            | usize::from(self.all_english) << 3
            | usize::from(self.previous_english) << 4
            | usize::from(self.sokuon) << 5
            | usize::from(self.context_switch) << 6
    }

    fn from_key(key: usize) -> Self {
        let side = match key & 3 {
            1 => Side::Japanese,
            2 => Side::English,
            3 => Side::Sentence,
            _ => Side::None,
        };
        Self {
            side,
            particle: key & 4 != 0,
            all_english: key & 8 != 0,
            previous_english: key & 16 != 0,
            sokuon: key & 32 != 0,
            context_switch: key & 64 != 0,
        }
    }

    fn is_english(self) -> bool {
        matches!(self.side, Side::English | Side::Sentence)
    }
}

#[derive(Clone, Default)]
struct Edge {
    start: usize,
    end: usize,
    english: bool,
    neutral: bool,
    kana: String,
    particle: bool,
    letters: bool,
}

#[derive(Clone)]
struct Node {
    cost: f64,
    previous: usize,
    previous_state: usize,
    edge: Edge,
}

impl Dictionary {
    pub fn composition_lattice(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
    ) -> String {
        self.composition_lattice_with_learning(
            raw,
            final_input,
            before,
            after,
            level,
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
    }

    /// Lattice with the session's learned English and user dictionary. This is
    /// the entry point used by `Session::detect_composition`.
    pub fn composition_lattice_with_learning(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
        learned_english: &BTreeMap<String, bool>,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> String {
        if raw.is_empty() || level == "Manual" || raw.len() > 1024 {
            return self.composition(raw, final_input, before, after, level);
        }
        if !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
            return self.composition_mixed(
                raw,
                final_input,
                before,
                after,
                level,
                learned_english,
                user_dictionary,
            );
        }
        let lower = raw.to_ascii_lowercase();
        let (bounds, _pending) = romaji::unit_boundaries(
            &lower,
            |prefix| self.known_english(prefix),
            |prefix| self.known_english(prefix),
        );
        // The lattice scores complete nodes; do not truncate at a pending
        // romaji suffix. If the full raw is a known/learned English word it
        // must be able to win as one edge.
        let end = raw.len();
        let mut offsets = vec![0];
        offsets.extend((1..=end).filter(|i| bounds[*i]));
        if offsets.last().copied() != Some(end) {
            offsets.push(end);
        }
        let n = offsets.len().saturating_sub(1);
        if n == 0 {
            return self.composition(raw, final_input, before, after, level);
        }

        let preceding = before.chars().last().is_some_and(|c| c.is_ascii_alphabetic());
        let following = after.chars().next().map(|c| c.is_ascii_alphabetic());
        let english_sentence = english_sentence(before);
        let start = State {
            side: if preceding {
                if english_sentence {
                    Side::Sentence
                } else {
                    Side::English
                }
            } else if before.is_empty() {
                Side::None
            } else {
                Side::Japanese
            },
            particle: false,
            all_english: true,
            previous_english: preceding,
            sokuon: false,
            context_switch: preceding,
        };

        let mut best: Vec<[Option<Node>; 128]> =
            (0..=n).map(|_| std::array::from_fn(|_| None)).collect();
        best[0][start.key()] = Some(Node {
            cost: 0.0,
            previous: 0,
            previous_state: start.key(),
            edge: Edge::default(),
        });

        for index in 0..n {
            for state_key in 0..128 {
                let Some(from) = best[index][state_key].clone() else {
                    continue;
                };
                let state = State::from_key(state_key);
                let unit = &raw[offsets[index]..offsets[index + 1]];
                if !unit.bytes().all(|b| b.is_ascii_alphabetic()) {
                    offer_edge(
                        &mut best,
                        index + 1,
                        index,
                        state,
                        &from,
                        Edge {
                            start: offsets[index],
                            end: offsets[index + 1],
                            neutral: true,
                            ..Edge::default()
                        },
                        0.0,
                        raw,
                        english_sentence,
                    );
                    continue;
                }
                for next in index + 1..=n.min(index + MAX_SPAN) {
                    let span = &raw[offsets[index]..offsets[next]];
                    if !span.bytes().all(|b| b.is_ascii_alphabetic()) {
                        break;
                    }
                    let japanese = if next == index + 1
                        || self.japanese_has_prefix(span, user_dictionary)
                    {
                        self.japanese_edge(span, user_dictionary)
                    } else {
                        None
                    };
                    if let Some((kana, cost, particle)) = japanese {
                        let letters = span.chars().any(|c| c.is_ascii_alphabetic());
                        offer_edge(
                            &mut best,
                            next,
                            index,
                            state,
                            &from,
                            Edge {
                                start: offsets[index],
                                end: offsets[next],
                                kana,
                                particle,
                                letters,
                                ..Edge::default()
                            },
                            cost,
                            raw,
                            english_sentence,
                        );
                    }
                    if let Some(cost) = self.english_edge(span, learned_english) {
                        offer_edge(
                            &mut best,
                            next,
                            index,
                            state,
                            &from,
                            Edge {
                                start: offsets[index],
                                end: offsets[next],
                                english: true,
                                ..Edge::default()
                            },
                            cost,
                            raw,
                            english_sentence,
                        );
                    }
                }
            }
        }

        let mut chosen: Option<(usize, usize, f64)> = None;
        for state_key in 0..128 {
            let Some(node) = &best[n][state_key] else {
                continue;
            };
            let state = State::from_key(state_key);
            let mut cost = node.cost;
            if state.sokuon {
                cost += DANGLING_SOKUON_COST;
            }
            if following == Some(true) && state.side == Side::Japanese && !state.particle {
                cost += SWITCH_COST;
            }
            if following == Some(false) && state.is_english() && node.edge.english {
                cost += SWITCH_COST;
            }
            let tie = if state.side == Side::Japanese {
                -ENGLISH_TIE_BREAK
            } else {
                0.0
            };
            if chosen.is_none_or(|(_, _, best)| cost + tie < best) {
                chosen = Some((n, state_key, cost + tie));
            }
        }
        let Some((mut position, mut state_key, _)) = chosen else {
            return self.composition(raw, final_input, before, after, level);
        };
        let mut edges = Vec::new();
        while position > 0 {
            let Some(node) = best[position][state_key].clone() else {
                break;
            };
            edges.push(node.edge);
            position = node.previous;
            state_key = node.previous_state;
        }
        edges.reverse();
        let mut output = String::new();
        for edge in edges {
            if edge.english || edge.neutral {
                output.push_str(&raw[edge.start..edge.end]);
            } else if edge.kana.is_empty() {
                output.push_str(&raw[edge.start..edge.end]);
            } else if let Some(word) = user_dictionary
                .get(&edge.kana)
                .and_then(|words| words.first())
            {
                output.push_str(word);
            } else {
                let text = self.composition(&edge.kana, final_input, before, after, level);
                if text.is_empty() {
                    output.push_str(&edge.kana);
                } else {
                    output.push_str(&text);
                }
            }
        }
        if let Some(rest) = raw.get(end..) {
            output.push_str(rest);
        }
        if output.is_empty() {
            self.composition(raw, final_input, before, after, level)
        } else {
            output
        }
    }

    /// Kana-input lattice: Japanese units are kana characters and English
    /// units come from the physical alphabet keys. Boundaries are kana units,
    /// matching upstream's `kanaInput` rule.
    pub fn composition_lattice_kana(
        &self,
        raw: &str,
        kana: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
        learned_english: &BTreeMap<String, bool>,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> String {
        if raw.is_empty() || kana.is_empty() || level == "Manual" {
            return self.composition_kana(raw, kana, final_input, before, after, level);
        }
        let raw_offsets: Vec<usize> = raw
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(raw.len()))
            .collect();
        let kana_offsets: Vec<usize> = kana
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(kana.len()))
            .collect();
        if raw_offsets.len() != kana_offsets.len() {
            return self.composition_kana(raw, kana, final_input, before, after, level);
        }
        let n = kana_offsets.len().saturating_sub(1);
        if n == 0 {
            return self.composition_kana(raw, kana, final_input, before, after, level);
        }
        let preceding = before.chars().last().is_some_and(|c| c.is_ascii_alphabetic());
        let following = after.chars().next().map(|c| c.is_ascii_alphabetic());
        let english_sentence = english_sentence(before);
        let start = State {
            side: if preceding {
                if english_sentence {
                    Side::Sentence
                } else {
                    Side::English
                }
            } else if before.is_empty() {
                Side::None
            } else {
                Side::Japanese
            },
            particle: false,
            all_english: true,
            previous_english: preceding,
            sokuon: false,
            context_switch: preceding,
        };
        let mut best: Vec<[Option<Node>; 128]> =
            (0..=n).map(|_| std::array::from_fn(|_| None)).collect();
        best[0][start.key()] = Some(Node {
            cost: 0.0,
            previous: 0,
            previous_state: start.key(),
            edge: Edge::default(),
        });
        for index in 0..n {
            for state_key in 0..128 {
                let Some(from) = best[index][state_key].clone() else {
                    continue;
                };
                let state = State::from_key(state_key);
                let raw_unit = &raw[raw_offsets[index]..raw_offsets[index + 1]];
                let kana_unit = &kana[kana_offsets[index]..kana_offsets[index + 1]];
                let neutral = !is_kana_text(kana_unit)
                    && !raw_unit.bytes().all(|b| b.is_ascii_alphabetic());
                if neutral {
                    offer_edge(
                        &mut best,
                        index + 1,
                        index,
                        state,
                        &from,
                        Edge {
                            start: raw_offsets[index],
                            end: raw_offsets[index + 1],
                            neutral: true,
                            ..Edge::default()
                        },
                        0.0,
                        raw,
                        english_sentence,
                    );
                    continue;
                }
                for next in index + 1..=n.min(index + MAX_SPAN) {
                    let raw_span = &raw[raw_offsets[index]..raw_offsets[next]];
                    let kana_span = &kana[kana_offsets[index]..kana_offsets[next]];
                    let japanese = if next == index + 1
                        || common_has_prefix(kana_span)
                        || user_dictionary.keys().any(|reading| reading.starts_with(kana_span))
                    {
                        self.japanese_edge_kana(kana_span, user_dictionary)
                    } else {
                        None
                    };
                    if let Some((cost, particle)) = japanese {
                        offer_edge(
                            &mut best,
                            next,
                            index,
                            state,
                            &from,
                            Edge {
                                start: raw_offsets[index],
                                end: raw_offsets[next],
                                kana: kana_span.to_owned(),
                                particle,
                                ..Edge::default()
                            },
                            cost,
                            raw,
                            english_sentence,
                        );
                    }
                    if raw_span.bytes().all(|b| b.is_ascii_alphabetic()) {
                        if let Some(cost) = self.english_edge(raw_span, learned_english) {
                        offer_edge(
                            &mut best,
                            next,
                            index,
                            state,
                            &from,
                            Edge {
                                start: raw_offsets[index],
                                end: raw_offsets[next],
                                english: true,
                                ..Edge::default()
                            },
                            cost,
                            raw,
                            english_sentence,
                        );
                        }
                    }
                }
            }
        }
        let mut chosen: Option<(usize, usize, f64)> = None;
        for state_key in 0..128 {
            let Some(node) = &best[n][state_key] else {
                continue;
            };
            let state = State::from_key(state_key);
            let mut cost = node.cost;
            if state.sokuon {
                cost += DANGLING_SOKUON_COST;
            }
            if following == Some(true) && state.side == Side::Japanese && !state.particle {
                cost += SWITCH_COST;
            }
            if following == Some(false) && state.is_english() && node.edge.english {
                cost += SWITCH_COST;
            }
            let tie = if state.side == Side::Japanese {
                -ENGLISH_TIE_BREAK
            } else {
                0.0
            };
            if chosen.is_none_or(|(_, _, best)| cost + tie < best) {
                chosen = Some((n, state_key, cost + tie));
            }
        }
        let Some((mut position, mut state_key, _)) = chosen else {
            return self.composition_kana(raw, kana, final_input, before, after, level);
        };
        let mut edges = Vec::new();
        while position > 0 {
            let Some(node) = best[position][state_key].clone() else {
                break;
            };
            edges.push(node.edge);
            position = node.previous;
            state_key = node.previous_state;
        }
        edges.reverse();
        let mut output = String::new();
        for edge in edges {
            if edge.english || edge.neutral || edge.kana.is_empty() {
                output.push_str(&raw[edge.start..edge.end]);
            } else {
                let text = self.composition(&edge.kana, final_input, before, after, level);
                output.push_str(if text.is_empty() { &edge.kana } else { &text });
            }
        }
        if output.is_empty() {
            self.composition_kana(raw, kana, final_input, before, after, level)
        } else {
            output
        }
    }

    fn japanese_edge_kana(
        &self,
        kana: &str,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> Option<(f64, bool)> {
        if !is_kana_text(kana) {
            return None;
        }
        let particle = is_noun_particle(kana);
        if let Some(cost) = packed_words::common_word_cost(kana) {
            return Some((cost, particle));
        }
        let known = self.is_japanese(kana)
            || self
                .candidates(kana, "Asia/Tokyo")
                .iter()
                .any(|candidate| candidate.chars().any(crate::linguistics::kanji));
        let phrase = phrase_readings().contains(kana);
        let user = user_dictionary.contains_key(kana);
        let katakana = katakana_readings().contains(kana) || brand_readings().contains(kana);
        let cost = if particle {
            0.8
        } else if user {
            0.8
        } else if known || phrase {
            JAPANESE_WORD_COST
        } else if katakana {
            KATAKANA_WORD_COST
        } else if kana.chars().count() == 1 {
            1.5
        } else {
            unknown_kana_cost(kana)
        };
        Some((cost, particle))
    }

    fn numeric_counter(reading: &str) -> Option<&'static str> {
        Some(match reading {
            "gatsu" => "月",
            "nen" => "年",
            "nin" => "人",
            "en" => "円",
            "ji" => "時",
            "fun" | "pun" => "分",
            "kai" => "回",
            "nichi" => "日",
            "ko" => "個",
            "sai" => "歳",
            "kagetsu" => "ヶ月",
            "mai" => "枚",
            "hai" => "杯",
            "hon" => "本",
            "dai" => "台",
            "satsu" => "冊",
            "hiki" => "匹",
            "wa" => "羽",
            "do" => "度",
            "ken" => "件",
            "tsu" => "つ",
            _ => return None,
        })
    }

    /// Mixed ASCII input: keep forced English spans (URLs, domains, files,
    /// usernames, hyphenated words, numeronyms, versions, tags) and run the
    /// lattice only on alphabetic runs.
    fn composition_mixed(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
        learned_english: &BTreeMap<String, bool>,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> String {
        let mut output = String::new();
        let mut at = 0;
        while at < raw.len() {
            let c = raw[at..].chars().next().unwrap();
            if c.is_ascii_digit() {
                let number_end = consume_while(raw, at, |c| {
                    c.is_ascii_digit() || c == ','
                });
                if number_end > at {
                    if let Some(next) = raw[number_end..].chars().next() {
                        if next.is_ascii_alphabetic() {
                            let unit_end = consume_while(raw, number_end, |c| {
                                c.is_ascii_alphabetic()
                            });
                            if let Some(counter) =
                                Self::numeric_counter(&raw[number_end..unit_end].to_ascii_lowercase())
                            {
                                output.push_str(&raw[at..number_end]);
                                output.push_str(counter);
                                at = unit_end;
                                continue;
                            }
                        }
                    }
                }
            }
            if let Some(end) = forced_english_end(raw, at, &|stem| {
                self.known_english(stem) || self.is_readable_english(stem)
            }) {
                output.push_str(&raw[at..end]);
                at = end;
                continue;
            }
            if c.is_ascii_alphabetic() {
                // Stop the lattice run before a forced English span starts
                // inside the letters (kyouhaiitenkidesu|iPhone15).
                let mut end = at;
                while end < raw.len() {
                    let next = raw[end..].chars().next().unwrap();
                    if !next.is_ascii_alphabetic() {
                        break;
                    }
                    if end > at
                        && forced_english_end(raw, end, &|stem| {
                            self.known_english(stem) || self.is_readable_english(stem)
                        })
                        .is_some()
                    {
                        break;
                    }
                    end += next.len_utf8();
                }
                let run = &raw[at..end];
                let previous_kana = output
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ('ぁ'..='ゖ').contains(&ch) || ch == 'ー');
                let hyphen_continuation = raw[end..].starts_with('-')
                    && raw[end + 1..]
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic());
                if previous_kana {
                    // Japanese context (pa-thisyon, ...-kata): keep the run as kana.
                    let kana = crate::romaji::hiragana(&run.to_ascii_lowercase(), true);
                    output.push_str(if kana.is_empty() { run } else { &kana });
                } else if hyphen_continuation {
                    output.push_str(&self.composition(run, final_input, before, after, level));
                } else {
                    output.push_str(&self.composition_lattice_with_learning(
                        run,
                        final_input,
                        before,
                        after,
                        level,
                        learned_english,
                        user_dictionary,
                    ));
                }
                at = end;
            } else {
                let previous_kana = output
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ('ぁ'..='ゖ').contains(&ch) || ch == 'ー');
                let next_alpha = raw[at + c.len_utf8()..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_alphabetic());
                if c == '-' && previous_kana && next_alpha {
                    output.push('ー');
                } else {
                    output.push(c);
                }
                at += c.len_utf8();
            }
        }
        output
    }

    fn japanese_has_prefix(
        &self,
        span: &str,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> bool {
        let kana = romaji::hiragana(&span.to_ascii_lowercase(), true);
        if kana.is_empty() || !is_kana_text(&kana) {
            return false;
        }
        common_has_prefix(&kana)
            || user_dictionary.keys().any(|reading| reading.starts_with(&kana))
    }

    fn japanese_edge(
        &self,
        span: &str,
        user_dictionary: &BTreeMap<String, Vec<String>>,
    ) -> Option<(String, f64, bool)> {
        let kana = romaji::hiragana(&span.to_ascii_lowercase(), true);
        if kana.is_empty()
            || kana
                .chars()
                .any(|c| !('ぁ'..='ゖ').contains(&c) && c != 'ー')
        {
            return None;
        }
        let particle = is_noun_particle(&kana);
        if let Some(cost) = packed_words::common_word_cost(&kana) {
            return Some((kana, cost, particle));
        }
        let known = self.is_japanese(span)
            || self.is_japanese(&kana)
            || self
                .candidates(&kana, "Asia/Tokyo")
                .iter()
                .any(|candidate| candidate.chars().any(crate::linguistics::kanji));
        let phrase = phrase_readings().contains(&kana);
        let user = user_dictionary.contains_key(&kana);
        let katakana = katakana_readings().contains(&kana) || brand_readings().contains(&kana);
        let cost = if particle {
            0.8
        } else if user {
            0.8
        } else if known || phrase {
            JAPANESE_WORD_COST
        } else if katakana {
            KATAKANA_WORD_COST
        } else if kana.chars().count() == 1 {
            // RomajiTypoCorrector.WordCost: a one-character Japanese word.
            1.5
        } else {
            unknown_kana_cost(&kana) + C_ROW_COST * c_row_spellings(span)
        };
        Some((kana, cost, particle))
    }

    fn english_edge(
        &self,
        span: &str,
        learned_english: &BTreeMap<String, bool>,
    ) -> Option<f64> {
        // Short all-uppercase tokens are acronyms (AI, IT, UI, SNS, API).
        // Prefer them over a Japanese reading such as あい.
        if span.chars().count() <= 4 && span.chars().all(|c| c.is_ascii_uppercase()) {
            return Some(0.4);
        }
        let lower = span.to_ascii_lowercase();
        let learned = learned_english.get(&lower).copied();
        if learned == Some(false) {
            return None;
        }
        let known = learned == Some(true)
            || self.is_english(&lower)
            || self.is_common_english(&lower);
        let upper = span.chars().any(|c| c.is_ascii_uppercase());
        if upper && !known {
            // Name-like spelling: Tanaka, McKinsey, iPhone, GitHub-like words.
            let mut chars = span.chars();
            let first = chars.next()?;
            let rest_lower = chars.all(|c| c.is_ascii_lowercase());
            let camel = span.chars().skip(1).any(|c| c.is_ascii_uppercase())
                && span.chars().count() <= 16;
            if span.chars().count() < 3 || !(first.is_ascii_uppercase() || camel) {
                return None;
            }
            let _ = rest_lower;
            return Some(NAME_LETTER_COST * span.chars().count() as f64);
        }
        if !known && !upper {
            // An unknown sequence that is completely romaji-readable is
            // Japanese (towo → とを), not an unknown English word.
            let kana = romaji::hiragana(&lower, true);
            if is_kana_text(&kana) {
                return None;
            }
        }
        if known {
            Some(if learned == Some(true) {
                0.6
            } else if TECH_ENGLISH.contains(&lower.as_str()) {
                TECH_ENGLISH_BOOST
            } else if self.is_readable_english(&lower) {
                0.7
            } else {
                KNOWN_ENGLISH_COST
            })
        } else if span.chars().count() >= 3 {
            Some(UNKNOWN_LETTER_COST * span.chars().count() as f64)
        } else {
            None
        }
    }
}

fn english_sentence(before: &str) -> bool {
    let trimmed = before.trim_end();
    trimmed.ends_with(' ')
        && trimmed
            .split_whitespace()
            .filter(|word| {
                !word.is_empty()
                    && word
                        .chars()
                        .all(|c| c.is_ascii_alphabetic() || c == '\'' || c == '-')
            })
            .count()
            >= 2
}

fn c_row_spellings(span: &str) -> f64 {
    let bytes = span.as_bytes();
    let mut count = 0.0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'c' && bytes.get(index + 1) != Some(&b'h') {
            count += 1.0;
        }
    }
    count
}

fn doubled_at(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    position + 1 < bytes.len()
        && bytes[position].is_ascii_alphabetic()
        && bytes[position].to_ascii_lowercase() == bytes[position + 1].to_ascii_lowercase()
}

fn spelled_sokuons(text: &str, start: usize, end: usize) -> f64 {
    let bytes = text.as_bytes();
    let mut count = 0.0;
    let mut i = start;
    while i + 3 <= end {
        if matches!(bytes[i].to_ascii_lowercase(), b'l' | b'x')
            && bytes[i + 1].to_ascii_lowercase() == b't'
        {
            let tail = if bytes[i + 2].to_ascii_lowercase() == b's' {
                i + 4
            } else {
                i + 3
            };
            if tail <= end
                && bytes[tail - 1].to_ascii_lowercase() == b'u'
                && tail < bytes.len()
            {
                count += 1.0;
            }
        }
        i += 1;
    }
    count
}

#[allow(clippy::too_many_arguments)]
fn offer_edge(
    best: &mut [[Option<Node>; 128]],
    position: usize,
    from_position: usize,
    previous: State,
    from: &Node,
    edge: Edge,
    cost: f64,
    text: &str,
    english_sentence: bool,
) {
    let japanese = !edge.english && !edge.neutral;
    let previous_byte = edge.start.checked_sub(1).map(|p| text.as_bytes()[p]);
    let vowel = if japanese
        && previous.side == Side::Japanese
        && previous.previous_english
        && previous_byte.is_some_and(|b| b.is_ascii_alphabetic())
        && edge
            .kana
            .chars()
            .next()
            .is_some_and(|c| "ぁぃぅぇぉあいうえお".contains(c))
        && !(previous_byte.is_some_and(|b| b"aeiou".contains(&b))
            && edge
                .kana
                .chars()
                .next()
                .is_some_and(|c| "あいうえお".contains(c)))
    {
        VOWEL_AFTER_ENGLISH_COST
    } else {
        0.0
    };
    let spelled = if japanese {
        SPELLED_SOKUON_COST * spelled_sokuons(text, edge.start, edge.end)
    } else {
        0.0
    };
    let sokuon = vowel
        + spelled
        + if previous.sokuon && !japanese {
            DANGLING_SOKUON_COST
        } else {
            0.0
        }
        + if japanese
            && edge.kana.ends_with('っ')
            && doubled_at(text, edge.start)
            && previous.side != Side::Japanese
        {
            DANGLING_SOKUON_COST
        } else {
            0.0
        };

    let (step, next) = if edge.english {
        let mut step = cost + ENGLISH_TIE_BREAK;
        if previous.side == Side::Japanese && previous.particle {
            step += 0.0;
        } else if previous.side == Side::Japanese {
            step += SWITCH_COST;
        }
        if previous.is_english()
            && edge.start > 0
            && text.as_bytes()[edge.start - 1].is_ascii_alphabetic()
            && text.as_bytes()[edge.start].is_ascii_alphabetic()
        {
            step += ADJACENT_ENGLISH_COST;
        }
        let side = if previous.side == Side::Sentence
            && !text.as_bytes()[edge.start].is_ascii_alphabetic()
        {
            Side::Sentence
        } else {
            Side::English
        };
        (
            step,
            State {
                side,
                particle: false,
                all_english: previous.all_english,
                previous_english: true,
                sokuon: false,
                context_switch: previous.context_switch,
            },
        )
    } else if edge.neutral {
        (
            0.0,
            State {
                previous_english: false,
                ..previous
            },
        )
    } else {
        let mut step = cost;
        if previous.side == Side::Sentence {
            step += SENTENCE_SWITCH_COST;
        } else if previous.is_english() && !edge.particle {
            let exception = edge.kana.chars().count() > 1
                && edge
                    .kana
                    .chars()
                    .next()
                    .is_some_and(|c| is_noun_particle(&c.to_string()))
                && from.edge.english
                && from.edge.end.saturating_sub(from.edge.start) >= 3;
            if !exception {
                step += SWITCH_COST;
            }
        }
        if previous.context_switch && edge.start > 0 {
            step += if english_sentence {
                SENTENCE_SWITCH_COST
            } else {
                SWITCH_COST
            };
        }
        (
            step,
            State {
                side: Side::Japanese,
                particle: edge.particle,
                all_english: previous.all_english && !edge.letters,
                previous_english: false,
                sokuon: japanese
                    && edge.kana.ends_with('っ')
                    && doubled_at(text, edge.end - 1),
                context_switch: previous.context_switch,
            },
        )
    };
    let next_key = next.key();
    let total = from.cost + step + sokuon;
    let slot = &mut best[position][next_key];
    if slot.as_ref().is_none_or(|node| total < node.cost) {
        *slot = Some(Node {
            cost: total,
            previous: from_position,
            previous_state: previous.key(),
            edge,
        });
    }
}
