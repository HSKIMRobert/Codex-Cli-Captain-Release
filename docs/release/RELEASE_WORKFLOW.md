# Release Workflow

CCC source work uses a `develop` to `main` release flow.

## Branches

- `develop`: default working branch for planned changes, release candidates, and local validation.
- `main`: released source state only. Merge into `main` when a version is ready to publish.

## Release Steps

1. Finish implementation and release-note updates on `develop`.
2. Run local source validation from `Codex-Cli-Captain`.
3. Merge `develop` into `main` when validation passes.
4. Sync only public installer defaults, README copy, release card text, and assets in `Codex-Cli-Captain-Release`.
5. Build release assets and run release checks from source repo `scripts/release/` before publishing.
6. Publish the GitHub release from `main` state.
7. Push `develop` after it matches the released `main` state.

## Release Asset Build Policy

Use the existing tarball-based release workflow for platform asset refreshes.
Do not switch to Docker for normal `ccc-<version>-<platform>.tar.gz` builds.

- Build or refresh the release-target binaries under `target/<triple>/release/`.
- Package each platform with `scripts/release/build-release-asset.sh <version> <platform>`.
- Verify the produced tarballs with `scripts/release/verify-release-asset-matrix.sh`.
- On macOS hosts, use the rustup cargo/rustc pair for cross-target builds; use
  `cargo-zigbuild`/Zig for Linux targets when the GNU Linux linker toolchains are
  not installed locally.

## Commit Boundaries

Prefer coherent commits:

- runtime/source changes
- focused tests
- source release docs
- public release-repo README, installer, and metadata
- generated release assets

Combining these is acceptable only when the captain records why separating them would make the release less clear or less safe.

## Token And Prompt Hygiene

Before publishing, inspect prompt/status surfaces for repeated long fields. Keep durable policy in runtime state, keep specialist instructions role-specific, and avoid duplicating task-card root fields inside compact nested views unless the nested copy is needed by a caller.
