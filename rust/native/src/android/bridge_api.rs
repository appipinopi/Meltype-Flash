// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 生成アダプターを介してAndroidのAPIをRustから呼び出す。
use super::*;
pub(super) type Result<T> = jni::errors::Result<T>;
pub(super) fn call<'a>(
    env: &mut JNIEnv<'a>,
    object: &JObject,
    method: &str,
    signature: &str,
    args: &[JValue],
) -> Result<jni::objects::JValueOwned<'a>> {
    env.call_method(object, method, signature, args)
}
pub(super) fn native<'a>(
    env: &mut JNIEnv<'a>,
    method: &str,
    signature: &str,
    args: &[JValue],
) -> Result<jni::objects::JValueOwned<'a>> {
    env.call_static_method("moe/meltype/rust/Native", method, signature, args)
}
pub(super) fn widget<'a>(
    env: &mut JNIEnv<'a>,
    class: &str,
    context: &JObject,
) -> Result<JObject<'a>> {
    env.new_object(
        class,
        "(Landroid/content/Context;)V",
        &[JValue::Object(context)],
    )
}
pub(super) fn text(env: &mut JNIEnv, object: &JObject, method: &str, text: &str) -> Result<()> {
    let text = env.new_string(text)?;
    call(
        env,
        object,
        method,
        "(Ljava/lang/CharSequence;)V",
        &[JValue::Object(&text)],
    )?;
    Ok(())
}
pub(super) fn string(env: &mut JNIEnv, object: &JObject) -> Result<String> {
    let string = JString::from(env.new_local_ref(object)?);
    let value = env.get_string(&string)?.into();
    Ok(value)
}
pub(super) fn object<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    name: &str,
    signature: &str,
) -> Result<JObject<'a>> {
    env.get_field(owner, name, signature)?.l()
}
pub(super) fn set_object(
    env: &mut JNIEnv,
    owner: &JObject,
    name: &str,
    signature: &str,
    value: &JObject,
) -> Result<()> {
    env.set_field(owner, name, signature, JValue::Object(value))
}
pub(super) fn integer(env: &mut JNIEnv, owner: &JObject, name: &str) -> Result<i32> {
    env.get_field(owner, name, "I")?.i()
}
pub(super) fn set_integer(env: &mut JNIEnv, owner: &JObject, name: &str, value: i32) -> Result<()> {
    env.set_field(owner, name, "I", JValue::Int(value))
}
pub(super) fn boolean(env: &mut JNIEnv, owner: &JObject, name: &str) -> Result<bool> {
    env.get_field(owner, name, "Z")?.z()
}
pub(super) fn set_boolean(
    env: &mut JNIEnv,
    owner: &JObject,
    name: &str,
    value: bool,
) -> Result<()> {
    env.set_field(owner, name, "Z", JValue::Bool(u8::from(value)))
}
pub(super) fn add(env: &mut JNIEnv, parent: &JObject, child: &JObject) -> Result<()> {
    call(
        env,
        parent,
        "addView",
        "(Landroid/view/View;)V",
        &[JValue::Object(child)],
    )?;
    Ok(())
}
pub(super) fn callback<'a>(
    env: &mut JNIEnv<'a>,
    owner: &JObject,
    event: i32,
    value: i32,
    detail: i32,
    data: &JObject,
) -> Result<JObject<'a>> {
    env.new_object(
        "moe/meltype/rust/BridgeCallback",
        "(Ljava/lang/Object;IIILjava/lang/Object;)V",
        &[
            JValue::Object(owner),
            JValue::Int(event),
            JValue::Int(value),
            JValue::Int(detail),
            JValue::Object(data),
        ],
    )
}
pub(super) fn click(env: &mut JNIEnv, button: &JObject, callback: &JObject) -> Result<()> {
    call(
        env,
        button,
        "setOnClickListener",
        "(Landroid/view/View$OnClickListener;)V",
        &[JValue::Object(callback)],
    )?;
    Ok(())
}
pub(super) fn density(env: &mut JNIEnv, context: &JObject) -> Result<f32> {
    let resources = call(
        env,
        context,
        "getResources",
        "()Landroid/content/res/Resources;",
        &[],
    )?
    .l()?;
    let metrics = call(
        env,
        &resources,
        "getDisplayMetrics",
        "()Landroid/util/DisplayMetrics;",
        &[],
    )?
    .l()?;
    env.get_field(metrics, "density", "F")?.f()
}
pub(super) fn session(env: &mut JNIEnv, owner: &JObject) -> Result<jlong> {
    env.get_field(owner, "session", "J")?.j()
}
pub(super) fn create_session(env: &mut JNIEnv, owner: &JObject) -> Result<jlong> {
    let file = call(env, owner, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;
    let path = call(env, &file, "getAbsolutePath", "()Ljava/lang/String;", &[])?.l()?;
    let handle = native(
        env,
        "create",
        "(Ljava/lang/String;)J",
        &[JValue::Object(&path)],
    )?
    .j()?;
    env.set_field(owner, "session", "J", JValue::Long(handle))?;
    Ok(handle)
}
pub(super) fn destroy_session(env: &mut JNIEnv, owner: &JObject) -> Result<()> {
    let handle = session(env, owner)?;
    native(env, "destroy", "(J)V", &[JValue::Long(handle)])?;
    env.set_field(owner, "session", "J", JValue::Long(0))
}
