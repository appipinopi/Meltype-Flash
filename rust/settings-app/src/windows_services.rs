// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Process-scoped hotkeys and notifications for the Windows tray companion.
use eframe::egui;
use std::sync::mpsc::Sender;
use windows_sys::{
    core::GUID,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
            },
            Shell::{
                DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW,
                NIF_GUID, NIF_INFO, NIF_REALTIME, NIIF_INFO, NIM_MODIFY, NOTIFYICONDATAW,
            },
            WindowsAndMessaging::WM_HOTKEY,
        },
    },
};
// Stable Rust-preview identity, distinct from upstream's notification icon.
pub const TRAY_GUID: u128 = 0xac59d28f_b690_4b1f_9732_847d1093cc68;
pub fn tray_guid(directory: &std::path::Path) -> u128 {
    let path = directory
        .canonicalize()
        .unwrap_or_else(|_| directory.to_path_buf());
    let hash = path
        .to_string_lossy()
        .to_lowercase()
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100000001b3)
        });
    (TRAY_GUID & (!0_u128 << 64)) | hash as u128
}
const SUBCLASS: usize = 0x4d454c54;
const HOTKEY_ID: i32 = 0x4d45;
struct Sink {
    sender: Sender<()>,
    ctx: egui::Context,
}
pub struct Hotkey {
    hwnd: HWND,
    sink: Box<Sink>,
    pub name: Option<String>,
}
fn choose(mut register: impl FnMut(u32, u32) -> bool) -> Option<&'static str> {
    [
        (MOD_CONTROL | MOD_ALT, 0x7b, "Ctrl+Alt+F12"),
        (MOD_CONTROL | MOD_ALT, 0x7a, "Ctrl+Alt+F11"),
        (MOD_CONTROL | MOD_ALT | MOD_SHIFT, 0x41, "Ctrl+Shift+Alt+A"),
        (MOD_CONTROL | MOD_ALT, 0x13, "Ctrl+Alt+Pause"),
    ]
    .into_iter()
    .find_map(|(modifiers, key, name)| register(modifiers | MOD_NOREPEAT, key).then_some(name))
}
fn choose_configured(text: &str, mut register: impl FnMut(u32, u32) -> bool) -> Option<String> {
    if let Some(shortcut) = meltype_core::shortcuts::parse_list(text).first() {
        let modifiers = if shortcut.modifiers & 1 != 0 {
            MOD_SHIFT
        } else {
            0
        } | if shortcut.modifiers & 2 != 0 {
            MOD_CONTROL
        } else {
            0
        } | if shortcut.modifiers & 4 != 0 {
            MOD_ALT
        } else {
            0
        } | if shortcut.modifiers & 8 != 0 { 8 } else { 0 };
        if register(modifiers | MOD_NOREPEAT, shortcut.vk as u32) {
            return Some(text.trim().to_owned());
        }
    }
    choose(register).map(str::to_owned)
}
impl Hotkey {
    #[cfg(test)]
    pub fn attach(hwnd: HWND, sender: Sender<()>, ctx: &egui::Context) -> Result<Self, String> {
        Self::attach_configured(hwnd, sender, ctx, "")
    }
    pub fn attach_configured(
        hwnd: HWND,
        sender: Sender<()>,
        ctx: &egui::Context,
        setting: &str,
    ) -> Result<Self, String> {
        let sink = Box::new(Sink {
            sender,
            ctx: ctx.clone(),
        });
        // The box stays at a stable address until the subclass has been removed.
        if unsafe {
            SetWindowSubclass(
                hwnd,
                Some(callback),
                SUBCLASS,
                (&*sink as *const Sink) as usize,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let name = choose_configured(setting, |modifiers, key| unsafe {
            RegisterHotKey(hwnd, HOTKEY_ID, modifiers, key) != 0
        });
        Ok(Self { hwnd, sink, name })
    }
}
impl Drop for Hotkey {
    fn drop(&mut self) {
        unsafe {
            if self.name.is_some() {
                UnregisterHotKey(self.hwnd, HOTKEY_ID);
            }
            RemoveWindowSubclass(self.hwnd, Some(callback), SUBCLASS);
        }
        // Keep the callback data alive through both native cleanup calls.
        let _ = &self.sink;
    }
}
unsafe extern "system" fn callback(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_HOTKEY && wparam == HOTKEY_ID as usize && data != 0 {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let sink = unsafe { &*(data as *const Sink) };
            let _ = sink.sender.send(());
            sink.ctx.request_repaint();
        }));
        return 0;
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}
fn utf16<const N: usize>(text: &str) -> [u16; N] {
    let mut result = [0; N];
    let mut offset = 0;
    for c in text.chars() {
        let mut buffer = [0; 2];
        let units = c.encode_utf16(&mut buffer);
        if offset + units.len() >= N {
            break;
        }
        result[offset..offset + units.len()].copy_from_slice(units);
        offset += units.len();
    }
    result
}
#[cfg(feature = "visual-test")]
pub fn dispatch_test_hotkey(hwnd: HWND) {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd,
            WM_HOTKEY,
            HOTKEY_ID as usize,
            0,
        );
    }
}
fn balloon(hwnd: HWND, guid: u128, title: &str, text: &str) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uFlags: NIF_GUID | NIF_INFO | NIF_REALTIME,
        guidItem: GUID::from_u128(guid),
        szInfoTitle: utf16(title),
        szInfo: utf16(text),
        dwInfoFlags: NIIF_INFO,
        ..unsafe { std::mem::zeroed() }
    }
}
pub fn notify(hwnd: HWND, guid: u128, title: &str, text: &str) -> Result<(), String> {
    if unsafe { Shell_NotifyIconW(NIM_MODIFY, &balloon(hwnd, guid, title, text)) } == 0 {
        return Err("Windowsへ通知を渡せませんでした".into());
    }
    Ok(())
}
/// winit requests redraws with RedrawWindow, which Windows does not deliver to
/// hidden windows. Keep the resident app's menu and maintenance polling alive
/// without showing or activating its settings window.
pub struct HiddenRepaint {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl HiddenRepaint {
    pub fn start(context: &eframe::CreationContext<'_>) -> Result<Self, String> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        use windows_sys::Win32::{
            System::Threading::GetCurrentProcessId,
            UI::WindowsAndMessaging::{
                GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_PAINT,
            },
        };
        let RawWindowHandle::Win32(handle) =
            context.window_handle().map_err(|e| e.to_string())?.as_raw()
        else {
            return Err("Windows settings window handle unavailable".into());
        };
        let hwnd = handle.hwnd.get() as usize;
        let pid = unsafe { GetCurrentProcessId() };
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let worker = std::thread::spawn(move || {
            while !signal.load(Ordering::Acquire) {
                unsafe {
                    let mut owner = 0;
                    GetWindowThreadProcessId(hwnd as _, &mut owner);
                    if owner != pid {
                        break;
                    }
                    if IsWindowVisible(hwnd as _) == 0 {
                        PostMessageW(hwnd as _, WM_PAINT, 0, 0);
                    }
                }
                std::thread::park_timeout(std::time::Duration::from_millis(500));
            }
        });
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for HiddenRepaint {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_pause_key_is_first_and_failure_or_invalid_input_uses_defaults() {
        let mut attempts = Vec::new();
        let selected = choose_configured("Shift+Win+F10", |modifiers, key| {
            attempts.push((modifiers, key));
            true
        });
        assert_eq!(selected.as_deref(), Some("Shift+Win+F10"));
        assert_eq!(attempts, [(MOD_SHIFT | 8 | MOD_NOREPEAT, 0x79)]);
        attempts.clear();
        let selected = choose_configured("Ctrl+Alt+F10", |modifiers, key| {
            attempts.push((modifiers, key));
            attempts.len() == 2
        });
        assert_eq!(selected.as_deref(), Some("Ctrl+Alt+F12"));
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            choose_configured("invalid", |_, _| true).as_deref(),
            Some("Ctrl+Alt+F12")
        );
    }
    #[test]
    fn hotkeys_fall_back_and_never_repeat_or_override_a_failed_registration() {
        let mut attempts = vec![];
        assert_eq!(
            choose(|modifiers, key| {
                attempts.push((modifiers, key));
                attempts.len() == 3
            }),
            Some("Ctrl+Shift+Alt+A")
        );
        assert_eq!(attempts.len(), 3);
        assert!(attempts.iter().all(|(m, _)| m & MOD_NOREPEAT != 0));
        assert_eq!(choose(|_, _| false), None);
    }
    #[test]
    fn notifications_use_the_owned_icon_guid_and_do_not_split_utf16_pairs() {
        let data = balloon(std::ptr::null_mut(), TRAY_GUID, "Meltype-FLASH", "有効");
        assert_eq!(data.uFlags, NIF_GUID | NIF_INFO | NIF_REALTIME);
        assert_eq!(data.guidItem.data1, GUID::from_u128(TRAY_GUID).data1);
        assert_eq!(utf16::<4>("a😀z"), [97, 0xd83d, 0xde00, 0]);
        assert_eq!(utf16::<3>("a😀"), [97, 0, 0]);
        assert_eq!(utf16::<1>("a"), [0]);
        assert_eq!(
            tray_guid(std::path::Path::new("C:/Data")),
            tray_guid(std::path::Path::new("c:/data"))
        );
        assert_ne!(
            tray_guid(std::path::Path::new("C:/Data")),
            tray_guid(std::path::Path::new("C:/Other"))
        );
    }
    #[test]
    fn native_hotkey_window_dispatches_and_removes_the_subclass_before_freeing_data() {
        use windows_sys::{
            core::w,
            Win32::UI::WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, SendMessageW, HWND_MESSAGE,
            },
        };
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let window = Window(unsafe {
            CreateWindowExW(
                0,
                w!("STATIC"),
                w!("Meltype hotkey test"),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        });
        assert!(!window.0.is_null());
        let (sender, receiver) = std::sync::mpsc::channel();
        let hotkey = Hotkey::attach(window.0, sender, &egui::Context::default()).unwrap();
        unsafe {
            SendMessageW(window.0, WM_HOTKEY, HOTKEY_ID as usize, 0);
        }
        assert_eq!(receiver.try_recv(), Ok(()));
        drop(hotkey);
        unsafe {
            SendMessageW(window.0, WM_HOTKEY, HOTKEY_ID as usize, 0);
        }
        assert_eq!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        );
    }
}
