# Phase 02

## Phase Name
ABI Surface, Headers, Pkg-config, And Bootstrap Runtime

## Implement Phase ID
`impl_02_abi_bootstrap`

## Preexisting Inputs
- `safe/build.rs`
- `safe/include/vips/`
- `safe/src/abi/`
- `safe/src/runtime/init.rs`
- `safe/src/runtime/error.rs`
- `safe/reference/abi/libvips.symbols`
- `safe/reference/headers/public-files.txt`
- `safe/reference/headers/public-api-decls.txt`
- `safe/reference/pkgconfig/vips.pc`
- `build-check-install/include/vips/`

## New Outputs
- Updated `safe/build.rs`
- Updated `safe/src/abi/basic.rs`
- Updated `safe/src/abi/object.rs`
- Updated `safe/src/abi/operation.rs`
- Updated `safe/src/abi/image.rs`
- Updated `safe/src/abi/region.rs`
- Updated `safe/src/abi/connection.rs`
- Updated `safe/src/abi/type.rs`
- Updated `safe/src/runtime/init.rs`
- Updated `safe/src/runtime/error.rs`
- Updated `safe/tests/abi_layout.rs`
- Updated `safe/tests/init_version_smoke.rs`

## File Changes
- Finish the `#[repr(C)]` ABI structs, enums, class structs, and exported getters so the full public surface matches the installed headers.
- Ensure `safe/build.rs` emits the full version script and shim set needed for the complete `libvips` symbol manifest.
- Keep `safe/include/vips/*` synchronized with the committed reference header snapshot and install those exact files.

## Implementation Details
- Treat the committed header snapshot as the source of truth for field order and public declarations. Do not invent missing fields from memory.
- Preserve the upstream SONAME, version node, and hidden-by-default export policy.
- Ensure `vips_error*`, `vips_init()`, `vips_shutdown()`, version getters, and base type registration keep working when loaded from the Meson-installed safe library, not only from direct Cargo tests.
- Make `compare_headers.py` and `compare_pkgconfig.py` strict enough that install trees and extracted Debian payloads can reuse them unchanged later.

## Verification Phases
### `check_02_abi_bootstrap`
- Type: `check`
- Fixed `bounce_target`: `impl_02_abi_bootstrap`
- Purpose: validate that the safe `libvips` ABI now matches the full committed symbol, header, and pkg-config contract, not only the bootstrap subset.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo build --release
cargo test --test abi_layout -- --nocapture
cargo test --test init_version_smoke -- --nocapture
meson setup build-abi . --wipe --prefix "$PWD/.tmp/abi-prefix"
meson compile -C build-abi
meson install -C build-abi
SAFE_LIB="$(find "$PWD/.tmp/abi-prefix" -type f -name 'libvips.so.42.17.1' | sort | sed -n '1p')"
SAFE_PC="$(find "$PWD/.tmp/abi-prefix" -type f -path '*/pkgconfig/vips.pc' | sort | sed -n '1p')"
test -n "${SAFE_LIB}"
test -n "${SAFE_PC}"
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips.so.42.17.1 \
  "${SAFE_LIB}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips.symbols \
  "${SAFE_LIB}"
python3 scripts/compare_headers.py \
  --files reference/headers/public-files.txt \
  --decls reference/headers/public-api-decls.txt \
  "$PWD/.tmp/abi-prefix"
python3 scripts/compare_pkgconfig.py \
  reference/pkgconfig/vips.pc \
  "${SAFE_PC}"
```

## Success Criteria
- `check_02_abi_bootstrap` passes without modification.
- The installed safe library, headers, and `vips.pc` match the committed compatibility manifests.

## Git Commit Requirement
The implementer must commit work to git before yielding.
