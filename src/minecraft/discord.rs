//! Discord Activity 연동 (Rich Presence, IPC)
//!
//! - Discord 클라이언트가 켜져 있을 때만 표시, 꺼져 있으면 조용히 비활성화
//! - Application ID는 https://discord.com/developers/applications 에서 생성 후
//!   설정 탭에 입력 (비어있으면 Activity 끄기)
//! - 상태: 홈 대기 / 모드 검색 / 플레이 중 / 설정

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity::Activity};

pub struct DiscordPresence {
    enabled: bool,
    client_id: String,
    client: Option<DiscordIpcClient>,
}

impl DiscordPresence {
    pub fn new(enabled: bool, client_id: &str) -> Self {
        Self {
            enabled,
            client_id: client_id.trim().to_string(),
            client: None,
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

    fn ensure_connected(&mut self) -> bool {
        if !self.enabled || self.client_id.is_empty() {
            return false;
        }
        if self.client.is_none() {
            let mut c = DiscordIpcClient::new(&self.client_id);
            if c.connect().is_err() {
                return false; // 디스코드 미실행 등 — 조용히 패스
            }
            self.client = Some(c);
        }
        true
    }

    fn set(&mut self, state: &str, details: &str) {
        if !self.ensure_connected() {
            return;
        }
        if let Some(c) = self.client.as_mut() {
            let payload = Activity::new().state(state).details(details);
            if c.set_activity(payload).is_err() {
                self.close(); // 연결 끊김 → 다음 기회에 재접속
            }
        }
    }

    pub fn show_home(&mut self, username: &str) {
        self.set(
            "런처에서 대기 중",
            &format!("{username} | Fabric 모드 고르는 중"),
        );
    }

    pub fn show_search(&mut self, provider: &str, query: &str) {
        let q = query.chars().take(40).collect::<String>();
        self.set("모드 검색 중", &format!("{provider}: {q}"));
    }

    pub fn show_playing(&mut self, instance_name: &str, version: &str) {
        self.set("플레이 중", &format!("{instance_name} ({version})"));
    }

    pub fn show_settings(&mut self) {
        self.set("런처 설정 중", "Nebulya Launcher");
    }

    pub fn close(&mut self) {
        if let Some(mut c) = self.client.take() {
            let _ = c.close();
        }
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        self.close();
    }
}
