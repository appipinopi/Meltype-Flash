// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Native service readiness/hello adapters: no C++ state or JSON callbacks.
use super::*;
use crate::tsf::{core_link, process};
use windows::Win32::System::Threading::GetCurrentThreadId;
type Notice = unsafe extern "C" fn(*mut c_void, u32, i32);
type Clock = unsafe extern "C" fn(*mut c_void) -> u64;
struct Owner {
    state: *mut ServiceState,
    worker: *const c_char,
    lifetime: *mut c_void,
    context: *mut c_void,
    notice: Option<Notice>,
    clock: Option<Clock>,
}
unsafe fn owner<'a>(data: *mut c_void) -> &'a Owner {
    &*data.cast::<Owner>()
}
unsafe extern "C" fn read(data: *mut c_void, last: *mut u64) -> u32 {
    let state = owner(data).state;
    last.write((*state).last_hello);
    u32::from((*state).disabled)
        | (u32::from((*state).server_active) << 1)
        | (u32::from((*state).deactivated) << 2)
}
unsafe extern "C" fn update(data: *mut c_void, op: u32, value: u64) {
    let owner = owner(data);
    let state = owner.state;
    match op {
        0 => (*state).last_hello = value,
        1 => {
            (*state).server_active = value & 1 != 0;
            (*state).direct_detection = value & 2 != 0;
            (*state).full_width_space = value & 1 != 0 && value & 8 != 0;
            (*state).code_app = value & 4 != 0;
            (*state).direct_word = false;
        }
        2 => {
            if let Some(notice) = owner.notice {
                notice(owner.context, 0, value as i32);
            }
        }
        _ => {}
    }
}
unsafe extern "C" fn request(data: *mut c_void, text: *const c_char, flags: *mut u32) -> i32 {
    let owner = owner(data);
    // Hello always has object members. Match the original before-final-brace tid
    // insertion rather than changing transport/request policy or JSON formatting.
    let mut request = CStr::from_ptr(text).to_bytes().to_vec();
    if (*owner.state).own_process && request.last() == Some(&b'}') {
        let suffix = format!(",\"tid\":{}", GetCurrentThreadId());
        request.splice(request.len() - 1..request.len() - 1, suffix.bytes());
    }
    let Ok(request) = CString::new(request) else {
        return 0;
    };
    let response = crate::take(meltype_tsf_service_request(
        owner.state,
        owner.worker,
        request.as_ptr(),
        800,
        0,
    ));
    let Some(response) = response else {
        return 0;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) else {
        if let Some(notice) = owner.notice {
            notice(owner.context, 1, 0);
        }
        return 0;
    };
    flags.write(
        u32::from(value["active"].as_bool().unwrap_or(false))
            | (u32::from(value["directDetection"].as_bool().unwrap_or(false)) << 1)
            | (u32::from(value["code"].as_bool().unwrap_or(false)) << 2)
            | (u32::from(value["fullWidthSpace"].as_bool().unwrap_or(false)) << 3),
    );
    1
}
unsafe extern "C" fn clock(data: *mut c_void) -> u64 {
    let owner = owner(data);
    owner.clock.unwrap()(owner.context)
}
unsafe fn callbacks(data: *mut Owner) -> core_link::StateCallbacks {
    let owner = &*data;
    core_link::StateCallbacks {
        owner: data.cast(),
        lifetime: owner.lifetime,
        epoch: ptr::addr_of!((*owner.state).focus_epoch),
        read,
        update,
        hello,
        request,
        clock: owner
            .clock
            .map(|_| clock as unsafe extern "C" fn(*mut c_void) -> u64),
    }
}
unsafe fn run_hello(data: *mut Owner) -> i32 {
    let process = process::name();
    let callbacks = callbacks(data);
    core_link::meltype_tsf_core_hello(process.as_ptr(), process.len() as u32, &callbacks)
}
unsafe extern "C" fn hello(data: *mut c_void) {
    run_hello(data.cast());
}
/// Readiness (hello=0) or explicit hello (hello!=0) uses the native service frame,
/// process identity, owned transport, response parsing and state publication.
/// Optional notice op0 reports active; op1 reports invalid JSON. Optional clock
/// exists for controlled hosts; production uses Windows uptime.
/// # Safety
/// Live state and borrowed lifetime IUnknown on the TSF thread. Lifetime keeps
/// state/context/callbacks alive for the call. Worker is borrowed trusted absolute
/// UTF-8 executable path; callbacks never unwind or retain stack arguments.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_core(
    state: *mut ServiceState,
    worker: *const c_char,
    lifetime: *mut c_void,
    context: *mut c_void,
    notice: Option<Notice>,
    clock: Option<Clock>,
    explicit_hello: i32,
) -> i32 {
    if state.is_null() || worker.is_null() || lifetime.is_null() {
        return 0;
    }
    let mut owner = Owner {
        state,
        worker,
        lifetime,
        context,
        notice,
        clock,
    };
    if explicit_hello != 0 {
        run_hello(&mut owner)
    } else {
        core_link::meltype_tsf_core_ready(&callbacks(&mut owner))
    }
}
