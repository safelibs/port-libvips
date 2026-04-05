# Phase 09

## Phase Name
Final Fixups, Full Release Gate, And Agentic Reviews

## Implement Phase ID
`impl_09_final_fixups`

## Preexisting Inputs
- Everything produced or updated by phases 1 through 8.
- `safe/scripts/run_release_gate.sh`
- `safe/tests/dependents/`
- `dependents.json`
- `test-original.sh`

## New Outputs
- Updated `safe/scripts/run_release_gate.sh`
- Final catch-all fixes across any files touched in phases 1 through 8

## File Changes
- Fold any final compatibility fixes back into the real owning modules instead of adding one-off release-gate hacks.
- Make the release gate the single executable summary of the completed port.

## Implementation Details
- Use this phase only for residual issues discovered by earlier checks, package extraction, link-compat relinks, upstream suites, or the fixed twelve-application harness.
- Do not weaken prior assertions to make the release gate pass. Fix the underlying runtime, ABI, packaging, or harness bug instead.
- The updated release gate must set `VIPS_SAFE_BUILD_DIR` before invoking the upstream pytest wrapper, must verify packaged libraries against the committed symbol manifests, must unpack `libvips42t64`, `libvips-dev`, `libvips-tools`, and `gir1.2-vips-8.0` into one temporary root and use `<temp>/usr` as the extracted package prefix, must run the packaged `vips -l operation` and `safe/scripts/compare_module_registry.py` against that extracted prefix, must run both `check_introspection.sh` and `g-ir-inspect` for installed and extracted-package introspection, must compile `safe/tests/link_compat/deprecated_c_api_smoke.c` against `/home/yans/code/safelibs/ported/libvips/build-check-install/lib/pkgconfig/vips.pc`, relink that object once against the release-gate safe install prefix and once against the assembled extracted-package `/usr` prefix, and execute both binaries under the matching `LD_LIBRARY_PATH` and `VIPSHOME`, and must invoke `test-original.sh` with `LIBVIPS_USE_EXISTING_DEBS=1` only after the workspace already contains the freshly built `.deb` set that the container harness is required to reuse.
- Preserve linear workflow semantics: the tester and senior-review phases both bounce only to this final implement phase.

## Verification Phases
### `check_09_release_gate`
- Type: `check`
- Fixed `bounce_target`: `impl_09_final_fixups`
- Purpose: run the full machine-executable release gate over Cargo, Meson, upstream wrappers, deprecated and non-deprecated link compatibility, Debian packages, and the twelve-application harness.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
scripts/run_release_gate.sh
```

### `check_09_tester_review`
- Type: `check`
- Fixed `bounce_target`: `impl_09_final_fixups`
- Purpose: perform a tester-style review focused on behavioural regressions, missing coverage, flaky command usage, and harness blind spots after the full release gate passes.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips
git log --oneline --reverse --max-count=30
cd /home/yans/code/safelibs/ported/libvips/safe
rg -n '\bunsafe\b' src tests
```
- Review checks it should perform:
- prioritize bugs, behavioural regressions, missing tests, and unchecked compatibility claims;
- verify every major fix has a reproducer in Rust tests, upstream wrappers, link-compat relinks, package checks, or the application harness;
- verify no stage still depends on copied reference libraries.

### `check_09_senior_review`
- Type: `check`
- Fixed `bounce_target`: `impl_09_final_fixups`
- Purpose: perform a senior-review pass focused on architecture, long-term maintainability, safety boundaries, and workflow integrity.
- Commands it should run:
```bash
cd /home/yans/code/safelibs/ported/libvips/safe
cargo test --all-features -- --nocapture
```
- Review checks it should perform:
- verify `unsafe` is limited to unavoidable FFI and raw-pointer boundaries;
- verify Meson, Debian, and Docker workflows all consume the same safe-produced artifacts;
- verify the final workflow remains linear and every checker would bounce only to `impl_09_final_fixups`.

## Success Criteria
- `check_09_release_gate`, `check_09_tester_review`, and `check_09_senior_review` all pass without modification.
- `safe/scripts/run_release_gate.sh` becomes the single executable summary of the completed port.

## Git Commit Requirement
The implementer must commit work to git before yielding.
