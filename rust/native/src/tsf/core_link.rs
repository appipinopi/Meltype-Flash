// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! Core availability, hello publication and synchronous request mediation.
use std::{
    ffi::{c_char, c_void, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice,
};
use windows::{
    core::{IUnknown, Interface},
    Win32::System::{SystemInformation::GetTickCount64, Threading::GetCurrentThreadId},
};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StateCallbacks {
    pub(super) owner: *mut c_void,
    pub(super) lifetime: *mut c_void,
    pub(super) epoch: *const u64,
    pub(super) read: unsafe extern "C" fn(*mut c_void, *mut u64) -> u32,
    pub(super) update: unsafe extern "C" fn(*mut c_void, u32, u64),
    pub(super) hello: unsafe extern "C" fn(*mut c_void),
    pub(super) request: unsafe extern "C" fn(*mut c_void, *const c_char, *mut u32) -> i32,
    pub(super) clock: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
}
unsafe fn snapshot(c: &StateCallbacks, epoch: u64) -> Option<(u32, u64)> {
    if c.epoch.read() != epoch {
        return None;
    }
    let mut last = 0;
    let flags = (c.read)(c.owner, &mut last);
    (c.epoch.read() == epoch && flags & 4 == 0).then_some((flags, last))
}
unsafe fn now(c: &StateCallbacks) -> u64 {
    c.clock
        .map_or_else(|| GetTickCount64(), |clock| clock(c.owner))
}
/// # Safety
/// TSF thread, borrowed owner. epoch/code/storage survive while lifetime is held;
/// callbacks never throw. read flags disabled=1,active=2,ended=4 plus last hello.
/// hello performs the refresh. Optional clock is a controlled-host time source.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_core_ready(callbacks: *const StateCallbacks) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(c) = callbacks.as_ref().copied() else {
            return 0;
        };
        if c.epoch.is_null() || c.lifetime.is_null() {
            return 0;
        }
        let epoch = c.epoch.read();
        let _owner = IUnknown::from_raw_borrowed(&c.lifetime).cloned().unwrap();
        let Some((flags, _)) = snapshot(&c, epoch) else {
            return 0;
        };
        if flags & 1 != 0 {
            return 0;
        }
        if flags & 2 != 0 {
            return 1;
        }
        let time = now(&c);
        let Some((flags, last)) = snapshot(&c, epoch) else {
            return 0;
        };
        if flags & 1 != 0 {
            return 0;
        }
        if flags & 2 == 0 && time.wrapping_sub(last) > 2000 {
            (c.hello)(c.owner);
        }
        i32::from(snapshot(&c, epoch).is_some_and(|(flags, _)| flags & 3 == 2))
    }))
    .unwrap_or(0)
}
/// # Safety
/// Same lifetime contract as core_ready; process is readable UTF16 (null only
/// for length zero). request returns active=1,detection=2,code=4,full-width Space=8.
/// update 0 stores last hello time,1 publishes normalized flags/clears direct word,
/// 2 logs active(value). Clock null uses Windows uptime. Reply applies only to
/// the same service epoch; ended/disabled services are inert.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_core_hello(
    process: *const u16,
    length: u32,
    callbacks: *const StateCallbacks,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(c) = callbacks.as_ref().copied() else {
            return 0;
        };
        if c.epoch.is_null() || c.lifetime.is_null() || (process.is_null() && length != 0) {
            return 0;
        }
        let epoch = c.epoch.read();
        let _owner = IUnknown::from_raw_borrowed(&c.lifetime).cloned().unwrap();
        if snapshot(&c, epoch).is_none_or(|(flags, _)| flags & 1 != 0) {
            return 0;
        }
        let time = now(&c);
        if snapshot(&c, epoch).is_none_or(|(flags, _)| flags & 1 != 0) {
            return 0;
        }
        (c.update)(c.owner, 0, time);
        if snapshot(&c, epoch).is_none_or(|(flags, _)| flags & 1 != 0) {
            return 0;
        }
        let process = if length == 0 {
            &[]
        } else {
            slice::from_raw_parts(process, length as usize)
        };
        let request = CString::new(
            serde_json::json!({"op":"hello","process":String::from_utf16_lossy(process)})
                .to_string(),
        )
        .unwrap();
        let mut flags = 0;
        let ok = (c.request)(c.owner, request.as_ptr(), &mut flags) != 0;
        if snapshot(&c, epoch).is_none_or(|(flags, _)| flags & 1 != 0) {
            return 0;
        }
        let flags = if ok && flags & 1 != 0 { flags & 15 } else { 0 };
        (c.update)(c.owner, 1, flags as u64);
        if snapshot(&c, epoch).is_none() {
            return 0;
        }
        (c.update)(c.owner, 2, (flags & 1) as u64);
        (flags & 1) as i32
    }))
    .unwrap_or(0)
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RequestCallbacks {
    pub(super) owner: *mut c_void,
    pub(super) lifetime: *mut c_void,
    pub(super) epoch: *const u64,
    pub(super) read: unsafe extern "C" fn(*mut c_void) -> u32,
    pub(super) transact: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        u32,
        u32,
        i32,
        *mut *const u8,
        *mut u32,
    ) -> i32,
    pub(super) parse: unsafe extern "C" fn(*mut c_void, *const u8, u32) -> i32,
    pub(super) notice: unsafe extern "C" fn(*mut c_void),
}
unsafe fn request_current(c: &RequestCallbacks, epoch: u64) -> Option<u32> {
    if c.epoch.read() != epoch {
        return None;
    }
    let flags = (c.read)(c.owner);
    (c.epoch.read() == epoch && flags & 4 == 0).then_some(flags)
}
/// # Safety
/// TSF thread, readable input bytes (null only for zero length). Owner/code/epoch
/// and callback storage survive while lifetime is held; callbacks never throw.
/// read bits ended=4,own process=8. transact forwards timeout/optional unchanged
/// and returns reply bytes borrowed through parse. parse copies into caller's
/// value and returns success. notice logs parse failure. Stale replies are skipped.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_core_request(
    input: *const u8,
    length: u32,
    timeout: u32,
    optional: i32,
    callbacks: *const RequestCallbacks,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(c) = callbacks.as_ref().copied() else {
            return 0;
        };
        if c.epoch.is_null() || c.lifetime.is_null() || (input.is_null() && length != 0) {
            return 0;
        }
        let epoch = c.epoch.read();
        let _owner = IUnknown::from_raw_borrowed(&c.lifetime).cloned().unwrap();
        let Some(flags) = request_current(&c, epoch) else {
            return 0;
        };
        let input = if length == 0 {
            &[]
        } else {
            slice::from_raw_parts(input, length as usize)
        };
        let mut request = input.to_vec();
        if flags & 8 != 0 && request.last() == Some(&b'}') {
            request.pop();
            request.extend_from_slice(format!(",\"tid\":{}}}", GetCurrentThreadId()).as_bytes());
        }
        let Ok(size) = u32::try_from(request.len()) else {
            return 0;
        };
        let mut reply = ptr::null();
        let mut count = 0;
        if (c.transact)(
            c.owner,
            request.as_ptr(),
            size,
            timeout,
            optional,
            &mut reply,
            &mut count,
        ) == 0
        {
            return 0;
        }
        if request_current(&c, epoch).is_none() || (reply.is_null() && count != 0) {
            return 0;
        }
        if (c.parse)(c.owner, reply, count) == 0 {
            if request_current(&c, epoch).is_some() {
                (c.notice)(c.owner);
            }
            return 0;
        }
        i32::from(request_current(&c, epoch).is_some())
    }))
    .unwrap_or(0)
}
