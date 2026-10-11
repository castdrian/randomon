#!/usr/bin/env bash
set -euo pipefail

parity_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${parity_dir}/../.." && pwd)"
upstream_src="${RANDOMON_UPSTREAM:-${repo_root}/target/upstream}/src"
classes_dir="${repo_root}/target/parity/classes"

rm -rf "${classes_dir}"
mkdir -p "${classes_dir}"

javac -nowarn -encoding UTF-8 -Xlint:none \
    -sourcepath "${upstream_src}" \
    -d "${classes_dir}" \
    "${parity_dir}/java/ParityRunner.java"

echo "Parity runner compiled into ${classes_dir}"
