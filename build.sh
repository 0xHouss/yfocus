#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

OUT="${YFOCUS_OUT:-bin/yfocus}"
PROFILE="${YFOCUS_PROFILE:-release}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "build.sh: cargo is required (https://rustup.rs)" >&2
  exit 1
fi

# Map PROFILE to cargo flag
if [ "$PROFILE" = "release" ]; then
  cargo build --release
  BIN="target/release/yfocus"
else
  cargo build
  BIN="target/debug/yfocus"
fi

mkdir -p "$(dirname "$OUT")"
cp "$BIN" "$OUT"
chmod +x "$OUT"

# Keep the symbol table (no strip) so marketplace reviewers can inspect
# the ELF with nm.
echo "Built $OUT ($(du -h "$OUT" | cut -f1))"
