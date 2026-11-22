// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Score a frozen reading/context -> expected IME conversion corpus.
use meltype_ai::{packages::install_shunti_variant, shunti::Engine, MODEL_CATALOG};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};

#[derive(Deserialize)]
struct Case {
    category: String,
    question: Question,
    answer: String,
}
#[derive(Deserialize)]
struct Question {
    reading: String,
    context: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_root = args.next().ok_or("usage: accuracy <model-root> <report.json> [model-id]")?;
    let report_path = args.next().ok_or("usage: accuracy <model-root> <report.json> [model-id]")?;
    let model_id = args.next().unwrap_or_else(|| "shunti-v7-xs5".into());
    let cases: Vec<Case> = serde_json::from_str(include_str!("../data/ime-qa-accuracy.json"))?;
    if cases.len() < 30 {
        return Err("IME Q&A corpus must contain at least 30 fixed cases".into());
    }
    let catalog: Value = serde_json::from_str(MODEL_CATALOG)?;
    let model = catalog["models"].as_array().and_then(|models| {
        models.iter().find(|entry| entry["id"] == model_id)
    }).ok_or("requested Shunti model is missing from model-catalog.json")?;
    let model_directory = install_shunti_variant(Path::new(&model_root), &model_id)?;
    let mut engine = Engine::open(&model_directory)?;
    let started = Instant::now();
    let mut exact_top1 = 0usize;
    let mut exact_top3 = 0usize;
    let mut exact_top8 = 0usize;
    let mut reciprocal_rank = 0.0f64;
    let mut results = Vec::with_capacity(cases.len());
    for (index, case) in cases.into_iter().enumerate() {
        if case.question.reading.is_empty()
            || case.question.reading.encode_utf16().count() > 160
            || case.question.context.chars().count() > 80
            || case.answer.is_empty()
            || case.answer.chars().any(char::is_control)
        {
            return Err(format!("invalid fixed IME Q&A case {}", index + 1).into());
        }
        let candidates = engine.convert(&case.question.reading, &case.question.context);
        let rank = candidates.iter().position(|candidate| candidate == &case.answer);
        exact_top1 += usize::from(rank == Some(0));
        exact_top3 += usize::from(rank.is_some_and(|rank| rank < 3));
        exact_top8 += usize::from(rank.is_some());
        reciprocal_rank += rank.map_or(0.0, |rank| 1.0 / (rank + 1) as f64);
        results.push(json!({
            "id": format!("ime-{:03}", index + 1),
            "category": case.category,
            "question": {"reading":case.question.reading,"context":case.question.context},
            "answer": case.answer,
            "rank": rank.map(|rank| rank + 1),
            "top1": rank == Some(0),
            "candidates": candidates,
        }));
    }
    let total = results.len();
    let corpus_sha256 = format!("{:x}", Sha256::digest(include_bytes!("../data/ime-qa-accuracy.json")));
    let files = model["files"].as_array().ok_or("model files are missing")?;
    let model_file_sha256: Vec<_> = files.iter().filter_map(|file| file["sha256"].as_str()).collect();
    let summary = json!({
        "kind": "ime-reading-context-qa",
        "model": {"id":model_id,"label":model["label"],"license":model["license"],"files":files},
        "model_file_sha256": model_file_sha256,
        "corpus": "rust/ai/data/ime-qa-accuracy.json",
        "corpus_sha256": corpus_sha256,
        "cases": total,
        "exact_top1": exact_top1,
        "top1_accuracy": exact_top1 as f64 / total as f64,
        "exact_top3": exact_top3,
        "top3_recall": exact_top3 as f64 / total as f64,
        "exact_top8": exact_top8,
        "top8_recall": exact_top8 as f64 / total as f64,
        "mean_reciprocal_rank": reciprocal_rank / total as f64,
        "inference_ms": started.elapsed().as_secs_f64() * 1000.0,
        "results": results,
    });
    std::fs::write(&report_path, serde_json::to_vec_pretty(&summary)?)?;
    println!("{}", serde_json::to_string(&json!({
        "model": model_id,
        "exact_top1": format!("{exact_top1}/{total}"),
        "top1_accuracy": exact_top1 as f64 / total as f64,
        "exact_top3": exact_top3,
        "exact_top8": exact_top8,
        "mean_reciprocal_rank": reciprocal_rank / total as f64,
        "inference_ms": started.elapsed().as_secs_f64() * 1000.0,
        "report": report_path,
    }))?);
    Ok(())
}
