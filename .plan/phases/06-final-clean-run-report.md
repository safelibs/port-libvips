# Phase 6: Final Clean Run And Report

## Phase Name
Produce final clean validator evidence, proof/site artifacts, and closing report

## Implement Phase ID
`impl_06_final_clean_run_and_report`

## Preexisting Inputs
- `validator/`
- `validator/.venv/`
- `validator-report.md`
- `validator-overrides/libvips/*.deb`
- `.work/validation/port-deb-lock.json`
- `.work/validation/artifacts/port/results/libvips/*.json`
- `validator/artifacts/libvips-safe-baseline-current/`
- `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- `validator/artifacts/libvips-safe-source-api/`
- `validator/artifacts/libvips-safe-source-api-port-lock.json`
- `validator/artifacts/libvips-safe-ops/`
- `validator/artifacts/libvips-safe-ops-port-lock.json`
- `validator/artifacts/libvips-safe-foreign/`
- `validator/artifacts/libvips-safe-foreign-port-lock.json`
- `validator/artifacts/libvips-safe-remaining/`
- `validator/artifacts/libvips-safe-remaining-port-lock.json`
- `validator/artifacts/libvips-safe-final/`
- `validator/site/libvips-safe-final/`
- `scripts/check-layout.sh`
- `scripts/build-debs.sh`
- `scripts/run-validation-tests.sh`
- `scripts/lib/build_port_lock.py`
- `safe/**`
- `safe/scripts/run_release_gate.sh`
- `original/**`
- `safe/reference/**`
- `safe/vendor/pyvips-3.1.1/**`
- `safe/tests/upstream/**`
- `safe/tests/dependents/**`
- `all_cves.json`
- `relevant_cves.json`
- `dependents.json`

## New Outputs
- Fresh final packages under `dist/` and `validator-overrides/libvips/`.
- Full canonical `validator/artifacts/libvips-safe-final-port-lock.json`.
- Final matrix artifact `validator/artifacts/libvips-safe-final/` and `matrix-exit-code.txt`.
- If an approved validator-bug skip exists, `validator/artifacts/libvips-safe-final-unmodified/` plus passing transient artifact `validator/artifacts/libvips-safe-final/` with adjusted counts and `.work/validator-final-approved/approved-skip-manifest.json`.
- Proof `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`.
- Rendered site under `validator/site/libvips-safe-final/`, including `site-data.json`.
- Exactly one active `## Final Clean Run` section.
- Final git commit `impl_06 record final validator clean run`.

## File Changes
- `validator-report.md`: replace or complete the unique active `## Final Clean Run` section with final validator/source commits, package hashes, final matrix/proof/site paths, and any approved skip audit details.
- Do not patch production code, tests, package scripts, validator tests, or tracked validator source in Phase 6. New ordinary defects must become a committed `## Final Clean Run Blocked` report and a failed phase.

## Implementation Details
- Phase 6 is evidence-only. Confirm the real validator checkout is clean, still on the Phase 1 recorded validator commit, and equal to `origin/main` without fetching or pulling. If a newer validator is required, commit a `## Final Clean Run Blocked` report and fail.
- Set `FINAL_SOURCE_COMMIT=$(git rev-parse HEAD)`. This exact commit is used for final package build, final lock, final matrix, proof, and site evidence. Post-phase checks must read `Final source commit` from the final report, not `git rev-parse HEAD` after the report commit.
- Run `bash scripts/check-layout.sh` and `cd safe && cargo test --all-features -- --nocapture`.
- Build final packages with `SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" bash scripts/build-debs.sh`, then run `cd safe && scripts/run_release_gate.sh`. If release gate fails for an ordinary defect, commit a blocker report and fail instead of patching code.
- Rewrite `validator-overrides/libvips/` and `validator/artifacts/libvips-safe-final-port-lock.json` exactly once from the final `dist/*.deb` files using `SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT"`. Assert canonical package order, hashes, sizes, commit/tag/release tag, and `unported_original_packages == []`.
- Normal final path: run real validator `test.sh --mode port --library libvips` into `validator/artifacts/libvips-safe-final/` with final lock, override root, `PYTHON="$ROOT/validator/.venv/bin/python"`, and `--record-casts`; write `matrix-exit-code.txt`; parse summary and every result JSON. A clean final artifact requires matrix exit `0`, `failed == 0`, canonical port packages, override debs installed, and no original-package fallback.
- If the unmodified final root fails from ordinary libvips-safe, package, dependency, timeout, environment, or unknown causes, append `## Final Clean Run Blocked` with failed testcase ids, artifact paths, and the reason a new linear workflow is required. Commit that report and fail.
- Phase 6 validator-bug skip procedure is self-contained. It may be used only after the final unmodified artifact fails exactly validator-bug testcase ids, or when the unique active Phase 5 section already documents approved ids and the final unmodified artifact under `validator/artifacts/libvips-safe-final-unmodified/` fails exactly those same ids. The approved ids and adjusted counts must be written in the unique active `## Final Clean Run` section before proof/site generation; never read them from Phase 5 or historical evidence.

Phase 6 approved-skip commands and checks:

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
APPROVED_VALIDATOR="$ROOT/.work/validator-final-approved"
APPROVED_SKIP_MANIFEST="$APPROVED_VALIDATOR/approved-skip-manifest.json"
FINAL_LOCK="$ROOT/validator/artifacts/libvips-safe-final-port-lock.json"
FINAL_ARTIFACT="$ROOT/validator/artifacts/libvips-safe-final"
FINAL_UNMODIFIED="$ROOT/validator/artifacts/libvips-safe-final-unmodified"

git -C "$ROOT/validator" status --porcelain --untracked-files=no | tee "$ROOT/.work/validator-final-approved-status-before.txt"
test ! -s "$ROOT/.work/validator-final-approved-status-before.txt"
rm -rf "$APPROVED_VALIDATOR"
mkdir -p "$ROOT/.work"
rsync -a --delete --exclude '.git' --exclude '.venv' --exclude 'artifacts' --exclude 'site' "$ROOT/validator/" "$APPROVED_VALIDATOR/"
```

```bash
APPROVED_SKIP_IDS="<space-separated ids from the unique active Final Clean Run section>" \
ORIGINAL_VALIDATOR="$ROOT/validator" \
APPROVED_VALIDATOR="$APPROVED_VALIDATOR" \
APPROVED_SKIP_MANIFEST="$APPROVED_SKIP_MANIFEST" \
  "$ROOT/validator/.venv/bin/python" - <<'PY'
import json, os, re
from pathlib import Path
original = Path(os.environ["ORIGINAL_VALIDATOR"])
approved = Path(os.environ["APPROVED_VALIDATOR"])
ids = [x for x in os.environ["APPROVED_SKIP_IDS"].split() if x]
if not ids:
    raise SystemExit("approved skip requires ids")
id_re = re.compile(r"^[a-z0-9][a-z0-9-]{1,78}[a-z0-9]$")
for testcase_id in ids:
    if not id_re.fullmatch(testcase_id):
        raise SystemExit(f"invalid testcase id: {testcase_id}")

def discover(root):
    found, counts = {}, {"source": 0, "usage": 0}
    for kind in ("source", "usage"):
        for path in sorted((root / "tests/libvips/tests/cases" / kind).glob("*.sh")):
            text = path.read_text()
            blocks = re.findall(r"(?m)^#\s*@testcase:\s*([^\s#]+)\s*$", text)
            counts[kind] += len(blocks)
            for testcase_id in blocks:
                found.setdefault(testcase_id, []).append((kind, path, len(blocks)))
    return found, counts

original_cases, original_counts = discover(original)
removed = []
for testcase_id in ids:
    matches = original_cases.get(testcase_id, [])
    if len(matches) != 1:
        raise SystemExit(f"{testcase_id}: expected one original script, found {len(matches)}")
    kind, original_path, block_count = matches[0]
    if block_count != 1:
        raise SystemExit(f"{testcase_id}: script has {block_count} testcase headers")
    rel = original_path.relative_to(original)
    copy_path = approved / rel
    if not copy_path.is_file() or copy_path.read_bytes() != original_path.read_bytes():
        raise SystemExit(f"{testcase_id}: invalid transient copy {copy_path}")
    copy_path.unlink()
    removed.append({"testcase_id": testcase_id, "kind": kind, "original_path": str(original_path), "removed_copy_path": str(copy_path)})

approved_cases, adjusted_counts = discover(approved)
missing = sorted(set(original_cases) - set(approved_cases))
if missing != sorted(ids):
    raise SystemExit(f"transient copy removed unexpected testcase ids: {missing!r}")
adjusted = {"source": adjusted_counts["source"], "usage": adjusted_counts["usage"], "total": adjusted_counts["source"] + adjusted_counts["usage"]}
manifest = {"library": "libvips", "approved_skip_ids": ids, "removed": removed, "original_counts": {"source": original_counts["source"], "usage": original_counts["usage"], "total": original_counts["source"] + original_counts["usage"]}, "adjusted_counts": adjusted}
Path(os.environ["APPROVED_SKIP_MANIFEST"]).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(adjusted["source"], adjusted["usage"], adjusted["total"])
PY
```

```bash
read ADJUSTED_SOURCE_CASES ADJUSTED_USAGE_CASES ADJUSTED_TOTAL_CASES < <(
  APPROVED_SKIP_MANIFEST="$APPROVED_SKIP_MANIFEST" "$ROOT/validator/.venv/bin/python" - <<'PY'
import json, os
from pathlib import Path
counts = json.loads(Path(os.environ["APPROVED_SKIP_MANIFEST"]).read_text())["adjusted_counts"]
print(counts["source"], counts["usage"], counts["total"])
PY
)
"$ROOT/validator/.venv/bin/python" "$APPROVED_VALIDATOR/tools/testcases.py" \
  --config "$APPROVED_VALIDATOR/repositories.yml" \
  --tests-root "$APPROVED_VALIDATOR/tests" \
  --library libvips --check --list-summary \
  --min-source-cases "$ADJUSTED_SOURCE_CASES" \
  --min-usage-cases "$ADJUSTED_USAGE_CASES" \
  --min-cases "$ADJUSTED_TOTAL_CASES"
git -C "$ROOT/validator" status --porcelain --untracked-files=no | tee "$ROOT/.work/validator-final-approved-status-after.txt"
test ! -s "$ROOT/.work/validator-final-approved-status-after.txt"
rm -rf "$FINAL_ARTIFACT"
cd "$APPROVED_VALIDATOR"
set +e
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml --tests-root tests --artifact-root "$FINAL_ARTIFACT" \
  --mode port --library libvips --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$FINAL_LOCK" --record-casts
MATRIX_EXIT=$?
set -e
printf '%s\n' "$MATRIX_EXIT" > "$FINAL_ARTIFACT/matrix-exit-code.txt"
```

- After the transient run, assert the manifest removed exactly the documented ids and no others, final transient summary has `failed == 0`, every result uses override debs and canonical port packages with no original fallback, and summary counts equal adjusted counts. Update the same unique final section with `Approved skip adjusted counts: source=<n> usage=<n> total=<n>` and re-parse only that bounded final section before proof/site generation.
- Generate proof. For a normal clean run, use the real validator checkout with thresholds `5`, `170`, `175`. For an approved skip, run `tools/verify_proof_artifacts.py` from `.work/validator-final-approved` with the adjusted counts parsed only from the active final section, while writing proof under `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`.
- Render and verify the site. For a normal clean run, run `tools/render_site.py` and `scripts/verify-site.sh` from `validator/`. For an approved skip, run them from `.work/validator-final-approved`, but keep artifact/proof/site roots under `$ROOT/validator` and put `$ROOT/validator/.venv/bin` first in `PATH` for `verify-site.sh`.
- Complete or replace the unique `## Final Clean Run` section with `Final validator commit`, `Final source commit`, checks executed, package hashes, final summary counts, session failures, fixes applied, regression tests added, approved skip details if any, and final matrix/proof/site paths.
- Restore any build-only `safe/debian/changelog` stamp to `FINAL_SOURCE_COMMIT`, assert exactly one active final heading remains, and commit `validator-report.md` as `impl_06 record final validator clean run`.

## Verification Phases
### `check_06_final_clean_run_software_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_06_final_clean_run_and_report`
- Purpose: Verify final full validator run, proof, site verification, package lock, and report all agree.
- Required preexisting inputs:
  - `validator/`
  - `validator/.venv/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-final-port-lock.json`
  - `validator/artifacts/libvips-safe-final/`
  - `validator/artifacts/libvips-safe-final/port/results/libvips/*.json`
  - `validator/artifacts/libvips-safe-final/matrix-exit-code.txt`
  - `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`
  - `validator/site/libvips-safe-final/site-data.json`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - `python3 -m json.tool validator/artifacts/libvips-safe-final-port-lock.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-final/port/results/libvips/summary.json >/dev/null`
  - `python3 -m json.tool validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json >/dev/null`
  - `test -f validator/site/libvips-safe-final/site-data.json`
  - run a Python assertion that parses only the unique `## Final Clean Run` section for `Final validator commit` and `Final source commit`; without approved skips, require matrix exit `0`, summary failed `0`, canonical final lock ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"], `unported_original_packages == []`, override files matching lock hashes/sizes, lock commit/tag matching final source commit, result JSON package proof, proof counts, and site data. With approved skips, require `.work/validator-final-approved/approved-skip-manifest.json`, final unmodified artifact failing only documented ids, final transient artifact passing with adjusted counts, canonical final lock, clean real validator checkout, and result JSON package proof.
  - for unmodified clean runs, rerender to `.work/final-site-render-check` from `validator` and compare `site-data.json`, then run `PATH="$PWD/validator/.venv/bin:$PATH" bash validator/scripts/verify-site.sh` with final artifact, proof, site root, and library libvips.
  - for approved-skip runs, rerender from `.work/validator-final-approved` to `.work/final-site-render-check`, compare `site-data.json`, then run `scripts/verify-site.sh` from the transient copy with absolute final artifact/proof/site roots.
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`

### `check_06_final_clean_run_senior_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_06_final_clean_run_and_report`
- Purpose: Final review of the report, failure-to-fix traceability, test coverage, and git history.
- Required preexisting inputs:
  - `validator/`
  - `validator-report.md`
  - `validator-overrides/libvips/*.deb`
  - `validator/artifacts/libvips-safe-final-port-lock.json`
  - `validator/artifacts/libvips-safe-final/port/results/libvips/summary.json`
  - `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`
  - `validator/site/libvips-safe-final/site-data.json`
  - `safe/src/**`
- Commands:
  - `cd /home/yans/safelibs/pipeline/ports/port-libvips`
  - assert exactly one exact `## Final Clean Run` active heading exists and inspect only that bounded final section for final evidence fields.
  - `git log --oneline --decorate -n 12`
  - `git status --short --branch`
  - `rg -n '\bunsafe\b|todo!|unimplemented!|panic!\(' safe/src || true`
  - verify the active final section names validator commit, checks executed, failures found, fixes applied, approved skips, package hashes, and final artifact paths.
  - confirm every failure row has a regression test or documented validator-bug justification; confirm any unsafe/todo/unimplemented/panic match is preexisting or justified, with no new `todo!` or `unimplemented!` left in `safe/src`.

## Success Criteria
- The Phase 1 validator commit in `port` mode produces acceptable final evidence for libvips.
- Without approved skips, final matrix exit is `0`, final summary has zero failures, proof generation succeeds with required casts, and site rendering/verification succeeds.
- With an approved validator bug, the report includes the unmodified failing artifact, exact skipped ids, adjusted counts, passing transient artifact, full canonical package evidence, and proof/site evidence from the transient copy.
- Ordinary libvips-safe or packaging failures are not accepted in Phase 6.

## Post-Phase-6 Final Verification
After all implementation phases, run this verification from `/home/yans/safelibs/pipeline/ports/port-libvips`. It consumes the `Final source commit` recorded in the unique active `## Final Clean Run` section; it must not use `git rev-parse HEAD` after Phase 6 because `HEAD` is normally the report-only final commit.

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
read FINAL_VALIDATOR_COMMIT FINAL_SOURCE_COMMIT < <(
  python3 - <<'PY'
import re
from pathlib import Path

text = Path("validator-report.md").read_text()
heading = "## Final Clean Run"
matches = list(re.finditer(r"(?m)^" + re.escape(heading) + r"$", text))
if len(matches) != 1:
    raise SystemExit(f"expected exactly one active {heading!r}, found {len(matches)}")
start = matches[0].start()
next_heading = re.search(r"(?m)^## ", text[matches[0].end():])
end = matches[0].end() + next_heading.start() if next_heading else len(text)
section = text[start:end]
validator = re.search(r"(?m)^Final validator commit: ([0-9a-f]{40})$", section)
source = re.search(r"(?m)^Final source commit: ([0-9a-f]{40})$", section)
if validator is None or source is None:
    raise SystemExit("missing final validator/source commit lines")
print(validator.group(1), source.group(1))
PY
)
test -z "$(git -C "$ROOT/validator" status --porcelain --untracked-files=no)"
test "$(git -C "$ROOT/validator" rev-parse HEAD)" = "$FINAL_VALIDATOR_COMMIT"
test "$(git -C "$ROOT" cat-file -t "$FINAL_SOURCE_COMMIT")" = commit
bash scripts/check-layout.sh
cd safe
cargo test --all-features -- --nocapture
cd ..
SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" bash scripts/build-debs.sh
cd safe
scripts/run_release_gate.sh
cd ..
PYTHON="$ROOT/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh
```

After `scripts/run-validation-tests.sh`, parse `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/summary.json`, and every testcase result JSON under `.work/validation/artifacts/port/results/libvips/`; do not rely on hook exit status alone. The `.work/validation` lock must have `debs[*].package == ["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]` and `unported_original_packages == []`, and every testcase result must have `override_debs_installed: true`, the same four `port_debs` packages, and no `unported_original_packages`. Without approved validator-bug skips, `.work/validation` summary must report `failed == 0`. With an approved validator-bug skip, `.work/validation` must fail only the documented testcase ids. If this post-phase check leaves only an automated `safe/debian/changelog` stamp, restore it before yielding.

Do not rerun the controlled final validator/proof/site sequence against official paths after Phase 6. Instead, verify the existing official Phase 6 artifacts using:

- final artifact root: `validator/artifacts/libvips-safe-final`
- final port lock: `validator/artifacts/libvips-safe-final-port-lock.json`
- local override root: `validator-overrides`
- validator mode: `port`
- library: `libvips`
- proof output: `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`
- expected current counts: 5 source cases, 170 usage cases, 175 total cases

If an approved validator-bug skip exists, verify the unmodified failing root at `validator/artifacts/libvips-safe-final-unmodified` and use the adjusted counts recorded in the unique active `## Final Clean Run` section for approved transient-skip proof.

If explicitly required to run a fresh controlled final rerun after Phase 6, write the rerun to scratch paths under `.work/final-verification-rerun/` using `FINAL_SOURCE_COMMIT`, a scratch lock, and a scratch override root. Do not overwrite `validator/artifacts/libvips-safe-final*`, `validator/artifacts/libvips-safe-final-port-lock.json`, `validator-overrides/libvips/`, or `validator/site/libvips-safe-final/`. To make a post-phase rerun official, update `validator-report.md` with the new artifact paths, package hashes, proof/site evidence, and source commit, then commit that report update.

Final acceptance criteria:

- Current validator checkout is on the Phase 1 recorded `origin/main` commit and has no tracked local diffs.
- Without approved validator-bug skips, `validator/artifacts/libvips-safe-final/matrix-exit-code.txt` contains `0`.
- Without approved validator-bug skips, `validator/artifacts/libvips-safe-final/port/results/libvips/summary.json` reports `failed == 0`.
- Without approved validator-bug skips, `.work/validation/artifacts/port/results/libvips/summary.json` reports `failed == 0` for the fresh CI-parity hook run.
- With an approved validator-bug skip, `validator/artifacts/libvips-safe-final-unmodified/` fails only the documented buggy testcase ids and `validator/artifacts/libvips-safe-final/` is the passing transient-skip artifact with adjusted counts.
- Every final testcase result has `override_debs_installed: true`, the full four canonical libvips `port_debs` list, and `unported_original_packages: []`.
- The final port lock commit, tag ref, and release tag match the `Final source commit` line in the unique active `## Final Clean Run` section; the final port lock has exactly the canonical package list `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]` in order, has `unported_original_packages: []`, matches `validator-overrides/libvips/*.deb`, and every final testcase result's `port_debs` list matches the final lock with no `unported_original_packages`.
- Proof generation with `--require-casts` succeeds.
- Site rendering and `scripts/verify-site.sh` succeed.
- `validator-report.md` names the validator commit, safe source commit, checks executed, failures found, fixes applied, regression tests, package hashes, and final artifact paths.
- Git history contains a commit from every implement phase that changed source, tests, or report content.

## Git Commit Requirement
The implementer must commit the phase work to git before yielding. Source/test/package fixes must be committed before official package evidence, and the report-only commit must be made after the phase evidence is recorded. Check phases must not commit.
