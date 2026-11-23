// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 公開Issueの入力例を共通コアで照合する。OS固有の変換器・実機の検証は含まない。
use meltype_core::{dictionary::Dictionary, Session};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("入力例のJSONファイルが必要です")?;
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let dictionary = Dictionary::bundled();
    for case in cases {
        let raw = case["raw"].as_str().ok_or("rawが必要です")?;
        let mut session = Session::default();
        let mut committed = String::new();
        for c in raw.chars() {
            let result = session.key(if c == ' ' { 0x20 } else { 0 }, c as i32, 0);
            for commit in result.commits {
                committed.push_str(&commit.text);
            }
            if !result.consumed {
                committed.push(c);
            }
        }
        let reading = dictionary.composition(raw, true, "", "", "Balanced");
        let display = session.display(false);
        let commits = session.commit().commits;
        for commit in commits {
            committed.push_str(&commit.text);
        }
        println!(
            "{}",
            json!({"issue":case["issue"],"raw":raw,"expected":case["expected"],
            "reading":reading,"display":display,"committed":committed,
            "scope":"共通コア・補助辞書のみ。OSの変換器と実機は未検証"})
        );
    }
    Ok(())
}
