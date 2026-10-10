#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
output_dir="${1:-${repo_root}/crates/randomon_java/tests/vectors}"
build_dir="$(mktemp -d)"
trap 'rm -rf "${build_dir}"' EXIT

java_flags=(--add-exports java.base/jdk.internal.math=ALL-UNNAMED)
runtime_flags=("${java_flags[@]}" --add-opens java.base/java.util=ALL-UNNAMED)

javac "${java_flags[@]}" -encoding UTF-8 -d "${build_dir}" "${script_dir}"/src/*.java
java "${runtime_flags[@]}" -Duser.language=en -Duser.country=US -cp "${build_dir}" VectorGen "${output_dir}"
