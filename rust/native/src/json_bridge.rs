// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! TSF JSON compatibility, including raw UTF-16 escapes and the upstream depth
//! limit/number conversion. The C++ owner receives values, never parses bytes.
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    slice,
};
#[derive(Debug, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<Value>),
    Object(BTreeMap<Vec<u16>, Value>),
}
fn decode_utf8(raw: &[u8]) -> Vec<u16> {
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{
            MultiByteToWideChar, CP_UTF8, MULTI_BYTE_TO_WIDE_CHAR_FLAGS,
        };
        if raw.is_empty() || raw.len() > i32::MAX as usize {
            return Vec::new();
        }
        // Match the original Windows replacement of malformed UTF8 sequences;
        // Rust's lossy decoder can emit a different number of replacement units.
        unsafe {
            let count = MultiByteToWideChar(CP_UTF8, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), raw, None);
            if count <= 0 {
                return Vec::new();
            }
            let mut result = vec![0; count as usize];
            MultiByteToWideChar(
                CP_UTF8,
                MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
                raw,
                Some(&mut result),
            );
            result
        }
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(raw).encode_utf16().collect()
    }
}
struct Parser<'a> {
    input: &'a [u8],
    index: usize,
}
impl Parser<'_> {
    fn skip(&mut self) {
        while self
            .input
            .get(self.index)
            .is_some_and(|c| b" \t\r\n".contains(c))
        {
            self.index += 1;
        }
    }
    fn take(&mut self, word: &[u8]) -> bool {
        if self.input[self.index..].starts_with(word) {
            self.index += word.len();
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > 32 {
            return None;
        }
        self.skip();
        match *self.input.get(self.index)? {
            b'"' => Some(Value::String(self.string()?)),
            b'[' => {
                self.index += 1;
                self.skip();
                let mut values = Vec::new();
                if self.take(b"]") {
                    return Some(Value::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.skip();
                    if self.take(b"]") {
                        return Some(Value::Array(values));
                    }
                    if !self.take(b",") {
                        return None;
                    }
                }
            }
            b'{' => {
                self.index += 1;
                self.skip();
                let mut values = BTreeMap::new();
                if self.take(b"}") {
                    return Some(Value::Object(values));
                }
                loop {
                    self.skip();
                    let key = self.string()?;
                    self.skip();
                    if !self.take(b":") {
                        return None;
                    }
                    values.insert(key, self.value(depth + 1)?);
                    self.skip();
                    if self.take(b"}") {
                        return Some(Value::Object(values));
                    }
                    if !self.take(b",") {
                        return None;
                    }
                }
            }
            _ => {
                if self.take(b"true") {
                    return Some(Value::Bool(true));
                }
                if self.take(b"false") {
                    return Some(Value::Bool(false));
                }
                if self.take(b"null") {
                    return Some(Value::Null);
                }
                let start = self.index;
                while self
                    .input
                    .get(self.index)
                    .is_some_and(|c| *c == 0 || b"+-0123456789.eE".contains(c))
                {
                    self.index += 1;
                }
                if start == self.index {
                    return None;
                }
                // strchr also matched its terminating NUL in the old scanner;
                // strtod then read only the prefix before that NUL.
                let prefix = self.input[start..self.index]
                    .split(|byte| *byte == 0)
                    .next()?;
                let number = CString::new(prefix).ok()?;
                // Preserve the original TSF parser's strtod conversion, including
                // its permissive numeric tokens. Core protocol producers emit
                // ordinary JSON numbers; this boundary also serves old shells.
                Some(Value::Number(unsafe {
                    libc::strtod(number.as_ptr(), std::ptr::null_mut())
                }))
            }
        }
    }
    fn string(&mut self) -> Option<Vec<u16>> {
        if !self.take(b"\"") {
            return None;
        }
        let mut out = Vec::new();
        let mut raw = Vec::new();
        loop {
            let byte = *self.input.get(self.index)?;
            self.index += 1;
            match byte {
                b'"' => {
                    out.extend(decode_utf8(&raw));
                    return Some(out);
                }
                b'\\' => {
                    let escape = *self.input.get(self.index)?;
                    self.index += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => raw.push(escape),
                        b'b' => raw.push(8),
                        b'f' => raw.push(12),
                        b'n' => raw.push(10),
                        b'r' => raw.push(13),
                        b't' => raw.push(9),
                        b'u' => {
                            let mut code = 0u16;
                            for _ in 0..4 {
                                let byte = *self.input.get(self.index)?;
                                self.index += 1;
                                let digit = (byte as char).to_digit(16)?;
                                code = code * 16 + digit as u16;
                            }
                            out.extend(decode_utf8(&raw));
                            raw.clear();
                            out.push(code);
                        }
                        _ => return None,
                    }
                }
                _ => raw.push(byte),
            }
        }
    }
}
fn parse(input: &[u8]) -> Option<Value> {
    let mut parser = Parser { input, index: 0 };
    let result = parser.value(1)?;
    parser.skip();
    (parser.index == input.len()).then_some(result)
}
#[cfg(windows)]
pub(crate) fn valid(input: &[u8]) -> bool {
    parse(input).is_some()
}
#[cfg(windows)]
pub(crate) fn mode_reply(input: &[u8]) -> Option<u32> {
    let value = parse(input)?;
    let Value::Object(fields) = value else {
        return Some(0);
    };
    Some(
        ["active", "code", "english", "consumed"]
            .iter()
            .enumerate()
            .fold(0, |bits, (index, key)| {
                let key = key.encode_utf16().collect::<Vec<_>>();
                bits | if fields.get(&key) == Some(&Value::Bool(true)) {
                    1 << index
                } else {
                    0
                }
            }),
    )
}
#[cfg(windows)]
pub(crate) fn key_reply(input: &[u8]) -> Option<(u32, u32)> {
    let value = parse(input)?;
    let get = |key: &str| match &value {
        Value::Object(fields) => fields.get(&key.encode_utf16().collect::<Vec<_>>()),
        _ => None,
    };
    let boolean = |key: &str| get(key) == Some(&Value::Bool(true));
    let mut flags = [
        "active",
        "directDetection",
        "directEnglishWord",
        "switchToJapanese",
        "consumed",
    ]
    .iter()
    .enumerate()
    .fold(0, |bits, (i, key)| bits | (u32::from(boolean(key)) << i));
    // JsonValue.array is empty for every non-array type; missing view is null.
    if !matches!(get("commits"), Some(Value::Array(items)) if !items.is_empty()) {
        flags |= 32;
    }
    if matches!(get("view"), None | Some(Value::Null)) {
        flags |= 64;
    }
    // #881: kept separate from the existing key-reply bits for older clients.
    flags |= u32::from(boolean("fullWidthSpace")) << 7;
    let mode = u32::from(boolean("active"))
        | (u32::from(boolean("code")) << 1)
        | (u32::from(boolean("english")) << 2);
    Some((flags, mode))
}
pub type Emit = unsafe extern "C" fn(*mut c_void, u32, f64, *const u16, u32) -> i32;
// Match the former C++ JsonValue -> CommitJson/ViewJson boundary, including
// UTF-16 replacement and MSVC's truncating numeric conversion (overflow=MIN).
#[cfg(windows)]
pub(crate) fn application_reply(input: &[u8]) -> Option<serde_json::Value> {
    fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
        match v {
            Value::Object(fields) => fields.get(&key.encode_utf16().collect::<Vec<_>>()),
            _ => None,
        }
    }
    fn text(v: Option<&Value>) -> String {
        match v {
            Some(Value::String(units)) => String::from_utf16_lossy(units),
            _ => String::new(),
        }
    }
    fn boolean(v: Option<&Value>) -> bool {
        v == Some(&Value::Bool(true))
    }
    fn integer(v: Option<&Value>) -> i32 {
        let Some(Value::Number(n)) = v else {
            return 0;
        };
        let n = n.trunc();
        if n >= f64::from(i32::MIN) && n <= f64::from(i32::MAX) {
            n as i32
        } else {
            i32::MIN
        }
    }
    fn array(v: Option<&Value>) -> &[Value] {
        match v {
            Some(Value::Array(items)) => items,
            _ => &[],
        }
    }
    let value = parse(input)?;
    let commits: Vec<_> = array(get(&value,"commits")).iter().map(|edit| {
        let mut value = serde_json::json!({
            "text":text(get(edit,"text")),"deleteBefore":integer(get(edit,"deleteBefore")),
            "expect":match get(edit,"expect") {Some(v@Value::String(_))=>Some(text(Some(v))),_=>None}
        });
        if let Some(caret) = get(edit, "caretBack") {
            value["caretBack"] = serde_json::json!(integer(Some(caret)));
        }
        value
    }).collect();
    let view = match get(&value, "view") {
        None | Some(Value::Null) => serde_json::Value::Null,
        Some(view) => {
            let mut fields = serde_json::Map::new();
            for name in ["candidates", "notes", "predictions", "clauses"] {
                fields.insert(
                    name.into(),
                    serde_json::json!(array(get(view, name))
                        .iter()
                        .map(|v| text(Some(v)))
                        .collect::<Vec<_>>()),
                );
            }
            for name in ["suggestion", "meaning", "typed", "text"] {
                fields.insert(name.into(), serde_json::json!(text(get(view, name))));
            }
            for name in ["selectedIndex", "selectedPrediction", "selectedClause"] {
                fields.insert(name.into(), serde_json::json!(integer(get(view, name))));
            }
            fields.insert(
                "converting".into(),
                serde_json::json!(boolean(get(view, "converting"))),
            );
            serde_json::Value::Object(fields)
        }
    };
    let mut reply =
        serde_json::json!({"active":boolean(get(&value,"active")),"commits":commits,"view":view});
    if let Some(expected) = get(&value, "selectionExpect") {
        reply["selectionExpect"] = match expected {
            Value::String(_) => serde_json::json!(text(Some(expected))),
            _ => serde_json::Value::Null,
        };
    }
    Some(reply)
}
unsafe fn emit(value: &Value, owner: *mut c_void, callback: Emit) -> bool {
    let empty = std::ptr::null();
    match value {
        Value::Null => callback(owner, 0, 0.0, empty, 0) != 0,
        Value::Bool(value) => callback(owner, 1, if *value { 1.0 } else { 0.0 }, empty, 0) != 0,
        Value::Number(value) => callback(owner, 2, *value, empty, 0) != 0,
        Value::String(value) => callback(owner, 3, 0.0, value.as_ptr(), value.len() as u32) != 0,
        Value::Array(values) => {
            callback(owner, 4, 0.0, empty, 0) != 0
                && values.iter().all(|value| emit(value, owner, callback))
                && callback(owner, 5, 0.0, empty, 0) != 0
        }
        Value::Object(values) => {
            callback(owner, 6, 0.0, empty, 0) != 0
                && values.iter().all(|(key, value)| {
                    callback(owner, 7, 0.0, key.as_ptr(), key.len() as u32) != 0
                        && emit(value, owner, callback)
                })
                && callback(owner, 8, 0.0, empty, 0) != 0
        }
    }
}
/// # Safety
/// input is readable for length bytes. callback/code/owner remain valid for this
/// synchronous call and never throw. Events: 0 null,1 bool(number),2 number,
/// 3 UTF16 string,4/5 array begin/end,6/7/8 object begin/key/end. Text is borrowed
/// for the callback only. Zero callback result stops traversal. Invalid JSON
/// emits no events. Returns 1 success,0 parse failure/cancel/invalid arguments.
#[no_mangle]
pub unsafe extern "C" fn meltype_json_visit(
    input: *const u8,
    length: u32,
    owner: *mut c_void,
    callback: Option<Emit>,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if input.is_null() || callback.is_none() {
            return 0;
        }
        let Some(value) = parse(slice::from_raw_parts(input, length as usize)) else {
            return 0;
        };
        i32::from(emit(&value, owner, callback.unwrap()))
    }))
    .unwrap_or(0)
}
pub type EmitUtf8 = unsafe extern "C" fn(*mut c_void, u32, f64, *const u8, u32) -> i32;
unsafe fn emit_utf8(value: &serde_json::Value, owner: *mut c_void, callback: EmitUtf8) -> bool {
    use serde_json::Value;
    let empty = std::ptr::null();
    match value {
        Value::Null => callback(owner, 0, 0., empty, 0) != 0,
        Value::Bool(value) => callback(owner, 1, f64::from(*value), empty, 0) != 0,
        Value::Number(value) => callback(owner, 2, value.as_f64().unwrap_or(0.), empty, 0) != 0,
        Value::String(value) => callback(owner, 3, 0., value.as_ptr(), value.len() as u32) != 0,
        Value::Array(values) => {
            callback(owner, 4, 0., empty, 0) != 0
                && values.iter().all(|value| emit_utf8(value, owner, callback))
                && callback(owner, 5, 0., empty, 0) != 0
        }
        Value::Object(values) => {
            callback(owner, 6, 0., empty, 0) != 0
                && values.iter().all(|(key, value)| {
                    callback(owner, 7, 0., key.as_ptr(), key.len() as u32) != 0
                        && emit_utf8(value, owner, callback)
                })
                && callback(owner, 8, 0., empty, 0) != 0
        }
    }
}
/// # Safety
/// input is readable for length bytes; callback/owner remain valid for this
/// synchronous call and callback never unwinds. Events match meltype_json_visit,
/// with UTF-8 text borrowed only during callbacks (including escaped NUL).
/// Strict JSON/UTF-8 parsing rejects malformed input before emitting any event;
/// unlike the TSF compatibility entry point, legacy numeric/UTF-16 quirks do not
/// apply. Duplicate keys use the last value. Zero callback result cancels.
#[no_mangle]
pub unsafe extern "C" fn meltype_json_visit_utf8(
    input: *const u8,
    length: u32,
    owner: *mut c_void,
    callback: Option<EmitUtf8>,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if input.is_null() || callback.is_none() {
            return 0;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(slice::from_raw_parts(
            input,
            length as usize,
        )) else {
            return 0;
        };
        i32::from(emit_utf8(&value, owner, callback.unwrap()))
    }))
    .unwrap_or(0)
}
fn quote(text: &[u16]) -> String {
    let mut out = String::from("\"");
    for ch in String::from_utf16_lossy(text).chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch < ' ' => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
/// # Safety
/// text is readable for length UTF16 units (null permitted only for zero length).
/// Returns UTF8 quoted JSON allocated for meltype_free, or null on invalid input.
#[no_mangle]
pub unsafe extern "C" fn meltype_json_quote(text: *const u16, length: u32) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if text.is_null() && length != 0 {
            return std::ptr::null_mut();
        }
        let text = if length == 0 {
            &[]
        } else {
            slice::from_raw_parts(text, length as usize)
        };
        crate::allocate(&quote(text))
    }))
    .unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn application_projection_preserves_bracket_caret_requests() {
        let value = application_reply(
            br#"{"active":true,"commits":[{"text":"()","caretBack":1},{"text":"plain"}]}"#,
        )
        .unwrap();
        assert_eq!(value["commits"][0]["caretBack"], 1);
        assert!(value["commits"][1].get("caretBack").is_none());
    }
    #[cfg(windows)]
    #[test]
    fn application_projection_keeps_typed_defaults_and_utf16_replacement() {
        let value=application_reply(br#"{"active":1,"commits":[{"text":"\ud800\ud83d\ude00","deleteBefore":2.9,"expect":false},null],"view":{"text":false,"predictions":[7,"\udc00"],"selectedIndex":"2","converting":1}}"#).unwrap();
        assert_eq!(value["active"], false);
        assert_eq!(
            value["commits"],
            serde_json::json!([{"text":"�😀","deleteBefore":2,"expect":null},{"text":"","deleteBefore":0,"expect":null}])
        );
        assert_eq!(value["view"]["text"], "");
        assert_eq!(value["view"]["predictions"], serde_json::json!(["", "�"]));
        assert_eq!(value["view"]["selectedIndex"], 0);
        assert_eq!(value["view"]["converting"], false);
    }
    #[cfg(windows)]
    #[test]
    fn application_projection_preserves_selection_guard_and_rejectable_types() {
        let expected = "原文😀";
        let value = application_reply(
            serde_json::json!({"selectionExpect":expected})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(value["selectionExpect"], expected);
        assert!(application_reply(b"{}")
            .unwrap()
            .get("selectionExpect")
            .is_none());
        for value in [
            serde_json::Value::Null,
            serde_json::json!(true),
            serde_json::json!(123),
            serde_json::json!([]),
        ] {
            let reply = application_reply(
                serde_json::json!({"selectionExpect":value})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
            assert!(reply
                .get("selectionExpect")
                .is_some_and(serde_json::Value::is_null));
        }
    }
    #[cfg(windows)]
    #[test]
    fn application_projection_numeric_boundaries_match_msvc() {
        for (input, expected) in [
            ("2147483647.9", i32::MAX),
            ("2147483648", i32::MIN),
            ("-2147483648.9", i32::MIN),
            ("-2147483649", i32::MIN),
            ("1e500", i32::MIN),
            ("-1.9", -1),
        ] {
            let input = format!("{{\"view\":{{\"selectedIndex\":{input}}}}}");
            assert_eq!(
                application_reply(input.as_bytes()).unwrap()["view"]["selectedIndex"],
                expected
            );
        }
    }
    #[cfg(windows)]
    #[test]
    fn application_projection_handles_nonobject_null_duplicate_and_invalid_inputs() {
        assert_eq!(
            application_reply(b"[]"),
            Some(serde_json::json!({"active":false,"commits":[],"view":null}))
        );
        assert_eq!(
            application_reply(br#"{"view":{"text":"old"},"view":null}"#).unwrap()["view"],
            serde_json::Value::Null
        );
        assert!(application_reply(br#"{"view":false}"#).unwrap()["view"].is_object());
        assert_eq!(application_reply(b"{} invalid"), None);
    }
    #[cfg(windows)]
    #[test]
    fn key_flags_preserve_boolean_types_and_all_combinations() {
        for bits in 0..128u32 {
            let reply = serde_json::json!({
                "active":bits&1!=0,"directDetection":bits&2!=0,
                "directEnglishWord":bits&4!=0,"switchToJapanese":bits&8!=0,
                "consumed":bits&16!=0,"commits":if bits&32!=0 {vec![]} else {vec![0]},
                "view":if bits&64!=0 {serde_json::Value::Null} else {serde_json::json!({})},
                "code":true,"english":true
            });
            assert_eq!(
                key_reply(reply.to_string().as_bytes()),
                Some((bits, (bits & 1) | 6))
            );
        }
        assert_eq!(
            key_reply(
                br#"{"active":1,"directDetection":"true","consumed":[],"commits":{},"view":false}"#
            ),
            Some((32, 0))
        );
    }
    #[cfg(windows)]
    #[test]
    fn key_flags_keep_compatible_missing_duplicate_and_surrogate_values() {
        assert_eq!(
            key_reply(br#"{"active":true,"fullWidthSpace":true}"#),
            Some((225, 1))
        );
        assert_eq!(
            key_reply(br#"{"active":true,"fullWidthSpace":1}"#),
            Some((97, 1))
        );
        assert_eq!(key_reply(b"[]"), Some((96, 0)));
        assert_eq!(
            key_reply(br#"{"active":true,"active":false,"commits":["\ud800"]}"#),
            Some((64, 0))
        );
        assert_eq!(
            key_reply(br#"{"view":"\ud800","commits":null}"#),
            Some((32, 0))
        );
        assert_eq!(key_reply(b"{} invalid"), None);
    }
    #[test]
    fn utf16_escapes_keep_unpaired_units_and_nuls() {
        assert_eq!(
            parse(br#""a\ud800\u0000\udc00\ud83d\ude00""#),
            Some(Value::String(vec![97, 0xd800, 0, 0xdc00, 0xd83d, 0xde00]))
        );
    }
    #[test]
    fn depth_limit_matches_empty_and_scalar_boundaries() {
        let root = format!("{}0{}", "[".repeat(31), "]".repeat(31));
        assert!(parse(root.as_bytes()).is_some());
        let deep = format!("{}0{}", "[".repeat(32), "]".repeat(32));
        assert!(parse(deep.as_bytes()).is_none());
        assert!(parse(format!("{}{}", "[".repeat(32), "]".repeat(32)).as_bytes()).is_some());
    }
    #[test]
    fn duplicate_keys_replace_and_invalid_tail_is_rejected() {
        assert_eq!(parse(b"1\0e5"), Some(Value::Number(1.0)));
        assert_eq!(
            parse(br#"{"a":true,"a":false}"#),
            Some(Value::Object(BTreeMap::from([(
                vec![97],
                Value::Bool(false)
            )])))
        );
        assert!(parse(b"{} trailing").is_none());
        assert!(parse(br#"[1,]"#).is_none());
    }
    #[test]
    fn quote_preserves_upstream_control_escapes() {
        assert_eq!(
            quote(&[0, 8, 12, 9, 10, 13, 34, 92, 0xd800]),
            "\"\\u0000\\u0008\\u000c\\t\\n\\r\\\"\\\\�\""
        );
    }
    #[test]
    fn utf8_visitor_delimits_unicode_nuls_and_rejects_before_callbacks() {
        type Events = Vec<(u32, f64, Vec<u8>)>;
        unsafe extern "C" fn collect(
            owner: *mut c_void,
            kind: u32,
            number: f64,
            text: *const u8,
            length: u32,
        ) -> i32 {
            let events = &mut *(owner as *mut Events);
            let text = if text.is_null() {
                Vec::new()
            } else {
                slice::from_raw_parts(text, length as usize).to_vec()
            };
            events.push((kind, number, text));
            1
        }
        unsafe extern "C" fn stop(_: *mut c_void, _: u32, _: f64, _: *const u8, _: u32) -> i32 {
            0
        }
        let mut events = Events::new();
        let owner = (&mut events as *mut Events).cast();
        let text = r#"{"text":"日本😀\u0000","items":[true,2,null]}"#;
        assert_eq!(
            unsafe {
                meltype_json_visit_utf8(text.as_ptr(), text.len() as u32, owner, Some(collect))
            },
            1
        );
        assert!(events
            .iter()
            .any(|(kind, _, text)| *kind == 3 && text == "日本😀\0".as_bytes()));
        assert_eq!(events.first().unwrap().0, 6);
        assert_eq!(events.last().unwrap().0, 8);
        for invalid in [
            b"[1,]".as_slice(),
            b"1e9999",
            b"1\0trailing",
            b"\"\xff\"",
            br#""\ud800""#,
            b"{} trailing",
        ] {
            events.clear();
            assert_eq!(
                unsafe {
                    meltype_json_visit_utf8(
                        invalid.as_ptr(),
                        invalid.len() as u32,
                        owner,
                        Some(collect),
                    )
                },
                0
            );
            assert!(events.is_empty());
        }
        assert_eq!(
            unsafe {
                meltype_json_visit_utf8(
                    text.as_ptr(),
                    text.len() as u32,
                    std::ptr::null_mut(),
                    Some(stop),
                )
            },
            0
        );
        assert_eq!(
            unsafe { meltype_json_visit_utf8(std::ptr::null(), 0, owner, Some(collect)) },
            0
        );
        assert_eq!(
            unsafe { meltype_json_visit_utf8(text.as_ptr(), text.len() as u32, owner, None) },
            0
        );
    }
}
