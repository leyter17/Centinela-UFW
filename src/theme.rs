//! Tema visual verde oscuro para Centinela UFW.

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

pub const BG: Color32 = Color32::from_rgb(0x0C, 0x11, 0x0E);
pub const SURFACE: Color32 = Color32::from_rgb(0x13, 0x1B, 0x16);
pub const SURFACE2: Color32 = Color32::from_rgb(0x1A, 0x24, 0x1D);
pub const SURFACE3: Color32 = Color32::from_rgb(0x21, 0x2E, 0x25);
pub const BORDER: Color32 = Color32::from_rgb(0x27, 0x35, 0x2C);
pub const TEXT: Color32 = Color32::from_rgb(0xE7, 0xF3, 0xEA);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x96, 0xAB, 0x9E);
pub const GREEN: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
pub const GREEN_PALE: Color32 = Color32::from_rgb(0x4A, 0xDE, 0x83);
pub const GREEN_DEEP: Color32 = Color32::from_rgb(0x15, 0x80, 0x3D);
pub const RED: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);
pub const RED_DEEP: Color32 = Color32::from_rgb(0x7F, 0x1D, 0x1D);
pub const RED_DARK_BG: Color32 = Color32::from_rgb(0x2A, 0x14, 0x14);
pub const AMBER: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);
pub const BLUE: Color32 = Color32::from_rgb(0x7D, 0xD3, 0xFC);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = Color32::from_rgb(0x09, 0x0D, 0x0A);
    v.faint_bg_color = SURFACE2;
    v.hyperlink_color = GREEN_PALE;
    v.warn_fg_color = AMBER;
    v.error_fg_color = RED;
    v.selection.bg_fill = GREEN.gamma_multiply(0.32);
    v.selection.stroke = Stroke::new(1.0_f32, GREEN);
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(14.0);
    v.menu_rounding = Rounding::same(10.0);
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.weak_bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    w.noninteractive.rounding = Rounding::same(8.0);

    w.inactive.bg_fill = SURFACE2;
    w.inactive.weak_bg_fill = SURFACE2;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.inactive.rounding = Rounding::same(9.0);

    w.hovered.bg_fill = SURFACE3;
    w.hovered.weak_bg_fill = SURFACE3;
    w.hovered.bg_stroke = Stroke::new(1.2_f32, GREEN_DEEP);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.hovered.rounding = Rounding::same(9.0);

    w.active.bg_fill = GREEN_DEEP;
    w.active.weak_bg_fill = GREEN_DEEP;
    w.active.bg_stroke = Stroke::new(1.4_f32, GREEN);
    w.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.active.rounding = Rounding::same(9.0);

    ctx.style_mut(|s| {
        s.spacing.button_padding = Vec2::new(12.0, 7.0);
        s.spacing.item_spacing = Vec2::new(10.0, 8.0);
        s.spacing.indent = 18.0;
    });
    ctx.set_visuals(v);
}
