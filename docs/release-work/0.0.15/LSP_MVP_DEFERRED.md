# 0.0.15-pre LSP MVP Surface

`0.0.15-pre` does not implement runtime LSP execution. CCC records a bounded
contract so future runtime work has a stable shape without pretending the
language-server bridge already exists.

## Advisory Capabilities

- `lsp_diagnostics`
- `lsp_references`
- `lsp_definition`
- `lsp_prepare_rename`
- `lsp_rename`
- `rust-analyzer-lsp`

## Config Surface

`ccc-config.toml` may include `[lsp]` with `runtime_execution = "deferred"` and
`enabled = false`. The default config records explicit language-server
contracts under `[lsp.language_servers]`; the schema requires both
`typescript_javascript` and `rust` entries when the LSP section is present.

The TypeScript/JavaScript server contract is:

- command: `typescript-language-server`
- args: `["--stdio"]`
- package hint: `npm install -g typescript typescript-language-server`
- extensions: `ts`, `tsx`, `js`, `jsx`, `mjs`, `cjs`

The Rust server contract is:

- command: `rust-analyzer`
- args: `[]`
- package hint: `rustup component add rust-analyzer`
- extensions: `rs`

The Raider SSL manifest also advertises `rust-analyzer-lsp` so Rust-specific
LSP readiness is visible in the advisory skill surface. This does not enable
language-server execution.

## Runtime Deferral

The current Rust architecture has no bounded language-server process manager,
document sync layer, workspace-root initialization contract, or edit-application
guard for rename operations. Shipping metadata only as a runtime feature would
overstate the implementation. Runtime support should add those pieces before
enabling any LSP tool command.

## Future Runtime Work

When CCC later makes Rust LSP executable at runtime, the bridge should cover:

- rust-analyzer process and session management, including lifecycle tracking
  for start, reuse, restart, and teardown
- request wrappers for `lsp_diagnostics`, `lsp_definition`,
  `lsp_references`, `lsp_prepare_rename`, and `lsp_rename`
- setup and check-install readiness signals that verify rust-analyzer is
  available before trying to use the runtime bridge
- safe rollback and fallback behavior when rust-analyzer is missing,
  misconfigured, or exits unexpectedly
- focused tests and acceptance checks that prove the runtime bridge works
  without changing the current metadata-only truth
