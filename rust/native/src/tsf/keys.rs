// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! TSF key acceptance and keyboard translation. Owner callbacks perform core
//! requests and expose live state after potentially reentrant OS/core actions.
use super::fields;
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
};
use windows::{
    core::{IUnknown, Interface},
    Win32::{
        System::{SystemInformation::GetTickCount64, Threading::GetCurrentThreadId},
        UI::{Input::KeyboardAndMouse::*, TextServices::ITfContext, WindowsAndMessaging::*},
    },
};
const SHIFT: u32 = 1;
const CONTROL: u32 = 2;
const ALT: u32 = 4;
const WIN: u32 = 8;
/// Pure classification: modifier=1, Japanese toggle=2, Alt-grave toggle=4,
/// composition Ctrl shortcut=8. mods uses Shift/Ctrl/Alt/Win bits 1/2/4/8.
#[no_mangle]
pub extern "C" fn meltype_tsf_key_flags(vk: u32, message: u64, mods: u32) -> u32 {
    let modifier = matches!(vk,0x10..=0x12|0xa0..=0xa5|0x5b|0x5c|0x14);
    let japanese = matches!(vk, 0x19 | 0xf3 | 0xf4);
    let grave = vk == 0xc0 || ((message >> 16) & 0xff) == 0x29 && message & (1 << 24) == 0;
    let toggle = grave
        && (mods & ALT != 0 || message & (1 << 29) != 0)
        && mods & (SHIFT | CONTROL | WIN) == 0;
    let shortcut = mods & CONTROL != 0
        && mods & (ALT | WIN) == 0
        && matches!(vk, 0x48 | 0x55 | 0x49 | 0x4b | 0x4c | 0x4f | 0x50);
    u32::from(modifier)
        | u32::from(japanese) << 1
        | u32::from(toggle) << 2
        | u32::from(shortcut) << 3
}
/// # Safety
/// Call on the TSF input thread whose keyboard state is being inspected.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_key_down(vk: i32) -> i32 {
    i32::from(GetKeyState(vk) < 0)
}
unsafe fn modifiers() -> u32 {
    u32::from(GetKeyState(VK_SHIFT.0 as i32) < 0)
        | u32::from(GetKeyState(VK_CONTROL.0 as i32) < 0) << 1
        | u32::from(GetKeyState(VK_MENU.0 as i32) < 0) << 2
        | u32::from(GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0) << 3
}
/// # Safety
/// Call on the TSF thread whose keyboard state is being inspected.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_key_modifiers() -> u32 {
    modifiers()
}
/// Return a single UTF-16 character or 0. Ctrl is removed for translation; Alt
/// state is cleared only when Alt is not held. Flag 4 preserves dead-key state.
/// # Safety
/// Call on the TSF input thread using its current keyboard layout.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_key_char(vk: u32, message: u64) -> u32 {
    let mut state = [0u8; 256];
    if GetKeyboardState(&mut state).is_err() {
        return 0;
    }
    for key in [VK_CONTROL, VK_LCONTROL, VK_RCONTROL] {
        state[key.0 as usize] = 0;
    }
    if GetKeyState(VK_MENU.0 as i32) >= 0 {
        for key in [VK_MENU, VK_LMENU, VK_RMENU] {
            state[key.0 as usize] = 0;
        }
    }
    let mut text = [0u16; 8];
    if ToUnicode(
        vk,
        ((message >> 16) & 0xff) as u32,
        Some(&state),
        &mut text,
        4,
    ) == 1
    {
        u32::from(text[0])
    } else {
        0
    }
}
#[repr(C)]
#[derive(Default)]
pub struct Snapshot {
    pub(super) composition: *mut c_void,
    pub(super) context: *mut c_void,
    pub(super) disabled: i32,
    pub(super) open: i32,
    pub(super) detection: i32,
    pub(super) english: i32,
    pub(super) stale_tries: u32,
    pub(super) stale_vk: u32,
    pub(super) stale_time: u64,
    pub(super) full_width_space: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callbacks {
    pub(super) owner: *mut c_void,
    pub(super) lifetime: *mut c_void,
    pub(super) read: Option<unsafe extern "C" fn(*mut c_void, *mut Snapshot)>,
    pub(super) update: Option<unsafe extern "C" fn(*mut c_void, u32, u64, u64)>,
    pub(super) resume: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub(super) commit_now: Option<unsafe extern "C" fn(*mut c_void, i32) -> i32>,
    pub(super) commit_async: Option<unsafe extern "C" fn(*mut c_void, i32)>,
    pub(super) abandon: Option<unsafe extern "C" fn(*mut c_void)>,
    pub(super) ready: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    pub(super) secret: Option<unsafe extern "C" fn(*mut c_void, u32, *mut c_void, i32) -> i32>,
    pub(super) pass: Option<unsafe extern "C" fn(*mut c_void, u32)>,
}
/// Decide whether Meltype receives a key, with the upstream key/focus/privacy
/// order. update operations: 0 reset per-key secret flags; 1 secret while
/// composing; 2 clear direct-English word; 3 store stale tries/vk packed in
/// high/low 32 bits of a and timestamp b; 4 secretChecked=a. pass reasons:
/// 0 clear, 1 English mode, 2 modifiers, 3 disabled field, 4 continuing English,
/// 5 unavailable core, 6 password, 7 menu. secret callback owns the short-lived cache.
/// # Safety
/// On the TSF input thread, borrowed context and callback/lifetime interfaces.
/// Output points to a writable u16. Callbacks/state/code survive until return
/// and never throw; lifetime is retained across all callbacks.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_would_eat(
    context: *mut c_void,
    vk: u32,
    message: u64,
    test: i32,
    callbacks: *const Callbacks,
    out: *mut u16,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    out.write(0);
    i32::from(
        catch_unwind(AssertUnwindSafe(|| {
            let Some(c) = callbacks.as_ref().copied() else {
                return false;
            };
            let (
                Some(read),
                Some(update),
                Some(resume),
                Some(now),
                Some(later),
                Some(abandon),
                Some(ready),
                Some(secret),
                Some(pass),
            ) = (
                c.read,
                c.update,
                c.resume,
                c.commit_now,
                c.commit_async,
                c.abandon,
                c.ready,
                c.secret,
                c.pass,
            )
            else {
                return false;
            };
            let Some(_owner) = IUnknown::from_raw_borrowed(&c.lifetime).cloned() else {
                return false;
            };
            let _context = ITfContext::from_raw_borrowed(&context).cloned();
            let state = || {
                let mut s = Snapshot::default();
                read(c.owner, &mut s);
                s
            };
            let reject = |reason| {
                pass(c.owner, reason);
                false
            };
            update(c.owner, 0, 0, 0);
            if state().disabled != 0 || vk >= 256 || meltype_tsf_key_flags(vk, 0, 0) & 1 != 0 {
                return false;
            }
            resume(c.owner, context);
            if vk == 0xe7 || vk == 0xe5 {
                if vk == 0xe7 && !state().composition.is_null() && now(c.owner, 1) == 0 {
                    later(c.owner, 1);
                }
                return false;
            }
            let s = state();
            if !s.composition.is_null() && s.context != context && now(c.owner, 1) == 0 {
                let s = state();
                let time = GetTickCount64();
                let counted =
                    test == 0 && vk == s.stale_vk && time.wrapping_sub(s.stale_time) < 500;
                let tries = if counted {
                    s.stale_tries
                } else {
                    s.stale_tries.saturating_add(1)
                };
                if !counted {
                    update(c.owner, 3, u64::from(tries) << 32 | u64::from(vk), time);
                }
                if tries <= 1 {
                    if !counted {
                        later(c.owner, 1);
                    }
                    return false;
                }
                abandon(c.owner);
            }
            if !state().composition.is_null() {
                if fields::meltype_tsf_context_disabled(context) != 0 {
                    if now(c.owner, 1) != 0 {
                        return false;
                    }
                    update(c.owner, 1, 0, 0);
                    return true;
                }
                let mods = modifiers();
                if mods & (CONTROL | ALT | WIN) != 0
                    && meltype_tsf_key_flags(vk, 0, mods) & 8 == 0
                    && now(c.owner, 0) != 0
                {
                    return false;
                }
                return true;
            }
            let mods = modifiers();
            let flags = meltype_tsf_key_flags(vk, message, mods);
            if flags & 4 != 0 {
                return true;
            }
            if flags & 2 != 0 {
                return mods & (CONTROL | ALT) == 0;
            }
            let s = state();
            if s.open == 0 && s.detection == 0 && vk != 0x1c {
                return reject(1);
            }
            if mods & (CONTROL | ALT | WIN) != 0 {
                return reject(2);
            }
            // #819/#827: menu access keys belong to the application's menu.
            let mut info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            if GetGUIThreadInfo(GetCurrentThreadId(), &mut info).is_ok() && menu_mode(info.flags.0)
            {
                return reject(7);
            }
            if fields::meltype_tsf_context_disabled(context) != 0 {
                return reject(3);
            }
            let ch = meltype_tsf_key_char(vk, 0) as u16;
            out.write(ch);
            let s = state();
            if s.open == 0 && s.english != 0 {
                if matches!(ch,0x30..=0x39|0x41..=0x5a|0x61..=0x7a|0x40|0x5f) {
                    return reject(4);
                }
                update(c.owner, 2, 0, 0);
            }
            if !(vk == 0x1c
                || (0x21..=0x7e).contains(&ch)
                || vk == 0x20
                    && (modifiers() & SHIFT != 0 || s.open != 0 && s.full_width_space != 0))
            {
                return false;
            }
            if ready(c.owner) == 0 {
                return reject(5);
            }
            let secret = secret(c.owner, vk, context, i32::from(test != 0));
            if secret == 1 {
                return reject(6);
            }
            update(c.owner, 4, u64::from(secret == 0), 0);
            pass(c.owner, 0);
            true
        }))
        .unwrap_or(false),
    )
}

fn menu_mode(flags: u32) -> bool {
    flags & (GUI_INMENUMODE.0 | GUI_POPUPMENUMODE.0 | GUI_SYSTEMMENUMODE.0) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_819_menu_flags_leave_caret_flags_alone() {
        assert!(!menu_mode(0));
        assert!(!menu_mode(GUI_CARETBLINKING.0));
        for flags in [GUI_INMENUMODE, GUI_POPUPMENUMODE, GUI_SYSTEMMENUMODE] {
            assert!(menu_mode(flags.0 | GUI_CARETBLINKING.0));
        }
    }
}
