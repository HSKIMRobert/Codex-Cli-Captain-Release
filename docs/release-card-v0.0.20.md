## v0.0.20

Install:

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin
```

Highlights:
- Durable host-hook teammode proof surfaces from `v0.0.19` are preserved.
- Installed App lifecycle path is repaired for natural `$ccc` runs.
- WAVE/ODYSSEY persistence, graph-card and compact projection, and host
  update_plan mirror/ACK are covered by the verified lifecycle smoke.
- PostToolUse hooks and App-native Worker/Arbiter spawn, fan-in, and close
  recovery are covered by the verified lifecycle smoke.
- Proof-command drift protection, shell-wrapped proof fail-closed behavior,
  LSP bridge/safety loop, team registry readiness, and Stop closeout with
  `CCC WAVE DEACTIVE` are covered by the verified lifecycle smoke.
- Raw prompt/repo/rule/hook/transcript/token/env/LSP payload omission is
  preserved.

Verification:
- `cargo publish -p codex-cli-captain --allow-dirty` succeeded.
- crates.io reports `codex-cli-captain = "0.0.20"`.
- Published install of `codex-cli-captain 0.0.20` succeeded.
- Installed `ccc --version` reported `0.0.20`.
- `ccc setup --with-plugin` completed and Codex App / Codex CLI were restarted.
- Post-publish natural `$ccc` lifecycle smoke passed with
  `install_runtime_ready=true`, `install_release_blocking_gaps=0`,
  `workflow_parity_gaps=0`, `captain_direct_work_drift=false`,
  `receipt_only_teammode=false`, `team_registry_ready=true`, and
  `CCC WAVE DEACTIVE`.
