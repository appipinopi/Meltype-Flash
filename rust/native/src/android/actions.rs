// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Keyboard/candidate actions. Safe JNI calls reuse the exported native editor
//! contracts; no registry lock or borrowed Rust state crosses editor callbacks.
use super::*;
type Result<T> = jni::errors::Result<T>;
pub(super) fn native<'a>(
    env: &mut JNIEnv<'a>,
    name: &str,
    signature: &str,
    args: &[JValue],
) -> Result<jni::objects::JValueOwned<'a>> {
    env.call_static_method("moe/meltype/rust/Native", name, signature, args)
}
pub(super) fn render(env: &mut JNIEnv, service: &JObject, reply: &JObject) -> Result<bool> {
    native(
        env,
        "renderInputView",
        "(Ljava/lang/Object;Ljava/lang/String;)Z",
        &[JValue::Object(service), JValue::Object(reply)],
    )?
    .z()
}
fn core_reply<'a>(env: &mut JNIEnv<'a>, pointer: *mut c_char) -> Result<JString<'a>> {
    let text = unsafe { take(pointer) }
        .unwrap_or_else(|| "{\"consumed\":false,\"commits\":[],\"view\":null}".into());
    env.new_string(text)
}
pub(super) fn apply(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    connection: &JObject,
    reply: &JObject,
    epoch: Option<u64>,
) -> Result<bool> {
    if service_state::epoch(handle) != epoch {
        return Ok(true);
    }
    if connection.is_null() {
        return Ok(false);
    }
    let pending = env.new_string(service_state::pending(handle))?;
    let outcome = native(
        env,
        "applyEditor",
        "(JLjava/lang/Object;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[
            JValue::Long(handle),
            JValue::Object(connection),
            JValue::Object(reply),
            JValue::Object(&pending),
        ],
    )?
    .l()?;
    render(env, service, &outcome)
}
fn key<'a>(
    env: &mut JNIEnv<'a>,
    handle: jlong,
    connection: &JObject,
    key: i32,
    scalar: i32,
    modifiers: i32,
) -> Result<JObject<'a>> {
    let pending = env.new_string(service_state::pending(handle))?;
    native(
        env,
        "editorKey",
        "(JLjava/lang/Object;IIIZZLjava/lang/String;)Ljava/lang/String;",
        &[
            JValue::Long(handle),
            JValue::Object(connection),
            JValue::Int(key),
            JValue::Int(scalar),
            JValue::Int(modifiers),
            JValue::Bool(0),
            JValue::Bool(u8::from(service_state::flags(handle) & 2 != 0)),
            JValue::Object(&pending),
        ],
    )?
    .l()
}
fn software_key(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    connection: &JObject,
    key_code: i32,
    scalar: i32,
    ime_options: i32,
) -> Result<()> {
    let epoch = service_state::epoch(handle);
    let reply = key(env, handle, connection, key_code, scalar, 0)?;
    if !apply(env, handle, service, connection, &reply, epoch)?
        && epoch == service_state::epoch(handle)
        && !connection.is_null()
    {
        native(
            env,
            "softwareFallback",
            "(Ljava/lang/Object;III)Z",
            &[
                JValue::Object(connection),
                JValue::Int(key_code),
                JValue::Int(scalar),
                JValue::Int(ime_options),
            ],
        )?;
    }
    Ok(())
}
fn switch_ime(env: &mut JNIEnv, service: &JObject) -> Result<()> {
    let api = env
        .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
        .i()?;
    if api >= 28 {
        env.call_method(
            service,
            "switchToNextInputMethod",
            "(Z)Z",
            &[JValue::Bool(0)],
        )?;
    } else {
        let name = env.new_string("input_method")?;
        let manager = env
            .call_method(
                service,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        if !manager.is_null() {
            env.call_method(manager, "showInputMethodPicker", "()V", &[])?;
        }
    }
    Ok(())
}
fn settings(env: &mut JNIEnv, service: &JObject) -> Result<()> {
    let class = env.find_class("moe/meltype/rust/SettingsActivity")?;
    let intent = env.new_object(
        "android/content/Intent",
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[JValue::Object(service), JValue::Object(&class)],
    )?;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x10000000)],
    )?;
    env.call_method(
        service,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )?;
    Ok(())
}
pub(super) fn dispatch(
    env: &mut JNIEnv,
    handle: jlong,
    service: &JObject,
    connection: &JObject,
    action: i32,
    value: i32,
    ime_options: i32,
) -> Result<()> {
    let epoch = service_state::epoch(handle);
    if epoch.is_none() {
        return Ok(());
    }
    match action {
        12 if service_state::flags(handle) & 7 == 4 => {
            let reply = core_reply(
                env,
                run(handle as *mut c_void, |s| {
                    let next = (!s.is_converting())
                        .then(|| {
                            s.raw()
                                .chars()
                                .next_back()
                                .and_then(crate::android_input::kana_variant)
                        })
                        .flatten();
                    if let Some(next) = next {
                        s.key(8, 0, 0);
                        s.key(0, next as i32, 0)
                    } else {
                        s.result(false)
                    }
                }),
            )?;
            apply(env, handle, service, connection, &reply, epoch)?;
        }
        0 => {
            let scalar = native(
                env,
                "takeShiftedScalar",
                "(JI)I",
                &[JValue::Long(handle), JValue::Int(value)],
            )?
            .i()?;
            software_key(env, handle, service, connection, 0, scalar, ime_options)?;
        }
        1 => {
            native(env, "toggleShift", "(J)V", &[JValue::Long(handle)])?;
        }
        2 => {
            let outcome = native(
                env,
                "toggleInputMode",
                "(JLjava/lang/Object;)Ljava/lang/String;",
                &[JValue::Long(handle), JValue::Object(connection)],
            )?
            .l()?;
            render(env, service, &outcome)?;
        }
        3 => {
            software_key(
                env,
                handle,
                service,
                connection,
                value,
                if value == 32 { 32 } else { 0 },
                ime_options,
            )?;
        }
        4 => switch_ime(env, service)?,
        5 | 6 => {
            let reply = key(
                env,
                handle,
                connection,
                if action == 5 { 0x25 } else { 0x27 },
                0,
                1,
            )?;
            apply(env, handle, service, connection, &reply, epoch)?;
        }
        7 if service_state::flags(handle) & 2 == 0 => {
            let pending = env.new_string(service_state::pending(handle))?;
            let reply = native(
                env,
                "editorUndo",
                "(JLjava/lang/Object;ZLjava/lang/String;)Ljava/lang/String;",
                &[
                    JValue::Long(handle),
                    JValue::Object(connection),
                    JValue::Bool(0),
                    JValue::Object(&pending),
                ],
            )?
            .l()?;
            apply(env, handle, service, connection, &reply, epoch)?;
        }
        8 => {
            let flags = service_state::flags(handle);
            let reply = native(
                env,
                "editorReconvert",
                "(JLjava/lang/Object;ZZ)Ljava/lang/String;",
                &[
                    JValue::Long(handle),
                    JValue::Object(connection),
                    JValue::Bool(u8::from(flags & 2 != 0)),
                    JValue::Bool(u8::from(flags & 4 != 0)),
                ],
            )?
            .l()?;
            if !reply.is_null()
                && !apply(env, handle, service, connection, &reply, epoch)?
                && epoch == service_state::epoch(handle)
            {
                let preview = env
                    .get_field(service, "preview", "Landroid/widget/TextView;")?
                    .l()?;
                if !preview.is_null() {
                    let text = env.new_string("読みが見つかりません")?;
                    env.call_method(
                        preview,
                        "setText",
                        "(Ljava/lang/CharSequence;)V",
                        &[JValue::Object(&text)],
                    )?;
                }
            }
        }
        9 => {
            let reply = core_reply(env, meltype_commit(handle as *mut c_void))?;
            apply(env, handle, service, connection, &reply, epoch)?;
            if epoch == service_state::epoch(handle) {
                settings(env, service)?;
            }
        }
        10 | 11 if value >= 0 => {
            let reply = core_reply(
                env,
                if action == 10 {
                    meltype_select_candidate(handle as *mut c_void, value)
                } else {
                    meltype_select_prediction(handle as *mut c_void, value)
                },
            )?;
            apply(env, handle, service, connection, &reply, epoch)?;
        }
        _ => {}
    }
    Ok(())
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_keyboardAction(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    service: JObject,
    connection: JObject,
    action: jint,
    value: jint,
    ime_options: jint,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        env.with_local_frame(64, |env| {
            dispatch(
                env,
                handle,
                &service,
                &connection,
                action,
                value,
                ime_options,
            )
        })
    }));
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}
