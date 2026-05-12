# CCC Memory Spec

This is the opt-in CCC memory design for `0.0.10-pre`. It is inspired by durable agent memory patterns, but uses generic CCC wording and does not depend on external Hermes code or docs.

## Scope

CCC memory is a workspace-scoped, small JSON file at `.ccc/memory.json`. It is off until the operator explicitly previews and writes entries.

Captain Instruction Memory is a companion workspace-managed instruction layer that sits outside hardcoded prompts. It is meant to hold captain-wide guidance that applies to all users in the workspace, including recurring clarification obligations that should be re-applied throughout a task instead of only at the first command.

Allowed entry kinds:

- `user_preference`: operator-stated preferences for this workspace.
- `repeated_rule`: recurring operator rules that should survive individual runs.
- `verified_project_fact`: facts verified from project files, tests, or explicit operator confirmation.
- `captain_instruction`: durable captain-wide instruction text that should be loaded from workspace memory rather than embedded in a prompt template.

Forbidden memory truth sources:

- LongWay state.
- Run state.
- Latest work result or delegate result.
- Inference-only observations.
- Hardcoded prompt text as a storage location for durable captain instructions.

Allowed `source_kind` values are normalized before storage and limited to `operator_confirmation`, `project_file`, and `test_result`. LongWay, run state, activity, fan-in, and latest worker output can point at work to verify, but they are never durable memory truth by themselves. Variants such as `latest work result`, `run-state`, and `inference_only` are rejected.

Captain Instruction Memory rules:

- Captain instructions must be persisted in workspace memory or another explicit memory-backed store, not hidden inside prompt strings.
- Captain instructions are cap-wide and apply to all users of the workspace unless a narrower scope is recorded.
- Clarification duties belong in this memory when they are recurring obligations, so the captain can re-assert them at any later turn in the same task.
- Captain instructions are guidance, not task state, and they must not absorb run history or latest evidence.

## Safety Contract

- Writes require an explicit preview first. The CLI write path requires `preview_ack=true`, the preview's `preview_token`, and `expected_updated_at_unix_ms` from the preview/status payload. The token binds to the normalized proposed entries and the current store timestamp, so `preview_ack=true` by itself is not sufficient.
- Stale-write protection rejects writes when the file changed after preview.
- Status marks memory stale when it has not been refreshed for 30 days.
- `verified_project_fact` entries require `evidence_paths`.
- `captain_instruction` entries must remain prompt-independent and may not be stored as literal prompt templates.
- Inferences are rejected instead of stored at lower confidence.
- The store is capped at 50 entries and 16 KiB.
- `ccc memory` defaults to read-only status.
- Public CLI JSON always uses workspace `.ccc/memory.json`; `store_path` is not accepted.
- `ccc memory {"action":"off"}` previews disabling memory; `apply=true` plus the preview token and expected timestamp are required to write the off state.

## CLI Surface

Examples:

```text
ccc memory --text
ccc memory --json '{"action":"preview","entries":[{"kind":"user_preference","text":"Prefer focused Rust tests for narrow CLI behavior."}]}'
ccc memory --json '{"action":"write","preview_ack":true,"preview_token":"ccc-memory-preview-v1:...","expected_updated_at_unix_ms":null,"entries":[{"kind":"verified_project_fact","text":"CCC memory stores workspace data in .ccc/memory.json.","source_kind":"project_file","evidence_paths":["rust/ccc-mcp/src/memory.rs"]}]}'
ccc memory --json '{"action":"off","apply":true,"preview_token":"ccc-memory-preview-v1:...","expected_updated_at_unix_ms":123}'
```

`ccc status --text` and compact status include a small memory summary: enabled/off/unconfigured, entry count, and stale flag.

Captain instruction status should make the source explicit when present so the operator can tell the difference between live prompt text and memory-backed instruction text.

## Deferred

- No automatic memory extraction from task cards, fan-in, latest work result, status, or LongWay.
- No cross-workspace or user-global memory file.
- No semantic search, embeddings, or external memory service.
- No automatic prompt injection beyond the explicit status/CLI surface.
- No hiding captain instructions only in prompt templates.
