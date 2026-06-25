# Install, Update, and Uninstall

This guide is bounded to the published `v0.0.20` crates.io release. The
crates.io package was published after installed-local pre-publish gates, and
post-publish verification passed after a registry install. The canonical
release repo tag, downloadable asset bundle, and GitHub Release card remain
separate follow-up publication steps.

Scope note: `v0.0.20` keeps the durable host-hook teammode proof surfaces from
`v0.0.19` and repairs the installed App lifecycle path used by natural `$ccc`
runs. The verified release covers WAVE/ODYSSEY persistence, graph-card and
compact projection, host update_plan mirror/ACK, PostToolUse hooks,
App-native Worker and Arbiter spawn/fan-in/close, proof-command drift
protection, LSP bridge/safety, team registry readiness, Stop closeout, and
`CCC WAVE DEACTIVE`.

## Install

```bash
cargo install codex-cli-captain
ccc setup --with-plugin
```

Then fully exit Codex CLI, start a new session, and run:

```bash
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

`ccc setup --with-plugin` refreshes the CCC MCP registration, custom agents,
plugin marketplace/cache, and hook assets. If `codex plugin list` does not show
`ccc@ccc-dev` installed and enabled, run `ccc setup --with-plugin` again before
relying on plugin hooks.

Some lifecycle proof surfaces, such as plugin hook visibility, graph-card
planning context, and LSP safety loop evidence, are observed during an explicit
`$ccc` task rather than at install time. After setup and restart, run one small
explicit `$ccc` request if those surfaces still report missing, then rerun
`ccc check-install --text` and `ccc status --text`.

## Update

For an existing Cargo install:

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin
```

Then restart Codex CLI and run `ccc check-install --text`.

## Uninstall

To remove the Cargo install:

```bash
cargo uninstall codex-cli-captain
```

If you also want CCC-managed cleanup, review Codex config first and remove local hook, MCP, or skill files only when you can identify them as CCC-managed.

## Config

Edit `~/.config/ccc/ccc-config.toml` to change CCC role models, reasoning tier, and fast-mode settings. After editing, run:

```bash
ccc setup --with-plugin
```

Then restart Codex CLI and run `ccc check-install --text`.

Codex plugin hooks are opt-in. If you enable them, edit `~/.codex/config.toml`, set `[features] plugin_hooks = true`, restart Codex CLI, review `/hooks`, approve the CCC hooks, and run `ccc check-install --text`.

## Legacy Fallback

The pinned `v0.0.12` release-bundle fallback remains available for environments that intentionally use the bundled installer.

```bash
curl -fsSL https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.12/install.sh | bash
```

Windows PowerShell:

```powershell
iwr -UseB https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.12/install.ps1 | iex
```

## Platform Note

CCC targets macOS, Windows, and Linux, but Windows and Linux may not work normally in some environments. Always verify with `ccc check-install --text` after installation or update.
