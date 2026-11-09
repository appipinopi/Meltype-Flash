// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Date-limited seasonal candidates (upstream issue #339). Only the active
//! season contributes candidates, so Christmas words do not appear in July.
use chrono::{Datelike, NaiveDate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    ChristmasEve,
    Christmas,
    NewYear,
    Valentine,
    Halloween,
}

impl Event {
    pub fn label(self) -> &'static str {
        match self {
            Event::ChristmasEve => "クリスマスイブ",
            Event::Christmas => "クリスマス",
            Event::NewYear => "元日",
            Event::Valentine => "バレンタインデー",
            Event::Halloween => "ハロウィン",
        }
    }
}

/// The event for a single date, if that date is a named day.
pub fn event(date: NaiveDate) -> Option<Event> {
    match (date.month(), date.day()) {
        (12, 24) => Some(Event::ChristmasEve),
        (12, 25) => Some(Event::Christmas),
        (1, 1) => Some(Event::NewYear),
        (2, 14) => Some(Event::Valentine),
        (10, 31) => Some(Event::Halloween),
        _ => None,
    }
}

/// Whether the reading belongs to an active seasonal window.
fn active(month: u32, day: u32) -> Option<Event> {
    match (month, day) {
        (12, 1..=25) => Some(if day == 24 {
            Event::ChristmasEve
        } else {
            Event::Christmas
        }),
        (12, 31) | (1, 1..=3) => Some(Event::NewYear),
        (2, 1..=14) => Some(Event::Valentine),
        (10, 25..=31) => Some(Event::Halloween),
        _ => None,
    }
}

/// Reading-specific seasonal candidates. Empty outside the season.
pub fn candidates(reading: &str, date: NaiveDate) -> Vec<String> {
    let Some(event) = active(date.month(), date.day()) else {
        return vec![];
    };
    let mut values = Vec::new();
    let mut add = |items: &[&str]| {
        for item in items {
            if !values.iter().any(|value| value == item) {
                values.push((*item).to_owned());
            }
        }
    };
    match event {
        Event::ChristmasEve | Event::Christmas => {
            if reading == "くりすますいぶ" {
                if event == Event::ChristmasEve {
                    add(&["クリスマスイブ", "🎄"]);
                }
            } else {
                match reading {
                    "くりすます" => add(&[
                        "クリスマス",
                        "メリークリスマス",
                        "クリスマスおめでとう",
                        "🎄",
                        "🎅",
                    ]),
                    "めりーくりすます" | "めりーくりすます!" => {
                        add(&["メリークリスマス", "Merry Christmas", "🎄"])
                    }
                    "さんた" | "さんたくろーす" => {
                        add(&["サンタ", "サンタクロース", "🎅"])
                    }
                    _ => {}
                }
            }
        }
        Event::NewYear => match reading {
            "あけましておめでとう" | "あけましておめでとうございます" => {
                add(&[
                    "明けましておめでとうございます",
                    "新年おめでとうございます",
                    "謹賀新年",
                    "🎍",
                ])
            }
            "おしょうがつ" => add(&["お正月", "謹賀新年", "🎍"]),
            _ => {}
        },
        Event::Valentine => match reading {
            "ばれんたいん" | "ばれんたいんでー" => {
                add(&["バレンタインデー", "チョコレート", "🍫"])
            }
            _ => {}
        },
        Event::Halloween => {
            if reading == "はろうぃん" {
                add(&["ハロウィン", "トリック・オア・トリート", "🎃"]);
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    #[test]
    fn christmas_candidates_only_appear_in_december() {
        let december = candidates("くりすます", date(12, 20));
        assert!(december.contains(&"メリークリスマス".into()));
        assert!(december.contains(&"🎄".into()));
        assert!(candidates("くりすます", date(7, 1)).is_empty());
        assert!(candidates("くりすます", date(12, 26)).is_empty());
        assert!(candidates("くりすますいぶ", date(12, 24)).contains(&"クリスマスイブ".into()));
        let december1 = candidates("くりすますいぶ", date(12, 1));
        assert!(
            december1.iter().all(|word| word != "クリスマスイブ"),
            "{december1:?}"
        );
    }

    #[test]
    fn other_events_stay_inside_their_windows() {
        assert_eq!(event(date(12, 24)), Some(Event::ChristmasEve));
        assert_eq!(event(date(12, 25)), Some(Event::Christmas));
        assert_eq!(event(date(1, 1)), Some(Event::NewYear));
        assert_eq!(event(date(2, 14)), Some(Event::Valentine));
        assert_eq!(event(date(10, 31)), Some(Event::Halloween));
        assert!(candidates("はろうぃん", date(10, 30)).contains(&"🎃".into()));
        assert!(candidates("はろうぃん", date(11, 1)).is_empty());
        assert!(candidates("ばれんたいん", date(2, 10)).contains(&"🍫".into()));
        assert!(candidates("あけましておめでとう", date(1, 2)).contains(&"謹賀新年".into()));
    }

    #[test]
    fn today_adds_the_named_event() {
        let now = date(12, 25).and_hms_opt(12, 0, 0).unwrap();
        assert!(
            crate::time_candidates::candidates("きょう", now, "Asia/Tokyo")
                .contains(&"クリスマス".into())
        );
        let new_year = date(1, 1).and_hms_opt(12, 0, 0).unwrap();
        assert!(
            crate::time_candidates::candidates("きょう", new_year, "Asia/Tokyo")
                .contains(&"元日".into())
        );
    }
}
