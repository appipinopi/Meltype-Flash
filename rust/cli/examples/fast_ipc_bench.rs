// SPDX-License-Identifier: GPL-3.0-or-later
//! Ultra-high-precision FastIPC benchmark client written in 100% Rust.
//! Measures:
//!   1. Serialization & Deserialization
//!   2. IPC Send (write + flush)
//!   3. Receive Wait & Thread Scheduling (OS pipe latency + context switch)
//!   4. Engine Call (meltype-core Session execution)
//!   5. Round-Trip Time (Total IPC latency)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

#[derive(Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    Context {
        before: Option<String>,
        after: Option<String>,
    },
    Key {
        vk: i32,
        ch: i32,
        modifiers: i32,
    },
    Commit,
    Reset,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    set: String,
    romaji: String,
    expected: String,
    judge: String,
    orig_flash_ok: bool,
    orig_flash_space: String,
}

struct LatencyStats {
    client_ser_us: Vec<f64>,
    client_send_us: Vec<f64>,
    recv_wait_us: Vec<f64>,
    server_deser_us: Vec<f64>,
    server_engine_us: Vec<f64>,
    server_ser_us: Vec<f64>,
    server_send_us: Vec<f64>,
    client_deser_us: Vec<f64>,
    total_rtt_us: Vec<f64>,
}

impl LatencyStats {
    fn new() -> Self {
        Self {
            client_ser_us: Vec::with_capacity(10000),
            client_send_us: Vec::with_capacity(10000),
            recv_wait_us: Vec::with_capacity(10000),
            server_deser_us: Vec::with_capacity(10000),
            server_engine_us: Vec::with_capacity(10000),
            server_ser_us: Vec::with_capacity(10000),
            server_send_us: Vec::with_capacity(10000),
            client_deser_us: Vec::with_capacity(10000),
            total_rtt_us: Vec::with_capacity(10000),
        }
    }
}

fn calc_percentiles(mut v: Vec<f64>) -> (f64, f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = v[v.len() / 2];
    let p95 = v[(v.len() as f64 * 0.95) as usize];
    let p99 = v[(v.len() as f64 * 0.99) as usize];
    let avg = v.iter().sum::<f64>() / v.len() as f64;
    (p50, p95, p99, avg)
}

fn main() {
    let binary_path = std::env::var("MELTYPE_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\opencode-rust\target\release\meltype-rs.exe"));

    println!("=========================================================================");
    println!("  Meltype-Flash: FastIPC 5-Section High-Precision Benchmark (100% Rust)  ");
    println!("=========================================================================");
    println!("Target Binary: {:?}", binary_path);

    let mut cmd = Command::new(&binary_path);
    cmd.arg("--fast-ipc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("MELTYPE_BENCH_TIMING", "1");

    let mut child: Child = cmd.spawn().expect("Failed to spawn meltype-rs");

    let mut stdin = child.stdin.take().expect("Failed to open child stdin");
    let mut stdout = child.stdout.take().expect("Failed to open child stdout");
    let stderr = child.stderr.take().expect("Failed to open child stderr");
    let mut stderr_reader = BufReader::new(stderr);

    let mut key_stats = LatencyStats::new();
    let mut commit_stats = LatencyStats::new();

    let mut send_request = |req: Request, stats: &mut LatencyStats| {
        let t_client_start = Instant::now();

        // 1. Client serialize
        let t_ser_start = Instant::now();
        let payload = serde_json::to_vec(&req).unwrap();
        let client_ser = t_ser_start.elapsed().as_nanos() as f64 / 1_000.0;

        // 2. Client IPC send (write + flush)
        let t_send_start = Instant::now();
        let len_bytes = (payload.len() as u32).to_le_bytes();
        stdin.write_all(&len_bytes).unwrap();
        stdin.write_all(&payload).unwrap();
        stdin.flush().unwrap();
        let client_send = t_send_start.elapsed().as_nanos() as f64 / 1_000.0;

        // 3. Client receive wait (OS pipe + server execution + context switch)
        let t_recv_start = Instant::now();
        let mut resp_len_bytes = [0u8; 4];
        stdout.read_exact(&mut resp_len_bytes).unwrap();
        let resp_len = u32::from_le_bytes(resp_len_bytes) as usize;
        let mut resp_payload = vec![0u8; resp_len];
        stdout.read_exact(&mut resp_payload).unwrap();
        let client_recv_wait = t_recv_start.elapsed().as_nanos() as f64 / 1_000.0;

        // 4. Client deserialize
        let t_deser_start = Instant::now();
        let _: Value = serde_json::from_slice(&resp_payload).unwrap();
        let client_deser = t_deser_start.elapsed().as_nanos() as f64 / 1_000.0;

        let total_rtt = t_client_start.elapsed().as_nanos() as f64 / 1_000.0;

        // Read server timing from stderr
        let mut server_timing_line = String::new();
        stderr_reader.read_line(&mut server_timing_line).unwrap();

        // Format: SERVER_TIMING: deser_ns=X engine_ns=Y ser_ns=Z send_ns=W
        let mut s_deser_us = 0.0;
        let mut s_engine_us = 0.0;
        let mut s_ser_us = 0.0;
        let mut s_send_us = 0.0;

        if let Some(pos) = server_timing_line.find("SERVER_TIMING:") {
            let parts = &server_timing_line[pos + 14..];
            for token in parts.split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    let ns: f64 = v.parse().unwrap_or(0.0);
                    let us = ns / 1_000.0;
                    match k {
                        "deser_ns" => s_deser_us = us,
                        "engine_ns" => s_engine_us = us,
                        "ser_ns" => s_ser_us = us,
                        "send_ns" => s_send_us = us,
                        _ => {}
                    }
                }
            }
        }

        // Calculate pipe & scheduling overhead:
        // recv_wait includes (pipe_transit_to_server + server_total + pipe_transit_to_client)
        let server_total_us = s_deser_us + s_engine_us + s_ser_us + s_send_us;
        let schedule_pipe_us = (client_recv_wait - server_total_us).max(0.0);

        stats.client_ser_us.push(client_ser);
        stats.client_send_us.push(client_send);
        stats.recv_wait_us.push(schedule_pipe_us);
        stats.server_deser_us.push(s_deser_us);
        stats.server_engine_us.push(s_engine_us);
        stats.server_ser_us.push(s_ser_us);
        stats.server_send_us.push(s_send_us);
        stats.client_deser_us.push(client_deser);
        stats.total_rtt_us.push(total_rtt);
    };

    // Warmup (5 sentences)
    println!("Warming up engine & FST dictionaries...");
    for _ in 0..10 {
        send_request(Request::Reset, &mut key_stats);
        for c in "kyouhagoogledekennsakushita".chars() {
            let vk = if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase() as i32
            } else {
                c as i32
            };
            send_request(
                Request::Key {
                    vk,
                    ch: c as i32,
                    modifiers: 0,
                },
                &mut key_stats,
            );
        }
        send_request(Request::Commit, &mut commit_stats);
    }
    // Clear warmup samples
    key_stats = LatencyStats::new();
    commit_stats = LatencyStats::new();

    // Load test cases
    let cases_json = include_str!("../../../rust/core/tests/data/matrix_649.json");
    let cases: Vec<TestCase> =
        serde_json::from_str(cases_json).expect("Failed to parse matrix JSON");
    println!("Running benchmark on all {} matrix cases...\n", cases.len());

    for case in &cases {
        send_request(Request::Reset, &mut key_stats);
        for c in case.romaji.chars() {
            let vk = if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase() as i32
            } else {
                c as i32
            };
            send_request(
                Request::Key {
                    vk,
                    ch: c as i32,
                    modifiers: 0,
                },
                &mut key_stats,
            );
        }
        send_request(Request::Commit, &mut commit_stats);
    }

    let mem_mb = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "([Math]::Round((Get-Process -Id {}).WorkingSet64 / 1MB, 2))",
                child.id()
            ),
        ])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let _ = child.kill();

    // Print Results
    let print_row = |name: &str, v: Vec<f64>| {
        let (p50, p95, p99, avg) = calc_percentiles(v);
        println!(
            "  {:<32} : p50 = {:>6.2} μs | p95 = {:>6.2} μs | p99 = {:>6.2} μs | avg = {:>6.2} μs",
            name, p50, p95, p99, avg
        );
    };

    println!("-------------------------------------------------------------------------");
    println!(
        "📊 【打鍵処理 (Key Event) の 5 区間内訳実測】  (サンプル数: {})",
        key_stats.total_rtt_us.len()
    );
    println!("-------------------------------------------------------------------------");
    print_row(
        "① [クライアント] JSON シリアライズ",
        key_stats.client_ser_us,
    );
    print_row(
        "② [クライアント] IPC パイプ送信 (Write+Flush)",
        key_stats.client_send_us,
    );
    print_row(
        "③ [サーバー]     JSON デシリアライズ",
        key_stats.server_deser_us,
    );
    print_row(
        "④ [サーバー]     ★エンジン呼び出し (Core Key)",
        key_stats.server_engine_us,
    );
    print_row(
        "⑤ [サーバー]     JSON シリアライズ",
        key_stats.server_ser_us,
    );
    print_row(
        "⑥ [サーバー]     IPC パイプ送信 (Write+Flush)",
        key_stats.server_send_us,
    );
    print_row(
        "⑦ [OS/スケジューラ] パイプ転送・待機・コンテキスト切替",
        key_stats.recv_wait_us,
    );
    print_row(
        "⑧ [クライアント] JSON デシリアライズ",
        key_stats.client_deser_us,
    );
    println!("  -----------------------------------------------------------------------");
    let (_p50, _p95, _p99, avg) = calc_percentiles(key_stats.total_rtt_us.clone());
    println!(
        "  🔥 ★合計打鍵レイテンシ (RTT / IPC込) : p50 = {:>6.2} μs ({:.3} ms) | p95 = {:>6.2} μs ({:.3} ms) | avg = {:>6.2} μs",
        _p50, _p50 / 1000.0, _p95, _p95 / 1000.0, avg
    );
    println!();

    println!("-------------------------------------------------------------------------");
    println!(
        "📊 【確定処理 (Commit / Enter) の 5 区間内訳実測】  (サンプル数: {})",
        commit_stats.total_rtt_us.len()
    );
    println!("-------------------------------------------------------------------------");
    print_row(
        "① [クライアント] JSON シリアライズ",
        commit_stats.client_ser_us,
    );
    print_row(
        "② [クライアント] IPC パイプ送信 (Write+Flush)",
        commit_stats.client_send_us,
    );
    print_row(
        "③ [サーバー]     JSON デシリアライズ",
        commit_stats.server_deser_us,
    );
    print_row(
        "④ [サーバー]     ★エンジン呼び出し (Commit)",
        commit_stats.server_engine_us,
    );
    print_row(
        "⑤ [サーバー]     JSON シリアライズ",
        commit_stats.server_ser_us,
    );
    print_row(
        "⑥ [サーバー]     IPC パイプ送信 (Write+Flush)",
        commit_stats.server_send_us,
    );
    print_row(
        "⑦ [OS/スケジューラ] パイプ転送・待機・コンテキスト切替",
        commit_stats.recv_wait_us,
    );
    print_row(
        "⑧ [クライアント] JSON デシリアライズ",
        commit_stats.client_deser_us,
    );
    println!("  -----------------------------------------------------------------------");
    let (_p50, _p95, _p99, avg) = calc_percentiles(commit_stats.total_rtt_us.clone());
    println!(
        "  🔥 ★合計確定レイテンシ (RTT / IPC込) : p50 = {:>6.2} μs ({:.3} ms) | p95 = {:>6.2} μs ({:.3} ms) | avg = {:>6.2} μs",
        _p50, _p50 / 1000.0, _p95, _p95 / 1000.0, avg
    );
    println!();
    println!("-------------------------------------------------------------------------");
    println!("💾 【プロセス実メモリ使用量 (RSS Working Set)】");
    println!("-------------------------------------------------------------------------");
    println!("  実測メモリ使用量 : {} MB  (上限目標: 40MB 以内)", mem_mb);
    println!("=========================================================================");
}
