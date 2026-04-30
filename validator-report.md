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
- Package-source commit: 909571ec603b7c6e1e624aeef92f4d414180156c
- Safe source edits in this phase: native JPEG header/decode support, foreign buffer wrapper ownership fixes, GLib-owned public buffers, and image object summaries with dimensions.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`
- Package staging command: refreshed validator-overrides/libvips from the rebuilt root-level `.deb` files for the four canonical packages.
- Port lock: validator/artifacts/libvips-safe-port-lock.json

| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1430036 | 097f7756514f31e6ca15ea147013e70190a2d2e31e69405c94b4379e863b168a |
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

## Phase 4 Foreign I/O And Buffer Rerun
- Implement phase: `impl_04_foreign_io_buffer_failures`.
- Root cause checked: the remaining failures all entered foreign JPEG load/materialization. Lazy JPEG decode delegated to external `convert`, which is absent in the validator container; generated load-buffer and save-buffer wrappers also needed explicit `VipsBlob`/`VipsArea` ownership handling; after load/save worked, `vipsheader` still needed image object summaries to report dimensions for CLI compatibility.
- Changed production files: `safe/src/foreign/loaders/jpeg.rs`, `safe/src/foreign/loaders/mod.rs`, `safe/src/foreign/mod.rs`, `safe/build.rs`, `safe/src/runtime/image.rs`, `safe/src/runtime/object.rs`, `safe/Cargo.toml`, and `safe/Cargo.lock`.
- Regression test path: `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert`, `safe/tests/runtime_io.rs::jpeg_public_save_paths_return_glib_owned_buffers_and_targets`, `safe/tests/runtime_io.rs::jpeg_thumbnail_materializes_and_saves_without_convert`, and `safe/tests/runtime_io.rs::image_object_summary_reports_dimensions_for_vipsheader`.
- Focused tests passed: `cargo test --all-features --test runtime_io -- --nocapture`, `cargo test --all-features --test security -- --nocapture`, `meson setup build-validator-foreign . --wipe --prefix "$PWD/.tmp/validator-foreign-prefix"`, `meson compile -C build-validator-foreign`, `tests/upstream/run-shell-suite.sh build-validator-foreign`, and `tests/upstream/run-fuzz-suite.sh build-validator-foreign`.
- Package-source commit used for the phase-4 rebuild: 909571ec603b7c6e1e624aeef92f4d414180156c.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`.
- Refreshed local lock: validator/artifacts/libvips-safe-port-lock.json.
- Phase-4 artifact root: validator/artifacts/libvips-safe-foreign.
- Phase-4 matrix exit status: 0.
- Phase-4 summary artifact: validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/summary.json.
- Inventory-derived counts: source cases 5, usage cases 80, total cases 85.
- Phase-4 results: 85 passed, 0 failed, 85 cast records, and no override package install failures.
- Phase-4 fixed testcases: `vips-cli-load-save`, `thumbnail-behavior`, and `usage-ruby-vips-crop-sample-jpeg` passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/.
- Remaining failure ownership after phase 4: no failures remain owned by `impl_02_source_surface_failures`, `impl_03_ruby_usage_operation_failures`, or `impl_04_foreign_io_buffer_failures`; the phase-4 full validator rerun has no remaining failed libvips testcase records.

## failure classification
| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |
| --- | --- | --- | --- | --- | --- | --- | --- |
| vips-cli-load-save | source | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/vips-cli-load-save.json | JPEG load/save materialization fell back to external `convert` inside the validator container, which is absent; after copy succeeded, `vipsheader` also needed image object summaries to include dimensions. | `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert`, `safe/tests/runtime_io.rs::jpeg_public_save_paths_return_glib_owned_buffers_and_targets`, and `safe/tests/runtime_io.rs::image_object_summary_reports_dimensions_for_vipsheader`. | Fixed with native JPEG header/decode materialization, generated `VipsBlob`/`VipsArea` buffer wrapper ownership, GLib-owned returned buffers, and image summaries containing dimensions; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/vips-cli-load-save.json. |
| thumbnail-behavior | source | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/thumbnail-behavior.json | `vipsthumbnail` hit the same JPEG materialization path and failed when external `convert` was absent. | `safe/tests/runtime_io.rs::jpeg_thumbnail_materializes_and_saves_without_convert`. | Fixed by materializing JPEG pixels with the native Rust decoder before thumbnail/save paths need pixels; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/thumbnail-behavior.json. |
| usage-ruby-vips-crop-sample-jpeg | usage | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json | Ruby crop of a JPEG fixture failed during materialization through the missing external `convert` path before `extract_area` could complete. | `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert` covers JPEG fixture load/materialization through public file, buffer, source, and explicit loader APIs. | Fixed by replacing JPEG lazy materialization's external decoder fallback with native JPEG decode; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json. |
| usage-ruby-vips-gravity-generated | usage | fixed | `impl_03_ruby_usage_operation_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json | Generated-image `gravity` dispatch reached ruby-vips but the safe operation implementation returned `gravity: operation not implemented`. | `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre` covers the generated-image gravity centre crop through the exported C ABI; `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend` covers libvips `background`/`extend` vararg semantics; `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics` covers `VIPS_EXTEND_MIRROR`. | Fixed by implementing `gravity` support in `safe/src/ops/conversion.rs`, routing it from `safe/src/ops/mod.rs`, and sharing embed/gravity optional argument handling so `background` without `extend` uses `VIPS_EXTEND_BACKGROUND` and mirror extension uses libvips' `2 * size` tile period; passed in validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json. |

## Later Owner Phases
- `impl_02_source_surface_failures`: no baseline failure is assigned here because the source-case failures depend on JPEG decode, save, or materialization rather than command, header, package identity, or metadata surface alone.
- `impl_03_ruby_usage_operation_failures`: generated ruby-vips operation behavior failure fixed in phase 3.
- `impl_04_foreign_io_buffer_failures`: file, buffer, loader, saver, lazy materialization, and external decoder fallback failures fixed in phase 4.
- `impl_05_packaging_container_and_remaining_failures`: no packaging or container setup failure remains; all override packages installed and the phase-4 matrix exit was 0.
