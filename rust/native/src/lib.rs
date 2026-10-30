// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! C ABI for the existing macOS/Linux frontends and native embedding.
//! Sessions use opaque integer tokens. Stale handles fail without dereferencing
//! freed memory. Returned strings use malloc/free, including Swift callbacks.
use meltype_core::{session::Clause, Session};
mod fcitx;
mod fcitx_dispatch;
mod fcitx_view;
pub mod json_bridge;
mod macos_application;
mod macos_apply;
mod macos_candidates;
mod macos_dictionary;
mod macos_dictionary_dialog;
mod macos_editor;
mod macos_indicator;
mod macos_key;
mod macos_meaning;
mod macos_menu;
mod macos_preferences;
mod macos_registration;
mod macos_services;
mod macos_suggestion;
mod macos_view;
mod macos_window;
pub mod tip;
#[cfg(windows)]
pub mod tip_transport;
#[cfg(windows)]
mod tsf;
#[cfg(windows)]
mod windows_clipboard;
#[cfg(windows)]
pub mod windows_converter;
#[cfg(windows)]
mod windows_github;
mod word_checker;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

pub type ClausesCallback = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char;
pub type CandidatesCallback = unsafe extern "C" fn(*const c_char) -> *mut c_char;
pub type WordCallback = unsafe extern "C" fn(*const c_char) -> i32;

#[derive(Clone, Copy, Default)]
struct Callbacks {
    clauses: Option<ClausesCallback>,
    candidates: Option<CandidatesCallback>,
    convert: Option<CandidatesCallback>,
}
static CALLBACKS: Mutex<Callbacks> = Mutex::new(Callbacks {
    clauses: None,
    candidates: None,
    convert: None,
});
static NEXT: AtomicUsize = AtomicUsize::new(1);
static MOZC: Mutex<Option<meltype_core::mozc::Mozc>> = Mutex::new(None);
fn sessions() -> &'static Mutex<HashMap<usize, Session>> {
    static SESSIONS: OnceLock<Mutex<HashMap<usize, Session>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn run(
    handle: *mut c_void,
    f: impl FnOnce(&mut Session) -> meltype_core::SessionResult,
) -> *mut c_char {
    run_optional(handle, |session| Some(f(session)))
}
fn run_optional(
    handle: *mut c_void,
    f: impl FnOnce(&mut Session) -> Option<meltype_core::SessionResult>,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        #[cfg(windows)]
        let learning = windows_converter::has_worker();
        #[cfg(not(windows))]
        let learning = false;
        let mut all = sessions().lock().ok()?;
        let session = all.get_mut(&(handle as usize))?;
        session.collect_engine_learning(learning);
        if session.raw().is_empty() {
            if let Err(error) = session.reload() {
                session.storage_error = Some(error.to_string());
            }
        }
        let result = f(session);
        let notifications = session.take_engine_learning();
        drop(all);
        // Engine IPC and callbacks must not run under the session registry lock.
        #[cfg(windows)]
        windows_converter::learn(notifications);
        #[cfg(not(windows))]
        let _ = notifications;
        Some(allocate(&serde_json::to_string(&result?).ok()?))
    }))
    .ok()
    .flatten()
    .unwrap_or(std::ptr::null_mut())
}
fn allocate(text: &str) -> *mut c_char {
    // CString rejects interior NUL; result JSON escapes it.
    let Ok(text) = CString::new(text) else {
        return std::ptr::null_mut();
    };
    let bytes = text.as_bytes_with_nul();
    unsafe {
        let pointer = libc::malloc(bytes.len()).cast::<c_char>();
        if !pointer.is_null() {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast(), bytes.len());
        }
        pointer
    }
}
unsafe fn read(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        None
    } else {
        CStr::from_ptr(pointer).to_str().ok().map(str::to_owned)
    }
}
unsafe fn take(pointer: *mut c_char) -> Option<String> {
    let value = read(pointer);
    libc::free(pointer.cast());
    value
}

#[no_mangle]
pub extern "C" fn meltype_init(
    clauses: Option<ClausesCallback>,
    candidates: Option<CandidatesCallback>,
    is_word: Option<WordCallback>,
) -> i32 {
    let Ok(mut callbacks) = CALLBACKS.lock() else {
        return 0;
    };
    *callbacks = Callbacks {
        clauses,
        candidates,
        convert: None,
    };
    word_checker::configure(is_word);
    1
}

/// Optional standalone conversion for resized clauses. A segmented sentence
/// conversion can differ from converting the same reading as one clause.
/// Register after meltype_init; returned UTF-8 strings follow its malloc/free
/// ownership contract. Passing None restores the legacy segmented fallback.
#[no_mangle]
pub extern "C" fn meltype_set_converter(convert: Option<CandidatesCallback>) -> i32 {
    let Ok(mut callbacks) = CALLBACKS.lock() else {
        return 0;
    };
    callbacks.convert = convert;
    1
}

/// # Safety
/// Non-null helper/profile arguments must be valid NUL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn meltype_init_mozc(helper: *const c_char, profile: *const c_char) -> i32 {
    catch_unwind(|| {
        let Some(helper) = read(helper) else { return 0 };
        let profile = read(profile);
        let Ok(backend) = meltype_core::mozc::Mozc::start(
            std::path::Path::new(&helper),
            profile.as_deref().map(std::path::Path::new),
        ) else {
            return 0;
        };
        let Ok(mut mozc) = MOZC.lock() else { return 0 };
        *mozc = Some(backend);
        1
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn meltype_create() -> *mut c_void {
    catch_unwind(|| {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            return std::ptr::null_mut();
        }
        let Ok(mut all) = sessions().lock() else {
            return std::ptr::null_mut();
        };
        #[cfg(not(test))]
        let session = Session::open(meltype_core::storage::Store::new(
            meltype_core::storage::Store::default_directory(),
        ))
        .unwrap_or_else(|error| {
            let mut session = Session::default();
            session.storage_error = Some(error.to_string());
            session
        });
        #[cfg(test)]
        let session = Session::default();
        all.insert(id, session);
        id as *mut c_void
    })
    .unwrap_or(std::ptr::null_mut())
}
/// # Safety
/// directory must be a valid NUL-terminated UTF-8 filesystem path.
#[no_mangle]
pub unsafe extern "C" fn meltype_create_at(directory: *const c_char) -> *mut c_void {
    catch_unwind(|| {
        let Some(directory) = read(directory) else {
            return std::ptr::null_mut();
        };
        let Ok(session) = Session::open(meltype_core::storage::Store::new(directory)) else {
            return std::ptr::null_mut();
        };
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let Ok(mut all) = sessions().lock() else {
            return std::ptr::null_mut();
        };
        all.insert(id, session);
        id as *mut c_void
    })
    .unwrap_or(std::ptr::null_mut())
}
/// # Safety
/// request must be valid NUL-terminated UTF-8 JSON. Returned JSON is freed with
/// meltype_free, including error results. File operations are explicitly named.
#[no_mangle]
pub unsafe extern "C" fn meltype_control(
    handle: *mut c_void,
    request: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let result = (|| -> Result<serde_json::Value, String> {
            let text = read(request).ok_or("invalid request")?;
            if text.len() > 1024 * 1024 {
                return Err("request too large".into());
            }
            let request = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let mut all = sessions().lock().map_err(|e| e.to_string())?;
            all.get_mut(&(handle as usize))
                .ok_or("invalid session")?
                .control(&request)
                .map_err(|e| e.to_string())
        })();
        allocate(&match result {
            Ok(value) => value.to_string(),
            Err(error) => serde_json::json!({"error":error}).to_string(),
        })
    }))
    .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "C" fn meltype_destroy(handle: *mut c_void) {
    fcitx::forget(handle);
    macos_meaning::forget(handle);
    if let Ok(mut all) = sessions().lock() {
        all.remove(&(handle as usize));
    }
}

unsafe fn external_conversion(reading: &str, before: *const c_char) -> (Vec<Clause>, Vec<String>) {
    external_conversion_min(reading, before, 0, true)
}
unsafe fn external_conversion_min(
    reading: &str,
    before: *const c_char,
    minimum: usize,
    expand_candidates: bool,
) -> (Vec<Clause>, Vec<String>) {
    let mut spans = meltype_core::protection::scan(reading);
    // Detected English within a Japanese reading is literal composition text,
    // not romaji for the external converter. Keep complete alphanumeric runs.
    if reading.chars().any(meltype_core::linguistics::kana) {
        let mut at = 0;
        while at < reading.len() {
            let ch = reading[at..].chars().next().unwrap();
            if !ch.is_ascii_alphanumeric() {
                at += ch.len_utf8();
                continue;
            }
            let end = reading[at..]
                .find(|c: char| !c.is_ascii_alphanumeric())
                .map_or(reading.len(), |n| at + n);
            if reading[at..end].bytes().any(|c| c.is_ascii_alphabetic())
                && !spans.iter().any(|span| at < span.end && end > span.start)
            {
                spans.push(meltype_core::protection::Span {
                    start: at,
                    end,
                    kind: "detected_english",
                });
            }
            at = end;
        }
        spans.sort_by_key(|span| span.start);
    }
    if spans.is_empty() {
        if reading.encode_utf16().count() < minimum {
            return (Vec::new(), Vec::new());
        }
        return external_conversion_unprotected(reading, before, expand_candidates);
    }
    let mut segments = Vec::new();
    let mut offset = 0;
    for (start, end, kind) in spans
        .iter()
        .map(|span| (span.start, span.end, Some(span.kind)))
        .chain(std::iter::once((reading.len(), reading.len(), None)))
    {
        if offset < start {
            let gap = &reading[offset..start];
            let (clauses, candidates) = if gap.encode_utf16().count() < minimum {
                (Vec::new(), Vec::new())
            } else {
                external_conversion_unprotected(gap, before, expand_candidates)
            };
            if clauses.is_empty() {
                segments.push(Clause::new(gap.to_owned(), candidates));
            } else {
                segments.extend(clauses);
            }
        }
        if let Some(kind) = kind {
            let text = reading[start..end].to_owned();
            if kind == "detected_english" {
                segments.extend(
                    meltype_core::dictionary::Dictionary::bundled()
                        .english_parts(&text)
                        .into_iter()
                        .map(|word| Clause::literal(word.to_owned(), true)),
                );
            } else {
                segments.push(Clause::literal(text, false));
            }
        }
        offset = end;
    }
    (segments, Vec::new())
}
unsafe fn external_conversion_unprotected(
    reading: &str,
    before: *const c_char,
    expand_candidates: bool,
) -> (Vec<Clause>, Vec<String>) {
    let mut external = Vec::new();
    let mut segments = Vec::new();
    if let Ok(mut mozc) = MOZC.lock() {
        if let Some(backend) = mozc.as_mut() {
            if let Ok(values) = backend.clauses(reading, &read(before).unwrap_or_default()) {
                segments = values;
            } else {
                *mozc = None;
            }
        }
    }
    let callbacks = CALLBACKS.lock().map(|c| *c).unwrap_or_default();
    let input = CString::new(reading).unwrap();
    if let Some(callback) = callbacks.clauses {
        if let Some(text) = take(callback(input.as_ptr(), before)) {
            let clauses: Option<Vec<_>> = text.lines().map(|line| line.split_once('\t')).collect();
            if let Some(clauses) = clauses {
                if clauses.iter().map(|(r, _)| *r).collect::<String>() == reading {
                    segments = clauses
                        .into_iter()
                        .map(|(r, t)| {
                            let mut candidates = vec![];
                            if let (true, Some(callback), Ok(input)) =
                                (expand_candidates, callbacks.candidates, CString::new(r))
                            {
                                if let Some(text) = take(callback(input.as_ptr())) {
                                    candidates.extend(
                                        text.lines().filter(|s| !s.is_empty()).map(str::to_owned),
                                    );
                                }
                            }
                            let mut clause = Clause::new(r.to_owned(), vec![t.to_owned()]);
                            clause.expansion = Some(candidates);
                            if expand_candidates {
                                clause.standalone = Some(standalone_conversion(r, callbacks));
                            }
                            clause
                        })
                        .collect();
                }
            }
        }
    }
    if let (true, Some(callback)) = (expand_candidates, callbacks.candidates) {
        if let Some(text) = take(callback(input.as_ptr())) {
            external.extend(text.lines().filter(|s| !s.is_empty()).map(str::to_owned));
        }
    }
    (segments, external)
}

unsafe fn standalone_conversion(reading: &str, callbacks: Callbacks) -> String {
    let Ok(input) = CString::new(reading) else {
        return reading.into();
    };
    if let Some(callback) = callbacks.convert {
        return take(callback(input.as_ptr()))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| reading.into());
    }
    if let Some(callback) = callbacks.clauses {
        if let Some(text) = take(callback(input.as_ptr(), std::ptr::null())) {
            let parts: Option<Vec<_>> = text.lines().map(|line| line.split_once('\t')).collect();
            if let Some(parts) = parts.filter(|parts| {
                !parts.is_empty() && parts.iter().map(|(r, _)| *r).collect::<String>() == reading
            }) {
                return parts.iter().map(|(_, text)| *text).collect();
            }
        }
    }
    reading.into()
}
unsafe fn external_reconversion(reading: &str) -> Vec<String> {
    let callbacks = CALLBACKS.lock().map(|c| *c).unwrap_or_default();
    if let Some(convert) = callbacks.convert {
        let input = CString::new(reading).unwrap();
        // Upstream Convert falls back to the reading when the engine has no
        // result. A candidate-list reply must not replace this fallback.
        let text = take(convert(input.as_ptr()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| reading.to_owned());
        let mut candidates = vec![text];
        if let Some(more) = callbacks.candidates {
            if let Some(text) = take(more(input.as_ptr())) {
                candidates.extend(text.lines().filter(|s| !s.is_empty()).map(str::to_owned));
            }
        }
        candidates
    } else {
        let (clauses, mut candidates) =
            external_conversion_unprotected(reading, std::ptr::null(), true);
        if !clauses.is_empty() {
            candidates.insert(0, clauses.iter().map(Clause::text).collect());
        }
        candidates
    }
}
unsafe fn registered_conversion(
    handle: *mut c_void,
    reading: &str,
    before: *const c_char,
    original: Vec<Clause>,
) -> Vec<Clause> {
    let plan = sessions().lock().ok().and_then(|all| {
        all.get(&(handle as usize))
            .and_then(|session| session.registered_conversion_plan(reading, &original))
    });
    let Some(plan) = plan else {
        return original;
    };
    let mut result = vec![];
    for (reading, word) in plan {
        let (clauses, extra) = external_conversion_unprotected(&reading, before, true);
        if let Some(word) = word {
            let mut clause = Clause::new(reading.clone(), vec![word]);
            clause.standalone = Some(standalone_conversion(
                &reading,
                CALLBACKS.lock().map(|c| *c).unwrap_or_default(),
            ));
            clause.expansion = Some(extra);
            result.push(clause);
        } else if clauses.is_empty() {
            let mut values = vec![reading.clone()];
            values.extend(extra);
            result.push(Clause::new(reading, values));
        } else {
            result.extend(clauses);
        }
    }
    result
}

unsafe fn external_selection_conversion(reading: &str, prefer_romaji: bool) -> Vec<Clause> {
    // ConvertJapanese keeps unreadable ASCII/symbol runs literal. Digits
    // touching such runs stay with them; other digits can be converted.
    let chars: Vec<_> = reading.char_indices().collect();
    let mut keep: Vec<_> = chars
        .iter()
        .map(|(_, c)| c.is_ascii_graphic() && !c.is_ascii_digit())
        .collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].1.is_ascii_digit() {
            i += 1;
            continue;
        }
        let end = (i..chars.len())
            .find(|j| !chars[*j].1.is_ascii_digit())
            .unwrap_or(chars.len());
        let touches = i > 0 && keep[i - 1] || end < keep.len() && keep[end];
        keep[i..end].fill(touches);
        i = end;
    }
    let mut clauses = Vec::new();
    let mut start = 0;
    for end in 1..=chars.len() {
        if end < chars.len() && keep[end] == keep[start] {
            continue;
        }
        let stop = chars.get(end).map_or(reading.len(), |(i, _)| *i);
        let run = &reading[chars[start].0..stop];
        if keep[start] {
            // The detector keeps known English words and following unknown
            // spellings in separate segments (team|Xyz). Prefer-Japanese
            // conversion must preserve those boundaries as normal Space does.
            let parts = if prefer_romaji && run.bytes().all(|c| c.is_ascii_alphabetic()) {
                meltype_core::dictionary::Dictionary::bundled().english_parts(run)
            } else {
                vec![run]
            };
            for run in parts {
                let mut candidates = Vec::new();
                if prefer_romaji {
                    let analysis = meltype_core::romaji::analyze(&run.to_ascii_lowercase(), true);
                    let kana = analysis.kana();
                    if analysis.valid && analysis.partial.is_empty() && !kana.is_empty() {
                        candidates.extend(external_reconversion(&kana));
                        let katakana = meltype_core::romaji::katakana(&kana);
                        candidates.extend([
                            kana,
                            katakana.clone(),
                            meltype_core::kana::halfwidth(&katakana),
                        ]);
                    }
                    let mut capitalized = run.to_owned();
                    capitalized[..1].make_ascii_uppercase();
                    candidates.extend([run.into(), capitalized, run.to_ascii_uppercase()]);
                }
                candidates.extend([run.into(), meltype_core::kana::fullwidth(run)]);
                let mut clause = Clause::new(run.into(), candidates);
                if prefer_romaji {
                    clause.english = true;
                    clause.kana = Some(String::new());
                }
                clauses.push(clause);
            }
        } else {
            let (segments, candidates) =
                external_conversion_unprotected(run, std::ptr::null(), true);
            if segments.is_empty() {
                clauses.push(Clause::new(run.into(), candidates));
            } else {
                clauses.extend(segments);
            }
        }
        start = end;
    }
    clauses
}

/// # Safety
/// before/after, when non-null, must point to valid NUL-terminated UTF-8.
/// Calls for one session must be serialized. Registered callbacks must remain
/// valid until all sessions are destroyed, and return malloc-allocated strings.
#[no_mangle]
pub unsafe extern "C" fn meltype_handle_key(
    handle: *mut c_void,
    vk: i32,
    ch: i32,
    modifiers: i32,
    before: *const c_char,
    after: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let Ok(_spelling) =
            word_checker::prepare(handle as usize, vk, ch, modifiers, before, after)
        else {
            return std::ptr::null_mut();
        };
        #[cfg(windows)]
        let clipboard = if matches!(vk, 0x20 | 0x1c | 9)
            && modifiers & 14 == 0
            && sessions().lock().ok().is_some_and(|all| {
                all.get(&(handle as usize))
                    .is_some_and(Session::wants_clipboard_candidate)
            }) {
            windows_clipboard::read()
        } else {
            None
        };
        #[cfg(windows)]
        if let Ok(mut all) = sessions().lock() {
            if let Some(s) = all.get_mut(&(handle as usize)) {
                s.set_clipboard_candidate(clipboard.as_deref());
            }
        }
        #[cfg(windows)]
        if matches!(vk, 0x20 | 0x1c | 9) && modifiers & 14 == 0 {
            let request = sessions().lock().ok().and_then(|all| {
                all.get(&(handle as usize))
                    .and_then(Session::github_title_request)
            });
            let title = request.and_then(|(repo, number)| windows_github::lookup(&repo, number));
            if let Ok(mut all) = sessions().lock() {
                if let Some(s) = all.get_mut(&(handle as usize)) {
                    s.set_github_title(title);
                }
            }
        }
        let reading = sessions().lock().ok().and_then(|all| {
            all.get(&(handle as usize))
                .filter(|s| {
                    (vk == 0x20 || vk == 0x1c)
                        && !(vk == 0x20 && modifiers & 1 == 0 && s.space_commits_alphanumeric())
                        && !(vk == 0x20
                            && modifiers & 1 == 0
                            && !s.settings.boolean("SeparateKeyRoles")
                            && s.ends_in_english())
                        && (!s.protects_tail() || vk == 0x20 && modifiers & 1 != 0)
                        && !s.is_converting()
                        && !s.raw().is_empty()
                        && !s.direct
                        && !s.private
                        && s.settings.boolean("Enabled")
                        && !s.code_english()
                        && (s.display(true) != s.raw()
                            || s.raw().chars().any(meltype_core::linguistics::kana)
                            || s.raw().bytes().all(|c| !c.is_ascii_alphabetic())
                            || s.settings.boolean("SeparateKeyRoles")
                            || vk == 0x1c
                            || vk == 0x20 && modifiers & 1 != 0)
                        && modifiers & 14 == 0
                })
                .map(Session::conversion_reading)
        });
        let (mut segments, external) = reading
            .as_deref()
            .map(|r| {
                if vk == 0x1c || vk == 0x20 && modifiers & 1 != 0 {
                    (external_selection_conversion(r, true), Vec::new())
                } else {
                    external_conversion(r, before)
                }
            })
            .unwrap_or_default();
        if let Some(reading) = reading.as_ref() {
            segments = registered_conversion(handle, reading, before, segments);
        }
        if segments.is_empty()
            && external.is_empty()
            && CALLBACKS.lock().is_ok_and(|c| c.clauses.is_some())
        {
            if let Some(reading) = reading.as_ref() {
                segments.push(Clause::new(reading.clone(), vec![reading.clone()]));
            }
        }
        // Resize reconverts both affected clauses using their new readings.
        // Keep callbacks outside the registry mutex, as for initial conversion.
        let resize_before =
            if matches!(vk, 0x25 | 0x27) && modifiers & 15 == 1 || modifiers & 14 == 2 {
                sessions().lock().ok().and_then(|all| {
                    all.get(&(handle as usize))
                        .filter(|s| s.is_converting())
                        .map(Session::clause_readings)
                })
            } else {
                None
            };
        let starting_conversion = !matches!(vk, 0x20 | 0x1c)
            && sessions().lock().ok().is_some_and(|all| {
                all.get(&(handle as usize))
                    .is_some_and(|s| !s.is_converting())
            });
        let tab_raw = if vk == 0x09 {
            sessions().lock().ok().and_then(|all| {
                all.get(&(handle as usize))
                    .filter(|s| s.is_converting())
                    .map(|s| s.raw().to_owned())
            })
        } else {
            None
        };
        let result = run(handle, |s| {
            s.set_context(read(before).as_deref(), read(after).as_deref());
            if !segments.is_empty() && s.begin_key_clauses(segments, vk == 0x20) {
                s.result(true)
            } else if !external.is_empty() {
                s.begin_conversion(external);
                s.result(true)
            } else {
                let result = s.key(vk, ch, modifiers);
                if !result.consumed && (matches!(vk, 9 | 0x21..=0x28 | 0x2e) || modifiers & 2 != 0)
                {
                    s.caret_moved(vk, modifiers & 2 != 0);
                }
                result
            }
        });
        // Let Core decide whether Tab, arrows or configured Control keys start conversion: suggestions,
        // predictions and protected input take precedence. Only then request
        // converter clauses, outside the session mutex as for Space.
        let tab_reading = if starting_conversion || tab_raw.is_some() {
            sessions().lock().ok().and_then(|all| {
                all.get(&(handle as usize))
                    .filter(|s| {
                        starting_conversion && s.is_converting()
                            || s.is_converting()
                                && tab_raw.as_deref().is_some_and(|raw| raw != s.raw())
                    })
                    .map(Session::conversion_reading)
            })
        } else {
            None
        };
        let result = if let Some(reading) = tab_reading {
            let (mut clauses, candidates) = external_conversion(&reading, before);
            clauses = registered_conversion(handle, &reading, before, clauses);
            if clauses.is_empty() && CALLBACKS.lock().is_ok_and(|c| c.clauses.is_some()) {
                let mut values = vec![reading.clone()];
                values.extend(candidates);
                clauses.push(Clause::new(reading, values));
            }
            if !clauses.is_empty() {
                if let Some(text) = take(result) {
                    if let Ok(previous) = serde_json::from_str::<meltype_core::SessionResult>(&text)
                    {
                        run(handle, |s| {
                            s.begin_clauses(clauses);
                            let mut reply = s.result(previous.consumed);
                            reply.commits = previous.commits;
                            reply
                        })
                    } else {
                        allocate(&text)
                    }
                } else {
                    result
                }
            } else {
                result
            }
        } else {
            result
        };
        let result = if let Some(previous) = resize_before {
            let current = sessions()
                .lock()
                .ok()
                .and_then(|all| all.get(&(handle as usize)).map(Session::clause_readings))
                .unwrap_or_default();
            let mut refreshed = Vec::new();
            if current != previous {
                let start = current
                    .iter()
                    .zip(&previous)
                    .take_while(|(a, b)| a == b)
                    .count();
                let suffix = current[start..]
                    .iter()
                    .rev()
                    .zip(previous[start..].iter().rev())
                    .take_while(|(a, b)| a == b)
                    .count();
                for (index, reading) in current
                    .iter()
                    .enumerate()
                    .take(current.len() - suffix)
                    .skip(start)
                {
                    let candidates = external_reconversion(reading);
                    if !candidates.is_empty() {
                        refreshed.push((index, candidates));
                    }
                }
            }
            if refreshed.is_empty() {
                result
            } else if let Some(text) = take(result) {
                if let Ok(previous_result) =
                    serde_json::from_str::<meltype_core::SessionResult>(&text)
                {
                    run(handle, |s| {
                        s.refresh_clause_candidates(&current, refreshed);
                        let mut next = s.result(previous_result.consumed);
                        next.commits = previous_result.commits;
                        next.selection_expect = previous_result.selection_expect;
                        next
                    })
                } else {
                    allocate(&text)
                }
            } else {
                std::ptr::null_mut()
            }
        } else {
            result
        };
        let live = if modifiers & 14 == 0 && (ch >= 0x20 || vk == 8) {
            sessions()
                .lock()
                .ok()
                .and_then(|all| all.get(&(handle as usize)).and_then(Session::live_reading))
        } else {
            None
        };
        if let Some(reading) = live {
            // CompositionController.LiveConvert applies this threshold to
            // each Japanese segment, excluding protected/English islands.
            // Live preview needs the primary clause texts only. Enumerating
            // alternatives (and reconverting every clause standalone) on every
            // letter grows with sentence length and can kill the TSF worker.
            let (mut clauses, candidates) = external_conversion_min(&reading, before, 4, false);
            if clauses.is_empty() && !candidates.is_empty() {
                clauses.push(Clause::new(reading, candidates));
            }
            if !clauses.is_empty() {
                if let Some(text) = take(result) {
                    if let Ok(previous) = serde_json::from_str::<meltype_core::SessionResult>(&text)
                    {
                        return run(handle, |s| {
                            s.update_live_conversion(clauses);
                            let mut result = s.result(previous.consumed);
                            result.commits = previous.commits;
                            result
                        });
                    }
                    return allocate(&text);
                }
                return std::ptr::null_mut();
            }
        }
        result
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn meltype_commit(handle: *mut c_void) -> *mut c_char {
    let Ok(_spelling) = (unsafe {
        word_checker::prepare(
            handle as usize,
            13,
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    }) else {
        return std::ptr::null_mut();
    };
    run(handle, Session::commit)
}
#[no_mangle]
pub extern "C" fn meltype_commit_preedit(handle: *mut c_void) -> *mut c_char {
    let Ok(_spelling) = (unsafe {
        word_checker::prepare(
            handle as usize,
            13,
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    }) else {
        return std::ptr::null_mut();
    };
    run(handle, Session::commit_preedit)
}
/// # Safety
/// Strings must be valid NUL-terminated UTF-8; reading may be null to infer it.
/// The host must verify selection_expect before replacing the selected text.
#[no_mangle]
pub unsafe extern "C" fn meltype_reconvert_selection(
    handle: *mut c_void,
    text: *const c_char,
    reading: *const c_char,
) -> *mut c_char {
    let Some(text) = read(text) else {
        return std::ptr::null_mut();
    };
    let mut unchanged_romaji = false;
    let parts = sessions().lock().ok().and_then(|all| {
        all.get(&(handle as usize))
            .and_then(|s| s.romaji_reconversion_parts(&text))
    });
    if let Some(parts) = parts {
        let mut converted = String::new();
        for part in parts {
            if part.english {
                if !converted.is_empty() && !converted.ends_with(' ') {
                    converted.push(' ');
                }
                converted.push_str(&part.text);
                converted.push(' ');
            } else {
                let candidates = external_reconversion(&part.text);
                let value = candidates.first().cloned().unwrap_or_else(|| {
                    meltype_core::dictionary::Dictionary::bundled().convert(&part.text)
                });
                converted.push_str(&value);
            }
        }
        let converted = converted.trim_end().to_owned();
        if converted != text && !converted.is_empty() {
            return run(handle, |s| s.begin_romaji_reconversion(&text, &converted));
        }
        unchanged_romaji = converted == text;
    }
    let supplied = read(reading);
    let result = run(handle, |s| {
        s.reconvert_selection(
            &text,
            supplied.as_deref().or(if unchanged_romaji {
                Some(text.as_str())
            } else {
                None
            }),
        )
    });
    let started = read(result)
        .and_then(|text| serde_json::from_str::<meltype_core::SessionResult>(&text).ok())
        .is_some_and(|value| {
            value.consumed && value.selection_expect.as_deref() == Some(text.as_str())
        });
    if !started {
        return result;
    }
    let current = sessions().lock().ok().and_then(|all| {
        all.get(&(handle as usize))
            .filter(|s| s.is_converting())
            .map(Session::reading)
    });
    if let Some(current) = current {
        let clauses = external_selection_conversion(&current, unchanged_romaji);
        if !clauses.is_empty() {
            if let Some(text) = take(result) {
                if let Ok(previous) = serde_json::from_str::<meltype_core::SessionResult>(&text) {
                    return run(handle, |s| {
                        s.begin_clauses(clauses);
                        let mut result = s.result(previous.consumed);
                        result.selection_expect = previous.selection_expect;
                        result.commits = previous.commits;
                        result
                    });
                }
                return allocate(&text);
            }
            return std::ptr::null_mut();
        }
    }
    result
}
/// # Safety
/// before must be a valid NUL-terminated UTF-8 snapshot of text before the caret.
#[no_mangle]
pub unsafe extern "C" fn meltype_undo_commit(
    handle: *mut c_void,
    before: *const c_char,
) -> *mut c_char {
    let before_pointer = before;
    let Some(before) = read(before) else {
        return std::ptr::null_mut();
    };
    let Ok(_spelling) =
        word_checker::prepare(handle as usize, -1, 0, 0, before_pointer, std::ptr::null())
    else {
        return std::ptr::null_mut();
    };
    run(handle, |s| {
        s.set_context(Some(&before), None);
        s.undo_commit()
    })
}
#[no_mangle]
pub extern "C" fn meltype_reset(handle: *mut c_void) -> *mut c_char {
    run(handle, |s| {
        s.reset();
        s.result(true)
    })
}
#[no_mangle]
pub extern "C" fn meltype_select_candidate(handle: *mut c_void, index: i32) -> *mut c_char {
    let Ok(_spelling) = (unsafe {
        word_checker::prepare(handle as usize, 0, 0, 0, std::ptr::null(), std::ptr::null())
    }) else {
        return std::ptr::null_mut();
    };
    run(handle, |s| s.select(index as usize))
}
#[no_mangle]
pub extern "C" fn meltype_select_prediction(handle: *mut c_void, index: i32) -> *mut c_char {
    run(handle, |s| s.select_prediction(index as usize))
}
fn set(handle: *mut c_void, f: impl FnOnce(&mut Session)) {
    if let Ok(mut all) = sessions().lock() {
        if let Some(s) = all.get_mut(&(handle as usize)) {
            f(s);
        }
    }
}
#[no_mangle]
pub extern "C" fn meltype_set_direct(handle: *mut c_void, enabled: i32) {
    set(handle, |s| s.set_direct(enabled != 0));
}
#[no_mangle]
pub extern "C" fn meltype_set_direct_detection(handle: *mut c_void, enabled: i32) {
    set(handle, |s| s.direct_detection = enabled != 0);
}
#[no_mangle]
pub extern "C" fn meltype_set_code_input(handle: *mut c_void, enabled: i32) {
    set(handle, |s| {
        if s.code_input != (enabled != 0) {
            s.caret_moved(0, false);
        }
        s.code_input = enabled != 0;
    });
}
/// Effective input mode: 1 English/direct, 0 Japanese, -1 stale handle.
#[no_mangle]
pub extern "C" fn meltype_input_mode(handle: *mut c_void) -> i32 {
    sessions()
        .lock()
        .ok()
        .and_then(|all| {
            all.get(&(handle as usize))
                .map(|session| i32::from(session.input_is_direct()))
        })
        .unwrap_or(-1)
}
#[no_mangle]
pub extern "C" fn meltype_caret_moved(handle: *mut c_void, vk: i32, control: i32) {
    set(handle, |session| session.caret_moved(vk, control != 0));
}
/// NULL for ordinary keys. Try composition-role keys before this function.
#[no_mangle]
pub extern "C" fn meltype_mode_key(handle: *mut c_void, vk: i32, modifiers: i32) -> *mut c_char {
    // Avoid a second profile reload/database read for every ordinary key.
    let matches = sessions()
        .lock()
        .ok()
        .and_then(|all| {
            all.get(&(handle as usize))
                .and_then(|session| session.mode_key_target(vk, modifiers))
        })
        .is_some();
    // A newly saved shortcut must work on its first press too. Refresh for
    // modified command keys; ordinary typing keeps the allocation-free probe.
    let command = modifiers & 14 != 0 || vk == 32 && modifiers & 1 != 0;
    if matches || command {
        run_optional(handle, |session| session.mode_key(vk, modifiers))
    } else {
        std::ptr::null_mut()
    }
}
/// Set a native process/bundle identity. Returns 1 for the code profile, 0 for
/// normal input, or -1 for an invalid session. Call on focus, before composition.
/// # Safety
/// identity must point to a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn meltype_set_application(
    handle: *mut c_void,
    identity: *const c_char,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if identity.is_null() {
            return -1;
        }
        let Ok(identity) = CStr::from_ptr(identity).to_str() else {
            return -1;
        };
        let mut all = sessions().lock().unwrap_or_else(|e| e.into_inner());
        let Some(session) = all.get_mut(&(handle as usize)) else {
            return -1;
        };
        session.set_process(identity);
        i32::from(session.code_input)
    }))
    .unwrap_or(-1)
}
#[no_mangle]
pub extern "C" fn meltype_set_private(handle: *mut c_void, enabled: i32) {
    set(handle, |s| {
        if s.private != (enabled != 0) {
            s.reset();
        }
        s.private = enabled != 0;
    });
}
/// Host input attributes. Bit 0: secret/raw input, 1: no learning, 2: terminal.
/// Changing privacy invalidates pending input before any further edit or learn.
#[no_mangle]
pub extern "C" fn meltype_set_input_attributes(handle: *mut c_void, flags: u32) {
    set(handle, |s| {
        let secret = flags & 1 != 0;
        let no_learning = flags & 3 != 0;
        if s.private != secret || s.no_learning != no_learning {
            s.reset();
            s.set_context(Some(""), Some(""));
        }
        s.private = secret;
        s.no_learning = no_learning;
        let code = flags & 4 != 0 || s.settings.text("AppProfile") == "Code";
        if s.code_input != code {
            s.caret_moved(0, false);
        }
        s.code_input = code;
    });
}
// Deletion is permitted only when the host supports expected-text validation.
#[no_mangle]
pub extern "C" fn meltype_set_can_delete(handle: *mut c_void, enabled: i32) {
    set(handle, |s| s.can_delete = enabled != 0);
}
#[no_mangle]
pub extern "C" fn meltype_set_can_move_caret(handle: *mut c_void, enabled: i32) {
    set(handle, |s| s.can_move_caret = enabled != 0);
}
// NULL tells existing frontends to apply their regular mode-switch behavior.
#[no_mangle]
pub extern "C" fn meltype_role_key(handle: *mut c_void, vk: i32) -> *mut c_char {
    let enabled = sessions()
        .lock()
        .ok()
        .and_then(|all| {
            all.get(&(handle as usize)).map(|s| {
                !s.raw().is_empty()
                    && (s.settings.boolean("SeparateKeyRoles")
                        || vk == 0x1d && s.settings.boolean("MuhenkanKatakanaCycle"))
            })
        })
        .unwrap_or(false);
    if enabled {
        run(handle, |s| s.key(vk, 0, 0))
    } else {
        std::ptr::null_mut()
    }
}
#[no_mangle]
pub extern "C" fn meltype_version() -> *mut c_char {
    allocate(concat!(env!("CARGO_PKG_VERSION"), "-rust-preview"))
}
#[no_mangle]
pub extern "C" fn meltype_data_directory() -> *mut c_char {
    let base = meltype_core::storage::Store::default_directory();
    allocate(&base.to_string_lossy())
}
#[no_mangle]
pub extern "C" fn meltype_report_url(_platform: *const c_char) -> *mut c_char {
    allocate("https://github.com/appipinopi/Meltype/issues/new")
}

/// # Safety
/// pointer must be NULL or a live result from this library, freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn meltype_free(pointer: *mut c_void) {
    libc::free(pointer);
}

#[cfg(any(target_os = "android", feature = "android-jni-test"))]
mod android;
#[cfg(any(target_os = "android", feature = "android-jni-test"))]
mod android_editor;
#[cfg(any(target_os = "android", feature = "android-jni-test"))]
mod android_input;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linux_attributes_distinguish_secret_no_learning_and_terminal() {
        let directory = tempfile::tempdir().unwrap();
        let path = CString::new(directory.path().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            let handle = meltype_create_at(path.as_ptr());
            let key = |ch: char| {
                serde_json::from_str::<serde_json::Value>(
                    &take(meltype_handle_key(
                        handle,
                        0,
                        ch as i32,
                        0,
                        std::ptr::null(),
                        std::ptr::null(),
                    ))
                    .unwrap(),
                )
                .unwrap()
            };
            meltype_set_input_attributes(handle, 2);
            assert_eq!(key('k')["view"]["text"], "k");
            assert_eq!(key('a')["view"]["text"], "か");
            {
                let all = sessions().lock().unwrap();
                let session = all.get(&(handle as usize)).unwrap();
                assert!(!session.private && session.no_learning);
            }
            meltype_free(meltype_commit(handle).cast());
            assert!(
                !directory.path().join("state.json").exists(),
                "学習禁止では履歴を保存しない"
            );
            meltype_set_input_attributes(handle, 1);
            assert_eq!(key('a')["consumed"], false);
            meltype_set_input_attributes(handle, 4);
            assert_eq!(key('a')["consumed"], false);
            meltype_set_input_attributes(handle, 0);
            assert_eq!(key('a')["view"]["text"], "あ");
            meltype_set_can_move_caret(handle, 0);
            assert!(
                !sessions()
                    .lock()
                    .unwrap()
                    .get(&(handle as usize))
                    .unwrap()
                    .can_move_caret
            );
            meltype_destroy(handle);
        }
    }
    #[test]
    fn persistent_control_and_prediction() {
        unsafe {
            let directory = tempfile::tempdir().unwrap();
            let path = CString::new(directory.path().to_str().unwrap()).unwrap();
            let handle = meltype_create_at(path.as_ptr());
            assert!(!handle.is_null());
            let request =
                CString::new(r#"{"op":"dictionary_set","reading":"めるたいぷ","word":"Meltype"}"#)
                    .unwrap();
            let result: serde_json::Value =
                serde_json::from_str(&take(meltype_control(handle, request.as_ptr())).unwrap())
                    .unwrap();
            assert_eq!(result["saved"], true);
            for c in "meru".chars() {
                let result =
                    meltype_handle_key(handle, 0, c as i32, 0, std::ptr::null(), std::ptr::null());
                meltype_free(result.cast());
            }
            let result: serde_json::Value =
                serde_json::from_str(&take(meltype_select_prediction(handle, 0)).unwrap()).unwrap();
            assert_eq!(result["commits"][0]["text"], "Meltype");
            meltype_destroy(handle);
            let reopened = meltype_create_at(path.as_ptr());
            assert!(!reopened.is_null());
            let request = CString::new(r#"{"op":"dictionary_list"}"#).unwrap();
            let result: serde_json::Value =
                serde_json::from_str(&take(meltype_control(reopened, request.as_ptr())).unwrap())
                    .unwrap();
            assert_eq!(result["めるたいぷ"][0], "Meltype");
            meltype_destroy(reopened);
        }
    }
    #[test]
    fn lifecycle_and_unicode() {
        unsafe {
            let handle = meltype_create();
            assert!(!handle.is_null());
            // Upstream composition consumes nn as one ん unit.
            for c in "konnnichiha".chars() {
                let p =
                    meltype_handle_key(handle, 0, c as i32, 0, std::ptr::null(), std::ptr::null());
                assert!(!p.is_null());
                meltype_free(p.cast());
            }
            let p = meltype_commit(handle);
            let value: serde_json::Value = serde_json::from_str(&read(p).unwrap()).unwrap();
            assert_eq!(value["commits"][0]["text"], "こんにちは");
            meltype_free(p.cast());
            meltype_destroy(handle);
            meltype_destroy(handle);
            assert!(meltype_commit(handle).is_null());
            assert!(meltype_commit(std::ptr::null_mut()).is_null());
        }
    }
    #[test]
    fn application_identity_controls_native_input_and_rejects_stale_handles() {
        let directory = tempfile::tempdir().unwrap();
        let store = meltype_core::storage::Store::new(directory.path());
        store
            .change_settings(|settings| {
                settings.set(
                    "AppRules",
                    serde_json::json!([
                        {"Process":"native-editor","Enabled":true,"Profile":"Code"},
                        {"Process":"native-chat","Enabled":true,"Profile":"General"}
                    ]),
                )
            })
            .unwrap();
        unsafe {
            let path = CString::new(directory.path().to_string_lossy().as_bytes()).unwrap();
            let handle = meltype_create_at(path.as_ptr());
            assert_eq!(
                meltype_set_application(handle, c"native-editor".as_ptr()),
                1
            );
            let output = meltype_handle_key(handle, 65, 97, 0, std::ptr::null(), std::ptr::null());
            let value: serde_json::Value = serde_json::from_str(&read(output).unwrap()).unwrap();
            assert_eq!(value["consumed"], false);
            meltype_free(output.cast());
            assert_eq!(meltype_set_application(handle, c"native-chat".as_ptr()), 0);
            let output = meltype_handle_key(handle, 65, 97, 0, std::ptr::null(), std::ptr::null());
            let value: serde_json::Value = serde_json::from_str(&read(output).unwrap()).unwrap();
            assert_eq!(value["view"]["text"], "あ");
            meltype_free(output.cast());
            meltype_destroy(handle);
            assert_eq!(
                meltype_set_application(handle, c"native-editor".as_ptr()),
                -1
            );
            assert_eq!(
                meltype_set_application(std::ptr::null_mut(), std::ptr::null()),
                -1
            );
        }
    }
}
