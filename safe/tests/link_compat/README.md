# Link Compatibility Fixtures

`scripts/link_compat.sh` relinks the captured upstream object surface from
`build-check` against a fresh safe install.

The helper keeps compile and link environments separate:

- It reuses the manifest-recorded object files for tests, tools, examples,
  fuzzers, and the relinked `libvips-cpp`.
- It compiles the C++ smoke consumer in
  [vips_cpp_smoke.cpp](/home/yans/code/safelibs/ported/libvips/safe/tests/link_compat/vips_cpp_smoke.cpp)
  against `build-check-install` via `env PKG_CONFIG_PATH=... pkg-config --cflags vips-cpp`.
- It links every relinked binary and the smoke consumer against the safe
  install, again through `env PKG_CONFIG_PATH=... pkg-config`.

The manifest at `reference/objects/link-compat-manifest.json` is treated as
the complete compatibility contract, including runtime arguments, prepare
steps, post-checks, and the fuzz corpus loop.
