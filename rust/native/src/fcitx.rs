// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! fcitx5 key policy, preserving the former adapter's mapping independently of
//! the SDK. The framework still supplies keySymToUnicode at the ABI boundary.
use super::*;
#[derive(Default)]
struct Frontend {
    direct: bool,
    preedit: bool,
    predicting: bool,
}
fn frontends() -> &'static Mutex<HashMap<usize, Frontend>> {
    static STATES: OnceLock<Mutex<HashMap<usize, Frontend>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn attach(handle: *mut c_void) -> *mut c_void {
    if !handle.is_null() {
        frontends()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(handle as usize, Frontend::default());
    }
    handle
}
pub(crate) fn forget(handle: *mut c_void) {
    frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&(handle as usize));
}
#[no_mangle]
pub extern "C" fn meltype_fcitx_create() -> *mut c_void {
    attach(meltype_create())
}
/// # Safety
/// directory points to a valid NUL-terminated UTF-8 profile path.
#[no_mangle]
pub unsafe extern "C" fn meltype_fcitx_create_at(directory: *const c_char) -> *mut c_void {
    attach(meltype_create_at(directory))
}
#[no_mangle]
pub extern "C" fn meltype_fcitx_destroy(handle: *mut c_void) {
    meltype_destroy(handle);
}
/// Bit 0: direct mode; bit 1: visible preedit; -1: missing/destroyed session.
#[no_mangle]
pub extern "C" fn meltype_fcitx_state(handle: *mut c_void) -> i32 {
    frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&(handle as usize))
        .map_or(-1, |state| {
            i32::from(state.direct) | (i32::from(state.preedit) << 1)
        })
}
#[no_mangle]
pub extern "C" fn meltype_fcitx_set_preedit(handle: *mut c_void, visible: i32) {
    if let Some(state) = frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&(handle as usize))
    {
        state.preedit = visible != 0;
    }
}
pub(crate) fn set_predicting(handle: *mut c_void, predicting: bool) {
    if let Some(state) = frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&(handle as usize))
    {
        state.predicting = predicting;
    }
}
pub(crate) fn predicting(handle: *mut c_void) -> bool {
    frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&(handle as usize))
        .is_some_and(|state| state.predicting)
}
#[no_mangle]
pub extern "C" fn meltype_fcitx_set_direct(handle: *mut c_void, direct: i32) {
    // Same lock order as destruction: frontend -> core. Neither operation calls
    // into a host callback while holding these locks.
    if let Some(state) = frontends()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&(handle as usize))
    {
        meltype_set_direct(handle, direct);
        state.direct = direct != 0;
    }
}
fn translate(symbol: u32, scalar: u32) -> (u32, u32) {
    let special = match symbol {
        0xff0d | 0xff8d => 13,
        0xff09 | 0xfe20 => 9,
        0x20 => 32,
        0xff08 => 8,
        0xffff => 0x2e,
        0xff1b => 27,
        0xff51 => 0x25,
        0xff52 => 0x26,
        0xff53 => 0x27,
        0xff54 => 0x28,
        0xff50 => 0x24,
        0xff57 => 0x23,
        0xff55 => 0x21,
        0xff56 => 0x22,
        0xffc3..=0xffc7 => 0x75 + symbol - 0xffc3,
        _ => 0,
    };
    if special != 0 {
        return (special, if special == 32 { 32 } else { 0 });
    }
    let Some(character) = char::from_u32(scalar).filter(|c| *c >= ' ') else {
        return (0, 0);
    };
    let key = match character {
        'a'..='z' => scalar - 32,
        'A'..='Z' | '0'..='9' => scalar,
        ',' => 0xbc,
        '.' => 0xbe,
        '-' => 0xbd,
        '/' => 0xbf,
        '[' => 0xdb,
        ']' => 0xdd,
        _ => 7,
    };
    (key, scalar)
}
pub(crate) fn sync_input_mode(handle: *mut c_void) -> Option<bool> {
    let direct = match meltype_input_mode(handle) {
        0 => false,
        1 => true,
        _ => return None,
    };
    let mut all = frontends().lock().ok()?;
    let state = all.get_mut(&(handle as usize))?;
    state.direct = direct;
    Some(direct)
}
/// Low 32 bits: virtual key; high 32 bits: Unicode scalar. Zero means unhandled.
#[no_mangle]
pub extern "C" fn meltype_fcitx_key(symbol: u32, scalar: u32) -> u64 {
    let (key, scalar) = translate(symbol, scalar);
    u64::from(key) | (u64::from(scalar) << 32)
}
/// fcitx KeyState bits: Shift=1, Ctrl=4, Alt=8, Super=64. Preserve the old
/// adapter's Super-only policy (Super2 and lock/repeat/internal bits are ignored).
#[no_mangle]
pub extern "C" fn meltype_fcitx_modifiers(state: u32) -> i32 {
    i32::from(state & 1 != 0)
        | (i32::from(state & 4 != 0) << 1)
        | (i32::from(state & 8 != 0) << 2)
        | (i32::from(state & 64 != 0) << 3)
}
/// Mode-key plan: bit 0 means mode switch, bit 1 means direct/English target,
/// bits 8..15 contain the conversion role key (zero when already in direct mode).
/// The caller preserves commit -> editor application -> mode update ordering.
#[no_mangle]
pub extern "C" fn meltype_fcitx_mode(symbol: u32, direct: i32) -> u32 {
    let direct = direct != 0;
    let toggle = matches!(symbol, 0xff28..=0xff2a);
    let english = matches!(symbol, 0xff22 | 0xff30);
    let role = if direct {
        0
    } else {
        match symbol {
            0xff23 => 0x1c,
            0xff22 => 0x1d,
            0xff27 => 0x15,
            _ => 0,
        }
    };
    let switch = toggle || english || matches!(symbol, 0xff23 | 0xff27);
    if !switch {
        return 0;
    }
    1 | (u32::from(english || toggle && !direct) << 1) | (role << 8)
}
fn offsets(text: &str, cursor: u32, limit: u32) -> Option<[u32; 3]> {
    if text.is_empty() {
        return None;
    }
    let mut result = [0; 3];
    let mut found = false;
    let start = cursor.saturating_sub(limit) as usize;
    let end = cursor.saturating_add(20) as usize;
    for (index, byte) in text
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .enumerate()
    {
        if index == start {
            result[0] = byte as u32;
        }
        if index == cursor as usize {
            result[1] = byte as u32;
            found = true;
        }
        result[2] = byte as u32;
        if index == end {
            break;
        }
    }
    found.then_some(result)
}
fn deletion(text: &str, cursor: u32, expected: &str, remove: i32) -> i32 {
    if expected.is_empty() || remove <= 0 || expected.encode_utf16().count() != remove as usize {
        return 0;
    }
    let Some([_, at, _]) = offsets(text, cursor, 0) else {
        return 0;
    };
    if !text[..at as usize].ends_with(expected) {
        return 0;
    }
    i32::try_from(expected.chars().count()).unwrap_or(0)
}
/// # Safety
/// text is readable for length bytes and output is writable for three u32s.
/// On success write before-start/cursor/after-end byte offsets; after is capped
/// at 20 characters. Empty/invalid UTF-8 or cursor returns 0 without writes.
#[no_mangle]
pub unsafe extern "C" fn meltype_fcitx_context_offsets(
    text: *const u8,
    length: u32,
    cursor: u32,
    limit: u32,
    output: *mut u32,
) -> i32 {
    if text.is_null() || output.is_null() {
        return 0;
    }
    let Ok(text) = std::str::from_utf8(std::slice::from_raw_parts(text, length as usize)) else {
        return 0;
    };
    let Some(offsets) = offsets(text, cursor, limit) else {
        return 0;
    };
    std::ptr::copy_nonoverlapping(offsets.as_ptr(), output, 3);
    1
}
/// # Safety
/// Both byte buffers are readable for their stated lengths during this call.
/// Return a character count only for a valid expected suffix whose UTF-16 length
/// exactly matches remove. Invalid text/cursor/mismatch returns 0.
#[no_mangle]
pub unsafe extern "C" fn meltype_fcitx_delete_count(
    text: *const u8,
    length: u32,
    cursor: u32,
    expected: *const u8,
    expected_length: u32,
    remove: i32,
) -> i32 {
    if text.is_null() || expected.is_null() {
        return 0;
    }
    let Ok(text) = std::str::from_utf8(std::slice::from_raw_parts(text, length as usize)) else {
        return 0;
    };
    let Ok(expected) = std::str::from_utf8(std::slice::from_raw_parts(
        expected,
        expected_length as usize,
    )) else {
        return 0;
    };
    deletion(text, cursor, expected, remove)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_preserve_special_scalar_and_punctuation_contracts() {
        for (symbol, key) in [
            (0xff0d, 13),
            (0xff8d, 13),
            (0xff09, 9),
            (0x20, 32),
            (0xff08, 8),
            (0xffff, 46),
            (0xff1b, 27),
            (0xff51, 37),
            (0xff52, 38),
            (0xff53, 39),
            (0xff54, 40),
            (0xff50, 36),
            (0xff57, 35),
            (0xff55, 33),
            (0xff56, 34),
            (0xffc3, 117),
            (0xffc7, 121),
        ] {
            assert_eq!(
                translate(symbol, 'x' as u32),
                (key, if key == 32 { 32 } else { 0 })
            );
        }
        for character in [
            'a', 'Z', '0', ',', '.', '-', '/', '[', ']', '!', 'İ', 'は', '😀', '\u{7f}',
        ] {
            let packed = meltype_fcitx_key(0, character as u32);
            assert_ne!(packed as u32, 0);
            assert_eq!((packed >> 32) as u32, character as u32);
        }
        assert_eq!(translate(0, 'a' as u32), (65, 97));
        assert_eq!(translate(0, 'İ' as u32), (7, 'İ' as u32));
        for invalid in [0, 31, 0xd800, 0x110000] {
            assert_eq!(meltype_fcitx_key(0, invalid), 0);
        }
        assert_eq!(meltype_fcitx_key(0xfe20, 0), 9); // ISO_Left_Tab must reach Core as Shift+Tab.
    }
    #[test]
    fn modifier_bits_preserve_all_combinations_without_locks_or_super2() {
        for mask in 0..16u32 {
            let source = (mask & 1) | ((mask & 2) << 1) | ((mask & 4) << 1) | ((mask & 8) << 3);
            assert_eq!(meltype_fcitx_modifiers(source), mask as i32);
            assert_eq!(
                meltype_fcitx_modifiers(source | 2 | 16 | (1 << 26) | (1 << 31)),
                mask as i32
            );
        }
    }
    #[test]
    fn japanese_mode_and_role_plans_preserve_direct_and_commit_policy() {
        for direct in [0, 1] {
            for symbol in [0xff28, 0xff29, 0xff2a] {
                assert_eq!(
                    meltype_fcitx_mode(symbol, direct),
                    if direct == 0 { 3 } else { 1 }
                );
            }
            assert_eq!(meltype_fcitx_mode(0xff30, direct), 3);
            assert_eq!(
                meltype_fcitx_mode(0xff22, direct),
                3 | if direct == 0 { 0x1d << 8 } else { 0 }
            );
            assert_eq!(
                meltype_fcitx_mode(0xff23, direct),
                1 | if direct == 0 { 0x1c << 8 } else { 0 }
            );
            assert_eq!(
                meltype_fcitx_mode(0xff27, direct),
                1 | if direct == 0 { 0x15 << 8 } else { 0 }
            );
            for ordinary in [0, 0xff21, 0xff24, 0xff25, 0xff26, 0xff2f, 0x61, 0x100304b] {
                assert_eq!(meltype_fcitx_mode(ordinary, direct), 0);
            }
        }
    }
    #[test]
    fn frontend_state_is_scoped_and_destroyed_with_core_handles() {
        let directory = tempfile::tempdir().unwrap();
        let path = CString::new(directory.path().to_str().unwrap()).unwrap();
        let first = unsafe { meltype_fcitx_create_at(path.as_ptr()) };
        let second = unsafe { meltype_fcitx_create_at(path.as_ptr()) };
        assert!(!first.is_null() && !second.is_null());
        assert_eq!(meltype_fcitx_state(first), 0);
        assert_eq!(meltype_fcitx_state(second), 0);
        meltype_fcitx_set_direct(first, 1);
        meltype_fcitx_set_preedit(first, 1);
        assert_eq!(meltype_fcitx_state(first), 3);
        assert_eq!(meltype_fcitx_state(second), 0);
        let reply = unsafe {
            take(meltype_handle_key(
                first,
                65,
                97,
                0,
                std::ptr::null(),
                std::ptr::null(),
            ))
        }
        .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&reply).unwrap()["consumed"],
            false
        );
        meltype_fcitx_set_direct(first, 0);
        assert_eq!(meltype_fcitx_state(first), 2);
        meltype_fcitx_set_preedit(first, 0);
        assert_eq!(meltype_fcitx_state(first), 0);
        meltype_destroy(first);
        assert_eq!(meltype_fcitx_state(first), -1);
        meltype_fcitx_set_direct(first, 1);
        meltype_fcitx_set_preedit(first, 1);
        assert_eq!(meltype_fcitx_state(first), -1);
        assert_eq!(meltype_fcitx_state(second), 0);
        meltype_fcitx_destroy(second);
        meltype_fcitx_destroy(second);
        assert_eq!(meltype_fcitx_state(second), -1);
        assert_eq!(meltype_fcitx_state(std::ptr::null_mut()), -1);
    }
    #[test]
    fn surrounding_and_deletion_distinguish_utf8_characters_and_utf16_units() {
        let text = "先前😀後";
        assert_eq!(offsets(text, 3, 2), Some([3, 10, 13]));
        assert_eq!(deletion(text, 3, "前😀", 3), 2);
        assert_eq!(deletion(text, 3, "前😀", 2), 0);
        assert_eq!(deletion(text, 2, "前😀", 3), 0);
        assert_eq!(deletion(text, 99, "前😀", 3), 0);
        assert_eq!(deletion(text, 3, "", 0), 0);
        assert_eq!(offsets("", 0, 20), None);
        assert_eq!(offsets(text, 99, 20), None);
        let long = "あ".repeat(50);
        assert_eq!(offsets(&long, 25, 2), Some([69, 75, 135]));
        assert_eq!(offsets(&long, 0, 20), Some([0, 0, 60]));
        let mut output = [123; 3];
        assert_eq!(
            unsafe {
                meltype_fcitx_context_offsets(b"\xff".as_ptr(), 1, 0, 20, output.as_mut_ptr())
            },
            0
        );
        assert_eq!(output, [123; 3]);
    }
}
