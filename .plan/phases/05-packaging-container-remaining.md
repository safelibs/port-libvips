# Phase 5. Packaging Container Remaining

## Phase Name

Fix package/container issues and every remaining validator failure

## Implement Phase ID

`impl_05_packaging_container_remaining_failures`

## Preexisting Inputs

- All previous phase report sections and classifications in `validator-report.md`.
- Previous phase artifacts and locks: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`, `validator/artifacts/libvips-safe-baseline-current/`, `validator/artifacts/libvips-safe-source-api-port-lock.json`, `validator/artifacts/libvips-safe-source-api/`, `validator/artifacts/libvips-safe-ops-port-lock.json`, `validator/artifacts/libvips-safe-ops/`, `validator/artifacts/libvips-safe-foreign-port-lock.json`, and `validator/artifacts/libvips-safe-foreign/`.
- Updated Phase 1 validator checkout and venv: `validator/`, `validator/.venv/`.
- Build, CI parity, and lock helpers: `scripts/build-debs.sh`, `scripts/install-build-deps.sh`, `scripts/run-validation-tests.sh`, `scripts/lib/build-deb-common.sh`, `scripts/lib/build_port_lock.py`.
- Package metadata and release gate inputs: `packaging/package.env`, `safe/debian/**`, `safe/meson.build`, `safe/scripts/run_release_gate.sh`.
- Remaining failing validator logs and testcase results from prior phases.
- Existing safety/dependent inputs for catch-all fixes: `safe/tests/**`, `safe/tests/dependents/**`, `all_cves.json`, `relevant_cves.json`, `dependents.json`.

## New Outputs

- Catch-all regression tests or package/script/source fixes.
- Fresh `dist/*.deb`, rewritten `validator-overrides/libvips/*.deb`, and full canonical `validator/artifacts/libvips-safe-remaining-port-lock.json`.
- Full stable rerun artifact `validator/artifacts/libvips-safe-remaining/`.
- CI-parity evidence from `scripts/run-validation-tests.sh`: `.work/validation/port-deb-lock.json` and `.work/validation/artifacts/`.
- If an approved validator-bug skip exists: unmodified failing artifact `validator/artifacts/libvips-safe-remaining-unmodified/`, passing transient-skip artifact `validator/artifacts/libvips-safe-remaining/`, `.work/validator-remaining-approved/approved-skip-manifest.json`, and status files `.work/validator-remaining-approved-status-before.txt` and `.work/validator-remaining-approved-status-after.txt`.
- Replaced unique active report section `## Phase 5 Packaging Container And Remaining Rerun`.
- Source commit `impl_05 fix remaining validator failures` when ordinary fixes are needed, followed by report commit `impl_05 record remaining validator rerun`. If no ordinary fixes are needed, only the report commit is required.

## File Changes

- Candidate files: `scripts/*.sh`, `scripts/lib/*.py`, `packaging/package.env`, `safe/debian/**`, `safe/meson.build`, `safe/scripts/run_release_gate.sh`, `safe/src/**`, and `safe/tests/**`.
- Do not edit validator tests for ordinary failures.

## Implementation Details

1. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips` and `PHASE_START_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`.
2. Collect all failures still failing after phases 2-4 and separate ordinary package/container/libvips-safe defects from clear validator bugs.
3. Treat these as ordinary defects to fix: missing canonical packages, partial locks, original Ubuntu fallback through `unported_original_packages`, wrong `.deb` dependencies, bad `scripts/lib/build_port_lock.py` output, release gate failures, CVE/security regressions, timeouts, performance issues, or any unknown failure.
4. Add or update focused regression tests for ordinary defects and fix the relevant `safe/**`, `scripts/**`, or `packaging/**` files. Do not raise validator timeouts to hide failures.
5. Run focused checks for files changed in this phase. Do not create official evidence from uncommitted changes.
6. Commit ordinary source/test/package fixes before building official packages as `impl_05 fix remaining validator failures`; this commit must not include `validator-report.md`, `validator/**`, `validator-overrides/**`, `dist/**`, `.work/**`, or generated build artifacts.
7. Set `SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"` and compute `SOURCE_FIX_COMMITS` from `"$PHASE_START_COMMIT"..HEAD` over `safe`, `scripts`, `packaging`, and `tests`. If no ordinary fixes were needed, set `SOURCE_COMMIT="$PHASE_START_COMMIT"` and `SOURCE_FIX_COMMITS=none`.
8. Run full local gates using `SOURCE_COMMIT` for traceability:

   ```bash
   bash scripts/check-layout.sh
   SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh
   PYTHON="$ROOT/validator/.venv/bin/python" \
   SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
   SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
   SAFELIBS_RECORD_CASTS=1 \
     bash scripts/run-validation-tests.sh
   cd safe && cargo test --all-features -- --nocapture
   cd "$ROOT/safe" && scripts/run_release_gate.sh
   ```

9. Inspect `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/summary.json`, and every `.work/validation/artifacts/port/results/libvips/*.json` except `summary.json`. The CI-parity lock and every result must prove the four canonical packages, `override_debs_installed is true`, and `unported_original_packages == []`. Do not trust hook exit status alone.
10. If CI-parity failures are ordinary or unknown, fix them, create another source/test/package commit, update `SOURCE_COMMIT`, and rerun from the full gates.
11. Rewrite the stable phase-5 lock and override root from the same `dist/*.deb` files, then run the controlled validator into `validator/artifacts/libvips-safe-remaining/`:

    ```bash
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
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

12. Assert the stable lock and every stable result have the full canonical package list, `override_debs_installed is true`, and `unported_original_packages == []`. In the ordinary clean path, require stable summary `failed == 0`.
13. Use the approved validator-bug skip path only after both CI-parity and the stable unmodified artifact prove the same failed testcase ids are validator bugs rather than ordinary defects. Before using a transient copy, record `Approved validator-bug testcase ids: <ids>` in the in-progress active Phase 5 report section.
14. For an approved validator-bug skip, preserve the unmodified failing artifact at `validator/artifacts/libvips-safe-remaining-unmodified/`, create `.work/validator-remaining-approved/` from the current local validator checkout without fetching or pulling, remove only the documented testcase scripts, write `.work/validator-remaining-approved/approved-skip-manifest.json`, verify adjusted counts, and run the transient validator into `validator/artifacts/libvips-safe-remaining/`.
15. The approved-skip report evidence must include testcase id, kind, reason, original testcase script path, removed transient-copy path, unmodified failing artifact path, transient-skip artifact path, transient skip manifest path, and `Approved skip adjusted counts: source=<n> usage=<n> total=<n>`.
16. Replace the unique active `## Phase 5 Packaging Container And Remaining Rerun` section. The section must begin with `Phase start commit`, `Source commit`, and `Source fix commits` machine-readable lines and include all remaining failures resolved or approved, package hash table, CI-parity/stable lock status, and explicit `unported_original_packages: []` evidence.
17. Restore a build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_05_packaging_container_remaining_software_tester` | check | `impl_05_packaging_container_remaining_failures` | Verify CI parity, release gate, package build, and stable validator rerun are clean or only documented validator-bug skips remain. | Run from repo root: parse `SOURCE_COMMIT` from the unique active Phase 5 section and validate it with `git cat-file`; `bash scripts/check-layout.sh`; `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh`; `PYTHON="$PWD/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" SAFELIBS_VALIDATOR_DIR="$PWD/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh`; `cd safe && cargo test --all-features -- --nocapture`; `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && scripts/run_release_gate.sh`; return to repo root; validate `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/summary.json`, `validator/artifacts/libvips-safe-remaining-port-lock.json`, and `validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json` with `python3 -m json.tool`; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; run a Python assertion that both CI-parity and stable locks have the canonical package list and empty `unported_original_packages`, every result has override debs installed with no fallback, stable lock commit/tag/release tag matches the active Phase 5 `SOURCE_COMMIT`, stable lock matches `validator-overrides/libvips/*.deb`, and both summaries have `failed == 0` unless the active Phase 5 section documents approved validator-bug ids; in that branch, assert `.work/validation` and `validator/artifacts/libvips-safe-remaining-unmodified/` fail only those ids, `.work/validator-remaining-approved/approved-skip-manifest.json` records the same ids/paths/counts as the report, and `validator/artifacts/libvips-safe-remaining/` is the passing transient-skip artifact. |
| `check_05_packaging_container_remaining_senior_tester` | check | `impl_05_packaging_container_remaining_failures` | Review catch-all changes and ensure real compatibility issues were not hidden as packaging skips. | Run from repo root: assert exactly one active Phase 5 heading; parse `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from only that section and validate SHAs; `git log --oneline "$PHASE_START"..HEAD`; `git diff --stat "$PHASE_START"..HEAD -- validator-report.md scripts packaging safe/debian safe/meson.build safe/scripts safe/src safe/tests`; if fixes exist, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- scripts packaging safe/debian safe/meson.build safe/scripts safe/src safe/tests`; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; if approved skips are documented, parse `.work/validator-remaining-approved/approved-skip-manifest.json` and verify ids, paths, and counts match the active section; inspect failure classification and changes to scripts/package/runtime files; confirm no validator source edits. |

## Success Criteria

The phase ends with zero remaining ordinary validator failures and full canonical libvips package evidence in both CI-parity and stable locks. Any nonzero unmodified result is acceptable only as a documented validator bug with a passing transient approved-skip artifact.

## Git Commit Requirement

The implementer must commit work to git before yielding. Ordinary source/test/package fixes must be committed before official package evidence, and the phase report must be committed after evidence is recorded.
