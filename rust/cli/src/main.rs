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

/// 端末に未確定文字と候補を表示し、確定した文字列だけを標準出力に渡す。
fn tty(mut session: Session) -> io::Result<()> {
    use crossterm::{
        cursor::MoveToColumn,
        event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
        terminal::{self, Clear, ClearType},
    };
    use std::io::{IsTerminal, Write};

    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(io::Error::other("--tty には対話端末が必要です"));
    }
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
        }
    }
    eprintln!("Meltype TTY  Space: 変換  ↑↓: 候補  Enter: 確定  Esc: 取消  Ctrl+C: 終了");
    terminal::enable_raw_mode()?;
    let _restore = Restore;
    let mut screen = io::stderr();
    let mut output = io::stdout();
    let mut result = session.result(false);
    loop {
        execute!(screen, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        for edit in result.commits.drain(..) {
            writeln!(output, "{}", edit.text)?;
            output.flush()?;
            execute!(screen, MoveToColumn(0))?;
        }
        let mut line = String::from("> ");
        if let Some(view) = &result.view {
            line.push_str(&view.text);
            if view.converting {
                line.push_str("   ");
                for (i, candidate) in view
                    .candidates
                    .iter()
                    .enumerate()
                    .skip(view.selected_index)
                    .take(4)
                {
                    line.push_str(&format!(" {}:{}", i + 1, candidate));
                }
            }
        }
        let width = usize::from(terminal::size()?.0.saturating_sub(1));
        // 全角の文字を含んでも表示を次の行へ折り返さない。
        write!(
            screen,
            "{}",
            line.chars()
                .filter(|c| !c.is_control())
                .take(width / 2)
                .collect::<String>()
        )?;
        screen.flush()?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(key.code, KeyCode::Char('c' | 'd')) {
                break;
            }
            continue;
        }
        let (vk, ch) = match key.code {
            KeyCode::Char(c) => (c.to_ascii_uppercase() as i32, c as i32),
            KeyCode::Enter => (13, 0),
            KeyCode::Esc => (27, 0),
            KeyCode::Backspace => (8, 0),
            KeyCode::Tab => (9, 0),
            KeyCode::Left => (37, 0),
            KeyCode::Up => (38, 0),
            KeyCode::Right => (39, 0),
            KeyCode::Down => (40, 0),
            _ => continue,
        };
        result = session.key(
            vk,
            ch,
            i32::from(key.modifiers.contains(KeyModifiers::SHIFT)),
        );
        if !result.consumed && ch > 0 {
            if let Some(c) = char::from_u32(ch as u32).filter(|c| !c.is_control()) {
                write!(output, "{c}")?;
                output.flush()?;
            }
        }
    }
    execute!(screen, MoveToColumn(0), Clear(ClearType::CurrentLine))
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
        println!(
            "Meltype-FLASH {}",
            meltype_core::version::display(env!("CARGO_PKG_VERSION"))
        );
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
    if args == ["--tty"] {
        tty(Session::open(store)?)?;
        return Ok(());
    }
    let fast_ipc = args.contains(&"--fast-ipc".to_string());
    if !args.is_empty() && !fast_ipc {
        return Err(
            "usage: meltype-rs [--tty | --version | --fast-ipc | --convert TEXT]; no arguments: JSON lines on stdin"
                .into(),
        );
    }
    let mut session = Session::open(store)?;
    let stdin = io::stdin();
    let stdout = io::stdout();

    if fast_ipc {
        use std::io::{Read, Write};
        let mut reader = stdin.lock();
        let mut writer = stdout.lock();
        let mut len_buf = [0u8; 4];
        let mut in_buf = vec![0u8; 64 * 1024];
        let mut out_buf = Vec::with_capacity(64 * 1024);

        while reader.read_exact(&mut len_buf).is_ok() {
            let req_len = u32::from_le_bytes(len_buf) as usize;
            if in_buf.len() < req_len {
                in_buf.resize(req_len, 0);
            }
            reader.read_exact(&mut in_buf[..req_len])?;

            let t_deser_start = std::time::Instant::now();
            let req_res = serde_json::from_slice::<Request>(&in_buf[..req_len]);
            let t_deser = t_deser_start.elapsed();

            let t_engine_start = std::time::Instant::now();
            let (result, err_val) = match req_res {
                Ok(request) => {
                    let res = match request {
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
                            let r = session.commit();
                            session.direct = enabled;
                            r
                        }
                    };
                    (Some(res), None)
                }
                Err(e) => (None, Some(serde_json::json!({"error": e.to_string()}))),
            };
            let t_engine = t_engine_start.elapsed();

            out_buf.clear();
            let t_ser_start = std::time::Instant::now();
            if let Some(r) = &result {
                serde_json::to_writer(&mut out_buf, r)?;
            } else if let Some(e) = &err_val {
                serde_json::to_writer(&mut out_buf, e)?;
            }
            let t_ser = t_ser_start.elapsed();

            let t_send_start = std::time::Instant::now();
            writer.write_all(&(out_buf.len() as u32).to_le_bytes())?;
            writer.write_all(&out_buf)?;
            writer.flush()?;
            let t_send = t_send_start.elapsed();

            if let Ok(v) = std::env::var("MELTYPE_BENCH_TIMING") {
                if v == "1" {
                    eprintln!(
                        "SERVER_TIMING: deser_ns={} engine_ns={} ser_ns={} send_ns={}",
                        t_deser.as_nanos(),
                        t_engine.as_nanos(),
                        t_ser.as_nanos(),
                        t_send.as_nanos()
                    );
                }
            }
        }
        return Ok(());
    }

    let mut writer = io::BufWriter::with_capacity(64 * 1024, stdout.lock());
    let mut line = String::with_capacity(512);
    let mut reader = stdin.lock();

    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let t0 = std::time::Instant::now();
            match serde_json::from_str::<Request>(trimmed) {
                Ok(request) => {
                    let t_parse = t0.elapsed();
                    let t_req_start = std::time::Instant::now();
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
                    let t_exec = t_req_start.elapsed();
                    let t_ser_start = std::time::Instant::now();
                    serde_json::to_writer(&mut writer, &result)?;
                    use std::io::Write;
                    writer.write_all(b"\n")?;
                    let t_ser = t_ser_start.elapsed();
                    let t_flush_start = std::time::Instant::now();
                    writer.flush()?;
                    let t_flush = t_flush_start.elapsed();
                    if let Ok(v) = std::env::var("MELTYPE_BENCH_TIMING") {
                        if v == "1" {
                            eprintln!(
                                "TIMING: parse={:?} exec={:?} ser={:?} flush={:?} total={:?}",
                                t_parse,
                                t_exec,
                                t_ser,
                                t_flush,
                                t0.elapsed()
                            );
                        }
                    }
                }
                Err(error) => {
                    let err = serde_json::json!({"error": error.to_string()});
                    serde_json::to_writer(&mut writer, &err)?;
                    use std::io::Write;
                    writer.write_all(b"\n")?;
                }
            }
            use std::io::Write;
            writer.flush()?;
        }
        line.clear();
    }
    Ok(())
}
