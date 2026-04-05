#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly DOCKER_BASE_IMAGE="${DOCKER_IMAGE:-ubuntu:24.04}"
readonly HARNESS_IMAGE_TAG="${HARNESS_IMAGE_TAG:-libvips-safe-dependents:local}"
readonly JOBS="${JOBS:-$(nproc)}"

# shellcheck source=/dev/null
source "${REPO_ROOT}/safe/tests/dependents/lib.sh"

load_application_inventory

docker build \
  --build-arg BASE_IMAGE="${DOCKER_BASE_IMAGE}" \
  -f "${REPO_ROOT}/safe/tests/dependents/Dockerfile" \
  -t "${HARNESS_IMAGE_TAG}" \
  "${REPO_ROOT}"

docker run --rm -i \
  -e DEBIAN_FRONTEND=noninteractive \
  -e JOBS="${JOBS}" \
  -e LIBVIPS_USE_EXISTING_DEBS="${LIBVIPS_USE_EXISTING_DEBS:-0}" \
  -v "${REPO_ROOT}:/work" \
  -w /work \
  "${HARNESS_IMAGE_TAG}" \
  /work/safe/tests/dependents/run-suite.sh
