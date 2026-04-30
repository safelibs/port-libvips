# Final Report And Clean Validator Run

**Phase Name:** Final Report And Clean Validator Run

**Implement Phase ID:** `impl_06_final_report_and_clean_run`

**Preexisting Inputs:**

- Everything produced by phases 1-5.
- Final safe source tree and all committed regression tests.
- Final staged package override flow, local lock, active validator checkout, and active validator inventory.

**New Outputs:**

- Fresh final unmodified validator artifacts under `validator/artifacts/libvips-safe-final/**`.
- No-skip path final proof under `validator/artifacts/libvips-safe-final/proof/`; approved-skip path adjusted proof under `validator/artifacts/libvips-safe-final-approved-skip/proof/`.
- Rendered site under the matching `validator/site/libvips-safe-final*` directory.
- Finalized `validator-report.md` with validator URL, remote main commit, active validator commit, checks executed, failures found, fixes applied, package hashes, proof/site paths, approved skips or `None`, and final clean run summary.
- A final git commit for the report and any last safe fixes.

**File Changes:**

- Finalize `validator-report.md`.
- Do not make production changes unless final verification exposes a defect that can be fixed safely within this final phase. Both final checkers use fixed `bounce_target` `impl_06_final_report_and_clean_run`; fix final behavioral, package, proof, site, or report failures directly in `impl_06_final_report_and_clean_run` with minimal regression coverage when applicable, or document a final external blocker. Do not redirect final checkers to any earlier implement phase.
- Do not edit validator tracked files.
- Do not commit generated validator artifacts, proof files, rendered site, packages, or build outputs.

**Implementation Details:**

1. Commit any final safe fixes that should be represented by rebuilt packages, or make no production changes if the tree is ready.
2. Rebuild packages from the final committed safe source.
3. Stage the four canonical override `.deb` files and refresh the lock from the final package-source commit.
4. Run a fresh final unmodified validator matrix into `validator/artifacts/libvips-safe-final`.
5. If no approved validator-bug skip exists, generate proof and site with thresholds from `validator/artifacts/libvips-safe-inventory.json`.
6. If an approved validator-bug skip exists, retain the unmodified failing final evidence and run the generated approved-skip copy separately with thresholds from `skip-meta.json`. The final report must identify both evidence roots.
7. Finalize `validator-report.md` with active validator details, exact commands, inventory counts, initial failures and owners, fixes, regression tests, package names, paths, sizes, SHA-256 hashes, approved skips or `None`, and final artifact/proof/site paths.
8. Commit the final report update before yielding.

**Verification Phases:**

- `check_06_final_software_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_06_final_report_and_clean_run`
  - Purpose: independently rerun final local tests, package staging, full validator matrix, proof generation, site rendering, and site verification.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    cd "$ROOT/safe"
    cargo test --all-features -- --nocapture
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
    cd "$ROOT/validator"
    VALIDATOR_PY="$ROOT/validator/.venv/bin/python"
    if ! test -x "$VALIDATOR_PY"; then VALIDATOR_PY=python3; fi
    SOURCE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["source_cases"])')
    USAGE_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["usage_cases"])')
    TOTAL_CASES=$("$VALIDATOR_PY" -c 'import json; print(json.load(open("/home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-inventory.json"))["total_cases"])')
    rm -rf artifacts/libvips-safe-final site/libvips-safe-final
    ARTIFACT_NAME=libvips-safe-final
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
    SKIP_META=$(find "$ROOT/validator/artifacts/approved-skips/libvips" -name skip-meta.json -print -quit 2>/dev/null || true)
    if test -z "$SKIP_META"; then
      export RESULT_ARTIFACT_NAME=libvips-safe-final EXPECTED_SOURCE_CASES="$SOURCE_CASES" EXPECTED_USAGE_CASES="$USAGE_CASES" EXPECTED_TOTAL_CASES="$TOTAL_CASES"
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
      "$VALIDATOR_PY" tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-final --proof-output proof/libvips-safe-validation-proof.json --mode port-04-test --library libvips --require-casts --min-source-cases "$SOURCE_CASES" --min-usage-cases "$USAGE_CASES" --min-cases "$TOTAL_CASES"
      "$VALIDATOR_PY" tools/render_site.py --config repositories.yml --tests-root tests --artifact-root artifacts/libvips-safe-final --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json --output-root site/libvips-safe-final
      bash scripts/verify-site.sh --config repositories.yml --tests-root tests --artifacts-root artifacts/libvips-safe-final --proof-path artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json --site-root site/libvips-safe-final --library libvips
    else
      export UNMODIFIED_ARTIFACT_NAME=libvips-safe-final
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
      rm -rf artifacts/libvips-safe-final-approved-skip site/libvips-safe-final-approved-skip
      set +e
      PYTHON="$VALIDATOR_PY" RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifact-root artifacts/libvips-safe-final-approved-skip --mode port-04-test --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-port-lock.json" --record-casts
      MATRIX_EXIT=$?
      set -e
      mkdir -p artifacts/libvips-safe-final-approved-skip
      printf '%s\n' "$MATRIX_EXIT" > artifacts/libvips-safe-final-approved-skip/matrix-exit-code.txt
      if ! test -f artifacts/libvips-safe-final-approved-skip/port-04-test/results/libvips/summary.json; then
        if test "$MATRIX_EXIT" -ne 0; then
          exit "$MATRIX_EXIT"
        fi
        echo "approved-skip final matrix produced no libvips summary" >&2
        exit 1
      fi
      export RESULT_ARTIFACT_NAME=libvips-safe-final-approved-skip EXPECTED_SOURCE_CASES="$SKIP_SOURCE" EXPECTED_USAGE_CASES="$SKIP_USAGE" EXPECTED_TOTAL_CASES="$SKIP_TOTAL"
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
      "$VALIDATOR_PY" tools/verify_proof_artifacts.py --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifact-root artifacts/libvips-safe-final-approved-skip --proof-output proof/libvips-safe-validation-proof.json --mode port-04-test --library libvips --require-casts --min-source-cases "$SKIP_SOURCE" --min-usage-cases "$SKIP_USAGE" --min-cases "$SKIP_TOTAL"
      "$VALIDATOR_PY" tools/render_site.py --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifact-root artifacts/libvips-safe-final-approved-skip --proof-path artifacts/libvips-safe-final-approved-skip/proof/libvips-safe-validation-proof.json --output-root site/libvips-safe-final-approved-skip
      bash scripts/verify-site.sh --config repositories.yml --tests-root "$SKIP_TESTS_ROOT" --artifacts-root artifacts/libvips-safe-final-approved-skip --proof-path artifacts/libvips-safe-final-approved-skip/proof/libvips-safe-validation-proof.json --site-root site/libvips-safe-final-approved-skip --library libvips
    fi
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    rg -n "Final Clean Run|Final Evidence|Approved validator-bug skips|active validator|checks executed|failures found|fixes applied" "$ROOT/validator-report.md"
    ```
- `check_06_final_senior_tester`
  - Type: `check`
  - Fixed `bounce_target`: `impl_06_final_report_and_clean_run`
  - Purpose: final architectural review of evidence, workflow integrity, report completeness, git hygiene, and no validator tampering.
  - Commands:
    ```bash
    set -euo pipefail
    ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
    git -C "$ROOT/validator" diff --exit-code -- tests repositories.yml README.md
    git -C "$ROOT" log --oneline --max-count=20
    git -C "$ROOT" status --short
    SKIP_META=$(find "$ROOT/validator/artifacts/approved-skips/libvips" -name skip-meta.json -print -quit 2>/dev/null || true)
    if test -n "$SKIP_META"; then
      export UNMODIFIED_ARTIFACT_NAME=libvips-safe-final
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
    python3 - <<'PY'
    import json
    from pathlib import Path
    root = Path("/home/yans/safelibs/pipeline/ports/port-libvips")
    inv = json.loads((root / "validator/artifacts/libvips-safe-inventory.json").read_text())
    lock = json.loads((root / "validator/artifacts/libvips-safe-port-lock.json").read_text())
    assert lock["mode"] == "port-04-test"
    meta_files = sorted((root / "validator/artifacts/approved-skips/libvips").glob("*/skip-meta.json"))
    if not meta_files:
        summary = json.loads((root / "validator/artifacts/libvips-safe-final/port-04-test/results/libvips/summary.json").read_text())
        proof = json.loads((root / "validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json").read_text())
        assert summary["failed"] == 0 and summary["passed"] == inv["total_cases"] and summary["casts"] == inv["total_cases"], summary
        assert proof["mode"] == "port-04-test"
        assert proof["totals"]["failed"] == 0 and proof["totals"]["cases"] == inv["total_cases"], proof
    else:
        assert len(meta_files) == 1, meta_files
        meta = json.loads(meta_files[0].read_text())
        unmodified = json.loads((root / "validator/artifacts/libvips-safe-final/port-04-test/results/libvips/summary.json").read_text())
        adjusted = json.loads((root / "validator/artifacts/libvips-safe-final-approved-skip/port-04-test/results/libvips/summary.json").read_text())
        proof = json.loads((root / "validator/artifacts/libvips-safe-final-approved-skip/proof/libvips-safe-validation-proof.json").read_text())
        assert unmodified["cases"] == inv["total_cases"] and unmodified["failed"] == 1, unmodified
        assert adjusted["cases"] == meta["expected_total_cases"] == inv["total_cases"] - 1, adjusted
        assert adjusted["source_cases"] == meta["expected_source_cases"], adjusted
        assert adjusted["usage_cases"] == meta["expected_usage_cases"], adjusted
        assert adjusted["failed"] == 0 and adjusted["passed"] == adjusted["cases"] and adjusted["casts"] == adjusted["cases"], adjusted
        assert proof["mode"] == "port-04-test"
        assert proof["totals"]["failed"] == 0 and proof["totals"]["cases"] == adjusted["cases"], proof
    PY
    rg -n "validator checkout|active validator|checks executed|failures found|fixes applied|Final Commands Executed|Final Evidence" "$ROOT/validator-report.md"
    ```

**Verification:**

- `cargo test --all-features -- --nocapture`.
- `safe/scripts/run_release_gate.sh`.
- Final full unmodified validator matrix with `ARTIFACT_NAME=libvips-safe-final`.
- No-skip path: shared clean validator result assertion, then `tools/verify_proof_artifacts.py` with `--require-casts` and inventory-derived thresholds.
- Approved-skip path: adjusted full matrix, shared clean validator result assertion, proof, and site from `skip-meta.json` thresholds, plus strict approved-skip verification.
- `tools/render_site.py`.
- `scripts/verify-site.sh`.
- Validator immutability diff.

**Success Criteria:**

- Final local tests, release gate, package rebuild, staging, and lock refresh all complete from the final committed source state.
- The final unmodified validator run exists under `validator/artifacts/libvips-safe-final/**`.
- No-skip path produces clean final matrix, proof, site, and site verification artifacts; approved-skip path preserves unmodified failing evidence and produces clean adjusted proof and site.
- `validator-report.md` records active validator details, commands, failures, fixes, regression tests, package hashes, proof/site paths, approved skips or `None`, and final totals.
- Both verifier phases pass, including validator immutability and git hygiene checks.

**Git Commit Requirement:**

The implementer must commit this phase's work to git before yielding. If the phase makes no production changes, it must still commit a `validator-report.md` update or create an explicit `--allow-empty` commit documenting the no-op and the evidence checked. Generated validator artifacts, override packages, proof/site output, Docker outputs, Debian package outputs, and build directories must not be committed.
