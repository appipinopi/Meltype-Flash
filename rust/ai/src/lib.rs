// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 入力内容を送信しないローカルAI。辞書・モデルは明示的な任意導入。
pub const PACKAGES: &str = include_str!("../packages.json");
#[cfg(feature = "download")]
pub mod packages;
#[cfg(feature = "engine")]
pub mod shunti;

pub const MODEL_CATALOG: &str = include_str!("../model-catalog.json");
