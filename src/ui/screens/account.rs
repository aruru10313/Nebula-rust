use crate::ui::theme;
use crate::ui::NebulyaApp;

/// 계정 전용 페이지 (Modrinth식 Accounts 화면).
/// Microsoft 정품 로그인/로그아웃/토큰 갱신을 한 곳에서 관리한다.
/// 인스턴스별 게임 버전·Fabric 로더는 인스턴스 탭에서 따로 관리된다.
pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    theme::section_header(ui, "●", "계정", "Nebulya 로그인");
    ui.add_space(8.0);

    // MS 로그인은 심사 승인 후 복구 예정이라 당분간 Nebulya 계정만 노출.
    // 단, 기존 MS 계정이 있으면 관리(갱신/로그아웃) UI를 보여준다.
    if app.config.account.is_some() {
        theme::card_frame().show(ui, |ui| {
            theme::section_header(ui, "★", "Microsoft 계정", "정품");
            if let Some(acc) = app.config.account.clone() {
                account_info(app, ui, &acc);
            }
        });
        ui.add_space(8.0);
    }

    nebula_card(app, ui);

    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        ui.label(egui::RichText::new("로그인 방식").size(13.0).strong());
        ui.label(
            egui::RichText::new("Nebulya 계정으로 로그인하면 바로 플레이할 수 있습니다.")
                .color(theme::TEXT_DIM)
                .size(12.0),
        );
        ui.label(
            egui::RichText::new("Microsoft 정품 연동은 심사 승인 후 제공됩니다.")
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

/// Nebulya 자체 계정 (서버 세션 방식). MS 로그인과 별개로 사용한다.
fn nebula_card(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.set_max_width(520.0);
        nebula_card_inner(app, ui);
    });
}

/// Nebulya 자체 계정 (서버 세션 방식). MS 로그인과 별개로 사용한다.
fn nebula_card_inner(app: &mut NebulyaApp, ui: &mut egui::Ui) {
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
            // 로그인 / 가입하기 탭 전환
            ui.horizontal(|ui| {
                let login_tab = !app.nebula_mode_signup;
                if ui.selectable_label(login_tab, "로그인").clicked() {
                    app.nebula_mode_signup = false;
                    app.nebula_pending_verify = false;
                }
                if ui
                    .selectable_label(app.nebula_mode_signup, "가입하기")
                    .clicked()
                {
                    app.nebula_mode_signup = true;
                }
            });
            ui.add_space(2.0);
            theme::form_row(ui, "이메일", |ui| {
                ui.text_edit_singleline(&mut app.nebula_email);
            });
            theme::form_row(ui, "비밀번호", |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.nebula_password).password(true));
            });
            if app.nebula_mode_signup {
                if app.nebula_pending_verify {
                    theme::form_row(ui, "인증 코드", |ui| {
                        ui.text_edit_singleline(&mut app.nebula_code);
                    });
                    ui.horizontal(|ui| {
                        if theme::small_accent_button(ui, "인증 확인").clicked() {
                            app.nebula_verify();
                        }
                        if ui.small_button("코드 재전송").clicked() {
                            app.nebula_signup();
                        }
                    });
                } else {
                    theme::form_row(ui, "닉네임", |ui| {
                        ui.text_edit_singleline(&mut app.nebula_username);
                    });
                    ui.add_space(2.0);
                    if theme::accent_button(ui, "가입하고 코드 받기").clicked() {
                        app.nebula_signup();
                    }
                    ui.label(
                        egui::RichText::new("비밀번호 8자 이상 · 인증 코드는 이메일로 발송됩니다")
                            .color(theme::TEXT_FAINT)
                            .size(11.0),
                    );
                }
            } else if theme::accent_button(ui, "Nebulya 로그인").clicked() {
                app.nebula_login();
            }
            if !app.nebula_mode_signup {
                ui.label(
                    egui::RichText::new("계정이 없으면 상단 가입하기 탭에서 만드세요")
                        .color(theme::TEXT_FAINT)
                        .size(11.0),
                );
            }
        }
        if !app.nebula_status.is_empty() {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(&app.nebula_status)
                    .color(theme::status_color(&app.nebula_status))
                    .size(12.0),
            );
        }
    });
}
