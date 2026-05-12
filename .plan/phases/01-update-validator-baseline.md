# Phase 1: Update Validator And Baseline

## Phase Name
Update validator checkout, build fresh package overrides, and run the current full validator baseline

## Implement Phase ID
`impl_01_update_validator_and_baseline`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator/artifacts/**`
- `validator/site/**`
- `validator-overrides/libvips/*.deb`
- `validator-report.md`
- `original/**`
- `safe/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`
- `scripts/install-build-deps.sh`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/run-validation-tests.sh`
- `scripts/lib/build_port_lock.py`
- `packaging/package.env`

## New Outputs
- Updated `validator/` checkout on local `main` at the pulled `origin/main` commit, or a cloned checkout if it was missing.
- Existing `validator/.venv/` preserved or created, with PyYAML installed only if missing.
- Fresh `dist/*.deb` built from `SOURCE_COMMIT`.
- Rewritten `validator-overrides/libvips/*.deb` from the same `dist/*.deb` files.
- Full canonical `validator/artifacts/libvips-safe-baseline-current-port-lock.json` with all four libvips packages ported and `unported_original_packages: []`.
- Full baseline artifact under `validator/artifacts/libvips-safe-baseline-current/` and `matrix-exit-code.txt`.
- Exactly one active `## Phase 1 Current Validator Baseline` section in `validator-report.md`.
- Git commit `impl_01 record current validator baseline`.

## File Changes
- `validator-report.md`: replace the active baseline section with fresh current-mode evidence, package hashes, testcase counts, matrix exit code, and failure classification. Preserve older copies only under renamed `## Historical Evidence - ...` headings.
- `validator/`: may be fetched/pulled exactly once in this phase; tracked validator files must remain clean after the update. Preserve untracked `.venv/`, `artifacts/`, and `site/` evidence.
- Do not change `safe/**` in Phase 1. If a build-only defect prevents `.deb` production, treat it as Phase 5-owned and record the blocker instead of changing source here.

## Implementation Details
- Start from `/home/yans/safelibs/pipeline/ports/port-libvips`. If `validator/.git` is missing, clone `https://github.com/safelibs/validator` into `validator/`; otherwise require `git -C validator status --porcelain --untracked-files=no` to be empty, switch to local `main`, fetch, and `git -C validator pull --ff-only origin main`. Record the final validator SHA; all later phases must reuse it and must not fetch or pull validator again.
- Preserve the existing virtualenv. Create `validator/.venv` only if missing and install PyYAML only if `validator/.venv/bin/python -c 'import yaml'` fails.
- Validate testcase metadata with `validator/.venv/bin/python validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libvips --check --list-summary --min-source-cases 5 --min-usage-cases 170 --min-cases 175`.
- Set `PHASE_START_COMMIT=$(git rev-parse HEAD)`, `SOURCE_COMMIT=$PHASE_START_COMMIT`, and `SOURCE_FIX_COMMITS=none`. Phase 1 is report-only for this repo.
- Run `bash scripts/check-layout.sh` and `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh`.
- Rewrite `validator-overrides/libvips/` and write `validator/artifacts/libvips-safe-baseline-current-port-lock.json` by running `scripts/lib/build_port_lock.py` with `SAFELIBS_LIBRARY=libvips`, `SAFELIBS_DIST_DIR="$ROOT/dist"`, `SAFELIBS_VALIDATOR_DIR="$ROOT/validator"`, `SAFELIBS_LOCK_PATH`, `SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides"`, and `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT"`.
- Before running validator, assert the lock packages are exactly `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`, `unported_original_packages == []`, and every override file matches the lock size and sha256. If this fails, record a package-production blocker in the baseline report, route it to Phase 5, commit the report, and stop.
- Run `validator/test.sh --mode port --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-baseline-current-port-lock.json" --artifact-root artifacts/libvips-safe-baseline-current --record-casts` with `PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1`, capture its exit code to `matrix-exit-code.txt`, and parse JSON results instead of trusting exit status alone.
- Require every result JSON to report override debs installed, the four canonical port packages, and no `unported_original_packages`. Classify every failing testcase to exactly one owner: Phase 2 for C API/header/pkg-config/GIR/type/metadata source-surface failures; Phase 3 for operation dispatch/argument/pixel/geometry/color/resample/draw/generated-operation semantics; Phase 4 for file/buffer/source/target/load/save/thumbnail/media/materialization/ownership; Phase 5 for packaging/container/dependency/timeout/release-gate/security/CVE or anything uncertain.
- Replace the unique active baseline report section. It must begin with `Phase start commit`, `Validator commit`, `Source commit`, and `Source fix commits: none`, then include validator mode `port`, testcase counts, package lock table, command, exit status, summary counts, and classification table.
- If the build left only an automated `safe/debian/changelog` stamp, restore it to `SOURCE_COMMIT` before committing `validator-report.md`.

## Verification Phases
### `check_01_baseline_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_01_update_validator_and_baseline`
- Purpose: Verify the validator is updated, libvips testcase metadata is current, package overrides and port lock match, the full baseline artifact exists, and every failure is classified.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator/repositories.yml`
  - `validator/tests/libvips/tests/cases/source/*.sh`
  - `validator/tests/libvips/tests/cases/usage/*.sh`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
  - `validator/artifacts/libvips-safe-baseline-current/`
  - `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-baseline-current/matrix-exit-code.txt`
  - `validator-report.md`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - `test "$(git -C validator rev-parse HEAD)" = "$(git -C validator rev-parse origin/main)"`
  - `validator/.venv/bin/python -c 'import yaml'`
  - `validator/.venv/bin/python validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libvips --check --list-summary --min-source-cases 5 --min-usage-cases 170 --min-cases 175`
  - `python3 -m json.tool validator/artifacts/libvips-safe-baseline-current-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json >/dev/null`
  - `cat validator/artifacts/libvips-safe-baseline-current/matrix-exit-code.txt`
  - run a Python assertion that parses only the unique `## Phase 1 Current Validator Baseline` section, validates `Phase start commit`, `Validator commit`, `Source commit`, `Source fix commits: none`, validates the lock/overrides and every result JSON against canonical packages ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"], asserts `unported_original_packages == []`, checks counts, rejects active old `port-04-test` evidence, and requires every failed testcase to be routed to `impl_02`, `impl_03`, `impl_04`, or `impl_05`.

### `check_01_baseline_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_01_update_validator_and_baseline`
- Purpose: Review that classification is defensible and that no validator source was edited to hide failures.
- Required preexisting inputs:
  - `validator/`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
  - `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/`
  - `validator-report.md`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - assert exactly one exact `## Phase 1 Current Validator Baseline` heading; parse `PHASE_START`, `VALIDATOR_COMMIT`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from that bounded section; validate SHAs; require `Source fix commits: none`.
  - `test "$(git -C validator rev-parse HEAD)" = "$VALIDATOR_COMMIT"`
  - inspect failed-case logs under validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/ and confirm owner mapping matches the phase classification rules.

## Success Criteria
- Both Phase 1 check phases pass.
- The baseline section appears exactly once and has the required machine-readable lines.
- Every baseline failure is classified to exactly one later implement phase.
- The validator checkout is clean across tracked files and all baseline lock/result JSON uses the full canonical libvips package set with `unported_original_packages: []`.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
