# libvips-safe Validator Report

## Validator Checkout
- Repository: https://github.com/safelibs/validator
- Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- README invocation followed: local override root at validator-overrides/libvips with a generated port-04-test lock for proof-compatible override results
- Validator Python: validator/.venv/bin/python with PyYAML; host python3 also imports PyYAML 6.0.3 from the user site so render-site unit subprocesses that invoke python3 directly pass under `PYTHON="$VALIDATOR_PY" make unit`.

## Safe Package Inputs
- Safe commit before validator: 12543f951c24648d94d82e9809a02ed679602ef7
- Override packages: libvips42t64_8.15.1-1.1build4_amd64.deb, libvips-dev_8.15.1-1.1build4_amd64.deb, libvips-tools_8.15.1-1.1build4_amd64.deb, gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb
- Safe crate: safe/Cargo.toml defines crate vips version 8.15.1 and produces cdylib, staticlib, and rlib outputs.
- Public surface: safe/src/lib.rs re-exports ABI, runtime, operation, foreign, pixel, SIMD, and generated metadata modules.
- Package contract: safe/meson.build keeps version 8.15.1, SONAME libvips.so.42, full library name libvips.so.42.17.1, Cargo feature mapping, and Debian package generation for libvips42t64, libvips-dev, libvips-tools, and gir1.2-vips-8.0.
- Reference install: build-check-install was derived from the local original/ snapshot because no existing build-check install was present; include/vips/, lib/libvips.so.42.17.1, and lib/libvips-cpp.so.42.17.1 were verified.

## Test Inventory
- Source cases: 5
- Usage cases: 80
- Total cases: 85
- Validator README context: Docker-based Ubuntu 24.04 original and port override matrix.
- Canonical apt packages: libvips42t64, libvips-dev, libvips-tools, gir1.2-vips-8.0.
- Source coverage: CLI load/save, vipsthumbnail, C API compile/link, GObject introspection, and metadata header output.
- Usage coverage: ruby-vips generated images, PNG/JPEG fixtures, file and buffer roundtrips, arithmetic/scalar operations, band operations, transforms, joins, insert/embed/gravity/zoom/subsample, statistics, boolean operations, and foreign PNG/JPEG behavior.

## Initial Run
- Command: `RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts`
- Artifact root: validator/artifacts/libvips-safe
- Mode: port-04-test
- Port deb lock: validator/artifacts/libvips-safe-port-lock.json
- Matrix exit status: 0
- Proof verification: passed, proof/libvips-safe-validation-proof.json
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 81
- Failed: 4

## Failure Classification
| Testcase ID | Status | Class | Owner phase | Evidence |
| --- | --- | --- | --- | --- |
| vips-cli-load-save | fixed | source surface | `impl_02_source_surface_failures` | Initial log validator/artifacts/libvips-safe/port-04-test/logs/libvips/vips-cli-load-save.log failed with `foreign: convert failed: No such file or directory`; rerun log validator/artifacts/libvips-safe-source/port-04-test/logs/libvips/vips-cli-load-save.log passes and reports `/tmp/validator-tmp/out.png: 290x442 uchar, 3 bands, srgb, pngload`. |
| thumbnail-behavior | fixed | source surface | `impl_02_source_surface_failures` | Initial log validator/artifacts/libvips-safe/port-04-test/logs/libvips/thumbnail-behavior.log failed with `foreign: convert failed: No such file or directory`; rerun log validator/artifacts/libvips-safe-source/port-04-test/logs/libvips/thumbnail-behavior.log passes and reports `/tmp/validator-tmp/thumb.jpg: 21x32 uchar, 3 bands, srgb, jpegload`. |
| usage-ruby-vips-gravity-generated | fixed | ruby usage operation | `impl_03_ruby_usage_operation_failures` | Initial result validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json and log validator/artifacts/libvips-safe/port-04-test/logs/libvips/usage-ruby-vips-gravity-generated.log failed with `gravity: operation not implemented`; phase 3 rerun result validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json passed. |
| usage-ruby-vips-crop-sample-jpeg | fixed by source JPEG materialization | foreign I/O and buffer | `impl_04_foreign_io_buffer_failures` | Initial log validator/artifacts/libvips-safe/port-04-test/logs/libvips/usage-ruby-vips-crop-sample-jpeg.log failed through the same missing external `convert` JPEG decode path; rerun result validator/artifacts/libvips-safe-source/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json passed. |

## Source Surface Phase Rerun
- Implement phase: `impl_02_source_surface_failures`
- Safe commit after fixes: 73d48f1567badf0110d1d562fc2a27d4c184ebbc
- Packages rebuilt and staged under validator-overrides/libvips: libvips42t64, libvips-dev, libvips-tools, and gir1.2-vips-8.0.
- Port deb lock refreshed in place at validator/artifacts/libvips-safe-port-lock.json with mode `port-04-test`, the four canonical packages, and `unported_original_packages: []`.
- Command: `RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-source --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts`
- Artifact root: validator/artifacts/libvips-safe-source
- Matrix exit status: 0
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 84
- Failed: 1
- Source cases passed: 5 of 5

## Source Case Details
| Testcase ID | Rerun result | Root cause | Regression coverage | Production files changed |
| --- | --- | --- | --- | --- |
| vips-cli-load-save | passed: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/vips-cli-load-save.json | Lazy JPEG materialization used the external `convert` binary, which is not present in the validator image. After that was fixed, `vipsheader` still printed only `image` because VipsImage did not provide a summary with dimensions. | safe/tests/runtime_io.rs: `jpeg_file_copy_to_png_materializes_without_external_convert` writes a JPEG through `vips_image_new_from_file` and `vips_image_write_to_file`, then asserts the public object summary contains the image dimensions. | safe/Cargo.toml, safe/Cargo.lock, safe/src/foreign/mod.rs, safe/src/foreign/loaders/mod.rs, safe/src/foreign/loaders/jpeg.rs, safe/src/runtime/object.rs |
| thumbnail-behavior | passed: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/thumbnail-behavior.json | `vipsthumbnail` hit the same pending JPEG decode path and failed when the external `convert` binary was absent. | safe/tests/runtime_io.rs: `jpeg_file_thumbnail_materializes_without_external_convert` exercises the public `vips_thumbnail` path for a JPEG file. | safe/Cargo.toml, safe/Cargo.lock, safe/src/foreign/mod.rs, safe/src/foreign/loaders/mod.rs, safe/src/foreign/loaders/jpeg.rs |
| c-api-compile-smoke | passed: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/c-api-compile-smoke.json | No source-surface failure in this phase rerun. | Existing C API/link compatibility coverage remained sufficient. | None |
| gir-introspection-smoke | passed: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/gir-introspection-smoke.json | No source-surface failure in this phase rerun. | Existing introspection smoke coverage remained sufficient. | None |
| metadata-header-checks | passed: validator/artifacts/libvips-safe-source/port-04-test/results/libvips/metadata-header-checks.json | No source-surface failure in this phase rerun. | Existing metadata/header coverage remained sufficient. | None |

## Ruby Usage Operation Phase Rerun
- Implement phase: `impl_03_ruby_usage_operation_failures`
- Safe commit after operation fixes: 9890f1a510031d1443e16d05a6228a31cc9e8590
- Packages rebuilt with `dpkg-buildpackage -us -uc -b` and staged under validator-overrides/libvips: libvips42t64, libvips-dev, libvips-tools, and gir1.2-vips-8.0.
- Port deb lock refreshed in place at validator/artifacts/libvips-safe-port-lock.json with mode `port-04-test`, commit 9890f1a510031d1443e16d05a6228a31cc9e8590, release tag `build-9890f1a51003`, the four canonical packages, and `unported_original_packages: []`.
- Command: `RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-ops --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts`
- Artifact root: validator/artifacts/libvips-safe-ops
- Matrix exit status: 0
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 85
- Failed: 0
- Summary: validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/summary.json reports 5 source cases and 80 usage cases, all passed.

## Ruby Usage Operation Case Details
| Testcase ID | Rerun result | Root cause | Regression coverage | Production files changed |
| --- | --- | --- | --- | --- |
| usage-ruby-vips-gravity-generated | passed: validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json | The generated wrapper and operation type exposed `vips_gravity`, but the conversion dispatch table did not handle the `gravity` nickname, so Ruby reached `generated_operation_build` and received `gravity: operation not implemented`. | safe/tests/ops_core.rs: `gravity_centre_crop_matches_ruby_usage_case` calls the exported `vips_gravity` symbol on a 3x3 generated grayscale image, asserts a 2x2 uchar single-band output, and verifies the centered crop payload `[1, 2, 4, 5]`. | safe/src/ops/conversion.rs, safe/src/ops/mod.rs |

## Remaining Open Failures
- None. The phase 3 operation-owned usage failure is closed. The earlier sample JPEG crop row remains documented as fixed by source JPEG materialization rather than as an operation failure.

## Skipped Validator Checks
- None

## Next Implementation Phase
- `impl_04_foreign_io_buffer_failures`
