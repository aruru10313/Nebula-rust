use crate::core::{Instance, LauncherConfig, instance};
use crate::minecraft::auth::MinecraftSession;
use crate::minecraft::discord::DiscordPresence;
use crate::minecraft::launcher::LaunchProgress;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Home,
    Instances,
    Mods,
    Settings,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "홈",
            Tab::Instances => "인스턴스",
            Tab::Mods => "모드",
            Tab::Settings => "설정",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Tab::Home => "▶",
            Tab::Instances => "▦",
            Tab::Mods => "◈",
            Tab::Settings => "⚙",
        }
    }
}

pub struct NebulyaApp {
    pub config: LauncherConfig,
    pub instances: Vec<Instance>,
    pub tab: Tab,
    pub versions: Vec<String>,
    pub loaders: Vec<String>,
    pub status: String,
    pub progress: Option<LaunchProgress>,
    pub launching: bool,
    pub logs: Vec<String>,
    pub new_instance_name: String,
    pub new_mc_version: String,
    pub new_loader_version: String,
    pub show_new_instance: bool,
    pub mods_ui: crate::ui::screens::mods::ModsUiState,
    pub discord: DiscordPresence,
    pub runtime: tokio::runtime::Runtime,
    pub http: reqwest::Client,
    progress_state: Arc<Mutex<Option<LaunchProgress>>>,
}

impl NebulyaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::theme::apply_theme(&cc.egui_ctx);
        let config = LauncherConfig::load();
        let instances = instance::load_instances(&config.game_root);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let http = reqwest::Client::builder()
            .user_agent("nebulya-launcher/0.1")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let discord = DiscordPresence::new(config.discord_enabled, &config.discord_client_id);

        let mut app = Self {
            config,
            instances,
            tab: Tab::Home,
            versions: vec!["1.20.1".into(), "1.20.4".into(), "1.21".into()],
            loaders: vec!["0.16.9".into()],
            status: "준비됨".to_string(),
            progress: None,
            launching: false,
            logs: vec!["Nebulya Launcher v0.2 초기화".to_string()],
            new_instance_name: "새 인스턴스".to_string(),
            new_mc_version: "1.20.1".to_string(),
            new_loader_version: "0.16.9".to_string(),
            show_new_instance: false,
            mods_ui: crate::ui::screens::mods::ModsUiState::default(),
            discord,
            runtime,
            http,
            progress_state: Arc::new(Mutex::new(None)),
        };
        app.refresh_remote_lists();
        app.persist();
        let user = app.config.username.clone();
        app.discord.show_home(&user);
        app
    }

    pub fn persist(&self) {
        let _ = self.config.save();
        let _ = instance::save_instances(&self.config.game_root, &self.instances);
    }

    /// 설정 변경 후 Discord 재연결 (설정 화면 저장 시 호출)
    pub fn sync_discord(&mut self) {
        self.discord
            .reconfigure(self.config.discord_enabled, &self.config.discord_client_id);
        let user = self.config.username.clone();
        self.discord.show_home(&user);
    }

    pub fn selected_instance(&self) -> Option<&Instance> {
        if let Some(id) = &self.config.selected_instance {
            if let Some(inst) = self.instances.iter().find(|i| &i.id == id) {
                return Some(inst);
            }
        }
        self.instances.first()
    }

    pub fn selected_instance_mut(&mut self) -> Option<&mut Instance> {
        if self.config.selected_instance.is_none() {
            if let Some(first) = self.instances.first() {
                self.config.selected_instance = Some(first.id.clone());
            }
        }
        let id = self.config.selected_instance.clone()?;
        self.instances.iter_mut().find(|i| i.id == id)
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len() > 500 {
            let drain = self.logs.len() - 500;
            self.logs.drain(..drain);
        }
    }

    /// Mojang/Fabric에서 버전 목록 비동기 갱신 (실패해도 기본값 유지)
    fn refresh_remote_lists(&mut self) {
        let http = self.http.clone();
        let versions = self.runtime.block_on(async {
            match crate::minecraft::version::fetch_manifest(&http).await {
                Ok(m) => {
                    let mut v: Vec<String> =
                        m.releases().into_iter().take(30).map(|e| e.id.clone()).collect();
                    if v.is_empty() {
                        v.push("1.20.1".into());
                    }
                    v
                }
                Err(e) => {
                    tracing::warn!("버전 목록 실패: {e:#}");
                    vec!["1.20.1".into(), "1.20.4".into(), "1.21".into()]
                }
            }
        });
        self.versions = versions;
        if !self.versions.contains(&self.new_mc_version) {
            if let Some(first) = self.versions.first().cloned() {
                self.new_mc_version = first;
            }
        }
    }

    fn refresh_loaders(&mut self, mc: &str) {
        let mc = mc.to_string();
        let http = self.http.clone();
        let loaders = self.runtime.block_on(async {
            match crate::minecraft::fabric::fetch_loaders_for_game(&http, &mc).await {
                Ok(list) => list.into_iter().take(20).map(|l| l.loader.version).collect(),
                Err(e) => {
                    tracing::warn!("Fabric loader 목록 실패: {e:#}");
                    vec!["0.16.9".into()]
                }
            }
        });
        if !loaders.is_empty() {
            self.loaders = loaders;
            if let Some(first) = self.loaders.first().cloned() {
                self.new_loader_version = first;
            }
        }
    }

    /// 플레이 버튼
    pub fn launch(&mut self) {
        if self.launching {
            return;
        }
        let Some(mut inst) = self.selected_instance().cloned() else {
            self.status = "인스턴스가 없습니다".into();
            return;
        };
        let config = self.config.clone();
        let session = MinecraftSession::offline(&config.username);
        self.launching = true;
        self.status = format!("실행 준비 중... ({})", inst.display_version());
        self.log(format!("▶ 실행: {} [{}]", inst.name, inst.display_version()));
        self.discord.show_playing(&inst.name, &inst.display_version());

        let progress_state = self.progress_state.clone();
        let cb = move |p: LaunchProgress| {
            if let Ok(mut guard) = progress_state.lock() {
                *guard = Some(p);
            }
        };

        // 준비+실행은 별도 스레드 (UI 블로킹 방지). tokio runtime는 block_on 사용.
        // child stdout은 이후 로그 탭에서 스트리밍 (phase 2에서 실시간 파이프 예정).
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            let rt = match rt {
                Ok(rt) => rt,
                Err(e) => return Err(anyhow::anyhow!("runtime: {e}")),
            };
            rt.block_on(crate::minecraft::launcher::prepare_and_launch(
                &config, &mut inst, &session, cb,
            ))
            .map(|_| inst)
        });

        // 결과 폴링은 다음 프레임들에서 progress와 함께 처리하기 위해
        // 여기서는 즉시 join하지 않고, launching 상태만 유지.
        // 단순화를 위해 phase 1에서는 짧은 블로킹 join + 상태 업데이트:
        match handle.join() {
            Ok(Ok(mut inst)) => {
                inst.total_plays += 1;
                inst.last_played = Some(chrono::Utc::now());
                if let Some(existing) = self.instances.iter_mut().find(|i| i.id == inst.id) {
                    *existing = inst.clone();
                }
                self.status = format!("{} 실행 중", inst.name);
                self.log("게임 프로세스 시작됨 (콘솔 로그는 phase 2에서 스트리밍)");
                self.persist();
            }
            Ok(Err(e)) => {
                self.status = format!("실행 실패: {e:#}");
                self.log(format!("실행 실패: {e:#}"));
                let user = self.config.username.clone();
                self.discord.show_home(&user);
            }
            Err(_) => {
                self.status = "스레드 오류".into();
                let user = self.config.username.clone();
                self.discord.show_home(&user);
            }
        }
        self.launching = false;
    }
}

// ---------------- egui ----------------

impl eframe::App for NebulyaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // progress 상태 동기화
        if let Ok(guard) = self.progress_state.lock() {
            if guard.is_some() {
                self.progress = guard.clone();
                if let Some(p) = &self.progress {
                    self.status = p.step.clone();
                }
            }
        }

        // 좌측 사이드바 (Lunar 스타일)
        egui::SidePanel::left("sidebar")
            .exact_width(210.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(14.0);
                ui.vertical_centered(|ui| {
                    ui.heading(egui::RichText::new("NEBULYA").size(22.0).strong().color(
                        crate::ui::theme::ACCENT_HOVER,
                    ));
                    ui.label(
                        egui::RichText::new("FABRIC LAUNCHER")
                            .size(10.0)
                            .color(crate::ui::theme::TEXT_DIM),
                    );
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                for tab in [Tab::Home, Tab::Instances, Tab::Mods, Tab::Settings] {
                    let selected = self.tab == tab;
                    let label = format!("{}  {}", tab.icon(), tab.label());
                    let rich = if selected {
                        egui::RichText::new(label).size(14.0).strong()
                    } else {
                        egui::RichText::new(label).size(14.0)
                    };
                    let btn = egui::Button::new(rich)
                        .selected(selected)
                        .min_size(egui::vec2(180.0, 38.0))
                        .corner_radius(egui::CornerRadius::same(10));
                    if ui.add(btn).clicked() {
                        self.tab = tab;
                        // 탭 이동 시 Discord 상태 반영
                        match tab {
                            Tab::Settings => self.discord.show_settings(),
                            _ => {
                                let user = self.config.username.clone();
                                self.discord.show_home(&user);
                            }
                        }
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("v0.2.0 • Fabric 전용")
                            .size(11.0)
                            .color(crate::ui::theme::TEXT_DIM),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("👤 {}", self.config.username))
                            .size(12.0)
                            .color(crate::ui::theme::TEXT_DIM),
                    );
                    ui.add_space(8.0);
                });
            });

        // 상단 바: 선택된 인스턴스 + 플레이
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("인스턴스").color(crate::ui::theme::TEXT_DIM));
                let mut selected_id = self
                    .config
                    .selected_instance
                    .clone()
                    .or_else(|| self.instances.first().map(|i| i.id.clone()))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("instance_select")
                    .selected_text(
                        self.selected_instance()
                            .map(|i| format!("{} ({})", i.name, i.display_version()))
                            .unwrap_or_else(|| "없음".into()),
                    )
                    .show_ui(ui, |ui| {
                        for inst in &self.instances {
                            ui.selectable_value(
                                &mut selected_id,
                                inst.id.clone(),
                                format!("{} ({})", inst.name, inst.display_version()),
                            );
                        }
                    });
                if !selected_id.is_empty() {
                    self.config.selected_instance = Some(selected_id);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn_label = if self.launching { "실행 중..." } else { "▶  플레이" };
                    let resp = crate::ui::theme::accent_button(ui, btn_label);
                    if resp.clicked() && !self.launching {
                        self.launch();
                    }
                });
            });
        });

        // 중앙 컨텐츠
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Home => crate::ui::screens::home::show(self, ui),
            Tab::Instances => crate::ui::screens::instances::show(self, ui),
            Tab::Mods => crate::ui::screens::mods::show(self, ui),
            Tab::Settings => crate::ui::screens::settings::show(self, ui),
        });

        // 하단 상태바
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("● {}", self.status))
                        .size(12.0)
                        .color(crate::ui::theme::TEXT_DIM),
                );
                if let Some(p) = &self.progress {
                    if p.total > 0 {
                        let frac = (p.done as f32 / p.total as f32).clamp(0.0, 1.0);
                        ui.add(
                            egui::ProgressBar::new(frac)
                                .show_percentage()
                                .desired_width(220.0),
                        );
                    }
                }
            });
        });

        // 새 인스턴스 모달
        if self.show_new_instance {
            egui::Window::new("새 인스턴스")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(360.0);
                    ui.label("이름");
                    ui.text_edit_singleline(&mut self.new_instance_name);
                    ui.label("마인크래프트 버전");
                    egui::ComboBox::from_id_salt("new_mc")
                        .selected_text(&self.new_mc_version)
                        .show_ui(ui, |ui| {
                            for v in self.versions.clone() {
                                ui.selectable_value(&mut self.new_mc_version, v.clone(), v);
                            }
                        });
                    if ui.button("Fabric 로더 목록 새로고침").clicked() {
                        let mc = self.new_mc_version.clone();
                        self.refresh_loaders(&mc);
                    }
                    ui.label("Fabric Loader 버전");
                    egui::ComboBox::from_id_salt("new_loader")
                        .selected_text(&self.new_loader_version)
                        .show_ui(ui, |ui| {
                            for v in self.loaders.clone() {
                                ui.selectable_value(&mut self.new_loader_version, v.clone(), v);
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("취소").clicked() {
                            self.show_new_instance = false;
                        }
                        if crate::ui::theme::accent_button(ui, "생성").clicked() {
                            let inst = Instance::new_fabric(
                                self.new_instance_name.clone(),
                                self.new_mc_version.clone(),
                                self.new_loader_version.clone(),
                            );
                            self.config.selected_instance = Some(inst.id.clone());
                            self.instances.push(inst);
                            self.persist();
                            self.show_new_instance = false;
                            self.log("새 인스턴스 생성됨");
                        }
                    });
                });
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}
