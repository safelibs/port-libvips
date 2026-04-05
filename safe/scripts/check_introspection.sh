#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly SMOKE_SOURCE="${SAFE_ROOT}/tests/introspection/gir_smoke.c"

lib_dir=""
typelib_dir=""
gir_path=""
expect_version=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --lib-dir)
      lib_dir="$2"
      shift 2
      ;;
    --typelib-dir)
      typelib_dir="$2"
      shift 2
      ;;
    --gir)
      gir_path="$2"
      shift 2
      ;;
    --expect-version)
      expect_version="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "${lib_dir}" || -z "${expect_version}" ]]; then
  echo "usage: $0 --lib-dir DIR (--typelib-dir DIR | --gir FILE) --expect-version VERSION" >&2
  exit 2
fi

if [[ -n "${typelib_dir}" && -n "${gir_path}" ]]; then
  echo "pass either --typelib-dir or --gir, not both" >&2
  exit 2
fi

if [[ -z "${typelib_dir}" && -z "${gir_path}" ]]; then
  echo "one of --typelib-dir or --gir is required" >&2
  exit 2
fi

tmp_dir="$(mktemp -d /tmp/libvips-safe-introspection.XXXXXX)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

run_with_target_introspection_env() {
  env \
    -u LD_LIBRARY_PATH \
    -u GI_TYPELIB_PATH \
    LD_LIBRARY_PATH="${lib_dir}" \
    GI_TYPELIB_PATH="${typelib_dir}" \
    "$@"
}

cc "${SMOKE_SOURCE}" -o "${tmp_dir}/gir-smoke" \
  $(pkg-config --cflags --libs gobject-introspection-1.0 glib-2.0 gio-2.0)

if [[ -n "${gir_path}" ]]; then
  typelib_dir="${tmp_dir}/typelib"
  mkdir -p "${typelib_dir}"
  g-ir-compiler \
    --output "${typelib_dir}/Vips-8.0.typelib" \
    "${gir_path}"
fi

run_with_target_introspection_env \
  "${tmp_dir}/gir-smoke" "${expect_version}"

inspect_output="$(
  run_with_target_introspection_env \
    g-ir-inspect --print-shlibs --print-typelibs --version=8.0 Vips
)"
if ! grep -Eq '(^|[[:space:]])libvips\.so\.42$' <<<"${inspect_output}"; then
  echo "g-ir-inspect did not resolve the safe libvips typelib payload" >&2
  printf '%s\n' "${inspect_output}" >&2
  exit 1
fi
