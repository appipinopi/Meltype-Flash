// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Original input menu entries/checkmarks and SDK action selectors.
use std::ffi::c_void;
/// Return +1 NSMenu for caller ARC. Main thread only.
/// # Safety
/// target is a live NSObject-compatible controller borrowed through this call.
/// It must implement the original action selectors. Menu target ownership follows
/// NSMenuItem SDK semantics; caller consumes the returned +1 reference once.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_menu(
    target: *mut c_void,
    direct: i32,
    code: i32,
    indicator: i32,
) -> *mut c_void {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    #[cfg(target_os = "macos")]
    {
        native::create(target, direct != 0, code != 0, indicator != 0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (direct, code, indicator);
        std::ptr::null_mut()
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{
        msg_send,
        rc::Retained,
        runtime::{AnyClass, AnyObject, Sel},
        sel, MainThreadMarker,
    };
    use objc2_foundation::NSString;
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
    pub unsafe fn create(
        target: *mut c_void,
        direct: bool,
        code: bool,
        indicator: bool,
    ) -> *mut c_void {
        if MainThreadMarker::new().is_none() {
            return std::ptr::null_mut();
        }
        let Some(target) = Retained::retain(target.cast::<AnyObject>()) else {
            return std::ptr::null_mut();
        };
        let Some(menu_class) = AnyClass::get(c"NSMenu") else {
            return std::ptr::null_mut();
        };
        let Some(item_class) = AnyClass::get(c"NSMenuItem") else {
            return std::ptr::null_mut();
        };
        let menu: Retained<AnyObject> = msg_send![menu_class, new];
        let empty = NSString::from_str("");
        let entries: [(&str, Sel, bool, bool); 7] = [
            (
                "日本語・英語の自動判定",
                sel!(selectAutomatic:),
                !code && !direct,
                true,
            ),
            ("英数の直接入力", sel!(selectDirect:), direct, true),
            (
                "コード入力 (コメント・文字列は日本語)",
                sel!(selectCode:),
                code && !direct,
                true,
            ),
            (
                "入力モードをミニウィンドウで表示",
                sel!(toggleModeIndicator:),
                indicator,
                true,
            ),
            (
                "macOSのユーザー辞書を取り込む…",
                sel!(importUserDictionary:),
                false,
                true,
            ),
            (
                "Meltype-FLASH のデータフォルダを開く (設定・ユーザー辞書)",
                sel!(openDataFolder:),
                false,
                false,
            ),
            (
                "不具合の報告・提案… (Mac 版はプレビュー版です)",
                sel!(openReport:),
                false,
                false,
            ),
        ];
        for (index, (title, action, selected, set_target)) in entries.iter().enumerate() {
            if index == 3 {
                let separator: Retained<AnyObject> = msg_send![item_class, separatorItem];
                let _: () = msg_send![&menu,addItem:&*separator];
            }
            let title = NSString::from_str(title);
            let item: Retained<AnyObject> =
                msg_send![&menu,addItemWithTitle:&*title,action:*action,keyEquivalent:&*empty];
            if *set_target {
                let _: () = msg_send![&item,setTarget:&*target];
            }
            // Original import/report/folder items retain their SDK default state.
            if index < 4 {
                let _: () = msg_send![&item,setState:if *selected{1isize}else{0isize}];
            }
        }
        Retained::into_raw(menu).cast()
    }
}
