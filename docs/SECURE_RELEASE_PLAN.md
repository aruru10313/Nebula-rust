# Nebulya 보안 릴리스 작업 목록

이 문서는 Nebulya의 인증 방식과 GitHub Actions 릴리스 파이프라인을
안전하게 정리하기 위한 작업 기준입니다.

## 로그인

- [ ] 공식 Minecraft Launcher와 같은 Microsoft 브라우저 로그인 UX 적용
- [ ] Azure Portal을 사용자가 방문하지 않도록 배포 빌드에 public client ID 주입
- [ ] OAuth Authorization Code + PKCE 및 `state` 검증 적용
- [ ] 비밀번호, device code, access token, refresh token을 로그에 출력하지 않기
- [ ] refresh token을 OS 자격 증명 저장소에 보관
- [ ] Minecraft 소유권 및 프로필 조회 후에만 게임 실행 허용
- [ ] 로그아웃 시 저장된 자격 증명과 세션 삭제

> Microsoft 인증을 사용하려면 런처 배포자에만 한 번 앱 등록(public client)이
> 필요합니다. client ID는 비밀 값이 아니지만 GitHub secret으로 취급하지 않고
> 빌드 변수 또는 저장소 변수에서 주입합니다. client secret은 네이티브 앱에
> 넣지 않습니다.

## GitHub Actions

- [ ] PR/push 빌드는 `contents: read`만 사용
- [ ] `contents: write`는 태그 release job에만 부여
- [ ] 모든 외부 Action을 full commit SHA로 고정
- [ ] `Cargo.lock`을 커밋하여 재현 가능한 빌드 사용
- [ ] Windows EXE/Setup, Linux tar.gz/.deb, macOS tar.gz/.dmg 생성
- [ ] 릴리스 자산에 SHA-256 체크섬 생성
- [ ] SBOM 및 provenance/attestation 적용 가능 여부 확인
- [ ] Dependabot으로 Rust와 GitHub Actions 의존성 업데이트
- [ ] PR/태그 실행에 스크립트 인젝션 방지용 환경변수 사용

## 릴리스 전 확인

- [ ] `cargo fmt --check`
- [ ] `cargo check --locked --all-targets`
- [ ] `cargo test --locked`
- [ ] GitHub Actions의 Windows/macOS/Linux 빌드 성공
- [ ] 설치 파일에 실행 권한 및 메타데이터가 올바른지 확인
- [ ] 태그와 `Cargo.toml` 버전이 일치하는지 확인
- [ ] 공개 배포 전 Windows 코드 서명 인증서 준비

## 현재 진행 상황

- [x] `Cargo.lock` 생성 및 추적 시작
- [x] 로컬 저장소를 원격 브랜치에 연결
- [ ] 인증 코드 보안 개선
- [ ] 분리된 CI/release workflow 작성
- [ ] GitHub Release 자동화
