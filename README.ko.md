# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>한 번의 접두어로 Codex CLI 작업을 끝까지 굴려 보세요.<br>
원하는 작업 앞에 <code>$cap</code>만 붙이면 CCC가 관리합니다.</em></p>

CCC는 Codex CLI를 위해 만든 Rust 기반 orchestration layer입니다. `$cap`을 공개 entrypoint로 유지하고, 내부 작업 흐름을 관리해서 더 큰 작업도 손이 덜 가게 이어갈 수 있게 돕습니다.

## 언제 쓰나

처음부터 끝까지 Codex가 맡아 처리하길 원할 때 CCC를 사용합니다. 기획, 편집, 검토, 재시작 후 인계처럼 여러 단계를 한 흐름으로 묶고 싶을 때 특히 잘 맞습니다.

## 설치, 업데이트, 삭제

기본 설치 경로:

```text
cargo install codex-cli-captain
ccc setup
```

그다음 Codex CLI를 완전히 종료하고 새 세션을 연 뒤 다음을 실행합니다.

```text
ccc check-install
```

기존 Cargo 설치를 업데이트할 때는 다음을 실행합니다.

```text
cargo install codex-cli-captain --force
ccc setup
```

그다음 Codex CLI를 다시 시작하고 `ccc check-install`로 확인합니다.

Cargo 설치를 삭제할 때는 다음을 실행합니다.

```text
cargo uninstall codex-cli-captain
```

CCC-managed 정리도 필요하면 먼저 `ccc uninstall --dry-run`으로 미리 확인하고, 내용이 맞을 때만 `ccc uninstall --confirm`를 실행합니다.

레거시 release-bundle fallback만 사용할 때:

```text
curl -fsSL https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.sh | bash
```

Windows PowerShell:

```text
iwr -UseB https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.ps1 | iex
```

## 기타 설정

| 설정 | 위치 | 메모 |
| --- | --- | --- |
| CCC role model, reasoning tier, fast-mode | `~/.config/ccc/ccc-config.toml` | 수정 후 `ccc setup`을 실행하고, Codex CLI를 재시작한 뒤 `ccc check-install`로 확인합니다. |
| Codex plugin hooks | `~/.codex/config.toml` | `[features] plugin_hooks = true`를 설정하고, Codex CLI를 재시작한 뒤 `/hooks` review에서 CCC hook을 승인하고 `ccc check-install`을 실행합니다. |

## 지원 OS

| OS | 상태 | 주의 |
| --- | --- | --- |
| macOS | 지원 | 설치 후 `ccc check-install`로 확인하세요. |
| Windows | 원칙적으로 지원 | 일부 환경에서는 정상 동작하지 않을 수 있으니 설치 후 확인하세요. |
| Linux | 원칙적으로 지원 | 일부 환경에서는 정상 동작하지 않을 수 있으니 설치 후 확인하세요. |
