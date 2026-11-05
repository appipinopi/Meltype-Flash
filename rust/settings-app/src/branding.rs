// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Shared product mark for the root window and native child windows.
use eframe::egui::IconData;
use std::sync::{Arc, OnceLock};

pub fn icon() -> Arc<IconData> {
    static ICON: OnceLock<Arc<IconData>> = OnceLock::new();
    ICON.get_or_init(|| {
        Arc::new(
            eframe::icon_data::from_png_bytes(include_bytes!(
                "../../../assets/branding/meltype-flash-256.png"
            ))
            .expect("embedded Meltype-FLASH icon must be valid PNG"),
        )
    })
    .clone()
}

/// CPU用の追加ランタイム。実モデル検証した版を固定し、本体には同梱しない。
pub fn ai_runtime_url() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "https://github.com/ggml-org/llama.cpp/releases/download/b12125/llama-b12125-bin-ubuntu-x64.tar.gz",
        ("linux", "aarch64") => "https://github.com/ggml-org/llama.cpp/releases/download/b12125/llama-b12125-bin-ubuntu-arm64.tar.gz",
        ("windows", "x86_64") => "https://github.com/ggml-org/llama.cpp/releases/download/b12125/llama-b12125-bin-win-cpu-x64.zip",
        ("windows", "aarch64") => "https://github.com/ggml-org/llama.cpp/releases/download/b12125/llama-b12125-bin-win-cpu-arm64.zip",
        _ => "https://github.com/ggml-org/llama.cpp/releases/tag/b12125",
    }
}
pub const AI_MODEL_URL: &str = "https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf/resolve/ddf6e44b2e05ab7ea9a3e31559c5e7948761365c/ggml-model-Q5_K_M.gguf";
