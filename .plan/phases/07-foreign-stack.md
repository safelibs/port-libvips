# Phase 07

## Phase Name
Foreign Formats, Dynamic Modules, Introspection, And Safe-Local Upstream Suites

## Implement Phase ID
`impl_07_foreign_stack`

## Preexisting Inputs
- `safe/src/foreign/`
- `safe/reference/modules/module-dir.txt`
- `safe/reference/modules/installed-modules.txt`
- `safe/reference/modules/module-registry.json`
- `safe/reference/tests/`
- `safe/tests/upstream/`
- `safe/vendor/pyvips-3.1.1/`
- `original/libvips/foreign/`
- `original/libvips/module/`
- `original/fuzz/`
- `original/test/`
- `original/test/test-suite/`

## New Outputs
- Updated `safe/src/foreign/mod.rs`
- Updated `safe/src/foreign/base.rs`
- Updated `safe/src/foreign/sniff.rs`
- Updated `safe/src/foreign/metadata.rs`
- Updated `safe/src/foreign/loaders/*.rs`
- Updated `safe/src/foreign/savers/*.rs`
- Updated `safe/src/foreign/modules.rs`
- Updated `safe/meson.build`
- Updated `safe/scripts/check_introspection.sh`
- Updated `safe/scripts/compare_modules.py`
- Updated `safe/scripts/compare_module_registry.py`
- Updated `safe/tests/upstream/manifest.json`
- Updated `safe/tests/upstream/run-meson-suite.sh`
- Updated `safe/tests/upstream/run-shell-suite.sh`
- Updated `safe/tests/upstream/run-pytest-suite.sh`
- Updated `safe/tests/upstream/run-fuzz-suite.sh`
- Updated `safe/tests/security/cve_2019_6976.rs`
- Updated `safe/tests/security/cve_2023_40032.rs`
- Updated `safe/tests/security/cve_2026_3146.rs`

## File Changes
- Complete the foreign loader/saver hierarchy and staged-module handling against the safe core library.
- Make the upstream suite wrappers execute only against the safe build tree and vendored `pyvips`.
- Ensure GObject-introspection artifacts are generated from and load the safe library.

## Implementation Details
- Preserve header-only sniffing and decode-later behavior where upstream libvips exposes it.
- Dynamic modules may continue to compile C helper code from `original/libvips/module/*.c` and `original/libvips/foreign/*.c`, but they must link against the safe `libvips`, load into the safe runtime, and register the expected operation/type surface.
- `build-check-install/` does not contain the module payload. Use the committed module manifest and registry manifest rather than assuming module coverage from the upstream install snapshot.
- Always pass `VIPS_SAFE_BUILD_DIR` to `safe/tests/upstream/run-pytest-suite.sh`; its current default points at `safe/build-compat` and would otherwise allow stale-tree false positives.
- Introspection verification in this phase must use both the committed smoke helper and an independent GI consumer such as `g-ir-inspect` against the same produced typelib.
- Keep `safe/tests/security.rs` as the single aggregation target for all CVE regressions.

## Verification Phases
### `check_07_foreign_stack`
- Type: `check`
- Fixed `bounce_target`: `impl_07_foreign_stack`
- Purpose: validate loaders/savers, dynamic modules, introspection, upstream shell tests, upstream pytest, and fuzz-suite execution against the safe build.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo test --test security -- cve_2019_6976
cargo test --test security -- cve_2023_40032
cargo test --test security -- cve_2026_3146
meson setup build-upstream . --wipe
meson compile -C build-upstream
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips.so.42.17.1 \
  "$PWD/build-upstream/lib/libvips.so.42.17.1"
python3 scripts/compare_modules.py \
  reference/modules \
  "$PWD/build-upstream"
python3 scripts/compare_module_registry.py \
  reference/modules/module-registry.json \
  "$PWD/build-upstream"
TYPELIB="$(find "$PWD/build-upstream" -type f -name 'Vips-8.0.typelib' | sort | sed -n '1p')"
GIR="$(find "$PWD/build-upstream" -type f -name 'Vips-8.0.gir' | sort | sed -n '1p')"
test -n "${TYPELIB}"
test -n "${GIR}"
scripts/check_introspection.sh \
  --lib-dir "$PWD/build-upstream/lib" \
  --typelib-dir "$(dirname "${TYPELIB}")" \
  --expect-version 8.15.1
GI_TYPELIB_PATH="$(dirname "${TYPELIB}")${GI_TYPELIB_PATH:+:${GI_TYPELIB_PATH}}" \
LD_LIBRARY_PATH="$PWD/build-upstream/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
  g-ir-inspect Vips >/dev/null
scripts/check_introspection.sh \
  --lib-dir "$PWD/build-upstream/lib" \
  --gir "${GIR}" \
  --expect-version 8.15.1
tests/upstream/run-meson-suite.sh build-upstream
tests/upstream/run-shell-suite.sh build-upstream
VIPS_SAFE_BUILD_DIR="$PWD/build-upstream" tests/upstream/run-pytest-suite.sh
tests/upstream/run-fuzz-suite.sh build-upstream
```

## Success Criteria
- `check_07_foreign_stack` passes without modification.
- The safe build provides compatible foreign loaders, dynamic modules, introspection artifacts, and safe-local upstream suite execution.

## Git Commit Requirement
The implementer must commit work to git before yielding.
