# Codex-Cli-Captain

[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md)

Rust runtime for the `ccc` Codex CLI harness.

Current source version: `0.0.15-pre`.

> Supported release targets are exactly `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`, and `windows-x86_64`. macOS targets are normally supported and expected to work. Linux and Windows targets are available, but may still have platform-specific issues.

## Local Install & Update

```bash
cargo build --offline
ccc setup
```

Then fully exit Codex CLI, start a new Codex CLI session, and run:

```bash
ccc check-install
```

For an existing local source checkout, pull or rebuild the latest source, rerun `ccc setup`, fully restart Codex CLI, and then run `ccc check-install`. `setup` refreshes the MCP registration, packaged `$cap` skill, and CCC-managed custom agents from the current binary and `ccc-config.toml`. The release installers stay pinned to `v0.0.15-pre` by default; set `CCC_VERSION` only for an intentional override. They stage the new bundle before switching the active path, keep previous release bundles for rollback, and clean only CCC-managed plugin cache/version entries plus the legacy `skills/cap` copy. Non-CCC Codex config is preserved.
Optional TypeScript/JavaScript LSP setup is recorded in config for future `lsp_diagnostics`, `lsp_references`, `lsp_definition`, `lsp_prepare_rename`, and `lsp_rename` support. Install it with `npm install -g typescript typescript-language-server` if you want the server available locally. Runtime LSP execution is deferred in `0.0.15-pre`; CCC does not start language servers yet. Optional `rust-analyzer` remains Rust-only local navigation support and can be installed with `rustup component add rust-analyzer`.

## Config Refresh

You can edit `~/.config/ccc/ccc-config.toml` to change each CCC role's model, reasoning tier, and fast-mode setting. Fresh installs generate `~/.config/ccc/ccc-config.toml` with `way`, `explorer`, `code specialist`, and `verifier` set to reasoning `variant = "high"` and `documenter`, `companion_reader`, and `companion_operator` set to `variant = "medium"`; all keep `fast_mode = true`. `ccc setup` preserves existing user-customized values while backfilling missing generated defaults or upgrading stale CCC-generated defaults. After editing it, paste this into Codex CLI:

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`setup` re-syncs the managed `$cap` skill, MCP registration, and CCC-managed custom agents from `ccc-config.toml`.

## 0.0.15-pre Behavior

- `$cap` is the public entrypoint.
- Configured `ccc_*` custom agents are the required specialist path while available. Generic `worker` and `explorer` labels are rejected for specialist-owned work unless the operator records an explicit override or fallback.
- `ccc memory` is opt-in and unconfigured by default.
- `status`, `checklist`, and projection expose a compact `state_contract` with the active gate, required artifact, and next allowed transition.
- Mutation-capable dispatch requires persisted evidence or approved LongWay/task-card scope before `ccc_raider` can run.
- The packaged CCC plugin distributes `.codex-plugin/plugin.json`, `.mcp.json`, and `skills/ccc/SKILL.md`; these files are install/discovery packaging, while operators invoke CCC with `$cap`.
- `recovery_lane` makes fallback, reclaim, retry, and reassign visibility explicit instead of hiding recovery in suffix text.
- `post_fan_in_captain_decision` is the canonical captain decision envelope after fan-in; report, retry, reassign, recovery, and advance all derive from that persisted truth.
- `ccc status --subagents --text` and `ccc checklist --subagents --text` show one compact row per subagent lane with display callsign plus stable ID where feasible, for example `role=Observer(ccc_scout)/explorer`; `ccc status --app-panel --text` keeps the richer route and source-label detail.
- `ccc status --projection --json '{...}'` and `ccc checklist --projection --json '{...}'` update one workspace-root `CCC_LONGWAY_PROJECTION.md` file so the LongWay/subagent view can be reviewed through `git diff -- CCC_LONGWAY_PROJECTION.md`. The file is overwritten on the next projection update, and CCC marks it intent-to-add so first creation is diff-visible.
- Projection headings follow the operator request language when CCC can detect it; Korean `$cap` requests render Korean projection labels.
- Terminal host-subagent updates release CCC's active handle, and recovery stays visible until fan-in or merge is complete.
- Mutation completion waits for specialist fan-in, and arbiter review remains the final gate for review-sensitive changes.

Stable `ccc_*` IDs are the source of truth; callsigns are display-only. `captain/orchestrator` maps to Command Center (or Captain, not a managed `ccc_*` role). `ccc_tactician/way` maps to Executor, `ccc_scout/explorer` to Observer, `ccc_raider/code specialist` to Marauder, `ccc_scribe/documenter` to Adjutant, `ccc_arbiter/verifier` to Arbiter, `ccc_sentinel` to Overseer, `ccc_companion_reader` to Probe, and `ccc_companion_operator` to SCV.

Host UI layers may still emit outer notifications such as `Closed Carver [ccc_scout]`; that wording is host-managed and not guaranteed by CCC. CCC-controlled status/projection output uses callsign-plus-stable-ID forms such as `Observer(ccc_scout)`.

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

Optional future agents are documented only and are not shipped runtime roles in `0.0.15-pre`: `ccc_release_arbiter`/Judicator for release gating, `ccc_qa_runner`/Valkyrie for test execution, and `ccc_lsp_scout`/Science Vessel for deeper language-server evidence.

## Recommended Role Defaults

For regular CCC use, ChatGPT Pro $100 is the recommended starting plan because `$cap` workflows can spend more Codex usage through repeated captain and specialist handoffs. Adjust reasoning by your working style and task risk: keep higher reasoning for broad planning, risky code changes, or reviews, and lower it for narrow, repetitive, or low-risk tasks.

| CCC role | Stable agent ID | Display callsign | Recommended model | Reasoning | Notes |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | Host-owned routing label, not a managed `ccc_*` specialist |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | Planning and bounded next-move selection |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | Read-only repo evidence |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | Code/config mutation and repair |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README, release notes, and operator text |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | Captain-mediated review, risk, regression, and acceptance checks |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | Low-cost filesystem/docs/web/git/gh read work |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | Low-cost bounded git/gh mutation and narrow tool work |

`gpt-5.5` is the recommended high-value role model for ChatGPT-authenticated Codex. If it is not available in the current account or execution path, use `gpt-5.4` for those high-value roles until rollout reaches that path.

## Flow

`$cap` is the public CCC entrypoint. Use it directly for CCC orchestration; host planning surfaces are not part of the CCC contract.

The CCC flow is:

1. `PLAN_SEQUENCE`: captain confirms intent and routes planning to the configured Way agent. Host Plan Mode cannot be used as a background Way engine and must not own or replace CCC planning.
2. Way produces a pending LongWay and candidate task cards. Planning is read-only.
3. The operator approves the pending LongWay.
4. `EXECUTE_SEQUENCE`: captain reloads the approved LongWay, materializes task cards, and routes work through scheduler/router blocks.
5. Specialists return result envelopes. CCC records compact fan-in, checklist/status projection, lifecycle, and evidence.
6. Captain decides whether to continue, repair, replan, reclaim, complete, or create a restart handoff, and mutation completion stays behind specialist fan-in plus arbiter review when required.

Host planning UI can frame what the operator types, but CCC does not trigger or depend on host Plan Mode in the Way agent. If host state conflicts with CCC state, persisted LongWay, checklist, fan-in, and resolve state win.

0.0.15-pre is a docs-and-release-gates pre-release that carries forward the stricter intent-state-machine behavior, adds callsign mapping guidance, and tracks the oh-my-openagent-inspired workflow set: github-triage, get-unpublished-changes, remove-deadcode, ai-slop-remover, lsp-safe-refactor, review-work, pre-publish-review, hyperplan, git-master, publish, release-command-discipline, release-note, readme-maintenance, changelog, role-ownership, lane-conflict, fallback-classification, and filesystem-evidence. It is not full runtime parity or a completed rebuild.

The per-agent workflow mapping is advisory: Observer handles `github-triage` and `get-unpublished-changes`; Marauder handles `remove-deadcode`, `ai-slop-remover`, and LSP safe refactor; Arbiter handles `review-work` and `pre-publish-review`; Executor handles `hyperplan`; SCV handles `git-master`, `publish`, and release command discipline; Adjutant handles release notes, README, and changelog work; Overseer handles role ownership, lane conflict, and fallback classification; Probe handles lightweight GitHub and filesystem evidence collection.

Specialists are selected from `ccc-config.toml`. Host Codex as captain owns LongWay, routing, lifecycle, fan-in, review, validation, and commit boundaries. Ordinary `$cap` work should go to the matching specialist first: read-only investigation to `ccc_scout`, docs/operator text to `ccc_scribe`, code/config mutation to `ccc_raider`, and review judgment to `ccc_arbiter`. The captain should only do the work directly for explicit fallback, trivial operator-side fixes, or recorded CCC degradation. The configured `ccc_*` custom agents are the default specialist names; generic `worker` and `explorer` labels do not apply unless an explicit override says otherwise.

Lightweight filesystem/docs/fetch/git/gh work routes to the configured mini companion roles instead of staying in the captain session when the tool route is backed by a specialist owner. Git and `gh` reads go to `companion_reader`; git and `gh` mutations go to `companion_operator` unless the captain records an explicit fallback or degradation reason.

Raider prompts now reinforce normal engineering boundaries: respect existing modules, split helpers only when they remove real duplication or isolate testable behavior, avoid giant functions, and avoid unrelated rewrites.

The `v0.0.15-pre` operator policy keeps review explicit and conditional. When the captain launches reviewers, it should treat them as bounded checking and verification input, keep accept/reassign/close decisions in the captain, and account for hardware, memory, and same-machine concurrency cost before starting more review work.

That draft says the captain may accept, close, or mark a subagent result unsatisfactory. Unsatisfactory output should be recorded in LongWay/task-card state with rationale and the chosen next action. CCC canonicalizes unsatisfactory or needs-work results into bounded specialist follow-ups, and the captain should not do local repair when CCC can route the repair or reassignment through a specialist. If the original scope still fits, the captain sends one bounded repair to the same specialist with a narrowed prompt that targets the missing delta, risk, or correction. If the role or approach was wrong, the captain sends one bounded reassignment to a better-fit specialist. The previous unsatisfactory result should stay visible in history; CCC should not do subagent-to-subagent handoffs, unbounded retries, scope widening without an explicit replan or re-scope, or silent degraded fallback without an explicit reason.

The planned intervention path is captain-owned. When the captain reviews a result or observes active-work risk and finds the output or direction unsatisfactory, it should record a bounded delta plus rationale in LongWay/task-card state and choose exactly one action: amend the same worker if safe, reclaim stale work when forced interruption is unsupported or scope changed materially, or reassign to a better-fit specialist. User guidance during active work should stay a secondary input through the captain, never a direct user-to-subagent side channel. Stale output should stay visible and cannot overwrite the chosen path unless the captain explicitly merges it. Intervention should use the same bounded retry/reassign budget as dissatisfaction repair, with no unlimited amend loops, scope widening without explicit replan or re-scope, or duplicate mutable workers just because an intervention was recorded.

## Active Requests

When a new `$cap` request arrives while an earlier run or subagent is still active, CCC surfaces the active run and recommends merge, replan, or reclaim handling.

Host custom subagents cannot always be forcibly canceled by CCC, so captain should mark stale work as reclaimed or merged and continue from the combined latest request.

If Codex reports file-descriptor pressure such as `Too many open files (os error 24)`, pause new reviewer or specialist launches. Keep the work single-path until each active host agent has a terminal lifecycle update, is merged or reclaimed by captain, and is closed in the host session so thread/file handles are released.

Terminal host-subagent updates now also release the run-level active handle and keep cleanup state visible in status, including repeated `failed`, `stalled`, `merged`, and `reclaimed` transitions.

When transcript folding hides longer status blocks, use the subagent-only or projection paths:

```bash
ccc status --subagents --text --json '{"run_id":"..."}'
ccc status --projection --json '{"run_id":"..."}'
git diff -- CCC_LONGWAY_PROJECTION.md
```

The projection file is a display artifact only. Persisted run state, task cards, lifecycle, and fan-in remain the source of truth.

## Parallel Lanes

- scout lanes default to 2 read-only lanes when broad or parallel investigation is useful, with a max of 4
- raider lanes default to 2 lanes for broad or multi-file mutation, with a max of 4
- single-file or shared-scope mutation stays sequential

## Release Hygiene

The release repo is intentionally minimal: installer, docs, packaged `$cap` skill, and the compiled `ccc` binary. The release asset builder strips binary symbols when `strip` is available, and the release repo includes a sensitive-string scan before publishing.

Release work should happen on `develop`. When a version is ready, validate on `develop`, merge to `main`, tag and publish from `main`, then fast-forward or merge `develop` back to the released `main` state. Keep source-release docs in this repo and public cards/assets in `Codex-Cli-Captain-Release`.

## 0.0.15-pre Operator Guidance

`0.0.15-pre` documents the current release-facing guidance for the `$cap` public contract, specialist-first routing, callsign mapping, release-gate hygiene, checkpoint/resume guidance, active-handle cleanup, and verification/fan-in visibility.

- When the operator asks for comments or annotations, preserve the requested chronological block format instead of flattening or reordering it.
- Treat OMO sisyphus or harness wording as CCC's operating shape: one captain, bounded specialist routing, and every specialist result returns to captain before the next decision.
- For complex or risky interpretations, captain confirms the reading with the operator before handing accepted work to Way/tactician.
- Keep role separation explicit: scout and companion_reader gather evidence, Way/tactician plans, raider and companion_operator mutate, scribe writes docs or operator text, arbiter reviews, and captain owns fan-in. For each specialist handoff, captain and Way should add task-specific expertise framing so the prompt states the subagent's role, stance, and thinking mode.
- If a routed host subagent stalls before fan-in, record the fallback reason and recover through bounded retry, reassign, or the codex exec worker harness before degraded captain-local fallback.
- If a small docs task only needs optional review, keep bounded status polling and visible follow-up active so the work does not wait indefinitely; reclaim, retry, or reassign instead of silent waiting.
- If routing drifts, captain records the drift, routes the work to the matching CCC specialist, and reviews any adoption or repair before merge.
- LongWay rows may include optional owner identity when it helps the operator follow the work, for example `[ ] Mill [ccc_scribe] : Clarify 0.0.15 docs routing requirements`.
- `ccc graph` and `ccc_code_graph` remain CCC-owned graph-facing surfaces. When `graph_context` is enabled and Graphify is ready, existing graph-facing surfaces route through a Graphify-backed provider/routing shim. This is config-gated and default-off. Graphify output stays read-only evidence, and if Graphify is missing or stale the flow falls back to normal scout/source evidence instead of a legacy graph backend. No new public graph command is added.
- `ccc memory` is opt in and workspace scoped. It stores only small user preferences, repeated rules, and verified project facts after preview/write confirmation; LongWay state, run state, latest work results, and inference-only observations are not memory truth.
- Status now carries assignment-quality routing drift warnings when a current task-card owner does not match the inferred specialist family.
- Release asset packaging, `install.sh`/`install.ps1` repair, and `gh release upload`/`gh release edit` stay routed through the correct specialist or operator role first.
- Documentation and translation requests should route to `ccc_scribe` when generated routing defaults apply.
- `$cap` works by itself. Do not document `/plan` or `/goal` as a CCC entry path.
- PLAN_SEQUENCE and EXECUTE_SEQUENCE stay separate. Broad, risky, ambiguous, release, branch, or multi-file work should not execute before pending LongWay approval.
- The public `skills/cap/SKILL.md` remains thin; internal routing, lifecycle, fan-in, fallback, context, and compatibility policy belongs in `CCC_MEMORY.md` or persisted `captain_instruction` guidance.
- Planned rows remain canonical under `longway.planned_rows`. `phase_rows[].planned_rows` is status/checklist projection only; matched `task_card_id` rows display under matching phases and unmatched rows remain top-level Planned rows.
- Long-session rollover guidance should checkpoint first and then let the operator choose `/compact`, `/new`, or `/exit`.

See [`docs/release/notes/v0.0.15-pre.md`](./docs/release/notes/v0.0.15-pre.md) for the current release notes.
See [`docs/release/notes/v0.0.14-pre.md`](./docs/release/notes/v0.0.14-pre.md) for the previous pre-release note.

## Key Paths

- `rust/ccc-mcp/`: CLI and MCP runtime
- `skills/cap/`: packaged `$cap` skill
- `skills/ccc/`: packaged CCC plugin skill
- `docs/install.md`: install and verification
- `docs/release/notes/v0.0.15-pre.md`: current release notes
- `docs/release/notes/v0.0.14-pre.md`: previous pre-release notes
- `docs/release/RELEASE_WORKFLOW.md`: develop-to-main release workflow
- public GitHub release cards belong in `Codex-Cli-Captain-Release`, not this source repository
