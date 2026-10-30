// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Androidのライフサイクル・設定画面・OS通知をRustで処理する。
use super::{bridge_api::*, *};
use jni::objects::JObjectArray;
#[path = "../../../../android/bridge_events.rs"]
pub(super) mod events;
use events::*;

fn keyboard<'a>(env: &mut JNIEnv<'a>, service: &JObject) -> Result<JObject<'a>> {
    object(env, service, "keyboard", "Lmoe/meltype/rust/FlickKeyboard;")
}
fn cancel(env: &mut JNIEnv, service: &JObject) -> Result<()> {
    let keyboard = keyboard(env, service)?;
    flick::cancel(env, &keyboard)
}
fn service<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    event: i32,
    value: i32,
    detail: i32,
    data: &JObject,
) -> Result<JObject<'a>> {
    match event {
        SERVICE_CREATE => {
            create_session(env, owner)?;
        }
        SERVICE_DESTROY => {
            cancel(env, owner)?;
            destroy_session(env, owner)?;
            for (field, signature) in [
                ("keyboard", "Lmoe/meltype/rust/FlickKeyboard;"),
                ("inputRoot", "Landroid/widget/LinearLayout;"),
                ("appearance", "Lmoe/meltype/rust/KeyboardAppearance;"),
            ] {
                set_object(env, owner, field, signature, &JObject::null())?;
            }
        }
        SERVICE_BEGIN => {
            cancel(env, owner)?;
            let handle = session(env, owner)?;
            native(
                env,
                if detail != 0 {
                    "serviceRestart"
                } else {
                    "serviceStart"
                },
                "(JLjava/lang/Object;Ljava/lang/Object;)V",
                &[
                    JValue::Long(handle),
                    JValue::Object(owner),
                    JValue::Object(data),
                ],
            )?;
        }
        SERVICE_END => {
            cancel(env, owner)?;
            let handle = session(env, owner)?;
            native(
                env,
                "serviceFinish",
                "(JLjava/lang/Object;)V",
                &[JValue::Long(handle), JValue::Object(owner)],
            )?;
        }
        SERVICE_HIDE => cancel(env, owner)?,
        SERVICE_SHOW => {
            let root = object(env, owner, "inputRoot", "Landroid/widget/LinearLayout;")?;
            if !root.is_null() {
                let appearance = object(
                    env,
                    owner,
                    "appearance",
                    "Lmoe/meltype/rust/KeyboardAppearance;",
                )?;
                appearance::apply(env, &appearance, &root)?;
            }
        }
        SERVICE_BUILD => {
            let handle = session(env, owner)?;
            let root = native(
                env,
                "serviceView",
                "(JLjava/lang/Object;)Ljava/lang/Object;",
                &[JValue::Long(handle), JValue::Object(owner)],
            )?
            .l()?;
            cancel(env, owner)?;
            set_object(
                env,
                owner,
                "inputRoot",
                "Landroid/widget/LinearLayout;",
                &root,
            )?;
            if !root.is_null() {
                let preferences = appearance::preferences_named(env, owner, "keyboard")?;
                let key = env.new_string("flick")?;
                let enabled = call(
                    env,
                    &preferences,
                    "getBoolean",
                    "(Ljava/lang/String;Z)Z",
                    &[JValue::Object(&key), JValue::Bool(1)],
                )?
                .z()?;
                let keyboard = flick::attach(env, owner, &root, owner, enabled)?;
                set_object(
                    env,
                    owner,
                    "keyboard",
                    "Lmoe/meltype/rust/FlickKeyboard;",
                    &keyboard,
                )?;
                let appearance = appearance::new(env, owner)?;
                set_object(
                    env,
                    owner,
                    "appearance",
                    "Lmoe/meltype/rust/KeyboardAppearance;",
                    &appearance,
                )?;
                appearance::apply(env, &appearance, &root)?;
            }
            return Ok(root);
        }
        SERVICE_SELECTION => {
            let end = call(env, data, "intValue", "()I", &[])?.i()?;
            let handle = session(env, owner)?;
            native(
                env,
                "serviceSelection",
                "(JLjava/lang/Object;III)V",
                &[
                    JValue::Long(handle),
                    JValue::Object(owner),
                    JValue::Int(value),
                    JValue::Int(detail),
                    JValue::Int(end),
                ],
            )?;
        }
        SERVICE_ACTION => {
            let handle = session(env, owner)?;
            native(
                env,
                "serviceClick",
                "(JLjava/lang/Object;II)V",
                &[
                    JValue::Long(handle),
                    JValue::Object(owner),
                    JValue::Int(value),
                    JValue::Int(detail),
                ],
            )?;
        }
        _ => unreachable!(),
    }
    Ok(JObject::null())
}
fn settings<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    event: i32,
    value: i32,
    detail: i32,
    data: &JObject,
) -> Result<JObject<'a>> {
    match event {
        SETTINGS_CREATE => {
            let handle = create_session(env, owner)?;
            if !data.is_null() {
                let key = env.new_string("documentOperation")?;
                let operation = call(
                    env,
                    data,
                    "getString",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(&key)],
                )?
                .l()?;
                native(
                    env,
                    "restoreDocumentPicker",
                    "(JLjava/lang/String;)V",
                    &[JValue::Long(handle), JValue::Object(&operation)],
                )?;
            }
            let result = native(
                env,
                "createSettingsActivity",
                "(Ljava/lang/Object;)Ljava/lang/String;",
                &[JValue::Object(owner)],
            )?
            .l()?;
            if string(env, &result)? != "{}" {
                destroy_session(env, owner)?;
                let message = string(env, &result)?;
                env.throw_new("java/lang/IllegalStateException", message)?;
                return Err(jni::errors::Error::JavaException);
            }
            let body = object(env, owner, "body", "Landroid/widget/LinearLayout;")?;
            let root = call(env, &body, "getParent", "()Landroid/view/ViewParent;", &[])?.l()?;
            let index = call(
                env,
                &root,
                "indexOfChild",
                "(Landroid/view/View;)I",
                &[JValue::Object(&body)],
            )?
            .i()?;
            let button = widget(env, "android/widget/Button", owner)?;
            text(env, &button, "setText", "キーボードの背景・テーマ")?;
            let callback = callback(env, owner, APPEAR_CHOOSE, 0, 1, &JObject::null())?;
            click(env, &button, &callback)?;
            call(
                env,
                &root,
                "addView",
                "(Landroid/view/View;I)V",
                &[JValue::Object(&button), JValue::Int(index)],
            )?;
        }
        SETTINGS_DESTROY => destroy_session(env, owner)?,
        SETTINGS_FORM => {
            let body = object(env, owner, "body", "Landroid/widget/LinearLayout;")?;
            let form=env.new_object("moe/meltype/rust/SettingsPage","(Landroid/content/Context;Landroid/widget/LinearLayout;Lmoe/meltype/rust/SettingsPage$Actions;)V",&[JValue::Object(owner),JValue::Object(&body),JValue::Object(owner)])?;
            set_object(env, owner, "form", "Lmoe/meltype/rust/SettingsPage;", &form)?;
        }
        SETTINGS_SAVE => {
            let handle = session(env, owner)?;
            let operation = native(
                env,
                "pendingDocument",
                "(J)Ljava/lang/String;",
                &[JValue::Long(handle)],
            )?
            .l()?;
            let key = env.new_string("documentOperation")?;
            call(
                env,
                data,
                "putString",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[JValue::Object(&key), JValue::Object(&operation)],
            )?;
        }
        SETTINGS_DISPATCH => {
            if value == 1 {
                let body = object(env, owner, "body", "Landroid/widget/LinearLayout;")?;
                call(env, &body, "removeAllViews", "()V", &[])?;
            } else {
                let actions = object(
                    env,
                    owner,
                    "actions",
                    "Lmoe/meltype/rust/SettingsPage$Actions;",
                )?;
                call(
                    env,
                    &actions,
                    "perform",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(data)],
                )?;
            }
        }
        SETTINGS_NAVIGATE | SETTINGS_ACTION | SETTINGS_CONFIRM | SETTINGS_RESULT => {
            let handle = session(env, owner)?;
            match event {
                SETTINGS_NAVIGATE => {
                    native(
                        env,
                        "settingsUiShow",
                        "(JLjava/lang/Object;I)Ljava/lang/String;",
                        &[
                            JValue::Long(handle),
                            JValue::Object(owner),
                            JValue::Int(value),
                        ],
                    )?;
                }
                SETTINGS_ACTION => {
                    native(
                        env,
                        "settingsUiAction",
                        "(JLjava/lang/Object;Ljava/lang/String;)Ljava/lang/String;",
                        &[
                            JValue::Long(handle),
                            JValue::Object(owner),
                            JValue::Object(data),
                        ],
                    )?;
                }
                SETTINGS_CONFIRM => {
                    native(
                        env,
                        "settingsUiConfirmation",
                        "(JLjava/lang/Object;Ljava/lang/String;Z)Ljava/lang/String;",
                        &[
                            JValue::Long(handle),
                            JValue::Object(owner),
                            JValue::Object(data),
                            JValue::Bool(u8::from(value != 0)),
                        ],
                    )?;
                }
                _ => {
                    native(
                        env,
                        "settingsUiResult",
                        "(JLjava/lang/Object;IILjava/lang/Object;)Ljava/lang/String;",
                        &[
                            JValue::Long(handle),
                            JValue::Object(owner),
                            JValue::Int(value),
                            JValue::Int(detail),
                            JValue::Object(data),
                        ],
                    )?;
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(JObject::null())
}
fn dispatch<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    event: i32,
    value: i32,
    detail: i32,
    data: &JObject,
) -> Result<JObject<'a>> {
    match event {
        SERVICE_CREATE..=SERVICE_SHOW | SERVICE_SELECTION | SERVICE_ACTION => {
            service(env, owner, event, value, detail, data)
        }
        FLICK_GROUPS => flick::groups(env),
        FLICK_ATTACH => {
            let args = JObjectArray::from(env.new_local_ref(owner)?);
            let context = env.get_object_array_element(&args, 0)?;
            let root = env.get_object_array_element(&args, 1)?;
            let listener = env.get_object_array_element(&args, 2)?;
            flick::attach(env, &context, &root, &listener, value != 0)
        }
        FLICK_CANCEL => {
            flick::cancel(env, owner)?;
            Ok(JObject::null())
        }
        FLICK_TAP => {
            flick::tap(env, owner)?;
            Ok(JObject::null())
        }
        FLICK_TOGGLE => {
            if detail == 1 {
                let prefs = appearance::preferences_named(env, owner, "keyboard")?;
                appearance::put_bool(env, &prefs, "flick", value != 0)?;
            } else {
                flick::toggle(env, owner)?;
            }
            Ok(JObject::null())
        }
        SETTINGS_CREATE..=SETTINGS_DISPATCH => settings(env, owner, event, value, detail, data),
        APPEAR_PREFERENCES => appearance::preferences(env, owner),
        APPEAR_FILE => appearance::image_file(env, owner),
        APPEAR_THEME => {
            let value = string(env, data)?;
            appearance::set_theme(env, owner, &value)?;
            Ok(JObject::null())
        }
        APPEAR_CLEAR => {
            appearance::clear_image(env, owner)?;
            Ok(JObject::null())
        }
        APPEAR_IMPORT => {
            appearance::import_image(env, owner, data)?;
            Ok(JObject::null())
        }
        APPEAR_APPLY => {
            appearance::apply(env, owner, data)?;
            Ok(JObject::null())
        }
        APPEAR_STYLE => {
            appearance::style(env, owner, data)?;
            Ok(JObject::null())
        }
        BACKGROUND_DRAW..=BACKGROUND_FILTER => {
            appearance::background(env, owner, event, value, data)?;
            Ok(JObject::null())
        }
        APPEAR_CREATE | APPEAR_RESULT | APPEAR_WORK..=APPEAR_REMOVE => {
            appearance_ui::event(env, owner, event, value, detail, data)?;
            Ok(JObject::null())
        }
        _ => {
            env.throw_new(
                "java/lang/IllegalArgumentException",
                "Unknown Rust bridge event",
            )?;
            Err(jni::errors::Error::JavaException)
        }
    }
}
fn report(env: &mut JNIEnv, error: jni::errors::Error, event: i32) {
    if !env.exception_check().unwrap_or(true) {
        let _ = env.throw_new(
            if event == APPEAR_IMPORT {
                "java/io/IOException"
            } else {
                "java/lang/IllegalStateException"
            },
            error.to_string(),
        );
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_bridge(
    mut env: JNIEnv,
    _class: JClass,
    owner: JObject,
    event: jint,
    value: jint,
    detail: jint,
    data: JObject,
) -> jni::sys::jobject {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        dispatch(&mut env, &owner, event, value, detail, &data).map(JObject::into_raw)
    }));
    match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            report(&mut env, error, event);
            std::ptr::null_mut()
        }
        Err(_) => {
            let _ = env.throw_new(
                "java/lang/IllegalStateException",
                "Rust Android bridge failed",
            );
            std::ptr::null_mut()
        }
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_bridgeBoolean(
    mut env: JNIEnv,
    _class: JClass,
    owner: JObject,
    event: jint,
    value: jint,
    _detail: jint,
    data: JObject,
) -> jboolean {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<bool> {
        match event {
            FLICK_TOUCH => flick::touch(&mut env, &owner, &data),
            SERVICE_KEY => {
                let handle = session(&mut env, &owner)?;
                native(
                    &mut env,
                    "serviceKey",
                    "(JLjava/lang/Object;ILjava/lang/Object;)Z",
                    &[
                        JValue::Long(handle),
                        JValue::Object(&owner),
                        JValue::Int(value),
                        JValue::Object(&data),
                    ],
                )?
                .z()
            }
            _ => Ok(false),
        }
    }));
    match result {
        Ok(Ok(handled)) => u8::from(handled),
        Ok(Err(error)) => {
            report(&mut env, error, event);
            u8::from(event == FLICK_TOUCH)
        }
        Err(_) => {
            let _ = env.throw_new(
                "java/lang/IllegalStateException",
                "Rust Android input failed",
            );
            1
        }
    }
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_flickDirection(
    _env: JNIEnv,
    _class: JClass,
    dx: f32,
    dy: f32,
    threshold: f32,
) -> jint {
    flick::direction(dx, dy, threshold)
}
