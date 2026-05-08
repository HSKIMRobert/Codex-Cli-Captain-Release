#!/usr/bin/env bash
set -euo pipefail

REPO="HoRi0506/Codex-Cli-Captain-Release"
VERSION="${CCC_VERSION:-v0.0.15-pre}"
INSTALL_ROOT="${CCC_INSTALL_ROOT:-$HOME/.local/share/ccc}"
BIN_DIR="${CCC_BIN_DIR:-$HOME/.local/bin}"
PLATFORM_OVERRIDE="${CCC_PLATFORM:-}"
PRINT_ASSET="${CCC_PRINT_ASSET:-}"
SUPPORTED_PLATFORMS="darwin-arm64 darwin-x86_64 linux-arm64 linux-x86_64 windows-x86_64"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

is_supported_platform() {
  case "$1" in
    darwin-arm64|darwin-x86_64|linux-arm64|linux-x86_64|windows-x86_64) return 0 ;;
    *) return 1 ;;
  esac
}

if [ -n "$PLATFORM_OVERRIDE" ]; then
  if ! is_supported_platform "$PLATFORM_OVERRIDE"; then
    echo "Unsupported platform override: $PLATFORM_OVERRIDE" >&2
    echo "Supported platforms: ${SUPPORTED_PLATFORMS}" >&2
    exit 1
  fi
  OS="${PLATFORM_OVERRIDE%-*}"
  ARCH="${PLATFORM_OVERRIDE#*-}"
else
  need_cmd uname

  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Darwin) OS="darwin" ;;
    Linux) OS="linux" ;;
    *)
      echo "Unsupported OS: $OS" >&2
      exit 1
      ;;
  esac

  case "$ARCH" in
    arm64|aarch64) ARCH="arm64" ;;
    x86_64|amd64) ARCH="x86_64" ;;
    *)
      echo "Unsupported architecture: $ARCH" >&2
      exit 1
      ;;
  esac
fi

PLATFORM="${OS}-${ARCH}"

if ! is_supported_platform "$PLATFORM"; then
  echo "Unsupported platform: $PLATFORM" >&2
  echo "Supported platforms: ${SUPPORTED_PLATFORMS}" >&2
  exit 1
fi

ASSET="ccc-${VERSION#v}-${PLATFORM}.tar.gz"
DOWNLOAD_URL="${CCC_DOWNLOAD_URL:-https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}}"

if [ "$PRINT_ASSET" = "1" ]; then
  echo "$ASSET"
  exit 0
fi

if [ "$OS" = "windows" ]; then
  echo "Windows release assets can be named with CCC_PRINT_ASSET=1, but this Bash installer does not perform native Windows installs." >&2
  exit 1
fi

need_cmd mktemp
need_cmd tar
need_cmd curl
need_cmd codex

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ccc-install.XXXXXX")"
EXTRACT_DIR="${TMP_DIR}/extract"
TARGET_DIR="${INSTALL_ROOT}/current"
TARGET_BIN="${BIN_DIR}/ccc"
RELEASES_DIR="${INSTALL_ROOT}/releases"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

prune_old_release_bundles() {
  local candidate

  for candidate in "${RELEASES_DIR}"/*-"${OS}"-"${ARCH}"; do
    if [ ! -d "$candidate" ] || [ "$candidate" = "$BUNDLE_DIR" ]; then
      continue
    fi
    rm -rf "$candidate"
  done
}

mkdir -p "$EXTRACT_DIR" "$BIN_DIR" "$INSTALL_ROOT" "$RELEASES_DIR"

echo "Downloading ${ASSET}..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${ASSET}" || {
  echo "Failed to download ${DOWNLOAD_URL}" >&2
  echo "This usually means the platform asset has not been published yet." >&2
  exit 1
}

echo "Unpacking ${ASSET}..."
tar -xzf "${TMP_DIR}/${ASSET}" -C "$EXTRACT_DIR"

if [ ! -x "${EXTRACT_DIR}/bin/ccc" ] || [ ! -s "${EXTRACT_DIR}/bin/ccc" ]; then
  echo "The downloaded bundle does not contain a non-empty executable bin/ccc." >&2
  exit 1
fi

BUNDLE_DIR="${RELEASES_DIR}/${VERSION#v}-${OS}-${ARCH}"
STAGED_BUNDLE_DIR="${RELEASES_DIR}/.${VERSION#v}-${OS}-${ARCH}.$$"

rm -rf "$STAGED_BUNDLE_DIR"
mkdir -p "$STAGED_BUNDLE_DIR"
cp -R "${EXTRACT_DIR}/." "$STAGED_BUNDLE_DIR/"

rm -rf "$BUNDLE_DIR"
mv "$STAGED_BUNDLE_DIR" "$BUNDLE_DIR"

if [ -e "$TARGET_DIR" ] && [ ! -L "$TARGET_DIR" ]; then
  rm -rf "$TARGET_DIR"
fi
ln -sfn "$BUNDLE_DIR" "$TARGET_DIR"
ln -sfn "${TARGET_DIR}/bin/ccc" "$TARGET_BIN"

echo "Refreshing Codex MCP registration..."
codex mcp remove ccc >/dev/null 2>&1 || true
codex mcp add ccc -- "${TARGET_DIR}/bin/ccc" mcp

echo "Running setup..."
"${TARGET_DIR}/bin/ccc" setup

echo
echo "Running check-install..."
"${TARGET_DIR}/bin/ccc" check-install

prune_old_release_bundles

echo
echo "Install complete."
echo "Installed root: ${TARGET_DIR}"
echo "Binary link: ${TARGET_BIN}"
echo "If ${BIN_DIR} is not on your PATH, add it before using ccc directly."
echo "You must restart Codex CLI before using \$cap or relying on the new MCP registration."
echo "After restarting Codex CLI, run: ccc check-install"
