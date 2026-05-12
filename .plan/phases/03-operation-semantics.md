# Phase 3: Operation Semantics Failures

## Phase Name
Fix ruby-vips operation dispatch, argument handling, and pixel semantics failures

## Implement Phase ID
`impl_03_operation_semantics_failures`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator-report.md`
- `validator-overrides/libvips/*.deb`
- `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
- `validator/artifacts/libvips-safe-source-api/`
- `validator/artifacts/libvips-safe-source-api-port-lock.json`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/lib/build_port_lock.py`
- `packaging/package.env`
- `safe/src/ops/mod.rs`
- `safe/src/ops/arithmetic.rs`
- `safe/src/ops/conversion.rs`
- `safe/src/ops/colour.rs`
- `safe/src/ops/convolution.rs`
- `safe/src/ops/create.rs`
- `safe/src/ops/draw.rs`
- `safe/src/ops/freqfilt.rs`
- `safe/src/ops/histogram.rs`
- `safe/src/ops/morphology.rs`
- `safe/src/ops/mosaicing.rs`
- `safe/src/ops/resample.rs`
- `safe/src/runtime/operation.rs`
- `safe/src/generated/operations_registry.rs`
- `safe/src/generated/operation_wrappers.rs`
- `safe/reference/operations.json`
- `safe/src/pixels/**`
- `safe/tests/ops_core.rs`
- `safe/tests/ops_advanced.rs`
- `safe/tests/operation_registry.rs`
- `safe/tests/security.rs`
- `original/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`

## New Outputs
- Regression tests in `safe/tests/ops_core.rs` or `safe/tests/ops_advanced.rs`, or a report-only zero-owned-failure decision.
- Operation implementation fixes in `safe/src/ops/**`, with helper changes in `safe/src/pixels/**` or `safe/src/runtime/operation.rs` only when required.
- Fresh package overrides and full canonical `validator/artifacts/libvips-safe-ops-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-ops/` and `matrix-exit-code.txt`.
- Exactly one active `## Phase 3 Operation Semantics Rerun` section.
- Source commit `impl_03 fix operation validator failures` when fixes are needed, followed by report commit `impl_03 record operation validator rerun`; if zero owned failures, only `impl_03 record no operation failures`.

## File Changes
- Candidate files: `safe/src/ops/**`, `safe/src/pixels/**`, `safe/src/runtime/operation.rs`, `safe/tests/ops_core.rs`, `safe/tests/ops_advanced.rs`, and `safe/tests/operation_registry.rs`.
- Avoid editing generated files directly unless the established local generator path is used and documented.
- Do not modify validator tests or tracked validator source.

## Implementation Details
- Record `PHASE_START_COMMIT=$(git rev-parse HEAD)` at phase start. If the baseline assigns zero failures to `impl_03_operation_semantics_failures`, do not edit `safe/**`; run focused operation tests, set `SOURCE_COMMIT=$PHASE_START_COMMIT` and `SOURCE_FIX_COMMITS=none`, then produce official phase-3 evidence and a report-only commit.
- For each owned failure, identify the libvips operation nickname from validator logs, ruby-vips stack traces, or generated wrapper names. If the issue is `operation not implemented`, add it to `SUPPORTED_OPERATIONS` only after implementing the real path.
- Add a minimal C ABI style regression test before production changes. Declare the relevant exported wrapper, construct small in-memory images with existing helpers or `vips_image_new_from_memory_copy`, and assert return code, output pointer behavior, dimensions, bands, format, interpretation, offsets/demand hints, metadata, and pixel values.
- Implement using existing helpers in `safe/src/ops/mod.rs` (`get_image_buffer`, `get_int`, `get_double`, `get_enum`, `get_bool`, `get_array_double`, `set_output_image`, `set_output_image_like`, and metadata-copy helpers). Preserve libvips conventions for null outputs on failure, useful operation-domain error messages, varargs/defaults from generated metadata, metadata/history propagation, format promotion, band handling, and checked/saturating/clamped overflow where appropriate.
- Run focused Rust tests, then commit source/test fixes as `impl_03 fix operation validator failures`, excluding report and generated evidence.
- Set `SOURCE_COMMIT=$(git rev-parse HEAD)` and `SOURCE_FIX_COMMITS=$(git log --format=%H --reverse "$PHASE_START_COMMIT"..HEAD -- safe scripts packaging tests | xargs)`. For nonzero fixes, require this list to be non-empty.
- Run the official phase-3 package/lock/validator command block below. This block is also used for the zero-owned-failure path and must not be run until `SOURCE_COMMIT` names committed source/test/package content:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
: "${SOURCE_COMMIT:?set SOURCE_COMMIT before phase-3 evidence}"
OPS_LOCK="$ROOT/validator/artifacts/libvips-safe-ops-port-lock.json"
OPS_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-ops"

cd "$ROOT"
bash scripts/check-layout.sh
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh

rm -rf "$ROOT/validator-overrides/libvips"
mkdir -p "$ROOT/validator-overrides"
SAFELIBS_LIBRARY=libvips \
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
SAFELIBS_DIST_DIR="$ROOT/dist" \
SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
SAFELIBS_LOCK_PATH="$OPS_LOCK" \
SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
  python3 "$ROOT/scripts/lib/build_port_lock.py"
python3 -m json.tool "$OPS_LOCK" >/dev/null
FULL_PORT_LOCK="$OPS_LOCK" OVERRIDE_DIR="$ROOT/validator-overrides/libvips" \
  "$ROOT/validator/.venv/bin/python" - <<'PY'
import hashlib
import json
import os
from pathlib import Path

canonical = ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]
lock = json.loads(Path(os.environ["FULL_PORT_LOCK"]).read_text())
library = lock["libraries"][0]
packages = [deb["package"] for deb in library["debs"]]
if packages != canonical:
    raise SystemExit(f"official libvips lock is incomplete or out of order: {packages!r}")
if library.get("unported_original_packages") != []:
    raise SystemExit(f"official libvips lock must not use original packages: {library.get('unported_original_packages')!r}")
override_dir = Path(os.environ["OVERRIDE_DIR"])
for deb in library["debs"]:
    path = override_dir / deb["filename"]
    if not path.is_file():
        raise SystemExit(f"missing override deb: {path}")
    if path.stat().st_size != deb["size"]:
        raise SystemExit(f"size mismatch for {path}")
    if hashlib.sha256(path.read_bytes()).hexdigest() != deb["sha256"]:
        raise SystemExit(f"sha256 mismatch for {path}")
PY

cd "$ROOT/validator"
rm -rf "$OPS_ARTIFACT"
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$OPS_ARTIFACT" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$OPS_LOCK" \
  --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > "$OPS_ARTIFACT/matrix-exit-code.txt"

PHASE_ARTIFACT="$OPS_ARTIFACT" PHASE_LOCK="$OPS_LOCK" SOURCE_COMMIT="$SOURCE_COMMIT" \
  "$ROOT/validator/.venv/bin/python" - <<'PY'
import json
import os
from pathlib import Path

artifact = Path(os.environ["PHASE_ARTIFACT"])
lock_path = Path(os.environ["PHASE_LOCK"])
source_commit = os.environ["SOURCE_COMMIT"]
summary_path = artifact / "port/results/libvips/summary.json"
summary = json.loads(summary_path.read_text())
assert summary["cases"] == summary["source_cases"] + summary["usage_cases"]
assert summary["source_cases"] >= 5 and summary["usage_cases"] >= 170 and summary["cases"] >= 175
lock = json.loads(lock_path.read_text())
library = lock["libraries"][0]
canonical = ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]
assert library["commit"] == source_commit
assert library["tag_ref"] == f"refs/tags/build-{source_commit[:12]}"
assert library["release_tag"] == f"build-{source_commit[:12]}"
assert [deb["package"] for deb in library["debs"]] == canonical
assert library.get("unported_original_packages") == []
result_files = [p for p in (artifact / "port/results/libvips").glob("*.json") if p.name != "summary.json"]
assert len(result_files) == summary["cases"]
for path in result_files:
    result = json.loads(path.read_text())
    assert [deb["package"] for deb in result.get("port_debs", [])] == canonical, path
    assert result.get("unported_original_packages") == [], path
    assert result.get("override_debs_installed") is True, path
PY
```
- Replace the unique active Phase 3 report section with `Phase start commit`, `Source commit`, `Source fix commits`, fixed operation names, regression tests, changed files, package hashes, matrix exit path, matrix summary, and remaining ownership.
- Restore any build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` before the report commit.

## Verification Phases
### `check_03_operation_semantics_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_03_operation_semantics_failures`
- Purpose: Verify phase-3 owned operation failures are fixed, Rust operation tests pass, and a full validator rerun exists.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-ops-port-lock.json`
  - `validator/artifacts/libvips-safe-ops/`
  - `validator/artifacts/libvips-safe-ops/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-ops/matrix-exit-code.txt`
  - `safe/src/ops/**`
  - `safe/src/pixels/**`
  - `safe/tests/ops_core.rs`
  - `safe/tests/ops_advanced.rs`
  - `safe/tests/operation_registry.rs`
  - `safe/tests/security.rs`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && cargo test --all-features --test ops_core --test ops_advanced --test operation_registry --test security -- --nocapture`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - `python3 -m json.tool validator/artifacts/libvips-safe-ops-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json >/dev/null`
  - `cat validator/artifacts/libvips-safe-ops/matrix-exit-code.txt`
  - run a Python assertion that parses only the unique Phase 1 and Phase 3 sections, validates the phase-3 lock and override debs, requires canonical packages ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"] and `unported_original_packages == []`, validates every result JSON, and requires every baseline `impl_03_operation_semantics_failures` testcase to pass unless Phase 3 records zero owned failures.

### `check_03_operation_semantics_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_03_operation_semantics_failures`
- Purpose: Review operation fixes for libvips semantic compatibility, metadata propagation, ownership, and edge cases.
- Required preexisting inputs:
  - `validator/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-ops-port-lock.json`
  - `validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json`
  - `safe/src/ops/**`
  - `safe/src/generated/**`
  - `safe/src/pixels/**`
  - `safe/src/runtime/operation.rs`
  - `safe/tests/ops_core.rs`
  - `safe/tests/ops_advanced.rs`
  - `safe/tests/operation_registry.rs`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - assert exactly one exact `## Phase 3 Operation Semantics Rerun` heading; parse and validate `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from that bounded section.
  - `git log --oneline "$PHASE_START"..HEAD`
  - `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/src/ops safe/src/pixels safe/src/runtime/operation.rs safe/src/generated safe/tests/ops_core.rs safe/tests/ops_advanced.rs safe/tests/operation_registry.rs`
  - if `SOURCE_FIX_COMMITS` is not `none`, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/src/ops safe/src/pixels safe/src/runtime/operation.rs safe/src/generated safe/tests/ops_core.rs safe/tests/ops_advanced.rs safe/tests/operation_registry.rs`.
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - inspect changed operation/pixel code and tests; confirm tests use exported C ABI or the same wrapper path used by ruby-vips.

## Success Criteria
- Every baseline testcase owned by Phase 3 passes in `validator/artifacts/libvips-safe-ops/`, unless the active Phase 3 section records zero owned failures.
- New operation behavior is covered by regression tests that exercise exported C ABI or the same wrapper path used by ruby-vips.
- Official phase-3 lock and every result JSON use the full canonical libvips package set with no original-package fallback.
- Validator tracked files remain clean.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
