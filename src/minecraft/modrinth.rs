//! Modrinth 연동 (Fabric 전용)
//! Docs: https://docs.modrinth.com/api-spec/
//!
//! - 검색: GET /v2/search (facets로 project_type:mod + categories:fabric 강제)
//! - 버전: GET /v2/project/{id}/version?loaders=["fabric"]&game_versions=[...]
//! - 설치: primary 파일 url → instances/<id>/mods/<filename>

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const API: &str = "https://api.modrinth.com/v2";
/// Fabric 전용 고정 (이 런처는 Fabric만 지원)
pub const FABRIC_LOADER: &str = "fabric";

fn headers() -> reqwest::header::HeaderMap {
    let mut h = reqwest::header::HeaderMap::new();
    h.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "nebulya-launcher/0.2 (github.com/aruru10313/Nebula-rust)",
        ),
    );
    h
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<ProjectHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectHit {
    pub slug: String,
    #[serde(default)]
    pub project_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectVersion {
    pub id: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
}

/// Modrinth 모드 검색 (Fabric + project_type:mod 강제 필터)
pub async fn search_projects(
    client: &reqwest::Client,
    query: &str,
    mc_version: Option<&str>,
    limit: u32,
) -> Result<Vec<ProjectHit>> {
    // facets: 모드 + fabric 로더. game_versions는 버전 목록 단계에서 정확히 필터.
    let mut facets: Vec<Vec<String>> = vec![
        vec!["project_type:mod".to_string()],
        vec![format!("categories:{FABRIC_LOADER}")],
    ];
    if let Some(mc) = mc_version {
        if !mc.is_empty() {
            facets.push(vec![format!("versions:{mc}")]);
        }
    }
    let facets_json = serde_json::to_string(&facets)?;

    let res = client
        .get(format!("{API}/search"))
        .headers(headers())
        .query(&[
            ("query", query),
            ("facets", facets_json.as_str()),
            ("limit", limit.to_string().as_str()),
            ("index", "relevance"),
        ])
        .send()
        .await
        .context("Modrinth 검색 요청 실패")?
        .error_for_status()
        .context("Modrinth 검색 응답 오류")?
        .json::<SearchResponse>()
        .await
        .context("Modrinth 검색 파싱 실패")?;

    Ok(res.hits)
}

/// 해당 프로젝트의 Fabric+MC버전 호환 버전 목록 (최신순)
pub async fn fetch_versions(
    client: &reqwest::Client,
    project_id_or_slug: &str,
    mc_version: &str,
) -> Result<Vec<ProjectVersion>> {
    let loaders = serde_json::to_string(&[FABRIC_LOADER])?;
    let games = serde_json::to_string(&[mc_version])?;
    let res = client
        .get(format!("{API}/project/{project_id_or_slug}/version"))
        .headers(headers())
        .query(&[
            ("loaders", loaders.as_str()),
            ("game_versions", games.as_str()),
        ])
        .send()
        .await
        .context("Modrinth 버전 조회 실패")?
        .error_for_status()?
        .json::<Vec<ProjectVersion>>()
        .await?;
    Ok(res)
}

/// 최신 호환 버전의 primary 파일 선택
pub fn pick_file(v: &ProjectVersion) -> Option<&VersionFile> {
    v.files.iter().find(|f| f.primary).or(v.files.first())
}

/// 모드 다운로드 → mods 폴더에 저장 후 저장된 경로 반환
pub async fn install_latest(
    client: &reqwest::Client,
    project_id_or_slug: &str,
    mc_version: &str,
    mods_dir: &std::path::Path,
) -> Result<(ProjectVersion, std::path::PathBuf)> {
    let versions = fetch_versions(client, project_id_or_slug, mc_version).await?;
    let v = versions
        .first()
        .context("해당 MC버전+Fabric 호환 버전이 없음")?;
    let file = pick_file(v).context("버전에 파일이 없음")?;
    std::fs::create_dir_all(mods_dir)?;
    let dest = super::mods::safe_join(mods_dir, &file.filename)?;
    super::mods::download_url_to(client, &file.url, &dest).await?;
    Ok((v.clone(), dest))
}
