#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RELEASE_REPO_ROOT="${CCC_RELEASE_REPO_ROOT:-${SOURCE_ROOT}/../Codex-Cli-Captain-Release}"
INSTALLER="${RELEASE_REPO_ROOT}/install.sh"
BUILDER="${SOURCE_ROOT}/scripts/release/build-release-asset.sh"
WINDOWS_SMOKE="${SOURCE_ROOT}/scripts/release/verify-windows-install-smoke.sh"
VERSION="0.0.15-pre"
REQUIRED_SSL_MANIFESTS=(
  ccc_tactician
  ccc_scout
  ccc_raider
  ccc_scribe
  ccc_arbiter
  ccc_sentinel
  ccc_companion_reader
  ccc_companion_operator
)
SUPPORTED_PLATFORMS=(
  darwin-arm64
  darwin-x86_64
  linux-arm64
  linux-x86_64
  windows-x86_64
)

fail() {
  echo "release asset matrix verification failed: $*" >&2
  exit 1
}

expect_success_output() {
  local expected="$1"
  shift
  local output

  if ! output="$("$@" 2>&1)"; then
    echo "$output" >&2
    fail "expected success from: $*"
  fi

  if [ "$output" != "$expected" ]; then
    echo "expected: ${expected}" >&2
    echo "actual: ${output}" >&2
    fail "unexpected output from: $*"
  fi
}

expect_failure_contains() {
  local expected_text="$1"
  shift
  local output

  if output="$("$@" 2>&1)"; then
    echo "$output" >&2
    fail "expected failure from: $*"
  fi

  if [[ "$output" != *"$expected_text"* ]]; then
    echo "expected text: ${expected_text}" >&2
    echo "actual: ${output}" >&2
    fail "unexpected failure text from: $*"
  fi
}

expect_file_contains() {
  local expected_text="$1"
  local path="$2"
  local output

  if ! output="$(file "$path" 2>&1)"; then
    echo "$output" >&2
    fail "file failed for ${path}"
  fi

  if [[ "$output" != *"$expected_text"* ]]; then
    echo "expected text: ${expected_text}" >&2
    echo "actual: ${output}" >&2
    fail "unexpected binary type for ${path}"
  fi
}

expect_text_contains() {
  local expected_text="$1"
  local path="$2"

  [ -s "$path" ] || fail "missing non-empty file: ${path}"

  if ! grep -Fq "$expected_text" "$path"; then
    fail "expected ${path} to contain: ${expected_text}"
  fi
}

expect_valid_json() {
  local path="$1"

  [ -s "$path" ] || fail "missing non-empty JSON file: ${path}"

  if command -v python3 >/dev/null 2>&1; then
    python3 -m json.tool "$path" >/dev/null || fail "invalid JSON: ${path}"
  elif command -v jq >/dev/null 2>&1; then
    jq empty "$path" >/dev/null || fail "invalid JSON: ${path}"
  else
    fail "missing python3 or jq to validate JSON: ${path}"
  fi
}

verify_asset_binary_type() {
  local platform="$1"
  local asset="${RELEASE_REPO_ROOT}/ccc-${VERSION}-${platform}.tar.gz"
  local tmp_dir extract_dir binary_path

  if [ ! -f "$asset" ]; then
    fail "missing built asset: ${asset}"
  fi

  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/ccc-asset-verify.XXXXXX")"
  extract_dir="${tmp_dir}/extract"
  mkdir -p "$extract_dir"
  tar -xzf "$asset" -C "$extract_dir"

  case "$platform" in
    darwin-arm64)
      binary_path="${extract_dir}/bin/ccc"
      [ -x "$binary_path" ] || fail "missing executable bin/ccc in ${asset}"
      expect_file_contains "Mach-O 64-bit executable arm64" "$binary_path"
      ;;
    darwin-x86_64)
      binary_path="${extract_dir}/bin/ccc"
      [ -x "$binary_path" ] || fail "missing executable bin/ccc in ${asset}"
      expect_file_contains "Mach-O 64-bit executable x86_64" "$binary_path"
      ;;
    linux-arm64)
      binary_path="${extract_dir}/bin/ccc"
      [ -x "$binary_path" ] || fail "missing executable bin/ccc in ${asset}"
      expect_file_contains "ELF 64-bit LSB pie executable, ARM aarch64" "$binary_path"
      ;;
    linux-x86_64)
      binary_path="${extract_dir}/bin/ccc"
      [ -x "$binary_path" ] || fail "missing executable bin/ccc in ${asset}"
      expect_file_contains "ELF 64-bit LSB pie executable, x86-64" "$binary_path"
      ;;
    windows-x86_64)
      binary_path="${extract_dir}/bin/ccc.exe"
      [ -x "$binary_path" ] || fail "missing executable bin/ccc.exe in ${asset}"
      expect_file_contains "PE32+ executable" "$binary_path"
      ;;
  esac

  verify_asset_skill_ssl_manifests "$asset" "$extract_dir"
  verify_asset_plugin_artifacts "$asset" "$extract_dir"

  rm -rf "$tmp_dir"
}

verify_asset_skill_ssl_manifests() {
  local asset="$1"
  local extract_dir="$2"
  local agent manifest_path

  for agent in "${REQUIRED_SSL_MANIFESTS[@]}"; do
    manifest_path="${extract_dir}/skills/ssl/${agent}.skill.ssl.json"
    [ -s "$manifest_path" ] || fail "missing packaged SSL manifest ${agent} in ${asset}"
    grep -q "\"skill_id\": \"${agent}\"" "$manifest_path" || \
      fail "packaged SSL manifest ${agent} has wrong skill_id in ${asset}"
  done
}

verify_asset_plugin_artifacts() {
  local asset="$1"
  local extract_dir="$2"
  local plugin_manifest="${extract_dir}/.codex-plugin/plugin.json"
  local plugin_mcp="${extract_dir}/.mcp.json"
  local plugin_skill="${extract_dir}/skills/ccc/SKILL.md"
  local cap_skill="${extract_dir}/share/skills/cap/SKILL.md"

  [ -s "$plugin_manifest" ] || fail "missing packaged plugin manifest in ${asset}"
  [ -s "$plugin_mcp" ] || fail "missing packaged plugin MCP config in ${asset}"
  [ -s "$plugin_skill" ] || fail "missing packaged CCC plugin skill in ${asset}"
  [ -s "$cap_skill" ] || fail "missing packaged public cap skill in ${asset}"

  expect_valid_json "$plugin_manifest"
  expect_valid_json "$plugin_mcp"

  expect_text_contains '"name": "ccc"' "$plugin_manifest"
  expect_text_contains '"skills": "./skills/"' "$plugin_manifest"
  expect_text_contains '"mcpServers": "./.mcp.json"' "$plugin_manifest"
  expect_text_contains '"$cap continue the current task"' "$plugin_manifest"

  expect_text_contains '"mcpServers": {' "$plugin_mcp"
  expect_text_contains '"ccc": {' "$plugin_mcp"
  expect_text_contains '"command": "ccc"' "$plugin_mcp"
  expect_text_contains '"mcp"' "$plugin_mcp"

  expect_text_contains 'name: ccc' "$plugin_skill"
  expect_text_contains '`$cap` remains the public CCC entry point.' "$plugin_skill"
  expect_text_contains 'not replacements for the `$cap` entry point' "$plugin_skill"

  expect_text_contains 'name: cap' "$cap_skill"
  expect_text_contains '`$cap` is the only public CCC entry point' "$cap_skill"
}

for platform in "${SUPPORTED_PLATFORMS[@]}"; do
  asset="ccc-${VERSION}-${platform}.tar.gz"
  expect_success_output \
    "$asset" \
    env CCC_PRINT_ASSET=1 CCC_VERSION="v${VERSION}" CCC_PLATFORM="$platform" "$INSTALLER"
  expect_success_output \
    "$asset" \
    env CCC_PRINT_ASSET=1 "$BUILDER" "$VERSION" "$platform"
  "$BUILDER" "$VERSION" "$platform" >/dev/null
  verify_asset_binary_type "$platform"
done

expect_failure_contains \
  "Supported platforms: darwin-arm64 darwin-x86_64 linux-arm64 linux-x86_64 windows-x86_64" \
  env CCC_PRINT_ASSET=1 CCC_VERSION="v${VERSION}" CCC_PLATFORM="unsupported-platform" "$INSTALLER"

expect_failure_contains \
  "Supported platforms: darwin-arm64 darwin-x86_64 linux-arm64 linux-x86_64 windows-x86_64" \
  env CCC_PRINT_ASSET=1 "$BUILDER" "$VERSION" "unsupported-platform"

expect_failure_contains \
  "Windows release assets can be named with CCC_PRINT_ASSET=1, but this Bash installer does not perform native Windows installs." \
  env CCC_VERSION="v${VERSION}" CCC_PLATFORM="windows-x86_64" "$INSTALLER"

"${WINDOWS_SMOKE}"

echo "Release asset matrix verification passed."
