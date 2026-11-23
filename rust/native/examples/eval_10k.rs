#![allow(dead_code)]

#[cfg(windows)]
#[path = "../src/windows_converter/system.rs"]
mod system;

#[cfg(windows)]
fn main() {
    run_benchmark();
}

#[cfg(not(windows))]
fn main() {
    println!("10k benchmark requires Windows IME / COM subsystem.");
}

#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::fs::File;
#[cfg(windows)]
use std::io::{BufRead, BufReader};
#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
fn parse_csv_line(line: &str) -> Option<(String, String, String, String, String)> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut cur));
            }
            _ => {
                cur.push(c);
            }
        }
    }
    fields.push(cur);

    if fields.len() >= 5 {
        Some((
            fields[0].clone(),
            fields[1].clone(),
            fields[2].clone(),
            fields[3].clone(),
            fields[4].clone(),
        ))
    } else {
        None
    }
}

#[cfg(windows)]
fn get_process_memory() -> usize {
    #[cfg(windows)]
    unsafe {
        #[repr(C)]
        struct PROCESS_MEMORY_COUNTERS {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }

        #[link(name = "psapi")]
        extern "system" {
            fn GetProcessMemoryInfo(
                h_process: *mut std::ffi::c_void,
                ppmc: *mut PROCESS_MEMORY_COUNTERS,
                cb: u32,
            ) -> i32;
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
        }

        let mut pmc = std::mem::zeroed::<PROCESS_MEMORY_COUNTERS>();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) != 0 {
            pmc.working_set_size
        } else {
            0
        }
    }
    #[cfg(not(windows))]
    {
        0
    }
}

#[cfg(windows)]
fn run_benchmark() {
    let csv_path = r"c:\Users\藤島\Downloads\ime_benchmark_2026-12.csv";
    println!("Loading benchmark dataset from: {csv_path}");

    let file = File::open(csv_path).expect("Failed to open benchmark CSV");
    let reader = BufReader::new(file);

    let mut rows = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");
        if idx == 0 {
            continue; // Header
        }
        if let Some(row) = parse_csv_line(&line) {
            rows.push(row);
        }
    }
    println!("Total cases loaded: {}", rows.len());

    let _apartment = system::Apartment::enter().expect("Failed to enter STA apartment");
    let mut converter = system::System::default();
    println!("Windows Converter available: {}", converter.available());

    let mut latencies: Vec<f64> = Vec::with_capacity(rows.len());
    let mut total_matches = 0usize;
    let mut category_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (matches, total)
    let mut subcategory_stats: HashMap<String, (usize, usize)> = HashMap::new();
    let mut sample_failures: Vec<(String, String, String, String)> = Vec::new();

    let start_all = Instant::now();
    let initial_mem = get_process_memory();

    for (i, (_id, cat, subcat, input, expected)) in rows.iter().enumerate() {
        let input_utf16: Vec<u16> = input.encode_utf16().collect();
        let t0 = Instant::now();
        let converted_u16 = converter.convert(&input_utf16);
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        latencies.push(elapsed_us);

        let converted = converted_u16
            .map(|s| String::from_utf16_lossy(&s))
            .unwrap_or_default();

        let is_match = converted == *expected;
        if is_match {
            total_matches += 1;
        } else if sample_failures.len() < 20 {
            sample_failures.push((
                cat.clone(),
                input.clone(),
                expected.clone(),
                converted.clone(),
            ));
        }

        let cat_entry = category_stats.entry(cat.clone()).or_insert((0, 0));
        cat_entry.1 += 1;
        if is_match {
            cat_entry.0 += 1;
        }

        let subcat_entry = subcategory_stats
            .entry(format!("{}/{}", cat, subcat))
            .or_insert((0, 0));
        subcat_entry.1 += 1;
        if is_match {
            subcat_entry.0 += 1;
        }

        if (i + 1) % 1000 == 0 || i + 1 == rows.len() {
            let current_acc = (total_matches as f64 / (i + 1) as f64) * 100.0;
            let current_mem_mb = get_process_memory() as f64 / (1024.0 * 1024.0);
            println!(
                "Progress: {:5}/{} | Acc: {:5.2}% ({} / {}) | WorkingSet: {:.2} MB",
                i + 1,
                rows.len(),
                current_acc,
                total_matches,
                i + 1,
                current_mem_mb
            );
        }
    }

    let total_time_sec = start_all.elapsed().as_secs_f64();
    let peak_mem_mb = get_process_memory() as f64 / (1024.0 * 1024.0);

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let count = latencies.len();
    let p50 = latencies[count / 2];
    let p90 = latencies[count * 90 / 100];
    let p95 = latencies[count * 95 / 100];
    let p99 = latencies[count * 99 / 100];
    let avg_lat = latencies.iter().sum::<f64>() / count as f64;

    println!("\n========================================================");
    println!("               10,000 IME BENCHMARK RESULTS             ");
    println!("========================================================");
    println!("Total Sentences:   {}", rows.len());
    println!(
        "Exact Matches:     {} / {} ({:.2}%)",
        total_matches,
        rows.len(),
        (total_matches as f64 / rows.len() as f64) * 100.0
    );
    println!(
        "Total Duration:    {:.2} s ({:.1} sent/sec)",
        total_time_sec,
        rows.len() as f64 / total_time_sec
    );
    println!(
        "Memory (RSS):      {:.2} MB (Start: {:.2} MB)",
        peak_mem_mb,
        initial_mem as f64 / (1024.0 * 1024.0)
    );
    println!("Latency (us):      Avg: {:.1}us | P50: {:.1}us | P90: {:.1}us | P95: {:.1}us | P99: {:.1}us", avg_lat, p50, p90, p95, p99);
    println!(
        "Latency (ms):      Avg: {:.3}ms | P50: {:.3}ms | P95: {:.3}ms",
        avg_lat / 1000.0,
        p50 / 1000.0,
        p95 / 1000.0
    );

    println!("\n--- CATEGORY BREAKDOWN ---");
    let mut cat_vec: Vec<(String, (usize, usize))> = category_stats.into_iter().collect();
    cat_vec.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    for (cat, (matches, total)) in cat_vec {
        let pct = (matches as f64 / total as f64) * 100.0;
        println!("{:25} : {:5} / {:5} ({:5.2}%)", cat, matches, total, pct);
    }

    println!("\n--- SAMPLE MISMATCHES ---");
    for (idx, (cat, input, exp, got)) in sample_failures.iter().take(10).enumerate() {
        println!("[{}] Category: {}", idx + 1, cat);
        println!("  Input:    {}", input);
        println!("  Expected: {}", exp);
        println!("  Got:      {}", got);
    }
}
