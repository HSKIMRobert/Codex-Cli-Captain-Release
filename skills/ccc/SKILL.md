---
name: ccc
description: Plugin distribution surface for Codex-Cli-Captain; keep $cap as the public CCC entry point.
metadata:
  short-description: CCC plugin package
---

# CCC Plugin

Use when the CCC plugin packaging context is relevant, such as install,
activation, or plugin capability discovery.

## Public Contract

- `$cap` remains the public CCC entry point.
- The CCC plugin is an install/distribution surface for the bundled skill
  assets and CCC MCP server configuration.
- Do not present plugin/UI affordances as replacements for `$cap`.
- When an operator wants CCC to run work, preserve the `$cap ...` request form
  and route through the installed CCC MCP server when available.

## Workflow Loop

When this plugin is active for CCC work, follow the CCC loop instead of direct
implementation:

1. Preserve `$cap` as the public request form and start a CCC run for the
   operator request.
2. Create or refresh the CCC plan before implementation begins.
3. Break the plan into bounded task cards with clear acceptance evidence.
4. Route specialist-owned work through configured CCC role agents and wait for
   fan-in before merge or review.
5. Check CCC status or projection while progressing so visible state stays tied
   to persisted LongWay truth.
6. After changes, require the review gate before treating the work as complete.
7. On failure, use bounded retry or replan through CCC instead of open-ended
   local repair.
8. Surface concise phase, role, and result updates to the operator.
9. Finish with a final summary that names the completed work, validation, and
   any blockers.

Keep plugin UI invocation secondary. Plugin UI controls may help discovery or
installation, but they are not replacements for the `$cap` entry point or the
CCC-owned workflow loop.
