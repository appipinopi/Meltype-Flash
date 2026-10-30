// SPDX-License-Identifier: GPL-3.0-or-later
//! Registration-free buffered input. The low-level callback only classifies and
//! queues; conversion/UI Automation never blocks the system hook callback.
use meltype_core::{settings::Settings, storage::Store, Session};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
use windows::Win32::{System::Com::*, UI::Accessibility::*};
use windows_sys::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::Gdi::{
            ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO,
            MONITOR_DEFAULTTONEAREST,
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
const INJECTED_TAG: usize = 0x4d52504b;
const DELIVERY_END_TAG: usize = 0x4d52504c;
// Windows wakes this wait for keyboard, mouse and focus callbacks immediately.
// The timeout only bounds how long an idle hook takes to notice shutdown.
unsafe fn wait_for_hook_activity(timeout_ms: u32) -> WAIT_EVENT {
    MsgWaitForMultipleObjectsEx(
        0,
        std::ptr::null(),
        timeout_ms,
        QS_ALLINPUT,
        MWMO_INPUTAVAILABLE,
    )
}
#[derive(Clone, Copy)]
struct Key {
    vk: u32,
    ch: i32,
    mods: i32,
    target: usize,
    generation: u64,
    selection: Option<(usize, bool, u64)>,
}
enum Event {
    Key(Key),
    Reset,
    Settings(Box<Settings>),
}
struct Shared {
    active: AtomicBool,
    permitted: AtomicBool,
    composing: AtomicBool,
    ready: AtomicBool,
    exit: AtomicBool,
    generation: AtomicU64,
    modifiers: AtomicU32,
    delivery_ack: AtomicU64,
    allow_injected: AtomicBool,
    remote_control: AtomicBool,
    // Sending never waits for the foreign UI Automation provider. An accepted
    // key stays in FIFO order; capacity exhaustion must not invalidate it.
    queue: Sender<Event>,
    shortcuts: Mutex<Vec<meltype_core::shortcuts::ToggleShortcut>>,
    candidates: Mutex<Vec<CandidateHit>>,
    #[cfg(test)]
    test_pid: u32,
}
static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();
fn accepts_injected(flags: u32, tag: usize, allowed: bool) -> bool {
    tag != INJECTED_TAG && (flags & LLKHF_INJECTED == 0 || allowed)
}
fn remote_process(name: &str) -> bool {
    ["remoting_desktop.exe", "parsecd.exe"]
        .iter()
        .any(|expected| name.eq_ignore_ascii_case(expected))
}
unsafe fn remote_control_active() -> bool {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == INVALID_HANDLE_VALUE {
        return false;
    }
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut more = Process32FirstW(snapshot, &mut entry) != 0;
    let mut active = false;
    while more {
        let length = entry
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(entry.szExeFile.len());
        if remote_process(&String::from_utf16_lossy(&entry.szExeFile[..length])) {
            active = true;
            break;
        }
        more = Process32NextW(snapshot, &mut entry) != 0;
    }
    CloseHandle(snapshot);
    active
}
const INJECTING: u32 = 1 << 8;
const MODIFIER_KEYS: [u16; 8] = [
    VK_LSHIFT,
    VK_RSHIFT,
    VK_LCONTROL,
    VK_RCONTROL,
    VK_LMENU,
    VK_RMENU,
    VK_LWIN,
    VK_RWIN,
];
fn held_keys(mask: u32) -> Vec<u16> {
    MODIFIER_KEYS
        .into_iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, vk)| vk)
        .collect()
}
fn modifier_bit(vk: u32, scan: u32, extended: bool) -> Option<u32> {
    let vk = match vk as u16 {
        VK_SHIFT => {
            if scan == 0x36 {
                VK_RSHIFT
            } else {
                VK_LSHIFT
            }
        }
        VK_CONTROL => {
            if extended {
                VK_RCONTROL
            } else {
                VK_LCONTROL
            }
        }
        VK_MENU => {
            if extended {
                VK_RMENU
            } else {
                VK_LMENU
            }
        }
        vk => vk,
    };
    MODIFIER_KEYS
        .iter()
        .position(|key| *key == vk)
        .map(|i| 1 << i)
}
fn modifier_flags(mask: u32) -> i32 {
    i32::from(mask & 3 != 0)
        | (i32::from(mask & 12 != 0) << 1)
        | (i32::from(mask & 48 != 0) << 2)
        | (i32::from(mask & 192 != 0) << 3)
}
#[derive(Clone, Copy)]
struct CandidateHit {
    rect: RECT,
    key: Key,
}
thread_local! { static SUPPRESSED: RefCell<[bool;256]> = const { RefCell::new([false;256]) }; }
thread_local! { static TOGGLES: RefCell<meltype_core::shortcuts::ToggleDetector> = RefCell::new(meltype_core::shortcuts::ToggleDetector::default()); }
thread_local! { static MOUSE_SUPPRESSED: RefCell<bool> = const { RefCell::new(false) }; }
fn toggle_shortcuts(settings: &Settings) -> Vec<meltype_core::shortcuts::ToggleShortcut> {
    settings.toggle_shortcuts()
}
fn initial_modifiers() -> u32 {
    unsafe {
        MODIFIER_KEYS
            .into_iter()
            .enumerate()
            .fold(0, |mask, (i, vk)| {
                mask | if GetAsyncKeyState(vk as i32) < 0 {
                    1 << i
                } else {
                    0
                }
            })
    }
}
fn captures(vk: u32, ch: i32, composing: bool) -> bool {
    let _ = composing;
    (0x20..=0x7e).contains(&ch) || matches!(vk, 8|9|13|27|0x1c|0x21..=0x28|0x2e|0x75..=0x79)
}
unsafe extern "system" fn hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    let consumed = std::panic::catch_unwind(|| {
        let Some(shared) = SHARED.get() else {
            return false;
        };
        if code < 0 || data == 0 {
            return false;
        }
        let event = &*(data as *const KBDLLHOOKSTRUCT);
        #[cfg(test)]
        let fixture = {
            let mut pid = 0;
            GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
            shared.test_pid != 0 && shared.test_pid == pid && event.dwExtraInfo == 0x4d525054
        };
        #[cfg(not(test))]
        let fixture = false;
        if event.dwExtraInfo == DELIVERY_END_TAG {
            shared.delivery_ack.fetch_add(1, Ordering::SeqCst);
            return false;
        }
        if !accepts_injected(
            event.flags,
            event.dwExtraInfo,
            fixture
                || shared.allow_injected.load(Ordering::Acquire)
                || shared.remote_control.load(Ordering::Acquire),
        ) || event.vkCode >= 256
        {
            return false;
        }
        let vk = event.vkCode;
        if let Some(bit) = modifier_bit(vk, event.scanCode, event.flags & LLKHF_EXTENDED != 0) {
            let previous = if matches!(message as u32, WM_KEYUP | WM_SYSKEYUP) {
                TOGGLES.with(|detector| {
                    detector.borrow_mut().release(
                        vk as i32,
                        event.scanCode,
                        event.flags & LLKHF_EXTENDED != 0,
                    )
                });
                shared.modifiers.fetch_and(!bit, Ordering::SeqCst)
            } else if matches!(message as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
                shared.modifiers.fetch_or(bit, Ordering::SeqCst)
            } else {
                return false;
            };
            // The delivery loop restores the latest physical state before it
            // atomically re-enables normal modifier delivery.
            return previous & INJECTING != 0;
        }
        if matches!(message as u32, WM_KEYUP | WM_SYSKEYUP) {
            let toggle = TOGGLES.with(|detector| {
                detector.borrow_mut().release(
                    vk as i32,
                    event.scanCode,
                    event.flags & LLKHF_EXTENDED != 0,
                )
            });
            let suppressed = SUPPRESSED
                .with(|keys| std::mem::replace(&mut keys.borrow_mut()[vk as usize], false));
            return toggle || suppressed;
        }
        if !matches!(message as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
            return false;
        }
        let mods = modifier_flags(shared.modifiers.load(Ordering::SeqCst));
        let unmodified = if event.scanCode == 0x29 && matches!(vk, 0x19 | 0xf3 | 0xf4) {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
            MapVirtualKeyExW(
                event.scanCode,
                MAPVK_VSC_TO_VK_EX,
                GetKeyboardLayout(thread),
            ) as i32
        } else {
            vk as i32
        };
        let toggle = TOGGLES.with(|detector| {
            let keys = shared.shortcuts.try_lock().ok();
            let bindings = keys.as_deref().map(Vec::as_slice).unwrap_or_default();
            detector.borrow_mut().press(
                vk as i32,
                event.scanCode,
                event.flags & LLKHF_EXTENDED != 0,
                unmodified,
                mods,
                if shared.permitted.load(Ordering::Acquire) {
                    bindings
                } else {
                    &[]
                },
            )
        });
        if toggle.consume {
            if toggle.toggle {
                shared.active.fetch_xor(true, Ordering::SeqCst);
                shared.generation.fetch_add(1, Ordering::SeqCst);
                let _ = shared.queue.send(Event::Reset);
            }
            return true;
        }
        if !shared.active.load(Ordering::Acquire)
            || !shared.permitted.load(Ordering::Acquire)
            || !shared.ready.load(Ordering::Acquire)
        {
            return false;
        }
        if matches!(vk,0x10..=0x12|0x5b..=0x5c|0xa0..=0xa5) {
            return false;
        }
        if mods & 14 != 0 && !shared.composing.load(Ordering::Acquire) {
            return false;
        }
        let target = GetForegroundWindow();
        if target.is_null() {
            return false;
        }
        let thread = GetWindowThreadProcessId(target, std::ptr::null_mut());
        let mut state = [0u8; 256];
        GetKeyboardState(state.as_mut_ptr());
        state[VK_CONTROL as usize] = if mods & 2 != 0 { 0x80 } else { 0 };
        state[VK_MENU as usize] = if mods & 4 != 0 { 0x80 } else { 0 };
        state[VK_SHIFT as usize] = if mods & 1 != 0 { 0x80 } else { 0 };
        state[vk as usize] = 0x80;
        let mut text = [0u16; 8];
        let count = ToUnicodeEx(
            vk,
            event.scanCode,
            state.as_ptr(),
            text.as_mut_ptr(),
            text.len() as i32,
            4,
            GetKeyboardLayout(thread),
        );
        let ch = if count == 1 { text[0] as i32 } else { 0 };
        if mods & 14 == 0 && !captures(vk, ch, shared.composing.load(Ordering::Acquire)) {
            return false;
        }
        let key = Key {
            vk,
            ch,
            mods,
            target: target as usize,
            generation: shared.generation.load(Ordering::Acquire),
            selection: None,
        };
        if shared.queue.send(Event::Key(key)).is_err() {
            return false;
        }
        if ch >= 0x20 {
            shared.composing.store(true, Ordering::Release);
        }
        SUPPRESSED.with(|keys| keys.borrow_mut()[vk as usize] = true);
        true
    })
    .unwrap_or(false);
    if consumed {
        1
    } else {
        CallNextHookEx(std::ptr::null_mut(), code, message, data)
    }
}
fn virtual_key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: INJECTED_TAG,
            },
        },
    }
}
fn unicode(text: &str) -> Vec<INPUT> {
    text.encode_utf16()
        .flat_map(|unit| {
            [false, true].map(|up| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE | if up { KEYEVENTF_KEYUP } else { 0 },
                        time: 0,
                        dwExtraInfo: INJECTED_TAG,
                    },
                },
            })
        })
        .collect()
}
unsafe fn send(inputs: &[INPUT], target: usize) -> bool {
    !inputs.is_empty()
        && GetForegroundWindow() as usize == target
        && SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        ) == inputs.len() as u32
}
// Replay against the modifier state at capture time, even when the user has
// released Ctrl/Shift before the asynchronous provider returns. Commit Unicode
// with modifiers temporarily released, then restore the physical state.
fn delivery_inputs(mut commits: Vec<INPUT>, passed: Option<Key>, held: &[u16]) -> Vec<INPUT> {
    if commits.is_empty() && passed.is_none() {
        return commits;
    }
    let mut inputs: Vec<_> = held.iter().map(|vk| virtual_key(*vk, true)).collect();
    inputs.append(&mut commits);
    if let Some(key) = passed {
        if key.mods & 14 == 0 && key.ch >= 0x20 {
            if let Some(ch) = char::from_u32(key.ch as u32) {
                inputs.extend(unicode(&ch.to_string()));
            }
        } else {
            let chord: Vec<_> = [
                (1, VK_LSHIFT),
                (2, VK_LCONTROL),
                (4, VK_LMENU),
                (8, VK_LWIN),
            ]
            .into_iter()
            .filter(|(bit, _)| key.mods & bit != 0)
            .map(|(_, vk)| vk)
            .collect();
            inputs.extend(chord.iter().map(|vk| virtual_key(*vk, false)));
            inputs.extend([
                virtual_key(key.vk as u16, false),
                virtual_key(key.vk as u16, true),
            ]);
            inputs.extend(chord.iter().rev().map(|vk| virtual_key(*vk, true)));
        }
    }
    inputs.extend(held.iter().map(|vk| virtual_key(*vk, false)));
    inputs
}
fn modifier_changes(old: u32, new: u32) -> Vec<INPUT> {
    MODIFIER_KEYS
        .into_iter()
        .enumerate()
        .filter_map(|(i, vk)| {
            let bit = 1 << i;
            ((old ^ new) & bit != 0).then(|| virtual_key(vk, new & bit == 0))
        })
        .collect()
}
unsafe fn send_acknowledged(mut inputs: Vec<INPUT>, target: Option<usize>, state: &Shared) -> bool {
    let Some(last) = inputs.last_mut() else {
        return true;
    };
    last.Anonymous.ki.dwExtraInfo = DELIVERY_END_TAG;
    if target.is_some_and(|target| GetForegroundWindow() as usize != target) {
        return false;
    }
    let previous = state.delivery_ack.load(Ordering::SeqCst);
    if SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    ) != inputs.len() as u32
    {
        return false;
    }
    // SendInput returns after insertion, before the hook has processed every
    // event. Keep physical modifier edges suppressed until the final edge has
    // crossed the hook; otherwise a queued restoration can outlive key-up.
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while state.delivery_ack.load(Ordering::SeqCst) == previous {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    true
}
unsafe fn deliver(commits: Vec<INPUT>, passed: Option<Key>, target: usize, state: &Shared) -> bool {
    if commits.is_empty() && passed.is_none() {
        return true;
    }
    let mut restored = state.modifiers.fetch_or(INJECTING, Ordering::SeqCst) & !INJECTING;
    let inputs = delivery_inputs(commits, passed, &held_keys(restored));
    let mut success = send_acknowledged(inputs, Some(target), state);
    loop {
        // A physical release can arrive while SendInput is sending Unicode.
        // Reconcile it before unlocking; a CAS prevents losing the final edge.
        let physical = state.modifiers.load(Ordering::SeqCst) & !INJECTING;
        let changes = modifier_changes(restored, physical);
        if !changes.is_empty() {
            success &= send_acknowledged(changes, None, state);
        }
        restored = physical;
        if state
            .modifiers
            .compare_exchange(
                physical | INJECTING,
                physical,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            break;
        }
    }
    success
}
fn input_step(session: &mut Session, key: Key) -> meltype_core::session::SessionResult {
    if let Some((index, prediction, expected)) = key.selection {
        let current = session.result(false);
        let candidate = current.view.as_ref().and_then(|view| {
            if prediction {
                view.predictions.get(index)
            } else {
                view.candidates.get(index)
            }
        });
        if candidate.is_none_or(|text| candidate_id(text) != expected) {
            return current;
        }
        if prediction {
            session.select_prediction(index)
        } else {
            session.select_composing(index)
        }
    } else {
        let mut result = session.key(key.vk as i32, key.ch, key.mods);
        // #19: 選択範囲の再変換を既存IMEへ渡すと、相手のIME状態で文字が消える。
        // 未確定文字への変換はCoreに任せ、未対応の再変換要求は消費する。
        if key.vk == 0x1c {
            result.consumed = true;
        }
        result
    }
}
fn candidate_id(text: &str) -> u64 {
    text.as_bytes().iter().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100000001b3)
    })
}
fn display_excerpt(text: &str) -> String {
    // Only the preview is bounded. The Session retains the complete input.
    let mut chars = text.chars();
    let preview: String = chars.by_ref().take(128).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}
fn popup_bounds(point: POINT, work: RECT, candidate_count: usize) -> RECT {
    let width = 520.min((work.right - work.left).max(1));
    let height = (88 + candidate_count.min(7) as i32 * 24).min((work.bottom - work.top).max(1));
    let left = point
        .x
        .clamp(work.left, (work.right - width).max(work.left));
    let top = point
        .y
        .clamp(work.top, (work.bottom - height).max(work.top));
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}
unsafe fn application_name(window: HWND) -> String {
    let mut pid = 0;
    GetWindowThreadProcessId(window, &mut pid);
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return String::new();
    }
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    let ok = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) != 0;
    CloseHandle(process);
    if !ok {
        return String::new();
    }
    let path = String::from_utf16_lossy(&path[..length as usize]);
    path.rsplit('\\').next().unwrap_or("").to_owned()
}
fn application_policy(session: &mut Session, process: &str) -> bool {
    session.set_process(process);
    // 変換ボックス方式は確定前のライブ変換を使わない。
    let _ = session
        .settings
        .set("LiveConversion", serde_json::json!(false));
    session.settings.boolean("Enabled")
}
fn text_input_app(settings: &Settings, process: &str) -> bool {
    let name = process.to_lowercase();
    !name.is_empty()
        && settings
            .text("TextInputApps")
            .split([',', ';', '\n'])
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry.eq_ignore_ascii_case(&name))
}
unsafe fn writable(automation: &IUIAutomation, force_text: bool) -> Option<IUIAutomationElement> {
    let element = automation.GetFocusedElement().ok()?;
    let value_readonly = element
        .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
        .ok()
        .and_then(|value| value.CurrentIsReadOnly().ok())
        .map(|v| v.as_bool());
    let text_readonly = || {
        element
            .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
            .ok()
            .and_then(|pattern| pattern.DocumentRange().ok())
            .and_then(|range| range.GetAttributeValue(UIA_IsReadOnlyAttributeId).ok())
            .and_then(read_only_bool)
    };
    let readonly = value_readonly.or_else(text_readonly);
    if !element.CurrentIsPassword().is_ok_and(|v| !v.as_bool())
        || !element.CurrentIsEnabled().is_ok_and(|v| v.as_bool())
        || readonly == Some(true)
    {
        return None;
    }
    let kind = element.CurrentControlType().ok()?;
    let focusable = element
        .CurrentIsKeyboardFocusable()
        .is_ok_and(|v| v.as_bool());
    let editable = readonly == Some(false)
        && (kind == UIA_EditControlTypeId || kind == UIA_DocumentControlTypeId || focusable)
        || focusable && (force_text || ia2_editable(&element));
    editable.then_some(element)
}

// Chromium/Electron can expose an empty contenteditable as a Group with no
// Value/TextPattern. Match the original host's IAccessible2 EDITABLE fallback.
// Never read the accessible value or document text here.
unsafe fn ia2_editable(element: &IUIAutomationElement) -> bool {
    use windows::core::{IUnknown, Interface, GUID, HRESULT};
    #[repr(C)]
    struct Ia2Vtable {
        unknown: windows::core::IUnknown_Vtbl,
        // IDispatch + IAccessible + IAccessible2 methods before get_states.
        before_states: [usize; 32],
        states: unsafe extern "system" fn(*mut std::ffi::c_void, *mut i32) -> HRESULT,
    }
    let query = || -> windows::core::Result<bool> {
        let legacy = element.GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(
            UIA_LegacyIAccessiblePatternId,
        )?;
        // MSAA READONLY / PROTECTED must override the IA2 fallback.
        if legacy.CurrentState()? & (0x40 | 0x20000000) != 0 {
            return Ok(false);
        }
        let accessible = legacy.GetIAccessible()?;
        let provider: IServiceProvider = accessible.cast()?;
        let service = GUID::from_u128(0x618736e0_3c3d_11cf_810c_00aa00389b71);
        let iid = GUID::from_u128(0xe89f726e_c4f4_4c19_bb19_b647d7fa8478);
        let mut raw = std::ptr::null_mut();
        (provider.vtable().QueryService)(provider.as_raw(), &service, &iid, &mut raw).ok()?;
        if raw.is_null() {
            return Ok(false);
        }
        // Own the queried interface so every return path releases it.
        let ia2 = IUnknown::from_raw(raw);
        let vtable = &**(ia2.as_raw() as *const *const Ia2Vtable);
        let mut states = 0;
        (vtable.states)(ia2.as_raw(), &mut states).ok()?;
        Ok(states & 0x8 != 0)
    };
    query().unwrap_or(false)
}

fn read_only_bool(value: windows::core::VARIANT) -> Option<bool> {
    // UIA's NotSupported/MixedAttribute sentinels must never coerce to false.
    if unsafe { value.as_raw().Anonymous.Anonymous.vt }
        != windows::Win32::System::Variant::VT_BOOL.0
    {
        return None;
    }
    bool::try_from(&value).ok()
}
fn invalidate() {
    if let Some(state) = SHARED.get() {
        state.ready.store(false, Ordering::Release);
        state.generation.fetch_add(1, Ordering::SeqCst);
        let _ = state.queue.send(Event::Reset);
    }
}
unsafe fn focus_targets_foreground(event_window: HWND, foreground: HWND) -> bool {
    !event_window.is_null()
        && !foreground.is_null()
        && GetAncestor(event_window, GA_ROOT) == foreground
}
unsafe extern "system" fn focus_changed(
    _: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
    _: u32,
    event_window: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    // Background applications also emit focus events. Only the foreground
    // document can invalidate its active composition; the worker separately
    // observes foreground changes, including events queued for an old window.
    if focus_targets_foreground(event_window, GetForegroundWindow()) {
        invalidate();
    }
}
unsafe extern "system" fn mouse_hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0
        && message as u32 == WM_LBUTTONUP
        && MOUSE_SUPPRESSED.with(|v| std::mem::replace(&mut *v.borrow_mut(), false))
    {
        return 1;
    }
    if code >= 0 && message as u32 == WM_LBUTTONDOWN && data != 0 {
        if let Some(state) = SHARED.get() {
            let point = (*(data as *const MSLLHOOKSTRUCT)).pt;
            let hit = state.candidates.try_lock().ok().and_then(|hits| {
                hits.iter()
                    .find(|hit| {
                        point.x >= hit.rect.left
                            && point.x < hit.rect.right
                            && point.y >= hit.rect.top
                            && point.y < hit.rect.bottom
                    })
                    .copied()
            });
            if let Some(hit) = hit.filter(|hit| {
                state.active.load(Ordering::Acquire)
                    && state.permitted.load(Ordering::Acquire)
                    && hit.key.generation == state.generation.load(Ordering::Acquire)
                    && GetForegroundWindow() as usize == hit.key.target
            }) {
                if state.queue.send(Event::Key(hit.key)).is_ok() {
                    MOUSE_SUPPRESSED.with(|v| *v.borrow_mut() = true);
                    return 1;
                }
            }
        }
    }
    if code >= 0
        && matches!(
            message as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_MOUSEWHEEL
        )
    {
        invalidate();
    }
    CallNextHookEx(std::ptr::null_mut(), code, message, data)
}
pub struct Keyboard {
    shared: Arc<Shared>,
    worker: Option<std::thread::JoinHandle<()>>,
    last_settings: Option<Settings>,
}
impl Keyboard {
    pub fn enabled(&self) -> bool {
        self.shared.active.load(Ordering::Acquire)
    }
    pub fn activate(&self) {
        if self.shared.permitted.load(Ordering::Acquire) {
            self.shared.active.store(true, Ordering::Release);
            self.shared.generation.fetch_add(1, Ordering::SeqCst);
            let _ = self.shared.queue.send(Event::Reset);
        }
    }
    pub fn configure(&mut self, settings: &Settings) {
        self.shared
            .allow_injected
            .store(settings.boolean("AllowInjectedInput"), Ordering::Release);
        self.shared.permitted.store(
            settings.boolean("Enabled") && settings.text("Mode") == "Keyboard",
            Ordering::Release,
        );
        if self
            .last_settings
            .as_ref()
            .is_some_and(|s| s.0 == settings.0)
        {
            return;
        }
        if let Ok(mut shortcuts) = self.shared.shortcuts.lock() {
            *shortcuts = toggle_shortcuts(settings);
        }
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        if self
            .shared
            .queue
            .send(Event::Settings(Box::new(settings.clone())))
            .is_ok()
        {
            self.last_settings = Some(settings.clone());
        }
    }
    pub fn start(store: Store) -> Result<Self, String> {
        let (queue, receiver) = mpsc::channel::<Event>();
        let shared = Arc::new(Shared {
            active: AtomicBool::new(false),
            permitted: AtomicBool::new(
                store
                    .settings()
                    .map(|s| s.boolean("Enabled") && s.text("Mode") == "Keyboard")
                    .unwrap_or(false),
            ),
            composing: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            exit: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            modifiers: AtomicU32::new(initial_modifiers()),
            delivery_ack: AtomicU64::new(0),
            allow_injected: AtomicBool::new(
                store
                    .settings()
                    .unwrap_or_default()
                    .boolean("AllowInjectedInput"),
            ),
            remote_control: AtomicBool::new(false),
            queue,
            shortcuts: Mutex::new(toggle_shortcuts(&store.settings().unwrap_or_default())),
            candidates: Mutex::new(Vec::new()),
            #[cfg(test)]
            test_pid: std::env::var("MELTYPE_PORTABLE_TEST_PID")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        });
        SHARED
            .set(shared.clone())
            .map_err(|_| "ポータブル入力は既に起動しています")?;
        let state = shared.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || unsafe {
            let init = (|| -> Result<(Session, IUIAutomation), String> {
                CoInitializeEx(None, COINIT_MULTITHREADED)
                    .ok()
                    .map_err(|e| e.to_string())?;
                let automation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| e.to_string())?;
                let mut session = Session::open(store).map_err(|e| e.to_string())?;
                session.can_delete = false;
                session.no_learning = true;
                session
                    .settings
                    .set("LiveConversion", serde_json::json!(false))?;
                // A deliberately bounded key path; real OS conversion belongs to TSF.
                session.display(false);
                Ok((session, automation))
            })();
            let (mut session, automation) = match init {
                Ok(value) => value,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            let hook_state = state.clone();
            let (hook_tx, hook_rx) = mpsc::sync_channel(1);
            let hook_thread = std::thread::spawn(move || {
                let handle = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(hook),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                );
                if handle.is_null() {
                    let _ = hook_tx.send(false);
                    return;
                }
                let mouse = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(mouse_hook),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                );
                let focus = windows_sys::Win32::UI::Accessibility::SetWinEventHook(
                    EVENT_OBJECT_FOCUS,
                    EVENT_OBJECT_FOCUS,
                    std::ptr::null_mut(),
                    Some(focus_changed),
                    0,
                    0,
                    0,
                );
                let _ = hook_tx.send(!mouse.is_null() && !focus.is_null());
                if mouse.is_null() || focus.is_null() {
                    hook_state.exit.store(true, Ordering::Release);
                }
                let mut message = MSG::default();
                while !hook_state.exit.load(Ordering::Acquire) {
                    while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    wait_for_hook_activity(20);
                }
                UnhookWindowsHookEx(handle);
                if !mouse.is_null() {
                    UnhookWindowsHookEx(mouse);
                }
                if !focus.is_null() {
                    windows_sys::Win32::UI::Accessibility::UnhookWinEvent(focus);
                }
            });
            if hook_rx.recv_timeout(Duration::from_secs(5)) != Ok(true) {
                let _ = ready_tx.send(Err("キーボードフックを登録できません".into()));
                state.exit.store(true, Ordering::Release);
                let _ = hook_thread.join();
                return;
            }
            let overlay = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Meltype-flash portable"),
                WS_POPUP | WS_BORDER | 0x80 | 0x0c,
                20,
                40,
                700,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            if overlay.is_null() {
                state.exit.store(true, Ordering::Release);
                let _ = ready_tx.send(Err("候補表示を作成できません".into()));
                let _ = hook_thread.join();
                CoUninitialize();
                return;
            }
            let rows: Vec<_> = (0..8)
                .map(|_| {
                    CreateWindowExW(
                        WS_EX_NOACTIVATE,
                        w!("STATIC"),
                        w!(""),
                        WS_CHILD | 0x80 | 0x0c,
                        0,
                        0,
                        1,
                        24,
                        overlay,
                        std::ptr::null_mut(),
                        GetModuleHandleW(std::ptr::null()),
                        std::ptr::null(),
                    )
                })
                .collect();
            if rows.iter().any(|row| row.is_null()) {
                DestroyWindow(overlay);
                state.exit.store(true, Ordering::Release);
                let _ = ready_tx.send(Err("候補の操作部を作成できません".into()));
                let _ = hook_thread.join();
                CoUninitialize();
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let mut target = 0usize;
            let mut profile_target = usize::MAX;
            let mut application_enabled = true;
            let mut current_application = String::new();
            let mut focused: Option<IUIAutomationElement> = None;
            let mut message = MSG::default();
            // #110/#372: inspect processes outside the hook, as upstream does.
            // remoting_host is only a listener; Parsec remains allowed while running.
            let mut remote_checked = None::<std::time::Instant>;
            while !state.exit.load(Ordering::Acquire) {
                if remote_checked.is_none_or(|at| at.elapsed() >= Duration::from_secs(5)) {
                    state
                        .remote_control
                        .store(remote_control_active(), Ordering::Release);
                    remote_checked = Some(std::time::Instant::now());
                }
                while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if !state.active.load(Ordering::Acquire)
                    || !state.permitted.load(Ordering::Acquire)
                    || GetForegroundWindow() as usize != target
                {
                    session.reset();
                    state.composing.store(false, Ordering::Release);
                    ShowWindow(overlay, SW_HIDE);
                    state.candidates.lock().unwrap().clear();
                }
                let foreground = GetForegroundWindow();
                if profile_target != foreground as usize {
                    session.reset();
                    current_application = application_name(foreground);
                    application_enabled = application_policy(&mut session, &current_application);
                    profile_target = foreground as usize;
                }
                let force_text = text_input_app(&session.settings, &current_application);
                let current = if application_enabled
                    && state.active.load(Ordering::Acquire)
                    && state.permitted.load(Ordering::Acquire)
                {
                    writable(&automation, force_text)
                } else {
                    None
                };
                let same = focused
                    .as_ref()
                    .zip(current.as_ref())
                    .is_some_and(|(a, b)| {
                        automation.CompareElements(a, b).is_ok_and(|v| v.as_bool())
                    });
                if !same {
                    session.reset();
                    ShowWindow(overlay, SW_HIDE);
                    state.candidates.lock().unwrap().clear();
                }
                state.ready.store(current.is_some(), Ordering::Release);
                focused = current;
                let Ok(event) = receiver.recv_timeout(Duration::from_millis(20)) else {
                    continue;
                };
                let key = match event {
                    Event::Settings(settings) => {
                        profile_target = usize::MAX;
                        session.reset();
                        session.settings = *settings;
                        let _ = session
                            .settings
                            .set("LiveConversion", serde_json::json!(false));
                        ShowWindow(overlay, SW_HIDE);
                        state.candidates.lock().unwrap().clear();
                        continue;
                    }
                    Event::Reset => {
                        state.composing.store(false, Ordering::Release);
                        session.reset();
                        ShowWindow(overlay, SW_HIDE);
                        state.candidates.lock().unwrap().clear();
                        continue;
                    }
                    Event::Key(key) => key,
                };
                if key.generation != state.generation.load(Ordering::Acquire)
                    || !state.active.load(Ordering::Acquire)
                    || !state.permitted.load(Ordering::Acquire)
                    || GetForegroundWindow() as usize != key.target
                {
                    continue;
                }
                if target != key.target {
                    session.reset();
                    target = key.target;
                }
                if !focused
                    .as_ref()
                    .zip(writable(&automation, force_text).as_ref())
                    .is_some_and(|(a, b)| {
                        automation.CompareElements(a, b).is_ok_and(|v| v.as_bool())
                    })
                {
                    session.reset();
                    state.active.store(false, Ordering::Release);
                    let _ = send(
                        &[
                            virtual_key(key.vk as u16, false),
                            virtual_key(key.vk as u16, true),
                        ],
                        target,
                    );
                    continue;
                }
                let reply = input_step(&mut session, key);
                if reply.commits.iter().any(|edit| {
                    edit.delete_before != 0
                        || edit.caret_back as usize > edit.text.encode_utf16().count()
                }) {
                    session.reset();
                    state.active.store(false, Ordering::Release);
                    continue;
                }
                if key.generation != state.generation.load(Ordering::Acquire)
                    || !focused
                        .as_ref()
                        .zip(writable(&automation, force_text).as_ref())
                        .is_some_and(|(a, b)| {
                            automation.CompareElements(a, b).is_ok_and(|v| v.as_bool())
                        })
                {
                    session.reset();
                    continue;
                }
                let commits: Vec<INPUT> = reply
                    .commits
                    .iter()
                    .flat_map(|edit| {
                        let mut inputs = unicode(&edit.text);
                        for _ in 0..edit.caret_back {
                            inputs
                                .extend([virtual_key(VK_LEFT, false), virtual_key(VK_LEFT, true)]);
                        }
                        inputs
                    })
                    .collect();
                if !deliver(
                    commits,
                    (!reply.consumed && key.selection.is_none()).then_some(key),
                    target,
                    &state,
                ) {
                    session.reset();
                    state.active.store(false, Ordering::Release);
                    continue;
                }
                let view = reply.view;
                state.composing.store(
                    view.as_ref().is_some_and(|v| !v.text.is_empty()),
                    Ordering::Release,
                );
                if let Some(view) = view.filter(|v| !v.text.is_empty()) {
                    let prediction = !view.converting && view.candidates.len() <= 1;
                    let visible_candidates = if prediction {
                        &view.predictions
                    } else {
                        &view.candidates
                    };
                    let selected = if prediction {
                        view.selected_prediction
                    } else {
                        view.selected_index as i32
                    };
                    let candidates: Vec<_> = visible_candidates
                        .iter()
                        .take(7)
                        .enumerate()
                        .map(|(i, text)| {
                            format!(
                                "{}{} {}",
                                if i as i32 == selected { ">" } else { "" },
                                i + 1,
                                text
                            )
                        })
                        .collect();
                    let display: Vec<u16> = format!(
                        "{}\r\n{}",
                        display_excerpt(view.typed.as_deref().unwrap_or("")),
                        display_excerpt(&view.text),
                    )
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                    SetWindowTextW(overlay, display.as_ptr());
                    // Use the input field, rather than a fixed desktop corner,
                    // when Chromium exposes no Win32 caret window.
                    if let Some(element) = &focused {
                        if let Ok(rect) = element.CurrentBoundingRectangle() {
                            let mut caret = GUITHREADINFO {
                                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                                ..Default::default()
                            };
                            let thread =
                                GetWindowThreadProcessId(target as HWND, std::ptr::null_mut());
                            let mut point = POINT {
                                x: rect.left,
                                y: rect.bottom,
                            };
                            if GetGUIThreadInfo(thread, &mut caret) != 0
                                && !caret.hwndCaret.is_null()
                            {
                                point = POINT {
                                    x: caret.rcCaret.left,
                                    y: caret.rcCaret.bottom,
                                };
                                ClientToScreen(caret.hwndCaret, &mut point);
                            }
                            let mut monitor = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            if GetMonitorInfoW(
                                MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST),
                                &mut monitor,
                            ) == 0
                            {
                                continue;
                            }
                            let bounds =
                                popup_bounds(point, monitor.rcWork, visible_candidates.len());
                            SetWindowPos(
                                overlay,
                                HWND_TOPMOST,
                                bounds.left,
                                bounds.top,
                                bounds.right - bounds.left,
                                bounds.bottom - bounds.top,
                                SWP_NOACTIVATE,
                            );
                        }
                    }
                    let mut hits = state.candidates.lock().unwrap();
                    hits.clear();
                    for (i, row) in rows.iter().take(7).enumerate() {
                        if let Some(text) = candidates.get(i) {
                            let text: Vec<u16> = display_excerpt(text)
                                .encode_utf16()
                                .chain(Some(0))
                                .collect();
                            SetWindowTextW(*row, text.as_ptr());
                            let mut client = RECT::default();
                            GetClientRect(overlay, &mut client);
                            SetWindowPos(
                                *row,
                                std::ptr::null_mut(),
                                4,
                                48 + i as i32 * 24,
                                (client.right - 8).max(1),
                                24,
                                SWP_NOACTIVATE | SWP_NOZORDER,
                            );
                            ShowWindow(*row, SW_SHOWNOACTIVATE);
                            let mut rect = RECT::default();
                            GetWindowRect(*row, &mut rect);
                            hits.push(CandidateHit {
                                rect,
                                key: Key {
                                    selection: Some((
                                        i,
                                        prediction,
                                        candidate_id(&visible_candidates[i]),
                                    )),
                                    vk: 0,
                                    ch: 0,
                                    mods: 0,
                                    target,
                                    generation: state.generation.load(Ordering::Acquire),
                                },
                            });
                        } else {
                            ShowWindow(*row, SW_HIDE);
                        }
                    }
                    let shortcut = session.settings.toggle_shortcuts_label();
                    let footer: Vec<u16> = format!("{shortcut}: 入力切替 / Esc: 取消")
                        .encode_utf16()
                        .chain(Some(0))
                        .collect();
                    SetWindowTextW(rows[7], footer.as_ptr());
                    SetWindowPos(
                        rows[7],
                        std::ptr::null_mut(),
                        4,
                        48 + candidates.len() as i32 * 24,
                        500,
                        24,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                    ShowWindow(rows[7], SW_SHOWNOACTIVATE);
                    ShowWindow(overlay, SW_SHOWNOACTIVATE);
                } else {
                    ShowWindow(overlay, SW_HIDE);
                    state.candidates.lock().unwrap().clear();
                }
            }
            DestroyWindow(overlay);
            let _ = hook_thread.join();
            CoUninitialize();
        });
        match ready_rx.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(())) => Ok(Self {
                shared,
                worker: Some(worker),
                last_settings: None,
            }),
            result => {
                shared.exit.store(true, Ordering::Release);
                Err(format!("ポータブル入力の起動に失敗しました: {result:?}"))
            }
        }
    }
}
impl Drop for Keyboard {
    fn drop(&mut self) {
        self.shared.active.store(false, Ordering::Release);
        self.shared.exit.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            // UI Automation providers can be waiting on this very UI thread.
            // Never join an in-flight foreign call from the application's Drop.
            // The worker owns its data and the disabled hook reads only Arc state;
            // process teardown releases remaining native resources.
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue_110_injected_opt_in_never_recaptures_our_own_output() {
        assert!(accepts_injected(0, 0, false));
        assert!(!accepts_injected(LLKHF_INJECTED, 0, false));
        assert!(accepts_injected(LLKHF_INJECTED, 0, true));
        for allowed in [false, true] {
            assert!(!accepts_injected(LLKHF_INJECTED, INJECTED_TAG, allowed));
            assert!(!accepts_injected(0, INJECTED_TAG, allowed));
        }
        assert!(remote_process("remoting_desktop.exe"));
        assert!(remote_process("ParsecD.exe"));
        assert!(!remote_process("remoting_host.exe"));
        assert!(!remote_process("parsecd-helper.exe"));
    }
    #[test]
    fn hook_wait_wakes_for_new_and_already_observed_messages() {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (wake_tx, wake_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || unsafe {
            let mut message = MSG::default();
            // Create the thread message queue before another thread posts to it.
            PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
            ready_tx
                .send(windows_sys::Win32::System::Threading::GetCurrentThreadId())
                .unwrap();
            let result = wait_for_hook_activity(5000);
            wake_tx.send(result).unwrap();
            assert_eq!(result, WAIT_OBJECT_0);
            assert_ne!(
                PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE),
                0
            );
            // Observing the message must not make the next wait miss it.
            assert_eq!(wait_for_hook_activity(0), WAIT_OBJECT_0);
            assert_ne!(
                PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE),
                0
            );
            assert_eq!(message.message, WM_APP);
            assert_eq!(wait_for_hook_activity(20), WAIT_TIMEOUT);
        });
        let thread_id = ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_ne!(unsafe { PostThreadMessageW(thread_id, WM_APP, 0, 0) }, 0);
        let result = wake_rx.recv_timeout(Duration::from_secs(1));
        worker.join().unwrap();
        assert_eq!(
            result,
            Ok(WAIT_OBJECT_0),
            "input waited for the idle timeout"
        );
    }
    #[test]
    fn issue_223_text_input_override_requires_an_explicit_executable_name() {
        let mut settings = Settings::default();
        settings
            .set(
                "TextInputApps",
                serde_json::json!("et.exe, Code.exe; Everything.exe\n"),
            )
            .unwrap();
        for name in ["et.exe", "code.EXE", "Everything.exe"] {
            assert!(text_input_app(&settings, name));
        }
        for name in ["", "Notepad.exe", "other-code.exe", "Code.exe.old"] {
            assert!(!text_input_app(&settings, name));
        }
    }
    #[test]
    fn issue_855_application_rules_disable_clibor_and_restore_normal_input() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path());
        store
            .change_settings(|settings| {
                settings.set(
                    "AppRules",
                    serde_json::json!([
                        {"Process":"Clibor.exe","Enabled":true,"Profile":"Game"},
                        {"Process":"Code.exe","Enabled":true,"Profile":"Code"}
                    ]),
                )
            })
            .unwrap();
        let mut session = Session::open(store).unwrap();
        assert!(!application_policy(&mut session, "Clibor.exe"));
        assert!(!session.key(0x31, '1' as i32, 0).consumed);
        assert!(application_policy(&mut session, "Code.exe"));
        assert!(session.code_input);
        assert!(!session.key(0, 'a' as i32, 0).consumed);
        assert!(application_policy(&mut session, "Notepad.exe"));
        assert!(!session.code_input);
        assert_eq!(session.key(0, 'a' as i32, 0).view.unwrap().text, "あ");
    }
    #[test]
    fn issue_19_convert_key_never_starts_the_other_ime_reconversion() {
        assert!(captures(0x1c, 0, false));
        let mut session = Session::default();
        let result = input_step(
            &mut session,
            Key {
                vk: 0x1c,
                ch: 0,
                mods: 0,
                target: 0,
                generation: 0,
                selection: None,
            },
        );
        assert!(result.consumed);
        assert!(result.commits.is_empty());
        assert!(result.view.is_none());
    }
    #[test]
    fn popup_stays_on_the_selected_monitor_including_negative_coordinates() {
        let work = RECT {
            left: -1920,
            top: -1080,
            right: 0,
            bottom: 0,
        };
        let bounds = popup_bounds(POINT { x: -100, y: -20 }, work, 7);
        assert_eq!(bounds.right, 0);
        assert_eq!(bounds.bottom, 0);
        assert!(bounds.left >= work.left && bounds.top >= work.top);
        let tiny = RECT {
            left: 10,
            top: 20,
            right: 100,
            bottom: 60,
        };
        let bounds = popup_bounds(POINT { x: 0, y: 0 }, tiny, 20);
        assert_eq!(
            (bounds.left, bounds.top, bounds.right, bounds.bottom),
            (10, 20, 100, 60)
        );
    }
    fn key(vk: u32, ch: i32, mods: i32) -> Key {
        Key {
            vk,
            ch,
            mods,
            target: 1,
            generation: 0,
            selection: None,
        }
    }
    fn session() -> Session {
        let mut session = Session::default();
        session
            .settings
            .set("PredictiveCandidates", serde_json::json!(false))
            .unwrap();
        session
            .settings
            .set("LiveConversion", serde_json::json!(false))
            .unwrap();
        session
    }
    #[test]
    fn queued_input_beyond_old_capacity_retains_order_and_long_composition() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..140 {
            tx.send(Event::Key(key(65, 97, 0))).unwrap();
        }
        tx.send(Event::Key(key(0x79, 0, 0))).unwrap();
        tx.send(Event::Key(key(13, 0, 0))).unwrap();
        drop(tx);
        let mut session = session();
        let mut committed = String::new();
        for event in rx {
            let Event::Key(key) = event else {
                panic!("key event");
            };
            let reply = input_step(&mut session, key);
            if let Some(view) = &reply.view {
                let _ = display_excerpt(&view.text);
            }
            for edit in reply.commits {
                committed.push_str(&edit.text);
            }
        }
        assert_eq!(committed, "a".repeat(140));
        assert_eq!(
            display_excerpt(&"日".repeat(140)),
            format!("{}…", "日".repeat(128))
        );
    }
    #[test]
    fn control_edit_and_passthrough_preserve_composition_and_modifier_snapshot() {
        let mut session = session();
        for ch in "nihongo".chars() {
            input_step(
                &mut session,
                key(ch.to_ascii_uppercase() as u32, ch as i32, 0),
            );
        }
        let edit = input_step(&mut session, key(0x4f, 0, 2)); // Ctrl+O = F10
        assert!(edit.consumed);
        assert_eq!(edit.view.unwrap().text, "nihongo");
        let shortcut = key(0x43, 0, 2);
        let reply = input_step(&mut session, shortcut);
        assert!(!reply.consumed);
        assert_eq!(reply.commits[0].text, "nihongo");
        let inputs = delivery_inputs(unicode("にほんご"), Some(shortcut), &[]);
        let keys: Vec<_> = inputs
            .iter()
            .map(|input| unsafe { (input.Anonymous.ki.wVk, input.Anonymous.ki.dwFlags) })
            .collect();
        assert_eq!(
            &keys[keys.len() - 4..],
            &[
                (VK_LCONTROL, 0),
                (0x43, 0),
                (0x43, KEYEVENTF_KEYUP),
                (VK_LCONTROL, KEYEVENTF_KEYUP)
            ]
        );
        let held = delivery_inputs(unicode("日"), None, &[VK_LSHIFT]);
        unsafe {
            assert_eq!(held[0].Anonymous.ki.dwFlags, KEYEVENTF_KEYUP);
            assert_eq!(held.last().unwrap().Anonymous.ki.wVk, VK_LSHIFT);
            assert_eq!(held.last().unwrap().Anonymous.ki.dwFlags, 0);
        }
    }
    #[test]
    fn candidate_click_uses_the_displayed_core_list() {
        let mut session = session();
        for ch in "hashi".chars() {
            input_step(
                &mut session,
                key(ch.to_ascii_uppercase() as u32, ch as i32, 0),
            );
        }
        input_step(&mut session, key(32, 32, 0));
        let view = session.result(false).view.unwrap();
        let index = view
            .candidates
            .iter()
            .position(|word| word == "箸")
            .expect("candidate");
        let mut click = key(0, 0, 0);
        click.selection = Some((index, false, candidate_id("箸")));
        let mut stale = click;
        stale.selection = Some((index, false, candidate_id("別の候補")));
        let before = session.display(false);
        assert_eq!(input_step(&mut session, stale).view.unwrap().text, before);
        let selected = input_step(&mut session, click);
        assert_eq!(selected.view.unwrap().text, "箸");
        assert_eq!(
            input_step(&mut session, key(13, 0, 0)).commits[0].text,
            "箸"
        );
    }
    #[test]
    fn configured_toggle_replaces_default_and_retains_exact_modifiers() {
        let mut settings = Settings::default();
        assert_eq!(toggle_shortcuts(&settings)[0].key, 0x19);
        settings
            .set(
                "ToggleShortcuts",
                serde_json::json!([{"Key":0x4b,"Modifiers":3},{"Key":0x78,"Modifiers":4}]),
            )
            .unwrap();
        let keys = toggle_shortcuts(&settings);
        assert_eq!(keys.len(), 2);
        assert_eq!((keys[0].key, keys[0].modifiers), (0x4b, 3));
        assert_eq!((keys[1].key, keys[1].modifiers), (0x78, 4));
    }
    #[test]
    fn background_focus_events_cannot_discard_foreground_composition() {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let first = CreateWindowExW(
                0,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                50,
                50,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            let second = CreateWindowExW(
                0,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                50,
                50,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            let child = CreateWindowExW(
                0,
                w!("EDIT"),
                w!(""),
                WS_CHILD,
                0,
                0,
                20,
                20,
                first,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            assert!(!first.is_null() && !second.is_null() && !child.is_null());
            assert!(
                focus_targets_foreground(child, first),
                "a same-document focus change must cancel preedit"
            );
            assert!(
                !focus_targets_foreground(child, second),
                "a queued old-document event must not cancel new input"
            );
            assert!(
                !focus_targets_foreground(second, first),
                "background focus must not erase the active composition"
            );
            assert!(!focus_targets_foreground(std::ptr::null_mut(), first));
            assert!(!focus_targets_foreground(child, std::ptr::null_mut()));
            DestroyWindow(first);
            DestroyWindow(second);
        }
    }
    #[test]
    fn modifier_release_during_unicode_delivery_is_reconciled_before_unlocking() {
        let ctrl = modifier_bit(VK_CONTROL as u32, 0, false).unwrap();
        let right_shift = modifier_bit(VK_SHIFT as u32, 0x36, false).unwrap();
        assert_eq!(modifier_flags(ctrl | right_shift), 3);
        let state = AtomicU32::new(INJECTING | ctrl);
        // A real key-up is swallowed while delivery owns the logical state.
        assert_eq!(
            state.fetch_and(!ctrl, Ordering::SeqCst) & INJECTING,
            INJECTING
        );
        assert!(state
            .compare_exchange(INJECTING | ctrl, ctrl, Ordering::SeqCst, Ordering::SeqCst)
            .is_err());
        let changes = modifier_changes(ctrl, 0);
        assert_eq!(changes.len(), 1);
        unsafe {
            assert_eq!(changes[0].Anonymous.ki.wVk, VK_LCONTROL);
            assert_eq!(changes[0].Anonymous.ki.dwFlags, KEYEVENTF_KEYUP);
        }
        assert!(state
            .compare_exchange(INJECTING, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok());
        assert_eq!(modifier_flags(state.load(Ordering::SeqCst)), 0);
    }
    #[test]
    fn shutdown_disables_capture_without_waiting_for_a_blocked_provider() {
        let (queue, _events) = mpsc::channel();
        let shared = Arc::new(Shared {
            active: AtomicBool::new(true),
            permitted: AtomicBool::new(true),
            composing: AtomicBool::new(false),
            ready: AtomicBool::new(true),
            exit: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            modifiers: AtomicU32::new(0),
            delivery_ack: AtomicU64::new(0),
            allow_injected: AtomicBool::new(false),
            remote_control: AtomicBool::new(false),
            queue,
            shortcuts: Mutex::new(Vec::new()),
            candidates: Mutex::new(Vec::new()),
            test_pid: 0,
        });
        let (release, blocked) = mpsc::channel();
        let (dropped, finished) = mpsc::channel();
        let worker_state = shared.clone();
        let worker = std::thread::spawn(move || {
            blocked.recv().unwrap();
            assert!(worker_state.exit.load(Ordering::Acquire));
            assert!(!worker_state.active.load(Ordering::Acquire));
            dropped.send(()).unwrap();
        });
        let keyboard = Keyboard {
            shared: shared.clone(),
            worker: Some(worker),
            last_settings: None,
        };
        let (shutdown, complete) = mpsc::channel();
        let dropper = std::thread::spawn(move || {
            drop(keyboard);
            shutdown.send(()).unwrap();
        });
        let result = complete.recv_timeout(Duration::from_secs(1));
        // Unblock and join even if the regression returns, so tests leave no thread.
        release.send(()).unwrap();
        dropper.join().unwrap();
        finished.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(result.is_ok(), "shutdown waited for the provider");
        assert!(!shared.active.load(Ordering::Acquire));
    }
    #[test]
    fn text_pattern_read_only_requires_an_explicit_boolean() {
        use windows::core::VARIANT;
        assert_eq!(read_only_bool(VARIANT::from(false)), Some(false));
        assert_eq!(read_only_bool(VARIANT::from(true)), Some(true));
        assert_eq!(read_only_bool(VARIANT::new()), None);
        assert_eq!(read_only_bool(VARIANT::from(0i32)), None);
        assert_eq!(read_only_bool(VARIANT::from("false")), None);
    }
    #[test]
    #[ignore = "interactive Windows acceptance; drive with tools/test-portable-input.py --unit-host"]
    fn external_input_host() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("MELTYPE_RUST_DATA_DIR").expect("isolated test data"),
        );
        let keyboard = Keyboard::start(Store::new(directory.clone())).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(90);
        while !directory.join("stop-host").exists() && std::time::Instant::now() < deadline {
            let ready = directory.join("ready-host");
            if keyboard.shared.active.load(Ordering::Acquire)
                && keyboard.shared.ready.load(Ordering::Acquire)
            {
                std::fs::write(&ready, b"ready").unwrap();
            } else if ready.exists() {
                std::fs::remove_file(&ready).unwrap();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(keyboard);
        assert!(
            directory.join("stop-host").exists(),
            "acceptance driver timed out"
        );
    }
    #[test]
    fn idle_navigation_passes_and_injection_is_tagged_utf16() {
        assert!(captures(13, 0, false));
        assert!(captures(13, 0, true));
        assert!(captures(0x41, 'a' as i32, false));
        assert!(!captures(0x70, 0, true));
        let inputs = unicode("日😀");
        assert_eq!(inputs.len(), 6);
        for input in inputs {
            unsafe {
                assert_eq!(input.Anonymous.ki.dwExtraInfo, INJECTED_TAG);
                assert_eq!(input.Anonymous.ki.wVk, 0);
            }
        }
    }
}
