// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Original modal dictionary import workflow using AppKit/Foundation directly.
use std::ffi::c_void;
pub type Directory = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
/// Present the original file chooser, import, notify and show its result.
/// 0: unavailable/wrong thread; 1: cancelled/no source/no directory; 2: result shown.
/// # Safety
/// context remains live until this synchronous call returns, including nested
/// AppKit event loops. directory never throws/unwinds and returns null or a +1
/// NSString consumed exactly once. It is called only after an accepted source.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_dictionary_dialog(
    context: *mut c_void,
    directory: Option<Directory>,
) -> i32 {
    let Some(directory) = directory else { return 0 };
    #[cfg(target_os = "macos")]
    {
        native::run(context, directory)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (context, directory);
        0
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{
        msg_send,
        rc::Retained,
        runtime::{AnyClass, AnyObject},
        MainThreadMarker,
    };
    use objc2_foundation::NSString;
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
    pub unsafe fn run(context: *mut c_void, directory: Directory) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 0;
        }
        let (Some(panel_class), Some(alert_class), Some(center_class), Some(app_class)) = (
            AnyClass::get(c"NSOpenPanel"),
            AnyClass::get(c"NSAlert"),
            AnyClass::get(c"NSDistributedNotificationCenter"),
            AnyClass::get(c"NSApplication"),
        ) else {
            return 0;
        };
        // #716/#766: an input method is a background app. Activate it before
        // entering the modal chooser so clicks do not immediately dismiss it.
        let application: Retained<AnyObject> = msg_send![app_class, sharedApplication];
        let _: () = msg_send![&application, activateIgnoringOtherApps:true];
        let panel: Retained<AnyObject> = msg_send![panel_class, new];
        let _: () = msg_send![&panel, setAllowsMultipleSelection:false];
        let _: () = msg_send![&panel, setCanChooseDirectories:false];
        let _: () = msg_send![&panel, setLevel:3isize];
        let _: () = msg_send![&panel, makeKeyAndOrderFront:std::ptr::null::<AnyObject>()];
        let response: isize = msg_send![&panel, runModal];
        if response != 1 {
            return 1;
        }
        let source: Option<Retained<AnyObject>> = msg_send![&panel, URL];
        let Some(source) = source else {
            return 1;
        };
        // Resolve after the chooser, matching the original short-circuit guard.
        // Consume the host's owned NSString even when validation rejects it.
        let Some(folder) = Retained::from_raw(directory(context).cast::<AnyObject>()) else {
            return 1;
        };
        let string_class = AnyClass::get(c"NSString").expect("Foundation NSString unavailable");
        let valid: bool = msg_send![&folder, isKindOfClass:string_class];
        if !valid {
            return 1;
        }
        let folder = Retained::cast_unchecked::<NSString>(folder);
        let units: Vec<u16> = folder.to_string().encode_utf16().collect();
        let Ok(length) = u32::try_from(units.len()) else {
            return 1;
        };
        let _: () = msg_send![&application, activateIgnoringOtherApps:true];
        let alert: Retained<AnyObject> = msg_send![alert_class, new];
        let mut failure = std::ptr::null_mut();
        let count = crate::macos_dictionary::meltype_macos_dictionary_import(
            Retained::as_ptr(&source).cast_mut().cast(),
            units.as_ptr(),
            length,
            &mut failure,
        );
        let failure = Retained::from_raw(failure.cast::<AnyObject>());
        if count >= 0 {
            let text = NSString::from_str(&format!("ユーザー辞書に{count}語を追加しました"));
            let _: () = msg_send![&alert, setMessageText:&*text];
            let center: Retained<AnyObject> = msg_send![center_class, defaultCenter];
            let name = NSString::from_str("MeltypeUserDictionaryChanged");
            // Also notify for a successful duplicate import (zero additions).
            let _: () = msg_send![&center,postNotificationName:&*name,object:std::ptr::null::<AnyObject>(),userInfo:std::ptr::null::<AnyObject>(),deliverImmediately:true];
        } else {
            let text = NSString::from_str("ユーザー辞書を取り込めませんでした");
            let _: () = msg_send![&alert, setMessageText:&*text];
            let description: Retained<NSString> = if let Some(error) = failure {
                msg_send![&error, localizedDescription]
            } else {
                NSString::from_str("読みと単語を含むmacOSユーザー辞書のplistを選んでください。")
            };
            let _: () = msg_send![&alert, setInformativeText:&*description];
        }
        let _: isize = msg_send![&alert, runModal];
        2
    }
}
