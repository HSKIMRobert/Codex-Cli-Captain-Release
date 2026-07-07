# Install, Update, and Uninstall

This guide is bounded to the published `v0.0.28` crates.io release. The
crates.io package was published after visible WAVE lifecycle repair,
ODYSSEY/graph-card visibility, global rule/LSP/compact discipline, fast-mode
role config, installed lifecycle readiness closure, and post-publish
verification passed after a registry install. The release card is published in
the repository README files.

Scope note: `v0.0.28` preserves the App-native role/team-mode path while
making the WAVE lifecycle and ODYSSEY planning proof visible to operators. The
verified release covers visible WAVE LAUNCH/ANCHOR, Scout-backed
graph-card preflight, ODYSSEY lane rationale and host update_plan progress,
global rule consumption, LSP safety, compact rehydration, configurable role
fast-mode, Scribe/Raider/Ghost/Arbiter proof binding, Stop closeout, and
`install_runtime_ready=true` with release-blocking gaps at zero.

## Install

```bash
cargo install codex-cli-captain --version 0.0.28 --force
ccc setup --with-plugin --recreate-config
```

Then fully exit Codex CLI, start a new session, and run:

```bash
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

`ccc setup --with-plugin --recreate-config` refreshes the CCC MCP registration,
custom agents, plugin marketplace/cache, hook assets, and config. If
`codex plugin list` does not show `ccc@ccc-dev` installed and enabled, run
`ccc setup --with-plugin --recreate-config` again before relying on plugin hooks.

Some lifecycle proof surfaces, such as plugin hook visibility, graph-card
planning context, and LSP safety loop evidence, are observed during an explicit
`$ccc` task rather than at install time. After setup and restart, run one small
explicit `$ccc` request if those surfaces still report missing, then rerun
`ccc check-install --text` and `ccc status --text`.

## Update

For an existing Cargo install:

```bash
cargo install codex-cli-captain --version 0.0.28 --force
ccc setup --with-plugin --recreate-config
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
ccc setup --with-plugin --recreate-config
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
