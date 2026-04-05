#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PROJECT_ROOT="$(cd -- "${SAFE_ROOT}/.." && pwd)"
readonly LIST_FILE="${SAFE_ROOT}/tests/upstream/standalone-shell-tests.txt"

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
  while IFS= read -r entry; do
    printf '%s\n' "${entry}"
    if [[ -n "${entry}" && "${entry}" != \#* ]]; then
      printf '%s\n' "${entry//./\\.}"
    fi
  done < "${LIST_FILE}"
  exit 0
fi

if [[ $# -ne 1 ]]; then
  echo "usage: $0 [--list] <build-dir>" >&2
  exit 2
fi

build_dir="$1"
build_dir="$(resolve_build_dir "${build_dir}")"

export VIPS_SAFE_BUILD_DIR="${build_dir}"
export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

while IFS= read -r entry; do
  [[ -z "${entry}" || "${entry}" == \#* ]] && continue
  script_name="$(basename "${entry}")"
  (
    cd "${build_dir}/test"
    log_file="$(mktemp "${TMPDIR:-/tmp}/libvips-shell-suite.XXXXXX.log")"
    trap 'rm -f "${log_file}"' EXIT
    if ! "./${script_name}" >"${log_file}" 2>&1; then
      cat "${log_file}" >&2
      exit 1
    fi
    if [[ "${VIPS_UPSTREAM_VERBOSE:-0}" == "1" ]]; then
      cat "${log_file}"
    fi
  )
done < "${LIST_FILE}"
