//! 앱 전역 설정 + 인스턴스(프로필) 정의

pub mod config;
pub mod instance;

pub use config::LauncherConfig;
pub use instance::{InstalledMod, Instance, LoaderType, ModSource};

/// 원자적 파일 쓰기: 임시 파일에 쓴 뒤 rename (쓰기 중 크래시에도 기존 파일 보존)
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let tmp = path.with_extension("nebtmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 파싱 실패한 데이터 파일 백업 (`<name>.json` → `<name>.json.bak-<stamp>`)
pub fn backup_corrupt(path: &std::path::Path) {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let bak = path.with_extension(format!("json.bak-{stamp}"));
    if std::fs::rename(path, &bak).is_ok() {
        tracing::warn!("깨진 데이터 파일 백업: {}", bak.display());
    }
}
