#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PROJECT_ROOT="$(cd -- "${SAFE_ROOT}/.." && pwd)"
readonly LIST_FILE="${SAFE_ROOT}/tests/upstream/meson-tests.txt"

resolve_build_dir() {
  local build_dir="$1"

  if [[ "${build_dir}" = /* ]]; then
    printf '%s\n' "${build_dir}"
    return
  fi
  if [[ -d "${build_dir}" ]]; then
    (cd -- "${build_dir}" && pwd)
    return
  fi
  if [[ -d "${SAFE_ROOT}/${build_dir}" ]]; then
    (cd -- "${SAFE_ROOT}/${build_dir}" && pwd)
    return
  fi
  if [[ -d "${PROJECT_ROOT}/${build_dir}" ]]; then
    (cd -- "${PROJECT_ROOT}/${build_dir}" && pwd)
    return
  fi

  printf '%s\n' "${SAFE_ROOT}/${build_dir}"
}

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
build_dir="$(resolve_build_dir "${build_dir}")"

mapfile -t meson_tests < <(
  grep -v '^[[:space:]]*#' "${LIST_FILE}" | sed '/^[[:space:]]*$/d'
)

export VIPS_SAFE_BUILD_DIR="${build_dir}"
export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

exec meson test -C "${build_dir}" --print-errorlogs "$@" "${meson_tests[@]}"
