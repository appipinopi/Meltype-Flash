// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Settings event orchestration and status are native. Java forwards events only.
use super::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, String>;
fn read(env: &mut JNIEnv, input: JString) -> Result<String> {
    env.get_string(&input)
        .map(String::from)
        .map_err(|e| e.to_string())
}
fn copy<'a>(env: &mut JNIEnv<'a>, object: &JObject) -> Result<JObject<'a>> {
    env.new_local_ref(object).map_err(|e| e.to_string())
}
fn form<'a>(env: &mut JNIEnv<'a>, activity: &JObject) -> Result<JObject<'a>> {
    env.get_field(activity, "form", "Lmoe/meltype/rust/SettingsPage;")
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())
}
fn status(env: &mut JNIEnv, activity: &JObject, text: &str) -> Result<()> {
    let view = env
        .get_field(activity, "status", "Landroid/widget/TextView;")
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    settings_view::property(env, &view, "setText", text)
}
fn show(env: &mut JNIEnv, handle: jlong, activity: &JObject, page: jint) -> Result<()> {
    let page = settings::page(handle, page)?;
    let bridge = form(env, activity)?;
    let input = env
        .new_string(page.to_string())
        .map_err(|e| e.to_string())?;
    settings_view::render(env, bridge, input)?;
    Ok(())
}
fn apply(env: &mut JNIEnv, handle: jlong, activity: &JObject, reply: Value) -> Result<()> {
    if reply["installShunti"] == true {
        ai_ui::start(env, handle, activity).map_err(|e| e.to_string())?;
    }
    if let Some(edits) = reply.get("edit") {
        let bridge = form(env, activity)?;
        let input = env
            .new_string(edits.to_string())
            .map_err(|e| e.to_string())?;
        settings_view::edit(env, bridge, input)?;
    }
    if let Some(pick) = reply.get("pick") {
        let operation = pick["operation"]
            .as_str()
            .ok_or("ファイル操作がありません")?;
        let operation = env.new_string(operation).map_err(|e| e.to_string())?;
        let activity = copy(env, activity)?;
        documents::pick(env, handle, activity, operation)?;
    }
    if let Some(confirmation) = reply.get("confirm") {
        let input = env
            .new_string(confirmation.to_string())
            .map_err(|e| e.to_string())?;
        let activity = copy(env, activity)?;
        dialogs::show(env, handle, activity, input)?;
    }
    if let Some(page) = reply.get("page") {
        let page = page
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .ok_or("設定ページが正しくありません")?;
        show(env, handle, activity, page)?;
    }
    Ok(())
}
fn action(env: &mut JNIEnv, handle: jlong, activity: &JObject, operation: JString) -> Result<()> {
    let text = read(env, operation)?;
    let operation: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let bridge = form(env, activity)?;
    let text = env.new_string(text).map_err(|e| e.to_string())?;
    let fields = settings_view::fields(env, bridge, text)?;
    let reply = settings::action(handle, operation, fields)?;
    apply(env, handle, activity, reply)
}
fn finish(env: &mut JNIEnv, activity: &JObject, result: Result<()>) -> jstring {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    let message = match &result {
        Ok(())
            if ai_ui::busy(
                env.get_field(activity, "session", "J")
                    .and_then(|v| v.j())
                    .unwrap_or(0),
            ) =>
        {
            "AI環境を準備しています…".to_owned()
        }
        Ok(()) => "完了".to_owned(),
        Err(error) => format!("エラー: {error}"),
    };
    let updated = status(env, activity, &message);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    settings::reply(env, result.and(updated).map(|_| json!({})))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_settingsUiShow(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    activity: JObject,
    page: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        show(&mut env, handle, &activity, page)
    }))
    .unwrap_or_else(|_| Err("設定ページを表示できません".into()));
    finish(&mut env, &activity, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_settingsUiAction(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    activity: JObject,
    input: JString,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        action(&mut env, handle, &activity, input)
    }))
    .unwrap_or_else(|_| Err("設定操作に失敗しました".into()));
    finish(&mut env, &activity, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_settingsUiResult(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    activity: JObject,
    code: jint,
    result: jint,
    data: JObject,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let context = copy(&mut env, &activity)?;
        let reply = documents::picker_result(&mut env, handle, context, code, result, data)?;
        apply(&mut env, handle, &activity, reply)
    }))
    .unwrap_or_else(|_| Err("ファイル操作に失敗しました".into()));
    finish(&mut env, &activity, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_settingsUiConfirmation(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    activity: JObject,
    input: JString,
    accept: jboolean,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let payload = read(&mut env, input)?;
        let reply = dialogs::complete(handle, &payload, accept != 0)?;
        apply(&mut env, handle, &activity, reply)
    }))
    .unwrap_or_else(|_| Err("確認操作に失敗しました".into()));
    finish(&mut env, &activity, result)
}
