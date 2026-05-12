# Phase 1. Update Validator Baseline

## Phase Name

Update validator checkout, build fresh package overrides, and run the current full validator baseline

## Implement Phase ID

`impl_01_update_validator_and_baseline`

## Preexisting Inputs

- Existing validator checkout: `validator/` on local `main` at `87b321fe728340d6fc6dd2f638583cca82c667c3`.
- Existing validator Python environment: `validator/.venv/`.
- Existing validator evidence to preserve and compare: `validator/artifacts/**`.
- Existing local override packages to replace from fresh builds: `validator-overrides/libvips/*.deb`.
- Existing report if present: `validator-report.md`.
- Existing port source and reference inputs: `safe/**`, `original/**`, `safe/reference/**`, `safe/vendor/pyvips-3.1.1/**`.
- Existing build and validation helpers: `scripts/build-debs.sh`, `scripts/run-validation-tests.sh`, `scripts/lib/build_port_lock.py`.
- Existing package metadata: `packaging/package.env`.
- Existing inventories: `all_cves.json`, `relevant_cves.json`, `dependents.json`.

## New Outputs

- Updated `validator/` checkout on `main` at the pulled `origin/main` commit.
- Fresh `dist/*.deb` from `scripts/build-debs.sh`.
- Rewritten `validator-overrides/libvips/*.deb`.
- Full canonical `validator/artifacts/libvips-safe-baseline-current-port-lock.json` with `libvips42t64`, `libvips-dev`, `libvips-tools`, and `gir1.2-vips-8.0` ported in that order and `unported_original_packages: []`.
- Baseline matrix artifact `validator/artifacts/libvips-safe-baseline-current/`, including `port/results/libvips/summary.json` and `matrix-exit-code.txt`.
- Replaced unique active report section `## Phase 1 Current Validator Baseline`.
- Git commit `impl_01 record current validator baseline`.

## File Changes

- `validator-report.md`: replace the active baseline section with fresh evidence, validator commit, testcase counts, package hashes, matrix exit code, and failure classification.
- Preserve previous baseline sections only under renamed `## Historical Evidence - ...` headings.
- Do not change `safe/**` in this phase. If package production needs source or package fixes, record a blocker routed to `impl_05_packaging_container_remaining_failures`.

## Implementation Details

1. Update `validator/`: clone `https://github.com/safelibs/validator` into `validator/` if missing; otherwise require `git -C validator status --porcelain --untracked-files=no` to be empty, switch to local `main`, and run `git -C validator pull --ff-only origin main`.
2. Ensure `validator/.venv/bin/python` exists and can `import yaml`; create the venv or install `PyYAML` only if missing.
3. Validate current testcase metadata with:

   ```bash
   validator/.venv/bin/python validator/tools/testcases.py \
     --config validator/repositories.yml \
     --tests-root validator/tests \
     --library libvips \
     --check \
     --list-summary \
     --min-source-cases 5 \
     --min-usage-cases 170 \
     --min-cases 175
   ```

4. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips`, `PHASE_START_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`, `SOURCE_COMMIT="$PHASE_START_COMMIT"`, and `SOURCE_FIX_COMMITS=none`.
5. Run `bash scripts/install-build-deps.sh` only if dependencies are missing, then run `bash scripts/check-layout.sh` and `SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh`.
6. Rewrite `validator-overrides/libvips/` and build `validator/artifacts/libvips-safe-baseline-current-port-lock.json` from the fresh `dist/*.deb` files with `scripts/lib/build_port_lock.py`.
7. Assert the lock and override directory contain exactly the four canonical packages in order, every recorded sha256 and size matches the copied override `.deb`, and `unported_original_packages == []`.
8. Run the current validator in `port` mode with `--record-casts`, writing artifacts to `validator/artifacts/libvips-safe-baseline-current/` and the exit code to `matrix-exit-code.txt`.
9. Parse every testcase result JSON, excluding `summary.json`, and require `override_debs_installed is true`, the four canonical `port_debs`, and `unported_original_packages == []`.
10. Classify every baseline failure to exactly one later owner: `impl_02_source_api_surface_failures`, `impl_03_operation_semantics_failures`, `impl_04_foreign_io_media_failures`, or `impl_05_packaging_container_remaining_failures`.
11. Replace the unique active `## Phase 1 Current Validator Baseline` section. The section must begin with:

   ```text
   Phase start commit: <40-hex-sha>
   Validator commit: <40-hex-sha>
   Source commit: <40-hex-sha>
   Source fix commits: none
   ```

12. Include the validator URL and commit, `SOURCE_COMMIT`, mode `port`, testcase counts, package lock table, matrix command, matrix exit status, passed/failed/cast counts, and classification table with `Testcase ID`, `Kind`, `Status`, `Owner phase`, `Artifact path`, and `Failure summary`.
13. Restore a build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_01_baseline_software_tester` | check | `impl_01_update_validator_and_baseline` | Verify validator update, testcase metadata, full package override lock, baseline artifact, and failure classification. | Run from repo root: `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; `test "$(git -C validator rev-parse HEAD)" = "$(git -C validator rev-parse origin/main)"`; `validator/.venv/bin/python -c 'import yaml'`; `validator/.venv/bin/python validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libvips --check --list-summary --min-source-cases 5 --min-usage-cases 170 --min-cases 175`; `python3 -m json.tool validator/artifacts/libvips-safe-baseline-current-port-lock.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json >/dev/null`; `cat validator/artifacts/libvips-safe-baseline-current/matrix-exit-code.txt`; run a Python assertion that the active `## Phase 1 Current Validator Baseline` heading appears exactly once, parses only that section, validates the phase/source/validator SHAs, requires `Source fix commits: none`, proves the lock matches `validator-overrides/libvips/*.deb`, proves the canonical package list and empty `unported_original_packages`, proves every result used override debs, proves lock commit/tag/release tag match `Source commit`, proves summary counts match discovered testcase counts, rejects active `port-04-test` fallback evidence, and requires every failed testcase to have one owner row. |
| `check_01_baseline_senior_tester` | check | `impl_01_update_validator_and_baseline` | Review classification quality and confirm validator source was not edited to hide failures. | Run from repo root: `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; assert exactly one active baseline heading; extract `PHASE_START`, `VALIDATOR_COMMIT`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from only that section; validate SHAs; require `SOURCE_FIX_COMMITS=none`; `test "$(git -C validator rev-parse HEAD)" = "$VALIDATOR_COMMIT"`; inspect `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/` and confirm owner mappings follow the phase ownership rules. |

## Success Criteria

Both check phases pass. Baseline validator failures are acceptable only when every failure is backed by full canonical package evidence and classified to exactly one later implement phase.

## Git Commit Requirement

The implementer must commit work to git before yielding. Phase 1 must commit the report evidence as `impl_01 record current validator baseline`; it must not leave the generated split or report work only in the working tree.
