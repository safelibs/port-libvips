# Phase 06

## Phase Name
Advanced Ops, SIMD Helpers, And Threading

## Implement Phase ID
`impl_06_advanced_ops`

## Preexisting Inputs
- `safe/src/ops/colour.rs`
- `safe/src/ops/resample.rs`
- `safe/src/ops/draw.rs`
- `safe/src/ops/mosaicing.rs`
- `original/libvips/colour/`
- `original/libvips/resample/`
- `original/libvips/draw/`
- `original/libvips/mosaicing/`
- `original/libvips/iofuncs/vector.cpp`
- `original/libvips/resample/reduceh_hwy.cpp`
- `original/libvips/resample/reducev_hwy.cpp`

## New Outputs
- Updated `safe/src/ops/colour.rs`
- Updated `safe/src/ops/resample.rs`
- Updated `safe/src/ops/draw.rs`
- Updated `safe/src/ops/mosaicing.rs`
- Updated `safe/src/simd/mod.rs`
- Updated `safe/src/simd/vector.rs`
- Updated `safe/src/simd/reduce.rs`
- Updated `safe/tests/ops_advanced.rs`
- Updated `safe/tests/threading.rs`
- Updated `safe/tests/security/cve_2018_7998.rs`

## File Changes
- Port the precision-sensitive and invalidation-sensitive operation families.
- Replace any remaining in-tree vector/SIMD glue that still relies on copied upstream runtime code.
- Extend the shared security suite with the delayed-load race regression.

## Implementation Details
- Preserve draw invalidation, thumbnail/resample caching behavior, and colour/profile surface expected by callers such as `nip2` and `photoqt`.
- Keep delayed-load failure behavior deterministic across threads; no panics, stale state, or partially initialized output images.
- Any temporary unsafe SIMD bridge must be isolated and either removed or explicitly justified by the end of phase 9.

## Verification Phases
### `check_06_advanced_ops`
- Type: `check`
- Fixed `bounce_target`: `impl_06_advanced_ops`
- Purpose: validate colour, resample, draw, mosaicing, SIMD-sensitive helpers, and delayed-load/threading behavior.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo test --test ops_advanced -- --nocapture
cargo test --test threading -- --nocapture
cargo test --test security -- cve_2018_7998
```

## Success Criteria
- `check_06_advanced_ops` passes without modification.
- The advanced operation families, SIMD helpers, and threaded delayed-load behavior remain compatible.

## Git Commit Requirement
The implementer must commit work to git before yielding.
