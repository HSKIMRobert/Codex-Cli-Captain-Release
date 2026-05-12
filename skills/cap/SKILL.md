---
name: cap
description: Enter the current user request through Codex-Cli-Captain so host Codex runs the captain-first LongWay loop instead of answering locally.
metadata:
  short-description: Captain LongWay loop entry
---

# $cap

Use when the operator invokes `$cap`: strip the token and route the rest through
CCC.

## Public Contract

- `$cap` is the only public CCC entry point; do not document host `/plan` or
  `/goal` as CCC paths.
- CCC owns persisted LongWay, task cards, checklist/projection, fan-in, status,
  and restart handoff.
- While a CCC run is active, Captain must spawn or route specialist-owned work
  to configured `ccc_*` role agents, wait for fan-in, then merge/review the
  result. Do not use `apply_patch`, direct shell mutation, or host-local
  implementation for specialist-owned work while a configured `ccc_*` role
  path is available. Generic `worker`/`explorer` suggestions are stale routing
  for that work unless the run has recorded explicit override or fallback, and
  host-local direct mutation is forbidden while the role-agent path is
  available.
- If host subagent capacity is exhausted, wait for fan-in, close completed host
  threads, or record reclaim/reassign/fallback before retrying. Terminal
  fallback or reclaim is allowed only after actual subagent unavailability,
  capacity exhaustion, or stall, or after explicit operator override, and CCC
  must record the reason and make it visible in status or projection output.
  Do not take over specialist-owned mutation directly.
- `$cap` dispatch is a two-step obligation, not only a status mutation. After
  `ccc start`, `ccc run`, or `ccc orchestrate` returns an executable
  `current_task`, inspect `current_task.assigned_agent_id`,
  `current_task.assigned_role`, and `next_step`. If the task is active and the
  assigned agent is a configured `ccc_*` or companion role, immediately hand the
  task to that host custom agent with the bounded task prompt, then record
  `ccc subagent-update` spawn/fan-in events. Do not continue captain-local
  implementation just because the run was successfully created.
- Skip immediate dispatch only when CCC is waiting for operator LongWay
  approval, the run is terminal, the assigned role is the orchestrator itself,
  no configured host role exists, or a recorded capacity/fallback condition
  applies.
- Host Plan Mode is not a Way engine. Route planning through CCC
  `PLAN_SEQUENCE`; host proposed-plan output must not own CCC state.
- Operator-visible CCC lifecycle mutations should be compact PATH `ccc` CLI
  runs with inline JSON: `start`, `orchestrate`, `subagent-update`, `memory`.
  Use `status/checklist --projection` or text status only for visible LongWay
  review; reserve MCP `ccc_*` for app/structured inspection or CLI unavailable.
- Reply in the operator's language; stored LongWay text stays English.
- Continue from CCC status/checklist/projection truth, not hidden host state.
- Follow-ups to a CCC plan/LongWay still belong to CCC even without `$cap`;
  route them through CCC or ask confirmation.
- When presenting a CCC plan for approval, include a copyable `$cap ...`
  follow-up that preserves the entry point.
- Keep internal routing/lifecycle/fallback/context rules in CCC memory or
  persisted captain guidance, not this public skill.
