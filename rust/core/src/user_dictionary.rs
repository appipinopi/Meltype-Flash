// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
use std::io;
pub struct Import {
    pub words: Vec<(String, String)>,
    pub skipped: usize,
    pub encoding: &'static str,
}
pub fn hiragana(text: &str) -> String {
    text.chars()
        .map(|c| {
            if ('ァ'..='ヶ').contains(&c) {
                char::from_u32(c as u32 - 0x60).unwrap()
            } else {
                c
            }
        })
        .collect()
}
pub fn valid(reading: &str, word: &str) -> bool {
    !reading.is_empty()
        && reading
            .chars()
            .all(|c| ('ぁ'..='ゖ').contains(&c) || "ー・".contains(c) || c.is_ascii_alphanumeric())
        && !word.is_empty()
        && !word.contains(['\t', '\n', '\r', '\0'])
}
pub fn parse(bytes: &[u8]) -> io::Result<Import> {
    if bytes.starts_with(b"bplist") {
        return Err(io::Error::other(
            "convert binary plist to XML with plutil first",
        ));
    }
    let (text, encoding) = decode(bytes)?;
    let mut result = Import {
        words: vec![],
        skipped: 0,
        encoding,
    };
    if text.trim_start().starts_with('<') {
        // No external DTD access; roxmltree never resolves network resources.
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        };
        let document =
            roxmltree::Document::parse_with_options(&text, options).map_err(io::Error::other)?;
        let root = document.root_element();
        let array = root
            .children()
            .find(|n| n.has_tag_name("array"))
            .filter(|_| root.has_tag_name("plist"))
            .ok_or_else(|| io::Error::other("not an Apple user dictionary plist"))?;
        result.encoding = "plist";
        for item in array.children().filter(|n| n.is_element()) {
            if !item.has_tag_name("dict") {
                result.skipped += 1;
                continue;
            }
            let children: Vec<_> = item.children().filter(|n| n.is_element()).collect();
            let mut reading = "";
            let mut word = "";
            for pair in children.windows(2) {
                if pair[0].has_tag_name("key") && pair[1].has_tag_name("string") {
                    match pair[0].text().unwrap_or("") {
                        "shortcut" => reading = pair[1].text().unwrap_or(""),
                        "phrase" => word = pair[1].text().unwrap_or(""),
                        _ => {}
                    }
                }
            }
            add(&mut result, reading, word);
        }
    } else {
        for line in text
            .lines()
            .filter(|s| !s.is_empty() && !s.starts_with(['#', '!']))
        {
            let mut fields = line.split('\t');
            let reading = fields.next().unwrap_or("");
            if let Some(word) = fields.next() {
                add(&mut result, reading, word);
            } else {
                result.skipped += 1;
            }
        }
    }
    Ok(result)
}
fn add(result: &mut Import, reading: &str, word: &str) {
    let reading = hiragana(reading.trim());
    let word = word.trim();
    if valid(&reading, word) {
        result.words.push((reading, word.into()));
    } else {
        result.skipped += 1;
    }
}
pub(crate) fn decode(bytes: &[u8]) -> io::Result<(String, &'static str)> {
    let utf16 = |bytes: &[u8], be: bool| -> io::Result<String> {
        if !bytes.len().is_multiple_of(2) {
            return Err(io::Error::other("odd UTF-16 byte count"));
        }
        let words: Vec<_> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if be {
                    u16::from_be_bytes([b[0], b[1]])
                } else {
                    u16::from_le_bytes([b[0], b[1]])
                }
            })
            .collect();
        String::from_utf16(&words).map_err(io::Error::other)
    };
    if bytes.starts_with(&[0xff, 0xfe]) {
        return Ok((utf16(&bytes[2..], false)?, "UTF-16"));
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return Ok((utf16(&bytes[2..], true)?, "UTF-16 BE"));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(bytes) {
        if !text.contains('\0') {
            return Ok((text.into(), "UTF-8"));
        }
    }
    if bytes.len().is_multiple_of(2) && bytes.iter().skip(1).step_by(2).any(|b| *b == 0) {
        return Ok((utf16(bytes, false)?, "UTF-16"));
    }
    Err(io::Error::other(
        "use UTF-8 or UTF-16 (Shift_JIS is unsupported)",
    ))
}
pub fn export(words: impl Iterator<Item = (String, String)>) -> Vec<u8> {
    let mut text=String::from("!Microsoft IME Dictionary Tool\r\n!Format:WORDLIST\r\n!User Dictionary Name: Meltype\r\n\r\n");
    for (reading, word) in words {
        if valid(&reading, &word) {
            text.push_str(&format!("{reading}\t{word}\t名詞\r\n"));
        }
    }
    let mut bytes = vec![0xff, 0xfe];
    for word in text.encode_utf16() {
        bytes.extend(word.to_le_bytes());
    }
    bytes
}
