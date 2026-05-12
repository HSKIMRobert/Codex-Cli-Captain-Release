# 0.0.8 Pre-Release Plan

`0.0.8-pre` should tighten operator-facing guidance and the supporting runtime checks around comment and annotation requests, confirmation-loop handling, routing guards, stalled recovery, LongWay owner rows, follow-up repair work, and token-aware skill/agent prompt routing.

## Release Goal

Make comment and annotation handling explicit: when the operator asks for comments or annotations, CCC should keep the requested chronological block format. Align the plan with the implementation scope for the confirmation loop, routing guards, stalled-subagent recovery, optional LongWay owner rows, and follow-up repair routing.

Also keep the `$cap` skill and generated CCC custom-agent prompts aligned with the runtime contract: route through the selected specialist, pass only bounded context, and avoid repeated status/LongWay lifecycle text unless a lifecycle boundary changed.

## Historical Baseline

- `0.0.7-pre` docs already emphasize captain-first routing, visible lifecycle blocks, and compact status.
- `v0.0.2` OMO validation already documents the harness boundary: one captain, one specialist at a time, explicit fan-in, and no silent fallback.
- The current README and release notes do not yet state how comment and annotation requests should preserve chronological block formatting.
- The current runtime already has visible degraded-host fallback and reclaim/replan plumbing, and this release is now hardening the confirmation loop, routing guards, stalled recovery, LongWay owner rows, and repair follow-up paths.
- In the 0.0.7-pre release-asset repair, CCC initially routed the documenter/scribe side, but captain directly performed non-trivial mutation, including packaging script edits and GitHub release asset/card changes.
- The installed `$cap` skill can drift from the repo copy between release installs. The 0.0.8 source guidance should make the desired compact behavior explicit so release packaging and `ccc setup` can sync it.

## Docs Gap Notes

- Comment and annotation requests need a direct instruction: preserve the requested chronological block format instead of flattening the content into unordered bullets.
- The docs should not imply that CCC runs together with OMO, sisyphus, or an external harness. The intended framing is what CCC needs in order to behave in that style.
- Harness-style operator guidance should keep compact LongWay/status visibility, terminal fan-in, explicit merge or reclaim language, and fallback truth visible so the workflow stays auditable.
- For complex or risky requests, captain should first analyze the request and send a concise interpretation or confirmation reply to the operator. Only after the operator responds positively with something like yes, correct, or proceed as-is should captain hand the accepted interpretation to Way/tactician for planning. If the operator rejects or corrects the interpretation, captain should revise it and repeat the confirmation loop.
- Role separation should be explicit: scout and companion_reader collect evidence, Way/tactician plans, raider and companion_operator implement bounded changes, scribe handles docs or operator text, and arbiter reviews risk or acceptance. Captain owns fan-in and passes accepted outputs exactly to the next agent.
- When routing selects a subagent but the host subagent stalls, shuts down, or otherwise fails to produce clean fan-in, the likely cause is a missing or delayed reclaim gate rather than a new execution model. The 0.0.8 docs should say CCC must prefer reclaim, retry, or reassignment before degraded captain fallback whenever delegation is still viable.
- LongWay checklist and status rows should be allowed to show optional owner identity when it helps the operator follow the work, for example `[ ] Mill [ccc_scribe] : Clarify 0.0.8 OMO harness-style requirements`.
- Release-asset packaging and install repair are not doc-only tasks. They should route code or script fixes to `ccc_raider` and release or GitHub mutation to `ccc_companion_operator`.
- Broad release or repair requests should split work across bounded lanes when a single specialist would own too much work. Keep write scopes disjoint, preserve captain fan-in between lanes, and prefer parallel raider/operator lanes for independent code, docs, packaging, and release-surface tasks.
- Release updates must preserve a downgrade path. Keep the previous public pre-release installable, do not delete previous release assets while repairing a newer pre-release, and record whether rollback was completed, skipped as unnecessary, or blocked.
- If the selected role is wrong, CCC should reclassify and replan before mutation instead of mutating under the wrong lane.
- Captain should not directly mutate release assets, install scripts, or GitHub releases unless a terminal specialist failure, stall, or reclaim has been recorded and an explicit fallback or degradation reason is present.
- Token discipline should be explicit in both the `$cap` skill and generated custom-agent prompts: avoid full task-card/status dumps, avoid full-history child forks, pass accepted interpretation/scope/evidence only, and keep fan-in compact.
- LongWay lifecycle visibility should remain auditable without becoming repetitive. Show compact status or LongWay text at initial start, terminal fan-in, merge/reclaim, replan, or closeout boundaries where state changed; avoid repeating unchanged lifecycle blocks.

## Work Items

### 1. Chronological comment and annotation format

- Add guidance to the operator-facing docs that comments and annotations follow the requested chronological block format.
- Treat chronology as a formatting constraint, not as a prompt to summarize or reorder the content.
- If the operator requests a different structure, follow that structure; otherwise preserve chronology when comments or annotations are requested.
- Include these exact block templates in the doc guidance:

```text
#===============================
# {yy-mm-dd}-{git 기준 작업한 사람 이름 or 닉네임} : {task title}
# {yy-mm-dd}-{git 기준 작업한 사람 이름 or 닉네임} : {task title}
# 시간 순으로 간단하게 요약
#================================
```

```text
#===============================
# {yy-mm-dd}-{git 기준 작업한 사람 이름 or 닉네임}-{task title}
# - {task 내용 너무 길지 않게}
#
# {yy-mm-dd}-{git 기준 작업한 사람 이름 or 닉네임}-{task title}
# - {task 내용 너무 길지 않게}
#================================
```

- For code-local comments, place the block directly above the modified function or newly added code.
- Repeat entries in chronological order when the same task receives additional edits; use the same format at another code location when the note belongs somewhere else.

### 2. OMO sisyphus and harness-style operating requirements

- Document this as CCC operating in an OMO sisyphus or harness-style form, not as CCC interoperating with an external OMO or harness process.
- Stay within the existing CCC shape: one public captain, persisted LongWay/task-card state, bounded specialist routing, and all specialist results returning to captain before the next decision.
- Keep the new confirmation loop inside the existing captain-first flow; do not introduce a separate runtime planner or a new agent hop outside captain -> confirmation -> Way/tactician -> specialist -> captain.
- Keep the required visible control surfaces explicit: compact status/LongWay lifecycle visibility, terminal `ccc subagent-update`, structured fan-in, explicit merge/reclaim, and recorded fallback truth.
- Keep MCP as the primary control-plane when available: long fan-in can be artifact-backed under the run directory, and compact-ref updates use `run_id`, `task_card_id`, `event_ref`, and `mode: "compact"` so MCP responses stay summary-only.
- For large multi-surface work, route independent slices to multiple bounded lanes instead of overloading one subagent. Captain remains the fan-in point and decides merge, repair, reclaim, or reassignment after each terminal lane result.
- Require review or validation gates only where risk justifies them; do not turn the note into a new broad runtime architecture.
- Treat missing visibility, silent specialist handoff, unrecorded fallback, or stale late fan-in as the main gaps CCC would need to avoid for this operating style.
- Add an explicit recovery rule: if a routed subagent stalls, becomes terminal, or disappears before fan-in, CCC should reclaim, retry, or reassign first when the delegation is still salvageable, and only then use the visible degraded captain fallback path.

### 3. LongWay row identity visibility

- Allow LongWay checklist and status rows to include optional agent or subagent identity when available, without forcing identity on every row.
- Keep the row compact and readable; identity should be additive metadata, not a second label system.
- Preserve the current simple checklist/status output when no identity is known.

### 4. Release asset repair routing

- Document the observed release-asset repair boundary in the 0.0.8 notes: release asset packaging, `install.sh`/`install.ps1` repair, and `gh release upload`/`gh release edit` are non-trivial mutation or operator-side work.
- Route code or script changes that affect packaging and install behavior to `ccc_raider`, and route GitHub release mutation and asset upload work to `ccc_companion_operator`.
- Keep downgrade safety in scope for every release/install mutation: preserve the previous release, keep explicit `CCC_VERSION` override paths working, and document any temporary public-default rollback or decision not to roll back.
- For complex or risky release/install mutation routing, captain should first confirm the interpretation with the operator before sending planning to Way/tactician, then route the accepted plan to the specialist lane that matches the work.
- Require CCC to reclassify or replan before mutation if the selected role does not match the work, rather than letting the captain mutate immediately.
- Allow captain-side mutation only after a terminal specialist failure, stall, or reclaim has been recorded with an explicit fallback or degradation reason.

### 5. Documentation cleanup

- Update the release docs index so the planned 0.0.8 work is easy to find.
- Keep the note terse and release-note-like so it fits alongside the existing pre-release docs.

### 6. Skill and prompt token discipline

- Keep repo `skills/cap/SKILL.md` and installed skill behavior aligned around compact lifecycle surfaces.
- Use compact status/LongWay lifecycle text at meaningful lifecycle boundaries, not as repeated unchanged output after every status read.
- Keep specialist prompts role-scoped: no full run history, no full task-card/status JSON, no duplicated unchanged acceptance text.
- Split broad work into multiple bounded specialist prompts when scopes can be separated cleanly; avoid asking one agent to perform release rollback, runtime implementation, docs, packaging, upload, and review in a single lane.
- Add a shared generated-agent instruction for token discipline so each managed custom agent avoids full-history dumps and returns short structured fan-in.
- Continue exposing token usage truthfully: use raw totals when supplied, context estimates when available, and explicit unavailable reasons when host custom subagents do not provide raw usage.

## Non-Goals

- Do not rewrite the existing `0.0.7-pre` release note.
- Do not duplicate the full OMO harness validation matrix here.
- Do not turn optional identity display into a required new task-card schema.
- Do not add new runtime behavior outside the confirmation loop, routing guards, stalled recovery, LongWay owner rows, follow-up repair paths, and prompt/skill token discipline described here.

## Acceptance

- The `0.0.8-pre` planning doc states the chronological block format rule for comment and annotation requests.
- The plan reflects the in-scope runtime hardening for the confirmation loop, routing guards, stalled recovery, LongWay owner rows, and follow-up repair paths.
- The docs state that captain must confirm complex or risky request interpretations with the operator before passing accepted work to Way/tactician.
- The docs make role separation and handoff explicit across scout/companion_reader, Way/tactician, raider/companion_operator, scribe, and arbiter, with captain owning fan-in and preserving accepted outputs exactly.
- The docs explain that a stalled routed subagent should trigger reclaim/retry/reassign before degraded captain fallback when delegation is still viable.
- LongWay checklist and status rows can optionally surface owner identity.
- The docs record release-asset packaging, install script repair, and GitHub release mutation as non-trivial work that should route through the correct specialist or operator role first.
- The docs require release/update work to preserve a downgrade path and keep previous release assets installable during newer pre-release repair.
- The docs require broad multi-surface work to split into bounded parallel lanes when scopes can be separated safely.
- The `$cap` skill documents the standalone `ccc checklist --text` LongWay block at lifecycle boundaries and avoids repeated unchanged lifecycle output.
- MCP remains the primary control-plane surface; the checklist command is a stable operator-visible projection over persisted LongWay state, not a replacement orchestrator.
- Generated custom-agent prompts include shared token-discipline guidance and preserve compact structured fan-in.
- The release docs index links to the new `0.0.8-pre` planning doc.
