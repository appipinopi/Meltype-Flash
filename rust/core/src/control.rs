// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Management operations shared by CLI and platform settings clients.
use crate::{
    storage::{atomic_write, Store},
    user_dictionary,
};
use serde_json::{json, Value};
use std::{io, path::Path};
fn field<'a>(r: &'a Value, n: &str) -> io::Result<&'a str> {
    r[n].as_str()
        .ok_or_else(|| io::Error::other(format!("missing {n}")))
}
pub(crate) fn snippet_name_valid(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_graphic() && c != ';')
}
pub(crate) fn snippet_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace(['\r', '\u{000c}', '\u{0085}', '\u{2028}', '\u{2029}'], "\n")
}
pub fn execute(store: &Store, r: &Value) -> io::Result<Value> {
    match field(r, "op")? {
        "detection_evaluate" => {
            let raw = field(r, "text")?;
            if raw.len() > 256 {
                return Err(io::Error::other("detection text exceeds 256 bytes"));
            }
            let keys: Vec<i32> = if let Some(keys) = r["keys"].as_array() {
                if keys.len() > 256 {
                    return Err(io::Error::other("too many detection keys"));
                }
                keys.iter()
                    .map(|key| {
                        key.as_i64()
                            .and_then(|n| i32::try_from(n).ok())
                            .ok_or_else(|| io::Error::other("invalid key"))
                    })
                    .collect::<io::Result<_>>()?
            } else {
                raw.chars().map(|c| c.to_ascii_uppercase() as i32).collect()
            };
            let state = store.load()?;
            Ok(serde_json::to_value(
                crate::detection::evaluate_with_model(
                    raw,
                    &keys,
                    r["final"].as_bool().unwrap_or(true),
                    &store.settings()?,
                    Some(&state.prefix_model),
                ),
            )?)
        }
        "settings_get" => Ok(serde_json::to_value(store.settings()?)?),
        "settings_schema" => Ok(crate::settings_schema::schema()),
        "settings_schema_export" => {
            store.change_settings(|s| {
                atomic_write(
                    &store.directory.join("config.schema.json"),
                    &serde_json::to_vec_pretty(&crate::settings_schema::schema())
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                s.set("$schema", json!("./config.schema.json"))
            })?;
            Ok(json!({"saved":true,"path":store.directory.join("config.schema.json")}))
        }
        "settings_merge" => {
            let parse = |key| -> io::Result<crate::settings::Settings> {
                crate::settings::Settings::parse(&serde_json::to_string(
                    r.get(key)
                        .ok_or_else(|| io::Error::other(format!("missing {key}")))?,
                )?)
                .map_err(io::Error::other)
            };
            let opened = parse("opened")?;
            let edited = parse("edited")?;
            store.change_settings(|current| {
                *current = crate::settings::Settings::merge_edits(&opened, &edited, current);
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "settings_set" => {
            let name = field(r, "name")?;
            let value = r
                .get("value")
                .ok_or_else(|| io::Error::other("missing value"))?
                .clone();
            store.change_settings(|s| s.set(name, value))?;
            Ok(json!({"saved":true}))
        }
        "profile_add" | "profile_switch" | "profile_delete" => {
            let name = field(r, "name")?;
            store.change_settings(|s| match field(r, "op").unwrap() {
                "profile_add" => s.add_profile(name),
                "profile_switch" => s.switch_profile(name),
                _ => s.remove_profile(name),
            })?;
            Ok(json!({"saved":true}))
        }
        "dictionary_list" => Ok(serde_json::to_value(
            store.load()?.preferences.user_dictionary,
        )?),
        "dictionary_set" => {
            let reading = user_dictionary::hiragana(field(r, "reading")?.trim());
            let word = field(r, "word")?.trim();
            if !user_dictionary::valid(&reading, word) {
                return Err(io::Error::other("invalid reading or word"));
            }
            store.update(|s| {
                let entries = s.preferences.user_dictionary.entry(reading).or_default();
                if !entries.iter().any(|w| w == word) {
                    entries.insert(0, word.into());
                }
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "dictionary_remove" => {
            let reading = field(r, "reading")?;
            let word = r["word"].as_str();
            store.update(|s| {
                if let Some(word) = word {
                    if let Some(words) = s.preferences.user_dictionary.get_mut(reading) {
                        words.retain(|w| w != word);
                    }
                } else {
                    s.preferences.user_dictionary.remove(reading);
                }
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "dictionary_import" => {
            let import = user_dictionary::parse(&std::fs::read(field(r, "path")?)?)?;
            let count = store.update(|s| {
                let mut count = 0;
                for (reading, word) in import.words {
                    let entries = s.preferences.user_dictionary.entry(reading).or_default();
                    if !entries.contains(&word) {
                        entries.insert(0, word);
                        count += 1;
                    }
                }
                Ok(count)
            })?;
            Ok(json!({"imported":count,"skipped":import.skipped,"encoding":import.encoding}))
        }
        "dictionary_export" => {
            let words = store
                .load()?
                .preferences
                .user_dictionary
                .into_iter()
                .flat_map(|(reading, words)| {
                    words.into_iter().map(move |word| (reading.clone(), word))
                });
            atomic_write(
                Path::new(field(r, "path")?),
                &user_dictionary::export(words),
            )?;
            Ok(json!({"saved":true}))
        }
        "snippets_list" => Ok(serde_json::to_value(store.load()?.snippets)?),
        "snippet_set" => {
            let name = field(r, "name")?;
            let text = field(r, "text")?;
            if !snippet_name_valid(name) || text.is_empty() {
                return Err(io::Error::other("invalid snippet"));
            }
            store.update(|s| {
                s.snippets.insert(name.into(), snippet_text(text));
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "snippet_remove" => {
            store.update(|s| {
                s.snippets.remove(field(r, "name")?);
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "snippets_import" => {
            let (text, _) = user_dictionary::decode(&std::fs::read(field(r, "path")?)?)?;
            // Validate the entire document before acquiring the write lock.
            // Upstream skips invalid names, empty values and null values.
            let parsed: Option<std::collections::BTreeMap<String, Option<String>>> =
                serde_json::from_str(&text)?;
            let entries: Vec<_> = parsed
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(name, text)| {
                    text.filter(|text| !text.is_empty())
                        .filter(|_| snippet_name_valid(&name))
                        .map(|text| (name, snippet_text(&text)))
                })
                .collect();
            let count = entries.len();
            if count > 0 {
                store.update(|s| {
                    s.snippets.extend(entries);
                    Ok(())
                })?;
            }
            Ok(json!({"imported":count}))
        }
        "snippets_export" => {
            atomic_write(
                Path::new(field(r, "path")?),
                &serde_json::to_vec_pretty(&store.load()?.snippets)?,
            )?;
            Ok(json!({"saved":true}))
        }
        "learning_list" => {
            let s = store.load()?;
            let languages: serde_json::Map<String, Value> = s
                .language_counts
                .iter()
                .map(|(word, entry)| {
                    let mut value = serde_json::to_value(entry).unwrap();
                    value["active"] = json!(entry.active(word));
                    (word.clone(), value)
                })
                .collect();
            Ok(
                json!({"languages":languages,"conversions":s.preferences.selected,"translations":s.translations,"phrases":s.phrases,"prefixes":s.prefix_model.prefixes}),
            )
        }
        "prefix_learn" => {
            let letters = field(r, "letters")?;
            let outcome = serde_json::from_value(r["outcome"].clone())?;
            let settings = store.settings()?;
            if !settings.boolean("LearningEnabled") {
                return Ok(json!({"saved":false}));
            }
            let saved = store.update(|state| {
                Ok(state.prefix_model.learn(
                    letters,
                    outcome,
                    r["decidedEnglish"].as_bool().unwrap_or(false),
                    chrono::Utc::now(),
                ))
            })?;
            Ok(json!({"saved":saved}))
        }
        "learning_clear" => {
            store.update(|s| {
                s.clear_learning();
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "learning_forget_since" => {
            let since = r["since"]
                .as_i64()
                .ok_or_else(|| io::Error::other("since must be a UTC Unix timestamp"))?;
            let count = store.update(|s| Ok(s.forget_since(since)))?;
            Ok(json!({"removed":count}))
        }
        "learning_remove" => {
            let key = field(r, "key")?;
            store.update(|s| {
                s.remove_learned(key);
                Ok(())
            })?;
            Ok(json!({"saved":true}))
        }
        "backup_create" => {
            atomic_write(Path::new(field(r, "path")?), &store.backup()?)?;
            Ok(json!({"saved":true}))
        }
        "backup_inspect" => Store::inspect_backup(&std::fs::read(field(r, "path")?)?),
        "backup_preview" => store.preview_backup(&std::fs::read(field(r, "path")?)?),
        "backup_restore" => {
            let count = store.restore(&std::fs::read(field(r, "path")?)?)?;
            Ok(json!({"restored":count}))
        }
        "diagnostics" => Ok(
            json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"dataDirectory":store.directory,"settingsReadable":store.settings().is_ok(),"stateReadable":store.load().is_ok()}),
        ),
        _ => Err(io::Error::other("unknown management operation")),
    }
}
