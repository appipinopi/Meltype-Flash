// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 利用状況の集計 (Aptabase)。送るのは出来事の名前と端末の種類・OSの版・アプリの版だけで、
//! 入力内容・入力履歴・辞書・設定値は送らない。同意した人だけが使い、送信は停止できない。
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const APP_KEY: &str = "A-US-4882313783";
const API_URL: &str = "https://us.aptabase.com/api/v0/events";
const SDK_VERSION: &str = "meltype-flash-http@1";

/// 背景スレッドで1件だけ送る。待たない・失敗しても本体の動作に影響させない。
pub fn track(event: &'static str) {
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        let _ = runtime.block_on(send(event));
    });
}

fn payload(event: &str) -> Value {
    json!([{
        "timestamp": timestamp(),
        "sessionId": session_id(),
        "eventName": event,
        "systemProps": {
            "isDebug": cfg!(debug_assertions),
            "locale": locale(),
            "osName": os_name(),
            "osVersion": os_version(),
            "appVersion": env!("CARGO_PKG_VERSION"),
            "appBuildNumber": env!("CARGO_PKG_VERSION"),
            "sdkVersion": SDK_VERSION,
            "deviceModel": ""
        }
    }])
}

async fn send(event: &str) -> Result<(), Box<dyn std::error::Error>> {
    let os_version = os_version();
    let locale = locale();
    let payload = payload(event);
    let response = reqwest::Client::new()
        .post(API_URL)
        .header("App-Key", APP_KEY)
        .header("User-Agent", format!("{}/{os_version} {locale}", os_name()))
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("Aptabase responded with {}", response.status()).into());
    }
    Ok(())
}

fn timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// SDKと同じ形 (エポック秒 * 10^8 + 乱数) の匿名セッションID。
fn session_id() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let random = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos()))
        .unwrap_or(0)
        % 100_000_000;
    format!("{}", seconds * 100_000_000 + random)
}

fn os_name() -> &'static str {
    if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "macOS"
    }
}

#[cfg(windows)]
fn os_version() -> String {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    unsafe { RtlGetVersion(&mut info) };
    format!(
        "{}.{}.{}",
        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
    )
}

#[cfg(target_os = "linux")]
fn os_version() -> String {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    for key in ["VERSION_ID", "PRETTY_NAME"] {
        if let Some(value) = text.lines().find_map(|line| {
            line.strip_prefix(&format!("{key}="))
                .map(|value| value.trim_matches('"').to_owned())
        }) {
            if !value.is_empty() {
                return value;
            }
        }
    }
    "unknown".into()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn os_version() -> String {
    "unknown".into()
}

#[cfg(windows)]
fn locale() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    use windows_sys::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    let length =
        unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH as i32) };
    if length <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(length as usize - 1)])
}

#[cfg(not(windows))]
fn locale() -> String {
    std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_shape() {
        let value = payload("app_open");
        assert_eq!(value[0]["eventName"], "app_open");
        assert_eq!(value[0]["systemProps"]["osName"], os_name());
        assert!(value[0]["systemProps"]["appVersion"].as_str().is_some());
        assert!(timestamp().ends_with('Z'));
        assert!(session_id().parse::<u64>().is_ok());
    }
}
