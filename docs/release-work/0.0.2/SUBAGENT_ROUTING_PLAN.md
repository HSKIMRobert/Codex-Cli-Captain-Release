# v0.0.2 Subagent Routing Plan

## Goal

Move CCC `0.0.2` toward a Codex-native subagent workflow where the host Codex
main thread stays captain, `Way` remains a bounded captain-side planning step,
and specialist execution can be delegated through official Codex custom agents
derived from `ccc-config.toml`.

The target runtime shape is:

1. operator sends `$cap ...`
2. host Codex main thread is `captain`
3. `Way` is created or updated in the main thread
4. captain chooses the next bounded specialist from config-backed routing
5. Codex spawns an official custom subagent
6. subagent returns a concise structured fan-in result to captain
7. captain updates the LongWay and either delegates again or resolves

## Why This Change

- detached `codex exec` workers are workable, but they are outside Codex's
  native thread/subagent lifecycle and have shown weaker behavior in public
  `$cap` flows
- Codex subagents are now an official documented surface
- `ccc-config.toml` already owns the role/model/routing policy, so CCC can stay
  the control plane while Codex becomes the preferred execution engine
- this should reduce context pollution in the captain thread while preserving
  visibility and bounded delegation

## Principles

- `ccc-config.toml` stays the source of truth
- CCC remains the owner of LongWay, task-card, routing, visibility, reclaim,
  and fallback state
- Codex custom agents become the preferred specialist execution surface
- read-heavy work may fan out selectively
- raider fan-out uses lane-aware parallel V1 with stable `raider-a`, `raider-b`,
  `raider-c`, and `raider-d` lanes
- the default raider fan-out is 2 lanes only for explicit disjoint parallel
  work
- the hard maximum is 4 lanes
- fan-in must wait for every active lane before merge
- write-heavy work remains mostly sequential
- `codex exec` stays available only as a fallback execution mode
- operator-facing progress stays in CCC status/activity; `/agent` is optional
  deeper inspection
- `/agent` is a thread-switching surface, not the orchestration engine
- fan-in must stay compact, but not summary-only
- child policy drift must be visible because parent sandbox and approvals can
  override child defaults
- host-side token status for custom subagents is limited and best-effort only
- `child_agent_id` should name the CCC role or managed agent, while `thread_id`
  should carry the raw host Codex session/thread identifier for correlation

## Execution Model

### Preferred path

- captain runs in the host Codex main thread
- captain uses config-backed category shortlist routing to select a specialist
  role
- CCC-synced custom agent files provide the actual Codex subagent definitions
- captain asks Codex to spawn the matching specialist and wait for a bounded
  structured result

### Fallback path

- if subagent delegation is unavailable, blocked, or explicitly disabled
- CCC may still launch the existing `codex exec` worker path
- fallback must be explicit in runtime state and operator visibility
- fallback reason codes should remain inspectable

## Config Strategy

### Source of truth

- `~/.config/foreman/ccc-config.toml`

### Derived surface

- `~/.codex/agents/ccc-*.toml`

The sync layer should:

- generate CCC-managed custom agent files from the shared config
- use stable managed names that do not clobber unrelated user-defined agents
- write atomically
- remove stale CCC-managed files only within the managed prefix
- record enough sync truth to detect stale or mismatched managed agents
- expose sync health through `ccc check-install`

## Role Mapping

The initial managed roles are:

- `orchestrator -> captain`
- `way -> tactician`
- `explorer -> scout`
- `code specialist -> raider`
- `documenter -> scribe`
- `verifier -> arbiter`
- `sentinel -> sentinel`
- `companion_reader -> companion_reader`
- `companion_operator -> companion_operator`

Each generated custom agent should carry:

- `name`
- `description`
- `developer_instructions`
- `sandbox_mode`
- `model` when configured
- `model_reasoning_effort` from `variant` when configured
- `service_tier = "fast"` when `fast_mode = true`

## Routing Model

Routing remains two-stage:

1. `category_shortlist` narrows the candidate roles cheaply
2. captain selects the next specialist role from the shortlist

The important change is that the selected role now maps to a Codex custom
subagent definition instead of defaulting to detached `codex exec` workers.

## Visibility Contract

CCC status/activity remains the stable operator surface and must continue to
show:

- LongWay progress
- current captain checkpoint
- selected specialist role
- fan-in readiness
- reclaim/fallback truth
- token discipline summaries
- preferred specialist execution mode and fallback execution mode
- structured fan-in contract for the active task-card
- whether policy drift checks are required for spawned children
- compact status and command templates for the ordinary public loop so captain
  does not repeatedly ingest full run/task payloads
- compact v2 status that exposes only the active subagent contract instead of
  re-sending the full delegation plan and all lifecycle template payloads
- `subagent-update` carries `lane_id` for lane-aware raider fan-out/fan-in
- lower-noise operator surfaces should prefer `--text`, `--quiet`, and
  `--json-file` for repeated lifecycle payloads

Codex `/agent` or `/subagents` views are useful secondary inspection surfaces,
but CCC must not depend on undocumented UI-only commands as the canonical
runtime contract.

## Workstreams

### 1. Config Sync

- add a sync layer from `ccc-config.toml` to `CODEX_HOME/agents`
- generate CCC-managed custom agents atomically
- expose sync health in `setup` and `check-install`

### 2. Captain-First Subagent Delegation

- keep `Way` in the main thread
- teach the `$cap` guidance to prefer official custom subagents
- treat `/agent` only as inspection or thread switching, not as the spawn
  primitive
- require a spawn contract that forbids full-history fork conflicts and avoids
  redundant agent/model/reasoning overrides when the custom agent already
  defines them
- record host-subagent lifecycle through the CLI subcommand path rather than
  the MCP tool form when the task-card says so
- prefer compact CLI calls in the public loop:
  `ccc start --json '{...,"compact":true}'`,
  `ccc status --json '{"run_id":"<run_id>","compact":true}'`,
  `ccc subagent-update --json '{...,"compact":true}'`, and
  `ccc orchestrate --json '{...,"compact":true}'`
- use compact `command_templates` instead of command-discovery probes such as
  help dumps or broad source/session searches
- keep `$cap` skill guidance compact because it is loaded on every public
  `$cap` request
- require compact structured fan-in fields:
  `summary`, `status`, `evidence_paths`, `next_action`, `open_questions`,
  `confidence`
- expose policy drift checks for model/sandbox/approval mismatches after spawn
- preserve lane identity in compact fan-in so raider lanes can merge in order

### 3. Runtime Mode Split

- add explicit execution mode selection
- prefer `codex_subagent`
- keep `codex_exec` as fallback
- keep fallback reason codes visible in CCC state
- block `codex_exec` fallback for the same task-card until CCC has either:
  an explicit terminal host-subagent update
  or an explicit recorded fallback reason

### 4. Validation

- config sync tests
- custom-agent drift detection tests
- execution-mode and delegation-plan payload tests
- single specialist delegation smoke
- captain -> specialist -> captain chaining smoke
- selective parallel read-only fan-out smoke
- installed-path public `$cap` smoke

## Acceptance

- `ccc setup` syncs CCC-managed Codex custom agents
- `ccc check-install` reports custom-agent sync health honestly
- unrelated user custom agents are preserved
- captain can route from `ccc-config.toml` roles to official custom subagents
- task-card and status payloads expose subagent-first delegation plans
- task-card fan-in contracts are structured rather than summary-only
- the preferred path no longer requires detached `codex exec` workers for every
  specialist step
- fallback remains available and visible

## Current Phase

Phase 1 in this document is the config-sync foundation:

- [x] plan document added
- [x] generate CCC-managed custom agent files from shared config
- [x] sync during `ccc setup`
- [x] surface sync health in `ccc check-install`
- [x] expose subagent-first delegation plans in task-card and status payloads
- [x] add structured fan-in and policy-drift requirements to the plan
- [x] add a host-subagent lifecycle recorder so CCC can persist spawned/completed/merged state
- [x] expose spawn/update/fallback contracts in task-card and status payloads
- [x] add compact status and command templates for lower-token public loops
- [x] compact the packaged `$cap` skill and installed skill from roughly 12KB to roughly 4KB
- [x] compact status output to expose `subagent_contract` instead of full `delegation_plan`
- [x] switch captain routing contract from detached workers to official subagents by default
- [x] keep `codex exec` blocked for the active task-card until subagent terminal/fallback state is recorded
- [x] surface and validate fallback reason codes from live runtime transitions
- [x] validate public `$cap` with live subagent delegation and CLI lifecycle fan-in
- [x] document lane-aware raider fan-out/fan-in V1 and compact lifecycle identifiers
