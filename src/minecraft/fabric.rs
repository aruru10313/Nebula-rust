//! Fabric 메타 API
//! https://meta.fabricmc.net/v2/versions/loader/<mc_version>
//! https://meta.fabricmc.net/v2/versions/loader (전체)
//! https://meta.fabricmc.net/v2/versions/game (지원 마크 버전)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FabricLoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FabricLoaderForGame {
    pub loader: FabricLoaderVersion,
    pub intermediary: Intermediary,
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: LauncherMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Intermediary {
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherMeta {
    pub version: u32,
    pub libraries: serde_json::Value,
    #[serde(rename = "mainClass")]
    pub main_class: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

/// 해당 마크 버전에서 사용 가능한 Fabric 로더 목록
pub async fn fetch_loaders_for_game(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<Vec<FabricLoaderForGame>> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc_version}");
    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Fabric loader 조회 실패: {mc_version}"))?
        .error_for_status()?
        .json::<Vec<FabricLoaderForGame>>()
        .await?;
    Ok(res)
}

/// 최신 stable 로더 버전 문자열만
pub async fn fetch_latest_stable_loader(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<String> {
    let list = fetch_loaders_for_game(client, mc_version).await?;
    let stable = list
        .iter()
        .find(|v| v.loader.stable)
        .or(list.first())
        .context("Fabric loader 목록이 비어있음")?;
    Ok(stable.loader.version.clone())
}

/// Fabric이 지원하는 마크 버전 목록
pub async fn fetch_game_versions(client: &reqwest::Client) -> Result<Vec<GameVersion>> {
    let res = client
        .get("https://meta.fabricmc.net/v2/versions/game")
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<GameVersion>>()
        .await?;
    Ok(res)
}
