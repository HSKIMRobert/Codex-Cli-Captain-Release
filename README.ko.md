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

현재 공개 릴리스: `0.0.1`.

CCC는 Codex CLI를 위한 captain-first orchestration layer입니다. `$cap`만 public entrypoint로 유지하고, LongWay/task-card/fan-in 상태를 저장하며, specialist 작업을 설정된 `ccc_*` agent로 라우팅한 뒤 captain review로 합칩니다.

## 설치 또는 업데이트

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

업데이트할 때도 같은 설치 명령을 다시 실행하고, Codex CLI를 완전히 재시작한 뒤 `ccc check-install`을 실행하세요. installer는 새 bundle을 active path로 바꾸기 전에 stage하고, 이전 release bundle을 rollback용으로 보존하며, CCC-managed plugin 및 `$cap` 파일을 갱신합니다. stale cache/version entry와 legacy packaged cap copy 중 CCC가 관리하는 항목만 정리하고, non-CCC Codex config는 보존합니다.

release installer는 기본적으로 `v0.0.1`에 고정되어 있습니다. `CCC_VERSION`은 의도적으로 다른 release를 설치할 때만 설정하세요.

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

## 추천 역할 설정

일반적인 CCC 사용에는 ChatGPT Pro $100 플랜을 시작점으로 권장합니다. `$cap` workflow는 captain과 specialist handoff를 반복하면서 Codex 사용량을 더 쓸 수 있기 때문입니다.

| CCC role | Stable agent ID | Display callsign | 추천 모델 | Reasoning | 설명 |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | host-owned routing label이며 managed `ccc_*` specialist가 아님 |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | 계획 수립과 bounded next-move 선택 |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | read-only repo evidence |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | code/config mutation과 repair |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README 및 operator text |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | captain-mediated review, risk, regression, acceptance check |
| `sentinel` | `ccc_sentinel` | `Overseer` | `gpt-5.4-mini` | `high` | run-scoped guardrail classification과 status visibility |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | 저비용 filesystem/docs/web/git/gh 읽기 작업 |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | 저비용 git/gh mutation과 좁은 tool 작업 |

Stable `ccc_*` ID가 routing truth입니다. StarCraft callsign은 display-only입니다.
