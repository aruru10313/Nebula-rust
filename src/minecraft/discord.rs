//! Discord Activity 연동 (Rich Presence, IPC)
//!
//! - Discord 클라이언트가 켜져 있을 때만 표시, 꺼져 있으면 조용히 비활성화
//! - Application ID는 https://discord.com/developers/applications 에서 생성 후
//!   설정 탭에 입력 (비어있으면 Activity 끄기)
//! - 상태: 홈 대기 / 모드 검색 / 플레이 중 / 설정 + 경과 시간 + GitHub 버튼

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Assets, Button, Timestamps},
};

const GITHUB_URL: &str = "https://github.com/aruru10313/Nebula-rust";

pub struct DiscordPresence {
    enabled: bool,
    client_id: String,
    client: Option<DiscordIpcClient>,
    /// 마지막 set_activity 성공 여부 (UI 상태 표시용)
    connected: bool,
    /// 런처 부팅 시각 (경과 시간 표시용, unix secs)
    boot_time: i64,
}

impl DiscordPresence {
    pub fn new(enabled: bool, client_id: &str) -> Self {
        Self {
            enabled,
            client_id: client_id.trim().to_string(),
            client: None,
            connected: false,
            boot_time: chrono::Utc::now().timestamp(),
        }
    }

    /// 설정 변경 시 (ID on/off) 재구성
    pub fn reconfigure(&mut self, enabled: bool, client_id: &str) {
        let id = client_id.trim().to_string();
        if self.enabled == enabled && self.client_id == id {
            return;
        }
        self.close();
        self.enabled = enabled;
        self.client_id = id;
    }

    /// UI 표시용 연결 상태 (마지막 전송 성공 여부)
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// 설정은 됐지만 아직 연결 확인 안 됨
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.client_id.is_empty()
    }

    fn ensure_connected(&mut self) -> bool {
        if !self.is_configured() {
            return false;
        }
        if self.client.is_none() {
            let mut c = DiscordIpcClient::new(&self.client_id);
            if c.connect().is_err() {
                self.connected = false;
                return false; // 디스코드 미실행 등 — 조용히 패스
            }
            self.client = Some(c);
        }
        true
    }

    fn set(&mut self, state: &str, details: &str, large_text: &str) {
        if !self.ensure_connected() {
            return;
        }
        let payload = Activity::new()
            .state(state)
            .details(details)
            .timestamps(Timestamps::new().start(self.boot_time))
            .assets(
                Assets::new()
                    .large_text(large_text)
                    .small_text("Nebulya Launcher"),
            )
            .buttons(vec![Button::new("★ Nebula-rust", GITHUB_URL)]);
        if let Some(c) = self.client.as_mut() {
            if c.set_activity(payload).is_err() {
                self.connected = false;
                self.close(); // 연결 끊김 → 다음 기회에 재접속
            } else {
                self.connected = true;
            }
        }
    }

    pub fn show_home(&mut self, username: &str) {
        self.set(
            "런처에서 대기 중",
            &format!("{username} | Fabric 모드 고르는 중"),
            "Nebulya Launcher",
        );
    }

    pub fn show_search(&mut self, provider: &str, query: &str) {
        let q = query.chars().take(40).collect::<String>();
        self.set(
            "모드 검색 중",
            &format!("{provider}: {q}"),
            "Modrinth · CurseForge",
        );
    }

    pub fn show_playing(&mut self, instance_name: &str, version: &str) {
        self.set(
            "플레이 중",
            &format!("{instance_name} ({version})"),
            "Minecraft Fabric",
        );
    }

    pub fn show_settings(&mut self) {
        self.set("런처 설정 중", "Nebulya Launcher", "Nebulya Launcher");
    }

    pub fn close(&mut self) {
        if let Some(mut c) = self.client.take() {
            let _ = c.close();
        }
        self.connected = false;
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        self.close();
    }
}
