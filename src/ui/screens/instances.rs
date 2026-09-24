use crate::ui::NebulyaApp;
use crate::ui::theme;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        theme::section_header(ui, "★", "인스턴스", "MC 버전 + Fabric 조합");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::accent_button(ui, "＋ 새 인스턴스").clicked() {
                app.show_new_instance = true;
            }
        });
    });
    ui.add_space(8.0);

    let mut to_delete: Option<String> = None;
    for inst in app.instances.clone() {
        let selected = app.config.selected_instance.as_deref() == Some(&inst.id);
        let frame = if selected {
            theme::glow_card_frame()
        } else {
            theme::card_frame()
        };
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // 상태 점
                ui.label(
                    egui::RichText::new(if selected { "★" } else { "☆" })
                        .size(22.0)
                        .color(if selected {
                            theme::NEBULA_LIGHT
                        } else {
                            theme::TEXT_FAINT
                        }),
                );
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new(&inst.name).size(17.0).strong());
                        if selected {
                            theme::badge(ui, "선택됨", theme::NEBULA_LIGHT);
                        }
                    });
                    ui.horizontal(|ui| {
                        theme::badge(ui, &inst.display_version(), theme::STAR_BLUE);
                        theme::badge(ui, "Fabric", theme::NEBULA_LIGHT);
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "🚀 {}회 항해{}",
                            inst.total_plays,
                            inst.last_played
                                .map(|t| format!(" · {}", t.format("%Y-%m-%d %H:%M")))
                                .unwrap_or_default()
                        ))
                        .size(12.0)
                        .color(theme::TEXT_DIM),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        if theme::accent_button(ui, "▶ 실행").clicked() {
                            app.config.selected_instance = Some(inst.id.clone());
                            app.launch();
                        }
                        ui.horizontal(|ui| {
                            if !selected && theme::ghost_button(ui, "선택").clicked() {
                                app.config.selected_instance = Some(inst.id.clone());
                            }
                            if theme::danger_button(ui, "삭제").clicked() {
                                to_delete = Some(inst.id.clone());
                            }
                        });
                    });
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
