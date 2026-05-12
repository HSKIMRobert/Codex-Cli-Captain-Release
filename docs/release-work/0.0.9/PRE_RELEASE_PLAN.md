# 0.0.9 Pre-Release Plan

`0.0.9-pre` should keep the MCP control-plane surfaces explicit while preserving the ordinary operator-facing `$cap` CLI shape.

## Release Goal

Clarify that CCC uses MCP control-plane entry points for `status`, `orchestrate`, `subagent_update`, and `recommend_entry`, but the operator experience should still read as compact local `ccc` output: LongWay/checklist, compact status, and the next `run_id` gauge. Do not describe CCC as fully MCP-first.

Clarify that Codex CLI `/plan` and `/goal` are TUI slash commands, not CCC-native subagent commands. CCC should express `/plan`-like work through captain-owned control-plane state: use `Way`/`task_kind: "way"` for new planning, and `ccc_orchestrate` with `replan_prompt` for existing runs. CCC should express `/goal`-like work through run and task-card `goal`, `intent`, `scope`, and `acceptance` fields. If statusline, title, autoreview, or keymap behavior is mentioned, treat it as wrapper or documentation surface area rather than shipped native integration.

Make `scribe`, `companion_reader`, and `companion_operator` default reasoning/variant `medium`, and update the README-facing role/default wording so the low-cost docs and companion roles match that operator-default profile.

Before captain or Way hands work to a specialist, generate task-specific expertise framing for the prompt so the subagent sees its role, stance, and thinking mode before it starts.

Keep the doc-release framing aligned with the Korean 0.0.9-pre findings: doc/translation requests should route to `scribe`, release-planning LongWay output should expand into multiple checklist lines, checklist state should stay synchronized with lifecycle/status, and optional review lanes should use bounded status polling, visible follow-up, and reclaim/retry/reassign instead of long silent waits.

The newly observed routing-drift defect also needs a hard rule: when a matching CCC specialist exists, captain must not directly perform implementation, docs, or review work under `$cap`; any drift must be recorded as a captain intervention, routed to the correct specialist for adoption or repair, and reviewed before merge.

## Docs Gap Notes

- Tool-call renderings such as `Called ccc.ccc_orchestrate({json})` are host-side instrumentation, not the intended operator surface.
- The docs should make the CLI-style operator projection explicit so the MCP control-plane boundary stays clear.
- The docs should state that Codex CLI `/plan` and `/goal` are TUI slash commands, not CCC-native actions, and that CCC maps them onto captain-owned Way/task-card state instead of claiming native slash-command integration.
- If statusline, title, autoreview, or keymap behavior is mentioned, it should be framed as wrapper or documentation surface area rather than shipped native integration.
- The Korean 0.0.9-pre plan identified a recurring gap where documentation-only requests can drift to `raider` instead of `scribe`, multi-step plans collapse into a single-line checklist, and fan-in does not immediately refresh visible checklist/status output.
- Small documentation changes can appear slow when captain spawns an optional reviewer and then waits too long without bounded status polling, visible follow-up, or reclaim/retry/reassign.
- Complex or risky requests should pass through an explicit captain intent-confirmation gate before Way or specialist handoff; this behavior should be rechecked because it has not been reliable enough.
- `ccc-config.toml` has become too dense for an operator-facing file; user-editable agent settings should stay easy to scan, while internal routing, lifecycle, and operational defaults should move behind code defaults or a separate internal config layer.
- Review follow-up: the child-agent drift check must compare any observed specialist `child_agent_id` against the expected owner, not only direct `captain` bypasses.
- Review follow-up: the minimal-config backfill comment should say wholly omitted operational sections stay omitted, while existing sections can still be backfilled.
- Keep the note terse and aligned with the existing pre-release tone.

## Work Item

- Improve `ccc-config.toml` readability and organization so the user-facing config stays focused on agent settings and practical overrides.
- Separate internal or non-user-facing CCC behavior from the operator-editable config surface, using code defaults or a less-visible internal config file when appropriate.
- Keep public release wording modest for this change: describe it as config readability/organization improvement without exposing internal configuration details.
- Add a short boundary note in the release docs that distinguishes MCP control-plane surfaces from the operator-facing `$cap` output.
- Prepare the release-note validation evidence section so it can list the validation operator's already-passed medium-default checks and leave a clear pending placeholder for the new expertise-framing validation until raider reports.
- Tighten small-doc-change handling so optional review lanes use bounded status polling, visible follow-up, and reclaim/retry/reassign instead of long silent waits.
- Recheck the captain intent-confirmation gate so captain confirms its interpretation with the operator before Way/specialist handoff for complex or risky requests.
- Add long Codex CLI session mitigation UX so captain/status pressure signals recommend `/compact`, `/new`, or `/exit` when session, context, or resource pressure gets long; require operator choice, record a checkpoint, and emit a compact resume prompt/command for the next session.
- Fix the routing-drift defect so captain does not directly perform implementation, docs, or review work under `$cap` when a matching CCC specialist exists; record any drift as a captain intervention, route it to the correct specialist for adoption or repair, and review the result before merge.
- Detect wrong-specialist `child_agent_id` drift against the expected owner while preserving the direct-captain acceptance gate.
- Add task-specific expertise framing to captain/Way specialist prompts so each subagent gets an explicit role, stance, and thinking mode before work begins.
- Keep the boundary explicit: do not claim captain can directly execute TUI slash commands unless Codex CLI or a wrapper exposes that control surface.

## Additional Harness Gap Items

- Add run resume and crash recovery so a persisted `run_id` can resume with a clear next action after Codex, terminal, or host interruption.
- Add hard acceptance gates so a completed specialist result is not treated as done until acceptance criteria, changed files, and verification results are checked.
- Define budget and resource policy for subagent count, review count, wait time, token/context pressure, and stop/replan/defer behavior.
- Define deterministic retry and repair budgets so failed or unsatisfactory fan-in has a bounded same-worker repair, reassignment, or stop path.
- Strengthen automatic stall detection and reclaim guidance so late lanes surface a reclaim/retry/reassign recommendation without relying on long manual waits.
- Standardize verification commands and evidence fields so every work unit records the checks run and the files, logs, or command output that support completion.
- Clarify active-run conflict handling so a new `$cap` request can be merged, split into a new run, or used to reclaim the existing run without blurring scope.
- Strengthen operator intervention records so mid-run user input is classified as clarification, scope change, or risk correction before it affects active work.
- Clarify release and commit boundaries so CCC can distinguish task completion from a verified, commit-ready work unit.
- Add long-session mitigation as a 0.0.9 requirement: keep `/compact`, `/new`, and `/exit` recommendations tied to status/context/resource pressure, operator choice, checkpointing, and compact resume output.

## Acceptance

- The docs state that MCP surfaces exist for control-plane work, while ordinary `$cap` output remains compact and CLI-shaped for operators.
- The docs explicitly avoid framing CCC as fully MCP-first.
- Rendered tool-call text like `Called ccc.ccc_orchestrate({json})` is not presented as the user-facing output model.
- The docs and README-facing role/default text state that `scribe`, `companion_reader`, and `companion_operator` default to `medium` reasoning/variant.
- The docs and README-facing prompt wording state that captain and Way generate task-specific expertise framing for each specialist handoff, including role, stance, and thinking mode.
- The docs and README-facing prompt wording state that `/plan`-like work maps to `Way` or `ccc_orchestrate` replan state, while `/goal`-like work maps to run/task-card `goal`, `intent`, `scope`, and `acceptance` fields.
- Doc/translation requests are described as `scribe`-first, not `raider`-first.
- LongWay/release-planning checklists are described as multi-line when a plan has multiple steps.
- Checklist and lifecycle/status updates are described as staying in sync after fan-in.
- Optional reviewer stalls are described as requiring bounded status polling, visible follow-up, and reclaim/retry/reassign behavior.
- Complex or risky requests are described as requiring captain interpretation confirmation before Way/specialist handoff.
- The operator-facing config is described as readable and focused on agent settings/practical overrides.
- Internal routing, lifecycle, and operational defaults are described as belonging outside the main user-editable config surface.
- Release-facing notes summarize the config change as a readability/organization improvement only.
- Release-facing notes treat assets and checksums as publish-stage artifacts instead of already-produced deliverables.
- Long-session mitigation surfaces a status/context/resource-pressure recommendation for `/compact`, `/new`, or `/exit`, requires the operator to choose the action, records a checkpoint before rollover, and emits a compact resume prompt or command.
- Captain routing-drift is treated as a required intervention path: direct captain implementation, docs, or review work under `$cap` is not allowed when a matching CCC specialist exists; drift is recorded, routed to the correct specialist for adoption or repair, and reviewed before merge.
- Wrong-specialist `child_agent_id` drift is recorded as a mismatch against the expected owner without reusing the direct-captain acceptance gate.
- The docs do not claim captain can directly execute Codex TUI slash commands unless Codex CLI or a wrapper exposes that control surface.
- The release note includes a validation evidence section with the already-passed medium-default checks and a clear pending placeholder for the new expertise-framing validation.
- Resume, hard acceptance, budget/resource policy, long-session mitigation, deterministic retry/repair, stall recovery, verification/evidence records, active-run conflict handling, operator intervention records, and release/commit boundaries are tracked as explicit harness-readiness gaps.
