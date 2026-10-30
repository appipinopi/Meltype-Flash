// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Service-owned key edits, replies, fallback writes and reinjection.
use super::*;
use crate::tsf::{key_edit, process};
use std::cell::{Cell, RefCell};
use windows::Win32::UI::Input::KeyboardAndMouse::INPUT;

type Apply = unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *const u8, u32) -> i32;
type Notice = unsafe extern "C" fn(*mut c_void, u32, i32);
type Sender = unsafe extern "C" fn(*mut c_void, *const INPUT, u32) -> u32;
struct Owner {
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    adapter: *mut c_void,
    apply: Option<Apply>,
    notice: Option<Notice>,
    sender: Option<Sender>,
    reply: RefCell<Option<Vec<u8>>>,
    flags: Cell<u32>,
    mode: Cell<u32>,
}
unsafe extern "C" fn log(data: *mut c_void, op: u32, value: i32) {
    let o = &*data.cast::<Owner>();
    if let Some(notice) = o.notice {
        notice(o.adapter, op, value);
    }
}
unsafe extern "C" fn scope(data: *mut c_void, op: u32, value: i32) {
    if matches!(op, 4 | 5) {
        log(data, op + 2, value);
    }
}
unsafe extern "C" fn mode_changed(data: *mut c_void, english: i32) {
    log(data, 5, english);
}
unsafe extern "C" fn read(data: *mut c_void) -> u32 {
    let s = (*data.cast::<Owner>()).state;
    u32::from(!(*s).composition.is_null())
        | (u32::from((*s).open) << 1)
        | (u32::from((*s).secret_checked) << 2)
        | (u32::from((*s).secret_composing) << 3)
        | (u32::from((*s).caret_moved) << 4)
        | (u32::from((*s).private_field) << 5)
        | (u32::from((*s).key_failed) << 6)
        | (u32::from((*s).consumed) << 7)
}
unsafe extern "C" fn update(data: *mut c_void, op: u32, value: u32) {
    let o = &*data.cast::<Owner>();
    let s = o.state;
    match op {
        0 => {
            (*s).key_failed = value & 1 != 0;
            (*s).consumed = value & 2 != 0;
        }
        1 => (*s).own_edit = value != 0,
        2 => (*s).caret_moved = false,
        3 => {
            (*s).server_active = value & 1 != 0;
            (*s).direct_detection = value & 2 != 0;
            (*s).direct_word = value & 4 != 0;
            (*s).full_width_space = value & 1 != 0 && value & 8 != 0;
        }
        4 => (*s).server_active = false,
        5 => (*s).last_hello = 0,
        6 => {
            meltype_tsf_service_write_open(s, i32::from(value != 0));
        }
        7 => {
            let _ = commit(o, 1, ptr::null_mut(), 0);
        }
        8 => {
            let mut error = 0;
            let result = key_state::meltype_tsf_reinject(
                (*s).keys,
                value,
                o.lifetime,
                o.adapter,
                o.sender,
                &mut error,
            );
            if result == 2 {
                log(data, 8, 2);
            } else if result != 1 {
                log(data, 9, error as i32);
            }
        }
        _ => {}
    }
}
unsafe fn commit(o: &Owner, mode: u32, context: *mut c_void, cookie: u32) -> HRESULT {
    match o.apply {
        Some(apply) => super::commit::from_key(
            o.state, o.lifetime, o.worker, mode, context, cookie, o.adapter, apply, o.notice,
        ),
        None => super::commit::native(
            o.state, o.lifetime, o.sink, o.worker, mode, context, cookie, 1, o.adapter, o.notice,
        ),
    }
}
unsafe extern "C" fn edit(data: *mut c_void, op: u32, cookie: u32, context: *mut c_void) -> i32 {
    let o = &*data.cast::<Owner>();
    let s = o.state;
    match op {
        0 => {
            return super::input::meltype_tsf_service_input_scope(
                s,
                o.lifetime,
                context,
                cookie,
                data,
                Some(scope),
            )
        }
        1 => {
            let _ = commit(o, 2, context, cookie);
        }
        2 => {
            meltype_tsf_service_hide_candidates(s);
            super::owned::meltype_tsf_service_end_composition(s, o.lifetime, cookie);
        }
        3 => {
            meltype_tsf_service_apply_mode(s, o.mode.get(), o.lifetime, data, Some(mode_changed));
        }
        4 => {
            // Copy out of the cell before calling the shell: reentry can publish
            // another reply while this application still borrows its bytes.
            let bytes = o.reply.borrow().clone();
            if let Some(bytes) = bytes {
                let applied = match o.apply {
                    Some(apply) => {
                        apply(
                            o.adapter,
                            cookie,
                            context,
                            bytes.as_ptr(),
                            bytes.len() as u32,
                        ) != 0
                    }
                    None => {
                        super::apply::meltype_tsf_service_apply_reply(
                            s,
                            o.lifetime,
                            o.sink,
                            o.worker,
                            context,
                            cookie,
                            bytes.as_ptr(),
                            bytes.len() as u32,
                            data,
                            Some(failed),
                        ) >= 0
                    }
                };
                if !applied {
                    (*s).key_failed = true;
                    (*s).consumed = false;
                }
            }
        }
        _ => {}
    }
    0
}
unsafe extern "C" fn parsed(data: *mut c_void, bytes: *const u8, length: u32) -> i32 {
    let bytes = std::slice::from_raw_parts(bytes, length as usize);
    let Some((flags, mode)) = crate::json_bridge::key_reply(bytes) else {
        return 0;
    };
    let o = &*data.cast::<Owner>();
    o.flags.set(flags);
    o.mode.set(mode);
    *o.reply.borrow_mut() = Some(bytes.to_vec());
    1
}
unsafe extern "C" fn failed(data: *mut c_void) {
    log(data, 4, 0);
}
unsafe extern "C" fn request(data: *mut c_void, text: *const c_char, flags: *mut u32) -> i32 {
    let o = &*data.cast::<Owner>();
    *o.reply.borrow_mut() = None;
    o.flags.set(0);
    o.mode.set(0);
    let bytes = CStr::from_ptr(text).to_bytes();
    let Ok(length) = u32::try_from(bytes.len()) else {
        return 0;
    };
    let result = super::exchange::meltype_tsf_service_exchange(
        o.state,
        o.worker,
        o.lifetime,
        bytes.as_ptr(),
        length,
        800,
        0,
        data,
        Some(parsed),
        Some(failed),
    );
    flags.write(o.flags.get());
    result
}
unsafe extern "C" fn write(
    data: *mut c_void,
    cookie: u32,
    context: *mut c_void,
    range: *mut c_void,
    ch: u16,
) -> i32 {
    let o = &*data.cast::<Owner>();
    let result = super::owned::meltype_tsf_service_write_commit(
        o.state, o.lifetime, o.sink, context, range, cookie, &ch, 1,
    );
    i32::from(result >= 0 && result & 1 != 0)
}
/// # Safety
/// Live state/root/context/composition sink on the TSF thread. Worker is trusted
/// absolute UTF-8. Root pins state/adapter/code until return; delayed SYNC edits
/// are invalidated. Apply borrows compatible reply bytes and never unwinds.
/// Sender optionally replaces SendInput (for private hosts); null sends real
/// input. Notice0..3=key logs,4=invalid reply,5=English display,6=scope source,
/// 7=unreadable scope,8=reinjection guard,9=SendInput error code.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_handle_key(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    context: *mut c_void,
    key: u32,
    ch: u16,
    adapter: *mut c_void,
    apply: Option<Apply>,
    notice: Option<Notice>,
    sender: Option<Sender>,
) -> HRESULT {
    if apply.is_none() {
        return E_INVALIDARG;
    }
    run(
        state, lifetime, sink, worker, context, key, ch, adapter, apply, notice, sender,
    )
}
// Native dispatch uses the same policy with service-owned reply application.
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn run(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    context: *mut c_void,
    key: u32,
    ch: u16,
    adapter: *mut c_void,
    apply: Option<Apply>,
    notice: Option<Notice>,
    sender: Option<Sender>,
) -> HRESULT {
    if state.is_null() || lifetime.is_null() || worker.is_null() {
        return E_INVALIDARG;
    }
    let sid = CStr::from_ptr(meltype_tsf_service_sid(state)).to_string_lossy();
    let process = String::from_utf16_lossy(&process::name());
    let metadata =
        CString::new(serde_json::json!({"sid":sid,"process":process}).to_string()).unwrap();
    let mut o = Owner {
        state,
        lifetime,
        sink,
        worker,
        adapter,
        apply,
        notice,
        sender,
        reply: RefCell::new(None),
        flags: Cell::new(0),
        mode: Cell::new(0),
    };
    let callbacks = key_edit::Callbacks {
        owner: ptr::addr_of_mut!(o).cast(),
        lifetime,
        read,
        update,
        edit,
        request,
        write,
        log,
    };
    key_edit::meltype_tsf_handle_key(
        context,
        (*state).client,
        (*state).keys,
        key,
        ch,
        metadata.as_ptr(),
        &callbacks,
    )
}
