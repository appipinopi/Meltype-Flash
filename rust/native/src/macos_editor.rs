// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! NSRange arithmetic and Swift-compatible canonical expected-text comparison.
use unicode_normalization::UnicodeNormalization;
const NOT_FOUND: u64 = i64::MAX as u64;
fn context(location: u64, length: u64, code: bool) -> Option<[u64; 4]> {
    let end = location.checked_add(length)?;
    if location >= NOT_FOUND || end >= NOT_FOUND {
        return None;
    }
    let start = location.saturating_sub(if code { 4000 } else { 20 });
    Some([start, location - start, end, 20])
}
fn replacement(
    marked: u64,
    marked_length: u64,
    selected: u64,
    remove: i64,
    expected: &[u16],
) -> Option<[u64; 2]> {
    if remove <= 0 {
        return Some([NOT_FOUND, 0]);
    }
    let caret = if marked != NOT_FOUND && marked_length > 0 {
        marked
    } else {
        selected
    };
    if caret >= NOT_FOUND
        || caret < remove as u64
        || expected.len() as u64 != remove as u64
        || char::decode_utf16(expected.iter().copied()).any(|v| v.is_err())
    {
        return None;
    }
    Some([caret - remove as u64, remove as u64])
}
unsafe fn utf16<'a>(text: *const u16, length: u32) -> Option<&'a [u16]> {
    if length == 0 {
        return Some(&[]);
    }
    if text.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts(text, length as usize))
}
/// Write before-location/length, after-location/length, in UTF-16 units.
/// # Safety
/// output is writable for four u64s. Invalid input leaves it untouched.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_context_range(
    location: u64,
    length: u64,
    code: i32,
    output: *mut u64,
) -> i32 {
    if output.is_null() {
        return 0;
    }
    let Some(value) = context(location, length, code != 0) else {
        return 0;
    };
    std::ptr::copy_nonoverlapping(value.as_ptr(), output, 4);
    1
}
/// # Safety
/// expected is readable for expected_length UTF-16 units; output is writable for
/// two u64s. Invalid ranges/length/surrogates leave output untouched.
#[no_mangle]
#[allow(clippy::too_many_arguments)] // Explicit scalar C ABI shared with Swift.
pub unsafe extern "C" fn meltype_macos_replacement_range(
    marked: u64,
    marked_length: u64,
    selected: u64,
    remove: i64,
    expected: *const u16,
    expected_length: u32,
    output: *mut u64,
) -> i32 {
    if output.is_null() {
        return 0;
    }
    let Some(expected) = utf16(expected, expected_length) else {
        return 0;
    };
    let Some(value) = replacement(marked, marked_length, selected, remove, expected) else {
        return 0;
    };
    std::ptr::copy_nonoverlapping(value.as_ptr(), output, 2);
    1
}
/// Swift String == uses canonical equivalence, unlike a byte comparison.
/// # Safety
/// Both buffers are readable for their stated UTF-16 lengths. Null is accepted
/// only for empty buffers; malformed UTF-16 is rejected rather than repaired.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_expected_equal(
    actual: *const u16,
    actual_length: u32,
    expected: *const u16,
    expected_length: u32,
) -> i32 {
    let (Some(actual), Some(expected)) = (
        utf16(actual, actual_length),
        utf16(expected, expected_length),
    ) else {
        return 0;
    };
    let (Ok(actual), Ok(expected)) = (String::from_utf16(actual), String::from_utf16(expected))
    else {
        return 0;
    };
    i32::from(actual.nfd().eq(expected.nfd()))
}
fn macos_session(handle: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    super::macos_meaning::attach(handle);
    if let Some(session) = super::sessions()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&(handle as usize))
    {
        session.external_converter_only = true;
        session.automatic_english_spacing = true;
    }
    handle
}
#[no_mangle]
pub extern "C" fn meltype_macos_create() -> *mut std::ffi::c_void {
    macos_session(super::meltype_create())
}
/// # Safety
/// directory is a valid NUL-terminated UTF-8 profile path.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_create_at(
    directory: *const std::ffi::c_char,
) -> *mut std::ffi::c_void {
    macos_session(super::meltype_create_at(directory))
}
/// Match upstream macOS selection: keep the selected candidate composing.
#[no_mangle]
pub extern "C" fn meltype_macos_select_candidate(
    handle: *mut std::ffi::c_void,
    index: i32,
) -> *mut std::ffi::c_char {
    let Ok(_spelling) = (unsafe {
        super::word_checker::prepare(handle as usize, 0, 0, 0, std::ptr::null(), std::ptr::null())
    }) else {
        return std::ptr::null_mut();
    };
    super::run(handle, |session| session.select_composing(index as usize))
}
/// Legacy macOS scalar boundary; other frontends retain full scalar input.
/// # Safety
/// before/after, if non-null, are valid NUL-terminated UTF-8 for this call.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_handle_key(
    handle: *mut std::ffi::c_void,
    vk: i32,
    ch: i32,
    modifiers: i32,
    before: *const std::ffi::c_char,
    after: *const std::ffi::c_char,
) -> *mut std::ffi::c_char {
    // パスワード欄などプライベートな入力は数えない。
    #[cfg(not(test))]
    {
        let private = super::sessions()
            .lock()
            .ok()
            .and_then(|all| all.get(&(handle as usize)).map(|s| s.private))
            .unwrap_or(true);
        if !private {
            super::count_key();
        }
    }
    let Some(scalar) = u32::try_from(ch).ok().and_then(char::from_u32) else {
        let Ok(_spelling) = super::word_checker::prepare(
            handle as usize,
            0,
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
        ) else {
            return std::ptr::null_mut();
        };
        return super::run(handle, |s| s.result(false));
    };
    let special = unicode_normalization::char::is_combining_mark(scalar) || ch > 0xffff;
    let _spelling = if special {
        match super::word_checker::prepare(handle as usize, vk, ch, modifiers, before, after) {
            Ok(scope) => scope,
            Err(()) => return std::ptr::null_mut(),
        }
    } else {
        None
    };
    if unicode_normalization::char::is_combining_mark(scalar) {
        return super::run(handle, |s| {
            s.commit_external((0x300..=0x36f).contains(&ch), true)
        });
    }
    if ch > 0xffff {
        return super::run(handle, |s| s.commit_external(false, false));
    }
    let was_converting = super::sessions()
        .lock()
        .ok()
        .and_then(|sessions| sessions.get(&(handle as usize)).map(|s| s.is_converting()))
        .unwrap_or(false);
    let result = super::meltype_handle_key(handle, vk, ch, modifiers, before, after);
    if vk != 9 || modifiers & 15 != 0 || was_converting || result.is_null() {
        return result;
    }
    // Decide after the core handles Tab: predictions, suggestions, snippets and
    // protected input must keep their existing roles and never invoke conversion.
    let reading = super::sessions().lock().ok().and_then(|sessions| {
        sessions
            .get(&(handle as usize))
            .filter(|s| s.is_converting())
            .map(|s| s.reading())
    });
    let Some(reading) = reading else {
        return result;
    };
    let (mut clauses, candidates) = super::external_conversion(&reading, before);
    if clauses.is_empty() && !candidates.is_empty() {
        clauses.push(meltype_core::session::Clause::new(reading, candidates));
    }
    if clauses.is_empty() {
        return result;
    }
    let Some(text) = super::take(result) else {
        return std::ptr::null_mut();
    };
    let Ok(previous) = serde_json::from_str::<meltype_core::SessionResult>(&text) else {
        return super::allocate(&text);
    };
    super::run(handle, |session| {
        if !session.begin_clauses(clauses) {
            return previous;
        }
        let mut updated = session.result(previous.consumed);
        updated.commits = previous.commits;
        updated
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nsrange_limits_and_utf16_lengths() {
        assert_eq!(context(25, 3, false), Some([5, 20, 28, 20]));
        assert_eq!(context(5000, 0, true), Some([1000, 4000, 5000, 20]));
        assert_eq!(context(NOT_FOUND, 0, false), None);
        assert_eq!(context(NOT_FOUND - 1, 2, false), None);
        assert_eq!(context(u64::MAX, 2, false), None);
        let expected: Vec<_> = "前😀".encode_utf16().collect();
        assert_eq!(replacement(10, 1, 99, 3, &expected), Some([7, 3]));
        assert_eq!(replacement(10, 0, 99, 3, &expected), Some([96, 3]));
        assert_eq!(replacement(NOT_FOUND, 1, 2, 3, &expected), None);
        assert_eq!(replacement(10, 1, 99, 2, &expected), None);
        assert_eq!(replacement(10, 1, 99, 1, &[0xd800]), None);
        let mut output = [123; 4];
        assert_eq!(
            unsafe { meltype_macos_context_range(NOT_FOUND, 0, 0, output.as_mut_ptr()) },
            0
        );
        assert_eq!(output, [123; 4]);
        let broken = [0xd800];
        assert_eq!(
            unsafe { meltype_macos_expected_equal(broken.as_ptr(), 1, broken.as_ptr(), 1) },
            0
        );
        assert_eq!(
            unsafe { meltype_macos_expected_equal(std::ptr::null(), 0, std::ptr::null(), 0) },
            1
        );
    }
}
