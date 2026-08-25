#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Compile to a single static binary. Bun's --compile emits a self-contained
# ELF that does not require a bun runtime on the target.
TARGET="${YFOCUS_TARGET:-bun-linux-x64}"
OUT="${YFOCUS_OUT:-bin/yfocus}"

if ! command -v bun >/dev/null 2>&1; then
  echo "build.sh: bun is required (mise use -g bun@latest, pacman -S bun, or https://bun.sh)" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"

bun build \
  --compile \
  --target "$TARGET" \
  --outfile "$OUT" \
  ts/cli.ts

chmod +x "$OUT"

# Keep the symbol table (no strip) so marketplace reviewers can inspect
# the ELF with nm.
echo "Built $OUT ($(du -h "$OUT" | cut -f1))"
