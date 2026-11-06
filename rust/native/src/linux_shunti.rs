// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 任意導入のローカル変換プロセス。未導入・無効・失敗時は標準変換を維持する。
use meltype_core::{mozc::Mozc, session::Clause, settings::Settings};
use std::path::Path;

#[derive(Default)]
pub(crate) struct Shunti {
    path: String,
    backend: Option<Mozc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn optional_engine_only_starts_when_enabled_and_stops_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("helper");
        std::fs::write(&helper, "#!/usr/bin/python3\nimport sys\nprint('READY',flush=True)\nfor line in sys.stdin:\n p=line.rstrip('\\n').split('\\t')\n print(p[2]+'\\x1f今日はいい天気です',flush=True)\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut settings = Settings::default();
        settings.set("Enabled", serde_json::json!(true)).unwrap();
        settings
            .set("ShuntiHelperPath", serde_json::json!(helper))
            .unwrap();
        let mut ai = Shunti::default();
        ai.configure(&settings);
        assert!(ai.convert("きょうはいいてんきです", "").is_none());
        settings.set("UseShunti", serde_json::json!(true)).unwrap();
        ai.configure(&settings);
        assert!(ai.convert("きょうはいいてんきです", "").is_some());
        settings.set("UseShunti", serde_json::json!(false)).unwrap();
        ai.configure(&settings);
        assert!(ai.convert("きょうはいいてんきです", "").is_none());
    }
}
impl Shunti {
    pub(crate) fn configure(&mut self, settings: &Settings) {
        let path = if settings.boolean("Enabled") && settings.boolean("UseShunti") {
            settings.text("ShuntiHelperPath").to_owned()
        } else {
            String::new()
        };
        if path != self.path {
            self.backend = None;
            self.path = path;
            if Path::new(&self.path).is_absolute() {
                self.backend = Mozc::start(Path::new(&self.path), None).ok();
            }
        }
    }
    pub(crate) fn convert(&mut self, reading: &str, context: &str) -> Option<Vec<Clause>> {
        let result = self.backend.as_mut()?.clauses(reading, context);
        if result.is_err() {
            self.backend = None;
        }
        result.ok()
    }
}
