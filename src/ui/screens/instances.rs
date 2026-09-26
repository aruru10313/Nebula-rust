use crate::core::Instance;
use crate::ui::theme;
use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        theme::section_header(ui, "★", "인스턴스", "MC 버전 + Fabric 조합");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::small_accent_button(ui, "＋ 새 인스턴스").clicked() {
                app.show_new_instance = true;
            }
        });
    });
    ui.add_space(8.0);

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
                    if ui.small_button("삭제").clicked() {
                        app.confirm_delete_instance = Some(inst.id.clone());
                    }
                    if ui.small_button("폴더").clicked() {
                        let _ = open::that(inst.game_dir(&app.config.game_root));
                    }
                    if ui.small_button("복제").clicked() {
                        app.duplicate_instance(&inst.id);
                    }
                    if ui.small_button("이름").clicked() {
                        app.rename_instance_id = Some(inst.id.clone());
                        app.rename_name = inst.name.clone();
                    }
                    if ui.small_button("▶ 실행").clicked() {
                        app.config.selected_instance = Some(inst.id.clone());
                        app.launch();
                    }
                    if !selected && ui.small_button("선택").clicked() {
                        app.config.selected_instance = Some(inst.id.clone());
                    }
                });
            });
        });
        ui.add_space(6.0);
    }

    if let Some(id) = app.confirm_delete_instance.clone() {
        let name = app
            .instances
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        egui::Window::new("인스턴스 삭제")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_width(320.0);
                ui.label(format!("'{name}' 인스턴스를 삭제할까요?"));
                ui.label(
                    egui::RichText::new("목록에서만 제거되며 게임 파일은 남습니다.")
                        .color(theme::TEXT_DIM)
                        .size(12.0),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if theme::danger_button(ui, "삭제").clicked() {
                        app.instances.retain(|i| i.id != id);
                        if app.config.selected_instance.as_deref() == Some(&id) {
                            app.config.selected_instance =
                                app.instances.first().map(|i| i.id.clone());
                        }
                        app.persist();
                        app.confirm_delete_instance = None;
                    }
                    if theme::ghost_button(ui, "취소").clicked() {
                        app.confirm_delete_instance = None;
                    }
                });
            });
    }

    if app.rename_instance_id.is_some() {
        egui::Window::new("인스턴스 이름 변경")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_width(320.0);
                ui.label("새 이름");
                ui.text_edit_singleline(&mut app.rename_name);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if theme::accent_button(ui, "저장").clicked() {
                        let name = app.rename_name.trim().to_string();
                        if let Some(id) = app.rename_instance_id.clone() {
                            if !name.is_empty() {
                                if let Some(inst) = app.instances.iter_mut().find(|i| i.id == id) {
                                    inst.name = name;
                                    app.persist();
                                    app.status = "인스턴스 이름 변경됨".to_string();
                                }
                            }
                        }
                        app.rename_instance_id = None;
                    }
                    if theme::ghost_button(ui, "취소").clicked() {
                        app.rename_instance_id = None;
                    }
                });
            });
    }
}
