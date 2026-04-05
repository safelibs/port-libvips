#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PROJECT_ROOT="$(cd -- "${SAFE_ROOT}/.." && pwd)"
readonly ORIGINAL_ROOT="${PROJECT_ROOT}/original"
readonly PYTHON_BIN_DEFAULT="/usr/bin/python3"

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

if [[ -z "${VIPS_SAFE_BUILD_DIR:-}" ]]; then
  echo "VIPS_SAFE_BUILD_DIR must point at the safe build tree" >&2
  exit 2
fi

build_dir="${VIPS_SAFE_BUILD_DIR}"
python_bin="${VIPS_SAFE_PYTHON:-${PYTHON_BIN_DEFAULT}}"
build_dir="$(resolve_build_dir "${build_dir}")"

export VIPS_SAFE_BUILD_DIR="${build_dir}"
export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export PYTHONPATH="${SAFE_ROOT}/vendor/pyvips-3.1.1${PYTHONPATH:+:${PYTHONPATH}}"
export PYTHONNOUSERSITE=1
export PIP_NO_INDEX=1
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1

cd "${ORIGINAL_ROOT}"
exec "${python_bin}" -m pytest \
  -c /dev/null \
  --rootdir "${ORIGINAL_ROOT}" \
  test/test-suite \
  "$@"
