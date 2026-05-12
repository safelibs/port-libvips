# Phase 6. Final Clean Run Report

## Phase Name

Produce final clean validator evidence, proof/site artifacts, and closing report

## Implement Phase ID

`impl_06_final_clean_run_and_report`

## Preexisting Inputs

- All previous phase artifacts and active report sections in `validator-report.md`.
- Updated Phase 1 validator checkout and venv: `validator/`, `validator/.venv/`.
- Latest safe source and regression tests: `safe/**`.
- Build, release, validation, and lock helpers: `scripts/check-layout.sh`, `scripts/build-debs.sh`, `scripts/run-validation-tests.sh`, `scripts/lib/build_port_lock.py`, `packaging/package.env`, `safe/scripts/run_release_gate.sh`.
- Latest local override packages from the previous build: `validator-overrides/libvips/*.deb`.
- Existing final evidence to preserve and compare before replacement: `validator/artifacts/libvips-safe-final-port-lock.json`, `validator/artifacts/libvips-safe-final/`, `validator/artifacts/libvips-safe-final/port/results/libvips/summary.json`, `validator/site/libvips-safe-final/`, and `validator/site/libvips-safe-final/site-data.json`.
- Do not assume `build-check/` or `build-check-install/` already exist; this phase refreshes them by running `scripts/build-debs.sh` before `safe/scripts/run_release_gate.sh`.

## New Outputs

- Fresh final packages under `dist/` and rewritten `validator-overrides/libvips/*.deb`.
- Full canonical `validator/artifacts/libvips-safe-final-port-lock.json`.
- Final matrix artifact `validator/artifacts/libvips-safe-final/`.
- If an approved validator-bug skip exists: `validator/artifacts/libvips-safe-final-unmodified/`, passing transient-skip artifact `validator/artifacts/libvips-safe-final/`, `.work/validator-final-approved/approved-skip-manifest.json`, and status files `.work/validator-final-approved-status-before.txt` and `.work/validator-final-approved-status-after.txt`.
- Proof output `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`.
- Rendered site under `validator/site/libvips-safe-final/`, including `site-data.json`.
- Replaced unique active report section `## Final Clean Run`.
- Final git commit `impl_06 record final validator clean run`.

## File Changes

- `validator-report.md`: replace or complete the unique active final summary section `## Final Clean Run`.
- Do not patch production code in this phase. If final validation finds a new ordinary libvips-safe or packaging defect, append `## Final Clean Run Blocked` with failing testcase ids, artifact paths, and the reason remediation must happen in a new linear workflow; commit that report update and yield failure.

## Implementation Details

1. Confirm the validator checkout is clean and still on the Phase 1 recorded commit. Do not fetch or pull. If a newer validator is required, record a `## Final Clean Run Blocked` report section, commit it, and fail this phase.
2. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips` and `FINAL_SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`. Phase 6 does not change production code, so this commit is the source/package commit for final evidence.
3. Run final local gates that do not depend on the reference Meson install:

   ```bash
   bash scripts/check-layout.sh
   cd safe && cargo test --all-features -- --nocapture
   ```

4. Build final packages with `SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" bash scripts/build-debs.sh`, then run `cd safe && scripts/run_release_gate.sh`.
5. Regenerate `validator/artifacts/libvips-safe-final-port-lock.json` and `validator-overrides/libvips/` exactly once from the step-4 `dist/*.deb` files:

   ```bash
   ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
   FINAL_SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
   FINAL_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-final"
   FINAL_LOCK="$ROOT/validator/artifacts/libvips-safe-final-port-lock.json"
   FINAL_UNMODIFIED="$ROOT/validator/artifacts/libvips-safe-final-unmodified"
   rm -rf "$ROOT/validator-overrides/libvips"
   mkdir -p "$ROOT/validator-overrides"
   SAFELIBS_LIBRARY=libvips \
   SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" \
   SAFELIBS_DIST_DIR="$ROOT/dist" \
   SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
   SAFELIBS_LOCK_PATH="$FINAL_LOCK" \
   SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
     python3 "$ROOT/scripts/lib/build_port_lock.py"
   ```

6. Assert the final lock has the canonical package list, `unported_original_packages == []`, matching override sha256s/sizes, and commit/tag/release tag matching `FINAL_SOURCE_COMMIT`.
7. If Phase 5 does not document approved validator-bug ids, run the unmodified validator into `validator/artifacts/libvips-safe-final/`. If it exits 0 and summary `failed == 0`, use that as final evidence. If it fails for ordinary, package, dependency, timeout, environment, or unknown reasons, record and commit a blocker instead of patching code.
8. If the unmodified final run first exposes a clear validator bug, create or update the in-progress active `## Final Clean Run` section before using a transient copy. It must include:

   ```text
   Final validator commit: <40-hex-sha>
   Final source commit: <40-hex-sha>
   Approved validator-bug testcase ids: <ids>
   ```

   Move the unmodified failing artifact to `validator/artifacts/libvips-safe-final-unmodified/`.
9. If Phase 5 already approved validator-bug ids, extract them only from the unique active Phase 5 section, run the unmodified validator into `validator/artifacts/libvips-safe-final-unmodified/`, and require it to fail exactly those ids.
10. For either approved-skip branch, create `.work/validator-final-approved/` from the current validator checkout without fetching or pulling, remove only documented testcase scripts, write `.work/validator-final-approved/approved-skip-manifest.json`, verify adjusted counts, and run the transient validator into `validator/artifacts/libvips-safe-final/`. The active final section must also include `Approved skip adjusted counts: source=<n> usage=<n> total=<n>`.
11. Generate proof for libvips only. Use normal thresholds `5`, `170`, and `175` for an unmodified clean run:

    ```bash
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    FINAL_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-final"
    FINAL_PROOF="$FINAL_ARTIFACT/proof/libvips-safe-validation-proof.json"
    cd "$ROOT/validator"
    "$ROOT/validator/.venv/bin/python" tools/verify_proof_artifacts.py \
      --config repositories.yml \
      --tests-root tests \
      --artifact-root "$FINAL_ARTIFACT" \
      --proof-output "$FINAL_PROOF" \
      --mode port \
      --library libvips \
      --min-source-cases 5 \
      --min-usage-cases 170 \
      --min-cases 175 \
      --require-casts \
      --ports-root /home/yans/safelibs/pipeline/ports
    ```

    For an approved skip, run proof from `.work/validator-final-approved/` using the adjusted counts recorded in the unique active final section.
12. Render and verify the site. For an unmodified clean run, use `validator/`; for an approved skip, use `.work/validator-final-approved/`. In both cases keep artifact, proof, and site roots under `$ROOT/validator` and run `scripts/verify-site.sh` with `PATH="$ROOT/validator/.venv/bin:$PATH"`.
13. Replace or complete the unique active `## Final Clean Run` section. It must include machine-readable `Final validator commit` and `Final source commit` lines before prose, final package hashes for all four canonical packages, final summary counts, failures found, fixes applied, regression tests added, approved skips if any, and final matrix/proof/site artifact paths.
14. Restore a build-only `safe/debian/changelog` stamp to `FINAL_SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_06_final_clean_run_software_tester` | check | `impl_06_final_clean_run_and_report` | Verify final full validator run, proof, site verification, package lock, and report all agree. | Run from repo root: `python3 -m json.tool validator/artifacts/libvips-safe-final-port-lock.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-final/port/results/libvips/summary.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json >/dev/null`; `test -f validator/site/libvips-safe-final/site-data.json`; run a Python assertion that exactly one active `## Final Clean Run` heading exists and parses only that section; without approved skips, assert `validator/artifacts/libvips-safe-final/matrix-exit-code.txt` contains `0`, summary `failed == 0`, lock package list is canonical with `unported_original_packages == []`, lock matches `validator-overrides/libvips/*.deb`, lock commit/tag/release tag matches `Final source commit`, every result has override debs and no fallback, proof counts match summary, and site data contains final proof rows; with approved skips, assert `.work/validator-final-approved/approved-skip-manifest.json` matches the active final ids/counts, real `validator/` is clean, `validator/artifacts/libvips-safe-final-unmodified/` fails only documented ids, `validator/artifacts/libvips-safe-final/` passes with adjusted counts, and final lock/result package evidence is still canonical; for an unmodified clean run, render to `.work/final-site-render-check` from `validator/`, compare `site-data.json`, and run `bash scripts/verify-site.sh`; for an approved skip, render and verify from `.work/validator-final-approved/` using absolute final artifact/proof/site roots; finally run `test -z "$(git -C validator status --porcelain --untracked-files=no)"`. |
| `check_06_final_clean_run_senior_tester` | check | `impl_06_final_clean_run_and_report` | Final review of report traceability, test coverage, and git history. | Run from repo root: assert exactly one active `## Final Clean Run` heading and inspect only that bounded final section; `git log --oneline --decorate -n 12`; `git status --short --branch`; `rg -n '\\bunsafe\\b|todo!|unimplemented!|panic!\\(' safe/src || true`; verify the final section names validator commit, checks executed, failures found, fixes applied, approved skips, package hashes, and final artifact paths; confirm every failure row has a regression test or validator-bug justification; confirm any `unsafe`, `todo!`, `unimplemented!`, or `panic!` match is preexisting or explicitly justified, with no new `todo!` or `unimplemented!` left in `safe/src`. |

## Success Criteria

The final run is acceptable when the Phase 1 validator commit in `port` mode exits 0 for libvips, the final summary reports zero failed testcases, every official lock and result JSON proves all four canonical libvips packages were ported with `unported_original_packages: []`, proof generation succeeds with required casts, site rendering and verification succeed, and the report provides complete traceability. The only exception is a documented validator bug with an unmodified failing artifact, exact skipped testcase ids, a passing transient-skip artifact, adjusted counts, and proof/site evidence from the approved transient run.

## Git Commit Requirement

The implementer must commit work to git before yielding. Phase 6 must commit the final report evidence as `impl_06 record final validator clean run`; if blocked, it must commit the blocker report update before yielding failure.
