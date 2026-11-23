// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 一時ベンチマーク用: shunti (端末内の組み込み変換器) をコーパスで測る。
//!
//! 本番 (`rust/native/src/windows_converter/worker.rs`) と同じく
//! `Engine::convert(読み, 文脈)` の先頭候補をその文の変換結果とし、
//! 1文ごとの実時間を JSONL に追記する。文脈は空 (各文は独立)。
use serde_json::json;
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data_root = PathBuf::from(value(&args, "--data-root").expect("--data-root"));
    let variant = value(&args, "--variant").expect("--variant");
    let cases_path = PathBuf::from(value(&args, "--cases").expect("--cases"));
    let out_path = PathBuf::from(value(&args, "--out").expect("--out"));
    let meta_path = PathBuf::from(value(&args, "--meta").unwrap_or_else(|| {
        let mut path = out_path.clone();
        path.set_extension("meta.json");
        path.to_string_lossy().into_owned()
    }));
    let shard: usize = value(&args, "--shard")
        .map(|v| v.parse().expect("--shard"))
        .unwrap_or(0);
    let shards: usize = value(&args, "--shards")
        .map(|v| v.parse().expect("--shards"))
        .unwrap_or(1);
    let limit: usize = value(&args, "--limit")
        .map(|v| v.parse().expect("--limit"))
        .unwrap_or(0);
    let max_seconds: f64 = value(&args, "--max-seconds")
        .map(|v| v.parse().expect("--max-seconds"))
        .unwrap_or(0.0);

    let mut rows: Vec<(u64, String)> = Vec::new();
    let mut files: Vec<PathBuf> = if cases_path.is_dir() {
        std::fs::read_dir(&cases_path)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
            .collect()
    } else {
        vec![cases_path.clone()]
    };
    files.sort();
    for file in files {
        for line in BufReader::new(File::open(&file)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let row: serde_json::Value = serde_json::from_str(&line)?;
            rows.push((
                row["id"].as_u64().expect("id"),
                row["input"].as_str().unwrap_or_default().to_owned(),
            ));
        }
    }
    rows.sort_by_key(|row| row.0);
    if limit > 0 {
        rows.truncate(limit);
    }
    let total = rows.len();
    let cases = &rows[total * shard / shards..total * (shard + 1) / shards];

    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let began = Instant::now();
    let install_began = Instant::now();
    let directory = meltype_ai::packages::install_shunti_variant(&data_root, &variant)?;
    let install_seconds = install_began.elapsed().as_secs_f64();
    let open_began = Instant::now();
    let mut engine = meltype_ai::shunti::Engine::open(&directory)?;
    let startup_seconds = open_began.elapsed().as_secs_f64();

    let mut latencies: Vec<f64> = Vec::new();
    let mut completed = 0usize;
    let mut reason = "completed";
    {
        let mut out = BufWriter::new(File::create(&out_path)?);
        for (id, input) in cases {
            if max_seconds > 0.0 && began.elapsed().as_secs_f64() > max_seconds {
                reason = "deadline";
                break;
            }
            let case_began = Instant::now();
            let candidates = engine.convert(input, "");
            let ms = case_began.elapsed().as_secs_f64() * 1000.0;
            let actual = candidates.first().cloned().unwrap_or_default();
            latencies.push(ms);
            writeln!(
                out,
                "{}",
                json!({"id": id, "actual": actual, "ms": (ms * 1000.0).round() / 1000.0})
            )?;
            out.flush()?;
            completed += 1;
        }
    }
    let mut sorted = latencies.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pick = |ratio: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let index = ((ratio * (sorted.len() - 1) as f64).round() as usize).min(sorted.len() - 1);
        (sorted[index] * 1000.0).round() / 1000.0
    };
    let mean = if sorted.is_empty() {
        0.0
    } else {
        (latencies.iter().sum::<f64>() / latencies.len() as f64 * 1000.0).round() / 1000.0
    };
    let meta = json!({
        "model": variant,
        "family": "Shunti",
        "protocol": "shunti",
        "shard": shard,
        "shards": shards,
        "limit": limit,
        "corpus_cases": total,
        "cases_in_shard": cases.len(),
        "completed": completed,
        "reason": reason,
        "started_at": started,
        "elapsed_seconds": (began.elapsed().as_secs_f64() * 10.0).round() / 10.0,
        "install_seconds": (install_seconds * 10.0).round() / 10.0,
        "startup_seconds": (startup_seconds * 1000.0).round() / 1000.0,
        "model_dir": directory.to_string_lossy(),
        "context": "",
        "p50_ms": pick(0.50),
        "p95_ms": pick(0.95),
        "mean_ms": mean,
    });
    std::fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)?;
    println!(
        "{} {}/{} ({}) p50 {:.3}ms",
        variant,
        completed,
        cases.len(),
        reason,
        pick(0.50)
    );
    Ok(())
}
