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

/// 모드 활성화/비활성화 토글. 반환값 = 토글 후 파일명
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
