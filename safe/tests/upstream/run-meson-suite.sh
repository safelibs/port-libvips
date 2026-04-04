#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly LIST_FILE="${SAFE_ROOT}/tests/upstream/meson-tests.txt"

if [[ "${1:-}" == "--list" ]]; then
  cat "${LIST_FILE}"
  exit 0
fi

if [[ $# -lt 1 ]]; then
  echo "usage: $0 [--list] <build-dir> [meson-test-args...]" >&2
  exit 2
fi

build_dir="$1"
shift
if [[ "${build_dir}" != /* ]]; then
  build_dir="${SAFE_ROOT}/${build_dir}"
fi

mapfile -t meson_tests < <(
  grep -v '^[[:space:]]*#' "${LIST_FILE}" | sed '/^[[:space:]]*$/d'
)

export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

exec meson test -C "${build_dir}" --print-errorlogs "$@" "${meson_tests[@]}"
