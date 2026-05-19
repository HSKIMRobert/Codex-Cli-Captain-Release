#!/usr/bin/env bash
set -euo pipefail

tracked=$(git ls-files -- 'ccc-*.tar.gz' 'install.sh' 'install.ps1')
if [ -n "$tracked" ]; then
  printf 'Tracked release assets found in the repo tree:\n%s\n' "$tracked" >&2
  exit 1
fi

for path in ccc-*.tar.gz install.sh install.ps1; do
  if [ -e "$path" ]; then
    printf 'Release asset file present in the repo tree: %s\n' "$path" >&2
    exit 1
  fi
done
