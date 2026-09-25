use crate::minecraft::auth::LoginState;
use crate::ui::theme;
use crate::ui::NebulyaApp;

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    theme::section_header(ui, "⚙", "설정", "런처를 내 별자리처럼");
    ui.add_space(8.0);

    account_card(app, ui);
    ui.add_space(8.0);
    game_card(app, ui);
    ui.add_space(8.0);
    link_card(app, ui);
    ui.add_space(8.0);
    advanced_card(app, ui);

    ui.add_space(12.0);
    if crate::ui::theme::accent_button(ui, "설정 저장").clicked() {
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

// ---- 1. 계정 ----
fn account_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "★", "계정", "Microsoft 정품 로그인");
        if let Some(acc) = app.config.account.clone() {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("★ {}", acc.username))
                        .size(16.0)
                        .strong(),
                );
                theme::badge(ui, "정품", theme::SUCCESS);
            });
            ui.monospace(format!("UUID {}", acc.uuid));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if theme::ghost_button(ui, "토큰 갱신").clicked() {
                    let http = app.http.clone();
                    let cid = app.config.ms_client_id_resolved();
                    let acc2 = acc.clone();
                    let rt_handle = std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .build();
                        match rt {
                            Ok(rt) => rt.block_on(crate::minecraft::auth::refresh_account(
                                &http, &cid, &acc2,
                            )),
                            Err(e) => Err(anyhow::anyhow!("{e}")),
                        }
                    });
                    match rt_handle.join() {
                        Ok(Ok(new_acc)) => {
                            app.config.username = new_acc.username.clone();
                            app.config.account = Some(new_acc);
                            app.persist();
                            app.status = "정품 토큰 갱신됨".to_string();
                        }
                        Ok(Err(e)) => app.status = format!("갱신 실패: {e:#}"),
                        Err(_) => app.status = "스레드 오류".to_string(),
                    }
                }
                if theme::danger_button(ui, "로그아웃").clicked() {
                    app.config.account = None;
                    app.config.onboarding_done = false;
                    app.persist();
                    app.status = "로그아웃됨".to_string();
                }
            });
        } else {
            ui.label(
                egui::RichText::new("정품 로그인하면 모든 정품 서버에 접속할 수 있습니다.")
                    .color(theme::TEXT_DIM)
                    .size(12.0),
            );
            ui.add_space(2.0);
            let snapshot = app.login_state.lock().unwrap().clone();
            match snapshot {
                LoginState::Idle => {
                    if theme::accent_button(ui, "★ Microsoft 로그인").clicked() {
                        app.start_ms_login();
                    }
                }
                LoginState::Code { user_code, uri } => {
                    theme::tile_frame().show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("브라우저에서 아래 코드를 입력하세요").strong(),
                        );
                        ui.heading(
                            egui::RichText::new(&user_code)
                                .size(28.0)
                                .strong()
                                .color(theme::NEBULA_LIGHT),
                        );
                        ui.monospace(&uri);
                        ui.horizontal(|ui| {
                            if theme::accent_button(ui, "브라우저 열기").clicked() {
                                let _ = open::that(&uri);
                            }
                            if theme::ghost_button(ui, "취소").clicked() {
                                app.cancel_ms_login();
                            }
                        });
                    });
                }
                LoginState::Working(msg) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(msg);
                    });
                    if theme::ghost_button(ui, "취소").clicked() {
                        app.cancel_ms_login();
                    }
                }
                LoginState::Done(_) | LoginState::Failed(_) => {
                    ui.spinner();
                }
            }
        }
    });
}

// ---- 2. 게임 ----
fn game_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "☕", "게임", "Java · 메모리 · 화면");
        ui.horizontal(|ui| {
            ui.label("Java 경로 (비우면 자동탐지)");
            ui.text_edit_singleline(&mut app.config.java_path);
            if ui.small_button("찾기").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    app.config.java_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.label(
            egui::RichText::new(format!("현재: {}", app.config.java_path))
                .color(theme::TEXT_DIM)
                .size(11.0),
        );
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
}

// ---- 3. 연동 (모드 제공자 + Discord) ----
fn link_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "◆", "연동", "모드 · Discord");
        ui.label(
            egui::RichText::new("Modrinth는 키 없이 사용 가능. CurseForge는 API 키가 필요합니다.")
                .color(theme::TEXT_DIM)
                .size(12.0),
        );
        ui.horizontal(|ui| {
            ui.label("CurseForge API 키");
            ui.text_edit_singleline(&mut app.config.curseforge_api_key);
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut app.config.discord_enabled, "디스코드에 상태 표시");
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
        ui.horizontal(|ui| {
            ui.label("Discord Application ID");
            ui.text_edit_singleline(&mut app.config.discord_client_id);
        });
    });
}

// ---- 4. 고급 ----
fn advanced_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "≡", "고급", "ID · 저장 위치");
        egui::CollapsingHeader::new("MS Client ID 직접 입력").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("MS Client ID");
                ui.text_edit_singleline(&mut app.config.ms_client_id);
            });
            ui.label(
                egui::RichText::new(
                    "Azure Portal → 앱 등록 → 애플리케이션(클라이언트) ID / 또는 환경변수 NEBULYA_MS_CLIENT_ID",
                )
                .color(theme::TEXT_DIM)
                .size(11.0),
            );
            ui.label(
                egui::RichText::new(
                    "발급: console.curseforge.com → API Keys / 또는 환경변수 NEBULYA_CF_API_KEY",
                )
                .color(theme::TEXT_DIM)
                .size(11.0),
            );
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(format!("현재 버전: v{}", env!("CARGO_PKG_VERSION")));
            if ui.small_button("업데이트 확인").clicked() {
                app.check_update_now();
            }
            if let Some(info) = app.update_info.clone() {
                if theme::accent_button(ui, &format!("v{}로 업데이트", info.version)).clicked() {
                    app.apply_update();
                }
            }
        });
        if !app.update_status.is_empty() {
            ui.label(
                egui::RichText::new(&app.update_status)
                    .color(theme::TEXT_DIM)
                    .size(11.0),
            );
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.monospace(format!("{}", app.config.game_root.display()));
            if ui.small_button("폴더 열기").clicked() {
                let _ = open::that(&app.config.game_root);
            }
        });
    });
}
