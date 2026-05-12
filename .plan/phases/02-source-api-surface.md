# Phase 2. Source API Surface

## Phase Name

Fix source-facing ABI, headers, metadata, pkg-config, and introspection failures

## Implement Phase ID

`impl_02_source_api_surface_failures`

## Preexisting Inputs

- Phase 1 active baseline classification in `validator-report.md`.
- Baseline results and logs under `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/` and `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/`.
- Updated Phase 1 validator checkout and venv: `validator/`, `validator/.venv/`.
- Source/API implementation inputs: `safe/include/vips/**`, `safe/src/abi/**`, `safe/src/runtime/{init,type,object,image,operation,header,error,buf,sbuf}.rs`.
- Build/install inputs: `safe/build.rs`, `safe/meson.build`, `safe/build_support/vips.pc.in`, `safe/build_support/vips-cpp.pc.in`, `safe/debian/control`, `safe/debian/rules`, and `safe/debian/*.install`.
- Existing regression tests: `safe/tests/abi_layout.rs`, `safe/tests/init_version_smoke.rs`, `safe/tests/operation_registry.rs`, `safe/tests/runtime_io.rs`, `safe/tests/introspection/gir_smoke.c`.
- Existing reference artifacts to consume in place: `original/**`, `safe/reference/**`, `safe/vendor/pyvips-3.1.1/**`.
- Existing package and lock helpers: `scripts/check-layout.sh`, `scripts/build-debs.sh`, `scripts/lib/build_port_lock.py`, `packaging/package.env`.

## New Outputs

- Minimal regression tests for each phase-2 owned failure.
- Source/API/package fixes in `safe/**` as needed.
- Fresh `dist/*.deb`, rewritten `validator-overrides/libvips/*.deb`, and full canonical `validator/artifacts/libvips-safe-source-api-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-source-api/` with `matrix-exit-code.txt`.
- Replaced unique active report section `## Phase 2 Source API Surface Rerun`.
- Source commit `impl_02 fix source api validator failures` when fixes are needed, followed by report commit `impl_02 record source api validator rerun`. If there are zero owned failures, only `impl_02 record no source api failures` is required.

## File Changes

- Candidate files: `safe/src/abi/**`, `safe/src/runtime/**`, `safe/include/vips/**`, `safe/build.rs`, `safe/meson.build`, `safe/build_support/*.pc.in`, `safe/debian/**`, `safe/tests/abi_layout.rs`, `safe/tests/init_version_smoke.rs`, `safe/tests/operation_registry.rs`, `safe/tests/runtime_io.rs`, and `safe/tests/introspection/gir_smoke.c`.
- Do not modify `validator/tests/**` or other tracked validator source files.

## Implementation Details

1. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips` and `PHASE_START_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`.
2. Use the unique active Phase 1 baseline section to find failures assigned to `impl_02_source_api_surface_failures`.
3. If there are zero owned failures, do not edit `safe/**`; run focused source-surface tests, set `SOURCE_COMMIT="$PHASE_START_COMMIT"` and `SOURCE_FIX_COMMITS=none`, then run the official package/lock/validator evidence path below.
4. For each owned failure, reproduce from the validator log and source testcase path under `validator/tests/libvips/tests/cases/source/`.
5. Add focused regression coverage before production changes:
   - ABI/layout issues: `safe/tests/abi_layout.rs`.
   - init/version/type registry issues: `safe/tests/init_version_smoke.rs`.
   - operation metadata/type registry issues: `safe/tests/operation_registry.rs`.
   - image metadata/header issues: `safe/tests/runtime_io.rs`.
   - GIR/typelib issues: `safe/tests/introspection/gir_smoke.c` or the existing introspection harness.
6. Fix the underlying source/API issue using the reference inputs already present in the workspace: headers under `safe/include/vips`, ABI declarations under `safe/src/abi`, runtime type/object/image/header behavior, `safe/build.rs`, `safe/meson.build`, pkg-config templates, or Debian install metadata.
7. Run focused tests. Do not create official evidence from uncommitted changes.
8. Commit source/test/package fixes before building official packages as `impl_02 fix source api validator failures`; this commit must not include `validator-report.md`, `validator/**`, `validator-overrides/**`, `dist/**`, or generated build artifacts.
9. Set `SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"` and compute `SOURCE_FIX_COMMITS` from `"$PHASE_START_COMMIT"..HEAD` over `safe`, `scripts`, `packaging`, and `tests`. For zero owned failures, keep `SOURCE_FIX_COMMITS=none`.
10. Run official evidence from committed source:

    ```bash
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    SOURCE_API_LOCK="$ROOT/validator/artifacts/libvips-safe-source-api-port-lock.json"
    SOURCE_API_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-source-api"
    cd "$ROOT"
    bash scripts/check-layout.sh
    SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" bash scripts/build-debs.sh
    rm -rf "$ROOT/validator-overrides/libvips"
    mkdir -p "$ROOT/validator-overrides"
    SAFELIBS_LIBRARY=libvips \
    SAFELIBS_COMMIT_SHA="$SOURCE_COMMIT" \
    SAFELIBS_DIST_DIR="$ROOT/dist" \
    SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
    SAFELIBS_LOCK_PATH="$SOURCE_API_LOCK" \
    SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
      python3 "$ROOT/scripts/lib/build_port_lock.py"
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
    ```

11. Assert the phase lock has the canonical package list `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`, `unported_original_packages == []`, matching override sha256s/sizes, commit/tag/release tag matching `SOURCE_COMMIT`, and every testcase result has override debs installed with no original-package fallback.
12. Replace the unique active `## Phase 2 Source API Surface Rerun` section. The section must begin with `Phase start commit`, `Source commit`, and `Source fix commits` machine-readable lines and include fixed testcase ids, regression tests, changed files, remaining failures, package hash table, matrix exit code path, and matrix summary.
13. Restore a build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_02_source_api_surface_software_tester` | check | `impl_02_source_api_surface_failures` | Verify phase-2 owned failures are fixed or explicitly absent, regression tests pass, and a full rerun exists. | Run from repo root: `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; `bash scripts/check-layout.sh`; `cd safe && cargo test --all-features --test abi_layout --test init_version_smoke --test operation_registry --test runtime_io -- --nocapture`; return to repo root; `python3 -m json.tool validator/artifacts/libvips-safe-source-api-port-lock.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json >/dev/null`; `cat validator/artifacts/libvips-safe-source-api/matrix-exit-code.txt`; run a Python assertion that exactly one active Phase 1 heading and one active Phase 2 heading exist, parses only those sections, proves the phase-2 lock commit/release tag matches the active phase-2 `SOURCE_COMMIT`, proves lock hashes/sizes match `validator-overrides/libvips/*.deb`, proves the canonical package list and empty `unported_original_packages`, proves every result used override debs with no fallback, and proves every baseline testcase assigned to this phase passed in `validator/artifacts/libvips-safe-source-api/port/results/libvips/<id>.json` unless the active phase-2 section records zero owned failures. |
| `check_02_source_api_surface_senior_tester` | check | `impl_02_source_api_surface_failures` | Review source-surface fixes for ABI compatibility and ensure no validator changes were used. | Run from repo root: assert exactly one active Phase 2 heading; parse `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from only that section and validate SHAs; `git log --oneline "$PHASE_START"..HEAD`; `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/include/vips safe/src/abi safe/src/runtime safe/build.rs safe/meson.build safe/debian safe/tests scripts packaging`; if fixes exist, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/include/vips safe/src/abi safe/src/runtime safe/build.rs safe/meson.build safe/debian safe/tests scripts packaging`; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; inspect changed source/API/package files against the original public surface. |

## Success Criteria

Every baseline failure owned by `impl_02_source_api_surface_failures` passes in the phase-2 validator artifact, or the report records that there were zero phase-2 owned failures. New fixes have focused regression tests and validator source remains clean.

## Git Commit Requirement

The implementer must commit work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the phase report must be committed after evidence is recorded.
