# Nebulya 보안 릴리스 작업 목록

이 문서는 Nebulya의 인증 방식과 GitHub Actions 릴리스 파이프라인을
안전하게 정리하기 위한 작업 기준입니다.

## 로그인

- [x] Microsoft Device Code 로그인 (브라우저 코드 입력 방식)
- [x] 배포 빌드에 public client ID 내장 (사용자 Azure 작업 없음)
- [x] access token 로그 마스킹
- [x] 정품 세션 갱신 실패 시 실행 중단 (오프라인 폴백 없음)
- [x] 로그아웃 시 로컬 계정 삭제
- [ ] OAuth Authorization Code + PKCE 전환 (향후)
- [ ] refresh token OS 자격 증명 저장소 보관 (현재 config.json 평문)
- [ ] entitlements 명시 확인 + 서버 측 토큰 revoke

> Microsoft 인증을 사용하려면 런처 배포자에만 한 번 앱 등록(public client)이
> 필요합니다. client ID는 비밀 값이 아니지만 GitHub secret으로 취급하지 않고
> 빌드 변수 또는 저장소 변수에서 주입합니다. client secret은 네이티브 앱에
> 넣지 않습니다.

## GitHub Actions

- [x] PR/push 빌드는 `contents: read`만 사용
- [x] `contents: write`는 태그 release job에만 부여
- [x] 모든 외부 Action을 full commit SHA로 고정
- [x] `Cargo.lock`을 커밋하여 재현 가능한 빌드 사용
- [x] Windows EXE/Setup, Linux tar.gz/.deb, macOS tar.gz/.dmg 생성
- [x] 릴리스 자산에 SHA-256 체크섬 생성
- [ ] SBOM 및 provenance/attestation 적용 가능 여부 확인
- [ ] Dependabot으로 Rust와 GitHub Actions 의존성 업데이트
- [ ] PR/태그 실행에 스크립트 인젝션 방지용 환경변수 사용

## 릴리스 전 확인

- [x] `cargo fmt --check`
- [x] `cargo check --locked --all-targets`
- [x] `cargo test --locked`
- [x] GitHub Actions의 Windows/macOS/Linux 빌드 성공 (v7.4.0 릴리스)
- [x] 설치 파일에 실행 권한 및 메타데이터 포함 (ISS + deb assets + tar)
- [x] 태그와 `Cargo.toml` 버전이 일치하는지 확인
- [ ] 공개 배포 전 Windows 코드 서명 인증서 준비

## 현재 진행 상황

- [x] `Cargo.lock` 생성 및 추적 시작
- [x] 로컬 저장소를 원격 브랜치에 연결
- [x] 인증 코드 보안 개선 (토큰 마스킹, 실패 시 중단, 파일명 검증)
- [x] 보안 CI/release workflow 코드 작성
- [x] GitHub Actions 실행 성공 확인 (v7.4.0)
- [x] GitHub Release 자동화 검증 (v7.4.0 파일 7종 공개됨)
