# Install & Update Codex-Cli-Captain

Use this guide for the Cargo-first `0.0.3` install surface.

## Install & Update

1. install the published Cargo crate with `cargo install codex-cli-captain`
2. run `ccc setup`
3. fully exit Codex CLI
4. start a new Codex CLI session
5. run `ccc check-install`

To update an existing Cargo install, rerun `cargo install codex-cli-captain --force`, then `ccc setup`, fully restart Codex CLI, and run `ccc check-install`.

### Cargo Migration and PATH Shadowing

Legacy release-bundle installs may leave `~/.local/bin/ccc` earlier in `PATH`
than `~/.cargo/bin/ccc`. In that state, `ccc setup` can invoke the old bundle
even after `cargo install codex-cli-captain` succeeds.

Run `ccc check-install` after Cargo install. It reports the shell-resolved
`ccc`, the Cargo candidate at `~/.cargo/bin/ccc`, the current executable,
whether `~/.local/bin/ccc` is shadowing Cargo, and stale release-bundle paths
or caches such as `~/.local/share/ccc/releases/*`,
`~/.local/share/ccc/current`, `~/.local/share/ccc/plugin-marketplace`, and
stale `~/.codex/plugins/cache/ccc-local/ccc/<version>` entries.

When shadowing is reported, run:

```bash
~/.cargo/bin/ccc setup
```

Then put `~/.cargo/bin` earlier in `PATH`, or remove or demote the legacy
`~/.local/bin/ccc` after reviewing cleanup. Release-bundle rollback paths are
preserved by default unless you explicitly prune them.

To uninstall the Cargo-installed binary, run `cargo uninstall codex-cli-captain`. If you also want CCC-managed cleanup, run `ccc uninstall --dry-run` first, then `ccc uninstall --confirm` if the preview is correct. Use `ccc check-install` before removing MCP registration, `ccc-config.toml`, skills, or custom agents.

Windows PowerShell uses the same primary Cargo path.

Legacy `install.sh`/`install.ps1` release-bundle fallback:

1. download the pinned `v0.0.2` fallback bundle from the release repository, or run the matching installer script
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

For updates, repeat `cargo install codex-cli-captain --force`, then `ccc setup`, restart Codex CLI, and run `ccc check-install`. `CCC_VERSION` remains the explicit override for the legacy bundle fallback, but the public installers stay pinned to `v0.0.2` until `v0.0.3` fallback bundle assets are published and validated. `ccc setup` refreshes MCP registration, the packaged `$cap` skill, and CCC-managed custom agents from the current binary and `ccc-config.toml`; restart Codex CLI before checking the refreshed install.

If you build from a local checkout, treat that as maintainer/local-development fallback only, not as the public install flow.

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
