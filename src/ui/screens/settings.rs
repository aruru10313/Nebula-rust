use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.heading("설정");
    ui.add_space(8.0);

    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading("계정 (오프라인)");
        ui.label(
            egui::RichText::new("Phase 1은 오프라인 모드. MS 로그인은 Client ID 발급 후 활성화됩니다.")
                .color(crate::ui::theme::TEXT_DIM)
                .size(12.0),
        );
        ui.horizontal(|ui| {
            ui.label("닉네임");
            if ui.text_edit_singleline(&mut app.config.username).changed() {
                if app.config.username.trim().is_empty() {
                    app.config.username = "Player".to_string();
                }
            }
        });
        if ui.small_button("Microsoft 로그인 (준비중)").clicked() {
            app.status = "MS 로그인은 Phase 2에서 활성화됩니다".to_string();
        }
    });

    ui.add_space(8.0);

    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading("Java & 성능");
        ui.horizontal(|ui| {
            ui.label("Java 경로 (비우면 자동탐지)");
            ui.text_edit_singleline(&mut app.config.java_path);
            if ui.small_button("찾기").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    app.config.java_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.label(format!("현재: {}", app.config.java_path));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("최대 RAM (MB)");
            ui.add(egui::Slider::new(&mut app.config.ram_mb, 1024..=16384).step_by(256.0));
        });
        ui.horizontal(|ui| {
            ui.label("최소 RAM (MB)");
            ui.add(egui::Slider::new(&mut app.config.min_ram_mb, 512..=4096).step_by(256.0));
        });
        ui.horizontal(|ui| {
            ui.label("해상도");
            ui.add(egui::DragValue::new(&mut app.config.width).range(640..=3840));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut app.config.height).range(480..=2160));
        });
        ui.checkbox(&mut app.config.hide_on_launch, "실행 시 런처 숨기기");
    });

    ui.add_space(8.0);

    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading("모드 제공자");
        ui.label(
            egui::RichText::new("Modrinth는 키 없이 사용 가능. CurseForge는 API 키가 필요합니다.")
                .color(crate::ui::theme::TEXT_DIM)
                .size(12.0),
        );
        ui.horizontal(|ui| {
            ui.label("CurseForge API 키");
            ui.text_edit_singleline(&mut app.config.curseforge_api_key);
        });
        ui.label(
            egui::RichText::new("발급: console.curseforge.com → API Keys / 또는 환경변수 NEBULYA_CF_API_KEY")
                .color(crate::ui::theme::TEXT_DIM)
                .size(11.0),
        );
    });

    ui.add_space(8.0);

    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading("Discord Activity");
        ui.checkbox(&mut app.config.discord_enabled, "디스코드에 상태 표시");
        ui.horizontal(|ui| {
            ui.label("Application ID");
            ui.text_edit_singleline(&mut app.config.discord_client_id);
        });
        ui.label(
            egui::RichText::new("discord.com/developers/applications 에서 앱 생성 후 Client ID 입력. 디스코드가 꺼져 있으면 자동으로 비활성화됩니다.")
                .color(crate::ui::theme::TEXT_DIM)
                .size(11.0),
        );
        if ui.small_button("지금 다시 연결").clicked() {
            app.sync_discord();
            app.status = "Discord 재연결 시도".to_string();
        }
    });

    ui.add_space(8.0);

    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading("저장 위치");
        ui.monospace(format!("{}", app.config.game_root.display()));
        if ui.small_button("폴더 열기").clicked() {
            let _ = open::that(&app.config.game_root);
        }
    });

    ui.add_space(12.0);
    if crate::ui::theme::accent_button(ui, "💾  설정 저장").clicked() {
        // persist는 on_exit에서도 되지만 즉시 저장
        let save_result: anyhow::Result<()> = (|| {
            app.config.save()?;
            crate::core::instance::save_instances(&app.config.game_root, &app.instances)?;
            Ok(())
        })();
        match save_result {
            Ok(()) => {
                app.sync_discord();
                app.status = "설정 저장됨".to_string();
            }
            Err(e) => app.status = format!("저장 실패: {e:#}"),
        }
    }
}
