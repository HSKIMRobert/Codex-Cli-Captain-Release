# Install & Update Codex-Cli-Captain

Use this guide for the Rust-only pinned `v0.0.15-pre` install surface.

## Install & Update

1. download the pinned `v0.0.15-pre` bundle from the release repository
2. unpack the archive
3. run `./bin/ccc setup`
4. fully exit Codex CLI
5. start a new Codex CLI session
6. run `./bin/ccc check-install`

For a local source build, the equivalent flow is:

```bash
cargo build --offline
./target/debug/ccc setup
```

Then fully exit Codex CLI, start a new Codex CLI session, and run:

```bash
./target/debug/ccc check-install
```

For updates, repeat the same flow with the selected bundle or rebuilt source. `CCC_VERSION` remains the explicit override, but the public installers stay pinned to `v0.0.15-pre` unless you set it intentionally. `ccc setup` refreshes MCP registration, the packaged `$cap` skill, and CCC-managed custom agents from the current binary and `ccc-config.toml`; fresh and minimal configs omit `companion_agents`, and compatibility comes from runtime default companion snapshots plus backfilling existing or customized `companion_agents` sections. A full Codex CLI restart is required before the host session reads the refreshed skill and agents.

The release bundle also carries the CCC plugin distribution files: `.codex-plugin/plugin.json`, `.mcp.json`, and `skills/ccc/SKILL.md`. Treat those files as install/discovery packaging for CCC, not as a new public command surface. `$cap` stays the public operator entrypoint.

TypeScript/JavaScript LSP settings are config surface for future `lsp_diagnostics`, `lsp_references`, `lsp_definition`, `lsp_prepare_rename`, and `lsp_rename` support. In `0.0.15-pre`, runtime LSP execution is deferred and CCC does not start a language server. Optional `rust-analyzer` remains Rust-only local navigation support.

Graph context readiness is tracked separately from LSP readiness. When graph context is enabled, `setup` and `check-install` should surface graph-readiness artifacts and state such as `GRAPH_REPORT.md`, `graph.json`, stale or missing output, and the resulting scout/source evidence path. This wording intentionally avoids claiming Graphify CLI availability is reported unless runtime evidence in the install surface shows it. In enabled `graph_context` mode, legacy graph fallback stays disabled; if Graphify is missing or stale, the runtime still uses the normal scout/source evidence flow rather than reviving the legacy graph path. When graph context is disabled, the legacy code graph path remains available. Graphify should not auto-install external dependencies unless the operator explicitly opts in.

The installers stage the new bundle before switching the active path, preserve previous release bundles for rollback, and clean only CCC-managed plugin cache/version entries plus the legacy `skills/cap` copy. Non-CCC Codex config is left alone. If the new bundle needs to be backed out, reinstall the earlier release bundle or point `CCC_VERSION` at the prior asset instead of deleting it during repair.

## Reapply Config Changes

After editing `~/.config/ccc/ccc-config.toml`, paste this into Codex CLI:

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`setup` reads the config, refreshes MCP registration, installs the packaged `$cap` skill, and syncs CCC-managed custom agents. Fresh and minimal configs omit `companion_agents`; runtime default companion snapshots preserve compatibility, and `setup` backfills existing or customized `companion_agents` sections. Restarting Codex CLI makes the refreshed skill and custom-agent definitions available to the host session.

## Active Requests

If a new `$cap` request arrives while an earlier run or subagent is still active, CCC surfaces the active run and recommends merge, replan, or reclaim handling.

Because host custom subagents cannot always be forcibly canceled by CCC, captain should treat stale work as reclaimed or merged and continue from the combined latest request.

Host Codex as captain owns LongWay, routing, lifecycle, fan-in, review, validation, and commit boundaries. Captain is the orchestrator/control-plane actor, not a normal spawnable specialist. Ordinary read-only investigation, docs edits, code/config mutation, and review judgment should go to `ccc_scout`, `ccc_scribe`, `ccc_raider`, and `ccc_arbiter` via custom subagents when available; direct captain work stays limited to explicit fallback, trivial operator-side fixes, or recorded CCC degradation. Lightweight tool-routed filesystem/docs/fetch/git/gh work should use the configured companion owner when one is selected: git and `gh` reads route to `companion_reader`, and git or `gh` mutations route to `companion_operator` unless the captain records an explicit fallback or degradation reason.

MCP remains the preferred control-plane when available. For long specialist fan-in, pass a stable `event_ref` and `mode: "compact"` to `ccc_subagent_update`; CCC records summary/status visibility inline and persists the full fan-in under the run-local `temp-artifacts/subagent-update/` directory.

If the host reports file-descriptor pressure such as `Too many open files (os error 24)`, stop opening additional reviewers or specialists. Record terminal lifecycle state for every active host agent, merge or reclaim it through captain, and close the host agent before continuing so file and thread handles are released.

Terminal host-subagent updates also release the run-level active handle and keep cleanup state visible in status, including repeated `failed`, `stalled`, `merged`, and `reclaimed` transitions.

## 0.0.15 Operator Guidance

`$cap` is the public CCC entrypoint and works without host `/plan` or `/goal`.
Those host surfaces are outer affordances: `/plan` or host Plan Mode can frame
planning, but it must not own or replace CCC `PLAN_SEQUENCE` and the configured
Way agent; `/goal` can act as an outer
objective hint, but it does not replace CCC LongWay, checklist, fan-in, status,
or completion gates.

0.0.15-pre separates planning and execution:

- `PLAN_SEQUENCE` is read-only and produces a pending LongWay for operator
  approval.
- `EXECUTE_SEQUENCE` starts after approval, materializes task cards, and routes
  work through scheduler/router blocks.
- Checklist, status, fan-in, lifecycle artifacts, and persisted run state are
  the source of truth for progress and completion.
- Context-health status should provide restart handoff guidance when pressure
  gets high.
- The public installed `skills/cap/SKILL.md` stays thin; internal routing,
  lifecycle, fan-in, fallback, context, and compatibility policy belongs in
  release-work guidance or persisted `captain_instruction` guidance.

0.0.15-pre is a docs-and-release-gates pre-release that carries forward the
stricter intent-state-machine behavior, adds callsign mapping guidance, and
tracks the workflow set documented in the README. It is not a claim of full
runtime parity or a completed rebuild.

For comment or annotation requests, keep the requested chronological block format and do not flatten or reorder the content.

When captain or Way hands work to a specialist, include task-specific expertise framing in the prompt so the subagent sees its role, stance, and thinking mode before it starts.

If a request could touch release asset packaging or install repair, captain should first confirm the interpretation with the operator, then route packaging or install code changes to `ccc_raider` and GitHub release mutation to `ccc_companion_operator`.

If a routed subagent stalls before fan-in, prefer reclaim, retry, or reassign before degraded captain fallback when delegation is still viable.

If a small docs task only needs optional review, keep bounded status polling and visible follow-up active so the work does not wait indefinitely; reclaim, retry, or reassign instead of silent waiting.

If routing drifts, captain records the drift, routes the work to the matching CCC specialist, and reviews any adoption or repair before merge.

For broad, risky, ambiguous, or irreversible work, captain should ask 1-3 high-signal clarification questions before execution or Way handoff. For narrow work, captain should proceed with explicit assumptions instead of opening an interview loop.

Way plans the full LongWay before dispatch, and each row should map to one subagent or task unit. LongWay rows may include optional owner identity, planned role, lifecycle, and compact planning detail when it helps the operator follow the work, for example `[ ] Planned: Clarify 0.0.15 docs routing requirements [ccc_scribe] role=documenter plan: scope="operator docs" accept="docs updated"`.

If Way used `ccc graph` or `ccc_code_graph` evidence, say that explicitly in the visible output instead of omitting the graph signal.

Treat `ccc memory` as an explicit opt-in command surface with `status`, `preview`, `write`, and `off` actions plus workspace-scoped storage. The `$cap` skill handles invocation and routing behavior, not durable memory storage, and runs do not automatically read memory unless the operator invokes it.

Docs and translation requests should route to `ccc_scribe` when generated routing defaults apply. Long-session rollover guidance should checkpoint first and then let the operator choose `/compact`, `/new`, or `/exit`.

## Parallel Lanes

- scout lanes default to 2 read-only lanes when broad or parallel investigation is useful, with a max of 4
- raider lanes default to 2 lanes for broad or multi-file mutation, with a max of 4
- single-file or shared-scope mutation stays sequential

## What `setup` Does

- registers or refreshes the MCP entrypoint
- creates `~/.config/ccc/ccc-config.toml` on first install using the canonical shared-config format
- reuses the existing `~/.config/ccc/ccc-config.toml` when it is already present
- migrates or reads previous CCC config fallbacks when present
- backfills missing `companion_agents` sections for existing customized configs and fills missing fields inside existing `routing`, `tool_routing`, or `runtime` sections while preserving user-customized values
- migrates legacy TOML config when present
- migrates legacy JSON config when present
- installs or refreshes the packaged `$cap` skill
- syncs CCC-managed spawnable Codex custom agents under `CODEX_HOME/agents`; captain remains configured as the orchestrator role but is not synced as a spawnable custom agent

The generated shared TOML config includes default per-role `model`, reasoning tier (`variant`), `fast_mode`, companion-agent compatibility via runtime default snapshots, and linked feature settings. Fresh installs keep routing, tool-routing, and runtime policy in code unless the operator customizes those sections, and they omit `companion_agents` entirely. Fresh installs set `explorer` to `variant = "high"` and `documenter`, `companion_reader`, and `companion_operator` to `variant = "medium"`; all keep `fast_mode = true`. `ccc setup` preserves user-customized values, backfills missing generated defaults, and upgrades stale generated defaults to version 16.

## Recommended Role Defaults

For regular CCC use, ChatGPT Pro $100 is the recommended starting plan because `$cap` workflows can spend more Codex usage through repeated captain and specialist handoffs. Adjust reasoning by your working style, task risk, and observed token usage: keep higher reasoning for broad planning, risky code changes, or reviews, and lower it for narrow, repetitive, or low-risk tasks.

| CCC role | Agent | Recommended model | Reasoning | Notes |
| --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `gpt-5.5` | `medium` | LongWay ownership and final routing judgment |
| `way` | `tactician` | `gpt-5.5` | `high` | Planning and bounded next-move selection |
| `explorer` | `scout` | `gpt-5.4-mini` | `high` | Read-only repo evidence |
| `code specialist` | `raider` | `gpt-5.5` | `high` | Code/config mutation and repair |
| `documenter` | `scribe` | `gpt-5.4-mini` | `medium` | README, release notes, and operator text |
| `verifier` | `arbiter` | `gpt-5.5` | `high` | Captain-mediated review, risk, regression, and acceptance checks |
| `companion_reader` | `companion_reader` | `gpt-5.4-mini` | `medium` | Low-cost filesystem/docs/web/git/gh read work |
| `companion_operator` | `companion_operator` | `gpt-5.4-mini` | `medium` | Low-cost bounded git/gh mutation and narrow tool work |

`gpt-5.5` is recommended for the high-value roles when Codex is signed in with ChatGPT. If that model is unavailable for the current account or execution path, use `gpt-5.4` for those roles until rollout reaches that path.

The current operator policy treats review as explicit and conditional, not attached to every agent task. Treat reviewers as bounded verification input, keep accept/reassign/close decisions with the captain, and account for hardware, memory, token, and same-machine concurrency cost before launching reviewers.

Under that draft, after a subagent result returns, the captain may accept it, close it, or mark it unsatisfactory. Unsatisfactory output should be recorded with rationale and next action in LongWay/task-card state. CCC canonicalizes unsatisfactory or needs-work results into bounded specialist follow-ups, and the captain should not do local repair when CCC can route the repair or reassignment through a specialist. If the original scope still fits, the captain sends one bounded repair to the same specialist with a narrowed prompt that targets the missing delta, risk, or correction. If the role or approach was wrong, the captain sends one bounded reassignment to a better-fit specialist. The previous unsatisfactory result should stay visible in history; do not hand work directly between subagents, widen scope without an explicit replan or re-scope, retry in an unbounded loop, or fall back silently without an explicit reason.

The planned intervention path keeps active-work feedback routed through the captain. The captain should record the intervention as a bounded delta plus rationale in LongWay/task-card state, classify it as clarification-only, bounded scope amendment, or direction/risk correction, and choose exactly one action: same-worker amend if safe, reclaim if forced interruption is unsupported or scope changed materially, or reassignment to a better-fit specialist. Stale output should stay visible and cannot overwrite the chosen path unless the captain explicitly merges it. Intervention should use the same bounded retry/reassign budget as dissatisfaction repair, so there are no unlimited amend loops, scope widening without explicit replan or re-scope, or duplicate mutable workers solely for intervention.

## Status And Tokens

Prefer `--text`, `--quiet`, and `--json-file` for lower-noise repeated lifecycle calls. `ccc status --text` prints token totals and a stacked gauge only when raw delegated-worker usage events are available. `ccc status --app-panel --text` prints the Codex-app-friendly LongWay/status fallback panel without the full status payload; omit `--text` to emit only the structured `app_panel` JSON. Quiet lifecycle lines (`ccc status --quiet`, `ccc start --quiet`, `ccc orchestrate --quiet`, `ccc subagent-update --quiet`) include compact token usage fields and explicit unavailable reason codes. Structured status/activity payloads also expose `token_usage_visibility.status` and `token_usage_visibility.unavailable_reason_code` so JSON consumers do not have to infer meaning from `token_usage: null`. Host-side custom subagent token totals are best-effort only; when raw usage is unavailable, CCC prints and persists a clear unavailable reason instead of inventing numbers.

## Check-Install Contract

`ccc check-install` is the stable install-health surface.

Expected top block:

```text
CCC install check: status=ok version=0.0.15-pre entry=$cap registration=matching_registration config=canonical-current config_action=preserved config_restart=not-required skill=matching_install
Install surface: status=current restart=not-required mcp=matching_registration skill=matching_install custom_agents=matching_sync
```

Base install expectation:

- `status=ok` when the Rust MCP registration, config file, `$cap` skill, and CCC-managed custom agents all match the local binary
- `status=warning` when one of those surfaces is missing or mismatched
- `installSurfaceVisibility` groups `mcp_registration`, `ccc_config`, `cap_skill`, and `custom_agents` with normalized `status`, `action_status`, `restart_status`, and summary fields
- setup/check-install reports whether those surfaces are current, missing, stale, migrated, conflicting, unreadable, and whether setup plus Codex CLI restart is required
- when graph context is enabled, setup/check-install also surfaces graph-readiness artifacts and state such as `GRAPH_REPORT.md`, `graph.json`, stale or missing graph output, and the resulting scout/source evidence path
- missing Graphify is a warning, not a fatal install failure, unless the operator explicitly opted into a stricter readiness gate
- missing or stale Graphify does not revive the legacy graph fallback path; normal scout/source evidence remains the fallback flow
- Graphify auto-install is not the default path for install repair; external dependency installation requires explicit opt-in and readiness handling
