//! Nebulya 자체 계정 인증 (https://mc.aruru.kr/auth/user/*).
//! MS 로그인과 별개로 동작하며, 게임 실행 시 오프라인 세션의 사용자명으로 사용된다.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const NEBULA_API_BASE: &str = "https://mc.aruru.kr";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NebulaAccount {
    pub email: String,
    pub username: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenResp {
    #[serde(default)]
    token: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    email: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MeResp {
    #[serde(default)]
    username: String,
    #[serde(default)]
    email: String,
}

fn api(path: &str) -> String {
    format!("{NEBULA_API_BASE}{path}")
}

/// 회원가입 (인증 메일 발송). 이후 verify로 코드 확인.
pub async fn signup(
    client: &reqwest::Client,
    email: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    client
        .post(api("/auth/user/signup"))
        .json(&serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
        }))
        .send()
        .await
        .context("가입 요청 실패")?
        .error_for_status()
        .context("가입 거부 (입력값· 중복 확인)")?;
    Ok(())
}

/// 이메일 인증 코드 확인.
pub async fn verify(client: &reqwest::Client, email: &str, code: &str) -> Result<()> {
    client
        .post(api("/auth/user/verify"))
        .json(&serde_json::json!({ "email": email, "code": code }))
        .send()
        .await
        .context("인증 요청 실패")?
        .error_for_status()
        .context("인증 실패 (코드 확인)")?;
    Ok(())
}

/// 로그인 → 세션 토큰 포함 계정 반환.
pub async fn login(client: &reqwest::Client, email: &str, password: &str) -> Result<NebulaAccount> {
    let r: TokenResp = client
        .post(api("/auth/user/login"))
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .context("로그인 요청 실패")?
        .error_for_status()
        .context("로그인 실패 (이메일·비밀번호 확인)")?
        .json()
        .await
        .context("로그인 응답 파싱 실패")?;
    if r.token.is_empty() {
        anyhow::bail!("로그인 응답에 토큰이 없습니다");
    }
    Ok(NebulaAccount {
        email: r.email,
        username: r.username,
        token: r.token,
    })
}

/// 토큰 유효성 확인 + 최신 사용자명 가져오기.
pub async fn me(client: &reqwest::Client, token: &str) -> Result<(String, String)> {
    let r: MeResp = client
        .get(api("/auth/user/me"))
        .bearer_auth(token)
        .send()
        .await
        .context("세션 확인 요청 실패")?
        .error_for_status()
        .context("세션이 만료되었습니다. 다시 로그인하세요")?
        .json()
        .await
        .context("세션 응답 파싱 실패")?;
    Ok((r.email, r.username))
}

/// 로그아웃 (서버 세션 폐기).
pub async fn logout(client: &reqwest::Client, token: &str) -> Result<()> {
    client
        .post(api("/auth/user/logout"))
        .bearer_auth(token)
        .json(&serde_json::json!({}))
        .send()
        .await
        .context("로그아웃 요청 실패")?;
    Ok(())
}
