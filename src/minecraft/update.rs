//! 자동 업데이트: GitHub Releases에서 최신 버전을 확인하고 설치한다.
//!
//! - Windows: Setup exe를 내려받아 실행 후 런처 종료 (설치기가 덮어씀)
//! - Linux/macOS: 해당 OS 패키지 다운로드 페이지를 브라우저로 연다

use anyhow::{Context, Result};
use serde::Deserialize;

const LATEST_URL: &str = "https://github.com/aruru10313/Nebula-rust";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub page_url: String,
    /// Windows Setup exe 직링크 (Windows에서만 사용)
    pub setup_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GhRelease {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GhAsset {
    #[serde(default)]
    name: String,
    #[serde(default)]
    browser_download_url: String,
}

/// "1.2.3" / "v1.2.3" → (1, 2, 3). 파싱 실패 시 None.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let mut it = s.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next()?.parse().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// 최신 릴리스 확인. 새 버전이 있으면 UpdateInfo, 최신이면 None.
pub async fn check_for_update(
    client: &reqwest::Client,
    current: &str,
) -> Result<Option<UpdateInfo>> {
    let rel: GhRelease = client
        .get("https://api.github.com/repos/aruru10313/Nebula-rust/releases/latest")
        .header(reqwest::header::USER_AGENT, "nebulya-launcher")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .context("릴리스 확인 요청 실패")?
        .error_for_status()
        .context("릴리스 확인 응답 오류")?
        .json()
        .await
        .context("릴리스 응답 파싱 실패")?;
    let latest = rel.tag_name.trim_start_matches('v').to_string();
    if !is_newer(current, &latest) {
        return Ok(None);
    }
    let setup_url = rel
        .assets
        .iter()
        .find(|a| a.name.starts_with("Nebulya-Launcher-Setup-") && a.name.ends_with(".exe"))
        .map(|a| a.browser_download_url.clone());
    Ok(Some(UpdateInfo {
        version: latest,
        page_url: if rel.html_url.is_empty() {
            LATEST_URL.to_string()
        } else {
            rel.html_url
        },
        setup_url,
    }))
}

/// Windows: Setup exe를 내려받아 실행하고 현재 프로세스 종료.
/// 호출 스레드에서 블로킹으로 실행한다.
pub fn download_and_run_setup(client: &reqwest::Client, url: &str) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let bytes = client
            .get(url)
            .header(reqwest::header::USER_AGENT, "nebulya-launcher")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let dest = std::env::temp_dir().join("Nebulya-Launcher-Setup-update.exe");
        std::fs::write(&dest, &bytes)?;
        Ok::<_, anyhow::Error>(dest)
    })
    .and_then(|dest| {
        std::process::Command::new(&dest)
            .spawn()
            .with_context(|| format!("설치기 실행 실패: {}", dest.display()))?;
        Ok(dest)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("0.4.0", "4.5.0"));
        assert!(!is_newer("4.5.0", "4.5.0"));
        assert!(!is_newer("4.5.0", "0.4.0"));
        assert!(is_newer("4.5.0", "v4.5.1"));
        assert!(!is_newer("abc", "4.5.0"));
    }
}
