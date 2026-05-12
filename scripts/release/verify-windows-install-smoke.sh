#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RELEASE_REPO_ROOT="${CCC_RELEASE_REPO_ROOT:-${SOURCE_ROOT}/../Codex-Cli-Captain-Release}"
INSTALLER_PS1="${RELEASE_REPO_ROOT}/install.ps1"
VERSION="${CCC_VERSION:-v0.0.15-pre}"
EXPECTED_ASSET="ccc-${VERSION#v}-windows-x86_64.tar.gz"

fail() {
  echo "Windows install smoke failed: $*" >&2
  exit 1
}

expect_contains() {
  local expected_text="$1"
  local actual="$2"

  if [[ "$actual" != *"$expected_text"* ]]; then
    echo "expected text: ${expected_text}" >&2
    echo "actual: ${actual}" >&2
    fail "unexpected output"
  fi
}

expect_file_contains() {
  local expected_text="$1"
  local path="$2"

  if ! grep -F "$expected_text" "$path" >/dev/null 2>&1; then
    fail "missing expected text in ${path}: ${expected_text}"
  fi
}

[ -f "$INSTALLER_PS1" ] || fail "missing installer: ${INSTALLER_PS1}"

if command -v pwsh >/dev/null 2>&1; then
  output="$(
    env \
      CCC_VERSION="$VERSION" \
      CCC_PLATFORM="windows-x86_64" \
      CCC_PRINT_ASSET=1 \
      pwsh -NoProfile -ExecutionPolicy Bypass -File "$INSTALLER_PS1" 2>&1
  )" || {
    echo "$output" >&2
    fail "expected install.ps1 asset naming to succeed"
  }
  [ "$output" = "$EXPECTED_ASSET" ] || fail "unexpected asset output: ${output}"

  if output="$(
    env \
      CCC_VERSION="$VERSION" \
      CCC_PLATFORM="linux-x86_64" \
      CCC_PRINT_ASSET=1 \
      pwsh -NoProfile -ExecutionPolicy Bypass -File "$INSTALLER_PS1" 2>&1
  )"; then
    echo "$output" >&2
    fail "expected install.ps1 to reject non-Windows platform override"
  fi
  expect_contains "install.ps1 performs native Windows installs only" "$output"

  if output="$(
    env \
      CCC_VERSION="$VERSION" \
      CCC_PLATFORM="unsupported-platform" \
      CCC_PRINT_ASSET=1 \
      pwsh -NoProfile -ExecutionPolicy Bypass -File "$INSTALLER_PS1" 2>&1
  )"; then
    echo "$output" >&2
    fail "expected install.ps1 to reject unsupported platform override"
  fi
  expect_contains "Supported platforms:" "$output"
else
  expect_file_contains '"windows-x86_64"' "$INSTALLER_PS1"
  expect_file_contains '$PrintAsset -eq "1"' "$INSTALLER_PS1"
  expect_file_contains 'install.ps1 performs native Windows installs only' "$INSTALLER_PS1"
  expect_file_contains 'bin\ccc.exe' "$INSTALLER_PS1"
  echo "PowerShell is unavailable; verified install.ps1 Windows install surface statically."
fi

echo "Windows install smoke passed."
