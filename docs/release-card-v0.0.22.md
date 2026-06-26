## v0.0.22

Install:

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin --recreate-config
```

Highlights:
- Short natural-language CCC requests now enter the orchestrated flow without
  the operator describing WAVE, Scout, ODYSSEY, role lanes, Arbiter, or Stop
  closeout.
- The installed `$ccc` path records WAVE activation, Scout/graph-card
  preflight, ODYSSEY lane contracts, host `update_plan` mirror/ACK, App-native
  role lifecycle receipts, compact fan-in, Arbiter reverify, LSP safety, and
  Stop closeout before `CCC WAVE DEACTIVE`.
- Role-specific CCC agents are installed for Tactician, Scout, Raider, Scribe,
  Ghost, Arbiter, Sentinel, and Oracle. Ghost is dedicated to approved local
  git/status/diff/stage/commit/release-prep work only.
- The Scribe/Arbiter lane binding path accepts ODYSSEY-created Scribe lane ids,
  README documentation targets, and bounded read-only documentation proof.
- `ccc status --text` and `ccc check-install --text` stay fail-closed when the
  installed App workflow can be bypassed.

Verification:
- Release commit: `cd5fb5d`.
- Git tag: `v0.0.22`.
- `cargo package` passed for `codex-cli-captain v0.0.22`.
- `cargo publish --dry-run` passed before release.
- `cargo publish` succeeded and published `codex-cli-captain v0.0.22` to
  crates.io.
- Published install with
  `cargo install codex-cli-captain --version 0.0.22 --force` succeeded.
- Installed `ccc --version` reported `0.0.22`.
- Final installed natural-command smoke used only a short request:
  `ccc 'README 설치 안내가 0.0.22 후보 기준으로 명확한지 검증해줘'`.
- The smoke completed WAVE, graph-card/Scout preflight, ODYSSEY lane contract,
  host `update_plan` mirror/ACK, App-native Scribe/Arbiter lifecycle receipts,
  fan-in, LSP safety, reverify CLEAR, Stop closeout, and `CCC WAVE DEACTIVE`.
- Final `ccc status --text` reported `current_phase=wave_lifecycle_closed`,
  `wave_deactivate_text=CCC WAVE DEACTIVE`,
  `install_release_blocking_gaps=0`, and `workflow_parity_gaps=0`.
- Final `ccc check-install --text` reported `install_runtime_ready=true`,
  `install_release_blocking_gaps=0`, and `workflow_parity_gaps=0`.
