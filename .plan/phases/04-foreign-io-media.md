# Phase 4: Foreign I/O And Media Failures

## Phase Name
Fix loaders, savers, buffers, sources, targets, thumbnails, and media materialization failures

## Implement Phase ID
`impl_04_foreign_io_media_failures`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator-report.md`
- `validator-overrides/libvips/*.deb`
- `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
- `validator/artifacts/libvips-safe-source-api/`
- `validator/artifacts/libvips-safe-source-api-port-lock.json`
- `validator/artifacts/libvips-safe-ops/`
- `validator/artifacts/libvips-safe-ops-port-lock.json`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/lib/build_port_lock.py`
- `packaging/package.env`
- `safe/build.rs`
- `safe/meson.build`
- `safe/include/vips/**`
- `safe/src/abi/**`
- `safe/src/foreign/base.rs`
- `safe/src/foreign/mod.rs`
- `safe/src/foreign/sniff.rs`
- `safe/src/foreign/loaders/**`
- `safe/src/foreign/savers/**`
- `safe/src/runtime/image.rs`
- `safe/src/runtime/source.rs`
- `safe/src/runtime/target.rs`
- `safe/src/runtime/connection.rs`
- `safe/src/runtime/buf.rs`
- `safe/src/runtime/dbuf.rs`
- `safe/src/runtime/memory.rs`
- `safe/Cargo.toml`
- `safe/Cargo.lock`
- `safe/tests/runtime_io.rs`
- `safe/tests/threading.rs`
- `safe/tests/security.rs`
- `original/test/test-suite/images/**`
- `original/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`

## New Outputs
- Regression tests in `safe/tests/runtime_io.rs`, `safe/tests/threading.rs`, or `safe/tests/security.rs`, or a report-only zero-owned-failure decision.
- Loader/saver/runtime fixes.
- Fresh package overrides and full canonical `validator/artifacts/libvips-safe-foreign-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-foreign/` and `matrix-exit-code.txt`.
- Exactly one active `## Phase 4 Foreign I/O And Media Rerun` section.
- Source commit `impl_04 fix foreign io validator failures` when fixes are needed, followed by report commit `impl_04 record foreign io validator rerun`; if zero owned failures, only `impl_04 record no foreign io failures`.

## File Changes
- Candidate files: `safe/src/foreign/**`, `safe/src/runtime/image.rs`, `safe/src/runtime/source.rs`, `safe/src/runtime/target.rs`, `safe/src/runtime/connection.rs`, `safe/src/runtime/buf.rs`, `safe/src/runtime/dbuf.rs`, `safe/src/runtime/memory.rs`, `safe/Cargo.toml`, `safe/Cargo.lock`, `safe/tests/runtime_io.rs`, `safe/tests/threading.rs`, and `safe/tests/security.rs`.
- Do not modify validator tests or tracked validator source.

## Implementation Details
- Record `PHASE_START_COMMIT=$(git rev-parse HEAD)` at phase start. If the baseline assigns zero failures to `impl_04_foreign_io_media_failures`, do not edit `safe/**`; run focused runtime/media tests, set `SOURCE_COMMIT=$PHASE_START_COMMIT` and `SOURCE_FIX_COMMITS=none`, then produce official phase-4 evidence and a report-only commit.
- For each owned failure, determine whether the path is file, buffer, source, target, thumbnail, CLI, ruby-vips `write_to_buffer`, ruby-vips `new_from_buffer`, or lazy pixel materialization.
- Add regression coverage through the exported C ABI shape: `vips_image_new_from_file`, `vips_image_new_from_buffer`, `vips_image_new_from_source`, `vips_image_write_to_file`, `vips_image_write_to_buffer`, `vips_image_write_to_target`, and format-specific wrappers such as `vips_jpegload_buffer`, `vips_pngsave_buffer`, or `vips_thumbnail`.
- Fix the safe implementation while preserving libvips/GLib ownership: use `PendingDecode` and `ensure_pixels` consistently, keep load-cache invalidation correct after failed decodes, return `VipsBlob`/`VipsArea` buffers with GLib-compatible ownership, preserve `fail_on` semantics, prefer native Rust loaders already in the tree, use external fallback only where intended, and set metadata (`vips-loader`, filename, history, bands, dimensions, interpretation, resolution) where consumers expect it.
- Run runtime I/O, threading, security, upstream shell, and fuzz wrapper tests, then commit source/test fixes as `impl_04 fix foreign io validator failures`, excluding report and generated evidence.
- Set `SOURCE_COMMIT=$(git rev-parse HEAD)` and `SOURCE_FIX_COMMITS=$(git log --format=%H --reverse "$PHASE_START_COMMIT"..HEAD -- safe scripts packaging tests | xargs)`. For nonzero fixes, require this list to be non-empty.
- Run the official phase-4 package/lock/validator command block below. This block is also used for the zero-owned-failure path and must not be run until `SOURCE_COMMIT` names committed source/test/package content:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
: "${SOURCE_COMMIT:?set SOURCE_COMMIT before phase-4 evidence}"
FOREIGN_LOCK="$ROOT/validator/artifacts/libvips-safe-foreign-port-lock.json"
FOREIGN_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-foreign"

cd "$ROOT"
bash scripts/check-layout.sh
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh

rm -rf "$ROOT/validator-overrides/libvips"
mkdir -p "$ROOT/validator-overrides"
SAFELIBS_LIBRARY=libvips \
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
SAFELIBS_DIST_DIR="$ROOT/dist" \
SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
SAFELIBS_LOCK_PATH="$FOREIGN_LOCK" \
SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
  python3 "$ROOT/scripts/lib/build_port_lock.py"
python3 -m json.tool "$FOREIGN_LOCK" >/dev/null
FULL_PORT_LOCK="$FOREIGN_LOCK" OVERRIDE_DIR="$ROOT/validator-overrides/libvips" \
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
rm -rf "$FOREIGN_ARTIFACT"
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$FOREIGN_ARTIFACT" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$FOREIGN_LOCK" \
  --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > "$FOREIGN_ARTIFACT/matrix-exit-code.txt"

PHASE_ARTIFACT="$FOREIGN_ARTIFACT" PHASE_LOCK="$FOREIGN_LOCK" SOURCE_COMMIT="$SOURCE_COMMIT" \
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
- Replace the unique active Phase 4 report section with `Phase start commit`, `Source commit`, `Source fix commits`, exact media paths fixed, package hashes, matrix exit path, summary, and remaining failures.
- Restore any build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` before the report commit.

## Verification Phases
### `check_04_foreign_io_media_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_04_foreign_io_media_failures`
- Purpose: Verify media/I/O failures are fixed, runtime I/O tests pass, and a full validator rerun exists.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-foreign-port-lock.json`
  - `validator/artifacts/libvips-safe-foreign/`
  - `validator/artifacts/libvips-safe-foreign/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-foreign/matrix-exit-code.txt`
  - `safe/tests/runtime_io.rs`
  - `safe/tests/threading.rs`
  - `safe/tests/security.rs`
  - `safe/tests/upstream/**`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && cargo test --all-features --test runtime_io --test threading --test security -- --nocapture`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && rm -rf build-validator-foreign && meson setup build-validator-foreign . --prefix "$PWD/.tmp/validator-foreign-prefix"`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && meson compile -C build-validator-foreign`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && tests/upstream/run-shell-suite.sh build-validator-foreign`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && tests/upstream/run-fuzz-suite.sh build-validator-foreign`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - `python3 -m json.tool validator/artifacts/libvips-safe-foreign-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json >/dev/null`
  - `cat validator/artifacts/libvips-safe-foreign/matrix-exit-code.txt`
  - run a Python assertion that parses only the unique Phase 1 and Phase 4 sections, validates the phase-4 lock and override debs, requires canonical packages ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"] and `unported_original_packages == []`, validates every result JSON, and requires every baseline `impl_04_foreign_io_media_failures` testcase to pass unless Phase 4 records zero owned failures.

### `check_04_foreign_io_media_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_04_foreign_io_media_failures`
- Purpose: Review ownership and safety at the C ABI/GLib boundary for media fixes.
- Required preexisting inputs:
  - `validator/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-foreign-port-lock.json`
  - `validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json`
  - `safe/src/foreign/**`
  - `safe/src/runtime/image.rs`
  - `safe/src/runtime/source.rs`
  - `safe/src/runtime/target.rs`
  - `safe/src/runtime/connection.rs`
  - `safe/src/runtime/buf.rs`
  - `safe/src/runtime/dbuf.rs`
  - `safe/src/runtime/memory.rs`
  - `safe/Cargo.toml`
  - `safe/Cargo.lock`
  - `safe/tests/runtime_io.rs`
  - `safe/tests/threading.rs`
  - `safe/tests/security.rs`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - assert exactly one exact `## Phase 4 Foreign I/O And Media Rerun` heading; parse and validate `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from that bounded section.
  - `git log --oneline "$PHASE_START"..HEAD`
  - `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/src/foreign safe/src/runtime/image.rs safe/src/runtime/source.rs safe/src/runtime/target.rs safe/src/runtime/connection.rs safe/src/runtime/buf.rs safe/src/runtime/dbuf.rs safe/src/runtime/memory.rs safe/Cargo.toml safe/Cargo.lock safe/tests/runtime_io.rs safe/tests/threading.rs safe/tests/security.rs`
  - if `SOURCE_FIX_COMMITS` is not `none`, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/src/foreign safe/src/runtime/image.rs safe/src/runtime/source.rs safe/src/runtime/target.rs safe/src/runtime/connection.rs safe/src/runtime/buf.rs safe/src/runtime/dbuf.rs safe/src/runtime/memory.rs safe/Cargo.toml safe/Cargo.lock safe/tests/runtime_io.rs safe/tests/threading.rs safe/tests/security.rs`.
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - inspect foreign/runtime/media changes, Cargo changes, and tests; confirm GLib-compatible ownership and no validator tests were edited.

## Success Criteria
- Every baseline testcase owned by Phase 4 passes in `validator/artifacts/libvips-safe-foreign/`, unless the active Phase 4 section records zero owned failures.
- Runtime/media tests cover ownership and materialization behavior.
- Official phase-4 lock and every result JSON use the full canonical libvips package set with no original-package fallback.
- Validator tracked files remain clean.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
