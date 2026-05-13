# Install & Update Codex-Cli-Captain

Use this guide for the Cargo-first `0.0.1` install surface.

## Install & Update

1. install the published Cargo crate with `cargo install codex-cli-captain`
2. run `ccc setup`
3. fully exit Codex CLI
4. start a new Codex CLI session
5. run `ccc check-install`

Windows PowerShell uses the same primary Cargo path.

Legacy `install.sh`/`install.ps1` release-bundle fallback:

1. download the pinned `v0.0.1` bundle from the release repository, or run the matching installer script
2. unpack the archive if you downloaded it directly
3. run `./bin/ccc setup`
4. fully exit Codex CLI
5. start a new Codex CLI session
6. run `./bin/ccc check-install`

For a local source build, the equivalent flow is:

```bash
cargo build --offline
./target/debug/ccc setup
```

Then fully exit Codex CLI, start a new Codex CLI session, and run:

```bash
./target/debug/ccc check-install
```

For updates, repeat `cargo install codex-cli-captain`, then `ccc setup`, restart Codex CLI, and run `ccc check-install`. `CCC_VERSION` remains the explicit override for the legacy bundle fallback, but the public installers stay pinned to `v0.0.1` unless you set it intentionally. `ccc setup` refreshes MCP registration, the packaged `$cap` skill, and CCC-managed custom agents from the current binary and `ccc-config.toml`; restart Codex CLI before checking the refreshed install.

The release bundle also carries the CCC plugin packaging needed for install and discovery. `$cap` stays the public operator entrypoint.

## Reapply Config Changes

After editing `~/.config/ccc/ccc-config.toml`, paste this into Codex CLI:

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```
