use crate::ui::theme;
use crate::ui::NebulyaApp;

/// 첫 실행 환영 화면 (Modrinth식: 메인 화면을 먼저 보여주고 로그인은 선택).
/// MS 로그인은 심사 승인 후 복구 예정이라 당분간 숨긴다.
pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    // 화면 전체에 성운 배경을 깔고 시작
    let full = ui.available_rect_before_wrap();
    theme::paint_backdrop(ui.ctx(), full);

    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
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
            egui::RichText::new("Rust 네이티브 Fabric 런처에 오신 것을 환영합니다")
                .size(15.0)
                .color(theme::STARLIGHT),
        );
        ui.add_space(18.0);

        if theme::accent_button(ui, "▶ 시작하기").clicked() {
            app.config.onboarding_done = true;
            app.persist();
        }
        ui.add_space(10.0);
        theme::star_divider(ui);
        ui.add_space(6.0);

        // Nebulya 계정으로 계속
        ui.label(
            egui::RichText::new("Nebulya 계정으로 계속")
                .strong()
                .size(13.0),
        );
        ui.horizontal(|ui| {
            ui.label("이메일");
            ui.add(egui::TextEdit::singleline(&mut app.nebula_email).desired_width(200.0));
        });
        ui.horizontal(|ui| {
            ui.label("비밀번호");
            ui.add(
                egui::TextEdit::singleline(&mut app.nebula_password)
                    .password(true)
                    .desired_width(200.0),
            );
        });
        ui.horizontal(|ui| {
            if theme::accent_button(ui, "Nebulya 로그인").clicked() {
                app.nebula_login();
            }
            if theme::ghost_button(ui, "계정 만들기").clicked() {
                app.config.onboarding_done = true;
                app.tab = crate::ui::Tab::Account;
                app.nebula_mode_signup = true;
                app.persist();
            }
        });
        if !app.nebula_status.is_empty() {
            ui.label(
                egui::RichText::new(&app.nebula_status)
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
        }
        ui.add_space(20.0);
    });
}
