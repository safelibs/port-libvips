# Phase 03

## Phase Name
Runtime I/O, Object/Region/Cache, And Generated Registry

## Implement Phase ID
`impl_03_runtime_registry`

## Preexisting Inputs
- `safe/src/runtime/`
- `safe/src/generated/operations_registry.rs`
- `safe/src/generated/operation_wrappers.rs`
- `safe/scripts/generate_operation_registry.py`
- `safe/reference/operations.json`
- `safe/reference/types.json`
- `original/libvips/iofuncs/`
- `original/test/test_connections.c`
- `original/test/test_descriptors.c`
- `original/tools/vips.c`

## New Outputs
- Updated `safe/src/runtime/object.rs`
- Updated `safe/src/runtime/type.rs`
- Updated `safe/src/runtime/operation.rs`
- Updated `safe/src/runtime/image.rs`
- Updated `safe/src/runtime/region.rs`
- Updated `safe/src/runtime/generate.rs`
- Updated `safe/src/runtime/threadpool.rs`
- Updated `safe/src/runtime/cache.rs`
- Updated `safe/src/runtime/source.rs`
- Updated `safe/src/runtime/target.rs`
- Updated `safe/src/generated/operations_registry.rs`
- Updated `safe/src/generated/operation_wrappers.rs`
- Updated `safe/tests/runtime_io.rs`
- Updated `safe/tests/operation_registry.rs`

## File Changes
- Complete the object model, region demand logic, metadata handling, connection/source/target callbacks, cache state, and operation instantiation on the safe library.
- Ensure the original C runtime tests and the original `tools/vips.c` CLI build and execute against the safe library stage.

## Implementation Details
- The live registry in this phase must come from the safe library, not from JSON-only generation. The checker explicitly runs the original `vips` CLI against the safe build tree.
- Preserve `vips_type_map_all()`, `vips_nickname_find()`, `vips_operation_new()`, `vips_call*()`, and `vips_object_build()` ownership and error semantics.
- Preserve `VipsSourceCustom` and `VipsTargetCustom` callback lifetime rules so original C tests keep passing under the Rust runtime.
- Keep the cache and threadpool state serializable across repeated init/shutdown cycles, since later threaded loaders depend on that behavior.

## Verification Phases
### `check_03_runtime_registry`
- Type: `check`
- Fixed `bounce_target`: `impl_03_runtime_registry`
- Purpose: validate the core `iofuncs` substrate, generated operation registry, and original runtime-facing C tests against the safe library.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo test --test runtime_io -- --nocapture
cargo test --test operation_registry -- --nocapture
python3 scripts/compare_operations.py \
  reference/operations.json \
  src/generated/operations.json
meson setup build-runtime . --wipe
meson compile -C build-runtime test_connections test_descriptors vips
python3 scripts/assert_not_reference_binary.py \
  /home/yans/code/safelibs/ported/libvips/build-check-install/lib/libvips.so.42.17.1 \
  "$PWD/build-runtime/lib/libvips.so.42.17.1"
LD_LIBRARY_PATH="$PWD/build-runtime/lib:${LD_LIBRARY_PATH:-}" \
  meson test -C build-runtime connections descriptors
LD_LIBRARY_PATH="$PWD/build-runtime/lib:${LD_LIBRARY_PATH:-}" \
  "$PWD/build-runtime/tools/vips" -l VipsOperation >/dev/null
LD_LIBRARY_PATH="$PWD/build-runtime/lib:${LD_LIBRARY_PATH:-}" \
  "$PWD/build-runtime/tools/vips" -l VipsObject >/dev/null
```

## Success Criteria
- `check_03_runtime_registry` passes without modification.
- The safe runtime executes the original runtime-facing C tests and serves a live operation and type registry.

## Git Commit Requirement
The implementer must commit work to git before yielding.
