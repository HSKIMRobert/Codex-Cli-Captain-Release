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

This release installs CCC through a local Codex plugin marketplace. The bundle includes the CCC plugin manifest, `.mcp.json`, and plugin-provided `$cap` skill; the installer enables the `ccc@ccc-local` plugin and removes the legacy direct `mcp_servers.ccc` registration plus any standalone `~/.codex/skills/cap` copy. The public operator entrypoint remains `$cap`.

Current public release: `0.0.15-pre`.

Supported release targets are exactly `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`, and `windows-x86_64`. macOS targets are normally supported and expected to work. Linux and Windows targets are available, but may still have platform-specific issues.

## Install

macOS or Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

To update, run the same install command again, restart Codex CLI, then run `codex mcp list`. The installer refreshes the local `ccc-local` marketplace, enables `plugins."ccc@ccc-local"`, and removes the legacy direct `mcp_servers.ccc` block and standalone `$cap` skill so CCC is loaded through the plugin.

Optional Rust LSP support is useful when working on CCC source or Rust-heavy repos:

```bash
rustup component add rust-analyzer
```

Stable `ccc_*` IDs remain the routing contract; callsigns are display-only. `ccc_tactician` is Executor, `ccc_scout` is Observer, `ccc_raider` is Marauder, `ccc_scribe` is Adjutant, `ccc_arbiter` is Arbiter, `ccc_sentinel` is Overseer, `ccc_companion_reader` is Probe, and `ccc_companion_operator` is SCV. The 0.0.15-pre metadata also advertises the oh-my-openagent-inspired workflow set: `github-triage`, `hyperplan`, `work-with-pr`, `pre-publish-review`, `git-master`, `review-work`, `remove-deadcode`, `get-unpublished-changes`, `ai-slop-remover`, and `rust-analyzer-lsp`.

Host UI layers may still emit outer notifications such as `Closed Carver [ccc_scout]`; that wording is host-managed and not guaranteed by CCC. CCC-controlled status/projection output uses callsign-plus-stable-ID forms such as `Observer(ccc_scout)`.

## Recommended Role Defaults

For regular CCC use, ChatGPT Pro $100 is the recommended starting plan because `$cap` workflows can spend more Codex usage through repeated captain and specialist handoffs. Adjust reasoning by your working style and task risk: keep higher reasoning for broad planning, risky code changes, or reviews, and lower it for narrow, repetitive, or low-risk tasks.

| CCC role | Stable agent ID | Display callsign | Recommended model | Reasoning | Notes |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | Host-owned routing label, not a managed `ccc_*` specialist |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | Planning and bounded next-move selection |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | Read-only repo evidence |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | Code/config mutation and repair |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README, release notes, and operator text |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | Review, risk, regression, and acceptance checks |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | Low-cost filesystem/docs/web/git/gh read work |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | Low-cost bounded git/gh mutation and narrow tool work |

`gpt-5.5` is recommended for the high-value roles when Codex is signed in with ChatGPT. If it is unavailable in the current account or execution path, use `gpt-5.4` for those roles until rollout reaches that path.
