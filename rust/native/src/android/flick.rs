// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! フリックの画面構築・方向判定・キャンセル・入力操作をRustで管理する。
use super::{bridge_api::*, *};
use jni::objects::JObjectArray;
const GROUPS: [&str; 12] = [
    "あいうえお",
    "かきくけこ",
    "さしすせそ",
    "たちつてと",
    "なにぬねの",
    "はひふへほ",
    "まみむめも",
    "や（ゆ）よ",
    "らりるれろ",
    "",
    "わをんー～",
    "、。？！…",
];
pub(super) fn direction(dx: f32, dy: f32, threshold: f32) -> i32 {
    if dx.abs().max(dy.abs()) < threshold {
        0
    } else if dx.abs() > dy.abs() {
        if dx < 0. {
            1
        } else {
            3
        }
    } else if dy < 0. {
        2
    } else {
        4
    }
}
pub(super) fn groups<'a>(env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
    let array = env.new_object_array(12, "java/lang/String", JObject::null())?;
    for (index, group) in GROUPS.iter().enumerate() {
        let text = env.new_string(group)?;
        env.set_object_array_element(&array, index as i32, text)?;
    }
    Ok(array.into())
}
fn array_field<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    name: &str,
    signature: &str,
) -> Result<JObjectArray<'a>> {
    Ok(object(env, owner, name, signature)?.into())
}
fn reset(env: &mut JNIEnv, gesture: &JObject) -> Result<()> {
    set_integer(env, gesture, "pointer", -1)?;
    set_integer(env, gesture, "choice", 0)?;
    let key = object(env, gesture, "key", "Landroid/widget/Button;")?;
    call(env, &key, "setPressed", "(Z)V", &[JValue::Bool(0)])?;
    let label = object(env, gesture, "label", "Ljava/lang/String;")?;
    call(
        env,
        &key,
        "setText",
        "(Ljava/lang/CharSequence;)V",
        &[JValue::Object(&label)],
    )?;
    Ok(())
}
pub(super) fn cancel(env: &mut JNIEnv, keyboard: &JObject) -> Result<()> {
    if keyboard.is_null() {
        return Ok(());
    }
    let gestures = array_field(
        env,
        keyboard,
        "gestures",
        "[Lmoe/meltype/rust/FlickGesture;",
    )?;
    for index in 0..env.get_array_length(&gestures)? {
        env.with_local_frame(12, |env| {
            let gesture = env.get_object_array_element(&gestures, index)?;
            reset(env, &gesture)
        })?;
    }
    Ok(())
}
fn show(env: &mut JNIEnv, keyboard: &JObject, enabled: bool) -> Result<()> {
    cancel(env, keyboard)?;
    set_boolean(env, keyboard, "flick", enabled)?;
    let rows = array_field(env, keyboard, "romanRows", "[Landroid/view/View;")?;
    for index in 0..5 {
        let row = env.get_object_array_element(&rows, index)?;
        call(
            env,
            &row,
            "setVisibility",
            "(I)V",
            &[JValue::Int(if enabled { 8 } else { 0 })],
        )?;
    }
    let panel = object(env, keyboard, "panel", "Landroid/widget/LinearLayout;")?;
    call(
        env,
        &panel,
        "setVisibility",
        "(I)V",
        &[JValue::Int(if enabled { 0 } else { 8 })],
    )?;
    let toggle = object(env, keyboard, "toggle", "Landroid/widget/Button;")?;
    text(
        env,
        &toggle,
        "setText",
        if enabled {
            "QWERTYに切替"
        } else {
            "かなフリックに切替"
        },
    )
}
pub(super) fn toggle(env: &mut JNIEnv, keyboard: &JObject) -> Result<()> {
    let enabled = !boolean(env, keyboard, "flick")?;
    show(env, keyboard, enabled)?;
    let listener = object(
        env,
        keyboard,
        "listener",
        "Lmoe/meltype/rust/FlickKeyboard$Listener;",
    )?;
    call(
        env,
        &listener,
        "layout",
        "(Z)V",
        &[JValue::Bool(u8::from(enabled))],
    )?;
    Ok(())
}
pub(super) fn attach<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject,
    root: &JObject,
    listener: &JObject,
    enabled: bool,
) -> Result<JObject<'a>> {
    let keyboard = env.new_object("moe/meltype/rust/FlickKeyboard", "()V", &[])?;
    set_object(
        env,
        &keyboard,
        "listener",
        "Lmoe/meltype/rust/FlickKeyboard$Listener;",
        listener,
    )?;
    let rows = env.new_object_array(5, "android/view/View", JObject::null())?;
    for index in 0..5 {
        let row = call(
            env,
            root,
            "getChildAt",
            "(I)Landroid/view/View;",
            &[JValue::Int(index + 2)],
        )?
        .l()?;
        env.set_object_array_element(&rows, index, row)?;
    }
    set_object(env, &keyboard, "romanRows", "[Landroid/view/View;", &rows)?;
    let gestures = env.new_object_array(12, "moe/meltype/rust/FlickGesture", JObject::null())?;
    set_object(
        env,
        &keyboard,
        "gestures",
        "[Lmoe/meltype/rust/FlickGesture;",
        &gestures,
    )?;
    let panel = widget(env, "android/widget/LinearLayout", context)?;
    call(env, &panel, "setOrientation", "(I)V", &[JValue::Int(1)])?;
    let density = density(env, context)?;
    for row_index in 0..4 {
        env.with_local_frame(32, |env| -> Result<()> {
            let row = widget(env, "android/widget/LinearLayout", context)?;
            call(env, &row, "setBaselineAligned", "(Z)V", &[JValue::Bool(0)])?;
            for column in 0..3 {
                env.with_local_frame(32, |env| -> Result<()> {
                    let index = row_index * 3 + column;
                    let group = GROUPS[index];
                    let key = widget(env, "android/widget/Button", context)?;
                    for method in ["setMinWidth", "setMinimumWidth"] {
                        call(env, &key, method, "(I)V", &[JValue::Int(0)])?;
                    }
                    call(
                        env,
                        &key,
                        "setMinHeight",
                        "(I)V",
                        &[JValue::Int((64. * density).round() as i32)],
                    )?;
                    call(env, &key, "setTextSize", "(F)V", &[JValue::Float(16.)])?;
                    call(env, &key, "setPadding", "(IIII)V", &[JValue::Int(1); 4])?;
                    let gesture = env.new_object("moe/meltype/rust/FlickGesture", "()V", &[])?;
                    set_object(env, &gesture, "key", "Landroid/widget/Button;", &key)?;
                    set_object(
                        env,
                        &gesture,
                        "listener",
                        "Lmoe/meltype/rust/FlickKeyboard$Listener;",
                        listener,
                    )?;
                    let chars: Vec<char> = group.chars().collect();
                    let label = if chars.is_empty() {
                        "゛゜小".to_string()
                    } else {
                        format!(
                            "↑{}\n{}  {}  {}\n↓{}",
                            chars[2], chars[1], chars[0], chars[3], chars[4]
                        )
                    };
                    let group = env.new_string(group)?;
                    let saved_label = env.new_string(&label)?;
                    set_object(env, &gesture, "group", "Ljava/lang/String;", &group)?;
                    set_object(env, &gesture, "label", "Ljava/lang/String;", &saved_label)?;
                    env.set_field(&gesture, "threshold", "F", JValue::Float(24. * density))?;
                    text(env, &key, "setText", &label)?;
                    let description = if chars.is_empty() {
                        "直前のかなの濁点・半濁点・小書きを切替".to_string()
                    } else {
                        format!(
                            "{}：左 {}、上 {}、右 {}、下 {}",
                            chars[0], chars[1], chars[2], chars[3], chars[4]
                        )
                    };
                    text(env, &key, "setContentDescription", &description)?;
                    click(env, &key, &gesture)?;
                    if !chars.is_empty() {
                        call(
                            env,
                            &key,
                            "setOnTouchListener",
                            "(Landroid/view/View$OnTouchListener;)V",
                            &[JValue::Object(&gesture)],
                        )?;
                    }
                    env.set_object_array_element(&gestures, index as i32, &gesture)?;
                    let params = env.new_object(
                        "android/widget/LinearLayout$LayoutParams",
                        "(IIF)V",
                        &[JValue::Int(0), JValue::Int(-2), JValue::Float(1.)],
                    )?;
                    call(
                        env,
                        &row,
                        "addView",
                        "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
                        &[JValue::Object(&key), JValue::Object(&params)],
                    )?;
                    Ok(())
                })?;
            }
            add(env, &panel, &row)
        })?;
    }
    set_object(
        env,
        &keyboard,
        "panel",
        "Landroid/widget/LinearLayout;",
        &panel,
    )?;
    call(
        env,
        root,
        "addView",
        "(Landroid/view/View;I)V",
        &[JValue::Object(&panel), JValue::Int(2)],
    )?;
    let toggle = widget(env, "android/widget/Button", context)?;
    click(env, &toggle, &keyboard)?;
    add(env, root, &toggle)?;
    set_object(env, &keyboard, "toggle", "Landroid/widget/Button;", &toggle)?;
    show(env, &keyboard, enabled)?;
    Ok(keyboard)
}
pub(super) fn tap(env: &mut JNIEnv, gesture: &JObject) -> Result<()> {
    let group = object(env, gesture, "group", "Ljava/lang/String;")?;
    let group = string(env, &group)?;
    let index = integer(env, gesture, "choice")?;
    set_integer(env, gesture, "choice", 0)?;
    let listener = object(
        env,
        gesture,
        "listener",
        "Lmoe/meltype/rust/FlickKeyboard$Listener;",
    )?;
    if group.is_empty() {
        call(env, &listener, "variant", "(I)V", &[JValue::Int(0)])?;
    } else if let Some(scalar) = group.chars().nth(index as usize) {
        call(env, &listener, "key", "(I)V", &[JValue::Int(scalar as i32)])?;
    }
    Ok(())
}
pub(super) fn touch(env: &mut JNIEnv, gesture: &JObject, event: &JObject) -> Result<bool> {
    let action = call(env, event, "getActionMasked", "()I", &[])?.i()?;
    if action == 3 || call(env, event, "getPointerCount", "()I", &[])?.i()? != 1 {
        reset(env, gesture)?;
        return Ok(true);
    }
    let id = call(env, event, "getPointerId", "(I)I", &[JValue::Int(0)])?.i()?;
    let key = object(env, gesture, "key", "Landroid/widget/Button;")?;
    let x = call(env, event, "getX", "()F", &[])?.f()?;
    let y = call(env, event, "getY", "()F", &[])?.f()?;
    if action == 0 {
        set_integer(env, gesture, "pointer", id)?;
        set_integer(env, gesture, "choice", 0)?;
        env.set_field(gesture, "x", "F", JValue::Float(x))?;
        env.set_field(gesture, "y", "F", JValue::Float(y))?;
        call(env, &key, "setPressed", "(Z)V", &[JValue::Bool(1)])?;
        return Ok(true);
    }
    let pointer = integer(env, gesture, "pointer")?;
    if pointer < 0 || pointer != id {
        return Ok(true);
    }
    let dx = x - env.get_field(gesture, "x", "F")?.f()?;
    let dy = y - env.get_field(gesture, "y", "F")?.f()?;
    let threshold = env.get_field(gesture, "threshold", "F")?.f()?;
    let choice = direction(dx, dy, threshold);
    set_integer(env, gesture, "choice", choice)?;
    if action == 2 {
        let group = object(env, gesture, "group", "Ljava/lang/String;")?;
        let group = string(env, &group)?;
        if let Some(scalar) = group.chars().nth(choice as usize) {
            text(env, &key, "setText", &scalar.to_string())?;
        }
    } else if action == 1 {
        set_integer(env, gesture, "pointer", -1)?;
        call(env, &key, "setPressed", "(Z)V", &[JValue::Bool(0)])?;
        let label = object(env, gesture, "label", "Ljava/lang/String;")?;
        call(
            env,
            &key,
            "setText",
            "(Ljava/lang/CharSequence;)V",
            &[JValue::Object(&label)],
        )?;
        call(env, &key, "performClick", "()Z", &[])?;
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directions_preserve_threshold_and_vertical_ties() {
        assert_eq!(direction(23., 0., 24.), 0);
        assert_eq!(direction(-24., 0., 24.), 1);
        assert_eq!(direction(0., -24., 24.), 2);
        assert_eq!(direction(24., 0., 24.), 3);
        assert_eq!(direction(0., 24., 24.), 4);
        assert_eq!(direction(24., -24., 24.), 2);
        assert_eq!(direction(-24., 24., 24.), 4);
    }
}
