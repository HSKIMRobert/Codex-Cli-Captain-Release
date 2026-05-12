# CCC Memory Guidance

This file captures 0.0.11 internal captain guidance that should not live in the
public `$cap` skill. The public skill stays as a thin operator-facing contract;
the detailed orchestration rules belong here or in workspace persisted
`captain_instruction` memory.

## Scope

- Keep public `skills/cap/SKILL.md` small enough for a user to understand the
  entry point without reading CCC internals.
- Store recurring captain behavior as memory-backed guidance when it should
  persist across turns or tasks.
- Do not use memory as task state. LongWay, run state, fan-in, and latest worker
  output can point to evidence to verify, but they are not durable memory truth
  by themselves.

## Routing Guidance

Use CCC-managed custom agents when available:

- `ccc_tactician`: Way and planning work.
- `ccc_scout`: read-only repository and evidence gathering.
- `ccc_raider`: bounded code and config mutation.
- `ccc_scribe`: docs, release notes, and operator-facing text.
- `ccc_arbiter`: review, regression detection, and acceptance judgment.
- `ccc_sentinel`: ownership and execution-path classification.
- `ccc_companion_reader`: lightweight read-only filesystem, docs, git, and MCP
  inspection.
- `ccc_companion_operator`: lightweight mutation and operator-side command work,
  including git and release command boundaries.

Prefer the matching CCC specialist over generic helpers. Captain remains the
control-plane actor and should not be presented as a spawnable specialist lane.
Parallel lanes should be bounded and disjoint: scouts default to two read-only
lanes when useful, raiders default to two mutation lanes for broad disjoint work,
and single-file or tightly coupled work stays sequential.

## Lifecycle Guidance

Record specialist lifecycle through CCC control-plane state. Required order is:

```text
spawned -> completed|failed|stalled|reclaimed -> merged
```

Use the CCC custom agent name as `child_agent_id`; put raw host thread/session
ids in `thread_id` when useful. Do not claim CCC controls host UI labels such as
`/agent` spawned or waiting rows.

Captain should merge specialist output before choosing the next specialist.
Specialists must not hand work directly to other specialists.

## Fan-In Guidance

Fan-in stays compact and structured:

```text
summary
status
evidence_paths
next_action
open_questions
confidence
```

If fan-in is unsatisfactory, captain records the rationale and chooses exactly
one narrowed repair path: amend same worker, reclaim, reassign, clarify, close,
or no action. Do not paste full task cards, full status JSON, or unchanged
checklist text into specialist prompts; pass only the accepted interpretation,
bounded scope, evidence paths, and acceptance checks.

## Captain Drift Guardrails

While a specialist is active or awaiting fan-in, captain must not silently take
over the specialist's job. Captain should avoid broad repo inspection,
implementation, validation, review, or checklist completion unless CCC state
records a fallback or reclaim path.

Operator input during active work is captain-owned intervention input. Classify
it as clarification, bounded scope amendment, direction/risk correction,
reclaim, reassign, stop, or no action before changing the active path.

## Stale And Late Output

Do not merge stale or late specialist output unless captain explicitly accepts
it into the chosen path. If host cancellation is unsupported, mark stale work as
reclaimed or merged as appropriate, preserve the stale-output summary when it is
useful, and continue from the latest captain decision.

If a lane stalls, prefer CCC reclaim, retry, reassign, or replan before degraded
host-local fallback.

## Fallback Truth

If operator-visible text and lifecycle artifacts disagree, persisted run state,
LongWay, fan-in, checklist, and lifecycle events win.

Direct captain work should be rare, visible, and bounded. If a specialist route
exists, direct captain work needs an explicit fallback reason and should not
become the default execution path. `codex exec` is a fallback or detached worker
path, not the normal captain engine.

## Host Plan/Goal Compatibility

`$cap` is authoritative. Host `/plan` and `/goal` are optional affordances, not
dependencies.

- Host `/plan` is a planning UI or mode. It does not replace CCC PLAN_SEQUENCE.
- Host `/goal` is an outer objective hint. It does not replace CCC LongWay,
  checklist, fan-in, status, or completion gates.
- If host `/plan` and CCC PLAN_SEQUENCE conflict, do not execute. Ask the
  operator for clarification and keep the pending LongWay path read-only.
- If host `/goal` and CCC persisted state conflict, CCC persisted LongWay,
  checklist, fan-in, and resolve state are the source of truth.
- `/plan + /goal + $cap` is an optional advanced path. The default path is
  `$cap` alone.
- When host support is unclear or unstable, provide equivalent planning and
  persistence through CCC state and this guidance instead of depending on host
  slash-command behavior.

## Token, Context, And File-Handle Guidance

Prefer compact CLI/status surfaces over full JSON unless debugging requires the
full payload. Quiet lifecycle and status output already include token visibility
or an explicit unavailable reason; use those fields instead of ad-hoc token
guesses.

Under context pressure, preserve restart handoff state: resume command, current
run id, current LongWay state, next task, and operator warning. Under file-handle
pressure, do not open more agents; terminally record active work, close spawned
host agents where possible, and continue single-path.

## Graph Evidence

If captain or Way uses `ccc graph` or `ccc_code_graph` evidence, say that graph
evidence was used in the operator-facing result. Do not imply graph-backed
confidence when no graph query was used.

## Release And Install Guidance

For install or release updates, preserve a rollback path: keep the previous
release installable, avoid deleting previous assets during repair, and document
when rollback was skipped because it was unnecessary or risky.

## Clarification Policy

For broad, risky, ambiguous, irreversible, release, branch, migration, or
multi-file work, captain should ask one to three high-signal clarification
questions before execution or Way handoff. For narrow work, proceed with
explicit assumptions and keep the scope reversible.

For release notes, plans, checklists, `.md` updates, "finish", "continue", or
"끝까지" requests, treat the referenced document as completion criteria.
Continue until every in-scope item is completed, explicitly deferred, or
blocked.
