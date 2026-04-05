# Phase 04

## Phase Name
Link Compatibility, Rebuilt C++ Wrapper, Tools, Examples, And Fuzz Consumers

## Implement Phase ID
`impl_04_link_compat_cpp`

## Preexisting Inputs
- `safe/reference/abi/libvips-cpp.symbols`
- `safe/reference/objects/link-compat-manifest.json`
- `safe/scripts/link_compat.sh`
- `safe/tests/link_compat/vips_cpp_smoke.cpp`
- `original/cplusplus/meson.build`
- `original/cplusplus/VImage.cpp`
- `original/cplusplus/VInterpolate.cpp`
- `original/cplusplus/VRegion.cpp`
- `original/cplusplus/VConnection.cpp`
- `original/cplusplus/VError.cpp`
- `original/tools/`
- `original/examples/`
- `original/fuzz/`

## New Outputs
- Updated `safe/meson.build`
- Updated `safe/tools/meson.build`
- Updated `safe/reference/pkgconfig/vips-cpp.pc`
- Updated `safe/scripts/link_compat.sh`
- Updated `safe/tests/link_compat/vips_cpp_smoke.cpp`

## File Changes
- Replace the `libvips_cpp_stage` reference copy in `safe/meson.build` with a real rebuild of the upstream C++ wrapper against the safe library.
- Ensure all original tools, examples, and fuzzers link against the safe build tree rather than a reference `runtime_link`.
- Tighten the link-compat script so it catches any accidental fallback to upstream libraries.

## Implementation Details
- Build `libvips-cpp` from the original C++ sources and headers. Reusing that wrapper implementation is acceptable; copying the already-built upstream `.so` is not.
- Preserve `libvips-cpp.so.42` SONAME and the reference `vips-cpp.pc` semantics.
- Keep compile-vs-link environments separate inside `scripts/link_compat.sh`: reference headers and `pkg-config` for compile flags, safe prefix for link flags and runtime execution.
- Use the committed link-compat manifest as the complete contract. Do not rediscover object coverage ad hoc.

## Verification Phases
### `check_04_link_compat_cpp`
- Type: `check`
- Fixed `bounce_target`: `impl_04_link_compat_cpp`
- Purpose: validate that `libvips-cpp`, original objects from `build-check/`, original tools, examples, and fuzz consumers all rebuild and run against the safe install.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
meson setup build-link . --wipe --prefix "$PWD/.tmp/link-prefix"
meson compile -C build-link
meson install -C build-link
SAFE_CPP="$(find "$PWD/.tmp/link-prefix" -type f -name 'libvips-cpp.so.42.17.1' | sort | sed -n '1p')"
test -n "${SAFE_CPP}"
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips-cpp.so.42.17.1 \
  "${SAFE_CPP}"
python3 scripts/compare_symbols.py \
  reference/abi/libvips-cpp.symbols \
  "${SAFE_CPP}"
scripts/link_compat.sh \
  --manifest reference/objects/link-compat-manifest.json \
  --reference-install /home/yans/code/safelibs/ported/libvips/build-check-install \
  --build-check /home/yans/code/safelibs/ported/libvips/build-check \
  --safe-prefix "$PWD/.tmp/link-prefix" \
  --workdir "$PWD/.tmp/link-work"
c++ tests/link_compat/vips_cpp_smoke.cpp \
  $(env PKG_CONFIG_PATH="$(dirname "$(find "$PWD/.tmp/link-prefix" -type f -path '*/pkgconfig/vips-cpp.pc' | sort | sed -n '1p')")" pkg-config --cflags --libs vips-cpp) \
  -Wl,-rpath,"$(dirname "${SAFE_CPP}")" \
  -o "$PWD/.tmp/vips_cpp_smoke"
LD_LIBRARY_PATH="$(dirname "${SAFE_CPP}"):${LD_LIBRARY_PATH:-}" \
  "$PWD/.tmp/vips_cpp_smoke" \
  /home/yans/code/safelibs/ported/libvips/original/test/test-suite/images/sample.jpg
```

## Success Criteria
- `check_04_link_compat_cpp` passes without modification.
- `libvips-cpp`, original objects, and original consumer-side binaries all rebuild and run against the safe artifacts.

## Git Commit Requirement
The implementer must commit work to git before yielding.
