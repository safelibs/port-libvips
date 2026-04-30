# libvips-safe Validator Report

## validator checkout
- Validator URL: https://github.com/safelibs/validator
- remote main commit: dc9f47b6054e9a51afde8a437a2e5e5562cc946a
- active validator commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- active validator reason: remote main manifest unusable for libvips; using last known runnable validator commit
- Selection artifact: validator/artifacts/libvips-safe-validator-selection.txt
- Inventory artifact: validator/artifacts/libvips-safe-inventory.json
- Manifest SHA-256: c44346195fbfa8dd5de2c29b14ac9474eb3b91802d4fc2e7a5325141f9ee6140
- Python: validator/.venv/bin/python imports PyYAML 6.0.3; host python3 also imports PyYAML 6.0.3.
- Validator input model: README.md, repositories.yml, tests/libvips/testcases.yml, and test.sh confirm validation installs `.deb` packages from `<override-deb-root>/<library>/*.deb`; it does not validate a raw library path.
- Bootstrap note: origin/main was fetched and inspected first. Its libvips manifest no longer contains a non-empty `testcases` list, so the checkout was moved to the pinned fallback commit with 85 runnable libvips cases.

## Package Inputs
- Package-source commit: fc7c6a5171b9bccb3aad6ecb503b9d70a7c612c5
- Safe source edits in this phase: none.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`
- Package staging command: refreshed validator-overrides/libvips from the rebuilt root-level `.deb` files for the four canonical packages.
- Port lock: validator/artifacts/libvips-safe-port-lock.json

| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1300482 | 1d22c27d7893a2d510d8d5e3cce68a03258a485f86df1c9df5ab7803049b7567 |
| libvips-dev | validator-overrides/libvips/libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98 |
| libvips-tools | validator-overrides/libvips/libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be |
| gir1.2-vips-8.0 | validator-overrides/libvips/gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | 362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82 |

## Manifest Counts
- Source cases: 5
- Usage cases: 80
- Total cases: 85
- Source IDs: vips-cli-load-save, thumbnail-behavior, c-api-compile-smoke, gir-introspection-smoke, metadata-header-checks

## Baseline Command
```bash
ARTIFACT_NAME=libvips-safe
cd /home/yans/safelibs/pipeline/ports/port-libvips/validator
PYTHON="/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe \
  --mode port-04-test \
  --library libvips \
  --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides \
  --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json \
  --record-casts
```

## Result Summary
- Artifact root: validator/artifacts/libvips-safe
- Matrix exit status: 0
- Matrix exit artifact: validator/artifacts/libvips-safe/matrix-exit-code.txt
- Summary artifact: validator/artifacts/libvips-safe/port-04-test/results/libvips/summary.json
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 81
- Failed: 4
- Source cases: 5
- Usage cases: 80
- Override package install failures: none

## Phase 2 Source Surface Rerun
- Implement phase: `impl_02_source_surface_failures`.
- Root cause checked: no failing row is owned by this phase. The source-surface package/header/GIR/C ABI cases passed in both the baseline artifacts and the focused local rerun; the remaining source failures are JPEG load/save/thumbnail materialization through the foreign I/O path and stay owned by `impl_04_foreign_io_buffer_failures`.
- Regression test path: no new regression test was added because no distinct `impl_02_source_surface_failures` root cause was found. Existing focused coverage rerun: `safe/tests/runtime_io.rs`, `safe/tests/abi_layout.rs`, `safe/tests/init_version_smoke.rs`, `safe/scripts/link_compat.sh`, and `safe/scripts/check_introspection.sh`.
- Changed production files: none for this phase.
- Package-source commit used for the phase-2 rebuild: ed9c36636e8d1b1cfb12e68306fcfa94b0032931.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`.
- Refreshed local lock: validator/artifacts/libvips-safe-port-lock.json.
- Phase-2 artifact root: validator/artifacts/libvips-safe-source.
- Phase-2 matrix exit status: 0.
- Phase-2 summary artifact: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/summary.json.
- Inventory-derived counts: source cases 5, usage cases 80, total cases 85.
- Phase-2 results: 81 passed, 4 failed, 85 cast records, and no override package install failures.
- Remaining failure ownership after phase 2: `usage-ruby-vips-gravity-generated` remains with `impl_03_ruby_usage_operation_failures`; `vips-cli-load-save`, `thumbnail-behavior`, and `usage-ruby-vips-crop-sample-jpeg` remain with `impl_04_foreign_io_buffer_failures`; no failure remains owned by `impl_02_source_surface_failures`.

## Phase 3 Ruby Usage Operation Rerun
- Implement phase: `impl_03_ruby_usage_operation_failures`.
- Root cause checked: the remaining phase-3 usage failure was `usage-ruby-vips-gravity-generated`; it mapped to libvips nickname `gravity` and generated wrapper `vips_gravity(VipsImage *in, VipsImage **out, VipsCompassDirection direction, int width, int height, ...)`.
- Changed production files: `safe/src/ops/mod.rs` and `safe/src/ops/conversion.rs` now expose and dispatch `gravity`, using the existing embed pixel path for output pixels, metadata copying, and shared embed/gravity optional argument handling. The shared handling preserves libvips vararg semantics where `background` without an explicit `extend` selects `VIPS_EXTEND_BACKGROUND`, while an explicit `extend` still takes precedence; `VIPS_EXTEND_MIRROR` now uses libvips' `2 * size` mirrored tile period.
- Regression test path: `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre` calls the exported C ABI with the same generated 3x3 grayscale to 2x2 centre crop shape as the Ruby validator case. `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend` covers the senior-review background vararg case and verifies explicit `extend` still wins over `background`. `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics` covers the libvips mirror-tile semantics that differ from a `2 * size - 2` reflection period.
- Focused tests passed: `cargo test --all-features --test ops_core -- --nocapture`, `cargo test --all-features --test ops_advanced -- --nocapture`, `cargo test --all-features --test operation_registry -- --nocapture`, and `cargo test --all-features --test security -- --nocapture`.
- Package-source commit used for the phase-3 rebuild: 4ec81e43c7d54bb908d5e7732979e476a42ae6e3.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`.
- Refreshed local lock: validator/artifacts/libvips-safe-port-lock.json.
- Phase-3 artifact root: validator/artifacts/libvips-safe-ops.
- Phase-3 matrix exit status: 0.
- Phase-3 summary artifact: validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/summary.json.
- Inventory-derived counts: source cases 5, usage cases 80, total cases 85.
- Phase-3 results: 82 passed, 3 failed, 85 cast records, and no override package install failures.
- Phase-3 fixed testcase: `usage-ruby-vips-gravity-generated` passed in validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json.
- Remaining failure ownership after phase 3: `vips-cli-load-save`, `thumbnail-behavior`, and `usage-ruby-vips-crop-sample-jpeg` remain with `impl_04_foreign_io_buffer_failures`; no failure remains owned by `impl_02_source_surface_failures` or `impl_03_ruby_usage_operation_failures`.

## failure classification
| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |
| --- | --- | --- | --- | --- | --- | --- | --- |
| vips-cli-load-save | source | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/vips-cli-load-save.json | JPEG load/save materialization falls back to external `convert` inside the validator container, which is absent; `copy` fails before PNG output. | Add phase 4 coverage for JPEG file load and save materialization without external decoder fallback. | Open for `impl_04_foreign_io_buffer_failures`. |
| thumbnail-behavior | source | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/thumbnail-behavior.json | `vipsthumbnail` hits the same JPEG materialization path and fails when external `convert` is absent. | Add phase 4 coverage for thumbnailing a JPEG fixture without external decoder fallback. | Open for `impl_04_foreign_io_buffer_failures`. |
| usage-ruby-vips-crop-sample-jpeg | usage | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json | Ruby crop of a JPEG fixture fails during materialization through the missing external `convert` path before `extract_area` can complete. | Add phase 4 coverage for ruby-vips JPEG fixture load, crop, and materialization through file or buffer APIs. | Open for `impl_04_foreign_io_buffer_failures`. |
| usage-ruby-vips-gravity-generated | usage | fixed | `impl_03_ruby_usage_operation_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json | Generated-image `gravity` dispatch reached ruby-vips but the safe operation implementation returned `gravity: operation not implemented`. | `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre` covers the generated-image gravity centre crop through the exported C ABI; `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend` covers libvips `background`/`extend` vararg semantics; `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics` covers `VIPS_EXTEND_MIRROR`. | Fixed by implementing `gravity` support in `safe/src/ops/conversion.rs`, routing it from `safe/src/ops/mod.rs`, and sharing embed/gravity optional argument handling so `background` without `extend` uses `VIPS_EXTEND_BACKGROUND` and mirror extension uses libvips' `2 * size` tile period; passed in validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json. |

## Later Owner Phases
- `impl_02_source_surface_failures`: no baseline failure is assigned here because the source-case failures depend on JPEG decode, save, or materialization rather than command, header, package identity, or metadata surface alone.
- `impl_03_ruby_usage_operation_failures`: generated ruby-vips operation behavior failure fixed in phase 3.
- `impl_04_foreign_io_buffer_failures`: owns file, buffer, loader, saver, lazy materialization, and external decoder fallback failures.
- `impl_05_packaging_container_and_remaining_failures`: no baseline packaging or container setup failure was observed; all override packages installed and matrix exit was 0.
