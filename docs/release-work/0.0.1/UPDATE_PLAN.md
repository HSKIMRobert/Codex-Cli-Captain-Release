# v0.0.1 Update Plan

## Goal

Stabilize the public `0.0.1` captain loop, align the product naming with `Codex-Cli-Captain`, and rename the public planning terminology so the operator-facing model matches the intended OMO harness.

The target public shape is:

1. operator sends `$cap ...`
2. Codex receives it as `captain`
3. `Way` creates a `LongWay`
4. `captain` reviews the `LongWay`
5. `captain` selects one CCC specialist at a time
6. specialist returns to `captain`
7. `captain` updates the `LongWay`
8. `captain` selects the next specialist or closes the run

## Problems To Fix

### 1. Public `$cap` entry instability

- the public `$cap` path can still hit `user cancelled MCP tool call` on `ccc_start`
- direct MCP calls succeed, so the remaining issue appears to be in the public wrapper flow rather than in the Rust MCP core
- ordinary `$cap` work must not degrade into host-local fallback before the public captain-first entry path has been honestly attempted

### 2. Fallback before reclaim/reassignment

- stalled workers should be reclaimed and explicitly reassigned by `captain`
- host-local fallback should happen only after bounded reclaim and bounded retry/reassignment are exhausted
- the operator should see a truthful captain checkpoint instead of a silent collapse into host-local work

### 3. Public product naming mismatch

- local and remote repository names still use `Codex-Cli-Captain` / `Codex-Cli-Captain-Release`
- internal product/runtime/install strings still expose `ccc`
- the public product naming should move to `Codex-Cli-Captain` / `Codex-Cli-Captain-Release`
- internal visible naming should move to `CCC`

### 4. Planning terminology mismatch

- the old `plan` naming does not match the intended public model
- the old `longway` naming does not match the intended public model
- public planning language should become:
  - `Way` for the planning role or planning checkpoint
  - `LongWay` for the persisted plan/longway artifact

### 5. Release card ownership mismatch

- the source repo should not publish or own the public release card
- the release/install repo should be the only repo that owns the public `0.0.1` release card
- docs and release steps should reflect that only the release repo gets the GitHub release card/body update

## Non-Goals

- changing the public version away from `0.0.1`
- introducing a new multi-version release train
- keeping `ccc_auto_entry` as the required public `$cap` front door
- preserving mixed naming where some public surfaces still say `CCC` while others say `CCC`

## Acceptance

- [ ] ordinary public `$cap` entry does not fail at the first captain-start boundary with `user cancelled MCP tool call`
- [ ] ordinary public `$cap` uses a stable explicit captain-first path by default
- [ ] worker timeout or stalled progress triggers reclaim/reassignment before any host-local fallback
- [ ] captain can explicitly reassign the next specialist within the same run
- [ ] captain can explicitly close the run within the same run boundary
- [ ] role model/profile/reasoning settings still come from `ccc-config.toml`
- [ ] source repo name and release repo name are updated to `Codex-Cli-Captain` and `Codex-Cli-Captain-Release`
- [ ] public/internal visible `ccc` naming is swept to `CCC`
- [ ] `plan` terminology is swept to `Way`
- [ ] `longway` terminology is swept to `LongWay`
- [ ] only `Codex-Cli-Captain-Release` owns the public GitHub release card/body
- [ ] `Codex-Cli-Captain` source docs stop implying that the source repo publishes a public release card
- [ ] docs, release notes, release card, install surface, and packaged skill all use the updated naming
- [ ] local install is refreshed from the updated `0.0.1` release artifact
- [ ] public `$cap` smoke verification succeeds after a fresh Codex CLI restart

## Workstreams

### 1. Captain Entry

- reproduce the public `$cap` cancellation path
- separate wrapper-side cancellation from MCP-side failures
- make explicit captain-first entry the stable default public flow
- keep `ccc_auto_entry` as compatibility/diagnostic-only, not as the normal `$cap` front door

### 2. Captain Loop

- keep the public loop as `captain -> Way -> captain -> specialist -> captain -> ... -> captain end`
- ensure `captain` chooses specialists one at a time based on the current `LongWay`
- ensure the next specialist is never selected directly by the previous specialist
- keep specialist launches bound to the shared role config from `ccc-config.toml`

### 3. Reclaim And Retry

- verify the existing worker kill/reclaim path still works in Rust
- make reclaim visible in captain status/activity output
- keep bounded retry and bounded reassignment inside the same run
- defer host-local fallback until reclaim and bounded retry/reassignment are exhausted

### 4. Naming Migration

- rename local source repo directory and release repo directory
- rename remote GitHub repositories
- sweep visible `ccc` names in:
  - docs
  - install text
  - release notes
  - runtime strings
  - packaged skill text
  - release asset wording
  - MCP/server labels where appropriate
- replace product-facing references with `CCC`

### 5. Terminology Migration

- rename public `plan` terminology to `Way`
- rename public `longway` terminology to `LongWay`
- sweep:
  - docs
  - skill text
  - runtime visibility text
  - release notes
  - schemas and structured payload labels where public/operator-facing
- keep internal semantics coherent so the operator-facing model reads consistently

### 6. Validation And Release

- run targeted Rust tests
- add or update tests for captain reassignment and captain close
- run public `$cap` smoke checks after the naming/flow changes
- update source docs
- update release docs
- update the GitHub release card/body in the release repo only
- rebuild the `0.0.1` release asset
- reinstall locally from the refreshed release

## Order Of Execution

1. fix the public `$cap` entry instability first
2. enforce reclaim/reassignment before fallback
3. finish the captain-loop runtime so captain can reassign and close explicitly
4. rename repos and sweep `ccc` naming to `CCC`
5. sweep `plan -> Way` and `longway -> LongWay`
6. update docs and release notes, and keep the public release card only in the release repo
7. rebuild the `0.0.1` release asset
8. reinstall locally and verify after a fresh Codex CLI restart

## Risks

- repository rename affects release URLs, install docs, and remote automation
- broad terminology replacement can accidentally rename internal-only meanings that should stay stable
- public `$cap` cancellation may still involve host-side Codex behavior outside the Rust MCP binary
- changing public runtime strings without aligned skill updates can create operator confusion
- release ownership can stay ambiguous unless the source docs explicitly say the release card belongs only to the release repo

## Verification

- `cargo test --manifest-path <repo>/Cargo.toml -q`
- `ccc check-install` until the binary/install rename is completed
- post-rename install check using the final `CCC` naming surface
- public `$cap` smoke run in a tiny workspace
- public `$cap` smoke run in a real repo workspace
- confirm captain-visible:
  - entry
  - Way creation
  - LongWay updates
  - specialist spawn
  - reclaim/retry
  - captain end
