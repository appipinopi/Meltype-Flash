// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Bundled greetings used as prediction fallbacks (upstream v1.5.0, #1213).
//!
//! `greetings.txt` has one `reading<TAB>phrase<TAB>start` row per greeting.
//! A greeting is offered only when the typed reading has reached `start`, is
//! still a prefix of `reading`, and is shorter than the full reading. That
//! keeps e.g. `こん` (今回・今度) from showing `こんにちは` before `こんにち`.
use std::sync::OnceLock;

struct Greeting {
    reading: String,
    phrase: String,
    start: String,
}

fn entries() -> &'static [Greeting] {
    static ENTRIES: OnceLock<Vec<Greeting>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        crate::system_data::text("greetings.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let reading = parts.next()?;
                let phrase = parts.next()?;
                let start = parts.next()?;
                (!reading.is_empty() && !phrase.is_empty() && !start.is_empty()).then(|| Greeting {
                    reading: reading.into(),
                    phrase: phrase.into(),
                    start: start.into(),
                })
            })
            .collect()
    })
}

/// Greeting phrases whose reading continues the typed kana.
pub(crate) fn predictions(kana: &str) -> Vec<String> {
    if kana.chars().count() < 2 {
        return vec![];
    }
    entries()
        .iter()
        .filter(|entry| {
            entry.reading.chars().count() > kana.chars().count()
                && entry.reading.starts_with(kana)
                && kana.starts_with(&entry.start)
        })
        .map(|entry| entry.phrase.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greetings_wait_for_the_start_reading_and_stop_at_the_full_reading() {
        assert_eq!(predictions("おはよ"), ["おはようございます"]);
        assert!(predictions("おは").contains(&"おはようございます".to_owned()));
        assert!(predictions("こんにちは").is_empty());
        assert!(predictions("こん").is_empty());
        assert_eq!(predictions("こんにち"), ["こんにちは"]);
        assert_eq!(predictions("よろし"), ["よろしくお願いします"]);
        assert_eq!(
            predictions("よろしくおねがいい"),
            ["よろしくお願いいたします"]
        );
        assert_eq!(predictions("どうぞよろ"), ["どうぞよろしくお願いします"]);
    }
}
