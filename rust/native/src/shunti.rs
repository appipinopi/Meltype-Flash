// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 任意導入のローカル変換プロセス。未導入・無効・失敗時は標準変換を維持する。
use meltype_core::{mozc::Mozc, session::Clause, settings::Settings};
use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
};

#[derive(Default)]
pub(crate) struct Shunti {
    path: String,
    backend: Option<Mozc>,
    starting: Option<Receiver<std::io::Result<Mozc>>>,
}

impl Shunti {
    pub(crate) fn configure(&mut self, settings: &Settings) -> bool {
        let path = if settings.boolean("Enabled")
            && settings.boolean("UseShunti")
            && meltype_core::ai_model::AiModel::selected(settings)
                == meltype_core::ai_model::AiModel::Shunti
        {
            settings.text("ShuntiHelperPath").to_owned()
        } else {
            String::new()
        };
        if path != self.path {
            self.backend = None;
            self.starting = None;
            self.path = path;
            if Path::new(&self.path).is_absolute() {
                let helper = std::path::PathBuf::from(&self.path);
                let (sender, receiver) = mpsc::channel();
                self.starting = Some(receiver);
                // Model loading must not delay the first keystroke. Dropping a
                // superseded receiver also drops/reaps the completed process.
                std::thread::spawn(move || {
                    let _ = sender.send(Mozc::start(&helper, None));
                });
            }
            return true;
        }
        false
    }
    pub(crate) fn convert(&mut self, reading: &str, context: &str) -> Option<Vec<Clause>> {
        if let Some(receiver) = &self.starting {
            match receiver.try_recv() {
                Ok(result) => {
                    self.backend = result.ok();
                    self.starting = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.starting = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let result = self.backend.as_mut()?.clauses(reading, context);
        if result.is_err() {
            self.backend = None;
        }
        result.ok()
    }
}

#[cfg(all(test, unix))]
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
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while ai.convert("きょうはいいてんきです", "").is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "helper did not become ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        settings.set("UseShunti", serde_json::json!(false)).unwrap();
        ai.configure(&settings);
        assert!(ai.convert("きょうはいいてんきです", "").is_none());
    }
}
