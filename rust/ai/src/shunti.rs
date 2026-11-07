// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Android/iOS用の組み込み実行器。ファイルをmapし、入力スレッドの待機を制限する。
use memmap2::Mmap;
use std::{
    ffi::c_void,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};
unsafe extern "C" {
    fn kkc_open(
        lex: *const c_void,
        lex_size: usize,
        model: *const c_void,
        model_size: usize,
    ) -> *mut c_void;
    fn kkc_close(engine: *mut c_void);
    fn kkc_set_threads(engine: *mut c_void, threads: i32);
    fn kkc_convert(
        engine: *mut c_void,
        context: *const u16,
        nctx: i32,
        kana: *const u16,
        nk: i32,
        maxout: i32,
        model: i32,
        out: *mut u16,
        capacity: i32,
    ) -> i32;
}
pub struct Engine {
    raw: *mut c_void,
    _lex: Mmap,
    _model: Mmap,
}
impl Engine {
    pub fn open(directory: &Path) -> io::Result<Self> {
        fn map(path: &Path) -> io::Result<Mmap> {
            let file = File::open(path)?;
            let size = file.metadata()?.len();
            if !(16..=128 * 1024 * 1024).contains(&size) {
                return Err(io::Error::other("Invalid shunti data size"));
            }
            // Files are immutable, verified downloads installed by atomic directory rename.
            unsafe { memmap2::MmapOptions::new().map(&file) }
        }
        // Verify with a small streaming buffer before mmap; touching the entire
        // mapped dictionary would inflate a keyboard extension's resident memory.
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let catalog: serde_json::Value = serde_json::from_str(crate::MODEL_CATALOG).unwrap();
        let mut hashes = std::collections::HashMap::new();
        for name in ["kkc_lex.bin", "kkc_model.bin"] {
            let mut file = File::open(directory.join(name))?;
            let mut hash = Sha256::new();
            let mut buffer = [0; 65536];
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            hashes.insert(name, format!("{:x}", hash.finalize()));
        }
        if !catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["family"] == "Shunti")
            .any(|m| {
                m["files"].as_array().unwrap().iter().all(|f| {
                    hashes.get(f["name"].as_str().unwrap()).map(String::as_str)
                        == f["sha256"].as_str()
                })
            })
        {
            return Err(io::Error::other("shunti data SHA-256 mismatch"));
        }
        let lex = map(&directory.join("kkc_lex.bin"))?;
        let model = map(&directory.join("kkc_model.bin"))?;
        let raw = unsafe {
            kkc_open(
                lex.as_ptr().cast(),
                lex.len(),
                model.as_ptr().cast(),
                model.len(),
            )
        };
        if raw.is_null() {
            return Err(io::Error::other("shuntiの辞書・モデルを開けません"));
        }
        unsafe {
            kkc_set_threads(raw, 1);
        }
        Ok(Self {
            raw,
            _lex: lex,
            _model: model,
        })
    }
    pub fn convert(&mut self, reading: &str, context: &str) -> Vec<String> {
        let kana: Vec<u16> = reading.encode_utf16().collect();
        if kana.is_empty() || kana.len() > 160 || reading.chars().any(char::is_control) {
            return vec![];
        }
        let context: String = context
            .chars()
            .rev()
            .take(80)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let context: Vec<u16> = context.encode_utf16().collect();
        let mut output = vec![0; 65536];
        let count = unsafe {
            kkc_convert(
                self.raw,
                context.as_ptr(),
                context.len() as i32,
                kana.as_ptr(),
                kana.len() as i32,
                8,
                1,
                output.as_mut_ptr(),
                output.len() as i32,
            )
        };
        if !(1..=8).contains(&count) {
            return vec![];
        }
        output
            .split(|c| *c == 0)
            .take(count as usize)
            .filter_map(|s| String::from_utf16(s).ok())
            .filter(|s| !s.is_empty() && !s.chars().any(char::is_control))
            .collect()
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            kkc_close(self.raw);
        }
    }
}
struct Request {
    reading: String,
    context: String,
    reply: mpsc::Sender<Vec<String>>,
}
#[derive(Default)]
pub struct Runtime {
    directory: Option<PathBuf>,
    requests: Option<SyncSender<Request>>,
    ready: Arc<AtomicBool>,
    pause: Option<Instant>,
}
impl Runtime {
    pub fn configure(&mut self, directory: Option<&Path>) {
        if self.directory.as_deref() == directory {
            return;
        }
        self.requests = None;
        self.directory = directory.map(Path::to_path_buf);
        self.ready = Arc::new(AtomicBool::new(false));
        self.pause = None;
        if let Some(directory) = self.directory.clone() {
            let (sender, receiver) = mpsc::sync_channel::<Request>(1);
            self.requests = Some(sender);
            let ready = self.ready.clone();
            std::thread::spawn(move || {
                let Ok(mut engine) = Engine::open(&directory) else {
                    return;
                };
                ready.store(true, Ordering::Release);
                while let Ok(request) = receiver.recv() {
                    let _ = request
                        .reply
                        .send(engine.convert(&request.reading, &request.context));
                }
                ready.store(false, Ordering::Release);
            });
        }
    }
    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
    pub fn convert(&mut self, reading: &str, context: &str) -> Option<Vec<String>> {
        if !self.ready() || self.pause.is_some_and(|until| Instant::now() < until) {
            return None;
        }
        let (reply, receiver) = mpsc::channel();
        self.requests
            .as_ref()?
            .try_send(Request {
                reading: reading.into(),
                context: context.into(),
                reply,
            })
            .ok()?;
        match receiver.recv_timeout(Duration::from_millis(400)) {
            Ok(candidates) if !candidates.is_empty() => Some(candidates),
            _ => {
                self.pause = Some(Instant::now() + Duration::from_secs(5));
                None
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_truncated_optional_files_never_enable_inference() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Engine::open(dir.path()).is_err());
        std::fs::write(dir.path().join("kkc_lex.bin"), b"invalid").unwrap();
        std::fs::write(dir.path().join("kkc_model.bin"), b"invalid").unwrap();
        assert!(Engine::open(dir.path()).is_err());
        let mut runtime = Runtime::default();
        runtime.configure(Some(dir.path()));
        assert!(runtime.convert("かな", "").is_none());
        runtime.configure(None);
        assert!(!runtime.ready());
        assert!(runtime.convert("かな", "").is_none());
    }
}
