// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Settings activity hierarchy. Publish fields and initialize listeners before
//! attaching the view, so the first Spinner callback sees a complete form.
use super::settings_view::{invoke, property};
use super::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, String>;
fn widget<'a>(env: &mut JNIEnv<'a>, class: &str, context: &JObject) -> Result<JObject<'a>> {
    env.new_object(
        class,
        "(Landroid/content/Context;)V",
        &[JValue::Object(context)],
    )
    .map_err(|e| e.to_string())
}
fn add(env: &mut JNIEnv, parent: &JObject, child: &JObject) -> Result<()> {
    invoke(
        env,
        parent,
        "addView",
        "(Landroid/view/View;)V",
        &[JValue::Object(child)],
    )?;
    Ok(())
}
fn build(env: &mut JNIEnv, activity: JObject) -> Result<Value> {
    let scroll = widget(env, "android/widget/ScrollView", &activity)?;
    let root = widget(env, "android/widget/LinearLayout", &activity)?;
    invoke(env, &root, "setOrientation", "(I)V", &[JValue::Int(1)])?;
    let resources = invoke(
        env,
        &activity,
        "getResources",
        "()Landroid/content/res/Resources;",
        &[],
    )?
    .l()
    .map_err(|e| e.to_string())?;
    let metrics = invoke(
        env,
        &resources,
        "getDisplayMetrics",
        "()Landroid/util/DisplayMetrics;",
        &[],
    )?
    .l()
    .map_err(|e| e.to_string())?;
    let density = env
        .get_field(&metrics, "density", "F")
        .and_then(|v| v.f())
        .map_err(|e| e.to_string())?;
    let padding = (16. * density) as i32;
    invoke(
        env,
        &root,
        "setPadding",
        "(IIII)V",
        &[
            JValue::Int(padding),
            JValue::Int(padding),
            JValue::Int(padding),
            JValue::Int(padding),
        ],
    )?;
    add(env, &scroll, &root)?;
    let title = widget(env, "android/widget/TextView", &activity)?;
    property(env, &title, "setText", "Meltype-FLASH 設定")?;
    invoke(env, &title, "setTextSize", "(F)V", &[JValue::Float(24.)])?;
    add(env, &root, &title)?;
    let status = widget(env, "android/widget/TextView", &activity)?;
    add(env, &root, &status)?;
    let navigation = widget(env, "android/widget/Spinner", &activity)?;
    let layout = env
        .get_static_field("android/R$layout", "simple_spinner_dropdown_item", "I")
        .and_then(|v| v.i())
        .map_err(|e| e.to_string())?;
    let labels = env
        .new_object_array(6, "java/lang/String", JObject::null())
        .map_err(|e| e.to_string())?;
    for (index, label) in [
        "入力設定",
        "ユーザー辞書",
        "定型文",
        "学習",
        "プロファイル",
        "バックアップ・診断",
    ]
    .iter()
    .enumerate()
    {
        let text = env.new_string(label).map_err(|e| e.to_string())?;
        env.set_object_array_element(&labels, index as i32, &text)
            .map_err(|e| e.to_string())?;
    }
    let adapter = env
        .new_object(
            "android/widget/ArrayAdapter",
            "(Landroid/content/Context;I[Ljava/lang/Object;)V",
            &[
                JValue::Object(&activity),
                JValue::Int(layout),
                JValue::Object(&labels),
            ],
        )
        .map_err(|e| e.to_string())?;
    invoke(
        env,
        &navigation,
        "setAdapter",
        "(Landroid/widget/SpinnerAdapter;)V",
        &[JValue::Object(&adapter)],
    )?;
    add(env, &root, &navigation)?;
    let body = widget(env, "android/widget/LinearLayout", &activity)?;
    invoke(env, &body, "setOrientation", "(I)V", &[JValue::Int(1)])?;
    add(env, &root, &body)?;
    env.set_field(
        &activity,
        "body",
        "Landroid/widget/LinearLayout;",
        JValue::Object(&body),
    )
    .map_err(|e| e.to_string())?;
    env.set_field(
        &activity,
        "status",
        "Landroid/widget/TextView;",
        JValue::Object(&status),
    )
    .map_err(|e| e.to_string())?;
    invoke(env, &activity, "settingsFormReady", "()V", &[])?;
    let listener = invoke(
        env,
        &activity,
        "settingsNavigationCallback",
        "()Landroid/widget/AdapterView$OnItemSelectedListener;",
        &[],
    )?
    .l()
    .map_err(|e| e.to_string())?;
    invoke(
        env,
        &navigation,
        "setOnItemSelectedListener",
        "(Landroid/widget/AdapterView$OnItemSelectedListener;)V",
        &[JValue::Object(&listener)],
    )?;
    invoke(
        env,
        &activity,
        "setContentView",
        "(Landroid/view/View;)V",
        &[JValue::Object(&scroll)],
    )?;
    Ok(json!({}))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_createSettingsActivity(
    mut env: JNIEnv,
    _class: JClass,
    activity: JObject,
) -> jstring {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(&mut env, activity)))
            .unwrap_or_else(|_| Err("設定画面を作成できません".into()));
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    settings::reply(&mut env, result)
}
