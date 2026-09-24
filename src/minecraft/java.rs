//! Java 탐색: 설정된 경로 → JAVA_HOME → PATH → 번들(.nebulya-launcher/runtime)
//! Dawn/Prism처럼 Adoptium 자동다운로드는 추후 `ensure_java()`에 추가 예정.

use std::path::PathBuf;

/// 후보들을 순서대로 확인해서 `java(.exe)` 실행파일 경로 반환
pub fn find_java() -> Option<String> {
    // 1. JAVA_HOME
    if let Ok(home) = std::env::var("JAVA_HOME") {
        for name in [format!("{home}/bin/java"), format!("{home}/bin/java.exe")] {
            if std::path::Path::new(&name).exists() {
                return Some(name);
            }
        }
    }

    // 2. PATH 안의 java
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for exe in ["java", "java.exe"] {
                let p: PathBuf = dir.join(exe);
                if p.exists() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }

    // 3. 런처 번들 runtime
    if let Some(home) = dirs::home_dir() {
        let bundled = if cfg!(windows) {
            home.join(".nebulya-launcher/runtime/bin/java.exe")
        } else {
            home.join(".nebulya-launcher/runtime/bin/java")
        };
        if bundled.exists() {
            return Some(bundled.to_string_lossy().to_string());
        }
    }

    None
}

pub fn java_version(java: &str) -> Option<String> {
    let out = std::process::Command::new(java)
        .arg("-version")
        .output()
        .ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    // openjdk version "17.0.11" ... 형태에서 첫 따옴표 안 추출
    stderr
        .split('"')
        .nth(1)
        .map(|s| s.to_string())
        .or_else(|| Some(stderr.lines().next().unwrap_or("unknown").to_string()))
}
