// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 背景設定の画面、画像選択、非同期取込みをRustで処理する。
use super::{appearance, bridge::events::*, bridge_api::*, *};
fn save_field(
    env: &mut JNIEnv,
    activity: &JObject,
    name: &str,
    signature: &str,
    view: &JObject,
) -> Result<()> {
    set_object(env, activity, name, signature, view)
}
fn button<'a>(
    env: &mut JNIEnv<'a>,
    activity: &JObject,
    body: &JObject,
    label: &str,
    event: i32,
    value: i32,
) -> Result<JObject<'a>> {
    let button = widget(env, "android/widget/Button", activity)?;
    text(env, &button, "setText", label)?;
    add(env, body, &button)?;
    let listener = callback(env, activity, event, value, 0, &JObject::null())?;
    click(env, &button, &listener)?;
    Ok(button)
}
fn status(env: &mut JNIEnv, activity: &JObject, message: &str) -> Result<()> {
    let status = object(env, activity, "status", "Landroid/widget/TextView;")?;
    text(env, &status, "setText", message)
}
fn refresh(env: &mut JNIEnv, activity: &JObject) -> Result<()> {
    let appearance = object(
        env,
        activity,
        "appearance",
        "Lmoe/meltype/rust/KeyboardAppearance;",
    )?;
    let preview = object(env, activity, "preview", "Landroid/widget/LinearLayout;")?;
    appearance::apply(env, &appearance, &preview)
}
fn enabled(env: &mut JNIEnv, activity: &JObject, value: bool) -> Result<()> {
    for field in ["choose", "remove"] {
        let button = object(env, activity, field, "Landroid/widget/Button;")?;
        call(
            env,
            &button,
            "setEnabled",
            "(Z)V",
            &[JValue::Bool(u8::from(value))],
        )?;
    }
    Ok(())
}
fn build(env: &mut JNIEnv, activity: &JObject) -> Result<()> {
    let scroll = widget(env, "android/widget/ScrollView", activity)?;
    let body = widget(env, "android/widget/LinearLayout", activity)?;
    call(env, &body, "setOrientation", "(I)V", &[JValue::Int(1)])?;
    let padding = (16. * density(env, activity)?).round() as i32;
    call(
        env,
        &body,
        "setPadding",
        "(IIII)V",
        &[JValue::Int(padding); 4],
    )?;
    add(env, &scroll, &body)?;
    call(
        env,
        activity,
        "setContentView",
        "(Landroid/view/View;)V",
        &[JValue::Object(&scroll)],
    )?;
    let title = widget(env, "android/widget/TextView", activity)?;
    text(env, &title, "setText", "キーボードの背景・テーマ")?;
    call(env, &title, "setTextSize", "(F)V", &[JValue::Float(24.)])?;
    add(env, &body, &title)?;
    let help = widget(env, "android/widget/TextView", activity)?;
    text(env,&help,"setText","選んだ画像だけを端末内に縮小保存します。画像は中央を基準に表示し、キーと候補の文字色はテーマに合わせます。")?;
    add(env, &body, &help)?;
    let themes = widget(env, "android/widget/RadioGroup", activity)?;
    add(env, &body, &themes)?;
    save_field(
        env,
        activity,
        "themes",
        "Landroid/widget/RadioGroup;",
        &themes,
    )?;
    let preferences = appearance::preferences(env, activity)?;
    let current = appearance::get_string(env, &preferences, "theme", "system")?;
    for (index, (value, label)) in appearance::THEMES.iter().enumerate() {
        env.with_local_frame(16, |env| -> Result<()> {
            let radio = widget(env, "android/widget/RadioButton", activity)?;
            let id = env
                .call_static_method("android/view/View", "generateViewId", "()I", &[])?
                .i()?;
            call(env, &radio, "setId", "(I)V", &[JValue::Int(id)])?;
            text(env, &radio, "setText", label)?;
            add(env, &themes, &radio)?;
            call(
                env,
                &radio,
                "setChecked",
                "(Z)V",
                &[JValue::Bool(u8::from(*value == current))],
            )?;
            let listener = callback(
                env,
                activity,
                APPEAR_SET_THEME,
                index as i32,
                0,
                &JObject::null(),
            )?;
            click(env, &radio, &listener)
        })?;
    }
    let choose = button(env, activity, &body, "好きな画像を選ぶ", APPEAR_CHOOSE, 0)?;
    save_field(env, activity, "choose", "Landroid/widget/Button;", &choose)?;
    let remove = button(
        env,
        activity,
        &body,
        "背景画像を外す（テーマは保持）",
        APPEAR_REMOVE,
        0,
    )?;
    save_field(env, activity, "remove", "Landroid/widget/Button;", &remove)?;
    button(
        env,
        activity,
        &body,
        "システムの配色へ戻す",
        APPEAR_RESET,
        0,
    )?;
    let status = widget(env, "android/widget/TextView", activity)?;
    call(
        env,
        &status,
        "setAccessibilityLiveRegion",
        "(I)V",
        &[JValue::Int(1)],
    )?;
    add(env, &body, &status)?;
    save_field(
        env,
        activity,
        "status",
        "Landroid/widget/TextView;",
        &status,
    )?;
    let preview = widget(env, "android/widget/LinearLayout", activity)?;
    call(env, &preview, "setOrientation", "(I)V", &[JValue::Int(1)])?;
    call(
        env,
        &preview,
        "setPadding",
        "(IIII)V",
        &[JValue::Int(padding); 4],
    )?;
    add(env, &body, &preview)?;
    save_field(
        env,
        activity,
        "preview",
        "Landroid/widget/LinearLayout;",
        &preview,
    )?;
    let label = widget(env, "android/widget/TextView", activity)?;
    text(env, &label, "setText", "背景のプレビュー")?;
    add(env, &preview, &label)?;
    let keys = widget(env, "android/widget/LinearLayout", activity)?;
    add(env, &preview, &keys)?;
    for label in ["あ", "か", "さ"] {
        let key = widget(env, "android/widget/Button", activity)?;
        text(env, &key, "setText", label)?;
        let params = env.new_object(
            "android/widget/LinearLayout$LayoutParams",
            "(IIF)V",
            &[JValue::Int(0), JValue::Int(-2), JValue::Float(1.)],
        )?;
        call(
            env,
            &keys,
            "addView",
            "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
            &[JValue::Object(&key), JValue::Object(&params)],
        )?;
    }
    let appearance = appearance::new(env, activity)?;
    save_field(
        env,
        activity,
        "appearance",
        "Lmoe/meltype/rust/KeyboardAppearance;",
        &appearance,
    )?;
    refresh(env, activity)
}
fn choose(env: &mut JNIEnv, activity: &JObject, settings: bool) -> Result<()> {
    if settings {
        let class = env.find_class("moe/meltype/rust/AppearanceActivity")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Landroid/content/Context;Ljava/lang/Class;)V",
            &[JValue::Object(activity), JValue::Object(&class)],
        )?;
        call(
            env,
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
    } else {
        let action = env.new_string("android.intent.action.OPEN_DOCUMENT")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let category = env.new_string("android.intent.category.OPENABLE")?;
        call(
            env,
            &intent,
            "addCategory",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&category)],
        )?;
        let mime = env.new_string("image/*")?;
        call(
            env,
            &intent,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&mime)],
        )?;
        call(
            env,
            activity,
            "startActivityForResult",
            "(Landroid/content/Intent;I)V",
            &[JValue::Object(&intent), JValue::Int(41)],
        )?;
    }
    Ok(())
}
pub(super) fn event(
    env: &mut JNIEnv,
    activity: &JObject,
    event: i32,
    value: i32,
    detail: i32,
    data: &JObject,
) -> Result<()> {
    match event {
        APPEAR_CREATE => build(env, activity)?,
        APPEAR_CHOOSE => choose(env, activity, detail == 1)?,
        APPEAR_SET_THEME => {
            if let Some((theme, _)) = appearance::THEMES.get(value as usize) {
                appearance::set_theme(env, activity, theme)?;
                refresh(env, activity)?;
            }
        }
        APPEAR_RESET => {
            appearance::set_theme(env, activity, "system")?;
            let themes = object(env, activity, "themes", "Landroid/widget/RadioGroup;")?;
            let first = call(
                env,
                &themes,
                "getChildAt",
                "(I)Landroid/view/View;",
                &[JValue::Int(0)],
            )?
            .l()?;
            let id = call(env, &first, "getId", "()I", &[])?.i()?;
            call(env, &themes, "check", "(I)V", &[JValue::Int(id)])?;
            refresh(env, activity)?;
        }
        APPEAR_REMOVE => {
            appearance::clear_image(env, activity)?;
            refresh(env, activity)?;
            status(env, activity, "背景画像を削除しました。")?;
        }
        APPEAR_RESULT => {
            if value != 41 || detail != -1 || data.is_null() {
                return Ok(());
            }
            let uri = call(env, data, "getData", "()Landroid/net/Uri;", &[])?.l()?;
            if uri.is_null() {
                return Ok(());
            }
            enabled(env, activity, false)?;
            status(env, activity, "画像を準備中…")?;
            let work = callback(env, activity, APPEAR_WORK, 0, 0, &uri)?;
            let thread = env.new_object(
                "java/lang/Thread",
                "(Ljava/lang/Runnable;)V",
                &[JValue::Object(&work)],
            )?;
            call(env, &thread, "start", "()V", &[])?;
        }
        APPEAR_WORK => {
            let context = call(
                env,
                activity,
                "getApplicationContext",
                "()Landroid/content/Context;",
                &[],
            )?
            .l()?;
            let result = appearance::import_image(env, &context, data);
            let message = if result.is_ok() {
                "背景画像を保存しました。キーボードを開き直すと反映されます。"
            } else {
                if env.exception_check()? {
                    env.exception_clear()?;
                }
                "画像を読み込めませんでした。別の画像を選んでください。"
            };
            let message = env.new_string(message)?;
            let completed = callback(env, activity, APPEAR_COMPLETE, 0, 0, &message)?;
            call(
                env,
                activity,
                "runOnUiThread",
                "(Ljava/lang/Runnable;)V",
                &[JValue::Object(&completed)],
            )?;
        }
        APPEAR_COMPLETE => {
            if !call(env, activity, "isDestroyed", "()Z", &[])?.z()? {
                enabled(env, activity, true)?;
                refresh(env, activity)?;
                let message = string(env, data)?;
                status(env, activity, &message)?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
