//! 인증 모듈
//! - 1단계: 오프라인 모드 (닉네임만으로 실행, 개발/디자인용)
//! - 2단계: Microsoft OAuth Device Flow (TODO: Client ID 발급 후 활성화)
//!
//! Dawn/Lunar도 결국 MS OAuth → Xbox → Minecraft Services 순서를 거친다.
//! 여기서는 흐름 뼈대 + 오프라인 세션을 제공한다.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    /// true면 offline (authlib-injector 없이 --accessToken 0)
    pub offline: bool,
}

impl MinecraftSession {
    pub fn offline(username: &str) -> Self {
        // 오프라인 UUID = v3 (username 기반) — 바닐라 런처와 동일한 규칙
        let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, format!("OfflinePlayer:{username}").as_bytes());
        Self {
            username: username.to_string(),
            // 하이픈 제거 (마크는 하이픈 없는 32자리 사용)
            uuid: uuid.simple().to_string(),
            access_token: "0".to_string(),
            offline: true,
        }
    }

    pub fn online(username: String, uuid: String, access_token: String) -> Self {
        Self {
            username,
            uuid: uuid.replace('-', ""),
            access_token,
            offline: false,
        }
    }
}

// ---- Microsoft Device Flow 뼈대 (Client ID 필요) ----

#[allow(dead_code)]
pub struct MicrosoftAuth {
    client_id: String,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl MicrosoftAuth {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            client: reqwest::Client::new(),
        }
    }

    /// TODO: https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode
    /// scope = XboxLive.signin offline_access
    /// 반환된 user_code를 UI에 보여주고 브라우저로 열어주면 됨.
    pub async fn device_code_flow(&self) -> anyhow::Result<String> {
        anyhow::bail!("MS Client ID를 설정해야 사용 가능합니다 (auth.rs 참고)")
    }
}
