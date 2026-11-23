// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Embed the supplied product icon in the Windows executable without requiring
// a resource crate or changing builds on other platforms.
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../../assets/branding/meltype-flash.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let icon = root
        .join("../../assets/branding/meltype-flash.ico")
        .canonicalize()
        .unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = out.join("flash-icon.rc");
    let resource = out.join("flash-icon.res");
    // Windows extended path prefixes are not understood inside RC strings.
    let icon = icon
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    fs::write(
        &source,
        format!("#pragma code_page(65001)\n101 ICON \"{icon}\"\n"),
    )
    .unwrap();
    let compiler = env::var_os("RC").map(PathBuf::from).unwrap_or_else(|| {
        let sdk = PathBuf::from(
            env::var_os("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".into()),
        )
        .join("Windows Kits/10/bin");
        let mut versions: Vec<_> = fs::read_dir(sdk)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("x64/rc.exe"))
            .filter(|path| path.is_file())
            .collect();
        versions.sort();
        versions.pop().unwrap_or_else(|| "rc.exe".into())
    });
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&source)
        .status()
        .expect("Windows SDK resource compiler is required for the Flash icon");
    assert!(
        status.success(),
        "Meltype-FLASH icon resource compilation failed"
    );
    println!(
        "cargo:rustc-link-arg-bin=meltype-flash={}",
        resource.display()
    );
}
