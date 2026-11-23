// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Windows の再生中メディアを `now_playing.txt` へ書く。
//!
//! 曲名・アーティストだけを端末内のファイルへ置き、入力の「なう」候補が読む。
//! 外部へは送らない。Windows 以外ではビルドしない。
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use meltype_core::storage::Store;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

const FILE: &str = "now_playing.txt";
const INTERVAL: Duration = Duration::from_secs(5);

pub struct NowPlaying {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl NowPlaying {
    pub fn start(store: Store) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            // WinRT の非同期 API を待つスレッド。既に初期化済みでも続行する。
            let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            let path = store.directory.join(FILE);
            let mut previous = String::new();
            loop {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let enabled = store
                    .settings()
                    .map(|settings| settings.boolean("NowPlayingCandidates"))
                    .unwrap_or(false);
                if !enabled {
                    if !previous.is_empty() {
                        let _ = std::fs::remove_file(&path);
                        previous.clear();
                    }
                    thread::sleep(Duration::from_millis(200));
                    continue;
                }
                if let Some(text) = current_media() {
                    if text != previous {
                        let _ = std::fs::write(&path, &text);
                        previous = text;
                    }
                } else if !previous.is_empty() {
                    let _ = std::fs::remove_file(&path);
                    previous.clear();
                }
                let mut waited = Duration::ZERO;
                while waited < INTERVAL && !flag.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(200));
                    waited += Duration::from_millis(200);
                }
            }
            let _ = std::fs::remove_file(&path);
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for NowPlaying {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn current_media() -> Option<String> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .get()
        .ok()?;
    let session = manager.GetCurrentSession().ok()?;
    let properties = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let title = properties.Title().ok()?.to_string();
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    let artist = properties.Artist().ok().map(|value| value.to_string());
    let artist = artist.as_deref().unwrap_or("").trim();
    Some(if artist.is_empty() {
        title.to_owned()
    } else {
        format!("{title} - {artist}")
    })
}
