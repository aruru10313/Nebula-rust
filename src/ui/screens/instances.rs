use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.heading("인스턴스");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::ui::theme::accent_button(ui, "+ 새 인스턴스").clicked() {
                app.show_new_instance = true;
            }
        });
    });
    ui.add_space(8.0);

    let mut to_delete: Option<String> = None;
    for inst in app.instances.clone() {
        let selected = app.config.selected_instance.as_deref() == Some(&inst.id);
        crate::ui::theme::card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new(&inst.name).size(17.0).strong());
                        if selected {
                            ui.label(
                                egui::RichText::new("선택됨")
                                    .color(crate::ui::theme::ACCENT_HOVER)
                                    .size(12.0),
                            );
                        }
                    });
                    ui.label(
                        egui::RichText::new(inst.display_version())
                            .color(crate::ui::theme::TEXT_DIM),
                    );
                    ui.label(format!(
                        "{}회 플레이{}",
                        inst.total_plays,
                        inst.last_played
                            .map(|t| format!(" • {}", t.format("%Y-%m-%d %H:%M")))
                            .unwrap_or_default()
                    ));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🗑 삭제").clicked() {
                        to_delete = Some(inst.id.clone());
                    }
                    if !selected && ui.small_button("선택").clicked() {
                        app.config.selected_instance = Some(inst.id.clone());
                    }
                    if ui.small_button("▶ 실행").clicked() {
                        app.config.selected_instance = Some(inst.id.clone());
                        app.launch();
                    }
                });
            });
        });
        ui.add_space(6.0);
    }

    if let Some(id) = to_delete {
        app.instances.retain(|i| i.id != id);
        if app.config.selected_instance.as_deref() == Some(&id) {
            app.config.selected_instance = app.instances.first().map(|i| i.id.clone());
        }
    }
}
