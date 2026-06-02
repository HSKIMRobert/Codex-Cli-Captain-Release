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

The grouped inventory below records current `0.0.14` source readiness until
that release is published. Published `0.0.13` evidence remains historical proof;
`0.0.14` smoke and readiness evidence is pre-release evidence only.

| Surface | What it does |
| --- | --- |
| Public entry | `$ccc` is the public Codex operator entry from `0.0.14`; `$cap` is historical compatibility language only. `ccc setup` installs the standalone skill at `~/.codex/skills/ccc/SKILL.md` by default. |
| Workflow/control plane | LongWay planning, checklist/projection, task cards, restart handoff, compact status, app-panel output, validation breadcrumbs, and next-action guidance are CCC-owned persisted surfaces. |
| Specialist routing and fan-in | CCC routes specialist-owned work to configured `ccc_*` roles, records host-subagent lifecycle, requires evidence-backed fan-in, preserves a run-local pre-dispatch snapshot where feasible, and separates reclaim/reassign/fallback/review from host UI spawn/toast wording. |
| Review boundary | Routine verification is Captain-owned; higher-risk release, destructive, security, or operator-requested work escalates to review. |
| Evidence/status/app-panel/check-install | `status`, `activity`, `check-install`, app-panel, capability projection, binary provenance, visibility signatures, install/update parity, PATH shadowing, and release-boundary diagnostics are evidence surfaces, not transcript claims. |
| Operator transport wording | Operator-visible CCC lifecycle mutations use PATH `ccc` shell runs with expected transcript wording `Ran ccc ...`. `Called ...` or MCP tool-call wording belongs only to host-owned spawn/toast lines, app surfaces, structured inspection, or recorded CLI-unavailable fallback. |
| Graph command surface | `ccc graph` is the public CLI keyword for graph queries, and `ccc graph generate` refreshes managed Graphify artifacts. Historical `codegraph` and `graphify generate` CLI names are hidden compatibility aliases only; the MCP tool name remains `ccc_code_graph`. |
| Runtime readiness | Graphify context, native code graph, LSP readiness/bounded operations, hooks, dynamic rules, PostToolUse gates, memory/Tolaria, SSL skill registry, install surfaces, and custom-agent sync are readiness surfaces whose advisory metadata is not runtime truth. Current verified evidence says Graphify refresh is fresh with `release_blocker=false`, native legacy remains separate/fallback-only, and hooks fallback is accepted when assets are absent or inactive. |
| Hook mutability | Dry-run and probe inspection are non-mutating; opt-in `entry_policy.auto_entry.enabled=true` may create a bounded run and must surface `auto_entry_outcome`. |
| Skill/install surfaces | Standalone skill install is the default in `0.0.14`; plugin marketplace/cache is optional compatibility. Setup/check-install install parity, `cap_skill`/`capContinuity` compatibility, `ccc_skill`/`cccContinuity` aliases, custom-agent sync, and legacy cleanup are separate install surfaces. |
| Authority/release gates | Preflight validation, Cargo install, package list, checksums, release assets, publish, public install smoke, generated-artifact hygiene, and companion authority split are separate gates. Local readiness does not imply publish readiness, and preflight does not imply package/tag/upload approval. |
| Component boundaries | CCC is one root package with internal component responsibilities. Current component boundaries are documented in `docs/release-work/0.0.14/COMPONENT_REGISTRY.md`; CCC does not claim separate LazyCodex-style component packages until package-level boundaries exist. |
| Specialist routing gate | Assigned-specialist work that mutates directly from Captain is a routing regression unless `specialist_dispatch_proof` or terminal fallback is recorded for the current task. Current `0.0.14` source has a fail-closed `specialist_dispatch_proof_or_terminal_fallback` merge gate and best-effort run-local pre-dispatch snapshots. |
| `wccc` internal planner | `wccc` is host-internal Captain/Way/LongWay planner context surfaced by `check-install` and status as `wayPlannerSkill`. It is not installed under `~/.codex/skills`, not a visible Codex skill, and not a public command; `$ccc` remains the public entry. Current `PLAN_SEQUENCE` smoke is verified evidence for visibility only. |

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
