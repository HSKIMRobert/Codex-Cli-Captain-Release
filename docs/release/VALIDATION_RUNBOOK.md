# Release Validation Runbook

Owner-facing smoke validation for CCC before release and after release.
Use this for maintainer checks, not end-user guidance.

Sprint 9 status: source version/install metadata is aligned to `0.0.15-pre`; publication/install/restart/live checks still remain before the release can be treated as complete.

For the `0.0.15` docs-and-release-gates slice, keep the checks lightweight and documentation-first: confirm the operator guidance matches the current CCC surfaces, does not add a new public command path, and keeps the documented gate coverage aligned with the 0.0.15 surfaces below without implying public release validation has been completed.

## Scope

For every release, validate the baseline install and control-plane surfaces. When a release has new harness behavior, add those changed slices to the smoke pass. For `0.0.15` docs-and-release-gates coverage, keep the changed slices documented as:

- checkpoint/resume coverage for gate state, delegated work, fan-in, and next legal action
- concurrency and background lifecycle coverage for task states, stale output, reclaim rules, and provider/model limits
- hook-tier coverage for recovery, compaction, guard, continuation, and notification lifecycle points
- config and `check-install` expansion for registry sync, fallback policy, hooks, prompts, and custom-agent readiness
- verification capsule coverage for acceptance, evidence, reviewer verdict, validation, and unresolved risk
- workflow-loop Skill guidance coverage for the bundled CCC loop and bounded retry/replan behavior
- plugin packaging coverage for `.codex-plugin/plugin.json`, `.mcp.json`, and `skills/ccc/SKILL.md`
- plugin packaging boundary coverage that keeps `$cap` as the public operator entrypoint

Earlier changed-feature slices that remain relevant include:

- no-mutation preflight before default run creation
- MCP as a supported control-plane surface
- compact status as the allowed-action contract
- captain action guard for direct mutation, finish, review skip, and fallback
- mandatory fan-in before finish or follow-up
- visible direct-fallback and degraded-path history

Every release should also keep the baseline install surfaces green:

- `ccc --version`
- `ccc setup`
- `ccc check-install`
- packaged `$cap` skill sync
- CCC-managed custom-agent sync
- release asset naming and install pruning

When validating an update or repair release, keep the previous bundle or asset published and installable so downgrade remains available. The update should be additive unless the release notes explicitly say otherwise.

## When To Use Local-Only Checks

Use local-only checks when iterating before publication, when network access is unavailable, or when you only need to confirm the source tree and the installed local bundle.

Local-only checks should cover:

- Rust source validation
- release-repo script checks that do not download assets
- local `ccc setup` and `ccc check-install`
- scratch-run smoke against an installed local binary

## When To Use Network Or Live Release Checks

Use network/live checks only after the release asset or pre-release tag is published, or when you need to prove the public install path.

Live checks should cover:

- `gh release view` against the published tag
- installer download from the release repository
- platform-specific install smoke on the published asset matrix
- post-install `ccc check-install` after a full Codex CLI restart

Do not use live checks as the default pre-release iteration loop.

## Baseline Validation

Run source checks from the source checkout.

```bash
cargo test -p ccc --offline
cargo build --offline
```

Expected pass signals:

- `cargo test` exits 0
- `cargo build` exits 0 and builds the `ccc` crate for the local platform

Run release packaging checks from the source checkout.

```bash
cd /Users/kwkim-hoir/dev/home/Codex-Cli-Captain
scripts/release/verify-release-asset-matrix.sh
scripts/release/verify-windows-install-smoke.sh
```

Expected pass signals:

- asset-matrix verification exits 0
- Windows install smoke exits 0, even when it only checks script behavior on the local host

Then verify the installed local surface:

```bash
ccc --version
ccc check-install
ccc server-identity
```

Expected pass signals:

- `ccc --version` reports the current release version (`0.0.15-pre` for the
  current pre-release pass)
- `ccc check-install` reports `status=ok`
- top-level install status is current, matching, and does not require restart
- packaged `$cap` skill is current or matching the install
- CCC-managed custom agents are in sync
- server identity reports the current release version (`0.0.15-pre` for the
  current pre-release pass) and a coherent install check payload

If you changed config or installed assets, also run:

```bash
ccc setup --dry-run
```

Expected pass signals:

- canonical config is preserved or backfilled as expected
- backup is only requested when needed
- restart is only required when setup actually changed a surface

## Changed-Feature Smoke

Use a scratch workspace and a fresh run when checking the harness behavior itself.

### 1. Preflight gate

In Codex, call `mcp__ccc__.ccc_recommend_entry` for a request that would otherwise create or mutate state.

Expected pass signals:

- no-mutation requests are classified as safer than direct entry
- the response includes the recommended action and reason
- ambiguous or risky requests do not auto-create state

### 2. Allowed-action contract

Create a scratch run, then inspect status before and after one bounded orchestration step.

```bash
ccc start --quiet --json '{"prompt":"scratch validation","title":"v005-smoke","intent":"validate control plane","goal":"confirm status and guard surfaces","scope":"scratch run only","acceptance":"done when status and guard surfaces are readable","task_kind":"way","compact":true}'
ccc status --text --json '{"run_id":"<run_id>"}'
ccc orchestrate --quiet --json '{"run_id":"<run_id>","progression_mode":"single_step","compact":true}'
ccc status --text --json '{"run_id":"<run_id>"}'
ccc activity --json '{"run_id":"<run_id>","compact":true}'
```

When validating inside Codex, prefer quiet CLI subcommands for operator-visible lifecycle mutations so the transcript records `ran`: `ccc start --quiet --json-file`, `ccc orchestrate --quiet --json-file`, `ccc subagent-update --quiet --json-file`, and `ccc memory --quiet --json-file` (or `--quiet --json` for inline payloads). Reserve MCP tool calls such as `mcp__ccc__.ccc_status` and `mcp__ccc__.ccc_server_identity` for app surfaces, structured inspection, or CLI-unavailable fallback.

Expected pass signals:

- `ccc start` returns a run id and task card id
- status remains readable after orchestration
- activity remains readable and includes the latest attempt/checkpoint summary
- active runs expose the next allowed step and whether advancement is allowed
- blocked states show a denial reason instead of silently skipping the guard

### 3. Fan-in and review discipline

Use a run that reaches specialist fan-in or review gating.

Expected pass signals:

- terminal specialist output does not become final without CCC fan-in
- finish or follow-up stays blocked until merge or reclaim is recorded
- review-required states keep the captain on the guarded path
- fallback or degraded-path history stays visible in status

### 4. Negative JSON path

Confirm status parsing still fails cleanly on invalid JSON.

```bash
ccc status --json '{not valid}'
```

Expected pass signals:

- command exits non-zero
- error text identifies invalid JSON clearly

### 5. 0.0.11 walking-skeleton smoke

Use a scratch repo/workspace and avoid reusing a release-run worktree. For
code-graph checks, avoid a shared directory like `/private/tmp` unless the run
also passes the intended repo `cwd`; otherwise status can correctly report graph
context as unavailable because the directory is not a repo and may contain
multiple child graph stores.

Expected pass signals:

- installed `skills/cap/SKILL.md` is thin and states that `$cap` works without
  host `/plan` or `/goal`
- `docs/release-work/0.0.11/CCC_MEMORY.md` documents internal routing,
  lifecycle, fan-in, fallback, context, `/plan`/`/goal` compatibility, and
  `captain_instruction` guidance
- a broad or risky request can produce a pending LongWay during PLAN_SEQUENCE
  without mutating files
- EXECUTE_SEQUENCE uses the approved LongWay to materialize task cards and
  checklist rows
- status/checklist output projects planned rows under matching phase rows and
  keeps unmatched planned rows top-level
- compact fan-in, lifecycle, checklist, and persisted run state agree on the
  current progress truth
- context-health output provides a restart handoff or an explicit not-needed
  state
- visibility-only smoke requests route as read-only diagnostics: review policy
  is skipped, assignment drift is false, context health is ok, and
  `ccc status --app-panel --text` prints a readable LongWay/status panel
- `ccc check-install` reports matching packaged `$cap` skill and custom-agent
  sync after `ccc setup` and a full Codex CLI restart

Focused test names to include when validating the planned-row slice:

```bash
cargo test -p ccc ccc_status_projects_distinct_longway_planned_rows
cargo test -p ccc ccc_status_projects_planned_rows_under_matching_phase_without_duplicate_top_level
cargo test -p ccc ccc_status_keeps_unattached_planned_rows_top_level
cargo test -p ccc ccc_start_persists_planned_longway_rows_without_materializing_task_cards
cargo test -p ccc ccc_orchestrate_advance_materializes_next_planned_row
cargo test -p ccc ccc_orchestrate_planned_row_materialization_is_idempotent
```

## Post-Release Live Checks

Run these only after the release tag or pre-release asset is published.

```bash
gh release view v0.0.15-pre --repo HoRi0506/Codex-Cli-Captain-Release
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash
```

Expected pass signals:

- the release exists and is marked as the intended release type
- the installer completes without download or install errors
- `ccc setup` runs during install
- `ccc check-install` runs during install

After install, fully restart Codex CLI and re-run:

```bash
ccc check-install
ccc status --text
```

Expected pass signals:

- `ccc check-install` still reports `status=ok`
- install surfaces are current after restart
- the live bundle matches the published release, not the old local checkout
- if the release includes a runtime-fix candidate, the original release asset is still available for rollback

For a direct asset smoke on a published bundle, use the asset URL captured from the release page and set `CCC_DOWNLOAD_URL` explicitly.

## Release-Stage Decision Rules

- Use local-only checks until the source tree and release docs are stable.
- Use live checks only after the release asset is published and you need to validate the public install path.
- Treat platform matrix checks as live validation only for platforms that actually have a published asset.
- Do not mark the release complete until the live install path passes on the intended release bundle.
- Do not mark the public release complete until the later runtime or live-release pass is actually run.

## Validation Record Template

Record each smoke pass in this shape:

```md
## Validation Record
- Date:
- Release:
- Environment:
- Run id:
- Commands:
- Expected pass signal:
- Actual result:
- Evidence:
- Notes:
```

## Validation Record
- Date: 2026-05-01
- Release: `v0.0.9-pre`
- Environment: macOS local source checkout, `/Users/kwkim-hoir/dev/home/Codex-Cli-Captain`, branch `develop`
- Run id: `c214a000-d26c-0c11-89f7-3e56203dbd10`
- Commands:
  - `cargo fmt --manifest-path rust/ccc-mcp/Cargo.toml --check`
  - `git diff --check`
  - `cargo build --offline`
  - `cargo test -p ccc`
- Expected pass signal:
  - format check exits 0
  - diff whitespace check exits 0
  - offline build exits 0
  - full `ccc` test suite exits 0
- Actual result:
  - `cargo fmt --check` passed
  - `git diff --check` passed
  - `cargo build --offline` passed
  - `cargo test -p ccc` passed: `180 passed; 0 failed`
- Evidence:
  - `docs/release/notes/v0.0.9-pre.md`
  - `docs/release-work/0.0.9/PRE_RELEASE_PLAN.md`
  - `rust/ccc-mcp/src/main_tests.rs`
- Notes:
  - This is a local pre-release validation pass before publishing `v0.0.9-pre`.
  - Live install smoke is intentionally deferred until the public pre-release asset is published.

Short form example:

```md
- Date: 2026-04-26
- Release: v0.0.6-pre
- Environment: macOS arm64, local install
- Run id: <run_id>
- Commands: ccc check-install; ccc status --text; cargo test -p ccc --offline
- Expected pass signal: install=current, status readable, tests pass
- Actual result: pass
- Evidence: <path or run id>
- Notes: none
```

## Completed Local Validation Record: 2026-05-08

- Date: 2026-05-08
- Release: `0.0.15-pre` validation slice over local `0.0.14-pre` runtime/install metadata
- Environment: macOS arm64 source checkout, `/Users/kwkim-hoir/dev/home/Codex-Cli-Captain`, dirty accumulated release worktree on `main`
- CCC run id: `b9440ce2-206e-378d-d814-2b7fa1802f4e`
- Scratch run id: `7414e956-03b5-7d03-9932-2d59522c6ce6`
- Commands:
  - `cargo fmt --all --check`
  - `git diff --check`
  - `jq empty .codex-plugin/plugin.json .mcp.json`
  - focused docs `rg` checks for `$cap`, plugin packaging, workflow loop, checkpoint/resume, concurrency/background, hooks, verification capsule, and default commit guidance
  - `cargo check -p ccc --offline`
  - `cargo test -p ccc --offline`
  - `cargo build --offline`
  - `scripts/release/verify-windows-install-smoke.sh`
  - `scripts/release/verify-install-pruning.sh`
  - `scripts/release/verify-release-asset-matrix.sh`
  - `ccc --version`
  - `ccc check-install`
  - `ccc server-identity`
  - `ccc setup --dry-run`
  - `ccc status --json '{not valid}'`
  - `ccc start/status/activity/orchestrate` scratch lifecycle smoke for run `7414e956-03b5-7d03-9932-2d59522c6ce6`
- Expected pass signal: source checks pass; plugin package structure is valid; docs keep `$cap` as the public entry; release scripts pass without publishing; installed local surface is current; invalid JSON fails clearly; scratch status/projection surfaces remain readable and guarded.
- Actual result: pass for the local validation slice.
- Evidence:
  - `cargo test -p ccc --offline`: `314` unit tests passed and `5` plugin package tests passed.
  - `ccc --version`: `0.0.14-pre`.
  - `ccc check-install`: `status=ok`, install surface current, matching MCP registration, packaged `$cap` skill current, custom agents synced, restart not required.
  - `ccc server-identity`: `server_version=0.0.14-pre` with coherent install check payload.
  - Negative JSON path exited non-zero with `Invalid JSON for status: key must be a string at line 1 column 2`.
  - Scratch lifecycle smoke showed `PLAN_SEQUENCE`, `planning-approval`, and `await_longway_approval` before and after bounded orchestration.
  - Release scripts passed; PowerShell was unavailable, so Windows smoke used static `install.ps1` checks.
- Notes:
  - Local `stable-v0.0.14-pre` exists and points at `origin/main`/`cc0e386379f8c2c4f5827a478f20c7ae4e0c1bc8` because the source repo has no `v0.0.14-pre` tag or dedicated source ref.
  - The asset-matrix script initially failed under sandboxed filesystem access while replacing an existing local sibling release tarball; rerun with approved filesystem access passed. No publishing or network download was performed.
  - Live GitHub release lookup, public installer download, live install, push, tag creation, remote release creation, branch switching, and post-restart live bundle validation were intentionally not run.

## Completed Local Smoke Record: 2026-04-26

- Date: 2026-04-26
- Release: `v0.0.6-pre`
- Environment: macOS arm64, local source checkout plus installed `0.0.6-pre` bundle
- CCC run id: `0c3a439b-65fc-ea64-c3a1-a72504821cb6`
- Scratch run id: `e026f151-c5ff-fa39-ef8b-ee4a6d60e27f`
- Commands:
  - `cargo test -p ccc --offline`
  - `cargo build --offline`
  - `ccc --version`
  - `ccc check-install`
  - `ccc setup --dry-run`
  - `mcp__ccc__.ccc_recommend_entry`
  - `mcp__ccc__.ccc_status`
  - `mcp__ccc__.ccc_server_identity`
  - `ccc start --quiet --json ...`
  - `ccc status --text --json ...`
  - `ccc orchestrate --quiet --json ...`
  - `ccc activity --json ...`
  - `ccc status --json '{not valid}'`
  - `scripts/release/verify-release-asset-matrix.sh`
  - `scripts/release/verify-windows-install-smoke.sh`
- Expected pass signal: source tests/build pass; install surface is current; MCP control-plane surfaces expose preflight, status, and identity; scratch lifecycle remains readable; invalid JSON fails clearly; release packaging scripts pass.
- Actual result: pass.
- Evidence:
  - `cargo test -p ccc --offline`: 141 passed, 0 failed.
  - `cargo build --offline`: finished successfully.
  - `ccc --version`: `0.0.6-pre`.
  - `ccc check-install`: `status=ok`, install surface current, no restart required.
  - `ccc setup --dry-run`: no files written.
  - `ccc_recommend_entry`: recommended explicit CCC control-plane entry for review-shaped smoke work.
  - `ccc_status`: exposed `captain_action_contract`, review policy, token visibility, and host subagent state.
  - `ccc_server_identity`: reported `server_version=0.0.6-pre` with install check `status=ok`.
  - Scratch lifecycle: `start`, `status`, `orchestrate`, and `activity` returned readable state for `e026f151-c5ff-fa39-ef8b-ee4a6d60e27f`.
  - Negative JSON path: `ccc status --json '{not valid}'` exited non-zero with an invalid JSON parse error.
  - Release scripts: asset matrix and Windows install smoke passed.
- Notes: live GitHub/download install checks were not rerun in this local smoke pass; use the post-release live section after publishing or when validating the public install path.

## Completed Local Pre-Release Record: 2026-04-28

- Date: 2026-04-28
- Release: `v0.0.6-pre`
- Environment: macOS arm64 source checkout, local cross-build toolchains for release asset generation
- CCC run id: `fa5416be-aa77-295d-0583-065a02aeef20`
- Commands:
  - `cargo fmt --all --check`
  - `cargo test -p ccc --offline`
  - `cargo build --offline`
  - `scripts/release/verify-release-asset-matrix.sh`
  - `scripts/release/verify-windows-install-smoke.sh`
  - `scripts/release/build-release-asset.sh 0.0.6-pre darwin-arm64`
  - `scripts/release/build-release-asset.sh 0.0.6-pre darwin-x86_64`
  - `scripts/release/build-release-asset.sh 0.0.6-pre linux-arm64`
  - `scripts/release/build-release-asset.sh 0.0.6-pre linux-x86_64`
  - `scripts/release/build-release-asset.sh 0.0.6-pre windows-x86_64`
- Expected pass signal: source format/tests/build pass; release scripts pass; packaged assets carry current `0.0.6-pre` docs, skill metadata, and asset-specific manifest platform values.
- Actual result: pass for local source and release packaging checks.
- Evidence:
  - `cargo test -p ccc --offline`: 143 passed, 0 failed.
  - `cargo build --offline`: finished successfully.
  - Release scripts: asset matrix and Windows install smoke passed.
  - Generated assets: `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`, and `windows-x86_64`.
- Notes: the active CCC MCP server used during this release run was still the previously installed `0.0.5-pre` server. Treat live `ccc_server_identity`, public GitHub release lookup, and download/install verification as post-publish checks for the newly installed `0.0.6-pre` bundle.
