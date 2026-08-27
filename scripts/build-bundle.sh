#!/usr/bin/env bash
# Reproducibly build bundled bun ELFs — inspired by obsidian-daily-qs.
# Produces bin/yfocus-x86_64, bin/yfocus-aarch64 (+ shim bin/yfocus) and hashes.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

command -v bun >/dev/null 2>&1 || { echo "build-bundle.sh: bun required" >&2; exit 1; }

# bun compile targets (glibc static via bun's --compile)
targets=("bun-linux-x64" "bun-linux-arm64")
arches=("x86_64" "aarch64")

bin_dir="$repo_root/bin"
install -d "$bin_dir"

for i in "${!targets[@]}"; do
  target="${targets[$i]}"
  arch="${arches[$i]}"
  echo "building $arch ($target)..."
  bun build --compile --target "$target" --outfile "$bin_dir/yfocus-$arch" ts/cli.ts
  chmod +x "$bin_dir/yfocus-$arch"
  (cd "$bin_dir" && sha256sum "yfocus-$arch" > "yfocus-$arch.sha256")
done

# Shim for hot-reload compat (old QML that looked for bin/yfocus)
# Handles being called via symlink ~/.local/bin/yfocus → plugin/bin/yfocus
cat > "$bin_dir/yfocus" <<'SHIM'
#!/bin/sh
set -eu
arch="$(uname -m)"
target="$0"
if command -v readlink >/dev/null 2>&1; then
  target="$(readlink -f "$0" 2>/dev/null || echo "$0")"
elif command -v realpath >/dev/null 2>&1; then
  target="$(realpath "$0" 2>/dev/null || echo "$0")"
fi
dir="$(CDPATH= cd -- "$(dirname -- "$target")" && pwd)"
case "$arch" in
x86_64) exec "$dir/yfocus-x86_64" "$@" ;;
aarch64) exec "$dir/yfocus-aarch64" "$@" ;;
*) echo "yfocus: unsupported architecture: $arch" >&2; exit 1 ;;
esac
SHIM
chmod 755 "$bin_dir/yfocus"

srcid="$("$repo_root/scripts/bundle-source-id.sh")"
printf '%s  ts manifest.json\n' "$srcid" > "$bin_dir/yfocus.srcid"

echo "bundled:"
for arch in "${arches[@]}"; do sha256sum "$bin_dir/yfocus-$arch"; done
echo "source-id: $srcid"
