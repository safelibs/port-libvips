# Phase 08

## Phase Name
Debian Packaging, Deprecated ABI, And Twelve-Application Container Harness

## Implement Phase ID
`impl_08_packaging_dependents`

## Preexisting Inputs
- `safe/debian/control`
- `safe/debian/rules`
- `safe/debian/*.install`
- `safe/debian/libvips42t64.shlibs`
- `safe/reference/abi/libvips.symbols`
- `safe/reference/abi/libvips-cpp.symbols`
- `safe/reference/headers/public-files.txt`
- `safe/reference/headers/public-api-decls.txt`
- `safe/reference/pkgconfig/vips.pc`
- `safe/reference/pkgconfig/vips-cpp.pc`
- `safe/reference/modules/installed-modules.txt`
- `safe/reference/modules/module-registry.json`
- `safe/scripts/assert_not_reference_binary.py`
- `safe/scripts/compare_symbols.py`
- `safe/scripts/compare_headers.py`
- `safe/scripts/compare_pkgconfig.py`
- `safe/scripts/compare_modules.py`
- `safe/scripts/compare_module_registry.py`
- `safe/scripts/check_introspection.sh`
- `safe/src/generated/deprecated_im.rs`
- `safe/src/generated/deprecated_vips7.rs`
- `build-check-install/include/vips/`
- `build-check-install/lib/pkgconfig/vips.pc`
- `dependents.json`
- `test-original.sh`
- `safe/vendor/pyvips-3.1.1/`
- `original/debian/`
- `original/debian/gir1.2-vips-8.0.install`
- existing dependent smoke helpers embedded in `test-original.sh`

## New Outputs
- Updated `safe/debian/control`
- Updated `safe/debian/rules`
- Updated `safe/debian/*.install`
- Updated `safe/debian/libvips42t64.shlibs`
- Updated `safe/src/generated/deprecated_im.rs`
- Updated `safe/src/generated/deprecated_vips7.rs`
- Updated `safe/build.rs`
- Updated `dependents.json`
- Updated `test-original.sh`
- New `safe/tests/link_compat/deprecated_c_api_smoke.c`
- New `safe/tests/dependents/apps.json`
- New `safe/tests/dependents/Dockerfile`
- New `safe/tests/dependents/run-suite.sh`
- New `safe/tests/dependents/lib.sh`
- New per-application harness files under `safe/tests/dependents/cases/`

## File Changes
- Finalize the deprecated `im_*` and vips7 compatibility export surface through generated shims compiled into the safe library.
- Add a dedicated deprecated C consumer that is compiled against the reference installed headers, relinked against the packaged safe library, and executed as part of this phase.
- Make Debian packages ship the safe libraries and rebuilt `libvips-cpp`, not copied upstream binaries.
- Make extracted-package verification assemble one shared `/usr` prefix from the runtime, development, tools, and GI packages and prove that the packaged `vips` binary loads module-backed operations from that extracted prefix.
- Expand the dependent application manifest and container harness from the current 3 inline cases to the fixed twelve-application matrix defined in the Context section, and mirror that exact ordered list in both `dependents.json` `selected_applications` and `safe/tests/dependents/apps.json` `applications`.

## Implementation Details
- Reuse and refactor the existing `nip2`, `photoqt`, and `ruby-vips` helpers in `test-original.sh` instead of rewriting them from scratch.
- Replace the hard-coded `expected = {"nip2", "photoqt", "ruby-vips"}` gate in `test-original.sh` with a manifest-driven loader that reads `dependents.json:selected_applications` and validates the exact ordered inventory `nip2`, `photoqt`, `ruby-vips`, `pyvips`, `php-vips`, `govips`, `lua-vips`, `sharp`, `bimg`, `imgproxy`, `carrierwave-vips`, and `sharp-for-go`.
- Keep the current Ubuntu package-provenance `dependents` array in `dependents.json` for `nip2`, `photoqt`, and `ruby-vips`, and add a new `selected_applications` array that records the full fixed twelve-case harness inventory with at least `package`, `category`, and `source_summary` keys for each entry.
- `safe/tests/dependents/apps.json` must contain the same twelve entries in the same order, with one harness definition per application. Each harness entry must record `package`, `source_acquisition`, `build_prerequisites`, `smoke_command`, and `patch_hook`. `pyvips` must use `{"kind": "workspace_path", "path": "safe/vendor/pyvips-3.1.1"}` exactly. Every other entry must use a `source_acquisition` object that records at least `kind`, `uri`, and `ref`, where `ref` is an immutable release identifier or full commit SHA.
- `sharp-for-go` is the manifest package ID for the upstream `DAddYE/vips` repository named as “sharp for Go” in `original/README.md`; do not use the bare package ID `vips`, which would collide with the libvips source package.
- Preserve Ubuntu 24.04 packaging layout and package names: `libvips42t64`, `libvips-dev`, `libvips-tools`, `libvips-doc`, and `gir1.2-vips-8.0`.
- `reference/abi/deprecated-im.symbols` remains the authoritative exported-name manifest for the deprecated surface, but phase completion also requires a committed runtime fixture at `safe/tests/link_compat/deprecated_c_api_smoke.c`. That fixture must include `vips/deprecated.h`, `vips/almostdeprecated.h`, and `vips/vips7compat.h`; it must compile against `/home/yans/code/safelibs/ported/libvips/build-check-install/lib/pkgconfig/vips.pc`; and it must exercise representative deprecated entry points from each public layer, specifically `im_init_world()`, `im_open()` or `im_open_local_array()`, `im_black()`, `IMAGE_BOX` plus `im_extract()`, `im_avg()` or `im_copy()`, and `im_filename_split()`, while also touching at least one token defined only by `deprecated.h` such as `FMTUCHAR` or `BBBYTE`.
- Keep the deprecated smoke module-independent: it should use in-memory or temporary images plus filename parsing, not codec-backed deprecated loaders or savers. Codec-dependent deprecated paths are already validated elsewhere by the packaged `vips` tool, module-registry checks, and the broader application harness.
- Package verification in this phase must unpack `libvips42t64`, `libvips-dev`, `libvips-tools`, and `gir1.2-vips-8.0` into one temporary root, treat `<temp>/usr` as the extracted package prefix, prove the extracted libraries still match the committed symbol manifests, run the packaged `vips` binary from that prefix under `LD_LIBRARY_PATH` and `VIPSHOME`, and then run both `safe/scripts/compare_modules.py` and `safe/scripts/compare_module_registry.py` against that same extracted prefix. The extracted introspection payload must also be usable through both `safe/scripts/check_introspection.sh` and `g-ir-inspect`; file presence alone is not sufficient.
- The deprecated compile/relink/runtime smoke in this phase must compile with the reference `build-check-install` `vips.pc`, but when it links against the extracted package `vips.pc` it must set `PKG_CONFIG_SYSROOT_DIR` to the extraction root so `/usr` resolves inside the extracted package tree rather than the host system.
- The fixed twelve-application harness must build a deterministic Docker image from committed files, install the locally built `.deb` packages, install or build exactly `nip2`, `photoqt`, `ruby-vips`, `pyvips`, `php-vips`, `govips`, `lua-vips`, `sharp`, `bimg`, `imgproxy`, `carrierwave-vips`, and `sharp-for-go` inside the container, and run at least one libvips-dependent smoke or regression test per application.
- After `dpkg-buildpackage` produces the local `.deb` set in the workspace root, `test-original.sh` must honor `LIBVIPS_USE_EXISTING_DEBS=1` by detecting and installing that existing package set inside Docker instead of rebuilding libvips-safe there. Container-side package rebuilds are not allowed once the host-side artifacts already exist.
- Every app harness entry must record enough metadata for deterministic execution: package name, source acquisition method, build prerequisites, smoke command, patch hook, and immutable source coordinates.

## Verification Phases
### `check_08_packaging_dependents`
- Type: `check`
- Fixed `bounce_target`: `impl_08_packaging_dependents`
- Purpose: validate Debian payload correctness, live packaged module loading through `libvips-tools`, deprecated ABI coverage through symbol plus compile/relink/runtime checks, and the expanded real-application Docker harness.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
dpkg-buildpackage -b -uc -us
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
package_root="$(mktemp -d)"
package_prefix="${package_root}/usr"
dpkg-deb -x "${runtime_deb}" "${package_root}"
dpkg-deb -x "${dev_deb}" "${package_root}"
dpkg-deb -x "${tools_deb}" "${package_root}"
dpkg-deb -x "${gir_deb}" "${package_root}"
packaged_libvips="$(find "${package_prefix}" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
packaged_libvips_cpp="$(find "${package_prefix}" -type f -name 'libvips-cpp.so.42.17.1' | sort | sed -n '1p')"
packaged_vips_bin="${package_prefix}/bin/vips"
packaged_vips_pc="$(find "${package_prefix}" -type f -path '*/pkgconfig/vips.pc' | sort | sed -n '1p')"
packaged_vips_cpp_pc="$(find "${package_prefix}" -type f -path '*/pkgconfig/vips-cpp.pc' | sort | sed -n '1p')"
packaged_gir="$(find "${package_prefix}" -type f -name 'Vips-8.0.gir' | sort | sed -n '1p')"
packaged_typelib_dir="$(find "${package_prefix}" -type d -path '*/girepository-1.0' | sort | sed -n '1p')"
test -n "${packaged_libvips}"
test -n "${packaged_libvips_cpp}"
test -x "${packaged_vips_bin}"
test -n "${packaged_vips_pc}"
test -n "${packaged_vips_cpp_pc}"
test -n "${packaged_gir}"
test -n "${packaged_typelib_dir}"
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips.so.42.17.1 \
  "${packaged_libvips}"
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips-cpp.so.42.17.1 \
  "${packaged_libvips_cpp}"
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
  "${package_prefix}"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips.pc \
  "${packaged_vips_pc}"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips-cpp.pc \
  "${packaged_vips_cpp_pc}"
test -f tests/link_compat/deprecated_c_api_smoke.c
mkdir -p "$PWD/.tmp"
deprecated_obj="$PWD/.tmp/deprecated_c_api_smoke.o"
deprecated_bin="$PWD/.tmp/deprecated_c_api_smoke"
read -r -a ref_cflags <<<"$(env \
  PKG_CONFIG_PATH=/home/yans/code/safelibs/ported/libvips/build-check-install/lib/pkgconfig \
  pkg-config --cflags vips)"
read -r -a packaged_libs <<<"$(env \
  PKG_CONFIG_PATH="$(dirname "${packaged_vips_pc}")" \
  PKG_CONFIG_SYSROOT_DIR="${package_root}" \
  pkg-config --libs vips)"
cc -c tests/link_compat/deprecated_c_api_smoke.c \
  -o "${deprecated_obj}" \
  "${ref_cflags[@]}"
cc "${deprecated_obj}" \
  -o "${deprecated_bin}" \
  -Wl,-rpath,"$(dirname "${packaged_libvips}")" \
  "${packaged_libs[@]}"
LD_LIBRARY_PATH="$(dirname "${packaged_libvips}")${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
VIPSHOME="${package_prefix}" \
  "${deprecated_bin}"
python3 scripts/compare_modules.py \
  reference/modules \
  "${package_prefix}"
LD_LIBRARY_PATH="$(dirname "${packaged_libvips}")${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
VIPSHOME="${package_prefix}" \
  "${packaged_vips_bin}" -l operation | rg 'heifload|jxlload|magickload|openslideload|pdfload'
python3 scripts/compare_module_registry.py \
  reference/modules/module-registry.json \
  "${package_prefix}"
scripts/check_introspection.sh \
  --lib-dir "$(dirname "${packaged_libvips}")" \
  --typelib-dir "${packaged_typelib_dir}" \
  --expect-version 8.15.1
GI_TYPELIB_PATH="${packaged_typelib_dir}${GI_TYPELIB_PATH:+:${GI_TYPELIB_PATH}}" \
LD_LIBRARY_PATH="$(dirname "${packaged_libvips}")${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
  g-ir-inspect Vips >/dev/null
scripts/check_introspection.sh \
  --lib-dir "$(dirname "${packaged_libvips}")" \
  --gir "${packaged_gir}" \
  --expect-version 8.15.1
python3 - <<'PY'
import json
from pathlib import Path
repo = Path('/home/yans/code/safelibs/ported/libvips')
manifest = json.loads((repo / 'dependents.json').read_text())
ubuntu_dependents = manifest['dependents']
selected = manifest['selected_applications']
apps = json.loads((repo / 'safe/tests/dependents/apps.json').read_text())['applications']
expected_ubuntu = ['nip2', 'photoqt', 'ruby-vips']
expected_selected = [
    'nip2',
    'photoqt',
    'ruby-vips',
    'pyvips',
    'php-vips',
    'govips',
    'lua-vips',
    'sharp',
    'bimg',
    'imgproxy',
    'carrierwave-vips',
    'sharp-for-go',
]
required_selected_keys = {'package', 'category', 'source_summary'}
required_app_keys = {'package', 'source_acquisition', 'build_prerequisites', 'smoke_command', 'patch_hook'}
ubuntu_names = [entry['package'] for entry in ubuntu_dependents]
selected_names = [entry['package'] for entry in selected]
app_names = [entry['package'] for entry in apps]
if ubuntu_names != expected_ubuntu:
    raise SystemExit(f'unexpected Ubuntu dependent provenance set: {ubuntu_names}')
if selected_names != expected_selected:
    raise SystemExit(f'unexpected selected application inventory: {selected_names}')
if app_names != expected_selected:
    raise SystemExit(f'unexpected app harness inventory: {app_names}')
for entry in selected:
    missing_keys = sorted(required_selected_keys - entry.keys())
    if missing_keys:
        raise SystemExit(f'selected_applications entry {entry.get("package", "<unknown>")} is missing keys: {missing_keys}')
for entry in apps:
    missing_keys = sorted(required_app_keys - entry.keys())
    if missing_keys:
        raise SystemExit(f'app harness entry {entry.get("package", "<unknown>")} is missing keys: {missing_keys}')
    acquisition = entry['source_acquisition']
    if entry['package'] == 'pyvips':
        if acquisition != {'kind': 'workspace_path', 'path': 'safe/vendor/pyvips-3.1.1'}:
            raise SystemExit(f'pyvips must use the vendored workspace path, found: {acquisition}')
    else:
        missing_acquisition_keys = sorted({'kind', 'uri', 'ref'} - acquisition.keys())
        if missing_acquisition_keys:
            raise SystemExit(
                f'app harness entry {entry["package"]} is missing source_acquisition keys: {missing_acquisition_keys}'
            )
PY
cd /home/yans/code/safelibs/ported/libvips
LIBVIPS_USE_EXISTING_DEBS=1 ./test-original.sh
```

## Success Criteria
- `check_08_packaging_dependents` passes without modification.
- The Debian payloads, deprecated ABI smoke coverage, extracted-package checks, and the fixed twelve-application harness all run against safe-produced artifacts.

## Git Commit Requirement
The implementer must commit work to git before yielding.
