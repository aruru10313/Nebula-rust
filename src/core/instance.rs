use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 인스턴스 = 마크 버전 + 로더 조합 1개 (Lunar의 프로필 / Prism의 인스턴스 개념)
/// Fabric 전용 런처이므로 로더는 기본 Fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderType,
    pub loader_version: String,
    pub last_played: Option<DateTime<Utc>>,
    pub total_plays: u32,
    /// 설치된 모드 목록 (Modrinth / CurseForge)
    #[serde(default, deserialize_with = "deserialize_mods")]
    pub mods: Vec<InstalledMod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LoaderType {
    #[default]
    Fabric,
    Vanilla,
    // 추후: Forge, NeoForge, Quilt
}

impl LoaderType {
    pub fn as_str(self) -> &'static str {
        match self {
            LoaderType::Fabric => "Fabric",
            LoaderType::Vanilla => "Vanilla",
        }
    }
}

/// 모드 제공자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModSource {
    #[default]
    Modrinth,
    CurseForge,
}

impl ModSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ModSource::Modrinth => "Modrinth",
            ModSource::CurseForge => "CurseForge",
        }
    }
}

/// 인스턴스에 설치된 모드 1개
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub source: ModSource,
    /// Modrinth slug/project_id 또는 CurseForge mod id
    pub project_id: String,
    pub title: String,
    /// 설치된 파일명 (mods 폴더 기준)
    pub file_name: String,
    /// 버전 표시 (version_number / file display name)
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// 구버전 호환: 과거 `mods: ["id", ...]` 문자열 배열도 읽어서 Modrinth 항목으로 변환
fn deserialize_mods<'de, D>(d: D) -> Result<Vec<InstalledMod>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Full(InstalledMod),
        Legacy(String),
    }
    let list = Vec::<Entry>::deserialize(d).map_err(serde::de::Error::custom)?;
    Ok(list
        .into_iter()
        .map(|e| match e {
            Entry::Full(m) => m,
            Entry::Legacy(id) => InstalledMod {
                source: ModSource::Modrinth,
                project_id: id.clone(),
                title: id.clone(),
                file_name: String::new(),
                version: String::new(),
                enabled: true,
            },
        })
        .collect())
}

impl Instance {
    pub fn new_fabric(name: String, mc_version: String, loader_version: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            minecraft_version: mc_version,
            loader: LoaderType::Fabric,
            loader_version,
            last_played: None,
            total_plays: 0,
            mods: vec![],
        }
    }

    /// 인스턴스 전용 게임 폴더
    pub fn game_dir(&self, root: &std::path::Path) -> std::path::PathBuf {
        root.join("instances").join(&self.id)
    }

    /// 모드 폴더 (없으면 생성)
    pub fn mods_dir(&self, root: &std::path::Path) -> std::path::PathBuf {
        let dir = self.game_dir(root).join("mods");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// 실제 mods 폴더 안의 jar 목록 (.jar = 활성, .jar.disabled = 비활성)
    pub fn scan_mod_files(&self, root: &std::path::Path) -> Vec<(String, bool)> {
        let dir = self.game_dir(root).join("mods");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return vec![];
        };
        let mut out = vec![];
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") {
                out.push((name, true));
            } else if name.ends_with(".jar.disabled") {
                out.push((name, false));
            }
        }
        out.sort();
        out
    }

    pub fn display_version(&self) -> String {
        match self.loader {
            LoaderType::Fabric => {
                format!(
                    "{} + Fabric {}",
                    self.minecraft_version, self.loader_version
                )
            }
            LoaderType::Vanilla => self.minecraft_version.clone(),
        }
    }
}

/// 인스턴스 목록 저장/로드 (`instances.json`)
pub fn load_instances(root: &std::path::Path) -> Vec<Instance> {
    let path = root.join("instances.json");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(list) = serde_json::from_slice::<Vec<Instance>>(&bytes) {
            return list;
        }
    }
    // 첫 실행 기본 인스턴스 1개
    vec![Instance::new_fabric(
        "Nebulya Fabric".to_string(),
        "1.20.1".to_string(),
        "0.16.9".to_string(),
    )]
}

pub fn save_instances(root: &std::path::Path, list: &[Instance]) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    let path = root.join("instances.json");
    let json = serde_json::to_string_pretty(list)?;
    std::fs::write(path, json)?;
    Ok(())
}
