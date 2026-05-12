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

Current public release: `0.0.15-pre`.

CCC is a captain-first orchestration layer for Codex CLI. It keeps `$cap` as the only public entrypoint, persists LongWay/task-card/fan-in state, and routes specialist work through configured `ccc_*` agents before captain review.

## Install Or Update

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

To update, run the same install command again, fully restart Codex CLI, then run `ccc check-install`. The installer stages the new bundle before switching the active path, preserves previous release bundles for rollback, refreshes CCC-managed plugin and `$cap` files, and only cleans CCC-managed stale cache/version entries plus the legacy packaged cap copy. Non-CCC Codex config is preserved.

Release installers are pinned to `v0.0.15-pre` by default. Set `CCC_VERSION` only when you intentionally want a different release.

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

## Recommended Role Defaults

For regular CCC use, ChatGPT Pro $100 is the recommended starting plan because `$cap` workflows can spend more Codex usage through repeated captain and specialist handoffs.

| CCC role | Stable agent ID | Display callsign | Recommended model | Reasoning | Notes |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | Host-owned routing label, not a managed `ccc_*` specialist |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | Planning and bounded next-move selection |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | Read-only repo evidence |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | Code/config mutation and repair |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README and operator text |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | Captain-mediated review, risk, regression, and acceptance checks |
| `sentinel` | `ccc_sentinel` | `Overseer` | `gpt-5.4-mini` | `high` | Run-scoped guardrail classification and status visibility |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | Low-cost filesystem/docs/web/git/gh read work |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | Low-cost bounded git/gh mutation and narrow tool work |

Stable `ccc_*` IDs are routing truth. StarCraft callsigns are display-only.
