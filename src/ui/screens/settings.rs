use crate::ui::theme;
use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    theme::section_header(ui, "●", "설정", "일반 설정");
    ui.add_space(8.0);

    // 계정 관리는 계정 탭에서 (인스턴스별 버전·로더 설정과 분리)
    game_card(app, ui);
    ui.add_space(8.0);
    link_card(app, ui);
    ui.add_space(8.0);
    advanced_card(app, ui);

    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        if crate::ui::theme::accent_button(ui, "설정 저장").clicked() {
            let save_result: anyhow::Result<()> = (|| {
                app.config.save()?;
                crate::core::instance::save_instances(&app.config.game_root, &app.instances)?;
                Ok(())
            })();
            match save_result {
                Ok(()) => {
                    app.sync_discord();
                    app.refresh_java_version();
                    app.status = "설정 저장됨".to_string();
                }
                Err(e) => app.status = format!("저장 실패: {e:#}"),
            }
        }
    });
}

use crate::ui::theme::form_row;

// ---- 1. 게임 ----
fn game_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "●", "게임", "Java · 메모리 · 화면");
        form_row(ui, "Java 경로", |ui| {
            ui.text_edit_singleline(&mut app.config.java_path);
            if ui.small_button("찾기").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    app.config.java_path = path.to_string_lossy().to_string();
                }
            }
            if ui.small_button("자동 설치").clicked() {
                app.install_java_now();
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(130.0);
            ui.label(
                egui::RichText::new(if app.java_install_status.is_empty() {
                    "비우면 자동탐지 · 자동 설치는 Adoptium JRE 21"
                } else {
                    &app.java_install_status
                })
                .color(theme::TEXT_FAINT)
                .size(11.0),
            );
        });
        ui.add_space(6.0);
        form_row(ui, "최대 RAM", |ui| {
            ui.add(
                egui::Slider::new(&mut app.config.ram_mb, 1024..=16384)
                    .step_by(256.0)
                    .suffix(" MB"),
            );
        });
        form_row(ui, "최소 RAM", |ui| {
            ui.add(
                egui::Slider::new(&mut app.config.min_ram_mb, 512..=4096)
                    .step_by(256.0)
                    .suffix(" MB"),
            );
        });
        form_row(ui, "해상도", |ui| {
            ui.add(egui::DragValue::new(&mut app.config.width).range(640..=3840));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut app.config.height).range(480..=2160));
        });
        ui.add_space(4.0);
        ui.checkbox(&mut app.config.hide_on_launch, "실행 시 런처 숨기기");
    });
}

// ---- 2. 연동 (모드 제공자 + Discord) ----
fn link_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "◆", "연동", "모드 · Discord");
        ui.label(
            egui::RichText::new("Modrinth는 키 없이 사용 가능. CurseForge는 API 키가 필요합니다.")
                .color(theme::TEXT_DIM)
                .size(12.0),
        );
        ui.add_space(2.0);
        form_row(ui, "CurseForge API 키", |ui| {
            ui.text_edit_singleline(&mut app.config.curseforge_api_key);
        });
        ui.add_space(4.0);
        form_row(ui, "Discord 상태", |ui| {
            ui.checkbox(&mut app.config.discord_enabled, "표시");
            let (dot, txt) =
                if !app.config.discord_enabled || app.config.discord_client_id.trim().is_empty() {
                    ("○", "꺼짐")
                } else if app.discord.is_connected() {
                    ("●", "활동 표시 중")
                } else {
                    ("●", "연결 대기 중")
                };
            theme::badge(
                ui,
                &format!("{dot} {txt}"),
                if app.discord.is_connected() {
                    theme::SUCCESS
                } else {
                    theme::TEXT_DIM
                },
            );
            if ui.small_button("다시 연결").clicked() {
                app.sync_discord();
                app.status = "Discord 재연결 시도".to_string();
            }
        });
        form_row(ui, "Discord App ID", |ui| {
            ui.text_edit_singleline(&mut app.config.discord_client_id);
        });
    });
}

// ---- 3. 고급 ----
fn advanced_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "≡", "고급", "버전 · 저장 위치");
        form_row(ui, "현재 버전", |ui| {
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            if ui.small_button("업데이트 확인").clicked() {
                app.check_update_now();
            }
            if let Some(info) = app.update_info.clone() {
                if theme::accent_button(ui, &format!("v{}로 업데이트", info.version)).clicked()
                {
                    app.show_update_dialog = true;
                }
            }
        });
        if !app.update_status.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(130.0);
                ui.label(
                    egui::RichText::new(&app.update_status)
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
            });
        }
        ui.add_space(2.0);
        form_row(ui, "저장 위치", |ui| {
            ui.monospace(format!("{}", app.config.game_root.display()));
            if ui.small_button("폴더 열기").clicked() {
                let _ = open::that(&app.config.game_root);
            }
        });
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("메타데이터: daedalus (Modrinth, MIT) · 실행부는 설계만 참조")
                .color(theme::TEXT_FAINT)
                .size(11.0),
        );
    });
}
