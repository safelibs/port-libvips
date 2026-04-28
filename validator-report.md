# libvips-safe Validator Report

## Validator Checkout
- Repository: https://github.com/safelibs/validator
- Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- README invocation followed: local override root at validator-overrides/libvips with a generated port-04-test lock for proof-compatible override results
- Validator Python: validator/.venv/bin/python with PyYAML; host python3 also imports PyYAML 6.0.3 from the user site so render-site unit subprocesses that invoke python3 directly pass under `PYTHON="$VALIDATOR_PY" make unit`.

## Safe Package Inputs
- Safe commit before phase 5 worktree fixes: dd47364b3944a3be152d91d9a72958d2fd01e1a3
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
| usage-ruby-vips-crop-sample-jpeg | fixed by source JPEG materialization | foreign I/O and buffer | `impl_04_foreign_io_buffer_failures` | Initial log validator/artifacts/libvips-safe/port-04-test/logs/libvips/usage-ruby-vips-crop-sample-jpeg.log failed through the same missing external `convert` JPEG decode path; source rerun result validator/artifacts/libvips-safe-source/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json passed, and phase 4 rerun result validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json passed. |

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

## Foreign I/O Buffer Phase Rerun
- Implement phase: `impl_04_foreign_io_buffer_failures`
- Safe commit after buffer fixes: 6e33af34dec46ac37b98b1ecf713a716ddc06643
- Packages rebuilt with `dpkg-buildpackage -b -uc -us` and staged under validator-overrides/libvips: libvips42t64, libvips-dev, libvips-tools, and gir1.2-vips-8.0.
- Port deb lock refreshed in place at validator/artifacts/libvips-safe-port-lock.json with mode `port-04-test`, commit 6e33af34dec46ac37b98b1ecf713a716ddc06643, release tag `build-6e33af34dec4`, the four canonical packages, and `unported_original_packages: []`.
- Local verification: `cargo test --all-features --test runtime_io -- --nocapture`, `cargo test --all-features --test security -- --nocapture`, `cargo test --all-features -- --nocapture`, `meson setup build-validator-foreign . --prefix "$PWD/.tmp/validator-foreign-prefix"`, `meson compile -C build-validator-foreign`, and `tests/upstream/run-shell-suite.sh build-validator-foreign` all passed.
- Package build verification: Debian package build ran the upstream Meson suite with 9 passed and 1 expected skip, then produced the four canonical override packages.
- Command: `RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-foreign --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts`
- Artifact root: validator/artifacts/libvips-safe-foreign
- Matrix exit status: 0
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 85
- Failed: 0
- Summary: validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/summary.json reports 5 source cases and 80 usage cases, all passed.

## Foreign I/O Buffer Case Details
| Surface | Rerun result | Root cause | Regression coverage | Production files changed |
| --- | --- | --- | --- | --- |
| JPEG buffer C API (`vips_jpegload_buffer`, `vips_jpegsave_buffer`) | Full phase 4 validator rerun passed: validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/summary.json | The generated varargs wrappers treated raw `(void *, size_t)` buffer parameters as operation required arguments, while the operation dispatch expects a `VipsBlob`; the generated `jpegsave_buffer` path also returned a boxed blob instead of GLib-freeable data plus length. PNG already had manual shims, so JPEG now mirrors that behavior. | safe/tests/runtime_io.rs: `public_foreign_buffer_apis_round_trip_png_and_jpeg` exercises `vips_image_new_from_buffer`, `vips_image_write_to_buffer`, `vips_source_new_from_memory`, `pngload_buffer`, `jpegload_buffer`, `pngsave_buffer`, and `jpegsave_buffer`, verifies dimensions/bands/format/interpretation/loader metadata, reloads returned buffers, and frees returned memory with `g_free`. | safe/build.rs, safe/build_support/api_shim.c, safe/tests/runtime_io.rs |
| File/source/target PNG/JPEG APIs | Full phase 4 validator rerun passed: validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/summary.json | No remaining validator failure after source and operation phases, but phase 4 required direct public API coverage for file/source/target I/O ownership and metadata preservation. | safe/tests/runtime_io.rs: `public_foreign_file_source_and_target_apis_round_trip_png_and_jpeg` exercises `pngload`, `jpegload`, `pngsave`, `jpegsave`, `vips_source_new_from_file`, `vips_target_new_to_file`, and target writes, then reloads outputs and checks sample dimensions, bands, band format, interpretation, and `vips-loader` metadata. | safe/tests/runtime_io.rs |

## Packaging Container And Remaining Phase Rerun
- Implement phase: `impl_05_packaging_container_and_remaining_failures`
- Remaining failure classification: no post-phase4 validator matrix failures remained, but `safe/scripts/run_release_gate.sh` exposed a behavior failure in the upstream Python suite: `TestForeign::test_truncated` loaded `original/test/test-suite/images/truncated.jpg` with default options, then `im.avg()` failed with `jpegload: failed to fill whole buffer; avg: operation failed`.
- Root cause: pending JPEG decode treated truncated scan data as fatal even when libvips default `fail_on=none` should allow permissive materialization. The loader also attempted to read `fail_on` as a string before falling back to enum handling.
- Fix: default JPEG materialization now falls back to zero-filled pixels sized from the JPEG header after a decode error; `fail_on=truncated` remains strict and reports the decoder error. Loader option extraction now reads `fail_on` as the enum value directly.
- Regression coverage: safe/tests/runtime_io.rs: `truncated_jpeg_default_load_materializes_permissively` loads the upstream truncated JPEG twice with default options, verifies dimensions and finite `avg`, then verifies `fail_on=truncated` stays strict at materialization time.
- Production files changed: safe/src/foreign/loaders/jpeg.rs, safe/src/foreign/mod.rs, safe/tests/runtime_io.rs.
- Release gate: `safe/scripts/run_release_gate.sh` passed. It covered Rust tests, Meson install/surface checks, the upstream Python suite (`203 passed, 49 skipped`), Debian package checks, extracted-package checks, and dependent application smokes.
- Packages rebuilt with `dpkg-buildpackage -b -uc -us` and staged under validator-overrides/libvips: libvips42t64, libvips-dev, libvips-tools, and gir1.2-vips-8.0.
- Port deb lock refreshed in place at validator/artifacts/libvips-safe-port-lock.json with mode `port-04-test`, the four canonical packages, and `unported_original_packages: []`.
- Command: `RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-remaining --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts`
- Artifact root: validator/artifacts/libvips-safe-remaining
- Matrix exit status: 0
- Proof verification: `validator/.venv/bin/python tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-remaining --proof-output proof/libvips-safe-validation-proof.json --mode port-04-test --library libvips --require-casts --min-source-cases 5 --min-usage-cases 80 --min-cases 85` passed and wrote validator/artifacts/libvips-safe-remaining/proof/libvips-safe-validation-proof.json.
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 85
- Failed: 0
- Summary: validator/artifacts/libvips-safe-remaining/port-04-test/results/libvips/summary.json reports 5 source cases and 80 usage cases, all passed.

## Final Clean Run
- Implement phase: `impl_06_final_report_and_clean_run`
- Validator repository: https://github.com/safelibs/validator
- Validator commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Safe source commit before first final run: 5688a1ddbe9289dea4ed85ebe6f913c542538e48
- Final safe source commit used for package lock and final validator evidence: 32d51b52bdba91942d1ed26994fc31c505bdca0d
- Evidence provenance: the required final clean-run block refreshed `validator/artifacts/libvips-safe-port-lock.json` and `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`; both record `32d51b52bdba91942d1ed26994fc31c505bdca0d` with release tag `build-32d51b52bdba`.
- Final phase production changes: None.
- Final phase safe test fix: `safe/tests/runtime_io.rs` now resets `vips_cache_set_max_files()` and `vips_cache_set_max_mem()` in `operation_cache_build_and_drop_all_are_stateful` before asserting cache size, preventing prior runtime tests' low cache limits from trimming the probe operation immediately.
- Approved validator-bug skips: None.
- Validator hygiene: `git -C validator diff -- README.md repositories.yml tests/libvips/testcases.yml tests/libvips/Dockerfile tests/libvips/host-run.sh tests/libvips/docker-entrypoint.sh` produced no diff. Validator status only contains untracked local venv/artifact roots.
- Top-level preexisting unrelated work preserved: deleted `.plan/phases/*`, modified `.plan/workflow-structure.yaml`, and untracked `safe/.tmp/` were not touched or committed by this phase.

## Final Package Overrides
| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1386062 | 87784773f188643e092d28e4dc4a548c23abb99d909626e24a2cf1eb6cd118b0 |
| libvips-dev | validator-overrides/libvips/libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98 |
| libvips-tools | validator-overrides/libvips/libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be |
| gir1.2-vips-8.0 | validator-overrides/libvips/gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | 362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82 |

## Final Commands Executed
Package rebuild:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
dpkg-buildpackage -b -uc -us
```

Package staging:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips
version=$(dpkg-parsechangelog -l safe/debian/changelog -SVersion)
arch=$(dpkg-architecture -qDEB_HOST_ARCH)
install -m 0644 "libvips42t64_${version}_${arch}.deb" validator-overrides/libvips/
install -m 0644 "libvips-dev_${version}_${arch}.deb" validator-overrides/libvips/
install -m 0644 "libvips-tools_${version}_${arch}.deb" validator-overrides/libvips/
install -m 0644 "gir1.2-vips-8.0_${version}_${arch}.deb" validator-overrides/libvips/
```

Local lock regeneration:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips
root=$(pwd)
override_dir="$root/validator-overrides/libvips"
lock_path="$root/validator/artifacts/libvips-safe-port-lock.json"
commit=$(git rev-parse HEAD)
release_tag="build-${commit:0:12}"
debs='[]'
for package in libvips42t64 libvips-dev libvips-tools gir1.2-vips-8.0; do
  deb_path=$(find "$override_dir" -maxdepth 1 -type f -name "${package}_*.deb" | sort | tail -n 1)
  filename=$(basename "$deb_path")
  architecture=$(dpkg-deb --field "$deb_path" Architecture)
  sha256=$(sha256sum "$deb_path" | awk '{print $1}')
  size=$(stat -c '%s' "$deb_path")
  debs=$(jq --arg package "$package" --arg filename "$filename" --arg architecture "$architecture" --arg sha256 "$sha256" --argjson size "$size" --arg url "file://$deb_path" '. + [{package:$package, filename:$filename, architecture:$architecture, sha256:$sha256, size:$size, browser_download_url:$url}]' <<<"$debs")
done
jq -n --arg commit "$commit" --arg release_tag "$release_tag" --argjson debs "$debs" '{schema_version:1, mode:"port-04-test", generated_at:"1970-01-01T00:00:00Z", source_config:"repositories.yml", source_inventory:"local-validator-overrides", libraries:[{library:"libvips", repository:"safelibs/port-libvips-local", tag_ref:"refs/tags/libvips/local-validator", commit:$commit, release_tag:$release_tag, debs:$debs, unported_original_packages:[]}]}' > "$lock_path"
```

Final validator matrix:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/validator
PYTHON="/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-final --mode port-04-test --library libvips --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json --record-casts
```

Proof, site render, and site verification:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/validator
/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-final --proof-output proof/libvips-safe-validation-proof.json --mode port-04-test --library libvips --require-casts --min-source-cases 5 --min-usage-cases 80 --min-cases 85
/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python tools/render_site.py --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-final --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json --output-root site/libvips-safe-final
bash scripts/verify-site.sh --config repositories.yml --tests-root tests --artifacts-root artifacts/libvips-safe-final --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json --site-root site/libvips-safe-final --library libvips
```

Local safe verification:
```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features -- --nocapture
scripts/run_release_gate.sh
```

## Final Evidence
- Port deb lock: validator/artifacts/libvips-safe-port-lock.json
- Final artifact root: validator/artifacts/libvips-safe-final
- Final summary: validator/artifacts/libvips-safe-final/port-04-test/results/libvips/summary.json
- Final proof: validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json
- Final rendered site: validator/site/libvips-safe-final
- Final summary totals: 85 cases, 5 source cases, 80 usage cases, 85 casts, 85 passed, 0 failed.
- Proof totals: 85 cases, 5 source cases, 80 usage cases, 85 casts, 85 passed, 0 failed.
- Result JSON records: 85 testcase records plus summary.json.
- Cast records: 85.
- Local safe verification: `cargo test --all-features -- --nocapture` passed; `safe/scripts/run_release_gate.sh` passed, including Rust tests, Meson install/surface checks, upstream Meson suite (`9 passed, 1 skipped`), upstream Python suite (`203 passed, 49 skipped`), Debian package checks, extracted-package checks, and dependent application smokes.
- Package lock verification: staged package names, architectures, sizes, and SHA-256 values match validator/artifacts/libvips-safe-port-lock.json.
- Lock/proof provenance verification: after the final clean-run block, `validator/artifacts/libvips-safe-port-lock.json` `.libraries[0].commit` and `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json` `.libraries[0].port_commit` both match `32d51b52bdba91942d1ed26994fc31c505bdca0d`.

## Remaining Open Failures
- None. The final no-skip validator run passed all 85 cases, and no approved validator-bug skip was used.

## Skipped Validator Checks
- None

## Next Workflow Phases
- `check_06_final_software_tester`
- `check_06_final_senior_tester`
- Bounce target for both verifiers: `impl_06_final_report_and_clean_run`
