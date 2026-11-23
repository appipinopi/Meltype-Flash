// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 amanetoki7, appipinopi
use eframe::egui;
pub fn wheel_step(ui: &egui::Ui, response: &egui::Response, enabled: bool) -> i64 {
    if !enabled || !response.hovered() {
        return 0;
    }
    ui.input_mut(|i| {
        let delta = i.smooth_scroll_delta.y;
        if delta.abs() < 1. {
            return 0;
        }
        i.smooth_scroll_delta.y = 0.;
        i.raw_scroll_delta.y = 0.;
        if delta > 0. {
            1
        } else {
            -1
        }
    })
}
/// Same 40×20 track, ON/OFF text and keyboard role as upstream ToggleSwitch.
pub fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(88., 28.), egui::Sense::click());
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("toggle_rect", label)), rect));
    // egui's generic click also includes held Space/Enter repeats. A switch
    // should change only once per press, while keeping accessibility activation.
    let repeated_key = response.has_focus()
        && ui.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        key: egui::Key::Space | egui::Key::Enter,
                        pressed: true,
                        repeat: true,
                        ..
                    }
                )
            })
        });
    if ui.is_enabled() && response.clicked() && !repeated_key {
        response.request_focus();
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, label)
    });
    let track = egui::Rect::from_min_size(rect.min + egui::vec2(4., 4.), egui::vec2(40., 20.));
    let accent = egui::Color32::from_rgb(0, 120, 212);
    let gray = ui.visuals().weak_text_color();
    ui.painter().rect_filled(
        track,
        10.,
        if *value {
            accent
        } else {
            ui.visuals().panel_fill
        },
    );
    if !*value {
        ui.painter().rect_stroke(
            track,
            10.,
            egui::Stroke::new(1_f32, gray),
            egui::StrokeKind::Inside,
        );
    }
    let radius = if response.hovered() { 7. } else { 6. };
    let center = egui::pos2(
        if *value {
            track.right() - 10.
        } else {
            track.left() + 10.
        },
        track.center().y,
    );
    ui.painter().circle_filled(
        center,
        radius,
        if *value { egui::Color32::WHITE } else { gray },
    );
    ui.painter().text(
        egui::pos2(track.right() + 12., track.center().y),
        egui::Align2::LEFT_CENTER,
        if *value { "ON" } else { "OFF" },
        egui::FontId::proportional(13.),
        ui.visuals().text_color(),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            2.,
            egui::Stroke::new(1_f32, accent),
            egui::StrokeKind::Inside,
        );
    }
    response
}

/// Match the reference Windows form in logical pixels; OS scaling remains native.
pub fn configure_style(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::light(),
        ..Default::default()
    };
    let background = egui::Color32::from_gray(240);
    style.visuals.panel_fill = background;
    style.visuals.window_fill = background;
    style.visuals.override_text_color = Some(egui::Color32::BLACK);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::WHITE;
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::WHITE;
    style.visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1_f32, egui::Color32::from_gray(200));
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(229, 241, 251);
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1_f32, egui::Color32::from_rgb(0, 120, 212));
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(3);
    }
    for text in [
        egui::TextStyle::Body,
        egui::TextStyle::Button,
        egui::TextStyle::Small,
    ] {
        style
            .text_styles
            .insert(text, egui::FontId::proportional(13.));
    }
    style.spacing.item_spacing = egui::vec2(8., 4.);
    style.spacing.button_padding = egui::vec2(8., 3.);
    style.spacing.interact_size.y = 24.;
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    ctx.set_theme(egui::Theme::Light);
    ctx.set_style_of(egui::Theme::Light, style);
}

/// GroupBox border with the title interrupting the top edge, as in WinForms.
pub fn group_border(ui: &egui::Ui, rect: egui::Rect, title: &str) {
    let rect = rect.shrink2(egui::vec2(6., 0.));
    let top = rect.top() + 7.;
    let border = egui::Stroke::new(1_f32, egui::Color32::from_gray(216));
    let painter = ui.painter();
    painter.rect_stroke(
        egui::Rect::from_min_max(egui::pos2(rect.left(), top), rect.max),
        0.,
        border,
        egui::StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        title.to_owned(),
        egui::FontId::proportional(13.),
        egui::Color32::BLACK,
    );
    let text = egui::Rect::from_min_size(egui::pos2(rect.left() + 8., rect.top()), galley.size());
    painter.rect_filled(
        text.expand2(egui::vec2(3., 0.)),
        0.,
        ui.visuals().panel_fill,
    );
    painter.galley(text.min, galley, egui::Color32::BLACK);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_toggle_changes_once_and_respects_disabled_controls() {
        let ctx = egui::Context::default();
        let mut value = false;
        let mut id = egui::Id::NULL;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_enabled_ui(true, |ui| {
                    id = toggle(ui, &mut value, "test switch").id;
                });
            });
        });
        ctx.memory_mut(|memory| memory.request_focus(id));
        for (repeat, enabled, expected) in [
            (false, true, true),
            (true, true, true),
            (false, false, true),
        ] {
            let raw = egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: Some(egui::Key::Space),
                    pressed: true,
                    repeat,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        toggle(ui, &mut value, "test switch");
                    });
                });
            });
            assert_eq!(value, expected, "repeat={repeat} enabled={enabled}");
        }
    }
}
