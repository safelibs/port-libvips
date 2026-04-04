#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly ORIGINAL_ROOT="${SAFE_ROOT}/../original"
readonly BUILD_DIR_DEFAULT="${SAFE_ROOT}/build-compat"
readonly PYTHON_BIN_DEFAULT="/usr/bin/python3"

build_dir="${VIPS_SAFE_BUILD_DIR:-${BUILD_DIR_DEFAULT}}"
python_bin="${VIPS_SAFE_PYTHON:-${PYTHON_BIN_DEFAULT}}"
if [[ "${build_dir}" != /* ]]; then
  build_dir="${SAFE_ROOT}/${build_dir}"
fi

export VIPSHOME="${build_dir}"
export LD_LIBRARY_PATH="${build_dir}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export PYTHONPATH="${SAFE_ROOT}/vendor/pyvips-3.1.1${PYTHONPATH:+:${PYTHONPATH}}"
export PYTHONNOUSERSITE=1
export PIP_NO_INDEX=1

exec "${python_bin}" -m pytest "${ORIGINAL_ROOT}/test/test-suite" "$@"
