//! 실제 게임 실행: 파일 다운로드 → classpath 구성 → java spawn
//! PrismLauncher / vanilla 방식과 동일한 규칙 기반.

use crate::core::{Instance, LauncherConfig, LoaderType};
use crate::minecraft::auth::MinecraftSession;
use crate::minecraft::version::{Library, VersionJson};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct LaunchProgress {
    pub step: String,
    pub done: u64,
    pub total: u64,
}

pub type ProgressCb = Box<dyn Fn(LaunchProgress) + Send + Sync>;

/// 인스턴스 실행 (blocking spawn 전 준비). 로그는 호출자가 받아서 UI에 표시.
pub async fn prepare_and_launch(
    config: &LauncherConfig,
    instance: &mut Instance,
    session: &MinecraftSession,
    on_progress: impl Fn(LaunchProgress) + Send + Sync + 'static,
) -> Result<std::process::Child> {
    let client = reqwest::Client::builder()
        .user_agent("nebulya-launcher/0.1")
        .build()?;

    on_progress(LaunchProgress {
        step: "버전 정보 조회 중...".into(),
        done: 0,
        total: 1,
    });

    // 1. Mojang manifest → 해당 버전 url 찾기 (오프라인이면 캐시 사용)
    let root = &config.game_root;
    let manifest_path = root.join("manifest_cache.json");
    let manifest = match crate::minecraft::version::fetch_manifest(&client).await {
        Ok(m) => {
            let _ = save_cache(&manifest_path, &m);
            m
        }
        Err(e) => {
            tracing::warn!("매니페스트 조회 실패, 캐시 사용: {e:#}");
            load_cache(&manifest_path).context("오프라인 상태이며 버전 캐시가 없습니다")?
        }
    };
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == instance.minecraft_version)
        .with_context(|| format!("지원하지 않는 버전: {}", instance.minecraft_version))?;
    let version_path = root
        .join("versions")
        .join(&instance.minecraft_version)
        .join(format!("{}.json", instance.minecraft_version));
    let mut version_json =
        match crate::minecraft::version::fetch_version_json(&client, &entry.url).await {
            Ok(v) => {
                let _ = save_cache(&version_path, &v);
                v
            }
            Err(e) => {
                tracing::warn!("버전 JSON 조회 실패, 캐시 사용: {e:#}");
                load_cache(&version_path).context("오프라인 상태이며 버전 캐시가 없습니다")?
            }
        };

    let game_dir = instance.game_dir(root);
    std::fs::create_dir_all(&game_dir)?;
    std::fs::create_dir_all(root.join("libraries"))?;
    std::fs::create_dir_all(root.join("assets"))?;

    // 2. Fabric이면 loader 주입 (mainClass 교체 + 라이브러리 추가)
    let mut fabric_extra_libs: Vec<Library> = vec![];
    let mut main_class = version_json.main_class.clone();

    if instance.loader == LoaderType::Fabric {
        on_progress(LaunchProgress {
            step: format!("Fabric {} 확인 중...", instance.loader_version),
            done: 0,
            total: 1,
        });
        // 로더 메타도 캐시 (오프라인 실행용)
        let meta_path = root.join("fabric-meta").join(format!(
            "{}-{}.json",
            instance.minecraft_version, instance.loader_version
        ));
        let found = match crate::minecraft::fabric::fetch_loaders_for_game(
            &client,
            &instance.minecraft_version,
        )
        .await
        {
            Ok(loaders) => {
                let found = loaders
                    .iter()
                    .find(|l| l.loader.version == instance.loader_version)
                    .or_else(|| loaders.iter().find(|l| l.loader.stable))
                    .context("Fabric loader 정보를 못 찾음")?
                    .clone();
                let _ = save_cache(&meta_path, &found);
                found
            }
            Err(e) => {
                tracing::warn!("Fabric 메타 조회 실패, 캐시 사용: {e:#}");
                load_cache(&meta_path).context("오프라인 상태이며 Fabric 캐시가 없습니다")?
            }
        };

        // launcherMeta.libraries.{common,client} → Library로 변환은 복잡하므로
        // phase 1에서는 maven 좌표 → URL로 직접 다운로드하는 단순 방식 사용
        // (fabric-loader, intermediary, tiny-mappings-parser 등 launcherMeta 전체는 phase 2에서 정식 파싱)
        main_class = extract_main_class(&found.launcher_meta.main_class)
            .unwrap_or_else(|| "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string());

        fabric_extra_libs =
            launcher_meta_to_libraries(&found.launcher_meta.libraries).unwrap_or_default();

        // 최소 보장: loader + intermediary maven 좌표 추가
        ensure_maven_artifact(&client, root, &found.loader.maven, &on_progress).await?;
        ensure_maven_artifact(&client, root, &found.intermediary.maven, &on_progress).await?;
    }
    version_json.main_class = main_class.clone();

    // 3. client.jar 다운로드
    let client_jar = root
        .join("versions")
        .join(&version_json.id)
        .join(format!("{}.jar", version_json.id));
    download_file(
        &client,
        &version_json.downloads.client.url,
        &client_jar,
        Some(&version_json.downloads.client.sha1),
    )
    .await?;

    // 4. libraries 다운로드 (rules 필터링)
    let all_libs: Vec<Library> = version_json
        .libraries
        .iter()
        .cloned()
        .chain(fabric_extra_libs)
        .collect();

    let mut classpath_entries: Vec<PathBuf> = vec![client_jar];
    let mut total = all_libs.len() as u64;
    if total == 0 {
        total = 1;
    }

    for (i, lib) in all_libs.iter().enumerate() {
        if !library_allowed(lib) {
            continue;
        }
        on_progress(LaunchProgress {
            step: format!("라이브러리 ({}/{})", i + 1, all_libs.len()),
            done: i as u64,
            total,
        });

        if let Some(dl) = &lib.downloads {
            // 일반 artifact
            if let Some(artifact) = &dl.artifact {
                let path = root.join("libraries").join(&artifact.path);
                // classifiers 중 natives가 있으면 해당 OS 것만
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(native_key) = native_classifier_key() {
                        if let Some(native) = classifiers.get(native_key) {
                            let npath = root.join("libraries").join(&native.path);
                            download_file(&client, &native.url, &npath, Some(&native.sha1)).await?;
                            extract_natives(&npath, &natives_dir(root, &version_json.id))?;
                        }
                    }
                }
                download_file(&client, &artifact.url, &path, Some(&artifact.sha1)).await?;
                classpath_entries.push(path);
            } else if let Some(classifiers) = &dl.classifiers {
                // artifact 없이 classifier만 있는 경우 (구 natives)
                if let Some(native_key) = native_classifier_key() {
                    if let Some(native) = classifiers.get(native_key) {
                        let npath = root.join("libraries").join(&native.path);
                        download_file(&client, &native.url, &npath, Some(&native.sha1)).await?;
                        extract_natives(&npath, &natives_dir(root, &version_json.id))?;
                    }
                }
            }
        } else {
            // downloads 없는 구형 lib (minecraftforge 등) → maven 좌표로 시도
            let _ = ensure_maven_artifact(&client, root, &lib.name, &on_progress).await;
            if let Some(p) = maven_to_path(root, &lib.name) {
                if p.exists() {
                    classpath_entries.push(p);
                }
            }
        }
    }

    // maven으로 받은 fabric jar들도 classpath에 추가
    for p in collect_maven_jars(root)? {
        if !classpath_entries.contains(&p) {
            // fabric 관련만 (과도한 추가 방지: loader/intermediary 포함 여부로 필터)
            let s = p.to_string_lossy().to_string();
            if s.contains("fabric") || s.contains("intermediary") || s.contains("tiny-") {
                classpath_entries.push(p);
            }
        }
    }

    // 5. assets 다운로드 (assetIndex + objects — 최소: index json만, objects는 lazy)
    download_assets(&client, root, &version_json).await?;

    // 6. java args 구성
    // Windows에서는 콘솔 창이 뜨지 않는 javaw.exe를 우선 사용
    let java = {
        let j = config.effective_java();
        if cfg!(windows) {
            let w = j
                .strip_suffix("java.exe")
                .map(|s| format!("{s}javaw.exe"))
                .or_else(|| j.strip_suffix("java").map(|s| format!("{s}javaw.exe")));
            match w {
                Some(w) if std::path::Path::new(&w).exists() => w,
                _ => j,
            }
        } else {
            j
        }
    };
    let natives = natives_dir(root, &version_json.id);
    let cp_sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath_entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(cp_sep);

    let asset_id = version_json.asset_index.id.clone();
    let assets_dir = root.join("assets");

    let mut args: Vec<String> = vec![
        format!("-Xmx{}M", config.ram_mb),
        format!("-Xms{}M", config.min_ram_mb),
        format!("-Djava.library.path={}", natives.to_string_lossy()),
        "-cp".to_string(),
        classpath,
        main_class,
        "--username".to_string(),
        session.username.clone(),
        "--version".to_string(),
        version_json.id.clone(),
        "--gameDir".to_string(),
        game_dir.to_string_lossy().to_string(),
        "--assetsDir".to_string(),
        assets_dir.to_string_lossy().to_string(),
        "--assetIndex".to_string(),
        asset_id,
        "--uuid".to_string(),
        session.uuid.clone(),
        "--accessToken".to_string(),
        session.access_token.clone(),
        "--userType".to_string(),
        if session.offline {
            "legacy".to_string()
        } else {
            "msa".to_string()
        },
        "--versionType".to_string(),
        "Nebulya".to_string(),
        "--width".to_string(),
        config.width.to_string(),
        "--height".to_string(),
        config.height.to_string(),
    ];

    // 구버전(1.13 미만 minecraftArguments 방식)은 phase 2에서 별도 처리
    let _ = &mut args;

    // 토큰이 로그에 남지 않게 마스킹 후 기록
    let logged_args: Vec<String> = args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i > 0 && args[i - 1] == "--accessToken" {
                "***".to_string()
            } else {
                a.clone()
            }
        })
        .collect();
    tracing::info!("launch: {} {}", java, logged_args.join(" "));

    on_progress(LaunchProgress {
        step: "게임 실행 중...".into(),
        done: total,
        total,
    });

    let child = std::process::Command::new(&java)
        .args(&args)
        .current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Java 실행 실패: {java}"))?;

    // 파이프가 차서 게임이 멈추지 않게 출력을 latest.log로 흘려보낸다
    Ok(drain_child_output(child, &game_dir))
}

/// 자식 프로세스의 stdout/stderr를 `logs/latest.log`에 append하는 스레드 분리 후 반환.
/// 로그는 실행마다 새로 쓴다.
fn drain_child_output(
    mut child: std::process::Child,
    game_dir: &std::path::Path,
) -> std::process::Child {
    let log_path = game_dir.join("logs").join("latest.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    rotate_game_logs(&log_path);
    // 새 실행이므로 기존 로그를 비운다
    let _ = std::fs::write(&log_path, "");
    if let Some(out) = child.stdout.take() {
        let path = log_path.clone();
        std::thread::spawn(move || append_lines(out, &path));
    }
    if let Some(err) = child.stderr.take() {
        let path = log_path.clone();
        std::thread::spawn(move || append_lines(err, &path));
    }
    child
}

/// 이전 latest.log를 타임스탬프 이름으로 보관 (최대 10개 유지)
fn rotate_game_logs(latest: &std::path::Path) {
    if !latest.exists() {
        return;
    }
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    if let Some(parent) = latest.parent() {
        let archived = parent.join(format!("game-{stamp}.log"));
        let _ = std::fs::rename(latest, &archived);
        // 오래된 로그 정리 (latest + 최대 10개)
        if let Ok(entries) = std::fs::read_dir(parent) {
            let mut logs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("game-") && n.ends_with(".log"))
                        .unwrap_or(false)
                })
                .collect();
            logs.sort();
            while logs.len() > 10 {
                if let Some(old) = logs.first() {
                    let _ = std::fs::remove_file(old);
                }
                logs.remove(0);
            }
        }
    }
}

fn append_lines(pipe: impl std::io::Read + Send + 'static, path: &std::path::Path) {
    use std::io::{BufRead, Write};
    let reader = std::io::BufReader::new(pipe);
    // append 모드로 열고 한 줄씩 기록 (두 스레드가 같은 파일에 씀)
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

// ---------- helpers ----------

/// JSON 캐시 저장 (오프라인 실행용)
fn save_cache(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// JSON 캐시 로드
fn load_cache<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path).with_context(|| format!("캐시 없음: {}", path.display()))?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn natives_dir(root: &Path, version_id: &str) -> PathBuf {
    root.join("versions").join(version_id).join("natives")
}

fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn library_allowed(lib: &Library) -> bool {
    let Some(rules) = &lib.rules else {
        return true;
    };
    // 마지막으로 매칭되는 rule이 우선 (vanilla 규칙)
    let mut allowed = false;
    for rule in rules {
        let os_match = match &rule.os {
            None => true,
            Some(os) => match &os.name {
                None => true,
                Some(n) => n == current_os(),
            },
        };
        if os_match {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn native_classifier_key() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("natives-windows")
    } else if cfg!(target_os = "macos") {
        // arm64/x64 구분은 phase 2 (현재 x86_64 기본)
        Some("natives-macos")
    } else {
        Some("natives-linux")
    }
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1_expected: Option<&str>,
) -> Result<()> {
    if dest.exists() {
        if let Some(expected) = sha1_expected {
            if verify_sha1(dest, expected).unwrap_or(false) {
                return Ok(());
            }
            // 불일치 → 재다운로드
            let _ = std::fs::remove_file(dest);
        } else {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // 임시 파일에 내려받은 뒤 해시 검증 → 원자적 교체 (중단 시 깨진 파일이 남지 않음)
    let tmp = dest.with_extension("nebpart");
    let res = client.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(&tmp).await?;
    use futures_util::StreamExt;
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = sha1_expected {
        if !verify_sha1(&tmp, expected).unwrap_or(false) {
            let _ = std::fs::remove_file(&tmp);
            anyhow::bail!("다운로드 무결성 검증 실패: {url}");
        }
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

fn verify_sha1(path: &Path, expected: &str) -> Result<bool> {
    use sha1::{Digest, Sha1};
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());
    Ok(hash.eq_ignore_ascii_case(expected))
}

fn extract_natives(jar: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let file = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(file)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        // META-INF 제외
        if name.starts_with("META-INF") || name.ends_with('/') {
            continue;
        }
        // Zip Slip 방지: dest 바깥으로 나가는 경로는 건너뜀
        let out = dest.join(&name);
        let normalized: PathBuf = out
            .components()
            .filter(|c| {
                !matches!(
                    c,
                    std::path::Component::ParentDir
                        | std::path::Component::Prefix(_)
                        | std::path::Component::RootDir
                )
            })
            .collect();
        if normalized != out {
            tracing::warn!("위험한 natives 경로 건너뜀: {name}");
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out_file = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

/// maven 좌표(net.fabricmc:fabric-loader:0.16.9) → libraries 경로
fn maven_to_path(root: &Path, coords: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let group_path = group.replace('.', "/");
    Some(
        root.join("libraries")
            .join(group_path)
            .join(artifact)
            .join(version)
            .join(format!("{artifact}-{version}.jar")),
    )
}

async fn ensure_maven_artifact(
    client: &reqwest::Client,
    root: &Path,
    coords: &str,
    on_progress: &(impl Fn(LaunchProgress) + Send + Sync),
) -> Result<PathBuf> {
    let dest = maven_to_path(root, coords).context("잘못된 maven 좌표")?;
    if dest.exists() {
        return Ok(dest);
    }
    // fabric / mojang / maven central 순서로 시도
    let parts: Vec<&str> = coords.split(':').collect();
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let rel = format!(
        "{}/{}/{}/{}-{}.jar",
        group.replace('.', "/"),
        artifact,
        version,
        artifact,
        version
    );
    let repos = [
        "https://maven.fabricmc.net/",
        "https://libraries.minecraft.net/",
        "https://repo1.maven.org/maven2/",
    ];
    on_progress(LaunchProgress {
        step: format!("다운로드 {coords}"),
        done: 0,
        total: 1,
    });
    for repo in repos {
        let url = format!("{repo}{rel}");
        match client.get(&url).send().await {
            Ok(res) if res.status().is_success() => {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let bytes = res.bytes().await?;
                std::fs::write(&dest, &bytes)?;
                return Ok(dest);
            }
            _ => continue,
        }
    }
    anyhow::bail!("maven artifact 다운로드 실패: {coords}")
}

fn collect_maven_jars(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    let lib_root = root.join("libraries");
    if !lib_root.exists() {
        return Ok(out);
    }
    // 단순 재귀 (depth 제한적)
    let mut stack = vec![lib_root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "jar").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

fn extract_main_class(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(map) => map.get("client").and_then(|c| match c {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Array(arr) => {
                arr.first().and_then(|x| x.as_str()).map(|s| s.to_string())
            }
            _ => None,
        }),
        _ => None,
    }
}

/// launcherMeta.libraries → Library 목록 (common + client 병합, rules 간소화)
fn launcher_meta_to_libraries(v: &serde_json::Value) -> Option<Vec<Library>> {
    let mut out: Vec<Library> = vec![];
    let obj = v.as_object()?;
    for key in ["common", "client"] {
        let arr = obj.get(key)?.as_array()?;
        for item in arr {
            let name = item.get("name")?.as_str()?.to_string();
            let url = item
                .get("url")?
                .as_str()
                .unwrap_or("https://maven.fabricmc.net/")
                .to_string();
            // maven path 유추
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let rel = format!(
                "{}/{}/{}/{}-{}.jar",
                parts[0].replace('.', "/"),
                parts[1],
                parts[2],
                parts[1],
                parts[2]
            );
            // sha1/size는 모르면 downloads 없이 maven 좌표만 유지 → ensure_maven_artifact에서 받음
            let _ = url;
            let _ = rel;
            out.push(Library {
                name,
                downloads: None,
                rules: None,
                natives: None,
                extract: None,
            });
        }
    }
    Some(out)
}

async fn download_assets(
    client: &reqwest::Client,
    root: &Path,
    version: &VersionJson,
) -> Result<()> {
    let indexes_dir = root.join("assets").join("indexes");
    std::fs::create_dir_all(&indexes_dir)?;
    let index_path = indexes_dir.join(format!("{}.json", version.asset_index.id));
    download_file(
        client,
        &version.asset_index.url,
        &index_path,
        Some(&version.asset_index.sha1),
    )
    .await?;
    // objects는 용량이 크므로 phase 1에서는 index만 받고,
    // 실제 objects 다운로드는 게임 실행 시 바닐라가 자동 복구하거나 phase 2에서 전체 미러링.
    // 최소 부팅용으로 index 존재만 보장.
    Ok(())
}
