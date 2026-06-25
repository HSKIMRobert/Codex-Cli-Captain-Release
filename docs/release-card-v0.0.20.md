# Codex-Cli-Captain v0.0.20

`codex-cli-captain v0.0.20` is published on crates.io.

## Install

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin
```

After setup, fully restart Codex CLI / Codex App, then verify:

```bash
command -v ccc
ccc --version
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

## Release Scope

`v0.0.20` keeps the durable host-hook teammode proof surfaces from `v0.0.19`
and repairs the installed App lifecycle path used by natural `$ccc` runs.

Verified coverage includes:

- WAVE and ODYSSEY persistence
- graph-card and compact projection
- host update_plan mirror/ACK
- PostToolUse hooks
- App-native Worker and Arbiter spawn/fan-in/close
- teammode readiness with `receipt_only_teammode=false`
- proof-command drift protection
- LSP bridge and safety loop
- team registry readiness
- Stop closeout with `CCC WAVE DEACTIVE`

## Verification

The release was published after installed-local pre-publish gates. A
post-publish registry install then verified the published crate path:

- `cargo install codex-cli-captain --version 0.0.20 --force`
- `ccc setup --with-plugin`
- Codex App / Codex CLI restart
- natural `$ccc` lifecycle smoke
- `ccc status --text`
- `ccc check-install --text`
- `git diff --check`

Final post-publish readiness evidence showed:

- `install_runtime_ready=true`
- `install_release_blocking_gaps=0`
- `workflow_parity_gaps=0`
- `captain_direct_work_drift=false`
- `receipt_only_teammode=false`
- `team_registry_ready=true`
- `CCC WAVE DEACTIVE`

## Publication Boundary

The crates.io package is published. The canonical release repo tag,
downloadable asset bundle, and GitHub Release card publication remain separate
operator-controlled steps unless they are explicitly performed.
