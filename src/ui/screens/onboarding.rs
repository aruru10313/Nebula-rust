use crate::minecraft::auth::LoginState;
use crate::ui::theme;
use crate::ui::NebulyaApp;

/// 첫 실행 온보딩: Microsoft 정품 로그인을 먼저 요구하는 게이트 화면.
/// 정품 로그인에 성공하면 자동으로 닫히고, 오프라인으로 시작할 수도 있다.
pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(36.0);
        ui.label(
            egui::RichText::new("★")
                .size(54.0)
                .color(theme::NEBULA_LIGHT),
        );
        ui.label(
            egui::RichText::new("NEBULYA")
                .size(40.0)
                .strong()
                .color(theme::STARLIGHT),
        );
        ui.label(
            egui::RichText::new("S T E L L A R · F A B R I C")
                .size(11.0)
                .color(theme::TEXT_FAINT),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("시작하려면 Microsoft 계정으로 로그인하세요")
                .size(15.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(18.0);

        theme::glow_card_frame().show(ui, |ui| {
            ui.set_min_size(egui::vec2(420.0, 0.0));
            ui.vertical_centered(|ui| {
                let snapshot = app.login_state.lock().unwrap().clone();
                match snapshot {
                    LoginState::Idle => {
                        if theme::accent_button(ui, "★ Microsoft 로그인").clicked() {
                            app.start_ms_login();
                        }
                        if !app.status.is_empty() && app.status != "준비됨" {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(&app.status)
                                    .size(12.0)
                                    .color(theme::WARN),
                            );
                        }
                    }
                    LoginState::Code { user_code, uri } => {
                        ui.label(
                            egui::RichText::new("브라우저에서 아래 코드를 입력하세요").strong(),
                        );
                        ui.add_space(4.0);
                        ui.heading(
                            egui::RichText::new(&user_code)
                                .size(34.0)
                                .strong()
                                .color(theme::NEBULA_LIGHT),
                        );
                        ui.monospace(&uri);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if theme::accent_button(ui, "브라우저 열기").clicked() {
                                let _ = open::that(&uri);
                            }
                            if theme::ghost_button(ui, "취소").clicked() {
                                app.cancel_ms_login();
                            }
                        });
                    }
                    LoginState::Working(msg) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(&msg);
                        });
                        ui.add_space(4.0);
                        if theme::ghost_button(ui, "취소").clicked() {
                            app.cancel_ms_login();
                        }
                    }
                    LoginState::Done(_) | LoginState::Failed(_) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("확인 중...");
                        });
                    }
                }
            });
        });

        ui.add_space(10.0);

        // 고급: MS Client ID 직접 입력 (배포 빌드는 추후 기본값 내장 예정)
        egui::CollapsingHeader::new("고급: MS Client ID 직접 입력").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("MS Client ID");
                ui.text_edit_singleline(&mut app.config.ms_client_id);
            });
            ui.label(
                egui::RichText::new("Azure Portal → 앱 등록 → 애플리케이션(클라이언트) ID")
                    .color(theme::TEXT_DIM)
                    .size(11.0),
            );
        });

        ui.add_space(14.0);
        theme::star_divider(ui);
        ui.add_space(8.0);

        // 오프라인 시작
        ui.horizontal(|ui| {
            ui.label("닉네임");
            ui.text_edit_singleline(&mut app.config.username);
            if theme::ghost_button(ui, "오프라인으로 시작").clicked() {
                if app.config.username.trim().is_empty() {
                    app.config.username = "Player".to_string();
                }
                app.complete_onboarding_offline();
            }
        });
        ui.label(
            egui::RichText::new("오프라인 모드에서는 정품 서버에 접속할 수 없습니다")
                .color(theme::TEXT_FAINT)
                .size(11.0),
        );
    });
}
