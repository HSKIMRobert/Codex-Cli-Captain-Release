#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RELEASE_REPO_ROOT="${CCC_RELEASE_REPO_ROOT:-${SOURCE_ROOT}/../Codex-Cli-Captain-Release}"
INSTALLER="${RELEASE_REPO_ROOT}/install.sh"

fail() {
  echo "install pruning verification failed: $*" >&2
  exit 1
}

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ccc-install-pruning.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

FAKE_BIN_DIR="${TMP_DIR}/fake-bin"
ASSET_DIR="${TMP_DIR}/asset"
INSTALL_ROOT="${TMP_DIR}/install-root"
COMMAND_BIN_DIR="${TMP_DIR}/command-bin"
mkdir -p "${FAKE_BIN_DIR}" "${ASSET_DIR}/bin" "${INSTALL_ROOT}/releases" "${COMMAND_BIN_DIR}"

cat > "${FAKE_BIN_DIR}/codex" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

case "$1 $2 $3" in
  "mcp remove ccc") exit 0 ;;
  "mcp add ccc") exit 0 ;;
  *) exit 1 ;;
esac
SH
chmod +x "${FAKE_BIN_DIR}/codex"

cat > "${ASSET_DIR}/bin/ccc" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
  setup|check-install) exit 0 ;;
  *) exit 1 ;;
esac
SH
chmod +x "${ASSET_DIR}/bin/ccc"

ASSET="${TMP_DIR}/ccc-0.0.7-pre-darwin-arm64.tar.gz"
tar -czf "$ASSET" -C "$ASSET_DIR" .

mkdir -p \
  "${INSTALL_ROOT}/releases/0.0.1-darwin-arm64" \
  "${INSTALL_ROOT}/releases/0.0.3-darwin-arm64" \
  "${INSTALL_ROOT}/releases/0.0.3-linux-arm64"

PATH="${FAKE_BIN_DIR}:${PATH}" \
  CCC_VERSION="v0.0.7-pre" \
  CCC_PLATFORM="darwin-arm64" \
  CCC_DOWNLOAD_URL="file://${ASSET}" \
  CCC_INSTALL_ROOT="$INSTALL_ROOT" \
  CCC_BIN_DIR="$COMMAND_BIN_DIR" \
  "$INSTALLER" >/dev/null

[ -d "${INSTALL_ROOT}/releases/0.0.7-pre-darwin-arm64" ] || fail "new bundle was not installed"
[ -L "${INSTALL_ROOT}/current" ] || fail "current is not a symlink"
[ "$(readlink "${INSTALL_ROOT}/current")" = "${INSTALL_ROOT}/releases/0.0.7-pre-darwin-arm64" ] || fail "current does not point at the new bundle"
[ -L "${COMMAND_BIN_DIR}/ccc" ] || fail "ccc command link was not created"
[ ! -e "${INSTALL_ROOT}/releases/0.0.1-darwin-arm64" ] || fail "old same-platform bundle 0.0.1 was not pruned"
[ ! -e "${INSTALL_ROOT}/releases/0.0.3-darwin-arm64" ] || fail "old same-platform bundle 0.0.3 was not pruned"
[ -d "${INSTALL_ROOT}/releases/0.0.3-linux-arm64" ] || fail "different-platform bundle was pruned"

echo "Install pruning verification passed."
