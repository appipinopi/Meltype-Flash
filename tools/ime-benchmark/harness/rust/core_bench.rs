// SPDX-License-Identifier: GPL-3.0-or-later
// 1万文コアベンチ用ランナー (meltype-core の Rust 版)。
//
//   使い方: cargo run --release -p meltype-core --example core_bench -- <cases.jsonl> <out.jsonl>
//   cases.jsonl: 1行1件 {"id":1,"typed":"kyouhaii..."} (ローマ字で打鍵する文字列)
//   out.jsonl:   1行1件 {"id":1,"enter":"…","space":"…","ms":1.23}
//                enter = 打鍵後に Enter で確定した文字列
//                space = 打鍵後に Space で変換したときの表示 (変換ボックスの文字列)
//
// 変換器: コアは外部変換器 (アプリ側) を前提にしている。`bundled-mozc` 機能を付けてビルドすると、
// コア同梱の移植 Mozc 辞書 (portable_mozc) で Space 変換する (Layer B)。
// 機能なしでビルドした場合は Space 変換が起きず、Layer A (かな・判定・保護) の測定になる。
use meltype_core::{session::TextEdit, Session};
use serde_json::{json, Value};
use std::io::Write;

fn apply(text: &mut String, edit: TextEdit) {
    if edit.delete_before > 0 && edit.expect.is_none() {
        // 確定し直し以外では後方削除は起きない。起きたら末尾から消す (ホストと同じ振る舞い)。
        let cut = text.len().saturating_sub(edit.delete_before.min(text.len()));
        text.truncate(cut);
    } else if let Some(expected) = &edit.expect {
        if text.ends_with(expected.as_str()) {
            text.truncate(text.len() - expected.len());
        }
    }
    text.push_str(&edit.text);
}

fn run_case(typed: &str) -> (String, String, f64) {
    let mut session = Session::default();
    session.can_delete = true;
    let _ = session.settings.set("PredictiveCandidates", json!(false));
    session.set_context(Some(""), None);
    // セッションの作成は計測に含めない (打鍵から Space・Enter までを測る)。
    let started = std::time::Instant::now();

    let mut committed = String::new();
    for c in typed.chars() {
        let vk = if c == ' ' {
            32
        } else if c.is_ascii_alphabetic() {
            c.to_ascii_uppercase() as i32
        } else {
            0
        };
        let result = session.key(vk, c as i32, 0);
        for edit in result.commits {
            apply(&mut committed, edit);
        }
    }

    let space_result = session.key(32, 32, 0);
    let mut space_buffer = committed.clone();
    let mut space_view = None;
    for edit in space_result.commits {
        apply(&mut space_buffer, edit);
    }
    if let Some(view) = space_result.view {
        space_view = Some(view.text);
    }
    let space = space_view.unwrap_or_else(|| space_buffer.clone());

    let enter_result = session.commit();
    let mut enter_buffer = space_buffer;
    for edit in enter_result.commits {
        apply(&mut enter_buffer, edit);
    }
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    (enter_buffer, space, ms)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cases_path = args
        .get(1)
        .ok_or("usage: core_bench <cases.jsonl> <out.jsonl>")?;
    let out_path = args
        .get(2)
        .ok_or("usage: core_bench <cases.jsonl> <out.jsonl>")?;
    let text = std::fs::read_to_string(cases_path)?;
    let mut out = std::fs::File::create(out_path)?;
    let mut index = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let case: Value = serde_json::from_str(line)?;
        let id = case["id"].as_u64().unwrap_or(0);
        let typed = case["typed"].as_str().unwrap_or("");
        let (enter, space, ms) = run_case(typed);
        writeln!(
            out,
            "{}",
            json!({"id": id, "enter": enter, "space": space, "ms": (ms * 1000.0).round() / 1000.0})
        )?;
        index += 1;
        if index % 500 == 0 {
            eprintln!("cases={index}");
        }
    }
    Ok(())
}
