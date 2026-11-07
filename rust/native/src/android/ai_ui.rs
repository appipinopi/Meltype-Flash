// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 同意後の任意モデル導入をワーカースレッドへ渡し、完了だけを画面へ返す。
use super::{bridge::events::*, bridge_api::*, *};
use serde_json::{json, Value};
use std::{collections::HashSet, path::PathBuf};
fn jobs() -> &'static Mutex<HashSet<jlong>> {
    static JOBS: OnceLock<Mutex<HashSet<jlong>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashSet::new()))
}
pub(super) fn busy(handle: jlong) -> bool {
    jobs().lock().is_ok_and(|jobs| jobs.contains(&handle))
}
pub(super) fn start(env: &mut JNIEnv, handle: jlong, activity: &JObject) -> Result<()> {
    if !jobs()
        .lock()
        .map_err(|_| jni::errors::Error::NullPtr("AI jobs"))?
        .insert(handle)
    {
        return Ok(());
    }
    let started = (|| -> Result<()> {
        let payload = env.new_string(json!({"handle":handle}).to_string())?;
        let work = callback(env, activity, AI_WORK, 0, 0, &payload)?;
        let thread = env.new_object(
            "java/lang/Thread",
            "(Ljava/lang/Runnable;)V",
            &[JValue::Object(&work)],
        )?;
        call(env, &thread, "start", "()V", &[])?;
        Ok(())
    })();
    if started.is_err() {
        if let Ok(mut jobs) = jobs().lock() {
            jobs.remove(&handle);
        }
    }
    started
}
pub(super) fn dispatch(
    env: &mut JNIEnv,
    activity: &JObject,
    event: i32,
    data: &JObject,
) -> Result<()> {
    let payload: Value = serde_json::from_str(&string(env, data)?)
        .map_err(|_| jni::errors::Error::NullPtr("AI payload"))?;
    let handle = payload["handle"].as_i64().unwrap_or(0);
    if event == AI_WORK {
        let result = (|| -> std::result::Result<(), String> {
            let root: PathBuf = sessions()
                .lock()
                .map_err(|e| e.to_string())?
                .get(&(handle as usize))
                .and_then(|s| s.data_directory())
                .ok_or("設定が閉じられました")?
                .to_path_buf();
            meltype_ai::packages::install_shunti_data(&root.join("ai"))
                .map_err(|e| e.to_string())?;
            let settings = settings::request(handle, json!({"op":"settings_get"}))?;
            if settings["AiModel"] == "Shunti" {
                settings::request(
                    handle,
                    json!({"op":"settings_set","name":"UseShunti","value":true}),
                )?;
            }
            Ok(())
        })();
        let message = match result {
            Ok(()) => "shuntiを導入しました。キーボードを開き直すと反映します。".into(),
            Err(e) => format!("shuntiを導入できませんでした: {e}"),
        };
        let payload = env.new_string(json!({"handle":handle,"message":message}).to_string())?;
        let completed = callback(env, activity, AI_COMPLETE, 0, 0, &payload)?;
        call(
            env,
            activity,
            "runOnUiThread",
            "(Ljava/lang/Runnable;)V",
            &[JValue::Object(&completed)],
        )?;
    } else {
        if let Ok(mut jobs) = jobs().lock() {
            jobs.remove(&handle);
        }
        if !call(env, activity, "isDestroyed", "()Z", &[])?.z()? {
            native(
                env,
                "settingsUiShow",
                "(JLjava/lang/Object;I)Ljava/lang/String;",
                &[
                    JValue::Long(handle),
                    JValue::Object(activity),
                    JValue::Int(0),
                ],
            )?;
            let status = object(env, activity, "status", "Landroid/widget/TextView;")?;
            text(
                env,
                &status,
                "setText",
                payload["message"]
                    .as_str()
                    .unwrap_or("AI導入が終了しました"),
            )?;
        }
    }
    Ok(())
}
