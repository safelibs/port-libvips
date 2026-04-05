#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

run_application_case() {
  local package="$1"
  local case_script="${DEPENDENTS_ROOT}/cases/${package}.sh"

  if [[ ! -f "${case_script}" ]]; then
    echo "missing dependent case script: ${case_script}" >&2
    exit 1
  fi

  unset -f run_case 2>/dev/null || true
  # shellcheck source=/dev/null
  source "${case_script}"
  if ! declare -F run_case >/dev/null 2>&1; then
    echo "case script did not define run_case(): ${case_script}" >&2
    exit 1
  fi
  run_case
  unset -f run_case
}

main() {
  export JOBS="${JOBS:-$(nproc)}"

  enable_source_repositories
  install_base_tools
  load_application_inventory
  build_and_install_safe_libvips
  prepare_extracted_prefix
  verify_packaged_prefix
  verify_deprecated_c_api_smoke

  local package
  for package in "${APPLICATIONS[@]}"; do
    run_application_case "${package}"
  done

  log "All extracted-package checks and dependent application smokes passed"
}

main "$@"
