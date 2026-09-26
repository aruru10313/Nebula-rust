//! Mojang 버전 메타데이터 — Modrinth `daedalus` 크레이트를 그대로 사용한다.
//! (MIT, https://github.com/modrinth/daedalus — `cargo` 의존성, 코드 복사 없음)
//!
//! 기존 hand-rolled 구조체(VersionJson 등)는 삭제됨. 해당 구조체의
//! `major`/`majorVersion` 혼동이 온라인 첫 실행 실패의 직접 원인이었다.
//! 조회(재시도·캐시)는 Nebulya 레이어(`net`, `launcher`)가 담당한다.

use anyhow::Result;
pub use daedalus::minecraft::{
    DownloadType, Library, Os, RuleAction, Version, VersionInfo, VersionManifest, VersionType,
    VERSION_MANIFEST_URL,
};

/// release 버전만 필터 (런처 기본 목록용)
pub fn releases(manifest: &VersionManifest) -> Vec<&Version> {
    manifest
        .versions
        .iter()
        .filter(|v| matches!(v.type_, VersionType::Release))
        .collect()
}

pub async fn fetch_manifest(client: &reqwest::Client) -> Result<VersionManifest> {
    crate::minecraft::net::get_json(client, VERSION_MANIFEST_URL, "버전 매니페스트").await
}

pub async fn fetch_version_json(client: &reqwest::Client, url: &str) -> Result<VersionInfo> {
    crate::minecraft::net::get_json(client, url, "버전 정보").await
}

#[cfg(test)]
mod version_json_tests {
    use super::*;

    const BASE: &str = r#"{
        "id": "1.20.1",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": {"id": "5", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example.com/5.json"},
        "assets": "5",
        "libraries": [],
        "downloads": {"client": {"sha1": "b", "size": 2, "url": "https://example.com/c.jar"}},
        "minimumLauncherVersion": 21,
        "releaseTime": "2023-06-12T13:25:51+00:00",
        "time": "2023-06-12T13:25:51+00:00",
        "type": "release"
    }"#;

    fn parse_minimal(json: &str) -> VersionInfo {
        serde_json::from_str(json).expect("VersionInfo 파싱 실패")
    }

    #[test]
    fn parses_mojang_java_version_shape() {
        // 실제 Mojang 형식: {"component": "java-runtime-gamma", "majorVersion": 17}
        let with_java = BASE.replace(
            "\"libraries\": []",
            "\"javaVersion\": {\"component\": \"java-runtime-gamma\", \"majorVersion\": 17}, \"libraries\": []",
        );
        let v = parse_minimal(&with_java);
        assert_eq!(v.java_version.as_ref().map(|j| j.major_version), Some(17));
    }

    #[test]
    fn parses_without_java_version() {
        // 구버전 형식: javaVersion 필드 자체가 없음
        let v = parse_minimal(BASE);
        assert!(v.java_version.is_none());
    }

    #[test]
    fn releases_filters_snapshots() {
        let m: VersionManifest = serde_json::from_str(
            r#"{
            "latest": {"release": "1.20.1", "snapshot": "23w31a"},
            "versions": [
                {"id": "1.20.1", "type": "release", "url": "https://example.com/a", "time": "2023-06-12T13:25:51+00:00", "releaseTime": "2023-06-12T13:25:51+00:00", "sha1": "a", "complianceLevel": 1},
                {"id": "23w31a", "type": "snapshot", "url": "https://example.com/b", "time": "2023-08-02T11:00:00+00:00", "releaseTime": "2023-08-02T11:00:00+00:00", "sha1": "b", "complianceLevel": 1}
            ]
        }"#,
        )
        .expect("매니페스트 파싱 실패");
        let rel = releases(&m);
        assert_eq!(rel.len(), 1);
        assert_eq!(rel[0].id, "1.20.1");
    }
}
