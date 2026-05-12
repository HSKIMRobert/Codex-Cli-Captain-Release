# 0.0.7 Pre-Release Plan

`0.0.7-pre` should make CCC easier to operate in long-running Codex CLI work: clearer user-facing progress, sharper specialist handoffs, and less accumulated runtime complexity. This plan keeps the historical smoke baseline, then records the runtime slices that have already landed separately from the remaining docs/presentation work.

## Release Goal

Make the captain-first loop easier to follow and maintain. Done means the operator can see major LongWay progress without reading every task-card detail, scribe work starts from a captain-authored writing brief, git commits default to a consistent Conventional Commit style, and the codebase has an explicit cleanup path for oversized or tightly coupled runtime modules.

## Historical Smoke Baseline

Companion operator smoke already covers the core route and operator-text surfaces:

- `check-install` passed.
- `way` routed to `ccc_tactician`.
- `explore` routed to `ccc_scout`.
- `git commit` routed to `ccc_companion_operator`.
- code mutation routed to `ccc_raider`.
- docs/operator-text previously routed to `ccc_raider` instead of `ccc_scribe`.
- `ccc start` previously did not show the selected route or agent until `ccc status`.
- `ccc status` previously showed `Current Item` but not a checklist breakdown.

## Smoke Findings

These smokes are the historical baseline that drove the now-landed runtime fixes.

## Landed Runtime Fixes

- docs/operator-text routing now prefers `ccc_scribe`.
- `ccc start` now shows the selected route and agent at launch time.
- `ccc status` now shows a checklist-style completed/current/remaining breakdown.

## Current Issues To Inspect First

- Find any obvious correctness, lifecycle, or visibility gaps in the current `0.0.6-pre` flow before adding larger behavior.
- Review whether large Rust modules such as the MCP entrypoint, status rendering, task-card orchestration, and lifecycle handling need module extraction or smaller helper boundaries.
- Initial size check: `main.rs` is about 2.4k lines, `main_tests.rs` is about 9.6k lines, and `specialist_roles.rs`, `status_render.rs`, `install_check.rs`, `review_policy.rs`, and `worker_lifecycle.rs` are each near or above 850 lines. These are the first refactor candidates for `0.0.7-pre`.
- Keep refactors bounded: split only when it reduces real duplication, isolates testable behavior, or makes ownership clearer.
- Record deferrals explicitly when an issue is real but too large for the current pre-release slice.

## Work Items

### 1. Scribe handoff brief

Docs and operator-facing text should still be written by `ccc_scribe`, but captain should first decide the writing frame.

Work items:

- Add guidance that captain provides a compact writing brief before spawning scribe.
- The brief should include audience, intent, source evidence, tone, language, required structure, and any phrases to preserve or avoid.
- Preserve the current top placement of the README positioning copy, but strengthen its visual emphasis and presentation.
- Scribe should treat the brief as the boundary for the doc update and return changed files plus documentation-only validation.
- Add tests that generated custom-agent instructions include this scribe handoff contract.

### 2. Default commit message style

When the operator asks CCC to create a commit but does not provide a style, default to a Conventional Commit-like first line:

```text
fix(scope): English summary
```

Work items:

- Add prompt guidance for commit-boundary workers and companion operators.
- Prefer `type(scope): summary` with `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, or similar types.
- Default the summary to English unless the operator explicitly requests another language.
- Do not override an explicit user-provided commit message style.
- Add tests that commit-related delegated prompts include this default style.

### 3. Major LongWay progress visibility

As LongWay checklist items advance, captain should refine the checklist-style progress panel in the CLI surface.

Target shape:

```text
LongWay Progress 2/5
[x] Plan release scope
[>] Update README positioning docs        ccc_scribe
[ ] Add checklist progress renderer       captain/raider
[ ] Validate and review
[ ] Commit and push
```

Work items:

- Surface the current major task title or phase alongside the existing `LongWay: n/m completed` count.
- Render top-level items as a compact todo-style view with completed, current, and remaining states.
- Prefer the operator's language when the task title or request language is available; otherwise show the stored title and default to English for fixed labels.
- Update the visible progress state after meaningful lifecycle events such as agent close, `ccc subagent-update`, `ccc orchestrate`, `ccc status`, and status transitions.
- Group and throttle refreshes so the CLI stays readable during active work and does not re-render on every low-signal event.
- Avoid printing every checklist item; show top-level LongWay phases by default, collapse nested task-card items behind a `+N more` style summary, and show the assigned role/agent only for the current or recently completed item.
- Re-render after terminal fan-in or merge so the operator can immediately see that the finished agent's item moved from current to completed and what remains next.
- Keep compact status machine-readable and text status readable.
- Add tests for the text status progress line.

### 4. Cleanup and modularity review

Before adding large runtime behavior, inspect current module size and coupling.

Work items:

- Identify modules that are doing too many things.
- Start with low-risk extractions: status text helper functions, custom-agent prompt fragments, install-check formatting, and test fixture builders.
- Prefer extracting small helpers for status text, prompt construction, lifecycle state transitions, or release/report formatting where it lowers risk.
- Avoid large rewrites during the pre-release planning slice.
- Capture any deferred refactor candidates in the release notes or a follow-up work document.

## Non-Goals

- Do not replace the host Codex captain with detached `codex exec`.
- Do not print every nested LongWay checklist item or child-agent heartbeat on every status update.
- Do not reduce progress visibility to a lone `Current Work` line.
- Do not force one commit language when the operator clearly requested another.
- Do not move documentation writing back into captain; captain frames, scribe writes.

## Acceptance

- README positioning copy is synchronized across the source and release documentation surfaces, with top-of-file callouts in English, Korean, and Japanese.
- `0.0.7-pre` has this work plan checked in.
- Scribe, commit-message, and major-progress guidance is represented in runtime prompts or generated custom-agent instructions.
- Focused tests cover the new prompt/status expectations.
- Any remaining cleanup candidates are documented rather than silently ignored.
