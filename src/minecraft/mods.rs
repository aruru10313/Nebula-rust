//! 모드 공용 헬퍼: URL 다운로드 + 활성화/비활성화 + 삭제
//!
//! - `.jar` = 활성, `.jar.disabled` = 비활성 (Prism/Dawn과 동일 규칙)

use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

/// URL → 파일 스트리밍 다운로드
pub async fn download_url_to(
    client: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let res = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("다운로드 요청 실패: {url}"))?
        .error_for_status()
        .with_context(|| format!("다운로드 응답 오류: {url}"))?;
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

/// 원격 파일명을 mods 폴더 안에 안전하게 join.
/// 디렉터리 탈출(`..`, 절대경로, 구분자)을 제거하고 basename만 사용한다.
pub fn safe_join(dir: &std::path::Path, remote_name: &str) -> Result<std::path::PathBuf> {
    let normalized = remote_name.replace('\\', "/");
    let base = std::path::Path::new(&normalized)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && *n != "." && *n != "..")
        .with_context(|| format!("유효하지 않은 파일명: {remote_name}"))?;
    Ok(dir.join(base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_blocks_traversal() {
        let dir = std::path::Path::new("/mods");
        assert!(safe_join(dir, "../../evil.jar").is_ok()); // basename만 취함
        assert_eq!(
            safe_join(dir, "../../evil.jar").unwrap(),
            dir.join("evil.jar")
        );
        assert!(safe_join(dir, "").is_err());
        assert!(safe_join(dir, "..").is_err());
        assert!(safe_join(dir, "/").is_err());
        assert_eq!(
            safe_join(dir, "sub\\name.jar").unwrap(),
            dir.join("name.jar")
        );
        assert_eq!(
            safe_join(dir, "sodium-fabric-1.2.3.jar").unwrap(),
            dir.join("sodium-fabric-1.2.3.jar")
        );
    }
}
pub fn toggle_mod_file(mods_dir: &std::path::Path, file_name: &str) -> Result<String> {
    let src = mods_dir.join(file_name);
    if file_name.ends_with(".jar.disabled") {
        let dst_name = file_name.trim_end_matches(".disabled").to_string();
        std::fs::rename(&src, mods_dir.join(&dst_name))?;
        Ok(dst_name)
    } else {
        let dst_name = format!("{file_name}.disabled");
        std::fs::rename(&src, mods_dir.join(&dst_name))?;
        Ok(dst_name)
    }
}

/// 모드 파일 삭제
pub fn delete_mod_file(mods_dir: &std::path::Path, file_name: &str) -> Result<()> {
    let p = mods_dir.join(file_name);
    if p.exists() {
        std::fs::remove_file(&p)?;
    }
    Ok(())
}
