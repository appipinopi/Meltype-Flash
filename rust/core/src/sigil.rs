// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Upstream SigilWord boundary state; retains no typed text.
#[derive(Clone)]
pub(crate) struct Sigil {
    boundary: Option<bool>,
    length: usize,
}
impl Default for Sigil {
    fn default() -> Self {
        Self {
            boundary: Some(true),
            length: 0,
        }
    }
}
fn name(c: char) -> bool {
    c.is_ascii_graphic()
}
impl Sigil {
    pub fn is_active(&self) -> bool {
        self.length > 0
    }
    pub fn context(&mut self, before: &str) {
        if let Some(last) = before.chars().next_back() {
            if self.length > 0 && !name(last) {
                self.length = 0;
            }
            self.boundary = Some(last.is_whitespace());
        } else if self.boundary.is_none() {
            self.boundary = Some(true);
        }
    }
    pub fn passes(&self, c: char) -> bool {
        if self.length > 0 {
            name(c)
        } else {
            matches!(c, '/' | '$' | '@') && self.boundary == Some(true)
        }
    }
    pub fn append(&mut self, text: &str) {
        for c in text.chars() {
            if self.length > 0 && name(c) {
                self.length = self.length.saturating_add(1);
                continue;
            }
            self.length = 0;
            if matches!(c, '/' | '$' | '@') && self.boundary == Some(true) {
                self.length = 1;
                self.boundary = Some(false);
            } else {
                self.boundary = Some(c.is_whitespace());
            }
        }
    }
    pub fn lose(&mut self) {
        self.boundary = None;
        self.length = 0;
    }
    pub fn passed(&mut self, vk: i32, ch: char, modifiers: i32) {
        if modifiers & 14 != 0 {
            self.lose();
            return;
        }
        match vk {
            0x0d => *self = Self::default(),
            8 => {
                if self.length > 0 {
                    self.length -= 1;
                    if self.length == 0 {
                        self.boundary = Some(true);
                    }
                } else {
                    self.boundary = None;
                }
            }
            0x20 => self.append(" "),
            0x1b | 0x10..=0x12 | 0xa0..=0xa5 | 0x5b..=0x5c => {}
            9 | 0x21..=0x28 | 0x2e => self.lose(),
            _ if !ch.is_control() => self.append(&ch.to_string()),
            _ => {}
        }
    }
}
