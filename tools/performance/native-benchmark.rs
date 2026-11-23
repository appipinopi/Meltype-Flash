// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! C ABI boundary benchmark. Copied into isolated version snapshots by Python.
use meltype_native::{
    meltype_create_at, meltype_destroy, meltype_free, meltype_handle_key, meltype_set_can_delete,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    ffi::{CStr, CString},
    hint::black_box,
    time::Instant,
};

#[derive(Deserialize)]
struct Case {
    name: String,
    keys: Vec<[i32; 3]>,
    #[serde(default)]
    scored: bool,
}

struct Session(*mut std::ffi::c_void);
impl Drop for Session {
    fn drop(&mut self) {
        meltype_destroy(self.0);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let cases: Vec<Case> = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let samples: usize = args[2].parse()?;
    let warmups: usize = args[3].parse()?;
    for case in cases {
        for sample in 0..warmups + samples {
            let profile = tempfile::tempdir()?;
            std::fs::write(
                profile.path().join("config.json"),
                json!({
                    "SettingsVersion":5, "ScoredSegmentation":case.scored,
                    "LiveConversion":false, "PredictiveCandidates":false, "LearningEnabled":false
                })
                .to_string(),
            )?;
            let path = CString::new(profile.path().to_str().ok_or("non UTF-8 profile")?)?;
            let session = Session(unsafe { meltype_create_at(path.as_ptr()) });
            if session.0.is_null() {
                return Err("session creation failed".into());
            }
            meltype_set_can_delete(session.0, 1);
            let mut responses = Vec::with_capacity(case.keys.len());
            let mut keys_ms = Vec::with_capacity(case.keys.len());
            let started = Instant::now();
            for [vk, ch, modifiers] in &case.keys {
                let key_started = Instant::now();
                let response = unsafe {
                    let pointer = meltype_handle_key(
                        session.0,
                        *vk,
                        *ch,
                        *modifiers,
                        std::ptr::null(),
                        std::ptr::null(),
                    );
                    if pointer.is_null() {
                        return Err("null key result".into());
                    }
                    let text = CStr::from_ptr(pointer).to_string_lossy().into_owned();
                    meltype_free(pointer.cast());
                    text
                };
                black_box(&response);
                keys_ms.push(key_started.elapsed().as_secs_f64() * 1000.0);
                responses.push(response);
            }
            let total_ms = started.elapsed().as_secs_f64() * 1000.0;
            if sample >= warmups {
                println!(
                    "{}",
                    json!({"case":case.name, "sample":sample-warmups,
                    "total_ms":total_ms, "keys_ms":keys_ms, "responses":responses})
                );
            }
        }
        eprintln!("Completed {}", case.name);
    }
    Ok(())
}
