// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Explicit offline verification against separately installed model data.
fn main() {
    let directory = std::env::args_os().nth(1).expect("model directory");
    let mut engine = meltype_ai::shunti::Engine::open(std::path::Path::new(&directory)).unwrap();
    let candidates = engine.convert("きょうはいいてんきです", "");
    assert_eq!(candidates.len(), 8);
    assert!(candidates.iter().any(|s| s.contains("今日")));
    println!("{}", serde_json::to_string(&candidates).unwrap());
}
