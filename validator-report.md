# libvips-safe Validator Report

## Validator Checkout
- Repository: https://github.com/safelibs/validator
- Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- README invocation followed: local override root at validator-overrides/libvips with a generated port-04-test lock for proof-compatible override results
- Validator Python: validator/.venv/bin/python with PyYAML; unit checks required this venv at the front of PATH because host python3 could not import yaml.

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
| vips-cli-load-save | open | source surface | `impl_02_source_surface_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/vips-cli-load-save.json and validator/artifacts/libvips-safe/port-04-test/logs/libvips/vips-cli-load-save.log: `foreign: convert failed: No such file or directory (os error 2); copy: operation failed` |
| thumbnail-behavior | open | source surface | `impl_02_source_surface_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/thumbnail-behavior.json and validator/artifacts/libvips-safe/port-04-test/logs/libvips/thumbnail-behavior.log: `foreign: convert failed: No such file or directory (os error 2); thumbnail: operation failed` |
| usage-ruby-vips-gravity-generated | open | ruby usage operation | `impl_03_ruby_usage_operation_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json and validator/artifacts/libvips-safe/port-04-test/logs/libvips/usage-ruby-vips-gravity-generated.log: `gravity: operation not implemented` |
| usage-ruby-vips-crop-sample-jpeg | open | foreign I/O and buffer | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json and validator/artifacts/libvips-safe/port-04-test/logs/libvips/usage-ruby-vips-crop-sample-jpeg.log: `foreign: convert failed: No such file or directory (os error 2); extract_area: operation failed` |

## Skipped Validator Checks
- None

## Next Implementation Phase
- `impl_02_source_surface_failures`
