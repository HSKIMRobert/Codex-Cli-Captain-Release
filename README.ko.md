# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI로 end-to-end로 하고 싶으시다구요?<br>
그런데 고사양 모델로 end-to-end를 돌리는 건 걱정된다구요?<br>
그렇다면 CCC를 이용해보시는 건 어떨까요?<br>
여러분은 그저 하고자 하는 것 앞에 <code>$cap</code>만 붙이면 됩니다.<br>
그러면 놀라운 일이 펼쳐질 거예요!</em></p>

현재 공개 버전: `0.0.11-pre`.

<table>
<tr>
<td>

<strong>주의 - 사전 릴리즈 기본 권장 경로</strong><br><br>
<code>0.0.11-pre</code>가 현재 공개 사전 릴리즈 기본값입니다. macOS는 공식 지원 경로이며, macOS arm64는 로컬에서 설치와 동작을 확인했습니다. Linux/Windows asset은 초기 테스트용으로 제공되며 같은 <code>ccc setup</code> / <code>ccc check-install</code> 흐름을 사용할 것으로 기대하지만, 실제 Linux/Windows 환경에서의 검증은 아직 충분하지 않습니다. 필요하면 <code>CCC_VERSION=v0.0.10-pre</code>로 이전 사전 릴리즈를 설치할 수 있습니다.

</td>
</tr>
</table>

## 0.0.11-pre 릴리즈 카드

`0.0.11-pre`는 walking skeleton과 후속 hardening을 위한 릴리즈이며, full parity나 rebuild 완료 릴리즈가 아닙니다. 이 릴리즈는 `$cap`을 public CCC entrypoint로 두고, host `/plan`과 `/goal`은 optional affordance로 둡니다. `$cap`은 두 host surface 없이도 동작합니다.

이번 릴리즈는 PLAN_SEQUENCE와 EXECUTE_SEQUENCE, 실행 전 pending LongWay 승인, checklist/status/fan-in을 운영상 truth로 삼는 흐름, persisted state에서 이어가기 위한 restart handoff를 공개 문서에 반영합니다.

검증 요약:

- Source repo PR #2부터 PR #7까지 merge됐으며, release-facing source docs commit은 `a92d25f2f73d5be06c96a150a0016d1cc48d3cbc`입니다.
- 최종 구현 slice에서 `cargo test -p ccc`가 통과했습니다.
- Public release repo 문서 갱신 전에 release-facing source docs가 merge됐습니다.

`0.0.11-pre`에서 의도적으로 미룬 항목:

- Loop 5 Intervention / Review-retry-replan-reclaim parity projection.
- Schema contract promotion.
- `setup_config` config surface change.
- Full runtime parity projection.

## 설치 및 업데이트

macOS/Linux에서는 Codex CLI에 아래 문구를 붙여넣으세요.

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

Windows PowerShell에서는 아래 문구를 사용하세요.

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

업데이트할 때도 같은 install 명령을 다시 실행하세요. 현재 릴리즈 asset을 내려받아 현재 설치 위치와 `ccc` command shim/binary를 갱신하고, setup과 `ccc check-install`을 다시 실행합니다. 이후 Codex CLI를 완전히 재시작해야 갱신된 `$cap` skill과 custom agent가 로드됩니다.

## 설정 변경 반영

`~/.config/ccc/ccc-config.toml`을 수정한 뒤 Codex CLI에 아래 문구를 붙여넣으세요. 새로 생성된 설치용 `~/.config/ccc/ccc-config.toml`은 `explorer`에는 reasoning `variant = "high"`를, `documenter`, `companion_reader`, `companion_operator`에는 `variant = "medium"`를 사용하며, 모든 role은 `fast_mode = true`를 유지합니다. `ccc setup`은 기존에 사용자가 수정한 값을 유지한 채 빠진 생성 기본값을 채우고 오래된 CCC 생성 기본값은 업그레이드합니다.

생성 기본값은 captain reasoning을 기본 medium으로 유지하면서 handoff 비용을 줄입니다. 설정된 mini/specialist role은 더 빠른 service 경로를 유지하고, delegated worker에게 넘기는 작업 설명은 필요한 길이로 줄이며, `$cap`과 custom-agent 지시문도 compact하게 유지합니다.

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

CCC를 자주 사용한다면 ChatGPT Pro $100 요금제를 시작점으로 권장합니다. `$cap` workflow는 captain과 specialist handoff를 반복하면서 Codex 사용량을 더 많이 쓸 수 있기 때문입니다. Reasoning은 사용자의 작업 스타일과 작업 위험도에 맞춰 조정하세요. 넓은 계획, 위험한 코드 변경, 리뷰에는 높은 reasoning을 유지하고, 좁고 반복적이거나 위험이 낮은 작업에는 낮춰도 됩니다.

## 추천 역할 설정

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

서브에이전트 결과가 unsatisfactory 또는 needs-work이면 CCC는 이를 bounded specialist follow-up으로 정규화하고, 가능한 경우 local captain repair보다 specialist 경유 repair나 reassignment를 우선합니다.
