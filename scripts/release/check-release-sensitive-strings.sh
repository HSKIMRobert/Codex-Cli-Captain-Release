#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RELEASE_REPO_ROOT="${CCC_RELEASE_REPO_ROOT:-${SOURCE_ROOT}/../Codex-Cli-Captain-Release}"

PATTERN='(/Users/kwkim-hoir|kwkim-hoir|BEGIN (RSA|OPENSSH|PRIVATE) KEY|api[_-]?key|secret|password|token=|Authorization:)'
EXCLUDES=(
  --glob '!*.tar.gz'
  --glob '!bin/ccc'
  --glob '!.git/**'
)

if ! command -v rg >/dev/null 2>&1; then
  echo "Missing required command: rg" >&2
  exit 1
fi

cd "$RELEASE_REPO_ROOT"

if rg -n -i "${EXCLUDES[@]}" "$PATTERN" .; then
  echo "Sensitive-string scan failed. Remove the matched private path, secret-like value, or internal-only reference before release." >&2
  exit 1
fi

echo "Sensitive-string scan passed."
