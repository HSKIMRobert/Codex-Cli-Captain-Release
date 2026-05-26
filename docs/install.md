# Install, Update, and Uninstall

This guide matches the current public install flow.

## Install

```bash
cargo install codex-cli-captain
ccc setup
```

Then fully exit Codex CLI, start a new session, and run:

```bash
ccc check-install
```

## Update

For an existing Cargo install:

```bash
cargo install codex-cli-captain --force
ccc setup
```

Then restart Codex CLI and run `ccc check-install`.

## Uninstall

To remove the Cargo install:

```bash
cargo uninstall codex-cli-captain
```

If you also want CCC-managed cleanup, run `ccc uninstall --dry-run` first, then `ccc uninstall --confirm` only after reviewing the preview.

## Config

Edit `~/.config/ccc/ccc-config.toml` to change CCC role models, reasoning tier, and fast-mode settings. After editing, run:

```bash
ccc setup
```

Then restart Codex CLI and run `ccc check-install`.

Codex plugin hooks are opt-in. If you enable them, edit `~/.codex/config.toml`, set `[features] plugin_hooks = true`, restart Codex CLI, review `/hooks`, approve the CCC hooks, and run `ccc check-install`.

## Legacy Fallback

The pinned `v0.0.11` release-bundle fallback remains available for environments that intentionally use the bundled installer.

```bash
curl -fsSL https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.sh | bash
```

Windows PowerShell:

```powershell
iwr -UseB https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.ps1 | iex
```

## Platform Note

CCC targets macOS, Windows, and Linux, but Windows and Linux may not work normally in some environments. Always verify with `ccc check-install` after installation or update.
