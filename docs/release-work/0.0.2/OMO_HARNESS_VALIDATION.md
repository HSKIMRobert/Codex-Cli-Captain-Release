# v0.0.2 OMO Harness Validation

## Goal

Validate that `$cap` behaves like a compact captain-first harness:

1. captain opens a bounded LongWay
2. captain selects the right CCC custom subagent from `ccc-config.toml`
3. specialist returns compact structured fan-in
4. captain merges the result before selecting the next specialist
5. fallback/reclaim state is explicit when subagent execution cannot continue
6. raider fan-out, when used, respects stable lane IDs and an all-lane fan-in barrier

This document tracks smoke tests, issues found, and fixes applied for the
remaining `0.0.2` update.

## Constraints

- Keep `$cap` skill and generated custom-agent prompts compact.
- Prefer compact CLI payloads with `compact=true`.
- Prefer `--text`, `--quiet`, and `--json-file` for lower-noise lifecycle calls.
- Do not use `ccc activity` unless compact status lacks required truth.
- Do not treat `/agent` or `/subagents` as the orchestrator.
- Do not fall back to detached `codex exec` while an active task-card has no
  terminal subagent update or accepted fallback reason code.
- Treat host-side token status for custom subagents as best-effort only.
- Record `child_agent_id` as the CCC role or managed agent name and `thread_id`
  as the raw host Codex thread/session identifier.

## Test Matrix

| ID | Smoke | Acceptance | Status | Notes |
| --- | --- | --- | --- | --- |
| OMO-01 | config sync | `ccc setup/check-install` syncs 9 managed agents and preserves compact prompts | pass | Installed `$cap` skill is 71 lines / 4015 bytes; managed agents total 103 lines / 5495 bytes. |
| OMO-02 | single specialist | `$cap` creates run, spawns one CCC custom subagent, records `spawned -> completed -> merged`, then resolves | pass | Covered by live chain run for `ccc_tactician`, `ccc_scout`, and `ccc_scribe`. |
| OMO-03 | sequential chain | captain merges agent1 before selecting agent2, then resolves | pass | Live run completed `ccc_tactician -> ccc_scout -> ccc_scribe -> ccc_scribe repair -> completed`. |
| OMO-04 | companion route | lightweight read/tool request routes to `ccc_companion_reader` or configured mini role | pass | CLI smoke selected `ccc_companion_reader` with `gpt-5.4-mini`, `medium`, `read-only`. |
| OMO-05 | fallback code | invalid fallback reason is rejected; valid reason opens visible fallback state | pass | CLI smoke rejects unknown reason and records `child_timeout`. |
| OMO-06 | compact output | compact status avoids full `delegation_plan`; `$cap` does not duplicate final answer | pass | Compact payload exposes `subagent_contract`; live output file contains final answer once. |
| OMO-07 | release install | release asset installs latest binary, compact skill, and custom agents | pass | Local release asset install smoke passed; tarball sha256 `3eef4ead9b4722c366ddf949d0389b688c4d1d5c8d9a31642b72259100b6aa77`. |
| OMO-08 | stalled recovery | stalled subagent state can be merged by captain, reassigned, and completed by a replacement specialist | pass | CLI smoke completed `stalled tactician -> captain merge -> scout reassign -> complete`. |
| OMO-09 | runtime drift visibility | observed model/variant/sandbox mismatch is visible in compact update and compact status | pass | CLI smoke exposes `subagent_policy_drift.ok=false` and mismatch fields in compact surfaces. |

## Issue Log

| ID | Found In | Problem | Fix | Status |
| --- | --- | --- | --- | --- |
| OMO-I01 | OMO-05 | `subagent-update --compact` recorded fallback reason internally but did not echo `fallback_reason` or `subagent_fallback` in the compact response. | Added both fields to the compact subagent-update response. | fixed |
| OMO-I02 | OMO-03 | Compact `orchestrate` command template suggested nested `replan`/`resolve` objects, but CLI accepts top-level `repair_action`, `replan_prompt`, `resolve_outcome`, and `resolve_summary`. | Changed compact template payload to top-level fields and added test coverage. | fixed |
| OMO-I03 | OMO-03 | First `ccc_scribe` pass appended an extra trailing period to the requested line. | Captain marked the result failed, merged the rejected fan-in, and reassigned `ccc_scribe` for a bounded repair. | handled by workflow |
| OMO-I04 | OMO-09 | Full status exposed `subagent_policy_drift`, but compact status and compact subagent-update omitted it. | Added `subagent_policy_drift` to compact status task-card and compact subagent-update response. | fixed |

## Evidence Log

Record commands or run ids here as each smoke is executed.

| ID | Evidence |
| --- | --- |
| OMO-01 | `ccc setup` and `ccc check-install` reported `status=ok`; 9 managed agents synced under `~/.codex/agents`. |
| OMO-02 | Live run `e17e7b6e-f538-b595-f085-1f44c4c72e98` recorded each specialist as `execution_mode=codex_subagent`. |
| OMO-03 | Live run `e17e7b6e-f538-b595-f085-1f44c4c72e98` ended with `status=completed`; task cards show captain merges between specialists. |
| OMO-04 | `/tmp/ccc-omo-companion-smoke/start.json` selected `ccc_companion_reader`, `gpt-5.4-mini`, `read-only`. |
| OMO-05 | `/tmp/ccc-omo-fallback-smoke-2` rejected invalid fallback reason and recorded `child_timeout`; compact status allowed `codex_exec_fallback_allowed=true` only after fallback state. |
| OMO-06 | `/tmp/ccc-omo-template-smoke/start.json` has top-level compact orchestrate fields and no `delegation_plan`; `/tmp/ccc-omo-chain-live/codex-output.txt` contains the final answer once. |
| OMO-07 | `CCC_DOWNLOAD_URL=file://.../ccc-0.0.2-darwin-arm64.tar.gz install.sh` completed; post-install `ccc check-install` reported `status=ok`. |
| OMO-08 | `/tmp/ccc-omo-stall-reassign-smoke` completed run `3e9c3c79-2440-a7f0-84bf-0a483969d4b4` after `stalled -> merged -> scout reassign -> completed`. |
| OMO-09 | `/tmp/ccc-omo-drift-smoke-2` confirmed compact `subagent-update` and compact `status` expose model, variant, and sandbox drift mismatches. |
