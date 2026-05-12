# Phase 3. Operation Semantics

## Phase Name

Fix ruby-vips operation dispatch, argument handling, and pixel semantics failures

## Implement Phase ID

`impl_03_operation_semantics_failures`

## Preexisting Inputs

- Phase 1 baseline classification and Phase 2 report evidence in `validator-report.md`.
- Baseline and Phase 2 artifacts: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`, `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`, `validator/artifacts/libvips-safe-source-api-port-lock.json`, and `validator/artifacts/libvips-safe-source-api/port/results/libvips/*.json`.
- Updated Phase 1 validator checkout and venv: `validator/`, `validator/.venv/`.
- Operation implementation inputs: `safe/src/ops/mod.rs`, operation modules under `safe/src/ops/**`, `safe/src/runtime/operation.rs`, `safe/src/generated/operations_registry.rs`, `safe/src/generated/operation_wrappers.rs`, `safe/reference/operations.json`, and `safe/src/pixels/**`.
- Existing operation tests: `safe/tests/ops_core.rs`, `safe/tests/ops_advanced.rs`, `safe/tests/operation_registry.rs`, `safe/tests/security.rs`.
- Existing package and lock helpers: `scripts/check-layout.sh`, `scripts/build-debs.sh`, `scripts/lib/build_port_lock.py`, `packaging/package.env`.

## New Outputs

- Regression tests in `safe/tests/ops_core.rs` or `safe/tests/ops_advanced.rs`.
- Operation/runtime/pixel fixes in `safe/src/ops/**`, `safe/src/pixels/**`, or `safe/src/runtime/operation.rs` as needed.
- Fresh `dist/*.deb`, rewritten `validator-overrides/libvips/*.deb`, and full canonical `validator/artifacts/libvips-safe-ops-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-ops/` with `matrix-exit-code.txt`.
- Replaced unique active report section `## Phase 3 Operation Semantics Rerun`.
- Source commit `impl_03 fix operation validator failures` when fixes are needed, followed by report commit `impl_03 record operation validator rerun`. If there are zero owned failures, only `impl_03 record no operation failures` is required.

## File Changes

- Candidate files: `safe/src/ops/**`, `safe/src/pixels/**`, `safe/src/runtime/operation.rs`, `safe/tests/ops_core.rs`, `safe/tests/ops_advanced.rs`, and `safe/tests/operation_registry.rs`.
- Avoid editing generated files directly unless the established local generator path is used and documented.
- Do not modify tracked validator files.

## Implementation Details

1. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips` and `PHASE_START_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`.
2. Use the unique active Phase 1 baseline section to find failures assigned to `impl_03_operation_semantics_failures`.
3. If there are zero owned failures, do not edit `safe/**`; run focused operation tests, set `SOURCE_COMMIT="$PHASE_START_COMMIT"` and `SOURCE_FIX_COMMITS=none`, then run the official package/lock/validator evidence path below.
4. For each phase-3 failure, identify the libvips operation nickname from the validator log, ruby-vips stack trace, or generated wrapper name.
5. If the failure is `operation not implemented`, add the nickname to `SUPPORTED_OPERATIONS` only after adding a real implementation path.
6. Add a minimal C ABI style regression test that declares the relevant wrapper, builds small in-memory images with existing helpers or `vips_image_new_from_memory_copy`, and asserts return code, dimensions, bands, format, interpretation, offsets/demand hints, and pixel values.
7. Implement with existing operation helpers such as `get_image_buffer`, `get_int`, `get_double`, `get_enum`, `get_bool`, `get_array_double`, `set_output_image`, `set_output_image_like`, and metadata copy helpers.
8. Preserve libvips conventions: null outputs on failure, useful operation-domain errors, vararg defaults matching metadata, metadata/history preservation, format promotion and band handling through existing pixel helpers, and checked/saturating/clamped overflow where appropriate.
9. Run focused Rust tests. Do not create official evidence from uncommitted changes.
10. Commit source/test fixes before building official packages as `impl_03 fix operation validator failures`; this commit must not include `validator-report.md`, `validator/**`, `validator-overrides/**`, `dist/**`, or generated build artifacts.
11. Set `SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"` and compute `SOURCE_FIX_COMMITS` from `"$PHASE_START_COMMIT"..HEAD` over `safe`, `scripts`, `packaging`, and `tests`. For zero owned failures, keep `SOURCE_FIX_COMMITS=none`.
12. Run official evidence from committed source:

    ```bash
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
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
    ```

13. Assert the phase lock has the canonical package list, `unported_original_packages == []`, matching override sha256s/sizes, commit/tag/release tag matching `SOURCE_COMMIT`, and every testcase result has override debs installed with no original-package fallback.
14. Replace the unique active `## Phase 3 Operation Semantics Rerun` section. The section must begin with `Phase start commit`, `Source commit`, and `Source fix commits` machine-readable lines and include fixed operation names, regression tests, changed files, package hash table, matrix exit code path, matrix summary, and remaining failure ownership.
15. Restore a build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_03_operation_semantics_software_tester` | check | `impl_03_operation_semantics_failures` | Verify phase-3 owned operation failures are fixed, Rust operation tests pass, and a full rerun exists. | `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && cargo test --all-features --test ops_core --test ops_advanced --test operation_registry --test security -- --nocapture`; return to repo root; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; `python3 -m json.tool validator/artifacts/libvips-safe-ops-port-lock.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json >/dev/null`; `cat validator/artifacts/libvips-safe-ops/matrix-exit-code.txt`; run a Python assertion that exactly one active Phase 1 heading and one active Phase 3 heading exist, parses only those sections, proves the phase-3 lock commit/release tag matches the active phase-3 `SOURCE_COMMIT`, proves lock hashes/sizes match `validator-overrides/libvips/*.deb`, proves the canonical package list and empty `unported_original_packages`, proves every result used override debs with no fallback, and proves every baseline testcase assigned to this phase passed in `validator/artifacts/libvips-safe-ops/port/results/libvips/<id>.json` unless the active phase-3 section records zero owned failures. |
| `check_03_operation_semantics_senior_tester` | check | `impl_03_operation_semantics_failures` | Review operation fixes for libvips semantic compatibility, metadata propagation, ownership, and edge cases. | Run from repo root: assert exactly one active Phase 3 heading; parse `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from only that section and validate SHAs; `git log --oneline "$PHASE_START"..HEAD`; `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/src/ops safe/src/pixels safe/src/runtime/operation.rs safe/src/generated safe/tests/ops_core.rs safe/tests/ops_advanced.rs safe/tests/operation_registry.rs`; if fixes exist, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/src/ops safe/src/pixels safe/src/runtime/operation.rs safe/src/generated safe/tests/ops_core.rs safe/tests/ops_advanced.rs safe/tests/operation_registry.rs`; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; inspect changed operation/pixel/test files and confirm tests use the exported C ABI or the same operation wrapper path used by ruby-vips. |

## Success Criteria

Every baseline failure owned by phase 3 passes in the phase-3 validator artifact, or the report records zero phase-3 owned failures. Any new operation support has a focused regression test.

## Git Commit Requirement

The implementer must commit work to git before yielding. Source/test fixes must be committed before official package evidence, and the phase report must be committed after evidence is recorded.
