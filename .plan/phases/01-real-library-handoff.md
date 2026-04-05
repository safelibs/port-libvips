# Phase 01

## Phase Name
Real Library Handoff And Identity Guardrails

## Implement Phase ID
`impl_01_real_library_handoff`

## Preexisting Inputs
- `safe/meson.build`
- `safe/debian/rules`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/reference/abi/core-bootstrap.symbols`
- `build-check/libvips/libvips.so`
- `build-check-install/lib/libvips.so.42.17.1`

## New Outputs
- Updated `safe/meson.build`
- Updated `safe/debian/rules`
- New `safe/scripts/assert_not_reference_binary.py`
- Updated `safe/scripts/run_release_gate.sh`

## File Changes
- Replace the `libvips_stage` custom target in `safe/meson.build` so it stages the Cargo-built Rust `cdylib`, not `reference_libvips_artifact`.
- Keep the full SONAME symlink chain in both the build tree and install tree.
- Add an identity-check helper used by later Meson, package, and Docker verifiers.
- Update the release gate so it fails immediately if the candidate `libvips.so` matches the upstream reference binary.

## Implementation Details
- Make the Meson wrapper actually invoke the already-declared Cargo build command and stage `target/release/libvips.so` as `libvips.so.42.17.1`, `libvips.so.42`, and `libvips.so`.
- Preserve `build_rpath` and any `LD_LIBRARY_PATH` assumptions already used by upstream wrapper scripts so later shell, pytest, and fuzz phases continue to work.
- Do not attempt to fix `libvips-cpp` in this phase; phase 4 owns that rebuild. This phase is only about replacing the core `libvips` payload and installing a guardrail that prevents any future checker from accepting a copied reference library.
- `assert_not_reference_binary.py` should compare at least SHA-256 and file size, and emit a clear error naming both paths if the candidate equals the reference.
- Update `safe/scripts/run_release_gate.sh` so every staged/install/package validation calls the identity checker before running compatibility assertions.

## Verification Phases
### `check_01_real_library_handoff`
- Type: `check`
- Fixed `bounce_target`: `impl_01_real_library_handoff`
- Purpose: prove that the uninstalled Meson build stages the Rust `libvips.so.42.17.1` instead of copying the upstream reference library, while preserving the correct SONAME symlink chain and a bootstrap-usable symbol surface.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo build --release
meson setup build-real . --wipe --prefix "$PWD/.tmp/real-prefix"
meson compile -C build-real
meson install -C build-real
SAFE_LIB="$(find "$PWD/.tmp/real-prefix" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
test -n "${SAFE_LIB}"
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips.so.42.17.1 \
  "${SAFE_LIB}"
readelf -d "${SAFE_LIB}" | rg 'SONAME.*libvips\.so\.42'
python3 scripts/compare_symbols.py \
  reference/abi/core-bootstrap.symbols \
  "${SAFE_LIB}"
test -L "$PWD/build-real/lib/libvips.so"
test -L "$PWD/build-real/lib/libvips.so.42"
test -f "$PWD/build-real/lib/libvips.so.42.17.1"
```

## Success Criteria
- `check_01_real_library_handoff` passes without modification.
- The Meson-staged and installed `libvips` payload is demonstrably not the upstream reference binary.

## Git Commit Requirement
The implementer must commit work to git before yielding.
