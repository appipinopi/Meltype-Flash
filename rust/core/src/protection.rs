// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 0924haruto12, appipinopi
//! Structural protection, with UTF-8 byte ranges that always lie on scalar
//! boundaries. Transformations share this scanner instead of ad hoc exceptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
}
fn path(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._-~/\\%+@".contains(c)
}
fn element(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._~".contains(c)
}
fn mention(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_-".contains(c)
}
pub fn scan(raw: &str) -> Vec<Span> {
    let chars: Vec<_> = raw.char_indices().collect();
    let n = chars.len();
    let mut result: Vec<Span> = vec![];
    let mut i = 0;
    // Every position in one scheme-character run has the same endpoint.
    // Reuse it when a run is ordinary prose instead of scanning its tail
    // again for every letter. Protected spans and UTF-8 offsets are unchanged.
    let mut scheme_end = 0;
    let end_byte = |index: usize| chars.get(index).map_or(raw.len(), |(b, _)| *b);
    while i < n {
        let (byte, c) = chars[i];
        let rest = &raw[byte..];
        let mut found = None;
        if c.is_ascii_alphabetic() {
            if scheme_end <= i {
                scheme_end = (i + 1..n)
                    .find(|j| {
                        let c = chars[*j].1;
                        !(c.is_ascii_alphanumeric() || "+-.".contains(c))
                    })
                    .unwrap_or(n);
            }
            let end = scheme_end;
            if raw[end_byte(end)..].starts_with("://") {
                let mut j = end + 3;
                while j < n
                    && (chars[j].1.is_ascii_alphanumeric()
                        || "-._~:/?#[]@!$&'()*+,;=%".contains(chars[j].1))
                {
                    j += 1;
                }
                found = Some((i, j, "url"));
            }
        }
        if found.is_none()
            && (rest.starts_with("\\\\")
                || c.is_ascii_alphabetic()
                    && rest
                        .get(1..)
                        .is_some_and(|r| r.starts_with(":\\") || r.starts_with(":/")))
        {
            let offset = if rest.starts_with("\\\\") { 2 } else { 3 };
            let mut j = i + offset;
            if j < n && element(chars[j].1) {
                while j < n && path(chars[j].1) {
                    j += 1;
                }
                found = Some((i, j, "windows_path"));
            }
        }
        if found.is_none() && c == '@' {
            let mut begin = i;
            while begin > 0
                && (chars[begin - 1].1.is_ascii_alphanumeric()
                    || "._%+-".contains(chars[begin - 1].1))
            {
                begin -= 1;
            }
            if begin < i && result.last().is_none_or(|p| end_byte(begin) >= p.end) {
                let mut j = i + 1;
                while j < n && (chars[j].1.is_ascii_alphanumeric() || ".-".contains(chars[j].1)) {
                    j += 1;
                }
                found = Some((begin, j, "email"));
            } else if (i == 0 || !mention(chars[i - 1].1))
                && i + 1 < n
                && (chars[i + 1].1.is_ascii_alphanumeric() || chars[i + 1].1 == '_')
            {
                let mut j = i + 2;
                while j < n && mention(chars[j].1) {
                    j += 1;
                }
                found = Some((i, j, "mention"));
            }
        }
        if found.is_none() && (i == 0 || !path(chars[i - 1].1)) {
            // A leading dot belongs to a filename or fractional number, not
            // Japanese punctuation (#844, #442). Limit this to token starts
            // so a sentence-ending dot before Japanese remains punctuation.
            let token_start =
                i == 0 || chars[i - 1].1.is_whitespace() || "([{'\"=:+".contains(chars[i - 1].1);
            if token_start && c == '.' && i + 1 < n && chars[i + 1].1.is_ascii_alphanumeric() {
                let mut j = i + 1;
                if chars[j].1.is_ascii_digit() {
                    while j < n && chars[j].1.is_ascii_digit() {
                        j += 1;
                    }
                    found = Some((i, j, "number"));
                } else {
                    while j < n
                        && (chars[j].1.is_ascii_alphanumeric() || "._-".contains(chars[j].1))
                    {
                        j += 1;
                    }
                    found = Some((i, j, "filename"));
                }
            }
        }
        if found.is_none() && (i == 0 || !path(chars[i - 1].1)) {
            let offset = if rest.starts_with("../") {
                3
            } else if rest.starts_with("./") || rest.starts_with("~/") {
                2
            } else if c == '/' {
                1
            } else {
                0
            };
            if offset > 0 && i + offset < n && element(chars[i + offset].1) {
                let mut j = i + offset;
                while j < n && path(chars[j].1) {
                    j += 1;
                }
                found = Some((i, j, "posix_path"));
            }
        }
        if let Some((start, end, kind)) = found {
            result.push(Span {
                start: end_byte(start),
                end: end_byte(end),
                kind,
            });
            i = end;
        } else {
            i += 1;
        }
    }
    result
}
pub fn transform(raw: &str, mut f: impl FnMut(&str) -> String) -> String {
    let mut out = String::new();
    let mut offset = 0;
    for span in scan(raw) {
        out.push_str(&f(&raw[offset..span.start]));
        out.push_str(&raw[span.start..span.end]);
        offset = span.end;
    }
    out.push_str(&f(&raw[offset..]));
    out
}
