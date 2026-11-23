// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Service input-scope state and stable source labels.
use super::*;
use windows::core::w;
type Notice = unsafe extern "C" fn(*mut c_void, u32, i32);
/// # Safety
/// Live state and borrowed lifetime/context on the TSF thread, valid cookie.
/// Lifetime pins state/context/notice code. Optional notice never unwinds;
/// op4=changed source (0..3), op5=unreadable reason. Invalid owner is secret.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_input_scope(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    context: *mut c_void,
    cookie: u32,
    adapter: *mut c_void,
    notice: Option<Notice>,
) -> i32 {
    if state.is_null() {
        return 1;
    }
    let Some(_pin) = IUnknown::from_raw_borrowed(&lifetime).cloned() else {
        return 1;
    };
    let value = fields::meltype_tsf_browser_input_scope(
        context,
        cookie,
        i32::from(crate::tsf::process::chromium_browser()),
    );
    (*state).private_field = value & 2 != 0;
    let source = (value >> 8) & 3;
    let label = match source {
        3 => w!("Windows とアプリ"),
        1 => w!("Windows"),
        2 => w!("アプリ"),
        _ => w!("無い"),
    }
    .as_ptr();
    if (*state).scope_source != label {
        (*state).scope_source = label;
        if let Some(notice) = notice {
            notice(adapter, 4, source as i32);
        }
    }
    if value & 4 != 0 {
        if let Some(notice) = notice {
            notice(adapter, 5, (value >> 16) as i32);
        }
    }
    i32::from(value & (1 | 8) != 0)
}
