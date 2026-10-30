// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Durable user data. Every mutation re-reads under an OS file lock, preventing
//! lost updates between IME sessions, settings windows and other processes.
use crate::settings::Settings;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
mod sqlite_dictionary;
mod upstream;

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("file has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Store {
    pub directory: PathBuf,
}
struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

mod state;
pub use state::{Change, LanguageEntry, Phrase, State};

impl Store {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
    pub fn default_directory() -> PathBuf {
        if let Some(path) = std::env::var_os("MELTYPE_FLASH_DATA_DIR")
            .or_else(|| std::env::var_os("MELTYPE_RUST_DATA_DIR"))
        {
            return path.into();
        }
        #[cfg(windows)]
        {
            PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
                .join("Meltype-flash")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                .join("Library/Application Support/Meltype-flash")
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                        .join(".local/share")
                })
                .join("Meltype-flash")
        }
    }
    fn lock(&self) -> io::Result<Lock> {
        fs::create_dir_all(&self.directory)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.directory.join(".lock"))?;
        fs2::FileExt::lock_exclusive(&file)?;
        Ok(Lock(file))
    }
    pub fn settings(&self) -> io::Result<Settings> {
        let (settings, migrate) = self.read_settings()?;
        if !migrate {
            return Ok(settings);
        }
        // Re-read after acquiring the cross-process lock; never overwrite a newer edit.
        // A read-only config can still supply its migrated in-memory settings.
        if let Ok(_lock) = self.lock() {
            let (current, migrate) = self.read_settings()?;
            if migrate {
                let bytes = serde_json::to_vec_pretty(&current).map_err(io::Error::other)?;
                let _ = atomic_write(&self.directory.join("config.json"), &bytes);
            }
            return Ok(current);
        }
        Ok(settings)
    }
    fn read_settings(&self) -> io::Result<(Settings, bool)> {
        match fs::read(self.directory.join("config.json")) {
            Ok(bytes) => {
                let (text, _) = crate::user_dictionary::decode(&bytes)?;
                Settings::parse_migrating(&text).map_err(io::Error::other)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let mut s = Settings::default();
                s.normalize();
                Ok((s, false))
            }
            Err(e) => Err(e),
        }
    }
    pub fn change_settings<T>(
        &self,
        f: impl FnOnce(&mut Settings) -> Result<T, String>,
    ) -> io::Result<T> {
        let _lock = self.lock()?;
        let (mut settings, _) = self.read_settings()?;
        let result = f(&mut settings).map_err(io::Error::other)?;
        atomic_write(
            &self.directory.join("config.json"),
            &serde_json::to_vec_pretty(&settings)?,
        )?;
        Ok(result)
    }
    pub fn load(&self) -> io::Result<State> {
        let path = self.directory.join("state.json");
        let mut state: State = match fs::read(&path) {
            Ok(bytes) => {
                let (text, _) = crate::user_dictionary::decode(&bytes)?;
                let mut document: Value = serde_json::from_str(&text)?;
                let object = document
                    .as_object_mut()
                    .ok_or_else(|| io::Error::other("invalid state document"))?;
                if !object.contains_key("prefix_model") {
                    object.insert(
                        "prefix_model".into(),
                        serde_json::to_value(self.legacy_prefix_model()?)?,
                    );
                }
                serde_json::from_value(document).map_err(io::Error::other)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => self.import_legacy(),
            Err(e) => Err(e),
        }?;
        if let Some(words) = sqlite_dictionary::read(&self.directory)? {
            state.preferences.user_dictionary = words;
        }
        state.rebuild_language_preferences();
        for (key, stats) in &state.prefix_model.prefixes {
            state.used.insert(format!("prefix:{key}"), stats.used());
        }
        Ok(state)
    }
    pub fn update<T>(&self, f: impl FnOnce(&mut State) -> io::Result<T>) -> io::Result<T> {
        let _lock = self.lock()?;
        let mut state = self.load()?;
        let previous_dictionary = sqlite_dictionary::read(&self.directory)?;
        let result = f(&mut state)?;
        let path = self.directory.join("state.json");
        let original = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let dictionary = if previous_dictionary.as_ref() == Some(&state.preferences.user_dictionary)
        {
            None
        } else {
            Some(sqlite_dictionary::Write::prepare(
                &self.directory,
                &state.preferences.user_dictionary,
            )?)
        };
        // JSON retains learning/snippets and is not a second writable dictionary.
        let words = std::mem::take(&mut state.preferences.user_dictionary);
        let bytes = serde_json::to_vec_pretty(&state);
        state.preferences.user_dictionary = words;
        atomic_write(&path, &bytes?)?;
        if let Err(error) = dictionary.map_or(Ok(()), |writer| writer.commit()) {
            let rollback = match original {
                Some(bytes) => atomic_write(&path, &bytes),
                None => fs::remove_file(&path),
            };
            return Err(io::Error::other(format!(
                "{error}; JSON rollback: {rollback:?}"
            )));
        }
        Ok(result)
    }
    pub fn record(&self, changes: Vec<Change>) -> io::Result<State> {
        self.update(|state| {
            for change in changes {
                state.apply(change);
            }
            Ok(state.clone())
        })
    }
    fn import_legacy(&self) -> io::Result<State> {
        let mut state = State {
            prefix_model: self.legacy_prefix_model()?,
            ..Default::default()
        };
        let read = |name: &str| -> io::Result<Value> {
            match fs::read(self.directory.join(name)) {
                Ok(bytes) => {
                    let (text, _) = crate::user_dictionary::decode(&bytes)?;
                    serde_json::from_str(&text).map_err(io::Error::other)
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Value::Null),
                Err(e) => Err(e),
            }
        };
        if let Some(languages) = read("languages.json")?.as_object() {
            for (raw, entry) in languages {
                let english = entry["English"].as_bool().unwrap_or(false);
                state.preferences.english.insert(raw.clone(), english);
                state.language_counts.insert(
                    raw.clone(),
                    LanguageEntry {
                        english,
                        count: entry["Count"].as_u64().unwrap_or(1).min(u32::MAX as u64) as u32,
                        explicit: entry["Explicit"] == true,
                        used: parse_time(&entry["Used"]),
                    },
                );
            }
        }
        if let Some(conversions) = read("conversions.json")?.as_object() {
            for (reading, entry) in conversions {
                if let Some(text) = entry["Text"]
                    .as_str()
                    .filter(|_| reading.encode_utf16().count() >= 2)
                {
                    state
                        .preferences
                        .selected
                        .insert(reading.clone(), text.into());
                    state
                        .used
                        .insert(format!("conversion:{reading}"), parse_time(&entry["Used"]));
                }
            }
        }
        if let Some(translations) = read("translations.json")?.as_object() {
            for (reading, entries) in translations {
                if let Some(entries) = entries.as_object() {
                    for (word, count) in entries {
                        state
                            .translations
                            .entry(reading.clone())
                            .or_default()
                            .insert(
                                word.clone(),
                                count.as_u64().unwrap_or(1).min(u32::MAX as u64) as u32,
                            );
                    }
                }
            }
        }
        if let Some(snippets) = read("snippets.json")?.as_object() {
            for (name, text) in snippets {
                if let Some(text) = text
                    .as_str()
                    .filter(|text| !text.is_empty() && crate::control::snippet_name_valid(name))
                {
                    state
                        .snippets
                        .insert(name.clone(), crate::control::snippet_text(text));
                }
            }
        }
        match fs::read(self.directory.join("userdict.txt")) {
            Ok(bytes) => {
                let (text, _) = crate::user_dictionary::decode(&bytes)?;
                for line in text.split('\n').filter(|l| !l.starts_with('#')) {
                    let mut parts = line.trim_end_matches('\r').split('\t');
                    if let (Some(reading), Some(word)) = (parts.next(), parts.next()) {
                        let (reading, word) = (reading.trim(), word.trim());
                        if reading.encode_utf16().count() >= 2 && !word.is_empty() {
                            let words = state
                                .preferences
                                .user_dictionary
                                .entry(reading.into())
                                .or_default();
                            words.retain(|w| w != word);
                            words.insert(0, word.into());
                        }
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        match fs::read(self.directory.join("phrases.txt")) {
            Ok(bytes) => {
                let (text, _) = crate::user_dictionary::decode(&bytes)?;
                for line in text.lines() {
                    let p: Vec<_> = line.split('\t').collect();
                    if p.len() >= 4 && !p[0].is_empty() && !p[1].is_empty() {
                        if let (Ok(count), Ok(ticks)) = (p[2].parse::<i32>(), p[3].parse::<i64>()) {
                            let phrase = Phrase {
                                reading: p[0].into(),
                                text: p[1].into(),
                                count: count.max(1) as u32,
                                used: ticks.div_euclid(10_000_000) - 62_135_596_800,
                            };
                            if let Some(old) = state
                                .phrases
                                .iter_mut()
                                .find(|p| p.reading == phrase.reading && p.text == phrase.text)
                            {
                                *old = phrase;
                            } else {
                                state.phrases.push(phrase);
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        for (raw, entry) in &state.language_counts {
            state.used.insert(format!("language:{raw}"), entry.used);
        }
        Ok(state)
    }
    fn legacy_prefix_model(&self) -> io::Result<crate::prefix_learning::Model> {
        let path = self.directory.join("model.json");
        match fs::read(&path) {
            Ok(bytes) => match crate::user_dictionary::decode(&bytes).and_then(|(text, _)| {
                crate::prefix_learning::Model::parse(text.as_bytes()).map_err(io::Error::other)
            }) {
                Ok(model) => Ok(model),
                Err(_) => {
                    let _ = atomic_write(&self.directory.join("model.json.broken"), &bytes);
                    Ok(Default::default())
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Default::default()),
            Err(e) => Err(e),
        }
    }
    pub fn backup(&self) -> io::Result<Vec<u8>> {
        self.backup_with_neural_policy(cfg!(any(target_os = "android", target_os = "ios")))
    }
    fn backup_with_neural_policy(&self, omit_neural: bool) -> io::Result<Vec<u8>> {
        let _lock = self.lock()?;
        let mut files = BTreeMap::new();
        for name in ROOT_FILES {
            let path = safe_path(&self.directory, name)?;
            if path.is_file() {
                let bytes = fs::read(path)?;
                let bytes = if *name == "config.json" {
                    archive_config(bytes, omit_neural)?
                } else {
                    bytes
                };
                files.insert((*name).to_owned(), STANDARD.encode(bytes));
            }
        }
        if self.directory.join("state.json").exists()
            || self.directory.join("userdict.sqlite3").exists()
        {
            let state = self.load()?;
            files.insert(
                "state.json".into(),
                STANDARD.encode(serde_json::to_vec_pretty(&state)?),
            );
            for (name, bytes) in self.upstream_files(&state)? {
                files.insert(name, STANDARD.encode(bytes));
            }
        }
        let dictionaries = self.directory.join("dictionaries");
        if dictionaries.exists() {
            safe_path(&self.directory, "dictionaries")?;
            for entry in fs::read_dir(dictionaries)? {
                let entry = entry?;
                let name = format!("dictionaries/{}", entry.file_name().to_string_lossy());
                if restorable_name(&name) {
                    files.insert(
                        name.clone(),
                        STANDARD.encode(fs::read(safe_path(&self.directory, &name)?)?),
                    );
                }
            }
        }
        Ok(serde_json::to_vec_pretty(
            &json!({"format":"meltype-backup","version":1,"app":env!("CARGO_PKG_VERSION"),"created":chrono::Utc::now().to_rfc3339(),"files":files}),
        )?)
    }
    pub fn inspect_backup(bytes: &[u8]) -> io::Result<Value> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(io::Error::other("backup too large"));
        }
        let value: Value = serde_json::from_slice(bytes)?;
        if value["format"] != "meltype-backup"
            || value["version"] != 1
            || !value["files"].is_object()
        {
            return Err(io::Error::other("invalid backup format or version"));
        }
        Ok(value)
    }
    pub fn restore(&self, bytes: &[u8]) -> io::Result<usize> {
        self.restore_with_neural_policy(bytes, cfg!(any(target_os = "android", target_os = "ios")))
    }
    fn restore_with_neural_policy(&self, bytes: &[u8], omit_neural: bool) -> io::Result<usize> {
        let value = Self::inspect_backup(bytes)?;
        let _lock = self.lock()?;
        let mut files = self.backup_entries(&value, omit_neural)?;
        let supplied_count = files.len();
        let has_state = value["files"].get("state.json").is_some();
        if !has_state
            && upstream::FILES
                .iter()
                .any(|n| value["files"].get(*n).is_some())
        {
            // Merge the incoming upstream files over the latest canonical
            // state before writing. Otherwise an existing state.json silently
            // shadows learning/dictionary changes restored from a C# backup.
            let staged = tempfile::tempdir_in(&self.directory)?;
            let mut current = self.load()?;
            for (name, bytes) in self.upstream_files(&current)? {
                fs::write(staged.path().join(name), bytes)?;
            }
            for (path, bytes) in &files {
                let relative = path
                    .strip_prefix(&self.directory)
                    .map_err(io::Error::other)?;
                if relative.components().count() == 1
                    && upstream::FILES.iter().any(|n| relative == Path::new(n))
                {
                    fs::write(staged.path().join(relative), bytes)?;
                }
            }
            let imported = Store::new(staged.path()).load()?;
            let present = |name: &str| value["files"].get(name).is_some();
            if present("model.json") {
                current.prefix_model = imported.prefix_model;
                current.used.retain(|key, _| !key.starts_with("prefix:"));
                current.used.extend(
                    imported
                        .used
                        .iter()
                        .filter(|(key, _)| key.starts_with("prefix:"))
                        .map(|(k, v)| (k.clone(), *v)),
                );
            }
            if present("languages.json") {
                current.language_counts = imported.language_counts;
                current.preferences.english = imported.preferences.english;
                current.used.retain(|key, _| !key.starts_with("language:"));
                current.used.extend(
                    imported
                        .used
                        .iter()
                        .filter(|(key, _)| key.starts_with("language:"))
                        .map(|(k, v)| (k.clone(), *v)),
                );
            }
            if present("conversions.json") {
                let old_readings: Vec<_> = current.preferences.selected.keys().cloned().collect();
                for reading in old_readings {
                    if !current.translations.contains_key(&reading) {
                        current.used.remove(&format!("conversion:{reading}"));
                    }
                }
                current.preferences.selected = imported.preferences.selected;
                for reading in current.preferences.selected.keys() {
                    let key = format!("conversion:{reading}");
                    if let Some(used) = imported.used.get(&key) {
                        current.used.insert(key, *used);
                    }
                }
            }
            if present("translations.json") {
                current.translations = imported.translations;
                current.used.retain(|key, _| {
                    key.strip_prefix("conversion:").is_none_or(|reading| {
                        current.preferences.selected.contains_key(reading)
                            || current.translations.contains_key(reading)
                    })
                });
            }
            if present("snippets.json") {
                current.snippets = imported.snippets;
            }
            if present("userdict.txt") {
                current.preferences.user_dictionary = imported.preferences.user_dictionary;
            }
            if present("phrases.txt") {
                current.phrases = imported.phrases;
            }
            files.push((
                safe_path(&self.directory, "state.json")?,
                serde_json::to_vec_pretty(&current)?,
            ));
        }
        // Parse an incoming canonical state before any write. Dictionary rows
        // and file restoration share rollback on observable failures.
        let restored_words = files
            .iter()
            .find(|(path, _)| path.file_name().is_some_and(|name| name == "state.json"))
            .map(|(_, bytes)| {
                serde_json::from_slice::<State>(bytes)
                    .map(|state| state.preferences.user_dictionary)
            })
            .transpose()?;
        // Snapshot every target first, including directory/read failures.
        let originals: Vec<_> = files
            .iter()
            .map(|(path, _)| match fs::read(path) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e),
            })
            .collect::<io::Result<_>>()?;
        // Validate every entry before the first write; preserve old contents.
        for ((path, _), original) in files.iter().zip(&originals) {
            if let Some(original) = original {
                let backup = path.with_file_name(format!(
                    "{}.before-restore",
                    path.file_name().unwrap().to_string_lossy()
                ));
                atomic_write(&backup, original)?;
            }
        }
        let dictionary = restored_words
            .as_ref()
            .map(|words| sqlite_dictionary::Write::prepare(&self.directory, words))
            .transpose()?;
        let written = files
            .iter()
            .try_for_each(|(path, bytes)| atomic_write(path, bytes))
            .and_then(|_| dictionary.map_or(Ok(()), |writer| writer.commit()));
        if let Err(error) = written {
            let mut rollback_errors = vec![];
            for ((path, _), previous) in files.iter().zip(&originals).rev() {
                let restored = match previous {
                    Some(bytes) => atomic_write(path, bytes),
                    None => match fs::remove_file(path) {
                        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                        result => result,
                    },
                };
                if let Err(e) = restored {
                    rollback_errors.push(e.to_string());
                }
            }
            return Err(io::Error::other(format!(
                "restore failed: {error}; rollback errors: {rollback_errors:?}"
            )));
        }
        Ok(supplied_count)
    }
    /// Metadata for a restoration decision, without exposing encoded user data.
    /// Use the same target/path/base64 validation as restore, without writing.
    pub fn preview_backup(&self, bytes: &[u8]) -> io::Result<Value> {
        let value = Self::inspect_backup(bytes)?;
        let files =
            self.backup_entries(&value, cfg!(any(target_os = "android", target_os = "ios")))?;
        let mut categories = Vec::new();
        let names: Vec<_> = value["files"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|name| restorable_name(name))
            .map(String::as_str)
            .collect();
        for (category, present) in [
            ("設定", names.contains(&"config.json")),
            (
                "学習データ",
                names.iter().any(|n| {
                    matches!(
                        *n,
                        "state.json"
                            | "model.json"
                            | "conversions.json"
                            | "translations.json"
                            | "languages.json"
                            | "phrases.txt"
                    )
                }),
            ),
            (
                "ユーザー辞書",
                names.contains(&"state.json")
                    || names
                        .iter()
                        .any(|n| *n == "userdict.txt" || n.starts_with("dictionaries/")),
            ),
            (
                "定型文",
                names.contains(&"state.json") || names.contains(&"snippets.json"),
            ),
        ] {
            if present {
                categories.push(category);
            }
        }
        let created = value["created"]
            .as_str()
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .map(|date| date.to_rfc3339())
            .or_else(|| {
                value["created"]
                    .as_str()
                    .filter(|text| naive_time(text).is_some())
                    .map(str::to_owned)
            });
        Ok(json!({"created":created,"fileCount":files.len(),
            "bytes":files.iter().map(|(_,bytes)|bytes.len() as u64).sum::<u64>(),
            "categories":categories,"ignoredFiles":value["files"].as_object().unwrap().len()-files.len()}))
    }
    fn backup_entries(
        &self,
        value: &Value,
        omit_neural: bool,
    ) -> io::Result<Vec<(PathBuf, Vec<u8>)>> {
        value["files"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(n, _)| restorable_name(n))
            .map(|(name, text)| {
                let path = safe_path(&self.directory, name)?;
                let bytes = STANDARD
                    .decode(
                        text.as_str()
                            .ok_or_else(|| io::Error::other("invalid backup bytes"))?,
                    )
                    .map_err(io::Error::other)?;
                let bytes = if name == "config.json" {
                    archive_config(bytes, omit_neural)?
                } else {
                    bytes
                };
                Ok((path, bytes))
            })
            .collect()
    }
}
// Mobile backups contain the effective supported settings, rather than opaque
// desktop model paths. Desktop archives preserve their original bytes exactly.
fn archive_config(bytes: Vec<u8>, omit_neural: bool) -> io::Result<Vec<u8>> {
    if !omit_neural {
        return Ok(bytes);
    }
    let (text, _) = crate::user_dictionary::decode(&bytes)?;
    let mut settings = Settings::parse(&text).map_err(io::Error::other)?;
    settings.remove_neural_settings();
    serde_json::to_vec_pretty(&settings).map_err(io::Error::other)
}
fn parse_time(v: &Value) -> i64 {
    v.as_str()
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|d| d.timestamp())
                .ok()
                .or_else(|| naive_time(s).map(|d| d.and_utc().timestamp()))
        })
        .unwrap_or(0)
}
fn naive_time(text: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f").ok()
}
const ROOT_FILES: &[&str] = &[
    "config.json",
    "state.json",
    "model.json",
    "conversions.json",
    "translations.json",
    "languages.json",
    "userdict.txt",
    "snippets.json",
    "phrases.txt",
];
pub fn restorable_name(name: &str) -> bool {
    if ROOT_FILES.contains(&name) {
        return true;
    }
    let Some(file) = name.strip_prefix("dictionaries/") else {
        return false;
    };
    if file.starts_with('.')
        || !file.ends_with(".txt")
        || file.encode_utf16().count() > 104
        || file
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return false;
    }
    let base = file.split('.').next().unwrap_or("").to_ascii_lowercase();
    !["con", "prn", "aux", "nul"].contains(&base.as_str())
        && !(base.len() == 4
            && (base.starts_with("com") || base.starts_with("lpt"))
            && base.as_bytes()[3].is_ascii_digit())
}
fn safe_path(root: &Path, name: &str) -> io::Result<PathBuf> {
    let mut path = root.to_path_buf();
    for part in Path::new(name).components() {
        match part {
            std::path::Component::Normal(p) => path.push(p),
            _ => return Err(io::Error::other("unsafe data path")),
        };
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(io::Error::other("symlink in data path"));
        }
    }
    Ok(path)
}

#[cfg(test)]
mod neural_archive_tests {
    use super::*;

    fn config() -> Vec<u8> {
        serde_json::to_vec_pretty(&json!({
            "SettingsVersion":7, "UseZenzai":true,
            "ZenzaiServerPath":"desktop-server", "ZenzaiModelPath":"desktop-model",
            "PredictiveCandidates":true, "LearningEnabled":false,
            "FutureSetting":{"preserve":[1,2,3]},
            "Profiles":[{"Name":"移行", "Values":{
                "UseZenzai":true, "ZenzaiServerPath":"profile-server", "ZenzaiModelPath":"profile-model",
                "PredictiveCandidates":false, "FutureSetting":{"nested":"kept"}
            }}]
        })).unwrap()
    }
    fn stored_config(backup: &[u8]) -> Vec<u8> {
        let archive: Value = serde_json::from_slice(backup).unwrap();
        STANDARD
            .decode(archive["files"]["config.json"].as_str().unwrap())
            .unwrap()
    }
    fn assert_mobile_config(bytes: &[u8]) {
        let value: Value = serde_json::from_slice(bytes).unwrap();
        for name in ["UseZenzai", "ZenzaiServerPath", "ZenzaiModelPath"] {
            assert!(value.get(name).is_none());
            for profile in value["Profiles"].as_array().unwrap() {
                assert!(profile["Values"].get(name).is_none());
            }
        }
        assert_eq!(value["PredictiveCandidates"], true);
        assert_eq!(value["LearningEnabled"], false);
        assert_eq!(value["FutureSetting"], json!({"preserve":[1,2,3]}));
        let profile = value["Profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["Name"] == "移行")
            .unwrap();
        assert_eq!(profile["Values"]["PredictiveCandidates"], false);
        assert_eq!(profile["Values"]["FutureSetting"], json!({"nested":"kept"}));
    }

    #[test]
    fn mobile_restore_and_reexport_strip_desktop_ai_without_losing_other_settings() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        fs::write(source.path().join("config.json"), config()).unwrap();
        fs::create_dir(source.path().join("dictionaries")).unwrap();
        fs::write(
            source.path().join("dictionaries/custom.txt"),
            b"future dictionary\n",
        )
        .unwrap();
        let source = Store::new(source.path());
        let desktop_archive = source.backup_with_neural_policy(false).unwrap();
        let target = Store::new(target.path());
        target
            .restore_with_neural_policy(&desktop_archive, true)
            .unwrap();
        assert_mobile_config(&fs::read(target.directory.join("config.json")).unwrap());
        let mobile_archive = target.backup_with_neural_policy(true).unwrap();
        assert_mobile_config(&stored_config(&mobile_archive));
        let archive: Value = serde_json::from_slice(&mobile_archive).unwrap();
        assert_eq!(
            STANDARD
                .decode(
                    archive["files"]["dictionaries/custom.txt"]
                        .as_str()
                        .unwrap()
                )
                .unwrap(),
            b"future dictionary\n"
        );
        // Export directly from a desktop document, before any settings read.
        assert_mobile_config(&stored_config(
            &source.backup_with_neural_policy(true).unwrap(),
        ));
    }

    #[test]
    fn desktop_backup_and_restore_keep_config_bytes_including_ai_exactly() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let bytes = [
            b"// retained desktop comment\n".as_slice(),
            config().as_slice(),
        ]
        .concat();
        fs::write(source.path().join("config.json"), &bytes).unwrap();
        let backup = Store::new(source.path())
            .backup_with_neural_policy(false)
            .unwrap();
        assert_eq!(stored_config(&backup), bytes);
        let target = Store::new(target.path());
        target.restore_with_neural_policy(&backup, false).unwrap();
        assert_eq!(
            fs::read(target.directory.join("config.json")).unwrap(),
            bytes
        );
    }

    #[test]
    fn malformed_mobile_config_fails_before_replacing_existing_data() {
        let target = tempfile::tempdir().unwrap();
        fs::write(target.path().join("config.json"), b"original bytes").unwrap();
        let backup = serde_json::to_vec(&json!({"format":"meltype-backup", "version":1,
            "files":{"config.json":STANDARD.encode(b"{invalid json")}}))
        .unwrap();
        assert!(Store::new(target.path())
            .restore_with_neural_policy(&backup, true)
            .is_err());
        assert_eq!(
            fs::read(target.path().join("config.json")).unwrap(),
            b"original bytes"
        );
        assert!(!target.path().join("config.json.before-restore").exists());
    }
}
