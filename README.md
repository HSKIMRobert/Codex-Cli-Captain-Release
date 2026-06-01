# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>One public entrypoint for Codex CLI work that needs a plan, execution, fan-in, review, and proof.</em></p>

CCC is the operator-facing control plane for Codex CLI. Use `$ccc` for public
workflow and CCC keeps the LongWay plan, task cards, specialist routing,
fan-in, review boundary, status projection, and install verification on
CCC-owned evidence surfaces.

Current public release: `0.0.13`.

## What CCC Gives You

| Surface | What it does |
| --- | --- |
| `$ccc` entry | The public workflow prefix for work that should go through CCC instead of ad hoc host transcript state. |
| LongWay planning | Persistent plan/checklist state for multi-step work, restart handoff, and follow-up continuity. |
| Specialist routing | CCC routes to configured `ccc_*` roles and records compact fan-in instead of relying on generic transcript text. |
| Review boundary | Routine verification is Captain-owned; higher-risk release, destructive, security, or operator-requested work escalates to review. |
| Status proof | `ccc status --text`, app-panel output, checklist/projection, and `ccc check-install --text` expose current evidence and stale boundaries. |
| Tool readiness | Graphify context, LSP readiness, hooks, memory, skill registry, and install surfaces are reported without turning advisory metadata into runtime truth. |

## Install With An AI Agent

Paste this block to Codex CLI, ChatGPT, or another AI agent that can run shell
commands on your machine. The point is not just to install the binary; the
agent should inspect the current state, install or update, run setup, restart
or ask you to restart Codex CLI, and verify the result.

```text
Install or update Codex-Cli-Captain 0.0.13 for Codex CLI.

Please do this carefully:

1. Inspect the current state first:
   - run `command -v ccc || true`
   - if `ccc` exists, run `ccc --version` and `ccc check-install --text`
   - report whether the existing binary is current, shadowed, stale, or missing

2. Install or update from Cargo:
   - run `cargo install codex-cli-captain --force`
   - run `ccc setup`

3. Tell me to fully exit Codex CLI and start a new session, or reload the
   Codex CLI cache if this host supports that safely.

4. After restart, verify:
   - run `command -v ccc`
   - run `ccc --version`
   - run `ccc check-install --text`
   - run `ccc status --text`
   - run `ccc status --app-panel --text`

5. Summarize the result:
   - version installed
   - whether the `$ccc` entry skill is current
   - whether MCP registration matches
   - whether custom agents are synced
   - whether restart or cache reload is still required
   - any PATH shadowing, stale plugin cache, or legacy bundle warning
```

For a direct manual install:

```bash
cargo install codex-cli-captain --force
ccc setup
```

Then fully restart Codex CLI before verifying with:

```bash
ccc check-install --text
ccc status --text
ccc status --app-panel --text
```

## Use It

Start a CCC-managed task by using `$ccc`:

```text
$ccc update the release docs, verify the current codebase evidence first, then report what changed
```

For follow-ups, keep using `$ccc` or the continuation hint printed by CCC. CCC
stores the LongWay/checklist/fan-in state under the workspace `.ccc` runtime
artifacts and renders the current state through `ccc status` and app-panel
views.

If your host still exposes a legacy skill entry, that entry may continue to
work as a compatibility path.

## Release Advisories

- macOS cross-asset provenance execution can hang if a release script tries to
  execute non-macOS binaries. `v0.0.13` used a temporary wrapper and Zig linker
  settings for cross assets.
- Crates.io-installed binaries can report `source_commit=unknown` / partial
  binary provenance because Cargo rebuilds from the published crate.
- Local release-work files such as tarballs or checksums can remain in this
  release repo checkout after upload. They are not source proof unless
  intentionally committed.

## Compatibility Notes

| Area | Status |
| --- | --- |
| macOS | Supported and verified for the local maintainer smoke path. |
| Linux | Release assets are published; verify with `ccc check-install --text` after install in your environment. |
| Windows | Release asset is published; verify with `ccc check-install --text` after install in your environment. |
| Codex plugin hooks | Optional. If enabled, restart Codex CLI and verify with `ccc check-install --text`. |

## Update

```bash
cargo install codex-cli-captain --force
ccc setup
```

Restart Codex CLI, then run `ccc check-install --text`.

## Uninstall

```bash
ccc uninstall --dry-run
ccc uninstall --confirm
cargo uninstall codex-cli-captain
```

Run the dry-run first and review the paths. `cargo uninstall` removes only the
Cargo binary; `ccc uninstall --confirm` removes CCC-managed Codex surfaces while
preserving unrelated Codex data.

## Public Surface Boundary

`$ccc` is the public user entrypoint. Internal role names, fan-in artifacts,
Graphify details, LSP state, hooks, and memory readiness are proof surfaces for
CCC itself; they should not be treated as host-owned transcript claims. Host
spawn/toast labels are rendered by the host and can differ from CCC status.
