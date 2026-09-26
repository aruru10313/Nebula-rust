# Nebulya Launcher — Rust 기반 Fabric 런처

Rust 네이티브 마인크래프트 **Fabric 전용** 런처입니다.
별(스텔라) 컨셉의 다크 UI + 단일 바이너리 GUI (`eframe` + `egui`, WebView 불필요).

- Microsoft 정품 로그인 (Device Code → Xbox → MC Services, 심사 승인 후 활성화)
- Nebulya 자체 계정 (이메일 인증 + 서버 세션)
- Mojang 버전 매니페스트 + Fabric Meta API 연동 (메타 캐시로 오프라인 실행 지원)
- **Modrinth + CurseForge (Fabric 강제 필터)** 검색·설치·활성화/삭제
- **Discord Activity** (상태 반영)
- Java 자동탐지 + Adoptium JRE 21 자동 설치 + RAM/해상도 설정
- 인스턴스 관리 (생성/복제/이름 변경/삭제 확인/폴더 열기)
- 게임 로그 캡처·회전 보관 + 런처 로그 파일
- 한글 번들 폰트 (NotoSansKR 서브셋, tofu 방지 + 커버리지 테스트)
- 인앱 자동 업데이트 (GitHub Releases 확인 → Setup 실행)
- Win/Linux/macOS 설치 파일 자동 생성 (Setup exe / .deb+tar.gz / .dmg+tar.gz)

저장소: https://github.com/aruru10313/Nebula-rust

## 실행

```bash
cargo run --release
```

필요 조건: Rust stable (rust-toolchain.toml 고정), Java 17+ (없으면 설정에서 자동 설치)

## 로그인

- 계정 탭에서 Nebulya 로그인/가입 (이메일 인증 코드 방식)
- Microsoft 정품 로그인은 API 심사 승인 후 복구 예정
- 로그인 없이 게스트 실행도 가능 (정품 서버 접속 불가)

## 모드 (Fabric 전용)

- 모드 탭 → 제공자 선택(Modrinth / CurseForge) → 검색 → 설치
- 검색/설치는 **선택된 인스턴스의 MC 버전 + fabric 로더** 기준으로 필터됨
- 설치된 모드는 `~/.nebulya-launcher/instances/<id>/mods/*.jar` 가 실제 기준
  - `끄기` = `.jar.disabled` 로 변경 (Prism/Dawn과 동일 규칙)
- CurseForge는 API 키 필요: https://console.curseforge.com/ 발급 후
  연동 카드 입력 또는 환경변수 `NEBULYA_CF_API_KEY`

## 구조

```
src/
  main.rs              # eframe 진입점 (릴리스에서 콘솔 숨김)
  core/
    config.rs          # ~/.nebulya-launcher/config.json
    instance.rs        # 인스턴스 + InstalledMod(Modrinth/CurseForge 메타)
  minecraft/
    version.rs         # piston-meta 버전 매니페스트
    fabric.rs          # meta.fabricmc.net
    auth.rs            # MS Device Flow + 세션 관리
    nebula_auth.rs     # Nebulya 자체 계정 API 클라이언트
    java.rs            # Java 탐색 + Adoptium JRE 자동 설치
    launcher.rs        # 다운로드(검증+원자 교체) → classpath → java spawn
    modrinth.rs        # Modrinth 검색/버전/설치 (Fabric 강제)
    curseforge.rs      # CurseForge 검색/파일/설치 (modLoaderType=Fabric)
    mods.rs            # 공용 다운로드 + 토글/삭제 + 파일명 검증
    update.rs          # 인앱 자동 업데이트 확인/적용
    discord.rs         # Discord Rich Presence
  ui/
    theme.rs           # 다크 테마 + 공용 위젯
    app.rs             # 사이드바 + 커스텀 타이틀바 + 상단바 + 상태바
    screens/
      home.rs instances.rs mods.rs account.rs settings.rs
installer/windows/nebulya-setup.iss  # Inno Setup (Windows 설치 exe)
scripts/
  package-linux.sh     # tar.gz + .deb
  package-macos.sh     # .app tar.gz + .dmg
.github/workflows/
  ci.yml               # PR/push 검증 (가벼운 check 중심)
  release.yml          # 태그 릴리스 (3OS 빌드 + draft 검증 후 공개)
```

## 설치 파일 만들기

```bash
# Linux (tar.gz + .deb)
cargo build --release --target x86_64-unknown-linux-gnu
bash scripts/package-linux.sh x86_64-unknown-linux-gnu <버전>

# macOS (tar.gz + .dmg, dmg는 macOS에서만)
cargo build --release --target aarch64-apple-darwin
bash scripts/package-macos.sh aarch64-apple-darwin <버전>

# Windows (Inno Setup 필요)
# iscc installer/windows/nebulya-setup.iss /DMyAppVersion=<버전> /DBinaryDir=target\x86_64-pc-windows-msvc\release
```

릴리스: `Cargo.toml` 버전과 일치하는 태그를 푸시하면 GitHub Actions가
3OS 빌드+패키징 후 draft 릴리스를 검증하고 공개합니다.

```bash
git tag vX.Y.Z && git push origin vX.Y.Z
```

## 로드맵

- [x] Vanilla/Fabric 실행 (온라인 + 오프라인 캐시)
- [x] Nebulya 자체 계정 + 자동 갱신되는 MS 세션 구조
- [x] Modrinth / CurseForge Fabric 모드 관리
- [x] Discord Activity
- [x] 3OS 설치 파일 + 인앱 자동 업데이트
- [x] 커스텀 타이틀바, 게임 로그 캡처
- [ ] MS 정품 로그인 복구 (API 심사 승인 후)
- [ ] 크래시 리포트 고도화

## 크레딧

- Minecraft 메타데이터 모델은 Modrinth의 [`daedalus`](https://github.com/modrinth/daedalus) 크레이트(MIT)를 그대로 의존성으로 사용합니다.
- 실행 파이프라인(재시도·캐시 폴백·관리 Java)의 설계도 [Modrinth App](https://github.com/modrinth/code)의 방식을 참조했습니다. 해당 부분의 코드는 Nebulya가 독자적으로 작성했으며, Modrinth의 GPL 코드·브랜딩을 포함하지 않습니다.
