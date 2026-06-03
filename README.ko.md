# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI 작업을 계획, 실행, fan-in, 검토, 증명까지 한 흐름으로 끝냅니다.</em></p>

CCC는 Codex CLI 작업용 공개 제어면입니다. 공개 워크플로에는 `$ccc`를 사용하고, CCC는 LongWay plan, task card, specialist routing, fan-in, review boundary, status projection, 설치 검증을 CCC-owned evidence surface 위에서 관리합니다.

현재 공개 릴리스: `0.0.14`.

## CCC가 제공하는 것

| Surface | 동작 |
| --- | --- |
| `$ccc` entry | 호스트 transcript 상태에 기대지 않고 CCC를 거치는 작업을 위한 공개 워크플로 접두어입니다. |
| LongWay planning | 다단계 작업, 재시작 인계, 후속 연속성을 위한 plan/checklist 상태를 유지합니다. |
| Specialist routing | CCC는 설정된 `ccc_*` role로 라우팅하고, generic transcript text 대신 간결한 fan-in을 기록합니다. |
| Review boundary | 일반 검증은 Captain-owned입니다. 더 높은 위험의 release, destructive, security, operator-requested 작업은 review로 올립니다. |
| Status proof | `ccc status --text`, checklist/projection, `ccc check-install --text`로 현재 증거와 stale boundary를 확인합니다. app-panel output은 진단용입니다. |
| Tool readiness | Graphify context, LSP readiness, hooks, memory, skill registry, install surface를 advisory metadata가 runtime truth처럼 취급되지 않도록 보고합니다. |

## AI 에이전트로 설치하기

이 블록을 Codex CLI, ChatGPT 또는 shell 명령을 실행할 수 있는 다른 AI 에이전트에 붙여넣으세요. 목적은 바이너리 설치만이 아니라, 현재 상태 확인, 설치 또는 업데이트, setup 실행, Codex CLI 재시작 또는 재시작 안내, 그리고 결과 검증입니다.

```text
Codex CLI용 Codex-Cli-Captain 0.0.14를 설치하거나 업데이트하세요.

다음 절차를 신중하게 진행하세요.

1. 먼저 현재 상태를 확인합니다.
   - `command -v ccc || true`를 실행합니다.
   - `ccc`가 있으면 `ccc --version`과 `ccc check-install --text`를 실행합니다.
   - 기존 binary가 current, shadowed, stale, missing 중 무엇인지 보고합니다.

2. Cargo로 설치하거나 업데이트합니다.
   - `cargo install codex-cli-captain --force`를 실행합니다.
   - `ccc setup`을 실행합니다.

3. Codex CLI를 완전히 종료하고 새 세션을 시작하라고 안내하거나, 이 호스트가 안전하게 지원하면 Codex CLI cache를 다시 불러오라고 안내합니다.

4. 재시작 후 다음을 검증합니다.
   - `command -v ccc`를 실행합니다.
   - `ccc --version`을 실행합니다.
   - `ccc check-install --text`를 실행합니다.
   - `ccc status --text`를 실행합니다.

5. 결과를 요약합니다.
   - 설치된 버전
   - `$ccc` entry skill이 current인지 여부
   - MCP registration이 일치하는지 여부
   - custom agents가 synced 되었는지 여부
   - restart 또는 cache reload가 아직 필요한지 여부
   - PATH shadowing, stale plugin cache, legacy bundle warning 여부
```

직접 설치할 때는 다음을 실행합니다.

```bash
cargo install codex-cli-captain --force
ccc setup
```

그다음 Codex CLI를 완전히 다시 시작한 뒤 다음으로 확인합니다.

```bash
ccc check-install --text
ccc status --text
```

## 사용

CCC-managed 작업을 시작하려면 요청에 `$ccc`를 사용합니다.

```text
$ccc release docs를 업데이트하고, 먼저 현재 코드베이스 증거를 확인한 뒤, 변경 내용을 보고해 줘
```

후속 작업에서도 `$ccc` 또는 CCC가 출력한 continuation hint를 계속 사용합니다. CCC는 workspace의 `.ccc` runtime artifact 아래에 LongWay/checklist/fan-in 상태를 저장하고, `ccc status`로 현재 상태를 표시합니다. app-panel output은 호스트가 아직 노출하는 경우의 진단용 표면입니다.

호스트가 아직 legacy skill entry를 노출하는 경우, 그 entry가 compatibility path로 계속 동작할 수 있습니다.

- macOS cross-asset provenance 실행은 release script가 non-macOS binary를 실행하려고 하면 멈출 수 있습니다. `v0.0.13`은 cross asset에 temporary wrapper와 Zig linker settings를 사용했습니다.
- crates.io-installed binary는 Cargo가 published crate에서 다시 빌드하므로 `source_commit=unknown` 또는 partial binary provenance를 보고할 수 있습니다.
- release 작업용 로컬 파일, 예를 들어 tarball이나 checksum은 업로드 후 이 release repo checkout에 남을 수 있습니다. 의도적으로 commit하지 않았다면 source proof로 취급하지 않습니다.

## 호환성 메모

| Area | Status |
| --- | --- |
| macOS | 지원하며 local maintainer smoke path에서 검증되었습니다. |
| Linux | release asset이 publish되어 있으며, 설치 후 환경에서 `ccc check-install --text`로 확인하세요. |
| Windows | release asset이 publish되어 있으며, 설치 후 환경에서 `ccc check-install --text`로 확인하세요. |
| Codex plugin hooks | 선택 사항입니다. 활성화했다면 Codex CLI를 다시 시작하고 `ccc check-install --text`로 확인하세요. |

## 업데이트

```bash
cargo install codex-cli-captain --force
ccc setup
```

Codex CLI를 다시 시작한 뒤 `ccc check-install --text`를 실행합니다.

## 삭제

```bash
ccc uninstall --dry-run
ccc uninstall --confirm
cargo uninstall codex-cli-captain
```

먼저 dry-run을 실행해 경로를 확인하세요. `cargo uninstall`은 Cargo binary만 제거하고, `ccc uninstall --confirm`는 unrelated Codex data는 보존한 채 CCC-managed Codex surface를 제거합니다.

## 공개 surface boundary

`$ccc`는 공개 user entrypoint입니다. internal role name, fan-in artifact, Graphify detail, LSP state, hook, memory readiness는 CCC 자체의 proof surface이며 host-owned transcript claim으로 취급하면 안 됩니다. host spawn/toast label은 host가 렌더링하므로 CCC status와 다를 수 있습니다.
