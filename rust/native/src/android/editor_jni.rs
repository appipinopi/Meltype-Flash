// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use super::*;
use crate::android_editor::{self, Editor, EditorError};
type Result<T> = std::result::Result<T, EditorError>;
thread_local! {
    static ENDING: std::cell::RefCell<std::collections::HashSet<jlong>> = std::cell::RefCell::new(std::collections::HashSet::new());
}
struct Ending(jlong);
impl Ending {
    fn enter(handle: jlong) -> Option<Self> {
        ENDING
            .with(|v| v.borrow_mut().insert(handle))
            .then(|| Self(handle))
    }
}
impl Drop for Ending {
    fn drop(&mut self) {
        ENDING.with(|v| v.borrow_mut().remove(&self.0));
    }
}
fn reset_session(handle: jlong) {
    unsafe {
        meltype_free(meltype_reset(handle as *mut c_void).cast());
    }
}
struct Connection<'a, 'b, 'c> {
    env: &'a mut JNIEnv<'b>,
    object: &'a JObject<'c>,
    scope: Option<(jlong, Option<u64>)>,
}
impl Connection<'_, '_, '_> {
    fn current(&self) -> Result<()> {
        if self
            .scope
            .is_some_and(|(handle, epoch)| service_state::epoch(handle) != epoch)
        {
            Err(EditorError)
        } else {
            Ok(())
        }
    }
    fn surrounding(&mut self, method: &str, count: i32) -> Result<Option<Vec<u16>>> {
        self.current()?;
        let text = self
            .env
            .call_method(
                self.object,
                method,
                "(II)Ljava/lang/CharSequence;",
                &[JValue::Int(count), JValue::Int(0)],
            )
            .and_then(|v| v.l())
            .map_err(|_| EditorError)?;
        self.units(text)
    }
    // Read charAt rather than converting a Java string through UTF-8: a lone
    // surrogate must not compare equal to a replacement character.
    fn units(&mut self, text: JObject<'_>) -> Result<Option<Vec<u16>>> {
        if text.is_null() {
            return Ok(None);
        }
        let result = self
            .env
            .with_local_frame(8, |env| {
                let size = env.call_method(&text, "length", "()I", &[])?.i()?;
                let mut units = Vec::with_capacity(size.max(0) as usize);
                for index in 0..size {
                    units.push(
                        env.call_method(&text, "charAt", "(I)C", &[JValue::Int(index)])?
                            .c()?,
                    );
                }
                Ok::<_, jni::errors::Error>(Some(units))
            })
            .map_err(|_| EditorError);
        let _ = self.env.delete_local_ref(text);
        result
    }
    fn boolean(&mut self, name: &str, signature: &str, args: &[JValue<'_, '_>]) -> Result<bool> {
        self.current()?;
        self.env
            .call_method(self.object, name, signature, args)
            .and_then(|v| v.z())
            .map_err(|_| EditorError)
    }
    fn text(&mut self, name: &str, text: &str) -> Result<bool> {
        let string = self.env.new_string(text).map_err(|_| EditorError)?;
        let result = self.boolean(
            name,
            "(Ljava/lang/CharSequence;I)Z",
            &[JValue::Object(&string), JValue::Int(1)],
        );
        let _ = self.env.delete_local_ref(string);
        result
    }
    fn clear_exception(&mut self) {
        if self.env.exception_check().unwrap_or(false) {
            let _ = self.env.exception_clear();
        }
    }
}
impl Editor for Connection<'_, '_, '_> {
    fn selected(&mut self) -> Result<Option<Vec<u16>>> {
        self.current()?;
        let text = self
            .env
            .call_method(
                self.object,
                "getSelectedText",
                "(I)Ljava/lang/CharSequence;",
                &[JValue::Int(0)],
            )
            .and_then(|v| v.l())
            .map_err(|_| EditorError)?;
        self.units(text)
    }
    fn before(&mut self, count: i32) -> Result<Option<Vec<u16>>> {
        self.surrounding("getTextBeforeCursor", count)
    }
    fn begin(&mut self) -> Result<()> {
        // Android permits false for a valid non-nesting implementation.
        self.boolean("beginBatchEdit", "()Z", &[]).map(|_| ())
    }
    fn end(&mut self) -> Result<()> {
        // An editor exception must not prevent balancing the batch.
        self.clear_exception();
        // Balance this old editor's batch even after the field generation changed.
        let scope = self.scope.take();
        let result = self.boolean("endBatchEdit", "()Z", &[]).map(|_| ());
        self.scope = scope;
        self.clear_exception();
        result
    }
    fn composing(&mut self, text: &str) -> Result<bool> {
        self.text("setComposingText", text)
    }
    fn delete(&mut self, count: i32) -> Result<bool> {
        self.boolean(
            "deleteSurroundingText",
            "(II)Z",
            &[JValue::Int(count), JValue::Int(0)],
        )
    }
    fn commit(&mut self, text: &str) -> Result<bool> {
        self.text("commitText", text)
    }
    fn finish(&mut self) -> Result<bool> {
        self.clear_exception();
        self.boolean("finishComposingText", "()Z", &[])
    }
    fn move_back(&mut self, count: u32) -> Result<bool> {
        for _ in 0..count {
            for action in [0, 1] {
                self.current()?;
                let event = self
                    .env
                    .new_object(
                        "android/view/KeyEvent",
                        "(II)V",
                        &[JValue::Int(action), JValue::Int(21)],
                    )
                    .map_err(|_| EditorError)?;
                let result = self.boolean(
                    "sendKeyEvent",
                    "(Landroid/view/KeyEvent;)Z",
                    &[JValue::Object(&event)],
                );
                let _ = self.env.delete_local_ref(event);
                if !result? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
fn context_snapshot(
    env: &mut JNIEnv,
    object: &JObject,
    private: bool,
    pending: JString,
    scope: Option<(jlong, Option<u64>)>,
) -> (String, String, bool) {
    // Private fields never invoke any surrounding-text method.
    if private || object.is_null() {
        return (String::new(), String::new(), false);
    }
    let mut editor = Connection { env, object, scope };
    let result = (|| -> Result<_> {
        let can_delete = editor.before(0)?.is_some();
        let after = editor
            .surrounding("getTextAfterCursor", 4000)?
            .unwrap_or_default();
        let mut before = editor.before(4000)?.unwrap_or_default();
        let pending = editor.units(pending.into())?.unwrap_or_default();
        if before.ends_with(&pending) {
            before.truncate(before.len() - pending.len());
        }
        Ok((
            String::from_utf16_lossy(&before),
            String::from_utf16_lossy(&after),
            can_delete,
        ))
    })();
    editor.clear_exception();
    result.unwrap_or_else(|_| (String::new(), String::new(), false))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editorKey(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    key: jint,
    scalar: jint,
    modifiers: jint,
    hardware: jboolean,
    private: jboolean,
    pending: JString,
) -> jstring {
    let epoch = service_state::epoch(handle);
    if hardware != 0 && crate::android_input::modifier_key(key) {
        return java_result(&mut env, std::ptr::null_mut());
    }
    let (before, after, can_delete) = context_snapshot(
        &mut env,
        &connection,
        private != 0,
        pending,
        Some((handle, epoch)),
    );
    if service_state::epoch(handle) != epoch {
        return java_result(&mut env, std::ptr::null_mut());
    }
    meltype_set_can_delete(handle as *mut c_void, i32::from(can_delete));
    let (vk, modifiers) = if hardware != 0 {
        (
            crate::android_input::virtual_key(key),
            crate::android_input::modifiers(modifiers),
        )
    } else {
        (key, modifiers)
    };
    let before = CString::new(before).ok();
    let after = CString::new(after).ok();
    let result = unsafe {
        meltype_handle_key(
            handle as *mut c_void,
            vk,
            scalar,
            modifiers,
            before.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
            after.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        )
    };
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editorUndo(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    private: jboolean,
    pending: JString,
) -> jstring {
    let epoch = service_state::epoch(handle);
    if private != 0 || connection.is_null() {
        return java_result(&mut env, std::ptr::null_mut());
    }
    let (before, _, can_delete) =
        context_snapshot(&mut env, &connection, false, pending, Some((handle, epoch)));
    if service_state::epoch(handle) != epoch {
        return java_result(&mut env, std::ptr::null_mut());
    }
    meltype_set_can_delete(handle as *mut c_void, i32::from(can_delete));
    let result = CString::new(before)
        .ok()
        .map_or(std::ptr::null_mut(), |text| unsafe {
            meltype_undo_commit(handle as *mut c_void, text.as_ptr())
        });
    java_result(&mut env, result)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_softwareFallback(
    mut env: JNIEnv,
    _class: JClass,
    connection: JObject,
    key: jint,
    scalar: jint,
    ime_options: jint,
) -> jboolean {
    if connection.is_null() {
        return 0;
    }
    let mut editor = Connection {
        env: &mut env,
        object: &connection,
        scope: None,
    };
    let result = (|| -> Result<bool> {
        if scalar > 0 {
            let text = char::from_u32(scalar as u32)
                .ok_or(EditorError)?
                .to_string();
            return editor.commit(&text);
        }
        match key {
            8 if editor.selected()?.is_some_and(|text| !text.is_empty()) => editor.commit(""),
            8 => editor.boolean(
                "deleteSurroundingTextInCodePoints",
                "(II)Z",
                &[JValue::Int(1), JValue::Int(0)],
            ),
            0x25 | 0x27 => {
                let code = if key == 0x25 { 21 } else { 22 };
                let mut accepted = true;
                for action in [0, 1] {
                    let event = editor
                        .env
                        .new_object(
                            "android/view/KeyEvent",
                            "(II)V",
                            &[JValue::Int(action), JValue::Int(code)],
                        )
                        .map_err(|_| EditorError)?;
                    let sent = editor.boolean(
                        "sendKeyEvent",
                        "(Landroid/view/KeyEvent;)Z",
                        &[JValue::Object(&event)],
                    );
                    let _ = editor.env.delete_local_ref(event);
                    // Preserve down/up pairing even when DOWN is refused.
                    accepted &= sent?;
                }
                Ok(accepted)
            }
            13 => {
                let action = ime_options & 0xff;
                if action != 0 && action != 1 && ime_options & 0x4000_0000 == 0 {
                    editor.boolean("performEditorAction", "(I)Z", &[JValue::Int(action)])
                } else {
                    editor.commit("\n")
                }
            }
            _ => Ok(false),
        }
    })();
    editor.clear_exception();
    u8::from(result.unwrap_or(false))
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_applyEditor(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    json: JString,
    pending: JString,
) -> jstring {
    let epoch = service_state::epoch(handle);
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if connection.is_null() {
            return android_editor::Outcome::failure(false);
        }
        let Ok(json) = env.get_string(&json).map(String::from) else {
            return android_editor::Outcome::failure(false);
        };
        let mut editor = Connection {
            env: &mut env,
            object: &connection,
            scope: Some((handle, epoch)),
        };
        let pending = match editor.units(pending.into()) {
            Ok(Some(value)) => value,
            _ => {
                editor.clear_exception();
                return android_editor::Outcome::failure(false);
            }
        };
        let outcome = android_editor::apply(&mut editor, &json, &pending);
        editor.clear_exception();
        outcome
    }))
    .unwrap_or_else(|_| android_editor::Outcome::failure(false));
    if outcome.reset && service_state::epoch(handle) == epoch {
        reset_session(handle);
    }
    let outcome = service_state::publish(handle, epoch, &outcome);
    outcome_string(&mut env, &outcome)
}
fn outcome_string(env: &mut JNIEnv, outcome: &android_editor::Outcome) -> jstring {
    // Clear a getter failure before allocating the Java result, including a
    // null JSON argument. No pending editor exception crosses this boundary.
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    let text = serde_json::to_string(&outcome).unwrap_or_default();
    env.new_string(text)
        .map(|v| v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_toggleInputMode(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
) -> jstring {
    let epoch = service_state::epoch(handle);
    let pending: Vec<_> = service_state::pending(handle).encode_utf16().collect();
    let reply = unsafe { take(meltype_commit(handle as *mut c_void)) };
    let outcome = if connection.is_null() {
        android_editor::Outcome::failure(false)
    } else {
        let mut editor = Connection {
            env: &mut env,
            object: &connection,
            scope: Some((handle, epoch)),
        };
        let outcome = reply.map_or_else(
            || android_editor::Outcome::failure(false),
            |reply| android_editor::apply(&mut editor, &reply, &pending),
        );
        editor.clear_exception();
        outcome
    };
    if service_state::epoch(handle) == epoch {
        if outcome.reset {
            reset_session(handle);
        }
        service_state::toggle_direct(handle);
    }
    let outcome = service_state::publish(handle, epoch, &outcome);
    outcome_string(&mut env, &outcome)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_selectionChanged(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    composing: jboolean,
    start: jint,
    end: jint,
    composing_end: jint,
) -> jboolean {
    if composing == 0 || start == end && end == composing_end {
        return 0;
    }
    let Some(_ending) = Ending::enter(handle) else {
        return 0;
    };
    service_state::clear(handle);
    reset_session(handle);
    if !connection.is_null() {
        let mut editor = Connection {
            env: &mut env,
            object: &connection,
            scope: None,
        };
        let _ = editor.finish();
        editor.clear_exception();
    }
    1
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_finishInput(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    composing: jboolean,
    pending: JString,
) -> jstring {
    let epoch = service_state::epoch(handle);
    let Some(_ending) = Ending::enter(handle) else {
        return outcome_string(&mut env, &android_editor::Outcome::failure(false));
    };
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if composing == 0 {
            return android_editor::Outcome::failure(false);
        }
        let reply = unsafe { take(meltype_commit(handle as *mut c_void)) };
        if connection.is_null() {
            return android_editor::Outcome::failure(false);
        }
        let mut editor = Connection {
            env: &mut env,
            object: &connection,
            scope: Some((handle, epoch)),
        };
        let result = match (reply, editor.units(pending.into())) {
            (Some(reply), Ok(Some(pending))) => {
                android_editor::apply(&mut editor, &reply, &pending)
            }
            _ => android_editor::Outcome::failure(false),
        };
        editor.clear_exception();
        result
    }))
    .unwrap_or_else(|_| android_editor::Outcome::failure(false));
    if service_state::epoch(handle) == epoch {
        reset_session(handle);
    }
    // Ending input invalidates the old view even if a host rejects a write.
    let mut outcome = outcome;
    outcome.composing = false;
    outcome.composing_text.clear();
    outcome.view = None;
    let outcome = service_state::publish(handle, epoch, &outcome);
    outcome_string(&mut env, &outcome)
}
#[no_mangle]
pub extern "system" fn Java_moe_meltype_rust_Native_editorReconvert(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    connection: JObject,
    private: jboolean,
    composing: jboolean,
) -> jstring {
    let epoch = service_state::epoch(handle);
    if private != 0 || composing != 0 || connection.is_null() {
        return std::ptr::null_mut();
    }
    let mut editor = Connection {
        env: &mut env,
        object: &connection,
        scope: None,
    };
    let selected = editor.selected();
    editor.clear_exception();
    if service_state::epoch(handle) != epoch {
        return std::ptr::null_mut();
    }
    let selected = selected
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())
        .and_then(|v| String::from_utf16(&v).ok())
        .and_then(|v| CString::new(v).ok());
    let Some(text) = selected else {
        return std::ptr::null_mut();
    };
    let result = unsafe {
        meltype_reconvert_selection(handle as *mut c_void, text.as_ptr(), std::ptr::null())
    };
    java_result(&mut env, result)
}
