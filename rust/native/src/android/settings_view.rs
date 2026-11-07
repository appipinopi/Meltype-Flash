// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Framework settings widgets, field extraction and edit application via JNI.
//! Java only supplies listener objects; no native global view refs are retained.
use super::*;
use serde::Deserialize;
use serde_json::{json, Map, Value};
type Result<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
struct Page {
    nodes: Vec<Node>,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Node {
    Label {
        text: String,
    },
    Field {
        id: String,
        hint: String,
        value: String,
        lines: i32,
    },
    Toggle {
        text: String,
        checked: bool,
        action: Value,
    },
    Button {
        text: String,
        action: Value,
    },
    Select {
        text: String,
        choices: Vec<String>,
        selected: i32,
        action: Value,
        enabled: bool,
    },
}
fn local<T>(
    env: &mut JNIEnv,
    capacity: i32,
    action: impl FnOnce(&mut JNIEnv) -> Result<T>,
) -> Result<T> {
    env.with_local_frame(capacity, |env| Ok::<_, jni::errors::Error>(action(env)))
        .map_err(|e| e.to_string())?
}
pub(super) fn invoke<'a>(
    env: &mut JNIEnv<'a>,
    object: &JObject,
    name: &str,
    signature: &str,
    args: &[JValue],
) -> Result<jni::objects::JValueOwned<'a>> {
    env.call_method(object, name, signature, args)
        .map_err(|e| e.to_string())
}
fn string(env: &mut JNIEnv, value: JString) -> Result<String> {
    env.get_string(&value)
        .map(String::from)
        .map_err(|e| e.to_string())
}
pub(super) fn property(env: &mut JNIEnv, view: &JObject, name: &str, text: &str) -> Result<()> {
    let text = env.new_string(text).map_err(|e| e.to_string())?;
    invoke(
        env,
        view,
        name,
        "(Ljava/lang/CharSequence;)V",
        &[JValue::Object(&text)],
    )?;
    Ok(())
}
fn body<'a>(env: &mut JNIEnv<'a>, bridge: &JObject) -> Result<JObject<'a>> {
    env.get_field(bridge, "body", "Landroid/widget/LinearLayout;")
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())
}
fn tag(env: &mut JNIEnv, view: &JObject, value: Value) -> Result<()> {
    let value = env
        .new_string(value.to_string())
        .map_err(|e| e.to_string())?;
    invoke(
        env,
        view,
        "setTag",
        "(Ljava/lang/Object;)V",
        &[JValue::Object(&value)],
    )?;
    Ok(())
}
fn node(
    env: &mut JNIEnv,
    bridge: &JObject,
    context: &JObject,
    body: &JObject,
    node: &Node,
) -> Result<()> {
    let class = match node {
        Node::Label { .. } => "android/widget/TextView",
        Node::Field { .. } => "android/widget/EditText",
        Node::Toggle { .. } => "android/widget/Switch",
        Node::Button { .. } => "android/widget/Button",
        Node::Select { .. } => "android/widget/Spinner",
    };
    let view = env
        .new_object(
            class,
            "(Landroid/content/Context;)V",
            &[JValue::Object(context)],
        )
        .map_err(|e| e.to_string())?;
    match node {
        Node::Label { text } => {
            property(env, &view, "setText", text)?;
            invoke(
                env,
                &view,
                "setTextIsSelectable",
                "(Z)V",
                &[JValue::Bool(1)],
            )?;
        }
        Node::Field {
            id,
            hint,
            value,
            lines,
        } => {
            property(env, &view, "setHint", hint)?;
            property(env, &view, "setText", value)?;
            invoke(env, &view, "setMinLines", "(I)V", &[JValue::Int(*lines)])?;
            tag(env, &view, json!({"field":id}))?;
        }
        Node::Toggle {
            text,
            checked,
            action,
        } => {
            property(env, &view, "setText", text)?;
            // Initial state must not run a settings mutation callback.
            invoke(
                env,
                &view,
                "setChecked",
                "(Z)V",
                &[JValue::Bool(u8::from(*checked))],
            )?;
            let serialized = env
                .new_string(action.to_string())
                .map_err(|e| e.to_string())?;
            let listener = invoke(
                env,
                bridge,
                "settingsToggleCallback",
                "(Ljava/lang/String;)Landroid/widget/CompoundButton$OnCheckedChangeListener;",
                &[JValue::Object(&serialized)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            invoke(
                env,
                &view,
                "setOnCheckedChangeListener",
                "(Landroid/widget/CompoundButton$OnCheckedChangeListener;)V",
                &[JValue::Object(&listener)],
            )?;
            tag(env, &view, json!({"toggle":action}))?;
        }
        Node::Select {
            text,
            choices,
            selected,
            action,
            enabled,
        } => {
            property(env, &view, "setContentDescription", text)?;
            let labels = env
                .new_object_array(choices.len() as i32, "java/lang/String", JObject::null())
                .map_err(|e| e.to_string())?;
            for (index, label) in choices.iter().enumerate() {
                let value = env.new_string(label).map_err(|e| e.to_string())?;
                env.set_object_array_element(&labels, index as i32, &value)
                    .map_err(|e| e.to_string())?;
            }
            let layout = env
                .get_static_field("android/R$layout", "simple_spinner_dropdown_item", "I")
                .and_then(|v| v.i())
                .map_err(|e| e.to_string())?;
            let adapter = env
                .new_object(
                    "android/widget/ArrayAdapter",
                    "(Landroid/content/Context;I[Ljava/lang/Object;)V",
                    &[
                        JValue::Object(context),
                        JValue::Int(layout),
                        JValue::Object(&labels),
                    ],
                )
                .map_err(|e| e.to_string())?;
            invoke(
                env,
                &view,
                "setAdapter",
                "(Landroid/widget/SpinnerAdapter;)V",
                &[JValue::Object(&adapter)],
            )?;
            invoke(
                env,
                &view,
                "setSelection",
                "(I)V",
                &[JValue::Int(*selected)],
            )?;
            invoke(
                env,
                &view,
                "setEnabled",
                "(Z)V",
                &[JValue::Bool(u8::from(*enabled))],
            )?;
            let serialized = env
                .new_string(action.to_string())
                .map_err(|e| e.to_string())?;
            let listener = invoke(
                env,
                bridge,
                "settingsSelectCallback",
                "(Ljava/lang/String;)Landroid/widget/AdapterView$OnItemSelectedListener;",
                &[JValue::Object(&serialized)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            invoke(
                env,
                &view,
                "setOnItemSelectedListener",
                "(Landroid/widget/AdapterView$OnItemSelectedListener;)V",
                &[JValue::Object(&listener)],
            )?;
        }
        Node::Button { text, action } => {
            property(env, &view, "setText", text)?;
            let serialized = env
                .new_string(action.to_string())
                .map_err(|e| e.to_string())?;
            let listener = invoke(
                env,
                bridge,
                "settingsButtonCallback",
                "(Ljava/lang/String;)Landroid/view/View$OnClickListener;",
                &[JValue::Object(&serialized)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            invoke(
                env,
                &view,
                "setOnClickListener",
                "(Landroid/view/View$OnClickListener;)V",
                &[JValue::Object(&listener)],
            )?;
        }
    }
    invoke(
        env,
        body,
        "addView",
        "(Landroid/view/View;)V",
        &[JValue::Object(&view)],
    )?;
    Ok(())
}
pub(super) fn render(env: &mut JNIEnv, bridge: JObject, input: JString) -> Result<Value> {
    let page: Page = serde_json::from_str(&string(env, input)?).map_err(|e| e.to_string())?;
    // Validate the complete definition before removing the current page.
    let mut ids = std::collections::HashSet::new();
    for node in &page.nodes {
        match node {
            Node::Field { id, lines, .. } if id.is_empty() || *lines < 1 || !ids.insert(id) => {
                return Err("入力項目の定義が正しくありません".into())
            }
            Node::Toggle { action, .. } | Node::Button { action, .. } if !action.is_object() => {
                return Err("操作の定義が正しくありません".into())
            }
            _ => {}
        }
    }
    let body = body(env, &bridge)?;
    let context = env
        .get_field(&bridge, "context", "Landroid/content/Context;")
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    invoke(env, &body, "removeAllViews", "()V", &[])?;
    for definition in &page.nodes {
        local(env, 24, |env| {
            node(env, &bridge, &context, &body, definition)
        })?;
    }
    Ok(json!({}))
}
pub(super) fn fields(env: &mut JNIEnv, bridge: JObject, operation: JString) -> Result<Value> {
    let operation: Value =
        serde_json::from_str(&string(env, operation)?).map_err(|e| e.to_string())?;
    let body = body(env, &bridge)?;
    let count = invoke(env, &body, "getChildCount", "()I", &[])?
        .i()
        .map_err(|e| e.to_string())?;
    let mut result = Map::new();
    for index in 0..count {
        local(env, 16, |env| -> Result<()> {
            let view = invoke(
                env,
                &body,
                "getChildAt",
                "(I)Landroid/view/View;",
                &[JValue::Int(index)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            let tag = invoke(env, &view, "getTag", "()Ljava/lang/Object;", &[])?
                .l()
                .map_err(|e| e.to_string())?;
            if tag.is_null() {
                return Ok(());
            }
            let tag: Value = serde_json::from_str(&string(env, JString::from(tag))?)
                .map_err(|e| e.to_string())?;
            if let Some(id) = tag["field"].as_str() {
                let text = invoke(env, &view, "getText", "()Landroid/text/Editable;", &[])?
                    .l()
                    .map_err(|e| e.to_string())?;
                let text = invoke(env, &text, "toString", "()Ljava/lang/String;", &[])?
                    .l()
                    .map_err(|e| e.to_string())?;
                result.insert(id.into(), Value::String(string(env, JString::from(text))?));
            }
            if tag.get("toggle") == Some(&operation) {
                let checked = invoke(env, &view, "isChecked", "()Z", &[])?
                    .z()
                    .map_err(|e| e.to_string())?;
                result.insert("checked".into(), Value::Bool(checked));
            }
            Ok(())
        })?;
    }
    Ok(Value::Object(result))
}
pub(super) fn edit(env: &mut JNIEnv, bridge: JObject, input: JString) -> Result<Value> {
    let edits: Map<String, Value> =
        serde_json::from_str(&string(env, input)?).map_err(|e| e.to_string())?;
    if edits.values().any(|value| !value.is_string()) {
        return Err("入力値が正しくありません".into());
    }
    let body = body(env, &bridge)?;
    let count = invoke(env, &body, "getChildCount", "()I", &[])?
        .i()
        .map_err(|e| e.to_string())?;
    let mut targets = Vec::new();
    // Resolve every field before writing, so a missing field cannot partly edit.
    for index in 0..count {
        local(env, 16, |env| {
            let view = invoke(
                env,
                &body,
                "getChildAt",
                "(I)Landroid/view/View;",
                &[JValue::Int(index)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            let tag = invoke(env, &view, "getTag", "()Ljava/lang/Object;", &[])?
                .l()
                .map_err(|e| e.to_string())?;
            if !tag.is_null() {
                let tag: Value = serde_json::from_str(&string(env, JString::from(tag))?)
                    .map_err(|e| e.to_string())?;
                if let Some(id) = tag["field"].as_str() {
                    if let Some(value) = edits.get(id) {
                        targets.push((index, value.as_str().unwrap().to_owned()));
                    }
                }
            }
            Ok(())
        })?;
    }
    if targets.len() != edits.len() {
        return Err("入力項目が見つかりません".into());
    }
    for (index, value) in targets {
        local(env, 8, |env| {
            let view = invoke(
                env,
                &body,
                "getChildAt",
                "(I)Landroid/view/View;",
                &[JValue::Int(index)],
            )?
            .l()
            .map_err(|e| e.to_string())?;
            property(env, &view, "setText", &value)
        })?;
    }
    Ok(json!({}))
}
fn finish(env: &mut JNIEnv, result: Result<Value>) -> jstring {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    settings::reply(env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_renderSettingsPage(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
    input: JString,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render(&mut env, bridge, input)
    }))
    .unwrap_or_else(|_| Err("設定画面の描画に失敗しました".into()));
    finish(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_settingsValues(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
    operation: JString,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fields(&mut env, bridge, operation)
    }))
    .unwrap_or_else(|_| Err("入力値を取得できません".into()));
    finish(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editSettingsFields(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
    input: JString,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        edit(&mut env, bridge, input)
    }))
    .unwrap_or_else(|_| Err("入力値を設定できません".into()));
    finish(&mut env, result)
}
