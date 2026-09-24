//! Lunar / Dawn 스타일 다크 테마

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(124, 93, 250); // 네뷸랴 보라
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(139, 109, 255);
pub const BG_DARK: Color32 = Color32::from_rgb(13, 14, 20);
pub const BG_PANEL: Color32 = Color32::from_rgb(19, 21, 30);
pub const BG_CARD: Color32 = Color32::from_rgb(25, 28, 40);
pub const TEXT_MAIN: Color32 = Color32::from_rgb(235, 238, 245);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 158, 178);

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = Style::default();
    style.visuals = dark_visuals();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(10);
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_PANEL;
    v.extreme_bg_color = BG_DARK;
    v.faint_bg_color = BG_CARD;
    v.code_bg_color = BG_CARD;
    v.override_text_color = Some(TEXT_MAIN);
    v.warn_fg_color = Color32::from_rgb(255, 190, 90);
    v.error_fg_color = Color32::from_rgb(255, 110, 120);

    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT_HOVER);
    v.hyperlink_color = ACCENT_HOVER;

    // 버튼 상태
    v.widgets.inactive.bg_fill = BG_CARD;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_MAIN);
    v.widgets.hovered.bg_fill = Color32::from_rgb(33, 37, 52);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.open.bg_fill = BG_CARD;
    v
}

/// 카드 프레임 (Lunar식 둥근 패널)
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(14))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(255, 255, 255, 14),
        ))
        .inner_margin(Margin::same(16))
}

/// 강조 버튼 스타일 헬퍼
pub fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label).strong().color(Color32::WHITE).size(15.0),
    )
    .fill(ACCENT)
    .corner_radius(CornerRadius::same(10))
    .min_size(egui::vec2(180.0, 44.0));
    ui.add(btn)
}
