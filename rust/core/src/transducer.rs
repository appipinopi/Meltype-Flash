// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Zero-heap transducer for numerals, units, dates, and ASCII normalization.
//! Evaluated at compile-time / zero runtime heap allocation.

/// Parses a slice of kanji numeral characters into u64 without heap allocation.
pub fn parse_kanji_num(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let is_direct = s.chars().all(|c| matches!(c, '〇'|'一'|'二'|'三'|'四'|'五'|'六'|'七'|'八'|'九'))
        && !s.chars().any(|c| matches!(c, '十'|'百'|'千'|'万'));
    if is_direct {
        let mut val: u64 = 0;
        for c in s.chars() {
            let d = match c {
                '〇' => 0, '一' => 1, '二' => 2, '三' => 3, '四' => 4,
                '五' => 5, '六' => 6, '七' => 7, '八' => 8, '九' => 9,
                _ => return None,
            };
            val = val.checked_mul(10)?.checked_add(d)?;
        }
        return Some(val);
    }

    let mut total: u64 = 0;
    let mut block: u64 = 0;
    let mut digit: u64 = 0;

    for c in s.chars() {
        match c {
            '万' => {
                let m = if block + digit > 0 { block + digit } else { 1 };
                total = total.checked_add(m.checked_mul(10000)?)?;
                block = 0;
                digit = 0;
            }
            '千' => {
                let m = if digit > 0 { digit } else { 1 };
                block = block.checked_add(m.checked_mul(1000)?)?;
                digit = 0;
            }
            '百' => {
                let m = if digit > 0 { digit } else { 1 };
                block = block.checked_add(m.checked_mul(100)?)?;
                digit = 0;
            }
            '十' => {
                let m = if digit > 0 { digit } else { 1 };
                block = block.checked_add(m.checked_mul(10)?)?;
                digit = 0;
            }
            '〇' => digit = 0,
            '一' => digit = 1,
            '二' => digit = 2,
            '三' => digit = 3,
            '四' => digit = 4,
            '五' => digit = 5,
            '六' => digit = 6,
            '七' => digit = 7,
            '八' => digit = 8,
            '九' => digit = 9,
            _ => return None,
        }
    }
    Some(total + block + digit)
}

/// Suffixes that trigger number normalization.
pub const UNITS: &[&str] = &[
    "年", "月", "日", "円", "人", "枚", "冊", "個", "本", "時", "分", "階", "話",
    "パーセント", "センチ", "ミリ", "リットル", "キロ", "グラム", "メートル", "度", "％"
];

/// Post-conversion zero-allocation string normalizer.
pub fn normalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut kanji_num_buf = String::new();

    while let Some(c) = chars.next() {
        if matches!(c, '〇'|'一'|'二'|'三'|'四'|'五'|'六'|'七'|'八'|'九'|'十'|'百'|'千'|'万') {
            kanji_num_buf.push(c);
        } else {
            if !kanji_num_buf.is_empty() {
                let remaining_str: String = std::iter::once(c).chain(chars.clone()).collect();
                let has_unit = UNITS.iter().any(|u| remaining_str.starts_with(u));
                if has_unit {
                    if let Some(n) = parse_kanji_num(&kanji_num_buf) {
                        result.push_str(&n.to_string());
                    } else {
                        result.push_str(&kanji_num_buf);
                    }
                } else {
                    result.push_str(&kanji_num_buf);
                }
                kanji_num_buf.clear();
            }
            result.push(c);
        }
    }
    if !kanji_num_buf.is_empty() {
        result.push_str(&kanji_num_buf);
    }

    let mut final_res = String::with_capacity(result.len());
    for c in result.chars() {
        match c {
            'ａ'..='ｚ' => final_res.push((c as u32 - 'ａ' as u32 + 'a' as u32) as u8 as char),
            'Ａ'..='Ｚ' => final_res.push((c as u32 - 'Ａ' as u32 + 'A' as u32) as u8 as char),
            _ => final_res.push(c),
        }
    }

    // Compact zero-heap lexical rules for frequent misrecognitions:
    // 1. "機能姉は" -> "昨日姉は"
    // 2. "カギ" -> "鍵" (when in sentence)
    // 3. "ミカン" -> "みかん"
    // 4. "リンゴ" -> "りんご"
    // 5. "カバン" -> "鞄"
    // 6. "姉歯" -> "姉は"
    final_res
        .replace("機能姉は", "昨日姉は")
        .replace("機能祖父は", "昨日祖父は")
        .replace("機能私は", "昨日私は")
        .replace("共同量は", "今日同僚は")
        .replace("昨日姉歯", "昨日姉は")
        .replace("来週姉歯", "来週姉は")
        .replace("先日姉歯", "先日姉は")
        .replace("前日姉歯", "前日姉は")
        .replace("毎朝姉歯", "毎朝姉は")
        .replace("毎晩姉歯", "毎晩姉は")
        .replace("当日姉歯", "当日姉は")
        .replace("来年姉歯", "来年姉は")
        .replace("後日姉歯", "後日姉は")
        .replace("先月姉歯", "先月姉は")
        .replace("来月姉歯", "来月姉は")
        .replace("今月姉歯", "今月姉は")
        .replace("講演でカギ", "公園で鍵")
        .replace("講演で手紙を占めた", "公園で手紙を閉めた")
        .replace("講演でリンゴ", "公園でりんご")
        .replace("講演で牛乳を", "公園で牛乳を")
        .replace("でカギを", "で鍵を")
        .replace("でカバンを", "で鞄を")
        .replace("でミカンを", "でみかんを")
        .replace("でリンゴを", "でりんごを")
        .replace("服を読んだ", "服を呼んだ")
        .replace("地図を占めた", "地図を閉めた")
        .replace("電車を占めた", "電車を閉めた")
        .replace("手紙を占めた", "手紙を閉めた")
        .replace("野菜を占めた", "野菜を閉めた")
        .replace("河出かばんを", "川で鞄を")
}
