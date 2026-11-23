// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Confirmation UI and action dispatch. Java provides listener instances only.
use super::settings_view::invoke;
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, String>;
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Action {
    Learning {},
    Ai {},
    Restore { token: u64 },
}
fn read(env: &mut JNIEnv, input: JString) -> Result<String> {
    env.get_string(&input)
        .map(String::from)
        .map_err(|e| e.to_string())
}
fn build(env: &mut JNIEnv, activity: JObject, confirmation: &Value, action: &Action) -> Result<()> {
    let message = confirmation["message"]
        .as_str()
        .ok_or("確認メッセージがありません")?;
    let positive = match action {
        Action::Learning {} | Action::Ai {} => confirmation["positive"]
            .as_str()
            .ok_or("確認ボタンがありません")?,
        Action::Restore { .. } => "復元",
    };
    let builder = env
        .new_object(
            "android/app/AlertDialog$Builder",
            "(Landroid/content/Context;)V",
            &[JValue::Object(&activity)],
        )
        .map_err(|e| e.to_string())?;
    for (method, value) in [
        ("setTitle", confirmation["title"].as_str()),
        ("setMessage", Some(message)),
    ] {
        if let Some(value) = value {
            let value = env.new_string(value).map_err(|e| e.to_string())?;
            invoke(
                env,
                &builder,
                method,
                "(Ljava/lang/CharSequence;)Landroid/app/AlertDialog$Builder;",
                &[JValue::Object(&value)],
            )?;
        }
    }
    let payload = env
        .new_string(serde_json::to_string(action).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for (method, label, accept) in [
        ("setNegativeButton", "キャンセル", false),
        ("setPositiveButton", positive, true),
    ] {
        let listener = invoke(
            env,
            &activity,
            "settingsConfirmationClick",
            "(Ljava/lang/String;Z)Landroid/content/DialogInterface$OnClickListener;",
            &[JValue::Object(&payload), JValue::Bool(u8::from(accept))],
        )?
        .l()
        .map_err(|e| e.to_string())?;
        let label = env.new_string(label).map_err(|e| e.to_string())?;
        invoke(env,&builder,method,"(Ljava/lang/CharSequence;Landroid/content/DialogInterface$OnClickListener;)Landroid/app/AlertDialog$Builder;",&[JValue::Object(&label),JValue::Object(&listener)])?;
    }
    let cancel = invoke(
        env,
        &activity,
        "settingsConfirmationCancel",
        "(Ljava/lang/String;)Landroid/content/DialogInterface$OnCancelListener;",
        &[JValue::Object(&payload)],
    )?
    .l()
    .map_err(|e| e.to_string())?;
    invoke(
        env,
        &builder,
        "setOnCancelListener",
        "(Landroid/content/DialogInterface$OnCancelListener;)Landroid/app/AlertDialog$Builder;",
        &[JValue::Object(&cancel)],
    )?;
    invoke(env, &builder, "show", "()Landroid/app/AlertDialog;", &[])?;
    Ok(())
}
pub(super) fn show(
    env: &mut JNIEnv,
    handle: jlong,
    activity: JObject,
    input: JString,
) -> Result<Value> {
    let confirmation: Value =
        serde_json::from_str(&read(env, input)?).map_err(|e| e.to_string())?;
    let action = if let Some(token) = confirmation["token"].as_u64() {
        if confirmation.get("action").is_some() {
            return Err("確認操作が正しくありません".into());
        }
        Action::Restore { token }
    } else if confirmation["action"] == json!({"action":"ai_install"}) {
        Action::Ai {}
    } else if confirmation["action"] == json!({"action":"learning_clear"}) {
        Action::Learning {}
    } else {
        return Err("確認操作が見つかりません".into());
    };
    let result = build(env, activity, &confirmation, &action);
    if result.is_err() {
        if let Action::Restore { token } = &action {
            let _ = documents::restore(handle, *token, false);
        }
    }
    result.map(|_| json!({}))
}
pub(super) fn complete(handle: jlong, payload: &str, accept: bool) -> Result<Value> {
    let action: Action = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    match action {
        Action::Ai {} => Ok(json!({"installShunti":accept})),
        Action::Learning {} if accept => {
            settings::action(handle, json!({"action":"learning_clear"}), json!({}))
        }
        Action::Learning {} => Ok(json!({})),
        Action::Restore { token } => documents::restore(handle, token, accept),
    }
}
fn finish(env: &mut JNIEnv, result: Result<Value>) -> jstring {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    settings::reply(env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_showSettingsConfirmation(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    activity: JObject,
    input: JString,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        show(&mut env, handle, activity, input)
    }))
    .unwrap_or_else(|_| Err("確認画面を作成できません".into()));
    finish(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_completeSettingsConfirmation(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    payload: JString,
    accept: jboolean,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        complete(handle, &read(&mut env, payload)?, accept != 0)
    }))
    .unwrap_or_else(|_| Err("確認操作に失敗しました".into()));
    finish(&mut env, result)
}
