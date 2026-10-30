// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Current host executable identity, preserving Windows UTF-16 paths.
use std::{panic::catch_unwind, ptr};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

fn filename(path: &[u16]) -> &[u16] {
    let start = path
        .iter()
        .rposition(|c| *c == b'\\' as u16)
        .map_or(0, |i| i + 1);
    &path[start..]
}
fn exe_suffix(name: &[u16]) -> bool {
    name.len() >= 4
        && name[name.len() - 4..].iter().zip(b".exe").all(|(&a, &b)| {
            a == b as u16 || (b.is_ascii_alphabetic() && a == b.to_ascii_uppercase() as u16)
        })
}
fn display_name(path: &[u16]) -> &[u16] {
    let name = filename(path);
    if name.len() > 4 && exe_suffix(name) {
        &name[..name.len() - 4]
    } else {
        name
    }
}
fn module_path() -> Vec<u16> {
    module_path_with(|path| unsafe { GetModuleFileNameW(None, path) } as usize)
}
fn module_path_with(mut read: impl FnMut(&mut [u16]) -> usize) -> Vec<u16> {
    let mut path = vec![0; 260];
    loop {
        let n = read(&mut path);
        if n == 0 {
            return Vec::new();
        }
        if n < path.len() {
            path.truncate(n);
            return path;
        }
        if path.len() >= 32768 {
            return Vec::new();
        }
        path.resize((path.len() * 2).min(32768), 0);
    }
}
pub(super) fn name() -> Vec<u16> {
    display_name(&module_path()).to_vec()
}
pub(super) fn own_process() -> bool {
    let path = module_path();
    is_own_name(filename(&path))
}
pub(super) fn chromium_browser() -> bool {
    is_chromium_name(&String::from_utf16_lossy(&name()))
}
fn is_chromium_name(name: &str) -> bool {
    ["chrome", "msedge", "brave", "vivaldi", "opera", "chromium"]
        .iter()
        .any(|browser| name.eq_ignore_ascii_case(browser))
}
fn is_own_name(name: &[u16]) -> bool {
    name.len() == b"meltype-flash.exe".len()
        && name.iter().zip(b"meltype-flash.exe").all(|(&a, &b)| {
            a == b.to_ascii_lowercase() as u16 || a == b.to_ascii_uppercase() as u16
        })
}

/// Required UTF-16 capacity including NUL, or zero on failure. A short buffer
/// is untouched. Callers can query with null/zero; the executable is immutable.
/// # Safety
/// out is null or writable for capacity UTF-16 code units.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_process_name(out: *mut u16, capacity: u32) -> u32 {
    catch_unwind(|| {
        let path = module_path();
        if path.is_empty() {
            return 0;
        }
        let name = display_name(&path);
        let required = name.len() + 1;
        if !out.is_null() && capacity as usize >= required {
            ptr::copy_nonoverlapping(name.as_ptr(), out, name.len());
            out.add(name.len()).write(0);
        }
        required as u32
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue_878_chromium_workaround_is_limited_to_exact_browser_names() {
        for name in ["brave", "Chrome", "MSEDGE", "vivaldi", "opera", "chromium"] {
            assert!(is_chromium_name(name));
        }
        for name in ["firefox", "brave-helper", "editor", ""] {
            assert!(!is_chromium_name(name));
        }
    }
    #[test]
    fn original_filename_rules_preserve_utf16() {
        for (path, expected) in [
            ("C:\\日本語\\editor.EXE", "editor"),
            ("Meltype.exe", "Meltype"),
            (".exe", ".exe"),
            ("test.exe.backup", "test.exe.backup"),
            ("C:\\dir\\", ""),
            ("", ""),
        ] {
            let path: Vec<_> = path.encode_utf16().collect();
            assert_eq!(
                display_name(&path),
                expected.encode_utf16().collect::<Vec<_>>()
            );
        }
        let path = [
            b'\\' as u16,
            0xd800,
            b'.' as u16,
            b'e' as u16,
            b'x' as u16,
            b'e' as u16,
        ];
        assert_eq!(display_name(&path), &[0xd800]);
        let path: Vec<_> = format!("C:\\{}\\日本語.exe", "a".repeat(1000))
            .encode_utf16()
            .collect();
        assert_eq!(
            display_name(&path),
            "日本語".encode_utf16().collect::<Vec<_>>()
        );
    }
    #[test]
    fn path_growth_failure_and_exact_executable_identity() {
        let expected: Vec<_> = format!("C:\\{}\\MeLtYpE-fLaSh.ExE", "あ".repeat(700))
            .encode_utf16()
            .collect();
        let mut calls = 0;
        let actual = module_path_with(|out| {
            calls += 1;
            let n = expected.len().min(out.len());
            out[..n].copy_from_slice(&expected[..n]);
            n
        });
        assert_eq!(actual, expected);
        assert!(calls > 1);
        assert!(is_own_name(filename(&actual)));
        for name in [
            "Meltype",
            "Meltype.exe",
            "OtherMeltype.exe",
            "Othermeltype-flash.exe",
            "meltype-flash.exe.old",
            "Meltype.exe.old",
            "Ｍeltype.exe",
        ] {
            assert!(!is_own_name(&name.encode_utf16().collect::<Vec<_>>()));
        }
        assert!(module_path_with(|_| 0).is_empty());
        assert!(module_path_with(|out| out.len()).is_empty());
    }
    #[test]
    fn current_process_buffer_contract() {
        unsafe {
            let required = meltype_tsf_process_name(ptr::null_mut(), 0);
            assert!(required > 1);
            let mut out = vec![0xaaaa; required as usize + 1];
            assert_eq!(
                meltype_tsf_process_name(out.as_mut_ptr(), required - 1),
                required
            );
            assert!(out.iter().all(|&c| c == 0xaaaa));
            assert_eq!(
                meltype_tsf_process_name(out.as_mut_ptr(), required),
                required
            );
            assert_eq!(&out[..required as usize - 1], display_name(&module_path()));
            assert_eq!(out[required as usize - 1], 0);
            assert_eq!(out[required as usize], 0xaaaa);
        }
    }
}
