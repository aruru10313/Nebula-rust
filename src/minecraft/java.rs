//! Java 탐색 + Adoptium JRE 자동 설치 (Theseus식 관리 런타임)
//! 순서: 설정된 경로 → JAVA_HOME → PATH → 관리 런타임(<root>/runtime/java-21)

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

/// major 버전이 최소 요구치를 만족하는지 (예: 21 >= 17)
pub fn java_major_ok(java: &str, min_major: u32) -> bool {
    java_version(java)
        .and_then(|v| v.split('.').next()?.parse::<u32>().ok())
        // "1.8" 같은 구 표기는 major가 1이므로 무조건 미달 처리
        .map(|m| m >= min_major && !(m == 1))
        .unwrap_or(false)
}

// ---- Adoptium JRE 자동 설치 (Theseus식 관리 런타임) ----

/// 관리 런타임 위치: <game_root>/runtime/java-21
fn managed_runtime_dir(root: &std::path::Path) -> PathBuf {
    root.join("runtime").join("java-21")
}

fn managed_marker(root: &std::path::Path) -> PathBuf {
    managed_runtime_dir(root).join(".nebulya-java-ok")
}

fn managed_java_exe(root: &std::path::Path) -> PathBuf {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    managed_runtime_dir(root).join("bin").join(exe)
}

/// Adoptium API용 (os, arch). arch는 x64/aarch64만 지원.
fn adoptium_platform() -> anyhow::Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "mac",
        other => anyhow::bail!("지원하지 않는 OS: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => anyhow::bail!("지원하지 않는 아키텍처: {other}"),
    };
    Ok((os, arch))
}

/// JRE 21 확보: 이미 있으면 경로 반환, 없으면 Adoptium에서 내려받아 설치.
/// 블로킹 함수이므로 반드시 백그라운드 스레드에서 호출할 것.
pub async fn ensure_java_21(
    client: &reqwest::Client,
    root: &std::path::Path,
) -> anyhow::Result<String> {
    let dest = managed_runtime_dir(root);
    let java_exe = managed_java_exe(root);
    if managed_marker(root).exists() && java_exe.exists() {
        return Ok(java_exe.to_string_lossy().to_string());
    }

    let (os, arch) = adoptium_platform()?;
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/21/ga/{os}/{arch}/jre/hotspot/normal/eclipse"
    );
    tracing::info!("Adoptium JRE 21 다운로드: {url}");
    let bytes = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "nebulya-launcher")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("JRE 다운로드 요청 실패: {e:#}"))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("JRE 다운로드 응답 오류: {e:#}"))?
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("JRE 본문 수신 실패: {e:#}"))?;

    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .map_err(|e| anyhow::anyhow!("기존 런타임 정리 실패: {e}"))?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| anyhow::anyhow!("런타임 폴더 생성 실패: {e}"))?;

    if cfg!(windows) {
        extract_zip_safe(&bytes, &dest)?;
    } else {
        extract_tar_gz_safe(&bytes, &dest)?;
    }

    // 아카이브 최상위 폴더(jdk-21+...) 안의 bin/java를 runtime/bin으로 승격
    promote_nested_bin(&dest)?;
    if !java_exe.exists() {
        anyhow::bail!("설치 후에도 java 실행 파일을 찾지 못했습니다");
    }
    // 설치 검증: 실행 + 버전 확인
    let ver = java_version(&java_exe.to_string_lossy()).unwrap_or_default();
    tracing::info!("관리 JRE 설치됨: {ver}");
    std::fs::write(managed_marker(root), "21")
        .map_err(|e| anyhow::anyhow!("마커 기록 실패: {e}"))?;
    Ok(java_exe.to_string_lossy().to_string())
}

/// Zip Slip 방지: 절대경로·`..` 포함 항목 거부
fn safe_join(dest: &std::path::Path, name: &str) -> anyhow::Result<PathBuf> {
    let rel = std::path::Path::new(name);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        anyhow::bail!("위험한 아카이브 경로: {name}");
    }
    Ok(dest.join(rel))
}

fn extract_zip_safe(bytes: &[u8], dest: &std::path::Path) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("zip 파싱 실패: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| anyhow::anyhow!("zip 항목 읽기 실패: {e}"))?;
        let out = safe_join(dest, entry.name())?;
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| anyhow::anyhow!("폴더 생성 실패: {e}"))?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("폴더 생성 실패: {e}"))?;
            }
            let mut f =
                std::fs::File::create(&out).map_err(|e| anyhow::anyhow!("파일 생성 실패: {e}"))?;
            std::io::copy(&mut entry, &mut f)
                .map_err(|e| anyhow::anyhow!("압축 해제 실패: {e}"))?;
        }
    }
    Ok(())
}

fn extract_tar_gz_safe(bytes: &[u8], dest: &std::path::Path) -> anyhow::Result<()> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut tar = tar::Archive::new(gz);
    for entry in tar
        .entries()
        .map_err(|e| anyhow::anyhow!("tar 목록 읽기 실패: {e}"))?
    {
        let mut entry = entry.map_err(|e| anyhow::anyhow!("tar 항목 읽기 실패: {e}"))?;
        let name = entry
            .path()
            .map_err(|e| anyhow::anyhow!("tar 경로 읽기 실패: {e}"))?
            .to_string_lossy()
            .to_string();
        let out = safe_join(dest, &name)?;
        entry
            .unpack(&out)
            .map_err(|e| anyhow::anyhow!("압축 해제 실패({name}): {e}"))?;
    }
    Ok(())
}

/// dest直下가 단일 폴더면 그 안의 bin을 dest/bin으로 이동 (중첩 승격)
fn promote_nested_bin(dest: &std::path::Path) -> anyhow::Result<()> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    if dest.join("bin").join(exe).exists() {
        return Ok(());
    }
    let mut dirs = vec![];
    for e in std::fs::read_dir(dest).map_err(|e| anyhow::anyhow!("런타임 탐색 실패: {e}"))? {
        let e = e.map_err(|e| anyhow::anyhow!("런타임 탐색 실패: {e}"))?;
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            dirs.push(e.path());
        }
    }
    if dirs.len() == 1 {
        let nested_bin = dirs[0].join("bin");
        if nested_bin.join(exe).exists() {
            // 기존 bin이 없으므로 rename으로 승격
            std::fs::rename(&nested_bin, dest.join("bin"))
                .map_err(|e| anyhow::anyhow!("bin 승격 실패: {e}"))?;
            let _ = std::fs::remove_dir_all(&dirs[0]);
            return Ok(());
        }
    }
    // 최후 수단: 하위 3단계까지 java 실행 파일을 찾아 bin으로 승격
    if let Some(found) = find_file_bfs(dest, exe, 3) {
        if let Some(bin_dir) = found.parent() {
            if bin_dir != dest.join("bin") && !dest.join("bin").exists() {
                let _ = std::fs::rename(bin_dir, dest.join("bin"));
                return Ok(());
            }
        }
    }
    Ok(())
}

/// 너비 우선으로 파일명 탐색 (깊이 제한)
fn find_file_bfs(dir: &std::path::Path, name: &str, max_depth: usize) -> Option<PathBuf> {
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        let entries = std::fs::read_dir(&d).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push((p, depth + 1));
            } else if e.file_name().to_string_lossy() == name {
                return Some(p);
            }
        }
    }
    None
}
