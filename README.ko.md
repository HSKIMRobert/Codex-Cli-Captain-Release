# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI나 Codex App 작업을 끝까지 굴리고 싶나요?<br>
고가 모델로 모든 작업을 그대로 돌리는 비용이 걱정되나요?<br>
그렇다면 CCC를 써보는 건 어떨까요?<br>
원하는 작업 앞에 <code>$cap</code>만 붙이면 됩니다.<br>
그러면 꽤 놀라운 일이 펼쳐질 수 있습니다.</em></p>

현재 공개 릴리스: `0.0.3`.

CCC는 Codex CLI를 위한 captain-first orchestration layer입니다. `$cap`만 public entrypoint로 유지하고, LongWay/task-card/fan-in 상태를 저장하며, specialist 작업을 managed agent로 라우팅한 뒤 captain review로 합칩니다.

## 설치, 업데이트, 삭제

기본 설치 경로:

```text
Cargo로 Codex-Cli-Captain을 설치합니다:
cargo install codex-cli-captain

설치가 끝나면 다음을 실행합니다:
ccc setup

그다음 Codex CLI를 완전히 종료합니다.
새 Codex CLI 세션을 시작합니다.
그다음 다음을 실행합니다:
ccc check-install
```

Windows PowerShell도 같은 Cargo 기본 경로를 사용합니다.

기존 Cargo install을 업데이트할 때는 `cargo install codex-cli-captain --force`를 다시 실행한 뒤 `ccc setup`을 실행하세요. 그 다음 Codex CLI를 완전히 재시작하고 `ccc check-install`로 확인합니다.

`ccc check-install`은 hooks 준비 상태, 선택된 실행 경로, 그리고 공개
config 형태까지 함께 보여 줍니다. 공개 config 형태는 top-level
`version`, `entry_policy.mode`, 보이는 agent 항목의
`name`/`model`/`variant`/`fast_mode`, `agents.ghost`, 그리고 보이는 LSP
및 `graph_context`/Graphify 기본값을 뜻합니다.

Cargo로 설치한 binary를 삭제할 때는 `cargo uninstall codex-cli-captain`를 실행하세요. CCC-managed 정리까지 필요하면 먼저 `ccc uninstall --dry-run`으로 계획을 확인하고, 내용이 맞을 때만 `ccc uninstall --confirm`를 실행합니다. MCP registration, `ccc-config.toml`, skills, custom agent를 지우기 전에 `ccc check-install`로 현재 상태를 확인하세요.

레거시 release-bundle fallback만:

macOS 또는 Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

업데이트할 때도 `cargo install codex-cli-captain --force`를 다시 실행한 뒤 `ccc setup`을 실행하세요. 그 다음 Codex CLI를 완전히 재시작하고 `ccc check-install`을 실행하세요. installer는 새 bundle을 active path로 바꾸기 전에 stage하고, 이전 release bundle을 rollback용으로 보존하며, CCC-managed plugin 및 `$cap` 파일을 갱신합니다. stale cache/version entry와 legacy packaged cap copy 중 CCC가 관리하는 항목만 정리하고, non-CCC Codex config는 보존합니다.

release-bundle fallback installer는 `v0.0.3` fallback bundle asset이 publish 및 검증되기 전까지 기본적으로 `v0.0.2`에 고정되어 있습니다. Cargo가 `0.0.3`의 기본 설치 경로입니다. `CCC_VERSION`은 의도적으로 다른 release-bundle fallback을 설치할 때만 설정하세요.

## 기본 사용

CCC-managed task는 요청 앞에 `$cap`을 붙여 시작합니다.

```text
$cap Refactor the auth flow and keep tests passing
```

CCC는 LongWay, task card, checklist/projection, fan-in, status, restart handoff를 소유합니다. host `/plan`, `/goal`, graph command는 CCC public entrypoint가 아닙니다.

## 설정 변경 반영

`~/.config/ccc/ccc-config.toml`에서 각 CCC role의 model, reasoning tier, fast-mode 값을 바꿀 수 있습니다. 수정 후 Codex CLI에 아래 내용을 붙여넣으세요.

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`ccc setup`은 사용자가 바꾼 값을 보존하면서 빠진 generated default를 채우고, MCP registration, packaged `$cap` skill, CCC-managed custom agent를 다시 동기화합니다.

Codex plugin hooks는 opt-in입니다. CCC는 under-development인 `plugin_hooks` feature를 자동으로 켜지 않으며, hook command를 실행하려면 Codex CLI에서 `/hooks` review를 먼저 통과해야 합니다. 켜는 순서는 다음과 같습니다.

1. `~/.codex/config.toml`을 수정합니다
2. `[features] plugin_hooks = true`를 설정합니다
3. Codex CLI를 재시작합니다
4. `/hooks`를 열고 `PermissionRequest`, `PostToolUse`, `SessionStart`, `UserPromptSubmit`, `Stop`의 5개 CCC hook을 승인합니다
5. `ccc check-install`을 실행합니다

opt-in 후 unstable feature warning을 숨기고 싶다면 top-level에 `suppress_unstable_features_warning = true`를 추가할 수 있습니다. 이것은 warning만 숨기며, hook review나 hook 동작은 바꾸지 않습니다. global config에 `plugin_hooks = true`가 남아 있는 한, 새로운 Codex CLI 시작마다 warning이 다시 표시됩니다. hooks가 활성화되지 않았거나 review되지 않았다면 `ccc check-install`은 `runtime=hooks_first`가 아니라 `ccc_fallback`을 유지합니다.

`ccc setup`은 대상 Codex surface가 안전하게 읽을 수 있을 때만 hook
asset을 설치하거나 갱신합니다. hooks를 사용할 수 없거나, 비활성화되어
있거나, 신뢰되지 않거나, 지원되지 않으면 CCC는 CLI/MCP/status/fan-in
fallback을 계속 유지합니다.

## Hooks 준비 상태

`ccc check-install`은 hooks를 사용할 수 있는지와 현재 세션이
hooks-first인지 CCC fallback인지 알려 줍니다. 위의 opt-in과 `/hooks`
review를 끝냈다면 `runtime=hooks_first`를 기대하면 되고, 그렇지 않으면
`runtime=ccc_fallback`이 유지됩니다. 이 출력과 restart 안내를 함께 사용
하면 내부 routing 세부사항을 드러내지 않고도 실행 경로를 확인할 수
있습니다.

## 공개 동작

- `$cap`이 공개 entrypoint입니다.
- CCC는 계획, 수정, 검토, fan-in을 위해 내부 managed role을 사용합니다. 내부 routing 세부사항은 runtime state와 release-work docs에 두고 public README에는 나열하지 않습니다.
- `ccc setup`은 현재 binary와 `ccc-config.toml`을 기준으로 packaged `$cap` skill, MCP registration, plugin cache, managed agent를 갱신합니다.
- `ccc setup`은 대상 Codex surface가 안전하게 읽을 수 있을 때만 hook asset도 갱신합니다.
- `ccc check-install`은 active binary, Cargo candidate, plugin/cache discovery, hooks 준비 상태, 공개 config 형태, packaged `$cap` skill, stale path, 선택된 실행 경로, restart 필요 여부를 보고합니다.
