// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! AI用CPUランタイムとモデルだけを、利用者の確認後に導入する。
use eframe::egui;
use serde_json::Value;
use std::{path::Path, sync::mpsc};
#[derive(Default)]
pub struct AiSetup {
    confirm: bool,
    pending: Option<mpsc::Receiver<Result<Value, String>>>,
}
impl AiSetup {
    pub fn request(&mut self) {
        if !self.busy() {
            self.confirm = true;
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn start(&mut self, root: &Path) {
        let root = root.to_owned();
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(install(&root));
        });
    }
    pub fn show(&mut self, ctx: &egui::Context, root: &Path) -> Option<Result<Value, String>> {
        let mut answer = None;
        if self.confirm {
            egui::Window::new("Zenzaiの環境を作成してよいですか？")
                .collapsible(false).resizable(false).show(ctx, |ui| {
                    ui.set_max_width(460.);
                    ui.label("AI用のCPU実行環境（llama.cpp）とZenzaiモデルを、Meltypeのユーザーフォルダーへダウンロードします。");
                    ui.label("本体とは別に取得し、管理者権限なしで準備します。完了後にZenzaiを有効にします。");
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
        None
    }
}
fn install(root: &Path) -> Result<Value, String> {
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
            .write_all(include_bytes!("../../Setup-AI.ps1"))
            .map_err(|e| e.to_string())?;
        script.flush().map_err(|e| e.to_string())?;
        // Close the write handle before Windows PowerShell opens the script.
        let script_path = script.into_temp_path();
        std::process::Command::new(shell)
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
            .arg(root)
            .creation_flags(0x08000000)
            .output()
    };
    #[cfg(not(windows))]
    let output = std::process::Command::new("python3")
        .args(["-c", include_str!("../../setup-ai-linux.py")])
        .arg(root)
        .output();
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let paths: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("作成結果を読み取れません: {e}"))?;
    for key in ["server", "model"] {
        if !paths[key]
            .as_str()
            .is_some_and(|path| Path::new(path).is_file())
        {
            return Err(format!("作成した{key}ファイルを確認できません"));
        }
    }
    Ok(paths)
}
#[cfg(test)]
mod tests {
    use super::*;
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
        let paths = install(root.path()).unwrap();
        assert_eq!(
            std::path::PathBuf::from(paths["server"].as_str().unwrap()),
            environment.join(server)
        );
        std::fs::write(environment.join("model.gguf"), b"changed").unwrap();
        assert!(install(root.path()).is_err());
    }

    #[test]
    fn asking_for_consent_never_starts_installation() {
        let mut setup = AiSetup::default();
        setup.request();
        assert!(setup.confirm);
        assert!(!setup.busy());
    }
}
