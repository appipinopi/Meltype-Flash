// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 同梱したリリースノート用の小さな Markdown 表示。
//!
//! 見出し・段落・強調・インラインコード・リンク・箇条書き・引用・コードブロック・
//! 表・区切り線を egui のウィジェットへ描く。画像は読み込まず、代替テキストをリンクとして出す。
use eframe::egui;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Default)]
struct InlineStyle {
    strong: bool,
    emphasis: bool,
    strike: bool,
}

enum Inline {
    Text(String, InlineStyle),
    Code(String),
    Link { text: String, url: String },
    SoftBreak,
    HardBreak,
}

#[derive(Default)]
struct ListLevel {
    ordered: bool,
    next: u64,
}

#[derive(Default)]
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
    in_head: bool,
}

#[derive(Default)]
struct Renderer {
    inline: Vec<Inline>,
    style: InlineStyle,
    link: Option<(String, String)>,
    lists: Vec<ListLevel>,
    item_prefix: Option<String>,
    quote: usize,
    code: Option<String>,
    table: Option<Table>,
    heading: Option<u8>,
    tables: u64,
}

/// Markdown を `ui` へ描く。
pub fn show(ui: &mut egui::Ui, markdown: &str) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let mut renderer = Renderer::default();
    for event in Parser::new_ext(markdown, options) {
        renderer.event(ui, event);
    }
    renderer.flush_paragraph(ui);
}

impl Renderer {
    fn event(&mut self, ui: &mut egui::Ui, event: Event<'_>) {
        // コードブロックの中では Markdown を解釈せず、テキストをそのまま集める。
        if self.code.is_some() {
            match event {
                Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                    if let Some(code) = &mut self.code {
                        code.push_str(&text);
                    }
                    return;
                }
                Event::SoftBreak | Event::HardBreak => {
                    if let Some(code) = &mut self.code {
                        code.push('\n');
                    }
                    return;
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.flush_code(ui);
                    return;
                }
                _ => return,
            }
        }
        // 表の中ではセルごとに文字を集め、最後に Grid へ描く。
        if self.table.is_some() {
            match event {
                Event::Start(Tag::TableHead) => {
                    if let Some(table) = &mut self.table {
                        table.in_head = true;
                        table.row.clear();
                    }
                    return;
                }
                Event::Start(Tag::TableRow) => {
                    if let Some(table) = &mut self.table {
                        table.in_head = false;
                        table.row.clear();
                    }
                    return;
                }
                Event::Start(Tag::TableCell) => {
                    if let Some(table) = &mut self.table {
                        table.cell.clear();
                    }
                    return;
                }
                Event::End(TagEnd::TableCell) => {
                    if let Some(table) = &mut self.table {
                        let cell = std::mem::take(&mut table.cell);
                        table.row.push(cell);
                    }
                    return;
                }
                Event::End(TagEnd::TableHead) => {
                    if let Some(table) = &mut self.table {
                        table.header = std::mem::take(&mut table.row);
                    }
                    return;
                }
                Event::End(TagEnd::TableRow) => {
                    if let Some(table) = &mut self.table {
                        let row = std::mem::take(&mut table.row);
                        if table.in_head {
                            table.header = row;
                        } else {
                            table.rows.push(row);
                        }
                    }
                    return;
                }
                Event::End(TagEnd::Table) => {
                    self.flush_table(ui);
                    return;
                }
                Event::Text(text)
                | Event::Code(text)
                | Event::Html(text)
                | Event::InlineHtml(text) => {
                    if let Some(table) = &mut self.table {
                        table.cell.push_str(&text);
                    }
                    return;
                }
                Event::SoftBreak | Event::HardBreak => {
                    if let Some(table) = &mut self.table {
                        table.cell.push(' ');
                    }
                    return;
                }
                _ => return,
            }
        }
        match event {
            Event::Start(tag) => self.start(ui, tag),
            Event::End(tag) => self.end(ui, tag),
            Event::Text(text) => self.text(text.into_string()),
            Event::Code(text) => self.code_span(text.into_string()),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            Event::Rule => {
                self.flush_paragraph(ui);
                ui.separator();
            }
            Event::TaskListMarker(done) => {
                let marker = if done { "[x] " } else { "[ ] " };
                self.inline
                    .push(Inline::Text(marker.into(), self.style.clone()));
            }
            Event::Html(text) | Event::InlineHtml(text) => self.text(strip_html(&text)),
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                self.code_span(text.into_string())
            }
            Event::FootnoteReference(text) => {
                let marker = format!("[{text}]");
                self.inline.push(Inline::Text(marker, self.style.clone()));
            }
        }
    }

    fn start(&mut self, ui: &mut egui::Ui, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                if self.heading.is_none() {
                    self.inline.clear();
                }
            }
            Tag::Heading { level, .. } => {
                self.heading = Some(level_number(level));
                self.inline.clear();
            }
            Tag::BlockQuote(_) => self.quote += 1,
            Tag::CodeBlock(_) => self.code = Some(String::new()),
            Tag::List(start) => self.lists.push(ListLevel {
                ordered: start.is_some(),
                next: start.unwrap_or(1),
            }),
            Tag::Item => {
                self.flush_paragraph(ui);
                let prefix = match self.lists.last_mut() {
                    Some(level) if level.ordered => {
                        let number = level.next;
                        level.next += 1;
                        format!("{number}. ")
                    }
                    _ => "• ".to_owned(),
                };
                self.item_prefix = Some(prefix);
                self.inline.clear();
            }
            Tag::Emphasis => self.style.emphasis = true,
            Tag::Strong => self.style.strong = true,
            Tag::Strikethrough => self.style.strike = true,
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                self.link = Some((dest_url.into_string(), String::new()));
            }
            Tag::Table(_) => self.table = Some(Table::default()),
            _ => {}
        }
    }

    fn end(&mut self, ui: &mut egui::Ui, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                if self.heading.is_none() {
                    self.flush_paragraph(ui);
                }
            }
            TagEnd::Heading(_) => self.flush_heading(ui),
            TagEnd::BlockQuote(_) => self.quote = self.quote.saturating_sub(1),
            TagEnd::CodeBlock => self.flush_code(ui),
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::Item => {
                self.flush_paragraph(ui);
                self.item_prefix = None;
            }
            TagEnd::Emphasis => self.style.emphasis = false,
            TagEnd::Strong => self.style.strong = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link | TagEnd::Image => {
                if let Some((url, text)) = self.link.take() {
                    self.inline.push(Inline::Link { text, url });
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: String) {
        if let Some((_, link_text)) = &mut self.link {
            link_text.push_str(&text);
            return;
        }
        self.inline.push(Inline::Text(text, self.style.clone()));
    }

    fn code_span(&mut self, text: String) {
        if let Some((_, link_text)) = &mut self.link {
            link_text.push_str(&text);
            return;
        }
        self.inline.push(Inline::Code(text));
    }

    fn soft_break(&mut self) {
        if let Some((_, link_text)) = &mut self.link {
            link_text.push(' ');
        } else {
            self.inline.push(Inline::SoftBreak);
        }
    }

    fn hard_break(&mut self) {
        if let Some((_, link_text)) = &mut self.link {
            link_text.push(' ');
        } else {
            self.inline.push(Inline::HardBreak);
        }
    }

    fn flush_paragraph(&mut self, ui: &mut egui::Ui) {
        if self.inline.is_empty() {
            return;
        }
        let indent = self.lists.len().saturating_sub(1) * 16 + self.quote * 16;
        let prefix = self.item_prefix.clone();
        let quoted = self.quote > 0;
        let inline = std::mem::take(&mut self.inline);
        ui.horizontal_wrapped(|ui| {
            if indent > 0 {
                ui.add_space(indent as f32);
            }
            if quoted {
                ui.label(egui::RichText::new("▏").color(ui.visuals().weak_text_color()));
            }
            if let Some(prefix) = &prefix {
                ui.label(egui::RichText::new(prefix).strong());
            }
            let mut start = 0;
            for (index, item) in inline.iter().enumerate() {
                if matches!(item, Inline::HardBreak) {
                    render_segment(ui, &inline[start..index]);
                    ui.add_space(2.);
                    start = index + 1;
                }
            }
            render_segment(ui, &inline[start..]);
        });
        ui.add_space(4.);
    }

    fn flush_heading(&mut self, ui: &mut egui::Ui) {
        let level = self.heading.take().unwrap_or(3);
        let text = plain(&self.inline);
        self.inline.clear();
        if text.is_empty() {
            return;
        }
        let size = match level {
            1 => 24.,
            2 => 20.,
            3 => 17.,
            _ => 15.,
        };
        ui.add_space(if level <= 2 { 10. } else { 6. });
        ui.add(egui::Label::new(egui::RichText::new(text).size(size).strong()).wrap());
        ui.add_space(2.);
    }

    fn flush_code(&mut self, ui: &mut egui::Ui) {
        let Some(code) = self.code.take() else {
            return;
        };
        ui.add_space(4.);
        egui::Frame::NONE
            .fill(ui.visuals().code_bg_color)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(code.trim_end()).monospace()).wrap());
            });
        ui.add_space(4.);
    }

    fn flush_table(&mut self, ui: &mut egui::Ui) {
        let Some(table) = self.table.take() else {
            return;
        };
        self.tables += 1;
        ui.add_space(4.);
        egui::Grid::new(("release-notes-table", self.tables))
            .striped(true)
            .spacing([14., 6.])
            .show(ui, |ui| {
                for cell in &table.header {
                    ui.strong(cell);
                }
                ui.end_row();
                for row in &table.rows {
                    for cell in row {
                        ui.label(cell);
                    }
                    ui.end_row();
                }
            });
        ui.add_space(6.);
    }
}

fn render_segment(ui: &mut egui::Ui, inline: &[Inline]) {
    for item in inline {
        match item {
            Inline::Text(text, style) => {
                if text.is_empty() {
                    continue;
                }
                let mut rich = egui::RichText::new(text);
                if style.strong {
                    rich = rich.strong();
                }
                if style.emphasis {
                    rich = rich.italics();
                }
                if style.strike {
                    rich = rich.strikethrough();
                }
                ui.label(rich);
            }
            Inline::Code(text) => {
                ui.code(text);
            }
            Inline::Link { text, url } => {
                ui.hyperlink_to(
                    if text.is_empty() {
                        url.as_str()
                    } else {
                        text.as_str()
                    },
                    url,
                );
            }
            Inline::SoftBreak => {
                ui.label(" ");
            }
            Inline::HardBreak => {}
        }
    }
}

fn plain(inline: &[Inline]) -> String {
    let mut text = String::new();
    for item in inline {
        match item {
            Inline::Text(run, _) | Inline::Code(run) => text.push_str(run),
            Inline::Link { text: run, .. } => text.push_str(run),
            Inline::SoftBreak | Inline::HardBreak => text.push(' '),
        }
    }
    text.trim().to_owned()
}

fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        _ => 4,
    }
}

fn strip_html(text: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for character in text.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_release_notes_and_tables_without_panicking() {
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600., 800.),
            )),
            ..Default::default()
        };
        let sample = "# title\n\n- **bold** and `code`\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n> quote\n\n```\ncode\n```\n\n[link](https://example.com)\n";
        let _ = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(ui, sample));
        });
        for english in [false, true] {
            let notes = crate::release_notes::notes("1.1.6", english).unwrap();
            let _ = ctx.run(input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| show(ui, notes));
            });
        }
    }
}
