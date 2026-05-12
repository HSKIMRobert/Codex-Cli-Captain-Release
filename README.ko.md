# Codex-Cli-Captain

[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md)

Codex CLI에서 `$cap` 요청을 captain-first 흐름으로 실행하는 Rust 런타임입니다.

현재 소스 버전: `0.0.15-pre`.

> 지원 release target은 정확히 `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`, `windows-x86_64`입니다. macOS target은 일반적으로 지원되며 동작할 것으로 기대합니다. Linux와 Windows target도 제공하지만 platform-specific 문제가 남아 있을 수 있습니다.

## 로컬 설치 및 업데이트

```bash
cargo build --offline
ccc setup
```

그 다음 Codex CLI를 완전히 종료하고 새 세션을 시작한 뒤 확인합니다.

```bash
ccc check-install
```

기존 local source checkout을 업데이트할 때는 최신 source를 받은 뒤 다시 빌드하고 `ccc setup`을 실행하세요. 그 다음 Codex CLI를 완전히 재시작하고 `ccc check-install`로 확인합니다. `setup`은 현재 바이너리와 `ccc-config.toml` 기준으로 MCP 등록, packaged `$cap` skill, CCC-managed custom agent를 갱신합니다. 릴리스 installer는 기본적으로 `v0.0.15-pre`에 고정되어 있으며, `CCC_VERSION`은 의도적인 override일 때만 사용합니다. 설치 과정은 새 bundle을 active path로 바꾸기 전에 stage하고, 이전 release bundle을 rollback용으로 보존하며, CCC-managed plugin cache/version entry와 legacy `skills/cap` 복사본만 정리합니다. non-CCC Codex config는 유지합니다. TypeScript/JavaScript LSP 설정은 향후 `lsp_diagnostics`, `lsp_references`, `lsp_definition`, `lsp_prepare_rename`, `lsp_rename`용 config surface로 기록됩니다. 필요하면 `npm install -g typescript typescript-language-server`로 서버를 준비할 수 있습니다. `0.0.15-pre`에서는 runtime LSP 실행이 deferred이며 CCC가 language server를 시작하지 않습니다. 선택적인 `rust-analyzer`는 Rust 전용 local navigation 지원이고, 필요하면 `rustup component add rust-analyzer`로 설치하세요.

## 설정 변경 반영

`~/.config/ccc/ccc-config.toml`에서 각 역할의 model, reasoning 단계, fast mode를 바꿀 수 있습니다. 새로 생성된 설치용 `~/.config/ccc/ccc-config.toml`은 `way`, `explorer`, `code specialist`, `verifier`에는 reasoning `variant = "high"`를, `documenter`, `companion_reader`, `companion_operator`에는 reasoning `variant = "medium"`를 사용하며, 모든 role은 `fast_mode = true`를 유지합니다. `ccc setup`은 기존에 사용자가 수정한 값을 유지한 채 빠진 생성 기본값을 채우거나 오래된 CCC 생성 기본값을 업그레이드합니다. 수정 후에는 Codex CLI에 아래 문구를 붙여넣으면 됩니다.

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`setup`은 `ccc-config.toml` 기준으로 MCP 등록, `$cap` skill, CCC-managed custom agent를 다시 동기화합니다.

## 0.0.15-pre 동작

- `$cap`은 public entrypoint입니다.
- 기본 specialist 대상은 설정된 `ccc_*` custom agent입니다. `worker`와 `explorer` 같은 generic label은 operator가 명시적으로 override하지 않으면 invalid입니다.
- `ccc memory`는 opt-in이며 기본값은 unconfigured입니다.
- SSL Skill Registry는 routing, planning, review용 bounded evidence로 제공되며 persisted run state를 대체하지 않습니다.
- `ccc status --subagents --text`와 `ccc checklist --subagents --text`는 가능한 곳에서 callsign과 stable ID를 함께 보여줍니다. 예: `[x] scout-b completed child=ccc_scout role=Observer(ccc_scout)/explorer task="Inspect scope"`.
- `ccc status --projection --json '{...}'`와 `ccc checklist --projection --json '{...}'`는 workspace root의 `CCC_LONGWAY_PROJECTION.md` 하나를 갱신합니다. `git diff -- CCC_LONGWAY_PROJECTION.md`로 LongWay/subagent projection을 확인할 수 있고, 다음 projection 갱신 때 덮어써집니다.
- projection heading은 요청 언어를 감지할 수 있으면 그 언어를 따릅니다. 한국어 `$cap` 요청은 한국어 projection label로 표시됩니다.
- terminal host-subagent update는 CCC active handle을 release하고, 완료된 host agent thread를 아직 닫아야 하는 경우 status에 표시합니다.
- mutation 완료는 specialist fan-in 뒤에 진행되며, review-sensitive 변경의 최종 gate는 arbiter review입니다.
- 0.0.15-pre는 callsign mapping 안내를 유지하면서 oh-my-openagent에서 영감을 받은 workflow set(github-triage, get-unpublished-changes, remove-deadcode, ai-slop-remover, lsp-safe-refactor, review-work, pre-publish-review, hyperplan, git-master, publish, release-command-discipline, release-note, readme-maintenance, changelog, role-ownership, lane-conflict, fallback-classification, filesystem-evidence)을 다룹니다.

안정적인 `ccc_*` ID가 source of truth이고, callsign은 display-only입니다. `captain/orchestrator`는 Command Center(또는 Captain, managed 된 `ccc_*` role 아님)에 대응합니다. `ccc_tactician/way`는 Executor, `ccc_scout/explorer`는 Observer, `ccc_raider/code specialist`는 Marauder, `ccc_scribe/documenter`는 Adjutant, `ccc_arbiter/verifier`는 Arbiter, `ccc_sentinel`은 Overseer, `ccc_companion_reader`는 Probe, `ccc_companion_operator`는 SCV에 대응합니다.

Host UI layer가 `Closed Carver [ccc_scout]` 같은 outer notification을 표시할 수도 있지만, 그 문구는 host-managed이며 CCC가 보장하는 출력이 아닙니다. CCC-controlled status/projection output은 `Observer(ccc_scout)`처럼 callsign과 stable ID를 함께 보여줍니다.

| Stable ID | Config role | Callsign | Theme |
| --- | --- | --- | --- |
| `ccc_tactician` | `way` | Executor | `starcraft_display_callsign` |
| `ccc_scout` | `explorer` | Observer | `starcraft_display_callsign` |
| `ccc_raider` | `code specialist` | Marauder | `starcraft_display_callsign` |
| `ccc_scribe` | `documenter` | Adjutant | `starcraft_display_callsign` |
| `ccc_arbiter` | `verifier` | Arbiter | `starcraft_display_callsign` |
| `ccc_sentinel` | `sentinel` | Overseer | `starcraft_display_callsign` |
| `ccc_companion_reader` | `companion_reader` | Probe | `starcraft_display_callsign` |
| `ccc_companion_operator` | `companion_operator` | SCV | `starcraft_display_callsign` |

미래 optional agent는 문서에만 남아 있으며 `0.0.15-pre` runtime role은 아닙니다: `ccc_release_arbiter`/Judicator, `ccc_qa_runner`/Valkyrie, `ccc_lsp_scout`/Science Vessel.

Per-agent workflow mapping은 advisory입니다. Observer는 `github-triage`와 `get-unpublished-changes`, Marauder는 `remove-deadcode`, `ai-slop-remover`, LSP safe refactor, Arbiter는 `review-work`와 `pre-publish-review`, Executor는 `hyperplan`, SCV는 `git-master`, publish, release command discipline, Adjutant는 release note/README/changelog, Overseer는 role ownership/lane conflict/fallback classification, Probe는 lightweight GitHub/filesystem evidence collection을 담당합니다.

## 추천 역할 설정

| CCC role | Stable agent ID | Display callsign | 추천 모델 | Reasoning | 용도 |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | host-owned 라우팅 label, managed `ccc_*` specialist 아님 |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | 계획 수립과 다음 작업 선택 |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | 읽기 전용 repo 조사 |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | 코드/config 수정과 복구 |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README, 릴리즈 노트, 사용자 문구 |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | 캡틴 주도 리뷰, 리스크, 회귀 확인 |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | 저비용 filesystem/docs/web/git/gh 읽기 작업 |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | 저비용 git/gh 변경 및 좁은 도구 실행 |

`gpt-5.5`는 ChatGPT 인증 Codex에서 고가치 역할에 권장되는 모델입니다. 현재 계정이나 실행 경로에서 아직 사용할 수 없다면 해당 고가치 역할은 rollout이 도달할 때까지 `gpt-5.4`를 사용합니다.

## 동작 흐름

`$cap`은 CCC public entrypoint입니다. CCC orchestration은 `$cap`을 직접 사용하며, host planning surface는 CCC contract가 아닙니다.

CCC 흐름은 다음처럼 나뉩니다.

1. `PLAN_SEQUENCE`: captain이 의도를 확인하고 설정된 Way agent에 planning을 맡깁니다. host Plan Mode는 백그라운드 Way engine으로 사용할 수 없으며 CCC planning을 소유하거나 대체하면 안 됩니다.
2. Way는 pending LongWay와 후보 task card를 만들며, 이 단계는 read-only입니다.
3. operator가 pending LongWay를 승인합니다.
4. `EXECUTE_SEQUENCE`: captain이 승인된 LongWay를 다시 읽고 task card를 materialize한 뒤 scheduler/router를 통해 specialist에 배정합니다.
5. specialist 결과는 result envelope과 compact fan-in으로 돌아오고, checklist/status/fan-in이 progress truth가 됩니다.
6. captain은 continue, repair, replan, reclaim, complete, restart handoff 중 다음 조치를 결정하며, mutation 완료는 specialist fan-in과 arbiter review 뒤에만 마무리됩니다.

Host planning UI는 operator가 입력하는 문장을 정리하는 데 도움을 줄 수 있지만, CCC가 Way agent 안에서 host Plan Mode를 트리거하거나 의존하지는 않습니다. 충돌하면 CCC persisted LongWay, checklist, fan-in, resolve state가 우선합니다.

0.0.15-pre는 stricter intent-state-machine 동작을 이어가고 callsign mapping 안내를 더하며, docs-and-release-gates를 정리하는 pre-release입니다. oh-my-openagent에서 영감을 받은 workflow set(github-triage, get-unpublished-changes, remove-deadcode, ai-slop-remover, lsp-safe-refactor, review-work, pre-publish-review, hyperplan, git-master, publish, release-command-discipline, release-note, readme-maintenance, changelog, role-ownership, lane-conflict, fallback-classification, filesystem-evidence)을 추적하지만, full runtime parity나 완성된 rebuild를 뜻하지는 않습니다. LSP runtime 실행은 여전히 deferred이며 CCC가 language server를 시작하지 않습니다.

특별한 역할은 `ccc-config.toml`에서 선택합니다. host Codex는 captain으로서 LongWay, routing, lifecycle, fan-in, review, validation, commit boundary를 책임집니다. ordinary `$cap` 작업은 먼저 맞는 specialist에게 넘겨야 하며, read-only 조사는 `ccc_scout`, docs/operator text는 `ccc_scribe`, code/config 변경은 `ccc_raider`, review 판단은 `ccc_arbiter`가 맡습니다. captain이 직접 작업하는 경우는 명시적 fallback, 정말 사소한 operator-side 수정, 또는 CCC가 눈에 띄게 degraded 되었다고 기록할 수 있는 경우로만 제한합니다. 설정된 `ccc_*` custom agent가 기본 specialist 이름이며, 명시적 override가 없으면 generic `worker`와 `explorer` label은 사용하지 않습니다.

가벼운 filesystem/docs/fetch/git/gh 작업은 tool route에 specialist owner가 있으면 captain 세션에 머무르지 않고 설정된 mini companion 역할로 라우팅합니다. Git과 `gh` 읽기는 `companion_reader`, git과 `gh` 변경은 captain이 명시적 fallback 또는 degradation 이유를 기록하지 않는 한 `companion_operator`가 맡습니다.

`raider` 프롬프트에는 모듈 경계 존중, 중복 감소 목적의 helper 분리, 거대한 함수/광범위 리라이트 회피, 관련 없는 변경 금지 원칙을 더 명확히 넣었습니다.

`v0.0.15-pre` 운영 정책은 리뷰를 모든 작업에 자동으로 붙이지 않고, 캡틴이 필요할 때만 명시적으로 여는 조건부 절차로 다룹니다. 리뷰어는 bounded verification input으로만 쓰고, accept/reassign/close 판단은 캡틴이 계속 맡으며, 새 리뷰를 시작하기 전에 hardware, memory, 같은 머신의 concurrency 부담도 함께 봐야 합니다. Long-session status는 필요할 때 checkpoint/resume 안내를 보여주며 `/compact`, `/new`, `/exit`는 operator 선택으로 남깁니다.

이 초안에서는 서브에이전트 결과가 돌아오면 캡틴이 이를 accept, close, 또는 unsatisfactory로 처리할 수 있습니다. unsatisfactory 결과는 LongWay/task-card state에 rationale과 다음 조치와 함께 기록되어야 합니다. CCC는 unsatisfactory 또는 needs-work 결과를 bounded specialist follow-up으로 정규화하고, CCC가 specialist 경로로 repair나 reassignment를 라우팅할 수 있을 때는 캡틴이 local repair를 직접 수행하지 않아야 합니다. 원래 scope가 여전히 유효하면 캡틴은 같은 specialist에게 missing delta, risk, correction target만 겨냥한 좁은 prompt로 한 번만 bounded repair를 보냅니다. role이나 approach가 잘못되었으면 더 적합한 specialist에게 한 번만 bounded reassignment을 보냅니다. 이전의 unsatisfactory 결과는 history에 그대로 보여야 하며, CCC는 subagent-to-subagent handoff, unbounded retry, 명시적 replan/re-scope 없는 scope widening, explicit reason 없는 silent degraded fallback을 하지 않는 방향으로 구현되어야 합니다.

계획된 개입 경로는 captain-owned입니다. 사용자가 서브에이전트가 active한 동안 개입하면 요청은 캡틴을 통해서만 전달되어야 합니다. 캡틴은 그 개입을 bounded delta와 rationale로 LongWay/task-card state에 기록하고, clarification-only, bounded scope amendment, direction/risk correction 중 하나로 분류한 뒤, 안전하면 같은 worker 수정, forced interruption이 지원되지 않거나 scope가 크게 바뀌었으면 reclaim, 더 적합한 specialist가 있으면 reassignment 중 정확히 하나를 선택하는 방향입니다. stale output은 계속 보여야 하고, 캡틴이 명시적으로 merge하지 않는 한 선택된 경로를 조용히 덮어쓸 수 없습니다. 개입은 dissatisfaction repair와 같은 bounded retry/reassign budget을 사용하므로, 무한 amend loop, 명시적 replan/re-scope 없는 scope widening, 개입만을 위한 duplicate mutable worker는 허용되지 않습니다.

Codex가 `Too many open files (os error 24)` 같은 file descriptor 압박을 보고하면 새 reviewer나 specialist를 더 열지 않습니다. 각 active host agent를 terminal lifecycle update로 기록하고, captain이 merge 또는 reclaim한 뒤, host session에서 해당 agent를 close해서 thread/file handle이 해제될 때까지 단일 경로로 진행합니다.

transcript folding 때문에 긴 status block이 접혀 보이면 subagent-only 또는 projection 경로를 사용하세요.

```bash
ccc status --subagents --text --json '{"run_id":"..."}'
ccc status --projection --json '{"run_id":"..."}'
git diff -- CCC_LONGWAY_PROJECTION.md
```

projection 파일은 표시용 artifact일 뿐입니다. persisted run state, task card, lifecycle, fan-in이 계속 source of truth입니다.

## 릴리즈 위생

release repo는 installer, docs, packaged `$cap` skill, 컴파일된 `ccc` 바이너리만 유지하는 방향입니다. 릴리즈 asset 빌드 시 가능한 경우 바이너리 심볼을 제거하고, 공개 전 민감 문자열 검사를 실행합니다.

## 0.0.15 오퍼레이터 가이드

0.0.15 문서는 `$cap` public contract, specialist-first routing, callsign mapping, release-gate hygiene, checkpoint/resume guidance, active-handle cleanup, and verification/fan-in visibility를 현재 release-facing 안내로 정리합니다.

- 사용자가 comments 또는 annotations 를 요청하면, 내용을 평탄화하거나 재정렬하지 말고 요청된 chronological block format 을 그대로 유지합니다.
- OMO sisyphus 또는 harness 표현은 외부 시스템과의 연동이 아니라 CCC의 operating shape 로 해석합니다. captain 1명, bounded specialist routing, 그리고 각 specialist 결과가 다음 판단 전 captain 으로 돌아오는 흐름을 유지합니다.
- 복잡하거나 위험한 해석은 Way/tactician 으로 넘기기 전에 captain 이 먼저 operator 에게 해석을 확인합니다.
- 역할 분리는 명시적으로 유지합니다. scout 와 companion_reader 는 evidence 를 모으고, Way/tactician 은 plan 을 만들고, raider 와 companion_operator 는 mutate 하며, scribe 는 docs/operator text 를 맡고, arbiter 는 risk 와 acceptance 를 검토하며, captain 이 fan-in 을 책임집니다. 각 specialist handoff 에서는 captain 과 Way 가 task-specific expertise framing 을 넣어 subagent 가 자신의 role, stance, thinking mode 를 바로 알 수 있게 합니다.
- routed host subagent 가 fan-in 전에 멈추면 fallback reason 을 기록하고, degraded captain-local fallback 전에 bounded retry, reassign, 또는 codex exec worker harness 로 회수합니다.
- 작은 docs 작업이 optional review만 필요하다면 bounded status polling 과 visible follow-up 을 유지해서 작업이 무한정 기다리지 않게 하고, silent waiting 대신 reclaim, retry, reassign 을 사용합니다.
- routing 이 어긋나면 captain 이 drift 를 기록하고 matching CCC specialist 로 다시 라우팅한 뒤, adoption 이나 repair 결과를 merge 전에 review 합니다.
- LongWay row 는 operator 가 작업을 따라가기 쉬울 때만 optional owner identity 를 표시할 수 있습니다. 예: `[ ] Mill [ccc_scribe] : Clarify 0.0.15 docs routing requirements`.
- `ccc graph`와 `ccc_code_graph`는 CCC-owned graph-facing surface로 유지됩니다. `graph_context`가 켜져 있고 Graphify가 준비되어 있으면, 기존 graph-facing surface는 Graphify-backed provider/routing shim을 통해 동작합니다. 이 경로는 config-gated이며 기본값은 off입니다. Graphify output은 read-only evidence로 유지되고, Graphify가 missing/stale이면 legacy graph backend 대신 normal scout/source evidence로 fallback합니다. 새로운 public graph command은 추가하지 않습니다.
- `ccc memory`는 opt-in workspace memory입니다. preview/write 확인 뒤에 사용자 선호, 반복 규칙, 검증된 프로젝트 사실만 저장하며 LongWay/run state/latest work result/inference-only observation은 memory truth로 쓰지 않습니다.
- Status는 current task-card owner가 추론된 specialist family와 맞지 않을 때 assignment-quality routing drift warning을 표시합니다.
- release asset packaging, `install.sh`/`install.ps1` repair, `gh release upload`/`gh release edit` 는 먼저 적절한 specialist 또는 operator role 로 라우팅합니다.
- docs/translation 요청은 generated routing defaults가 적용될 때 `ccc_scribe`로 라우팅합니다.
- `$cap`은 단독으로 동작합니다. `/plan`이나 `/goal`을 CCC entry path처럼 문서화하지 않습니다.
- PLAN_SEQUENCE와 EXECUTE_SEQUENCE는 분리됩니다. 넓거나 위험하거나 모호한 작업, release/branch/multi-file 작업은 pending LongWay 승인 전 실행하지 않습니다.
- public `skills/cap/SKILL.md`는 얇게 유지합니다. 내부 routing, lifecycle, fan-in, fallback, context, compatibility policy는 `CCC_MEMORY.md` 또는 persisted `captain_instruction` guidance에 둡니다.
- Planned row의 canonical truth는 `longway.planned_rows`입니다. `phase_rows[].planned_rows`는 status/checklist projection 전용이고, matching `task_card_id` row만 phase 아래 표시하며 unmatched row는 top-level Planned row로 남깁니다.
- long-session rollover 안내는 먼저 checkpoint를 요구하고 `/compact`, `/new`, `/exit` 중 operator가 선택하게 둡니다.

현재 release notes는 [`docs/release/notes/v0.0.15-pre.md`](./docs/release/notes/v0.0.15-pre.md) 를 보세요.
