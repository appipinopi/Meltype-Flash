// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
use meltype_core::Session;
use serde::Deserialize;
use std::io::{self, BufRead};

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    Context {
        before: Option<String>,
        after: Option<String>,
    },
    SelectPrediction {
        index: usize,
    },
    Key {
        vk: i32,
        #[serde(default)]
        ch: i32,
        #[serde(default)]
        modifiers: i32,
    },
    Commit,
    ReconvertSelection {
        text: String,
        reading: Option<String>,
    },
    UndoCommit {
        before: String,
    },
    Reset,
    Select {
        index: usize,
    },
    Direct {
        enabled: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let store =
        meltype_core::storage::Store::new(meltype_core::storage::Store::default_directory());
    if args.first().is_some_and(|a| a == "--control") {
        let request = serde_json::from_str(args.get(1).ok_or("--control requires JSON")?)?;
        println!("{}", meltype_core::control::execute(&store, &request)?);
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--version") {
        println!("Meltype-FLASH {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--convert") {
        let raw = args.get(1).ok_or("--convert requires text")?;
        println!(
            "{}",
            meltype_core::dictionary::Dictionary::bundled().mixed(raw, true)
        );
        return Ok(());
    }
    if !args.is_empty() {
        return Err(
            "usage: meltype-rs [--version | --convert TEXT]; no arguments: JSON lines on stdin"
                .into(),
        );
    }
    let mut session = Session::open(store)?;
    for line in io::stdin().lock().lines() {
        let line = line?;
        let result = match serde_json::from_str::<Request>(&line) {
            Ok(request) => {
                let result = match request {
                    Request::Context { before, after } => {
                        session.set_context(before.as_deref(), after.as_deref());
                        session.result(false)
                    }
                    Request::SelectPrediction { index } => session.select_prediction(index),
                    Request::Key { vk, ch, modifiers } => session.key(vk, ch, modifiers),
                    Request::Commit => session.commit(),
                    Request::ReconvertSelection { text, reading } => {
                        session.reconvert_selection(&text, reading.as_deref())
                    }
                    Request::UndoCommit { before } => {
                        session.can_delete = true;
                        session.set_context(Some(&before), None);
                        session.undo_commit()
                    }
                    Request::Reset => {
                        session.reset();
                        session.result(true)
                    }
                    Request::Select { index } => session.select(index),
                    Request::Direct { enabled } => {
                        let result = session.commit();
                        session.direct = enabled;
                        result
                    }
                };
                serde_json::to_value(result)?
            }
            Err(error) => serde_json::json!({"error": error.to_string()}),
        };
        println!("{}", result);
    }
    Ok(())
}
