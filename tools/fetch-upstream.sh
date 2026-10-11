#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
destination="${RANDOMON_UPSTREAM:-${repo_root}/target/upstream}"
pinned_commit="7f00eb866ed35c8fe3963f078b6a2e0979dc2b8c"

if [ ! -d "${destination}/.git" ]; then
    git clone --quiet https://github.com/Ajarmar/universal-pokemon-randomizer-zx "${destination}"
fi
git -C "${destination}" fetch --quiet origin "${pinned_commit}" 2>/dev/null || true
git -C "${destination}" checkout --quiet "${pinned_commit}"
echo "${destination}"
