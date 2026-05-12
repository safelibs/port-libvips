# Phase 4. Foreign I/O Media

## Phase Name

Fix loaders, savers, buffers, sources, targets, thumbnails, and media materialization failures

## Implement Phase ID

`impl_04_foreign_io_media_failures`

## Preexisting Inputs

- Phase 1 baseline classification and prior phase report evidence in `validator-report.md`.
- Baseline, Phase 2, and Phase 3 artifacts: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`, `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/*.json`, `validator/artifacts/libvips-safe-source-api-port-lock.json`, `validator/artifacts/libvips-safe-source-api/port/results/libvips/*.json`, `validator/artifacts/libvips-safe-ops-port-lock.json`, and `validator/artifacts/libvips-safe-ops/port/results/libvips/*.json`.
- Updated Phase 1 validator checkout and venv: `validator/`, `validator/.venv/`.
- Foreign I/O implementation inputs: `safe/src/foreign/base.rs`, `safe/src/foreign/mod.rs`, `safe/src/foreign/sniff.rs`, `safe/src/foreign/loaders/**`, `safe/src/foreign/savers/**`.
- Runtime I/O inputs: `safe/src/runtime/image.rs`, `safe/src/runtime/source.rs`, `safe/src/runtime/target.rs`, `safe/src/runtime/connection.rs`, `safe/src/runtime/buf.rs`, `safe/src/runtime/dbuf.rs`, `safe/src/runtime/memory.rs`.
- Rust dependency inputs: `safe/Cargo.toml`, `safe/Cargo.lock`.
- Existing tests and samples: `safe/tests/runtime_io.rs`, `safe/tests/threading.rs`, `safe/tests/security.rs`, `original/test/test-suite/images/**`, `safe/tests/upstream/**`.
- Existing package and lock helpers: `scripts/check-layout.sh`, `scripts/build-debs.sh`, `scripts/lib/build_port_lock.py`, `packaging/package.env`.

## New Outputs

- Regression tests in `safe/tests/runtime_io.rs`, `safe/tests/threading.rs`, or `safe/tests/security.rs`.
- Loader/saver/source/target/runtime fixes.
- Fresh `dist/*.deb`, rewritten `validator-overrides/libvips/*.deb`, and full canonical `validator/artifacts/libvips-safe-foreign-port-lock.json`.
- Full rerun artifact `validator/artifacts/libvips-safe-foreign/` with `matrix-exit-code.txt`.
- Replaced unique active report section `## Phase 4 Foreign I/O And Media Rerun`.
- Source commit `impl_04 fix foreign io validator failures` when fixes are needed, followed by report commit `impl_04 record foreign io validator rerun`. If there are zero owned failures, only `impl_04 record no foreign io failures` is required.

## File Changes

- Candidate files: `safe/src/foreign/**`, `safe/src/runtime/image.rs`, `safe/src/runtime/source.rs`, `safe/src/runtime/target.rs`, `safe/src/runtime/connection.rs`, `safe/src/runtime/buf.rs`, `safe/src/runtime/dbuf.rs`, `safe/src/runtime/memory.rs`, `safe/Cargo.toml`, `safe/Cargo.lock`, `safe/tests/runtime_io.rs`, `safe/tests/threading.rs`, and `safe/tests/security.rs`.
- Do not modify tracked validator files.

## Implementation Details

1. Set `ROOT=/home/yans/safelibs/pipeline/ports/port-libvips` and `PHASE_START_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"`.
2. Use the unique active Phase 1 baseline section to find failures assigned to `impl_04_foreign_io_media_failures`.
3. If there are zero owned failures, do not edit `safe/**`; run focused runtime/media tests, set `SOURCE_COMMIT="$PHASE_START_COMMIT"` and `SOURCE_FIX_COMMITS=none`, then run the official package/lock/validator evidence path below.
4. For each phase-4 failure, determine whether it involves file, buffer, source, target, thumbnail, CLI, ruby-vips `write_to_buffer`, ruby-vips `new_from_buffer`, or lazy pixel materialization.
5. Add regression coverage through exported C ABI paths such as `vips_image_new_from_file`, `vips_image_new_from_buffer`, `vips_image_new_from_source`, `vips_image_write_to_file`, `vips_image_write_to_buffer`, `vips_image_write_to_target`, format-specific wrappers, or `vips_thumbnail`.
6. Fix the safe implementation while preserving upstream behavior: use `PendingDecode` and `ensure_pixels` consistently, keep `file_load_cache` invalidation correct, preserve `VipsBlob`/`VipsArea` and GLib-owned buffer semantics, honor `fail_on`, prefer native Rust loaders already in the tree, and set expected metadata including loader, filename, history, bands, dimensions, interpretation, and resolution.
7. Run runtime I/O, threading, security, upstream shell, and fuzz wrapper tests. Do not create official evidence from uncommitted changes.
8. Commit source/test fixes before building official packages as `impl_04 fix foreign io validator failures`; this commit must not include `validator-report.md`, `validator/**`, `validator-overrides/**`, `dist/**`, or generated build artifacts.
9. Set `SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"` and compute `SOURCE_FIX_COMMITS` from `"$PHASE_START_COMMIT"..HEAD` over `safe`, `scripts`, `packaging`, and `tests`. For zero owned failures, keep `SOURCE_FIX_COMMITS=none`.
10. Run official evidence from committed source:

    ```bash
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
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
    ```

11. Assert the phase lock has the canonical package list, `unported_original_packages == []`, matching override sha256s/sizes, commit/tag/release tag matching `SOURCE_COMMIT`, and every testcase result has override debs installed with no original-package fallback.
12. Replace the unique active `## Phase 4 Foreign I/O And Media Rerun` section. The section must begin with `Phase start commit`, `Source commit`, and `Source fix commits` machine-readable lines and include exact media paths fixed, package hash table, matrix exit code path, matrix summary, and remaining failures.
13. Restore a build-only `safe/debian/changelog` stamp to `SOURCE_COMMIT` if it is the only tracked build artifact left.

## Verification Phases

| Phase ID | Type | Fixed `bounce_target` | Purpose | Commands |
| --- | --- | --- | --- | --- |
| `check_04_foreign_io_media_software_tester` | check | `impl_04_foreign_io_media_failures` | Verify media/I/O failures are fixed, runtime I/O tests pass, and a full rerun exists. | `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && cargo test --all-features --test runtime_io --test threading --test security -- --nocapture`; `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && rm -rf build-validator-foreign && meson setup build-validator-foreign . --prefix "$PWD/.tmp/validator-foreign-prefix"`; `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && meson compile -C build-validator-foreign`; `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && tests/upstream/run-shell-suite.sh build-validator-foreign`; `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && tests/upstream/run-fuzz-suite.sh build-validator-foreign`; return to repo root; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; `python3 -m json.tool validator/artifacts/libvips-safe-foreign-port-lock.json >/dev/null`; `python3 -m json.tool validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json >/dev/null`; `cat validator/artifacts/libvips-safe-foreign/matrix-exit-code.txt`; run a Python assertion that exactly one active Phase 1 heading and one active Phase 4 heading exist, parses only those sections, proves the phase-4 lock commit/release tag matches active phase-4 `SOURCE_COMMIT`, proves lock hashes/sizes match `validator-overrides/libvips/*.deb`, proves the canonical package list and empty `unported_original_packages`, proves every result used override debs with no fallback, and proves every baseline testcase assigned to this phase passed in `validator/artifacts/libvips-safe-foreign/port/results/libvips/<id>.json` unless the active phase-4 section records zero owned failures. |
| `check_04_foreign_io_media_senior_tester` | check | `impl_04_foreign_io_media_failures` | Review ownership and safety at the C ABI/GLib boundary for media fixes. | Run from repo root: assert exactly one active Phase 4 heading; parse `PHASE_START`, `SOURCE_COMMIT`, and `SOURCE_FIX_COMMITS` from only that section and validate SHAs; `git log --oneline "$PHASE_START"..HEAD`; `git diff --stat "$PHASE_START"..HEAD -- validator-report.md safe/src/foreign safe/src/runtime/image.rs safe/src/runtime/source.rs safe/src/runtime/target.rs safe/src/runtime/connection.rs safe/src/runtime/buf.rs safe/src/runtime/dbuf.rs safe/src/runtime/memory.rs safe/Cargo.toml safe/Cargo.lock safe/tests/runtime_io.rs safe/tests/threading.rs safe/tests/security.rs`; if fixes exist, run `git show --stat --oneline $SOURCE_FIX_COMMITS` and `git diff "$PHASE_START".."$SOURCE_COMMIT" -- safe/src/foreign safe/src/runtime/image.rs safe/src/runtime/source.rs safe/src/runtime/target.rs safe/src/runtime/connection.rs safe/src/runtime/buf.rs safe/src/runtime/dbuf.rs safe/src/runtime/memory.rs safe/Cargo.toml safe/Cargo.lock safe/tests/runtime_io.rs safe/tests/threading.rs safe/tests/security.rs`; `test -z "$(git -C validator status --porcelain --untracked-files=no)"`; inspect changed foreign/runtime/test files, confirm GLib-compatible ownership for returned buffers, and confirm validator tests were not edited. |

## Success Criteria

Every baseline failure owned by phase 4 passes in the phase-4 validator artifact, or the report records zero phase-4 owned failures. Runtime tests cover ownership and materialization behavior.

## Git Commit Requirement

The implementer must commit work to git before yielding. Source/test fixes must be committed before official package evidence, and the phase report must be committed after evidence is recorded.
