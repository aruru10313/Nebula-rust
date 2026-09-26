//! CurseForge 연동 (Fabric 전용)
//! Docs: https://docs.curseforge.com/
//!
//! API 키 필요: https://console.curseforge.com/ 에서 발급
//! 설정(Settings) 또는 환경변수 NEBULYA_CF_API_KEY 로 입력.
//! - 검색: GET /v1/mods/search?gameId=432&classId=6&modLoaderType=4(Fabric)
//! - 파일: GET /v1/mods/{modId}/files?gameVersion=..&modLoaderType=4
//! - 설치: 최신 파일의 downloadUrl → mods 폴더

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const API: &str = "https://api.curseforge.com/v1";
/// Minecraft gameId
pub const MC_GAME_ID: u32 = 432;
/// Mods classId
pub const MODS_CLASS_ID: u32 = 6;
/// ModLoaderType::Fabric = 4 (이 런처는 Fabric만 지원)
pub const FABRIC_LOADER_TYPE: u32 = 4;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub data: Vec<CfMod>,
    pub pagination: CfPagination,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfPagination {
    #[serde(default)]
    pub total_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfMod {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: f64,
    #[serde(default)]
    pub links: Option<CfLinks>,
    #[serde(default)]
    pub logo: Option<CfLogo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CfLinks {
    #[serde(default)]
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CfLogo {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilesResponse {
    pub data: Vec<CfFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CfFile {
    pub id: u32,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
}

fn key_headers(api_key: &str) -> reqwest::header::HeaderMap {
    let mut h = reqwest::header::HeaderMap::new();
    h.insert(
        "x-api-key",
        reqwest::header::HeaderValue::from_str(api_key)
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
    );
    h
}

fn require_key(api_key: &str) -> Result<()> {
    if api_key.trim().is_empty() {
        anyhow::bail!(
            "CurseForge API 키가 없습니다. console.curseforge.com에서 발급받아 설정 탭에 입력하세요."
        );
    }
    Ok(())
}

/// CurseForge 모드 검색 (Fabric 강제)
pub async fn search_mods(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    mc_version: Option<&str>,
    page_size: u32,
) -> Result<Vec<CfMod>> {
    require_key(api_key)?;
    let mut req = client
        .get(format!("{API}/mods/search"))
        .headers(key_headers(api_key))
        .query(&[
            ("gameId", MC_GAME_ID.to_string().as_str()),
            ("classId", MODS_CLASS_ID.to_string().as_str()),
            ("modLoaderType", FABRIC_LOADER_TYPE.to_string().as_str()),
            ("pageSize", page_size.to_string().as_str()),
            ("searchFilter", query),
        ]);
    if let Some(mc) = mc_version {
        if !mc.is_empty() {
            req = req.query(&[("gameVersion", mc)]);
        }
    }
    let res = req
        .send()
        .await
        .context("CurseForge 검색 요청 실패")?
        .error_for_status()
        .context("CurseForge 검색 응답 오류 (API 키 확인)")?
        .json::<SearchResponse>()
        .await
        .context("CurseForge 검색 파싱 실패")?;
    Ok(res.data)
}

/// 해당 모드의 Fabric+MC버전 호환 파일 목록 (최신순)
pub async fn fetch_files(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: u32,
    mc_version: &str,
) -> Result<Vec<CfFile>> {
    require_key(api_key)?;
    let res = client
        .get(format!("{API}/mods/{mod_id}/files"))
        .headers(key_headers(api_key))
        .query(&[
            ("gameVersion", mc_version),
            ("modLoaderType", FABRIC_LOADER_TYPE.to_string().as_str()),
        ])
        .send()
        .await
        .context("CurseForge 파일 조회 실패")?
        .error_for_status()?
        .json::<FilesResponse>()
        .await?;
    Ok(res.data)
}

/// 최신 호환 파일 다운로드 → mods 폴더
pub async fn install_latest(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: u32,
    mc_version: &str,
    mods_dir: &std::path::Path,
) -> Result<(CfFile, std::path::PathBuf)> {
    let files = fetch_files(client, api_key, mod_id, mc_version).await?;
    let f = files
        .iter()
        .find(|f| f.download_url.is_some())
        .context("해당 MC버전+Fabric 호환 파일이 없음")?;
    let url = f
        .download_url
        .clone()
        .context("선택된 파일에 다운로드 URL이 없음")?;
    std::fs::create_dir_all(mods_dir)?;
    let dest = super::mods::safe_join(mods_dir, &f.file_name)?;
    super::mods::download_url_to(client, &url, &dest).await?;
    Ok((f.clone(), dest))
}
