# v0.0.1 Rust Reset Plan

## Goal

Reset `codex-foreman` and `codex-foreman-release` to a fresh `0.0.1` baseline with a Rust-only tracked codebase, a Rust-only release surface, and rewritten remote history.

The operator-facing target is simple:

- no tracked TypeScript source remains in `codex-foreman`
- no tracked Node/TS release surface remains in `codex-foreman-release`
- the new `0.0.1` build can be installed and exercised locally
- both remotes are republished from fresh history starting at `0.0.1`

## Non-Goals

- preserving old package/version history
- keeping npm as the required public installer
- preserving old tags, release cards, or commit graph
- keeping preview-only Rust/TS hybrid messaging in docs

## Acceptance

- [x] `codex-foreman` tracked source is Rust-only
- [x] `codex-foreman` builds with `cargo build`
- [x] `codex-foreman` tests pass with `cargo test`
- [x] the Rust CLI exposes the install/runtime commands needed to test `0.0.1`
- [x] the worker launch path no longer stalls at `Reading additional input from stdin...` during the bounded orchestration flow
- [x] `codex-foreman-release` is regenerated as a Rust-only release/install repo
- [x] local `0.0.1` install and smoke verification succeeds
- [ ] both remotes are rewritten to fresh history
- [ ] old tags and release cards are removed
- [ ] new tags/releases start at `0.0.1`

## Workstreams

### 1. Product Boundary

- [x] make the Rust runtime the only tracked implementation
- [x] remove tracked `src/*.ts`, `tests/*.ts`, `scripts/*.cjs`, `dist/*`, and npm/package metadata that only support the old Node surface
- [x] decide the new public install contract: release assets plus Rust CLI, not npm tarball as the primary surface

### 2. Rust Runtime Parity

- [x] keep `foreman_start`, `foreman_run`, `foreman_orchestrate`, and `foreman_status` working
- [x] port the minimum install/runtime diagnostics needed for `setup` and `check-install`
- [x] port public-entry install assets for `$cap`
- [x] fix the delegated worker launch contract so `codex exec` produces bounded results instead of hanging on stdin
- [x] switch the shared Foreman config surface to `foreman-config.toml` with legacy JSON migration fallback
- [x] apply shared role model/profile/reasoning config to delegated `codex exec` launches
- [x] remove the ordinary manual `codex_bin` requirement by resolving the effective Codex binary from run state or local PATH
- [x] restore Rust status/activity visibility for fan-in readiness, active delegations, and best-effort token usage

### 3. Release Surface

- [x] regenerate `codex-foreman-release` from the Rust-only product
- [x] remove stale public claims about packaged TS/JS surfaces
- [x] reset version strings, release notes, and install docs to `0.0.1`
- [x] verify the release repo no longer exposes internal-only material or old packaging artifacts

### 4. Validation

- [x] `cargo build`
- [x] `cargo test`
- [x] local CLI setup/check-install verification
- [x] local bounded run verification
- [x] local release artifact verification

### 5. Remote Reset

- [ ] commit the Rust-only `codex-foreman` baseline
- [ ] rewrite `codex-foreman-release` to a fresh single-history baseline
- [ ] rewrite `codex-foreman` to a fresh single-history baseline if needed for the reset
- [ ] delete old remote tags and release cards
- [ ] create fresh `0.0.1` tags/releases on both remotes

## Order Of Execution

1. Stabilize the current Rust runtime and fix the worker launch stall.
2. Add the Rust CLI/install surfaces needed to replace the old Node entrypoints.
3. Delete the tracked TS/Node implementation surface.
4. Convert docs and release generation to the new Rust-only contract.
5. Validate locally until the new `0.0.1` is installable and testable.
6. Rewrite both remotes and republish `0.0.1`.

## Progress Log

- [x] plan rewritten for the Rust-only `0.0.1` reset
- [x] local Rust-only reset implementation completed
- [x] post-reset runtime/docs polish added for TOML config, delegated model routing, and richer Rust status/activity visibility
- [ ] remote reset still pending
