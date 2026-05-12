#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RELEASE_REPO_ROOT="${CCC_RELEASE_REPO_ROOT:-${SOURCE_ROOT}/../Codex-Cli-Captain-Release}"
SOURCE_SKILL_PATH="${CCC_SKILL_SOURCE_PATH:-${SOURCE_ROOT}/skills/cap/SKILL.md}"
SOURCE_PLUGIN_MANIFEST_PATH="${CCC_PLUGIN_MANIFEST_SOURCE_PATH:-${SOURCE_ROOT}/.codex-plugin/plugin.json}"
SOURCE_PLUGIN_MCP_PATH="${CCC_PLUGIN_MCP_SOURCE_PATH:-${SOURCE_ROOT}/.mcp.json}"
SOURCE_PLUGIN_SKILL_PATH="${CCC_PLUGIN_SKILL_SOURCE_PATH:-${SOURCE_ROOT}/skills/ccc/SKILL.md}"
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

VERSION="${1:-${CCC_VERSION:-}}"
PLATFORM="${2:-${CCC_PLATFORM:-}}"
VERSION="${VERSION#v}"

if [ -z "${VERSION}" ] || [ -z "${PLATFORM}" ]; then
  echo "Usage: $0 <version> <platform>" >&2
  echo "Or set CCC_VERSION and CCC_PLATFORM." >&2
  exit 1
fi

if ! is_supported_platform "${PLATFORM}"; then
  echo "Unsupported platform: ${PLATFORM}" >&2
  echo "Supported platforms: ${SUPPORTED_PLATFORMS}" >&2
  exit 1
fi

ASSET_NAME="ccc-${VERSION}-${PLATFORM}.tar.gz"

if [ "${PRINT_ASSET}" = "1" ]; then
  echo "${ASSET_NAME}"
  exit 0
fi

if [ ! -f "${SOURCE_SKILL_PATH}" ] || [ ! -s "${SOURCE_SKILL_PATH}" ]; then
  echo "Missing authoritative source skill: ${SOURCE_SKILL_PATH}" >&2
  echo "Set CCC_SKILL_SOURCE_PATH if the source repo lives outside the default sibling path." >&2
  exit 1
fi

for source_plugin_artifact in \
  "${SOURCE_PLUGIN_MANIFEST_PATH}" \
  "${SOURCE_PLUGIN_MCP_PATH}" \
  "${SOURCE_PLUGIN_SKILL_PATH}"
do
  if [ ! -f "${source_plugin_artifact}" ] || [ ! -s "${source_plugin_artifact}" ]; then
    echo "Missing source plugin artifact: ${source_plugin_artifact}" >&2
    exit 1
  fi
done

need_cmd tar
need_cmd mktemp

OUTPUT_PATH="${RELEASE_REPO_ROOT}/${ASSET_NAME}"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ccc-release-asset.XXXXXX")"
STAGE_DIR="${TMP_DIR}/stage"
EXTRACT_DIR="${TMP_DIR}/extract"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

case "${PLATFORM}" in
  darwin-arm64)
    BINARY_PATH="${SOURCE_ROOT}/target/aarch64-apple-darwin/release/ccc"
    BINARY_NAME="ccc"
    ;;
  darwin-x86_64)
    BINARY_PATH="${SOURCE_ROOT}/target/x86_64-apple-darwin/release/ccc"
    BINARY_NAME="ccc"
    ;;
  linux-arm64)
    BINARY_PATH="${SOURCE_ROOT}/target/aarch64-unknown-linux-gnu/release/ccc"
    BINARY_NAME="ccc"
    ;;
  linux-x86_64)
    BINARY_PATH="${SOURCE_ROOT}/target/x86_64-unknown-linux-gnu/release/ccc"
    BINARY_NAME="ccc"
    ;;
  windows-x86_64)
    BINARY_PATH="${SOURCE_ROOT}/target/x86_64-pc-windows-gnu/release/ccc.exe"
    BINARY_NAME="ccc.exe"
    ;;
esac

if [ ! -x "${BINARY_PATH}" ] || [ ! -s "${BINARY_PATH}" ]; then
  echo "Expected a non-empty ${PLATFORM} executable at ${BINARY_PATH} before packaging." >&2
  echo "Build it first, for example: cargo build --release --target <matching-rust-target>." >&2
  exit 1
fi

mkdir -p "${STAGE_DIR}" "${EXTRACT_DIR}"

for entry in README.md README.ko.md README.ja.md install.sh install.ps1 bin; do
  if [ -e "${RELEASE_REPO_ROOT}/${entry}" ]; then
    cp -R "${RELEASE_REPO_ROOT}/${entry}" "${STAGE_DIR}/${entry}"
  fi
done

mkdir -p "${STAGE_DIR}/bin"
rm -f "${STAGE_DIR}/bin/ccc" "${STAGE_DIR}/bin/ccc.exe"
cp "${BINARY_PATH}" "${STAGE_DIR}/bin/${BINARY_NAME}"
chmod 755 "${STAGE_DIR}/bin/${BINARY_NAME}"

if [ -e "${RELEASE_REPO_ROOT}/docs/assets" ]; then
  mkdir -p "${STAGE_DIR}/docs"
  cp -R "${RELEASE_REPO_ROOT}/docs/assets" "${STAGE_DIR}/docs/assets"
fi

mkdir -p "${STAGE_DIR}/share/skills/cap"
cp "${SOURCE_SKILL_PATH}" "${STAGE_DIR}/share/skills/cap/SKILL.md"

mkdir -p "${STAGE_DIR}/.codex-plugin" "${STAGE_DIR}/skills/ccc"
cp "${SOURCE_PLUGIN_MANIFEST_PATH}" "${STAGE_DIR}/.codex-plugin/plugin.json"
cp "${SOURCE_PLUGIN_MCP_PATH}" "${STAGE_DIR}/.mcp.json"
cp "${SOURCE_PLUGIN_SKILL_PATH}" "${STAGE_DIR}/skills/ccc/SKILL.md"

SOURCE_SSL_MANIFEST_DIR="${CCC_SKILL_SSL_SOURCE_DIR:-${SOURCE_ROOT}/skills/ssl}"
if [ -d "${SOURCE_SSL_MANIFEST_DIR}" ]; then
  # The runtime searches bundle ancestors for skills/ssl; stage the advisory
  # manifests beside bin/ so packaged installs can report registry health.
  mkdir -p "${STAGE_DIR}/skills/ssl"
  cp "${SOURCE_SSL_MANIFEST_DIR}"/*.skill.ssl.json "${STAGE_DIR}/skills/ssl/"
fi

if command -v strip >/dev/null 2>&1; then
  if strip "${STAGE_DIR}/bin/${BINARY_NAME}" >/dev/null 2>&1 || strip -x "${STAGE_DIR}/bin/${BINARY_NAME}" >/dev/null 2>&1; then
    echo "Stripped debug symbols from staged bin/${BINARY_NAME}."
  else
    echo "strip could not process staged bin/${BINARY_NAME}; packaging unstripped binary." >&2
  fi
else
  echo "strip not found; packaging unstripped bin/${BINARY_NAME}." >&2
fi

rm -f "${OUTPUT_PATH}"
COPYFILE_DISABLE=1 tar -czf "${OUTPUT_PATH}" -C "${STAGE_DIR}" .

tar -xzf "${OUTPUT_PATH}" -C "${EXTRACT_DIR}"

if [ ! -x "${EXTRACT_DIR}/bin/${BINARY_NAME}" ] || [ ! -s "${EXTRACT_DIR}/bin/${BINARY_NAME}" ]; then
  echo "Generated ${ASSET_NAME}, but the extracted bundle has an invalid bin/${BINARY_NAME}." >&2
  exit 1
fi

echo "Built ${OUTPUT_PATH}"
ls -lh "${OUTPUT_PATH}" "${EXTRACT_DIR}/bin/${BINARY_NAME}"
