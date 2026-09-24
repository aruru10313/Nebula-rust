use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.heading(
        egui::RichText::new(format!("안녕하세요, {}님", app.config.username)).size(24.0).strong(),
    );
    ui.label(
        egui::RichText::new("오늘도 네뷸랴와 함께 즐거운 마크 시간 되세요.")
            .color(crate::ui::theme::TEXT_DIM),
    );
    ui.add_space(12.0);

    ui.horizontal(|ui| {
        // 선택된 인스턴스 카드
        crate::ui::theme::card_frame().show(ui, |ui| {
            ui.set_min_size(egui::vec2(420.0, 170.0));
            if let Some(inst) = app.selected_instance().cloned() {
                ui.label(
                    egui::RichText::new("선택된 인스턴스")
                        .size(12.0)
                        .color(crate::ui::theme::TEXT_DIM),
                );
                ui.heading(egui::RichText::new(&inst.name).size(20.0).strong());
                ui.label(
                    egui::RichText::new(inst.display_version())
                        .color(crate::ui::theme::ACCENT_HOVER),
                );
                ui.add_space(6.0);
                ui.label(format!(
                    "플레이 {}회{}",
                    inst.total_plays,
                    inst.last_played
                        .map(|t| format!(" • 마지막: {}", t.format("%m/%d %H:%M")))
                        .unwrap_or_default()
                ));
                ui.add_space(8.0);
                if crate::ui::theme::accent_button(ui, "▶  지금 플레이").clicked() {
                    app.launch();
                }
            } else {
                ui.label("인스턴스가 없습니다. 새로 만들어주세요.");
            }
        });

        // 빠른 상태 카드
        crate::ui::theme::card_frame().show(ui, |ui| {
            ui.set_min_size(egui::vec2(300.0, 170.0));
            ui.label(
                egui::RichText::new("시스템")
                    .size(12.0)
                    .color(crate::ui::theme::TEXT_DIM),
            );
            let java = app.config.effective_java();
            ui.label(format!("Java: {java}"));
            if let Some(ver) = crate::minecraft::java::java_version(&java) {
                ui.label(format!("버전: {ver}"));
            }
            ui.label(format!("RAM: {} MB", app.config.ram_mb));
            ui.label(format!("루트: {}", app.config.game_root.display()));
            ui.add_space(6.0);
            if ui.small_button("Java 다시 찾기").clicked() {
                if let Some(j) = crate::minecraft::java::find_java() {
                    app.config.java_path = j;
                }
            }
        });
    });

    ui.add_space(12.0);

    // 로그 카드 (Dawn 스타일 콘솔 미리보기)
    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("최근 로그");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("지우기").clicked() {
                    app.logs.clear();
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            for line in app.logs.iter().rev().take(60) {
                ui.monospace(line);
            }
        });
    });
}
