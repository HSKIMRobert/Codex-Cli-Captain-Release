#!/usr/bin/env bash
set -euo pipefail

REPO="HoRi0506/Codex-Cli-Captain-Release"
VERSION="${CCC_VERSION:-v0.0.1}"
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
MARKETPLACE_NAME="ccc-local"
MARKETPLACE_DIR="${INSTALL_ROOT}/plugin-marketplace"
PLUGIN_DIR="${MARKETPLACE_DIR}/plugins/ccc"
CODEX_CONFIG="${CODEX_HOME:-$HOME/.codex}/config.toml"
PLUGIN_CACHE_DIR="${CODEX_HOME:-$HOME/.codex}/plugins/cache/${MARKETPLACE_NAME}/ccc/${VERSION#v}"
LEGACY_CAP_SKILL_DIR="${CODEX_HOME:-$HOME/.codex}/skills/cap"

cleanup() {
  if [ -n "${STAGED_BUNDLE_DIR:-}" ] && [ -d "$STAGED_BUNDLE_DIR" ]; then
    rm -rf "$STAGED_BUNDLE_DIR"
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

prune_old_plugin_cache_versions() {
  local candidate
  local cache_root="${CODEX_HOME:-$HOME/.codex}/plugins/cache/${MARKETPLACE_NAME}/ccc"

  if [ ! -d "$cache_root" ]; then
    return 0
  fi

  for candidate in "$cache_root"/*; do
    if [ ! -d "$candidate" ] || [ "${candidate##*/}" = "${VERSION#v}" ]; then
      continue
    fi
    rm -rf "$candidate"
  done
}

configure_codex_plugin() {
  local source_root="$1"
  local tmp_config

  rm -rf "$PLUGIN_DIR"
  mkdir -p "$PLUGIN_DIR"
  cp -R "${source_root}/." "$PLUGIN_DIR/"
  if [ -f "${PLUGIN_DIR}/share/skills/cap/SKILL.md" ]; then
    rm -rf "${PLUGIN_DIR}/skills/cap"
    mkdir -p "${PLUGIN_DIR}/skills/cap"
    cp "${PLUGIN_DIR}/share/skills/cap/SKILL.md" "${PLUGIN_DIR}/skills/cap/SKILL.md"
  fi

  cat > "${PLUGIN_DIR}/.mcp.json" <<EOF
{
  "mcpServers": {
    "ccc": {
      "command": "${PLUGIN_CACHE_DIR}/bin/ccc",
      "args": [
        "mcp"
      ]
    }
  }
}
EOF

  rm -rf "$PLUGIN_CACHE_DIR"
  mkdir -p "$PLUGIN_CACHE_DIR"
  cp -R "${PLUGIN_DIR}/." "$PLUGIN_CACHE_DIR/"
  prune_old_plugin_cache_versions
  rm -rf "$LEGACY_CAP_SKILL_DIR"

  mkdir -p "${MARKETPLACE_DIR}/.agents/plugins"
  cat > "${MARKETPLACE_DIR}/.agents/plugins/marketplace.json" <<EOF
{
  "name": "${MARKETPLACE_NAME}",
  "interface": {
    "displayName": "CCC local"
  },
  "plugins": [
    {
      "name": "ccc",
      "source": {
        "source": "local",
        "path": "./plugins/ccc"
      },
      "policy": {
        "installation": "AVAILABLE",
        "authentication": "ON_INSTALL"
      },
      "category": "Engineering"
    }
  ]
}
EOF

  mkdir -p "$(dirname "$CODEX_CONFIG")"
  touch "$CODEX_CONFIG"
  tmp_config="$(mktemp "${TMPDIR:-/tmp}/ccc-codex-config.XXXXXX")"

  awk '
    /^\[mcp_servers\.ccc(\.|])/ { skip = 1; next }
    /^\[marketplaces\.ccc-local]/ { skip = 1; next }
    /^\[plugins\."ccc@ccc-local"]/ { skip = 1; next }
    /^\[/ { skip = 0 }
    !skip { print }
  ' "$CODEX_CONFIG" > "$tmp_config"

  cat >> "$tmp_config" <<EOF

[marketplaces.${MARKETPLACE_NAME}]
source_type = "local"
source = "${MARKETPLACE_DIR}"

[plugins."ccc@${MARKETPLACE_NAME}"]
enabled = true
EOF

  mv "$tmp_config" "$CODEX_CONFIG"
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

echo "Running setup..."
"${BUNDLE_DIR}/bin/ccc" setup

echo
echo "Running check-install..."
"${BUNDLE_DIR}/bin/ccc" check-install

echo
echo "Refreshing Codex CCC plugin marketplace..."
mkdir -p "${MARKETPLACE_DIR}/plugins"
configure_codex_plugin "$BUNDLE_DIR"

if [ -e "$TARGET_DIR" ] && [ ! -L "$TARGET_DIR" ]; then
  rm -rf "$TARGET_DIR"
fi
ln -sfn "$BUNDLE_DIR" "$TARGET_DIR"
ln -sfn "${TARGET_DIR}/bin/ccc" "$TARGET_BIN"

echo
echo "Install complete."
echo "Installed root: ${TARGET_DIR}"
echo "Binary link: ${TARGET_BIN}"
echo "Plugin marketplace: ${MARKETPLACE_DIR}"
echo "If ${BIN_DIR} is not on your PATH, add it before using ccc directly."
echo "You must restart Codex CLI before using \$cap or relying on the new CCC plugin registration."
echo "After restarting Codex CLI, run: codex mcp list"
