#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$repo_root/bin"
arches=("x86_64" "aarch64")

fail() { echo "verify-bundle: $*" >&2; exit 1; }
cd "$repo_root"

for arch in "${arches[@]}"; do
  bin="$bin_dir/yfocus-$arch"
  sha="$bin_dir/yfocus-$arch.sha256"
  [[ -f "$bin" ]] || fail "missing $bin (run scripts/build-bundle.sh)"
  [[ -f "$sha" ]] || fail "missing $sha"
  expected="$(awk '{print $1}' "$sha")"
  [[ ${#expected} -eq 64 ]] || fail "bad sha in $sha"
  committed="$(sha256sum "$bin" | awk '{print $1}')"
  [[ "$committed" == "$expected" ]] || fail "hash mismatch $arch: expected $expected got $committed"
  file_out="$(file -b "$bin")"
  [[ "$file_out" == ELF* ]] || fail "not ELF: $file_out"
  [[ "$file_out" == *"not stripped"* ]] || echo "warn: $arch ELF stripped (nm may fail)"
  if command -v nm >/dev/null; then nm "$bin" >/dev/null 2>&1 || echo "warn: nm cannot read $arch"
  fi
  echo "verified ($arch): $committed"
done

[[ -f "$bin_dir/yfocus" ]] || fail "missing shim bin/yfocus"
[[ -f "$bin_dir/yfocus.srcid" ]] || fail "missing bin/yfocus.srcid"
recorded="$(awk '{print $1}' "$bin_dir/yfocus.srcid")"
actual="$("$repo_root/scripts/bundle-source-id.sh")"
[[ "$recorded" == "$actual" ]] || fail "srcid stale: recorded $recorded actual $actual (run scripts/build-bundle.sh)"

manifest_version="$(python3 -c 'import json; print(json.load(open("manifest.json"))["version"])')"
echo "verified: $manifest_version, all ELFs"

if [[ "${VERIFY_BUNDLE_SKIP_REBUILD:-}" == "1" ]]; then exit 0; fi

# Rebuild check (best-effort, skip if bun missing)
if ! command -v bun >/dev/null 2>&1; then echo "skip rebuild (no bun)"; exit 0; fi
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
for arch in "${arches[@]}"; do
  case "$arch" in x86_64) target="bun-linux-x64";; aarch64) target="bun-linux-arm64";; esac
  expected="$(awk '{print $1}' "$bin_dir/yfocus-$arch.sha256")"
  bun build --compile --target "$target" --outfile "$tmp/yfocus-$arch" ts/cli.ts
  actual="$(sha256sum "$tmp/yfocus-$arch" | awk '{print $1}')"
  [[ "$expected" == "$actual" ]] || echo "warn: rebuild mismatch $arch (expected $expected actual $actual) — may be bun version diff"
  echo "rebuild $arch: $actual"
done
