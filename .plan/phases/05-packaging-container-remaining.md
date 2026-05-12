# Phase 5: Packaging, Container, And Remaining Failures

## Phase Name
Fix package/container issues and every remaining validator failure

## Implement Phase ID
`impl_05_packaging_container_remaining_failures`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator-report.md`
- `validator-overrides/libvips/*.deb`
- `validator/artifacts/libvips-safe-baseline-current/`
- `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- `validator/artifacts/libvips-safe-source-api/`
- `validator/artifacts/libvips-safe-source-api-port-lock.json`
- `validator/artifacts/libvips-safe-ops/`
- `validator/artifacts/libvips-safe-ops-port-lock.json`
- `validator/artifacts/libvips-safe-foreign/`
- `validator/artifacts/libvips-safe-foreign-port-lock.json`
- `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/`
- `validator/artifacts/libvips-safe-source-api/port/logs/libvips/`
- `validator/artifacts/libvips-safe-ops/port/logs/libvips/`
- `validator/artifacts/libvips-safe-foreign/port/logs/libvips/`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/install-build-deps.sh`
- `scripts/run-validation-tests.sh`
- `scripts/lib/build-deb-common.sh`
- `scripts/lib/build_port_lock.py`
- `packaging/package.env`
- `safe/Cargo.toml`
- `safe/Cargo.lock`
- `safe/build.rs`
- `safe/include/vips/**`
- `safe/src/**`
- `safe/tests/**`
- `safe/debian/**`
- `safe/meson.build`
- `safe/scripts/run_release_gate.sh`
- `original/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`

## New Outputs
- Catch-all regression tests or package/script fixes for every ordinary remaining failure.
- Fresh package overrides and full canonical `validator/artifacts/libvips-safe-remaining-port-lock.json`.
- Stable rerun artifact `validator/artifacts/libvips-safe-remaining/` and `matrix-exit-code.txt`.
- If an approved validator-bug skip exists, `validator/artifacts/libvips-safe-remaining-unmodified/` plus passing transient-skip artifact `validator/artifacts/libvips-safe-remaining/` and `.work/validator-remaining-approved/approved-skip-manifest.json`.
- CI-parity artifact from `scripts/run-validation-tests.sh` under `.work/validation/artifacts` and `.work/validation/port-deb-lock.json`.
- Exactly one active `## Phase 5 Packaging Container And Remaining Rerun` section.
- Source/test/package commit `impl_05 fix remaining validator failures` when ordinary fixes are needed, followed by report commit `impl_05 record remaining validator rerun`; if no ordinary fixes, only the report commit is required.

## File Changes
- Possible files: `scripts/*.sh`, `scripts/lib/*.py`, `packaging/package.env`, `safe/debian/**`, `safe/meson.build`, `safe/scripts/run_release_gate.sh`, or any `safe/src/**` file required for remaining compatibility or safety failures.
- Do not edit validator tests for ordinary failures; use a transient copy only for the documented validator-bug skip procedure.

## Implementation Details
- Record `PHASE_START_COMMIT=$(git rev-parse HEAD)` and collect all failures still failing after Phases 2-4.
- Separate ordinary package/container/libvips-safe defects from validator bugs. Fix package names and Debian metadata so `dist/` covers `libvips42t64`, `libvips-dev`, `libvips-tools`, and `gir1.2-vips-8.0` exactly once each. Fix dependency metadata, lock synthesis, release-gate/CVE/security regressions, and timeout/performance problems at their source; do not accept `unported_original_packages` fallback and do not raise validator timeouts.
- Run focused checks for changed files. Commit ordinary source/test/package fixes as `impl_05 fix remaining validator failures`, excluding report and generated evidence. If no ordinary fixes were needed, set `SOURCE_COMMIT=$PHASE_START_COMMIT` and `SOURCE_FIX_COMMITS=none`.
- With `SOURCE_COMMIT` set to committed content, run full local gates: `bash scripts/check-layout.sh`; `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh`; `PYTHON="$ROOT/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh`; `cd safe && cargo test --all-features -- --nocapture`; `cd safe && scripts/run_release_gate.sh`. Build must precede release gate so `build-check/` and `build-check-install/` are fresh.
- If any full gate fails from an ordinary defect, fix it, add/update a regression test, create another source/test/package commit, update `SOURCE_COMMIT` and `SOURCE_FIX_COMMITS`, discard prior phase-local packages/locks/artifacts, and rerun the full gates.
- Inspect `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/summary.json`, and every `.work/validation` testcase result. Require canonical packages, no `unported_original_packages`, and `override_debs_installed is true` before considering skip approval. If failed ids are ordinary, unknown, install, dependency, timeout, environment, or package fallback failures, fix them as ordinary defects.
- Rewrite stable `validator-overrides/libvips/` and `validator/artifacts/libvips-safe-remaining-port-lock.json` from the same `dist/*.deb` files using `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT"`. Validate package order, hashes, sizes, commit/tag/release tag, and `unported_original_packages == []`.
- Run the stable unmodified validator from the real Phase 1 checkout into `validator/artifacts/libvips-safe-remaining/`, capture `matrix-exit-code.txt`, and parse summary/result JSON. In the ordinary path, require `failed == 0`.
- Approved validator-bug skip path, only after both CI-parity and stable unmodified artifacts prove the same failed ids are validator bugs: write `Approved validator-bug testcase ids: <ids>` in the active Phase 5 section being prepared; move `validator/artifacts/libvips-safe-remaining/` to `validator/artifacts/libvips-safe-remaining-unmodified/` and preserve `unmodified-matrix-exit-code.txt`; create `APPROVED_VALIDATOR="$ROOT/.work/validator-remaining-approved"` by `rsync -a --delete` from the real validator while excluding `.git`, `.venv`, `artifacts`, and `site`; require real validator tracked status clean before and after; remove only the documented testcase scripts whose single `# @testcase` id matches the approved ids; write `.work/validator-remaining-approved/approved-skip-manifest.json` with testcase id, kind, original path, removed copy path, original counts, and adjusted counts; run the transient `tools/testcases.py --check --list-summary` with adjusted count thresholds; run transient `test.sh --mode port` into `validator/artifacts/libvips-safe-remaining/`; require failed `0`, canonical packages in every result, no original fallback, and summary counts equal the manifest.

Use this exact stable Phase 5 lock and validator command block after the CI-parity gate:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
: "${SOURCE_COMMIT:?set SOURCE_COMMIT before phase-5 evidence}"
REMAINING_LOCK="$ROOT/validator/artifacts/libvips-safe-remaining-port-lock.json"
rm -rf "$ROOT/validator-overrides/libvips"
mkdir -p "$ROOT/validator-overrides"
SAFELIBS_LIBRARY=libvips \
SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
SAFELIBS_DIST_DIR="$ROOT/dist" \
SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
SAFELIBS_LOCK_PATH="$REMAINING_LOCK" \
SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
  python3 "$ROOT/scripts/lib/build_port_lock.py"
FULL_PORT_LOCK="$REMAINING_LOCK" OVERRIDE_DIR="$ROOT/validator-overrides/libvips" \
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
rm -rf artifacts/libvips-safe-remaining
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-remaining \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$REMAINING_LOCK" \
  --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > artifacts/libvips-safe-remaining/matrix-exit-code.txt
```

For a Phase 5 approved validator-bug skip, preserve the unmodified failing artifact before creating the transient copy:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
rm -rf "$ROOT/validator/artifacts/libvips-safe-remaining-unmodified"
mv "$ROOT/validator/artifacts/libvips-safe-remaining" "$ROOT/validator/artifacts/libvips-safe-remaining-unmodified"
cp "$ROOT/validator/artifacts/libvips-safe-remaining-unmodified/matrix-exit-code.txt" \
   "$ROOT/validator/artifacts/libvips-safe-remaining-unmodified/unmodified-matrix-exit-code.txt"
```

Then create and audit the transient validator copy without editing the real checkout:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
APPROVED_VALIDATOR="$ROOT/.work/validator-remaining-approved"
APPROVED_SKIP_MANIFEST="$APPROVED_VALIDATOR/approved-skip-manifest.json"
mkdir -p "$ROOT/.work"
git -C "$ROOT/validator" status --porcelain --untracked-files=no | tee "$ROOT/.work/validator-remaining-approved-status-before.txt"
test ! -s "$ROOT/.work/validator-remaining-approved-status-before.txt"
rm -rf "$APPROVED_VALIDATOR"
rsync -a --delete \
  --exclude '.git' \
  --exclude '.venv' \
  --exclude 'artifacts' \
  --exclude 'site' \
  "$ROOT/validator/" "$APPROVED_VALIDATOR/"

APPROVED_SKIP_IDS="<space-separated ids from validator-report.md>" \
ORIGINAL_VALIDATOR="$ROOT/validator" \
APPROVED_VALIDATOR="$APPROVED_VALIDATOR" \
APPROVED_SKIP_MANIFEST="$APPROVED_SKIP_MANIFEST" \
  "$ROOT/validator/.venv/bin/python" - <<'PY'
import json
import os
import re
from pathlib import Path

original = Path(os.environ["ORIGINAL_VALIDATOR"])
approved = Path(os.environ["APPROVED_VALIDATOR"])
ids = [item for item in os.environ["APPROVED_SKIP_IDS"].split() if item]
if not ids:
    raise SystemExit("approved skip branch requires at least one testcase id")
id_re = re.compile(r"^[a-z0-9][a-z0-9-]{1,78}[a-z0-9]$")
for testcase_id in ids:
    if not id_re.fullmatch(testcase_id):
        raise SystemExit(f"invalid testcase id: {testcase_id}")

def discover(root):
    found = {}
    counts = {"source": 0, "usage": 0}
    for kind in ("source", "usage"):
        for path in sorted((root / "tests/libvips/tests/cases" / kind).glob("*.sh")):
            text = path.read_text()
            blocks = re.findall(r"(?m)^#\s*@testcase:\s*([^\s#]+)\s*$", text)
            for testcase_id in blocks:
                found.setdefault(testcase_id, []).append((kind, path, len(blocks)))
                counts[kind] += 1
    return found, counts

original_cases, original_counts = discover(original)
removed = []
for testcase_id in ids:
    matches = original_cases.get(testcase_id, [])
    if len(matches) != 1:
        raise SystemExit(f"{testcase_id}: expected exactly one original script, found {len(matches)}")
    kind, original_path, block_count = matches[0]
    if block_count != 1:
        raise SystemExit(f"{testcase_id}: script has {block_count} testcase headers; do not partially edit scripts")
    rel = original_path.relative_to(original)
    copy_path = approved / rel
    if not copy_path.is_file():
        raise SystemExit(f"{testcase_id}: missing transient copy path {copy_path}")
    if copy_path.read_bytes() != original_path.read_bytes():
        raise SystemExit(f"{testcase_id}: transient copy differs before removal")
    copy_path.unlink()
    removed.append({
        "testcase_id": testcase_id,
        "kind": kind,
        "original_path": str(original_path),
        "removed_copy_path": str(copy_path),
    })

approved_cases, adjusted_counts = discover(approved)
missing = sorted(set(original_cases) - set(approved_cases))
if missing != sorted(ids):
    raise SystemExit(f"transient copy removed unexpected testcase ids: {missing!r}")
adjusted = {
    "source": adjusted_counts["source"],
    "usage": adjusted_counts["usage"],
    "total": adjusted_counts["source"] + adjusted_counts["usage"],
}
original_total = original_counts["source"] + original_counts["usage"]
manifest = {
    "library": "libvips",
    "approved_skip_ids": ids,
    "removed": removed,
    "original_counts": {
        "source": original_counts["source"],
        "usage": original_counts["usage"],
        "total": original_total,
    },
    "adjusted_counts": adjusted,
}
Path(os.environ["APPROVED_SKIP_MANIFEST"]).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(adjusted["source"], adjusted["usage"], adjusted["total"])
PY
```

Validate the transient inventory and run the transient Phase 5 matrix into the stable artifact path:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
APPROVED_VALIDATOR="$ROOT/.work/validator-remaining-approved"
APPROVED_SKIP_MANIFEST="$APPROVED_VALIDATOR/approved-skip-manifest.json"
REMAINING_LOCK="$ROOT/validator/artifacts/libvips-safe-remaining-port-lock.json"
REMAINING_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-remaining"
read ADJUSTED_SOURCE_CASES ADJUSTED_USAGE_CASES ADJUSTED_TOTAL_CASES < <(
  APPROVED_SKIP_MANIFEST="$APPROVED_SKIP_MANIFEST" \
  "$ROOT/validator/.venv/bin/python" - <<'PY'
import json
import os
from pathlib import Path
manifest = json.loads(Path(os.environ["APPROVED_SKIP_MANIFEST"]).read_text())
counts = manifest["adjusted_counts"]
print(counts["source"], counts["usage"], counts["total"])
PY
)
"$ROOT/validator/.venv/bin/python" "$APPROVED_VALIDATOR/tools/testcases.py" \
  --config "$APPROVED_VALIDATOR/repositories.yml" \
  --tests-root "$APPROVED_VALIDATOR/tests" \
  --library libvips \
  --check \
  --list-summary \
  --min-source-cases "$ADJUSTED_SOURCE_CASES" \
  --min-usage-cases "$ADJUSTED_USAGE_CASES" \
  --min-cases "$ADJUSTED_TOTAL_CASES"
git -C "$ROOT/validator" status --porcelain --untracked-files=no | tee "$ROOT/.work/validator-remaining-approved-status-after.txt"
test ! -s "$ROOT/.work/validator-remaining-approved-status-after.txt"

rm -rf "$REMAINING_ARTIFACT"
cd "$APPROVED_VALIDATOR"
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$REMAINING_ARTIFACT" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$REMAINING_LOCK" \
  --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > "$REMAINING_ARTIFACT/matrix-exit-code.txt"
```
- Replace the unique active Phase 5 section with `Phase start commit`, `Source commit`, `Source fix commits`, all remaining failures resolved or approved, package hashes, stable and CI-parity lock proof, and, for skips, testcase id, kind, reason, original script path, removed copy path, unmodified artifact path, transient artifact path, manifest path, and `Approved skip adjusted counts: source=<n> usage=<n> total=<n>`.
- Restore any build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` before the report commit.

## Verification Phases
### `check_05_packaging_container_remaining_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_05_packaging_container_remaining_failures`
- Purpose: Verify CI parity, release gate, package build, and validator rerun are clean or only documented validator-bug skips remain.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-remaining-port-lock.json`
  - `validator/artifacts/libvips-safe-remaining/`
  - `validator/artifacts/libvips-safe-remaining/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-remaining/matrix-exit-code.txt`
  - `.work/validation/port-deb-lock.json`
  - `.work/validation/artifacts/port/results/libvips/*.json`
  - `scripts/check-layout.sh`
  - `scripts/build-debs.sh`
  - `scripts/run-validation-tests.sh`
  - `scripts/lib/build_port_lock.py`
  - `safe/**`
  - `safe/scripts/run_release_gate.sh`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - extract `SOURCE_COMMIT` from the unique bounded active `## Phase 5 Packaging Container And Remaining Rerun` section and validate it with `git cat-file`.
  - `bash scripts/check-layout.sh`
  - `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh`
  - `PYTHON="$PWD/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" SAFELIBS_VALIDATOR_DIR="$PWD/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh`
  - `cd safe && cargo test --all-features -- --nocapture`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && scripts/run_release_gate.sh`
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `python3 -m json.tool .work/validation/port-deb-lock.json >/dev/null`
  - `python3 -m json.tool .work/validation/artifacts/port/results/libvips/summary.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-remaining-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json >/dev/null`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - run a Python assertion over only the unique active Phase 5 section, `.work/validation`, and stable remaining artifacts; both locks must use canonical packages ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"], `unported_original_packages == []`, and every result must have override debs installed. Without approved skips, both summaries must have `failed == 0`. With approved skips, failed ids, unmodified artifact, `.work/validator-remaining-approved/approved-skip-manifest.json`, adjusted counts, and transient passing artifact must match the active Phase 5 section.

### `check_05_packaging_container_remaining_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_05_packaging_container_remaining_failures`
- Purpose: Review catch-all changes and ensure real compatibility issues were not hidden as packaging skips.
- Required preexisting inputs:
  - `validator/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-remaining-port-lock.json`
  - `validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json`
  - `scripts/**`
  - `packaging/package.env`
  - `safe/debian/**`
  - `safe/meson.build`
  - `safe/scripts/**`
  - `safe/src/**`
  - `safe/tests/**`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - assert exactly one exact `## Phase 5 Packaging Container And Remaining Rerun` heading; parse and validate `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from that bounded section.
  - `git log --oneline "$PHASE_START"..HEAD`
  - `git diff --stat "$PHASE_START"..HEAD -- validator-report.md scripts packaging safe/debian safe/meson.build safe/scripts safe/src safe/tests`
  - if `SOURCE_FIX_COMMITS` is not `none`, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- scripts packaging safe/debian safe/meson.build safe/scripts safe/src safe/tests`.
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - if approved validator-bug skips are documented, parse `.work/validator-remaining-approved/approved-skip-manifest.json` and verify ids, original paths, removed-copy paths, and adjusted counts match the active Phase 5 section.
  - inspect failure classification and broad package/script/runtime changes; confirm no validator source edits.

## Success Criteria
- There are zero remaining ordinary validator failures.
- CI-parity and stable Phase 5 locks are full canonical libvips locks with `unported_original_packages: []`.
- Any nonzero unmodified result is solely a documented validator bug with a passing transient-skip artifact and full audit manifest.
- Validator tracked files remain clean.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
