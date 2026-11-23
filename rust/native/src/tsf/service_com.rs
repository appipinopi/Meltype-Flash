// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! TSF service COM identity and SDK vtables. Remaining data adapters are called
//! through a synchronous C ABI while a native owner reference pins their lifetime.
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{fence, AtomicU32, Ordering},
    thread::{self, ThreadId},
};
use windows::{
    core::{IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT},
    Win32::{Foundation::*, UI::TextServices::*},
};
#[repr(C)]
pub struct Call {
    pub op: u32,
    pub a: *mut c_void,
    pub b: *mut c_void,
    pub c: *mut c_void,
    pub n1: usize,
    pub n2: isize,
    /// True when this call is a key-up (OnKeyUp / OnTestKeyUp) event.
    pub is_up: bool,
}
type Dispatch = unsafe extern "C" fn(*mut c_void, *const Call) -> HRESULT;
type Destroy = unsafe extern "C" fn(*mut c_void);
#[repr(C)]
struct Slot {
    vtable: *const c_void,
    owner: *mut Service,
}
#[repr(C)]
struct Service {
    slots: [Slot; 7],
    refs: AtomicU32,
    thread: ThreadId,
    context: *mut c_void,
    dispatch: Dispatch,
    destroy: Destroy,
}
unsafe fn owner(this: *mut c_void) -> *mut Service {
    (*this.cast::<Slot>()).owner
}
unsafe fn interface(service: *mut Service, iid: &GUID) -> *mut c_void {
    let index = if [
        IUnknown::IID,
        ITfTextInputProcessor::IID,
        ITfTextInputProcessorEx::IID,
    ]
    .contains(iid)
    {
        0
    } else if *iid == ITfThreadMgrEventSink::IID {
        1
    } else if *iid == ITfKeyEventSink::IID {
        2
    } else if *iid == ITfCompositionSink::IID {
        3
    } else if *iid == ITfDisplayAttributeProvider::IID {
        4
    } else if *iid == ITfCompartmentEventSink::IID {
        5
    } else if *iid == ITfTextEditSink::IID {
        6
    } else {
        return ptr::null_mut();
    };
    ptr::addr_of_mut!((*service).slots[index]).cast()
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
    let result = interface(owner(this), &*iid);
    if result.is_null() {
        return E_NOINTERFACE;
    }
    add_ref(this);
    out.write(result);
    S_OK
}
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    (*owner(this)).refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let raw = owner(this);
    let remaining = (*raw).refs.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        let service = Box::from_raw(raw);
        (service.destroy)(service.context);
        drop(service);
    }
    remaining
}
const UNKNOWN: IUnknown_Vtbl = IUnknown_Vtbl {
    QueryInterface: query,
    AddRef: add_ref,
    Release: release,
};
unsafe fn call(this: *mut c_void, value: Call) -> HRESULT {
    let service = owner(this);
    if (*service).thread != thread::current().id() {
        return RPC_E_WRONG_THREAD;
    }
    add_ref(this);
    let result = ((*service).dispatch)((*service).context, &value);
    // Destruction can free the adapter and its state. Do not access them again.
    release(this);
    result
}
fn args(op: u32, a: *mut c_void, b: *mut c_void, c: *mut c_void, n1: usize, n2: isize) -> Call {
    Call {
        op,
        a,
        b,
        c,
        n1,
        n2,
        is_up: false,
    }
}
unsafe extern "system" fn activate(t: *mut c_void, m: *mut c_void, id: u32) -> HRESULT {
    call(
        t,
        args(0, m, ptr::null_mut(), ptr::null_mut(), id as usize, 0),
    )
}
unsafe extern "system" fn activate_ex(
    t: *mut c_void,
    m: *mut c_void,
    id: u32,
    flags: u32,
) -> HRESULT {
    call(
        t,
        args(
            1,
            m,
            ptr::null_mut(),
            ptr::null_mut(),
            id as usize,
            flags as isize,
        ),
    )
}
unsafe extern "system" fn deactivate(t: *mut c_void) -> HRESULT {
    call(
        t,
        args(2, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), 0, 0),
    )
}
unsafe extern "system" fn ignore(_t: *mut c_void, _p: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn focus(t: *mut c_void, a: *mut c_void, b: *mut c_void) -> HRESULT {
    call(t, args(5, a, b, ptr::null_mut(), 0, 0))
}
unsafe extern "system" fn foreground(t: *mut c_void, value: BOOL) -> HRESULT {
    call(
        t,
        args(
            8,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            value.0 as usize,
            0,
        ),
    )
}
unsafe fn key(
    t: *mut c_void,
    op: u32,
    context: *mut c_void,
    vk: WPARAM,
    message: LPARAM,
    eaten: *mut BOOL,
) -> HRESULT {
    if eaten.is_null() {
        return E_INVALIDARG;
    }
    eaten.write(BOOL(0));
    let mut call_args = args(op, context, eaten.cast(), ptr::null_mut(), vk.0, message.0);
    // OnTestKeyUp (10) and OnKeyUp (12) are key-up events. Carry the flag so
    // the worker JSON can include `"is_up": true` for the core session.
    call_args.is_up = matches!(op, 10 | 12);
    call(t, call_args)
}
unsafe extern "system" fn test_down(
    t: *mut c_void,
    c: *mut c_void,
    w: WPARAM,
    l: LPARAM,
    e: *mut BOOL,
) -> HRESULT {
    key(t, 9, c, w, l, e)
}
unsafe extern "system" fn test_up(
    t: *mut c_void,
    c: *mut c_void,
    w: WPARAM,
    l: LPARAM,
    e: *mut BOOL,
) -> HRESULT {
    key(t, 10, c, w, l, e)
}
unsafe extern "system" fn down(
    t: *mut c_void,
    c: *mut c_void,
    w: WPARAM,
    l: LPARAM,
    e: *mut BOOL,
) -> HRESULT {
    key(t, 11, c, w, l, e)
}
unsafe extern "system" fn up(
    t: *mut c_void,
    c: *mut c_void,
    w: WPARAM,
    l: LPARAM,
    e: *mut BOOL,
) -> HRESULT {
    key(t, 12, c, w, l, e)
}
unsafe extern "system" fn preserved(
    _t: *mut c_void,
    _c: *mut c_void,
    _g: *const GUID,
    e: *mut BOOL,
) -> HRESULT {
    if e.is_null() {
        return E_INVALIDARG;
    }
    e.write(BOOL(0));
    S_OK
}
unsafe extern "system" fn terminated(t: *mut c_void, cookie: u32, c: *mut c_void) -> HRESULT {
    call(
        t,
        args(14, c, ptr::null_mut(), ptr::null_mut(), cookie as usize, 0),
    )
}
unsafe extern "system" fn enum_attributes(_t: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    let result = super::meltype_tsf_create_display_attribute_enum();
    out.write(result);
    if result.is_null() {
        E_OUTOFMEMORY
    } else {
        S_OK
    }
}
unsafe extern "system" fn attribute(
    _t: *mut c_void,
    g: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    let result = super::meltype_tsf_create_display_attribute(g);
    out.write(result);
    if result.is_null() {
        E_INVALIDARG
    } else {
        S_OK
    }
}
unsafe extern "system" fn changed(t: *mut c_void, g: *const GUID) -> HRESULT {
    if g.is_null() {
        return E_INVALIDARG;
    }
    call(
        t,
        args(
            17,
            g.cast_mut().cast(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
        ),
    )
}
unsafe extern "system" fn edited(
    t: *mut c_void,
    context: *mut c_void,
    cookie: u32,
    record: *mut c_void,
) -> HRESULT {
    call(
        t,
        args(18, context, record, ptr::null_mut(), cookie as usize, 0),
    )
}
static PROCESSOR: ITfTextInputProcessorEx_Vtbl = ITfTextInputProcessorEx_Vtbl {
    base__: ITfTextInputProcessor_Vtbl {
        base__: UNKNOWN,
        Activate: activate,
        Deactivate: deactivate,
    },
    ActivateEx: activate_ex,
};
static THREAD: ITfThreadMgrEventSink_Vtbl = ITfThreadMgrEventSink_Vtbl {
    base__: UNKNOWN,
    OnInitDocumentMgr: ignore,
    OnUninitDocumentMgr: ignore,
    OnSetFocus: focus,
    OnPushContext: ignore,
    OnPopContext: ignore,
};
static KEY: ITfKeyEventSink_Vtbl = ITfKeyEventSink_Vtbl {
    base__: UNKNOWN,
    OnSetFocus: foreground,
    OnTestKeyDown: test_down,
    OnTestKeyUp: test_up,
    OnKeyDown: down,
    OnKeyUp: up,
    OnPreservedKey: preserved,
};
static COMPOSITION: ITfCompositionSink_Vtbl = ITfCompositionSink_Vtbl {
    base__: UNKNOWN,
    OnCompositionTerminated: terminated,
};
static DISPLAY: ITfDisplayAttributeProvider_Vtbl = ITfDisplayAttributeProvider_Vtbl {
    base__: UNKNOWN,
    EnumDisplayAttributeInfo: enum_attributes,
    GetDisplayAttributeInfo: attribute,
};
static COMPARTMENT: ITfCompartmentEventSink_Vtbl = ITfCompartmentEventSink_Vtbl {
    base__: UNKNOWN,
    OnChange: changed,
};
static EDIT: ITfTextEditSink_Vtbl = ITfTextEditSink_Vtbl {
    base__: UNKNOWN,
    OnEndEdit: edited,
};
/// Return one owned native IUnknown/ITfTextInputProcessorEx reference. Destroy is
/// called once after the final Release; construction failure never consumes owner.
/// # Safety
/// Context and callbacks remain callable until final Release on the creating TSF
/// thread. Dispatch is synchronous, cannot unwind, and may release its caller's
/// reference. It must never retain the borrowed Call pointer.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_com_create(
    context: *mut c_void,
    dispatch: Option<Dispatch>,
    destroy: Option<Destroy>,
) -> *mut c_void {
    let (Some(dispatch), Some(destroy)) = (dispatch, destroy) else {
        return ptr::null_mut();
    };
    let tables = [
        (&PROCESSOR as *const ITfTextInputProcessorEx_Vtbl).cast(),
        (&THREAD as *const ITfThreadMgrEventSink_Vtbl).cast(),
        (&KEY as *const ITfKeyEventSink_Vtbl).cast(),
        (&COMPOSITION as *const ITfCompositionSink_Vtbl).cast(),
        (&DISPLAY as *const ITfDisplayAttributeProvider_Vtbl).cast(),
        (&COMPARTMENT as *const ITfCompartmentEventSink_Vtbl).cast(),
        (&EDIT as *const ITfTextEditSink_Vtbl).cast(),
    ];
    let service = Box::into_raw(Box::new(Service {
        slots: tables.map(|vtable| Slot {
            vtable,
            owner: ptr::null_mut(),
        }),
        refs: AtomicU32::new(1),
        thread: thread::current().id(),
        context,
        dispatch,
        destroy,
    }));
    for slot in &mut (*service).slots {
        slot.owner = service;
    }
    interface(service, &IUnknown::IID)
}
/// Return a borrowed interface without changing the reference count.
/// # Safety
/// Root is a live interface from service_com_create; iid is readable. The caller
/// keeps the service alive while using this borrowed interface.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_service_com_interface(
    root: *mut c_void,
    iid: *const GUID,
) -> *mut c_void {
    if root.is_null() || iid.is_null() {
        return ptr::null_mut();
    }
    interface(owner(root), &*iid)
}
