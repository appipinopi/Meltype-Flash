// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! InputMethodService event orchestration. One captured connection per event;
//! never continue an old event after a getter replaces the field generation.
use super::actions::{native, render};
use super::*;
type Result<T> = jni::errors::Result<T>;
fn connection<'a>(env: &mut JNIEnv<'a>, service: &JObject) -> Result<JObject<'a>> {
    env.call_method(
        service,
        "getCurrentInputConnection",
        "()Landroid/view/inputmethod/InputConnection;",
        &[],
    )?
    .l()
}
fn pending<'a>(env: &mut JNIEnv<'a>, handle: jlong) -> Result<JString<'a>> {
    env.new_string(service_state::pending(handle))
}
fn refresh(env: &mut JNIEnv, handle: jlong, service: &JObject) -> Result<()> {
    let view = native(
        env,
        "editorView",
        "(J)Ljava/lang/String;",
        &[JValue::Long(handle)],
    )?
    .l()?;
    render(env, service, &view)?;
    Ok(())
}
fn start(env: &mut JNIEnv, handle: jlong, service: &JObject, info: &JObject) -> Result<()> {
    if service_state::epoch(handle).is_none() {
        return Ok(());
    }
    let (input, options) = if info.is_null() {
        (0, 0)
    } else {
        (
            env.get_field(info, "inputType", "I")?.i()?,
            env.get_field(info, "imeOptions", "I")?.i()?,
        )
    };
    native(
        env,
        "startInput",
        "(JII)I",
        &[
            JValue::Long(handle),
            JValue::Int(input),
            JValue::Int(options),
        ],
    )?;
    let preview = env
        .get_field(service, "preview", "Landroid/widget/TextView;")?
        .l()?;
    if !preview.is_null() {
        let blank = env.new_string("")?;
        env.call_method(
            &preview,
            "setText",
            "(Ljava/lang/CharSequence;)V",
            &[JValue::Object(&blank)],
        )?;
    }
    let candidates = env
        .get_field(service, "candidates", "Landroid/widget/LinearLayout;")?
        .l()?;
    if !candidates.is_null() {
        env.call_method(&candidates, "removeAllViews", "()V", &[])?;
    }
    Ok(())
}
fn key(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    key: jint,
    event: &JObject,
) -> Result<bool> {
    if crate::android_input::modifier_key(key) || event.is_null() {
        return Ok(false);
    }
    let Some(epoch) = service_state::epoch(handle) else {
        return Ok(false);
    };
    let scalar = env.call_method(event, "getUnicodeChar", "()I", &[])?.i()?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(true);
    }
    let modifiers = env.call_method(event, "getMetaState", "()I", &[])?.i()?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(true);
    }
    let connection = connection(env, service)?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(true);
    }
    if connection.is_null() {
        return Ok(false);
    }
    let text = pending(env, handle)?;
    let reply = native(
        env,
        "editorKey",
        "(JLjava/lang/Object;IIIZZLjava/lang/String;)Ljava/lang/String;",
        &[
            JValue::Long(handle),
            JValue::Object(&connection),
            JValue::Int(key),
            JValue::Int(scalar),
            JValue::Int(modifiers),
            JValue::Bool(1),
            JValue::Bool(u8::from(service_state::flags(handle) & 2 != 0)),
            JValue::Object(&text),
        ],
    )?
    .l()?;
    actions::apply(env, handle, service, &connection, &reply, Some(epoch))
}
fn restart(env: &mut JNIEnv, handle: jlong, service: &JObject, info: &JObject) -> Result<()> {
    let Some(epoch) = service_state::epoch(handle) else {
        return Ok(());
    };
    let flags = if info.is_null() {
        0
    } else {
        crate::android_input::field_flags(
            env.get_field(info, "inputType", "I")?.i()?,
            env.get_field(info, "imeOptions", "I")?.i()?,
        )
    };
    let old = service_state::flags(handle);
    let connection = connection(env, service)?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(());
    }
    let pending = service_state::pending(handle);
    let compatible = old & 2 == flags & 2 && (flags & 1 == 0 || old & 1 != 0);
    let mut matches = pending.is_empty();
    if compatible && flags & 2 == 0 && !pending.is_empty() && !connection.is_null() {
        let text = env
            .call_method(
                &connection,
                "getTextBeforeCursor",
                "(II)Ljava/lang/CharSequence;",
                &[
                    JValue::Int(pending.encode_utf16().count() as i32),
                    JValue::Int(0),
                ],
            )?
            .l()?;
        if service_state::epoch(handle) != Some(epoch) {
            return Ok(());
        }
        if !text.is_null() {
            let text = env
                .call_method(text, "toString", "()Ljava/lang/String;", &[])?
                .l()?;
            matches = String::from(env.get_string(&JString::from(text))?) == pending;
        }
    }
    if compatible && matches && flags & 2 == 0 && !pending.is_empty() && !connection.is_null() {
        let selected = env
            .call_method(
                &connection,
                "getSelectedText",
                "(I)Ljava/lang/CharSequence;",
                &[JValue::Int(0)],
            )?
            .l()?;
        if service_state::epoch(handle) != Some(epoch) {
            return Ok(());
        }
        matches = selected.is_null() || env.call_method(selected, "length", "()I", &[])?.i()? == 0;
    }
    if compatible && matches {
        service_state::restart(handle);
        return refresh(env, handle, service);
    }
    // A changed editor owns its current text. Retire its old span without
    // replacing it using the stale composition from before restartInput.
    if !connection.is_null() {
        env.call_method(&connection, "finishComposingText", "()Z", &[])?;
        if service_state::epoch(handle) != Some(epoch) {
            return Ok(());
        }
    }
    start(env, handle, service, info)
}
fn finish(env: &mut JNIEnv, handle: jlong, service: &JObject) -> Result<()> {
    let Some(epoch) = service_state::epoch(handle) else {
        return Ok(());
    };
    let connection = connection(env, service)?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(());
    }
    let text = pending(env, handle)?;
    let reply = native(
        env,
        "finishInput",
        "(JLjava/lang/Object;ZLjava/lang/String;)Ljava/lang/String;",
        &[
            JValue::Long(handle),
            JValue::Object(&connection),
            JValue::Bool(u8::from(service_state::flags(handle) & 4 != 0)),
            JValue::Object(&text),
        ],
    )?
    .l()?;
    render(env, service, &reply)?;
    Ok(())
}
fn selection(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    start: jint,
    end: jint,
    composing_end: jint,
) -> Result<()> {
    let Some(epoch) = service_state::epoch(handle) else {
        return Ok(());
    };
    let connection = connection(env, service)?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(());
    }
    let changed = native(
        env,
        "selectionChanged",
        "(JLjava/lang/Object;ZIII)Z",
        &[
            JValue::Long(handle),
            JValue::Object(&connection),
            JValue::Bool(u8::from(service_state::flags(handle) & 4 != 0)),
            JValue::Int(start),
            JValue::Int(end),
            JValue::Int(composing_end),
        ],
    )?
    .z()?;
    if changed {
        refresh(env, handle, service)?;
    }
    Ok(())
}
fn click(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    action: jint,
    value: jint,
) -> Result<()> {
    let Some(epoch) = service_state::epoch(handle) else {
        return Ok(());
    };
    let info = env
        .call_method(
            service,
            "getCurrentInputEditorInfo",
            "()Landroid/view/inputmethod/EditorInfo;",
            &[],
        )?
        .l()?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(());
    }
    let options = if info.is_null() {
        0
    } else {
        env.get_field(&info, "imeOptions", "I")?.i()?
    };
    let connection = connection(env, service)?;
    if service_state::epoch(handle) != Some(epoch) {
        return Ok(());
    }
    actions::dispatch(env, handle, service, &connection, action, value, options)
}
fn clear(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceStart(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    info: JObject,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        start(&mut env, handle, &service, &info)
    }));
    clear(&mut env);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceRestart(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    info: JObject,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        restart(&mut env, handle, &service, &info)
    }));
    clear(&mut env);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceKey(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    key_code: jint,
    event: JObject,
) -> jboolean {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        key(&mut env, handle, &service, key_code, &event)
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(false);
    clear(&mut env);
    u8::from(result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceFinish(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| finish(&mut env, handle, &service)));
    clear(&mut env);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceSelection(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    start: jint,
    end: jint,
    composing_end: jint,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        selection(&mut env, handle, &service, start, end, composing_end)
    }));
    clear(&mut env);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceClick(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    action: jint,
    value: jint,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        click(&mut env, handle, &service, action, value)
    }));
    clear(&mut env);
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_serviceView(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
) -> jni::sys::jobject {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<JObject> {
        if service_state::epoch(handle).is_none() {
            return Ok(JObject::null());
        }
        let view = native(
            &mut env,
            "createInputView",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            &[JValue::Object(&service)],
        )?
        .l()?;
        refresh(&mut env, handle, &service)?;
        Ok(view)
    }))
    .ok()
    .and_then(Result::ok);
    clear(&mut env);
    result.map_or(std::ptr::null_mut(), JObject::into_raw)
}
