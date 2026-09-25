use crate::core::{instance, Instance, LauncherConfig};
use crate::minecraft::auth::LoginState;
use crate::minecraft::discord::DiscordPresence;
use crate::minecraft::launcher::LaunchProgress;
use crate::minecraft::update::UpdateInfo;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Home,
    Instances,
    Mods,
    Account,
    Settings,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "홈",
            Tab::Instances => "인스턴스",
            Tab::Mods => "모드",
            Tab::Account => "계정",
            Tab::Settings => "설정",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Tab::Home => "▶",
            Tab::Instances => "▦",
            Tab::Mods => "◆",
            Tab::Account => "👤",
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
    pub login_wait_secs: Arc<Mutex<u64>>,
    pub update_info: Option<UpdateInfo>,
    pub update_status: String,
    update_result: Arc<Mutex<Option<Result<Option<UpdateInfo>, String>>>>,
    pub java_install_status: String,
    java_install_result: Arc<Mutex<Option<Result<String, String>>>>,
    maximized: bool,
    pub show_update_dialog: bool,
    update_dl: Arc<Mutex<Option<(u64, Option<u64>)>>>,
    launch_rx: Option<std::sync::mpsc::Receiver<LaunchOutcome>>,
    pub nebula_email: String,
    pub nebula_username: String,
    pub nebula_password: String,
    pub nebula_code: String,
    pub nebula_status: String,
    pub nebula_pending_verify: bool,
    pub nebula_mode_signup: bool,
    /// Java 버전 캐시 (매 프레임 프로세스 실행 방지)
    pub java_version_cache: String,
    minimize_after_launch: bool,
    pub confirm_delete_instance: Option<String>,
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
            logs: vec![format!(
                "★ Nebulya Launcher v{} stellar 초기화",
                env!("CARGO_PKG_VERSION")
            )],
            new_instance_name: "새 인스턴스".to_string(),
            new_mc_version: "1.20.1".to_string(),
            new_loader_version: "0.16.9".to_string(),
            show_new_instance: false,
            mods_ui: crate::ui::screens::mods::ModsUiState::default(),
            discord,
            login_state: Arc::new(Mutex::new(LoginState::Idle)),
            login_wait_secs: Arc::new(Mutex::new(0)),
            update_info: None,
            update_status: String::new(),
            update_result: Arc::new(Mutex::new(None)),
            java_install_status: String::new(),
            java_install_result: Arc::new(Mutex::new(None)),
            maximized: false,
            show_update_dialog: false,
            update_dl: Arc::new(Mutex::new(None)),
            launch_rx: None,
            nebula_email: String::new(),
            nebula_username: String::new(),
            nebula_password: String::new(),
            nebula_code: String::new(),
            nebula_status: String::new(),
            nebula_pending_verify: false,
            nebula_mode_signup: false,
            java_version_cache: String::new(),
            minimize_after_launch: false,
            confirm_delete_instance: None,
            runtime,
            http,
            progress_state: Arc::new(Mutex::new(None)),
        };
        app.refresh_remote_lists();
        app.refresh_java_version();
        app.persist();
        let user = app.config.username.clone();
        app.discord.show_home(&user);
        app.check_update_now();
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
            self.status = "MS Client ID가 없습니다.".to_string();
            return;
        }
        let state = self.login_state.clone();
        let wait_secs = self.login_wait_secs.clone();
        let http = self.http.clone();
        *self.login_state.lock().unwrap_or_else(|e| e.into_inner()) =
            LoginState::Working("코드 요청 중...".into());
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            let rt = match rt {
                Ok(rt) => rt,
                Err(e) => {
                    *state.lock().unwrap_or_else(|e| e.into_inner()) =
                        LoginState::Failed(format!("런타임 오류: {e}"));
                    return;
                }
            };
            rt.block_on(async {
                let set = |s: LoginState| {
                    *state.lock().unwrap_or_else(|e| e.into_inner()) = s;
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
                *wait_secs.lock().unwrap_or_else(|e| e.into_inner()) = dc.expires_in;
                // 2. 승인 대기 (Code 화면을 유지한 채 남은 시간만 갱신)
                let ms = match crate::minecraft::auth::poll_device_token(
                    &http,
                    &client_id,
                    &dc,
                    |left| {
                        *wait_secs.lock().unwrap_or_else(|e| e.into_inner()) = left;
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
        let snapshot = self
            .login_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        match snapshot {
            LoginState::Idle => {}
            LoginState::Code { .. } | LoginState::Working(_) => {
                // 상태 표시는 설정 화면에서 렌더링
            }
            LoginState::Done(acc) => {
                self.config.username = acc.username.clone();
                self.config.account = Some(acc.clone());
                self.config.onboarding_done = true;
                self.persist();
                self.status = format!("★ {}님, 정품 로그인 완료", acc.username);
                self.log(format!("정품 로그인: {} ({})", acc.username, acc.uuid));
                *self.login_state.lock().unwrap_or_else(|e| e.into_inner()) = LoginState::Idle;
            }
            LoginState::Failed(msg) => {
                self.status = format!("로그인 실패: {msg}");
                self.log(format!("로그인 실패: {msg}"));
                *self.login_state.lock().unwrap_or_else(|e| e.into_inner()) = LoginState::Idle;
            }
        }
    }

    /// 커스텀 타이틀바 (OS 기본 타이틀바 대신 다크 스타일)
    fn render_titlebar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::BG_PANEL)
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 6,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| {
                let bar = ui.allocate_response(
                    egui::vec2(ui.available_width(), 34.0),
                    egui::Sense::click_and_drag(),
                );
                if bar.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if bar.double_clicked() {
                    self.maximized = !self.maximized;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized));
                }
                ui.scope_builder(egui::UiBuilder::new().max_rect(bar.rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("★")
                                .size(14.0)
                                .color(crate::ui::theme::NEBULA_LIGHT),
                        );
                        ui.label(
                            egui::RichText::new("Nebulya Launcher")
                                .size(12.0)
                                .color(crate::ui::theme::TEXT_DIM),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(
                                    egui::RichText::new("×")
                                        .size(14.0)
                                        .color(crate::ui::theme::TEXT_DIM),
                                )
                                .clicked()
                            {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui
                                .small_button(
                                    egui::RichText::new("＋")
                                        .size(13.0)
                                        .color(crate::ui::theme::TEXT_DIM),
                                )
                                .clicked()
                            {
                                self.maximized = !self.maximized;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(
                                    self.maximized,
                                ));
                            }
                            if ui
                                .small_button(
                                    egui::RichText::new("–")
                                        .size(13.0)
                                        .color(crate::ui::theme::TEXT_DIM),
                                )
                                .clicked()
                            {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                        });
                    });
                });
            });
    }

    /// Microsoft 로그인 취소 (다음 폴링 사이클에서 스레드 종료)
    pub fn cancel_ms_login(&mut self) {
        *self.login_state.lock().unwrap_or_else(|e| e.into_inner()) = LoginState::Idle;
        self.status = "로그인 취소됨".to_string();
    }

    /// Adoptium JRE 21 자동 설치 (백그라운드, 결과는 다음 프레임에 반영)
    pub fn install_java_now(&mut self) {
        self.java_install_status = "Java 설치 중... (수 분 소요)".to_string();
        let http = self.http.clone();
        let root = self.config.game_root.clone();
        let slot = self.java_install_result.clone();
        std::thread::spawn(move || {
            let res = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt
                    .block_on(crate::minecraft::java::ensure_java_21(&http, &root))
                    .map_err(|e| format!("설치 실패: {e:#}")),
                Err(e) => Err(format!("런타임 오류: {e}")),
            };
            *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(res);
        });
    }

    fn poll_java_install(&mut self) {
        let res = if let Ok(mut slot) = self.java_install_result.lock() {
            slot.take()
        } else {
            None
        };
        if let Some(res) = res {
            match res {
                Ok(path) => {
                    self.config.java_path = path.clone();
                    self.refresh_java_version();
                    self.persist();
                    self.java_install_status = "Java 설치 완료".to_string();
                    self.log(format!("관리 JRE 설치됨: {path}"));
                }
                Err(e) => {
                    self.java_install_status = e;
                }
            }
        }
    }

    /// 업데이트 확인 (백그라운드, 결과는 다음 프레임에 반영)
    pub fn check_update_now(&mut self) {
        self.update_status = "업데이트 확인 중...".to_string();
        let http = self.http.clone();
        let slot = self.update_result.clone();
        let current = env!("CARGO_PKG_VERSION").to_string();
        std::thread::spawn(move || {
            let res = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt
                    .block_on(crate::minecraft::update::check_for_update(&http, &current))
                    .map_err(|e| format!("확인 실패: {e:#}")),
                Err(e) => Err(format!("런타임 오류: {e}")),
            };
            *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(res);
        });
    }

    /// 업데이트 적용 (Windows: Setup 실행 후 종료 / 그 외: 릴리스 페이지 열기)
    pub fn apply_update(&mut self) {
        let Some(info) = self.update_info.clone() else {
            return;
        };
        #[cfg(windows)]
        if let Some(url) = info.setup_url.clone() {
            self.show_update_dialog = false;
            self.update_status = "설치기를 내려받는 중...".to_string();
            *self.update_dl.lock().unwrap_or_else(|e| e.into_inner()) = Some((0, None));
            let http = self.http.clone();
            let slot = self.update_result.clone();
            let dl = self.update_dl.clone();
            std::thread::spawn(move || {
                let progress = |done: u64, total: Option<u64>| {
                    *dl.lock().unwrap_or_else(|e| e.into_inner()) = Some((done, total));
                };
                match crate::minecraft::update::download_and_run_setup(&http, &url, progress) {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        *dl.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        *slot.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(Err(format!("업데이트 실패: {e:#}")));
                    }
                }
            });
            return;
        }
        let _ = open::that(&info.page_url);
    }

    /// 업데이트 다운로드 진행 상황 (바이트, 전체). 다운로드 중이 아니면 None.
    pub fn update_download_progress(&self) -> Option<(u64, Option<u64>)> {
        self.update_dl
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn poll_update_result(&mut self) {
        let res = if let Ok(mut slot) = self.update_result.lock() {
            slot.take()
        } else {
            None
        };
        if let Some(res) = res {
            match res {
                Ok(Some(info)) => {
                    self.update_status = format!("새 버전 v{} 사용 가능", info.version);
                    self.log(format!("업데이트 발견: v{}", info.version));
                    self.update_info = Some(info);
                }
                Ok(None) => {
                    self.update_info = None;
                    self.update_status = "최신 버전입니다".to_string();
                }
                Err(e) => {
                    self.update_status = e;
                }
            }
        }
    }

    /// Nebulya 로그인 (블로킹 join — 기존 토큰 갱신과 같은 방식)
    pub fn nebula_login(&mut self) {
        let http = self.http.clone();
        let email = self.nebula_email.trim().to_string();
        let password = self.nebula_password.clone();
        self.nebula_password.clear();
        let rt_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => rt.block_on(crate::minecraft::nebula_auth::login(
                    &http, &email, &password,
                )),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        });
        match rt_handle.join() {
            Ok(Ok(acc)) => {
                self.config.username = acc.username.clone();
                self.config.nebula_account = Some(acc);
                self.config.onboarding_done = true;
                self.nebula_status = "Nebulya 로그인 완료".to_string();
                self.nebula_pending_verify = false;
                self.persist();
            }
            Ok(Err(e)) => self.nebula_status = format!("로그인 실패: {e:#}"),
            Err(_) => self.nebula_status = "스레드 오류".to_string(),
        }
    }

    /// Nebulya 회원가입 (인증 메일 발송)
    pub fn nebula_signup(&mut self) {
        let http = self.http.clone();
        let email = self.nebula_email.trim().to_string();
        let username = self.nebula_username.trim().to_string();
        let password = self.nebula_password.clone();
        self.nebula_password.clear();
        let rt_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => rt.block_on(crate::minecraft::nebula_auth::signup(
                    &http, &email, &username, &password,
                )),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        });
        match rt_handle.join() {
            Ok(Ok(())) => {
                self.nebula_pending_verify = true;
                self.nebula_status = "인증 코드를 이메일로 보냈습니다".to_string();
            }
            Ok(Err(e)) => self.nebula_status = format!("가입 실패: {e:#}"),
            Err(_) => self.nebula_status = "스레드 오류".to_string(),
        }
    }

    /// Nebulya 이메일 인증 코드 확인
    pub fn nebula_verify(&mut self) {
        let http = self.http.clone();
        let email = self.nebula_email.trim().to_string();
        let code = self.nebula_code.trim().to_string();
        let rt_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => rt.block_on(crate::minecraft::nebula_auth::verify(&http, &email, &code)),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        });
        match rt_handle.join() {
            Ok(Ok(())) => {
                self.nebula_pending_verify = false;
                self.nebula_code.clear();
                self.nebula_status = "인증 완료 — 로그인하세요".to_string();
            }
            Ok(Err(e)) => self.nebula_status = format!("인증 실패: {e:#}"),
            Err(_) => self.nebula_status = "스레드 오류".to_string(),
        }
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len() > 500 {
            let drain = self.logs.len() - 500;
            self.logs.drain(..drain);
        }
    }

    /// Java 버전 캐시 갱신 (프로세스 1회 실행)
    pub fn refresh_java_version(&mut self) {
        let java = self.config.effective_java();
        self.java_version_cache = crate::minecraft::java::java_version(&java).unwrap_or_default();
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

    /// 플레이 버튼 (정품 전용: 계정 없으면 실행 불가)
    /// 준비·다운로드·실행 전체를 백그라운드 스레드로 돌려 UI가 얼지 않는다.
    pub fn launch(&mut self) {
        if self.launching {
            return;
        }
        // 로그인은 필요한 사람만: 계정이 없어도 게스트 오프라인으로 실행
        if self.config.username.trim().is_empty() {
            self.config.username = "Player".to_string();
        }
        let Some(inst) = self.selected_instance().cloned() else {
            self.status = "인스턴스가 없습니다".into();
            return;
        };
        let config = self.config.clone();
        let http = self.http.clone();
        let progress_state = self.progress_state.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.launch_rx = Some(rx);
        self.launching = true;
        self.status = format!("실행 준비 중... ({})", inst.display_version());
        self.log(format!(
            "▶ 실행: {} [{}]",
            inst.name,
            inst.display_version()
        ));

        std::thread::spawn(move || {
            let send = |o: LaunchOutcome| {
                let _ = tx.send(o);
            };
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    send(LaunchOutcome::Failed(format!("런타임 오류: {e}")));
                    return;
                }
            };
            // MS 계정이 있으면 정품 세션 확보, Nebulya 계정만 있으면 오프라인 세션
            let (session, refreshed) = if let Some(ms_acc) = config.account.as_ref() {
                let (s, r) = rt.block_on(crate::minecraft::auth::ensure_session(
                    &http,
                    &config.ms_client_id_resolved(),
                    &config.username,
                    Some(ms_acc),
                ));
                (s, r)
            } else if let Some(neb) = config.nebula_account.as_ref() {
                (
                    crate::minecraft::auth::MinecraftSession::offline(&neb.username),
                    None,
                )
            } else {
                // 게스트 실행 (로그인 없음)
                (
                    crate::minecraft::auth::MinecraftSession::offline(&config.username),
                    None,
                )
            };
            if session.offline && config.account.is_some() {
                send(LaunchOutcome::Failed(
                    "정품 세션 갱신 실패 — 계정 탭에서 다시 로그인하세요".into(),
                ));
                return;
            }
            if session.offline {
                tracing::info!("Nebulya 계정으로 오프라인 실행");
            }
            let cb = move |p: LaunchProgress| {
                if let Ok(mut guard) = progress_state.lock() {
                    *guard = Some(p);
                }
            };
            let mut inst = inst;
            match rt.block_on(crate::minecraft::launcher::prepare_and_launch(
                &config, &mut inst, &session, cb,
            )) {
                Ok(_) => send(LaunchOutcome::Done { inst, refreshed }),
                Err(e) => send(LaunchOutcome::Failed(format!("{e:#}"))),
            }
        });
    }

    /// 백그라운드 실행 결과 반영 (매 프레임)
    fn poll_launch(&mut self) {
        let outcome = match &self.launch_rx {
            Some(rx) => match rx.try_recv() {
                Ok(o) => Some(o),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(LaunchOutcome::Failed("실행 스레드 종료".into()))
                }
            },
            None => None,
        };
        let Some(outcome) = outcome else {
            return;
        };
        self.launch_rx = None;
        self.launching = false;
        self.progress = None;
        *self
            .progress_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        match outcome {
            LaunchOutcome::Done {
                mut inst,
                refreshed,
            } => {
                if let Some(acc) = refreshed {
                    self.config.account = Some(acc);
                    self.config.username = self.config.account.as_ref().unwrap().username.clone();
                    self.log("정품 토큰 자동 갱신됨");
                }
                inst.total_plays += 1;
                inst.last_played = Some(chrono::Utc::now());
                if let Some(existing) = self.instances.iter_mut().find(|i| i.id == inst.id) {
                    *existing = inst.clone();
                }
                self.status = format!("{} 실행 중", inst.name);
                self.log("게임 프로세스 시작됨 (게임 로그 버튼으로 확인)");
                self.discord
                    .show_playing(&inst.name, &inst.display_version());
                self.minimize_after_launch = self.config.hide_on_launch;
                self.persist();
            }
            LaunchOutcome::Failed(e) => {
                self.status = format!("실행 실패: {e}");
                self.log(format!("실행 실패: {e}"));
                let user = self.config.username.clone();
                self.discord.show_home(&user);
            }
        }
    }
}

/// 백그라운드 실행 스레드 → UI 전달용 결과
enum LaunchOutcome {
    Done {
        inst: Instance,
        refreshed: Option<crate::minecraft::auth::StoredAccount>,
    },
    Failed(String),
}

// ---------------- egui ----------------

impl eframe::App for NebulyaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 다크 테마 강제 (초기화 순서와 무관하게 매 프레임 보장)
        ctx.set_visuals(crate::ui::theme::dark_visuals());
        // MS 로그인 상태 반영
        self.poll_login_state();
        // 업데이트 확인 결과 반영
        self.poll_update_result();
        // Java 설치 결과 반영
        self.poll_java_install();
        // 백그라운드 실행 결과 반영
        self.poll_launch();

        // 커스텀 타이틀바 (가장 먼저 렌더링)
        self.render_titlebar(ctx);

        // 실행 직후 숨기기 옵션
        if self.minimize_after_launch {
            self.minimize_after_launch = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

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

                for tab in [
                    Tab::Home,
                    Tab::Instances,
                    Tab::Mods,
                    Tab::Account,
                    Tab::Settings,
                ] {
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
                            &format!("★ v{} stellar", env!("CARGO_PKG_VERSION")),
                            crate::ui::theme::NEBULA_LIGHT,
                        );
                    });
                    ui.add_space(6.0);
                    // 유저 카드 (계정 상태만 표시)
                    crate::ui::theme::tile_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("★")
                                    .size(20.0)
                                    .color(crate::ui::theme::STAR_PINK),
                            );
                            ui.vertical(|ui| {
                                let display_name = self
                                    .config
                                    .account
                                    .as_ref()
                                    .map(|a| a.username.clone())
                                    .or_else(|| {
                                        self.config
                                            .nebula_account
                                            .as_ref()
                                            .map(|a| a.username.clone())
                                    })
                                    .unwrap_or_else(|| self.config.username.clone());
                                ui.label(egui::RichText::new(&display_name).size(13.0).strong());
                                if self.config.account.is_some() {
                                    ui.label(
                                        egui::RichText::new("● 정품 계정")
                                            .size(11.0)
                                            .color(crate::ui::theme::SUCCESS),
                                    );
                                } else if self.config.nebula_account.is_some() {
                                    ui.label(
                                        egui::RichText::new("● Nebulya 계정")
                                            .size(11.0)
                                            .color(crate::ui::theme::STAR_BLUE),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new("○ 미로그인")
                                            .size(11.0)
                                            .color(crate::ui::theme::TEXT_DIM),
                                    );
                                }
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
            Tab::Account => crate::ui::screens::account::show(self, ui),
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

        // 업데이트 확인 다이얼로그
        if self.show_update_dialog {
            if let Some(info) = self.update_info.clone() {
                egui::Window::new("런처 업데이트")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.set_min_width(380.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "v{} → v{}",
                                env!("CARGO_PKG_VERSION"),
                                info.version
                            ))
                            .size(18.0)
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(
                                "업데이트하면 설치기가 실행되고 런처가 다시 시작됩니다.",
                            )
                            .color(crate::ui::theme::TEXT_DIM)
                            .size(12.0),
                        );
                        ui.add_space(4.0);
                        if let Some((done, total)) = self.update_download_progress() {
                            match total {
                                Some(t) if t > 0 => {
                                    ui.add(
                                        egui::ProgressBar::new(done as f32 / t as f32)
                                            .show_percentage(),
                                    );
                                }
                                _ => {
                                    ui.spinner();
                                }
                            }
                            ui.label(
                                egui::RichText::new(format!(
                                    "내려받는 중... {:.1} MB",
                                    done as f32 / 1_048_576.0
                                ))
                                .size(11.0)
                                .color(crate::ui::theme::TEXT_DIM),
                            );
                        } else {
                            ui.horizontal(|ui| {
                                if crate::ui::theme::accent_button(ui, "지금 업데이트").clicked()
                                {
                                    self.apply_update();
                                }
                                if crate::ui::theme::ghost_button(ui, "나중에").clicked() {
                                    self.show_update_dialog = false;
                                }
                                if ui.small_button("변경 내용 보기").clicked() {
                                    let _ = open::that(&info.page_url);
                                }
                            });
                        }
                    });
            } else {
                self.show_update_dialog = false;
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 항해일지를 파일로 보관 (폴더에서 바로 확인 가능)
        let log_path = self.config.game_root.join("launcher.log");
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&log_path, self.logs.join("\n"));
        self.persist();
    }
}
