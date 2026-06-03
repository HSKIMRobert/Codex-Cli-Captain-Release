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

Release version: `0.0.14`.

## Install

For a direct install:

```bash
cargo install codex-cli-captain --force
ccc setup
```

Then fully restart Codex CLI and verify:

```bash
ccc check-install --text
ccc status --text
```

If you want an AI agent to handle the install, paste this:

```text
Install or update Codex-Cli-Captain 0.0.14 for Codex CLI.

1. Check the current state with:
   - command -v ccc || true
   - ccc --version, if ccc exists
   - ccc check-install --text, if ccc exists

2. Install or update:
   - cargo install codex-cli-captain --force
   - ccc setup

3. Ask me to fully restart Codex CLI.

4. After restart, verify:
   - command -v ccc
   - ccc --version
   - ccc check-install --text
   - ccc status --text

5. Report whether $ccc, MCP registration, hooks, custom agents, and Graph
   Context are current, and whether any restart or PATH cleanup is still needed.
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
| `ccc setup` | Refresh the Codex CLI integration after install or update. |
| `ccc check-install --text` | Verify install, hooks, skills, agents, and Graph Context. |
| `ccc status --text` | See the current task state and next action. |

## What CCC Gives You

| Feature | What it means for you |
| --- | --- |
| Plan-first work | Broad tasks get a clear plan before edits happen. |
| Captain orchestration | The host Codex stays in charge while CCC routes specialist work when useful. |
| Quiet progress tracking | Status, checklist, and fan-in stay in CCC surfaces instead of noisy transcript text. |
| Evidence-based finish | CCC treats current validation and review evidence as the source of truth. |

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
ccc setup
```

Restart Codex CLI, then run:

```bash
ccc check-install --text
```

## Uninstall

```bash
ccc uninstall --dry-run
ccc uninstall --confirm
cargo uninstall codex-cli-captain
```

Run the dry-run first and review the paths. `cargo uninstall` removes the Cargo
binary. `ccc uninstall --confirm` removes CCC-managed Codex surfaces while
preserving unrelated Codex data.

## Notes

- `$ccc` is the public entrypoint. `wccc`, task cards, fan-in, Graph Context,
  hooks, and review details are internal CCC proof surfaces.
- Codex plugin hooks are optional. If enabled, restart Codex CLI and verify with
  `ccc check-install --text`.
- macOS is the primary maintainer smoke path. Linux and Windows builds should be
  verified in their target environments with `ccc check-install --text`.
- Crates.io-installed binaries can report partial binary provenance because
  Cargo rebuilds from the published crate.
