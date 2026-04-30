# Validator Bootstrap, Package Staging, And Baseline Run

**Phase Name:** Validator Bootstrap, Package Staging, And Baseline Run

**Implement Phase ID:** `impl_01_validator_bootstrap_baseline`

**Preexisting Inputs:**

- `original/`, `safe/`, `build-check/`, `build-check-install/`.
- Prepared JSON inventories, harnesses, reference artifacts, and existing validator artifacts.
- Existing `validator/` checkout if present, otherwise no validator checkout.
- Existing `validator-overrides/libvips/*.deb` and `validator-report.md` if present.
- Current untracked `.gitignore`, `safe/.tmp/`, `safe/src/foreign/loaders/jpeg.rs`, and `validator-report.md`.

**New Outputs:**

- Updated or cloned validator checkout, with active commit selected by manifest usability.
- `validator/artifacts/libvips-safe-validator-selection.txt`.
- `validator/artifacts/libvips-safe-inventory.json`.
- `validator/.venv/` with PyYAML if host Python cannot import `yaml`.
- Freshly rebuilt `validator-overrides/libvips/*.deb` from the committed safe source state.
- Fresh `validator/artifacts/libvips-safe-port-lock.json`.
- Baseline full validator artifacts under `validator/artifacts/libvips-safe/**`, including `matrix-exit-code.txt`.
- Updated `validator-report.md` with validator URL, remote main commit, active commit, active selection reason, package inputs, manifest counts, baseline command, result summary, and failure ownership table.
- Root `.gitignore` entries for generated artifacts when required.

**File Changes:**

- Create or update `validator-report.md`.
- Create or update root `.gitignore` only if generated artifacts are not already ignored.
- Do not edit validator tracked files.
- Do not edit `safe/**` unless a trivial build/package blocker prevents any validator execution; if so, add a minimal regression/build check and document it in `validator-report.md`.

**Implementation Details:**

1. Preserve dirty worktree state. Do not delete untracked safe files or generated artifacts.
2. Clone or update the validator:
   ```bash
   ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
   VALIDATOR_URL=https://github.com/safelibs/validator
   FALLBACK_COMMIT=1319bb0374ef66428a42dd71e49553c6d057feaf
   if test -d "$ROOT/validator/.git"; then
     git -C "$ROOT/validator" fetch origin main
     if git -C "$ROOT/validator" symbolic-ref -q HEAD >/dev/null; then
       git -C "$ROOT/validator" pull --ff-only
     else
       git -C "$ROOT/validator" checkout -B main origin/main
     fi
   else
     git clone "$VALIDATOR_URL" "$ROOT/validator"
   fi
   ```
3. Inspect `validator/tests/libvips/testcases.yml` after update. If it has a non-empty valid `testcases` list, keep that active checkout and write selection reason `updated validator manifest is runnable`.
4. If the updated manifest is empty or invalid, check out `1319bb0374ef66428a42dd71e49553c6d057feaf`, verify it has a non-empty valid libvips manifest, and write selection reason `remote main manifest unusable for libvips; using last known runnable validator commit`. Record both the remote main commit and active commit in `validator-report.md`.
5. If neither checkout has a runnable libvips manifest, update `validator-report.md` with a bootstrap blocker and commit that report; do not continue to package, validator, proof, or later implementation phases.
6. Read `validator/README.md`, `validator/repositories.yml`, `validator/tests/libvips/testcases.yml`, and `validator/test.sh`. Record that validation uses installed `.deb` packages rather than a raw library path.
7. Ensure validator Python can import PyYAML. Use host `python3` when it works; otherwise create `validator/.venv/` and install `PyYAML` there.
8. Create `validator/artifacts/libvips-safe-inventory.json` with this command:
   ```bash
   set -euo pipefail
   ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
   VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
   if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
   "$VALIDATOR_PY" - "$ROOT" <<'PY'
   import hashlib
   import json
   import subprocess
   import sys
   from pathlib import Path

   import yaml

   root = Path(sys.argv[1])
   validator = root / "validator"
   manifest_path = validator / "tests/libvips/testcases.yml"
   payload = yaml.safe_load(manifest_path.read_text())
   if not isinstance(payload, dict):
       raise SystemExit("libvips testcase manifest is not a mapping")
   cases = payload.get("testcases")
   if not isinstance(cases, list) or not cases:
       raise SystemExit("active libvips testcase manifest has zero runnable testcases")
   source_ids = []
   usage_ids = []
   case_kinds = {}
   for case in cases:
       if not isinstance(case, dict):
           raise SystemExit("testcase entry is not a mapping")
       testcase_id = case.get("id")
       kind = case.get("kind")
       if not isinstance(testcase_id, str) or not testcase_id:
           raise SystemExit("testcase id is missing")
       if kind == "source":
           source_ids.append(testcase_id)
       elif kind == "usage":
           usage_ids.append(testcase_id)
       else:
           raise SystemExit(f"unexpected testcase kind for {testcase_id}: {kind!r}")
       if testcase_id in case_kinds:
           raise SystemExit(f"duplicate testcase id: {testcase_id}")
       case_kinds[testcase_id] = kind
   if len(source_ids) + len(usage_ids) != len(cases):
       raise SystemExit("source plus usage count does not equal total case count")
   active_commit = subprocess.check_output(["git", "-C", str(validator), "rev-parse", "HEAD"], text=True).strip()
   try:
       remote_main = subprocess.check_output(["git", "-C", str(validator), "rev-parse", "origin/main"], text=True).strip()
   except subprocess.CalledProcessError:
       remote_main = ""
   reason_path = validator / "artifacts/libvips-safe-validator-selection.txt"
   active_reason = reason_path.read_text().strip() if reason_path.is_file() else "active checkout selected by phase 1"
   inventory = {
       "validator_url": "https://github.com/safelibs/validator",
       "remote_main_commit": remote_main,
       "active_validator_commit": active_commit,
       "active_validator_reason": active_reason,
       "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
       "source_cases": len(source_ids),
       "usage_cases": len(usage_ids),
       "total_cases": len(cases),
       "source_ids": source_ids,
       "usage_ids": usage_ids,
       "case_kinds": case_kinds,
   }
   out = validator / "artifacts/libvips-safe-inventory.json"
   out.parent.mkdir(parents=True, exist_ok=True)
   out.write_text(json.dumps(inventory, indent=2) + "\n")
   print(json.dumps(inventory, indent=2))
   PY
   ```
9. Rebuild safe Debian packages from the committed safe source state; do not reuse existing root or override `.deb` files. If no safe changes are needed before the build, the package-source commit is the current `HEAD`. If a trivial build/package blocker requires a safe fix, commit that fix first, then run `cd "$ROOT/safe" && dpkg-buildpackage -b -uc -us`.
10. Stage exactly `libvips42t64`, `libvips-dev`, `libvips-tools`, and `gir1.2-vips-8.0`; refresh `validator/artifacts/libvips-safe-port-lock.json` with this command after committing the package-source state:
   ```bash
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
   ```
11. Run the full validator suite into `validator/artifacts/libvips-safe` with this command:
   ```bash
   ARTIFACT_NAME=libvips-safe
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
   ```
12. Parse every JSON under `validator/artifacts/libvips-safe/port-04-test/results/libvips/` and classify failures in the required report table:
    - Source CLI/API/GIR/package identity, header formatting, and metadata value failures that do not depend on image loader/saver/materialization behavior: `impl_02_source_surface_failures`.
    - Generated-image `ruby-vips` operation behavior failures: `impl_03_ruby_usage_operation_failures`.
    - Any source or usage failure caused by file/buffer/source/target APIs, PNG/JPEG loader/saver behavior, lazy materialization, external decoder fallback, or memory ownership: `impl_04_foreign_io_buffer_failures`.
    - Docker package install failures, override package issues, proof/site issues, validator bugs, or uncategorized leftovers: `impl_05_packaging_container_and_remaining_failures`. Real validator testcase bugs still keep their manifest `source` or `usage` kind in the table; use `approved-skip` status only after phase 5 creates and verifies the approved-skip artifacts.
    - For source cases such as `vips-cli-load-save`, `thumbnail-behavior`, and `metadata-header-checks`, assign ownership from the root cause: installed command/header/package surface defects go to phase 2, while decode, encode, file I/O, or materialization defects go to phase 4.
13. Commit `validator-report.md` and `.gitignore` if changed before yielding. If the phase was a no-op because all inputs already existed, make a report update or an `--allow-empty` commit documenting that phase 1 verified existing artifacts.

**Verification Phases:**

- `check_01_validator_bootstrap_software_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_01_validator_bootstrap_baseline`
  - Purpose: verify validator checkout selection, manifest inventory, Python setup, staged packages, local lock, baseline matrix artifacts, and initial report.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    test -d "$ROOT/validator/.git"
    test -f "$ROOT/validator/README.md"
    test -f "$ROOT/validator/repositories.yml"
    test -f "$ROOT/validator/tests/libvips/testcases.yml"
    test -f "$ROOT/validator/artifacts/libvips-safe-inventory.json"
    VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
    if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
    "$VALIDATOR_PY" -c 'import yaml'
    cd "$ROOT/validator"
    PYTHON="$VALIDATOR_PY" make unit
    "$VALIDATOR_PY" - <<'PY'
    import json
    import subprocess
    from pathlib import Path
    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inv = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    assert inv["total_cases"] > 0, inv
    assert inv["source_cases"] + inv["usage_cases"] == inv["total_cases"], inv
    assert len(inv["source_ids"]) == inv["source_cases"], inv
    assert len(inv["usage_ids"]) == inv["usage_cases"], inv
    active = subprocess.check_output(["git", "-C", str(root / "validator"), "rev-parse", "HEAD"], text=True).strip()
    assert active == inv["active_validator_commit"], (active, inv)
    PY
    SOURCE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["source_cases"])')
    USAGE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["usage_cases"])')
    TOTAL_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["total_cases"])')
    "$VALIDATOR_PY" tools/testcases.py --config repositories.yml --tests-root tests --library libvips --check --min-source-cases "$SOURCE_CASES" --min-usage-cases "$USAGE_CASES" --min-cases "$TOTAL_CASES"
    test -d "$ROOT/validator-overrides/libvips"
    for package in libvips42t64 libvips-dev libvips-tools gir1.2-vips-8.0; do
      test "$(find "$ROOT/validator-overrides/libvips" -maxdepth 1 -type f -name "${package}_*.deb" | wc -l)" -eq 1
    done
    test -f "$ROOT/validator/artifacts/libvips-safe-port-lock.json"
    test -f "$ROOT/validator/artifacts/libvips-safe/matrix-exit-code.txt"
    test -f "$ROOT/validator/artifacts/libvips-safe/port-04-test/results/libvips/summary.json"
    "$VALIDATOR_PY" - <<'PY'
    import json
    import subprocess
    from pathlib import Path
    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inv = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    lock = json.loads((root / "validator/artifacts/libvips-safe-port-lock.json").read_text())
    assert lock["mode"] == "port-04-test"
    entry = lock["libraries"][0]
    assert entry["library"] == "libvips"
    assert [d["package"] for d in entry["debs"]] == ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]
    assert entry["unported_original_packages"] == []
    assert entry["release_tag"] == f"build-{entry['commit'][:12]}", entry
    subprocess.check_call(["git", "-C", str(root), "cat-file", "-e", f"{entry['commit']}^{{commit}}"])
    summary = json.loads((root / "validator/artifacts/libvips-safe/port-04-test/results/libvips/summary.json").read_text())
    assert summary["mode"] == "port-04-test"
    assert summary["source_cases"] == inv["source_cases"], summary
    assert summary["usage_cases"] == inv["usage_cases"], summary
    assert summary["cases"] == inv["total_cases"], summary
    report = (root / "validator-report.md").read_text()
    assert entry["commit"] in report, "validator-report.md must record the package-source commit"
    assert "dpkg-buildpackage -b -uc -us" in report, "validator-report.md must record the package rebuild command"
    results = [p for p in (root / "validator/artifacts/libvips-safe/port-04-test/results/libvips").glob("*.json") if p.name != "summary.json"]
    install_blocked = [json.loads(p.read_text())["testcase_id"] for p in results if json.loads(p.read_text()).get("override_debs_installed") is not True]
    matrix_exit = int((root / "validator/artifacts/libvips-safe/matrix-exit-code.txt").read_text().strip())
    if install_blocked or matrix_exit != 0:
        assert summary["casts"] <= inv["total_cases"], summary
        assert "__packaging_container_setup__" in report, (install_blocked, matrix_exit)
        assert "impl_05_packaging_container_and_remaining_failures" in report, report
    else:
        assert summary["casts"] == inv["total_cases"], summary
    PY
    python3 - <<'PY'
    import json
    from pathlib import Path

    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inventory = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    report = (root / "validator-report.md").read_text()
    result_dir = root / "validator/artifacts/libvips-safe/port-04-test/results/libvips"
    summary = json.loads((result_dir / "summary.json").read_text())
    if summary["source_cases"] != inventory["source_cases"] or summary["usage_cases"] != inventory["usage_cases"] or summary["cases"] != inventory["total_cases"]:
        raise SystemExit(f"baseline summary counts do not match inventory: {summary}")

    expected_header = ["Testcase ID", "Kind", "Status", "Owner phase", "First artifact", "Root cause", "Regression test", "Resolution"]
    allowed_owners = {
        "impl_02_source_surface_failures",
        "impl_03_ruby_usage_operation_failures",
        "impl_04_foreign_io_buffer_failures",
        "impl_05_packaging_container_and_remaining_failures",
    }
    allowed_statuses = {"open", "fixed", "deferred", "approved-skip", "not-a-defect"}
    allowed_kinds = {"source", "usage", "packaging-container"}
    if "| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |" not in report:
        raise SystemExit("validator-report.md is missing required failure ownership table")

    rows = {}
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
        if testcase_id in rows:
            duplicates.append(testcase_id)
            continue
        if kind not in allowed_kinds:
            raise SystemExit(f"bad kind for {testcase_id}: {kind}")
        if status not in allowed_statuses:
            raise SystemExit(f"bad status for {testcase_id}: {status}")
        if owner not in allowed_owners:
            raise SystemExit(f"bad owner phase for {testcase_id}: {owner}")
        if testcase_id != "__packaging_container_setup__":
            expected_kind = inventory["case_kinds"].get(testcase_id)
            if expected_kind is None:
                raise SystemExit(f"ownership row references unknown testcase: {testcase_id}")
            if kind != expected_kind:
                raise SystemExit(f"ownership kind mismatch for {testcase_id}: row={kind} manifest={expected_kind}")
        if not first_artifact or not root_cause or not regression_test or not resolution:
            raise SystemExit(f"ownership row has empty required field: {testcase_id}")
        rows[testcase_id] = {
            "kind": kind,
            "status": status,
            "owner": owner,
            "first_artifact": first_artifact,
            "root_cause": root_cause,
            "regression_test": regression_test,
            "resolution": resolution,
        }
    if duplicates:
        raise SystemExit(f"duplicate ownership rows: {duplicates}")

    results = sorted(path for path in result_dir.glob("*.json") if path.name != "summary.json")
    if len(results) != inventory["total_cases"]:
        raise SystemExit(f"expected {inventory['total_cases']} baseline result JSON files, found {len(results)}")
    installed_failures = []
    install_blocked = []
    for path in results:
        payload = json.loads(path.read_text())
        testcase_id = payload["testcase_id"]
        if payload.get("status") == "passed":
            continue
        if payload.get("override_debs_installed") is not True:
            install_blocked.append(testcase_id)
        else:
            installed_failures.append(testcase_id)
    for testcase_id in installed_failures:
        row = rows.get(testcase_id)
        if row is None:
            raise SystemExit(f"baseline failure lacks ownership row: {testcase_id}")
        if row["status"] == "fixed":
            raise SystemExit(f"currently failing baseline testcase is marked fixed: {testcase_id}")
    matrix_exit_path = root / "validator/artifacts/libvips-safe/matrix-exit-code.txt"
    if not matrix_exit_path.is_file():
        raise SystemExit(f"missing baseline matrix exit code: {matrix_exit_path}")
    matrix_exit = int(matrix_exit_path.read_text().strip())
    if install_blocked or matrix_exit != 0:
        row = rows.get("__packaging_container_setup__")
        if row is None:
            raise SystemExit(f"missing packaging/container owner row for setup issue: install_blocked={install_blocked}, matrix_exit={matrix_exit}")
        if row["kind"] != "packaging-container" or row["owner"] != "impl_05_packaging_container_and_remaining_failures":
            raise SystemExit(f"bad packaging/container owner row: {row}")
    print(f"baseline classification validated: installed_failures={len(installed_failures)} install_blocked={len(install_blocked)} matrix_exit={matrix_exit}")
    PY
    rg -n "validator checkout|remote main|active validator|failure classification|Owner phase|Testcase ID" "$ROOT/validator-report.md"
    ```
- `check_01_validator_bootstrap_senior_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_01_validator_bootstrap_baseline`
  - Purpose: review workflow integrity, validator immutability, active revision decision, and failure classification for later phases.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    git -C "$ROOT" status --short
    rg -n "impl_02_source_surface_failures|impl_03_ruby_usage_operation_failures|impl_04_foreign_io_buffer_failures|impl_05_packaging_container_and_remaining_failures" "$ROOT/validator-report.md"
    python3 - <<'PY'
    import json
    from pathlib import Path
    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inv = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    result_dir = root / "validator/artifacts/libvips-safe/port-04-test/results/libvips"
    cases = sorted(p for p in result_dir.glob("*.json") if p.name != "summary.json")
    assert len(cases) == inv["total_cases"], (len(cases), inv)
    failures = [json.loads(p.read_text()) for p in cases if json.loads(p.read_text()).get("status") != "passed"]
    print(f"baseline failures: {len(failures)}")
    PY
    python3 - <<'PY'
    import json
    from pathlib import Path

    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inventory = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    report = (root / "validator-report.md").read_text()
    result_dir = root / "validator/artifacts/libvips-safe/port-04-test/results/libvips"
    summary = json.loads((result_dir / "summary.json").read_text())
    if summary["source_cases"] != inventory["source_cases"] or summary["usage_cases"] != inventory["usage_cases"] or summary["cases"] != inventory["total_cases"]:
        raise SystemExit(f"baseline summary counts do not match inventory: {summary}")

    expected_header = ["Testcase ID", "Kind", "Status", "Owner phase", "First artifact", "Root cause", "Regression test", "Resolution"]
    allowed_owners = {
        "impl_02_source_surface_failures",
        "impl_03_ruby_usage_operation_failures",
        "impl_04_foreign_io_buffer_failures",
        "impl_05_packaging_container_and_remaining_failures",
    }
    allowed_statuses = {"open", "fixed", "deferred", "approved-skip", "not-a-defect"}
    allowed_kinds = {"source", "usage", "packaging-container"}
    if "| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |" not in report:
        raise SystemExit("validator-report.md is missing required failure ownership table")

    rows = {}
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
        if testcase_id in rows:
            duplicates.append(testcase_id)
            continue
        if kind not in allowed_kinds:
            raise SystemExit(f"bad kind for {testcase_id}: {kind}")
        if status not in allowed_statuses:
            raise SystemExit(f"bad status for {testcase_id}: {status}")
        if owner not in allowed_owners:
            raise SystemExit(f"bad owner phase for {testcase_id}: {owner}")
        if testcase_id != "__packaging_container_setup__":
            expected_kind = inventory["case_kinds"].get(testcase_id)
            if expected_kind is None:
                raise SystemExit(f"ownership row references unknown testcase: {testcase_id}")
            if kind != expected_kind:
                raise SystemExit(f"ownership kind mismatch for {testcase_id}: row={kind} manifest={expected_kind}")
        if not first_artifact or not root_cause or not regression_test or not resolution:
            raise SystemExit(f"ownership row has empty required field: {testcase_id}")
        rows[testcase_id] = {
            "kind": kind,
            "status": status,
            "owner": owner,
            "first_artifact": first_artifact,
            "root_cause": root_cause,
            "regression_test": regression_test,
            "resolution": resolution,
        }
    if duplicates:
        raise SystemExit(f"duplicate ownership rows: {duplicates}")

    results = sorted(path for path in result_dir.glob("*.json") if path.name != "summary.json")
    if len(results) != inventory["total_cases"]:
        raise SystemExit(f"expected {inventory['total_cases']} baseline result JSON files, found {len(results)}")
    installed_failures = []
    install_blocked = []
    for path in results:
        payload = json.loads(path.read_text())
        testcase_id = payload["testcase_id"]
        if payload.get("status") == "passed":
            continue
        if payload.get("override_debs_installed") is not True:
            install_blocked.append(testcase_id)
        else:
            installed_failures.append(testcase_id)
    for testcase_id in installed_failures:
        row = rows.get(testcase_id)
        if row is None:
            raise SystemExit(f"baseline failure lacks ownership row: {testcase_id}")
        if row["status"] == "fixed":
            raise SystemExit(f"currently failing baseline testcase is marked fixed: {testcase_id}")
    matrix_exit_path = root / "validator/artifacts/libvips-safe/matrix-exit-code.txt"
    if not matrix_exit_path.is_file():
        raise SystemExit(f"missing baseline matrix exit code: {matrix_exit_path}")
    matrix_exit = int(matrix_exit_path.read_text().strip())
    if install_blocked or matrix_exit != 0:
        row = rows.get("__packaging_container_setup__")
        if row is None:
            raise SystemExit(f"missing packaging/container owner row for setup issue: install_blocked={install_blocked}, matrix_exit={matrix_exit}")
        if row["kind"] != "packaging-container" or row["owner"] != "impl_05_packaging_container_and_remaining_failures":
            raise SystemExit(f"bad packaging/container owner row: {row}")
    print(f"baseline classification validated: installed_failures={len(installed_failures)} install_blocked={len(install_blocked)} matrix_exit={matrix_exit}")
    PY
    ```

**Verification:**

- `make unit` from the active validator checkout.
- `tools/testcases.py --check` using thresholds from `validator/artifacts/libvips-safe-inventory.json`.
- Full validator command with `ARTIFACT_NAME=libvips-safe`.
- Shared baseline failure classification verification that rejects missing, duplicate, malformed, or wrong-owner rows for every failed baseline testcase before phase 2 starts.
- Lock structure validation, summary validation against inventory counts, package/container setup classification when matrix exit is nonzero, and validator immutability diff.

**Success Criteria:**

- Active validator checkout selected from a runnable libvips manifest, with fallback reason documented if needed.
- `validator/artifacts/libvips-safe-inventory.json` exists and matches the active manifest counts and testcase IDs.
- Fresh safe packages are rebuilt from the committed source state, staged under `validator-overrides/libvips/`, and represented by `validator/artifacts/libvips-safe-port-lock.json`.
- Baseline validator artifacts exist under `validator/artifacts/libvips-safe/**` and every baseline failure is classified once in the required ownership table.
- Both verifier phases pass, including validator immutability checks.

**Git Commit Requirement:**

The implementer must commit this phase's work to git before yielding. If the phase makes no production changes, it must still commit a `validator-report.md` update or create an explicit `--allow-empty` commit documenting the no-op and the evidence checked. Generated validator artifacts, override packages, proof/site output, Docker outputs, Debian package outputs, and build directories must not be committed.
