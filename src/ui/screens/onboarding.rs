use crate::minecraft::auth::LoginState;
use crate::ui::theme;
use crate::ui::NebulyaApp;

/// 첫 실행 온보딩: Microsoft 정품 로그인을 먼저 요구하는 게이트 화면.
/// 정품 로그인에 성공하면 자동으로 닫히고, 오프라인으로 시작할 수도 있다.
pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    // 화면 전체에 성운 배경을 깔고 시작
    let full = ui.available_rect_before_wrap();
    theme::paint_backdrop(ui.ctx(), full);

    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        ui.label(
            egui::RichText::new("★")
                .size(60.0)
                .color(theme::NEBULA_LIGHT),
        );
        ui.label(
            egui::RichText::new("NEBULYA")
                .size(46.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.label(
            egui::RichText::new("S T E L L A R · F A B R I C")
                .size(11.0)
                .color(theme::NEBULA_LIGHT),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("시작하려면 Microsoft 계정으로 로그인하세요")
                .size(15.0)
                .color(theme::STARLIGHT),
        );
        ui.add_space(16.0);

        theme::glow_card_frame().show(ui, |ui| {
            ui.set_min_size(egui::vec2(400.0, 0.0));
            ui.set_max_width(400.0);
            ui.vertical_centered(|ui| {
                let snapshot = app.login_state.lock().unwrap().clone();
                match snapshot {
                    LoginState::Idle => {
                        ui.add_space(4.0);
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
                        ui.add_space(4.0);
                    }
                    LoginState::Code { user_code, uri } => {
                        ui.label(
                            egui::RichText::new("브라우저에서 아래 코드를 입력하세요").strong(),
                        );
                        ui.add_space(4.0);
                        ui.heading(
                            egui::RichText::new(&user_code)
                                .size(36.0)
                                .strong()
                                .color(theme::NEBULA_LIGHT),
                        );
                        ui.monospace(&uri);
                        ui.add_space(8.0);
                        if theme::accent_button(ui, "브라우저 열기").clicked() {
                            let _ = open::that(&uri);
                        }
                        ui.add_space(4.0);
                        if theme::ghost_button(ui, "취소").clicked() {
                            app.cancel_ms_login();
                        }
                    }
                    LoginState::Working(msg) => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(&msg);
                        });
                        ui.add_space(8.0);
                        if theme::ghost_button(ui, "취소").clicked() {
                            app.cancel_ms_login();
                        }
                    }
                    LoginState::Done(_) | LoginState::Failed(_) => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("확인 중...");
                        });
                        ui.add_space(4.0);
                    }
                }
            });
        });

        ui.add_space(8.0);

        // 고급: MS Client ID 변경 (비워두면 내장 ID 사용)
        egui::CollapsingHeader::new("고급: MS Client ID 변경").show(ui, |ui| {
            ui.set_max_width(400.0);
            ui.horizontal(|ui| {
                ui.label("MS Client ID");
                ui.add(
                    egui::TextEdit::singleline(&mut app.config.ms_client_id).desired_width(240.0),
                );
            });
            ui.label(
                egui::RichText::new("비워두면 내장된 공용 ID로 로그인합니다")
                    .color(theme::TEXT_DIM)
                    .size(11.0),
            );
        });

        ui.add_space(10.0);
        theme::star_divider(ui);
        ui.add_space(6.0);

        // 정품 전용: 오프라인 시작 없음
        ui.label(
            egui::RichText::new("정품 Minecraft: Java Edition이 필요합니다")
                .color(theme::TEXT_FAINT)
                .size(11.0),
        );
        ui.label(
            egui::RichText::new("정품 Minecraft: Java Edition이 필요합니다")
                .color(theme::TEXT_FAINT)
                .size(11.0),
        );
        ui.add_space(20.0);
        ui.add_space(20.0);
    });
}
