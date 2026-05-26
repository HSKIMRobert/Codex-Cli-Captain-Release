# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Run Codex CLI work end-to-end with one command prefix.<br>
Add <code>$cap</code> before the task you want CCC to manage.</em></p>

CCC is a Rust-based orchestration layer built for Codex CLI. It keeps `$cap` as the public entrypoint, manages task flow behind the scenes, and helps you carry larger tasks through with fewer manual handoffs.

## When To Use It

Use CCC when you want Codex to handle a task from start to finish. It is a good fit for work that needs planning, edits, review, or a restart-safe handoff, especially when you want to keep the flow in one place instead of juggling steps manually.

## Install, Update, Uninstall

Primary install path:

```text
cargo install codex-cli-captain
ccc setup
```

Then fully exit Codex CLI, start a new session, and run:

```text
ccc check-install
```

To update an existing Cargo install:

```text
cargo install codex-cli-captain --force
ccc setup
```

Then restart Codex CLI and run `ccc check-install`.

To uninstall the Cargo install:

```text
cargo uninstall codex-cli-captain
```

If you also want CCC-managed cleanup, run `ccc uninstall --dry-run` first, then `ccc uninstall --confirm` only after reviewing the preview.

Legacy release-bundle fallback only:

```text
curl -fsSL https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.sh | bash
```

Windows PowerShell:

```text
iwr -UseB https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.ps1 | iex
```

## Other Settings

| Setting | Location | Notes |
| --- | --- | --- |
| CCC role model, reasoning tier, fast-mode | `~/.config/ccc/ccc-config.toml` | Edit the file, then run `ccc setup`, restart Codex CLI, and run `ccc check-install`. |
| Codex plugin hooks | `~/.codex/config.toml` | Set `[features] plugin_hooks = true`, restart Codex CLI, review `/hooks`, approve the CCC hooks, and then run `ccc check-install`. |

## Supported OS

| OS | Status | Caveat |
| --- | --- | --- |
| macOS | Supported | Verify with `ccc check-install` after installation. |
| Windows | Supported in principle | Some environments may not work normally, so verify after installation. |
| Linux | Supported in principle | Some environments may not work normally, so verify after installation. |
