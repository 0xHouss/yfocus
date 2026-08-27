#!/usr/bin/env bash
# Reproducibly build bundled backends — inspired by obsidian-daily-qs.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
all_targets=("x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl")
if [[ $# -gt 0 ]]; then
  targets=("$1")
else
  targets=("${all_targets[@]}")
fi

export RUSTFLAGS="${RUSTFLAGS:-} \
  --remap-path-prefix=${cargo_home}/registry/src=./registry/src \
  --remap-path-prefix=${repo_root}=."

cd "$repo_root"

for target in "${targets[@]}"; do
  normalized="${target^^}"
  normalized="${normalized//-/_}"
  linker_var="CARGO_TARGET_${normalized}_LINKER"
  if [[ -z "${!linker_var:-}" ]]; then
    declare -x "$linker_var"="rust-lld"
  fi
done

for target in "${targets[@]}"; do
  cargo build --release --locked --target "$target"
done

bin_dir="$repo_root/bin"
install -d "$bin_dir"
for target in "${targets[@]}"; do
  arch="${target%%-*}"
  src="$repo_root/target/$target/release/yfocus"
  dst="$bin_dir/yfocus-$arch"
  install -Dm755 "$src" "$dst"
  (cd "$bin_dir" && sha256sum "yfocus-$arch" > "yfocus-$arch.sha256")
done

# Shim for hot-reload compat (old QML that looked for bin/yfocus)
cat > "$bin_dir/yfocus" <<'SHIM'
#!/bin/sh
set -eu
arch="$(uname -m)"
dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
case "$arch" in
x86_64|aarch64) exec "$dir/yfocus-$arch" "$@" ;;
*) echo "yfocus: unsupported architecture: $arch" >&2; exit 1 ;;
esac
SHIM
chmod 755 "$bin_dir/yfocus"
# shim is shell, not ELF — no sha for it

srcid="$("$repo_root/scripts/bundle-source-id.sh")"
printf '%s  src Cargo.toml Cargo.lock rust-toolchain.toml\n' "$srcid" > "$bin_dir/yfocus.srcid"

echo "bundled:"
for target in "${targets[@]}"; do
  arch="${target%%-*}"
  sha256sum "$bin_dir/yfocus-$arch"
done
echo "source-id: $srcid"
