use crate::minecraft::auth::LoginState;
use crate::ui::theme;
use crate::ui::NebulyaApp;

/// 계정 전용 페이지 (Modrinth식 Accounts 화면).
/// Microsoft 정품 로그인/로그아웃/토큰 갱신을 한 곳에서 관리한다.
/// 인스턴스별 게임 버전·Fabric 로더는 인스턴스 탭에서 따로 관리된다.
pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    theme::section_header(ui, "👤", "계정", "Microsoft 정품 로그인");
    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        if let Some(acc) = app.config.account.clone() {
            account_info(app, ui, &acc);
        } else {
            login_flow(app, ui);
        }
    });

    ui.add_space(8.0);

    nebula_card(app, ui);

    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        ui.label(egui::RichText::new("로그인 방식").size(13.0).strong());
        ui.label(
            egui::RichText::new("Microsoft Device Code → Xbox → Minecraft 순서로 인증합니다.")
                .color(theme::TEXT_DIM)
                .size(12.0),
        );
        ui.label(
            egui::RichText::new("게임 버전과 Fabric 로더는 인스턴스마다 따로 설정됩니다.")
                .color(theme::TEXT_DIM)
                .size(12.0),
        );
    });
}

fn account_info(
    app: &mut NebulyaApp,
    ui: &mut egui::Ui,
    acc: &crate::minecraft::auth::StoredAccount,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("★ {}", acc.username))
                .size(18.0)
                .strong(),
        );
        theme::badge(ui, "정품", theme::SUCCESS);
    });
    ui.monospace(format!("UUID {}", acc.uuid));
    ui.add_space(6.0);
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
                    Ok(rt) => {
                        rt.block_on(crate::minecraft::auth::refresh_account(&http, &cid, &acc2))
                    }
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
}

fn login_flow(app: &mut NebulyaApp, ui: &mut egui::Ui) {
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
                ui.label(egui::RichText::new("브라우저에서 아래 코드를 입력하세요").strong());
                let mut code = user_code.clone();
                ui.add(
                    egui::TextEdit::singleline(&mut code)
                        .desired_width(220.0)
                        .font(egui::TextStyle::Heading)
                        .horizontal_align(egui::Align::Center),
                );
                let left = *app.login_wait_secs.lock().unwrap();
                if left > 0 {
                    ui.label(
                        egui::RichText::new(format!("남은 시간: {left}초"))
                            .color(theme::TEXT_DIM)
                            .size(12.0),
                    );
                }
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

/// Nebulya 자체 계정 (서버 세션 방식). MS 로그인과 별개로 사용한다.
fn nebula_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        theme::section_header(ui, "★", "Nebulya 계정", "런처 자체 로그인");
        if let Some(acc) = app.config.nebula_account.clone() {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("★ {}", acc.username))
                        .size(16.0)
                        .strong(),
                );
                theme::badge(ui, "Nebulya", theme::STAR_BLUE);
            });
            ui.monospace(&acc.email);
            ui.add_space(4.0);
            if theme::danger_button(ui, "로그아웃").clicked() {
                let http = app.http.clone();
                let token = acc.token.clone();
                let rt_handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build();
                    match rt {
                        Ok(rt) => rt.block_on(crate::minecraft::nebula_auth::logout(&http, &token)),
                        Err(e) => Err(anyhow::anyhow!("{e}")),
                    }
                });
                match rt_handle.join() {
                    Ok(Ok(())) => app.nebula_status = "로그아웃됨".to_string(),
                    Ok(Err(e)) => {
                        app.nebula_status = format!("서버 로그아웃 실패(로컬에서 제거): {e:#}")
                    }
                    Err(_) => app.nebula_status = "스레드 오류".to_string(),
                }
                app.config.nebula_account = None;
                app.persist();
            }
        } else {
            ui.horizontal(|ui| {
                ui.label("이메일");
                ui.text_edit_singleline(&mut app.nebula_email);
            });
            ui.horizontal(|ui| {
                ui.label("비밀번호");
                ui.add(egui::TextEdit::singleline(&mut app.nebula_password).password(true));
            });
            if app.nebula_pending_verify {
                ui.horizontal(|ui| {
                    ui.label("인증 코드");
                    ui.text_edit_singleline(&mut app.nebula_code);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("닉네임 (가입 시)");
                    ui.text_edit_singleline(&mut app.nebula_username);
                });
            }
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if theme::accent_button(ui, "Nebulya 로그인").clicked() {
                    app.nebula_login();
                }
                if app.nebula_pending_verify {
                    if theme::accent_button(ui, "인증 확인").clicked() {
                        app.nebula_verify();
                    }
                } else if theme::ghost_button(ui, "가입하기").clicked() {
                    app.nebula_signup();
                }
            });
        }
        if !app.nebula_status.is_empty() {
            ui.label(
                egui::RichText::new(&app.nebula_status)
                    .color(theme::TEXT_DIM)
                    .size(12.0),
            );
        }
    });
}
