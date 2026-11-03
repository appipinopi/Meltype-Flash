// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Standalone TSF DLL exports: no C++ loader, owner or logging bridge.
use std::{
    ffi::c_void,
    io::Write,
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
    ptr,
    sync::{
        atomic::{AtomicU32, Ordering},
        OnceLock,
    },
};
use windows::{
    core::{GUID, HRESULT, PCWSTR},
    Win32::{
        Foundation::*,
        System::{
            LibraryLoader::*, SystemInformation::GetLocalTime, Threading::GetCurrentThreadId,
        },
    },
};
static LOCKS: AtomicU32 = AtomicU32::new(0);
pub(super) fn retain_module() {
    LOCKS.fetch_add(1, Ordering::Relaxed);
}
pub(super) fn release_module() {
    LOCKS.fetch_sub(1, Ordering::Release);
}
unsafe extern "C" fn lock(value: i32) {
    if value != 0 {
        retain_module();
    } else {
        release_module();
    }
}
fn module_path() -> Option<PathBuf> {
    unsafe {
        let mut module = HMODULE::default();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(DllGetClassObject as *const () as *const u16),
            &mut module,
        )
        .ok()?;
        let mut path = vec![0; 32768];
        let count = GetModuleFileNameW(module, &mut path) as usize;
        if count == 0 || count >= path.len() {
            return None;
        }
        use std::os::windows::ffi::OsStringExt;
        Some(std::ffi::OsString::from_wide(&path[..count]).into())
    }
}
fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("MELTYPE_TIP_LOG").is_some_and(|v| v.to_string_lossy().starts_with('1'))
            || std::env::var_os("LOCALAPPDATA")
                .is_some_and(|p| PathBuf::from(p).join("Meltype-flash/tip-log").exists())
    })
}
fn log(message: String) {
    if !enabled() {
        return;
    }
    let Some(path) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .share_mode(3)
        .open(PathBuf::from(path).join("Meltype-flash/tip.log"))
    else {
        return;
    };
    unsafe {
        let now = GetLocalTime();
        let mut exe = vec![0; 32768];
        let count = GetModuleFileNameW(None, &mut exe) as usize;
        let exe = String::from_utf16_lossy(&exe[..count.min(exe.len())]);
        let name = exe.rsplit(['\\', '/']).next().unwrap_or("");
        let line = format!(
            "{:02}:{:02}:{:02}.{:03} [{} {}] {}\r\n",
            now.wHour,
            now.wMinute,
            now.wSecond,
            now.wMilliseconds,
            name,
            GetCurrentThreadId(),
            message
        );
        let _ = file.write_all(line.as_bytes());
    }
}
unsafe extern "C" fn mode(_: *mut c_void, op: u32, value: i32) {
    let _ = std::panic::catch_unwind(|| {
        let text = match op {
            0 => format!(
                "meltype-flash.exe: {}",
                if value != 0 {
                    "つながりました"
                } else {
                    "使えません (動いていない・モードが Meltype-FLASH IME でない)"
                }
            ),
            1 => "応答を読めません".into(),
            2 => format!(
                "入力モードの表示: {}",
                if value != 0 {
                    "英数 (コードの行)"
                } else {
                    "日本語"
                }
            ),
            3 => format!(
                "コードの行: {}",
                if value != 0 {
                    "日本語にしました"
                } else {
                    "英数に戻しました"
                }
            ),
            4 => format!(
                "入力欄の種類を読むところ: {}",
                match value {
                    3 => "Windows とアプリ",
                    1 => "Windows",
                    2 => "アプリ",
                    _ => "無い",
                }
            ),
            5 => format!(
                "入力欄の種類を読めないので、パスワード欄として扱います ({})",
                match value {
                    1 => "ITfInputScope",
                    2 => "GetInputScopes",
                    3 => "VARIANT",
                    _ => "Rust API",
                }
            ),
            6 => {
                let reasons = [
                    "",
                    "英数",
                    "Ctrl・Alt・Windows キーを押している",
                    "この入力欄では使わない",
                    "英語の単語の続き",
                    "meltype-flash.exe が使えない",
                    "パスワードの入力欄",
                ];
                if !(1..reasons.len() as i32).contains(&value) {
                    return;
                }
                format!("キーを通します: {}", reasons[value as usize])
            }
            7 => "アプリが確定した打ちかけの文字を、変換中に戻しました".into(),
            10 => "キャレットが変換中の文字の外に出たので確定します".into(),
            11 => format!("入力モード: {}", if value != 0 { "日本語" } else { "英数" }),
            _ => return,
        };
        log(text);
    });
}
unsafe extern "C" fn key(_: *mut c_void, op: u32, value: i32) {
    let _ = std::panic::catch_unwind(|| match op {
        0 => log("パスワードの入力欄なので何もしません".into()),
        1 => log("英数文字を直接入力できませんでした。キーをアプリへ送り直します".into()),
        2 => log(format!(
            "編集セッションを始められません: 0x{:08X}",
            value as u32
        )),
        3 => log("meltype-flash.exe から応答が来ないので、打った文字をそのまま入れました".into()),
        4 => mode(ptr::null_mut(), 1, 0),
        5 => mode(ptr::null_mut(), 2, value),
        6 => mode(ptr::null_mut(), 4, value),
        7 => mode(ptr::null_mut(), 5, value),
        8 => log("同じキーの送り直しが続くので、やめます".into()),
        9 => log(format!("キーをアプリに送り直せません: {}", value as u32)),
        _ => {}
    });
}
unsafe extern "C" fn startup(_: *mut c_void, op: u32, value: i32) {
    let _ = std::panic::catch_unwind(|| {
        if op == 0 {
            log(format!(
                "Activate ({}){}",
                String::from_utf16_lossy(&super::process::name()),
                if value != 0 {
                    " 無効: セキュリティ保護された画面"
                } else {
                    ""
                }
            ));
        } else {
            mode(ptr::null_mut(), op - 1, value);
        }
    });
}
unsafe extern "C" fn shutdown(_: *mut c_void, op: u32, value: i32) {
    let _ = std::panic::catch_unwind(|| {
        if op == 0 {
            log(format!(
                "手放した変換中の文字を終えられませんでした: 0x{:08X}",
                value as u32
            ));
        } else if op == 1 {
            mode(ptr::null_mut(), 1, 0);
        }
    });
}
unsafe extern "C" fn create(iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_INVALIDARG;
    }
    out.write(ptr::null_mut());
    let Some(path) =
        module_path().and_then(|path| path.parent().map(|p| p.join("meltype-tip-worker.exe")))
    else {
        return E_FAIL;
    };
    let Ok(worker) = std::ffi::CString::new(path.to_string_lossy().as_bytes()) else {
        return E_FAIL;
    };
    super::service_state::object::meltype_tsf_service_object_create(
        worker.as_ptr(),
        iid,
        out,
        ptr::null_mut(),
        Some(mode),
        Some(key),
        Some(startup),
        Some(shutdown),
        Some(lock),
    )
}
/// # Safety
/// Standard COM DLL entry: readable CLSID/IID and writable output interface.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    std::panic::catch_unwind(|| {
        super::factory::meltype_tsf_get_class_object(clsid, iid, out, Some(create), Some(lock))
    })
    .unwrap_or(E_FAIL)
}
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LOCKS.load(Ordering::Acquire) == 0
        && crate::tip_transport::threads_idle()
        && unsafe { super::candidates::release_classes() }
    {
        S_OK
    } else {
        S_FALSE
    }
}
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    std::panic::catch_unwind(|| {
        let Some(path) = module_path() else {
            return E_FAIL;
        };
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<_> = path.as_os_str().encode_wide().collect();
        unsafe {
            super::registration::meltype_tsf_register_server(path.as_ptr(), path.len() as u32)
        }
    })
    .unwrap_or(E_FAIL)
}
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    super::registration::meltype_tsf_unregister_server()
}
