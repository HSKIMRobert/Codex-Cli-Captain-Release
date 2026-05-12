# Release Docs

This directory tracks the current public release note, local validation notes, and public release-facing install guidance.
- [`notes/v0.0.15-pre.md`](./notes/v0.0.15-pre.md): current public pre-release note.
- [`../release-work/0.0.15/PRE_RELEASE_PLAN.md`](../release-work/0.0.15/PRE_RELEASE_PLAN.md): active planning `0.0.15` pre-release work document, focused on the docs-and-release-gates slice that aligns Rust-first CCC, `$cap`, `ccc_*` routing, checkpoint/resume, concurrency/background lifecycle, hooks, verification, workflow-loop Skill guidance, and plugin packaging files.
- [`../release-work/0.0.14/PRE_RELEASE_PLAN.md`](../release-work/0.0.14/PRE_RELEASE_PLAN.md): previous `0.0.14` pre-release work context, focused on intent-state-machine gates, subagent-first routing, evidence-before-mutation enforcement, recovery visibility, and compact surface alignment; see `notes/v0.0.14-pre.md` for the previous public pre-release note.
- [`notes/v0.0.14-pre.md`](./notes/v0.0.14-pre.md): previous public pre-release note.
- [`notes/v0.0.13-pre.md`](./notes/v0.0.13-pre.md): previous public pre-release note.
- [`../release-work/0.0.13/PRE_RELEASE_PLAN.md`](../release-work/0.0.13/PRE_RELEASE_PLAN.md): previous `0.0.13` pre-release work document, focused on SSL-backed Skill Registry hardening, Way clarification interviews, App/CLI visibility normalization, and packaged SSL manifest smoke coverage.
- [`../release-work/0.0.12/PRE_RELEASE_PLAN.md`](../release-work/0.0.12/PRE_RELEASE_PLAN.md): previous `0.0.12` pre-release work document, focused on structured runtime flow, phase-aware routing, scheduler-owned planned-row materialization, document-root graph/memory, optional SSL skill manifests, Sisyphus harness coverage, and Codex App/CLI visibility normalization.
- [`notes/v0.0.12-pre.md`](./notes/v0.0.12-pre.md): previous public pre-release note.
- [`../release-work/0.0.11/PRE_RELEASE_PLAN.md`](../release-work/0.0.11/PRE_RELEASE_PLAN.md): previous `0.0.11` pre-release work document, focused on the CCC Sisyphus walking skeleton, PLAN_SEQUENCE / EXECUTE_SEQUENCE split, pending LongWay approval, task cards, checklist/status/fan-in truth, restart handoff, and the explicit follow-up hardening boundary.
- [`../release-work/0.0.11/CCC_MEMORY.md`](../release-work/0.0.11/CCC_MEMORY.md): internal captain guidance for routing, lifecycle, fan-in, fallback, context, host `/plan`/`/goal` compatibility, and persisted `captain_instruction` policy that should not live in the public `$cap` skill.
- [`../release-work/0.0.11/CCC_SISYPHUS_LOOP_REFERENCE.md`](../release-work/0.0.11/CCC_SISYPHUS_LOOP_REFERENCE.md): preserved operator-provided Sisyphus loop design input for `$cap`, `/plan`, `/goal`, LongWay, scheduler/router, fan-in, context health, and restart handoff.
- [`notes/v0.0.11-pre.md`](./notes/v0.0.11-pre.md): previous public pre-release note.
- [`../release-work/0.0.10/PRE_RELEASE_PLAN.md`](../release-work/0.0.10/PRE_RELEASE_PLAN.md): previous `0.0.10` pre-release work document, focused on LongWay checklist semantics, planned owner/role metadata, assignment-quality routing drift visibility, the CCC-native Rust graph core and CLI/MCP/status wiring, upstream parity/spec reference, temporary bridge language only for migration or development, the shipped narrow compact per-row lifecycle projection, and the opt-in workspace CCC memory surface.
- [`../release-work/0.0.10/CCC_MEMORY_SPEC.md`](../release-work/0.0.10/CCC_MEMORY_SPEC.md): focused `0.0.10` CCC memory spec for the small opt-in workspace memory file, preview/write/off/status safety contract, and deferred automation.
- [`notes/v0.0.10-pre.md`](./notes/v0.0.10-pre.md): previous public pre-release note.
- [`notes/v0.0.9-pre.md`](./notes/v0.0.9-pre.md): previous public pre-release note.
- [`../release-work/0.0.9/PRE_RELEASE_PLAN.md`](../release-work/0.0.9/PRE_RELEASE_PLAN.md): planned `0.0.9` pre-release work document, including optional-review stall handling, medium-default reasoning for `scribe` and the companion roles, task-specific expertise framing for captain/Way specialist prompts, and matching-specialist review rules.
- [`notes/v0.0.8-pre.md`](./notes/v0.0.8-pre.md): previous pre-release note.
- [`../release-work/0.0.8/PRE_RELEASE_PLAN.md`](../release-work/0.0.8/PRE_RELEASE_PLAN.md): planned `0.0.8` pre-release work document.
- [`notes/v0.0.7-pre.md`](./notes/v0.0.7-pre.md): older pre-release note.
- [`notes/v0.0.6-pre.md`](./notes/v0.0.6-pre.md): previous pre-release note.
- [`RELEASE_WORKFLOW.md`](./RELEASE_WORKFLOW.md): develop-to-main release workflow.
- [`notes/v0.0.5-pre.md`](./notes/v0.0.5-pre.md): previous pre-release note.
- [`notes/v0.0.4.md`](./notes/v0.0.4.md): previous public release note.
- [`notes/v0.0.3.md`](./notes/v0.0.3.md): older public release note.
- [`notes/v0.0.2.md`](./notes/v0.0.2.md): previous public release note.
- [`../release-work/0.0.5/PRE_RELEASE_PLAN.md`](../release-work/0.0.5/PRE_RELEASE_PLAN.md): planned `0.0.5` pre-release work document.
- [`../release-work/0.0.1/README.md`](../release-work/0.0.1/README.md): active Rust reset checklist for the fresh-history `v0.0.1` train.

This reset train treats `v0.0.1` as the fresh-history public baseline. `v0.0.15-pre` is the current public pre-release note on top of that baseline, `v0.0.14-pre` is the previous public pre-release note, and `v0.0.4` is the previous public release note. Public GitHub release cards should be updated in `Codex-Cli-Captain-Release` when the current release is published.

## Commit Boundaries

Release work should be committed by coherent work unit. Keep runtime changes, focused test repairs, source docs, release-repo docs, release metadata, and generated assets in separate commits unless the captain records why combining them is safer.

Before publishing, verify that each release-facing commit has a clear purpose, the relevant validation is recorded, and any required sync from source docs to `Codex-Cli-Captain-Release` is either committed separately or called out as pending.

Source changes should land on `develop` first. Release only after `develop` is validated and merged to `main`; publish tags, public release cards, and release assets from the released `main` state.
