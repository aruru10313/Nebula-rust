# Nebulya 서명 가이드

## 현재 상태

- 릴리스 워크플로우는 서명용 인증서 secret이 없으면 서명을 건너뛰고 정상 릴리스합니다.
- 서명 없이 배포하면 Windows에서 SmartScreen 경고가 뜰 수 있습니다.
  이는 정상이며, 서명 자체를 코드로 "알아서" 우회할 수는 없습니다.

## Windows Authenticode 서명

1. 코드 서명 인증서를 발급받습니다.
   - SmartScreen 평판을 바로 얻으려면 EV 인증서가 가장 좋습니다 (OV도 가능, 평판 누적 필요).
   - 테스트용이면 self-signed 인증서도 됩니다 (단, 경고는 그대로 뜹니다).
2. 인증서를 `.pfx`로 내보낸 뒤 Base64로 변환합니다.
   ```powershell
   [Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.pfx")) | Set-Content cert-base64.txt
   ```
3. GitHub 저장소 → Settings → Secrets → Actions에 등록합니다.
   - `WINDOWS_CERT_BASE64`: 위 Base64 문자열
   - `WINDOWS_CERT_PASSWORD`: pfx 비밀번호
4. 다음 태그 릴리스부터 `exe` + `Setup exe`가 자동 서명됩니다
   (SHA-256 + DigiCert 타임스탬프, 서명 후 `verify` 검증).

## macOS 서명/공증

Apple Developer Program 가입(유료)이 필요합니다. 계정이 생기면
`release.yml`에 `codesign` + `notarytool` 단계를 추가할 수 있습니다.
계정 없이는 Gatekeeper 경고를 피할 방법이 없습니다.

## 참고

- 개인 키·pfx를 저장소나 채팅에 올리지 마세요. Secrets에만 넣습니다.
- 인증서 만료일을 주기적으로 확인하세요.
