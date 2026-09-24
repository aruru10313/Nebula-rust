//! Mojang 버전 매니페스트 조회
//! https://piston-meta.mojang.com/mc/game/version_manifest_v2.json

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

impl VersionManifest {
    /// release 버전만 필터 (런처 기본 목록용)
    pub fn releases(&self) -> Vec<&ManifestEntry> {
        self.versions
            .iter()
            .filter(|v| v.version_type == "release")
            .collect()
    }
}

pub async fn fetch_manifest(client: &reqwest::Client) -> Result<VersionManifest> {
    let res = client
        .get(MANIFEST_URL)
        .send()
        .await
        .context("버전 매니페스트 요청 실패")?
        .error_for_status()?
        .json::<VersionManifest>()
        .await
        .context("버전 매니페스트 파싱 실패")?;
    Ok(res)
}

// ---- version 상세 json (assets, libraries, mainClass 등) ----

#[derive(Debug, Clone, Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<VersionArguments>,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub assets: Option<String>,
    pub libraries: Vec<Library>,
    pub downloads: Downloads,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionArguments {
    pub game: Vec<serde_json::Value>,
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    pub client: DownloadArtifact,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadArtifact {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaVersion {
    pub major: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<std::collections::HashMap<String, String>>,
    pub extract: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
    pub classifiers: Option<std::collections::HashMap<String, LibraryArtifact>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

pub async fn fetch_version_json(client: &reqwest::Client, url: &str) -> Result<VersionJson> {
    let v = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<VersionJson>()
        .await?;
    Ok(v)
}
