# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Want to run Codex CLI or Codex App end-to-end?<br>
Worried about running the whole thing on high-end models?<br>
Then how about using CCC?<br>
Just put <code>$cap</code> in front of what you want to do.<br>
Then something remarkable can unfold.</em></p>

Current release: `0.0.7`.

CCC is a captain-first orchestration layer for Codex CLI. It keeps `$cap` as the only public entrypoint, persists LongWay/task-card/fan-in state, and routes specialist work through managed agents before captain review.

## Install, Update, Uninstall

Primary install path:

```text
Install Codex-Cli-Captain with Cargo:
cargo install codex-cli-captain

After installation finishes, run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

To update an existing Cargo install, rerun:

```text
cargo install codex-cli-captain --force
ccc setup
```

Then fully restart Codex CLI and run `ccc check-install`.

If a previous release-bundle install left `~/.local/bin/ccc` earlier than
`~/.cargo/bin` in `PATH`, your shell can keep running the legacy binary after
Cargo install. `ccc setup` and `ccc check-install` report the shell-resolved
`ccc`, the Cargo candidate at `~/.cargo/bin/ccc`, the current executable, and
whether `~/.local/bin/ccc` is shadowing Cargo. `ccc check-install` also
reports hooks readiness, the selected runtime path, and the public config
shape: top-level `version`, `entry_policy.mode`, visible agent entries
(`name`/`model`/`variant`/`fast_mode`), `agents.ghost`, and visible LSP plus
`graph_context`/Graphify defaults. When shadowing is reported, run
`~/.cargo/bin/ccc setup`, put `~/.cargo/bin` earlier in `PATH`, or remove or
demote the legacy `~/.local/bin/ccc` after reviewing cleanup. Release-bundle
rollback paths under `~/.local/share/ccc` stay preserved by default unless you
explicitly prune them.

To uninstall the Cargo-installed binary:

```text
cargo uninstall codex-cli-captain
```

If you also want CCC-managed cleanup, run `ccc uninstall --dry-run` first to review the plan, then `ccc uninstall --confirm` only if the preview is correct. Use `ccc check-install` before removing MCP registration, `ccc-config.toml`, skills, or custom agents.

Windows PowerShell uses the same primary Cargo path.

Legacy release-bundle fallback only:

macOS or Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

To update, rerun `cargo install codex-cli-captain --force` and then `ccc setup`, fully restart Codex CLI, and run `ccc check-install`. Use the legacy release-bundle installer only when you intentionally want the packaged `install.sh`/`install.ps1` fallback. The bundle installer stages the new bundle before switching the active path, preserves previous release bundles for rollback, refreshes CCC-managed plugin and `$cap` files, and only cleans CCC-managed stale cache/version entries plus the legacy packaged cap copy. Non-CCC Codex config is preserved. `cargo publish` is maintainer-only release work that needs the release token and approval outside this end-user README.

Release-bundle fallback installers target `v0.0.7` and are published with
matching validated bundle assets. Cargo remains the primary `0.0.7` install
path. Set `CCC_VERSION` only when you intentionally want a different
release-bundle fallback.

## Basic Use

Start a CCC-managed task by adding `$cap` before your request:

```text
$cap Refactor the auth flow and keep tests passing
```

CCC owns persisted LongWay, task cards, checklist/projection, fan-in, status, and restart handoff. Host `/plan`, `/goal`, and graph commands are not CCC public entrypoints.

## Reapply Config Changes

Edit `~/.config/ccc/ccc-config.toml` to change each CCC role's model, reasoning tier, and fast-mode setting. After editing, paste this into Codex CLI:

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`ccc setup` preserves user-customized values while backfilling missing generated defaults and refreshing MCP registration, the packaged `$cap` skill, and CCC-managed custom agents.

Codex plugin hooks are opt-in. CCC does not silently enable the under-development `plugin_hooks` feature for you, and hook commands require a `/hooks` review in Codex CLI before they can run. To turn them on:

1. edit `~/.codex/config.toml`
2. set `[features] plugin_hooks = true`
3. restart Codex CLI
4. open `/hooks` and approve the five CCC hooks: `PermissionRequest`, `PostToolUse`, `SessionStart`, `UserPromptSubmit`, and `Stop`
5. run `ccc check-install`

If you intentionally want to hide the unstable-feature warning after opting in, you can also set `suppress_unstable_features_warning = true` at the top level. That suppresses only the warning; it does not skip hook review or change hook behavior. The warning appears again on each fresh Codex CLI startup when the global config still has `plugin_hooks = true`. When hooks are not enabled or have not been reviewed, `ccc check-install` stays on the `ccc_fallback` path instead of reporting `runtime=hooks_first`.

A Stop-hook warning after a turn is normal when the CCC Stop hook returns `status=clear`. Codex may label that message as a warning even though CCC did not block or escalate.

`ccc setup` only installs or refreshes hook assets when the target Codex
surface can load them safely. If hooks are unavailable, disabled, untrusted, or
unsupported, CCC keeps the CLI/MCP/status/fan-in fallback active and visible.

## Hooks and Readiness

`ccc check-install` reports whether hooks are available and whether CCC is
using hooks-first or the CCC fallback path for the current session. If you
completed the opt-in steps above, expect `runtime=hooks_first`; otherwise
expect `runtime=ccc_fallback`. Use that output together with the
install-health and restart guidance to confirm the runtime path without
exposing internal routing details.

## Public Behavior

- `$cap` is the public entrypoint.
- CCC uses managed roles for planning, mutation, review, and fan-in behind the
  scenes. Internal routing details stay in runtime state and release-work docs,
  not in the public README.
- `ccc setup` refreshes the packaged `$cap` skill, MCP registration, plugin
  cache, and managed agents from the current binary and `ccc-config.toml`, and
  only refreshes hook assets when the installed Codex surface can load them
  safely.
- `ccc check-install` reports the active binary, Cargo candidate, plugin/cache
  discovery, hooks readiness, public config shape, packaged `$cap` skill,
  stale paths, selected runtime path, and whether a restart is still required.
