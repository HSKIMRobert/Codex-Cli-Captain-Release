# 0.0.15 Pre-Release Plan

`0.0.15-pre` should turn the 0.0.14 intent-state-machine work into a
Rust-owned harness layer that feels closer to Oh My Openagent's operator
experience without copying OMO's command surface. CCC remains a Captain-first
Codex orchestration layer: persisted LongWay, task cards, routing, fan-in,
review, and status truth stay inside CCC, and the operator continues to enter
the system through `$cap`.

## Release Goal

0.0.15-pre should improve CCC's harness behavior in three areas:

- make role routing and fallback more explicit
- make task progress, background work, and verification easier to inspect
- make prompts and repo-local guidance more consistent across runs

The release should keep the public surface deliberately small. New behavior may
appear in `$cap` flows, status/projection/app-panel output, setup/check-install
diagnostics, and CCC-managed custom agents, but it should not add new public
skill commands or require the operator to learn an OMO-style command set.

## Hard Invariants

- CCC implementation remains Rust-first for runtime, state, routing, and
  install/check-install behavior.
- `$cap` remains the only public operator entrypoint.
- `ccc_sentinel`/Overseer is a run-scoped guardrail layer inside `$cap`, not
  an always-running host subagent, not a public skill, and not a public
  command.
- Raw operator prompts may pass through an internal run-scoped
  prompt-refinement specialist before LongWay planning or executable task-card
  creation, but Captain remains the owner of final intent, routing, approval,
  fan-in, review, fallback, and completion decisions.
- The prompt-refinement specialist is internal `$cap` machinery, not a public
  skill command or a replacement Captain.
- Prompt-refinement output must be compact and English-only. It is internal
  planning material and does not change the language expected for final
  user-facing responses.
- New mode, routing, hook, background, memory, and visibility behavior is
  internal CCC machinery behind `$cap`.
- Do not add separate user-facing skill commands as part of this release.
- Persisted run state, task cards, LongWay rows, fan-in, and review decisions
  remain the runtime truth.
- Goal Bridge is opt-in internal `$cap` machinery. `[features].goals` defaults
  to `false`, and the implemented slice adds the `[goal_bridge]`
  config/schema/default shape, generated defaults version 16, focused tests,
  and a minimal internal non-executing Captain-owned `ccc.goal_bridge.v1`
  run record when enabled.
- Goal Bridge does not add a public command, skill, or entrypoint.
- Host goal state, when available, is a secondary host aid for guidance and
  coordination, not CCC truth.
- Persisted LongWay rows, task cards, fan-in records, review decisions,
  fallback records, and verification capsules remain authoritative.
- Stable `ccc_*` IDs remain routing truth, and display callsigns are
  display-only.
- `ccc graph` and `ccc_code_graph` stay CCC-owned graph-facing surfaces.
- Graphify-backed Graph Context Provider is the config-gated routing shim
  behind the existing CCC graph-facing surfaces when `graph_context` is
  enabled and Graphify is ready.
- The existing graph-facing surfaces remain stable. When `graph_context` is
  disabled, the runtime still preserves the legacy code graph path.
- If Graphify is unavailable, stale, or missing in enabled `graph_context`
  mode, CCC falls back to normal LongWay/task-card/scout/source evidence flow.
- Graphify output is read-only graph evidence, not CCC source of truth.
- Do not promise a public `set_goal()` API.
- If host goal capability is unavailable, CCC continues from persisted
  LongWay/task-card state only.
- LSP runtime execution remains deferred in `0.0.15-pre`.
- OMO is reference material for useful harness patterns, not a dependency or
  public UX contract.

## Naming Guidance

- New 0.0.15 concepts do not need to keep OMO, Sisyphus, or Boulder names; use
  CCC vocabulary first.
- Prefer names that fit Captain, LongWay, task cards, fan-in, projection/status,
  and the existing StarCraft-inspired role naming direction where it applies.
- Treat OMO names as reference patterns only, not required terminology.

## Planned Work

### 1. Execution Contract Registry With Modes, Cost Tiers, Fallback, And Tool Restrictions

Problem:

- CCC has configured `ccc_*` role agents, but 0.0.15 needs a more explicit
  registry that can answer what each role may do, which modes it supports, and
  how it degrades when a model or provider is unavailable.

Work:

- Add a Rust execution contract registry that records role identity, cost
  tier, mode support, model policy, fallback policy, tool restrictions,
  mutation capability, and review capability.
- Treat the registry as a contract for how configured `ccc_*` roles execute,
  not as an agent generator or public role creation surface.
- Surface registry status through check-install/status so stale or missing
  custom agents are diagnosable.
- Keep execution decisions grounded in persisted run state, not registry hints
  alone.

Reason:

- OMO's dynamic agent construction makes model/runtime policy and tool
  boundaries visible. CCC should gain the same reliability benefit while
  keeping the smaller `ccc_*` role model.

### 2. Category And Skill Delegation Protocol Behind `$cap`

Problem:

- OMO's category/skill routing helps route work to the right specialist, but
  copying its public commands would conflict with CCC's `$cap`-only design.

Work:

- Add an internal routing taxonomy that maps `$cap` intent to category and
  skill, then uses that mapping for task category, risk, mutation intent,
  evidence need, and verification need.
- Let LongWay planning and scheduler decisions use the taxonomy when selecting
  `ccc_scout`, `ccc_scribe`, `ccc_raider`, `ccc_arbiter`, companion readers,
  or companion operators.
- Show the chosen category, skill, and reason compactly in status/projection
  output so the operator can see why the internal route was selected.
- Do not expose direct public commands such as `visual-engineering` or other
  OMO-style public skill verbs.

Reason:

- The operator gets better routing and explanations without needing new public
  commands or extra skills.

### 2a. Prompt Refinement Intake Specialist Behind `$cap`

Problem:

- Raw operator prompts are often underspecified, mixed with background context,
  or phrased as high-level intent.
- If Captain directly interprets and executes such prompts, it can miss
  constraints, skip specialist routing, duplicate work, or start
  implementation before the request is shaped into a CCC task brief.
- Long prompt rewrites can also waste context, so the refinement output must
  stay compact.

Work:

- Add an internal run-scoped prompt-refinement specialist that receives each
  raw `$cap` operator request before LongWay planning or executable task-card
  creation.
- The specialist returns a structured prompt brief for Captain, not an
  execution result.
- The structured brief must be compact and English-only, regardless of the
  operator prompt language.
- The brief should be short enough to fit into repeated Captain routing loops
  without bloating context.
- The structured brief should include only the necessary fields:
  - normalized goal
  - key constraints
  - ambiguity or confirmation needs
  - proposed category/skill
  - likely owner role
  - mutation risk
  - evidence need
  - verification need
  - suggested LongWay/task-card wording
  - forbidden assumptions
- The specialist should avoid verbose explanations, long background
  summaries, and user-facing prose.
- Captain must review and accept, adjust, or reject the refined brief before
  planning or dispatch.
- The prompt-refinement specialist must not directly mutate files, run tools
  for implementation, create final routing decisions, or mark work complete.
- Persist the raw prompt, compact refined brief summary, and Captain
  acceptance/adjustment decision in run state when it affects routing or task
  creation.
- Keep this inside `$cap` and do not expose a public prompt-refinement command.
- The English-only brief must not force the final user-facing response to be
  English; final response language remains governed by Captain/user policy.

Recommended role shape:

- Stable ID: `ccc_promptsmith`
- Config role: `prompt_refiner`
- Display callsign: `Ghost`
- Purpose: compact English prompt refinement, task brief shaping, and
  ambiguity extraction
- Public: no
- Mutation: no
- Runtime role type: run-scoped intake specialist
- Output language: English only
- Output length: compact, bounded, non-verbose

This is a planned/internal role shape and must not be described as a completed
runtime role unless future implementation and tests prove it.

Ghost output contract:

The prompt-refinement specialist returns a compact English-only task brief in a
bounded shape such as:

- Goal:
- Constraints:
- Ambiguities:
- Category/Skill:
- Owner:
- Risk:
- Evidence:
- Verification:
- Task wording:
- Forbidden assumptions:

The specialist must not return long reasoning, chain-of-thought,
implementation output, or user-facing final answers.

Ghost and Overseer have separate responsibilities:

- `ccc_promptsmith` / Ghost shapes raw operator prompts into compact English
  task briefs before planning or task-card creation.
- `ccc_sentinel` / Overseer guards execution, routing, mutation, fallback,
  verification, and lane-conflict boundaries through run-scoped hook checks.

Ghost improves intake quality. Overseer enforces execution safety. Neither role
is public, and neither role replaces Captain ownership.

Reason:

- This gives CCC the same benefit as having an expert prompt engineer between
  the user and the harness, while preserving Captain ownership, avoiding
  context bloat, and keeping the `$cap`-only public surface.

### Graphify-Gated Canonical Graph Context Routing

Document the implemented Graphify-backed Graph Context Provider as the
config-gated routing shim for existing CCC graph-facing surfaces while keeping
the public graph surface stable.

```toml
[features]
graph_context = false

[graph_context]
enabled = false
provider = "graphify"
mode = "read_only"
canonical_backend = "graphify"
replace_legacy_ccc_graph_backend = true
allow_legacy_graph_backend_fallback = false
fallback_when_unavailable = "scout_source_evidence"
report_path = "graphify-out/GRAPH_REPORT.md"
graph_path = "graphify-out/graph.json"
max_report_bytes = 20000
max_query_bytes = 8000
prefer_report_before_grep = true
allow_cli_query = true
allow_mcp_query = false
allow_rebuild = false
auto_install_external_dependency = false
source_of_truth = false

[graph_context.install]
managed_by_ccc_setup = true
check_install_reports_readiness = true
require_graphify_cli_for_queries = true
allow_missing_provider_fallback = true

[graph_context.edges]
allow_extracted = true
allow_inferred = true
allow_ambiguous = false
require_source_check_for_mutation = true
```

- `ccc graph` and `ccc_code_graph` remain CCC-owned graph-facing surfaces; no
  `/graphify`, `@graph`, graph skill, or new public graph command is added.
- Graph context is default-off. When enabled and Graphify is ready, existing
  `ccc graph` / `ccc_code_graph` surfaces route through the Graphify-backed
  provider.
- The provider currently consumes bounded `graphify-out/GRAPH_REPORT.md`
  content and graph metadata only; full `graph.json` content remains
  metadata-only and must not be loaded into prompts by default.
- Focused Graphify query/path/explain outputs are future work unless later
  implementation evidence proves they are available in this slice.
- The Graphify-backed provider is the implemented config-gated graph context
  routing shim for graph-facing surfaces in `0.0.15-pre`, not a new public
  surface.
- Legacy CCC graph backend fallback is not available in enabled
  `graph_context` mode. If Graphify is unavailable, stale, or missing, CCC
  falls back to the normal LongWay/task-card/scout/source evidence flow.
- Graphify output is read-only graph evidence, not CCC source of truth.
  Persisted LongWay rows, task cards, fan-in records, review decisions,
  fallback records, and verification capsules remain authoritative.
- Raider must not mutate based only on inferred, ambiguous, or graph-only
  evidence.
- Arbiter may use graph evidence for impact and risk review, but final
  acceptance still requires exact source or validation evidence.
- Do not reimplement Graphify in Rust for this release.
- The enabled `graph_context` mode stays bounded to read-only existing
  Graphify artifacts plus scout/source evidence fallback, while the disabled
  default keeps the legacy code graph path available.
- LSP runtime execution remains deferred.

Reason:

- This keeps graph-facing public surfaces stable while using a read-only,
  config-gated provider to inform routing and review without treating
  Graphify output as CCC truth or loading full graph artifacts by default.

### 3. Checkpoint And Resume UX

Problem:

- CCC already persists run state, but 0.0.15 should make resume and continuation
  feel more intentional after restarts, stalled workers, or partial completion.

Work:

- Absorb OMO `boulder.json` ideas into CCC run-state so checkpoints capture
  current gate, completed evidence, delegated work, fan-in state, pending
  approval, and next legal action. The Boulder/Sisyphus naming is reference
  material only; the CCC surface should use CCC-native wording.
- Auto-detect active checkpoints when `$cap` starts a new operator turn so
  continuation resumes from persisted CCC truth instead of host-local memory.
- Clarify `$cap continue ...` as a continuation request that resumes from the
  active CCC checkpoint rather than creating a fresh public path.
- Keep stale or late subagent output visible without letting it overwrite the
  chosen lifecycle path.

Reason:

- OMO's Boulder/Sisyphus style is strong because work can continue from a known
  state. CCC should make the same guarantee through LongWay/task-card truth.

### 4. Default Commit Message Guidance

Problem:

- CCC needs a documented default for git commit messages when the user does
  not provide special instructions, so commit output stays Conventional
  Commit-shaped and predictable.

Work:

- Document that CCC should enforce a Conventional Commit-style default through
  delegated prompt/custom-agent guidance for commit-related work, using a
  message such as `fix(hub, worker): 비전 기본 가중치를 metric 0.4 text
  0.6으로 조정` when the user does not supply special commit-message
  instructions.
- Make clear that explicit user instructions for the commit message always
  override the default format.
- Make clear that CCC does not directly change git's behavior or execute
  commits differently; the default is enforced through CCC guidance and
  delegation.

Reason:

- A stable fallback message format improves consistency, while user intent must
  still take precedence whenever it is stated.

### 5. Background Task Lifecycle And Concurrency

Problem:

- Long-running subagents can stall, overlap, or return late. CCC needs a more
  complete lifecycle model so background work is visible and bounded.

Work:

- Track `pending -> running -> completed/error/cancelled/reclaimed` background
  task states with compact operator-facing summaries.
- Add per-model and per-provider concurrency limits, reclaim thresholds, and
  stale-detection rules that are visible in status.
- Ensure terminal states release handles or stay marked as stale/reclaimed
  rather than blocking later work invisibly.

Reason:

- OMO treats background agent work as a first-class harness concern. CCC should
  keep the same discipline around fan-in and resource pressure.

### 6. Lifecycle Hook Tiers

Problem:

- CCC needs predictable extension points without letting hooks become a second
  command system.

Work:

- Define Rust-owned event hook tiers for recovery, compaction, tool guard,
  continuation, and notification lifecycle points, plus the existing
  planning/fan-in/review/reporting boundaries where needed. Treat the hook
  layer as an always-applied run-scoped Sentinel guardrail layer inside
  `$cap`, not as an always-running host subagent or a public command surface.
- Hydrate the guardrail context on every run and every hook evaluation with the
  current LongWay/task-card state, owner role, allowed transitions, fallback
  policy, and any prior sentinel intervention state.
- Keep hooks internal to CCC policy/config and avoid user-facing hook commands
  in 0.0.15.
- Record hook decisions, skips, and failures in status when they affect
  routing, mutation, or verification, including `sentinel_intervention` or a
  compact equivalent in persisted state when the guardrail intervenes.

Reason:

- OMO's hook structure is useful for harness consistency. CCC should adopt the
  lifecycle concept while keeping operator UX centered on `$cap`.

### 6a. Sentinel/Overseer Guardrail For Captain-Local Work

Problem:

- `ccc_sentinel`/Overseer should guard Captain-local work as an always-applied
  run-scoped guardrail layer inside `$cap`, but it must not become an
  always-running host subagent, a public skill, or a public command.

Work:

- Route guardrail decisions through stable `ccc_*` IDs as the routing truth;
  treat StarCraft callsigns as display-only labels in operator-facing text.
- Hydrate guardrail instructions on every run and every hook evaluation with
  current LongWay/task-card state, owner role, allowed transitions, fallback
  policy, and the last persisted sentinel outcome.
- Use `Observer`/`Probe` for read-only work, `Adjutant` for docs/release-note/
  operator text, `Marauder` for code/config mutation or repair, `Arbiter` for
  review/risk/regression, `Overseer` for lane conflict, fallback, and ownership
  guard, and `SCV` for bounded git/gh mutation.
- Apply `observe`, `warn`, and `enforce` phases: observe for low-risk
  read-only or clearly owned work, warn when Captain-local work drifts toward
  specialist-owned code/config/release/test-repair/docs-routing-drift cases,
  and enforce when the work attempts dangerous mutation completion or
  verification bypass.
- Record Captain-local direct-work detection and any guardrail intervention as
  `sentinel_intervention` or a compact equivalent in persisted state,
  including explicit fallback/degradation reasons when Captain-local fallback
  is allowed.
- Block dangerous mutation completion and verification bypass in enforcement,
  and require an explicit persisted fallback/degradation reason before any
  Captain-local fallback path is accepted.
- Connect the hook tiers as follows:
  - `UserPromptSubmit`: hydrate the guardrail context, classify route
    ownership, and decide whether the run should stay in observe, move to
    warn, or require enforcement before planning continues.
  - `PreToolUse`: stop unsafe tool execution, block specialist-owned direct
    work that is trying to stay Captain-local, and require explicit persisted
    fallback/degradation reasons before any deviation from the owned route.
  - `PostToolUse`: persist the hook result, intervention state, and verification
    impact, then re-evaluate whether the next step still matches the allowed
    transition map.
  - `Stop`: finalize the run-scoped guardrail outcome, carry forward
    unresolved warnings, and ensure any bypass attempt or missing verification
    remains visible in status/projection.

Reason:

- CCC needs a guardrail layer that can explain when Captain should step back,
  while still keeping the public surface centered on `$cap` and persisted CCC
  truth.

### 7. Config Schema And Check-Install Visibility

Problem:

- As role registry, fallback, hooks, and prompt composition grow, setup and
  check-install must explain whether the local install is current and coherent.

Work:

- Extend a `ccc-config.toml`-like schema and version checks for registry,
  category routing, fallback policy, concurrency, prompt sections,
  directory-rule injection, hook settings, and custom-agent sync.
- Make `ccc check-install` validate those settings and report missing/stale/
  conflict states for every 0.0.15 surface.
- Keep user-owned config preservation, dry-run migration, backup, rollback, and
  restart guidance visible.

Reason:

- A richer harness is only useful if the operator can quickly tell whether the
  installed `$cap`, MCP server, and custom agents match the expected runtime.

### 8. Task And Session Visibility Surfaces

Problem:

- Codex CLI currently shows raw MCP calls, and CCC's richer state is easier to
  inspect through status/projection than through the default tool transcript.

Work:

- Improve compact status, projection, and app-panel payloads for active task,
  current gate, delegated agent, model, fallback state, evidence, verification,
  unresolved risk, and internal task/session state.
- Treat MCP and CLI support surfaces as internal CCC plumbing for task,
  session, and status inspection.
- Add watch/tmux-friendly text output if it can be done without adding another
  public command path beyond existing CCC lifecycle/status surfaces.
- Treat true Codex CLI side-panel rendering as host-dependent. CCC can provide
  structured panel data, but the only user-visible operator surface remains
  `$cap` plus status/projection output.

Reason:

- OMO/opencode-like side visibility is valuable, but CCC should first own the
  data model and compact rendering. Host UI integration can consume that later.

### 8a. Natural CCC Workflow Loop Projection

Problem:

- Operators reported that `0.0.14-pre` had a natural visible loop:
  requirements understanding -> planning -> exploration -> modification ->
  review -> verification. The `0.0.15-pre` harness work added richer hook,
  checkpoint, task-session, and lifecycle details, but the loop itself could be
  hard to see as one operator-facing projection.

Work:

- Add a compact Rust-owned `workflow_loop` projection derived from persisted
  LongWay/task-card truth.
- Surface the fixed loop in status text, projection output, compact status,
  and the app-panel payload without changing specialist routing or adding a new
  public command.
- Keep lifecycle hooks internal and separate from this operator-facing loop:
  hooks explain policy boundaries, while `workflow_loop` explains the natural
  work progression.

Reason:

- The richer `0.0.15-pre` state should still read like a coherent CCC work
  loop, not only as individual lifecycle artifacts.

### 8b. Deferred Follow-Up Themes

Pending themes to keep easy to summarize:

1. Hooks-based guardrails: use `UserPromptSubmit`, `PreToolUse`,
   `PostToolUse`, and `Stop` style lifecycle hooks to harden the run-scoped
   Sentinel/Overseer guardrail layer so routing drift, unsafe commands,
   Captain-local execution of specialist-owned work, missed verification, and
   missing `$cap`/CCC loop behavior are caught before they escape persisted
   CCC state.
2. Compact evidence/output: compress validation and tool output while
   preserving raw artifacts and log paths for audit. This is RTK-inspired
   discipline, not a commitment to adopt RTK wholesale.
3. App-server/remote status exposure: strengthen CCC status, projection,
   fan-in, and release-validation surfaces for app-server and remote workflows.

### 9. CCC Plugin Packaging For MCP Distribution

Problem:

- CCC currently has Rust-owned runtime, MCP, and skill assets, but 0.0.15-pre
  also needs a packaging track so the CCC MCP can be distributed as a Codex
  plugin without changing CCC's public command contract. The plugin direction
  should make CCC feel like a repeatable Sisyphus-like work loop, not merely a
  bundle of MCP tools. UI is secondary; the key is a fixed loop:
  user request -> `cap start` -> plan creation -> task breakdown -> execute ->
  review -> bounded retry/replan on failure -> final summary.

Work:

- Define the CCC Plugin as the packaging unit for `CCC skill + CCC MCP config +
  plugin manifest`.
- Encode the bundled plugin Skill so Codex is instructed to follow the CCC
  loop instead of starting direct implementation first: start a CCC run, create
  a plan, check status while progressing, pass the review gate after changes,
  apply bounded retry/replan on failure, and surface concise phase/role/result
  updates to the user.
- Use a proposed plugin-root layout where the plugin root may include
  `skills/`, `.mcp.json`, `hooks/`, and `assets/`, while only
  `.codex-plugin/plugin.json` lives under `.codex-plugin/`.
- Keep manifest paths plugin-root relative and `./` prefixed so the plugin
  metadata stays portable.
- Add plugin manifest work for `.codex-plugin/plugin.json` and wire
  `mcpServers` to point at `.mcp.json`.
- Define `.mcp.json` launcher work for stdio startup, including support for a
  direct server map or a wrapped `mcp_servers` payload, and verify the MCP
  stdio/tool schema is ready for plugin install use.
- Bundle `skills/ccc/SKILL.md` as the plugin-authored skill asset while
  keeping skills as the authoring format and the plugin as the installable
  distribution unit.
- Validate the package with a local marketplace/install test through the Codex
  plugin workflow.
- Treat the versioned `ccc@ccc-local` plugin-cache MCP binary as an accepted
  install-surface registration for the same `0.0.15-pre` runtime, while keeping
  wrong versions, commands, or args stale.
- Document the plugin packaging boundary: CCC's public entry remains `$cap`,
  and the plugin experience is a distribution/install surface, not a
  replacement command strategy.
- Update install README/release docs to explain the packaging, install path,
  and operator-facing limits.

Benefits:

1. Codex recognizes CCC as a workflow, not just a bundle of tools.
2. Installation and activation become cleaner through `/plugins` instead of
   manual MCP copy-paste.
3. `cap` keeps a fixed meaning as the CCC loop entrypoint.
4. Plugin versioning can bundle the Skill, MCP config, logo/assets, and
   descriptions together.
5. Future extension can add optional `cap-review`, `cap-debug`, or
   `cap-plan` skills later, but not as a 0.0.15 public command expansion.

Reason:

- CCC should be distributable through the Codex plugin mechanism while keeping
  the Captain-first public contract intact and avoiding any new public skill
  command surface.

### 10. 0.0.15-pre Extra Work And Validation Notes

These are the extra release-prep items that need to stay visible in the plan
alongside the main work tree:

- Propagate the StarCraft-style display callsign mapping to the source
  READMEs, the release README trio, the release notes, and the SSL manifests
  so the display-only mapping stays consistent everywhere, with the role
  tables making the stable-ID/display-callsign split explicit.
- Keep the locally applicable oh-my-openagent-inspired workflow/skill list
  explicit in the schema, skill manifests, release notes, and release README
  content, including the explicit LSP safe-refactor workflow where it applies.
- Preserve optional TypeScript/JavaScript LSP setup guidance for
  `typescript-language-server`/`typescript`, and keep `rust-analyzer` guidance
  Rust-only and optional.
- Keep the LSP runtime claim truthful: `0.0.15-pre` provides config/schema/
  manifest metadata for `lsp_diagnostics`, `lsp_references`, `lsp_definition`,
  `lsp_prepare_rename`, and `lsp_rename`, but CCC does not start language
  servers in this release. See `docs/release-work/0.0.15/LSP_MVP_DEFERRED.md`
  for the bounded deferred design.
- Clarify that host UI notifications such as `Closed Carver [ccc_scout]` are
  outer-layer artifacts if they appear, while CCC-controlled status/projection
  text uses `Callsign(stable_id)` forms like `Observer(ccc_scout)`.
- Add agent parallelization as part of the `0.0.15-pre` scope itself, not as a
  staged check-first-then-later plan: implement the full set of parallel-task
  decomposer/task graph, read-only fan-out, path conflict guard, fan-in
  aggregator, Sentinel routing, Captain-local-work guardrail,
  lane-conflict classification, Codex hooks guardrail integration, worktree
  isolation for parallel mutation, parallel and specialized review lanes,
  app-server/remote visibility for parallel state, and focused
  tests/acceptance while keeping stable `ccc_*` role IDs, Sentinel
  interventions, and StarCraft-style callsigns consistent.
- Future work only, not implemented in `0.0.15-pre`: prefer StarCraft-style
  display callsigns such as `[Marauder]` and `[Observer]` for operator-visible
  bracket labels like `[ccc_raider]` and `[ccc_scout]` where CCC can influence
  the presentation, while keeping stable `ccc_*` IDs as the machine-readable
  routing truth. If these display changes are implemented later, the source
  README and release README surfaces may also need updates so user-facing docs
  match the new output.
- Future work only, not implemented in `0.0.15-pre`: prefer compact host
  command framing such as `Ran ccc check-install --text` instead of the full
  absolute-path form when CCC can influence the run-log presentation. Stable
  `ccc_*` IDs must remain unchanged, and any later change needs acceptance
  that distinguishes CCC-controlled text from host-rendered log framing.
- Future work only, not implemented in `0.0.15-pre`: improve the routing
  classifier for cases where the only target is documentation or release notes
  so docs-only work routes to `Adjutant(ccc_scribe)` instead of
  `Marauder(ccc_raider)`. This routing change is separate from the display
  label and run-log wording work. Stable `ccc_*` IDs must remain unchanged,
  and the acceptance should verify that docs/release-note-only prompts no
  longer drift into the mutation path.
- Keep `check-install` wording aligned with optional setup surfaces so it reads
  as readiness/installation guidance rather than runtime LSP activation, and
  keep the Rust-only `rust-analyzer` surface explicitly deferred at runtime
  unless later docs say otherwise.
- Future work only, not implemented in `0.0.15-pre`: make the Rust LSP
  contract executable at runtime in a later release. That later work should
  add rust-analyzer process/session management, request wrappers for
  diagnostics, definition, references, prepareRename, and rename, setup and
  check-install readiness checks for rust-analyzer availability, safe
  rollback/fallback when rust-analyzer is missing or unusable, and focused
  tests/acceptance for the runtime bridge. The current `0.0.15-pre` truth
  stays metadata-only and does not start language servers.
- Restore the natural CCC workflow-loop visibility from `0.0.14-pre` by
  projecting requirements understanding, planning, exploration, modification,
  review, and verification as one compact status/app-panel surface.
- Validate the doc/config sync with focused `rg` checks, `git diff --check`
  over touched files, and a GitHub release card/content check for
  `v0.0.15-pre`.

## Additional OMO-Inspired Items

### Model Fallback Policy

Add ordered model and role fallback policy to the CCC role registry. Each
fallback should persist the attempted role/model, selected fallback,
unavailable reason, and degraded capability note.

Reason:

- Provider/model failure should produce visible degraded execution instead of
  ambiguous role drift or silent captain-local fallback.

### Prompt Section Composition

Build specialist prompts from named sections such as identity, task, routing,
hard blocks, evidence, verification, anti-duplication, and reporting.

- Add an internal `ccc_promptsmith`/Ghost intake shape for prompt refinement.
  The intake brief should be compact and English-only, Captain retains
  ownership of the work, and Ghost stays internal rather than becoming a
  public command, skill, or user-facing agent.
- Have Ghost surface idle or unfinished host-subagent observations in the same
  compact intake brief so Captain can follow up and close them normally.
- Keep Ghost prompt refinement from changing the final response language; it
  only normalizes the task brief that feeds Captain-owned planning and
  delegation.

Reason:

- OMO's dynamic prompt sections reduce drift. CCC can make prompts easier to
  maintain and review by composing stable Rust-owned sections instead of
  growing monolithic prompt strings.

### Prompt Refinement Intake Specialist

Document `ccc_promptsmith`/Ghost as a partially implemented internal
run-scoped intake specialist behind `$cap`. The current runtime slice only
persists prompt-refinement plumbing and surfaces it in status text; the
feature stays default-off and non-executing. Ghost/model handoff, Captain
accept/adjust/reject gating, downstream LongWay/task-card creation, and any
normal closure action remain future work unless later runtime evidence and
tests prove them.

- Stable ID: `ccc_promptsmith`
- Config role: `prompt_refiner`
- Display callsign: `Ghost`
- Public: no
- Mutation: no
- Output: compact, bounded, English-only task brief

This is a partially implemented runtime slice and must not be described as a
completed runtime role. Ghost does not replace Captain, does not create final
routing decisions, does not dispatch implementation, does not mutate files,
does not directly close agents, and does not mark work complete. Its
English-only brief is internal planning material and does not change the final
user-facing response language.

Reason:

- Prompt intake quality should improve before planning starts, but `$cap` must
  remain the only public operator entrypoint and Captain must retain final
  ownership.

### Agent Parallelization And Review Lanes

Treat agent parallelization as a first-class `0.0.15-pre` addition, not a
later staged experiment. The intended work is the full implementation set:
parallel-task decomposer and task graph, read-only fan-out, path conflict
guard, fan-in aggregator, Sentinel routing, Captain-local-work guardrail,
lane-conflict classification, Codex hooks guardrail integration, worktree
isolation for parallel mutation, parallel and specialized review lanes,
app-server/remote visibility for parallel state, and focused
tests/acceptance. Keep stable `ccc_*` role IDs and StarCraft-style callsigns
aligned across the plan and release-note surfaces.

Reason:

- Parallel work only helps if the decomposition, mutation boundaries, routing
  conflicts, review lanes, and visibility surfaces are all designed together
  and validated together.

### Directory Rules Injection

Before LongWay generation or specialist dispatch, summarize nearby repo-local
rules such as `AGENTS.md`, `README`, and directory-specific guidance when they
are relevant to the target paths.

Reason:

- Repo-local rules should influence planning before work is delegated. This
  improves compliance without making the operator manually restate local
  constraints.

### Verification Capsule

Require completed work to attach a concise capsule containing acceptance,
evidence, reviewer verdict when applicable, validation commands or checks, and
unresolved risk.

Reason:

- The final answer and fan-in surface should make it obvious why CCC believes a
  task is done, what was checked, and what risk remains.

### Wisdom Promotion

Promote only verified, repeated, generally useful lessons into future run
guidance or memory. Do not promote every memory-like observation.

Reason:

- OMO-style accumulated wisdom is useful only when it is filtered. CCC should
  avoid turning one-off failures, stale assumptions, or unverified preferences
  into durable guidance.

### Anti-Duplication Discipline

Persist delegated search and mutation ownership in status and generated
prompts. Captain should not repeat a subagent's assigned exploration unless the
run records a reclaim, stale output, or explicit reason.

Reason:

- Parallel agents lose value when Captain duplicates their work. The status and
  prompt surfaces should make delegated scope clear enough to avoid repeated
  searches and conflicting mutations.

### Sentinel / Overseer Guardrail Layer

Add a run-scoped `ccc_sentinel`/Overseer guardrail layer that is applied inside
`$cap` on every run and hook evaluation, keeps `ccc_*` IDs authoritative, and
records persisted intervention state whenever Captain-local work needs to
yield to a specialist or a blocked transition. Keep it internal to `$cap`
rather than treating it as an always-running host subagent, a public command,
or a public skill.

Reason:

- OMO-style guardrails are only useful when they are explicit, stateful, and
  separate from public commands.

### Goal Bridge And Role-Specific Subgoals

Document Goal Bridge as opt-in internal `$cap` machinery with accepted
subgoal context only, while keeping host goal state secondary to persisted
CCC truth. The implemented runtime slice now covers the config gate,
schema/default shape, generated defaults version 16, focused tests, and the
minimal internal non-executing Captain-owned `ccc.goal_bridge.v1` run record
when Goal Bridge is enabled.

```toml
[features]
goals = false

[goal_bridge]
enabled = false
mode = "captain_owned"
brief_language = "en"
brief_max_lines = 12
require_verifiable_stop = true
host_goal_state_is_truth = false

[goal_bridge.specialists]
allow_specialist_goal_context = true
allow_specialist_set_goal = false
allow_specialist_clear_goal = false
allow_specialist_override_goal = false
max_subgoal_lines = 8
require_captain_acceptance = true
```

- Goal Bridge is internal `$cap` machinery and does not add a public command,
  tool, skill, replacement entrypoint, or public `set_goal()` API.
- `[features].goals` defaults to `false`, and the runtime slice is backed by
  the `[goal_bridge]` config/schema/default shape.
- Host goal state, when available, is only a secondary host aid for guidance
  and coordination; CCC truth stays in persisted LongWay rows, task cards,
  fan-in records, review decisions, fallback records, and verification
  capsules.
- The implemented slice records a minimal internal `ccc.goal_bridge.v1` run
  record only when enabled, and that record remains Captain-owned and
  non-executing.
- Specialists may receive accepted subgoal context in the future, but that
  injection is still runtime-gated and not implemented in this slice.
- Specialists may not set, clear, or override host goal state directly.
- Host goal capability integration, host goal mutation, fan-in goal
  annotations, and subgoal context injection remain future/runtime-gated
  work.
- Stable `ccc_*` IDs remain routing truth, and display callsigns stay
  display-only.
- LSP runtime execution remains deferred in `0.0.15-pre`.

Reason:

- This keeps Goal Bridge as a bounded host aid for goal shaping while
  preserving persisted CCC truth and the `$cap`-only public surface.
- Sentinel stays internal to `$cap`; it is not a public command or a
  standalone skill.

## 0.0.15-Pre Progress Tracker

- [x] Keep Rust-first implementation, `$cap` as the only public entrypoint, no
  new public skill commands, and existing naming guidance intact.
- [x] Foundations and invariants: confirm run-state truth, persisted LongWay,
  and CCC-owned lifecycle boundaries.
- [x] Execution contract registry: define role capabilities, fallback, tool
  restrictions, and mutation/review boundaries.
- [x] Category and skill routing: map `$cap` intent to internal routing,
  reason codes, and specialist selection.
- [x] Graphify-backed config-gated graph routing shim design: document the
  Graphify provider routing, stable CCC graph-facing surfaces, and read-only
  evidence model.
- [x] Graphify-backed config-gated graph routing shim implementation: wire the
  provider, readiness visibility, default-off legacy compatibility,
  scout/source fallback, and bounded read-only Graphify artifact use without
  reimplementing Graphify in Rust.
- [x] Checkpoint and resume: capture gates, delegated work, fan-in, and next
  legal action for restart continuity.
- [x] Background lifecycle and concurrency: track task states, stale output,
  reclaim rules, and provider/model limits.
- [x] Lifecycle hook tiers: document recovery, compaction, guard,
  continuation, and notification hook design/readiness/surface.
- [ ] Lifecycle hook enforcement: implement `UserPromptSubmit`,
  `PreToolUse`, `PostToolUse`, and `Stop` enforcement only if runtime/test
  evidence exists.
- [x] Prompt refinement intake design: document an internal run-scoped
  prompt-refinement specialist that turns raw `$cap` prompts into compact
  English CCC task briefs, observes idle or unfinished host subagents for
  Captain follow-up, and does not add a public command.
- [x] Prompt refinement intake implementation: the first runtime plumbing
  slice is in place (`ccc.prompt_refinement.v1` metadata/status,
  `prompt_refinement_handoff_decision` envelope, status/text surfacing,
  schema coverage, focused tests, default-off non-executing state, and
  captain-owned gating fields such as `owner=captain`,
  `captain_gate=accept_adjust_reject`,
  `longway_materialization_allowed=false`, and
  `task_card_creation_allowed=false`). The envelope records pending Captain
  accept/adjust/reject semantics but remains internal and non-executing.
  Actual Ghost/model dispatch, refined-brief materialization, and downstream
  LongWay/task-card materialization remain future/runtime-gated work.
- [x] Goal Bridge design: document an opt-in internal `$cap` goal bridge,
  `[features].goals` defaulting to `false`, host goal state as a secondary
  host aid, and persisted CCC truth as authoritative.
- [x] Goal Bridge implementation: wire the opt-in goal bridge config/schema
  shape, generated defaults version 16, focused tests, and the minimal
  internal non-executing Captain-owned `ccc.goal_bridge.v1` run record while
  keeping public commands/tools/skills, a replacement entrypoint, public
  `set_goal()`, specialist host-goal mutation, and host-goal capability
  integration out of this slice.
- [x] Sentinel/Overseer guardrail design: document run-scoped guardrail
  semantics, hook mapping, observe/warn/enforce phases, and internal-only
  boundary.
- [x] Sentinel/Overseer guardrail implementation: enforce
  captain-local-work checks, persisted intervention state, context hydration,
  and focused tests. The generated config schema declares
  `agents.sentinel/defaults`, run/task-card schemas persist
  `sentinel_intervention` fields/history/latest entry trace, and
  `subagent-update --text` renders a compact Sentinel line.
- [x] Config schema and check-install: validate config, registry sync,
  fallback policy, hooks, prompts, and custom-agent readiness.
- [x] Task, session, and status visibility: surface compact current-state,
  delegation, fallback, evidence, and risk details.
- [x] Natural workflow-loop visibility: status/projection/compact/app-panel
  output surfaces requirements understanding -> planning -> exploration ->
  modification -> review -> verification from persisted LongWay/task-card
  truth.
- [ ] Deferred follow-up themes: hooks-based guardrails, compact
  evidence/output, and app-server/remote status exposure remain future work.
- [x] Verification capsule, wisdom, and anti-duplication: require evidence,
  promote only verified lessons, and prevent repeated delegated work.
- [x] CCC plugin packaging: define the plugin manifest, `.mcp.json` launcher,
  bundled `skills/ccc/SKILL.md`, and install/marketplace validation path.
- [x] CCC plugin workflow-loop instruction: encode the fixed CCC loop in the
  bundled Skill, keep UI secondary, and preserve bounded retry/replan behavior.
- [x] Docs and release gates: keep release docs, operator guidance, and
  readiness gates aligned with the current CCC surfaces, including Rust-first
  runtime, `$cap`, `ccc_*` routing, checkpoint/resume, concurrency/background
  lifecycle, hooks, verification, workflow-loop Skill guidance, plugin
  packaging files.
- [x] Default commit message guidance: document a Conventional Commit-style
  fallback for git commits when the user does not provide special instructions,
  while preserving explicit user override.
- [x] Validation: run the agreed checks and confirm the checklist is reflected
  in status/projection and release notes.
  - [x] Extra doc/config validation: confirm the callsign mapping,
    workflow/skill list, optional TypeScript/JavaScript plus Rust LSP
    guidance, table clarity, and host UI limitation wording are consistent
    across source docs, release docs, schema, and SSL manifests.
  - [ ] Public release card sync: parent/operator owns GitHub release-card
    review and publication after source review.

## Validation Record

- Date: 2026-05-08
- Release: `0.0.15-pre` validation slice over local `0.0.14-pre` runtime/install metadata
- Environment: macOS arm64 source checkout, `/Users/kwkim-hoir/dev/home/Codex-Cli-Captain`, dirty accumulated 0.0.14/0.0.15 worktree on `main`
- CCC run id: `b9440ce2-206e-378d-d814-2b7fa1802f4e`
- Scratch smoke run id: `7414e956-03b5-7d03-9932-2d59522c6ce6`
- Branch caveat: local `stable-v0.0.14-pre` exists and points at `origin/main`/`cc0e386379f8c2c4f5827a478f20c7ae4e0c1bc8` because the source repo has no `v0.0.14-pre` tag or dedicated source ref.
- Commands:
  - `git status --short --branch`
  - `git branch --show-current`
  - `git rev-parse stable-v0.0.14-pre origin/main HEAD`
  - `cargo fmt --all --check`
  - `git diff --check`
  - `jq empty .codex-plugin/plugin.json .mcp.json`
  - `rg -n "new public skill command|public commands beyond|\$cap remains|only public|public entry" README.md docs/install.md docs/release/README.md docs/release-work/0.0.15/PRE_RELEASE_PLAN.md docs/release/VALIDATION_RUNBOOK.md skills/ccc/SKILL.md`
  - `rg -n "checkpoint|resume|concurrency|background|hook|verification capsule|workflow loop|plugin packaging|Default commit|default commit|Conventional Commit" docs/release-work/0.0.15/PRE_RELEASE_PLAN.md docs/release/VALIDATION_RUNBOOK.md docs/release/README.md README.md docs/install.md skills/ccc/SKILL.md`
  - `cargo check -p ccc --offline`
  - `cargo test -p ccc --offline`
  - `cargo build --offline`
  - `scripts/release/verify-windows-install-smoke.sh`
  - `scripts/release/verify-install-pruning.sh`
  - `scripts/release/verify-release-asset-matrix.sh`
  - `ccc --version`
  - `ccc check-install`
  - `ccc server-identity`
  - `ccc setup --dry-run`
  - `ccc status --json '{not valid}'`
  - `ccc start --quiet --json '{"prompt":"scratch 0.0.15 validation","title":"0.0.15-validation-smoke","intent":"validate control plane","goal":"confirm status and guard surfaces","scope":"scratch run only","acceptance":"done when status and activity surfaces are readable","task_kind":"way","compact":true}'`
  - `ccc status --text --json '{"run_id":"7414e956-03b5-7d03-9932-2d59522c6ce6"}'`
  - `ccc activity --json '{"run_id":"7414e956-03b5-7d03-9932-2d59522c6ce6","compact":true}'`
  - `ccc orchestrate --quiet --json '{"run_id":"7414e956-03b5-7d03-9932-2d59522c6ce6","progression_mode":"single_step","compact":true}'`
- Actual result:
  - Source checks passed: format, diff whitespace, offline check, offline build, and `cargo test -p ccc --offline` (`314` unit tests plus `5` plugin package tests).
  - Plugin/package structure passed: `.codex-plugin/plugin.json` and `.mcp.json` parse as JSON; focused plugin package tests passed; docs grep confirmed `$cap` remains the public entrypoint.
  - Release scripts passed: Windows install smoke used the static `install.ps1` path because PowerShell is unavailable; install pruning passed; asset matrix passed after rerunning with approved filesystem access to replace local sibling release-repo tarballs.
  - Installed local surface passed: `ccc --version` reported `0.0.14-pre`; `ccc check-install` reported `status=ok`, matching MCP registration, current packaged `$cap` skill, synced custom agents, and no restart required; `ccc setup --dry-run` wrote no files.
  - Negative JSON path passed by failing cleanly with `Invalid JSON for status: key must be a string at line 1 column 2`.
  - Scratch status/projection smoke passed: run `7414e956-03b5-7d03-9932-2d59522c6ce6` stayed readable, exposed the planning approval gate, and kept orchestration blocked at `await_longway_approval` until explicit approval.
- Deferred/not run:
  - No live GitHub release lookup, public installer download, live install, push, tag creation, remote release creation, or post-restart live bundle validation was run in this slice.
  - No branch switch was performed.
- Decision: the requested local `0.0.15-pre` validation item is complete. Public release completion remains gated on the later live publication/download/install/restart checks.

### Follow-up Natural Workflow Loop Validation

- Date: 2026-05-08
- Requirement: restore/surface the `0.0.14-pre` natural loop for
  requirements understanding -> planning -> exploration -> modification ->
  review -> verification in `0.0.15-pre`.
- Implementation status: implemented as a Rust-owned `workflow_loop`
  projection over persisted LongWay/task-card truth and exposed through
  status text, projection text, compact status, and app-panel payloads.
- Commands:
  - `cargo fmt --check -p ccc`
  - `cargo test -p ccc ccc_status_surfaces_natural_workflow_loop_projection --offline`
  - `cargo test -p ccc ccc_status_surfaces_task_session_state_for_watch_text --offline`
  - `cargo test -p ccc ccc_status_surfaces --offline`
  - `git diff --check`
- Result: passed. Public release card sync remains parent/operator-owned.

## Acceptance Checklist

- A 0.0.15 execution contract registry exists in Rust and can explain cost
  tier, mode, model, fallback, and tool/mutation restrictions for each
  configured `ccc_*` role.
- `$cap` remains the only documented public operator entrypoint.
- No new public skill commands are introduced.
- `ccc_sentinel`/Overseer is documented as an always-applied run-scoped
  guardrail layer inside `$cap`, with stable `ccc_*` IDs as routing truth and
  display-only StarCraft callsigns. It is not a public command, skill, or
  always-running host subagent.
- `ccc_promptsmith`/Ghost prompt refinement is a partially implemented
  internal runtime slice. The current plumbing is config-only: generated
  defaults version 16 sets `[features].prompt_refinement = false`, bootstrap
  uses config-derived enabled state, and the feature persists
  `ccc.prompt_refinement.v1` metadata/status plus an internal
  `prompt_refinement_handoff_decision` envelope while staying default-off and
  non-executing. The envelope records pending Captain accept/adjust/reject
  semantics. Actual Ghost/model dispatch, refined-brief materialization,
  LongWay/task-card materialization, and direct closure handling remain
  future/runtime-gated work unless later evidence proves them.
- Ghost-hosted idle or unfinished subagent reporting remains future/runtime-
  gated unless runtime evidence and tests prove it; Ghost does not directly
  close agents.
- Raw `$cap` operator prompts are not yet routed through an executing Ghost
  model. The intended future flow is an internal prompt-refinement specialist
  before LongWay planning or executable task-card creation when enabled, but
  current release work stops at config/persistence plumbing and default-off
  gating.
- The prompt-refinement specialist is intended to return a compact
  English-only task brief containing goal, constraints, ambiguity, proposed
  category/skill, likely owner role, mutation risk, evidence need,
  verification need, task wording, and forbidden assumptions, with any
  resulting handoff decision recorded in the internal
  `prompt_refinement_handoff_decision` envelope before future/runtime-gated
  materialization.
- The prompt-refinement specialist output is bounded and non-verbose so
  repeated Captain routing loops do not bloat context.
- The prompt-refinement specialist does not emit long reasoning,
  chain-of-thought, or user-facing response drafts.
- Captain must accept, adjust, or reject the refined brief before creating
  LongWay rows or executable task cards once that future gating is enabled.
- The prompt-refinement specialist cannot mutate files, dispatch
  implementation work, directly close agents, mark tasks complete, or replace
  Captain routing/fan-in/review/closure ownership.
- The raw prompt, compact refined brief summary, and Captain
  acceptance/adjustment decision are persisted when they affect routing or
  task creation.
- Prompt refinement remains internal to `$cap`, keeps `$cap` as the only
  public entrypoint, and does not add a public skill command.
- The English-only internal brief does not override user-facing response
  language policy.
- Sentinel/Overseer guardrail remains separate from prompt refinement:
  `ccc_promptsmith` shapes prompt intake, Overseer guards
  execution/routing/verification boundaries.
- Goal Bridge is opt-in, default-off internal `$cap` machinery in CCC config.
- Enabling Goal Bridge does not add a public command, public skill, or
  replacement entrypoint for `$cap`.
- Ghost may produce a compact English goal brief as future/runtime-gated
  design, but Captain must accept, adjust, or reject it before goal context is
  used.
- Captain may derive role-specific subgoal context for specialists when the
  feature is enabled.
- Specialists may use accepted subgoal context but must not set, clear, or
  override host goal state directly.
- Host goal state is never treated as CCC truth.
- Persisted LongWay, task cards, fan-in, review decisions, fallback records,
  and verification capsules remain authoritative.
- Fan-in can report goal alignment, subgoal completion, evidence, and
  remaining risk.
- If host goal capability is unavailable, CCC continues with LongWay/task-card
  state only.
- `set_goal()` or equivalent host goal capability must not be documented as a
  guaranteed public API.
- LongWay planning can use internal category/skill/risk/evidence/verification
  routing signals and explain the chosen route in status/projection.
- Resume/status output auto-detects active checkpoints, shows current gate,
  active background work, stale or reclaimed subagents, fallback state, and
  next legal action.
- Sentinel implementation currently scopes to `subagent-update` guardrail
  classification, persistence, compact status/projection surfacing, and
  focused tests.
- `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, and `Stop` hook enforcement
  remains future/deferred unless runtime/test evidence exists.
- `ccc_raider`-owned mutation tasks are warned or blocked if Captain tries to
  complete code/config mutation without a fallback reason.
- Docs-only or release-note-only tasks that drift to mutation path are routed
  back through `Adjutant(ccc_scribe)` and recorded as routing drift.
- `ccc graph` and `ccc_code_graph` remain stable CCC-owned graph-facing
  surfaces; there is no `/graphify`, `@graph`, graph skill, or new public
  graph command.
- Graphify-backed Graph Context Provider is the implemented config-gated
  routing shim, not a new public surface.
- Graph context is default-off; when enabled and Graphify is ready, existing
  graph-facing surfaces route through the provider.
- Full `graph.json` content is metadata-only and not loaded into prompts by
  default.
- `GRAPH_REPORT.md` is the bounded report source used when Graphify is
  available; focused graph queries remain future work unless later evidence
  proves otherwise.
- Existing graph-facing surfaces stay stable across modes.
- Legacy CCC graph backend fallback is disabled in enabled graph_context mode.
- Graphify output is read-only evidence, not CCC truth; persisted LongWay,
  task cards, fan-in, review decisions, fallback records, and verification
  capsules remain authoritative.
- Missing, stale, or unavailable Graphify output falls back to normal
  scout/source evidence flow instead of any legacy graph fallback path.
- When `graph_context` is disabled, the runtime preserves the legacy code graph
  path.
- Raider never mutates on graph-only evidence, and Arbiter still requires
  exact source or validation evidence for final acceptance.
- Sentinel context clears per run, and guardrail instructions rehydrate on
  each run or hook evaluation with current LongWay/task-card state.
- Sentinel intervention stays compact in status/projection/app-panel output,
  with raw evidence/log paths retained for audit.
- Routing ownership covers read-only/probe, docs/release-note/operator text,
  code/config mutation or repair, review/risk/regression, lane conflict /
  fallback / ownership guard, and bounded git/gh mutation.
- check-install reports 0.0.15 config, registry, category, fallback,
  concurrency, custom-agent sync, prompt-section, hook, and
  directory-rule-injection readiness.
- CCC plugin packaging is represented in the plan, with a proposed
  plugin-root layout, `.codex-plugin/plugin.json`, `.mcp.json` launcher, and
  bundled CCC skill asset.
- Local Codex plugin install/marketplace testing confirms the CCC package is
  structurally valid and loadable without changing `$cap` into a plugin
  command replacement.
- Completed tasks can produce a verification capsule with acceptance, evidence,
  reviewer verdict, validation, and unresolved risk.
- Status/projection/compact/app-panel output shows the natural CCC loop
  (requirements understanding, planning, exploration, modification, review, and
  verification) without changing `$cap` routing or adding public commands.
- Wisdom promotion requires verified repeated evidence and does not promote
  all memory-like observations.
- Anti-duplication state is visible enough that Captain can avoid repeating
  delegated specialist work.
- Internal MCP/CLI task and session support surfaces exist, but users only see
  `$cap` and projection/status output.
- Install and release docs explain the plugin distribution surface, including
  `.codex-plugin/plugin.json`, `.mcp.json`, and `skills/ccc/SKILL.md`, while
  keeping `$cap` as the public operator entrypoint.
- The StarCraft-style display callsign mapping, oh-my-openagent-inspired
  workflow/skill list, optional TypeScript/JavaScript LSP setup, and Rust-only
  optional `rust-analyzer` install guidance are explicitly represented in the
  source docs, release docs, schema, and SSL manifests. Runtime LSP execution is
  deferred rather than claimed complete.
- Release-facing docs now describe the implemented config-gated
  Graphify-backed graph routing shim, keep public graph-facing surfaces
  stable, and avoid introducing a new public graph command while still
  treating graph output as read-only evidence.
- If the user does not provide special commit-message instructions, CCC uses a
  Conventional Commit-style default message; explicit user instructions
  override that default.

## Out Of Scope

- Adding public commands beyond `$cap`.
- Replacing CCC's Rust runtime with OMO's TypeScript architecture.
- Depending on OMO internals at runtime.
- Adding an indefinite always-running public monitoring subagent or a new
  public Sentinel/Overseer command.
- Exposing `ccc_promptsmith`/Ghost as a public command, public skill,
  standalone user-facing agent, or prompt-refinement command.
- Letting Ghost change the final response language, replace Captain ownership,
  dispatch implementation directly, mutate files, mark work complete, or
  produce verbose reasoning/user-facing final answers.
- Treating Goal Bridge as public, mandatory, or default-on.
- Adding a public goal command, skill command, or entrypoint.
- Treating host goal state as CCC truth or allowing it to override persisted
  CCC state.
- Promising a public `set_goal()` API.
- Letting specialists set, clear, or override host goal state directly.
- Replacing persisted LongWay/task-card/fan-in/review/fallback/verification
  truth with live host goal state.
- Reimplementing Graphify in Rust for `0.0.15-pre`.
- Adding `/graphify`, `@graph`, a graph skill, or a new public graph command.
- Keeping the legacy CCC graph backend as a parallel fallback path.
- Treating Graphify output as CCC source of truth or using graph-only
  evidence to authorize Raider mutation or final acceptance without exact
  source or validation evidence.
- Loading full `graph.json` into prompts by default.
- Allowing mutation based only on inferred, ambiguous, or graph-only
  evidence.
- Un-deferring LSP runtime execution in `0.0.15-pre` as part of this plan.
- Treating Codex CLI side-panel rendering as required for 0.0.15 completion.
- Building a broad plugin marketplace or general-purpose command framework.
- Reframing the CCC plugin as a replacement for the `$cap` public entrypoint
  rather than a distribution/install vehicle.
