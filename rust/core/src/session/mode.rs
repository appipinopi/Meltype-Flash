// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! フロントエンド共通の、設定済み切替キーとコード行のモード遷移。
use super::{Session, SessionResult};

impl Session {
    pub fn input_is_direct(&self) -> bool {
        self.direct || self.code_english()
    }

    /// None means this is an ordinary key. Frontends must try role keys first.
    pub fn mode_key(&mut self, vk: i32, modifiers: i32) -> Option<SessionResult> {
        let japanese = self.mode_key_target(vk, modifiers)?;
        if matches!(vk, 0x1c | 0x15) && !self.input_is_direct() {
            let mut result = self.commit_preedit();
            result.consumed = true;
            return Some(result);
        }
        Some(self.switch_input_mode(japanese))
    }

    pub fn mode_key_target(&self, vk: i32, modifiers: i32) -> Option<bool> {
        if !self.settings.boolean("Enabled") {
            return None;
        }
        let japanese = match (vk, modifiers & 15) {
            (0x16 | 0x1c | 0x15, 0) => true,
            (0x1a | 0x1d, 0) => false,
            _ => {
                if !self
                    .settings
                    .toggle_shortcuts()
                    .iter()
                    .any(|key| key.matches(vk, modifiers & 15))
                {
                    return None;
                }
                self.input_is_direct()
            }
        };
        Some(japanese)
    }

    /// ImeOn is idempotent while composing. Other transitions commit exactly
    /// the visible preedit once before changing mode (#925, #1022).
    pub fn switch_input_mode(&mut self, japanese: bool) -> SessionResult {
        if japanese != self.input_is_direct() && (japanese || self.raw.is_empty()) {
            return self.result(true);
        }
        let mut result = if self.raw.is_empty() {
            self.result(true)
        } else {
            self.commit_preedit()
        };
        self.set_direct(false);
        if self.code_input
            && crate::context::classify(&self.before) == crate::context::LineKind::Code
        {
            self.set_code_line(japanese);
        } else {
            self.set_code_line(false);
            self.set_direct(!japanese);
        }
        result.consumed = true;
        result
    }
}
