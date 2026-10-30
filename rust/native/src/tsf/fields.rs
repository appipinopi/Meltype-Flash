// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! TSF field privacy. Only metadata is read; no input text reaches this module.
use super::ranges;
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};
use windows::{
    core::{IUnknown, Interface, GUID, VARIANT},
    Win32::{System::Com::CoTaskMemFree, UI::TextServices::*},
};
const INPUT_SCOPE: GUID = GUID::from_u128(0x1713dd5a_68e7_4a5b_9af6_592a595c778d);
const SECRET: u32 = 1;
const PRIVATE: u32 = 2;
const UNREADABLE: u32 = 4;
const ASCII: u32 = 8;
// VARIANT discriminants from wtypes.h: EMPTY, NULL, UNKNOWN, I4.
const EMPTY: u16 = 0;
const NULL: u16 = 1;
const UNKNOWN: u16 = 13;
const I4: u16 = 3;
struct ScopeList(*mut InputScope);
impl Drop for ScopeList {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0.cast()));
        }
    }
}
pub(super) unsafe fn query<T: Interface>(owner: &IUnknown) -> Option<T> {
    let mut raw = ptr::null_mut();
    let status = (Interface::vtable(owner).QueryInterface)(owner.as_raw(), &T::IID, &mut raw);
    let value = (!raw.is_null()).then(|| T::from_raw(raw));
    if status.is_ok() {
        value
    } else {
        None
    }
}
unsafe fn property(context: &ITfContext, app: bool) -> Option<ITfReadOnlyProperty> {
    let mut raw = ptr::null_mut();
    let method = if app {
        Interface::vtable(context).GetAppProperty
    } else {
        Interface::vtable(context).GetProperty
    };
    let status = method(context.as_raw(), &INPUT_SCOPE, &mut raw);
    // ITfProperty inherits ITfReadOnlyProperty at the same interface address.
    let value = (!raw.is_null()).then(|| ITfReadOnlyProperty::from_raw(raw));
    if status.is_ok() {
        value
    } else {
        None
    }
}
unsafe fn inspect(context: *mut c_void, cookie: u32, chromium: bool) -> u32 {
    let Some(context) = ITfContext::from_raw_borrowed(&context).cloned() else {
        return 0;
    };
    let (windows, app) = (property(&context, false), property(&context, true));
    let sources = (u32::from(windows.is_some()) | u32::from(app.is_some()) << 1) << 8;
    if sources == 0 {
        return 0;
    }
    let Some(selection) = ranges::selection(&context, cookie) else {
        return sources;
    };
    let (mut flags, mut failure, mut app_secret) = (sources, 0, false);
    for (from_app, property) in [(false, windows), (true, app)].into_iter() {
        let Some(property) = property else { continue };
        let mut value = VARIANT::new();
        // Keep an initialized owner even if the host fills output before failing.
        let status = (Interface::vtable(&property).GetValue)(
            property.as_raw(),
            cookie,
            selection.as_raw(),
            (&mut value as *mut VARIANT).cast(),
        );
        if status.is_err() {
            continue;
        }
        let raw = &value.as_raw().Anonymous.Anonymous;
        match raw.vt {
            EMPTY | NULL => {}
            UNKNOWN if raw.Anonymous.punkVal.is_null() => {}
            UNKNOWN => {
                let scope = IUnknown::try_from(&value)
                    .ok()
                    .and_then(|v| query::<ITfInputScope>(&v));
                let Some(scope) = scope else {
                    failure = 1;
                    continue;
                };
                let mut list = ScopeList(ptr::null_mut());
                let mut count = 0;
                let status = scope.GetInputScopes(&mut list.0, &mut count);
                if status.is_err() || (list.0.is_null() && count != 0) {
                    failure = 2;
                    continue;
                }
                if !list.0.is_null() {
                    for index in 0..count as usize {
                        let scope = *list.0.add(index);
                        if matches!(
                            scope,
                            IS_PASSWORD
                                | IS_NUMERIC_PASSWORD
                                | IS_NUMERIC_PIN
                                | IS_ALPHANUMERIC_PIN
                                | IS_ALPHANUMERIC_PIN_SET
                        ) {
                            if from_app {
                                app_secret = true;
                            } else {
                                flags |= SECRET;
                            }
                        }
                        // #992: メール・半角指定欄ではキーをそのままアプリへ渡す。
                        // URL は検索欄でも使われるので日本語入力を禁止しない。
                        if matches!(
                            scope,
                            IS_EMAIL_USERNAME
                                | IS_EMAIL_SMTPEMAILADDRESS
                                | IS_ALPHANUMERIC_HALFWIDTH
                        ) {
                            flags |= ASCII;
                        }
                        if scope == IS_PRIVATE {
                            flags |= PRIVATE;
                        }
                    }
                }
            }
            _ => failure = 3,
        }
    }
    // #878/#874: Chromium sometimes advertises a normal field as an app
    // password scope. Windows scopes, disabled contexts and read failures
    // remain authoritative; private/no-learning and ASCII flags are retained.
    if app_secret
        && !(chromium && flags & SECRET == 0 && meltype_tsf_context_disabled(context.as_raw()) == 0)
    {
        flags |= SECRET;
    }
    if failure != 0 {
        flags |= SECRET | UNREADABLE | failure << 16;
    }
    flags
}
/// Return bit flags: secret=1, private/no-learning=2, unreadable=4, ASCII/raw=8;
/// sources=(Windows=1,application=2)<<8; unreadable reason (QI=1,list=2,
/// variant=3)<<16. Missing metadata/selection is an ordinary unspecified field.
/// # Safety
/// Borrowed context (or null), valid read cookie, on its TSF thread.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_input_scope(context: *mut c_void, cookie: u32) -> u32 {
    meltype_tsf_browser_input_scope(context, cookie, 0)
}

/// Apply the upstream Chromium app-scope workaround without bypassing Windows
/// password metadata, IME-disabled contexts, ASCII fields or read errors.
/// # Safety
/// Same borrowed context/thread/cookie requirements as meltype_tsf_input_scope.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_browser_input_scope(
    context: *mut c_void,
    cookie: u32,
    chromium: i32,
) -> u32 {
    catch_unwind(AssertUnwindSafe(|| inspect(context, cookie, chromium != 0)))
        .unwrap_or(SECRET | UNREADABLE)
}
pub(super) unsafe fn manager(owner: *mut c_void) -> Option<ITfCompartmentMgr> {
    let owner = IUnknown::from_raw_borrowed(&owner)?;
    query::<ITfCompartmentMgr>(owner)
}
pub(super) unsafe fn compartment(
    manager: &ITfCompartmentMgr,
    guid: &GUID,
) -> Option<ITfCompartment> {
    let mut raw = ptr::null_mut();
    let status = (Interface::vtable(manager).GetCompartment)(manager.as_raw(), guid, &mut raw);
    let value = (!raw.is_null()).then(|| ITfCompartment::from_raw(raw));
    if status.is_ok() {
        value
    } else {
        None
    }
}
unsafe fn value(compartment: &ITfCompartment) -> Option<VARIANT> {
    let mut value = VARIANT::new();
    let status = (Interface::vtable(compartment).GetValue)(
        compartment.as_raw(),
        (&mut value as *mut VARIANT).cast(),
    );
    if status.is_ok() {
        Some(value)
    } else {
        None
    }
}
unsafe fn integer(value: &VARIANT) -> Option<i32> {
    let raw = &value.as_raw().Anonymous.Anonymous;
    if raw.vt == I4 {
        Some(raw.Anonymous.lVal)
    } else {
        None
    }
}
/// Read open/close as 0/1, -1 for an unavailable value, -2 for no manager.
/// # Safety
/// Borrowed thread manager on its TSF thread, or null.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_read_open(owner: *mut c_void) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(manager) = manager(owner) else {
            return -2;
        };
        compartment(&manager, &GUID_COMPARTMENT_KEYBOARD_OPENCLOSE)
            .and_then(|c| value(&c))
            .map(|v| i32::from(integer(&v).is_some_and(|v| v != 0)))
            .unwrap_or(-1)
    }))
    .unwrap_or(-1)
}
/// Write the open/close integer using the current TSF client id.
/// # Safety
/// Borrowed thread manager on its TSF thread, or null.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_write_open(owner: *mut c_void, client: u32, open: i32) -> i32 {
    i32::from(
        catch_unwind(AssertUnwindSafe(|| {
            let Some(compartment) =
                manager(owner).and_then(|m| compartment(&m, &GUID_COMPARTMENT_KEYBOARD_OPENCLOSE))
            else {
                return false;
            };
            compartment
                .SetValue(client, &VARIANT::from(i32::from(open != 0)))
                .is_ok()
        }))
        .unwrap_or(false),
    )
}
/// Null context is disabled; missing/unsupported compartment metadata is not.
/// # Safety
/// Borrowed context on its TSF thread, or null.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_context_disabled(context: *mut c_void) -> i32 {
    if context.is_null() {
        return 1;
    }
    i32::from(
        catch_unwind(AssertUnwindSafe(|| {
            let Some(manager) = manager(context) else {
                return false;
            };
            let mut disabled = false;
            for guid in [
                GUID_COMPARTMENT_KEYBOARD_DISABLED,
                GUID_COMPARTMENT_EMPTYCONTEXT,
            ]
            .iter()
            {
                disabled |= compartment(&manager, guid)
                    .and_then(|c| value(&c))
                    .as_ref()
                    .and_then(|v| integer(v))
                    .is_some_and(|v| v != 0);
            }
            disabled
        }))
        .unwrap_or(true),
    )
}
