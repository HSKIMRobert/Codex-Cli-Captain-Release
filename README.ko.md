# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI 작업을 하나의 공개 진입점으로 계획하고, 위임하고, 검증하고, 끝냅니다.</em></p>

CCC는 Codex CLI를 위한 작은 control plane입니다. 빠른 답변을 넘어 계획,
순서 있는 작업, specialist 도움, review, 완료 증거가 필요한 작업에는 `$ccc`를
사용합니다.

릴리스 버전: `0.0.14`.

## 설치

직접 설치하려면:

```bash
cargo install codex-cli-captain --force
ccc setup
```

그다음 Codex CLI를 완전히 다시 시작하고 확인합니다.

```bash
ccc check-install --text
ccc status --text
```

AI 에이전트에게 설치를 맡기려면 아래 내용을 붙여넣으세요.

```text
Codex CLI용 Codex-Cli-Captain 0.0.14를 설치하거나 업데이트해줘.

1. 먼저 현재 상태를 확인해줘.
   - command -v ccc || true
   - ccc가 있으면 ccc --version
   - ccc가 있으면 ccc check-install --text

2. 설치 또는 업데이트해줘.
   - cargo install codex-cli-captain --force
   - ccc setup

3. 내가 Codex CLI를 완전히 재시작하도록 안내해줘.

4. 재시작 후 확인해줘.
   - command -v ccc
   - ccc --version
   - ccc check-install --text
   - ccc status --text

5. $ccc, MCP registration, hooks, custom agents, Graph Context가 current인지,
   restart나 PATH cleanup이 아직 필요한지 보고해줘.
```

## 명령

Codex CLI 안에서는 아래 명령을 입력합니다.

| 명령 | 용도 |
| --- | --- |
| `$ccc "task"` | CCC가 관리하는 Codex 작업을 시작합니다. |

터미널에서는 아래 명령을 실행합니다.

| 명령 | 용도 |
| --- | --- |
| `cargo install codex-cli-captain --force` | crates.io에서 CCC binary를 설치하거나 업데이트합니다. |
| `ccc setup` | 설치 또는 업데이트 후 Codex CLI 연동을 새로 고칩니다. |
| `ccc check-install --text` | 설치, hooks, skills, agents, Graph Context를 확인합니다. |
| `ccc status --text` | 현재 작업 상태와 다음 행동을 확인합니다. |

## CCC가 해주는 것

| 기능 | 사용자에게 의미하는 것 |
| --- | --- |
| 계획 우선 작업 | 넓은 작업은 수정 전에 명확한 계획을 세웁니다. |
| Captain orchestration | host Codex가 지휘하고, 필요한 specialist 작업은 CCC가 라우팅합니다. |
| 조용한 진행 추적 | status, checklist, fan-in을 transcript noise 대신 CCC 표면에 보관합니다. |
| 증거 기반 완료 | 현재 검증과 review evidence를 완료 판단의 기준으로 삼습니다. |

## 사용

`$ccc`로 시작합니다.

```text
$ccc release docs를 업데이트하고, 먼저 현재 증거를 확인한 뒤, 변경 내용을 보고해줘
```

후속 작업도 `$ccc`를 사용하거나, CCC가 persisted run을 만들었을 때 출력하는
continuation command를 사용합니다.

`$ccc`에 잘 맞는 작업:

- 여러 단계의 코드 또는 문서 작업
- release, install, verification에 민감한 작업
- 저장된 plan에서 이어서 진행해야 하는 작업
- subagent fan-in이나 review가 도움이 되는 작업

작은 단발성 질문은 일반 Codex CLI 사용만으로 충분한 경우가 많습니다.

## 업데이트

```bash
cargo install codex-cli-captain --force
ccc setup
```

Codex CLI를 다시 시작한 뒤 실행합니다.

```bash
ccc check-install --text
```

## 삭제

```bash
ccc uninstall --dry-run
ccc uninstall --confirm
cargo uninstall codex-cli-captain
```

먼저 dry-run으로 경로를 확인하세요. `cargo uninstall`은 Cargo binary를
제거합니다. `ccc uninstall --confirm`은 unrelated Codex data는 보존하면서
CCC-managed Codex surface를 제거합니다.

## 메모

- `$ccc`가 공개 진입점입니다. `wccc`, task card, fan-in, Graph Context,
  hooks, review detail은 CCC 내부 proof surface입니다.
- Codex plugin hooks는 선택 사항입니다. 활성화했다면 Codex CLI를 다시
  시작하고 `ccc check-install --text`로 확인하세요.
- macOS는 maintainer smoke의 주된 경로입니다. Linux와 Windows build는 각
  target 환경에서 `ccc check-install --text`로 확인하세요.
- crates.io로 설치한 binary는 Cargo가 published crate에서 다시 빌드하므로
  binary provenance가 partial로 보일 수 있습니다.
