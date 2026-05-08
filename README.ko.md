# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI나 Codex App으로 end-to-end로 하고 싶으시다구요?<br>
그런데 고사양 모델로 end-to-end를 돌리는 건 걱정된다구요?<br>
그렇다면 CCC를 이용해보시는 건 어떨까요?<br>
여러분은 그저 하고자 하는 것 앞에 <code>$cap</code>만 붙이면 됩니다.<br>
그러면 놀라운 일이 펼쳐질 거예요!</em></p>

이 릴리스는 로컬 Codex plugin marketplace를 통해 CCC를 설치합니다. 패키지에는 CCC plugin manifest, `.mcp.json`, plugin 제공 `$cap` skill이 포함되며, installer는 `ccc@ccc-local` plugin을 활성화하고 기존 직접 `mcp_servers.ccc` 등록과 standalone `~/.codex/skills/cap` 복사본을 제거합니다. 공개 operator entrypoint는 여전히 `$cap`입니다.

현재 공개 버전: `0.0.15-pre`.

지원 release target은 정확히 `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`, `windows-x86_64`입니다. macOS target은 일반적으로 지원되며 동작할 것으로 기대합니다. Linux와 Windows target도 제공하지만 platform-specific 문제가 남아 있을 수 있습니다.

## 설치

macOS 또는 Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

업데이트할 때도 같은 설치 명령을 다시 실행한 뒤 Codex CLI를 재시작하고 `codex mcp list`를 실행하세요. Installer는 로컬 `ccc-local` marketplace를 갱신하고 `plugins."ccc@ccc-local"`을 활성화하며, CCC가 plugin으로 로드되도록 기존 직접 `mcp_servers.ccc` 블록과 standalone `$cap` skill을 제거합니다.

CCC source나 Rust 중심 repo에서 작업할 때는 선택적으로 Rust LSP를 설치하면 도움이 됩니다.

```bash
rustup component add rust-analyzer
```

안정적인 `ccc_*` ID는 계속 routing contract이고, callsign은 display-only입니다. `ccc_tactician`은 Executor, `ccc_scout`은 Observer, `ccc_raider`는 Marauder, `ccc_scribe`는 Adjutant, `ccc_arbiter`는 Arbiter, `ccc_sentinel`은 Overseer, `ccc_companion_reader`는 Probe, `ccc_companion_operator`는 SCV입니다. 0.0.15-pre metadata에는 oh-my-openagent에서 영감을 받은 workflow set도 포함됩니다: `github-triage`, `hyperplan`, `work-with-pr`, `pre-publish-review`, `git-master`, `review-work`, `remove-deadcode`, `get-unpublished-changes`, `ai-slop-remover`, `rust-analyzer-lsp`.

## 추천 역할 설정

CCC를 자주 사용한다면 ChatGPT Pro $100 요금제를 시작점으로 권장합니다. `$cap` workflow는 captain과 specialist handoff를 반복하면서 Codex 사용량을 더 많이 쓸 수 있기 때문입니다. Reasoning은 사용자의 작업 스타일과 작업 위험도에 맞춰 조정하세요. 넓은 계획, 위험한 코드 변경, 리뷰에는 높은 reasoning을 유지하고, 좁고 반복적이거나 위험이 낮은 작업에는 낮춰도 됩니다.

| CCC role | Agent | 추천 모델 | Reasoning | 용도 |
| --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `gpt-5.5` | `medium` | LongWay 관리와 최종 라우팅 판단 |
| `way` | `tactician` | `gpt-5.5` | `high` | 계획 수립과 다음 작업 선택 |
| `explorer` | `scout` | `gpt-5.4-mini` | `high` | 읽기 전용 repo 조사 |
| `code specialist` | `raider` | `gpt-5.5` | `high` | 코드/config 수정과 복구 |
| `documenter` | `scribe` | `gpt-5.4-mini` | `medium` | README, 릴리즈 노트, 사용자 문구 |
| `verifier` | `arbiter` | `gpt-5.5` | `high` | 리뷰, 리스크, 회귀 확인 |
| `companion_reader` | `companion_reader` | `gpt-5.4-mini` | `medium` | 저비용 filesystem/docs/web/git/gh 읽기 작업 |
| `companion_operator` | `companion_operator` | `gpt-5.4-mini` | `medium` | 저비용 git/gh 변경 및 좁은 도구 실행 |

`gpt-5.5`는 ChatGPT 인증 Codex에서 고가치 역할에 권장되는 모델입니다. 현재 계정이나 실행 경로에서 아직 사용할 수 없다면 해당 역할은 rollout이 도달할 때까지 `gpt-5.4`를 사용합니다.
