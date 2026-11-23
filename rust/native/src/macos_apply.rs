// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! macOS edit-response order with synchronous SDK queries and no session lock.
use serde::Deserialize;
use std::ffi::c_void;
#[derive(Deserialize)]
struct Edit {
    #[serde(rename = "deleteBefore")]
    remove: i64,
    text: String,
    expect: Option<String>,
}
#[derive(Deserialize)]
struct Response {
    commits: Vec<Edit>,
    view: Option<serde_json::Value>,
}
type Host = unsafe extern "C" fn(*mut c_void, u32, u64, u64, *const u16, u32, *mut u64) -> i32;
/// Host ops: marked range(0), selected range(1), substring(2), insert(3),
/// render original view(4: location 1=show,0=hide), clear marked text(5). Queries write two u64s;
/// substring writes UTF-16 pointer/length, borrowed until the next callback.
/// Inserts receive UTF-16 text and NSRange, including NSNotFound/0.
/// Return 1 for completion; 0 for malformed input, mismatch or host failure.
/// # Safety
/// JSON bytes and host/context are valid throughout this synchronous call.
/// Host callbacks must not throw; returned text buffers match their lengths.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_apply_utf8(
    input: *const u8,
    length: u32,
    context: *mut c_void,
    host: Option<Host>,
) -> i32 {
    let Some(host) = host else {
        return 0;
    };
    if input.is_null() {
        return 0;
    }
    let Ok(response) =
        serde_json::from_slice::<Response>(std::slice::from_raw_parts(input, length as usize))
    else {
        return 0;
    };
    // Allocate all edit text before the first host callback.
    let edits: Vec<_> = response
        .commits
        .iter()
        .map(|edit| {
            (
                edit,
                edit.text.encode_utf16().collect::<Vec<_>>(),
                edit.expect
                    .as_ref()
                    .map(|s| s.encode_utf16().collect::<Vec<_>>()),
            )
        })
        .collect();
    for (edit, text, expected) in edits {
        let mut range = [i64::MAX as u64, 0];
        if edit.remove > 0 {
            let mut marked = [0; 2];
            let mut selected = [i64::MAX as u64, 0];
            if host(context, 0, 0, 0, std::ptr::null(), 0, marked.as_mut_ptr()) <= 0 {
                return 0;
            }
            if (marked[0] == i64::MAX as u64 || marked[1] == 0)
                && host(context, 1, 0, 0, std::ptr::null(), 0, selected.as_mut_ptr()) <= 0
            {
                return 0;
            }
            let Some(expected) = expected else {
                return 0;
            };
            if super::macos_editor::meltype_macos_replacement_range(
                marked[0],
                marked[1],
                selected[0],
                edit.remove,
                expected.as_ptr(),
                expected.len() as u32,
                range.as_mut_ptr(),
            ) == 0
            {
                return 0;
            }
            let mut actual = [0; 2];
            if host(
                context,
                2,
                range[0],
                range[1],
                std::ptr::null(),
                0,
                actual.as_mut_ptr(),
            ) <= 0
                || actual[1] > u32::MAX as u64
            {
                return 0;
            }
            if super::macos_editor::meltype_macos_expected_equal(
                actual[0] as usize as *const u16,
                actual[1] as u32,
                expected.as_ptr(),
                expected.len() as u32,
            ) == 0
            {
                return 0;
            }
            // #137: 未確定範囲の外を置換する前に、検証済みの未確定表示を解除する。
            if marked[0] != i64::MAX as u64
                && marked[1] > 0
                && host(context, 5, 0, 0, std::ptr::null(), 0, std::ptr::null_mut()) <= 0
            {
                return 0;
            }
        }
        if text.len() > u32::MAX as usize
            || host(
                context,
                3,
                range[0],
                range[1],
                text.as_ptr(),
                text.len() as u32,
                std::ptr::null_mut(),
            ) <= 0
        {
            return 0;
        }
    }
    i32::from(
        host(
            context,
            4,
            u64::from(response.view.is_some()),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        ) > 0,
    )
}
