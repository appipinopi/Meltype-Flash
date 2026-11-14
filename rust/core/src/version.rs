// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 画面に出す版番号。桁を 4 桁に揃えて、更新の前後で数字の位置が動かないようにする。

/// `1000.1.1` を `v1000.0001.0001` のように表示する。
/// 版の比較・タグ・報告先には元の文字列を使い、画面だけこの形にする。
pub fn display(version: &str) -> String {
    let (core, suffix) = version
        .split_once(['-', '+'])
        .map_or((version, ""), |(core, _rest)| {
            (core, &version[core.len()..])
        });
    let parts: Vec<_> = core.split('.').collect();
    if parts.len() != 3 {
        return format!("v{version}");
    }
    let mut padded = Vec::with_capacity(3);
    for part in parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) || part.len() > 4 {
            return format!("v{version}");
        }
        padded.push(format!("{part:0>4}"));
    }
    format!("v{}{suffix}", padded.join("."))
}

#[cfg(test)]
mod tests {
    use super::display;

    #[test]
    fn pads_each_number_to_four_digits() {
        assert_eq!(display("1.1.1"), "v0001.0001.0001");
        assert_eq!(display("1000.1.1"), "v1000.0001.0001");
        assert_eq!(display("1.0.1"), "v0001.0000.0001");
        assert_eq!(display("0.1.0-pre.7"), "v0000.0001.0000-pre.7");
        assert_eq!(display("12.345.6"), "v0012.0345.0006");
    }

    #[test]
    fn leaves_unexpected_versions_readable() {
        assert_eq!(display("dev"), "vdev");
        assert_eq!(display("1.2"), "v1.2");
        assert_eq!(display("1.2.3.4"), "v1.2.3.4");
    }
}
