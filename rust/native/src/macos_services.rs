// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Native Services receiver; weak owner/target, no borrow spans a host callback.
use std::ffi::{c_char, c_void, CStr};
pub type Host = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, i32) -> i32;
fn message(code: i32) -> &'static CStr {
    match code {
        1 => c"Meltype-FLASHの入力モードはメインスレッドで変更してください。",
        2 => c"Meltype-FLASHの入力モードの指定が不正です。",
        3 => c"入力ソースでMeltype-FLASHを選択してください。",
        _ => c"Meltype-FLASHの入力欄を取得できませんでした。",
    }
}
#[no_mangle]
pub extern "C" fn meltype_macos_services_error(code: i32) -> *const c_char {
    message(code).as_ptr()
}
/// Create a +1 native NSObject service receiver, holding owner weakly.
/// host ops: 0 selected-source query, 1 apply action to target, 2 idle selection.
/// # Safety
/// owner is a live NSObject-compatible Swift/ObjC object; host lives through the
/// receiver's lifetime, never throws/unwinds, and does not use the borrowed
/// owner/target after returning. Caller consumes the returned +1 reference.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_mode_services_create(
    owner: *mut c_void,
    host: Option<Host>,
) -> *mut c_void {
    if owner.is_null() || host.is_none() {
        return std::ptr::null_mut();
    }
    #[cfg(target_os = "macos")]
    {
        native::create(owner, host.unwrap())
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::ptr::null_mut()
    }
}
/// Main-thread activation/replacement or identity-checked deactivation.
/// # Safety
/// receiver and target are live NSObject-compatible borrowed objects.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_mode_services_target(
    receiver: *mut c_void,
    target: *mut c_void,
    active: i32,
) -> i32 {
    if receiver.is_null() || target.is_null() {
        return 0;
    }
    #[cfg(target_os = "macos")]
    {
        native::target(receiver, target, active != 0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = active;
        0
    }
}
/// Same receiver path as changeInputMode:userData:error:. Result 0=success,
/// 1=wrong thread, 2=invalid identifier, 3=other input source, 4=unavailable.
/// # Safety
/// receiver is live; data is readable for length UTF-16 units (null for empty).
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_mode_services_dispatch(
    receiver: *mut c_void,
    data: *const u16,
    length: u32,
    present: i32,
) -> i32 {
    #[cfg(target_os = "macos")]
    {
        native::dispatch(receiver, data, length, present != 0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (receiver, data, length, present);
        4
    }
}
/// Read-only Carbon input-source identity comparison; never selects a source.
/// # Safety
/// expected is readable for length UTF-16 units (null for empty).
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_input_source_matches(
    expected: *const u16,
    length: u32,
) -> i32 {
    #[cfg(target_os = "macos")]
    {
        native::source_matches(expected, length)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (expected, length);
        0
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{
        define_class, msg_send,
        rc::{Retained, Weak},
        runtime::{AnyObject, NSObject, NSObjectProtocol},
        AnyThread, DefinedClass, MainThreadMarker,
    };
    use objc2_foundation::NSString;
    use std::cell::RefCell;
    struct Ivars {
        owner: Weak<AnyObject>,
        target: RefCell<Option<Weak<AnyObject>>>,
        host: Host,
    }
    define_class!(
        // NSObject has no subclassing requirements. Fields are touched only on
        // main thread; off-main methods only construct the original error.
        #[unsafe(super = NSObject)]
        #[ivars = Ivars]
        struct ModeServices;
        unsafe impl NSObjectProtocol for ModeServices {}
        impl ModeServices {
            #[unsafe(method(changeInputMode:userData:error:))]
            unsafe fn change(&self, _pasteboard: *mut AnyObject, user_data: *mut NSString, error: *mut *mut NSString) {
                let code = if MainThreadMarker::new().is_none() { 1 } else {
                    self.deliver(user_data.as_ref().map(|value| value.to_string()))
                };
                if code != 0 && !error.is_null() {
                    *error = Retained::autorelease_ptr(NSString::from_str(message(code).to_str().unwrap()));
                }
            }
        }
    );
    impl ModeServices {
        fn deliver(&self, identifier: Option<String>) -> i32 {
            let action = match identifier.as_deref() {
                Some("automatic") => 0,
                Some("direct") => 1,
                Some("code") => 2,
                Some("cycle") => 3,
                _ => return 2,
            };
            let Some(owner) = self.ivars().owner.load() else {
                return 4;
            };
            let owner_ptr = Retained::as_ptr(&owner).cast_mut().cast();
            unsafe {
                if (self.ivars().host)(owner_ptr, std::ptr::null_mut(), 0, action) <= 0 {
                    return 3;
                }
                // Copy the weak slot first. No RefCell borrow spans a weak load,
                // retained-object drop, or reentrant application callback.
                let weak = self.ivars().target.borrow().clone();
                let target = weak.as_ref().and_then(Weak::load);
                let (target_ptr, operation) = match target.as_ref() {
                    Some(target) => (Retained::as_ptr(target).cast_mut().cast(), 1),
                    None => (std::ptr::null_mut(), 2),
                };
                if (self.ivars().host)(owner_ptr, target_ptr, operation, action) > 0 {
                    0
                } else {
                    4
                }
            }
        }
    }
    unsafe fn receiver(pointer: *mut c_void) -> Option<Retained<ModeServices>> {
        Retained::retain(pointer.cast::<AnyObject>())?
            .downcast::<ModeServices>()
            .ok()
    }
    pub unsafe fn create(owner: *mut c_void, host: Host) -> *mut c_void {
        if MainThreadMarker::new().is_none() {
            return std::ptr::null_mut();
        }
        let Some(owner) = Retained::retain(owner.cast::<AnyObject>()) else {
            return std::ptr::null_mut();
        };
        let this = ModeServices::alloc().set_ivars(Ivars {
            owner: Weak::new(&owner),
            target: RefCell::new(None),
            host,
        });
        let result: Retained<ModeServices> = msg_send![super(this), init];
        Retained::into_raw(result).cast()
    }
    pub unsafe fn target(pointer: *mut c_void, target: *mut c_void, active: bool) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 0;
        }
        let Some(receiver) = receiver(pointer) else {
            return 0;
        };
        let Some(target) = Retained::retain(target.cast::<AnyObject>()) else {
            return 0;
        };
        if active {
            *receiver.ivars().target.borrow_mut() = Some(Weak::new(&target));
        } else {
            let weak = receiver.ivars().target.borrow().clone();
            let current = weak.as_ref().and_then(Weak::load);
            if current
                .as_ref()
                .is_some_and(|current| Retained::as_ptr(current) == Retained::as_ptr(&target))
            {
                *receiver.ivars().target.borrow_mut() = None;
            }
        }
        1
    }
    unsafe fn text(data: *const u16, length: u32) -> Option<String> {
        if data.is_null() && length != 0 {
            return None;
        }
        let data = if length == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(data, length as usize)
        };
        String::from_utf16(data).ok()
    }
    pub unsafe fn dispatch(
        pointer: *mut c_void,
        data: *const u16,
        length: u32,
        present: bool,
    ) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 1;
        }
        let identifier = if present {
            match text(data, length) {
                Some(value) => Some(value),
                None => return 2,
            }
        } else {
            None
        };
        let Some(receiver) = receiver(pointer) else {
            return 4;
        };
        receiver.deliver(identifier)
    }
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
        fn TISGetInputSourceProperty(source: *const c_void, key: *const c_void) -> *const c_void;
        static kTISPropertyInputSourceID: *const c_void;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(value: *const c_void);
    }
    struct Source(*const c_void);
    impl Drop for Source {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) }
        }
    }
    pub unsafe fn source_matches(expected: *const u16, length: u32) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 0;
        }
        let Some(expected) = text(expected, length) else {
            return 0;
        };
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return 0;
        }
        let source = Source(source);
        let value = TISGetInputSourceProperty(source.0, kTISPropertyInputSourceID);
        let Some(value) = value.cast::<NSString>().as_ref() else {
            return 0;
        };
        i32::from(value.to_string() == expected)
    }
}
