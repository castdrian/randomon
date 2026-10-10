#!/usr/bin/env bash
set -euo pipefail

harness_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${harness_dir}/../../.." && pwd)"
upstream_src="${repo_root}/upstream/src"
config_dir="${upstream_src}/com/dabomstew/pkrandom/config"
classes_dir="${harness_dir}/build/classes"
work_dir="${harness_dir}/build/work"
output_dir="${repo_root}/crates/randomon_settings/tests/vectors"

rm -rf "${harness_dir}/build"
mkdir -p "${classes_dir}" "${work_dir}" "${output_dir}"

javac -nowarn -encoding UTF-8 -Xlint:none \
    -sourcepath "${upstream_src}" \
    -d "${classes_dir}" \
    "${harness_dir}"/src/*.java

run_generator() {
    local line_separator="$1"
    shift
    (cd "${work_dir}" && java \
        -Djava.awt.headless=true \
        -Dline.separator="${line_separator}" \
        -cp "${classes_dir}:${upstream_src}" \
        SettingsVectorGenerator "$@")
}

run_generator $'\n' all "${output_dir}" "${config_dir}"
run_generator $'\r\n' line-separator "${output_dir}" "${config_dir}" custom_names_crlf.json
run_generator $'\n' line-separator "${output_dir}" "${config_dir}" custom_names_lf.json

echo "Vectors written to ${output_dir}"
