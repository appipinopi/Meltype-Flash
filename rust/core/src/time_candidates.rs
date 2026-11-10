// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Date/clock candidates and segment joining, ported from CompositionController.
use crate::session::Clause;
use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike};
const DAYS: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];
fn number(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .map(|c| {
            if ('０'..='９').contains(&c) {
                char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap()
            } else {
                c
            }
        })
        .collect();
    if !(1..=2).contains(&digits.len()) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}
fn now_playing(directory: &std::path::Path, reading: &str) -> Option<String> {
    if reading != "なう" {
        return None;
    }
    let text = std::fs::read_to_string(directory.join("now_playing.txt")).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn shape(reading: &str) -> Option<(bool, u32, u32)> {
    if let Some((month, day)) = reading.split_once('/') {
        return Some((true, number(month)?, number(day)?));
    }
    if let Some((month, day)) = reading.split_once('月') {
        return Some((true, number(month)?, number(day.strip_suffix('日')?)?));
    }
    if let Some((month, day)) = reading.split_once("がつ") {
        return Some((true, number(month)?, number(day.strip_suffix("にち")?)?));
    }
    if let Some((hour, minute)) = reading.split_once('じ') {
        return Some((
            false,
            number(hour)?,
            number(
                minute
                    .strip_suffix("ふん")
                    .or_else(|| minute.strip_suffix("ぷん"))?,
            )?,
        ));
    }
    None
}
pub fn typed(reading: &str, today: NaiveDate) -> Option<Vec<String>> {
    let (date, a, b) = shape(reading)?;
    if date {
        // Prefer the upcoming occurrence: this year if the date has not
        // passed, otherwise next year. Dates up to 30 days back stay in the
        // past (writing about a recent date); otherwise last year is used.
        let chosen = (today.year() - 1..=today.year() + 4).find_map(|year| {
            let day = NaiveDate::from_ymd_opt(year, a, b)?;
            (day >= today - chrono::Duration::days(30)).then_some(day)
        });
        let Some(day) = chosen else {
            return Some(vec![]);
        };
        let week = DAYS[day.weekday().num_days_from_sunday() as usize];
        let year = day.year();
        Some(vec![
            format!("{a}/{b}"),
            format!("{a:02}/{b:02}"),
            format!("{a}月{b}日"),
            format!("{a}/{b}({week})"),
            format!("{a}月{b}日({week})"),
            format!("{year:04}/{a:02}/{b:02}"),
            format!("{year:04}/{a:02}/{b:02}({week})"),
            format!("{year}年{a}月{b}日"),
        ])
    } else if a <= 23 && b <= 59 {
        Some(vec![format!("{a}:{b:02}"), format!("{a:02}:{b:02}")])
    } else {
        Some(vec![])
    }
}
/// 「とけい」「じこく」で、設定したタイムゾーンの現在時刻を返す。
pub fn clock_candidates(reading: &str, utc: NaiveDateTime, timezone: &str) -> Vec<String> {
    if !matches!(reading, "とけい" | "じこく") {
        return vec![];
    }
    let zone: chrono_tz::Tz = timezone.parse().unwrap_or(chrono_tz::Asia::Tokyo);
    let local = zone.from_utc_datetime(&utc);
    let (hour, minute) = (local.hour(), local.minute());
    let time = format!("{hour}時{minute}分");
    vec![
        format!("{hour}:{minute:02}"),
        format!("{hour:02}:{minute:02}"),
        time.clone(),
        format!(
            "{}{h}時{minute}分",
            if hour < 12 { "午前" } else { "午後" },
            h = hour % 12
        ),
        format!("{time} ({timezone})"),
    ]
}

pub fn candidates(reading: &str, now: NaiveDateTime, timezone: &str) -> Vec<String> {
    if matches!(reading, "とけい" | "じこく") {
        return clock_candidates(reading, chrono::Utc::now().naive_utc(), timezone);
    }
    if let Some(typed) = typed(reading, now.date()) {
        return typed;
    }
    let seasonal = crate::seasonal::candidates(reading, now.date());
    if !seasonal.is_empty() {
        return seasonal;
    }
    if matches!(reading, "じこう" | "じこうのあいさつ") {
        return crate::number_candidates::seasonal(now.month());
    }
    if let Some(values) = crate::number_candidates::era_to_year(reading)
        .or_else(|| crate::number_candidates::traditional_units(reading))
    {
        return values;
    }
    let is_now = ["いま", "なう"].contains(&reading);
    if !is_now && reading != "きょう" {
        return vec![];
    }
    // Windows の設定アプリが再生中の曲名を now_playing.txt へ置く。
    let mut media = Vec::new();
    if let Some(title) = now_playing(&crate::storage::Store::default_directory(), reading) {
        media.push(title);
    }
    let (year, month, day, hour, minute) =
        (now.year(), now.month(), now.day(), now.hour(), now.minute());
    let week = DAYS[now.weekday().num_days_from_sunday() as usize];
    let date = format!("{year}年{month}月{day}日");
    let month_day = format!("{month}月{day}日");
    let slash_date = format!("{year:04}/{month:02}/{day:02}");
    let clock = format!("{hour:02}:{minute:02}");
    if !is_now {
        let mut values = vec![
            date.clone(),
            format!("{date}({week})"),
            month_day.clone(),
            format!("{month_day}({week})"),
            slash_date.clone(),
            format!("{slash_date}({week})"),
            format!("{month}/{day}({week})"),
            format!("{year:04}-{month:02}-{day:02}"),
            format!("{week}曜日"),
            format!("{year:04}{month:02}{day:02}"),
        ];
        if let Some(event) = crate::seasonal::event(now.date()) {
            values.push(event.label().to_owned());
        }
        return values;
    }
    let time = format!("{hour}時{minute}分");
    let mut values = media;
    values.extend([
        clock.clone(),
        time.clone(),
        format!(
            "{}{h}時{minute}分",
            if hour < 12 { "午前" } else { "午後" },
            h = hour % 12
        ),
        format!("{date}({week}) {time}"),
        format!("{month_day}({week}) {time}"),
        format!("{date}({week}) {clock}"),
        format!("{slash_date} {clock}"),
    ]);
    values
}
pub fn is_dynamic_choice(reading: &str, text: &str) -> bool {
    ["くりっぷぼーど", "ぺーすと"].contains(&reading)
        || crate::math_candidates::is_choice(reading, text)
        || ["いま", "なう", "きょう", "とけい", "じこく"].contains(&reading)
            && (text.chars().any(|c| c.is_ascii_digit()) || text.ends_with("曜日"))
        || shape(reading).is_some() && text.contains(['/', ':', '月'])
}
pub fn join(clauses: Vec<Clause>) -> Vec<Clause> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < clauses.len() {
        let end = (i + 2..=clauses.len().min(i + 4)).rev().find(|end| {
            shape(
                &clauses[i..*end]
                    .iter()
                    .map(|c| c.reading.as_str())
                    .collect::<String>(),
            )
            .is_some()
        });
        if let Some(end) = end {
            let reading = clauses[i..end].iter().map(|c| c.reading.as_str()).collect();
            let text = clauses[i..end].iter().map(Clause::text).collect();
            result.push(Clause::new(reading, vec![text]));
            i = end;
        } else {
            result.push(clauses[i].clone());
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn now_playing_is_read_only_for_the_now_reading() {
        let directory =
            std::env::temp_dir().join(format!("meltype-now-playing-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&directory);
        std::fs::write(
            directory.join("now_playing.txt"),
            "曲名 - 歌手
",
        )
        .unwrap();
        assert_eq!(now_playing(&directory, "なう"), Some("曲名 - 歌手".into()));
        assert_eq!(now_playing(&directory, "いま"), None);
        assert_eq!(now_playing(&directory, "とけい"), None);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn clock_candidates_use_the_configured_timezone() {
        let utc = NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 0)
            .unwrap();
        let tokyo = clock_candidates("とけい", utc, "Asia/Tokyo");
        assert!(tokyo.contains(&"12:04".to_owned()));
        assert!(tokyo.contains(&"12時4分".to_owned()));
        let utc_values = clock_candidates("じこく", utc, "UTC");
        assert!(utc_values.contains(&"3:04".to_owned()));
        assert!(clock_candidates("きょう", utc, "UTC").is_empty());
    }
}
