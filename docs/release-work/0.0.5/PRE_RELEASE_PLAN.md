# 0.0.5 Pre-Release Plan

`0.0.5` should make CCC feel more like a harness without replacing the host Codex captain with `codex exec`.

The direction is captain-constrained orchestration:

- Host Codex stays the public captain because its live routing quality is better than detached `codex exec` captain turns.
- CCC becomes the control plane that owns allowed state transitions, fallback truth, fan-in, review gates, and completion authority.
- `codex exec` remains a visible fallback or detached worker path, not the main captain engine.

## Release Goal

Make the normal `$cap` path stricter and more OMO-like by requiring host captain decisions to pass through CCC state, while preserving the current custom-subagent routing path.

Done means a captain cannot silently skip CCC state by directly editing, finishing, reviewing, or spawning extra work when compact status says another action is required.

## Non-Goals

- Do not replace host Codex captain with an autonomous `codex exec` captain loop.
- Do not make `ccc_auto_entry` the default path for every request.
- Do not couple CCC to unstable Codex CLI internals or hidden host-agent state.
- Do not introduce unbounded retry, review, or reassignment loops.

## Core Design

### 1. Prefer preflight over auto-entry

`ccc_auto_entry` remains opt-in for bounded cases. The normal stricter entry path should use a no-mutation recommendation or preflight gate before creating or advancing a run.

Work items:

- Add or formalize a `ccc_preflight` surface, or harden `ccc_recommend_entry` as the no-mutation gate.
- Return `recommended_action`, `direct_allowed`, `requires_user_confirmation`, `active_run_summary`, `risk`, and `reason`.
- Keep low-confidence or ambiguous requests out of automatic run creation.
- Add tests for Korean, English, mixed-language, lookup, mutation, review, and active-run continuation requests.

Why:

- Earlier `auto_entry` behavior was error-prone because classification mistakes immediately created state.
- A no-write preflight lets host Codex correct routing while CCC still owns the contract.

### 2. Promote MCP from diagnostics to control-plane

The `$cap` skill should describe `mcp__ccc__...` tools as supported control-plane surfaces, not diagnostics-only helpers. For `0.0.5`, MCP should be a supported control-plane path for host captain state transitions.

Work items:

- Update packaged `$cap` guidance to allow MCP control-plane calls for status, orchestration, lifecycle, and preflight.
- Keep CLI command templates as the operator/debug fallback.
- Ensure MCP and CLI outputs expose the same compact contract fields.
- Add tests for MCP tool schemas and structured content for any new or promoted control-plane fields.

Why:

- A harness needs one authoritative transition API.
- MCP is the natural control-plane surface inside Codex, while CLI remains useful for manual recovery and debugging.

### 3. Enforce `next_step` as the captain action boundary

Compact status should be the captain's source of truth for the next allowed action.

Work items:

- Define a documented allowed-action matrix for `next_step`, `can_advance`, active subagent state, review state, and completion state.
- Block direct captain finish when fan-in, review, repair, or merge is pending.
- Block direct captain mutation when a matching specialist route is active or required.
- Surface a concise denied-action reason in status/preflight output.
- Add regression tests for `spawn_subagent`, `await_fan_in`, `captain_advance`, `review_required`, `blocked`, and `finish_allowed` states.

Why:

- Host Codex keeps good judgment, but CCC must decide whether the judgment is allowed in the current run state.
- This prevents silent skips around specialist fan-in, review gates, or completion discipline.

### 4. Add a captain action guard

Add a bounded preflight-style guard for captain intent before risky host-captain actions.

Proposed surface:

```text
ccc_captain_action
```

Example input:

```json
{
  "run_id": "...",
  "task_card_id": "...",
  "intended_action": "direct_mutation",
  "reason": "small operator-side fix",
  "evidence_paths": ["..."]
}
```

Example output:

```json
{
  "allowed": false,
  "required_action": "record_fallback_or_spawn_specialist",
  "reason": "Active task requires ccc_raider fan-in before direct mutation."
}
```

Work items:

- Support at least `direct_mutation`, `direct_finish`, `skip_review`, `spawn_extra_specialist`, `merge_fan_in`, and `fallback_codex_exec`.
- Require fallback metadata for direct captain work.
- Persist approved direct captain work as visible fallback/degradation metadata.
- Add tests for allowed low-risk direct actions and denied active-specialist/review/fan-in bypass attempts.

Why:

- This gives CCC an explicit enforcement hook without removing the host captain.
- It turns "captain should not do that" into a visible allowed/denied state transition.

### 5. Require fan-in before follow-up

Specialist output is input to CCC, not the final answer.

Work items:

- Make the required sequence explicit: subagent result -> `ccc_subagent_update` -> `ccc_orchestrate` -> `ccc_status` -> next action.
- Deny finish or replan when a terminal specialist result has not been merged or explicitly reclaimed.
- Keep stale or late output visible without letting it overwrite the chosen path.
- Add tests for missing fan-in, late fan-in, reclaimed fan-in, merged fan-in, and repeated terminal update cases.

Why:

- Without mandatory fan-in, CCC becomes a logger instead of a harness.
- Fan-in is where review, repair, reassignment, and completion discipline can be enforced.

### 6. Strengthen direct-fallback truth

Direct captain work should be rare, visible, and classed.

Work items:

- Define fallback categories such as `trivial_operator_side_fix`, `operator_override`, `degraded_host_fallback`, `sandbox_blocked`, and `tool_unavailable`.
- Require a reason and scope for direct captain work when a specialist route exists.
- Surface fallback history in full status, compact status, activity, and text output.
- Add tests that direct mutation without fallback metadata is denied when a specialist route is required.

Why:

- The practical harness boundary is not "never direct work"; it is "no silent direct work."
- Visible fallback lets operators audit when the harness was bypassed and why.

### 7. Keep review and repair bounded

Review and repair should remain captain-owned and resource-aware.

Work items:

- Keep skipped/recommended/required/suppressed review states visible.
- Enforce same-worker amend and reassignment budgets through state.
- Deny duplicate mutable workers for the same scope unless an explicit replan changes the scope.
- Add tests for budget exhausted, budget unavailable, missing reassign target, duplicate repair, and resource-pressure review suppression.

Why:

- Harness quality comes from bounded loops, not more agents.
- Review must remain input to the captain, not a free agent-to-agent chain.

### 8. Reduce packaged `$cap` skill exposure

The current release repository exposes the packaged `$cap` skill text. That is technically workable because the installer needs to place a readable skill under the user's Codex home, but it exposes a large part of the harness contract directly in the public release repository.

`0.0.5` should treat the skill as a thin bootstrap surface and move durable harness policy into the Rust runtime and MCP control plane.

Work items:

- Review whether `Codex-Cli-Captain-Release/share/skills/cap/SKILL.md` should remain browsable in the public release repository.
- Replace the public release-repo skill source with a minimal placeholder or installation note if public browsing is not desired.
- Keep the installable `$cap` skill available through the packaged release asset or another distribution path.
- Shrink the installed `$cap` skill to bootstrap guidance only: enter CCC, read compact status, obey the control-plane contract, and avoid bypassing CCC state.
- Move detailed routing, fallback, fan-in, review, and repair policy out of the skill text and into runtime/MCP-enforced state.
- Document that any skill installed locally must be readable by the user and Codex; release packaging can reduce public repository exposure, but cannot make installed local skill text secret from the installing user.

Why:

- The skill is necessary for installation and operator guidance, but it should not carry the full strategic harness logic.
- A thin skill plus runtime-enforced policy is harder to copy from the public repo and easier to keep consistent.
- The only enforceable boundary is the runtime/control-plane contract; text-only skill instructions are not a strong protection mechanism.

### 9. Decide release-repo history treatment

Because the current release repository has already committed the packaged `$cap` skill, deleting it in a future commit will not remove it from public Git history.

Work items:

- Decide whether public repository history must be rewritten before the next release.
- If history rewrite is required, create a new clean release repository history or force-push a scrubbed history after explicit operator approval.
- Rotate or invalidate any release artifacts, tags, or caches that still expose the old skill text if the operator treats the old text as sensitive.
- Update install URLs, branch assumptions, tags, release cards, and verification scripts if the release repository is recreated or history is rewritten.
- Record the tradeoff: history rewrite reduces casual public access, but cannot recall already cloned copies, downloaded assets, forks, mirrors, or cached pages.
- Treat destructive git operations, force pushes, tag rewrites, and repository recreation as explicit release-operator actions, not automatic CCC steps.

Why:

- Public Git history is durable. Removing the file in a normal commit only hides it from the latest tree.
- If the goal is "new users cannot browse old skill text from the public repo," history rewrite or a fresh release repository may be necessary.
- The decision has operational risk because installers, release tags, GitHub release assets, and public documentation may rely on the current repository layout.

## Documentation Work

Source repository updates:

- Update `README.md`, `README.ko.md`, and `README.ja.md` with the `0.0.5` captain-constrained harness contract after implementation lands.
- Update `docs/project-plan.md` with the 0.0.5 section and final landed/deferred/blocked ledger.
- Add or update `docs/release/notes/v0.0.5.md` when the pre-release scope is ready for public release.
- Update `docs/release/README.md` so the current and previous release-note links are correct.
- Update `skills/cap/SKILL.md` to describe MCP as a supported control-plane surface, not diagnostics-only.
- Update source docs to explain that the installed `$cap` skill is not secret from the installing user, and that sensitive harness behavior belongs in runtime-enforced policy rather than long prompt text.
- Use `docs/release/VALIDATION_RUNBOOK.md` for maintainer-only smoke validation before publishing and after the public release path is available.

Current docs-sync slice landed:

- source `README.md`, `README.ko.md`, and `README.ja.md` now use the same captain-constrained preflight/control-plane wording
- `Codex-Cli-Captain-Release/README.md` now carries a non-published 0.0.5 pre-release planning note
- `docs/project-plan.md` now carries a concise `0.0.5` work ledger with the remaining blockers
- `docs/release/notes/v0.0.5-pre.md` now distinguishes landed runtime slices from the still-open pre-release items

Release repository updates:

- Sync `Codex-Cli-Captain-Release/README.md`, `README.ko.md`, and `README.ja.md`.
- Sync `Codex-Cli-Captain-Release/docs/install.md` if install/check-install or setup output changes.
- Either sync `Codex-Cli-Captain-Release/share/skills/cap/SKILL.md` as a thin public bootstrap file or remove it from the public tree and package the installable skill only in release assets.
- Add `Codex-Cli-Captain-Release/docs/release/notes/v0.0.5.md` as the GitHub release card body source.
- Update `release-repo-manifest.json`, install defaults, and verification scripts only when the packaged version changes.
- If history rewrite or repository recreation is selected, update all release-repo docs and install instructions to match the new repository state.

Why:

- Operators should see the same harness contract in source docs, installed skill guidance, release docs, and the public release card.
- Release docs should not claim a stricter harness before the runtime and packaged skill actually enforce it.

## Release Work

Pre-release checklist:

- Bump source crate/package version to `0.0.5-pre` or the chosen pre-release version only when the runtime scope starts landing.
- Decide the `$cap` skill exposure policy before packaging.
- If needed, prepare a scrubbed release repository history or fresh release repository before publishing the next release card.
- Run the full source test suite.
- Run setup/check-install against the built binary.
- Verify the installed `$cap` skill and CCC-managed custom agents match the source package.
- Run release-repo non-mutating asset and install-smoke checks.
- Build platform assets for the supported matrix only after source docs, release docs, and packaged skill content are synchronized.
- Draft the GitHub release card from `Codex-Cli-Captain-Release/docs/release/notes/v0.0.5.md`.
- Publish as a pre-release first; promote only after install/update and real `$cap` workflows are verified.

## Open Blockers

- Raw token totals for host custom subagents remain deferred when no raw usage events exist. The current runtime only has raw usage events, `worker_result.total_token_usage`, and context estimates; it does not have a separate accounting source-of-truth that would justify fabricating raw totals.
- Platform install matrix and full download/install smoke remain external validation work. They need networked install/download verification and should stay marked as pending until that local run is explicitly captured.

Validation commands:

```bash
cargo test -p ccc --offline
ccc check-install
npm run test:release
```

Adjust the release-repo validation command if the release repository changes its script names before `0.0.5`.

## Acceptance Checklist

- [ ] Preflight/recommendation path is no-mutation and safer than `auto_entry` for default use.
- [ ] MCP is documented and tested as a control-plane path for captain state transitions.
- [ ] Compact status exposes a clear allowed-action contract.
- [ ] Captain action guard denies direct finish/mutation/review skip when CCC state forbids it.
- [ ] Direct captain fallback requires visible metadata.
- [ ] Specialist output cannot become final without CCC fan-in.
- [ ] Review and repair remain bounded and captain-owned.
- [ ] `$cap` skill exposure policy is decided and documented.
- [ ] Public release repo either keeps only a thin bootstrap skill or packages the installable skill outside the browsable tree.
- [ ] If required, release-repo history is scrubbed or recreated with explicit operator approval and updated install URLs/tags/docs.
- [ ] `$cap` packaged skill, generated custom agents, source README files, release repo README files, install docs, release note, and GitHub release card body are synchronized.
- [ ] Source and release-repo validation pass.
- [ ] Public release is marked pre-release until real operator workflows verify the stricter harness contract.

## Release Card Draft Outline

Use this outline for the public release card after runtime and docs land:

- Title: `v0.0.5-pre: Captain-constrained harness control plane`
- Summary: CCC keeps host Codex as the routing captain while moving state transitions, fan-in, fallback truth, and review gates into a stricter CCC control-plane contract.
- Highlights:
  - no-mutation preflight before default run creation
  - MCP control-plane path for host captain state transitions
  - `next_step` allowed-action enforcement
  - captain action guard for direct mutation, finish, review skip, and fallback
  - mandatory fan-in before finish or follow-up
  - visible fallback/degradation history
  - thinner `$cap` bootstrap skill with core harness policy enforced by CCC runtime/control-plane
- Upgrade notes:
  - restart Codex CLI after install/update
  - run `ccc check-install`
  - re-run `ccc setup` after config or packaged skill changes
- Validation:
  - source tests
  - setup/check-install
  - release-repo non-mutating checks
  - manual `$cap` workflow smoke tests
