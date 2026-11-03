// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! Windows TSF COM objects. These use the generated Windows SDK vtables,
//! including system calling conventions on x86, rather than a C++ bridge.
mod abandon_control;
mod abandoned;
mod apply_reply;
mod candidates;
mod commit_request;
mod commits;
mod composition;
mod composition_owner;
mod core_link;
mod edit_events;
mod edit_session;
mod factory;
mod fields;
mod focus;
mod identity;
mod key_edit;
mod key_state;
mod keys;
mod langbar;
mod mode;
mod open_change;
mod process;
mod ranges;
mod recovery;
mod recovery_control;
mod registration;
mod service_com;
mod service_state;
mod shutdown;
mod startup;
mod subscriptions;
mod termination;
use std::{
    ffi::c_void,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{fence, AtomicU32, Ordering},
};
use windows::{
    core::{IUnknown, IUnknown_Vtbl, Interface, BSTR, GUID, HRESULT},
    Win32::{Foundation::*, UI::TextServices::*},
};

struct Spec {
    guid: GUID,
    description: &'static str,
    line: TF_DA_LINESTYLE,
    bold: bool,
    attr: TF_DA_ATTR_INFO,
}
static SPECS: [Spec; 3] = [
    Spec {
        guid: GUID::from_u128(0x51987acc_d57a_5c81_ad25_5c1e8d83f258),
        description: "Meltype-FLASH: 入力中",
        line: TF_LS_DOT,
        bold: false,
        attr: TF_ATTR_INPUT,
    },
    Spec {
        guid: GUID::from_u128(0x52a75cec_c221_5443_bb39_eb8d777c688f),
        description: "Meltype-FLASH: 変換した文節",
        line: TF_LS_SOLID,
        bold: false,
        attr: TF_ATTR_CONVERTED,
    },
    Spec {
        guid: GUID::from_u128(0x711d8fcb_cfbd_56d2_9a8e_672ee6ed74c1),
        description: "Meltype-FLASH: 選んでいる文節",
        line: TF_LS_SOLID,
        bold: true,
        attr: TF_ATTR_TARGET_CONVERTED,
    },
];

// Both objects have one interface. The atomic cursor also keeps enumerator
// state coherent if a client invokes it from more than one thread.
#[repr(C)]
struct Object {
    vtable: *const c_void,
    refs: AtomicU32,
    index: AtomicU32,
    _module: ModuleLifetime,
}
// COM children can outlive the service that returned them. Keep standalone
// DLL code resident until their final Release and all captured cleanup finish.
struct ModuleLifetime;
impl ModuleLifetime {
    fn new() -> Self {
        #[cfg(feature = "tsf-server")]
        server::retain_module();
        Self
    }
}
impl Drop for ModuleLifetime {
    fn drop(&mut self) {
        #[cfg(feature = "tsf-server")]
        server::release_module();
    }
}
fn object(vtable: *const c_void, index: u32) -> *mut c_void {
    Box::into_raw(Box::new(Object {
        vtable,
        refs: AtomicU32::new(1),
        index: AtomicU32::new(index),
        _module: ModuleLifetime::new(),
    }))
    .cast()
}
unsafe fn state<'a>(this: *mut c_void) -> &'a Object {
    &*this.cast::<Object>()
}
unsafe extern "system" fn query(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    out.write(ptr::null_mut());
    if iid.is_null() {
        return E_INVALIDARG;
    }
    let own = if state(this).vtable == ptr::addr_of!(INFO).cast() {
        ITfDisplayAttributeInfo::IID
    } else {
        IEnumTfDisplayAttributeInfo::IID
    };
    if *iid != IUnknown::IID && *iid != own {
        return E_NOINTERFACE;
    }
    add_ref(this);
    out.write(this);
    S_OK
}
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    state(this).refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let count = state(this).refs.fetch_sub(1, Ordering::Release) - 1;
    if count == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(this.cast::<Object>()));
    }
    count
}
const UNKNOWN: IUnknown_Vtbl = IUnknown_Vtbl {
    QueryInterface: query,
    AddRef: add_ref,
    Release: release,
};
static INFO: ITfDisplayAttributeInfo_Vtbl = ITfDisplayAttributeInfo_Vtbl {
    base__: UNKNOWN,
    GetGUID: guid,
    GetDescription: description,
    GetAttributeInfo: attribute,
    SetAttributeInfo: set_attribute,
    Reset: reset_attribute,
};
static ENUM: IEnumTfDisplayAttributeInfo_Vtbl = IEnumTfDisplayAttributeInfo_Vtbl {
    base__: UNKNOWN,
    Clone: clone_enum,
    Next: next,
    Reset: reset_enum,
    Skip: skip,
};
unsafe fn spec(this: *mut c_void) -> &'static Spec {
    &SPECS[state(this).index.load(Ordering::Relaxed) as usize]
}
unsafe extern "system" fn guid(this: *mut c_void, out: *mut GUID) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    out.write(spec(this).guid);
    S_OK
}
unsafe extern "system" fn description(this: *mut c_void, out: *mut MaybeUninit<BSTR>) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    out.write(MaybeUninit::new(BSTR::new()));
    let value: Vec<u16> = spec(this).description.encode_utf16().collect();
    match BSTR::from_wide(&value) {
        Ok(value) => {
            out.write(MaybeUninit::new(value));
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn attribute(this: *mut c_void, out: *mut TF_DISPLAYATTRIBUTE) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    let spec = spec(this);
    out.write(TF_DISPLAYATTRIBUTE {
        lsStyle: spec.line,
        fBoldLine: BOOL::from(spec.bold),
        bAttr: spec.attr,
        // Text, background and underline retain the application's colors.
        ..Default::default()
    });
    S_OK
}
unsafe extern "system" fn set_attribute(
    _this: *mut c_void,
    _value: *const TF_DISPLAYATTRIBUTE,
) -> HRESULT {
    E_NOTIMPL
}
unsafe extern "system" fn reset_attribute(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn clone_enum(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    out.write(object(
        ptr::addr_of!(ENUM).cast(),
        state(this).index.load(Ordering::Relaxed),
    ));
    S_OK
}
unsafe extern "system" fn next(
    this: *mut c_void,
    count: u32,
    out: *mut *mut c_void,
    fetched: *mut u32,
) -> HRESULT {
    if !fetched.is_null() {
        fetched.write(0);
    }
    if out.is_null() || (count != 1 && fetched.is_null()) {
        return E_INVALIDARG;
    }
    let index = &state(this).index;
    let start = advance(index, count);
    let actual = count.min(SPECS.len() as u32 - start);
    for i in 0..actual {
        out.add(i as usize)
            .write(object(ptr::addr_of!(INFO).cast(), start + i));
    }
    if !fetched.is_null() {
        fetched.write(actual);
    }
    if actual == count {
        S_OK
    } else {
        S_FALSE
    }
}
unsafe extern "system" fn reset_enum(this: *mut c_void) -> HRESULT {
    state(this).index.store(0, Ordering::Relaxed);
    S_OK
}
unsafe extern "system" fn skip(this: *mut c_void, count: u32) -> HRESULT {
    let old = advance(&state(this).index, count);
    if count <= SPECS.len() as u32 - old {
        S_OK
    } else {
        S_FALSE
    }
}
fn advance(index: &AtomicU32, count: u32) -> u32 {
    let mut old = index.load(Ordering::Relaxed);
    loop {
        let new = old.saturating_add(count).min(SPECS.len() as u32);
        match index.compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return old,
            Err(actual) => old = actual,
        }
    }
}

/// Returns an owned COM reference. The caller must Release it, not meltype_free.
/// # Safety
/// `id` must be null or point to a readable GUID.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_create_display_attribute(id: *const GUID) -> *mut c_void {
    if id.is_null() {
        return ptr::null_mut();
    }
    SPECS
        .iter()
        .position(|s| s.guid == *id)
        .map_or(ptr::null_mut(), |index| {
            object(ptr::addr_of!(INFO).cast(), index as u32)
        })
}
/// Returns an owned IEnumTfDisplayAttributeInfo reference, released with COM Release.
#[no_mangle]
pub extern "C" fn meltype_tsf_create_display_attribute_enum() -> *mut c_void {
    object(ptr::addr_of!(ENUM).cast(), 0)
}
#[cfg(feature = "tsf-server")]
mod server;
