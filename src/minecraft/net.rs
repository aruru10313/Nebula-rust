//! HTTP 조회 보조: 재시도 + 명시적 에러 전파.
//!
//! Modrinth App(theseus/app-lib)의 실행 파이프라인 설계를 참조했다.
//! (이 파일의 코드는 Nebulya가 독자적으로 작성 — GPL 코드 복사 없음)
//! - 메타데이터 조회는 최대 3회 재시도 (1초·2초 대기)
//! - 전송 오류만 재시도, 파싱 오류는 즉시 반환 (재시도해도 안 고쳐지므로)
//! - 모든 실패는 "무엇을+왜" 형태로 호출자에게 전달 (원인 삼키기 금지)

use anyhow::{Context, Result};

/// GET → JSON. `what`은 로그·에러용 설명문 (예: "버전 매니페스트").
pub async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    what: &str,
) -> Result<T> {
    let mut last_err = String::new();
    for attempt in 1..=3u32 {
        let res = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("{what} 요청 실패: {e:#}");
                tracing::warn!("{last_err} (시도 {attempt}/3)");
                sleep_before_retry(attempt).await;
                continue;
            }
        };
        let res = match res.error_for_status() {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("{what} 응답 오류: {e:#}");
                tracing::warn!("{last_err} (시도 {attempt}/3)");
                sleep_before_retry(attempt).await;
                continue;
            }
        };
        match res.json::<T>().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                return Err(anyhow::anyhow!("{what} 파싱 실패: {e:#}"))
                    .with_context(|| format!("{what} 조회 중단 ({url})"));
            }
        }
    }
    Err(anyhow::anyhow!("{last_err}"))
        .with_context(|| format!("{what} 조회 실패 (3회 시도, {url})"))
}

async fn sleep_before_retry(attempt: u32) {
    if attempt < 3 {
        tokio::time::sleep(std::time::Duration::from_secs(attempt as u64)).await;
    }
}
