use crate::ui::theme;
use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);

    // ---- 풀블리드 성운 히어로 ----
    let avail = ui.available_width();
    let hero_h = 248.0;
    let (hero_rect, _) = ui.allocate_exact_size(egui::vec2(avail, hero_h), egui::Sense::hover());
    theme::paint_hero(ui.ctx(), hero_rect);

    // 히어로 위 콘텐츠 오버레이
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(hero_rect.shrink(22.0)),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("★ S T E L L A R · F A B R I C")
                        .size(12.0)
                        .color(theme::NEBULA_LIGHT)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("NEBULYA")
                        .size(44.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.label(
                    egui::RichText::new(format!("안녕하세요, {}님", app.config.username))
                        .size(15.0)
                        .color(theme::STARLIGHT),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if let Some(inst) = app.selected_instance().cloned() {
                        theme::badge(ui, &format!("◆ {}", inst.name), theme::STAR_BLUE);
                        theme::badge(ui, &inst.display_version(), theme::NEBULA_LIGHT);
                        ui.label(
                            egui::RichText::new(format!(
                                "★ {}회 항해{}",
                                inst.total_plays,
                                inst.last_played
                                    .map(|t| format!(" · 마지막: {}", t.format("%m/%d %H:%M")))
                                    .unwrap_or_default()
                            ))
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                        );
                    } else {
                        ui.label("인스턴스가 없습니다. 새로 만들어주세요.");
                    }
                });
                ui.add_space(10.0);
                if theme::accent_button(
                    ui,
                    if app.launching {
                        "★  워프 중..."
                    } else {
                        "▶  지금 항해"
                    },
                )
                .clicked()
                    && !app.launching
                {
                    app.launch();
                }
            });
        },
    );

    ui.add_space(12.0);

    // ---- 스탯 타일 3개 ----
    let (plays, mods_count, java_short) = {
        let plays = app
            .selected_instance()
            .map(|i| i.total_plays.to_string())
            .unwrap_or_else(|| "-".into());
        let mods_count = app
            .selected_instance()
            .map(|i| i.scan_mod_files(&app.config.game_root).len().to_string())
            .unwrap_or_else(|| "-".into());
        let java = app.config.effective_java();
        let ver = crate::minecraft::java::java_version(&java).unwrap_or_default();
        let short = ver.split('.').take(2).collect::<Vec<_>>().join(".");
        (plays, mods_count, short)
    };
    ui.horizontal(|ui| {
        theme::stat_tile(ui, "🚀", &plays, "항해 횟수", theme::NEBULA_LIGHT);
        theme::stat_tile(ui, "◆", &mods_count, "탑재 모드", theme::STAR_PINK);
        theme::stat_tile(
            ui,
            "☕",
            if java_short.is_empty() {
                "-"
            } else {
                &java_short
            },
            "Java 버전",
            theme::STAR_BLUE,
        );
        theme::stat_tile(
            ui,
            "💾",
            &format!("{}M", app.config.ram_mb),
            "할당 RAM",
            theme::SUCCESS,
        );
    });

    ui.add_space(12.0);

    // ---- 항해일지 (로그) ----
    theme::glow_card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_header(ui, "≡", "항해일지", "최근 로그");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, "지우기").clicked() {
                    app.logs.clear();
                }
            });
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                for line in app.logs.iter().rev().take(60) {
                    ui.monospace(line);
                }
            });
    });
}
