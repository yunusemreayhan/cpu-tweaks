use egui::{Color32, CornerRadius, Stroke, Vec2, Visuals};

/// Dark theme colors.
pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(28, 28, 38);
pub const BG_CARD: Color32 = Color32::from_rgb(38, 38, 52);
pub const ACCENT: Color32 = Color32::from_rgb(99, 155, 255);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(60, 100, 180);
pub const GREEN: Color32 = Color32::from_rgb(76, 175, 80);
pub const ORANGE: Color32 = Color32::from_rgb(255, 152, 0);
pub const RED: Color32 = Color32::from_rgb(244, 67, 54);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 180);
pub const TEXT_DIM: Color32 = Color32::from_rgb(100, 100, 120);

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_CARD;
    visuals.faint_bg_color = BG_CARD;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 62);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 75);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.window_corner_radius = CornerRadius::same(10);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    ctx.set_style(style);
}

/// Color for frequency bar based on percentage of max.
pub fn freq_color(pct: f32) -> Color32 {
    if pct < 0.3 { GREEN }
    else if pct < 0.7 { ORANGE }
    else { RED }
}
