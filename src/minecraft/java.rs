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
    let mut cmd = std::process::Command::new(java);
    cmd.arg("-version");
    #[cfg(windows)]
    {
        // 콘솔 창이 깜빡이지 않게 (CREATE_NO_WINDOW)
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output().ok()?;
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

/// 실행용 Java 선택 (Modrinth App식 managed-first):
/// 1. 사용자가 지정한 경로가 요구치 만족 → 그대로
/// 2. 관리 런타임(java-21)이 요구치 만족 → 사용
/// 3. 시스템 Java가 요구치 만족 → 사용
/// 4. 요구치가 16+ → Adoptium JRE 21 자동 설치
/// 5. 구버전(8 이하)인데 쓸 Java가 없음 → 명확한 에러
pub async fn resolve_java_for_launch(
    client: &reqwest::Client,
    config: &crate::core::LauncherConfig,
    required_major: u32,
) -> anyhow::Result<String> {
    let want = if required_major == 0 {
        8
    } else {
        required_major
    };

    // 구버전(Java 8 계열): 관리 런타임(21)은 오히려 실행 불가 — 시스템 Java 우선
    if want <= 8 {
        if !config.java_path.trim().is_empty() {
            return Ok(config.java_path.clone());
        }
        if let Some(sys) = find_java() {
            return Ok(sys);
        }
        anyhow::bail!("Java 8이 필요하지만 찾지 못했습니다 — 설정에서 Java 경로를 지정하세요");
    }

    if !config.java_path.trim().is_empty() {
        if java_major_ok(&config.java_path, want) {
            return Ok(config.java_path.clone());
        }
        tracing::warn!("지정 Java가 요구치(Java {want}+) 미달 — 자동 선택으로 전환");
    }
    let managed = managed_java_exe(&config.game_root);
    if managed.exists() && java_major_ok(&managed.to_string_lossy(), want) {
        return Ok(managed.to_string_lossy().to_string());
    }
    if let Some(sys) = find_java() {
        if java_major_ok(&sys, want) {
            return Ok(sys);
        }
    }
    tracing::info!("Java {want}+ 필요 — 관리 JRE 21 설치");
    ensure_java_21(client, &config.game_root).await
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
    // 마커 + 실행 검증 통과 시에만 재사용 (깨진 잔재는 재설치)
    if managed_marker(root).exists()
        && java_exe.exists()
        && java_major_ok(&java_exe.to_string_lossy(), 21)
    {
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
    // 설치 검증: 실행 + 버전 확인 (통과 못 하면 마커 없이 실패)
    let ver = java_version(&java_exe.to_string_lossy()).unwrap_or_default();
    if !java_major_ok(&java_exe.to_string_lossy(), 21) {
        anyhow::bail!("관리 JRE 검증 실패 (실행 불가: {ver})");
    }
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

/// dest直下가 단일 폴더면 그 안의 내용 전체를 dest로 승격
/// (bin만 올리면 lib/conf가 빠져 java가 실행 불가 — 반드시 통째로 이동)
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
        let nested = &dirs[0];
        if nested.join("bin").join(exe).exists() {
            for e in
                std::fs::read_dir(nested).map_err(|e| anyhow::anyhow!("런타임 탐색 실패: {e}"))?
            {
                let e = e.map_err(|e| anyhow::anyhow!("런타임 탐색 실패: {e}"))?;
                let target = dest.join(e.file_name());
                if !target.exists() {
                    std::fs::rename(e.path(), &target)
                        .map_err(|e| anyhow::anyhow!("런타임 승격 실패: {e}"))?;
                }
            }
            let _ = std::fs::remove_dir_all(nested);
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
