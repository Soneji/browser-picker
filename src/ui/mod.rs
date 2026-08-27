pub mod home;
pub mod picker;

/// Apply a shared modern dark theme: flat, rounded, with the accent yellow
/// (#f3a200) the user favours on a near-black (#1b1b1b) background.
pub fn apply_theme(ctx: &egui::Context) {
    use egui::{Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle, Visuals};

    let bg = Color32::from_rgb(0x1b, 0x1b, 0x1b);
    let panel = Color32::from_rgb(0x2a, 0x2a, 0x2a);
    let accent = Color32::from_rgb(0xf3, 0xa2, 0x00);
    let text = Color32::from_rgb(0xea, 0xea, 0xea);
    let round = Rounding::same(10.0);

    let mut v = Visuals::dark();
    v.panel_fill = bg;
    v.window_fill = bg;
    v.extreme_bg_color = bg;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text);
    v.widgets.inactive.bg_fill = panel;
    v.widgets.inactive.weak_bg_fill = panel;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text);
    v.widgets.inactive.rounding = round;
    v.widgets.hovered.bg_fill = accent;
    v.widgets.hovered.weak_bg_fill = accent;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.hovered.rounding = round;
    v.widgets.active.bg_fill = accent;
    v.widgets.active.weak_bg_fill = accent;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    v.widgets.active.rounding = round;

    let mut style = (*ctx.style()).clone();
    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 10.0);
    style.spacing.window_margin = Margin::same(16.0);
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(16.0, FontFamily::Proportional));
    ctx.set_style(style);
}
