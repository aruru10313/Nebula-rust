use crate::core::{instance, Instance, LauncherConfig};
use crate::minecraft::auth::LoginState;
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
            Tab::Mods => "◆",
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
    pub login_state: Arc<Mutex<LoginState>>,
    pub runtime: tokio::runtime::Runtime,
    pub http: reqwest::Client,
    progress_state: Arc<Mutex<Option<LaunchProgress>>>,
}

impl NebulyaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::theme::apply_theme(&cc.egui_ctx);
        crate::ui::fonts::install_korean_fonts(&cc.egui_ctx);
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
            logs: vec!["★ Nebulya Launcher v0.3 stellar 초기화".to_string()],
            new_instance_name: "새 인스턴스".to_string(),
            new_mc_version: "1.20.1".to_string(),
            new_loader_version: "0.16.9".to_string(),
            show_new_instance: false,
            mods_ui: crate::ui::screens::mods::ModsUiState::default(),
            discord,
            login_state: Arc::new(Mutex::new(LoginState::Idle)),
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

    /// Microsoft 정품 로그인 시작 (백그라운드 스레드)
    pub fn start_ms_login(&mut self) {
        if let Ok(s) = self.login_state.lock() {
            if !matches!(*s, LoginState::Idle) {
                return; // 이미 진행 중
            }
        }
        let client_id = self.config.ms_client_id_resolved();
        if client_id.is_empty() {
            self.status = "MS Client ID가 없습니다. Azure 앱 등록 후 입력하세요.".to_string();
            return;
        }
        let state = self.login_state.clone();
        let http = self.http.clone();
        *self.login_state.lock().unwrap() = LoginState::Working("코드 요청 중...".into());
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            let rt = match rt {
                Ok(rt) => rt,
                Err(e) => {
                    *state.lock().unwrap() = LoginState::Failed(format!("런타임 오류: {e}"));
                    return;
                }
            };
            rt.block_on(async {
                let set = |s: LoginState| {
                    *state.lock().unwrap() = s;
                };
                // 1. device code
                let dc = match crate::minecraft::auth::request_device_code(&http, &client_id).await
                {
                    Ok(dc) => dc,
                    Err(e) => {
                        set(LoginState::Failed(format!("{e:#}")));
                        return;
                    }
                };
                set(LoginState::Code {
                    user_code: dc.user_code.clone(),
                    uri: dc.verification_uri.clone(),
                });
                // 2. 승인 대기
                let ms = match crate::minecraft::auth::poll_device_token(
                    &http,
                    &client_id,
                    &dc,
                    |left| {
                        *state.lock().unwrap() =
                            LoginState::Working(format!("브라우저 승인 대기 중... ({left}초)"));
                    },
                )
                .await
                {
                    Ok(ms) => ms,
                    Err(e) => {
                        set(LoginState::Failed(format!("{e:#}")));
                        return;
                    }
                };
                // 3. Xbox → MC 체인
                set(LoginState::Working("Xbox 연결 중...".into()));
                match crate::minecraft::auth::login_with_ms_token(
                    &http,
                    &ms.access_token,
                    &ms.refresh_token,
                )
                .await
                {
                    Ok(acc) => set(LoginState::Done(acc)),
                    Err(e) => set(LoginState::Failed(format!("{e:#}"))),
                }
            });
        });
    }

    /// 로그인 상태 폴링 (매 프레임) — 완료/실패를 config에 반영
    fn poll_login_state(&mut self) {
        let snapshot = self.login_state.lock().unwrap().clone();
        match snapshot {
            LoginState::Idle => {}
            LoginState::Code { .. } | LoginState::Working(_) => {
                // 상태 표시는 설정 화면에서 렌더링
            }
            LoginState::Done(acc) => {
                self.config.username = acc.username.clone();
                self.config.account = Some(acc.clone());
                self.persist();
                self.status = format!("★ {}님, 정품 로그인 완료", acc.username);
                self.log(format!("정품 로그인: {} ({})", acc.username, acc.uuid));
                *self.login_state.lock().unwrap() = LoginState::Idle;
            }
            LoginState::Failed(msg) => {
                self.status = format!("로그인 실패: {msg}");
                self.log(format!("로그인 실패: {msg}"));
                *self.login_state.lock().unwrap() = LoginState::Idle;
            }
        }
    }

    /// Microsoft 로그인 취소 (다음 폴링 사이클에서 스레드 종료)
    pub fn cancel_ms_login(&mut self) {
        *self.login_state.lock().unwrap() = LoginState::Idle;
        self.status = "로그인 취소됨".to_string();
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
                    let mut v: Vec<String> = m
                        .releases()
                        .into_iter()
                        .take(30)
                        .map(|e| e.id.clone())
                        .collect();
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
                Ok(list) => list
                    .into_iter()
                    .take(20)
                    .map(|l| l.loader.version)
                    .collect(),
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
        // 정품 계정이 있으면 갱신 시도, 없거나 실패하면 오프라인
        let (session, refreshed) = self
            .runtime
            .block_on(crate::minecraft::auth::ensure_session(
                &self.http,
                &config.ms_client_id_resolved(),
                &config.username,
                config.account.as_ref(),
            ));
        if let Some(acc) = refreshed {
            self.config.account = Some(acc);
            self.config.username = self.config.account.as_ref().unwrap().username.clone();
            self.persist();
            self.log("정품 토큰 자동 갱신됨");
        }
        if session.offline && self.config.account.is_some() {
            self.log("정품 갱신 실패 — 오프라인으로 실행합니다");
        }
        self.launching = true;
        self.status = format!("실행 준비 중... ({})", inst.display_version());
        self.log(format!(
            "▶ 실행: {} [{}]",
            inst.name,
            inst.display_version()
        ));
        self.discord
            .show_playing(&inst.name, &inst.display_version());

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
        // MS 로그인 상태 반영
        self.poll_login_state();
        // progress 상태 동기화
        if let Ok(guard) = self.progress_state.lock() {
            if guard.is_some() {
                self.progress = guard.clone();
                if let Some(p) = &self.progress {
                    self.status = p.step.clone();
                }
            }
        }

        // 좌측 사이드바 — 별빛 내비게이션
        egui::SidePanel::left("sidebar")
            .exact_width(224.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(18.0);
                // 로고
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("★")
                            .size(34.0)
                            .color(crate::ui::theme::NEBULA_LIGHT),
                    );
                    ui.heading(
                        egui::RichText::new("NEBULYA")
                            .size(24.0)
                            .strong()
                            .color(crate::ui::theme::STARLIGHT),
                    );
                    ui.label(
                        egui::RichText::new("S T E L L A R · F A B R I C")
                            .size(9.0)
                            .color(crate::ui::theme::TEXT_FAINT),
                    );
                });
                ui.add_space(10.0);
                crate::ui::theme::star_divider(ui);
                ui.add_space(6.0);

                for tab in [Tab::Home, Tab::Instances, Tab::Mods, Tab::Settings] {
                    let selected = self.tab == tab;
                    let label = format!("{}  {}", tab.icon(), tab.label());
                    let rich = if selected {
                        egui::RichText::new(label)
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::WHITE)
                    } else {
                        egui::RichText::new(label)
                            .size(14.0)
                            .color(crate::ui::theme::TEXT_DIM)
                    };
                    let mut btn = egui::Button::new(rich)
                        .selected(selected)
                        .min_size(egui::vec2(192.0, 40.0))
                        .corner_radius(egui::CornerRadius::same(12));
                    if selected {
                        btn = btn.fill(crate::ui::theme::NEBULA);
                    }
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
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        crate::ui::theme::badge(
                            ui,
                            "★ v0.3.0 stellar",
                            crate::ui::theme::NEBULA_LIGHT,
                        );
                    });
                    ui.add_space(6.0);
                    // 유저 카드
                    crate::ui::theme::tile_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("★")
                                    .size(20.0)
                                    .color(crate::ui::theme::STAR_PINK),
                            );
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&self.config.username)
                                        .size(13.0)
                                        .strong(),
                                );
                                let (dot, dot_color, txt) = if !self.config.discord_enabled
                                    || self.config.discord_client_id.trim().is_empty()
                                {
                                    ("○", crate::ui::theme::TEXT_FAINT, "Discord 꺼짐")
                                } else if self.discord.is_connected() {
                                    ("●", crate::ui::theme::SUCCESS, "활동 표시 중")
                                } else {
                                    ("●", crate::ui::theme::WARN, "연결 대기 중")
                                };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(dot).size(11.0).color(dot_color));
                                    ui.label(
                                        egui::RichText::new(txt)
                                            .size(11.0)
                                            .color(crate::ui::theme::TEXT_DIM),
                                    );
                                });
                            });
                        });
                    });
                    ui.add_space(4.0);
                });
            });

        // 상단 바: 선택된 인스턴스 + 플레이
        egui::TopBottomPanel::top("topbar")
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::BG_PANEL)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 10,
                        bottom: 10,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("◆  INSTANCE")
                            .size(11.0)
                            .color(crate::ui::theme::TEXT_FAINT),
                    );
                    let mut selected_id = self
                        .config
                        .selected_instance
                        .clone()
                        .or_else(|| self.instances.first().map(|i| i.id.clone()))
                        .unwrap_or_default();
                    egui::ComboBox::from_id_salt("instance_select")
                        .selected_text(
                            self.selected_instance()
                                .map(|i| format!("★ {}  ·  {}", i.name, i.display_version()))
                                .unwrap_or_else(|| "없음".into()),
                        )
                        .show_ui(ui, |ui| {
                            for inst in &self.instances {
                                ui.selectable_value(
                                    &mut selected_id,
                                    inst.id.clone(),
                                    format!("★ {}  ·  {}", inst.name, inst.display_version()),
                                );
                            }
                        });
                    if !selected_id.is_empty() {
                        self.config.selected_instance = Some(selected_id);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_label = if self.launching {
                            "★  실행 중..."
                        } else {
                            "▶  플레이"
                        };
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

        // 하단 상태바 — 별빛 상태
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::BG_PANEL)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 6,
                        bottom: 6,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("★")
                            .color(crate::ui::theme::NEBULA_LIGHT)
                            .size(12.0),
                    );
                    ui.label(
                        egui::RichText::new(&self.status)
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
