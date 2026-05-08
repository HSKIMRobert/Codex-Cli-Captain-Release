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

이 릴리스는 플러그인 배포용으로 패키징되어 있습니다. 패키지에는 CCC plugin manifest, `.mcp.json`, `skills/ccc/SKILL.md`가 포함됩니다. `@cap`는 플러그인 install/discovery 흐름에서 보조 탐색 힌트로 보일 수 있지만, 공개 operator entrypoint는 여전히 `$cap`입니다.

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

업데이트할 때도 같은 설치 명령을 다시 실행한 뒤 Codex CLI를 재시작하고 `ccc check-install`을 실행하세요.

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
