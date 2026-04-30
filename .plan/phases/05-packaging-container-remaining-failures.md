# Packaging, Container, Validator-Bug, And Remaining Failures

**Phase Name:** Packaging, Container, Validator-Bug, And Remaining Failures

**Implement Phase ID:** `impl_05_packaging_container_and_remaining_failures`

**Preexisting Inputs:**

- Everything produced by phases 1-4.
- Debian package files and scripts under `safe/debian/**`.
- Meson install, module, GIR, C++ wrapper, pkg-config, and tool packaging logic in `safe/meson.build`, `safe/build_support/**`, and `safe/scripts/**`.
- Dependent and upstream harnesses under `safe/tests/dependents/**`, `safe/tests/upstream/**`, `dependents.json`, and `test-original.sh`.
- Validator Docker harness files under `validator/tests/libvips/`.

**New Outputs:**

- Packaging/container/release-gate fixes in safe code or packaging.
- Any final catch-all safe behavior fixes not clearly owned by phases 2-4, each with regression tests.
- Approved validator-bug skip artifacts under `validator/artifacts/approved-skips/libvips/<testcase-id>/**` only if a validator bug is clearly documented.
- Full rerun artifacts under `validator/artifacts/libvips-safe-remaining/**`.
- Updated `validator-report.md`.
- A git commit for this phase.

**File Changes:**

- Fix package metadata and install layout in `safe/debian/**` and Meson install rules.
- Fix release-gate blind spots in `safe/scripts/run_release_gate.sh` without weakening prior checks.
- Fix dependent/upstream harness issues only when the safe harness is wrong.
- If a remaining failure is truly a validator bug, generate adjusted artifacts under `validator/artifacts/approved-skips/libvips/<testcase-id>/` and document the exact testcase and justification in `validator-report.md`; the report row must use the testcase's manifest `source` or `usage` kind, `Status` `approved-skip`, `Owner phase` `impl_05_packaging_container_and_remaining_failures`, and `Regression test` `N/A`. Leave validator tracked files untouched.

**Implementation Details:**

1. Run `safe/scripts/run_release_gate.sh` before making changes. Treat release-gate failures as real defects unless the gate itself is stale or wrong.
2. Fix package layout and install behavior so validator Docker runs install override packages and set `override_debs_installed: true`.
3. For remaining validator failures, add minimal safe regression tests and fix the owning module. This phase is the catch-all for defects not clearly owned by phases 2-4.
4. If a validator bug is claimed, keep failing evidence from an unmodified validator run and add a documented approved-skip generated copy under `validator/artifacts/approved-skips/libvips/<testcase-id>/`. Update the existing report row in place so it keeps the manifest `source` or `usage` kind, changes status to `approved-skip`, keeps owner `impl_05_packaging_container_and_remaining_failures`, and names `N/A` as the regression test.
5. For an approved skip, create `validator/artifacts/approved-skips/libvips/<testcase-id>/skip-meta.json` and `test-root/`. `skip-meta.json` must contain `skipped_testcase_id`, `skipped_kind`, absolute `tests_root`, `expected_source_cases`, `expected_usage_cases`, and `expected_total_cases`. Copy the libvips harness to both `test-root/libvips/` and `test-root/tests/libvips/`; leave `test-root/libvips/` byte-for-byte identical to the active validator harness for Docker context, and remove only the skipped testcase from the actual manifest resolved by validator tooling at `test-root/tests/libvips/testcases.yml`. Verify the adjusted manifest path with `tools/testcases.py --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --library libvips --check` and do not weaken any script or fixture.
6. Commit packaging/container/catch-all safe changes before rebuilding packages.
7. Rebuild packages, refresh lock, run the full validator into `validator/artifacts/libvips-safe-remaining`, generate approved-skip proof when an approved skip exists, and update the report.
8. Commit the report update before yielding.

**Verification Phases:**

- `check_05_packaging_container_software_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_05_packaging_container_and_remaining_failures`
  - Purpose: run the full release gate, rebuild/stage packages, run the full validator matrix, and verify no unowned failures remain.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    scripts/run_release_gate.sh
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
    ARTIFACT_NAME=libvips-safe-remaining
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
    cd "$ROOT/validator"
    VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
    if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
    SOURCE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["source_cases"])')
    USAGE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["usage_cases"])')
    TOTAL_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["total_cases"])')
    SKIP_META=$(find "$ROOT/validator/artifacts/approved-skips/libvips" -name skip-meta.json -print -quit 2>/dev/null || true)
    if test -z "$SKIP_META"; then
      export RESULT_ARTIFACT_NAME=libvips-safe-remaining BLOCKING_OWNERS=impl_02_source_surface_failures:impl_03_ruby_usage_operation_failures:impl_04_foreign_io_buffer_failures:impl_05_packaging_container_and_remaining_failures LATER_OWNERS= ALLOW_INSTALL_BLOCKED=0
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
      export RESULT_ARTIFACT_NAME=libvips-safe-remaining EXPECTED_SOURCE_CASES="$SOURCE_CASES" EXPECTED_USAGE_CASES="$USAGE_CASES" EXPECTED_TOTAL_CASES="$TOTAL_CASES"
      python3 - <<'PY'
      import json
      import os
      from pathlib import Path

      root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
      artifact = os.environ["RESULT_ARTIFACT_NAME"]
      expected_source = int(os.environ["EXPECTED_SOURCE_CASES"])
      expected_usage = int(os.environ["EXPECTED_USAGE_CASES"])
      expected_total = int(os.environ["EXPECTED_TOTAL_CASES"])
      artifact_root = root / "validator/artifacts" / artifact
      exit_path = artifact_root / "matrix-exit-code.txt"
      if not exit_path.is_file():
          raise SystemExit(f"missing matrix exit code for {artifact}: {exit_path}")
      matrix_exit = int(exit_path.read_text().strip())
      if matrix_exit != 0:
          raise SystemExit(f"validator matrix exited {matrix_exit} for {artifact}")
      result_dir = artifact_root / "port-04-test/results/libvips"
      summary = json.loads((result_dir / "summary.json").read_text())
      if summary["source_cases"] != expected_source or summary["usage_cases"] != expected_usage or summary["cases"] != expected_total:
          raise SystemExit(f"summary counts do not match expected counts: {summary}")
      if summary["failed"] != 0 or summary["passed"] != expected_total or summary["casts"] != expected_total:
          raise SystemExit(f"validator run is not clean: {summary}")
      results = [path for path in sorted(result_dir.glob("*.json")) if path.name != "summary.json"]
      if len(results) != expected_total:
          raise SystemExit(f"expected {expected_total} result JSON files, found {len(results)}")
      bad = []
      for path in results:
          payload = json.loads(path.read_text())
          testcase_id = payload.get("testcase_id", path.stem)
          if payload.get("status") != "passed":
              bad.append(f"{testcase_id}: status={payload.get('status')}")
          if payload.get("override_debs_installed") is not True:
              bad.append(f"{testcase_id}: override_debs_installed={payload.get('override_debs_installed')!r}")
          if payload.get("cast_path") is None:
              bad.append(f"{testcase_id}: missing cast")
      if bad:
          raise SystemExit("clean validator assertion failed: " + "; ".join(bad))
      PY
    else
      export UNMODIFIED_ARTIFACT_NAME=libvips-safe-remaining
      ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
      VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
      if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
      "$VALIDATOR_PY" - <<'PY'
      import filecmp
      import json
      import os
      import sys
      from pathlib import Path

      import yaml

      root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
      sys.path.insert(0, str(root / "validator"))
      from tools import select_libraries
      from tools.inventory import load_manifest
      from tools.testcases import load_manifests

      inventory = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
      meta_files = sorted((root / "validator/artifacts/approved-skips/libvips").glob("*/skip-meta.json"))
      if not meta_files:
          raise SystemExit(0)
      if len(meta_files) != 1:
          raise SystemExit(f"expected exactly one approved skip, found {len(meta_files)}")
      meta_path = meta_files[0]
      meta = json.loads(meta_path.read_text())
      skipped = meta["skipped_testcase_id"]
      skipped_kind = meta["skipped_kind"]
      expected_skip_root = root / "validator/artifacts/approved-skips/libvips" / skipped
      expected_tests_root = expected_skip_root / "test-root"
      if meta_path.parent != expected_skip_root:
          raise SystemExit("skip-meta.json is not under the approved testcase-id path")
      if Path(meta["tests_root"]) != expected_tests_root:
          raise SystemExit("skip-meta tests_root is not the exact approved absolute path")
      if skipped_kind not in {"source", "usage"}:
          raise SystemExit(f"bad skipped_kind: {skipped_kind}")
      if inventory["case_kinds"].get(skipped) != skipped_kind:
          raise SystemExit("skipped testcase kind does not match active manifest")

      report = (root / "validator-report.md").read_text()
      expected_header = ["Testcase ID", "Kind", "Status", "Owner phase", "First artifact", "Root cause", "Regression test", "Resolution"]
      skip_row = None
      for line in report.splitlines():
          if not line.startswith("|"):
              continue
          columns = [part.strip() for part in line.strip().strip("|").split("|")]
          if columns == expected_header:
              continue
          if len(columns) != len(expected_header) or columns[0] in {"---", ""} or set(columns[0]) == {"-"}:
              continue
          if columns[0] == skipped:
              if skip_row is not None:
                  raise SystemExit(f"duplicate approved-skip report rows for {skipped}")
              skip_row = columns
      if skip_row is None:
          raise SystemExit(f"approved skip lacks validator-report.md row: {skipped}")
      if skip_row[1] != skipped_kind:
          raise SystemExit(f"approved-skip report kind must match manifest kind: row={skip_row[1]} manifest={skipped_kind}")
      if skip_row[2] != "approved-skip":
          raise SystemExit(f"approved-skip report row has wrong status: {skip_row[2]}")
      if skip_row[3].strip("`") != "impl_05_packaging_container_and_remaining_failures":
          raise SystemExit(f"approved-skip report row has wrong owner: {skip_row[3]}")
      if skip_row[6] != "N/A":
          raise SystemExit("approved-skip report row must use Regression test N/A")
      if not skip_row[4] or not skip_row[5] or not skip_row[7]:
          raise SystemExit("approved-skip report row has empty required evidence fields")
      if "validator" not in f"{skip_row[5]} {skip_row[7]}".lower():
          raise SystemExit("approved-skip report row must identify the validator bug in root cause or resolution")

      if meta["expected_source_cases"] != inventory["source_cases"] - (1 if skipped_kind == "source" else 0):
          raise SystemExit("bad expected_source_cases")
      if meta["expected_usage_cases"] != inventory["usage_cases"] - (1 if skipped_kind == "usage" else 0):
          raise SystemExit("bad expected_usage_cases")
      if meta["expected_total_cases"] != inventory["total_cases"] - 1:
          raise SystemExit("bad expected_total_cases")

      upstream = root / "validator/tests/libvips"
      copy_for_context = expected_tests_root / "libvips"
      copy_for_manifest = expected_tests_root / "tests/libvips"
      if not copy_for_context.is_dir() or not copy_for_manifest.is_dir():
          raise SystemExit("approved skip test-root must contain both libvips/ and tests/libvips/")

      def compare_tree(expected: Path, actual: Path, *, ignore_manifest: bool) -> None:
          left = {p.relative_to(expected) for p in expected.rglob("*") if p.is_file()}
          right = {p.relative_to(actual) for p in actual.rglob("*") if p.is_file()}
          if ignore_manifest:
              left.discard(Path("testcases.yml"))
              right.discard(Path("testcases.yml"))
          if left != right:
              raise SystemExit(f"approved skip harness file set differs: missing={sorted(left-right)} extra={sorted(right-left)}")
          for rel in sorted(left):
              if not filecmp.cmp(expected / rel, actual / rel, shallow=False):
                  raise SystemExit(f"approved skip harness file differs: {rel}")

      compare_tree(upstream, copy_for_context, ignore_manifest=False)
      compare_tree(upstream, copy_for_manifest, ignore_manifest=True)

      orig = yaml.safe_load((upstream / "testcases.yml").read_text())
      adj = yaml.safe_load((copy_for_manifest / "testcases.yml").read_text())
      orig_cases = orig.get("testcases")
      adj_cases = adj.get("testcases")
      if not isinstance(orig_cases, list) or not isinstance(adj_cases, list):
          raise SystemExit("original or adjusted manifest lacks testcase list")
      orig_ids = [case["id"] for case in orig_cases]
      adj_ids = [case["id"] for case in adj_cases]
      if skipped not in orig_ids:
          raise SystemExit("skipped testcase not in original manifest")
      if skipped in adj_ids:
          raise SystemExit("skipped testcase still present in adjusted manifest")
      if [case_id for case_id in orig_ids if case_id != skipped] != adj_ids:
          raise SystemExit("adjusted manifest changed more than the single skipped testcase")
      orig_without_skip = [case for case in orig_cases if case["id"] != skipped]
      adj_copy = dict(adj)
      orig_copy = dict(orig)
      adj_copy["testcases"] = adj_cases
      orig_copy["testcases"] = orig_without_skip
      if adj_copy != orig_copy:
          raise SystemExit("adjusted manifest differs beyond removing the skipped testcase")

      config = load_manifest(root / "validator/repositories.yml")
      selected = select_libraries(config, ["libvips"])
      selected_config = dict(config)
      selected_config["libraries"] = selected
      resolved = load_manifests(selected_config, tests_root=expected_tests_root)["libvips"]
      resolved_ids = [case.id for case in resolved.testcases]
      if resolved_ids != adj_ids:
          raise SystemExit("validator tooling does not resolve the adjusted approved-skip manifest")
      if skipped in resolved_ids:
          raise SystemExit("validator tooling still resolves the skipped testcase")

      artifact_name = os.environ.get("UNMODIFIED_ARTIFACT_NAME", "libvips-safe-final")
      result_dir = root / f"validator/artifacts/{artifact_name}/port-04-test/results/libvips"
      summary = json.loads((result_dir / "summary.json").read_text())
      if summary["source_cases"] != inventory["source_cases"] or summary["usage_cases"] != inventory["usage_cases"] or summary["cases"] != inventory["total_cases"]:
          raise SystemExit(f"unmodified approved-skip evidence has wrong counts: {summary}")
      failures = []
      install_blocked = []
      for path in sorted(result_dir.glob("*.json")):
          if path.name == "summary.json":
              continue
          payload = json.loads(path.read_text())
          if payload.get("override_debs_installed") is not True:
              install_blocked.append(payload["testcase_id"])
          if payload.get("status") != "passed":
              failures.append(payload["testcase_id"])
      if install_blocked:
          raise SystemExit(f"approved-skip evidence still has override package or container setup failures: {install_blocked}")
      if failures != [skipped]:
          raise SystemExit(f"unmodified approved-skip evidence must fail only the skipped testcase: {failures}")
      PY
      SKIP_TESTS_ROOT=$("$VALIDATOR_PY" - "$SKIP_META" <<'PY'
    import json, sys
    from pathlib import Path
    print(json.loads(Path(sys.argv[1]).read_text())["tests_root"])
    PY
      )
      SKIP_SOURCE=$("$VALIDATOR_PY" - "$SKIP_META" <<'PY'
    import json, sys
    from pathlib import Path
    print(json.loads(Path(sys.argv[1]).read_text())["expected_source_cases"])
    PY
      )
      SKIP_USAGE=$("$VALIDATOR_PY" - "$SKIP_META" <<'PY'
    import json, sys
    from pathlib import Path
    print(json.loads(Path(sys.argv[1]).read_text())["expected_usage_cases"])
    PY
      )
      SKIP_TOTAL=$("$VALIDATOR_PY" - "$SKIP_META" <<'PY'
    import json, sys
    from pathlib import Path
    print(json.loads(Path(sys.argv[1]).read_text())["expected_total_cases"])
    PY
      )
      "$VALIDATOR_PY" tools/testcases.py --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --library libvips --check --min-source-cases "$SKIP_SOURCE" --min-usage-cases "$SKIP_USAGE" --min-cases "$SKIP_TOTAL"
      rm -rf artifacts/libvips-safe-remaining-approved-skip
      set +e
      PYTHON="$VALIDATOR_PY" RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifact-root artifacts/libvips-safe-remaining-approved-skip --mode port-04-test --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-port-lock.json" --record-casts
      MATRIX_EXIT=$?
      set -e
      mkdir -p artifacts/libvips-safe-remaining-approved-skip
      printf '%s\n' "$MATRIX_EXIT" > artifacts/libvips-safe-remaining-approved-skip/matrix-exit-code.txt
      if ! test -f artifacts/libvips-safe-remaining-approved-skip/port-04-test/results/libvips/summary.json; then
        if test "$MATRIX_EXIT" -ne 0; then
          exit "$MATRIX_EXIT"
        fi
        echo "approved-skip matrix produced no libvips summary" >&2
        exit 1
      fi
      export RESULT_ARTIFACT_NAME=libvips-safe-remaining-approved-skip EXPECTED_SOURCE_CASES="$SKIP_SOURCE" EXPECTED_USAGE_CASES="$SKIP_USAGE" EXPECTED_TOTAL_CASES="$SKIP_TOTAL"
      python3 - <<'PY'
      import json
      import os
      from pathlib import Path

      root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
      artifact = os.environ["RESULT_ARTIFACT_NAME"]
      expected_source = int(os.environ["EXPECTED_SOURCE_CASES"])
      expected_usage = int(os.environ["EXPECTED_USAGE_CASES"])
      expected_total = int(os.environ["EXPECTED_TOTAL_CASES"])
      artifact_root = root / "validator/artifacts" / artifact
      exit_path = artifact_root / "matrix-exit-code.txt"
      if not exit_path.is_file():
          raise SystemExit(f"missing matrix exit code for {artifact}: {exit_path}")
      matrix_exit = int(exit_path.read_text().strip())
      if matrix_exit != 0:
          raise SystemExit(f"validator matrix exited {matrix_exit} for {artifact}")
      result_dir = artifact_root / "port-04-test/results/libvips"
      summary = json.loads((result_dir / "summary.json").read_text())
      if summary["source_cases"] != expected_source or summary["usage_cases"] != expected_usage or summary["cases"] != expected_total:
          raise SystemExit(f"summary counts do not match expected counts: {summary}")
      if summary["failed"] != 0 or summary["passed"] != expected_total or summary["casts"] != expected_total:
          raise SystemExit(f"validator run is not clean: {summary}")
      results = [path for path in sorted(result_dir.glob("*.json")) if path.name != "summary.json"]
      if len(results) != expected_total:
          raise SystemExit(f"expected {expected_total} result JSON files, found {len(results)}")
      bad = []
      for path in results:
          payload = json.loads(path.read_text())
          testcase_id = payload.get("testcase_id", path.stem)
          if payload.get("status") != "passed":
              bad.append(f"{testcase_id}: status={payload.get('status')}")
          if payload.get("override_debs_installed") is not True:
              bad.append(f"{testcase_id}: override_debs_installed={payload.get('override_debs_installed')!r}")
          if payload.get("cast_path") is None:
              bad.append(f"{testcase_id}: missing cast")
      if bad:
          raise SystemExit("clean validator assertion failed: " + "; ".join(bad))
      PY
      "$VALIDATOR_PY" tools/verify_proof_artifacts.py --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifact-root artifacts/libvips-safe-remaining-approved-skip --proof-output proof/libvips-safe-validation-proof.json --mode port-04-test --library libvips --require-casts --min-source-cases "$SKIP_SOURCE" --min-usage-cases "$SKIP_USAGE" --min-cases "$SKIP_TOTAL"
    fi
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    ```
- `check_05_packaging_container_senior_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_05_packaging_container_and_remaining_failures`
  - Purpose: review package correctness, validator override installation, release-gate coverage, approved skip discipline, and remaining failure handling.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features -- --nocapture
    rg -n "Packaging|override|approved skip|validator bug|impl_05_packaging_container_and_remaining_failures|release gate|run_release_gate|Owner phase" "$ROOT/validator-report.md"
    python3 - <<'PY'
    import json
    from pathlib import Path
    lock = json.loads(Path("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json").read_text())
    assert lock["mode"] == "port-04-test"
    assert lock["libraries"][0]["unported_original_packages"] == []
    assert len(lock["libraries"][0]["debs"]) == 4
    PY
    SKIP_META=$(find "$ROOT/validator/artifacts/approved-skips/libvips" -name skip-meta.json -print -quit 2>/dev/null || true)
    if test -n "$SKIP_META"; then
      export UNMODIFIED_ARTIFACT_NAME=libvips-safe-remaining
      ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
      VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
      if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
      "$VALIDATOR_PY" - <<'PY'
      import filecmp
      import json
      import os
      import sys
      from pathlib import Path

      import yaml

      root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
      sys.path.insert(0, str(root / "validator"))
      from tools import select_libraries
      from tools.inventory import load_manifest
      from tools.testcases import load_manifests

      inventory = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
      meta_files = sorted((root / "validator/artifacts/approved-skips/libvips").glob("*/skip-meta.json"))
      if not meta_files:
          raise SystemExit(0)
      if len(meta_files) != 1:
          raise SystemExit(f"expected exactly one approved skip, found {len(meta_files)}")
      meta_path = meta_files[0]
      meta = json.loads(meta_path.read_text())
      skipped = meta["skipped_testcase_id"]
      skipped_kind = meta["skipped_kind"]
      expected_skip_root = root / "validator/artifacts/approved-skips/libvips" / skipped
      expected_tests_root = expected_skip_root / "test-root"
      if meta_path.parent != expected_skip_root:
          raise SystemExit("skip-meta.json is not under the approved testcase-id path")
      if Path(meta["tests_root"]) != expected_tests_root:
          raise SystemExit("skip-meta tests_root is not the exact approved absolute path")
      if skipped_kind not in {"source", "usage"}:
          raise SystemExit(f"bad skipped_kind: {skipped_kind}")
      if inventory["case_kinds"].get(skipped) != skipped_kind:
          raise SystemExit("skipped testcase kind does not match active manifest")

      report = (root / "validator-report.md").read_text()
      expected_header = ["Testcase ID", "Kind", "Status", "Owner phase", "First artifact", "Root cause", "Regression test", "Resolution"]
      skip_row = None
      for line in report.splitlines():
          if not line.startswith("|"):
              continue
          columns = [part.strip() for part in line.strip().strip("|").split("|")]
          if columns == expected_header:
              continue
          if len(columns) != len(expected_header) or columns[0] in {"---", ""} or set(columns[0]) == {"-"}:
              continue
          if columns[0] == skipped:
              if skip_row is not None:
                  raise SystemExit(f"duplicate approved-skip report rows for {skipped}")
              skip_row = columns
      if skip_row is None:
          raise SystemExit(f"approved skip lacks validator-report.md row: {skipped}")
      if skip_row[1] != skipped_kind:
          raise SystemExit(f"approved-skip report kind must match manifest kind: row={skip_row[1]} manifest={skipped_kind}")
      if skip_row[2] != "approved-skip":
          raise SystemExit(f"approved-skip report row has wrong status: {skip_row[2]}")
      if skip_row[3].strip("`") != "impl_05_packaging_container_and_remaining_failures":
          raise SystemExit(f"approved-skip report row has wrong owner: {skip_row[3]}")
      if skip_row[6] != "N/A":
          raise SystemExit("approved-skip report row must use Regression test N/A")
      if not skip_row[4] or not skip_row[5] or not skip_row[7]:
          raise SystemExit("approved-skip report row has empty required evidence fields")
      if "validator" not in f"{skip_row[5]} {skip_row[7]}".lower():
          raise SystemExit("approved-skip report row must identify the validator bug in root cause or resolution")

      if meta["expected_source_cases"] != inventory["source_cases"] - (1 if skipped_kind == "source" else 0):
          raise SystemExit("bad expected_source_cases")
      if meta["expected_usage_cases"] != inventory["usage_cases"] - (1 if skipped_kind == "usage" else 0):
          raise SystemExit("bad expected_usage_cases")
      if meta["expected_total_cases"] != inventory["total_cases"] - 1:
          raise SystemExit("bad expected_total_cases")

      upstream = root / "validator/tests/libvips"
      copy_for_context = expected_tests_root / "libvips"
      copy_for_manifest = expected_tests_root / "tests/libvips"
      if not copy_for_context.is_dir() or not copy_for_manifest.is_dir():
          raise SystemExit("approved skip test-root must contain both libvips/ and tests/libvips/")

      def compare_tree(expected: Path, actual: Path, *, ignore_manifest: bool) -> None:
          left = {p.relative_to(expected) for p in expected.rglob("*") if p.is_file()}
          right = {p.relative_to(actual) for p in actual.rglob("*") if p.is_file()}
          if ignore_manifest:
              left.discard(Path("testcases.yml"))
              right.discard(Path("testcases.yml"))
          if left != right:
              raise SystemExit(f"approved skip harness file set differs: missing={sorted(left-right)} extra={sorted(right-left)}")
          for rel in sorted(left):
              if not filecmp.cmp(expected / rel, actual / rel, shallow=False):
                  raise SystemExit(f"approved skip harness file differs: {rel}")

      compare_tree(upstream, copy_for_context, ignore_manifest=False)
      compare_tree(upstream, copy_for_manifest, ignore_manifest=True)

      orig = yaml.safe_load((upstream / "testcases.yml").read_text())
      adj = yaml.safe_load((copy_for_manifest / "testcases.yml").read_text())
      orig_cases = orig.get("testcases")
      adj_cases = adj.get("testcases")
      if not isinstance(orig_cases, list) or not isinstance(adj_cases, list):
          raise SystemExit("original or adjusted manifest lacks testcase list")
      orig_ids = [case["id"] for case in orig_cases]
      adj_ids = [case["id"] for case in adj_cases]
      if skipped not in orig_ids:
          raise SystemExit("skipped testcase not in original manifest")
      if skipped in adj_ids:
          raise SystemExit("skipped testcase still present in adjusted manifest")
      if [case_id for case_id in orig_ids if case_id != skipped] != adj_ids:
          raise SystemExit("adjusted manifest changed more than the single skipped testcase")
      orig_without_skip = [case for case in orig_cases if case["id"] != skipped]
      adj_copy = dict(adj)
      orig_copy = dict(orig)
      adj_copy["testcases"] = adj_cases
      orig_copy["testcases"] = orig_without_skip
      if adj_copy != orig_copy:
          raise SystemExit("adjusted manifest differs beyond removing the skipped testcase")

      config = load_manifest(root / "validator/repositories.yml")
      selected = select_libraries(config, ["libvips"])
      selected_config = dict(config)
      selected_config["libraries"] = selected
      resolved = load_manifests(selected_config, tests_root=expected_tests_root)["libvips"]
      resolved_ids = [case.id for case in resolved.testcases]
      if resolved_ids != adj_ids:
          raise SystemExit("validator tooling does not resolve the adjusted approved-skip manifest")
      if skipped in resolved_ids:
          raise SystemExit("validator tooling still resolves the skipped testcase")

      artifact_name = os.environ.get("UNMODIFIED_ARTIFACT_NAME", "libvips-safe-final")
      result_dir = root / f"validator/artifacts/{artifact_name}/port-04-test/results/libvips"
      summary = json.loads((result_dir / "summary.json").read_text())
      if summary["source_cases"] != inventory["source_cases"] or summary["usage_cases"] != inventory["usage_cases"] or summary["cases"] != inventory["total_cases"]:
          raise SystemExit(f"unmodified approved-skip evidence has wrong counts: {summary}")
      failures = []
      install_blocked = []
      for path in sorted(result_dir.glob("*.json")):
          if path.name == "summary.json":
              continue
          payload = json.loads(path.read_text())
          if payload.get("override_debs_installed") is not True:
              install_blocked.append(payload["testcase_id"])
          if payload.get("status") != "passed":
              failures.append(payload["testcase_id"])
      if install_blocked:
          raise SystemExit(f"approved-skip evidence still has override package or container setup failures: {install_blocked}")
      if failures != [skipped]:
          raise SystemExit(f"unmodified approved-skip evidence must fail only the skipped testcase: {failures}")
      PY
    fi
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    ```

**Verification:**

- `safe/scripts/run_release_gate.sh`.
- Full validator command with `ARTIFACT_NAME=libvips-safe-remaining`.
- Lock/package validation and validator immutability checks.
- No-skip path: shared clean validator result assertion proves matrix exit code `0`, zero remaining failures, all override packages installed, all casts present, and all inventory-derived cases passed.
- Approved-skip path: exactly one unmodified failure, reported with the skipped testcase's manifest kind, `approved-skip` status, owner `impl_05_packaging_container_and_remaining_failures`, `skip-meta.json`, adjusted tests root, adjusted proof, and a shared clean validator result assertion for the approved-skip rerun whose source/usage counts are reduced by exactly the skipped testcase's kind.

**Success Criteria:**

- `safe/scripts/run_release_gate.sh` passes before final handoff.
- Override package installation and container setup issues are fixed, with `override_debs_installed: true` for clean no-skip evidence.
- No-skip path has a clean full validator run under `validator/artifacts/libvips-safe-remaining/**`; approved-skip path has exactly one documented validator-bug failure and a clean adjusted rerun.
- Approved-skip artifacts, if used, remove exactly one testcase and preserve the active upstream harness copy rules.
- Both verifier phases pass, including validator immutability checks.

**Git Commit Requirement:**

The implementer must commit this phase's work to git before yielding. If the phase makes no production changes, it must still commit a `validator-report.md` update or create an explicit `--allow-empty` commit documenting the no-op and the evidence checked. Generated validator artifacts, override packages, proof/site output, Docker outputs, Debian package outputs, and build directories must not be committed.
