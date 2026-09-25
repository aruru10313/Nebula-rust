//! Nebulya 성운(nebula) 디자인 시스템
//! 컨셉: 깊은 우주의 밤하늘 + 보라/핑크 성운 + 별빛
//!
//! - BG: 거의 검은 남청색 (우주)
//! - 카드: 짙은 인디고 + 별빛 테두리
//! - 강조: 성운 바이올렛 → hover 라벤더, 포인트 핑크
//! - 장식: ★ 반짝이, 별 구분선, 히어로 성운 페인터

use egui::{Color32, CornerRadius, Margin, Stroke, Visuals};

// ---- 팔레트 ----
pub const SPACE: Color32 = Color32::from_rgb(7, 8, 16); // 우주 배경
pub const BG_PANEL: Color32 = Color32::from_rgb(11, 12, 24); // 사이드/상단 패널
pub const BG_CARD: Color32 = Color32::from_rgb(17, 19, 35); // 카드
pub const BG_TILE: Color32 = Color32::from_rgb(22, 25, 46); // 타일/입력
pub const BG_HOVER: Color32 = Color32::from_rgb(28, 31, 56);

pub const NEBULA: Color32 = Color32::from_rgb(139, 92, 246); // 성운 바이올렛
pub const NEBULA_LIGHT: Color32 = Color32::from_rgb(167, 139, 250); // 라벤더
pub const STAR_PINK: Color32 = Color32::from_rgb(236, 72, 153); // 성운 핑크
pub const STAR_BLUE: Color32 = Color32::from_rgb(96, 165, 250); // 별빛 블루

pub const STARLIGHT: Color32 = Color32::from_rgb(240, 241, 250); // 별빛 텍스트
pub const TEXT_DIM: Color32 = Color32::from_rgb(148, 155, 190); // 라벤더 그레이
pub const TEXT_FAINT: Color32 = Color32::from_rgb(105, 111, 148);

pub const SUCCESS: Color32 = Color32::from_rgb(52, 211, 153);
pub const WARN: Color32 = Color32::from_rgb(251, 191, 36);
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);

// 구 이름 호환 (외부 참조용)
pub const ACCENT_HOVER: Color32 = NEBULA_LIGHT;

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: dark_visuals(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(10);
    // 콤보박스/메뉴 팝업도 우주색으로
    style.visuals.popup_shadow.blur = 24;
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
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
    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, STARLIGHT);
    v.widgets.active.bg_fill = NEBULA;
    v.widgets.open.bg_fill = BG_CARD;

    // 위젯 테두리에 별빛 실선
    let star_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(160, 170, 220, 40));
    v.widgets.noninteractive.bg_stroke = star_stroke;
    v.widgets.inactive.bg_stroke = star_stroke;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, NEBULA_LIGHT);
    v
}

// ---- 카드 ----
/// 기본 카드 (별빛 헤어라인 테두리)
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(16))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(170, 180, 230, 26),
        ))
        .inner_margin(Margin::same(16))
}

/// 선택/히어로용 성운 글로우 카드 (바이올렛 테두리)
pub fn glow_card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(20, 18, 42))
        .corner_radius(CornerRadius::same(16))
        .stroke(Stroke::new(1.5_f32, NEBULA))
        .inner_margin(Margin::same(16))
}

/// 작은 타일 (스탯/입력 행)
pub fn tile_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_TILE)
        .corner_radius(CornerRadius::same(12))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(170, 180, 230, 18),
        ))
        .inner_margin(Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
}

// ---- 버튼 ----
/// 주 버튼: 성운 바이올렛
pub fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .strong()
            .color(Color32::WHITE)
            .size(15.0),
    )
    .fill(NEBULA)
    .corner_radius(CornerRadius::same(12))
    .min_size(egui::vec2(190.0, 46.0));
    ui.add(btn)
}

/// 보조 버튼: 별빛 고스트
pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(label).color(STARLIGHT).size(13.0))
        .fill(BG_TILE)
        .corner_radius(CornerRadius::same(10))
        .min_size(egui::vec2(110.0, 34.0));
    ui.add(btn)
}

/// 위험 버튼: 삭제 등
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(label).color(Color32::WHITE).size(13.0))
        .fill(Color32::from_rgb(120, 40, 60))
        .corner_radius(CornerRadius::same(10))
        .min_size(egui::vec2(110.0, 34.0));
    ui.add(btn)
}

// ---- 장식 ----
/// 섹션 헤더: ★ 제목 + 희미한 부제
pub fn section_header(ui: &mut egui::Ui, sparkle: &str, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(sparkle).color(NEBULA_LIGHT).size(16.0));
        ui.heading(egui::RichText::new(title).size(18.0).strong());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).color(TEXT_DIM).size(12.0));
        }
    });
}

/// 별 구분선: ★ ── ★ ── ★
pub fn star_divider(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("★ ───────── ★ ───────── ★")
                .color(TEXT_FAINT)
                .size(11.0),
        );
    });
}

/// 작은 알약 배지 (버전/상태 표시)
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            26,
        ))
        .corner_radius(CornerRadius::same(20))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90),
        ))
        .inner_margin(Margin {
            left: 10,
            right: 10,
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
            ui.label(egui::RichText::new(icon).size(22.0).color(accent));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(value).size(18.0).strong());
                ui.label(egui::RichText::new(label).size(11.0).color(TEXT_DIM));
            });
        });
    });
}

// ---- 성운 히어로 페인터 ----
/// 주어진 rect에 성운 배경(글로우 + 별)을 그림. 내용은 호출자가 overlay로 배치.
pub fn paint_nebula(ctx: &egui::Context, rect: egui::Rect) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let r = CornerRadius::same(18);

    // 깊은 우주 바탕
    painter.rect_filled(rect, r, Color32::from_rgb(13, 12, 30));

    // 성운 글로우 (반투명 원 3개)
    let glow = |center: egui::Pos2, radius: f32, color: Color32| {
        for (i, alpha) in [22u8, 14, 8].iter().enumerate() {
            painter.circle_filled(
                center,
                radius * (1.0 - i as f32 * 0.22),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), *alpha),
            );
        }
    };
    glow(
        egui::pos2(rect.right() - 90.0, rect.center().y - 10.0),
        130.0,
        NEBULA,
    );
    glow(
        egui::pos2(rect.right() - 200.0, rect.bottom() - 20.0),
        80.0,
        STAR_PINK,
    );
    glow(
        egui::pos2(rect.left() + 120.0, rect.top() + 20.0),
        70.0,
        STAR_BLUE,
    );

    // 별 (결정적 해시 배치)
    for i in 0..46u32 {
        let h1 = hash01(i.wrapping_mul(2654435761).wrapping_add(97));
        let h2 = hash01(i.wrapping_mul(40503).wrapping_add(5449));
        let x = rect.min.x + 8.0 + h1 * (rect.width() - 16.0);
        let y = rect.min.y + 8.0 + h2 * (rect.height() - 16.0);
        let big = i % 7 == 0;
        let a = if big { 200 } else { 90 + (h1 * 80.0) as u8 };
        painter.circle_filled(
            egui::pos2(x, y),
            if big { 1.8 } else { 1.0 },
            Color32::from_rgba_unmultiplied(235, 238, 255, a),
        );
        if big {
            // 십자 반짝이
            let c = Color32::from_rgba_unmultiplied(200, 190, 255, 120);
            painter.line_segment(
                [egui::pos2(x - 4.0, y), egui::pos2(x + 4.0, y)],
                Stroke::new(1.0_f32, c),
            );
            painter.line_segment(
                [egui::pos2(x, y - 4.0), egui::pos2(x, y + 4.0)],
                Stroke::new(1.0_f32, c),
            );
        }
    }

    // 테두리
    painter.rect_stroke(
        rect,
        r,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(167, 139, 250, 70)),
        egui::StrokeKind::Inside,
    );
}

fn hash01(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    (x as f32) / (u32::MAX as f32)
}
