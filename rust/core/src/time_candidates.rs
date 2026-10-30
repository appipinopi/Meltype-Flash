// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Date/clock candidates and segment joining, ported from CompositionController.
use crate::session::Clause;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
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
pub fn typed(reading: &str, year: i32) -> Option<Vec<String>> {
    let (date, a, b) = shape(reading)?;
    if date {
        let Some(day) = NaiveDate::from_ymd_opt(year, a, b) else {
            return Some(vec![]);
        };
        let week = DAYS[day.weekday().num_days_from_sunday() as usize];
        Some(vec![
            format!("{a}/{b}"),
            format!("{a:02}/{b:02}"),
            format!("{a}/{b}({week})"),
            format!("{year:04}/{a:02}/{b:02}"),
            format!("{a}月{b}日"),
            format!("{a}月{b}日({week})"),
            format!("{year:04}/{a:02}/{b:02}({week})"),
        ])
    } else if a <= 23 && b <= 59 {
        Some(vec![format!("{a}:{b:02}"), format!("{a:02}:{b:02}")])
    } else {
        Some(vec![])
    }
}
pub fn candidates(reading: &str, now: NaiveDateTime) -> Vec<String> {
    if let Some(typed) = typed(reading, now.year()) {
        return typed;
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
    let (year, month, day, hour, minute) =
        (now.year(), now.month(), now.day(), now.hour(), now.minute());
    let week = DAYS[now.weekday().num_days_from_sunday() as usize];
    let date = format!("{year}年{month}月{day}日");
    let month_day = format!("{month}月{day}日");
    let slash_date = format!("{year:04}/{month:02}/{day:02}");
    let clock = format!("{hour:02}:{minute:02}");
    if !is_now {
        return vec![
            date.clone(),
            format!("{date}({week})"),
            month_day.clone(),
            format!("{month_day}({week})"),
            slash_date.clone(),
            format!("{slash_date}({week})"),
            format!("{month}/{day}({week})"),
            format!("{year:04}-{month:02}-{day:02}"),
            format!("{week}曜日"),
        ];
    }
    let time = format!("{hour}時{minute}分");
    vec![
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
    ]
}
pub fn is_dynamic_choice(reading: &str, text: &str) -> bool {
    ["くりっぷぼーど", "ぺーすと"].contains(&reading)
        || crate::math_candidates::is_choice(reading, text)
        || ["いま", "なう", "きょう"].contains(&reading)
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
