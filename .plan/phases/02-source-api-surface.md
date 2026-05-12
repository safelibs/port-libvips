# Phase 2: Source API Surface Failures

## Phase Name
Fix source-facing ABI, headers, metadata, pkg-config, and introspection failures

## Implement Phase ID
`impl_02_source_api_surface_failures`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator-report.md`
- `validator-overrides/libvips/*.deb`
- `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
- `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/lib/build_port_lock.py`
- `packaging/package.env`
- `safe/include/vips/**`
- `safe/src/abi/**`
- `safe/src/runtime/init.rs`
- `safe/src/runtime/type.rs`
- `safe/src/runtime/object.rs`
- `safe/src/runtime/image.rs`
- `safe/src/runtime/operation.rs`
- `safe/src/runtime/header.rs`
- `safe/src/runtime/error.rs`
- `safe/src/runtime/buf.rs`
- `safe/src/runtime/sbuf.rs`
- `safe/build.rs`
- `safe/meson.build`
- `safe/build_support/vips.pc.in`
- `safe/build_support/vips-cpp.pc.in`
- `safe/debian/**`
- `safe/tests/abi_layout.rs`
- `safe/tests/init_version_smoke.rs`
- `safe/tests/operation_registry.rs`
- `safe/tests/runtime_io.rs`
- `safe/tests/introspection/gir_smoke.c`
- `original/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`

## New Outputs
- Minimal regression tests for each Phase 2-owned failure, or a report-only zero-owned-failure decision.
- Source/API fixes in `safe/**` when required.
- Fresh `dist/*.deb`, rewritten `validator-overrides/libvips/*.deb`, and full canonical `validator/artifacts/libvips-safe-source-api-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-source-api/` and `matrix-exit-code.txt`.
- Exactly one active `## Phase 2 Source API Surface Rerun` section.
- Source commit `impl_02 fix source api validator failures` when fixes are needed, followed by report commit `impl_02 record source api validator rerun`; if zero owned failures, only `impl_02 record no source api failures`.

## File Changes
- Candidate files: `safe/src/abi/**`, `safe/src/runtime/**`, `safe/include/vips/**`, `safe/build.rs`, `safe/meson.build`, `safe/build_support/*.pc.in`, `safe/debian/**`, `safe/tests/abi_layout.rs`, `safe/tests/init_version_smoke.rs`, `safe/tests/operation_registry.rs`, `safe/tests/runtime_io.rs`, and `safe/tests/introspection/gir_smoke.c`.
- Do not modify `validator/tests/**` or tracked validator source.

## Implementation Details
- Record `PHASE_START_COMMIT=$(git rev-parse HEAD)` at phase start. Later senior review must use this range, not `git show HEAD`.
- If the active baseline section assigns zero failures to `impl_02_source_api_surface_failures`, do not edit `safe/**`; run focused source-surface tests, set `SOURCE_COMMIT=$PHASE_START_COMMIT` and `SOURCE_FIX_COMMITS=none`, then produce package/lock/validator evidence and a report-only commit.
- For each owned failure, reproduce from the baseline log and source testcase path. Add a focused regression test before production changes: ABI layout in `abi_layout.rs`, init/version/type registry in `init_version_smoke.rs`, operation metadata in `operation_registry.rs`, image/header metadata in `runtime_io.rs`, and GIR/typelib in `safe/tests/introspection/gir_smoke.c` or an equivalent existing harness.
- Fix the underlying public-surface issue by matching `original/` and `safe/reference/**`: header declarations in `safe/include/vips/**`; ABI layout in `safe/src/abi/**`; type/object/operation metadata in `safe/src/runtime/**`; image/header behavior in `safe/src/runtime/image.rs` or `header.rs`; install, pkg-config, and GIR issues in `safe/meson.build`, `safe/build_support/*.pc.in`, or `safe/debian/**`.
- Run focused tests, then commit source/test/package changes before official evidence as `impl_02 fix source api validator failures`. The source commit must exclude `validator-report.md`, `validator/**`, `validator-overrides/**`, `dist/**`, and generated build artifacts.
- Set `SOURCE_COMMIT=$(git rev-parse HEAD)` and `SOURCE_FIX_COMMITS=$(git log --format=%H --reverse "$PHASE_START_COMMIT"..HEAD -- safe scripts packaging tests | xargs)`. For nonzero fixes, require this list to be non-empty.
- Run the official phase-2 package/lock/validator command block below. This block is also used for the zero-owned-failure path and must not be run until `SOURCE_COMMIT` names committed source/test/package content:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
: "${SOURCE_COMMIT:?set SOURCE_COMMIT before phase-2 evidence}"
SOURCE_API_LOCK="$ROOT/validator/artifacts/libvips-safe-source-api-port-lock.json"
SOURCE_API_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-source-api"

cd "$ROOT"
bash scripts/check-layout.sh
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh

# If the phase touched installed ABI, Meson, GIR, or packaging, run this
# after the package build so release-gate can consume build-check/.
# (cd "$ROOT/safe" && scripts/run_release_gate.sh)

rm -rf "$ROOT/validator-overrides/libvips"
mkdir -p "$ROOT/validator-overrides"
SAFELIBS_LIBRARY=libvips \
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
SAFELIBS_DIST_DIR="$ROOT/dist" \
SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
SAFELIBS_LOCK_PATH="$SOURCE_API_LOCK" \
SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
  python3 "$ROOT/scripts/lib/build_port_lock.py"
python3 -m json.tool "$SOURCE_API_LOCK" >/dev/null
FULL_PORT_LOCK="$SOURCE_API_LOCK" OVERRIDE_DIR="$ROOT/validator-overrides/libvips" \
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
rm -rf "$SOURCE_API_ARTIFACT"
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$SOURCE_API_ARTIFACT" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$SOURCE_API_LOCK" \
  --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > "$SOURCE_API_ARTIFACT/matrix-exit-code.txt"

PHASE_ARTIFACT="$SOURCE_API_ARTIFACT" PHASE_LOCK="$SOURCE_API_LOCK" SOURCE_COMMIT="$SOURCE_COMMIT" \
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
- Replace the unique active Phase 2 report section. It must include `Phase start commit`, `Source commit`, `Source fix commits`, fixed testcase ids, regression test names, changed files, remaining failures, package hash table, matrix exit path, and matrix summary.
- Restore any build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` before the report commit.

## Verification Phases
### `check_02_source_api_surface_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_02_source_api_surface_failures`
- Purpose: Verify phase-2 owned failures are fixed or explicitly absent, regression tests pass, and the full validator rerun exists.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-source-api-port-lock.json`
  - `validator/artifacts/libvips-safe-source-api/`
  - `validator/artifacts/libvips-safe-source-api/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-source-api/matrix-exit-code.txt`
  - `scripts/check-layout.sh`
  - `safe/src/**`
  - `safe/tests/abi_layout.rs`
  - `safe/tests/init_version_smoke.rs`
  - `safe/tests/operation_registry.rs`
  - `safe/tests/runtime_io.rs`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - `bash scripts/check-layout.sh`
  - `cd safe && cargo test --all-features --test abi_layout --test init_version_smoke --test operation_registry --test runtime_io -- --nocapture`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `python3 -m json.tool validator/artifacts/libvips-safe-source-api-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json >/dev/null`
  - `cat validator/artifacts/libvips-safe-source-api/matrix-exit-code.txt`
  - run a Python assertion that parses only the unique Phase 1 and Phase 2 sections, checks the phase-2 lock commit/release tag against `SOURCE_COMMIT`, matches lock hashes/sizes to validator-overrides, requires canonical packages ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"] and `unported_original_packages == []`, validates every result JSON, and requires every baseline `impl_02_source_api_surface_failures` testcase to pass unless Phase 2 records zero owned failures.

### `check_02_source_api_surface_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_02_source_api_surface_failures`
- Purpose: Review source-surface fixes for ABI compatibility and ensure no validator changes were used.
- Required preexisting inputs:
  - `validator/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-source-api-port-lock.json`
  - `validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json`
  - `safe/include/vips/**`
  - `safe/src/abi/**`
  - `safe/src/runtime/**`
  - `safe/build.rs`
  - `safe/meson.build`
  - `safe/debian/**`
  - `safe/tests/**`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - assert exactly one exact `## Phase 2 Source API Surface Rerun` heading; parse `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from that bounded section and validate SHAs.
  - `git log --oneline "$PHASE_START"..HEAD`
  - `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/include/vips safe/src/abi safe/src/runtime safe/build.rs safe/meson.build safe/debian safe/tests scripts packaging`
  - if `SOURCE_FIX_COMMITS` is not `none`, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/include/vips safe/src/abi safe/src/runtime safe/build.rs safe/meson.build safe/debian safe/tests scripts packaging`.
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - inspect changed public headers, ABI/runtime files, Meson/build/debian files, and tests against the original public surface.

## Success Criteria
- Every baseline testcase owned by Phase 2 passes in `validator/artifacts/libvips-safe-source-api/`, unless the active Phase 2 section records zero owned failures.
- Focused ABI/header/runtime/package tests pass.
- Official phase-2 lock and every result JSON use the full canonical libvips package set with no original-package fallback.
- Validator tracked files remain clean.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
