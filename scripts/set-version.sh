#!/usr/bin/env bash
set -euo pipefail

version="$1"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

perl -0pi -e "s/(\[workspace\.package\][^\[]*?\nversion = \")[^\"]+(\")/\${1}${version}\${2}/" "${root}/Cargo.toml"
cargo update --workspace --offline --manifest-path "${root}/Cargo.toml" >/dev/null 2>&1 || cargo update --workspace --manifest-path "${root}/Cargo.toml"
grep -m1 '^version = ' "${root}/Cargo.toml"
