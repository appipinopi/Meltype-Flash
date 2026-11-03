// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Product dialogs. Reports are local drafts until the user opens a browser.
use eframe::egui;
use meltype_core::{settings::Settings, storage::Store};
use std::{
    collections::VecDeque,
    fs::File,
    io::{Read, Seek, SeekFrom},
    time::{Duration, Instant},
};

const LOG_LIMIT: u64 = 256 * 1024;
const RECENT_LIMIT: usize = 64;
const KEYS: [&str; 5] = [
    "Space (変換)",
    "Enter (確定)",
    "打っている途中 (まだ確定していない)",
    "句読点・記号",
    "そのほか",
];
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Correction {
    pub typed: String,
    pub shown: String,
    pub expected: String,
    pub key: usize,
    pub before: String,
}
impl Correction {
    fn body(&self, environment: &str) -> String {
        format!("## 変換・判定の間違い\n\n打ったもの: {}\n出たもの: {}\n期待した結果: {}\n最後に押したキー: {}\n前後の文: {}\n\n## 実行環境\n{}\n", self.typed, self.shown, self.expected, KEYS[self.key.min(KEYS.len()-1)], self.before, environment)
    }
}
#[derive(Default)]
pub struct Dialogs {
    pub log: bool,
    pub report: bool,
    pub corrections: bool,
    log_text: String,
    decisions_only: bool,
    refreshed: Option<Instant>,
    report_log: String,
    status: String,
    recent: VecDeque<Correction>,
    selected: Option<usize>,
    draft: Correction,
    #[cfg(windows)]
    ime_recent: Vec<meltype_core::session::report_corrections::Entry>,
}
fn environment(settings: &Settings) -> String {
    format!(
        "Meltype-FLASH {}\nOS: {} ({})\n動作モード: {}\n自動判定の強さ: {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        settings.text("Mode"),
        settings.text("DetectionLevel")
    )
}
fn encode(text: &str) -> String {
    let mut output = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            output.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}
fn report_url(template: &str, fields: &[(&str, &str)]) -> String {
    let mut url = format!(
        "{}?template={}",
        crate::lifecycle::REPORT.trim_end_matches("/choose"),
        encode(template)
    );
    for (name, value) in fields {
        url.push('&');
        url.push_str(&encode(name));
        url.push('=');
        url.push_str(&encode(value));
    }
    url
}
fn correction_url(draft: &Correction, environment: &str) -> String {
    report_url(
        "2-misdetection.yml",
        &[
            ("version", env!("CARGO_PKG_VERSION")),
            ("environment", environment),
            ("typed", &draft.typed),
            ("actual", &draft.shown),
            ("expected", &draft.expected),
            ("key", KEYS[draft.key.min(KEYS.len() - 1)]),
            ("context", &draft.before),
        ],
    )
}
/// Read only this store's opt-in diagnostic file, with bounded memory. No file is created.
fn recent_log(store: &Store) -> std::io::Result<String> {
    let mut file = match File::open(store.directory.join("tip.log")) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(e) => return Err(e),
    };
    let length = file.metadata()?.len();
    let offset = length.saturating_sub(LOG_LIMIT);
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    file.take(LOG_LIMIT).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    // A partial first line may contain half of a UTF-8 scalar; omit that line.
    let text = if offset > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    Ok(text.to_owned())
}
fn decision_line(line: &str) -> bool {
    [
        "判定", "IME", "無効", "失敗", "WARN", "ERROR", "Decision", "Error", "Warn",
    ]
    .iter()
    .any(|word| line.contains(word))
}
/// Real decorated OS windows; use the same contents embedded for capture tests / web fallback.
pub fn window(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    mut body: impl FnMut(&mut egui::Ui) -> bool,
) -> bool {
    let mut closed = false;
    let mut draw = |ctx: &egui::Context, embedded: bool| {
        if embedded {
            let mut open = true;
            egui::Window::new(title)
                .id(egui::Id::new(id))
                .open(&mut open)
                .default_size(size)
                .min_size([size[0].min(480.), size[1].min(420.)])
                .collapsible(false)
                .show(ctx, |ui| closed |= body(ui));
            closed |= !open;
        } else {
            closed |=
                ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape));
            egui::CentralPanel::default().show(ctx, |ui| closed |= body(ui));
        }
    };
    #[cfg(feature = "visual-test")]
    if std::env::var_os("MELTYPE_UI_CAPTURE").is_some()
        && std::env::var_os("MELTYPE_UI_CAPTURE_NATIVE_DIALOG").is_none()
    {
        draw(ctx, true);
        return !closed;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of(id),
        egui::ViewportBuilder::default()
            .with_title(title)
            .with_icon(crate::branding::icon())
            .with_inner_size(size)
            .with_min_inner_size([size[0].min(480.), size[1].min(420.)]),
        |ctx, class| draw(ctx, class == egui::ViewportClass::Embedded),
    );
    !closed
}
impl Dialogs {
    pub fn open_report(&mut self, store: &Store) {
        self.report = true;
        self.status.clear();
        match recent_log(store) {
            Ok(text) => self.report_log = text,
            Err(error) => {
                self.report_log.clear();
                self.status = format!("ログを読めません: {error}");
            }
        }
    }
    pub fn record(&mut self, typed: &str, shown: &str, expected: &str, key: usize) {
        if typed.is_empty() || shown == expected || expected.is_empty() {
            return;
        }
        let correction = Correction {
            typed: typed.to_owned(),
            shown: shown.to_owned(),
            expected: expected.to_owned(),
            key,
            before: String::new(),
        };
        if self.recent.back() == Some(&correction) {
            return;
        }
        if self.recent.len() == RECENT_LIMIT {
            self.recent.pop_front();
        }
        self.recent.push_back(correction.clone());
        self.selected = Some(self.recent.len() - 1);
        self.draft = correction;
    }
    pub fn show(&mut self, ctx: &egui::Context, store: &Store, settings: &Settings) {
        if self.log {
            ctx.request_repaint_after(Duration::from_millis(500));
            if self
                .refreshed
                .is_none_or(|time| time.elapsed() >= Duration::from_millis(500))
            {
                self.log_text =
                    recent_log(store).unwrap_or_else(|error| format!("ログを読めません: {error}"));
                self.refreshed = Some(Instant::now());
            }
            self.log = window(
                ctx,
                "flash-log",
                "Meltype-FLASH ログ / 判定理由",
                [900., 480.],
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("前面アプリの IME: 不明");
                        ui.checkbox(&mut self.decisions_only, "判定と IME 切替だけ表示");
                        if ui.button("コピー").clicked() {
                            let text = if self.decisions_only {
                                self.log_text
                                    .lines()
                                    .filter(|line| decision_line(line))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            } else {
                                self.log_text.clone()
                            };
                            ui.ctx().copy_text(text);
                        }
                    });
                    let text = if self.decisions_only {
                        self.log_text
                            .lines()
                            .filter(|line| decision_line(line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        self.log_text.clone()
                    };
                    if text.is_empty() {
                        ui.weak("表示するログはありません。ログの記録を有効にした場合に、このFlash保存先の tip.log を表示します。");
                    }
                    let mut readonly = text.as_str();
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut readonly)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(25),
                            );
                        });
                    false
                },
            );
        }
        if self.report {
            self.report = window(
                ctx,
                "flash-report",
                "Meltype-FLASH 不具合の報告・提案",
                [700., 580.],
                |ui| {
                    ui.label("「フォームで報告」を押すと、報告の画面がブラウザーで開きます。下の実行環境が入ります。");
                    ui.label("ログは中身を確かめ、見られて困る部分を消してから「ログをコピー」で貼り付けてください。");
                    ui.add_space(8.);
                    ui.group(|ui| {
                        ui.label("実行環境 (報告画面に入ります)");
                        let env = environment(settings);
                        let mut readonly = env.as_str();
                        ui.add(
                            egui::TextEdit::multiline(&mut readonly)
                                .desired_width(f32::INFINITY)
                                .desired_rows(4),
                        );
                    });
                    ui.group(|ui| {
                        ui.label("最近のログ (編集できます)");
                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .max_height((ui.available_height() - 100.).max(100.))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.report_log)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(12),
                                );
                            });
                    });
                    ui.weak(&self.status);
                    let mut close = false;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.button("閉じる").clicked();
                        if ui.button("フォームで報告 (GitHub)").clicked() {
                            // The log is deliberately excluded even after local editing.
                            ui.ctx().open_url(egui::OpenUrl::new_tab(report_url(
                                "1-bug.yml",
                                &[
                                    ("environment", &environment(settings)),
                                    ("version", env!("CARGO_PKG_VERSION")),
                                ],
                            )));
                        }
                        if ui
                            .add_enabled(
                                !self.report_log.is_empty(),
                                egui::Button::new("ログをコピー"),
                            )
                            .clicked()
                        {
                            ui.ctx().copy_text(self.report_log.clone());
                            self.status = "コピーしました".into();
                        }
                    });
                    close
                },
            );
        }
        if self.corrections {
            #[cfg(windows)]
            {
                ctx.request_repaint_after(Duration::from_millis(500));
                let entries = meltype_core::session::report_corrections::recent(&store.directory);
                for entry in &entries {
                    if self.ime_recent.contains(entry)
                        || self.recent.iter().any(|old| {
                            old.typed == entry.typed
                                && old.shown == entry.shown
                                && old.expected == entry.expected
                                && old.before == entry.before
                        })
                    {
                        continue;
                    }
                    if self.recent.len() == RECENT_LIMIT {
                        self.recent.pop_front();
                        self.selected = self.selected.and_then(|i| i.checked_sub(1));
                    }
                    self.recent.push_back(Correction {
                        typed: entry.typed.clone(),
                        shown: entry.shown.clone(),
                        expected: entry.expected.clone(),
                        before: entry.before.clone(),
                        key: 1,
                    });
                }
                self.ime_recent = entries;
            }
            self.corrections = window(
                ctx,
                "flash-corrections",
                "Meltype-FLASH 直した誤判定を報告",
                [640., 480.],
                |ui| {
                    ui.label("直した判定を選んで中身を確かめ、個人的な文章は似た例に書き換えてから「GitHub で報告」を押してください。");
                    ui.weak("IMEで英字・かなの判定を直して確定した履歴と、この画面の入力テストを表示します。秘密欄・学習禁止欄は記録しません。履歴はメモリだけに保持し、Meltypeの全プロセス終了で消えます。");
                    if ui.button("履歴を消す").clicked() {
                        self.recent.clear();
                        self.selected = None;
                        self.draft = Correction::default();
                        #[cfg(windows)]
                        {
                            meltype_core::session::report_corrections::clear(&store.directory);
                            self.ime_recent.clear();
                        }
                    }
                    ui.add_space(6.);
                    let list_height = (ui.available_height() - 220.).max(80.);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    egui::ScrollArea::vertical().id_salt("correction-list").auto_shrink([false, false]).max_height(list_height).min_scrolled_height(list_height).show(ui, |ui| {
                        if self.recent.is_empty() { ui.weak("まだ直した判定がありません。下の欄に報告内容を入力することもできます。"); }
                        for (index, entry) in self.recent.iter().enumerate() {
                            if ui.selectable_label(self.selected == Some(index), format!("{}  : {} → {}", entry.typed, entry.shown, entry.expected)).clicked() {
                                self.selected = Some(index); self.draft = entry.clone();
                            }
                        }
                    });
                });
                    egui::Grid::new("correction-fields")
                        .num_columns(2)
                        .spacing([10., 7.])
                        .show(ui, |ui| {
                            for (label, text) in [
                                ("打ったもの (キーをそのまま)", &mut self.draft.typed),
                                ("出たもの", &mut self.draft.shown),
                                ("期待した結果", &mut self.draft.expected),
                            ] {
                                ui.label(label);
                                ui.add(
                                    egui::TextEdit::singleline(text)
                                        .desired_width(ui.available_width()),
                                );
                                ui.end_row();
                            }
                            ui.label("最後に押したキー");
                            egui::ComboBox::from_id_salt("correction-key")
                                .selected_text(KEYS[self.draft.key.min(4)])
                                .show_ui(ui, |ui| {
                                    for (index, key) in KEYS.iter().enumerate() {
                                        ui.selectable_value(&mut self.draft.key, index, *key);
                                    }
                                });
                            ui.end_row();
                            ui.label("前後の文 (分かれば)");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft.before)
                                    .desired_width(ui.available_width()),
                            );
                            ui.end_row();
                        });
                    ui.add_space(10.);
                    let ready = !self.draft.typed.trim().is_empty()
                        && !self.draft.expected.trim().is_empty();
                    let mut close = false;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.button("閉じる").clicked();
                        if ui
                            .add_enabled(ready, egui::Button::new("GitHub で報告"))
                            .clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(correction_url(
                                &self.draft,
                                &environment(settings),
                            )));
                        }
                        if ui.add_enabled(ready, egui::Button::new("コピー")).clicked() {
                            ui.ctx().copy_text(self.draft.body(&environment(settings)));
                        }
                    });
                    close
                },
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_reads_are_bounded_and_do_not_initialize_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("untouched"));
        assert!(recent_log(&store).unwrap().is_empty());
        assert!(!store.directory.exists());
        let store = Store::new(dir.path());
        let text = format!(
            "SECRET_BEFORE_LIMIT\n{}\n判定: 日本語\n",
            "あ".repeat(LOG_LIMIT as usize)
        );
        std::fs::write(store.directory.join("tip.log"), text).unwrap();
        let result = recent_log(&store).unwrap();
        assert!(!result.contains("SECRET_BEFORE_LIMIT"));
        assert!(result.ends_with("判定: 日本語\n"));
        assert!(result.len() <= LOG_LIMIT as usize);
    }
    #[test]
    fn correction_report_encodes_only_the_reviewed_draft() {
        let mut dialogs = Dialogs::default();
        dialogs.record("hashi", "はし", "hashi", 2);
        dialogs.record("hashi", "はし", "hashi", 2);
        assert_eq!(dialogs.recent.len(), 1);
        dialogs.draft.before = "似た例 & #?\n😀".into();
        let body = dialogs.draft.body("Windows");
        let url = correction_url(&dialogs.draft, "Windows");
        assert!(body.contains("期待した結果: hashi"));
        assert!(url.contains("&typed=hashi&actual="));
        assert!(url.contains("&expected=hashi&key="));
        assert!(!url.contains("&body="));
        assert!(url.starts_with(
            "https://github.com/appipinopi/Meltype-Flash/issues/new?template=2-misdetection.yml&"
        ));
        assert!(url.contains("%26%20%23%3F%0A"));
        assert!(!url.contains('\n'));
        assert!(!url.contains('😀'));
        for index in 0..100 {
            dialogs.record(&format!("example{index}"), "日本語", "英語", 4);
        }
        assert_eq!(dialogs.recent.len(), RECENT_LIMIT);
    }
}
