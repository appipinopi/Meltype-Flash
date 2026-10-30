// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use super::*;
use jni::{
    objects::{JClass, JObject, JString, JValue},
    sys::{jboolean, jint, jlong, jstring},
    JNIEnv,
};
mod actions;
mod activity_view;
mod appearance;
mod appearance_ui;
mod bridge;
mod bridge_api;
mod dialogs;
mod documents;
mod editor_jni;
mod flick;
mod input_view;
mod service_events;
mod service_state;
mod settings;
mod settings_ui;
mod settings_view;
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_startInput(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    input_type: jint,
    ime_options: jint,
) -> jint {
    let handle = handle as *mut c_void;
    unsafe {
        meltype_free(meltype_reset(handle).cast());
    }
    meltype_set_can_delete(handle, 0);
    let flags = crate::android_input::field_flags(input_type, ime_options);
    meltype_set_private(handle, (flags >> 1) & 1);
    meltype_set_direct(handle, flags & 1);
    service_state::start(handle as jlong, flags);
    flags
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_isModifierKey(
    _env: JNIEnv,
    _class: JClass,
    key: jint,
) -> jboolean {
    u8::from(crate::android_input::modifier_key(key))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_androidKey<'a>(
    mut env: JNIEnv<'a>,
    class: JClass<'a>,
    handle: jlong,
    key: jint,
    scalar: jint,
    meta: jint,
    before: JString<'a>,
    after: JString<'a>,
) -> jstring {
    if crate::android_input::modifier_key(key) {
        return java_result(&mut env, std::ptr::null_mut());
    }
    Java_moe_meltype_rust_Native_keyContext(
        env,
        class,
        handle,
        crate::android_input::virtual_key(key),
        scalar,
        crate::android_input::modifiers(meta),
        before,
        after,
    )
}
fn java_result(env: &mut JNIEnv, result: *mut c_char) -> jstring {
    let text = unsafe { take(result) }
        .unwrap_or_else(|| "{\"consumed\":false,\"commits\":[],\"view\":null}".into());
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_create(
    mut env: JNIEnv,
    _class: JClass,
    directory: JString,
) -> jlong {
    let Ok(directory) = env.get_string(&directory) else {
        return 0;
    };
    let Ok(directory) = CString::new(String::from(directory)) else {
        return 0;
    };
    let handle = unsafe { meltype_create_at(directory.as_ptr()) as jlong };
    service_state::create(handle);
    documents::create(handle);
    handle
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_control(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    request: JString,
) -> jstring {
    let Ok(request) = env.get_string(&request) else {
        return std::ptr::null_mut();
    };
    let Ok(request) = CString::new(String::from(request)) else {
        return std::ptr::null_mut();
    };
    let result = unsafe { meltype_control(handle as *mut c_void, request.as_ptr()) };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_predict(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    index: jint,
) -> jstring {
    java_result(
        &mut env,
        meltype_select_prediction(handle as *mut c_void, index),
    )
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_destroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    service_state::destroy(handle);
    documents::destroy(handle);
    meltype_destroy(handle as *mut c_void);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_key(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    vk: jint,
    ch: jint,
    modifiers: jint,
) -> jstring {
    let result = unsafe {
        meltype_handle_key(
            handle as *mut c_void,
            vk,
            ch,
            modifiers,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_keyContext(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    vk: jint,
    ch: jint,
    modifiers: jint,
    before: JString,
    after: JString,
) -> jstring {
    let before = env
        .get_string(&before)
        .ok()
        .and_then(|s| CString::new(String::from(s)).ok());
    let after = env
        .get_string(&after)
        .ok()
        .and_then(|s| CString::new(String::from(s)).ok());
    let result = unsafe {
        meltype_handle_key(
            handle as *mut c_void,
            vk,
            ch,
            modifiers,
            before.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            after.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
        )
    };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_undo(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    before: JString,
) -> jstring {
    let Ok(before) = env.get_string(&before) else {
        return std::ptr::null_mut();
    };
    let Ok(before) = CString::new(String::from(before)) else {
        return std::ptr::null_mut();
    };
    meltype_set_can_delete(handle as *mut c_void, 1);
    let result = unsafe { meltype_undo_commit(handle as *mut c_void, before.as_ptr()) };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_canDelete(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    meltype_set_can_delete(handle as *mut c_void, i32::from(enabled != 0));
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_reconvert(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    text: JString,
) -> jstring {
    let Ok(text) = env.get_string(&text) else {
        return std::ptr::null_mut();
    };
    let Ok(text) = CString::new(String::from(text)) else {
        return std::ptr::null_mut();
    };
    let result = unsafe {
        meltype_reconvert_selection(handle as *mut c_void, text.as_ptr(), std::ptr::null())
    };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_commit(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    java_result(&mut env, meltype_commit(handle as *mut c_void))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_reset(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    service_state::clear(handle);
    java_result(&mut env, meltype_reset(handle as *mut c_void))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_select(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    index: jint,
) -> jstring {
    java_result(
        &mut env,
        meltype_select_candidate(handle as *mut c_void, index),
    )
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_direct(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    service_state::direct(handle, enabled != 0);
    meltype_set_direct(handle as *mut c_void, enabled as i32);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_privateInput(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    service_state::private(handle, enabled != 0);
    meltype_set_private(handle as *mut c_void, enabled as i32);
}
