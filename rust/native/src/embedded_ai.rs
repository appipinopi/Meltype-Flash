// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Androidのshunti。ネットワークは設定画面の同意後だけ、入力は端末内だけで処理する。
use super::*;
use std::path::{Path, PathBuf};
pub(crate) fn directory(root: &Path) -> PathBuf {
    root.join("ai/shunti-14801bff-s8-data")
}
pub(crate) fn profile(session: &Session) -> Option<PathBuf> {
    let root = session.data_directory()?;
    let path = directory(root);
    (session.settings.boolean("Enabled")
        && session.settings.boolean("UseShunti")
        && !session.private
        && path.join("kkc_lex.bin").is_file()
        && path.join("kkc_model.bin").is_file())
    .then_some(path)
}
pub(crate) fn complete(
    handle: *mut c_void,
    profile: Option<PathBuf>,
    request: Option<(String, String)>,
    result: &mut Option<meltype_core::SessionResult>,
) {
    static RUNTIME: OnceLock<Mutex<meltype_ai::shunti::Runtime>> = OnceLock::new();
    let Ok(mut runtime) = RUNTIME
        .get_or_init(|| Mutex::new(Default::default()))
        .lock()
    else {
        return;
    };
    runtime.configure(profile.as_deref());
    let Some((reading, context)) = request else {
        return;
    };
    let Some(candidates) = runtime.convert(&reading, &context) else {
        return;
    };
    drop(runtime);
    if let Ok(mut all) = sessions().lock() {
        if let Some(session) = all.get_mut(&(handle as usize)) {
            if session.apply_ai_candidates(&reading, candidates) {
                if let Some(result) = result {
                    result.view = session.result(result.consumed).view;
                }
            }
        }
    }
}
