#!/usr/bin/env bash
set -euo pipefail

readonly SAFE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PROJECT_ROOT="$(cd -- "${SAFE_ROOT}/.." && pwd)"
readonly REFERENCE_LIBVIPS="${PROJECT_ROOT}/build-check-install/lib/libvips.so.42.17.1"
readonly REFERENCE_LIBVIPS_CPP="${PROJECT_ROOT}/build-check-install/lib/libvips-cpp.so.42.17.1"
readonly REFERENCE_VIPS_PC="${PROJECT_ROOT}/build-check-install/lib/pkgconfig/vips.pc"

cleanup_paths=()
cleanup() {
  for path in "${cleanup_paths[@]}"; do
    if [[ -n "${path}" && -e "${path}" ]]; then
      rm -rf "${path}"
    fi
  done
}
trap cleanup EXIT

assert_manifest_subset() {
  local subset="$1"
  local superset="$2"
  python3 - "${subset}" "${superset}" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

subset = {
    line.strip()
    for line in Path(sys.argv[1]).read_text().splitlines()
    if line.strip() and not line.startswith("#")
}
superset = {
    line.strip()
    for line in Path(sys.argv[2]).read_text().splitlines()
    if line.strip() and not line.startswith("#")
}

missing = sorted(subset - superset)
if missing:
    print("subset manifest contains symbols missing from superset:", file=sys.stderr)
    for symbol in missing:
        print(f"  {symbol}", file=sys.stderr)
    raise SystemExit(1)

print(f"validated manifest subset with {len(subset)} symbols")
PY
}

assert_not_reference_binary() {
  local reference="$1"
  local candidate="$2"
  python3 scripts/assert_not_reference_binary.py \
    "${reference}" \
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

pkg_config_cflags() {
  local pcdir="$1"
  env PKG_CONFIG_PATH="${pcdir}" pkg-config --cflags vips
}

pkg_config_libs() {
  local pcdir="$1"
  local sysroot="${2:-}"

  if [[ -n "${sysroot}" ]]; then
    env PKG_CONFIG_PATH="${pcdir}" PKG_CONFIG_SYSROOT_DIR="${sysroot}" pkg-config --libs vips
    return
  fi

  env PKG_CONFIG_PATH="${pcdir}" pkg-config --libs vips
}

assert_operation_listing_matches_manifest() {
  local manifest="$1"
  local listing_file="$2"
  python3 - "${manifest}" "${listing_file}" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text())
listing = Path(sys.argv[2]).read_text()

missing = []
for module_name, entry in sorted(manifest.items()):
    probe = str(entry["probe_operation"])
    if probe not in listing:
        missing.append(f"{module_name}: {probe}")

if missing:
    print("missing module probe operations from `vips -l operation`:", file=sys.stderr)
    for item in missing:
        print(f"  {item}", file=sys.stderr)
    raise SystemExit(1)

print(f"matched {len(manifest)} module probe operations in CLI operation listing")
PY
}

assert_probe_operations_execute() {
  local manifest="$1"
  local vips_bin="$2"
  local libdir="$3"
  local prefix="$4"
  local probe
  local probe_output

  while IFS= read -r probe; do
    if [[ -z "${probe}" ]]; then
      continue
    fi

    probe_output="$(
      LD_LIBRARY_PATH="${libdir}" \
      VIPSHOME="${prefix}" \
        "${vips_bin}" "${probe}" 2>&1
    )"
    grep -F "usage: ${probe}" <<<"${probe_output}" >/dev/null
  done < <(
    python3 - "${manifest}" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text())
for module_name in sorted(manifest):
    print(manifest[module_name]["probe_operation"])
PY
  )
}

compile_deprecated_c_api_object() {
  local obj="$1"
  local reference_pc_dir

  reference_pc_dir="$(dirname "${REFERENCE_VIPS_PC}")"
  test -f "${REFERENCE_VIPS_PC}"

  env PKG_CONFIG_PATH="${reference_pc_dir}" \
    cc -c tests/link_compat/deprecated_c_api_smoke.c \
      -o "${obj}" \
      $(pkg_config_cflags "${reference_pc_dir}")
}

link_deprecated_c_api_smoke_binary() {
  local obj="$1"
  local bin="$2"
  local pcdir="$3"
  local sysroot="${4:-}"

  if [[ -n "${sysroot}" ]]; then
    env PKG_CONFIG_PATH="${pcdir}" PKG_CONFIG_SYSROOT_DIR="${sysroot}" \
      cc -o "${bin}" "${obj}" \
        $(pkg_config_libs "${pcdir}" "${sysroot}")
    return
  fi

  env PKG_CONFIG_PATH="${pcdir}" \
    cc -o "${bin}" "${obj}" \
      $(pkg_config_libs "${pcdir}")
}

run_deprecated_c_api_smoke_binary() {
  local bin="$1"
  local libdir="$2"
  local prefix="$3"

  LD_LIBRARY_PATH="${libdir}" \
  VIPSHOME="${prefix}" \
    "${bin}"
}

cd "${SAFE_ROOT}"
export VIPS_SAFE_EXPORT_SURFACE=full
export PYTHONDONTWRITEBYTECODE=1

echo "[release-gate] manifest sanity"
assert_manifest_subset \
  reference/abi/core-bootstrap.symbols \
  reference/abi/libvips.symbols

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
assert_not_reference_binary "${REFERENCE_LIBVIPS}" "${SAFE_STAGED_LIBVIPS}"
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
assert_not_reference_binary "${REFERENCE_LIBVIPS}" "${SAFE_LIBVIPS}"
assert_not_reference_binary "${REFERENCE_LIBVIPS_CPP}" "${SAFE_LIBVIPS_CPP}"
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

echo "[release-gate] introspection and upstream wrappers"
SAFE_PKGCONFIG="$(dirname "${SAFE_VIPS_PC}")"
SAFE_LIBDIR="$(dirname "${SAFE_LIBVIPS}")"
SAFE_GIRDIR="$(dirname "${SAFE_TYPELIB}")"
(
  export SAFE_PKGCONFIG
  export SAFE_LIBDIR
  export SAFE_GIRDIR
  export LD_LIBRARY_PATH="${SAFE_LIBDIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export PKG_CONFIG_PATH="${SAFE_PKGCONFIG}${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"
  export GI_TYPELIB_PATH="${SAFE_GIRDIR}${GI_TYPELIB_PATH:+:${GI_TYPELIB_PATH}}"
  export PYTHONNOUSERSITE=1
  export PIP_NO_INDEX=1
  export VIPS_SAFE_BUILD_DIR="${SAFE_ROOT}/build-release"

  scripts/check_introspection.sh \
    --lib-dir "${SAFE_LIBDIR}" \
    --typelib-dir "${SAFE_GIRDIR}" \
    --expect-version 8.15.1
  env -u GI_TYPELIB_PATH -u LD_LIBRARY_PATH \
    GI_TYPELIB_PATH="${SAFE_GIRDIR}" \
    LD_LIBRARY_PATH="${SAFE_LIBDIR}" \
    g-ir-inspect --print-shlibs --print-typelibs --version=8.0 Vips >/dev/null
  scripts/check_introspection.sh \
    --lib-dir "${SAFE_LIBDIR}" \
    --gir "${SAFE_GIR}" \
    --expect-version 8.15.1

  tests/upstream/run-meson-suite.sh build-release
  tests/upstream/run-shell-suite.sh --list | rg 'test_thumbnail\.sh'
  tests/upstream/run-shell-suite.sh build-release
  tests/upstream/run-pytest-suite.sh
  tests/upstream/run-fuzz-suite.sh build-release
)

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
find "${PROJECT_ROOT}" -maxdepth 1 -type f -newer "${build_stamp}" -name '*.deb' | sort

version="$(dpkg-parsechangelog -SVersion)"
arch="$(dpkg-architecture -qDEB_HOST_ARCH)"
runtime_deb="${PROJECT_ROOT}/libvips42t64_${version}_${arch}.deb"
dev_deb="${PROJECT_ROOT}/libvips-dev_${version}_${arch}.deb"
tools_deb="${PROJECT_ROOT}/libvips-tools_${version}_${arch}.deb"
doc_deb="${PROJECT_ROOT}/libvips-doc_${version}_all.deb"
gir_deb="${PROJECT_ROOT}/gir1.2-vips-8.0_${version}_${arch}.deb"
test -f "${runtime_deb}"
test -f "${dev_deb}"
test -f "${tools_deb}"
test -f "${doc_deb}"
test -f "${gir_deb}"
for deb in "${runtime_deb}" "${dev_deb}" "${tools_deb}" "${doc_deb}" "${gir_deb}"; do
  test "${deb}" -nt "${build_stamp}"
done

cleanup_paths+=(
  "${runtime_deb}"
  "${dev_deb}"
  "${tools_deb}"
  "${doc_deb}"
  "${gir_deb}"
  "${PROJECT_ROOT}/libvips42t64-dbgsym_${version}_${arch}.ddeb"
  "${PROJECT_ROOT}/libvips-tools-dbgsym_${version}_${arch}.ddeb"
  "${PROJECT_ROOT}/vips_${version}_${arch}.buildinfo"
  "${PROJECT_ROOT}/vips_${version}_${arch}.changes"
  "${SAFE_ROOT}/debian/.debhelper"
  "${SAFE_ROOT}/debian/debhelper-build-stamp"
  "${SAFE_ROOT}/debian/files"
  "${SAFE_ROOT}/debian/gir1.2-vips-8.0.debhelper.log"
  "${SAFE_ROOT}/debian/gir1.2-vips-8.0.substvars"
  "${SAFE_ROOT}/debian/gir1.2-vips-8.0"
  "${SAFE_ROOT}/debian/libvips-dev.debhelper.log"
  "${SAFE_ROOT}/debian/libvips-dev.substvars"
  "${SAFE_ROOT}/debian/libvips-dev"
  "${SAFE_ROOT}/debian/libvips-doc.debhelper.log"
  "${SAFE_ROOT}/debian/libvips-doc.postinst.debhelper"
  "${SAFE_ROOT}/debian/libvips-doc.postrm.debhelper"
  "${SAFE_ROOT}/debian/libvips-doc.preinst.debhelper"
  "${SAFE_ROOT}/debian/libvips-doc.prerm.debhelper"
  "${SAFE_ROOT}/debian/libvips-doc.substvars"
  "${SAFE_ROOT}/debian/libvips-doc"
  "${SAFE_ROOT}/debian/libvips-tools.debhelper.log"
  "${SAFE_ROOT}/debian/libvips-tools.substvars"
  "${SAFE_ROOT}/debian/libvips-tools"
  "${SAFE_ROOT}/debian/libvips42t64.debhelper.log"
  "${SAFE_ROOT}/debian/libvips42t64.substvars"
  "${SAFE_ROOT}/debian/libvips42t64"
  "${SAFE_ROOT}/debian/tmp"
)

extracted_root="$(mktemp -d /tmp/libvips-safe-extracted-deb.XXXXXX)"
cleanup_paths+=("${extracted_root}")
for deb in "${runtime_deb}" "${dev_deb}" "${tools_deb}" "${gir_deb}"; do
  dpkg-deb -x "${deb}" "${extracted_root}"
done

extracted_prefix="${extracted_root}/usr"
packaged_libvips="$(find "${extracted_prefix}" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
packaged_libvips_cpp="$(find "${extracted_prefix}" -type f -name 'libvips-cpp.so.42.17.1' | sort | sed -n '1p')"
packaged_libdir="$(dirname "${packaged_libvips}")"
packaged_pkgconfig_dir="$(dirname "$(find "${extracted_prefix}" -type f -path '*/pkgconfig/vips.pc' | sort | sed -n '1p')")"
packaged_typelib_dir="$(find "${extracted_prefix}" -type d -path '*/girepository-1.0' | sort | sed -n '1p')"
packaged_gir="$(find "${extracted_prefix}" -type f -name 'Vips-8.0.gir' | sort | sed -n '1p')"
packaged_vips_bin="${extracted_prefix}/bin/vips"
packaged_vipsedit_bin="${extracted_prefix}/bin/vipsedit"
packaged_vipsheader_bin="${extracted_prefix}/bin/vipsheader"
test -n "${packaged_libvips}"
test -n "${packaged_libvips_cpp}"
test -n "${packaged_typelib_dir}"
test -n "${packaged_gir}"
test -n "${packaged_pkgconfig_dir}"
test -x "${packaged_vips_bin}"
test -x "${packaged_vipsedit_bin}"
test -x "${packaged_vipsheader_bin}"

echo "[release-gate] packaged-prefix checks"
assert_not_reference_binary "${REFERENCE_LIBVIPS}" "${packaged_libvips}"
assert_not_reference_binary "${REFERENCE_LIBVIPS_CPP}" "${packaged_libvips_cpp}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips.symbols \
  "${packaged_libvips}"
python3 scripts/compare_symbols.py \
  reference/abi/deprecated-im.symbols \
  "${packaged_libvips}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips-cpp.symbols \
  "${packaged_libvips_cpp}"
python3 scripts/compare_headers.py \
  --files reference/headers/public-files.txt \
  --decls reference/headers/public-api-decls.txt \
  "${extracted_prefix}"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips.pc \
  "${packaged_pkgconfig_dir}/vips.pc"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips-cpp.pc \
  "${packaged_pkgconfig_dir}/vips-cpp.pc"
python3 scripts/compare_modules.py \
  reference/modules \
  "${extracted_prefix}"

operation_listing_file="$(mktemp /tmp/libvips-safe-operations.XXXXXX)"
cleanup_paths+=("${operation_listing_file}")
LD_LIBRARY_PATH="${packaged_libdir}" \
VIPSHOME="${extracted_prefix}" \
  "${packaged_vips_bin}" -l operation >"${operation_listing_file}"
assert_operation_listing_matches_manifest \
  reference/modules/module-registry.json \
  "${operation_listing_file}"
assert_probe_operations_execute \
  reference/modules/module-registry.json \
  "${packaged_vips_bin}" \
  "${packaged_libdir}" \
  "${extracted_prefix}"
python3 scripts/compare_module_registry.py \
  reference/modules/module-registry.json \
  "${extracted_prefix}"

echo "[release-gate] packaged tool xml contract"
tool_contract_workdir="$(mktemp -d /tmp/libvips-safe-tool-contract.XXXXXX)"
cleanup_paths+=("${tool_contract_workdir}")
tool_contract_image="${tool_contract_workdir}/tool-contract.v"
tool_contract_xml="${tool_contract_workdir}/setext.xml"
tool_contract_getext_before="${tool_contract_workdir}/getext.before.xml"
tool_contract_getext_after="${tool_contract_workdir}/getext.after.xml"
tool_contract_source="${PROJECT_ROOT}/original/test/test-suite/images/sample.jpg"
test -f "${tool_contract_source}"
LD_LIBRARY_PATH="${packaged_libdir}" \
VIPSHOME="${extracted_prefix}" \
  "${packaged_vips_bin}" copy "${tool_contract_source}" "${tool_contract_image}"
LD_LIBRARY_PATH="${packaged_libdir}" \
VIPSHOME="${extracted_prefix}" \
  "${packaged_vipsheader_bin}" -f getext "${tool_contract_image}" >"${tool_contract_getext_before}"
grep -F '<?xml version="1.0"?>' "${tool_contract_getext_before}" >/dev/null
grep -F 'type="VipsBlob" name="exif-data"' "${tool_contract_getext_before}" >/dev/null
cat >"${tool_contract_xml}" <<'EOF'
<?xml version="1.0"?>
<root xmlns="http://www.vips.ecs.soton.ac.uk/vips/8.15.1">
  <header>
  </header>
  <meta>
    <field type="VipsRefString" name="comment">edited-by-setext</field>
    <field type="gint" name="page-height">7</field>
  </meta>
</root>
EOF
python3 - "${tool_contract_xml}" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
path.write_text(path.read_text().rstrip())
PY
LD_LIBRARY_PATH="${packaged_libdir}" \
VIPSHOME="${extracted_prefix}" \
  "${packaged_vipsedit_bin}" --setext "${tool_contract_image}" <"${tool_contract_xml}"
LD_LIBRARY_PATH="${packaged_libdir}" \
VIPSHOME="${extracted_prefix}" \
  "${packaged_vipsheader_bin}" -f getext "${tool_contract_image}" >"${tool_contract_getext_after}"
cmp -s "${tool_contract_xml}" "${tool_contract_getext_after}"
test "$(
  LD_LIBRARY_PATH="${packaged_libdir}" \
  VIPSHOME="${extracted_prefix}" \
    "${packaged_vipsheader_bin}" -f comment "${tool_contract_image}"
)" = 'edited-by-setext'
test "$(
  LD_LIBRARY_PATH="${packaged_libdir}" \
  VIPSHOME="${extracted_prefix}" \
    "${packaged_vipsheader_bin}" -f page-height "${tool_contract_image}"
)" = '7'

scripts/check_introspection.sh \
  --lib-dir "${packaged_libdir}" \
  --typelib-dir "${packaged_typelib_dir}" \
  --expect-version 8.15.1
env -u GI_TYPELIB_PATH -u LD_LIBRARY_PATH \
  GI_TYPELIB_PATH="${packaged_typelib_dir}" \
  LD_LIBRARY_PATH="${packaged_libdir}" \
    g-ir-inspect --print-shlibs --print-typelibs --version=8.0 Vips >/dev/null
scripts/check_introspection.sh \
  --lib-dir "${packaged_libdir}" \
  --gir "${packaged_gir}" \
  --expect-version 8.15.1

echo "[release-gate] deprecated C API smoke"
deprecated_smoke_workdir="$(mktemp -d /tmp/libvips-safe-deprecated-smoke.XXXXXX)"
cleanup_paths+=("${deprecated_smoke_workdir}")
deprecated_smoke_obj="${deprecated_smoke_workdir}/deprecated_c_api_smoke.o"
deprecated_smoke_safe_bin="${deprecated_smoke_workdir}/deprecated_c_api_smoke.safe-install"
deprecated_smoke_packaged_bin="${deprecated_smoke_workdir}/deprecated_c_api_smoke.packaged"
compile_deprecated_c_api_object "${deprecated_smoke_obj}"
link_deprecated_c_api_smoke_binary \
  "${deprecated_smoke_obj}" \
  "${deprecated_smoke_safe_bin}" \
  "${SAFE_PKGCONFIG}"
link_deprecated_c_api_smoke_binary \
  "${deprecated_smoke_obj}" \
  "${deprecated_smoke_packaged_bin}" \
  "${packaged_pkgconfig_dir}" \
  "${extracted_root}"
run_deprecated_c_api_smoke_binary \
  "${deprecated_smoke_safe_bin}" \
  "${SAFE_LIBDIR}" \
  "${SAFE_INSTALL_ROOT}"
run_deprecated_c_api_smoke_binary \
  "${deprecated_smoke_packaged_bin}" \
  "${packaged_libdir}" \
  "${extracted_prefix}"

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
LIBVIPS_USE_EXISTING_DEBS=1 ./test-original.sh
