//! Nebulya 디자인 시스템 — Modrinth식 플랫 다크 테마
//!
//! - 배경: 차가운 다크 슬레이트, 카드: 1px 실선 테두리
//! - 강조: 모드리민트 그린, 정보: 블루, 위험: 레드
//! - 글로우·그라데이션 없음. 반경 작게, 대비 또렷하게.

use egui::{Color32, CornerRadius, Margin, Stroke, Visuals};

// ---- 팔레트 ----
pub const SPACE: Color32 = Color32::from_rgb(15, 17, 19); // 최외곽 배경
pub const BG_PANEL: Color32 = Color32::from_rgb(23, 25, 28); // 사이드/상단 패널
pub const BG_CARD: Color32 = Color32::from_rgb(31, 35, 40); // 카드
pub const BG_TILE: Color32 = Color32::from_rgb(39, 44, 51); // 타일/입력

pub const BORDER: Color32 = Color32::from_rgb(58, 64, 73); // 1px 실선 테두리

pub const NEBULA: Color32 = Color32::from_rgb(27, 217, 106); // 모드리민트 그린
pub const NEBULA_LIGHT: Color32 = Color32::from_rgb(64, 217, 119); // 밝은 그린(텍스트 강조)
pub const NEBULA_DARK_TEXT: Color32 = Color32::from_rgb(11, 20, 16); // 그린 버튼 위 글자
pub const STAR_PINK: Color32 = Color32::from_rgb(232, 131, 58); // 오렌지 포인트
pub const STAR_BLUE: Color32 = Color32::from_rgb(58, 169, 255); // 정보 블루

pub const STARLIGHT: Color32 = Color32::from_rgb(242, 244, 245); // 본문 텍스트
pub const TEXT_DIM: Color32 = Color32::from_rgb(155, 161, 168); // 보조 텍스트
pub const TEXT_FAINT: Color32 = Color32::from_rgb(100, 107, 116);

pub const SUCCESS: Color32 = Color32::from_rgb(27, 217, 106);
pub const WARN: Color32 = Color32::from_rgb(232, 185, 62);
pub const DANGER: Color32 = Color32::from_rgb(236, 95, 95);

// 구 이름 호환 (외부 참조용)
pub const ACCENT_HOVER: Color32 = NEBULA_LIGHT;

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: dark_visuals(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(8);
    style.visuals.popup_shadow.blur = 16;
    ctx.set_style(style);
}

pub fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_PANEL;
    v.extreme_bg_color = SPACE;
    v.faint_bg_color = BG_CARD;
    v.code_bg_color = BG_TILE;
    v.override_text_color = Some(STARLIGHT);
    v.warn_fg_color = WARN;
    v.error_fg_color = DANGER;

    v.selection.bg_fill = NEBULA;
    v.selection.stroke = Stroke::new(1.0_f32, NEBULA_LIGHT);
    v.hyperlink_color = NEBULA_LIGHT;

    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.inactive.bg_fill = BG_TILE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, STARLIGHT);
    v.widgets.inactive.weak_bg_fill = BG_TILE;
    v.widgets.hovered.bg_fill = Color32::from_rgb(46, 74, 60);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, STARLIGHT);
    v.widgets.active.bg_fill = NEBULA;
    v.widgets.open.bg_fill = BG_CARD;

    // 위젯 테두리: 얇은 실선
    let edge = Stroke::new(1.0_f32, BORDER);
    v.widgets.noninteractive.bg_stroke = edge;
    v.widgets.inactive.bg_stroke = edge;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, NEBULA);
    v
}

// ---- 카드 ----
/// 기본 카드 (1px 실선 테두리, 플랫)
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(Margin::same(16))
}

/// 선택 강조 카드 (그린 테두리)
pub fn glow_card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(28, 38, 33))
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::new(1.5_f32, NEBULA))
        .inner_margin(Margin::same(16))
}

/// 작은 타일 (스탯/입력 행)
pub fn tile_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_TILE)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
}

// ---- 버튼 ----
/// 주 버튼: 그린 (짙은 글자)
pub fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .strong()
            .color(NEBULA_DARK_TEXT)
            .size(15.0),
    )
    .fill(NEBULA)
    .corner_radius(CornerRadius::same(8))
    .min_size(egui::vec2(190.0, 46.0));
    ui.add(btn)
}

/// 보조 버튼: 다크 플랫
pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(label).color(STARLIGHT).size(13.0))
        .fill(BG_TILE)
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(110.0, 34.0));
    ui.add(btn)
}

/// 작은 주 버튼 (상단바·카드용)
pub fn small_accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .strong()
            .color(NEBULA_DARK_TEXT)
            .size(13.0),
    )
    .fill(NEBULA)
    .corner_radius(CornerRadius::same(8))
    .min_size(egui::vec2(120.0, 34.0));
    ui.add(btn)
}

/// 타이틀바 창 버튼 (규격 통일 46x32, 닫기는 붉은 기운)
pub fn titlebar_button(ui: &mut egui::Ui, label: &str, close: bool) -> egui::Response {
    let fill = if close {
        Color32::from_rgba_unmultiplied(236, 95, 95, 26)
    } else {
        Color32::TRANSPARENT
    };
    let btn = egui::Button::new(egui::RichText::new(label).size(14.0).color(if close {
        DANGER
    } else {
        TEXT_DIM
    }))
    .fill(fill)
    .corner_radius(CornerRadius::same(6))
    .min_size(egui::vec2(46.0, 32.0));
    ui.add_sized([46.0, 32.0], btn)
}

/// 위험 버튼: 삭제 등
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(label).color(Color32::WHITE).size(13.0))
        .fill(Color32::from_rgb(150, 60, 60))
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(110.0, 34.0));
    ui.add(btn)
}

// ---- 장식 ----
/// 섹션 헤더: 작은 마크 + 제목 + 희미한 부제
pub fn section_header(ui: &mut egui::Ui, sparkle: &str, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(sparkle).color(NEBULA).size(14.0));
        ui.heading(egui::RichText::new(title).size(17.0).strong());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).color(TEXT_DIM).size(12.0));
        }
    });
}

/// 얇은 구분선
pub fn star_divider(ui: &mut egui::Ui) {
    let avail = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail, 8.0), egui::Sense::hover());
    let y = rect.center().y;
    ui.painter().line_segment(
        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(100, 107, 116, 90)),
    );
}

/// 폼 두 칸 행: 왼쪽 고정 라벨 + 오른쪽 컨텐츠 (줄 맞춤용)
pub fn form_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [130.0, 20.0],
            egui::Label::new(egui::RichText::new(label).color(TEXT_DIM)),
        );
        add(ui);
    });
}

/// 작은 칩 (버전/상태 표시)
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            30,
        ))
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 110),
        ))
        .inner_margin(Margin {
            left: 8,
            right: 8,
            top: 3,
            bottom: 3,
        })
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(11.0).strong());
        });
}

/// 스탯 타일: 큰 값 + 작은 라벨
pub fn stat_tile(ui: &mut egui::Ui, icon: &str, value: &str, label: &str, accent: Color32) {
    tile_frame().show(ui, |ui| {
        ui.set_min_size(egui::vec2(150.0, 64.0));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon).size(20.0).color(accent));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(value).size(18.0).strong());
                ui.label(egui::RichText::new(label).size(11.0).color(TEXT_DIM));
            });
        });
    });
}

// ---- 홈 히어로 배너 ----
/// 플랫 배너: 카드 바탕 + 실선 + 상단 그린 스트립. 내용은 호출자가 오버레이로 배치.
pub fn paint_nebula(ctx: &egui::Context, rect: egui::Rect) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, CornerRadius::same(10), BG_CARD);
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
}

/// 홈 히어로: 플랫 배너 + 상단 그린 스트립
pub fn paint_hero(ctx: &egui::Context, rect: egui::Rect) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let r = CornerRadius::same(12);
    painter.rect_filled(rect, r, BG_CARD);
    painter.rect_stroke(
        rect,
        r,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    // 상단 그린 스트립
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 24.0, rect.min.y),
            egui::vec2(rect.width() - 48.0, 3.0),
        ),
        CornerRadius::same(1),
        NEBULA,
    );
}

// ---- 풀화면 배경 (온보딩/로그인 게이트용) ----
pub fn paint_backdrop(ctx: &egui::Context, rect: egui::Rect) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, CornerRadius::ZERO, SPACE);
}
