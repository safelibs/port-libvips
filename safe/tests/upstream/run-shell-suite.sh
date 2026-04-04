#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly LIST_FILE="${SAFE_ROOT}/tests/upstream/standalone-shell-tests.txt"

if [[ "${1:-}" == "--list" ]]; then
  cat "${LIST_FILE}"
  exit 0
fi

if [[ $# -ne 1 ]]; then
  echo "usage: $0 [--list] <build-dir>" >&2
  exit 2
fi

build_dir="$1"
if [[ "${build_dir}" != /* ]]; then
  build_dir="${SAFE_ROOT}/${build_dir}"
fi

export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

while IFS= read -r entry; do
  [[ -z "${entry}" || "${entry}" == \#* ]] && continue
  script_name="$(basename "${entry}")"
  (
    cd "${build_dir}/test"
    "./${script_name}"
  )
done < "${LIST_FILE}"
