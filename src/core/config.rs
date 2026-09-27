use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 런처 전역 설정. `~/.nebulya-launcher/config.json` 에 저장됨.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    /// Mojang/Microsoft 계정 이름 (오프라인 모드에서도 사용)
    pub username: String,
    /// 할당 RAM (MB)
    pub ram_mb: u32,
    /// 최소 RAM (MB)
    pub min_ram_mb: u32,
    /// Java 실행 파일 경로 (비어있으면 자동탐지)
    pub java_path: String,
    /// 게임 해상도
    pub width: u32,
    pub height: u32,
    /// 전체화면으로 시작 (끄면 창모드 — Alt+Tab 전환이 편함)
    #[serde(default)]
    pub fullscreen: bool,
    /// 선택된 인스턴스 id
    pub selected_instance: Option<String>,
    /// 게임 디렉토리 루트
    pub game_root: PathBuf,
    /// 닫기 시 런처 숨기기 (Lunar 스타일)
    pub hide_on_launch: bool,
    /// CurseForge API 키 (console.curseforge.com 발급, 없으면 Modrinth만 사용)
    /// 환경변수 NEBULYA_CF_API_KEY 가 있으면 그 값을 우선 사용
    #[serde(default)]
    pub curseforge_api_key: String,
    /// Discord Activity 표시 여부
    #[serde(default = "default_true")]
    pub discord_enabled: bool,
    /// Discord Developers Application ID (비어있으면 Activity 비활성화)
    #[serde(default)]
    pub discord_client_id: String,
    /// Microsoft Client ID (Azure 앱 등록, 정품 로그인용)
    /// 환경변수 NEBULYA_MS_CLIENT_ID 가 있으면 그 값을 우선 사용
    #[serde(default)]
    pub ms_client_id: String,
    /// 저장된 정품 계정 (로그인 시 생성, refresh_token으로 자동 갱신)
    #[serde(default)]
    pub account: Option<crate::minecraft::auth::StoredAccount>,
    /// 저장된 Nebulya 자체 계정 (서버 세션 토큰)
    #[serde(default)]
    pub nebula_account: Option<crate::minecraft::nebula_auth::NebulaAccount>,
    /// 첫 실행 온보딩(로그인 게이트) 완료 여부
    #[serde(default)]
    pub onboarding_done: bool,
}

/// 릴리스 빌드에 내장된 공용 MS Client ID (NOAHSOFT KOREA).
/// Client ID는 OAuth URL에 그대로 노출되는 공개 값이므로 코드에 포함해도 된다.
/// 사용자가 설정/환경변수로 덮어쓸 수 있다.
pub const DEFAULT_MS_CLIENT_ID: &str = "e36da7c2-0cfc-45a6-9ba2-718fece7c46d";

fn default_true() -> bool {
    true
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            username: "Player".to_string(),
            ram_mb: 4096,
            min_ram_mb: 1024,
            java_path: String::new(),
            width: 854,
            height: 480,
            fullscreen: false,
            selected_instance: None,
            game_root: default_game_root(),
            hide_on_launch: false,
            curseforge_api_key: String::new(),
            discord_enabled: true,
            discord_client_id: String::new(),
            ms_client_id: String::new(),
            account: None,
            nebula_account: None,
            onboarding_done: false,
        }
    }
}

impl LauncherConfig {
    pub fn config_path() -> PathBuf {
        default_game_root().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(cfg) => cfg,
                Err(e) => {
                    // 파싱 실패 시 덮어쓰기 전에 백업 (계정 정보 보호)
                    tracing::warn!("config 파싱 실패, 백업 후 기본값 사용: {e}");
                    crate::core::backup_corrupt(&path);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)?;
        crate::core::write_atomic(&path, json.as_bytes()).context("config 저장 실패")?;
        Ok(())
    }

    /// 실제 사용할 Java 경로 (비어있으면 자동탐지 결과)
    pub fn effective_java(&self) -> String {
        if !self.java_path.trim().is_empty() {
            return self.java_path.clone();
        }
        crate::minecraft::java::find_java().unwrap_or_else(|| "java".to_string())
    }

    /// CurseForge API 키 (설정값 → 환경변수 순)
    pub fn curseforge_key(&self) -> String {
        if !self.curseforge_api_key.trim().is_empty() {
            return self.curseforge_api_key.clone();
        }
        std::env::var("NEBULYA_CF_API_KEY").unwrap_or_default()
    }

    /// MS Client ID (설정값 → 환경변수 → 내장 기본값 순)
    pub fn ms_client_id_resolved(&self) -> String {
        if !self.ms_client_id.trim().is_empty() {
            return self.ms_client_id.clone();
        }
        if let Ok(v) = std::env::var("NEBULYA_MS_CLIENT_ID") {
            if !v.trim().is_empty() {
                return v;
            }
        }
        DEFAULT_MS_CLIENT_ID.to_string()
    }
}

pub fn default_game_root() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".nebulya-launcher")
    } else {
        PathBuf::from(".nebulya-launcher")
    }
}
