// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! Full key-edit orchestration. The shell supplies its remaining apply/commit
//! primitives; ordering, request construction and fallback decisions live here.
use super::{edit_session, key_state, keys, ranges};
use serde_json::{json, Value};
use std::{
    cell::Cell,
    ffi::{c_char, c_void, CStr, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
use windows::{
    core::{IUnknown, Interface, HRESULT},
    Win32::{Foundation::*, UI::TextServices::*},
};
// Snapshot: composing/open/secret checked/secret while composing/moved/private/failed/consumed.
const COMPOSING: u32 = 1;
const OPEN: u32 = 2;
const CHECKED: u32 = 4;
const SECRET: u32 = 8;
const MOVED: u32 = 16;
const PRIVATE: u32 = 32;
const FAILED: u32 = 64;
const CONSUMED: u32 = 128;
// Update operations, shared with the shell ABI.
const OUTCOME: u32 = 0;
const OWN: u32 = 1;
const CLEAR_MOVED: u32 = 2;
const SERVER: u32 = 3;
const SERVER_OFF: u32 = 4;
const HELLO_AGAIN: u32 = 5;
const OPEN_IME: u32 = 6;
const COMMIT_ASYNC: u32 = 7;
const REINJECT: u32 = 8;
// Edit primitives: secret/commit/end+hide/apply-mode/apply-reply.
const READ_SECRET: u32 = 0;
const COMMIT: u32 = 1;
const END: u32 = 2;
const APPLY_MODE: u32 = 3;
const APPLY: u32 = 4;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callbacks {
    pub(super) owner: *mut c_void,
    pub(super) lifetime: *mut c_void,
    pub(super) read: unsafe extern "C" fn(*mut c_void) -> u32,
    pub(super) update: unsafe extern "C" fn(*mut c_void, u32, u32),
    pub(super) edit: unsafe extern "C" fn(*mut c_void, u32, u32, *mut c_void) -> i32,
    pub(super) request: unsafe extern "C" fn(*mut c_void, *const c_char, *mut u32) -> i32,
    pub(super) write: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void, u16) -> i32,
    pub(super) log: unsafe extern "C" fn(*mut c_void, u32, i32),
}
#[derive(Clone, Copy, Default)]
struct Outcome {
    inserted_failure: bool,
    inserted_direct: bool,
    switch_japanese: bool,
    executed: bool,
}
struct Work {
    context: ITfContext,
    _owner: IUnknown,
    c: Callbacks,
    token: *mut c_void,
    key: u32,
    ch: u16,
    mods: u32,
    metadata: Value,
    result: Rc<Cell<Outcome>>,
    active: Rc<Cell<bool>>,
}
unsafe extern "C" fn surrounding(
    owner: *mut c_void,
    before: *const u16,
    nb: u32,
    after: *const u16,
    na: u32,
) {
    let out = &mut *owner.cast::<(String, String)>();
    out.0 = String::from_utf16_lossy(std::slice::from_raw_parts(before, nb as usize));
    out.1 = String::from_utf16_lossy(std::slice::from_raw_parts(after, na as usize));
}
impl Work {
    unsafe fn flags(&self) -> u32 {
        (self.c.read)(self.c.owner)
    }
    unsafe fn update(&self, op: u32, value: u32) {
        (self.c.update)(self.c.owner, op, value);
    }
    unsafe fn edit(&self, op: u32, cookie: u32) -> i32 {
        (self.c.edit)(self.c.owner, op, cookie, self.context.as_raw())
    }
    unsafe fn write(&self, cookie: u32) -> bool {
        let Some(range) = ranges::selection(&self.context, cookie) else {
            return false;
        };
        (self.c.write)(
            self.c.owner,
            cookie,
            self.context.as_raw(),
            range.as_raw(),
            self.ch,
        ) != 0
    }
    unsafe fn selection_matches(&self, cookie: u32, selected: &Option<(ITfRange, String)>) -> bool {
        let Some((original, text)) = selected else {
            return true;
        };
        self.flags() & COMPOSING == 0
            && ranges::selection(&self.context, cookie).is_some_and(|current| {
                current
                    .CompareStart(cookie, original, TF_ANCHOR_START)
                    .is_ok_and(|n| n == 0)
                    && current
                        .CompareEnd(cookie, original, TF_ANCHOR_END)
                        .is_ok_and(|n| n == 0)
                    && ranges::matches_text(
                        &current,
                        cookie,
                        &text.encode_utf16().collect::<Vec<_>>(),
                    )
            })
    }
    unsafe fn cancel_selection(&self, outcome: Outcome) {
        // A nested key may already own a new composition. Do not reset it.
        if self.flags() & COMPOSING == 0 {
            let reset = CString::new(
                json!({"op":"reset","sid":self.metadata["sid"],"direct":self.flags()&OPEN==0})
                    .to_string(),
            )
            .unwrap();
            let mut ignored = 0;
            if (self.c.request)(self.c.owner, reset.as_ptr(), &mut ignored) == 0 {
                self.update(SERVER_OFF, 0);
            }
        }
        // Replaying Convert would act on the newly selected document.
        self.update(OUTCOME, 2);
        self.result.set(outcome);
    }
    unsafe fn run(&self, cookie: u32) -> HRESULT {
        if !self.active.get() {
            return E_ABORT;
        }
        let mut outcome = Outcome {
            executed: true,
            ..Default::default()
        };
        let flags = self.flags();
        if flags & SECRET != 0 || flags & CHECKED == 0 && self.edit(READ_SECRET, cookie) != 0 {
            if self.flags() & COMPOSING != 0 {
                self.edit(COMMIT, cookie);
            }
            self.update(OUTCOME, u32::from(self.flags() & FAILED != 0));
            (self.c.log)(self.c.owner, 0, 0);
            self.result.set(outcome);
            return S_OK;
        }
        self.update(OWN, 1);
        let mut request = json!({"op":"key","sid":self.metadata["sid"],"process":self.metadata["process"],"vk":self.key,"ch":self.ch,"mods":self.mods,"code":true,"direct":self.flags()&OPEN==0});
        let selection = ranges::selection(&self.context, cookie);
        let can_delete = selection
            .as_ref()
            .and_then(|r| r.IsEmpty(cookie).ok())
            .is_some_and(|b| b.as_bool());
        // Hold a cloned range as well as its text: another range containing
        // the same word must not become the target after a reentrant request.
        let reconversion = if self.key == 0x1c
            && self.mods & 14 == 0
            && self.flags() & COMPOSING == 0
            && !can_delete
        {
            selection.and_then(|range| {
                let units = ranges::read(&range, cookie, 2049)?;
                let text = String::from_utf16(&units).ok()?;
                if text.is_empty() || text.chars().count() > 1024 {
                    return None;
                }
                Some((range.Clone().ok()?, text))
            })
        } else {
            None
        };
        if let Some((_, text)) = &reconversion {
            request["op"] = json!("reconvert");
            request["text"] = json!(text);
        }
        request["canDelete"] = json!(can_delete);
        let flags = self.flags();
        if flags & MOVED != 0 {
            request["moved"] = json!(true);
        }
        if flags & PRIVATE != 0 {
            request["private"] = json!(true);
        }
        let mut passed = key_state::Passed::default();
        if key_state::meltype_tsf_key_passed(self.token, 0, 0, &mut passed) != 0 {
            request["passed"] = json!(passed.key);
            if passed.control != 0 {
                request["ctrl"] = json!(true);
            }
        }
        self.update(CLEAR_MOVED, 0);
        if self.flags() & COMPOSING == 0 {
            let mut text = (String::new(), String::new());
            ranges::meltype_tsf_read_surrounding(
                self.context.as_raw(),
                cookie,
                (&mut text as *mut (String, String)).cast(),
                surrounding,
            );
            request["before"] = json!(text.0);
            request["after"] = json!(text.1);
        } else {
            request["composing"] = json!(true);
        }
        let Ok(request) = CString::new(request.to_string()) else {
            return E_FAIL;
        };
        let mut reply = 0;
        let responded = (self.c.request)(self.c.owner, request.as_ptr(), &mut reply) != 0;
        if !self.selection_matches(cookie, &reconversion) {
            self.cancel_selection(outcome);
            return S_OK;
        }
        if !responded {
            // Keep the existing consumed flag, as the original branch does.
            self.update(
                OUTCOME,
                1 | if self.flags() & CONSUMED != 0 { 2 } else { 0 },
            );
            self.update(SERVER_OFF, 0);
            self.edit(END, cookie);
            if self.ch >= 0x20 && self.mods & 14 == 0 {
                outcome.inserted_failure = self.write(cookie);
            }
            self.result.set(outcome);
            return S_OK;
        }
        // Reply bits: active, detection, English word, switch, consumed,
        // empty commits, null view. Normalize dependent fields before publishing.
        let active = reply & 1 != 0;
        let detection = active && reply & 2 != 0;
        let english = detection && reply & 4 != 0;
        self.update(
            SERVER,
            u32::from(active)
                | if detection { 2 } else { 0 }
                | if english { 4 } else { 0 }
                | if active && reply & 128 != 0 { 8 } else { 0 },
        );
        outcome.switch_japanese = reply & 8 != 0;
        self.update(
            OUTCOME,
            u32::from(self.flags() & FAILED != 0) | if reply & 16 != 0 { 2 } else { 0 },
        );
        self.edit(APPLY_MODE, cookie);
        if !self.selection_matches(cookie, &reconversion) {
            self.cancel_selection(outcome);
            return S_OK;
        }
        if !active && self.flags() & COMPOSING != 0 {
            self.update(HELLO_AGAIN, 0);
        }
        self.edit(APPLY, cookie);
        let flags = self.flags();
        if flags & CONSUMED == 0
            && flags & OPEN == 0
            && !outcome.switch_japanese
            && (0x20..=0x7e).contains(&self.ch)
            && self.key != 0x20
            && self.mods & 14 == 0
            && reply & 96 == 96
        {
            outcome.inserted_direct = self.write(cookie);
            if !outcome.inserted_direct {
                (self.c.log)(self.c.owner, 1, 0);
            }
        }
        self.result.set(outcome);
        S_OK
    }
}
unsafe extern "C" fn invoke(owner: *mut c_void, cookie: u32) -> HRESULT {
    catch_unwind(AssertUnwindSafe(|| (&*owner.cast::<Work>()).run(cookie))).unwrap_or(E_FAIL)
}
unsafe extern "C" fn destroy(owner: *mut c_void) {
    drop(Box::from_raw(owner.cast::<Work>()));
}
/// # Safety
/// Borrowed owner/context on their TSF thread. Non-null callback functions never
/// throw. metadata is UTF-8 JSON {sid:string,process:string}; callback-owned reply
/// storage survives this call. Deferred/refused SYNC sessions cannot access it
/// after return. See the public header for snapshot, update and edit operations.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_handle_key(
    context: *mut c_void,
    client: u32,
    token: *mut c_void,
    key: u32,
    ch: u16,
    metadata: *const c_char,
    callbacks: *const Callbacks,
) -> HRESULT {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(c) = callbacks.as_ref().copied() else {
            return E_INVALIDARG;
        };
        let Some(owner) = IUnknown::from_raw_borrowed(&c.lifetime).cloned() else {
            return E_INVALIDARG;
        };
        if metadata.is_null() || key >= 256 {
            return E_INVALIDARG;
        }
        let Ok(metadata) = serde_json::from_slice::<Value>(CStr::from_ptr(metadata).to_bytes())
        else {
            return E_INVALIDARG;
        };
        if !metadata["sid"].is_string() || !metadata["process"].is_string() {
            return E_INVALIDARG;
        }
        let context = ITfContext::from_raw_borrowed(&context).cloned();
        let mods = keys::meltype_tsf_key_modifiers();
        (c.update)(c.owner, OUTCOME, 2);
        let result = Rc::new(Cell::new(Outcome::default()));
        let active = Rc::new(Cell::new(true));
        let status = if let Some(context) = &context {
            let work = Box::new(Work {
                context: context.clone(),
                _owner: owner.clone(),
                c,
                token,
                key,
                ch,
                mods,
                metadata,
                result: result.clone(),
                active: active.clone(),
            });
            edit_session::meltype_tsf_request_edit_session(
                context.as_raw(),
                client,
                (TF_ES_SYNC | TF_ES_READWRITE).0,
                Box::into_raw(work).cast(),
                Some(invoke),
                destroy,
            )
        } else {
            E_INVALIDARG
        };
        active.set(false);
        (c.update)(c.owner, OWN, 0);
        let outcome = result.get();
        // A host retaining a SYNC session must not turn it into a later key edit.
        let status = if status.is_ok() && !outcome.executed {
            E_ABORT
        } else {
            status
        };
        let mut flags = (c.read)(c.owner);
        if status.is_ok() && outcome.switch_japanese && flags & FAILED == 0 && flags & OPEN == 0 {
            (c.update)(c.owner, OPEN_IME, 1);
        }
        if status.is_err() {
            (c.log)(c.owner, 2, status.0);
            flags = (c.read)(c.owner);
            (c.update)(
                c.owner,
                OUTCOME,
                1 | if flags & CONSUMED != 0 { 2 } else { 0 },
            );
            if flags & COMPOSING != 0 {
                (c.update)(c.owner, COMMIT_ASYNC, 1);
            }
        } else if (c.read)(c.owner) & FAILED != 0 {
            (c.log)(c.owner, 3, 0);
        }
        flags = (c.read)(c.owner);
        if flags & FAILED != 0 && !outcome.inserted_failure
            || flags & CONSUMED == 0 && !outcome.inserted_direct
        {
            (c.update)(c.owner, REINJECT, key);
        }
        status
    }))
    .unwrap_or(E_FAIL)
}
