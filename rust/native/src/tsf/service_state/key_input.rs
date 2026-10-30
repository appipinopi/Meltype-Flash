// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Service-owned key snapshots, privacy cache, acceptance and four event entries.
use super::*;
use crate::tsf::{edit_session, keys};
use std::{cell::Cell, rc::Rc};
use windows::{
    core::{w, HRESULT},
    Win32::{
        Foundation::{E_ABORT, S_OK},
        UI::TextServices::*,
    },
};
type Commit = unsafe extern "C" fn(*mut c_void, u32, i32) -> i32;
type Handle = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, u16);
type Notice = unsafe extern "C" fn(*mut c_void, u32, i32);
#[derive(Clone, Copy)]
struct Owner {
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    adapter: *mut c_void,
    commit: Option<Commit>,
    handle: Option<Handle>,
    notice: Option<Notice>,
}
unsafe extern "C" fn relay(data: *mut c_void, op: u32, value: i32) {
    let o = &*data.cast::<Owner>();
    if let Some(notice) = o.notice {
        notice(o.adapter, op, value);
    }
}
struct Read {
    owner: Owner,
    _pin: IUnknown,
    context: ITfContext,
    active: Rc<Cell<bool>>,
    result: Rc<Cell<Option<i32>>>,
}
unsafe extern "C" fn read_scope(data: *mut c_void, cookie: u32) -> HRESULT {
    let work = &*data.cast::<Read>();
    if !work.active.get() {
        return E_ABORT;
    }
    let mut owner = work.owner;
    let value = super::input::meltype_tsf_service_input_scope(
        owner.state,
        owner.lifetime,
        work.context.as_raw(),
        cookie,
        ptr::addr_of_mut!(owner).cast(),
        Some(relay),
    );
    work.result.set(Some(value));
    S_OK
}
unsafe extern "C" fn destroy_read(data: *mut c_void) {
    drop(Box::from_raw(data.cast::<Read>()));
}
unsafe extern "C" fn reader(data: *mut c_void, context: *mut c_void, private: *mut i32) -> i32 {
    let owner = *data.cast::<Owner>();
    let Some(pin) = IUnknown::from_raw_borrowed(&owner.lifetime).cloned() else {
        return 1;
    };
    let Some(context) = ITfContext::from_raw_borrowed(&context).cloned() else {
        return -1;
    };
    let active = Rc::new(Cell::new(true));
    let result = Rc::new(Cell::new(None));
    let raw = context.as_raw();
    let work = Box::new(Read {
        owner,
        _pin: pin,
        context,
        active: active.clone(),
        result: result.clone(),
    });
    let status = edit_session::meltype_tsf_request_edit_session(
        raw,
        (*owner.state).client,
        (TF_ES_SYNC | TF_ES_READ).0,
        Box::into_raw(work).cast(),
        Some(read_scope),
        destroy_read,
    );
    active.set(false);
    private.write(i32::from((*owner.state).private_field));
    if status.is_err() {
        -1
    } else {
        result.get().unwrap_or(-1)
    }
}
unsafe extern "C" fn secret(data: *mut c_void, key: u32, context: *mut c_void, test: i32) -> i32 {
    let owner = *data.cast::<Owner>();
    let Some(_pin) = IUnknown::from_raw_borrowed(&owner.lifetime).cloned() else {
        return 1;
    };
    let mut private = 0;
    let result = key_state::meltype_tsf_key_secret(
        (*owner.state).keys,
        context,
        key,
        test,
        data,
        owner.lifetime,
        Some(reader),
        &mut private,
    );
    (*owner.state).private_field = private != 0;
    result
}
unsafe extern "C" fn snapshot(data: *mut c_void, out: *mut keys::Snapshot) {
    let s = (*data.cast::<Owner>()).state;
    let mut stale = key_state::Stale::default();
    key_state::meltype_tsf_key_stale_read((*s).keys, &mut stale);
    out.write(keys::Snapshot {
        composition: (*s).composition,
        context: (*s).composition_context,
        disabled: i32::from((*s).disabled),
        open: i32::from((*s).open),
        detection: i32::from((*s).direct_detection),
        english: i32::from((*s).direct_word),
        stale_tries: stale.tries,
        stale_vk: stale.key,
        stale_time: stale.time,
        full_width_space: i32::from((*s).full_width_space),
    });
}
unsafe extern "C" fn update(data: *mut c_void, op: u32, a: u64, b: u64) {
    let s = (*data.cast::<Owner>()).state;
    match op {
        0 => {
            (*s).secret_checked = false;
            (*s).secret_composing = false;
        }
        1 => (*s).secret_composing = true,
        2 => (*s).direct_word = false,
        3 => key_state::meltype_tsf_key_stale_write((*s).keys, (a >> 32) as u32, a as u32, b),
        4 => (*s).secret_checked = a != 0,
        _ => {}
    }
}
unsafe extern "C" fn resume(data: *mut c_void, context: *mut c_void) {
    let o = *data.cast::<Owner>();
    let result = super::recovered::meltype_tsf_service_resume(
        o.state,
        o.lifetime,
        o.worker,
        o.sink,
        context,
        data,
        Some(parse_failure),
    );
    if result == 1 {
        relay(data, 7, 1);
    }
}
unsafe extern "C" fn parse_failure(data: *mut c_void) {
    relay(data, 1, 0);
}
unsafe extern "C" fn now(data: *mut c_void, keep: i32) -> i32 {
    let o = &*data.cast::<Owner>();
    o.commit.unwrap()(o.adapter, 0, keep)
}
unsafe extern "C" fn later(data: *mut c_void, keep: i32) {
    let o = &*data.cast::<Owner>();
    o.commit.unwrap()(o.adapter, 1, keep);
}
unsafe extern "C" fn abandon(data: *mut c_void) {
    let o = *data.cast::<Owner>();
    super::recovered::meltype_tsf_service_abandon(
        o.state,
        o.lifetime,
        o.worker,
        data,
        Some(parse_failure),
    );
}
unsafe extern "C" fn ready(data: *mut c_void) -> i32 {
    let o = &*data.cast::<Owner>();
    super::core::meltype_tsf_service_core(o.state, o.worker, o.lifetime, data, Some(relay), None, 0)
}
unsafe extern "C" fn pass(data: *mut c_void, reason: u32) {
    let s = (*data.cast::<Owner>()).state;
    let label = match reason {
        1 => w!("英数").as_ptr(),
        2 => w!("Ctrl・Alt・Windows キーを押している").as_ptr(),
        3 => w!("この入力欄では使わない").as_ptr(),
        4 => w!("英語の単語の続き").as_ptr(),
        5 => w!("meltype-flash.exe が使えない").as_ptr(),
        6 => w!("パスワードの入力欄").as_ptr(),
        7 => w!("メニューを開いている").as_ptr(),
        _ => ptr::null(),
    };
    let changed = (*s).pass_reason != label;
    (*s).pass_reason = label;
    if changed && !label.is_null() {
        relay(data, 6, reason as i32);
    }
}
unsafe extern "C" fn would(
    data: *mut c_void,
    context: *mut c_void,
    key: u32,
    message: u64,
    test: i32,
    out: *mut u16,
) -> i32 {
    let o = &*data.cast::<Owner>();
    let callbacks = keys::Callbacks {
        owner: data,
        lifetime: o.lifetime,
        read: Some(snapshot),
        update: Some(update),
        resume: Some(resume),
        commit_now: Some(now),
        commit_async: Some(later),
        abandon: Some(abandon),
        ready: Some(ready),
        secret: Some(secret),
        pass: Some(pass),
    };
    keys::meltype_tsf_would_eat(context, key, message, test, &callbacks, out)
}
unsafe extern "C" fn mark(data: *mut c_void, _key: u32, mark: i32) {
    let s = (*data.cast::<Owner>()).state;
    if mark == 1 {
        (*s).direct_word = false;
    }
    (*s).caret_moved = true;
}
unsafe extern "C" fn composing(data: *mut c_void) -> i32 {
    i32::from(!(*(*data.cast::<Owner>()).state).composition.is_null())
}
unsafe extern "C" fn toggle(data: *mut c_void, context: *mut c_void) {
    let o = &*data.cast::<Owner>();
    super::mode_action::meltype_tsf_service_mode_action(
        o.state,
        o.lifetime,
        o.worker,
        context,
        0,
        0,
        0,
        data,
        Some(relay),
    );
}
unsafe extern "C" fn handle(data: *mut c_void, context: *mut c_void, key: u32, ch: u16) {
    let o = &*data.cast::<Owner>();
    o.handle.unwrap()(o.adapter, context, key, ch);
}
/// # Safety
/// Live service/lifetime/context on TSF thread. Lifetime pins state/notice code.
/// Null context gives unknown metadata. Notice follows mode-action codes, with
/// op6=changed pass reason. Delayed reads are disabled and remain unconfirmed.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_key_secret(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    context: *mut c_void,
    key: u32,
    test: i32,
    adapter: *mut c_void,
    notice: Option<Notice>,
) -> i32 {
    if state.is_null() || lifetime.is_null() {
        return 1;
    }
    let mut owner = Owner {
        state,
        lifetime,
        sink: ptr::null_mut(),
        worker: ptr::null(),
        adapter,
        commit: None,
        handle: None,
        notice,
    };
    secret(ptr::addr_of_mut!(owner).cast(), key, context, test)
}
/// # Safety
/// Same thread/lifetime contract. Worker is trusted absolute UTF-8; commit mode
/// 0=sync,1=async with keep-caret. Callbacks never unwind. out is writable UTF-16.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_would_eat(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    context: *mut c_void,
    key: u32,
    message: u64,
    test: i32,
    adapter: *mut c_void,
    commit: Option<Commit>,
    notice: Option<Notice>,
    out: *mut u16,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    out.write(0);
    if state.is_null() || lifetime.is_null() || worker.is_null() || commit.is_none() {
        return 0;
    }
    let mut owner = Owner {
        state,
        lifetime,
        sink,
        worker,
        adapter,
        commit,
        handle: None,
        notice,
    };
    would(
        ptr::addr_of_mut!(owner).cast(),
        context,
        key,
        message,
        test,
        out,
    )
}
/// # Safety
/// Same service/worker/callback contract. kind0=test down,1=down,2=test up,3=up.
/// Handle runs on the TSF thread and never retains arguments or unwinds.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_key_event(
    state: *mut ServiceState,
    lifetime: *mut c_void,
    sink: *mut c_void,
    worker: *const c_char,
    kind: u32,
    context: *mut c_void,
    key: u32,
    message: u64,
    adapter: *mut c_void,
    commit: Option<Commit>,
    handler: Option<Handle>,
    notice: Option<Notice>,
) -> i32 {
    if state.is_null()
        || lifetime.is_null()
        || worker.is_null()
        || commit.is_none()
        || handler.is_none()
    {
        return 0;
    }
    let mut owner = Owner {
        state,
        lifetime,
        sink,
        worker,
        adapter,
        commit,
        handle: handler,
        notice,
    };
    let entry = key_state::Entry {
        owner: ptr::addr_of_mut!(owner).cast(),
        lifetime,
        would: Some(would),
        mark: Some(mark),
        composing: Some(composing),
        toggle: Some(toggle),
        handle: Some(handle),
    };
    key_state::meltype_tsf_key_event((*state).keys, kind, context, key, message, &entry)
}
