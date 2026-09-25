//! 인증 모듈: 오프라인 + Microsoft 정품 로그인
//!
//! 정품 흐름 (정품 런처/Prism과 동일):
//! 1. MS Device Code → 브라우저에서 코드 입력
//! 2. MS 토큰 → XboxLive 인증 → XSTS 인가 → Minecraft Services 로그인
//! 3. 프로필 조회 (닉네임/UUID) → refresh_token 저장 →次回 자동 갱신
//!
//! Client ID: Azure Portal 앱 등록 후 설정 탭 입력
//! (또는 환경변수 NEBULYA_MS_CLIENT_ID)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    /// true면 offline (--accessToken 0, userType legacy)
    pub offline: bool,
}

impl MinecraftSession {
    pub fn offline(username: &str) -> Self {
        // 오프라인 UUID = v3 (username 기반) — 바닐라 런처와 동일한 규칙
        let uuid = uuid::Uuid::new_v3(
            &uuid::Uuid::NAMESPACE_DNS,
            format!("OfflinePlayer:{username}").as_bytes(),
        );
        Self {
            username: username.to_string(),
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

/// 로그인 진행 상태 (백그라운드 스레드 → UI 전달용)
#[derive(Debug, Clone, Default)]
pub enum LoginState {
    #[default]
    Idle,
    /// 브라우저에서 코드 입력 대기
    Code {
        user_code: String,
        uri: String,
    },
    /// 승인 후 체인 진행 중 (표시용 메시지)
    Working(String),
    /// 완료 (UI가 config에 반영 후 Idle로)
    Done(StoredAccount),
    Failed(String),
}

/// 저장된 정품 계정 (config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccount {
    pub username: String,
    /// 하이픈 없는 32자리
    pub uuid: String,
    pub mc_token: String,
    pub ms_refresh_token: String,
    /// mc_token 만료 시각 (unix secs)
    pub mc_expires_at: i64,
}

impl StoredAccount {
    pub fn mc_valid(&self) -> bool {
        chrono::Utc::now().timestamp() < self.mc_expires_at - 300
    }
}

// ---------------- Device Code ----------------

const MS_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MsTokenOk {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenErr {
    error: String,
}

pub async fn request_device_code(client: &reqwest::Client, client_id: &str) -> Result<DeviceCode> {
    require_client_id(client_id)?;
    let res = client
        .post(MS_DEVICE_CODE_URL)
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await
        .context("MS devicecode 요청 실패")?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        let code = serde_json::from_str::<TokenErr>(&body)
            .map(|e| e.error)
            .unwrap_or_else(|_| "unknown".into());
        anyhow::bail!("MS devicecode 거부 ({status}, {code}) — Client ID·앱 설정을 확인하세요");
    }
    let dc = res
        .json::<DeviceCode>()
        .await
        .context("MS devicecode 파싱 실패")?;
    Ok(dc)
}

/// 브라우저 승인 대기 (polling). 성공 시 MS 토큰 반환.
pub async fn poll_device_token(
    client: &reqwest::Client,
    client_id: &str,
    dc: &DeviceCode,
    on_wait: impl Fn(u64),
) -> Result<MsTokenOk> {
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(dc.expires_in.saturating_sub(10));
    let interval = dc.interval.max(5);
    loop {
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("시간 초과 — 코드가 만료됐습니다. 다시 시도하세요.");
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        let res = client
            .post(MS_TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", dc.device_code.as_str()),
            ])
            .send()
            .await
            .context("MS 토큰 요청 실패")?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if status.is_success() {
            let ok: MsTokenOk = serde_json::from_str(&body).context("MS 토큰 파싱 실패")?;
            return Ok(ok);
        }
        let err: TokenErr = serde_json::from_str(&body).unwrap_or(TokenErr {
            error: "unknown".into(),
        });
        match err.error.as_str() {
            "authorization_pending" => {
                let left = deadline
                    .saturating_duration_since(std::time::Instant::now())
                    .as_secs();
                on_wait(left);
                continue;
            }
            "slow_down" => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            "authorization_declined" => anyhow::bail!("브라우저에서 거부됨"),
            "expired_token" => anyhow::bail!("코드가 만료됨 — 다시 시도하세요."),
            "bad_verification_code" => anyhow::bail!("잘못된 코드"),
            other => anyhow::bail!("MS 토큰 오류: {other}"),
        }
    }
}

pub async fn refresh_ms_token(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<MsTokenOk> {
    require_client_id(client_id)?;
    let res = client
        .post(MS_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .context("MS 토큰 갱신 실패")?;
    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("MS 토큰 갱신 거부 (다시 로그인 필요): {body}");
    }
    let ok: MsTokenOk = res.json().await.context("MS 토큰 파싱 실패")?;
    Ok(ok)
}

fn require_client_id(client_id: &str) -> Result<()> {
    if client_id.trim().is_empty() {
        anyhow::bail!("MS Client ID가 없습니다. Azure Portal에서 앱 등록 후 설정 탭에 입력하세요.");
    }
    Ok(())
}

// ---------------- Xbox → Minecraft ----------------

#[derive(Debug, Deserialize)]
struct XboxResp {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    claims: XboxClaims,
}

#[derive(Debug, Deserialize)]
struct XboxClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct XstsErr {
    #[serde(rename = "XErr")]
    xerr: Option<i64>,
}

/// 2148916233 = Xbox 계정 없음, 2148916238 = 자녀 계정(성인 승인 필요)
fn xerr_message(xerr: i64) -> &'static str {
    match xerr {
        2148916233 => "이 MS 계정에 Xbox 프로필이 없습니다 (xbox.com에서 생성)",
        2148916238 => "자녀 계정은 성인 계정 승인이 필요합니다",
        _ => "XSTS 인가 실패",
    }
}

pub async fn xbox_live_auth(
    client: &reqwest::Client,
    ms_access_token: &str,
) -> Result<(String, String)> {
    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}"),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });
    let res = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&body)
        .send()
        .await
        .context("XboxLive 인증 실패")?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(300).collect();
        anyhow::bail!("XboxLive 거부 ({status}): {snippet}");
    }
    let r: XboxResp = res.json().await.context("Xbox 응답 파싱 실패")?;
    let uhs = r.claims.xui.first().context("Xbox uhs 없음")?.uhs.clone();
    Ok((r.token, uhs))
}

pub async fn xsts_authorize(client: &reqwest::Client, xbl_token: &str) -> Result<(String, String)> {
    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token],
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT",
    });
    let res = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .context("XSTS 요청 실패")?;
    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        let xerr = serde_json::from_str::<XstsErr>(&body)
            .ok()
            .and_then(|e| e.xerr);
        match xerr {
            Some(code) => anyhow::bail!("{}", xerr_message(code)),
            None => anyhow::bail!("XSTS 인가 실패: {body}"),
        }
    }
    let r: XboxResp =
        serde_json::from_str(&res.text().await.unwrap_or_default()).context("XSTS 파싱 실패")?;
    let uhs = r.claims.xui.first().context("XSTS uhs 없음")?.uhs.clone();
    Ok((r.token, uhs))
}

#[derive(Debug, Deserialize)]
struct McLoginResp {
    access_token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct McProfile {
    name: String,
    id: String,
}

pub async fn minecraft_login(
    client: &reqwest::Client,
    uhs: &str,
    xsts_token: &str,
) -> Result<(String, i64)> {
    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
    });
    let res = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Minecraft 로그인 실패")?;
    if !res.status().is_success() {
        if res.status() == reqwest::StatusCode::FORBIDDEN {
            anyhow::bail!(
                "앱이 Minecraft API 심사를 통과하지 못했습니다. 승인 후 다시 시도하세요."
            );
        }
        let status = res.status();
        let snippet: String = res
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        anyhow::bail!("Minecraft 로그인 거부 ({status}): {snippet}");
    }
    let r: McLoginResp = res.json().await.context("Minecraft 응답 파싱 실패")?;
    Ok((r.access_token, r.expires_in))
}

pub async fn fetch_profile(client: &reqwest::Client, mc_token: &str) -> Result<(String, String)> {
    let r: McProfile = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_token)
        .send()
        .await
        .context("프로필 조회 실패")?
        .error_for_status()
        .context("프로필 조회 거부 (게임 미보유 가능)")?
        .json()
        .await?;
    Ok((r.name, r.id))
}

/// MS access_token → 전체 체인 → StoredAccount
pub async fn login_with_ms_token(
    client: &reqwest::Client,
    ms_access_token: &str,
    ms_refresh_token: &str,
) -> Result<StoredAccount> {
    let (xbl, _uhs) = xbox_live_auth(client, ms_access_token).await?;
    let (xsts, uhs) = xsts_authorize(client, &xbl).await?;
    let (mc_token, expires_in) = minecraft_login(client, &uhs, &xsts).await?;
    let (name, id) = fetch_profile(client, &mc_token).await?;
    Ok(StoredAccount {
        username: name,
        uuid: id.replace('-', ""),
        mc_token,
        ms_refresh_token: ms_refresh_token.to_string(),
        mc_expires_at: chrono::Utc::now().timestamp() + expires_in,
    })
}

/// refresh_token으로 계정 갱신
pub async fn refresh_account(
    client: &reqwest::Client,
    client_id: &str,
    account: &StoredAccount,
) -> Result<StoredAccount> {
    let ms = refresh_ms_token(client, client_id, &account.ms_refresh_token).await?;
    login_with_ms_token(client, &ms.access_token, &ms.refresh_token).await
}

/// 실행용 세션 확보: 정품 계정 있으면 갱신 시도, 실패하면 오프라인 폴백
pub async fn ensure_session(
    client: &reqwest::Client,
    client_id: &str,
    username_fallback: &str,
    account: Option<&StoredAccount>,
) -> (MinecraftSession, Option<StoredAccount>) {
    if let Some(acc) = account {
        if acc.mc_valid() {
            return (
                MinecraftSession::online(
                    acc.username.clone(),
                    acc.uuid.clone(),
                    acc.mc_token.clone(),
                ),
                None,
            );
        }
        match refresh_account(client, client_id, acc).await {
            Ok(new_acc) => {
                let sess = MinecraftSession::online(
                    new_acc.username.clone(),
                    new_acc.uuid.clone(),
                    new_acc.mc_token.clone(),
                );
                return (sess, Some(new_acc));
            }
            Err(e) => {
                tracing::warn!("정품 갱신 실패, 오프라인으로 실행: {e:#}");
            }
        }
    }
    (MinecraftSession::offline(username_fallback), None)
}
