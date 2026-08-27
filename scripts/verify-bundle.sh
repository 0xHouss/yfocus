#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
all_targets=("x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl")
if [[ $# -gt 0 ]]; then
  targets=("$1")
else
  targets=("${all_targets[@]}")
fi
bin_dir="$repo_root/bin"

fail() { echo "verify-bundle: $*" >&2; exit 1; }
cd "$repo_root"

verify_one() {
  local target="$1"
  local arch="${target%%-*}"
  local bin="$bin_dir/yfocus-$arch"
  local expected_file="$bin_dir/yfocus-$arch.sha256"
  local srcid_file="$bin_dir/yfocus.srcid"
  [[ -f "$bin" ]] || fail "missing committed ELF $bin"
  [[ -f "$expected_file" ]] || fail "missing $expected_file"
  local expected committed recorded_srcid actual_srcid
  expected="$(awk '{print $1}' "$expected_file")"
  [[ ${#expected} -eq 64 ]] || fail "recorded hash in $expected_file is not SHA-256"
  committed="$(sha256sum "$bin" | awk '{print $1}')"
  [[ "$committed" == "$expected" ]] || fail "committed ELF $bin hash mismatch: expected $expected got $committed. Run scripts/build-bundle.sh"
  [[ -f "$srcid_file" ]] || fail "missing $srcid_file"
  recorded_srcid="$(awk '{print $1}' "$srcid_file")"
  actual_srcid="$("$repo_root/scripts/bundle-source-id.sh")"
  [[ "$recorded_srcid" == "$actual_srcid" ]] || fail "source id stale: recorded $recorded_srcid actual $actual_srcid. Run scripts/build-bundle.sh"
  local file_out
  file_out="$(file -b "$bin")"
  [[ "$file_out" == ELF* ]] || fail "not ELF: $file_out"
  [[ "$file_out" == *"not stripped"* ]] || fail "ELF stripped: $file_out"
  command -v nm >/dev/null || fail "nm required"
  nm "$bin" >/dev/null 2>&1 || fail "nm cannot read $bin"
  if ! nm "$bin" | grep -q 'yfocus'; then
    fail "no yfocus symbols — not inspectable"
  fi
  echo "verified ($arch): $committed"
}

grep -q '^strip = "debuginfo"' "$repo_root/Cargo.toml" || fail "Cargo.toml must set strip = \"debuginfo\""
if grep -Eq '^strip[[:space:]]*=[[:space:]]*(true|"symbols"|"all")' "$repo_root/Cargo.toml"; then
  fail "Cargo.toml must not strip fully"
fi
toolchain_components="$(grep -E '^components[[:space:]]*=' "$repo_root/rust-toolchain.toml" || true)"
grep -q 'rustfmt' <<<"$toolchain_components" || fail "rust-toolchain.toml must pin rustfmt"
grep -q 'clippy' <<<"$toolchain_components" || fail "rust-toolchain.toml must pin clippy"

crate_version="$(awk -F '"' '/^version = / {print $2; exit}' "$repo_root/Cargo.toml")"
manifest_version="$(python3 -c 'import json; print(json.load(open(sys.argv[1]))["version"])' "$repo_root/manifest.json")"
[[ "$crate_version" == "$manifest_version" ]] || fail "Cargo.toml $crate_version != manifest.json $manifest_version"

if [[ "${GITHUB_REF_TYPE:-}" == tag ]]; then
  tag="${GITHUB_REF_NAME#v}"
  [[ "$crate_version" == "$tag" ]] || fail "tag ${GITHUB_REF_NAME} != $crate_version"
fi

for target in "${targets[@]}"; do verify_one "$target"; done

if [[ "${VERIFY_BUNDLE_SKIP_REBUILD:-}" == 1 ]]; then
  echo "verified: $crate_version (rebuild skipped)"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${cargo_home}/registry/src=./registry/src --remap-path-prefix=${repo_root}=."
export CARGO_TARGET_DIR="$tmp/target"
for target in "${targets[@]}"; do
  normalized="${target^^}"; normalized="${normalized//-/_}"; linker_var="CARGO_TARGET_${normalized}_LINKER"
  [[ -n "${!linker_var:-}" ]] || declare -x "$linker_var"="rust-lld"
done
for target in "${targets[@]}"; do
  arch="${target%%-*}"
  expected="$(awk '{print $1}' "$bin_dir/yfocus-$arch.sha256")"
  cargo build --release --locked --target "$target"
  actual="$(sha256sum "$tmp/target/$target/release/yfocus" | awk '{print $1}')"
  [[ "$expected" == "$actual" ]] || fail "rebuild mismatch $arch: expected $expected actual $actual"
  echo "rebuild verified ($arch): $actual"
done
echo "verified: all ELFs non-stripped, version $crate_version, reproducible"
