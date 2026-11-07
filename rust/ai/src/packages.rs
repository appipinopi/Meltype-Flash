// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 同意後にだけ呼ぶモデル導入。入力処理からネットワークへ接続しない。
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Deserialize)]
pub struct DataFile {
    pub name: String,
    pub url: String,
    pub sha256: String,
}
#[derive(Deserialize)]
pub struct Package {
    pub revision: String,
    pub files: Vec<DataFile>,
}
pub fn shunti_package() -> Package {
    serde_json::from_value(
        serde_json::from_str::<Value>(crate::PACKAGES).unwrap()["shunti"].clone(),
    )
    .unwrap()
}
fn failure(message: impl ToString) -> io::Error {
    io::Error::other(message.to_string())
}
pub fn sha256(path: &Path) -> io::Result<String> {
    let mut input = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn regular(path: &Path) -> io::Result<()> {
    let info = fs::symlink_metadata(path)?;
    if !info.is_file() || info.file_type().is_symlink() {
        return Err(failure("AIパッケージに通常ファイル以外が含まれています"));
    }
    Ok(())
}
fn child(root: &Path, name: &str) -> io::Result<PathBuf> {
    if name.is_empty()
        || name.contains(['\\', ':'])
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(failure("AIパッケージのパスが不正です"));
    }
    Ok(root.join(name))
}
pub fn download(url: &str, path: &Path, expected: &str) -> io::Result<()> {
    if !url.starts_with("https://")
        || expected.len() != 64
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(failure("AI配布元またはSHA-256が不正です"));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", "Meltype-AI-Setup")
        .call()
        .map_err(failure)?;
    let mut output = File::create(path)?;
    let copied = io::copy(
        &mut response.body_mut().as_reader().take(1024 * 1024 * 1024 + 1),
        &mut output,
    )?;
    output.sync_all()?;
    if copied > 1024 * 1024 * 1024 || sha256(path)? != expected.to_ascii_lowercase() {
        return Err(failure("AIファイルのSHA-256が一致しません"));
    }
    Ok(())
}
fn installed(target: &Path, package: &Package) -> io::Result<()> {
    if fs::symlink_metadata(target)?.file_type().is_symlink() {
        return Err(failure("AI環境にリンクは使用できません"));
    }
    let receipt: Value = serde_json::from_slice(&fs::read(target.join("receipt.json"))?)?;
    if receipt["revision"] != package.revision {
        return Err(failure("AI環境の版が一致しません"));
    }
    for spec in &package.files {
        let path = child(target, &spec.name)?;
        regular(&path)?;
        if receipt["files"][&spec.name] != spec.sha256 || sha256(&path)? != spec.sha256 {
            return Err(failure("既存のAI環境が変更されています"));
        }
    }
    Ok(())
}
fn install_with(
    root: &Path,
    package: &Package,
    mut fetch: impl FnMut(&str, &Path, &str) -> io::Result<()>,
) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    let target = child(root, &format!("shunti-{}-data", package.revision))?;
    if target.exists() {
        installed(&target, package)?;
        return Ok(target);
    }
    let stage = tempfile::Builder::new()
        .prefix(".setup-shunti-")
        .tempdir_in(root)?;
    let mut files = BTreeMap::new();
    for spec in &package.files {
        let path = child(stage.path(), &spec.name)?;
        fetch(&spec.url, &path, &spec.sha256)?;
        regular(&path)?;
        if sha256(&path)? != spec.sha256 {
            return Err(failure("取得したAIモデルの検証に失敗しました"));
        }
        files.insert(spec.name.clone(), spec.sha256.clone());
    }
    fs::write(
        stage.path().join("receipt.json"),
        serde_json::to_vec(&json!({"revision":package.revision,"files":files}))?,
    )?;
    fs::rename(stage.path(), &target)?;
    installed(&target, package)?;
    Ok(target)
}
pub fn install_shunti_data(root: &Path) -> io::Result<PathBuf> {
    install_with(root, &shunti_package(), download)
}

/// macOSの専用環境。Rust内蔵HTTPSを使い、Pythonや開発ツールを導入しない。
#[cfg(target_os = "macos")]
pub fn install_macos(root: &Path, shunti: bool) -> io::Result<Value> {
    use std::{io::Write, os::unix::fs::PermissionsExt};
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let parent = root.join("ai");
    fs::create_dir_all(&parent)?;
    let name = if shunti {
        format!("shunti-14801bff-s8-macos-{arch}")
    } else {
        format!("zenz-v3.1-b12125-macos-{arch}")
    };
    let target = parent.join(name);
    fn result(directory: &Path, shunti: bool) -> io::Result<Value> {
        let receipt: Value = serde_json::from_slice(&fs::read(directory.join("receipt.json"))?)?;
        let files = receipt["files"]
            .as_object()
            .ok_or_else(|| failure("AI環境の記録が不正です"))?;
        for key in if shunti {
            &["helper"][..]
        } else {
            &["server", "model"][..]
        } {
            if !receipt[*key]
                .as_str()
                .is_some_and(|name| files.contains_key(name))
            {
                return Err(failure("AI環境が不完全です"));
            }
        }
        for (name, expected) in files {
            let path = child(directory, name)?;
            regular(&path)?;
            if Some(sha256(&path)?.as_str()) != expected.as_str() {
                return Err(failure("AI環境が変更されています"));
            }
        }
        Ok(if shunti {
            json!({"engine":"shunti", "helper":child(directory,receipt["helper"].as_str().unwrap())?})
        } else {
            json!({"server":child(directory,receipt["server"].as_str().unwrap())?,"model":directory.join("model.gguf")})
        })
    }
    if target.exists() {
        return result(&target, shunti);
    }
    let stage = tempfile::Builder::new()
        .prefix(".setup-ai-")
        .tempdir_in(&parent)?;
    let archive = stage.path().join("runtime.archive");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let runtime = stage.path().join("runtime");
    fs::create_dir(&runtime)?;
    let executable;
    if shunti {
        let mut response = agent
            .get("https://api.github.com/repos/appipinopi/Meltype-Flash/releases/latest")
            .header("User-Agent", "Meltype-AI-Setup")
            .call()
            .map_err(failure)?;
        let release: Value =
            serde_json::from_reader(response.body_mut().as_reader().take(2 * 1024 * 1024))?;
        let tag = release["tag_name"]
            .as_str()
            .filter(|s| {
                s.starts_with("flash-v")
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
            })
            .ok_or_else(|| failure("配布版が不正です"))?;
        let asset_name = format!("Meltype-flash-Shunti-macOS-{arch}.zip");
        let asset = release["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|a| a["name"] == asset_name))
            .ok_or_else(|| failure("この公開版にはmacOS用shuntiがありません"))?;
        let url = format!(
            "https://github.com/appipinopi/Meltype-Flash/releases/download/{tag}/{asset_name}"
        );
        if asset["browser_download_url"] != url {
            return Err(failure("shuntiの配布元が一致しません"));
        }
        let hash = asset["digest"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .ok_or_else(|| failure("SHA-256がありません"))?;
        download(&url, &archive, hash)?;
        let mut zip = zip::ZipArchive::new(File::open(&archive)?).map_err(failure)?;
        let mut seen = std::collections::HashSet::new();
        let mut total = 0;
        for n in 0..zip.len() {
            let mut entry = zip.by_index(n).map_err(failure)?;
            let path = child(&runtime, entry.name())?;
            total += entry.size();
            if total > 128 * 1024 * 1024
                || !seen.insert(path.clone())
                || entry
                    .unix_mode()
                    .is_some_and(|m| m & 0xf000 != 0x8000 && m & 0xf000 != 0)
            {
                return Err(failure("不正なshuntiパッケージです"));
            }
            io::copy(&mut entry, &mut File::create(path)?)?;
        }
        let receipt: Value =
            serde_json::from_slice(&fs::read(runtime.join("runtime-receipt.json"))?)?;
        if receipt["commit"] != "14801bffb90c9ea5f2dd0f594943a97ea35c8a1a" {
            return Err(failure("shuntiの版が一致しません"));
        }
        let files = receipt["files"]
            .as_object()
            .ok_or_else(|| failure("shuntiの記録が不正です"))?;
        for name in [
            "shunti-helper",
            "LICENSE",
            "MELTYPE-LICENSE.txt",
            "THIRD_PARTY_NOTICES.md",
            "DATA_SOURCES.md",
        ] {
            if !files.contains_key(name) {
                return Err(failure("shuntiのパッケージが不完全です"));
            }
        }
        for (name, expected) in files {
            let path = child(&runtime, name)?;
            regular(&path)?;
            if expected.as_str() != Some(sha256(&path)?.as_str()) {
                return Err(failure("shuntiのファイルが変更されています"));
            }
        }
        for spec in shunti_package().files {
            download(&spec.url, &runtime.join(&spec.name), &spec.sha256)?;
        }
        executable = runtime.join("shunti-helper");
    } else {
        let asset = format!("llama-b12125-bin-macos-{arch}.tar.gz");
        let hash = if arch == "arm64" {
            "2585d0d097ba3277b8856d81b8bb1b3d1b1556f27ccf3d6717f6438bb27070a6"
        } else {
            "434b6c4bad334d937392a2f2629b4337544e39bb0881710659cf3b70d501f71a"
        };
        download(
            &format!("https://github.com/ggml-org/llama.cpp/releases/download/b12125/{asset}"),
            &archive,
            hash,
        )?;
        let decoder = flate2::read::GzDecoder::new(File::open(&archive)?);
        let mut archive_reader = tar::Archive::new(decoder);
        let mut total = 0;
        for entry in archive_reader.entries()? {
            let mut entry = entry?;
            total += entry.size();
            if total > 1024 * 1024 * 1024 || !entry.unpack_in(&runtime)? {
                return Err(failure("不正なAI実行環境です"));
            }
        }
        // Official llama distributions may use dylib aliases. Resolve only
        // links to regular files inside this extracted runtime, then persist
        // independent regular copies so receipt checks never follow links.
        fn flatten_links(root: &Path, at: &Path) -> io::Result<()> {
            let canonical_root = root.canonicalize()?;
            for entry in fs::read_dir(at)? {
                let path = entry?.path();
                let meta = fs::symlink_metadata(&path)?;
                if meta.is_symlink() {
                    let resolved = path.canonicalize()?;
                    if !resolved.starts_with(&canonical_root) || !resolved.is_file() {
                        return Err(failure(
                            "実行環境のリンクが配布フォルダー外を参照しています",
                        ));
                    }
                    let bytes = fs::read(&resolved)?;
                    let permissions = fs::metadata(&resolved)?.permissions();
                    fs::remove_file(&path)?;
                    fs::write(&path, bytes)?;
                    fs::set_permissions(&path, permissions)?;
                } else if meta.is_dir() {
                    flatten_links(root, &path)?;
                } else if !meta.is_file() {
                    return Err(failure("実行環境に通常ファイル以外が含まれています"));
                }
            }
            Ok(())
        }
        flatten_links(&runtime, &runtime)?;
        executable =
            find(&runtime, "llama-server").ok_or_else(|| failure("llama-serverがありません"))?;
        download("https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf/resolve/ddf6e44b2e05ab7ea9a3e31559c5e7948761365c/ggml-model-Q5_K_M.gguf", &stage.path().join("model.gguf"), "4de930c06bef8c263aa1aa40684af206db4ce1b96375b3b8ed0ea508e0b14f6c")?;
    }
    fs::remove_file(archive)?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
    let mut command = std::process::Command::new(&executable);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if !shunti {
        command.arg("--version");
    }
    let mut probe = command.spawn()?;
    if shunti {
        probe
            .stdin
            .take()
            .unwrap()
            .write_all("C\t\tきょうはいいてんきです\n".as_bytes())?;
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while probe.try_wait()?.is_none() {
        if std::time::Instant::now() > deadline {
            let _ = probe.kill();
            let _ = probe.wait();
            return Err(failure("AI起動確認がタイムアウトしました"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = probe.wait_with_output()?;
    if !output.status.success() || shunti && !output.stdout.contains(&0x1f) {
        return Err(failure("AI起動・変換の確認に失敗しました"));
    }
    fn find(root: &Path, name: &str) -> Option<PathBuf> {
        for entry in fs::read_dir(root).ok()?.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(p) = find(&p, name) {
                    return Some(p);
                }
            } else if p.file_name().is_some_and(|n| n == name) {
                return Some(p);
            }
        }
        None
    }
    fn collect(root: &Path, at: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
        for entry in fs::read_dir(at)? {
            let p = entry?.path();
            if p.is_dir() {
                collect(root, &p, files)?;
            } else {
                files.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    sha256(&p)?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    collect(stage.path(), stage.path(), &mut files)?;
    let mut receipt = json!({"files":files});
    receipt[if shunti { "helper" } else { "server" }] =
        json!(executable.strip_prefix(stage.path()).unwrap());
    if !shunti {
        receipt["model"] = json!("model.gguf");
    }
    fs::write(
        stage.path().join("receipt.json"),
        serde_json::to_vec(&receipt)?,
    )?;
    fs::rename(stage.path(), &target)?;
    result(&target, shunti)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verified_data_is_reused_and_modified_files_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let bytes = b"test model";
        let sha = format!("{:x}", Sha256::digest(bytes));
        let package = Package {
            revision: "fixture".into(),
            files: vec![DataFile {
                name: "model.bin".into(),
                url: "https://example.invalid/model".into(),
                sha256: sha,
            }],
        };
        let installed =
            install_with(root.path(), &package, |_, path, _| fs::write(path, bytes)).unwrap();
        assert_eq!(
            install_with(root.path(), &package, |_, _, _| panic!(
                "unexpected download"
            ))
            .unwrap(),
            installed
        );
        fs::write(installed.join("model.bin"), b"modified").unwrap();
        assert!(install_with(root.path(), &package, |_, _, _| panic!(
            "unexpected download"
        ))
        .is_err());
    }
    #[test]
    fn failed_download_is_not_published_and_paths_cannot_escape() {
        let root = tempfile::tempdir().unwrap();
        let package = Package {
            revision: "fixture".into(),
            files: vec![DataFile {
                name: "model.bin".into(),
                url: "https://example.invalid/model".into(),
                sha256: "0".repeat(64),
            }],
        };
        assert!(install_with(root.path(), &package, |_, path, _| fs::write(
            path, b"wrong"
        ))
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        for path in ["../escape", "/root", "C:escape", "a/../b", "a\\b"] {
            assert!(child(root.path(), path).is_err());
        }
    }
}
