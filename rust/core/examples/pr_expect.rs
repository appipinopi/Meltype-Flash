// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! PR本文の入力・期待を実際のRust Sessionで照合する。期待値は完全一致。
use meltype_core::{session::TextEdit, Session};
use serde_json::json;

fn apply(text: &mut String, edit: TextEdit) -> Result<(), String> {
    if edit.delete_before != 0 {
        let expected = edit.expect.ok_or("unguarded deletion")?;
        if !text.ends_with(&expected) || expected.encode_utf16().count() != edit.delete_before {
            return Err("replacement does not match the host".into());
        }
        text.truncate(text.len() - expected.len());
    }
    text.push_str(&edit.text);
    Ok(())
}

fn committed(raw: &str) -> Result<String, String> {
    let mut session = Session::default();
    let mut text = String::new();
    for c in raw.chars() {
        let result = session.key(if c == ' ' { 32 } else { 0 }, c as i32, 0);
        for edit in result.commits {
            apply(&mut text, edit)?;
        }
        if !result.consumed {
            text.push(c);
        }
    }
    for edit in session.commit().commits {
        apply(&mut text, edit)?;
    }
    Ok(text)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: pr_expect CASES_TSV OUTPUT_JSON".into());
    }
    let mut results = Vec::new();
    for line in std::fs::read_to_string(&args[0])?
        .lines()
        .filter(|l| !l.is_empty())
    {
        let (typed, expected) = line.split_once('\t').ok_or("case requires a tab")?;
        let actual = committed(typed)?;
        results.push(json!({"typed":typed,"expected":expected,"actual":actual,
            "ok":actual == expected,"kind":"Rust Session / 完全一致"}));
    }
    std::fs::write(&args[1], serde_json::to_vec_pretty(&results)?)?;
    if results.iter().any(|r| r["ok"] != true) {
        return Err("PRの入力例に不一致があります".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_actual_commit_including_intermediate_spaces() {
        assert_eq!(committed("mainnipushsuru").unwrap(), "mainにpushする");
        assert_eq!(committed("dive into summer").unwrap(), "dive into summer");
        assert_ne!(committed("kwo").unwrap(), "くお");
    }
    #[test]
    fn refuses_unverified_host_replacement() {
        let edit = TextEdit {
            text: "新".into(),
            delete_before: 1,
            caret_back: 0,
            expect: Some("別".into()),
        };
        assert!(apply(&mut "元".into(), edit).is_err());
    }
}
