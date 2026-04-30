# Ruby Usage Operation Failures

**Phase Name:** Ruby Usage Operation Failures

**Implement Phase ID:** `impl_03_ruby_usage_operation_failures`

**Preexisting Inputs:**

- Phase 1 and phase 2 reports, active validator inventory, and failure ownership rows.
- Validator usage scripts under `validator/tests/libvips/tests/cases/usage/`.
- Operation dispatch in `safe/src/ops/mod.rs` and family modules under `safe/src/ops/*.rs`.
- Generated operation metadata under `safe/src/generated/**`.
- Pixel helpers under `safe/src/pixels/**` and runtime operation/object support under `safe/src/runtime/**`.
- Existing local tests `safe/tests/ops_core.rs`, `safe/tests/ops_advanced.rs`, `safe/tests/operation_registry.rs`, and `safe/tests/security.rs`.

**New Outputs:**

- Minimal regression tests for each operation failure.
- Safe operation fixes in `safe/src/ops/*.rs`, `safe/src/pixels/**`, or operation/object runtime when needed.
- Rebuilt packages, refreshed lock, and full rerun artifacts under `validator/artifacts/libvips-safe-ops/**`.
- Updated `validator-report.md`.
- A git commit for this phase.

**File Changes:**

- Add missing dispatch arms in the owning operation family and route from `safe/src/ops/mod.rs`.
- Preserve GLib/libvips varargs, output, metadata, demand hint, and reference semantics.
- Use existing `ImageBuffer` and `pixels::format` helpers before adding new pixel helpers.
- Do not edit validator usage scripts.

**Implementation Details:**

1. Identify failing usage cases owned by this phase from the required report table.
2. Map each Ruby call to the libvips nickname and generated wrapper metadata.
3. Treat sample-transform cases that load validator PNG/JPEG fixtures as operation failures by default. Move one to phase 4 only after documenting that the matching generated-image transform passes and I/O behavior is the real failure.
4. Add a Rust regression that calls the exported C ABI or operation wrapper with the same shape as the Ruby case, using generated in-memory images when possible.
5. Implement operation behavior with correct scalar/image inputs, bands, format promotion, clipping, offsets, metadata, and error returns.
6. Commit operation safe code and regression tests before rebuilding packages.
7. Rebuild packages, refresh lock, run the full validator into `validator/artifacts/libvips-safe-ops`, and update the report.
8. Commit the report update before yielding.

**Verification Phases:**

- `check_03_ruby_usage_operation_software_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_03_ruby_usage_operation_failures`
  - Purpose: validate generated-image and scalar/image operation behavior through local Rust tests and a full validator rerun.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features --test ops_core -- --nocapture
    cargo test --all-features --test ops_advanced -- --nocapture
    cargo test --all-features --test operation_registry -- --nocapture
    cargo test --all-features --test security -- --nocapture
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
    ARTIFACT_NAME=libvips-safe-ops
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
    export RESULT_ARTIFACT_NAME=libvips-safe-ops BLOCKING_OWNERS=impl_02_source_surface_failures:impl_03_ruby_usage_operation_failures LATER_OWNERS=impl_04_foreign_io_buffer_failures:impl_05_packaging_container_and_remaining_failures
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
- `check_03_ruby_usage_operation_senior_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_03_ruby_usage_operation_failures`
  - Purpose: review operation implementations for correctness, metadata/demand-style preservation, error behavior, and regression coverage.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features -- --nocapture
    rg -n "operation not implemented|impl_03_ruby_usage_operation_failures|usage-ruby-vips|Owner phase" "$ROOT/validator-report.md"
    rg -n "dispatch\\(|set_output_image|ensure_pixels|copy_metadata|VIPS_INTERESTING|VIPS_ANGLE|gravity|join|embed|subsample|zoom" src/ops tests
    ```

**Verification:**

- Focused operation/security tests from the software tester.
- Full validator command with `ARTIFACT_NAME=libvips-safe-ops`.
- No failed testcase owned by `impl_02_source_surface_failures` or `impl_03_ruby_usage_operation_failures` may remain.

**Success Criteria:**

- Every operation or Ruby usage failure owned by `impl_03_ruby_usage_operation_failures` is fixed or correctly re-owned with evidence.
- Focused operation, registry, and security tests pass.
- The full validator rerun under `validator/artifacts/libvips-safe-ops/**` has no remaining phase 2 or phase 3 owned failures.
- Sample-transform ownership remains operation-owned unless the report proves a loader/saver/source/target cause.
- Both verifier phases pass, including validator immutability checks.

**Git Commit Requirement:**

The implementer must commit this phase's work to git before yielding. If the phase makes no production changes, it must still commit a `validator-report.md` update or create an explicit `--allow-empty` commit documenting the no-op and the evidence checked. Generated validator artifacts, override packages, proof/site output, Docker outputs, Debian package outputs, and build directories must not be committed.
