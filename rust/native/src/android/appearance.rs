// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 背景画像の取込み・保存・キャッシュ・描画と配色をRustで管理する。
use super::{bridge::events::*, bridge_api::*, *};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
pub(super) const THEMES: [(&str, &str); 6] = [
    ("system", "システムに合わせる"),
    ("light", "ライト"),
    ("dark", "ダーク"),
    ("blue", "ブルー"),
    ("pink", "ピンク"),
    ("mint", "ミント"),
];
static IMAGE_IO: Mutex<()> = Mutex::new(());
static IMAGE_REQUEST: AtomicU64 = AtomicU64::new(0);
pub(super) fn new<'a>(env: &mut JNIEnv<'a>, context: &JObject) -> Result<JObject<'a>> {
    env.new_object(
        "moe/meltype/rust/KeyboardAppearance",
        "(Landroid/content/Context;)V",
        &[JValue::Object(context)],
    )
}
pub(super) fn preferences_named<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject,
    name: &str,
) -> Result<JObject<'a>> {
    let name = env.new_string(name)?;
    call(
        env,
        context,
        "getSharedPreferences",
        "(Ljava/lang/String;I)Landroid/content/SharedPreferences;",
        &[JValue::Object(&name), JValue::Int(0)],
    )?
    .l()
}
pub(super) fn preferences<'a>(env: &mut JNIEnv<'a>, context: &JObject) -> Result<JObject<'a>> {
    preferences_named(env, context, "appearance")
}
fn edit<'a>(env: &mut JNIEnv<'a>, preferences: &JObject) -> Result<JObject<'a>> {
    call(
        env,
        preferences,
        "edit",
        "()Landroid/content/SharedPreferences$Editor;",
        &[],
    )?
    .l()
}
pub(super) fn get_string(
    env: &mut JNIEnv,
    preferences: &JObject,
    key: &str,
    default: &str,
) -> Result<String> {
    let key = env.new_string(key)?;
    let default = env.new_string(default)?;
    let value = call(
        env,
        preferences,
        "getString",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[JValue::Object(&key), JValue::Object(&default)],
    )?
    .l()?;
    string(env, &value)
}
fn put_string(
    env: &mut JNIEnv,
    preferences: &JObject,
    key: &str,
    value: &str,
    commit: bool,
) -> Result<()> {
    let editor = edit(env, preferences)?;
    let key = env.new_string(key)?;
    let value = env.new_string(value)?;
    call(
        env,
        &editor,
        "putString",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;",
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    if commit {
        if !call(env, &editor, "commit", "()Z", &[])?.z()? {
            return failure(env, "背景の設定を保存できませんでした。");
        }
    } else {
        call(env, &editor, "apply", "()V", &[])?;
    }
    Ok(())
}
pub(super) fn put_bool(
    env: &mut JNIEnv,
    preferences: &JObject,
    key: &str,
    value: bool,
) -> Result<()> {
    let editor = edit(env, preferences)?;
    let key = env.new_string(key)?;
    call(
        env,
        &editor,
        "putBoolean",
        "(Ljava/lang/String;Z)Landroid/content/SharedPreferences$Editor;",
        &[JValue::Object(&key), JValue::Bool(u8::from(value))],
    )?;
    call(env, &editor, "apply", "()V", &[])?;
    Ok(())
}
pub(super) fn set_theme(env: &mut JNIEnv, context: &JObject, value: &str) -> Result<()> {
    if !THEMES.iter().any(|(theme, _)| *theme == value) {
        return failure(env, "対応していないテーマです。");
    }
    let preferences = preferences(env, context)?;
    put_string(env, &preferences, "theme", value, false)
}
pub(super) fn image_file<'a>(env: &mut JNIEnv<'a>, context: &JObject) -> Result<JObject<'a>> {
    let directory = call(env, context, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;
    let name = env.new_string("keyboard-background.jpg")?;
    let file = env.new_object(
        "java/io/File",
        "(Ljava/io/File;Ljava/lang/String;)V",
        &[JValue::Object(&directory), JValue::Object(&name)],
    )?;
    env.new_object(
        "android/util/AtomicFile",
        "(Ljava/io/File;)V",
        &[JValue::Object(&file)],
    )
}
pub(super) fn clear_image(env: &mut JNIEnv, context: &JObject) -> Result<()> {
    let _lock = IMAGE_IO.lock().unwrap_or_else(|error| error.into_inner());
    IMAGE_REQUEST.fetch_add(1, Ordering::SeqCst);
    let preferences = preferences(env, context)?;
    let editor = edit(env, &preferences)?;
    let key = env.new_string("imageRevision")?;
    call(
        env,
        &editor,
        "remove",
        "(Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;",
        &[JValue::Object(&key)],
    )?;
    call(env, &editor, "apply", "()V", &[])?;
    let file = image_file(env, context)?;
    call(env, &file, "delete", "()V", &[])?;
    Ok(())
}
fn failure<T>(env: &mut JNIEnv, message: &str) -> Result<T> {
    env.throw_new("java/io/IOException", message)?;
    Err(jni::errors::Error::JavaException)
}
// 例外が発生していてもOSリソースを閉じ、元の例外を保持する。
fn cleanup(env: &mut JNIEnv, operation: impl FnOnce(&mut JNIEnv) -> Result<()>) -> Result<()> {
    let original = env.exception_occurred()?;
    if !original.is_null() {
        env.exception_clear()?;
    }
    let result = operation(env);
    if !original.is_null() {
        if env.exception_check()? {
            env.exception_clear()?;
        }
        env.throw(original)?;
    }
    result
}
fn close_after<T>(env: &mut JNIEnv, stream: &JObject, result: Result<T>) -> Result<T> {
    let closed = cleanup(env, |env| {
        call(env, stream, "close", "()V", &[])?;
        Ok(())
    });
    result.and_then(|value| closed.map(|()| value))
}
fn decode<'a>(
    env: &mut JNIEnv<'a>,
    resolver: &JObject,
    uri: &JObject,
    options: &JObject,
) -> Result<JObject<'a>> {
    let input = call(
        env,
        resolver,
        "openInputStream",
        "(Landroid/net/Uri;)Ljava/io/InputStream;",
        &[JValue::Object(uri)],
    )?
    .l()?;
    if input.is_null() {
        return failure(env, "画像を開けませんでした。");
    }
    let result=env.call_static_method("android/graphics/BitmapFactory","decodeStream","(Ljava/io/InputStream;Landroid/graphics/Rect;Landroid/graphics/BitmapFactory$Options;)Landroid/graphics/Bitmap;",&[
        JValue::Object(&input),JValue::Object(&JObject::null()),JValue::Object(options)]).and_then(|value|value.l());
    close_after(env, &input, result)
}
fn orientation(env: &mut JNIEnv, resolver: &JObject, uri: &JObject) -> Result<i32> {
    let input = call(
        env,
        resolver,
        "openInputStream",
        "(Landroid/net/Uri;)Ljava/io/InputStream;",
        &[JValue::Object(uri)],
    )?
    .l()?;
    if input.is_null() {
        return Ok(1);
    }
    let result = (|| -> Result<i32> {
        let exif = env.new_object(
            "android/media/ExifInterface",
            "(Ljava/io/InputStream;)V",
            &[JValue::Object(&input)],
        )?;
        let tag = env.new_string("Orientation")?;
        call(
            env,
            &exif,
            "getAttributeInt",
            "(Ljava/lang/String;I)I",
            &[JValue::Object(&tag), JValue::Int(1)],
        )?
        .i()
    })();
    // EXIF未対応の画像も使える。読取例外を破棄した後にストリームを閉じる。
    if result.is_err() && env.exception_check()? {
        env.exception_clear()?;
    }
    call(env, &input, "close", "()V", &[])?;
    Ok(result.unwrap_or(1))
}
pub(super) fn import_image(env: &mut JNIEnv, context: &JObject, uri: &JObject) -> Result<()> {
    let ticket = IMAGE_REQUEST.fetch_add(1, Ordering::SeqCst) + 1;
    let resolver = call(
        env,
        context,
        "getContentResolver",
        "()Landroid/content/ContentResolver;",
        &[],
    )?
    .l()?;
    let options = env.new_object("android/graphics/BitmapFactory$Options", "()V", &[])?;
    set_boolean(env, &options, "inJustDecodeBounds", true)?;
    decode(env, &resolver, uri, &options)?;
    let width = integer(env, &options, "outWidth")?;
    let height = integer(env, &options, "outHeight")?;
    if width <= 0 || height <= 0 {
        return failure(env, "画像を読み込めませんでした。");
    }
    let mut sample = 1;
    while width.max(height) / sample > 1280 {
        sample *= 2;
    }
    set_boolean(env, &options, "inJustDecodeBounds", false)?;
    set_integer(env, &options, "inSampleSize", sample)?;
    let mut bitmap = decode(env, &resolver, uri, &options)?;
    if bitmap.is_null() {
        return failure(env, "対応していない画像です。");
    }
    let result = (|| -> Result<()> {
        let orientation = orientation(env, &resolver, uri)?;
        let matrix = env.new_object("android/graphics/Matrix", "()V", &[])?;
        match orientation {
            2 => {
                call(
                    env,
                    &matrix,
                    "setScale",
                    "(FF)V",
                    &[JValue::Float(-1.), JValue::Float(1.)],
                )?;
            }
            3 => {
                call(env, &matrix, "setRotate", "(F)V", &[JValue::Float(180.)])?;
            }
            4 => {
                call(
                    env,
                    &matrix,
                    "setScale",
                    "(FF)V",
                    &[JValue::Float(1.), JValue::Float(-1.)],
                )?;
            }
            5..=8 => {
                call(
                    env,
                    &matrix,
                    "setRotate",
                    "(F)V",
                    &[JValue::Float(if orientation <= 6 { 90. } else { 270. })],
                )?;
                if orientation == 5 || orientation == 7 {
                    call(
                        env,
                        &matrix,
                        "postScale",
                        "(FF)Z",
                        &[JValue::Float(-1.), JValue::Float(1.)],
                    )?;
                }
            }
            _ => {}
        }
        let width = call(env, &bitmap, "getWidth", "()I", &[])?.i()?;
        let height = call(env, &bitmap, "getHeight", "()I", &[])?.i()?;
        let oriented=env.call_static_method("android/graphics/Bitmap","createBitmap","(Landroid/graphics/Bitmap;IIIILandroid/graphics/Matrix;Z)Landroid/graphics/Bitmap;",&[
            JValue::Object(&bitmap),JValue::Int(0),JValue::Int(0),JValue::Int(width),JValue::Int(height),JValue::Object(&matrix),JValue::Bool(1)])?.l()?;
        if !env.is_same_object(&bitmap, &oriented)? {
            call(env, &bitmap, "recycle", "()V", &[])?;
            bitmap = oriented;
        }
        let _lock = IMAGE_IO.lock().unwrap_or_else(|error| error.into_inner());
        if IMAGE_REQUEST.load(Ordering::SeqCst) != ticket {
            return failure(env, "背景はすでに変更されています。");
        }
        let file = image_file(env, context)?;
        let output = call(
            env,
            &file,
            "startWrite",
            "()Ljava/io/FileOutputStream;",
            &[],
        )?
        .l()?;
        let written = (|| -> Result<()> {
            let jpeg = env
                .get_static_field(
                    "android/graphics/Bitmap$CompressFormat",
                    "JPEG",
                    "Landroid/graphics/Bitmap$CompressFormat;",
                )?
                .l()?;
            if !call(
                env,
                &bitmap,
                "compress",
                "(Landroid/graphics/Bitmap$CompressFormat;ILjava/io/OutputStream;)Z",
                &[
                    JValue::Object(&jpeg),
                    JValue::Int(88),
                    JValue::Object(&output),
                ],
            )?
            .z()?
            {
                return failure(env, "画像を保存できませんでした。");
            }
            call(
                env,
                &file,
                "finishWrite",
                "(Ljava/io/FileOutputStream;)V",
                &[JValue::Object(&output)],
            )?;
            Ok(())
        })();
        if written.is_err() {
            let _ = cleanup(env, |env| {
                call(
                    env,
                    &file,
                    "failWrite",
                    "(Ljava/io/FileOutputStream;)V",
                    &[JValue::Object(&output)],
                )?;
                Ok(())
            });
        }
        written?;
        let preferences = preferences(env, context)?;
        let uuid = env
            .call_static_method("java/util/UUID", "randomUUID", "()Ljava/util/UUID;", &[])?
            .l()?;
        let revision = call(env, &uuid, "toString", "()Ljava/lang/String;", &[])?.l()?;
        let revision = string(env, &revision)?;
        put_string(env, &preferences, "imageRevision", &revision, true)
    })();
    let recycled = cleanup(env, |env| {
        call(env, &bitmap, "recycle", "()V", &[])?;
        Ok(())
    });
    result.and(recycled)
}
fn cached_image<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject,
    revision: &str,
) -> Result<JObject<'a>> {
    if revision.is_empty() {
        return Ok(JObject::null());
    }
    let _lock = IMAGE_IO.lock().unwrap_or_else(|error| error.into_inner());
    let file = image_file(env, context)?;
    let result = (|| -> Result<JObject<'a>> {
        let input = call(env, &file, "openRead", "()Ljava/io/FileInputStream;", &[])?.l()?;
        let decoded = env
            .call_static_method(
                "android/graphics/BitmapFactory",
                "decodeStream",
                "(Ljava/io/InputStream;)Landroid/graphics/Bitmap;",
                &[JValue::Object(&input)],
            )
            .and_then(|value| value.l());
        close_after(env, &input, decoded)
    })();
    match result {
        Ok(image) => Ok(image),
        Err(_) => {
            if env.exception_check()? {
                env.exception_clear()?;
            }
            Ok(JObject::null())
        }
    }
}
pub(super) fn apply(env: &mut JNIEnv, appearance: &JObject, root: &JObject) -> Result<()> {
    let context = object(env, appearance, "context", "Landroid/content/Context;")?;
    let preferences = preferences(env, &context)?;
    let revision = get_string(env, &preferences, "imageRevision", "")?;
    let previous = object(env, appearance, "revision", "Ljava/lang/String;")?;
    if previous.is_null() || string(env, &previous)? != revision {
        let image = cached_image(env, &context, &revision)?;
        set_object(
            env,
            appearance,
            "image",
            "Landroid/graphics/Bitmap;",
            &image,
        )?;
        let revision = env.new_string(revision)?;
        set_object(env, appearance, "revision", "Ljava/lang/String;", &revision)?;
    }
    let theme = get_string(env, &preferences, "theme", "system")?;
    let dark = if theme == "system" {
        let resources = call(
            env,
            &context,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )?
        .l()?;
        let configuration = call(
            env,
            &resources,
            "getConfiguration",
            "()Landroid/content/res/Configuration;",
            &[],
        )?
        .l()?;
        integer(env, &configuration, "uiMode")? & 0x30 == 0x20
    } else {
        theme == "dark"
    };
    let color: u32 = match theme.as_str() {
        "blue" => 0xffdbeafc,
        "pink" => 0xfff9e1ed,
        "mint" => 0xffdbf1e5,
        _ => {
            if dark {
                0xff1c2028
            } else {
                0xffe5e8ee
            }
        }
    };
    let key = if dark { 0xff343a45_u32 } else { 0xffffffff };
    let text = if dark { 0xffffffff_u32 } else { 0xff1b2330 };
    set_integer(env, appearance, "background", color as i32)?;
    set_integer(env, appearance, "key", key as i32)?;
    set_integer(env, appearance, "text", text as i32)?;
    let drawable = env.new_object("moe/meltype/rust/KeyboardBackground", "()V", &[])?;
    let paint = env.new_object("android/graphics/Paint", "(I)V", &[JValue::Int(2)])?;
    let image = object(env, appearance, "image", "Landroid/graphics/Bitmap;")?;
    set_object(env, &drawable, "paint", "Landroid/graphics/Paint;", &paint)?;
    set_object(env, &drawable, "image", "Landroid/graphics/Bitmap;", &image)?;
    set_integer(env, &drawable, "color", color as i32)?;
    set_integer(
        env,
        &drawable,
        "scrim",
        if dark { 0x44000000 } else { 0x33ffffff },
    )?;
    call(
        env,
        root,
        "setBackground",
        "(Landroid/graphics/drawable/Drawable;)V",
        &[JValue::Object(&drawable)],
    )?;
    style(env, appearance, root)
}
pub(super) fn style(env: &mut JNIEnv, appearance: &JObject, view: &JObject) -> Result<()> {
    env.with_local_frame(24, |env| -> Result<()> {
        let text = integer(env, appearance, "text")?;
        let key = integer(env, appearance, "key")?;
        if env.is_instance_of(view, "android/widget/TextView")? {
            call(env, view, "setTextColor", "(I)V", &[JValue::Int(text)])?;
            if env.is_instance_of(view, "android/widget/Button")? {
                let tint = env
                    .call_static_method(
                        "android/content/res/ColorStateList",
                        "valueOf",
                        "(I)Landroid/content/res/ColorStateList;",
                        &[JValue::Int(key)],
                    )?
                    .l()?;
                call(
                    env,
                    view,
                    "setBackgroundTintList",
                    "(Landroid/content/res/ColorStateList;)V",
                    &[JValue::Object(&tint)],
                )?;
            } else {
                call(env, view, "setBackgroundColor", "(I)V", &[JValue::Int(key)])?;
            }
        }
        if env.is_instance_of(view, "android/view/ViewGroup")? {
            let count = call(env, view, "getChildCount", "()I", &[])?.i()?;
            for index in 0..count {
                env.with_local_frame(4, |env| -> Result<()> {
                    let child = call(
                        env,
                        view,
                        "getChildAt",
                        "(I)Landroid/view/View;",
                        &[JValue::Int(index)],
                    )?
                    .l()?;
                    style(env, appearance, &child)
                })?;
            }
            let listener = env.new_object(
                "moe/meltype/rust/AppearanceHierarchy",
                "(Lmoe/meltype/rust/KeyboardAppearance;)V",
                &[JValue::Object(appearance)],
            )?;
            call(
                env,
                view,
                "setOnHierarchyChangeListener",
                "(Landroid/view/ViewGroup$OnHierarchyChangeListener;)V",
                &[JValue::Object(&listener)],
            )?;
        }
        Ok(())
    })
}
pub(super) fn background(
    env: &mut JNIEnv,
    drawable: &JObject,
    event: i32,
    value: i32,
    data: &JObject,
) -> Result<()> {
    let paint = object(env, drawable, "paint", "Landroid/graphics/Paint;")?;
    match event {
        BACKGROUND_ALPHA => {
            if !paint.is_null() {
                call(env, &paint, "setAlpha", "(I)V", &[JValue::Int(value)])?;
            }
            call(env, drawable, "invalidateSelf", "()V", &[])?;
        }
        BACKGROUND_FILTER => {
            if !paint.is_null() {
                call(
                    env,
                    &paint,
                    "setColorFilter",
                    "(Landroid/graphics/ColorFilter;)Landroid/graphics/ColorFilter;",
                    &[JValue::Object(data)],
                )?;
            }
            call(env, drawable, "invalidateSelf", "()V", &[])?;
        }
        _ => {
            let color = integer(env, drawable, "color")?;
            call(env, data, "drawColor", "(I)V", &[JValue::Int(color)])?;
            let image = object(env, drawable, "image", "Landroid/graphics/Bitmap;")?;
            if image.is_null() {
                return Ok(());
            }
            let bounds = call(env, drawable, "getBounds", "()Landroid/graphics/Rect;", &[])?.l()?;
            let width = call(env, &bounds, "width", "()I", &[])?.i()? as f32;
            let height = call(env, &bounds, "height", "()I", &[])?.i()? as f32;
            let image_width = call(env, &image, "getWidth", "()I", &[])?.i()? as f32;
            let image_height = call(env, &image, "getHeight", "()I", &[])?.i()? as f32;
            let scale = (width / image_width).max(height / image_height);
            let width = image_width * scale;
            let height = image_height * scale;
            let x = call(env, &bounds, "centerX", "()I", &[])?.i()? as f32 - width / 2.;
            let y = call(env, &bounds, "centerY", "()I", &[])?.i()? as f32 - height / 2.;
            let destination = env.new_object(
                "android/graphics/RectF",
                "(FFFF)V",
                &[
                    JValue::Float(x),
                    JValue::Float(y),
                    JValue::Float(x + width),
                    JValue::Float(y + height),
                ],
            )?;
            let checkpoint = call(env, data, "save", "()I", &[])?.i()?;
            let drawn = (|| -> Result<()> {
                call(
                    env,
                    data,
                    "clipRect",
                    "(Landroid/graphics/Rect;)Z",
                    &[JValue::Object(&bounds)],
                )?;
                call(env,data,"drawBitmap","(Landroid/graphics/Bitmap;Landroid/graphics/Rect;Landroid/graphics/RectF;Landroid/graphics/Paint;)V",&[JValue::Object(&image),JValue::Object(&JObject::null()),JValue::Object(&destination),JValue::Object(&paint)])?;
                let scrim = integer(env, drawable, "scrim")?;
                call(env, data, "drawColor", "(I)V", &[JValue::Int(scrim)])?;
                Ok(())
            })();
            let restored = cleanup(env, |env| {
                call(
                    env,
                    data,
                    "restoreToCount",
                    "(I)V",
                    &[JValue::Int(checkpoint)],
                )?;
                Ok(())
            });
            drawn.and(restored)?;
        }
    }
    Ok(())
}
