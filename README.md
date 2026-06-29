# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Plan, delegate, verify, and finish Codex CLI work through one public entrypoint.</em></p>

CCC is a small control plane for Codex CLI. Use `$ccc` when a task needs more
than a quick answer: planning, ordered work, specialist help, review, or proof
that the result is really done.

Current crates.io release: `v0.0.24`. The crates.io package was published after
App-native role/team-mode orchestration hardening, installed readiness gap
closure, and post-publish verification passed after a registry install.

## Release Card

| Item | Status |
| --- | --- |
| Version | `v0.0.24` |
| crates.io | `codex-cli-captain 0.0.24` published |
| Git tag | `v0.0.24` |
| GitHub Release | [`v0.0.24`](https://github.com/HoRi0506/Codex-Cli-Captain/releases/tag/v0.0.24) |
| Release commit | `201cff4` |
| Published install | `cargo install codex-cli-captain --version 0.0.24 --force` succeeded |
| Installed verification | `ccc --version` -> `0.0.24` |
| Natural-command E2E smoke | Short natural role/team requests completed WAVE, graph-card/Scout preflight, ODYSSEY lane queue, host `update_plan` ACK, App-native role lifecycle receipts, durable fan-in, LSP safety, Arbiter reverify CLEAR, Stop closeout, `CCC WAVE DEACTIVE`, and final `ccc status/check-install` green gates. |
| Final gates | `install_runtime_ready=true`, `install_release_blocking_gaps=0`, `workflow_parity_gaps=0` |

Highlights:

- App-native role/team-mode orchestration was hardened for Scout, Scribe,
  Raider, Ghost, and Arbiter proof binding.
- Durable fan-in, Arbiter reverify, Stop closeout, and `CCC WAVE DEACTIVE`
  projection now agree on the same run and lane evidence.
- Installed readiness gaps for host native hook dispatch, App-native spawn
  bridge, rule consumption, and LSP search/safety proof were closed.
- README/install documentation target coverage now includes localized README
  verification and install guide proof binding.
- Stale, wrong-run, wrong-role, fake lifecycle, missing fan-in, missing Stop
  closeout, and public mutation bypasses remain fail-closed.
- Final preflight passed: `cargo fmt --check`, `git diff --check`,
  `cargo check`, `fresh_runtime_architecture` 467/467, `cargo package`,
  `cargo publish --dry-run`, published install, and installed
  `ccc status/check-install` green gates.

## Install

For a first-time install or a safe update on a new PC:

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin --recreate-config
```

Verify the Codex MCP/plugin registration, then fully restart Codex CLI /
Codex App when `ccc setup --with-plugin --recreate-config` or
`ccc check-install` asks for it:

```bash
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
```

After restart, verify again:

```bash
command -v ccc
ccc --version
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

If you want an AI agent to handle the install, paste this:

```text
Install or update Codex-Cli-Captain for Codex CLI.
Use the published `v0.0.24` release from crates.io, then refresh the Codex CLI
integration with plugin support.

1. Check the current state with:
   - command -v ccc || true
   - ccc --version, if ccc exists
   - ccc check-install --text, if ccc exists

2. Install or update:
   - cargo install codex-cli-captain --force
   - ccc setup --with-plugin --recreate-config

3. Verify before restart:
   - codex mcp list
   - codex plugin marketplace list
   - codex plugin list
   - ccc check-install --text

4. Ask me to fully restart Codex CLI / Codex App if the check asks for it.

5. After restart, verify:
   - command -v ccc
   - ccc --version
   - codex mcp list
   - codex plugin marketplace list
   - codex plugin list
   - ccc check-install --text
   - ccc status --text

6. If `codex plugin list` does not show `ccc@ccc-dev` installed and enabled,
   run `ccc setup --with-plugin --recreate-config` again before relying on plugin hooks.

7. If plugin hooks, graph-card planning context, or LSP lifecycle proof still
   show missing, run one small explicit $ccc request so the host can observe
   WAVE/PostToolUse/LSP/Stop lifecycle proof, then run:
   - ccc check-install --text
   - ccc status --text

8. Report whether $ccc, MCP registration, hooks, graph-card planning context,
   LSP safety surface, restart requirements, and PATH cleanup are current.
```

## Commands

Use this inside Codex CLI:

| Command | Use it for |
| --- | --- |
| `$ccc "task"` | Start a CCC-managed Codex task. |

Run these in your terminal:

| Command | Use it for |
| --- | --- |
| `cargo install codex-cli-captain --force` | Install or update the CCC binary from crates.io. |
| `ccc setup --with-plugin --recreate-config` | Refresh the Codex CLI integration, plugin marketplace/cache, hooks, skills, agents, and config after install or update. |
| `ccc check-install --text` | Verify install, hooks, skills, agents, and Graph Context. |
| `ccc status --text` | See the current task state and next action. |
| `ccc activity --json` | Inspect bounded activity and proof paths. |
| `ccc readiness-runbook --json` | Inspect readiness gaps and the next bounded command. |

## What CCC Gives You

| Feature | What it means for you |
| --- | --- |
| Plan-first work | Broad tasks get a clear plan before edits happen. |
| Captain orchestration | The host Codex stays in charge while CCC routes specialist work when useful. |
| Quiet progress tracking | Status, checklist, and fan-in stay in CCC surfaces instead of noisy transcript text. |
| Evidence-based finish | CCC treats current validation and review evidence as the source of truth. |

Scope note: `v0.0.24` keeps short natural-language `$ccc` requests on the
installed orchestrator path while tightening App-native proof binding. The
verified release covers WAVE activation, Scout/graph-card preflight, ODYSSEY
lane contracts and lane queues, host update_plan mirror/ACK, role-specific
Tactician/Scout/Raider/Scribe/Ghost/Arbiter/Sentinel/Oracle surfaces,
App-native lifecycle receipts, docs read-only proof, same-run durable fan-in,
LSP safety, Arbiter reverify, Stop closeout, `CCC WAVE DEACTIVE`, and
`install_runtime_ready=true` with release-blocking gaps at zero.

## Use It

Start with `$ccc`:

```text
$ccc update the release docs, verify the current evidence first, and report what changed
```

Use `$ccc` again for follow-ups, or use the continuation command printed by CCC
when a persisted run is available.

Good fits for `$ccc`:

- multi-step code or documentation work
- release, install, or verification-sensitive tasks
- tasks that should continue from a saved plan
- work that benefits from subagent fan-in or review

For small one-off questions, normal Codex CLI usage is usually enough.

## Update

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin --recreate-config
```

Verify MCP/plugin registration, restart Codex CLI / Codex App when asked, then
verify again:

```bash
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

Use the Cargo-first path above for normal updates. CCC does not require
release-bundle scripts for the normal update path.

## Uninstall

```bash
cargo uninstall codex-cli-captain
```

`cargo uninstall` removes the Cargo binary. Review Codex config before removing
any local CCC-managed hook, MCP, or skill files by hand.

After uninstall, fully restart Codex CLI / Codex App and verify:

```bash
command -v ccc || true
codex mcp list
```

## Notes

- `$ccc` is the public entrypoint. Role task cards, fan-in, Graph Context,
  hooks, and review details are internal CCC proof surfaces.
- Codex plugin hooks are optional. If enabled, restart Codex CLI and verify with
  `ccc check-install --text`.
- macOS is the primary maintainer smoke path. Linux and Windows builds should be
  verified in their target environments with `ccc check-install --text`.
- Crates.io-installed binaries can report partial binary provenance because
  Cargo rebuilds from the published crate.
