// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! IMEベンチマーク用ランナー。
//!
//! 使い方: cargo run --release -p meltype-core --example ime_benchmark -- <cases.jsonl> <out.jsonl>
//! cases.jsonl: 1行1件 {"id":1,"typed":"kyouha..."}
//! out.jsonl:   1行1件 {"id":1,"actual":"きょうは..."}
//!
//! quality.rs と同じ手順で Session にキーを送り、確定した文字列を集める。
//! tools/ime-benchmark/run_benchmark.py から、対象バージョンのソースへ
//! このファイルを差し込んでビルドされる (旧バージョンの測定用)。
use meltype_core::{session::TextEdit, Session};
use serde_json::{json, Value};
use std::io::Write;

fn apply(text: &mut String, edit: TextEdit) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(expected) = edit.expect {
        if !text.ends_with(&expected) || expected.encode_utf16().count() != edit.delete_before {
            return Err("host rejected an invalid replacement".into());
        }
        text.truncate(text.len() - expected.len());
    } else if edit.delete_before != 0 {
        return Err("unguarded backward deletion".into());
    }
    text.push_str(&edit.text);
    Ok(())
}

fn run_case(typed: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut session = Session::default();
    session.can_delete = true;
    session.settings.set("PredictiveCandidates", json!(false))?;
    session.set_context(Some(""), None);
    let mut actual = String::new();
    for c in typed.chars() {
        let vk = if c == ' ' { 32 } else { 0 };
        let result = session.key(vk, c as i32, 0);
        for edit in result.commits {
            apply(&mut actual, edit)?;
        }
        if !result.consumed {
            actual.push(c);
        }
    }
    for edit in session.commit().commits {
        apply(&mut actual, edit)?;
    }
    Ok(actual)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cases_path = args
        .get(1)
        .ok_or("usage: ime_benchmark <cases.jsonl> <out.jsonl>")?;
    let out_path = args
        .get(2)
        .ok_or("usage: ime_benchmark <cases.jsonl> <out.jsonl>")?;
    let text = std::fs::read_to_string(cases_path)?;
    let mut out = std::fs::File::create(out_path)?;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let case: Value = serde_json::from_str(line)?;
        let id = case["id"].as_u64().unwrap_or(0);
        let typed = case["typed"].as_str().unwrap_or("");
        let actual = run_case(typed).unwrap_or_else(|e| format!("<error: {e}>"));
        writeln!(out, "{}", json!({"id": id, "actual": actual}))?;
    }
    Ok(())
}
