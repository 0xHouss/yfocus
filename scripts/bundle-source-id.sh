#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
{
  find src -type f -name '*.rs' | LC_ALL=C sort
  printf '%s\n' Cargo.toml Cargo.lock rust-toolchain.toml
} | xargs sha256sum | sha256sum | awk '{print $1}'
