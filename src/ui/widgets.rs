pub fn placeholder(ui: &mut egui::Ui, title: &str, desc: &str) {
    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.heading(title);
        ui.label(desc);
    });
}
