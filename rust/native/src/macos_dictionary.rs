// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! macOS plist import compatibility with the original Foundation implementation.
use serde::Deserialize;
use std::ffi::{c_char, c_void};
use unicode_normalization::UnicodeNormalization;

#[cfg(any(target_os = "macos", test))]
fn decode_existing_utf8(bytes: &[u8], consume_bom: bool) -> Result<&str, std::str::Utf8Error> {
    // macOS 26のFoundationは先頭BOMを1個消費し、以前の版は保持する。
    // どちらもNULを含む全バイトを厳密に検証する。
    let bytes = if consume_bom {
        bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)
    } else {
        bytes
    };
    std::str::from_utf8(bytes)
}

#[derive(Deserialize)]
struct Merge {
    entries: Vec<[String; 2]>,
    existing: String,
}
fn newline(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}
fn canonical(s: &str) -> String {
    s.nfd().collect()
}
fn merge(request: Merge) -> (String, usize) {
    // Swift split(separator: Character("\n")) does not split a CRLF grapheme.
    let text = request.existing;
    let mut seen = std::collections::HashSet::new();
    let mut begin = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' && (index == 0 || text.as_bytes()[index - 1] != b'\r') {
            if index > begin {
                seen.insert(canonical(text[begin..index].trim_matches(newline)));
            }
            begin = index + 1;
        }
    }
    if begin < text.len() {
        seen.insert(canonical(text[begin..].trim_matches(newline)));
    }
    let mut added = Vec::new();
    for [reading, word] in request.entries {
        let line = format!("{reading}\t{word}");
        if seen.insert(canonical(&line)) {
            added.push(line);
        }
    }
    if added.is_empty() {
        return (text, 0);
    }
    let separator = if text.is_empty() || text.ends_with('\n') && !text.ends_with("\r\n") {
        ""
    } else {
        "\n"
    };
    let count = added.len();
    (format!("{text}{separator}{}\n", added.join("\n")), count)
}
/// Merge request JSON {entries:[[reading,word]],existing:string}. malloc JSON
/// {text,count}, or null for malformed input. Free with meltype_free.
/// # Safety
/// input is readable for length UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_dictionary_merge(
    input: *const u8,
    length: u32,
) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(request) =
        serde_json::from_slice::<Merge>(std::slice::from_raw_parts(input, length as usize))
    else {
        return std::ptr::null_mut();
    };
    let (text, count) = merge(request);
    crate::allocate(&serde_json::json!({"text":text,"count":count}).to_string())
}
/// Parse XML/binary plist and normalize/filter original supported rows. malloc
/// JSON [[reading,word]] (possibly empty); errors return null and +1 NSError.
/// # Safety
/// bytes are readable for length, and error is null or a writable object slot.
/// Caller consumes error's +1 reference on failure and frees JSON with meltype_free.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_dictionary_entries(
    bytes: *const u8,
    length: u32,
    error: *mut *mut c_void,
) -> *mut c_char {
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    if bytes.is_null() && length != 0 {
        return std::ptr::null_mut();
    }
    #[cfg(target_os = "macos")]
    {
        native::entries(bytes, length, error)
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::ptr::null_mut()
    }
}
/// Import a source NSURL into directory/userdict.txt through the original
/// Foundation read/create/atomic-write APIs. >=0 added count, -1 SDK error,
/// -2 unsupported entries/invalid ABI. Error is a +1 NSError for the caller.
/// # Safety
/// source is a live NSURL; directory is readable UTF-16 for directory_length;
/// error is null or a writable object slot. Caller consumes error's +1 reference.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_dictionary_import(
    source: *mut c_void,
    directory: *const u16,
    directory_length: u32,
    error: *mut *mut c_void,
) -> i64 {
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    if source.is_null() || directory.is_null() && directory_length != 0 {
        return -2;
    }
    #[cfg(target_os = "macos")]
    {
        native::import(source, directory, directory_length, error)
    }
    #[cfg(not(target_os = "macos"))]
    {
        -2
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{
        msg_send,
        rc::Retained,
        runtime::{AnyClass, AnyObject},
    };
    use objc2_foundation::{NSArray, NSProcessInfo, NSString};
    unsafe fn class(name: &std::ffi::CStr) -> &'static AnyClass {
        AnyClass::get(name).expect("Foundation class unavailable")
    }
    unsafe fn is(object: &AnyObject, name: &std::ffi::CStr) -> bool {
        msg_send![object,isKindOfClass:class(name)]
    }
    unsafe fn field(object: &AnyObject, key: &str) -> Option<Retained<AnyObject>> {
        let key = NSString::from_str(key);
        msg_send![object,objectForKey:&*key]
    }
    unsafe fn string(object: &AnyObject) -> Option<Retained<NSString>> {
        if !is(object, c"NSString") {
            return None;
        }
        Retained::retain((object as *const AnyObject).cast_mut().cast::<NSString>())
    }
    // Swift Character comparison permits CRLF as one grapheme; isolated CR/LF
    // and tab are rejected, preserving the original acceptance boundary.
    fn has_separator(s: &str) -> bool {
        let b = s.as_bytes();
        b.iter().enumerate().any(|(i, c)| {
            *c == b'\t'
                || *c == b'\n' && (i == 0 || b[i - 1] != b'\r')
                || *c == b'\r' && (i + 1 == b.len() || b[i + 1] != b'\n')
        })
    }
    #[link(name = "Foundation", kind = "framework")]
    extern "C" {
        static NSStringTransformHiraganaToKatakana: *const NSString;
    }
    pub unsafe fn entries(bytes: *const u8, length: u32, error: *mut *mut c_void) -> *mut c_char {
        let data: Retained<AnyObject> =
            msg_send![class(c"NSData"),dataWithBytes:bytes.cast::<c_void>(),length:length as usize];
        match parse(&data, error) {
            Some(result) => crate::allocate(&serde_json::to_string(&result).unwrap()),
            None => std::ptr::null_mut(),
        }
    }
    unsafe fn capture(failure: *mut AnyObject, error: *mut *mut c_void) {
        if !error.is_null() {
            if let Some(failure) = Retained::retain(failure) {
                *error = Retained::into_raw(failure).cast();
            }
        }
    }
    unsafe fn parse(data: &AnyObject, error: *mut *mut c_void) -> Option<Vec<[String; 2]>> {
        let mut failure: *mut AnyObject = std::ptr::null_mut();
        let root: Option<Retained<AnyObject>> = msg_send![class(c"NSPropertyListSerialization"),propertyListWithData:data,options:0usize,format:std::ptr::null_mut::<usize>(),error:&mut failure];
        let Some(root) = root else {
            capture(failure, error);
            return None;
        };
        let rows = if is(&root, c"NSArray") {
            Some(root)
        } else if is(&root, c"NSDictionary") {
            field(&root, "UserDictionary")
        } else {
            None
        };
        let mut result = Vec::<[String; 2]>::new();
        if let Some(rows) = rows.filter(|r| is(r, c"NSArray")) {
            let rows = Retained::retain(
                Retained::as_ptr(&rows)
                    .cast_mut()
                    .cast::<NSArray<AnyObject>>(),
            )
            .unwrap();
            if rows.iter().all(|row| is(&row, c"NSDictionary")) {
                let trim: Retained<AnyObject> =
                    msg_send![class(c"NSCharacterSet"), whitespaceAndNewlineCharacterSet];
                for row in rows.iter() {
                    let reading = field(&row, "reading")
                        .or_else(|| field(&row, "shortcut"))
                        .and_then(|v| string(&v));
                    let word = field(&row, "phrase")
                        .or_else(|| field(&row, "word"))
                        .and_then(|v| string(&v));
                    let (Some(reading), Some(word)) = (reading, word) else {
                        continue;
                    };
                    let transformed: Option<Retained<NSString>> = msg_send![&reading,stringByApplyingTransform:NSStringTransformHiraganaToKatakana,reverse:true];
                    let reading = transformed.unwrap_or(reading);
                    let reading: Retained<NSString> =
                        msg_send![&reading,stringByTrimmingCharactersInSet:&*trim];
                    let word: Retained<NSString> =
                        msg_send![&word,stringByTrimmingCharactersInSet:&*trim];
                    let reading = reading.to_string();
                    let word = word.to_string();
                    if reading.encode_utf16().count() >= 2
                        && !word.is_empty()
                        && !has_separator(&reading)
                        && !has_separator(&word)
                    {
                        result.push([reading, word]);
                    }
                }
            }
        }
        Some(result)
    }
    pub unsafe fn import(
        source: *mut c_void,
        directory: *const u16,
        length: u32,
        error: *mut *mut c_void,
    ) -> i64 {
        let Some(source) = Retained::retain(source.cast::<AnyObject>()) else {
            return -2;
        };
        if !is(&source, c"NSURL") {
            return -2;
        }
        let units = if length == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(directory, length as usize)
        };
        let Ok(directory) = String::from_utf16(units) else {
            return -2;
        };
        let directory = NSString::from_str(&directory);
        let mut failure: *mut AnyObject = std::ptr::null_mut();
        let data: Option<Retained<AnyObject>> = msg_send![class(c"NSData"),dataWithContentsOfURL:&*source,options:0usize,error:&mut failure];
        let Some(data) = data else {
            capture(failure, error);
            return -1;
        };
        let Some(entries) = parse(&data, error) else {
            return -1;
        };
        if entries.is_empty() {
            return -2;
        }
        let folder: Retained<AnyObject> =
            msg_send![class(c"NSURL"),fileURLWithPath:&*directory,isDirectory:true];
        let name = NSString::from_str("userdict.txt");
        let destination: Retained<AnyObject> =
            msg_send![&folder,URLByAppendingPathComponent:&*name];
        let manager: Retained<AnyObject> = msg_send![class(c"NSFileManager"), defaultManager];
        let path: Retained<NSString> = msg_send![&destination, path];
        let exists: bool = msg_send![&manager,fileExistsAtPath:&*path];
        let existing = if exists {
            failure = std::ptr::null_mut();
            let bytes: Option<Retained<AnyObject>> = msg_send![class(c"NSData"),dataWithContentsOfURL:&*destination,options:0usize,error:&mut failure];
            let Some(bytes) = bytes else {
                capture(failure, error);
                return -1;
            };
            let byte_count: usize = msg_send![&bytes, length];
            let pointer: *const u8 = msg_send![&bytes, bytes];
            let buffer = if byte_count == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(pointer, byte_count)
            };
            // 実行中のFoundationに合わせる。型付きAPIでOSの版を取得し、
            // UTF-8として不正な場合のcorrupt-file 259は以前と同じにする。
            let consume_bom = NSProcessInfo::processInfo()
                .operatingSystemVersion()
                .majorVersion
                >= 26;
            let Ok(text) = decode_existing_utf8(buffer, consume_bom) else {
                let domain = NSString::from_str("NSCocoaErrorDomain");
                let failure: Retained<AnyObject> = msg_send![class(c"NSError"),errorWithDomain:&*domain,code:259isize,userInfo:std::ptr::null::<AnyObject>()];
                capture(Retained::as_ptr(&failure).cast_mut(), error);
                return -1;
            };
            text.to_owned()
        } else {
            String::new()
        };
        let (merged, count) = merge(Merge { entries, existing });
        if count > 0 {
            failure = std::ptr::null_mut();
            let created: bool = msg_send![&manager,createDirectoryAtURL:&*folder,withIntermediateDirectories:true,attributes:std::ptr::null::<AnyObject>(),error:&mut failure];
            if !created {
                capture(failure, error);
                return -1;
            }
            // Preserve the UTF-8 bytes, including a leading BOM. NSString's
            // initWithBytes UTF-8 conversion can consume that marker.
            let data: Retained<AnyObject> = msg_send![class(c"NSData"),dataWithBytes:merged.as_ptr().cast::<c_void>(),length:merged.len()];
            failure = std::ptr::null_mut();
            let written: bool =
                msg_send![&data,writeToURL:&*destination,options:1usize,error:&mut failure];
            if !written {
                capture(failure, error);
                return -1;
            }
        }
        count as i64
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_existing_utf8, merge, Merge};

    #[test]
    fn existing_utf8_without_bom_and_nuls_are_preserved_in_both_modes() {
        for text in ["", "かな\t単語\n", "a\0b\0", "a\u{feff}b"] {
            for consume_bom in [false, true] {
                assert_eq!(decode_existing_utf8(text.as_bytes(), consume_bom), Ok(text));
            }
        }
    }

    #[test]
    fn existing_utf8_consumes_exactly_one_leading_bom_in_new_mode() {
        let line = "かな\t単語\n";
        let one = format!("\u{feff}{line}");
        let two = format!("\u{feff}{one}");
        assert_eq!(
            decode_existing_utf8(one.as_bytes(), false),
            Ok(one.as_str())
        );
        assert_eq!(decode_existing_utf8(one.as_bytes(), true), Ok(line));
        assert_eq!(
            decode_existing_utf8(two.as_bytes(), false),
            Ok(two.as_str())
        );
        assert_eq!(decode_existing_utf8(two.as_bytes(), true), Ok(one.as_str()));
    }

    #[test]
    fn existing_utf8_rejects_invalid_bytes_even_after_nul_or_bom() {
        let cases: [&[u8]; 5] = [
            &[0xc0, 0xaf],
            &[0xe3, 0x81],
            &[0xff, 0xfe, 0x41, 0],
            b"valid\0\xff",
            b"\xef\xbb\xbfvalid\0\xc0\xaf",
        ];
        for bytes in cases {
            for consume_bom in [false, true] {
                assert!(decode_existing_utf8(bytes, consume_bom).is_err());
            }
        }
    }

    #[test]
    fn existing_utf8_bom_policy_preserves_dictionary_duplicate_boundary() {
        let line = "かな\t単語\n";
        let original = format!("\u{feff}{line}");
        for (consume_bom, expected_count) in [(false, 1), (true, 0)] {
            let existing = decode_existing_utf8(original.as_bytes(), consume_bom)
                .unwrap()
                .to_owned();
            let (merged, count) = merge(Merge {
                entries: vec![["かな".into(), "単語".into()]],
                existing,
            });
            assert_eq!(count, expected_count);
            assert_eq!(
                merged,
                if consume_bom {
                    line.to_owned()
                } else {
                    format!("{original}{line}")
                }
            );
        }
    }
}
