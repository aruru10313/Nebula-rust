# Nebulya Launcher — Rust 기반 Fabric 런처 (v0.2)

Lunar Client / 구 Feather Client / Dawn Launcher 스타일을 목표로 한
Rust 네이티브 마인크래프트 **Fabric 전용** 런처입니다.

- 단일 바이너리 GUI (`eframe` + `egui`, WebView 불필요 → GitHub Actions 빌드 간단)
- Mojang 버전 매니페스트 + Fabric Meta API 연동
- **Modrinth + CurseForge (Fabric 강제 필터)** 검색·설치·활성화/삭제
- **Discord Activity** (대기 중 / 검색 중 / 플레이 중 / 설정 중)
- Java 자동탐지 + RAM/해상도 설정 + 인스턴스(프로필) 관리
- Win/Linux/macOS 설치 파일 자동 생성 (Setup exe / .deb+tar.gz / .dmg+tar.gz)

저장소: https://github.com/aruru10313/Nebula-rust

## 실행

```bash
cargo run --release
```

필요 조건: Rust stable, Java 17+ (마인크 1.20+ 기준)

## 모드 (Fabric 전용)

- 모드 탭 → 제공자 선택(Modrinth / CurseForge) → 검색 → 설치
- 검색/설치는 **선택된 인스턴스의 MC 버전 + fabric 로더** 기준으로 필터됨
- 설치된 모드는 `~/.nebulya-launcher/instances/<id>/mods/*.jar` 가 실제 기준
  - `끄기` = `.jar.disabled` 로 변경 (Prism/Dawn과 동일 규칙)
- CurseForge는 API 키 필요: https://console.curseforge.com/ 발급 후
  설정 탭 입력 또는 환경변수 `NEBULYA_CF_API_KEY`

## Discord Activity

1. https://discord.com/developers/applications 에서 앱 생성
2. Client ID를 설정 탭 → Discord Activity에 입력 → 저장
3. 디스코드가 켜져 있으면 홈/검색/플레이/설정 상태가 표시됨
4. 디스코드가 꺼져 있으면 조용히 비활성화 (런처 정상 동작)

## 구조

```
src/
  main.rs              # eframe 진입점
  core/
    config.rs          # ~/.nebulya-launcher/config.json (+CF키/Discord 설정)
    instance.rs        # 인스턴스 + InstalledMod(Modrinth/CurseForge 메타)
  minecraft/
    version.rs         # piston-meta 버전 매니페스트
    fabric.rs          # meta.fabricmc.net
    auth.rs            # 오프라인 세션 + MS Device Flow 뼈대
    java.rs            # Java 탐색
    launcher.rs        # 다운로드 → classpath → java spawn
    modrinth.rs        # Modrinth 검색/버전/설치 (Fabric 강제)
    curseforge.rs      # CurseForge 검색/파일/설치 (modLoaderType=Fabric)
    mods.rs            # 공용 다운로드 + 토글/삭제
    discord.rs         # Discord Rich Presence
  ui/
    theme.rs           # Lunar/Dawn식 다크 테마
    app.rs             # 사이드바 + 상단바 + 상태바 + Discord 상태 반영
    screens/
      home.rs instances.rs mods.rs settings.rs
installer/windows/nebulya-setup.iss  # Inno Setup (Windows 설치 exe)
scripts/
  package-linux.sh     # tar.gz + .deb
  package-macos.sh     # .app tar.gz + .dmg
.github/workflows/build.yml  # 3OS 빌드 + 패키징 + Release
```

## 설치 파일 만들기

```bash
# Linux (tar.gz + .deb)
cargo build --release --target x86_64-unknown-linux-gnu
bash scripts/package-linux.sh x86_64-unknown-linux-gnu

# macOS (tar.gz + .dmg, dmg는 macOS에서만)
cargo build --release --target aarch64-apple-darwin
bash scripts/package-macos.sh aarch64-apple-darwin

# Windows (Inno Setup 필요)
# iscc installer/windows/nebulya-setup.iss /DBinaryDir=target\x86_64-pc-windows-msvc\release
```

GitHub Actions가 `main` 푸시 / `v*` 태그마다 3OS 빌드+패키징을 자동 수행하고,
태그 푸시 시 Release에 설치 파일을 첨부합니다.

## 로드맵

- [x] 기초 틀 + Vanilla/Fabric 실행 (오프라인)
- [x] Modrinth / CurseForge Fabric 모드 관리
- [x] Discord Activity
- [x] 3OS 설치 파일 틀
- [ ] MS OAuth 로그인 (Client ID 발급 후 `auth.rs` 활성화)
- [ ] 실시간 게임 로그 스트리밍 + 크래시 리포트
- [ ] 디자인 고도화 (커스텀 타이틀바, 애니메이션, 테마)
