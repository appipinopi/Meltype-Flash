// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! AI用CPUランタイムとモデルだけを、利用者の確認後に導入する。
use eframe::egui;
use serde_json::Value;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
};
#[derive(Default)]
pub struct AiSetup {
    confirm: bool,
    shunti: bool,
    model: Option<meltype_core::ai_catalog::Model>,
    checked: String,
    status_epoch: Arc<AtomicU64>,
    verified: bool,
    verifying: Option<mpsc::Receiver<Option<Value>>>,
    pending: Option<mpsc::Receiver<Result<Value, String>>>,
}
impl AiSetup {
    #[cfg(any(windows, target_os = "linux", target_os = "macos", test))]
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn request_model(&mut self, model: &meltype_core::ai_catalog::Model) {
        if !self.busy() && model.available {
            self.shunti = model.family == "Shunti";
            self.model = Some(model.clone());
            self.confirm = true;
        }
    }
    pub fn status(&mut self, root: &Path, model: &meltype_core::ai_catalog::Model) -> bool {
        if self.checked != model.id {
            let epoch = self.status_epoch.fetch_add(1, Ordering::AcqRel) + 1;
            let current = self.status_epoch.clone();
            self.checked = model.id.clone();
            self.verified = false;
            let root = root.to_owned();
            let model = model.clone();
            let (tx, rx) = mpsc::channel();
            self.verifying = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(
                    verified_paths_while(&root, &model, || {
                        current.load(Ordering::Acquire) == epoch
                    })
                    .ok(),
                );
            });
        }
        self.verified
    }
    pub fn locked(&self) -> bool {
        self.busy() || self.confirm
    }
    pub fn reset_status(&mut self) {
        self.status_epoch.fetch_add(1, Ordering::AcqRel);
        self.checked.clear();
        self.verifying = None;
        self.verified = false;
    }
    pub fn checking(&self) -> bool {
        self.verifying.is_some()
    }
    fn start(&mut self, root: &Path) {
        let root = root.to_owned();
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        let shunti = self.shunti;
        let model = self.model.clone();
        std::thread::spawn(move || {
            let _ = sender.send(install_model(&root, shunti, model.as_ref()));
        });
    }
    pub fn show(&mut self, ctx: &egui::Context, root: &Path) -> Option<Result<Value, String>> {
        let mut answer = None;
        if self.confirm {
            egui::Window::new(if self.shunti { "shuntiの追加パッケージを導入してよいですか？" } else { "Zenzaiの環境を作成してよいですか？" })
                .collapsible(false).resizable(false).show(ctx, |ui| {
                    ui.set_max_width(460.);
                    if let Some(m) = &self.model { ui.label(&m.label); ui.label(format!("モデルデータ {:.1} MB / {}", m.files.iter().map(|f| f.size).sum::<u64>() as f64 / 1_000_000.,m.license)); ui.hyperlink_to("配布元",&m.source); }
                    ui.label(if self.shunti { "shuntiのCPU実行環境、辞書（CC BY-SA 4.0）とモデル（CC BY 4.0）を個人領域へ別ダウンロードします。コードはMITです。" } else { "AI用のCPU実行環境（llama.cpp）と選択したGGUFモデルを、Meltypeのユーザーフォルダーへダウンロードします。" });
                    ui.label(if self.shunti { "本体とは別に取得し、管理者権限なしで準備します。完了後にshuntiを有効にします。" } else { "本体とは別に取得し、管理者権限なしで準備します。完了後に選択したモデルを有効にします。" });
                    ui.horizontal(|ui| {
                        if ui.button("はい、作成する").clicked() { answer = Some(true); }
                        if ui.button("いいえ").clicked() { answer = Some(false); }
                    });
                });
        }
        if let Some(yes) = answer {
            self.confirm = false;
            if yes {
                self.start(root);
            }
        }
        if let Some(receiver) = &self.pending {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            match receiver.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    self.checked.clear();
                    return Some(result);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    return Some(Err(
                        "環境作成処理が終了しました。再度お試しください。".into()
                    ));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(rx) = &self.verifying {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            match rx.try_recv() {
                Ok(paths) => {
                    self.verifying = None;
                    self.verified = paths.is_some();
                    if let Some(paths) = paths {
                        return Some(Ok(paths));
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.verifying = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        None
    }
}
#[cfg(not(target_os = "macos"))]
fn run_installer(
    root: &Path,
    shunti: bool,
    model: Option<&meltype_core::ai_catalog::Model>,
) -> Result<Value, String> {
    use std::io::Write as _;
    let mut spec = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    if let Some(model) = model {
        spec.write_all(&serde_json::to_vec(model).unwrap())
            .map_err(|e| e.to_string())?;
    }
    let spec = spec.into_temp_path();
    #[cfg(windows)]
    let output = {
        use std::{
            io::Write,
            os::windows::{ffi::OsStringExt, process::CommandExt},
        };
        let mut system = vec![0u16; 32768];
        let n = unsafe {
            windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
                system.as_mut_ptr(),
                system.len() as u32,
            )
        } as usize;
        if n == 0 || n >= system.len() {
            return Err("Windowsフォルダーを取得できません".into());
        }
        let shell = std::path::PathBuf::from(std::ffi::OsString::from_wide(&system[..n]))
            .join("WindowsPowerShell/v1.0/powershell.exe");
        let mut script = tempfile::Builder::new()
            .prefix("Meltype-ai-")
            .suffix(".ps1")
            .tempfile()
            .map_err(|e| e.to_string())?;
        script
            .write_all(if shunti {
                include_bytes!("../../Setup-Shunti.ps1")
            } else {
                include_bytes!("../../Setup-AI.ps1")
            })
            .map_err(|e| e.to_string())?;
        script.flush().map_err(|e| e.to_string())?;
        // Close the write handle before Windows PowerShell opens the script.
        let script_path = script.into_temp_path();
        let mut command = std::process::Command::new(shell);
        command
            .env_remove("PSModulePath")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script_path.as_os_str())
            .arg("-Root")
            .arg(root);
        if model.is_some() {
            command.arg("-ModelSpec").arg(&spec);
        }
        command.creation_flags(0x08000000).output()
    };
    #[cfg(not(windows))]
    let output = {
        let mut command = std::process::Command::new("python3");
        command
            .args([
                "-c",
                if shunti {
                    include_str!("../../setup-shunti-linux.py")
                } else {
                    include_str!("../../setup-ai-linux.py")
                },
            ])
            .arg(root);
        if model.is_some() {
            command.arg(&spec);
        }
        command.output()
    };
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let paths: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("作成結果を読み取れません: {e}"))?;
    Ok(paths)
}

#[cfg(test)]
fn verified_paths(root: &Path, model: &meltype_core::ai_catalog::Model) -> Result<Value, String> {
    verified_paths_while(root, model, || true)
}
fn verified_paths_while(
    root: &Path,
    model: &meltype_core::ai_catalog::Model,
    current: impl Fn() -> bool,
) -> Result<Value, String> {
    use sha2::{Digest, Sha256};
    use std::{fs, io::Read, path::Component};
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let shunti = model.family == "Shunti";
    let suffix = if cfg!(windows) {
        "windows-"
    } else if cfg!(target_os = "macos") {
        "macos-"
    } else {
        ""
    };
    let directory = if shunti {
        if cfg!(target_os = "linux") {
            model.directory.clone()
        } else {
            format!("{}-{suffix}{arch}", model.directory)
        }
    } else {
        format!(
            "{}-b12125-{}{arch}",
            model.directory,
            if cfg!(target_os = "macos") {
                "macos-"
            } else {
                ""
            }
        )
    };
    let target = root.join("ai").join(directory);
    let check = || -> Result<Value, Box<dyn std::error::Error>> {
        if fs::symlink_metadata(&target)?.file_type().is_symlink() {
            return Err("AI directory is a link".into());
        }
        if !current() {
            return Err("model selection changed".into());
        }
        let bytes = fs::read(target.join("receipt.json"))?;
        let receipt: Value =
            serde_json::from_slice(bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes))?;
        let files = receipt["files"]
            .as_object()
            .ok_or("missing receipt files")?;
        let checked = |name: &str| -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
            let relative = Path::new(name);
            if name.is_empty()
                || relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err("unsafe receipt path".into());
            }
            let path = target.join(relative);
            if !path.canonicalize()?.starts_with(target.canonicalize()?)
                || fs::symlink_metadata(&path)?.file_type().is_symlink()
            {
                return Err("unsafe AI file".into());
            }
            Ok(path)
        };
        for (name, expected) in files {
            let mut file = fs::File::open(checked(name)?)?;
            let mut hash = Sha256::new();
            let mut buffer = [0; 65536];
            loop {
                if !current() {
                    return Err("model selection changed".into());
                }
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            if expected
                .as_str()
                .is_none_or(|s| !s.eq_ignore_ascii_case(&format!("{:x}", hash.finalize())))
            {
                return Err("AI file hash mismatch".into());
            }
        }
        for spec in &model.files {
            let name = if shunti && cfg!(target_os = "macos") {
                format!("runtime/{}", spec.name)
            } else {
                spec.name.clone()
            };
            if files
                .get(&name)
                .and_then(Value::as_str)
                .is_none_or(|sha| !sha.eq_ignore_ascii_case(&spec.sha256))
                || checked(&name)?.metadata()?.len() != spec.size
            {
                return Err("selected model mismatch".into());
            }
        }
        let mut result = serde_json::json!({"variant":model.id,"family":model.family});
        if shunti {
            let name = receipt["helper"].as_str().unwrap_or(if cfg!(windows) {
                "shunti-helper.exe"
            } else {
                "shunti-helper"
            });
            if !files.contains_key(name) {
                return Err("missing helper".into());
            }
            result["engine"] = serde_json::json!("shunti");
            result["helper"] = serde_json::json!(checked(name)?);
        } else {
            for key in ["server", "model"] {
                let name = receipt[key].as_str().ok_or("missing executable/model")?;
                if !files.contains_key(name) {
                    return Err("missing required file".into());
                }
                result[key] = serde_json::json!(checked(name)?);
            }
        }
        Ok(result)
    };
    check().map_err(|e| e.to_string())
}
fn install_model(
    root: &Path,
    shunti: bool,
    model: Option<&meltype_core::ai_catalog::Model>,
) -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    let paths = meltype_ai::packages::install_macos_variant(
        root,
        shunti,
        model.map(|m| serde_json::to_value(m).unwrap()).as_ref(),
    )
    .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "macos"))]
    let paths = run_installer(root, shunti, model)?;
    for key in if shunti {
        &["helper"][..]
    } else {
        &["server", "model"][..]
    } {
        if !paths[*key]
            .as_str()
            .is_some_and(|path| Path::new(path).is_file())
        {
            return Err(format!("作成した{key}ファイルを確認できません"));
        }
    }
    let mut paths = paths;
    if let Some(m) = model {
        paths["variant"] = serde_json::json!(m.id);
        paths["family"] = serde_json::json!(m.family);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_status_requires_selected_model_and_complete_verified_files() {
        use sha2::{Digest, Sha256};
        let root = tempfile::tempdir().unwrap();
        let mut model = meltype_core::ai_catalog::find("jinen-v2-small-q5_k_m")
            .unwrap()
            .clone();
        let hash = format!("{:x}", Sha256::digest(b"model"));
        model.files[0].sha256 = hash.clone();
        model.files[0].size = 5;
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };
        let name = format!(
            "{}-b12125-{}{arch}",
            model.directory,
            if cfg!(target_os = "macos") {
                "macos-"
            } else {
                ""
            }
        );
        let folder = root.path().join("ai").join(name);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("server"), b"model").unwrap();
        std::fs::write(folder.join("model.gguf"), b"model").unwrap();
        std::fs::write(folder.join("receipt.json"),serde_json::to_vec(&serde_json::json!({"server":"server","model":"model.gguf","files":{"server":hash.to_uppercase(),"model.gguf":hash}})).unwrap()).unwrap();
        assert!(verified_paths_while(root.path(), &model, || false).is_err());
        assert_eq!(
            verified_paths(root.path(), &model).unwrap()["variant"],
            model.id
        );
        // Another quantization is a different package, even in the same family.
        let other = meltype_core::ai_catalog::find("jinen-v2-small-q8_0").unwrap();
        assert!(verified_paths(root.path(), other).is_err());
        std::fs::write(folder.join("model.gguf"), b"other").unwrap();
        assert!(verified_paths(root.path(), &model).is_err());
        std::fs::remove_file(folder.join("server")).unwrap();
        assert!(verified_paths(root.path(), &model).is_err());
    }
    #[test]
    fn pending_model_requires_consent_and_cancelled_status_cannot_activate_it() {
        let mut setup = AiSetup::default();
        let model = meltype_core::ai_catalog::find("jinen-v2-small-q5_k_m").unwrap();
        setup.request_model(model);
        assert!(setup.confirm && setup.locked());
        assert!(setup.pending.is_none());
        assert_eq!(setup.model.as_ref().unwrap().id, model.id);
        setup.reset_status();
        assert!(setup.verifying.is_none() && !setup.verified);
        let mut unavailable = model.clone();
        unavailable.available = false;
        setup.confirm = false;
        setup.model = None;
        setup.request_model(&unavailable);
        assert!(!setup.confirm && setup.model.is_none());
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn installer_reuses_verified_files_and_rejects_tampering() {
        let root = tempfile::tempdir().unwrap();
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };
        let environment = root
            .path()
            .join("ai")
            .join(format!("zenz-v3.1-b12125-{arch}"));
        std::fs::create_dir_all(&environment).unwrap();
        let server = if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        };
        let hash = "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb";
        std::fs::write(environment.join(server), b"a").unwrap();
        std::fs::write(environment.join("model.gguf"), b"a").unwrap();
        let receipt = serde_json::json!({"server": server,"model":"model.gguf", "files": {server: hash, "model.gguf":hash}});
        std::fs::write(
            environment.join("receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        let paths = install_model(root.path(), false, None).unwrap();
        assert_eq!(
            std::fs::canonicalize(paths["server"].as_str().unwrap()).unwrap(),
            std::fs::canonicalize(environment.join(server)).unwrap()
        );
        std::fs::write(environment.join("model.gguf"), b"changed").unwrap();
        assert!(install_model(root.path(), false, None).is_err());
    }

    #[test]
    fn asking_for_consent_never_starts_installation() {
        let mut setup = AiSetup::default();
        setup.request_model(meltype_core::ai_catalog::find("zenz-v3.1-small-q5_k_m").unwrap());
        assert!(setup.confirm);
        assert!(!setup.busy());
        {
            setup.request_model(meltype_core::ai_catalog::find("shunti-v7-s8p").unwrap());
            assert!(setup.shunti && setup.confirm);
            assert!(!setup.busy());
        }
    }
}
