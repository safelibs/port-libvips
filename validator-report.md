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
- Package-source commit: 7251b2d7efbc4adf60b0a98ce84c380fcaf1f415
- Safe source edits in this phase: default truncated JPEG materialization now falls back only when libvips defaults allow it, failed file-load cache entries are invalidated before fallback, strict `fail_on=truncated` still rejects pixels, and `fail_on` loader options are read through enum access without GLib critical warnings.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`
- Package staging command: refreshed validator-overrides/libvips from the rebuilt root-level `.deb` files for the four canonical packages.
- Port lock: validator/artifacts/libvips-safe-port-lock.json

| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1430662 | ecda9f408ce52e33f3b65d20d844b821155af24c55973e13c3a51515bf3fd279 |
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

## Phase 5 Packaging, Container, And Remaining Failures Rerun
- Implement phase: `impl_05_packaging_container_and_remaining_failures`.
- Release gate pre-change evidence: `safe/scripts/run_release_gate.sh` failed in upstream pytest `original/test/test-suite/test_foreign.py::TestForeign::test_truncated`; `im.avg()` on `original/test/test-suite/images/truncated.jpg` raised `jpegload: failed to fill whole buffer; avg: operation failed`.
- Root cause checked: no validator package override or container installation defect was present. The remaining catch-all defect was libvips compatibility for default truncated JPEG materialization: header load succeeded, but lazy pixel decode returned an error even though default `fail_on` behavior should tolerate the truncated fixture; strict `fail_on=truncated` should still reject pixels.
- Changed production files: `safe/src/foreign/base.rs`, `safe/src/foreign/loaders/jpeg.rs`, and `safe/src/foreign/mod.rs`.
- Regression test path: `safe/tests/runtime_io.rs::truncated_jpeg_default_materializes_but_fail_on_truncated_rejects_pixels`.
- Focused test passed: `cargo test --test runtime_io truncated_jpeg_default_materializes_but_fail_on_truncated_rejects_pixels -- --nocapture`.
- Full Rust suite passed after isolating the operation-cache regression test from prior cache memory/file limit settings: `cargo test --all-features -- --nocapture`.
- Release gate passed after the fix: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && scripts/run_release_gate.sh`.
- Package-source commit used for the phase-5 rebuild: 7251b2d7efbc4adf60b0a98ce84c380fcaf1f415.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`.
- Refreshed local lock: validator/artifacts/libvips-safe-port-lock.json.
- Phase-5 artifact root: validator/artifacts/libvips-safe-remaining.
- Phase-5 matrix exit status: 0.
- Phase-5 summary artifact: validator/artifacts/libvips-safe-remaining/port-04-test/results/libvips/summary.json.
- Inventory-derived counts: source cases 5, usage cases 80, total cases 85.
- Phase-5 results: 85 passed, 0 failed, 85 cast records, and all result JSON records report `override_debs_installed: true`.
- Approved-skip artifacts: none.
- Remaining failure ownership after phase 5: no libvips validator testcase failures remain.

## failure classification
| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |
| --- | --- | --- | --- | --- | --- | --- | --- |
| __packaging_container_setup__ | packaging-container | fixed | `impl_05_packaging_container_and_remaining_failures` | pre-change `safe/scripts/run_release_gate.sh` upstream pytest `TestForeign::test_truncated` failure | Packaging/container override installation had no defect, but phase-5 release gate found the remaining catch-all default truncated JPEG materialization mismatch: native JPEG decode failed after header load when libvips default `fail_on` should tolerate truncated pixel decode. | `safe/tests/runtime_io.rs::truncated_jpeg_default_materializes_but_fail_on_truncated_rejects_pixels`. | Fixed by invalidating failed cached file loads and accepting ImageMagick fallback pixel decode only when it matches the JPEG header and `fail_on` is default; strict `fail_on=truncated` still fails. `safe/scripts/run_release_gate.sh` passed and validator/artifacts/libvips-safe-remaining/port-04-test/results/libvips/summary.json reports 85 passed, 0 failed, and all override packages installed. |
| vips-cli-load-save | source | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/vips-cli-load-save.json | JPEG load/save materialization fell back to external `convert` inside the validator container, which is absent; after copy succeeded, `vipsheader` also needed image object summaries to include dimensions. | `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert`, `safe/tests/runtime_io.rs::jpeg_public_save_paths_return_glib_owned_buffers_and_targets`, and `safe/tests/runtime_io.rs::image_object_summary_reports_dimensions_for_vipsheader`. | Fixed with native JPEG header/decode materialization, generated `VipsBlob`/`VipsArea` buffer wrapper ownership, GLib-owned returned buffers, and image summaries containing dimensions; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/vips-cli-load-save.json. |
| thumbnail-behavior | source | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/thumbnail-behavior.json | `vipsthumbnail` hit the same JPEG materialization path and failed when external `convert` was absent. | `safe/tests/runtime_io.rs::jpeg_thumbnail_materializes_and_saves_without_convert`. | Fixed by materializing JPEG pixels with the native Rust decoder before thumbnail/save paths need pixels; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/thumbnail-behavior.json. |
| usage-ruby-vips-crop-sample-jpeg | usage | fixed | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json | Ruby crop of a JPEG fixture failed during materialization through the missing external `convert` path before `extract_area` could complete. | `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert` covers JPEG fixture load/materialization through public file, buffer, source, and explicit loader APIs. | Fixed by replacing JPEG lazy materialization's external decoder fallback with native JPEG decode; passed in validator/artifacts/libvips-safe-foreign/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json. |
| usage-ruby-vips-gravity-generated | usage | fixed | `impl_03_ruby_usage_operation_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json | Generated-image `gravity` dispatch reached ruby-vips but the safe operation implementation returned `gravity: operation not implemented`. | `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre` covers the generated-image gravity centre crop through the exported C ABI; `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend` covers libvips `background`/`extend` vararg semantics; `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics` covers `VIPS_EXTEND_MIRROR`. | Fixed by implementing `gravity` support in `safe/src/ops/conversion.rs`, routing it from `safe/src/ops/mod.rs`, and sharing embed/gravity optional argument handling so `background` without `extend` uses `VIPS_EXTEND_BACKGROUND` and mirror extension uses libvips' `2 * size` tile period; passed in validator/artifacts/libvips-safe-ops/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json. |

## Later Owner Phases
- `impl_02_source_surface_failures`: no baseline failure is assigned here because the source-case failures depend on JPEG decode, save, or materialization rather than command, header, package identity, or metadata surface alone.
- `impl_03_ruby_usage_operation_failures`: generated ruby-vips operation behavior failure fixed in phase 3.
- `impl_04_foreign_io_buffer_failures`: file, buffer, loader, saver, lazy materialization, and external decoder fallback failures fixed in phase 4.
- `impl_05_packaging_container_and_remaining_failures`: release-gate catch-all truncated JPEG compatibility fixed; no packaging or container setup failure remains; all override packages installed and the phase-5 matrix exit was 0.

## Final Clean Run
- Implement phase: `impl_06_final_report_and_clean_run`.
- Validator URL: https://github.com/safelibs/validator.
- remote main commit: dc9f47b6054e9a51afde8a437a2e5e5562cc946a.
- active validator commit: 1319bb0374ef66428a42dd71e49553c6d057feaf.
- active validator reason: remote main manifest unusable for libvips; using last known runnable validator commit.
- Final package-source commit used for rebuild, override staging, lock refresh, and validator proof: 0b8b403eec9ad52e26e51b7f787e91917207c653. This phase's git commit is report-only and does not change `safe/**` package inputs.
- Inventory counts: 5 source cases, 80 usage cases, 85 total cases.
- checks executed: `cargo test --all-features -- --nocapture`, `safe/scripts/run_release_gate.sh`, `dpkg-buildpackage -b -uc -us`, canonical override staging, port lock refresh, final full unmodified validator matrix, clean-result assertion, `tools/verify_proof_artifacts.py --require-casts`, `tools/render_site.py`, `scripts/verify-site.sh`, and `git -C validator diff --exit-code -- tests repositories.yml README.md`.
- failures found: none in the final clean run. Baseline failures were `vips-cli-load-save`, `thumbnail-behavior`, `usage-ruby-vips-crop-sample-jpeg`, `usage-ruby-vips-gravity-generated`, plus the phase-5 release-gate truncated JPEG compatibility failure recorded in the failure classification table.
- fixes applied: none in `impl_06_final_report_and_clean_run`; earlier fixes remain the phase-3 `gravity` operation implementation, phase-4 native JPEG file/buffer/source materialization and CLI object summary compatibility, and phase-5 default truncated JPEG materialization compatibility while preserving strict `fail_on=truncated` rejection.
- Regression tests retained: `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre`, `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend`, `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics`, `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert`, `safe/tests/runtime_io.rs::jpeg_public_save_paths_return_glib_owned_buffers_and_targets`, `safe/tests/runtime_io.rs::jpeg_thumbnail_materializes_and_saves_without_convert`, `safe/tests/runtime_io.rs::image_object_summary_reports_dimensions_for_vipsheader`, and `safe/tests/runtime_io.rs::truncated_jpeg_default_materializes_but_fail_on_truncated_rejects_pixels`.
- Approved validator-bug skips: None.

## Final Commands Executed
```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT/safe"
cargo test --all-features -- --nocapture
scripts/run_release_gate.sh
dpkg-buildpackage -b -uc -us

cd "$ROOT"
mkdir -p validator-overrides/libvips
rm -f validator-overrides/libvips/*.deb
version=$(dpkg-parsechangelog -l safe/debian/changelog -SVersion)
arch=$(dpkg-architecture -qDEB_HOST_ARCH)
for package in libvips42t64 libvips-dev libvips-tools gir1.2-vips-8.0; do
  install -m 0644 "${package}_${version}_${arch}.deb" validator-overrides/libvips/
done
SAFE_SOURCE_COMMIT=$(git rev-parse HEAD)
LOCK="$ROOT/validator/artifacts/libvips-safe-port-lock.json"
python3 - "$ROOT" "$LOCK" "$SAFE_SOURCE_COMMIT" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
lock_path = Path(sys.argv[2])
commit = sys.argv[3].strip()
canonical = ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]
debs = []
for package in canonical:
    matches = sorted((root / "validator-overrides/libvips").glob(f"{package}_*.deb"))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one staged deb for {package}, found {len(matches)}")
    path = matches[0]
    package_name = subprocess.check_output(["dpkg-deb", "--field", str(path), "Package"], text=True).strip()
    architecture = subprocess.check_output(["dpkg-deb", "--field", str(path), "Architecture"], text=True).strip()
    if package_name != package:
        raise SystemExit(f"unexpected package name for {path}: {package_name}")
    if architecture not in {"amd64", "all"}:
        raise SystemExit(f"unexpected architecture for {path}: {architecture}")
    data = path.read_bytes()
    debs.append({
        "package": package,
        "filename": path.name,
        "architecture": architecture,
        "sha256": hashlib.sha256(data).hexdigest(),
        "size": path.stat().st_size,
    })
lock = {
    "schema_version": 1,
    "mode": "port-04-test",
    "generated_at": "1970-01-01T00:00:00Z",
    "source_config": "repositories.yml",
    "source_inventory": "local-validator-overrides",
    "libraries": [{
        "library": "libvips",
        "repository": "safelibs/port-libvips-local",
        "tag_ref": "refs/tags/libvips/local-validator",
        "commit": commit,
        "release_tag": f"build-{commit[:12]}",
        "debs": debs,
        "unported_original_packages": [],
    }],
}
lock_path.parent.mkdir(parents=True, exist_ok=True)
lock_path.write_text(json.dumps(lock, indent=2) + "\n")
PY

cd "$ROOT/validator"
rm -rf artifacts/libvips-safe-final site/libvips-safe-final
set +e
PYTHON=/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-final \
  --mode port-04-test \
  --library libvips \
  --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides \
  --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json \
  --record-casts
MATRIX_EXIT=$?
set -e
mkdir -p artifacts/libvips-safe-final
printf '%s\n' "$MATRIX_EXIT" > artifacts/libvips-safe-final/matrix-exit-code.txt
RESULT_ARTIFACT_NAME=libvips-safe-final EXPECTED_SOURCE_CASES=5 EXPECTED_USAGE_CASES=80 EXPECTED_TOTAL_CASES=85 python3 - <<'PY'
import json
import os
from pathlib import Path

root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
artifact = os.environ["RESULT_ARTIFACT_NAME"]
expected_source = int(os.environ["EXPECTED_SOURCE_CASES"])
expected_usage = int(os.environ["EXPECTED_USAGE_CASES"])
expected_total = int(os.environ["EXPECTED_TOTAL_CASES"])
artifact_root = root / "validator/artifacts" / artifact
exit_path = artifact_root / "matrix-exit-code.txt"
matrix_exit = int(exit_path.read_text().strip())
if matrix_exit != 0:
    raise SystemExit(f"validator matrix exited {matrix_exit} for {artifact}")
result_dir = artifact_root / "port-04-test/results/libvips"
summary = json.loads((result_dir / "summary.json").read_text())
if summary["source_cases"] != expected_source or summary["usage_cases"] != expected_usage or summary["cases"] != expected_total:
    raise SystemExit(f"summary counts do not match expected counts: {summary}")
if summary["failed"] != 0 or summary["passed"] != expected_total or summary["casts"] != expected_total:
    raise SystemExit(f"validator run is not clean: {summary}")
results = [path for path in sorted(result_dir.glob("*.json")) if path.name != "summary.json"]
if len(results) != expected_total:
    raise SystemExit(f"expected {expected_total} result JSON files, found {len(results)}")
bad = []
for path in results:
    payload = json.loads(path.read_text())
    testcase_id = payload.get("testcase_id", path.stem)
    if payload.get("status") != "passed":
        bad.append(f"{testcase_id}: status={payload.get('status')}")
    if payload.get("override_debs_installed") is not True:
        bad.append(f"{testcase_id}: override_debs_installed={payload.get('override_debs_installed')!r}")
    if payload.get("cast_path") is None:
        bad.append(f"{testcase_id}: missing cast")
if bad:
    raise SystemExit("clean validator assertion failed: " + "; ".join(bad))
PY
/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python tools/verify_proof_artifacts.py \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-final \
  --proof-output proof/libvips-safe-validation-proof.json \
  --mode port-04-test \
  --library libvips \
  --require-casts \
  --min-source-cases 5 \
  --min-usage-cases 80 \
  --min-cases 85
/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python tools/render_site.py \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-final \
  --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json \
  --output-root site/libvips-safe-final
bash scripts/verify-site.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifacts-root artifacts/libvips-safe-final \
  --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json \
  --site-root site/libvips-safe-final \
  --library libvips
git -C /home/yans/safelibs/pipeline/ports/port-libvips/validator diff --exit-code -- tests repositories.yml README.md
```

## Final Package Hashes
| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1430662 | ecda9f408ce52e33f3b65d20d844b821155af24c55973e13c3a51515bf3fd279 |
| libvips-dev | validator-overrides/libvips/libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98 |
| libvips-tools | validator-overrides/libvips/libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be |
| gir1.2-vips-8.0 | validator-overrides/libvips/gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | 362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82 |

## Final Evidence
- Final unmodified artifact root: validator/artifacts/libvips-safe-final.
- Matrix exit artifact: validator/artifacts/libvips-safe-final/matrix-exit-code.txt (`0`).
- Summary artifact: validator/artifacts/libvips-safe-final/port-04-test/results/libvips/summary.json.
- Final summary: 85 cases, 5 source cases, 80 usage cases, 85 passed, 0 failed, 85 casts, and all result JSON records have `override_debs_installed: true`.
- Proof path: validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json.
- Proof totals: 1 library, 85 cases, 5 source cases, 80 usage cases, 85 passed, 0 failed, 85 casts.
- Rendered site: validator/site/libvips-safe-final.
- Site files verified: validator/site/libvips-safe-final/index.html, validator/site/libvips-safe-final/library/libvips.html, validator/site/libvips-safe-final/site-data.json, validator/site/libvips-safe-final/assets/site.css, and validator/site/libvips-safe-final/assets/player.js.
- Approved-skip adjusted artifact root: N/A; no approved validator-bug skip exists.

## Phase 1 Baseline Run

Date: 2026-05-02. Phase: `impl_01_validator_baseline_run`. Repo HEAD: `3e6ad46e8bf603936ea074e3e2d102bd23132cbf` (`fix(libvips): drop duplicate op_gravity from merge`).

### Validator Pinning

- Pinned validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf` (already checked out before this phase; `git -C validator rev-parse HEAD` confirmed).
- Recorded `origin/main` after `git -C validator fetch origin`: `87b321fe728340d6fc6dd2f638583cca82c667c3` (`pages: trigger downstream apt, docker, website deploys after deploy`).
- Manifest divergence: `git -C validator show origin/main:tests/libvips/testcases.yml` shows the upstream manifest has been collapsed to apt-package metadata only (no `testcases:` key); the pinned commit still carries the 85 inline testcases. `grep -c '^testcases:'` against the upstream blob returns `0`. Decision: stayed at `1319bb0374ef66428a42dd71e49553c6d057feaf` to preserve the inline testcase suite this baseline run depends on.
- Pinned manifest sha256: `c44346195fbfa8dd5de2c29b14ac9474eb3b91802d4fc2e7a5325141f9ee6140  validator/tests/libvips/testcases.yml`.
- `validator/.venv/bin/python -c 'import yaml'` → PyYAML 6.0.3 (no reinstall needed).

### Sanity-Check Status (Pre-Baseline)

Both repository sanity checks fail in the current `safe/` state and are not blocking for the validator matrix because the .deb build is driven by `dh --buildsystem=meson` rather than the Rust `cargo` crate.

- `cd safe && cargo test --all-features -- --nocapture` → fails immediately. The pinned toolchain in `safe/rust-toolchain.toml` (`channel = "1.78"`) cannot resolve `indexmap@2.13.1` from `safe/Cargo.lock`, which requires `rustc >= 1.82`. Root cause: the Apr 30 → May 2 merge (`6124a7f`) brought in the upstream-template `safe/rust-toolchain.toml` while keeping the local `safe/Cargo.lock` carrying `indexmap 2.13.1`, and the local `safe/src/**` already requires Rust 1.82+ features (`unsafe extern "C" {}` blocks in `runtime/error.rs`, `runtime/object.rs`, `foreign/loaders/legacy_vips.rs`; `Option::is_none_or` in `ops/morphology.rs`). Out of scope for this phase: instructions explicitly forbid `safe/**` changes.
- `cd safe && scripts/run_release_gate.sh` → fails at `[release-gate] cargo` for the same `indexmap@2.13.1` requirement.
- Attempted workaround with `RUSTUP_TOOLCHAIN=1.82.0 cargo build --release`: source compiles, but linking the cdylib fails with `/usr/bin/ld: anonymous version tag cannot be combined with other version tags`. Rustc ≥1.81 emits its own anonymous `--version-script` for cdylibs; `safe/build.rs` separately injects `-Wl,--version-script=safe/generated/export-full.map` (a named `VIPS_42 { ... }` map). Modern `ld` (binutils 2.42 here) refuses the combination. Out of scope to fix in this phase.

### Built .deb Artifacts (Refreshed via dpkg-buildpackage)

Build path: `cd safe && dpkg-buildpackage -b -uc -us`. Despite the cargo failures above, `dh --buildsystem=meson` still produced the four canonical binary packages (debian/rules sets `RUSTUP_TOOLCHAIN ?= stable-x86_64-unknown-linux-gnu` and the meson driver linked successfully under that toolchain). Staged via `install -m 0644 …` into `validator-overrides/libvips/`.

| Package | Filename | Architecture | Size (bytes) | sha256 |
|---|---|---|---|---|
| libvips42t64 | libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1388394 | `b67dca304501cfabc5131c7e7f40888386078110c3115dee8dbdfe59d3e19e2c` |
| libvips-dev | libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | `baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98` |
| libvips-tools | libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | `c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be` |
| gir1.2-vips-8.0 | gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | `362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82` |

Three of four .deb files are byte-identical to the prior `validator/artifacts/libvips-safe-port-lock.json` entries (libvips-dev, libvips-tools, gir1.2-vips-8.0). `libvips42t64` rebuilt deterministically against the current `safe/` source tree, producing a different sha256 (prior `ecda9f408ce5…`, new `b67dca304501…`) and a smaller package (prior 1430662 bytes → new 1388394 bytes). `validator-overrides/` is gitignored (`.gitignore` line 10) and `*.deb` is gitignored (line 31), so no `.deb` files are tracked in git history; the refresh is recorded by sha256 in this report and in the port-deb-lock JSON instead.

### Port-Deb-Lock JSON

Path: `validator/artifacts/libvips-safe-baseline-port-lock.json` (schema_version 1, mode `port-04-test`, source_inventory `local-validator-overrides`, commit `3e6ad46e8bf603936ea074e3e2d102bd23132cbf`, release_tag `build-3e6ad46e8bf6`). Each `debs[].sha256` matches the on-disk file shown in the table above. Canonical order: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`. `unported_original_packages: []`.

### Matrix Invocation

```
PYTHON=validator/.venv/bin/python bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-baseline \
  --mode port-04-test \
  --library libvips \
  --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides \
  --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-baseline-port-lock.json \
  --record-casts
```

Matrix exit code: `0` (recorded at `validator/artifacts/libvips-safe-baseline/matrix-exit-code.txt`).

### Result Summary

`validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json`:

- schema_version: 2
- library: libvips
- mode: port-04-test
- cases: 85
- source_cases: 5
- usage_cases: 80
- passed: 85
- failed: 0
- casts: 85
- duration_seconds: 0.0

Casts captured under `validator/artifacts/libvips-safe-baseline/port-04-test/casts/libvips/` (85 `.cast` files, one per testcase).

### Failure Classification

Failure classification: none — baseline is clean.

All 85 cases passed. There are no failures to route to `impl_02_source_surface_failures`, `impl_03_ruby_usage_operation_failures`, `impl_04_foreign_io_buffer_failures`, or `impl_05_packaging_container_and_remaining_failures`.

## Phase 2 Source Surface Rerun (Baseline-Clean No-Op)

Date: 2026-05-02. Phase: `impl_02_source_surface_failures`. Repo HEAD before this phase: `4b4faf8` (`impl_01 record validator baseline run`). No failure of this class exists in the Phase 1 baseline: `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json` reports `cases: 85`, `source_cases: 5`, `usage_cases: 80`, `passed: 85`, `failed: 0`, `casts: 85`, and the Phase 1 failure classification table records "none — baseline is clean", explicitly stating that no failures are routed to `impl_02_source_surface_failures`. Per the phase contract, when Phase 1 owns zero failures for this phase the implementer skips the build/stage/lock/rerun, no `safe/**` source or test changes are made, no `safe/tests/{abi_layout,init_version_smoke,runtime_io,operation_registry}.rs` regression test is added, no `validator-overrides/libvips/*.deb` rebuild is performed, no `validator/artifacts/libvips-safe-source-port-lock.json` is synthesized, no focused `validator/artifacts/libvips-safe-source/` rerun is produced, and no approved-skip `validator/artifacts/libvips-safe-source-approved/` rerun is needed (no validator-bug verdict applies). The validator working tree is unchanged: `git -C validator diff --exit-code -- tests repositories.yml README.md` is expected to remain clean. The remaining-ownership row for this phase therefore stays "no failure remains owned by `impl_02_source_surface_failures`", consistent with the prior session's Phase 2, Phase 3, Phase 4, Phase 5, and Final Clean Run sections above. Commit recorded as `impl_02 record no source surface failures`.

## Phase 3 Ruby Usage Operation Rerun (Baseline-Clean No-Op)

Date: 2026-05-02. Phase: `impl_03_ruby_usage_operation_failures`. Repo HEAD before this phase: `8017bc5` (`impl_02 record no source surface failures`). No failure of this class exists in the Phase 1 baseline: `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json` reports `cases: 85`, `source_cases: 5`, `usage_cases: 80`, `passed: 85`, `failed: 0`, `casts: 85`, and the Phase 1 failure classification table records "none — baseline is clean", explicitly stating that no failures are routed to `impl_03_ruby_usage_operation_failures`. The prior session's `## Phase 3 Ruby Usage Operation Rerun` section above (alongside the failure-classification row for `usage-ruby-vips-gravity-generated`) confirms the original gravity-class fix has already landed in `safe/src/ops/conversion.rs` and `safe/src/ops/mod.rs`, and is exercised by the regression tests `safe/tests/ops_advanced.rs::gravity_crops_generated_image_from_centre`, `safe/tests/ops_advanced.rs::gravity_background_without_extend_uses_background_extend`, and `safe/tests/ops_advanced.rs::gravity_mirror_extend_matches_libvips_tile_semantics`; this current Phase 1 baseline already passes that ruby-vips gravity testcase, so no new ruby-vips/php-vips/govips/lua-vips/sharp/bimg/imgproxy/carrierwave-vips/sharp-for-go usage-class operation-not-implemented or operation-semantics failure is owned here. Per the phase contract, when Phase 1 owns zero failures for this phase the implementer skips the build/stage/lock/rerun, no `safe/src/ops/**` or `safe/src/runtime/**` changes are made, no `safe/tests/{ops_core,ops_advanced,operation_registry,security}.rs` regression test is added (existing gravity coverage is retained), no `validator-overrides/libvips/*.deb` rebuild is performed, no `validator/artifacts/libvips-safe-ops-port-lock.json` is synthesized, no focused `validator/artifacts/libvips-safe-ops/` rerun is produced, and no approved-skip `validator/artifacts/libvips-safe-ops-approved/` rerun is needed (no validator-bug verdict applies). The validator working tree is unchanged: `git -C validator diff --exit-code -- tests repositories.yml README.md` is expected to remain clean. No duplicate registration of `gravity` is introduced — it remains the single dispatch entry in `safe/src/ops/mod.rs` and the single `op_gravity` body in `safe/src/ops/conversion.rs`. The remaining-ownership row for this phase therefore stays "no failure remains owned by `impl_03_ruby_usage_operation_failures`", consistent with the prior session's Phase 3, Phase 4, Phase 5, and Final Clean Run sections above. Commit recorded as `impl_03 record no ruby usage operation failures`.

## Phase 4 Foreign I/O And Buffer Rerun (Baseline-Clean No-Op)

Date: 2026-05-02. Phase: `impl_04_foreign_io_buffer_failures`. Repo HEAD before this phase: `712659e` (`impl_03 record no ruby usage operation failures`). No failure of this class exists in the Phase 1 baseline: `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json` reports `cases: 85`, `source_cases: 5`, `usage_cases: 80`, `passed: 85`, `failed: 0`, `casts: 85`, and the Phase 1 failure classification table records "none — baseline is clean", explicitly stating that no failures are routed to `impl_04_foreign_io_buffer_failures`. The prior session's `## Phase 4 Foreign I/O And Buffer Rerun` section above (alongside the failure-classification rows for `vips-cli-load-save`, `thumbnail-behavior`, and `usage-ruby-vips-crop-sample-jpeg`) confirms the original native JPEG file/buffer/source materialization, GLib-owned `VipsBlob`/`VipsArea` save-buffer ownership, and `vipsheader`-compatible image object summary fixes have already landed in `safe/src/foreign/loaders/jpeg.rs`, `safe/src/foreign/loaders/mod.rs`, `safe/src/foreign/mod.rs`, `safe/build.rs`, `safe/src/runtime/image.rs`, `safe/src/runtime/object.rs`, `safe/Cargo.toml`, and `safe/Cargo.lock`, and are exercised by the regression tests `safe/tests/runtime_io.rs::jpeg_file_buffer_source_and_explicit_load_materialize_without_convert`, `safe/tests/runtime_io.rs::jpeg_public_save_paths_return_glib_owned_buffers_and_targets`, `safe/tests/runtime_io.rs::jpeg_thumbnail_materializes_and_saves_without_convert`, and `safe/tests/runtime_io.rs::image_object_summary_reports_dimensions_for_vipsheader`; this current Phase 1 baseline already passes those JPEG/PNG/WebP/TIFF foreign-loader, foreign-saver, thumbnail-materialization, and buffer/area ownership testcases (`validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/vips-cli-load-save.json`, `thumbnail-behavior.json`, and the `usage-ruby-vips-crop-sample-jpeg.json` record all carry `status: passed`), so no new foreign load/save/thumbnail materialization or `VipsBlob`/`VipsArea` ownership failure is owned here. Per the phase contract, when Phase 1 owns zero failures for this phase the implementer skips the build/stage/lock/rerun, no `safe/src/foreign/**`, `safe/src/runtime/**`, `safe/build.rs`, or `safe/Cargo.toml`/`safe/Cargo.lock` changes are made, no new decoder dependency is added (`jpeg-decoder = "0.3"` already present in `safe/Cargo.toml` is not duplicated), no new `Command::new("convert")` invocation is introduced in `safe/src/foreign/**`, no `safe/tests/runtime_io.rs` or `safe/tests/security.rs` regression test is added (existing JPEG file/buffer/source/save/thumbnail/object-summary coverage is retained), no upstream meson shell or fuzz suite rerun is performed (no `meson setup build-validator-foreign . --wipe --prefix "$PWD/.tmp/validator-foreign-prefix"`, no `meson compile -C build-validator-foreign`, and no `tests/upstream/run-shell-suite.sh build-validator-foreign` or `tests/upstream/run-fuzz-suite.sh build-validator-foreign` execution), no `validator-overrides/libvips/*.deb` rebuild is performed, no `validator/artifacts/libvips-safe-foreign-port-lock.json` is synthesized, no focused `validator/artifacts/libvips-safe-foreign/` rerun is produced, and no approved-skip `validator/artifacts/libvips-safe-foreign-approved/` rerun is needed (no validator-bug verdict applies, so no transient `--tests-root` override copy of `validator/tests/libvips/tests/cases/` with a failing per-case script removed is created). The validator working tree is unchanged: `git -C validator diff --exit-code -- tests repositories.yml README.md` is expected to remain clean. The remaining-ownership row for this phase therefore stays "no failure remains owned by `impl_04_foreign_io_buffer_failures`", consistent with the prior session's Phase 4, Phase 5, and Final Clean Run sections above. Commit recorded as `impl_04 record no foreign io and buffer failures`.

## Phase 5 Packaging, Container, And Remaining Failures Rerun (Baseline-Clean No-Op)

Date: 2026-05-02. Phase: `impl_05_packaging_container_and_remaining_failures`. Repo HEAD before this phase: `431c583` (`impl_04 record no foreign io and buffer failures`). No failure of this class exists in the Phase 1 baseline: `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json` reports `cases: 85`, `source_cases: 5`, `usage_cases: 80`, `passed: 85`, `failed: 0`, `casts: 85`; programmatically inspecting all 85 per-case JSONs under `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/*.json` (excluding `summary.json`) confirms every record has `status == "passed"` and `override_debs_installed == true`, so there is no override-deb container install failure, no canonical package mismatch, no `dpkg -i` dependency-mismatch failure, and no per-case packaging/container setup failure to route to this phase. The Phase 1 failure classification table records "none — baseline is clean", explicitly stating that no failures are routed to `impl_05_packaging_container_and_remaining_failures`. The prior session's `## Phase 5 Packaging, Container, And Remaining Failures Rerun` section above (alongside the failure-classification row for `__packaging_container_setup__`) confirms the original release-gate catch-all default truncated JPEG materialization fix has already landed in `safe/src/foreign/base.rs`, `safe/src/foreign/loaders/jpeg.rs`, and `safe/src/foreign/mod.rs`, and is exercised by the regression test `safe/tests/runtime_io.rs::truncated_jpeg_default_materializes_but_fail_on_truncated_rejects_pixels`; the canonical override-package set (`libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`) staged under `validator-overrides/libvips/` already installed cleanly in the Phase 1 baseline (each per-case JSON carries `override_debs_installed: true`). Per the phase contract, when Phase 1 owns zero failures for this phase the implementer skips the build/stage/lock/rerun, no `safe/src/**`, `safe/build.rs`, `safe/meson.build`, `safe/debian/**`, or `safe/Cargo.toml`/`safe/Cargo.lock` changes are made (no `safe/debian/control`, `safe/debian/rules`, or `safe/debian/*.install` package-metadata edits, and no `packaging/package.env` `DEB_*` field changes), no pre-change `safe/scripts/run_release_gate.sh` evidence run is performed (the prior session's evidence and fix already cover the canonical truncated-JPEG case `original/test/test-suite/test_foreign.py::TestForeign::test_truncated`), no new `safe/tests/runtime_io.rs` or other `safe/tests/<area>.rs` regression test is added (existing truncated-JPEG, JPEG file/buffer/source/save/thumbnail/object-summary, and gravity coverage is retained), no `cd safe && cargo test --all-features -- --nocapture` rerun and no `safe/scripts/run_release_gate.sh` post-change rerun is performed, no `validator-overrides/libvips/*.deb` rebuild is performed (the four canonical `.deb` files staged for the Phase 1 baseline remain in place and are not re-staged), no `validator/artifacts/libvips-safe-remaining-port-lock.json` is synthesized, no focused `validator/artifacts/libvips-safe-remaining/` rerun is produced (the prior session's Phase 5 rerun under that path is preserved unchanged and not re-executed), and no approved-skip `validator/artifacts/libvips-safe-remaining-approved/` rerun is needed (no validator-bug verdict applies, so no transient `--tests-root` override copy of `validator/tests/libvips/tests/cases/` with a failing per-case script removed is created and no `excluded testcase id` is recorded here). The validator working tree is unchanged: `git -C validator diff --exit-code -- tests repositories.yml README.md` exits zero. All 85 baseline result JSON records explicitly assert `override_debs_installed: true`, satisfying the phase-5 explicit override-debs-installed assertion across all 85 records. The remaining-ownership row for this phase therefore stays "no failure remains owned by `impl_05_packaging_container_and_remaining_failures`", consistent with the prior session's Phase 5 and Final Clean Run sections above. Commit recorded as `impl_05 record no remaining failures`.

## Phase 1 Baseline Run

Date: 2026-05-03. Phase: `impl_01_validator_baseline_run`. Repo HEAD before this phase: `1cfba4096f9a313d52f230ceaee2fef032c4612c` (`impl_05 record no remaining failures`).

### Validator Pinning

- Pinned validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf` (already checked out before this phase; `git -C validator rev-parse HEAD` confirmed equals the pin).
- Recorded `origin/main` after `git -C validator fetch origin`: `87b321fe728340d6fc6dd2f638583cca82c667c3`.
- `git -C validator status --porcelain` shows only untracked entries (`.venv/` and the locally generated `artifacts/**` paths); no tracked-file divergence from the pinned commit.
- Manifest divergence: `git -C validator show origin/main:tests/libvips/testcases.yml | grep -c '^testcases:'` returns `0` — the upstream manifest at `origin/main` has been collapsed to apt-package metadata only (`schema_version`, `library`, `apt_packages`) and no longer carries the inline `testcases:` block. The pinned commit still carries the 85 inline testcases (`sha256(validator/tests/libvips/testcases.yml) = c44346195fbfa8dd5de2c29b14ac9474eb3b91802d4fc2e7a5325141f9ee6140`). Decision: stayed at `1319bb0374ef66428a42dd71e49553c6d057feaf` to preserve the inline testcase suite this baseline run depends on.
- `validator/.venv/bin/python -c 'import yaml'` → PyYAML 6.0.3 (no reinstall needed).

### Sanity-Check Status (Pre-Baseline)

Both repository sanity checks fail in the current `safe/` state with the same root cause recorded in the prior session's Phase 1 section, and remain not-blocking for the validator matrix because the `.deb` build is driven by `dh --buildsystem=meson` rather than the Rust `cargo` crate.

- `cd safe && cargo test --all-features -- --nocapture` → fails immediately. The pinned toolchain in `safe/rust-toolchain.toml` (`channel = "1.78"`) cannot resolve `indexmap@2.13.1` from `safe/Cargo.lock`, which requires `rustc >= 1.82`. Out of scope for this phase: instructions explicitly forbid `safe/**` changes.
- `cd safe && scripts/run_release_gate.sh` → fails at `[release-gate] cargo` for the same `indexmap@2.13.1` requirement. Out of scope to fix in this phase.

### Built .deb Artifacts (Refreshed via dpkg-buildpackage)

Build path: `cd safe && dpkg-buildpackage -b -uc -us`. Despite the cargo failures above, `dh --buildsystem=meson` still produced the four canonical binary packages. Staged via `install -m 0644 …` into `validator-overrides/libvips/`.

| Package | Filename | Architecture | Size (bytes) | sha256 |
|---|---|---|---|---|
| libvips42t64 | libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1388394 | `b67dca304501cfabc5131c7e7f40888386078110c3115dee8dbdfe59d3e19e2c` |
| libvips-dev | libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | `baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98` |
| libvips-tools | libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | `c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be` |
| gir1.2-vips-8.0 | gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | `362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82` |

All four .deb files are byte-identical to the prior session's Phase 1 build (same sha256 for each canonical package), confirming the deterministic rebuild against the unchanged `safe/` source tree at HEAD `1cfba4096f9a313d52f230ceaee2fef032c4612c`. `validator-overrides/` is gitignored (`.gitignore` line 10) and `*.deb` is gitignored (line 31), so no `.deb` files are tracked in git history; the refresh is recorded by sha256 in this report and in `validator/artifacts/libvips-safe-baseline-port-lock.json` instead.

### Port-Deb-Lock JSON

Path: `validator/artifacts/libvips-safe-baseline-port-lock.json` (schema_version 1, mode `port-04-test`, source_inventory `local-validator-overrides`, commit `1cfba4096f9a313d52f230ceaee2fef032c4612c`, release_tag `build-1cfba4096f9a`, generated_at `2026-05-03T19:56:07Z`). Each `debs[].sha256` matches the on-disk file shown in the table above (programmatic verification: all four `sha256_ok=True size_ok=True`). Canonical order: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`. `unported_original_packages: []`.

### Matrix Invocation

```
PYTHON=validator/.venv/bin/python bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-baseline \
  --mode port-04-test \
  --library libvips \
  --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides \
  --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-baseline-port-lock.json \
  --record-casts
```

Matrix exit code: `0` (recorded at `validator/artifacts/libvips-safe-baseline/matrix-exit-code.txt`).

### Result Summary

`validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/summary.json`:

- schema_version: 2
- library: libvips
- mode: port-04-test
- cases: 85
- source_cases: 5
- usage_cases: 80
- passed: 85
- failed: 0
- casts: 85
- duration_seconds: 0.0

Casts captured under `validator/artifacts/libvips-safe-baseline/port-04-test/casts/libvips/` (85 `.cast` files, one per testcase). Programmatic inspection of all 85 per-case JSONs under `validator/artifacts/libvips-safe-baseline/port-04-test/results/libvips/*.json` (excluding `summary.json`) confirms `passed=85 failed=0 override_true=85` (every record has `status == "passed"` and `override_debs_installed == true`).

### Failure Classification

Failure classification: none — baseline is clean.

All 85 cases passed. There are no failures to route to `impl_02_source_surface_failures`, `impl_03_ruby_usage_operation_failures`, `impl_04_foreign_io_buffer_failures`, or `impl_05_packaging_container_and_remaining_failures`.
