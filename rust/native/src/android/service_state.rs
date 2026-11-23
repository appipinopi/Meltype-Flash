// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Android service state is native. Never hold this mutex across JNI/editor
//! callbacks; a field change invalidates every previously captured generation.
use super::*;
use crate::android_editor::Outcome;
use std::sync::atomic::{AtomicU64, Ordering};
static EPOCH: AtomicU64 = AtomicU64::new(1);
struct State {
    epoch: u64,
    flags: i32,
    shifted: bool,
    outcome: Outcome,
}
impl State {
    fn new(flags: i32) -> Self {
        Self {
            epoch: EPOCH.fetch_add(1, Ordering::Relaxed),
            flags,
            shifted: false,
            outcome: Outcome::failure(false),
        }
    }
}
fn states() -> &'static Mutex<HashMap<jlong, State>> {
    static STATES: OnceLock<Mutex<HashMap<jlong, State>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}
pub(super) fn create(handle: jlong) {
    if handle != 0 {
        states()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(handle, State::new(0));
    }
}
pub(super) fn destroy(handle: jlong) {
    states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&handle);
}
pub(super) fn start(handle: jlong, flags: i32) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        *state = State::new(flags);
    }
}
pub(super) fn epoch(handle: jlong) -> Option<u64> {
    states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&handle)
        .map(|s| s.epoch)
}
pub(super) fn restart(handle: jlong) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        state.epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
        state.shifted = false;
    }
}
pub(super) fn pending(handle: jlong) -> String {
    states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&handle)
        .map_or_else(String::new, |s| s.outcome.composing_text.clone())
}
pub(super) fn publish(handle: jlong, expected: Option<u64>, outcome: &Outcome) -> Outcome {
    let mut states = states().lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = states.get_mut(&handle) else {
        return Outcome::failure(outcome.consumed);
    };
    if Some(state.epoch) == expected {
        state.outcome = outcome.clone();
    }
    let mut result = state.outcome.clone();
    // Render the newer field, but retain this call's consumption result. A
    // completed old-field edit must not replay its key into the new field.
    if Some(state.epoch) != expected {
        result.consumed = outcome.consumed;
    }
    result
}
pub(super) fn clear(handle: jlong) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        state.epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
        state.outcome = Outcome::failure(false);
    }
}
pub(super) fn direct(handle: jlong, value: bool) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        state.flags = (state.flags & !1) | i32::from(value);
    }
}
pub(super) fn private(handle: jlong, value: bool) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        let flags = (state.flags & !2) | (i32::from(value) << 1);
        if flags != state.flags {
            state.epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
            state.outcome = Outcome::failure(false);
            state.flags = flags;
        }
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editorFlags(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    flags(handle)
}
pub(super) fn flags(handle: jlong) -> i32 {
    states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&handle)
        .map_or(0, |s| {
            s.flags | if s.outcome.composing { 4 } else { 0 } | if s.shifted { 8 } else { 0 }
        })
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_composingText(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    let text = pending(handle);
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editorView(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    let mut view = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&handle)
        .map_or_else(|| Outcome::failure(false), |s| s.outcome.clone());
    view.consumed = false;
    env.new_string(serde_json::to_string(&view).unwrap_or_default())
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_toggleShift(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if let Some(state) = states()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&handle)
    {
        state.shifted = !state.shifted;
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_takeShiftedScalar(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    scalar: jint,
) -> jint {
    let mut states = states().lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = states.get_mut(&handle) else {
        return scalar;
    };
    let shifted = std::mem::replace(&mut state.shifted, false);
    if shifted && (b'a' as i32..=b'z' as i32).contains(&scalar) {
        scalar - 32
    } else {
        scalar
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_toggleDirect(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    toggle_direct(handle);
}
pub(super) fn toggle_direct(handle: jlong) {
    let direct = {
        let mut states = states().lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = states.get_mut(&handle) else {
            return;
        };
        state.flags ^= 1;
        state.flags & 1
    };
    meltype_set_direct(handle as *mut c_void, direct);
}
