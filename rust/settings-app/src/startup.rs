// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! User-logon task. Registration happens only on an explicit tray action.
use eframe::egui;
use std::{
    ffi::OsString,
    io::Write,
    os::windows::{ffi::OsStringExt, process::CommandExt},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use windows_sys::Win32::Security::Authentication::Identity::{GetUserNameExW, NameSamCompatible};
#[derive(Clone)]
struct Spec {
    user: String,
    executable: PathBuf,
    task: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunEntry {
    Exists,
    Enable,
    Disable,
}
impl Spec {
    fn current() -> Result<Self, String> {
        let mut count = 0;
        unsafe {
            GetUserNameExW(NameSamCompatible, std::ptr::null_mut(), &mut count);
        }
        if count == 0 || count > 32768 {
            return Err("Windowsのユーザー名を取得できません".into());
        }
        let mut buffer = vec![0; count as usize];
        if !unsafe { GetUserNameExW(NameSamCompatible, buffer.as_mut_ptr(), &mut count) } {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let user = String::from_utf16(&buffer[..count as usize]).map_err(|e| e.to_string())?;
        Ok(Self::new(
            user,
            std::env::current_exe().map_err(|e| e.to_string())?,
        ))
    }
    fn new(user: String, executable: PathBuf) -> Self {
        let hash = user
            .to_lowercase()
            .as_bytes()
            .iter()
            .fold(0xcbf29ce484222325_u64, |h, b| {
                (h ^ *b as u64).wrapping_mul(0x100000001b3)
            });
        Self {
            user,
            executable,
            task: format!("Meltype-flash-{hash:016x}"),
        }
    }
    fn xml(&self) -> String {
        let user = escape(&self.user);
        let exe = escape(&self.executable.to_string_lossy());
        let directory = escape(
            &self
                .executable
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_string_lossy(),
        );
        format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<RegistrationInfo><Description>Meltype-FLASH をサインインしたらすぐ起動する</Description></RegistrationInfo>
<Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId><Delay>PT5S</Delay></LogonTrigger></Triggers>
<Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><RestartOnFailure><Interval>PT1M</Interval><Count>3</Count></RestartOnFailure><Priority>5</Priority></Settings>
<Actions Context="Author"><Exec><Command>{exe}</Command><Arguments>--tray</Arguments><WorkingDirectory>{directory}</WorkingDirectory></Exec></Actions></Task>"#
        )
    }
    fn query(
        &self,
        run: &mut impl FnMut(&[OsString]) -> Result<i32, String>,
    ) -> Result<bool, String> {
        match run(&["/Query".into(), "/TN".into(), self.task.clone().into()])? {
            0 => Ok(true),
            1 => Ok(false),
            code => Err(format!("起動設定を取得できません (終了コード{code})")),
        }
    }
    fn set(
        &self,
        enabled: bool,
        run: &mut impl FnMut(&[OsString]) -> Result<i32, String>,
    ) -> Result<bool, String> {
        let code = if enabled {
            let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            let bytes: Vec<_> = std::iter::once(0xfeff_u16)
                .chain(self.xml().encode_utf16())
                .flat_map(u16::to_le_bytes)
                .collect();
            file.write_all(&bytes)
                .and_then(|()| file.flush())
                .map_err(|e| e.to_string())?;
            // schtasks must be able to reopen the XML on Windows.
            let xml_path = file.into_temp_path();
            run(&[
                "/Create".into(),
                "/TN".into(),
                self.task.clone().into(),
                "/XML".into(),
                xml_path.as_os_str().into(),
                "/F".into(),
            ])?
        } else if self.query(run)? {
            run(&[
                "/Delete".into(),
                "/TN".into(),
                self.task.clone().into(),
                "/F".into(),
            ])?
        } else {
            return Ok(false);
        };
        if code != 0 {
            return Err(format!("起動設定を変更できません (終了コード{code})"));
        }
        Ok(enabled)
    }
    fn query_combined(
        &self,
        run: &mut impl FnMut(&[OsString]) -> Result<i32, String>,
        registry: &mut impl FnMut(RunEntry) -> Result<bool, String>,
    ) -> Result<bool, String> {
        match self.query(run) {
            Ok(true) => Ok(true),
            Ok(false) => registry(RunEntry::Exists),
            Err(error) => {
                if registry(RunEntry::Exists)? {
                    Ok(true)
                } else {
                    Err(error)
                }
            }
        }
    }
    fn set_combined(
        &self,
        enabled: bool,
        run: &mut impl FnMut(&[OsString]) -> Result<i32, String>,
        registry: &mut impl FnMut(RunEntry) -> Result<bool, String>,
    ) -> Result<bool, String> {
        if enabled {
            if self.set(true, run).is_ok() {
                // A redundant owned Run entry is harmless: the process lock
                // still prevents duplicates if cleanup is denied by policy.
                if registry(RunEntry::Exists) == Ok(true) {
                    let _ = registry(RunEntry::Disable);
                }
                Ok(true)
            } else {
                registry(RunEntry::Enable)
            }
        } else {
            let scheduled = self.set(false, run);
            let legacy = registry(RunEntry::Disable);
            scheduled?;
            legacy?;
            Ok(false)
        }
    }
}
fn registry(spec: &Spec, action: RunEntry) -> Result<bool, String> {
    use windows_sys::{
        core::w,
        Win32::{
            Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND},
            System::Registry::*,
        },
    };
    let name: Vec<u16> = spec.task.encode_utf16().chain(Some(0)).collect();
    let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let error = |code| std::io::Error::from_raw_os_error(code as i32).to_string();
    if action == RunEntry::Exists {
        let code = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                path,
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        return match code {
            0 => Ok(true),
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(false),
            _ => Err(error(code)),
        };
    }
    if action == RunEntry::Disable {
        let code = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, path, name.as_ptr()) };
        return match code {
            0 | ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(false),
            _ => Err(error(code)),
        };
    }
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }
    let mut raw = std::ptr::null_mut();
    let code = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path,
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut raw,
            std::ptr::null_mut(),
        )
    };
    if code != 0 {
        return Err(error(code));
    }
    let key = Key(raw);
    let command = format!("\"{}\" --tray", spec.executable.to_string_lossy());
    let bytes: Vec<u8> = command
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let code = unsafe {
        RegSetValueExW(
            key.0,
            name.as_ptr(),
            0,
            REG_SZ,
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    };
    if code != 0 {
        return Err(error(code));
    }
    Ok(true)
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn scheduled_enabled(bytes: &[u8]) -> Result<bool, String> {
    let text = if bytes.starts_with(&[0xff, 0xfe]) {
        let units: Vec<_> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let document = roxmltree::Document::parse(&text)
        .map_err(|e| format!("起動設定XMLを読み取れません: {e}"))?;
    let settings = document
        .descendants()
        .find(|node| node.has_tag_name("Settings"))
        .ok_or("起動設定の状態がありません")?;
    Ok(!settings
        .children()
        .find(|node| node.has_tag_name("Enabled"))
        .and_then(|node| node.text())
        .is_some_and(|text| text.trim() == "false"))
}
fn run(args: &[OsString]) -> Result<i32, String> {
    // The installer can call this process elevated. Do not trust an inherited
    // SystemRoot when selecting the executable that manages scheduled tasks.
    let mut system = vec![0u16; 32768];
    let count = unsafe {
        windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
            system.as_mut_ptr(),
            system.len() as u32,
        )
    } as usize;
    if count == 0 || count >= system.len() {
        return Err("Windowsのシステムフォルダーを取得できません".into());
    }
    let system = PathBuf::from(OsString::from_wide(&system[..count]));
    let query = args.first().is_some_and(|arg| arg == "/Query");
    let mut command = std::process::Command::new(system.join("schtasks.exe"));
    command.args(args);
    if query {
        command.arg("/XML");
    }
    let mut child = command
        .creation_flags(0x08000000)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("起動設定の応答を取得できません")?;
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let bytes = reader
                .join()
                .map_err(|_| "起動設定の読み取りが終了しました")?
                .map_err(|e| e.to_string())?;
            if query && status.success() {
                return scheduled_enabled(&bytes).map(|enabled| if enabled { 0 } else { 1 });
            }
            return Ok(status.code().unwrap_or(-1));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("起動設定の処理がタイムアウトしました".into());
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Maintenance {
    StopTray,
    Query,
    Rebind,
    Disable,
}
impl Maintenance {
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "stop-tray" => Ok(Self::StopTray),
            "query" => Ok(Self::Query),
            "rebind" => Ok(Self::Rebind),
            "disable" => Ok(Self::Disable),
            _ => Err("startup action must be query, rebind, disable or stop-tray".into()),
        }
    }
}
type Request = (Maintenance, Option<PathBuf>);
fn arguments(args: &[OsString]) -> Result<Option<Request>, String> {
    if !args
        .iter()
        .any(|a| a == "--startup-action" || a == "--startup-result")
    {
        return Ok(None);
    }
    let mut action = None;
    let mut result = None;
    let (pairs, remainder) = args.as_chunks::<2>();
    for pair in pairs {
        if pair[0] == "--startup-action" && action.is_none() {
            action = Some(Maintenance::parse(
                pair[1].to_str().ok_or("invalid action")?,
            )?);
        } else if pair[0] == "--startup-result" && result.is_none() && !pair[1].is_empty() {
            result = Some(PathBuf::from(&pair[1]));
        } else {
            return Err("invalid or duplicate startup argument".into());
        }
    }
    if !remainder.is_empty() {
        return Err("missing startup argument value".into());
    }
    Ok(Some((
        action.ok_or("--startup-action is required")?,
        result,
    )))
}
fn maintain(
    spec: &Spec,
    action: Maintenance,
    run: &mut impl FnMut(&[OsString]) -> Result<i32, String>,
    registry: &mut impl FnMut(RunEntry) -> Result<bool, String>,
) -> Result<(bool, bool), String> {
    let enabled = spec.query_combined(run, registry)?;
    match action {
        Maintenance::StopTray => Err("stop-tray requires the cooperative tray endpoint".into()),
        Maintenance::Query => Ok((enabled, false)),
        Maintenance::Rebind if !enabled => Ok((false, false)),
        Maintenance::Rebind => {
            // Never leave an old task racing an updated Run entry after a task
            // replacement fails. Fallback is valid only if no task existed.
            let scheduled = spec.query(run)?;
            match spec.set(true, run) {
                Ok(_) => {
                    registry(RunEntry::Disable)?;
                    Ok((true, true))
                }
                Err(error) if scheduled => Err(error),
                Err(_) => registry(RunEntry::Enable).map(|v| (v, true)),
            }
        }
        Maintenance::Disable => spec
            .set_combined(false, run, registry)
            .map(|v| (v, enabled)),
    }
}
/// Headless, current-user startup maintenance for the package manager. This
/// returns before egui, tray setup or user-data loading can have side effects.
pub fn command_line(args: &[OsString]) -> Option<i32> {
    let request = match arguments(args) {
        Ok(None) => return None,
        Ok(Some(request)) => request,
        Err(error) => {
            eprintln!("{error}");
            return Some(2);
        }
    };
    let result = if request.0 == Maintenance::StopTray {
        crate::tray_control::stop(&meltype_core::storage::Store::default_directory())
            .map(|stopped| (false, stopped))
    } else {
        Spec::current().and_then(|spec| {
            maintain(&spec, request.0, &mut run, &mut |action| {
                registry(&spec, action)
            })
        })
    };
    let value = match &result {
        Ok((enabled, changed)) => {
            serde_json::json!({"protocol":1,"ok":true,"enabled":enabled,"changed":changed})
        }
        Err(error) => serde_json::json!({"protocol":1,"ok":false,"error":error}),
    };
    if let Some(path) = request.1 {
        let bytes = serde_json::to_vec(&value).expect("startup result JSON");
        if let Err(error) = meltype_core::storage::atomic_write(&path, &bytes) {
            eprintln!("startup result: {error}");
            return Some(1);
        }
    } else {
        println!("{value}");
    }
    Some(i32::from(result.is_err()))
}
pub struct Startup {
    spec: Spec,
    receiver: Option<Receiver<Result<bool, String>>>,
    pub enabled: Option<bool>,
    pub pending: bool,
}
impl Startup {
    pub fn new(ctx: &egui::Context) -> Result<Self, String> {
        let mut result = Self {
            spec: Spec::current()?,
            receiver: None,
            enabled: None,
            pending: false,
        };
        result.request(None, ctx)?;
        Ok(result)
    }
    pub fn request(&mut self, value: Option<bool>, ctx: &egui::Context) -> Result<(), String> {
        if self.pending {
            return Err("起動設定の処理中です".into());
        }
        let spec = self.spec.clone();
        let ctx = ctx.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("meltype-startup".into())
            .spawn(move || {
                let result = match value {
                    Some(value) => {
                        spec.set_combined(value, &mut run, &mut |action| registry(&spec, action))
                    }
                    None => spec.query_combined(&mut run, &mut |action| registry(&spec, action)),
                };
                let _ = sender.send(result);
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        self.receiver = Some(receiver);
        self.pending = true;
        Ok(())
    }
    pub fn poll(&mut self) -> Option<Result<bool, String>> {
        let receiver = self.receiver.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err("起動設定の処理が終了しました".into()),
        };
        self.receiver = None;
        self.pending = false;
        if let Ok(enabled) = result {
            self.enabled = Some(enabled);
        }
        Some(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_task_is_not_reported_as_enabled() {
        assert!(
            !scheduled_enabled(b"<Task><Settings><Enabled>false</Enabled></Settings></Task>")
                .unwrap()
        );
        assert!(scheduled_enabled(b"<Task><Triggers><Enabled>false</Enabled></Triggers><Settings><Enabled>true</Enabled></Settings></Task>").unwrap());
    }
    #[test]
    fn maintenance_arguments_are_explicit_and_never_fall_through_to_gui() {
        let args = |s: &[&str]| s.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(arguments(&args(&["--tray"])).unwrap().is_none());
        let request = arguments(&args(&[
            "--startup-result",
            "C:/結果.json",
            "--startup-action",
            "query",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(
            request,
            (Maintenance::Query, Some(PathBuf::from("C:/結果.json")))
        );
        for input in [
            vec!["--startup-action"],
            vec!["--startup-action", "unknown"],
            vec!["--startup-result", "file"],
            vec!["--startup-action", "query", "--tray"],
            vec!["--startup-action", "query", "--startup-action", "disable"],
        ] {
            assert!(arguments(&args(&input)).is_err(), "{input:?}");
        }
    }
    #[test]
    fn maintenance_rebind_preserves_opt_out_and_updates_enabled_run_fallback() {
        let spec = spec();
        let mut calls = vec![];
        let mut invoke = |args: &[OsString]| {
            calls.push(args.to_vec());
            Ok(1)
        };
        assert_eq!(
            maintain(&spec, Maintenance::Rebind, &mut invoke, &mut |_| Ok(false)),
            Ok((false, false))
        );
        assert_eq!(calls.len(), 1, "disabled startup must not be enabled");
        let mut created = false;
        let mut registry_ops = vec![];
        let mut invoke = |args: &[OsString]| {
            if args[0] == "/Create" {
                let path = PathBuf::from(&args[4]);
                let bytes = std::fs::read(path).unwrap();
                let units: Vec<_> = bytes[2..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect();
                let xml = String::from_utf16(&units).unwrap();
                assert!(xml.contains(&escape(&spec.executable.to_string_lossy())));
                created = true;
                Ok(0)
            } else {
                Ok(1)
            }
        };
        let result = maintain(&spec, Maintenance::Rebind, &mut invoke, &mut |op| {
            registry_ops.push(op);
            Ok(true)
        });
        assert_eq!(result, Ok((true, true)));
        assert!(created);
        assert_eq!(registry_ops, vec![RunEntry::Exists, RunEntry::Disable]);
    }
    #[test]
    fn maintenance_disables_both_owned_entries_and_query_failure_does_not_mutate() {
        let spec = spec();
        let mut calls = vec![];
        let mut registry_ops = vec![];
        let result = maintain(
            &spec,
            Maintenance::Disable,
            &mut |args| {
                calls.push(args[0].clone());
                Ok(0)
            },
            &mut |op| {
                registry_ops.push(op);
                Ok(false)
            },
        );
        assert_eq!(result, Ok((false, true)));
        assert_eq!(
            calls,
            vec![OsString::from("/Query"), "/Query".into(), "/Delete".into()]
        );
        assert_eq!(registry_ops, vec![RunEntry::Disable]);
        let mut calls = vec![];
        assert!(maintain(
            &spec,
            Maintenance::Rebind,
            &mut |args| {
                calls.push(args[0].clone());
                Ok(5)
            },
            &mut |_| Ok(false)
        )
        .is_err());
        assert_eq!(calls, vec![OsString::from("/Query")]);
    }
    #[test]
    fn rebind_refuses_to_hide_failure_replacing_an_existing_task() {
        let mut commands = vec![];
        let result = maintain(
            &spec(),
            Maintenance::Rebind,
            &mut |args| {
                commands.push(args[0].clone());
                Ok(if args[0] == "/Create" { 5 } else { 0 })
            },
            &mut |_| panic!("must not create a competing Run entry"),
        );
        assert!(result.is_err());
        assert_eq!(commands.last(), Some(&OsString::from("/Create")));
        // When no task exists, a policy-denied Create may update the owned Run entry.
        let mut registry_ops = vec![];
        let result = maintain(
            &spec(),
            Maintenance::Rebind,
            &mut |args| Ok(if args[0] == "/Query" { 1 } else { 5 }),
            &mut |op| {
                registry_ops.push(op);
                Ok(true)
            },
        );
        assert_eq!(result, Ok((true, true)));
        assert_eq!(registry_ops, vec![RunEntry::Exists, RunEntry::Enable]);
    }
    fn spec() -> Spec {
        Spec::new(
            "DOMAIN\\藤島&<\"'".into(),
            PathBuf::from("C:/Some & <folder>/Meltype/settings.exe"),
        )
    }
    #[test]
    fn task_xml_round_trips_paths_and_uses_current_user_least_privilege_logon() {
        let spec = spec();
        let xml = spec.xml();
        let document = roxmltree::Document::parse(&xml).unwrap();
        let text = |name| {
            document
                .descendants()
                .find(|n| n.has_tag_name(name))
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(text("UserId"), spec.user);
        assert_eq!(text("Command"), spec.executable.to_string_lossy());
        assert_eq!(text("Arguments"), "--tray");
        assert_eq!(text("RunLevel"), "LeastPrivilege");
        assert_eq!(text("LogonType"), "InteractiveToken");
        assert_eq!(text("DisallowStartIfOnBatteries"), "false");
        assert_eq!(text("ExecutionTimeLimit"), "PT0S");
        assert!(spec.task.starts_with("Meltype-flash-"));
        assert_ne!(spec.task, "Meltype-FLASH");
        assert_eq!(
            spec.task,
            Spec::new(spec.user.to_uppercase(), spec.executable.clone()).task
        );
        assert_ne!(
            spec.task,
            Spec::new("OtherUser".into(), spec.executable.clone()).task
        );
    }
    #[test]
    fn startup_mutations_target_only_the_owned_task_and_write_utf16_without_a_shell() {
        let spec = spec();
        let mut calls = vec![];
        assert!(spec
            .set(true, &mut |args| {
                calls.push(args.to_vec());
                use std::{io::Read, os::windows::fs::OpenOptionsExt};
                let mut input = std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(1 | 4)
                    .open(&args[4])
                    .unwrap();
                let mut file = Vec::new();
                input.read_to_end(&mut file).unwrap();
                assert_eq!(&file[..2], &[0xff, 0xfe]);
                let units: Vec<u16> = file[2..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect();
                assert_eq!(String::from_utf16(&units).unwrap(), spec.xml());
                Ok(0)
            })
            .unwrap());
        assert_eq!(calls[0][0], "/Create");
        assert_eq!(calls[0][2], spec.task.as_str());
        assert!(!std::path::Path::new(&calls[0][4]).exists());
        calls.clear();
        assert!(!spec
            .set(false, &mut |args| {
                calls.push(args.to_vec());
                Ok(0)
            })
            .unwrap());
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1][0], "/Delete");
        assert_eq!(calls[1][2], spec.task.as_str());
        calls.clear();
        assert!(!spec
            .set(false, &mut |args| {
                calls.push(args.to_vec());
                Ok(1)
            })
            .unwrap());
        assert_eq!(calls.len(), 1);
        assert!(spec.set(true, &mut |_| Ok(5)).is_err());
    }
    #[test]
    fn startup_falls_back_to_owned_run_entry_and_disables_both_without_touching_real_registry() {
        let spec = spec();
        let mut entries = vec![];
        assert!(spec
            .set_combined(true, &mut |_| Ok(5), &mut |action| {
                entries.push(action);
                Ok(true)
            })
            .unwrap());
        assert_eq!(entries, [RunEntry::Enable]);
        entries.clear();
        assert!(spec
            .set_combined(true, &mut |_| Ok(0), &mut |action| {
                entries.push(action);
                Ok(true)
            })
            .unwrap());
        assert_eq!(entries, [RunEntry::Exists, RunEntry::Disable]);
        entries.clear();
        assert!(!spec
            .set_combined(false, &mut |_| Ok(0), &mut |action| {
                entries.push(action);
                Ok(false)
            })
            .unwrap());
        assert_eq!(entries, [RunEntry::Disable]);
        assert!(spec
            .query_combined(&mut |_| Ok(1), &mut |_| Ok(true))
            .unwrap());
        assert!(spec
            .set_combined(true, &mut |_| Ok(5), &mut |_| Err("denied".into()))
            .is_err());
    }
}
