// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! TSF-thread timer for non-blocking conversion. The worker owns the
//! Session; this module only polls the `async_poll_apply` JSON operation and
//! applies the returned SessionResult through the existing edit-session bridge.
use super::*;
use crate::tsf::edit_session;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_char,
    ptr,
};
use windows::Win32::{
    Foundation::{HWND, S_OK},
    UI::{
        TextServices::{TF_ES_READWRITE, TF_ES_SYNC},
        WindowsAndMessaging::{KillTimer, SetTimer},
    },
};

const TIMER_MS: u32 = 10;

#[derive(Clone, Copy)]
struct Entry {
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    context: *mut c_void,
    adapter: *mut c_void,
}

struct Capture {
    active: *mut bool,
}

unsafe extern "C" fn capture_active(data: *mut c_void, bytes: *const u8, length: u32) -> i32 {
    let capture = &*data.cast::<Capture>();
    let text = std::slice::from_raw_parts(bytes, length as usize);
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(text) {
        if value["asyncActive"] == true {
            *capture.active = true;
        }
    }
    1
}

thread_local! {
    static TIMERS: RefCell<HashMap<usize, Entry>> = RefCell::new(HashMap::new());
}

/// Start an async conversion if the worker setting is enabled. Returns true
/// when the key was consumed and a poll timer was armed.
pub(super) unsafe fn begin(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    context: *mut c_void,
    adapter: *mut c_void,
    notice: Option<unsafe extern "C" fn(*mut c_void, u32, i32)>,
) -> bool {
    if state.is_null() || lifetime.is_null() || worker.is_null() || context.is_null() {
        return false;
    }
    let _ = notice;
    let sid = CStr::from_ptr(meltype_tsf_service_sid(state)).to_string_lossy();
    let request = serde_json::json!({"op":"async_begin_session","sid":sid}).to_string();
    let Ok(request) = CString::new(request) else {
        return false;
    };
    let mut active = false;
    let mut capture = Capture {
        active: &mut active,
    };
    let bytes = request.as_bytes();
    super::exchange::meltype_tsf_service_exchange(
        state,
        worker,
        lifetime,
        bytes.as_ptr(),
        bytes.len() as u32,
        200,
        0,
        ptr::addr_of_mut!(capture).cast(),
        Some(capture_active),
        None,
    );
    if !active {
        return false;
    }
    let id = state as usize;
    TIMERS.with(|timers| {
        timers.borrow_mut().insert(
            id,
            Entry {
                state,
                lifetime,
                sink,
                worker,
                context,
                adapter,
            },
        );
    });
    if SetTimer(HWND(std::ptr::null_mut()), id, TIMER_MS, Some(timer)) == 0 {
        cancel(state);
        return false;
    }
    true
}

/// Remove a pending timer. Safe to call repeatedly and from state teardown.
pub(super) unsafe fn cancel(state: *mut ServiceState) {
    let id = state as usize;
    let had = TIMERS.with(|timers| timers.borrow_mut().remove(&id).is_some());
    if had {
        let _ = KillTimer(HWND(std::ptr::null_mut()), id);
    }
}

unsafe extern "system" fn timer(_: HWND, _: u32, id: usize, _: u32) -> () {
    let _ = KillTimer(HWND(std::ptr::null_mut()), id);
    let entry = TIMERS.with(|timers| timers.borrow_mut().remove(&id));
    let Some(entry) = entry else {
        return;
    };
    let sid = CStr::from_ptr(meltype_tsf_service_sid(entry.state)).to_string_lossy();
    let request = serde_json::json!({"op":"async_poll_apply","sid":sid}).to_string();
    let Ok(request) = CString::new(request) else {
        return;
    };
    let mut reply = ReplyCapture::default();
    let bytes = request.as_bytes();
    let ok = super::exchange::meltype_tsf_service_exchange(
        entry.state,
        entry.worker,
        entry.lifetime,
        bytes.as_ptr(),
        bytes.len() as u32,
        300,
        0,
        (&mut reply as *mut ReplyCapture).cast(),
        Some(capture_reply),
        None,
    );
    let Some(reply) = reply.bytes.take() else {
        return;
    };
    if ok == 0 || reply.is_empty() {
        return;
    }
    let apply = Box::new(Apply {
        entry,
        reply,
    });
    let status = edit_session::meltype_tsf_request_edit_session(
        entry.context,
        (*entry.state).client,
        (TF_ES_SYNC | TF_ES_READWRITE).0,
        Box::into_raw(apply).cast(),
        Some(apply_reply),
        destroy_apply,
    );
    let _ = status;
}

#[derive(Default)]
struct ReplyCapture {
    bytes: Option<Vec<u8>>,
}

unsafe extern "C" fn capture_reply(data: *mut c_void, bytes: *const u8, length: u32) -> i32 {
    let capture = &mut *data.cast::<ReplyCapture>();
    capture.bytes = Some(std::slice::from_raw_parts(bytes, length as usize).to_vec());
    1
}

struct Apply {
    entry: Entry,
    reply: Vec<u8>,
}

unsafe extern "C" fn apply_reply(data: *mut c_void, cookie: u32) -> HRESULT {
    let apply = &*data.cast::<Apply>();
    let entry = apply.entry;
    super::apply::meltype_tsf_service_apply_reply(
        entry.state,
        entry.lifetime,
        entry.sink,
        entry.worker,
        entry.context,
        cookie,
        apply.reply.as_ptr(),
        apply.reply.len() as u32,
        entry.adapter,
        None,
    );
    S_OK
}

unsafe extern "C" fn destroy_apply(data: *mut c_void) {
    drop(Box::from_raw(data.cast::<Apply>()));
}
