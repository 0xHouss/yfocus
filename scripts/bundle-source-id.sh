#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
{
  find ts -type f -name '*.ts' | LC_ALL=C sort
  for f in package.json bun.lock manifest.json; do [ -f "$f" ] && printf '%s\n' "$f"; done
} | xargs sha256sum | sha256sum | awk '{print $1}'
