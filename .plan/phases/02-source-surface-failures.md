# Source Surface Failures

**Phase Name:** Source Surface Failures

**Implement Phase ID:** `impl_02_source_surface_failures`

**Preexisting Inputs:**

- Phase 1 report, active validator inventory, and baseline artifacts.
- Source testcase scripts under `validator/tests/libvips/tests/cases/source/`.
- Safe source-surface files in `safe/src/runtime/**`, `safe/src/abi/**`, `safe/include/vips/**`, `safe/build_support/**`, `safe/build.rs`, `safe/Cargo.toml`, `safe/meson.build`, `safe/meson_options.txt`, and `safe/debian/**`.
- Existing source-surface tests and scripts: `safe/tests/runtime_io.rs`, `safe/tests/init_version_smoke.rs`, `safe/tests/abi_layout.rs`, `safe/tests/introspection/**`, `safe/tests/link_compat/**`, `safe/scripts/check_introspection.sh`, `safe/scripts/link_compat.sh`, and `safe/tests/upstream/test_thumbnail.sh`.

**New Outputs:**

- Minimal regression tests in the existing source-surface test files or a new `safe/tests/validator_source.rs`.
- Source-surface fixes in safe runtime/ABI/header/build/package files.
- Rebuilt safe Debian packages and refreshed local lock.
- Full rerun artifacts under `validator/artifacts/libvips-safe-source/**`.
- Updated `validator-report.md` with root cause, regression test path, changed production files, rerun summary, and remaining failure ownership.
- A git commit for this phase.

**File Changes:**

- Use `safe/tests/runtime_io.rs` for CLI, header, source/target, thumbnail, and metadata reproductions.
- Use `safe/tests/abi_layout.rs`, `safe/tests/link_compat/**`, and `safe/scripts/link_compat.sh` for ABI and C compile/link failures.
- Use `safe/tests/introspection/**` and `safe/scripts/check_introspection.sh` for GIR failures.
- Fix runtime behavior in the owning safe modules, not in validator scripts.
- Edit packaging/build/header files only when validator evidence proves the installed surface is wrong.

**Implementation Details:**

1. Read the phase 1 JSON/logs for failures whose required ownership row names `impl_02_source_surface_failures` and reproduce each source-surface failure locally. Do not take ownership of source-kind failures assigned to `impl_04_foreign_io_buffer_failures`; leave those rows deferred for phase 4 unless the evidence proves the owner row is wrong, in which case update the row with the evidence.
2. Add one minimal regression test per distinct root cause before or alongside the fix.
3. Fix the real safe source issue without weakening existing surface comparisons or generated manifest checks.
4. Update the required failure table so fixed source rows become `fixed` and any remaining later-phase rows have exact owner phases.
5. Commit source-surface safe code, tests, and package/build changes that should be represented by rebuilt packages.
6. Rebuild packages from that commit, stage overrides, refresh the local lock, and run the full validator matrix into `validator/artifacts/libvips-safe-source`.
7. Update `validator-report.md` with evidence paths, rerun totals from inventory-derived counts, and remaining ownership.
8. Commit the report update before yielding.

**Verification Phases:**

- `check_02_source_surface_software_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_02_source_surface_failures`
  - Purpose: rerun focused local source-surface checks, rebuild packages, run the full validator matrix, and fail if source-owned failures remain.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features --test runtime_io -- --nocapture
    cargo test --all-features --test abi_layout -- --nocapture
    cargo test --all-features --test init_version_smoke -- --nocapture
    rm -rf "$PWD/.tmp/validator-source-prefix" "$PWD/.tmp/validator-source-link"
    meson setup build-validator-source . --wipe --prefix "$PWD/.tmp/validator-source-prefix"
    meson compile -C build-validator-source
    meson install -C build-validator-source
    scripts/link_compat.sh --manifest reference/objects/link-compat-manifest.json --reference-install "$ROOT/build-check-install" --build-check "$ROOT/build-check" --safe-prefix "$PWD/.tmp/validator-source-prefix" --workdir "$PWD/.tmp/validator-source-link"
    scripts/check_introspection.sh --lib-dir "$PWD/build-validator-source/lib" --typelib-dir "$PWD/build-validator-source" --expect-version 8.15.1
    dpkg-buildpackage -b -uc -us
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    mkdir -p "$ROOT/validator-overrides/libvips"
    rm -f "$ROOT/validator-overrides/libvips"/*.deb
    version=$(dpkg-parsechangelog -l "$ROOT/safe/debian/changelog" -SVersion)
    arch=$(dpkg-architecture -qDEB_HOST_ARCH)
    for package in libvips42t64 libvips-dev libvips-tools gir1.2-vips-8.0; do
      install -m 0644 "$ROOT/${package}_${version}_${arch}.deb" "$ROOT/validator-overrides/libvips/"
    done
    LOCK="$ROOT/validator/artifacts/libvips-safe-port-lock.json"
    SAFE_SOURCE_COMMIT=$(git -C "$ROOT" rev-parse HEAD)
    python3 - "$ROOT" "$LOCK" "$SAFE_SOURCE_COMMIT" <<'PY'
    import hashlib
    import json
    import subprocess
    import sys
    from pathlib import Path

    root = Path(sys.argv[1])
    lock_path = Path(sys.argv[2])
    commit = sys.argv[3].strip()
    canonical = ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]
    debs = []
    for package in canonical:
        matches = sorted((root / "validator-overrides/libvips").glob(f"{package}_*.deb"))
        if len(matches) != 1:
            raise SystemExit(f"expected exactly one staged deb for {package}, found {len(matches)}")
        path = matches[0]
        package_name = subprocess.check_output(["dpkg-deb", "--field", str(path), "Package"], text=True).strip()
        architecture = subprocess.check_output(["dpkg-deb", "--field", str(path), "Architecture"], text=True).strip()
        if package_name != package:
            raise SystemExit(f"unexpected package name for {path}: {package_name}")
        if architecture not in {"amd64", "all"}:
            raise SystemExit(f"unexpected architecture for {path}: {architecture}")
        data = path.read_bytes()
        debs.append({
            "package": package,
            "filename": path.name,
            "architecture": architecture,
            "sha256": hashlib.sha256(data).hexdigest(),
            "size": path.stat().st_size,
        })
    lock = {
        "schema_version": 1,
        "mode": "port-04-test",
        "generated_at": "1970-01-01T00:00:00Z",
        "source_config": "repositories.yml",
        "source_inventory": "local-validator-overrides",
        "libraries": [{
            "library": "libvips",
            "repository": "safelibs/port-libvips-local",
            "tag_ref": "refs/tags/libvips/local-validator",
            "commit": commit,
            "release_tag": f"build-{commit[:12]}",
            "debs": debs,
            "unported_original_packages": [],
        }],
    }
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_path.write_text(json.dumps(lock, indent=2) + "\n")
    PY
    ARTIFACT_NAME=libvips-safe-source
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    ARTIFACT_NAME=${ARTIFACT_NAME:?set ARTIFACT_NAME}
    cd "$ROOT/validator"
    VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
    if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
    rm -rf "artifacts/${ARTIFACT_NAME}"
    set +e
    PYTHON="$VALIDATOR_PY" RECORD_CASTS=1 bash test.sh \
      --config repositories.yml \
      --tests-root tests \
      --artifact-root "artifacts/${ARTIFACT_NAME}" \
      --mode port-04-test \
      --library libvips \
      --override-deb-root "$ROOT/validator-overrides" \
      --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-port-lock.json" \
      --record-casts
    MATRIX_EXIT=$?
    set -e
    mkdir -p "artifacts/${ARTIFACT_NAME}"
    printf '%s\n' "$MATRIX_EXIT" > "artifacts/${ARTIFACT_NAME}/matrix-exit-code.txt"
    if ! test -f "artifacts/${ARTIFACT_NAME}/port-04-test/results/libvips/summary.json"; then
      if test "$MATRIX_EXIT" -ne 0; then
        exit "$MATRIX_EXIT"
      fi
      echo "validator matrix produced no libvips summary for ${ARTIFACT_NAME}" >&2
      exit 1
    fi
    export RESULT_ARTIFACT_NAME=libvips-safe-source BLOCKING_OWNERS=impl_02_source_surface_failures LATER_OWNERS=impl_03_ruby_usage_operation_failures:impl_04_foreign_io_buffer_failures:impl_05_packaging_container_and_remaining_failures
    python3 - <<'PY'
    import json
    import os
    from pathlib import Path

    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inventory = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    report = (root / "validator-report.md").read_text()
    artifact = os.environ["RESULT_ARTIFACT_NAME"]
    blocking_owners = {item for item in os.environ.get("BLOCKING_OWNERS", "").split(":") if item}
    later_owners = {item for item in os.environ.get("LATER_OWNERS", "").split(":") if item}
    allowed_owners = {
        "impl_02_source_surface_failures",
        "impl_03_ruby_usage_operation_failures",
        "impl_04_foreign_io_buffer_failures",
        "impl_05_packaging_container_and_remaining_failures",
    }
    expected_header = ["Testcase ID", "Kind", "Status", "Owner phase", "First artifact", "Root cause", "Regression test", "Resolution"]
    allowed_kinds = {"source", "usage", "packaging-container"}
    allowed_statuses = {"open", "fixed", "deferred", "approved-skip", "not-a-defect"}
    owners = {}
    statuses = {}
    duplicates = []
    for line in report.splitlines():
        if not line.startswith("|"):
            continue
        columns = [part.strip() for part in line.strip().strip("|").split("|")]
        if columns == expected_header:
            continue
        if len(columns) != len(expected_header) or columns[0] in {"---", ""} or set(columns[0]) == {"-"}:
            continue
        testcase_id, kind, status, owner, first_artifact, root_cause, regression_test, resolution = columns
        owner = owner.strip("`")
        if testcase_id in owners:
            duplicates.append(testcase_id)
            continue
        if kind not in allowed_kinds:
            raise SystemExit(f"bad kind for {testcase_id}: {kind}")
        if status not in allowed_statuses:
            raise SystemExit(f"bad status for {testcase_id}: {status}")
        if owner not in allowed_owners:
            raise SystemExit(f"bad owner phase for {testcase_id}: {owner}")
        if testcase_id == "__packaging_container_setup__":
            if kind != "packaging-container":
                raise SystemExit(f"packaging/container row has wrong kind: {kind}")
        else:
            expected_kind = inventory["case_kinds"].get(testcase_id)
            if expected_kind is None:
                raise SystemExit(f"ownership row references unknown testcase: {testcase_id}")
            if kind != expected_kind:
                raise SystemExit(f"ownership kind mismatch for {testcase_id}: row={kind} manifest={expected_kind}")
        if not first_artifact or not root_cause or not regression_test or not resolution:
            raise SystemExit(f"ownership row has empty required field: {testcase_id}")
        owners[testcase_id] = owner
        statuses[testcase_id] = status
    if "| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |" not in report:
        raise SystemExit("validator-report.md is missing required failure ownership table")
    if duplicates:
        raise SystemExit(f"duplicate ownership rows: {duplicates}")

    result_dir = root / f"validator/artifacts/{artifact}/port-04-test/results/libvips"
    summary = json.loads((result_dir / "summary.json").read_text())
    assert summary["source_cases"] == inventory["source_cases"], summary
    assert summary["usage_cases"] == inventory["usage_cases"], summary
    assert summary["cases"] == inventory["total_cases"], summary
    allow_install_blocked = os.environ.get("ALLOW_INSTALL_BLOCKED", "1") != "0"

    blocking = []
    unowned = []
    install_blocked = []
    for path in sorted(result_dir.glob("*.json")):
        if path.name == "summary.json":
            continue
        payload = json.loads(path.read_text())
        testcase_id = payload["testcase_id"]
        if payload.get("status") == "passed":
            continue
        if payload.get("override_debs_installed") is not True:
            install_blocked.append(testcase_id)
            continue
        owner = owners.get(testcase_id)
        if owner in blocking_owners:
            blocking.append((testcase_id, owner))
        elif owner in later_owners:
            continue
        else:
            unowned.append(testcase_id)
    if install_blocked:
        if not allow_install_blocked:
            raise SystemExit(f"override package or container setup failures remain: {install_blocked}")
        owner = owners.get("__packaging_container_setup__")
        if owner != "impl_05_packaging_container_and_remaining_failures":
            raise SystemExit(f"missing packaging/container owner row for install-blocked results: {install_blocked}")
    matrix_exit_path = root / f"validator/artifacts/{artifact}/matrix-exit-code.txt"
    if not matrix_exit_path.is_file():
        raise SystemExit(f"missing matrix exit code for {artifact}: {matrix_exit_path}")
    matrix_exit = int(matrix_exit_path.read_text().strip())
    if matrix_exit != 0 and not install_blocked:
        if not allow_install_blocked:
            raise SystemExit(f"validator matrix exited {matrix_exit} without install-blocked testcase results")
        owner = owners.get("__packaging_container_setup__")
        if owner != "impl_05_packaging_container_and_remaining_failures":
            raise SystemExit(f"missing packaging/container owner row for nonzero matrix exit {matrix_exit}")
    if blocking:
        raise SystemExit(f"blocking owner failures remain: {blocking}")
    if unowned:
        raise SystemExit(f"unowned failures remain: {unowned}")
    PY
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    ```
- `check_02_source_surface_senior_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_02_source_surface_failures`
  - Purpose: review source-surface fixes for ABI/header/pkg-config/GIR/package correctness, regression coverage, and proper ownership for anything still failing.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features -- --nocapture
    rg -n "impl_02_source_surface_failures|Testcase ID|Owner phase" "$ROOT/validator-report.md"
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    ```

**Verification:**

- Focused Rust, Meson, link, and GIR commands from the software tester.
- Full validator command with `ARTIFACT_NAME=libvips-safe-source`.
- No failed testcase owned by `impl_02_source_surface_failures` may remain.

**Success Criteria:**

- Every failure owned by `impl_02_source_surface_failures` is fixed or correctly re-owned with evidence in `validator-report.md`.
- Focused ABI, runtime, Meson install, link compatibility, and introspection checks pass.
- The full validator rerun under `validator/artifacts/libvips-safe-source/**` has no remaining source-surface-owned failures.
- Remaining failures, if any, have exact later owner phase IDs in the report table.
- Both verifier phases pass, including validator immutability checks.

**Git Commit Requirement:**

The implementer must commit this phase's work to git before yielding. If the phase makes no production changes, it must still commit a `validator-report.md` update or create an explicit `--allow-empty` commit documenting the no-op and the evidence checked. Generated validator artifacts, override packages, proof/site output, Docker outputs, Debian package outputs, and build directories must not be committed.
