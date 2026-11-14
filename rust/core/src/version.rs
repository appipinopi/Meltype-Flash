// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 画面に出す版番号。アプリ内では Cargo の版をそのまま表示する。
//! GitHub のリリースタグだけがゼロ埋め表記（例: flash-v0001.0001.0018）になる。

/// `1.1.18` を `v1.1.18` のように表示する。
/// 版の比較・タグ・報告先には元の文字列を使う。
pub fn display(version: &str) -> String {
    format!("v{version}")
}

#[cfg(test)]
mod tests {
    use super::display;

    #[test]
    fn shows_the_cargo_version_without_padding() {
        assert_eq!(display("1.1.18"), "v1.1.18");
        assert_eq!(display("1.1.7"), "v1.1.7");
        assert_eq!(display("0.1.0-pre.7"), "v0.1.0-pre.7");
        assert_eq!(display("dev"), "vdev");
    }
}
