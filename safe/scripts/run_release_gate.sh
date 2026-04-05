#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PROJECT_ROOT="$(cd -- "${SAFE_ROOT}/.." && pwd)"
readonly REFERENCE_LIBVIPS="${PROJECT_ROOT}/build-check-install/lib/libvips.so.42.17.1"

cleanup_paths=()
cleanup() {
  for path in "${cleanup_paths[@]}"; do
    if [[ -e "${path}" ]]; then
      rm -rf "${path}"
    fi
  done
}
trap cleanup EXIT

assert_not_reference_binary() {
  local candidate="$1"
  python3 scripts/assert_not_reference_binary.py \
    "${REFERENCE_LIBVIPS}" \
    "${candidate}"
}

assert_expected_symbols_present() {
  local manifest="$1"
  local candidate="$2"
  python3 - "${manifest}" "${candidate}" <<'PY'
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

version_node_re = re.compile(r"^VIPS(?:_CPP)?_[0-9]+$")
manifest = Path(sys.argv[1])
candidate = Path(sys.argv[2])

expected = {
    line.strip()
    for line in manifest.read_text().splitlines()
    if line.strip() and not line.startswith("#")
}

output = subprocess.check_output(
    ["nm", "-D", "--defined-only", str(candidate)],
    text=True,
)
actual = set()
for line in output.splitlines():
    parts = line.split()
    if not parts:
        continue
    symbol = parts[-1].split("@@", 1)[0].split("@", 1)[0]
    if version_node_re.match(symbol):
        continue
    actual.add(symbol)

missing = sorted(expected - actual)
if missing:
    print("missing symbols:", file=sys.stderr)
    for symbol in missing:
        print(f"  {symbol}", file=sys.stderr)
    raise SystemExit(1)

print(f"matched {len(expected)} required symbols")
PY
}

assert_libvips_soname_chain() {
  local libdir="$1"
  test -f "${libdir}/libvips.so.42.17.1"
  test -L "${libdir}/libvips.so.42"
  test -L "${libdir}/libvips.so"
  test "$(readlink "${libdir}/libvips.so.42")" = 'libvips.so.42.17.1'
  test "$(readlink "${libdir}/libvips.so")" = 'libvips.so.42'
}

cd "${SAFE_ROOT}"
export VIPS_SAFE_EXPORT_SURFACE=full

echo "[release-gate] cargo"
cargo build --release
cargo test --all-features -- --nocapture
rg -n '\bunsafe\b' src tests

SAFE_INSTALL_ROOT="$(mktemp -d /tmp/libvips-safe-install.XXXXXX)"
SAFE_LINK_WORKDIR="$(mktemp -d /tmp/libvips-safe-link-compat.XXXXXX)"
cleanup_paths+=("${SAFE_INSTALL_ROOT}" "${SAFE_LINK_WORKDIR}")

echo "[release-gate] meson install"
meson setup build-release . --wipe --prefix "${SAFE_INSTALL_ROOT}"
meson compile -C build-release

SAFE_STAGED_LIBDIR="${SAFE_ROOT}/build-release/lib"
SAFE_STAGED_LIBVIPS="${SAFE_STAGED_LIBDIR}/libvips.so.42.17.1"

echo "[release-gate] staged-surface checks"
assert_libvips_soname_chain "${SAFE_STAGED_LIBDIR}"
assert_not_reference_binary "${SAFE_STAGED_LIBVIPS}"
assert_expected_symbols_present \
  reference/abi/core-bootstrap.symbols \
  "${SAFE_STAGED_LIBVIPS}"

meson install -C build-release

SAFE_LIBVIPS="$(find "${SAFE_INSTALL_ROOT}" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
SAFE_LIBVIPS_CPP="$(find "${SAFE_INSTALL_ROOT}" -type f -name 'libvips-cpp.so.42.17.1' | sort | sed -n '1p')"
SAFE_VIPS_PC="$(find "${SAFE_INSTALL_ROOT}" -type f -path '*/pkgconfig/vips.pc' | sort | sed -n '1p')"
SAFE_VIPS_CPP_PC="$(find "${SAFE_INSTALL_ROOT}" -type f -path '*/pkgconfig/vips-cpp.pc' | sort | sed -n '1p')"
SAFE_GIR="$(find "${SAFE_INSTALL_ROOT}" -type f -name 'Vips-8.0.gir' | sort | sed -n '1p')"
SAFE_TYPELIB="$(find "${SAFE_INSTALL_ROOT}" -type f -name 'Vips-8.0.typelib' | sort | sed -n '1p')"
test -n "${SAFE_LIBVIPS}"
test -n "${SAFE_LIBVIPS_CPP}"
test -n "${SAFE_VIPS_PC}"
test -n "${SAFE_VIPS_CPP_PC}"
test -n "${SAFE_GIR}"
test -n "${SAFE_TYPELIB}"

echo "[release-gate] install-surface checks"
assert_libvips_soname_chain "$(dirname "${SAFE_LIBVIPS}")"
assert_not_reference_binary "${SAFE_LIBVIPS}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips.symbols \
  "${SAFE_LIBVIPS}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips-cpp.symbols \
  "${SAFE_LIBVIPS_CPP}"
python3 scripts/compare_headers.py \
  --files reference/headers/public-files.txt \
  --decls reference/headers/public-api-decls.txt \
  "${SAFE_INSTALL_ROOT}"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips.pc \
  "${SAFE_VIPS_PC}"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips-cpp.pc \
  "${SAFE_VIPS_CPP_PC}"
python3 scripts/compare_modules.py \
  reference/modules \
  "${SAFE_INSTALL_ROOT}"
python3 scripts/compare_module_registry.py \
  reference/modules/module-registry.json \
  "${SAFE_INSTALL_ROOT}"
python3 scripts/compare_test_port.py \
  reference/tests \
  tests/upstream

export SAFE_PKGCONFIG="$(dirname "${SAFE_VIPS_PC}")"
export SAFE_LIBDIR="$(dirname "${SAFE_LIBVIPS}")"
export SAFE_GIRDIR="$(dirname "${SAFE_TYPELIB}")"
export LD_LIBRARY_PATH="${SAFE_LIBDIR}:${LD_LIBRARY_PATH:-}"
export PKG_CONFIG_PATH="${SAFE_PKGCONFIG}:${PKG_CONFIG_PATH:-}"
export GI_TYPELIB_PATH="${SAFE_GIRDIR}:${GI_TYPELIB_PATH:-}"
export PYTHONNOUSERSITE=1
export PIP_NO_INDEX=1

echo "[release-gate] introspection and upstream wrappers"
scripts/check_introspection.sh \
  --lib-dir "${SAFE_LIBDIR}" \
  --typelib-dir "${SAFE_GIRDIR}" \
  --expect-version 8.15.1
g-ir-inspect Vips >/dev/null
scripts/check_introspection.sh \
  --lib-dir "${SAFE_LIBDIR}" \
  --gir "${SAFE_GIR}" \
  --expect-version 8.15.1

tests/upstream/run-meson-suite.sh build-release
tests/upstream/run-shell-suite.sh --list | rg 'test_thumbnail\.sh'
tests/upstream/run-shell-suite.sh build-release
VIPS_SAFE_BUILD_DIR="${SAFE_ROOT}/build-release" tests/upstream/run-pytest-suite.sh
tests/upstream/run-fuzz-suite.sh build-release

echo "[release-gate] link compatibility"
scripts/link_compat.sh \
  --manifest reference/objects/link-compat-manifest.json \
  --reference-install "${PROJECT_ROOT}/build-check-install" \
  --build-check "${PROJECT_ROOT}/build-check" \
  --safe-prefix "${SAFE_INSTALL_ROOT}" \
  --workdir "${SAFE_LINK_WORKDIR}"

build_stamp="$(mktemp)"
cleanup_paths+=("${build_stamp}")
touch "${build_stamp}"

echo "[release-gate] debian packages"
dpkg-buildpackage -b -uc -us
find .. -maxdepth 1 -type f -newer "${build_stamp}" -name '*.deb' | sort

version="$(dpkg-parsechangelog -SVersion)"
arch="$(dpkg-architecture -qDEB_HOST_ARCH)"
runtime_deb="../libvips42t64_${version}_${arch}.deb"
dev_deb="../libvips-dev_${version}_${arch}.deb"
tools_deb="../libvips-tools_${version}_${arch}.deb"
doc_deb="../libvips-doc_${version}_all.deb"
gir_deb="../gir1.2-vips-8.0_${version}_${arch}.deb"
test -f "${runtime_deb}"
test -f "${dev_deb}"
test -f "${tools_deb}"
test -f "${doc_deb}"
test -f "${gir_deb}"

runtime_root="$(mktemp -d /tmp/libvips-safe-runtime-deb.XXXXXX)"
dev_root="$(mktemp -d /tmp/libvips-safe-dev-deb.XXXXXX)"
gir_root="$(mktemp -d /tmp/libvips-safe-gir-deb.XXXXXX)"
cleanup_paths+=("${runtime_root}" "${dev_root}" "${gir_root}")

dpkg-deb -x "${runtime_deb}" "${runtime_root}"
dpkg-deb -x "${dev_deb}" "${dev_root}"
dpkg-deb -x "${gir_deb}" "${gir_root}"

packaged_libvips="$(find "${runtime_root}" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
packaged_libdir="$(dirname "${packaged_libvips}")"
packaged_typelib_dir="$(find "${gir_root}" -type d -path '*/girepository-1.0' | sort | sed -n '1p')"
packaged_gir="$(find "${dev_root}" -type f -name 'Vips-8.0.gir' | sort | sed -n '1p')"
test -n "${packaged_libvips}"
test -n "${packaged_typelib_dir}"
test -n "${packaged_gir}"

assert_not_reference_binary "${packaged_libvips}"
python3 scripts/compare_modules.py \
  reference/modules \
  "${runtime_root}"

scripts/check_introspection.sh \
  --lib-dir "${packaged_libdir}" \
  --typelib-dir "${packaged_typelib_dir}" \
  --expect-version 8.15.1
GI_TYPELIB_PATH="${packaged_typelib_dir}${GI_TYPELIB_PATH:+:${GI_TYPELIB_PATH}}" \
LD_LIBRARY_PATH="${packaged_libdir}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
  g-ir-inspect Vips >/dev/null
scripts/check_introspection.sh \
  --lib-dir "${packaged_libdir}" \
  --gir "${packaged_gir}" \
  --expect-version 8.15.1

for runtime_lib in \
  'usr/lib/.*/libvips\.so\.42' \
  'usr/lib/.*/libvips-cpp\.so\.42'
do
  dpkg-deb -c "${runtime_deb}" | rg "${runtime_lib}"
done

for locale_payload in \
  'usr/share/locale/de/LC_MESSAGES/vips8\.15\.mo' \
  'usr/share/locale/en_GB/LC_MESSAGES/vips8\.15\.mo'
do
  dpkg-deb -c "${runtime_deb}" | rg "${locale_payload}"
done

for dev_payload in \
  'usr/include/vips/vips\.h' \
  'usr/include/vips/VImage8\.h' \
  'usr/include/vips/vips8' \
  'usr/lib/.*/pkgconfig/vips\.pc' \
  'usr/lib/.*/pkgconfig/vips-cpp\.pc' \
  'usr/share/gir-1\.0/Vips-8\.0\.gir'
do
  dpkg-deb -c "${dev_deb}" | rg "${dev_payload}"
done

for tool_bin in \
  'usr/bin/vips$' \
  'usr/bin/vipsedit$' \
  'usr/bin/vipsheader$' \
  'usr/bin/vipsthumbnail$' \
  'usr/bin/vipsprofile$'
do
  dpkg-deb -c "${tools_deb}" | rg "${tool_bin}"
done

for manpage in \
  'usr/share/man/man1/vips\.1' \
  'usr/share/man/man1/vipsedit\.1' \
  'usr/share/man/man1/vipsheader\.1' \
  'usr/share/man/man1/vipsthumbnail\.1' \
  'usr/share/man/man1/vipsprofile\.1'
do
  dpkg-deb -c "${tools_deb}" | rg "${manpage}"
done

for doc_payload in \
  'usr/share/doc/libvips-doc/html' \
  'usr/share/gtk-doc/html'
do
  dpkg-deb -c "${doc_deb}" | rg "${doc_payload}"
done

dpkg-deb -c "${gir_deb}" | rg 'usr/lib/girepository-1\.0/Vips-8\.0\.typelib'

echo "[release-gate] dependent harness"
cd "${PROJECT_ROOT}"
./test-original.sh
