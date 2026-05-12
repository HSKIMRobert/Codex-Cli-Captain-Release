# 0.0.10 Pre-Release Plan

`0.0.10-pre` should make the operator-visible LongWay and subagent lifecycle story easier to read after the `0.0.9-pre` behavior observation.

## Release Goal

Clarify that LongWay rows are completion and accountability units, but they can also carry explicit owner metadata. For broad work, Way should decide which subagent/role owns each row when it drafts the LongWay, so the operator can see planned assignments without losing the checklist's task-level shape. The rows remain useful for progress tracking, while activity, status, and fan-in artifacts continue to hold the lifecycle truth.

This is a LongWay design direction for `0.0.10-pre`, not control over host Codex `/agent` row labels. CCC can document and render the planned assignment rows it owns, but specialist-to-specialist handoff still does not exist: captain routes, specialists return to captain, and Way only proposes the assignment plan.

The 0.0.9 dynamic context and context-pressure restart-resume, or slash `/new`, mitigation is already implemented and verified in the current 0.0.10 surfaces. The supporting evidence lives in `rust/ccc-mcp/src/long_session.rs`, `status_payload.rs`, `status_compact.rs`, `status_render.rs`, `run_locator.rs`, `main.rs`, and `worker_lifecycle.rs`, with coverage in `rust/ccc-mcp/src/main_tests.rs` through `ccc_status_recommends_long_session_rollover_when_context_pressure_is_high`, `ccc_status_recommends_new_rollover_when_host_subagent_pressure_is_high`, `ccc_status_tool_reads_persisted_run_truth_from_workspace_run_id`, `run_locator_global_fallback_resolves_run_id_from_different_cwd`, and `ccc_orchestrate_advance_with_resolve_outcome_closes_run`.

The graph work plan is CCC-native Rust work inside the user's CCC repo. `0.0.10-pre` treats that Rust path as a core release target because it can reduce token use and help captain understand repository structure before routing or review. Final usability should surface from CCC itself, not become a permanent second MCP server. The upstream `code-review-graph` repo stays a parity/spec reference for behavior and shape, but this train does not imply a line-by-line Python port.

Operators should see the graph/review capability as explicit `ccc graph` / `ccc_code_graph` review-context queries with compact/status visibility into the returned context. The temporary bridge language only covers migration or development aid while the Rust implementation catches up; deeper automatic review/fan-in consumption stays out of the core path unless separately implemented.

`0.0.10-pre` also includes an opt-in CCC Memory Spec for durable workspace notes. The implemented surface is intentionally small: `.ccc/memory.json`, `ccc memory` status/preview/write/off actions, and a compact `ccc status` memory summary. Memory can store only user preferences, repeated rules, and verified project facts. LongWay state, run state, latest work result, and inference-only observations are explicitly forbidden as memory truth. Automatic memory extraction is not shipped.

Operator-requested correction plan for the remaining `0.0.10-pre` docs pass:

1. LongWay display should nest subagent lane lines under each task row, so each row reads as the task unit first and then shows the planned lane details underneath it.
2. Graph reference messaging should stay concise and natural, using Captain/Way language such as `graph reference at {scope/query} found {issue}; I will plan the work` instead of emitting a large trace.
3. Ran-style CLI discipline should stay visible to operators through local `ccc ...` and shell commands. MCP calls remain internal orchestration details and should not be mirrored as operator output.
4. Captain Instruction Memory should exist outside hardcoded prompts as a captain-wide instruction source for all users, including recurring clarification obligations throughout a task and not only on the first command.
5. The docs should include an implementation checklist and acceptance criteria for the correction work so the operator can verify what still needs to land.

Example LongWay shape for the operator-facing display:

```text
LongWay
[>] Fix release smoke failure
- scout-a      completed   mapped release smoke failure
- raider-a     running     patching installer path handling
- arbiter-a    pending     waits for raider-a fan-in
[] Fix release packaging note
- scout-b      pending     mapped release smoke failure
- raider-b     pending     patching installer path handling
- arbiter-b    pending     waits for raider-b fan-in
```

Follow-up implementation checklist:

- [x] Keep LongWay rows as task units with nested lane lines, not as a flat roster of subagents.
- [x] Keep graph-reference text short, operator-readable, and rooted in Captain/Way planning language, e.g. `graph reference at {scope/query} found {issue}; I will plan the work`.
- [x] Keep operator-visible lifecycle/evidence on local `ccc`/shell surfaces; treat MCP orchestration as internal control-plane detail.
- [x] Keep Captain Instruction Memory separate from hardcoded prompts at the memory/status/schema layer and allow repeated clarification prompts to be stored as `captain_instruction` entries.
- [x] Keep the plan and release note aligned so the correction direction is documented in both work and publish surfaces.

Runtime-fix smoke candidate:

- `ccc-0.0.10-pre-darwin-arm64-runtime-fix.tar.gz`
- `sha256:0573580e12bfddaf6943bedfe7b6608f4284126c9a0d8014004c1c589ae28657`
- The candidate is additive so the original `v0.0.10-pre` macOS arm64 asset remains available as the rollback path.

Routing note for this train: the prior `/goals` investigation was read-only design assessment work, so it should not have been routed to `ccc_raider`. Captain/Way routing should classify read-only investigation or design assessment as `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician` depending on shape, and reserve `ccc_raider` for bounded code or config mutation.

Routing note for 0.0.10 docs/planning work: documentation and release-plan updates should route to `ccc_scribe`. Read-only investigation should route to `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician`, and mutation should route to `ccc_raider` or the operator-side mutation path. If captain auto-assigns docs/planning work to a non-docs role, treat that as routing drift and correct the plan rather than normalizing it.

Assignment-quality guard note: captain/Way can still choose the wrong specialist, but that should be visible as routing drift rather than normalized by fan-in. Status now carries an `assignment_quality` check for the current task card and renders an assignment warning when the inferred family does not match the assigned role/agent. The guard is intentionally advisory and bounded: docs/planning expects `ccc_scribe`; read-only investigation or design assessment expects `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician`; bounded code/config mutation expects `ccc_raider`; review/acceptance expects `ccc_arbiter`; narrow operator-side git/GitHub mutation expects `companion_operator`.

Process note: a captain direct-work drift happened in the prior work stream, where captain edited specialist-owned material instead of keeping to route/fan-in/review. 0.0.10 should prevent that pattern by requiring stricter routing and fan-in discipline, with captain direct work limited to explicit fallback, trivial work, or recorded degradation.

Codex `/goal` note: public open-source evidence now confirms persisted `/goal` workflows in Codex CLI 0.128.0, but CCC should not claim to intercept Codex TUI slash commands. CCC goal-like behavior stays CCC-native through `$cap` / `ccc start` -> Way -> LongWay/task cards -> subagent fan-in -> captain review/status/activity. Use `/goal` only when referring to the operator's wording or the upstream singular command.

Priority 1 features for the Rust-native graph path:

1. Persistent graph store/schema.
2. Repo root detection, ignore filtering, and incremental update.
3. Diff-to-impact and blast-radius analysis.
4. Callers, callees, imports, tests, and file summary query.
5. Minimal review context plus risk scoring.
6. CCC status, review, and fan-in integration.

Second-wave features after the core path is usable from CCC:

1. Flow tracing.
2. Criticality scoring.
3. Community and architecture overview.
4. Full-text search.
5. Multi-repo search.

The quality bar for acceptance is not just correctness. The Rust-native implementation should be modular, with clear and useful comments where the code would otherwise be hard to follow. The plan should also keep the UX practical: graph answers must be available from CCC surfaces without requiring operators to adopt a permanent second MCP.

If a temporary upstream bridge is kept around, pin versions, keep it opt in, and use health checks plus integration smoke tests to verify discovery, invocation, and failure handling. The smoke coverage should prove the bridge can be discovered, invoked, and fail safely rather than silently drifting.

## Caveats

- `code-review-graph` integration is planned and staged as a 0.0.10 target; do not mark it complete until the CCC-native Rust graph path is implemented and validated.
- Target UX should be CCC-native Rust graph capability, with no final requirement for a second MCP server.
- A temporary upstream bridge is acceptable only as a parity/spec aid or migration path while Rust-native coverage is incomplete.
- If the bridge exists, keep it operator opt in, version pinned, and covered by health checks plus integration smoke tests for present, absent, and failure cases.
- Risks to call out explicitly: upstream beta/API churn, repo indexing and cache/watch side effects, bridge install steps mutating MCP/tool config, and optional embedding-provider data egress.
- The MIT license permits external use, but any copied or vendored code must preserve license and copyright notices.
- The Rust-native path should still land in stages: graph store/schema first, then blast-radius context, then CCC status/review/fan-in integration, then broader review workflow features.
- Compact per-row lifecycle sync is now shipped as a narrow LongWay phase-row projection. Task-card lifecycle, review lifecycle, parallel lane lifecycle, and fan-in records remain the source of truth; the row-level `lifecycle_sync` field only summarizes that truth for status/checklist readability.
- CCC memory is opt in and workspace scoped. It is not a run-history summarizer, not an inference cache, and not an automatic writer from LongWay/status/activity/fan-in output.

Keep the `$cap` docs and checklist output aligned with that expectation so the operator can tell which surface answers which question.

## Docs Gap Notes

- The observed `Verify CCC 0.0.9 pre release` run used three host subagents, but the visible checklist showed one row because the checklist projected task-card completion, not every lifecycle event.
- `0.0.9-pre` tracked the split between MCP/tool-call renderings and the intended operator-facing compact CLI projection. For 0.0.10, CCC-owned CLI and `$cap` guidance keep ordinary lifecycle boundaries on compact checklist/status surfaces, while any host UI tool-call transcript remains outside CCC's direct rendering control.
- The docs should state that `ccc checklist --text` renders LongWay completion rows with explicit owner/role metadata when available, while `ccc activity`, `ccc status`, fan-in artifacts, and subagent lifecycle records carry the per-subagent details.
- Stalled, failed, and reassigned review visibility should stay on the lifecycle/status side, not by duplicating checklist rows.
- `$cap` guidance should set operator expectation up front: checklist rows track decomposed progress items and their planned owner/role, while lifecycle and review history are separate surfaces.
- Broad requests should be planned as multiple task rows up front, with clearer row names that read like task-card items and a visible owner/role per row rather than a single umbrella label.
- Planned rows should carry compact planning detail when available: owner/agent, planned role, lifecycle, scope, acceptance, routing summary, and evidence count.
- This matters because captain coordination is only trustworthy when specialist-owned edits stay specialist-owned; review and fan-in should validate the work, not blur execution ownership.
- In the prior docs-only CCC run, final closeout and lookup through the compact CLI plus MCP `status`/`orchestrate` failed with `No such file or directory` for a known `run_id`, while a later fresh `ccc_start`/`status` resolved run files under `/Users/kwkim-hoir/.config/ccc/workspaces/...`; treat that as historical regression evidence for run persistence, lookup, and finalization robustness. The current implementation already addresses the fallback and compact/status truth surfaces through `rust/ccc-mcp/src/run_locator.rs`, `main.rs`, `status_payload.rs`, `status_compact.rs`, `status_render.rs`, and `worker_lifecycle.rs`, with verification in `ccc_status_tool_reads_persisted_run_truth_from_workspace_run_id`, `run_locator_global_fallback_resolves_run_id_from_different_cwd`, and `ccc_orchestrate_advance_with_resolve_outcome_closes_run`.
- `code-review-graph` is the upstream reference for functions, classes, imports, call sites, blast radius, and incremental review context; CCC should treat it as a parity/spec source while building a native Rust graph surface, not as the final integration shape.
- Cursor-style harness wording should describe CCC's own orchestration harness around LongWay/status/activity, not an external co-execution model and not host `/agent` row control.
- Example CCC LongWay/status graph adaptation:

  ```text
  CCC LongWay/status graph adaptation
  upstream `code-review-graph`: structural source-code graph for review context, blast radius, callers/callees, and related tests
  CCC LongWay/status/activity: adapted lifecycle projection for operator-visible planning and fan-in
  row: ccc_scribe -> row: ccc_companion_operator -> review gate -> row: ccc_arbiter
  stalled/reassigned/failed states show on the graph nodes and lifecycle surfaces
  captain routing decides the next hop; the checklist keeps the planned owner/role rows
  ```
- Example LongWay shape:

  ```text
  LongWay
  [>] ccc_scribe                Update 0.0.10 pre release docs
  [ ] ccc_companion_operator    Verify install/test surface
  [ ] ccc_arbiter               Review and accept release readiness
  ```

- Progress rendering example:

  ```text
  Before captain update
  LongWay
  [>] ccc_scribe                Update 0.0.10 pre release docs
  [ ] ccc_companion_operator    Verify install/test surface
  [ ] ccc_arbiter               Review and accept release readiness

  After captain update
  LongWay
  [~] ccc_scribe                Update 0.0.10 pre release docs
  [>] ccc_companion_operator    Verify install/test surface
  [ ] ccc_arbiter               Review and accept release readiness
  ```

- Row status can progress through spawned, running, completed, merged, stalled, and reassigned without turning the checklist into a host-subagent roster. The assignment shown in the row is the planned owner for that unit of work, not a claim that CCC controls host `/agent` row labels.
- Captain updates can reflect planned-owner changes in the row shape for planning purposes, while stalled, reassigned, failed, and merged truth stays on the lifecycle/status/activity/fan-in surfaces and is summarized onto rows through the compact `lifecycle_sync` projection when a row has a task-card id.
- The row example above is about task-card decomposition with explicit owner metadata, not host-subagent count. Subagent execution can still fan out underneath those rows, but that activity belongs in the lifecycle surfaces.
- Compact per-row lifecycle view now uses a narrow activity/status projection without changing the checklist row model.
- Graph availability and query output should include a compact evidence note, for example top indexed directories on status or risk/count/path context on `ccc graph`, so operator-visible routing is not reduced to only `CodeGraph: N files`.
- Captain remains a configured orchestrator role, but CCC should not sync or advertise it as a normal spawnable custom agent. Spawnable custom-agent sync starts at Way and specialist roles.
- Clarification policy is bounded: ask 1-3 high-signal questions for broad, risky, ambiguous, or irreversible work, and proceed with explicit assumptions for narrow work.
- Keep the note brief and aligned with the existing pre-release tone.

## Work Item

- Keep this plan aligned with implementation status as 0.0.10 work lands; checked items are implemented or already covered by existing runtime surfaces, unchecked items are future work, and deferred items are explicitly out of this train.
- Update the 0.0.10 plan so the requested Rust-native graph work is described as a core 0.0.10 implementation target, not a graph-only release note.
- Update docs and operator text so LongWay rows carry explicit owner/role fields, Way's assignment plan is visible, and subagent lifecycle truth stays in activity/status/fan-in surfaces.
- Improve checklist row naming and granularity so broad requests are split into multiple meaningful rows instead of one generic row.
- Add expectation-setting language to `$cap` guidance and checklist output so users do not read checklist row count as host-subagent count.
- Require docs/planning work to route to `ccc_scribe`; reserve read-only investigation for `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician`, and reserve mutation for `ccc_raider` or the operator-side mutation path.
- Treat automatic assignment of docs/planning work to a non-docs role as routing drift that should be corrected in the plan language.
- Surface assignment-quality routing drift in status/fan-in-adjacent visibility where feasible instead of silently accepting the wrong specialist family.
- Close the CCC-owned part of the 0.0.9 output-mode issue: separate MCP/tool-call instrumentation from the operator-facing compact CLI projection, and make ordinary `$cap` lifecycle boundaries consistently use checklist/status text rather than raw status JSON.
- Add 0.0.10 language for Way assignment planning, row owner metadata, checklist/status rendering, and lifecycle sync.
- Add routing guidance that read-only investigation or design assessment stays with `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician`, while `ccc_raider` is reserved for bounded mutation.
- Add a concise note that Codex CLI 0.128.0 has real persisted `/goal` workflows, but CCC remains CCC-native and does not claim TUI slash-command interception.
- Reframe `code-review-graph` as a CCC-native Rust target with an upstream parity/spec reference and only temporary bridge language where needed.
- Make the Rust graph roadmap explicit with priority 1 and second-wave feature ordering, including persistent store/schema, root detection, ignore filtering, incremental updates, blast-radius analysis, callers/callees/imports/tests/file summaries, review context, risk scoring, and CCC status/review/fan-in integration.
- Add the quality expectations that the Rust implementation should be modular and use clear, useful comments where they add value.
- Improve stalled/reassigned review visibility in the lifecycle surfaces that already own that truth.
- Track and fix the observed compact CLI plus MCP closeout/lookup failure as a run persistence, lookup, and finalization robustness gap.
- Keep compact per-row lifecycle projection narrow: it is shipped as row-local status metadata, not checklist inflation or a replacement for lifecycle/fan-in artifacts.
- Add the opt-in CCC Memory Spec for `0.0.10`: workspace-scoped small file only, allowed kinds limited to user preferences/repeated rules/verified project facts, preview/diff before write, stale memory detection, verified facts versus inference distinction, and memory off/status CLI/status visibility.
- Do not overclaim memory automation: automatic memory writes from task cards, fan-in, latest work result, LongWay, status, or activity stay deferred.
- Do not present goal-mode behavior or anything that depends on goal-mode behavior as shipped in this train.

## Status Snapshot

Completed by existing docs and runtime surfaces:

- [x] LongWay rows already render with optional `owner_agent` metadata in status/checklist text.
- [x] `ccc status`, `ccc activity`, and fan-in payloads already separate checklist progress from lifecycle truth.
- [x] The 0.0.9 dynamic context/context-pressure restart-resume and slash `/new` mitigation is already implemented and verified in the long-session/status/run-locator surfaces and tests listed above.
- [x] Review lifecycle state already stays distinct from subagent lifecycle state.
- [x] Routing guidance already keeps read-only investigation on `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician`, and reserves `ccc_raider` for bounded mutation.
- [x] The entry policy already keeps Way as the bounded planning step before specialist execution.
- [x] Captain direct-work drift is now documented as a routing/fan-in discipline problem, with captain direct work limited to explicit fallback, trivial work, or recorded degradation.
- [x] Docs/planning routing is now documented: `ccc_scribe` for docs and release-plan work, `ccc_scout` / `ccc_companion_reader` / `ccc_tactician` for read-only investigation, and `ccc_raider` / operator-side mutation for bounded mutation.
- [x] The acceptance language now blocks completion if captain performs specialist-owned edits without a recorded fallback or degradation reason.
- [x] Assignment-quality routing drift is visible in status through a focused `assignment_quality` payload and warning line when the assigned role/agent mismatches the inferred specialist family.
- [x] Captain is documented as the orchestrator/control-plane actor, not a normal spawnable specialist.
- [x] Bounded clarification/deep-interview policy is documented: 1-3 high-signal questions for broad/risky/ambiguous/irreversible work, explicit assumptions for narrow work.

Current implementation status:

- [x] The Rust-native graph core plus CLI/MCP/status wiring path and its upstream `code-review-graph` parity/spec framing.
- [x] Graph CLI/MCP/status text now includes compact graph evidence notes alongside file-count availability.
- [x] A consistent CCC-owned operator-facing projection that separates compact CLI/checklist/status output from MCP/tool-call instrumentation during ordinary `$cap` runs. Host UI tool-call transcripts can still be visible outside CCC's direct rendering control.
- [x] Compact per-row lifecycle projection is shipped in this train as `longway.phase_rows[].lifecycle_sync`, with status/checklist text showing row-local lifecycle status when available.
- [x] The lifecycle sync stays narrower than the current checklist/status/activity split by projecting task-card lifecycle truth instead of duplicating or replacing it.
- [x] Planned LongWay rows render compact planning detail, including scope, acceptance, routing summary, and evidence count when available.
- [x] Focused opt-in CCC Memory Spec is documented in [`CCC_MEMORY_SPEC.md`](./CCC_MEMORY_SPEC.md).

Implementation checklist:

- [x] Implement the CCC-native Rust graph system as a 0.0.10 core feature so captain can use structural repository context while spending fewer tokens. First reusable Rust store/index/query engine slice plus CLI/MCP/status wiring is implemented.
- [x] Build the persistent graph store/schema.
- [x] Add repo root detection, ignore filtering, and incremental graph updates.
- [x] Add diff-to-impact and blast-radius analysis.
- [x] Add callers, callees, imports, tests, and file summary queries.
- [x] Add minimal review context generation and risk scoring.
- [x] Connect graph results into CCC CLI, MCP, and quiet status/compact visibility surfaces.
- [x] Add visible graph/planning evidence notes to graph/status text so planning/routing context is auditable without expanding the full graph payload.
- [x] Decide whether deeper automatic review/fan-in graph consumption is needed beyond explicit `ccc graph` / `ccc_code_graph` review-context queries. Decision for 0.0.10: keep automatic consumption out of the core path; expose review context explicitly through CLI/MCP/status first.
- [x] Implement the second-wave graph features after the core path is usable: flow tracing, criticality scoring, community/architecture overview, full-text search, and multi-repo search.
- [x] Implement minimal opt-in workspace memory surface: `.ccc/memory.json`, `ccc memory` status/preview/write/off actions, preview diff summary, timestamp stale-write guard, 30-day stale status, allowed kind validation, forbidden source rejection, and compact/status memory visibility.
- [x] Make ordinary `$cap` lifecycle output consistently distinguish the compact operator projection from MCP/tool-call instrumentation in CCC-owned docs/skill guidance and CLI surfaces.
- [x] Fix the compact CLI plus MCP run lookup/finalization path so known `run_id` closeout and status/orchestrate calls can fall back to central workspace storage when the current cwd hint misses. This is now implemented and verified; the historical `No such file or directory` note above stays as regression evidence only.
- [x] Keep captain/orchestrator separate from spawnable custom-agent sync; `ccc-captain.toml` is treated as stale managed output instead of a current spawnable specialist.

## Verification Note

Final captain-run verification for the run-id fallback and status truth work was completed with targeted Rust tests:

- `cargo test ccc_status_recommends_long_session_rollover_when_context_pressure_is_high` passed: 1 passed, 0 failed.
- `cargo test ccc_status_recommends_new_rollover_when_host_subagent_pressure_is_high` passed: 1 passed, 0 failed.
- `cargo test ccc_status_tool_reads_persisted_run_truth_from_workspace_run_id` passed: 1 passed, 0 failed.
- `cargo test run_locator_global_fallback_resolves_run_id_from_different_cwd` passed: 1 passed, 0 failed.
- `cargo test ccc_orchestrate_advance_with_resolve_outcome_closes_run` passed: 1 passed, 0 failed.
- `cargo test ccc_status_projects_compact_lifecycle_sync_onto_longway_rows` passed: 1 passed, 0 failed.
- `cargo test code_graph_second_wave_queries_cover_flow_criticality_architecture_and_search` passed: 1 passed, 0 failed.
- `cargo test ccc_memory_previews_filters_and_requires_stale_write_guard` passed: 1 passed, 0 failed.
- `cargo test ccc_memory_off_status_and_ccc_status_surface_are_opt_in` passed: 1 passed, 0 failed.
- `cargo test assignment_quality_flags_docs_routed_to_raider_as_drift` passed: 1 passed, 0 failed.
- `cargo test assignment_quality_accepts_expected_specialist_families` passed: 1 passed, 0 failed.
- `cargo test ccc_status_text_surfaces_assignment_quality_warning` passed: 1 passed, 0 failed.
- `cargo test` in `rust/ccc-mcp` passed: 206 passed, 0 failed.

The first combined cargo invocation failed because Cargo accepts only one test-name filter before `--`; that was a command-shape issue, not a product or test failure, and the targeted tests were rerun individually. This command-shape issue also occurred during validation of the two new focused tests, and both were rerun individually with passing results.

Explicitly excluded or newly completed for this train:

- [-] Goal-mode behavior and any work that depends on it remains excluded.
- [x] Compact per-row lifecycle sync beyond current status/activity/fan-in lifecycle truth is implemented as a compact row projection backed by task-card lifecycle records.
- [x] Second-wave graph features are implemented through `ccc graph` / `ccc_code_graph` query modes: `flow_trace`, `criticality`, `architecture_overview` / `communities`, `full_text_search` / `search`, and `multi_repo_search`.
- [x] Minimal opt-in CCC memory is implemented through explicit `ccc memory` CLI actions and status projection only.
- [-] Automatic memory writes, cross-workspace memory, semantic search/embeddings, external memory service integration, and automatic prompt injection are deferred.

## Follow-Up Work Completed In This Slice

The operator decision for `0.0.10-pre` was to proceed with five follow-up work items: the existing four LongWay/graph-assisted planning items plus a CCC_MEMORY basic-rules/workspace-memory item. This slice completes the four LongWay/graph-assisted implementation items and keeps CCC_MEMORY as the already-implemented explicit workspace memory surface:

1. [x] Add a Way plan row schema. Planned rows now carry `title`, `planned_role`, `planned_agent_id`, `scope`, `acceptance`, `status`, optional `task_card_id`, routing evidence fields, and materialization timestamps.
2. [x] Update `ccc start` / `ccc run` so initial LongWay creation can persist multiple planned rows while preserving the initial executable task-card behavior.
3. [x] Use captain just-in-time materialization for planned rows. `ccc orchestrate` advance materializes only the next planned row into a task card and remains idempotent.
4. [x] Keep graph-assisted routing as a cautious evidence phase. Bounded routing evidence can be persisted for planned rows/materialized task cards, while raw graph dumps are filtered from persisted/status surfaces and graph evidence is not automatically injected into prompts.
5. [x] Keep the CCC_MEMORY basic-rules/workspace-memory item in scope as the explicit opt-in `.ccc/memory.json` surface. CCC_MEMORY is similar in purpose to persistent workspace rules/preferences, but it is not an `AGENTS.md` replacement: `AGENTS.md` remains the repo-level instruction source, while CCC_MEMORY is a managed store for opt-in workspace facts, preferences, and repeated rules.

## Acceptance

- The docs explicitly say LongWay rows are decomposed task-card completion items with optional owner/role metadata, not one row per spawned host subagent.
- The docs show nested lane lines under each LongWay task row instead of a flat subagent roster.
- The docs show a concrete multi-row LongWay example for broad work with planned owners.
- The docs say read-only investigation or design assessment is not a `ccc_raider` case; routing should use `ccc_scout`, `ccc_companion_reader`, or `ccc_tactician` as appropriate.
- The docs say docs/planning updates route to `ccc_scribe`, and any automatic assignment of that work to a non-docs role is routing drift to improve.
- Status warns when the current task card assignment appears to mismatch the expected specialist family, so wrong assignment is visible before or during fan-in/status review where feasible.
- The docs also mention `code-review-graph` and Cursor-style harness wording, but they must distinguish the upstream repo's code-review context graph from CCC's own LongWay/status/activity adaptation and its Rust-native target direction.
- The docs explicitly state that the Rust-native graph work lives in the user's CCC repo and that final usability should come from CCC, not a permanent second MCP.
- The docs list the priority 1 and second-wave feature sets in the requested order.
- The CCC-native Rust graph system is treated as a 0.0.10 core implementation target and is not moved to deferred scope.
- Graph implementation acceptance includes persistent storage, incremental indexing, impact/blast-radius queries, review context/risk scoring, explicit CLI/MCP query access, and compact/status visibility.
- The docs state the modularization and clear-comment quality bar as an acceptance expectation.
- The docs note that Codex CLI 0.128.0 includes persisted `/goal` workflows, while CCC keeps goal-like behavior CCC-native and does not claim to intercept Codex TUI slash commands.
- The docs keep graph reference messaging concise and operator-natural, rather than reproducing a long trace or raw graph dump.
- The docs keep operator-visible lifecycle and evidence on local `ccc ...` / shell command surfaces and treat MCP calls as internal orchestration.
- The docs define Captain Instruction Memory as a non-hardcoded, captain-wide instruction source that applies throughout the task, including recurring clarification obligations.
- The docs explicitly state that the 0.0.9 dynamic context/context-pressure restart-resume or slash `/new` mitigation is already implemented and verified, and they cite the current status, long-session, run-locator, worker-lifecycle, and render surfaces instead of leaving a contradictory active-gap note.
- The docs direct subagent lifecycle visibility to activity, status, and fan-in artifacts.
- The docs mention stalled and reassigned review visibility as a lifecycle concern rather than a checklist concern.
- The `$cap` docs and checklist output set operator expectation before the run starts or fans in, including that broad requests should split into multiple task rows with assigned owners.
- The docs do not mark work complete if captain took specialist-owned edits without a recorded fallback or degradation reason.
- The docs close the CCC-owned 0.0.9 output-mode boundary by requiring ordinary `$cap` lifecycle output to distinguish the compact operator projection from MCP/tool-call instrumentation.
- Any per-row lifecycle visibility improvement is described as the shipped compact `lifecycle_sync` projection, not checklist inflation or lifecycle-truth replacement.
- The docs explicitly say the graph/harness view is CCC's own LongWay/status/activity projection and orchestration harness, while the upstream `code-review-graph` repo is a parity/spec reference for CCC's Rust-native graph path, not host Codex `/agent` row control.
- The compact per-row lifecycle projection is no longer deferred; docs record it as implemented with test evidence.
- The implementation covers the prior compact CLI plus MCP `status`/`orchestrate` closeout lookup failure with a central workspace run-id fallback and tests.
- The memory docs and implementation keep memory opt in, workspace scoped, previewed before writes, stale-guarded, and limited to user preferences, repeated rules, and verified project facts.
- The memory docs and implementation explicitly forbid treating LongWay/run state/latest work result as durable memory truth and do not claim automatic memory writes.
