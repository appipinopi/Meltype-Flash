// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
//! Explicit regsvr32 operations. Tests inject all mutations; loading the DLL
//! never registers a COM server, input profile or category.
use super::identity::CLSID;
use super::identity::PROFILE;
use std::panic::{catch_unwind, AssertUnwindSafe};
use windows::{
    core::{w, Result, GUID, HRESULT, PCWSTR},
    Win32::{
        Foundation::*,
        System::{Com::*, Ole::SELFREG_E_CLASS, Registry::*},
        UI::{Input::KeyboardAndMouse::HKL, TextServices::*},
    },
};
const KEY: PCWSTR = w!("Software\\Classes\\CLSID\\{C540BAF6-88B1-5B30-B524-2C8702B034FC}");
const CATEGORIES: [GUID; 5] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
];

trait Backend {
    fn server(&mut self, path: &[u16]) -> Result<()>;
    fn profile(&mut self, path: &[u16]) -> Result<()>;
    fn categories(&mut self, add: bool) -> Result<()>;
    fn remove_profile(&mut self);
    fn remove_server(&mut self);
}
fn remove(backend: &mut impl Backend) {
    let _ = backend.categories(false);
    backend.remove_profile();
    backend.remove_server();
}
fn register(backend: &mut impl Backend, path: &[u16]) -> HRESULT {
    let result = backend
        .server(path)
        .and_then(|_| backend.profile(path))
        .and_then(|_| backend.categories(true));
    match result {
        Ok(()) => S_OK,
        Err(error) => {
            remove(backend);
            error.code()
        }
    }
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}
impl Key {
    unsafe fn create(parent: HKEY, name: PCWSTR) -> Result<Self> {
        let mut key = HKEY::default();
        if RegCreateKeyExW(
            parent,
            name,
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        ) != ERROR_SUCCESS
        {
            return Err(SELFREG_E_CLASS.into());
        }
        Ok(Self(key))
    }
    unsafe fn string(&self, name: PCWSTR, value: &[u16]) -> Result<()> {
        let bytes: Vec<u8> = value
            .iter()
            .copied()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        if RegSetValueExW(self.0, name, 0, REG_SZ, Some(&bytes)) == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(SELFREG_E_CLASS.into())
        }
    }
}
struct Windows;
impl Backend for Windows {
    fn server(&mut self, path: &[u16]) -> Result<()> {
        unsafe {
            let class = Key::create(HKEY_LOCAL_MACHINE, KEY)?;
            class.string(
                PCWSTR::null(),
                &"Meltype-FLASH".encode_utf16().collect::<Vec<_>>(),
            )?;
            let server = Key::create(class.0, w!("InprocServer32"))?;
            server.string(PCWSTR::null(), path)?;
            server.string(
                w!("ThreadingModel"),
                &"Apartment".encode_utf16().collect::<Vec<_>>(),
            )
        }
    }
    fn profile(&mut self, path: &[u16]) -> Result<()> {
        unsafe {
            let profiles: ITfInputProcessorProfileMgr =
                CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
            profiles.RegisterProfile(
                &CLSID,
                0x0411,
                &PROFILE,
                &"Meltype-FLASH".encode_utf16().collect::<Vec<_>>(),
                path,
                0,
                HKL::default(),
                0,
                true,
                0,
            )
        }
    }
    fn categories(&mut self, add: bool) -> Result<()> {
        unsafe {
            let manager: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
            let mut result = Ok(());
            for category in &CATEGORIES {
                let one = if add {
                    manager.RegisterCategory(&CLSID, category, &CLSID)
                } else {
                    manager.UnregisterCategory(&CLSID, category, &CLSID)
                };
                if add && one.is_err() {
                    result = one;
                }
            }
            result
        }
    }
    fn remove_profile(&mut self) {
        unsafe {
            if let Ok(profiles) = CoCreateInstance::<_, ITfInputProcessorProfileMgr>(
                &CLSID_TF_InputProcessorProfiles,
                None,
                CLSCTX_INPROC_SERVER,
            ) {
                let _ = profiles.UnregisterProfile(&CLSID, 0x0411, &PROFILE, 0);
            }
        }
    }
    fn remove_server(&mut self) {
        unsafe {
            let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, KEY);
        }
    }
}
/// Explicitly register the TSF shell at `path`, which must be UTF-16 without a
/// NUL terminator. Called only by the shell's DllRegisterServer entry point.
/// # Safety
/// `path` must point to `length` readable UTF-16 units.
#[no_mangle]
pub unsafe extern "C" fn meltype_tsf_register_server(path: *const u16, length: u32) -> HRESULT {
    catch_unwind(AssertUnwindSafe(|| {
        if path.is_null() || length == 0 || length > 32767 {
            return E_INVALIDARG;
        }
        let path = std::slice::from_raw_parts(path, length as usize);
        if path.contains(&0) {
            return E_INVALIDARG;
        }
        register(&mut Windows, path)
    }))
    .unwrap_or(E_FAIL)
}
/// Explicit unregistration; retains upstream's best-effort cleanup behavior.
#[no_mangle]
pub extern "C" fn meltype_tsf_unregister_server() -> HRESULT {
    catch_unwind(AssertUnwindSafe(|| {
        remove(&mut Windows);
        S_OK
    }))
    .unwrap_or(E_FAIL)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        fail: Option<&'static str>,
        calls: Vec<&'static str>,
        paths: Vec<Vec<u16>>,
    }
    impl Fake {
        fn call(&mut self, name: &'static str) -> Result<()> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(E_ACCESSDENIED.into())
            } else {
                Ok(())
            }
        }
    }
    impl Backend for Fake {
        fn server(&mut self, path: &[u16]) -> Result<()> {
            self.paths.push(path.to_vec());
            self.call("server")
        }
        fn profile(&mut self, path: &[u16]) -> Result<()> {
            self.paths.push(path.to_vec());
            self.call("profile")
        }
        fn categories(&mut self, add: bool) -> Result<()> {
            self.call(if add {
                "categories"
            } else {
                "remove_categories"
            })
        }
        fn remove_profile(&mut self) {
            let _ = self.call("remove_profile");
        }
        fn remove_server(&mut self) {
            let _ = self.call("remove_server");
        }
    }
    #[test]
    fn registration_keeps_utf16_path_and_rolls_back_each_failed_stage() {
        let path: Vec<_> = "C:\\日本語 😀\\MeltypeTip.dll".encode_utf16().collect();
        for failed in [None, Some("server"), Some("profile"), Some("categories")] {
            let mut backend = Fake {
                fail: failed,
                calls: vec![],
                paths: vec![],
            };
            let result = register(&mut backend, &path);
            assert_eq!(
                result,
                if failed.is_none() {
                    S_OK
                } else {
                    E_ACCESSDENIED
                }
            );
            assert!(backend.paths.iter().all(|p| *p == path));
            let mut expected = match failed {
                Some("server") => vec!["server"],
                Some("profile") => vec!["server", "profile"],
                _ => vec!["server", "profile", "categories"],
            };
            if failed.is_some() {
                expected.extend(["remove_categories", "remove_profile", "remove_server"]);
            }
            assert_eq!(backend.calls, expected);
        }
    }
    #[test]
    fn unregister_continues_when_category_cleanup_fails() {
        let mut backend = Fake {
            fail: Some("remove_categories"),
            calls: vec![],
            paths: vec![],
        };
        remove(&mut backend);
        assert_eq!(
            backend.calls,
            ["remove_categories", "remove_profile", "remove_server"]
        );
    }
}
