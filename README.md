# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Want to do end-to-end work in Codex CLI?<br>
Worried about running the whole thing on a high-capability model?<br>
Then give CCC a try.<br>
Just put <code>$cap</code> in front of what you want to do.<br>
Watch the magic happen.</em></p>

Current public release: `0.0.11-pre`.

<table>
<tr>
<td>

<strong>CAUTION - PRE-RELEASE DEFAULT PATH</strong><br><br>
<code>0.0.11-pre</code> is the current public pre-release default. macOS is the officially supported path, with macOS arm64 locally tested. Linux and Windows assets are provided for early testing and are expected to use the same <code>ccc setup</code> / <code>ccc check-install</code> flow, but they have not yet been fully verified on real Linux or Windows environments. If needed, install the previous pre-release with <code>CCC_VERSION=v0.0.10-pre</code>.

</td>
</tr>
</table>

## 0.0.11-pre Release Card

`0.0.11-pre` is a walking skeleton plus follow-up hardening release, not a full parity or rebuild-complete release. It makes `$cap` the public CCC entrypoint while keeping host `/plan` and `/goal` as optional affordances; `$cap` works without either host surface.

This release documents the CCC operator flow around PLAN_SEQUENCE and EXECUTE_SEQUENCE, pending LongWay approval before execution, checklist/status/fan-in as the operational truth, and restart handoff for continuing work from persisted state.

Validation summary:

- Source repo PR #2 through PR #7 are merged, ending with release-facing source docs commit `a92d25f2f73d5be06c96a150a0016d1cc48d3cbc`.
- `cargo test -p ccc` passed in the final implementation slices.
- Release-facing source docs are merged before this public release repo documentation update.

Deferred from `0.0.11-pre`:

- Loop 5 Intervention / Review-retry-replan-reclaim parity projection.
- Schema contract promotion.
- `setup_config` config surface change.
- Full runtime parity projection.

## Install & Update

Copy this into Codex CLI on macOS or Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

On Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

For updates, run the same install command again. It downloads the current release asset, updates the current install pointer or Windows install directory plus `ccc` command shim, refreshes setup, and reruns `ccc check-install`; fully restart Codex CLI afterward so the refreshed `$cap` skill and custom agents are loaded.

## Reapply Config Changes

After editing `~/.config/ccc/ccc-config.toml`, paste this into Codex CLI.  Fresh installs generate `~/.config/ccc/ccc-config.toml` with `explorer` set to reasoning `variant = "high"` and `documenter`, `companion_reader`, and `companion_operator` set to `variant = "medium"`; all keep `fast_mode = true`. `ccc setup` preserves any existing user-customized values while backfilling missing generated defaults or upgrading stale CCC-generated defaults.

Generated defaults keep the captain reasoning at medium by default while making handoffs cheaper: configured mini/specialist roles stay on the faster service path, copied task details are shortened before they are sent to delegated workers, and the refreshed `$cap` skill plus custom-agent instructions stay compact while the active-specialist drift guardrail keeps stale output from silently overriding the captain's chosen path.

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

## Recommended Role Defaults

For regular CCC use, ChatGPT Pro $100 is the recommended starting plan because `$cap` workflows can spend more Codex usage through repeated captain and specialist handoffs. Adjust reasoning by your working style and task risk: keep higher reasoning for broad planning, risky code changes, or reviews, and lower it for narrow, repetitive, or low-risk tasks.

| CCC role | Agent | Recommended model | Reasoning | Notes |
| --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `gpt-5.5` | `medium` | LongWay ownership and final routing judgment |
| `way` | `tactician` | `gpt-5.5` | `high` | Planning and bounded next-move selection |
| `explorer` | `scout` | `gpt-5.4-mini` | `high` | Read-only repo evidence |
| `code specialist` | `raider` | `gpt-5.5` | `high` | Code/config mutation and repair |
| `documenter` | `scribe` | `gpt-5.4-mini` | `medium` | README, release notes, and operator text |
| `verifier` | `arbiter` | `gpt-5.5` | `high` | Review, risk, regression, and acceptance checks |
| `companion_reader` | `companion_reader` | `gpt-5.4-mini` | `medium` | Low-cost filesystem/docs/web/git/gh read work |
| `companion_operator` | `companion_operator` | `gpt-5.4-mini` | `medium` | Low-cost bounded git/gh mutation and narrow tool work |

`gpt-5.5` is recommended for the high-value roles when Codex is signed in with ChatGPT. If it is unavailable in the current account or execution path, use `gpt-5.4` for those roles until rollout reaches that path.

When a subagent result is unsatisfactory or needs-work, CCC canonicalizes it into a bounded specialist follow-up and prefers repair or reassignment through a specialist instead of local captain repair whenever that route is available.
