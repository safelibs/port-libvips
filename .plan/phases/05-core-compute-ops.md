# Phase 05

## Phase Name
Core Compute Families And Initial Security Regressions

## Implement Phase ID
`impl_05_core_compute_ops`

## Preexisting Inputs
- `safe/src/pixels/`
- `safe/src/ops/mod.rs`
- `original/libvips/arithmetic/`
- `original/libvips/conversion/`
- `original/libvips/convolution/`
- `original/libvips/create/`
- `original/libvips/histogram/`
- `original/libvips/morphology/`
- `original/libvips/freqfilt/`

## New Outputs
- Updated `safe/src/pixels/mod.rs`
- Updated `safe/src/pixels/format.rs`
- Updated `safe/src/pixels/iter.rs`
- Updated `safe/src/pixels/kernel.rs`
- Updated `safe/src/ops/arithmetic.rs`
- Updated `safe/src/ops/conversion.rs`
- Updated `safe/src/ops/convolution.rs`
- Updated `safe/src/ops/create.rs`
- Updated `safe/src/ops/histogram.rs`
- Updated `safe/src/ops/morphology.rs`
- Updated `safe/src/ops/freqfilt.rs`
- Updated `safe/tests/ops_core.rs`
- Updated `safe/tests/security.rs`
- Updated `safe/tests/security/cve_2021_27847.rs`
- Updated `safe/tests/security/cve_2026_3284.rs`

## File Changes
- Flesh out the pixel buffer, typed-sample, and kernel helpers used by the core compute families.
- Finish the manifest-driven operation handlers for the core image-processing families.
- Keep the shared security test target as the single place for CVE regressions.

## Implementation Details
- Preserve format promotion, saturation, metadata propagation, and region-demand behavior expected by upstream callers.
- Prefer shared typed-kernel helpers over ad hoc per-operation pointer code so safety and numerical rules stay consistent.
- Preserve checked arithmetic in extract-area and degenerate-dimension handling in mask/eye operations.

## Verification Phases
### `check_05_core_compute_ops`
- Type: `check`
- Fixed `bounce_target`: `impl_05_core_compute_ops`
- Purpose: validate arithmetic, conversion, convolution, create, histogram, morphology, and frequency-filter behavior together with the first core security regressions.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo test --test ops_core -- --nocapture
cargo test --test security -- cve_2021_27847
cargo test --test security -- cve_2026_3284
```

## Success Criteria
- `check_05_core_compute_ops` passes without modification.
- The core compute families and initial security regressions behave compatibly through the safe implementation.

## Git Commit Requirement
The implementer must commit work to git before yielding.
