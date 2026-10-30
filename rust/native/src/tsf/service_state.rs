// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! Rust-owned TSF state storage. Temporary C++ aliases use a checked C layout.
//! Raw COM/subscription slots retain their TSF lifecycle ownership policy.
//! Key/resume/abandoned state, SID, transport and UI tokens are freed by Drop.
mod activate;
mod apply;
mod commit;
mod core;
mod deactivate;
mod dispatch;
mod edited;
mod exchange;
mod focus;
mod input;
mod key_input;
mod key_work;
mod mode_action;
mod notifications;
pub(super) mod object;
mod opened;
mod owned;
mod recovered;
mod route;
use super::{abandoned, candidates, fields, key_state, langbar, mode, recovery, subscriptions};
use std::{
    alloc::{alloc, Layout},
    ffi::{c_char, c_void, CStr, CString},
    mem::{align_of, offset_of, size_of},
    panic::catch_unwind,
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};
use windows::{
    core::{IUnknown, Interface, HRESULT},
    Win32::{
        Foundation::{E_INVALIDARG, S_OK},
        UI::TextServices::{ITfLangBarItem, ITfLangBarItemMgr},
    },
};
#[repr(C)]
pub struct ServiceState {
    refs: i32,
    manager: *mut c_void,
    client: u32,
    thread_subscription: *mut c_void,
    key_subscription: *mut c_void,
    mode_subscription: *mut c_void,
    edit_subscription: *mut c_void,
    edit_context: *mut c_void,
    composition: *mut c_void,
    composition_context: *mut c_void,
    atom_input: u32,
    atom_converted: u32,
    atom_target: u32,
    keys: *mut c_void,
    resume: *mut c_void,
    abandoned: *mut c_void,
    last_hello: u64,
    focus_epoch: u64,
    mode_epoch: u64,
    shutdown_phase: u32,
    server_active: bool,
    direct_detection: bool,
    direct_word: bool,
    open: bool,
    code_app: bool,
    code_english: bool,
    disabled: bool,
    own_process: bool,
    last_converting: bool,
    caret_moved: bool,
    own_edit: bool,
    secret_checked: bool,
    secret_composing: bool,
    private_field: bool,
    keep_caret: bool,
    deactivated: bool,
    key_failed: bool,
    consumed: bool,
    scope_source: *const u16,
    pass_reason: *const u16,
    sid: *mut c_char,
    transport: *mut c_void,
    langbar: *mut c_void,
    candidates: *mut c_void,
    edit_epoch: u64,
    full_width_space: bool,
}
impl Default for ServiceState {
    fn default() -> Self {
        Self {
            refs: 1,
            manager: ptr::null_mut(),
            client: 0,
            thread_subscription: ptr::null_mut(),
            key_subscription: ptr::null_mut(),
            mode_subscription: ptr::null_mut(),
            edit_subscription: ptr::null_mut(),
            edit_context: ptr::null_mut(),
            composition: ptr::null_mut(),
            composition_context: ptr::null_mut(),
            atom_input: 0,
            atom_converted: 0,
            atom_target: 0,
            keys: ptr::null_mut(),
            resume: ptr::null_mut(),
            abandoned: ptr::null_mut(),
            last_hello: 0,
            focus_epoch: 0,
            mode_epoch: 0,
            shutdown_phase: 0,
            server_active: false,
            direct_detection: false,
            direct_word: false,
            open: true,
            code_app: false,
            code_english: false,
            disabled: false,
            own_process: false,
            last_converting: false,
            caret_moved: false,
            own_edit: false,
            secret_checked: false,
            secret_composing: false,
            private_field: false,
            keep_caret: false,
            deactivated: false,
            key_failed: false,
            consumed: true,
            scope_source: ptr::null(),
            pass_reason: ptr::null(),
            sid: ptr::null_mut(),
            transport: ptr::null_mut(),
            langbar: ptr::null_mut(),
            candidates: ptr::null_mut(),
            edit_epoch: 0,
            full_width_space: false,
        }
    }
}
impl Drop for ServiceState {
    fn drop(&mut self) {
        unsafe {
            self.shutdown_phase = 2;
            self.deactivated = true;
            key_state::meltype_tsf_key_state_destroy(
                ptr::addr_of_mut!(self.keys).replace(ptr::null_mut()),
            );
            recovery::meltype_tsf_destroy_resume(
                ptr::addr_of_mut!(self.resume).replace(ptr::null_mut()),
            );
            abandoned::meltype_tsf_abandoned_destroy(
                ptr::addr_of_mut!(self.abandoned).replace(ptr::null_mut()),
            );
            candidates::meltype_tsf_candidate_destroy(
                ptr::addr_of_mut!(self.candidates).replace(ptr::null_mut()),
            );
            retire_bar(ptr::addr_of_mut!(self.langbar).replace(ptr::null_mut()));
            let sid = std::mem::replace(&mut self.sid, ptr::null_mut());
            if !sid.is_null() {
                drop(CString::from_raw(sid));
            }
            close_transport(self);
        }
    }
}
/// Current mode bits: open=1, code app=2, code English=4, server active=8.
/// # Safety
/// State is null or live on its TSF thread; no callbacks occur.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_mode(state: *const ServiceState) -> u32 {
    if state.is_null() {
        return 0;
    }
    u32::from((*state).open)
        | (u32::from((*state).code_app) << 1)
        | (u32::from((*state).code_english) << 2)
        | (u32::from((*state).server_active) << 3)
}
/// # Safety
/// State is null or live on its TSF thread at entry. Manager/client are captured
/// before compartment COM callbacks; no subsequent state access occurs.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_write_open(
    state: *mut ServiceState,
    open: i32,
) -> i32 {
    if state.is_null() {
        return 0;
    }
    let manager = (*state).manager;
    let client = (*state).client;
    fields::meltype_tsf_write_open(manager, client, open)
}
struct ModeOwner {
    state: *mut ServiceState,
    context: *mut c_void,
    notice: Option<unsafe extern "C" fn(*mut c_void, i32)>,
}
unsafe extern "C" fn mode_read(owner: *mut c_void) -> u32 {
    meltype_tsf_service_mode((*owner.cast::<ModeOwner>()).state)
}
unsafe extern "C" fn mode_write(owner: *mut c_void, flags: u32) {
    let state = (*owner.cast::<ModeOwner>()).state;
    (*state).code_app = flags & 2 != 0;
    (*state).code_english = flags & 4 != 0;
}
unsafe extern "C" fn mode_notify(owner: *mut c_void, english: i32) {
    let owner = &*owner.cast::<ModeOwner>();
    if let Some(notice) = owner.notice {
        notice(owner.context, english);
    }
    // apply_mode holds the service lifetime even if notice releases the caller.
    meltype_tsf_service_update_langbar(owner.state);
}
/// Normalize reply bits active=1,code=2,English=4 and publish before UI updates.
/// Only a change in normalized code English sends a notice and refreshes the bar.
/// # Safety
/// Live state/context on their TSF thread, pinned by borrowed lifetime IUnknown.
/// Optional notice is synchronous, cannot unwind or retain its stack arguments.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_apply_mode(
    state: *mut ServiceState,
    reply: u32,
    lifetime: *mut c_void,
    context: *mut c_void,
    notice: Option<unsafe extern "C" fn(*mut c_void, i32)>,
) -> i32 {
    if state.is_null() || lifetime.is_null() {
        return 0;
    }
    let mut owner = ModeOwner {
        state,
        context,
        notice,
    };
    let callbacks = mode::ApplyCallbacks {
        owner: ptr::addr_of_mut!(owner).cast(),
        lifetime,
        read: mode_read,
        write: mode_write,
        notify: mode_notify,
    };
    mode::meltype_tsf_apply_mode(reply, &callbacks)
}
unsafe fn release_interface(raw: *mut c_void) {
    if !raw.is_null() {
        drop(IUnknown::from_raw(raw));
    }
}
/// Own edit-subscription replacement: detach old slots before releasing COM,
/// retain the service through callbacks, and discard a superseded late token.
/// Null document unadvises without resetting own-edit, matching upstream.
/// # Safety
/// Live state and borrowed lifetime/sink/document SDK interfaces on their TSF
/// thread. The lifetime reference keeps state alive until this function returns.
/// No concurrent use. Null document accepts null sink; lifetime is required.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_edit_sink(
    state: *mut ServiceState,
    document: *mut c_void,
    lifetime: *mut c_void,
    sink: *mut c_void,
) -> HRESULT {
    if state.is_null() || lifetime.is_null() || (!document.is_null() && sink.is_null()) {
        return E_INVALIDARG;
    }
    let epoch = (*state).edit_epoch.wrapping_add(1);
    (*state).edit_epoch = epoch;
    let focus_epoch = (*state).focus_epoch;
    let _owner = IUnknown::from_raw_borrowed(&lifetime).cloned().unwrap();
    if (*state).edit_epoch != epoch || (*state).focus_epoch != focus_epoch {
        return S_OK;
    }
    let old = ptr::addr_of_mut!((*state).edit_subscription).replace(ptr::null_mut());
    let old_context = ptr::addr_of_mut!((*state).edit_context).replace(ptr::null_mut());
    subscriptions::meltype_tsf_unsubscribe(old);
    release_interface(old_context);
    if (*state).edit_epoch != epoch
        || (*state).focus_epoch != focus_epoch
        || document.is_null()
        || (*state).shutdown_phase != 0
        || (*state).deactivated
    {
        return S_OK;
    }
    let client = (*state).client;
    let mut context = ptr::null_mut();
    let token = subscriptions::meltype_tsf_subscribe(3, document, client, sink, &mut context);
    if (*state).edit_epoch != epoch
        || (*state).focus_epoch != focus_epoch
        || (*state).shutdown_phase != 0
        || (*state).deactivated
    {
        subscriptions::meltype_tsf_unsubscribe(token);
        release_interface(context);
        return S_OK;
    }
    (*state).edit_subscription = token;
    (*state).edit_context = context;
    (*state).own_edit = false;
    S_OK
}
/// Set upstream reactivation defaults, preserving the fields that upstream does
/// not reset (including open/code/private/scope/caret policy and existing tokens).
/// # Safety
/// Live state on its TSF thread; sid is borrowed NUL-terminated bytes. This call
/// performs no COM/GUI callbacks and leaves all state untouched for null inputs.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_initialize(
    state: *mut ServiceState,
    client: u32,
    disabled: i32,
    own: i32,
    sid: *const c_char,
) -> i32 {
    if state.is_null() || sid.is_null() {
        return 0;
    }
    if meltype_tsf_service_set_sid(state, sid) == 0 {
        return 0;
    }
    let s = &mut *state;
    s.mode_epoch = s.mode_epoch.wrapping_add(1);
    s.client = client;
    s.deactivated = false;
    s.server_active = false;
    s.direct_detection = false;
    s.full_width_space = false;
    s.direct_word = false;
    s.last_hello = 0;
    s.caret_moved = false;
    s.own_edit = false;
    s.secret_checked = false;
    s.last_converting = false;
    s.pass_reason = ptr::null();
    s.key_failed = false;
    s.consumed = true;
    s.own_process = own != 0;
    s.disabled = disabled != 0;
    1
}
/// # Safety
/// Live state on its TSF thread at entry; no state access follows COM releases.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_clear_resume(state: *mut ServiceState) {
    if !state.is_null() {
        let resume = ptr::addr_of_mut!((*state).resume).replace(ptr::null_mut());
        recovery::meltype_tsf_destroy_resume(resume);
    }
}
/// # Safety
/// Live state/context on their TSF thread throughout creation (graphics or
/// window teardown may reenter). Callback remains live until replacement/drop.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_create_candidates(
    state: *mut ServiceState,
    owner: *mut c_void,
    select: Option<unsafe extern "C" fn(*mut c_void, i32, i32)>,
) -> *mut c_void {
    create_candidates(state, owner, select, None)
}
unsafe fn create_candidates(
    state: *mut ServiceState,
    owner: *mut c_void,
    select: Option<unsafe extern "C" fn(*mut c_void, i32, i32)>,
    adapter: Option<std::rc::Rc<dyn std::any::Any>>,
) -> *mut c_void {
    if state.is_null() || (*state).shutdown_phase != 0 {
        return ptr::null_mut();
    }
    let epoch = (*state).focus_epoch;
    let created = candidates::create(owner, select, adapter);
    if (*state).shutdown_phase != 0 || (*state).focus_epoch != epoch {
        candidates::meltype_tsf_candidate_destroy(created);
        return ptr::null_mut();
    }
    let old = ptr::addr_of_mut!((*state).candidates).replace(created);
    candidates::meltype_tsf_candidate_destroy(old);
    if (*state).shutdown_phase == 0
        && (*state).focus_epoch == epoch
        && (*state).candidates == created
    {
        created
    } else {
        ptr::null_mut()
    }
}
/// # Safety
/// State is null or live at entry on its TSF thread. Context/composition/json
/// follow candidate_show_tsf. No state access follows reentrant COM/GUI calls.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_show_candidates(
    state: *mut ServiceState,
    context: *mut c_void,
    composition: *mut c_void,
    cookie: u32,
    json: *const c_char,
) -> i32 {
    if state.is_null() {
        return 0;
    }
    let handle = ptr::addr_of!((*state).candidates).read();
    candidates::presentation::meltype_tsf_candidate_show_tsf(
        handle,
        context,
        composition,
        cookie,
        json,
    )
}
/// # Safety
/// State is null or live at entry on its TSF thread. Reentrant window events may
/// destroy the service; no state access follows the native hide/destroy call.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_hide_candidates(state: *mut ServiceState) {
    if !state.is_null() {
        let handle = ptr::addr_of!((*state).candidates).read();
        candidates::meltype_tsf_candidate_hide(handle);
    }
}
/// # Safety
/// Same state/thread lifetime contract as service_hide_candidates.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_drop_candidates(state: *mut ServiceState) {
    if !state.is_null() {
        let handle = ptr::addr_of_mut!((*state).candidates).replace(ptr::null_mut());
        candidates::meltype_tsf_candidate_destroy(handle);
    }
}
unsafe fn retire_bar(raw: *mut c_void) {
    if !raw.is_null() {
        langbar::meltype_tsf_langbar_detach(raw);
        drop(IUnknown::from_raw(raw));
    }
}
/// Replace the owned bar and return a borrowed ITfLangBarItem. Publish the new
/// slot before detaching/releasing the old bar, whose sink release may reenter.
/// # Safety
/// Live state and callback context on their TSF thread for this entire call and
/// until removal/destruction. Caller retains the service across reentrant sink
/// releases. Callback signatures follow create_langbar and never unwind.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_create_langbar(
    state: *mut ServiceState,
    owner: *mut c_void,
    read: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    toggle: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut c_void {
    create_langbar(state, owner, read, toggle, None)
}
unsafe fn create_langbar(
    state: *mut ServiceState,
    owner: *mut c_void,
    read: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    toggle: Option<unsafe extern "C" fn(*mut c_void)>,
    adapter: Option<std::rc::Rc<dyn std::any::Any>>,
) -> *mut c_void {
    if state.is_null() || (*state).shutdown_phase != 0 {
        return ptr::null_mut();
    }
    let epoch = (*state).focus_epoch;
    let created = langbar::create(owner, read, toggle, adapter);
    let previous = ptr::addr_of_mut!((*state).langbar).replace(created);
    retire_bar(previous);
    if (*state).shutdown_phase == 0 && (*state).focus_epoch == epoch && (*state).langbar == created
    {
        created
    } else {
        ptr::null_mut()
    }
}
/// # Safety
/// State is null or live on its TSF thread at entry. A local COM reference keeps
/// the bar alive if its notification destroys the service. No state access follows.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_update_langbar(state: *mut ServiceState) {
    if state.is_null() {
        return;
    }
    let raw = ptr::addr_of!((*state).langbar).read();
    if let Some(bar) = IUnknown::from_raw_borrowed(&raw).cloned() {
        langbar::meltype_tsf_langbar_update(bar.as_raw());
    }
}
/// Detach the callback context and slot before querying/removing from TSF.
/// # Safety
/// State is null or live on its TSF thread at entry. Manager is null or borrowed
/// IUnknown kept alive for the call. Reentrant replacement is left untouched.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_remove_langbar(
    state: *mut ServiceState,
    manager: *mut c_void,
) {
    if state.is_null() {
        return;
    }
    let raw = ptr::addr_of_mut!((*state).langbar).replace(ptr::null_mut());
    if raw.is_null() {
        return;
    }
    langbar::meltype_tsf_langbar_detach(raw);
    let bar = ITfLangBarItem::from_raw(raw);
    if let Some(manager) = IUnknown::from_raw_borrowed(&manager) {
        if let Some(items) = fields::query::<ITfLangBarItemMgr>(manager) {
            let _ = items.RemoveItem(&bar);
        }
    }
}
unsafe fn close_transport(state: *mut ServiceState) {
    let handle = ptr::addr_of_mut!((*state).transport).replace(ptr::null_mut());
    crate::tip_transport::meltype_tip_transport_destroy(handle);
}
/// # Safety
/// Live service state on its TSF thread; sid is a NUL-terminated byte string.
/// The bytes are copied before replacing old storage, so self-assignment is valid.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_set_sid(
    state: *mut ServiceState,
    sid: *const c_char,
) -> i32 {
    if state.is_null() || sid.is_null() {
        return 0;
    }
    let owned = CStr::from_ptr(sid).to_owned().into_raw();
    let old = ptr::addr_of_mut!((*state).sid).replace(owned);
    if !old.is_null() {
        drop(CString::from_raw(old));
    }
    1
}
/// Borrowed until the next set_sid or state destruction; null state returns null.
/// # Safety
/// state is null or live on its TSF thread; callers never free the returned bytes.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_sid(state: *const ServiceState) -> *const c_char {
    if state.is_null() {
        return ptr::null();
    }
    let sid = ptr::addr_of!((*state).sid).read();
    if sid.is_null() {
        c"".as_ptr()
    } else {
        sid
    }
}
/// # Safety
/// Live state on its TSF thread with no concurrent request. Worker and request are
/// borrowed NUL-terminated UTF-8, as for tip_transport_create/request. No COM or
/// shell callbacks occur. Returned text is owned and freed with meltype_free.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_request(
    state: *mut ServiceState,
    worker: *const c_char,
    request: *const c_char,
    timeout: u32,
    optional: i32,
) -> *mut c_char {
    if state.is_null() || worker.is_null() || request.is_null() {
        return ptr::null_mut();
    }
    let slot = ptr::addr_of_mut!((*state).transport);
    let mut handle = slot.read();
    if handle.is_null() {
        handle = crate::tip_transport::meltype_tip_transport_create(worker);
        if handle.is_null() {
            return ptr::null_mut();
        }
        slot.write(handle);
    }
    crate::tip_transport::meltype_tip_transport_request(handle, request, timeout, optional)
}
/// Detach the transport before stopping its worker. Repeated close is inert.
/// # Safety
/// state is null or live on its TSF thread; no concurrent request/destruction.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_close(state: *mut ServiceState) {
    if !state.is_null() {
        close_transport(state);
    }
}
static LIVE: AtomicU32 = AtomicU32::new(0);
/// Allocate a TSF-thread state with upstream defaults. Null means allocation
/// failure. The pointer is opaque except to a matching checked shell layout.
#[no_mangle]
pub extern "C" fn meltype_tsf_service_state_create() -> *mut ServiceState {
    catch_unwind(|| unsafe {
        let data = alloc(Layout::new::<ServiceState>()).cast::<ServiceState>();
        if data.is_null() {
            return data;
        }
        data.write(ServiceState::default());
        LIVE.fetch_add(1, Ordering::Relaxed);
        data
    })
    .unwrap_or(ptr::null_mut())
}
/// # Safety
/// data is null or a live allocation from service_state_create, freed once on
/// its TSF thread after all borrowed aliases/callbacks have ceased. Raw COM and
/// subscription slots must have been cleaned up by the lifecycle beforehand.
/// Rust-owned key/resume/abandoned, SID, transport and UI state is freed here.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_state_destroy(data: *mut ServiceState) {
    if !data.is_null() {
        drop(Box::from_raw(data));
        LIVE.fetch_sub(1, Ordering::Relaxed);
    }
}
/// # Safety
/// out, if nonnull with sufficient capacity, is writable for the returned count.
/// Metadata is version,size,alignment, then field offsets in the checked header.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_state_layout(out: *mut u32, capacity: u32) -> u32 {
    let values = [
        6,
        size_of::<ServiceState>() as u32,
        align_of::<ServiceState>() as u32,
        offset_of!(ServiceState, refs) as u32,
        offset_of!(ServiceState, manager) as u32,
        offset_of!(ServiceState, client) as u32,
        offset_of!(ServiceState, thread_subscription) as u32,
        offset_of!(ServiceState, key_subscription) as u32,
        offset_of!(ServiceState, mode_subscription) as u32,
        offset_of!(ServiceState, edit_subscription) as u32,
        offset_of!(ServiceState, edit_context) as u32,
        offset_of!(ServiceState, composition) as u32,
        offset_of!(ServiceState, composition_context) as u32,
        offset_of!(ServiceState, atom_input) as u32,
        offset_of!(ServiceState, atom_converted) as u32,
        offset_of!(ServiceState, atom_target) as u32,
        offset_of!(ServiceState, keys) as u32,
        offset_of!(ServiceState, resume) as u32,
        offset_of!(ServiceState, abandoned) as u32,
        offset_of!(ServiceState, last_hello) as u32,
        offset_of!(ServiceState, focus_epoch) as u32,
        offset_of!(ServiceState, mode_epoch) as u32,
        offset_of!(ServiceState, shutdown_phase) as u32,
        offset_of!(ServiceState, server_active) as u32,
        offset_of!(ServiceState, direct_detection) as u32,
        offset_of!(ServiceState, direct_word) as u32,
        offset_of!(ServiceState, open) as u32,
        offset_of!(ServiceState, code_app) as u32,
        offset_of!(ServiceState, code_english) as u32,
        offset_of!(ServiceState, disabled) as u32,
        offset_of!(ServiceState, own_process) as u32,
        offset_of!(ServiceState, last_converting) as u32,
        offset_of!(ServiceState, caret_moved) as u32,
        offset_of!(ServiceState, own_edit) as u32,
        offset_of!(ServiceState, secret_checked) as u32,
        offset_of!(ServiceState, secret_composing) as u32,
        offset_of!(ServiceState, private_field) as u32,
        offset_of!(ServiceState, keep_caret) as u32,
        offset_of!(ServiceState, deactivated) as u32,
        offset_of!(ServiceState, key_failed) as u32,
        offset_of!(ServiceState, consumed) as u32,
        offset_of!(ServiceState, scope_source) as u32,
        offset_of!(ServiceState, pass_reason) as u32,
        offset_of!(ServiceState, sid) as u32,
        offset_of!(ServiceState, transport) as u32,
        offset_of!(ServiceState, langbar) as u32,
        offset_of!(ServiceState, candidates) as u32,
        offset_of!(ServiceState, edit_epoch) as u32,
        offset_of!(ServiceState, full_width_space) as u32,
    ];
    if !out.is_null() && capacity as usize >= values.len() {
        ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
    }
    values.len() as u32
}
#[no_mangle]
pub extern "C" fn meltype_tsf_service_state_live_count() -> u32 {
    LIVE.load(Ordering::Relaxed)
}
