# Project Plan

## Vision

Codex-Cli-Captain will provide a practical orchestration and visibility layer for Codex CLI work, with clear role separation and explicit handoffs. The aim is not to replace Codex CLI, but to keep workflow state readable and resilient while Codex CLI remains the execution engine.

## Problem statement

Single-session agent work can blur planning, execution, and verification. When that happens, tasks become harder to trace, review quality drops, and the final result is harder to trust. If the orchestration layer is too tightly coupled to Codex CLI internals, upstream changes can also break the workflow, hide agent activity, or make failures hard to localize.

## Goals

- Separate planning, execution, and verification into distinct roles.
- Keep orchestration local, lightweight, and thin over Codex CLI.
- Own workflow state, task state, handoff state, active-agent visibility, and child-agent visibility in CCC.
- Make task progress easy to understand from documentation and workflow state.
- Support repeatable work handoffs between roles.
- Reach an MVP that can guide Codex CLI work without a large framework around it.
- Prefer stable Codex CLI interfaces over unstable internals.
- Preserve graceful fallback behavior when upstream CLI behavior changes.
- Report failures with separate normalized stage and reason fields.

## Non-goals

- Full project management features.
- Remote collaboration or multi-user permissions.
- Opinionated agent frameworks beyond the four core roles.
- Low-level implementation design before the workflow is agreed.
- Broad automation outside Codex CLI orchestration.
- Coupling to transient Codex CLI internals that would make the layer fragile across updates.
- Treating experimental or undocumented Codex CLI surfaces as foundational.

## Canonical terms

This section is the single canonical source for the core workflow terms used across CCC docs.

### Role

A role is a stable responsibility boundary in the workflow. Roles define what kind of work is being done and who is accountable for that step. In the MVP, the canonical roles are orchestrator, planner, code specialist, and verifier. Named agent labels may be documented separately, but they do not change the canonical role set.

### Agent

An agent is a concrete worker instance operating in a role during a run. A run can involve multiple agents over time, including multiple agents serving the same role in separate steps or retries.

CCC only persists a named agent id when it currently assigns a concrete worker; otherwise `active_agent_id` / `assigned_agent_id` may be `null`, which means no concrete worker is currently assigned.

### Child agent

A child agent is an agent delegated by another active agent to complete a bounded subtask. Child agents are visible within the parent run and task context, but they do not become the owner of the overall run. Their status must remain attributable to the parent task and visible to the operator.

### Run

A run is the top-level CCC-tracked workflow instance for a single user goal, milestone slice, or explicitly bounded unit of work. A run owns the normalized workflow state, task progression, handoff history, visibility state, and references to the Codex CLI execution attempts that occur inside it.

### Task card

A task card is the normalized unit of work inside a run. It records the current objective, scope, assigned role or agent, expected outcome, and the latest verification or failure state for that work item.

### Handoff

A handoff is the explicit transfer of responsibility from one role or agent to another. A handoff should carry enough context that the next step can continue without reinterpreting the run from scratch.

## Role responsibilities

### Orchestrator

Owns the session flow, assigns work to the right role, and keeps the overall sequence aligned with the current milestone.

### Planner

Turns the project goal into a concrete plan, defines scope for each step, and keeps milestone boundaries clear.

### Code specialist

Handles focused build work for the current task, staying inside the scope handed off by the planner or orchestrator.

### Verifier

Checks that output matches the stated goal, looks for gaps, and confirms whether the work is ready for the next phase.

Host Codex as captain owns LongWay, routing, lifecycle, fan-in, review, validation, and commit boundaries. Ordinary read-only investigation, docs edits, code/config mutation, and review judgment should be delegated to `ccc_scout`, `ccc_scribe`, `ccc_raider`, and `ccc_arbiter` via custom subagents when available; direct captain work should stay limited to explicit fallback, trivial operator-side fixes, or recorded CCC degradation.

See `docs/reference/agent-roster.md` for the operator-facing named agent roster that maps to these roles.

## Captain-mediated review

CCC review is captain-mediated, not free agent-to-agent chat. The review step is a bounded checking and verification input that the captain may request when the task risk justifies it.

Review is explicit and conditional, not attached to every agent task. The captain keeps the final accept, reassign, and close decisions after the reviewer responds.

Before launching reviewers, the captain should account for hardware, memory, token, and same-machine concurrency burden. Review gating should be resource-aware:

- no review for low-risk or routine work
- a single bounded review for moderate risk or unclear outcomes
- a required review for high-risk, critical, or failed work

Reviewer and pass counts must stay bounded. CCC should not create unbounded review swarms or open-ended reviewer chains.

### Captain dissatisfaction handling

After a subagent result returns, the captain may accept it, close it, or mark it unsatisfactory.

Unsatisfactory output must be recorded in LongWay/task-card state with the rationale and the chosen next action.

When the original scope still holds, the captain sends one bounded repair to the same specialist and narrows the next prompt to the missing delta, risk, or correction target.

When the role or approach was wrong, the captain sends one bounded reassignment to a better-fit specialist.

The previous unsatisfactory result must remain visible in history. CCC must not hide it, overwrite it, hand it directly to another subagent, widen scope without an explicit replan or re-scope, retry in an unbounded loop, or fall back silently without an explicit reason.

### Captain review intervention and active-work amendment

The primary intervention path is captain-initiated. When the captain reviews a subagent result or observes active-work risk and finds the output or direction unsatisfactory, it records the concern in LongWay/task-card state and intervenes with a bounded repair, reclaim, or reassignment decision.

User feedback during active work is a secondary input to the same captain-owned path. If the user adds guidance while a subagent is still active, the request must enter through the captain; CCC should not create a direct, untracked user-to-subagent side channel.

The captain records any intervention as a bounded delta with rationale, then classifies it as one of:

- clarification-only
- bounded scope amendment
- direction or risk correction

The captain then chooses exactly one action:

- amend the same worker when the current worker can safely continue with a narrowed prompt
- reclaim the active work when forced interruption is unsupported or the scope changed materially
- reassign to a better-fit specialist when the role or approach is wrong

Host custom subagents cannot always be forcibly interrupted. If a worker is reclaimed but continues and later returns output, that stale output must stay visible as `late_subagent_output` and cannot silently overwrite the captain's chosen reclaimed lifecycle or fan-in path unless the captain explicitly merges it.

Intervention uses the same bounded retry and reassignment budget as dissatisfaction repair. It must not create unlimited amend loops, widen scope without explicit replan or re-scope, or start duplicate mutable workers on the same scope solely because an intervention was recorded.

Because reclaim overlap can temporarily increase CPU, token, and memory pressure, intervention should prefer one visible path and persist only the intervention delta plus the captain decision artifact rather than replaying full transcripts.

## Commit discipline

CCC work should be committed by coherent work unit. A release or feature effort can contain multiple commits, but each commit should group one bounded purpose such as runtime change, docs update, release metadata, or test repair.

Avoid one oversized mixed commit that combines unrelated code, docs, release, and operator-surface changes. When a task spans multiple work units, captain should either create separate commits or explicitly record why a combined commit is safer.

## Config regeneration and migration policy

CCC needs an explicit regeneration and migration policy for default changes such as new model releases, role-model policy updates, custom-agent template changes, and operational fixes like subagent cleanup.

The policy should separate generated defaults from user-owned values:

- checked-in defaults define the latest recommended config for new installs
- existing `ccc-config.toml` values are user-owned and must not be overwritten silently
- migration should backfill missing keys and update only fields that are still known generated defaults
- user-customized role models, variants, fast-mode settings, tool routing, and companion settings must be preserved unless the operator explicitly asks to reset them
- every migration should produce a concise summary of changed, preserved, skipped, and conflict fields
- dry-run must not write files and should report planned migration, backfill, backup, and restart actions before setup applies them
- setup must create a timestamped backup before overwriting or migrating an existing config source; rollback is explicit only and the named-backup helper is available through `ccc setup --rollback-config <backup_path>`

For model updates, CCC should support a safe regeneration flow:

1. detect the current config version, generated-default marker, and role-model drift
2. create a timestamped backup before modifying `ccc-config.toml`
3. support dry-run output that shows the proposed model/config changes
4. apply the migration only after explicit operator action, or through a documented setup flag
5. run `ccc setup` to regenerate the packaged `$cap` skill and CCC-managed custom agents from the migrated config
6. require Codex CLI restart when agent definitions or MCP registration changed

For release work, default-model changes should update the source defaults, generated custom-agent templates, README/install tables, release notes, release repo docs, GitHub release card, release asset, and tests in the same coherent work unit unless captain records why the update is split.

The migration test matrix should include new install, legacy config migration, user-customized model preservation, generated-default model upgrade, missing-key backfill, dry-run output, backup creation, setup resync, and rollback from backup.

## 0.0.4 work checklist

The `0.0.4` release should make Captain-Mediated Review and Captain Review Intervention complete workflow features, not only documentation concepts.

Runtime slice landed: host-subagent lifecycle helpers now live in `rust/ccc-mcp/src/host_subagent_lifecycle.rs`, terminal host-subagent updates release run-level active handles back to the captain, archive released thread-handle history, surface `active_handle_cleanup` in status, and include repeated terminal-update stress coverage across `failed`, `stalled`, `merged`, and `reclaimed` paths so repeated terminal updates cannot leave stale active host refs.

Runtime slice landed: delegated worker raw-events parsing and worker completion snapshot helpers now live in `rust/ccc-mcp/src/worker_events.rs`, keeping the completion/failure classification and worker result JSON shape unchanged while reducing the oversized MCP entrypoint.

Runtime slice landed: initial review-policy plumbing now persists a captain-owned `review_policy` decision on new runs and task cards, covering skipped low-risk work, recommended single review for moderate-risk mutation, required review for high-risk or explicit review work, and resource-limit suppression, with reviewer caps and policy state visible in status output. This slice does not spawn reviewer workers.

Runtime slice landed: review-policy creation now accepts a bounded runtime-pressure snapshot and suppresses review when local truth surfaces show high pressure from active-run continuity, stale/timed-out/reclaim-needed workers, or configured token soft-limit pressure. The persisted policy records resource-pressure metadata, keeps explicit request-text suppression intact, and suppressed decisions still create no review task card.

Runtime slice landed: explicit file-handle pressure request text, including `Too many open files`, `os error 24`, `EMFILE`, and file descriptor/open-file pressure variants, now suppresses review through the existing resource-limit path with focused review-policy tests.

Runtime slice landed: compact status `subagent_update` command templates now list the accepted lifecycle statuses (`spawned`, `acknowledged`, `running`, `stalled`, `completed`, `failed`, `merged`, and `reclaimed`) so operator guidance matches parser/runtime behavior.

Runtime slice landed: captain-owned review task-card creation now opens exactly one verifier/arbiter review task card when an explicit persisted review policy requires or recommends review, links it to the source task card, leaves skipped/suppressed policy without a review card, avoids duplicate review cards, and keeps the source task active. This slice does not add reviewer orchestration, queues, scheduling, or multi-review behavior.

Runtime slice landed: review lifecycle fan-in now records read-only reviewer outcomes through the existing subagent update path, covering `passed`, `needs_work`, `blocked`, `stalled`, and `reclaimed` outcomes with reviewed task-card links, evidence paths, unresolved findings, captain next-action hints, and explicit captain authority. This slice exposes the state in full, compact, and text status output without spawning reviewers or allowing reviewer findings to auto-accept or auto-reassign work.

Runtime slice landed: captain dissatisfaction/intervention state now records through the existing subagent update path, covering classification, rationale, chosen next action, retry/reassign budget snapshot with visibly blocked exhausted choices, stale-output preservation summary, evidence paths, and open questions. `ccc_subagent_update` now queues `captain_intervention.pending_follow_up` for same-worker amend or explicit reassign, and `ccc_orchestrate` consumes that queue into exactly one follow-up task card, marks it consumed, dedupes duplicate consumption by key, and only creates follow-up when the retry/reassign budget is valid and positive. Missing or malformed budgets are visibly blocked as unavailable, explicit reassign follow-ups require `reassign_target`, missing targets are visibly blocked, and budget-exhausted interventions still remain blocked with no follow-up. The latest artifact is persisted on the task card and run summary and is visible in full, compact, activity, and text status output.

Runtime slice landed: `ccc-config.toml` setup migration/backfill now has a no-write `ccc setup --dry-run` planner, creates timestamped backups before setup modifies or migrates existing config sources, preserves customized values while backfilling generated defaults, surfaces backup/restart guidance in setup/check-install state, and supports explicit named-backup rollback through `ccc setup --rollback-config <backup_path>`. Newly generated `~/.config/ccc/ccc-config.toml` defaults also set the `gpt-5.4-mini` mini roles to `variant = "high"` and `fast_mode = true` while preserving user-customized values.

Runtime slice landed: route-backed companion tool ownership now becomes the selected specialist even for Way-shaped requests, so git/gh reads select `companion_reader` and git/gh mutations select `companion_operator` unless captain records fallback/degradation. Status/activity payloads now also expose structured `token_usage_visibility.status` and `token_usage_visibility.unavailable_reason_code` fields when raw usage is unavailable.

Runtime slice landed: setup/check-install visibility now groups MCP registration, canonical config, packaged `$cap` skill, and CCC-managed custom agents under `installSurfaceVisibility`, with normalized current/missing/stale/migrated/conflict/unreadable status, setup action, and restart requirement. The packaged `$cap` skill check now compares installed content against the packaged source instead of only checking for file existence.

Runtime slice landed: CCC entry recommendation and deterministic auto-entry policy helpers now live in `rust/ccc-mcp/src/entry_policy.rs`. Document/checklist-backed requests that ask to finish or continue work to completion now persist `completion_discipline` through recommendation, auto-entry, run/task-card state, compact status, and human status so captain-visible completion criteria are not confused with a first bounded checkpoint.

Runtime slice landed: review fan-in helper logic now lives with `rust/ccc-mcp/src/review_policy.rs`, including review task detection, accepted review outcome validation, inferred reviewer outcomes, pass-cap blocking findings, verification-state mapping, and review fan-in payload construction. The existing reviewer lifecycle JSON contract and parser validation behavior are preserved.

Runtime slice landed: shared config path, JSON/TOML document IO, generated id, backup, and atomic text write helpers now live in `rust/ccc-mcp/src/config_io.rs`, while the crate-level helper names stay available to existing setup, install-check, run locator, specialist-role, and runtime modules.

Docs slice landed: release workflow docs now include commit-boundary guidance for separating runtime changes, test repairs, source docs, release-repo docs, release metadata, and generated assets into coherent work-unit commits.

- Define review state in the run and task-card model, including explicit review links such as `review_of_task_card_ids`, the captain's review gate decision, review pass count, reviewer status, unresolved findings, and the final captain resolution.
- Extend captain-owned review creation beyond the minimal single-card persistence slice only after reviewer orchestration, resource limits, and lifecycle cleanup have separate bounded contracts. Specialists should not directly spawn other specialists or create untracked review chains.
- Complete resource-aware gating policy for v0.0.4: runtime-pressure suppression now covers local active-run continuity, worker reclaim pressure, file-handle pressure, configured token soft-limit pressure, low OS memory availability, and single-thread CPU pressure. Failed validation, failed verification, failed acceptance, unresolved acceptance, and failing-test signals now require a bounded captain-owned review gate through the existing review-policy path.
- Cap review concurrency and retry depth. The default persisted cap now prefers no review or one reviewer; any multi-review path still needs an explicit maximum active reviewer count, maximum pass count, and stalled/reclaim behavior.
- Keep review read-only by default. Reviewers may report findings, evidence paths, severity, and suggested next action, but the captain decides whether to accept, reassign, repair, or close.
- Update CLI/operator surfaces to show review decisions without implying that every task receives a verifier. The output should make skipped review intentional, not invisible.
- Continue `rust/ccc-mcp/src/main.rs` modularization as repeated seam-by-seam maintenance rather than a one-off cleanup. Each bounded extraction should name the cohesive seam, document the landing, preserve behavior and CLI/MCP JSON contracts, and keep focused regression coverage around the moved surface until `main.rs` is manageable.
- Recent modularization slices have landed for shared config/file IO (`config_io.rs`), worker heartbeat/reclaim/visibility/lifecycle payload helpers (`worker_lifecycle.rs`), status text rendering (`status_render.rs`), shared text compaction (`text_utils.rs`), worker supervisor/launch (`worker_supervisor.rs`), compact status payload construction (`status_compact.rs`), status payload assembly (`status_payload.rs`), orchestration attempt helpers (`orchestration_attempt.rs`), orchestration state mutation helpers (`orchestration_state.rs`), subagent-update identity/policy/fan-in/lifecycle/state-file/run-record helpers (`subagent_update.rs`), MCP tool schema/result/call-response helpers (`mcp_tools.rs`), MCP dispatch (`mcp_dispatch.rs`), and the extracted test module (`main_tests.rs`). `main.rs` is now 2,382 lines and the v0.0.4 oversized-entrypoint modularization target is landed.
- Continue post-v0.0.4 cleanup only as follow-up maintenance unless the final audit finds a concrete behavioral risk. Each future seam should still update the release note ledger with the new `main.rs` line count and remaining seams before any completion claim.
- Keep document/checklist-driven "finish the work" requests tied to persisted completion discipline: derive remaining items from the referenced source, continue bounded slices until every in-scope item is completed/deferred/blocked, and surface that state in status instead of stopping after the first partial slice.
- Captain-owned bounded repair and reassignment execution after a subagent result now flows through queued follow-up state, including the narrowed prompt for the missing delta, risk, or correction target, with exactly one consumed follow-up task card per dedupe key. Follow-up creation requires a valid positive retry/reassign budget; missing or malformed budgets are visibly unavailable, explicit reassign follow-ups require `reassign_target`, missing targets are visibly blocked, and exhausted budgets still produce no follow-up.
- Extend captain-owned intervention handling beyond state capture after review/result evaluation or active-work risk detection, with any user-provided guidance routed only through the captain, no direct untracked user-to-subagent side channel, and exactly one captain action chosen per intervention: same-worker amend when safe, reclaim when forced interruption is unsupported or scope changed materially, or reassignment to a better-fit specialist.
- Keep intervention and dissatisfaction repair execution on the same bounded retry/reassign budget, disallow scope widening without explicit replan or re-scope, and avoid parallel duplicate mutable workers solely because an intervention was recorded. The current persisted slice records the budget snapshot, visibly blocks exhausted selected actions, and produces no follow-up when the budget is already exhausted.
- Record that reclaim overlap can temporarily raise CPU, token, and memory load, so the default should stay single-path and store only the intervention delta plus the decision artifact.
- Add subagent auto-close and cleanup after terminal states so completed, failed, stalled, merged, or reclaimed host subagents release their thread/resource handles and do not need manual captain cleanup before later work can proceed.
- Show cleanup state in lifecycle/status surfaces, including when a host subagent cannot be force-closed and remains visible as stale or reclaimed instead of blocking the run invisibly.
- Add config regeneration and migration policy for new model releases and default changes, including generated-default detection, explicit apply, and rollback from backup. `ccc-config.toml` dry-run, user-value preservation, timestamped backup, setup guidance, restart guidance, explicit named-backup rollback, the `gpt-5.4-mini` mini-role `variant = "high"` / `fast_mode = true` default-policy slice, and generated-default drift upgrade via the current generated-defaults version marker have landed.
- Add migration visibility to setup/check-install so operators can see when packaged `$cap`, MCP registration, and CCC-managed custom agents are current, stale, migrated, or require restart. Initial `ccc-config.toml` backup/backfill/migration visibility and the install-surface visibility summary have landed.
- Add Linux and Windows release-surface support before v0.0.4 release. macOS and Linux native Bash install/update paths are covered by `install.sh`; native Windows install/update is covered by `install.ps1`; release assets, path handling, docs, asset matrix verification, and Windows install smoke verification cover the v0.0.4 platform matrix.
- Enforce commit-per-work-unit discipline in operator guidance and release workflow: runtime, docs, release metadata, and test repairs should be committed separately unless captain records a reason to combine them. Initial release workflow commit-boundary guidance has landed.
- Focused runtime regression coverage landed for the completed/no-intervention path, inferred reviewer failure -> `needs_work`, active reclaim when host cancellation is unsupported, and late stale output preserved as `late_subagent_output` without overwriting reclaimed lifecycle/fan-in; existing reviewer stall/reclaim and repeated completed-subagent coverage remains.
- Add any remaining terminal cleanup edge-case tests beyond the repeated failed/stalled/merged/reclaimed stress coverage. Initial terminal handle release, repeated terminal-update stress coverage, late stale-output preservation, repeated completed-subagent coverage, `os error 24` request-text resource-pressure suppression coverage, and release workflow commit-boundary guidance have landed.
- Add tests for generated-default model upgrade and setup resync after migration. Config dry-run, user-customized value preservation, backup creation, check-install guidance, successful rollback, missing/non-file backup rejection tests, current missing-default backfill coverage, and stale generated-default drift upgrade coverage have landed.
- Sync README, install docs, release notes, packaged `$cap` skill wording, release-repo docs, and packaged binary so operators see one consistent 0.0.4 review, completion-discipline, platform-scope, and active-work intervention contract. The final v0.0.4 continuation sync has landed; rerun the audit if any later runtime/doc change lands before publishing.

## 0.0.5 work checklist

The `0.0.5-pre` docs-sync slice now carries the captain-constrained harness wording through the source docs while the release remains unpublished.

- Landed: source `README.md` now describes the no-mutation preflight gate, supported MCP control-plane surfaces, and the plan to reduce `$cap` toward bootstrap guidance as runtime guards land.
- Landed: localized `README.ko.md` and `README.ja.md` mirror the same 0.0.5 control-plane wording.
- Landed: `docs/release/notes/v0.0.5-pre.md` and `docs/release-work/0.0.5/PRE_RELEASE_PLAN.md` now distinguish the landed runtime slices from the remaining pre-release blockers.
- Blocked: final runtime validation, public release-card publication, release-repo history decision, and any public `$cap` exposure change remain pre-release decisions.

### 0.0.4 completion discipline

`v0.0.4` is not complete merely because one implementation slice landed. The captain must keep a live completion ledger from `docs/release/notes/v0.0.4.md`, classify every planned item as landed/deferred/blocked, and continue bounded slices until no open item remains without an explicit disposition. The current continuation has closed the ledger by landing the runtime/docs/release-sync scope, including OS hardware/memory review-pressure telemetry, failed-validation review scheduling, generated-default drift upgrades, and native Windows installer/runtime smoke release surfaces.

Before release work starts, run a final audit gate: re-read the release note and this project plan, verify `main.rs` modularization is either actually manageable or has explicit deferred seams, run the full source tests plus release-repo non-mutating checks, confirm source/release docs and packaged assets are synchronized, and check both git worktrees are clean. If any item fails that audit, the documented work is not done.

## Model policy

### GPT-5.5 default policy

As of April 24, 2026, OpenAI's Codex model docs recommend starting most Codex tasks with `gpt-5.5` when it appears in the model picker, while also noting that `gpt-5.5` is currently available for ChatGPT-authenticated Codex and not API-key authentication. The latest-model API guide still lists `gpt-5.4` as the API default and says GPT-5.5 API availability is coming soon. The same Codex docs recommend `gpt-5.4-mini` for faster, lower-cost lighter coding tasks or subagents.

Sources:

- Latest model guide: <https://developers.openai.com/api/docs/guides/latest-model>
- Codex recommended models: <https://developers.openai.com/codex/models#recommended-models>

Given the current CCC role split, `0.0.3` checked-in defaults should use `gpt-5.5` for the high-value Codex roles while keeping lower-cost mini defaults for lighter roles.

### Phase 0: v0.0.3 checked-in defaults

Change CCC's checked-in default high-value role models to `gpt-5.5` for ChatGPT-authenticated Codex usage, with `gpt-5.4` as the documented fallback when `gpt-5.5` is unavailable.

Reasoning:

- CCC's preferred path is Codex custom subagents, where OpenAI's Codex docs now recommend `gpt-5.5` when available.
- The same docs say `gpt-5.5` is available in Codex when signed in with ChatGPT, but not with API-key authentication.
- The compatibility split is handled by documenting `gpt-5.4` as the fallback for accounts or execution paths where `gpt-5.5` is unavailable.

### Phase 1: high-value role upgrade

Use `gpt-5.5` in `ccc-config.toml` for the high-value frontier roles first:

- `orchestrator` / `captain`: replace `gpt-5.4` with `gpt-5.5`
- `way` / `tactician`: replace `gpt-5.4` with `gpt-5.5`
- `code specialist` / `raider`: replace `gpt-5.3-codex` or `gpt-5.4` with `gpt-5.5`
- `verifier` / `arbiter`: replace `gpt-5.4` with `gpt-5.5`

Keep these roles on lower-cost defaults:

- `explorer` / `scout`: keep `gpt-5.4-mini`
- `documenter` / `scribe`: keep `gpt-5.4-mini`
- `companion_reader`: keep `gpt-5.4-mini`
- `companion_operator`: keep `gpt-5.4-mini`

Reasoning:

- OpenAI's Codex docs position `gpt-5.5` as the strongest model for complex coding, computer use, knowledge work, and research workflows.
- Those strengths map directly to captain decisions, bounded planning, difficult mutation work, and acceptance review.
- OpenAI's docs still recommend `gpt-5.4-mini` for lighter coding tasks or subagents, which fits CCC's scout/scribe/companion roles better than a blanket frontier-model upgrade.

### Phase 2: fallback while availability finishes rolling out

Keep the checked-in high-value defaults on `gpt-5.5`, but document `gpt-5.4` as the fallback for operators whose current account or execution path cannot launch `gpt-5.5`.

The high-value fallback set is:

- `captain` -> `gpt-5.4`
- `tactician` -> `gpt-5.4`
- `raider` -> `gpt-5.4`
- `arbiter` -> `gpt-5.4`

The default hold set should remain:

- `scout` -> `gpt-5.4-mini`
- `scribe` -> `gpt-5.4-mini`
- `companion_reader` -> `gpt-5.4-mini`
- `companion_operator` -> `gpt-5.4-mini`

Unless OpenAI publishes a `gpt-5.5-mini` tier and explicitly recommends it for lighter subagent work, CCC should not replace the mini roles with the full frontier model by default.

### Phase 3: validation before publishing refreshed release surfaces

Before publishing refreshed release surfaces, CCC should validate:

- ChatGPT-authenticated Codex subagents can consistently launch with `gpt-5.5`
- host-subagent execution reports usable observed-model evidence for the new model string
- long-running captain and raider tasks remain stable under the new default
- companion-role latency and cost still justify staying on `gpt-5.4-mini`
- README tables, release notes, generated custom-agent templates, and config backfill tests are updated together

### Explicit non-goal

Do not treat `gpt-5.5` as a blanket replacement for every CCC role.

The current CCC structure intentionally separates:

- high-value frontier reasoning roles
- low-cost read-heavy subagent roles
- low-cost narrow operator roles

That separation should remain intact unless official OpenAI guidance changes enough to justify a new mini-tier migration or a broader cost/latency rebalance.

## Ownership and runtime state

CCC is the owner of normalized runtime workflow state. That ownership includes:

- run state
- task-card state
- handoff state
- current stage
- active role and active agent tracking
- child-agent visibility and child-agent status
- current task visibility
- latest handoff visibility
- latest verification state
- latest normalized failure state

Codex CLI remains the execution engine and the source of raw execution evidence. That ownership includes the actual command execution, raw structured event stream, raw terminal output, execution exit status, and documented configuration or profile behavior.

The boundary is intentionally strict:

- CCC owns normalized workflow, task, handoff, and visibility state.
- Codex CLI owns execution behavior and raw execution or event output.
- CCC may derive its normalized state from documented Codex CLI surfaces, but that does not transfer workflow ownership back to Codex CLI.
- If Codex CLI output changes or becomes incompatible with CCC's documented assumptions, CCC must report a compatibility-stage failure instead of silently inventing state.

One run may correlate to one or more Codex CLI execution attempts. Correlation is allowed through documented identifiers such as `thread_id`, but the run remains a CCC concept rather than a Codex CLI concept.

CCC may materialize `visibility.json` as a derived projection for operator-facing views, but it is not required as canonical persisted state. The canonical state remains the normalized run, task-card, handoff, and visibility model owned by CCC.

## Minimum visibility UX

The MVP must provide a minimum visibility surface that makes the current state of work readable without opening raw event logs first. That surface can be implemented in a terminal view, generated file, or lightweight UI, but the information contract is fixed.

At minimum, the operator must be able to see:

- **Active run**: run identifier, goal summary, and overall run status.
- **Current stage**: the current workflow stage for the run.
- **Active agent**: the role, agent identity, and active execution correlation for the agent currently holding the task.
- **Child agents and their status**: each visible child agent, its parent relationship, and whether it is queued, running, completed, failed, or cancelled.
- **Current task card**: the active task-card title or summary, owner, task status, and expected outcome.
- **Latest handoff**: the most recent handoff summary, including from-role, to-role, and current handoff outcome.
- **Latest verification or failure state**: the latest verification result, or the latest normalized failure record when work is blocked or has failed.

This minimum visibility UX is intentionally narrow. It is not a requirement for a full timeline view, transcript browser, or analytics layer. The purpose is to make the live workflow state legible and reviewable while staying thin over Codex CLI.

## Failure taxonomy

Failure reporting must keep **stage** and **reason** as separate concepts.

- **Stage** answers where in the workflow the failure was identified.
- **Reason** answers the normalized cause category for that failure.

### Canonical stage family

The MVP stage family is:

- `planning`
- `handoff`
- `execution`
- `verification`
- `compatibility`

These values should be treated as the canonical stage set for MVP reporting.

### Canonical reason baseline

The MVP reason baseline is:

- `surface_mismatch`
- `invalid_output`
- `timeout`
- `cancelled`
- `blocked_dependency`
- `verification_failed`
- `unknown`

These reasons provide a normalized starting vocabulary. They can be extended later if needed, but the MVP should use the baseline set consistently before introducing new reason values.

### Usage rules

- `stage=compatibility` should be used when CCC cannot safely rely on the expected documented Codex CLI surface.
- `reason=surface_mismatch` should be used when the documented surface is missing, changed, or no longer parseable in the expected way.
- `stage=verification` and `reason=verification_failed` should be used when execution finished but acceptance or validation did not pass.
- `reason=unknown` is the last-resort category when the workflow cannot classify the cause more precisely.

The normalized failure record should preserve both values together so operators can distinguish location in the workflow from the cause of failure.

## Codex CLI surface contract

CCC must treat Codex CLI as an external execution engine with a documented interface boundary. The contract below separates required foundational surfaces from optional conveniences.

### Foundational documented surfaces

The following are foundational for the MVP because they are documented and stable enough to build the orchestration contract around:

- `codex exec` as the primary non-interactive execution entry point.
- `--json` for structured output mode where documented.
- Documented flags that are explicitly supported by the Codex CLI docs.
- Documented JSONL event types emitted by Codex CLI in structured mode.
- `thread_id` correlation when present in documented output, so CCC can connect execution evidence to a run or task card.
- Documented config and profile usage, including selecting or applying supported profiles in documented ways.

CCC may depend on these surfaces for the MVP contract. If one of these documented surfaces changes in a breaking way, the correct CCC response is a compatibility-stage failure, not silent reinterpretation.

### Optional conveniences

The following may improve ergonomics, but they are not required to preserve the MVP contract:

- Wrapper scripts around `codex exec`.
- Local formatting or projection of structured JSONL into CCC-friendly views.
- Cached summaries, stitched transcripts, or operator-focused dashboards derived from raw output.
- Local aliases, launch helpers, or thin adapters that can be removed without changing the underlying contract.
- Best-effort local evidence probes that correlate documented `thread_id` values with local Codex state stores to enrich observed model, provider, or reasoning metadata, provided those probes remain optional, execution-source aware, and fail gracefully with normalized unavailable reasons rather than a single opaque failure bucket. When such probes are surfaced to operators, CCC should also persist enough metadata to show where the observed evidence came from and whether it is local best-effort evidence or stronger provider-confirmed proof.

Optional conveniences should remain replaceable. CCC should still have a coherent operating model without them.

### Explicitly non-foundational surfaces

The following must not be treated as foundational for the MVP:

- undocumented flags or undocumented commands
- undocumented event fields or undocumented event ordering assumptions
- internal process behavior that is not part of the public CLI contract
- internal Codex state databases or logs treated as required canonical proof rather than optional local evidence
- scraped UI output or prompt text used as if it were a stable API
- experimental surfaces that are not documented as supported

Using these surfaces may be acceptable for local experimentation, but they cannot define the canonical CCC contract.

## Initial architecture direction

Start with a documentation-first workflow model that can later be connected to lightweight orchestration logic. The first version should preserve clear role boundaries, explicit handoff points, and visible status for each step. Codex CLI remains the execution engine, while CCC owns normalized workflow and visibility state as a thin coordination layer around it.

The architecture should stay simple enough that each role's responsibility is obvious from the repo structure and task flow alone, and flexible enough to survive Codex CLI version drift without major rewrites. It should prefer the documented Codex CLI surface contract defined above and should treat anything undocumented as non-foundational.

## MVP boundaries

The MVP should cover:

- canonical role and workflow definitions
- milestone-driven workflow
- explicit handoff order
- verification checkpoints
- stable-interface-first compatibility assumptions
- normalized stage-and-reason failure reporting
- minimum visibility UX
- a clear readiness path for future implementation

The MVP should not try to solve every workflow problem at once. It should focus on making Codex CLI work more organized and easier to verify, while avoiding assumptions that depend on unstable CLI internals.

## Risks

- Role overlap could blur ownership.
- The workflow could become too abstract if the docs are too broad.
- Overdesign could slow down the first usable version.
- Verification may be too loose if exit criteria are not specific enough.
- Upstream Codex CLI changes could break the workflow if the layer couples too closely to transient behavior.
- Fallback behavior may be underspecified if compatibility expectations are not documented clearly.
- Visibility gaps could make it hard to understand active work or delegated child-agent state.

## Remaining implementation questions

- What is the smallest persisted runtime shape that can represent run, task-card, handoff, and visibility state without overfitting the first implementation?
- How much visibility state should be materialized directly versus derived on read from normalized run data and raw Codex CLI evidence?
