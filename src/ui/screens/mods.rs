//! 모드 화면: Modrinth / CurseForge 검색 + 설치 + 관리 (Fabric 전용)
//!
//! - 검색 결과는 선택된 인스턴스의 MC 버전과 Fabric 로더 기준으로 필터됨
//! - 설치된 모드는 `instances/<id>/mods/*.jar` 가 실제 기준,
//!   `instance.mods` 메타는 출처/버전 표시용으로 함께 유지

use crate::core::{InstalledMod, ModSource};
use crate::minecraft::{curseforge, modrinth};
use crate::ui::NebulyaApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModProvider {
    #[default]
    Modrinth,
    CurseForge,
}

impl ModProvider {
    fn label(self) -> &'static str {
        match self {
            ModProvider::Modrinth => "Modrinth",
            ModProvider::CurseForge => "CurseForge",
        }
    }
}

#[derive(Debug, Default)]
pub struct ModsUiState {
    pub provider: ModProvider,
    pub query: String,
    pub mr_results: Vec<modrinth::ProjectHit>,
    pub cf_results: Vec<curseforge::CfMod>,
    pub searching: bool,
    pub installing: Option<String>,
    pub message: String,
}

/// 백그라운드 모드 작업 결과 (검색·설치)
pub enum ModTaskOutcome {
    SearchDone {
        provider: ModProvider,
        mr: Vec<modrinth::ProjectHit>,
        cf: Vec<curseforge::CfMod>,
        message: String,
    },
    SearchFailed(String),
    InstallDone {
        instance_id: String,
        installed: InstalledMod,
        message: String,
        log: String,
    },
    InstallFailed(String),
}

fn provider_label(p: ModProvider) -> &'static str {
    match p {
        ModProvider::Modrinth => "Modrinth · 키 불필요",
        ModProvider::CurseForge => "CurseForge · API 키 필요",
    }
}

fn format_downloads(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        format!("{n:.0}")
    }
}

impl NebulyaApp {
    /// 현재 입력된 검색어로 모드 검색 (선택 인스턴스 MC버전 기준 Fabric 필터)
    pub fn search_mods_now(&mut self) {
        let mc = self
            .selected_instance()
            .map(|i| i.minecraft_version.clone())
            .unwrap_or_default();
        let query = self.mods_ui.query.trim().to_string();
        if query.is_empty() {
            self.mods_ui.message = "검색어를 입력하세요.".to_string();
            return;
        }
        let provider = self.mods_ui.provider;
        if self.mod_task_rx.is_some() {
            self.mods_ui.message = "이미 작업 중입니다. 잠시 기다리세요.".to_string();
            return;
        }
        self.mods_ui.searching = true;
        self.mods_ui.message = format!("{}에서 \"{query}\" 검색 중...", provider.label());
        self.discord.show_search(provider.label(), &query);

        let http = self.http.clone();
        let key = self.config.curseforge_key();
        let (tx, rx) = std::sync::mpsc::channel();
        self.mod_task_rx = Some(rx);
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(ModTaskOutcome::SearchFailed(format!("런타임 오류: {e}")));
                    return;
                }
            };
            match provider {
                ModProvider::Modrinth => {
                    match rt.block_on(modrinth::search_projects(
                        &http,
                        &query,
                        Some(mc.as_str()),
                        20,
                    )) {
                        Ok(hits) => {
                            let n = hits.len();
                            let _ = tx.send(ModTaskOutcome::SearchDone {
                                provider,
                                mr: hits,
                                cf: vec![],
                                message: format!("Modrinth 결과 {n}개 (Fabric 전용)"),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(ModTaskOutcome::SearchFailed(format!(
                                "Modrinth 검색 실패: {e:#}"
                            )));
                        }
                    }
                }
                ModProvider::CurseForge => {
                    match rt.block_on(curseforge::search_mods(
                        &http,
                        &key,
                        &query,
                        Some(mc.as_str()),
                        20,
                    )) {
                        Ok(list) => {
                            let n = list.len();
                            let _ = tx.send(ModTaskOutcome::SearchDone {
                                provider,
                                mr: vec![],
                                cf: list,
                                message: format!("CurseForge 결과 {n}개 (Fabric 전용)"),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(ModTaskOutcome::SearchFailed(format!(
                                "CurseForge 검색 실패: {e:#}"
                            )));
                        }
                    }
                }
            }
        });
    }

    /// Modrinth 설치 (최신 Fabric 호환 버전, 백그라운드)
    pub fn install_modrinth(&mut self, slug: &str, title: &str) {
        let Some(inst) = self.selected_instance().cloned() else {
            self.mods_ui.message = "인스턴스를 먼저 선택하세요.".into();
            return;
        };
        if self.mod_task_rx.is_some() {
            self.mods_ui.message = "이미 작업 중입니다. 잠시 기다리세요.".into();
            return;
        }
        let mc = inst.minecraft_version.clone();
        let mods_dir = inst.mods_dir(&self.config.game_root);
        let instance_id = inst.id.clone();
        let slug = slug.to_string();
        let title = title.to_string();
        self.mods_ui.installing = Some(title.clone());
        let http = self.http.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.mod_task_rx = Some(rx);
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(ModTaskOutcome::InstallFailed(format!("런타임 오류: {e}")));
                    return;
                }
            };
            match rt.block_on(modrinth::install_latest(&http, &slug, &mc, &mods_dir)) {
                Ok((ver, path)) => {
                    let file_name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let _ = tx.send(ModTaskOutcome::InstallDone {
                        instance_id,
                        installed: InstalledMod {
                            source: ModSource::Modrinth,
                            project_id: slug.clone(),
                            title: title.clone(),
                            file_name: file_name.clone(),
                            version: ver.version_number.clone(),
                            enabled: true,
                        },
                        message: format!("설치됨: {file_name}"),
                        log: format!("모드 설치(Modrinth): {title} {file_name}"),
                    });
                }
                Err(e) => {
                    let _ = tx.send(ModTaskOutcome::InstallFailed(format!("설치 실패: {e:#}")));
                }
            }
        });
    }

    /// CurseForge 설치 (최신 Fabric 호환 파일)
    pub fn install_curseforge(&mut self, mod_id: u32, name: &str) {
        let Some(inst) = self.selected_instance().cloned() else {
            self.mods_ui.message = "인스턴스를 먼저 선택하세요.".into();
            return;
        };
        let key = self.config.curseforge_key();
        if key.is_empty() {
            self.mods_ui.message = "CurseForge API 키가 없습니다. 설정 탭에서 입력하세요.".into();
            return;
        }
        if self.mod_task_rx.is_some() {
            self.mods_ui.message = "이미 작업 중입니다. 잠시 기다리세요.".into();
            return;
        }
        let mc = inst.minecraft_version.clone();
        let mods_dir = inst.mods_dir(&self.config.game_root);
        let instance_id = inst.id.clone();
        let name = name.to_string();
        self.mods_ui.installing = Some(name.clone());
        let http = self.http.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.mod_task_rx = Some(rx);
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(ModTaskOutcome::InstallFailed(format!("런타임 오류: {e}")));
                    return;
                }
            };
            match rt.block_on(curseforge::install_latest(
                &http, &key, mod_id, &mc, &mods_dir,
            )) {
                Ok((file, path)) => {
                    let file_name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let _ = tx.send(ModTaskOutcome::InstallDone {
                        instance_id,
                        installed: InstalledMod {
                            source: ModSource::CurseForge,
                            project_id: mod_id.to_string(),
                            title: name.clone(),
                            file_name: file_name.clone(),
                            version: file.display_name.clone(),
                            enabled: true,
                        },
                        message: format!("설치됨: {file_name}"),
                        log: format!("모드 설치(CurseForge): {name} {file_name}"),
                    });
                }
                Err(e) => {
                    let _ = tx.send(ModTaskOutcome::InstallFailed(format!("설치 실패: {e:#}")));
                }
            }
        });
    }

    fn upsert_installed_mod(&mut self, instance_id: &str, m: InstalledMod) {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == instance_id) {
            inst.mods
                .retain(|e| !(e.source == m.source && e.project_id == m.project_id));
            inst.mods.push(m);
        }
    }

    /// 백그라운드 모드 작업 결과 반영 (매 프레임)
    pub fn poll_mod_tasks(&mut self) {
        let outcome = match &self.mod_task_rx {
            Some(rx) => match rx.try_recv() {
                Ok(o) => Some(o),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(ModTaskOutcome::SearchFailed("작업 스레드 종료".into()))
                }
            },
            None => None,
        };
        let Some(outcome) = outcome else {
            return;
        };
        self.mod_task_rx = None;
        self.mods_ui.searching = false;
        self.mods_ui.installing = None;
        match outcome {
            ModTaskOutcome::SearchDone {
                provider,
                mr,
                cf,
                message,
            } => {
                match provider {
                    ModProvider::Modrinth => self.mods_ui.mr_results = mr,
                    ModProvider::CurseForge => self.mods_ui.cf_results = cf,
                }
                self.mods_ui.message = message;
            }
            ModTaskOutcome::SearchFailed(e) => {
                self.mods_ui.message = e.clone();
                self.log(format!("모드 작업 실패: {e}"));
            }
            ModTaskOutcome::InstallDone {
                instance_id,
                installed,
                message,
                log,
            } => {
                self.upsert_installed_mod(&instance_id, installed);
                self.invalidate_mod_files();
                self.mods_ui.message = message;
                self.log(log);
                self.persist();
            }
            ModTaskOutcome::InstallFailed(e) => {
                self.mods_ui.message = e.clone();
                self.log(format!("모드 설치 실패: {e}"));
            }
        }
    }

    /// 활성화/비활성화 토글
    pub fn toggle_installed_mod(&mut self, file_name: &str) {
        let Some(inst) = self.selected_instance().cloned() else {
            return;
        };
        let mods_dir = inst.mods_dir(&self.config.game_root);
        match crate::minecraft::mods::toggle_mod_file(&mods_dir, file_name) {
            Ok(new_name) => {
                if let Some(e) = self.instances.iter_mut().find(|i| i.id == inst.id) {
                    for m in e.mods.iter_mut() {
                        if m.file_name == file_name {
                            m.file_name = new_name.clone();
                            // ".disabled"를 뗐으면 켜진 것, 붙였으면 꺼진 것
                            m.enabled = file_name.ends_with(".disabled");
                        }
                    }
                }
                self.mods_ui.message = format!("변경됨: {new_name}");
                self.invalidate_mod_files();
                self.persist();
            }
            Err(e) => self.mods_ui.message = format!("토글 실패: {e:#}"),
        }
    }

    /// 모드 삭제 (파일 + 메타)
    pub fn delete_installed_mod(&mut self, file_name: &str) {
        let Some(inst) = self.selected_instance().cloned() else {
            return;
        };
        let mods_dir = inst.mods_dir(&self.config.game_root);
        if let Err(e) = crate::minecraft::mods::delete_mod_file(&mods_dir, file_name) {
            self.mods_ui.message = format!("삭제 실패: {e:#}");
            return;
        }
        if let Some(e) = self.instances.iter_mut().find(|i| i.id == inst.id) {
            e.mods.retain(|m| m.file_name != file_name);
        }
        self.mods_ui.message = format!("삭제됨: {file_name}");
        self.log(format!("모드 삭제: {file_name}"));
        self.invalidate_mod_files();
        self.persist();
    }
}

pub fn show(app: &mut NebulyaApp, ui: &mut egui::Ui) {
    use crate::ui::theme;
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        theme::section_header(ui, "◆", "모드", "Modrinth · CurseForge");
        theme::badge(ui, "Fabric 전용", theme::NEBULA_LIGHT);
    });
    let mc = app
        .selected_instance()
        .map(|i| {
            format!(
                "{} 기준: {} ({})",
                i.name,
                i.minecraft_version,
                i.display_version()
            )
        })
        .unwrap_or_else(|| "인스턴스 없음".into());
    ui.label(egui::RichText::new(mc).color(crate::ui::theme::TEXT_DIM));
    ui.add_space(6.0);

    // 제공자 선택 + 검색창 (좁은 창에서도 찌그러지지 않게 2행)
    ui.horizontal_wrapped(|ui| {
        for p in [ModProvider::Modrinth, ModProvider::CurseForge] {
            let selected = app.mods_ui.provider == p;
            if ui.selectable_label(selected, p.label()).clicked() {
                app.mods_ui.provider = p;
            }
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let resp = ui.add_sized(
            [ui.available_width() - 100.0, 24.0],
            egui::TextEdit::singleline(&mut app.mods_ui.query).hint_text("모드 검색..."),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            app.search_mods_now();
        }
        let searching = app.mods_ui.searching;
        ui.add_enabled_ui(!searching, |ui| {
            if ui
                .button(if searching {
                    "검색 중..."
                } else {
                    "🔍 검색"
                })
                .clicked()
            {
                app.search_mods_now();
            }
        });
    });

    if app.mods_ui.provider == ModProvider::CurseForge && app.config.curseforge_key().is_empty() {
        ui.label(
            egui::RichText::new(
                "⚠ CurseForge API 키가 없습니다. 설정 탭에서 입력하면 검색이 활성화됩니다.",
            )
            .color(crate::ui::theme::TEXT_DIM),
        );
    }

    if !app.mods_ui.message.is_empty() {
        ui.label(
            egui::RichText::new(&app.mods_ui.message)
                .size(12.0)
                .color(crate::ui::theme::ACCENT_HOVER),
        );
    }
    ui.add_space(6.0);

    // 검색 결과
    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_header(ui, "🔭", "검색 결과", provider_label(app.mods_ui.provider));
        });
        ui.add_space(2.0);
        match app.mods_ui.provider {
            ModProvider::Modrinth => {
                if app.mods_ui.mr_results.is_empty() {
                    ui.label("검색 결과가 없습니다. 검색어를 입력하고 검색하세요.");
                } else {
                    let hits = app.mods_ui.mr_results.clone();
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .show(ui, |ui| {
                            for hit in hits {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&hit.title).strong());
                                        if !hit.description.is_empty() {
                                            ui.label(
                                                egui::RichText::new(
                                                    hit.description
                                                        .chars()
                                                        .take(90)
                                                        .collect::<String>(),
                                                )
                                                .size(12.0)
                                                .color(crate::ui::theme::TEXT_DIM),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "⬇ {}",
                                                format_downloads(hit.downloads as f64)
                                            ))
                                            .size(11.0)
                                            .color(crate::ui::theme::TEXT_DIM),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let installing = app.mods_ui.installing.is_some();
                                            let slug = hit.slug.clone();
                                            let title = hit.title.clone();
                                            if ui
                                                .add_enabled(!installing, egui::Button::new("설치"))
                                                .clicked()
                                            {
                                                app.install_modrinth(&slug, &title);
                                            }
                                        },
                                    );
                                });
                                ui.separator();
                            }
                        });
                }
            }
            ModProvider::CurseForge => {
                if app.mods_ui.cf_results.is_empty() {
                    ui.label("검색 결과가 없습니다. 검색어를 입력하고 검색하세요.");
                } else {
                    let list = app.mods_ui.cf_results.clone();
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .show(ui, |ui| {
                            for m in list {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&m.name).strong());
                                        if !m.summary.is_empty() {
                                            ui.label(
                                                egui::RichText::new(
                                                    m.summary.chars().take(90).collect::<String>(),
                                                )
                                                .size(12.0)
                                                .color(crate::ui::theme::TEXT_DIM),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "⬇ {}",
                                                format_downloads(m.download_count)
                                            ))
                                            .size(11.0)
                                            .color(crate::ui::theme::TEXT_DIM),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let installing = app.mods_ui.installing.is_some();
                                            let name = m.name.clone();
                                            let id = m.id;
                                            if ui
                                                .add_enabled(!installing, egui::Button::new("설치"))
                                                .clicked()
                                            {
                                                app.install_curseforge(id, &name);
                                            }
                                        },
                                    );
                                });
                                ui.separator();
                            }
                        });
                }
            }
        }
    });

    ui.add_space(8.0);

    // 설치된 모드
    crate::ui::theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_header(ui, "★", "설치된 모드", "클릭으로 켜기/끄기");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, "📂 폴더 열기").clicked() {
                    if let Some(inst) = app.selected_instance() {
                        let dir = inst.mods_dir(&app.config.game_root);
                        let _ = open::that(&dir);
                    }
                }
            });
        });
        ui.add_space(2.0);
        let files = match app.selected_instance().cloned() {
            Some(inst) => {
                let root = app.config.game_root.clone();
                app.mod_files_cached(&inst.id, &root)
            }
            None => vec![],
        };
        if files.is_empty() {
            ui.label("설치된 모드가 없습니다. 위에서 검색 후 설치하세요.");
        } else {
            // 메타 매칭용
            let metas = app
                .selected_instance()
                .map(|i| i.mods.clone())
                .unwrap_or_default();
            for (file_name, enabled) in files {
                let meta = metas.iter().find(|m| {
                    m.file_name == file_name
                        || m.file_name.trim_end_matches(".disabled")
                            == file_name.trim_end_matches(".disabled")
                });
                theme::tile_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(if enabled { "✅" } else { "⏸" }).size(16.0));
                        ui.vertical(|ui| {
                            ui.monospace(&file_name);
                            if let Some(m) = meta {
                                ui.horizontal(|ui| {
                                    theme::badge(ui, m.source.as_str(), theme::NEBULA_LIGHT);
                                    if !m.version.is_empty() {
                                        theme::badge(ui, &m.version, theme::STAR_BLUE);
                                    }
                                });
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let f = file_name.clone();
                            if theme::danger_button(ui, "삭제").clicked() {
                                app.delete_installed_mod(&f);
                            }
                            let f2 = file_name.clone();
                            if theme::ghost_button(ui, if enabled { "끄기" } else { "켜기" })
                                .clicked()
                            {
                                app.toggle_installed_mod(&f2);
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        }
    });
}
