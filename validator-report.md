# Validator Report

## Phase 1 Current Validator Baseline

Phase start commit: e7f1354519acf1df3b49e7a9e26c162df33b2676
Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
Source commit: e7f1354519acf1df3b49e7a9e26c162df33b2676
Source fix commits: none

This is the refreshed current-validator baseline for `libvips` in `port` mode. The validator checkout was fast-forwarded once on `main`, then the baseline packages, override packages, lock, matrix artifacts, and report evidence were generated from the phase-start source commit above.

### Dependency-hook status

- Command: `bash scripts/install-build-deps.sh`
- Outcome: success, exit status `0`
- Ordering: completed before the first official `SAFELIBS_COMMIT_SHA=e7f1354519acf1df3b49e7a9e26c162df33b2676 bash scripts/build-debs.sh` package build.
- Environment result: existing apt dependencies were already present; Rust toolchain `1.82.0` was refreshed or verified by rustup.

### Validator and source commits

- Validator URL: https://github.com/safelibs/validator
- Validator branch: `main`
- Phase-start validator pre-pull local commit: `9ae971508c9381f32a531078037851d960cab61f`
- Phase-start validator pre-pull remote `origin/main`: `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`
- Active validator commit after the single fast-forward update: `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`
- Source commit used for package build, override debs, lock synthesis, and matrix: `e7f1354519acf1df3b49e7a9e26c162df33b2676`
- Source fix commits: none
- `packaging/package.env` still provides `SAFELIBS_LIBRARY="libvips"`.

### Testcase counts

- Pre-pull libvips testcase inventory from the source plan: `5 source, 240 usage, 4 regression, 249 total`
- Post-update inventory path: `validator/artifacts/libvips-safe-baseline-current-testcases.txt`

| Library | Source cases | Usage cases | Regression cases | Total cases |
| --- | ---: | ---: | ---: | ---: |
| libvips | 5 | 250 | 4 | 259 |

### Lock package table

- Lock path: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Release tag: `build-e7f1354519ac`
- Tag ref: `refs/tags/build-e7f1354519ac`
- Canonical validator package order: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- `unported_original_packages == []`: confirmed

| Package | Deb filename | SHA256 | Size |
| --- | --- | --- | ---: |
| `libvips42t64` | `libvips42t64_8.15.1-1.1build4+safelibs1778655052_amd64.deb` | `3faa9983d31477c4e4523e40188b81242ad0b447712bc98375b3322ce044577f` | 1434254 |
| `libvips-dev` | `libvips-dev_8.15.1-1.1build4+safelibs1778655052_amd64.deb` | `fc914a1016ca08654700ba28d4336127c93959b73681ef17d794a052e28b9a62` | 83414 |
| `libvips-tools` | `libvips-tools_8.15.1-1.1build4+safelibs1778655052_amd64.deb` | `562476e69e36567582d3c51cf0ba4ca74c2ad8347b122b549bcac41238c71180` | 27928 |
| `gir1.2-vips-8.0` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778655052_amd64.deb` | `e24f564ad4d7026b6f347f0c62e3a87a1001382e43553bd2c09c67a2df5e895d` | 5188 |

### Validator exit status

- Direct matrix command: `PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" --config "$ROOT/validator/repositories.yml" --tests-root "$ROOT/validator/tests" --artifact-root "$ARTIFACT_ROOT" --mode port --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$LOCK_PATH" --record-casts`
- Exit status: `0`
- Status path: `validator/artifacts/libvips-safe-baseline-current/validator-exit-status.txt`
- Tracked validator source cleanliness: clean before update, before validator unit/check-testcase commands, before the matrix, and after the matrix.

### Package-completeness status

- Status: `0`
- Status path: `validator/artifacts/libvips-safe-baseline-current/package-completeness-status.txt`
- Detail path: `validator/artifacts/libvips-safe-baseline-current/package-completeness-failures.txt`
- Assertion result: every testcase result JSON lists `apt_packages`, `port_debs`, and `override_installed_packages` in canonical order; `override_debs_installed is True`; `port_commit` equals `e7f1354519acf1df3b49e7a9e26c162df33b2676`; `unported_original_packages == []`; and `casts == cases`.

### Matrix summary

- Summary path: `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json`
- Cases: `259`
- Passed: `258`
- Failed: `1`
- Casts: `259`
- `casts == cases`: confirmed

### Failure table

| Testcase ID | Kind | Owner phase | Concise root cause | Result JSON path | Log path |
| --- | --- | --- | --- | --- | --- |
| `usage-ruby-vips-r16-text-hello-image-has-positive-width` | `usage` | Phase 3 | Ruby `Vips::Image.text("Hello")` fails because the `text` operation is not implemented in the safe libvips operation surface. | `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/usage-ruby-vips-r16-text-hello-image-has-positive-width.json` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-r16-text-hello-image-has-positive-width.log` |

### Artifact paths

- Inventory file: `validator/artifacts/libvips-safe-baseline-current-testcases.txt`
- Lock file: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- Artifact root: `validator/artifacts/libvips-safe-baseline-current/`
- Result summary: `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json`
- Validator status file: `validator/artifacts/libvips-safe-baseline-current/validator-exit-status.txt`
- Package-completeness status file: `validator/artifacts/libvips-safe-baseline-current/package-completeness-status.txt`
- Package-completeness detail file: `validator/artifacts/libvips-safe-baseline-current/package-completeness-failures.txt`
- Dist debs: `dist/gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778655052_amd64.deb`, `dist/libvips-dev_8.15.1-1.1build4+safelibs1778655052_amd64.deb`, `dist/libvips-doc_8.15.1-1.1build4+safelibs1778655052_all.deb`, `dist/libvips-tools_8.15.1-1.1build4+safelibs1778655052_amd64.deb`, `dist/libvips42t64_8.15.1-1.1build4+safelibs1778655052_amd64.deb`
- Override packages: `validator-overrides/libvips/`

### Classification counts

| Owner phase | Failure count |
| --- | ---: |
| Phase 2 | 0 |
| Phase 3 | 1 |
| Phase 4 | 0 |
| Phase 5 | 0 |

Classification counts sum to the matrix failed count: `1`.

## Historical Evidence - impl_01_update_validator_and_baseline - Phase 1 Current Validator Baseline - 9ae971508c9381f32a531078037851d960cab61f

Phase start commit: b82520f7f943976b2f62660fb9ad2a78e73a3dc3
Validator commit: 9ae971508c9381f32a531078037851d960cab61f
Source commit: b82520f7f943976b2f62660fb9ad2a78e73a3dc3
Source fix commits: none

This is the fresh current-validator baseline for `libvips` in `port` mode. It uses the committed source SHA above for package stamping, override deb generation, lock synthesis, and the validator matrix run.

### Validator Checkout

- Validator URL: https://github.com/safelibs/validator
- Active validator commit: `9ae971508c9381f32a531078037851d960cab61f`
- Active local branch: `main`
- README invocation mode: `port`
- Testcase metadata source: current per-script `# @testcase` headers, checked with `validator/tools/testcases.py`.

### Testcase Counts

| Library | Source cases | Usage cases | Regression cases | Total cases |
| --- | ---: | ---: | ---: | ---: |
| libvips | 5 | 240 | 4 | 249 |

### Package Lock

- Source commit used for package build, override debs, lock synthesis, and matrix: `b82520f7f943976b2f62660fb9ad2a78e73a3dc3`
- Lock path: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Release tag: `build-b82520f7f943`
- Tag ref: `refs/tags/build-b82520f7f943`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1440256 | `b11f8969eb675a9eb2a7b64d712744e9296ce88ba0140873cedddbd23a9712c9` | `libvips42t64_8.15.1-1.1build4+safelibs1778624203_amd64.deb` |
| `libvips-dev` | `amd64` | 83430 | `83cbf384265dda5e4970de7eb87d71c7839b69bb6de3e84361b1a39cb3ac9893` | `libvips-dev_8.15.1-1.1build4+safelibs1778624203_amd64.deb` |
| `libvips-tools` | `amd64` | 27970 | `f4bd38b41acbc06154287c606ccf4df4f785ca750686cdaa0d7221afe336094f` | `libvips-tools_8.15.1-1.1build4+safelibs1778624203_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5198 | `602763411cbb891bbfd95515fc3750ec7118584949b103e0b60341779fba2c50` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778624203_amd64.deb` |

### Baseline Command

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT/validator"
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-baseline-current \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-baseline-current-port-lock.json" \
  --record-casts
```

### Matrix Summary

- Matrix exit code: `0`
- Matrix exit path: `validator/artifacts/libvips-safe-baseline-current/matrix-exit-code.txt`
- Summary path: `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json`
- Passed: `244`
- Failed: `5`
- Casts recorded: `249`
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`

### Failure Classification

- `impl_02_source_api_surface_failures`: 0 failures
- `impl_03_operation_semantics_failures`: 4 failures
- `impl_04_foreign_io_media_failures`: 0 failures
- `impl_05_packaging_container_remaining_failures`: 1 failure

| Testcase ID | Kind | Status | Owner phase | Artifact path | Failure summary |
| --- | --- | --- | --- | --- | --- |
| `cve-2026-3284` | `regression` | `failed` | `impl_05_packaging_container_remaining_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/cve-2026-3284.log` | CVE regression: extract_area overflow coordinates were accepted instead of rejected. |
| `usage-ruby-vips-r11-add-alpha-three-to-four-bands` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-r11-add-alpha-three-to-four-bands.log` | Ruby Image#add_alpha failed because vips_addalpha is not implemented in the safe compatibility layer. |
| `usage-ruby-vips-r11-fwfft-invfft-roundtrip` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-r11-fwfft-invfft-roundtrip.log` | Ruby fwfft/invfft failed because operation dispatch reports unknown operation fwfft. |
| `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band.log` | Ruby colourspace(:b_w) returned 3 bands; expected the greyscale conversion to return 1 band. |
| `usage-ruby-vips-r12-composite-over-yields-input-bands` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-r12-composite-over-yields-input-bands.log` | Ruby composite2(:over) rejected the compositing_space option, so default compositor argument handling is incomplete. |

## Historical Evidence - Phase 1 Current Validator Baseline (pre-validator-9ae97150)

This report replaces stale fallback-era evidence with the current validator `port`-mode baseline for `libvips`. The old 85-case `validator/artifacts/libvips-safe-final/port-04-test/` artifacts are historical context only and were not used as active validation evidence for this phase.

### Validator Checkout

- Validator URL: https://github.com/safelibs/validator
- Initial detached validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf`
- Plan-time `origin/main` commit: `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Active Phase 1 validator commit: `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Active local branch: `main`
- README invocation mode: mode: port
- Testcase metadata source: current per-script `# @testcase` headers, checked with `validator/tools/testcases.py`.

### Testcase Counts

| Library | Source cases | Usage cases | Total cases |
| --- | ---: | ---: | ---: |
| libvips | 5 | 170 | 175 |

### Package Lock

- Port commit used for the synthetic release tag: `bd3bd7c37f01d3e7864708217220223bf63ba291`
- Release tag: `build-bd3bd7c37f01`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Lock path: `validator/artifacts/libvips-safe-baseline-current-port-lock.json`
- Override root: `validator-overrides/libvips/`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1390008 | `b5efe0c8e35fe486c2f0b5a1963d186ea88fc70e5b638e4334a7b5e8232d273f` | `libvips42t64_8.15.1-1.1build4+safelibs1777841828_amd64.deb` |
| `libvips-dev` | `amd64` | 83410 | `5120a1b2763c0b9846631598c546e873d5986f8ea09635de64ec1dc127b68b70` | `libvips-dev_8.15.1-1.1build4+safelibs1777841828_amd64.deb` |
| `libvips-tools` | `amd64` | 27940 | `177140183e5692374b4f954cec6ff60f4a8820cadfa33347a1a049ae4550da00` | `libvips-tools_8.15.1-1.1build4+safelibs1777841828_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5192 | `cf6c56ecaad894092c1d42d894ec6972863bfda209c5ba3838ecdee9d3d08fe9` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1777841828_amd64.deb` |

### Baseline Command

```bash
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-baseline-current \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-baseline-current-port-lock.json" \
  --record-casts
```

### Matrix Summary

- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-baseline-current/port/results/libvips/summary.json`
- Passed: `116`
- Failed: `59`
- Casts recorded: `175`

### Failure Classification

- `impl_03_operation_semantics_failures`: 7 failures
- `impl_04_foreign_io_media_failures`: 52 failures
- `impl_02_source_api_surface_failures`: 0 failures
- `impl_05_packaging_container_remaining_failures`: 0 failures

| Testcase ID | Kind | Status | Owner phase | Artifact path | Failure summary |
| --- | --- | --- | --- | --- | --- |
| `usage-ruby-vips-abs-of-signed-image` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-abs-of-signed-image.log` | not a PNG: /tmp/validator-tmp/abs.png: data |
| `usage-ruby-vips-affine-rotation` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-affine-rotation.log` | not a PNG |
| `usage-ruby-vips-affine-shear` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-affine-shear.log` | not a PNG: /tmp/validator-tmp/shear.png: data |
| `usage-ruby-vips-arithmetic-multiply-divide` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-arithmetic-multiply-divide.log` | not a TIFF |
| `usage-ruby-vips-arrayjoin-grid` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-arrayjoin-grid.log` | not a PNG: /tmp/validator-tmp/grid.png: data |
| `usage-ruby-vips-arrayjoin-vertical-stack` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-arrayjoin-vertical-stack.log` | not a PNG: /tmp/validator-tmp/stack.png: data |
| `usage-ruby-vips-autorot-no-orientation` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-autorot-no-orientation.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:228:in `build': autorot: operation not implemented (Vips::Error) |
| `usage-ruby-vips-bandfold-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-bandfold-roundtrip.log` | not a TIFF: /tmp/validator-tmp/fold.tif: data |
| `usage-ruby-vips-bandjoin-extract-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-bandjoin-extract-roundtrip.log` | not a PNG |
| `usage-ruby-vips-canny-edges` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-canny-edges.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:228:in `build': canny: operation not implemented (Vips::Error) |
| `usage-ruby-vips-colourspace-bw` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-colourspace-bw.log` | not a PNG |
| `usage-ruby-vips-colourspace-hsv-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-colourspace-hsv-roundtrip.log` | not a PNG: /tmp/validator-tmp/hsv.png: data |
| `usage-ruby-vips-composite-over` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-composite-over.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:381:in `call': unable to call composite: you supplied 2 arguments, but operation needs 0. (Vips::Error) |
| `usage-ruby-vips-conv-custom-kernel` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-conv-custom-kernel.log` | not a TIFF: /tmp/validator-tmp/conv.tif: data |
| `usage-ruby-vips-dilate-cross-mask` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-dilate-cross-mask.log` | not a PNG: /tmp/validator-tmp/dilate.png: data |
| `usage-ruby-vips-draw-circle-mutable` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-draw-circle-mutable.log` | not a PNG: /tmp/validator-tmp/circle.png: data |
| `usage-ruby-vips-draw-line-mutable` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-draw-line-mutable.log` | not a PNG: /tmp/validator-tmp/line.png: data |
| `usage-ruby-vips-draw-rect-mutable` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-draw-rect-mutable.log` | not a PNG: /tmp/validator-tmp/rect.png: data |
| `usage-ruby-vips-embed-extend-background-color` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-embed-extend-background-color.log` | not a PNG: /tmp/validator-tmp/embed_bg.png: data |
| `usage-ruby-vips-embed-extend-modes` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-embed-extend-modes.log` | not a PNG |
| `usage-ruby-vips-erode-cross-mask` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-erode-cross-mask.log` | not a PNG: /tmp/validator-tmp/erode.png: data |
| `usage-ruby-vips-extract-band-two-at-offset` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-extract-band-two-at-offset.log` | not a TIFF: /tmp/validator-tmp/extract-two.tif: data |
| `usage-ruby-vips-falsecolour-grayscale` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-falsecolour-grayscale.log` | not a PNG: /tmp/validator-tmp/false.png: data |
| `usage-ruby-vips-find-trim-bbox` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-find-trim-bbox.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:228:in `build': find_trim: operation not implemented (Vips::Error) |
| `usage-ruby-vips-find-trim-custom-threshold` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-find-trim-custom-threshold.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:228:in `build': find_trim: operation not implemented (Vips::Error) |
| `usage-ruby-vips-gamma-explicit-exponent` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-gamma-explicit-exponent.log` | not a PNG: /tmp/validator-tmp/gamma.png: data |
| `usage-ruby-vips-gaussnoise-generator` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-gaussnoise-generator.log` | not a PNG: /tmp/validator-tmp/gaussnoise.png: data |
| `usage-ruby-vips-gravity-east-west` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-gravity-east-west.log` | not a PNG: /tmp/validator-tmp/gravity_ew.png: data |
| `usage-ruby-vips-gravity-placement` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-gravity-placement.log` | not a PNG: /tmp/validator-tmp/gravity.png: data |
| `usage-ruby-vips-grid-tile-layout` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-grid-tile-layout.log` | not a PNG: /tmp/validator-tmp/grid.png: data |
| `usage-ruby-vips-hist-equal-histogram` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-hist-equal-histogram.log` | not a PNG: /tmp/validator-tmp/hist_equal.png: data |
| `usage-ruby-vips-hist-local-equalisation` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-hist-local-equalisation.log` | not a PNG: /tmp/validator-tmp/hist_local.png: data |
| `usage-ruby-vips-hist-norm-stretch` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-hist-norm-stretch.log` | -:15:in `<main>': out dims (RuntimeError) |
| `usage-ruby-vips-ifthenelse-comparison-mask` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-ifthenelse-comparison-mask.log` | not a PNG: /tmp/validator-tmp/ifthenelse_cmp.png: data |
| `usage-ruby-vips-ifthenelse-multiband-sources` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-ifthenelse-multiband-sources.log` | not a PNG: /tmp/validator-tmp/ifthenelse.png: data |
| `usage-ruby-vips-invert-roundtrip-identity` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-invert-roundtrip-identity.log` | not a PNG: /tmp/validator-tmp/roundtrip.png: data |
| `usage-ruby-vips-jpeg-quality-buffer` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-jpeg-quality-buffer.log` | -:12:in `<main>': expected high quality > low quality (12339 vs 12339) (RuntimeError) |
| `usage-ruby-vips-matrixload-external-file` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-matrixload-external-file.log` | /usr/lib/ruby/vendor_ruby/vips/operation.rb:228:in `build': matrixload: matrix header requires width height scale offset (Vips::Error) |
| `usage-ruby-vips-memory-ppm-roundtrip-batch11` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-memory-ppm-roundtrip-batch11.log` | /usr/lib/ruby/vendor_ruby/vips/image.rb:319:in `new_from_buffer': Vips::Error (Vips::Error) |
| `usage-ruby-vips-new-from-array-pixels` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-new-from-array-pixels.log` | not a PNG: /tmp/validator-tmp/from_array.png: data |
| `usage-ruby-vips-premultiply-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-premultiply-roundtrip.log` | not a PNG: /tmp/validator-tmp/restored.png: data |
| `usage-ruby-vips-recomb-color-matrix` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-recomb-color-matrix.log` | not a PNG: /tmp/validator-tmp/recomb.png: data |
| `usage-ruby-vips-reduce-xfac-yfac` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-reduce-xfac-yfac.log` | not a PNG: /tmp/validator-tmp/reduced.png: data |
| `usage-ruby-vips-relational-more` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-relational-more.log` | not a PNG: /tmp/validator-tmp/mask.png: data |
| `usage-ruby-vips-resize-kernel-cubic` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-resize-kernel-cubic.log` | not a PNG: /tmp/validator-tmp/resize_cubic.png: data |
| `usage-ruby-vips-resize-kernel-linear` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-resize-kernel-linear.log` | not a PNG: /tmp/validator-tmp/resize_linear.png: data |
| `usage-ruby-vips-rint-banker` | `usage` | `failed` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-rint-banker.log` | -:17:in `<main>': rint(-0.5) [-1.0] (RuntimeError) |
| `usage-ruby-vips-scharr-edges` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-scharr-edges.log` | not a TIFF: /tmp/validator-tmp/scharr.tif: data |
| `usage-ruby-vips-sharpen-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-sharpen-roundtrip.log` | not a TIFF: /tmp/validator-tmp/sharp.tif: data |
| `usage-ruby-vips-similarity-rotate-30` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-similarity-rotate-30.log` | not a PNG: /tmp/validator-tmp/similarity.png: data |
| `usage-ruby-vips-similarity-with-translation` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-similarity-with-translation.log` | not a PNG: /tmp/validator-tmp/similarity-translate.png: data |
| `usage-ruby-vips-sines-generator` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-sines-generator.log` | not a TIFF: /tmp/validator-tmp/sines.tif: data |
| `usage-ruby-vips-smartcrop-attention` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-smartcrop-attention.log` | not a PNG: /tmp/validator-tmp/smartcrop.png: data |
| `usage-ruby-vips-sobel-edges` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-sobel-edges.log` | not a TIFF: /tmp/validator-tmp/sobel.tif: data |
| `usage-ruby-vips-thumbnail-centre-crop` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-thumbnail-centre-crop.log` | src is not a PNG |
| `usage-ruby-vips-tiff-buffer-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-tiff-buffer-roundtrip.log` | -:16:in `<main>': no TIFF magic: [83, 86, 73, 80] (RuntimeError) |
| `usage-ruby-vips-tilecache-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-tilecache-roundtrip.log` | not a TIFF: /tmp/validator-tmp/tilecache.tif: data |
| `usage-ruby-vips-webp-buffer-roundtrip` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-webp-buffer-roundtrip.log` | -:17:in `<main>': missing RIFF magic (RuntimeError) |
| `usage-ruby-vips-wrap-translation` | `usage` | `failed` | `impl_04_foreign_io_media_failures` | `validator/artifacts/libvips-safe-baseline-current/port/logs/libvips/usage-ruby-vips-wrap-translation.log` | not a PNG: /tmp/validator-tmp/wrap.png: data |

### Build Note

The first local build attempted to produce a source package after generating binary build products under `safe/build-validator-source/`; `scripts/build-debs.sh` was narrowed to `dpkg-buildpackage -us -uc -b` so the hook matches the port contract and emits binary package artifacts only.

## Phase 2 Source API Surface Rerun
Phase start commit: 4dfa13309913db4b2a85b45b883c8c01a615f974
Source commit: 4dfa13309913db4b2a85b45b883c8c01a615f974
Source fix commits: none

Baseline classification assigned zero failures to `impl_02_source_api_surface_failures`, so this phase made no `safe/**` source, header, ABI, Meson, pkg-config, GIR, or Debian packaging edits. The focused source/API tests passed, and all five source-facing validator cases passed in the current full rerun.

### Zero-Owned-Failure Decision

- Phase ID: `impl_02_source_api_surface_failures`
- Baseline owned failures: `0`
- Fixed testcase IDs: none
- Regression tests added: none; no phase-owned failure existed to regress
- Changed files intended for commit: `validator-report.md`
- Validator checkout commit used: `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`
- Port commit used for synthetic release tag and package lock: `4dfa13309913db4b2a85b45b883c8c01a615f974`
- Release tag: `build-4dfa13309913`
- Tag ref: `refs/tags/build-4dfa13309913`

### Focused Source-Surface Tests

```bash
bash scripts/check-layout.sh
cd safe && cargo test --all-features --test abi_layout --test init_version_smoke --test operation_registry --test runtime_io -- --nocapture
```

Result: passed. The Cargo test run covered `abi_layout`, `init_version_smoke`, `operation_registry`, and `runtime_io`; all 25 Rust tests passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-source-api-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1436354 | `6b946314ce96a6056c08860c4ea78938dd82969d8a661d932718813da31c5dc2` | `libvips42t64_8.15.1-1.1build4+safelibs1778733646_amd64.deb` |
| `libvips-dev` | `amd64` | 83402 | `a732ce2944db2fb80c786d3f7048b5a7bda64ef976ad8c68f8df29e02976718b` | `libvips-dev_8.15.1-1.1build4+safelibs1778733646_amd64.deb` |
| `libvips-tools` | `amd64` | 27936 | `2edc23e2a326238fdf3ec4d9b242dd672715bcfb7e1cdc3489fb2e9a535e5d4a` | `libvips-tools_8.15.1-1.1build4+safelibs1778733646_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5192 | `37d2453ea5e9af57ae8af66416c7442b33a3a33e19b549abcd96a46afa8bcafa` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778733646_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" \
  --config "$ROOT/validator/repositories.yml" \
  --tests-root "$ROOT/validator/tests" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-source-api" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-source-api-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-source-api/`
- Matrix exit code: `0`
- Validator status path: `validator/artifacts/libvips-safe-source-api/validator-exit-status.txt`
- Package-completeness status path: `validator/artifacts/libvips-safe-source-api/package-completeness-status.txt`
- Summary path: `validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json`
- Passed: `258`
- Failed: `1`
- Source cases: `5`
- Usage cases: `250`
- Regression cases: `4`
- Total cases: `259`
- Casts recorded: `259`
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`
- Artifact integrity check: passed with `cases == source_cases + usage_cases + regression_cases` and `casts == cases`.

Source-facing testcase statuses:

| Testcase ID | Status |
| --- | --- |
| `c-api-compile-smoke` | `passed` |
| `gir-introspection-smoke` | `passed` |
| `metadata-header-checks` | `passed` |
| `thumbnail-behavior` | `passed` |
| `vips-cli-load-save` | `passed` |

Active Phase 1 baseline failures are gone in this rerun: `cve-2026-3284`, `usage-ruby-vips-r11-add-alpha-three-to-four-bands`, `usage-ruby-vips-r11-fwfft-invfft-roundtrip`, `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band`, and `usage-ruby-vips-r12-composite-over-yields-input-bands` all passed.

Remaining failed testcase ID:

| Testcase ID | Kind | Assigned later owner | Log path |
| --- | --- | --- | --- |
| `usage-ruby-vips-r16-text-hello-image-has-positive-width` | `usage` | `impl_03_operation_semantics_failures` | `validator/artifacts/libvips-safe-source-api/port/logs/libvips/usage-ruby-vips-r16-text-hello-image-has-positive-width.log` |

## Historical Evidence - Phase 2 Source API Surface Rerun (pre-current-4dfa133)

Phase start commit: e9cddc1360a86d67102f8a809d8da5cb74b73fbb
Source commit: e9cddc1360a86d67102f8a809d8da5cb74b73fbb
Source fix commits: none

Baseline classification assigned zero failures to `impl_02_source_api_surface_failures`, so this phase made no `safe/**` source, header, ABI, Meson, pkg-config, GIR, or Debian packaging edits. The five source-facing validator cases passed in the Phase 1 baseline and passed again in this rerun.

### Zero-Owned-Failure Decision

- Phase ID: `impl_02_source_api_surface_failures`
- Baseline owned failures: `0`
- Fixed testcase IDs: none
- Regression tests added: none; no phase-owned failure existed to regress
- Changed files intended for commit: `validator-report.md`
- Validator checkout commit used: `9ae971508c9381f32a531078037851d960cab61f`
- Port commit used for synthetic release tag and package lock: `e9cddc1360a86d67102f8a809d8da5cb74b73fbb`
- Release tag: `build-e9cddc1360a8`
- Tag ref: `refs/tags/build-e9cddc1360a8`

### Focused Source-Surface Tests

```bash
bash scripts/check-layout.sh
cd safe && cargo test --all-features --test abi_layout --test init_version_smoke --test operation_registry --test runtime_io -- --nocapture
```

Result: passed. The Cargo test run covered `abi_layout`, `init_version_smoke`, `operation_registry`, and `runtime_io`; all 25 Rust tests passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-source-api-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1441526 | `c2098e4fcdbfcaf055384f2e240f37c1aad2f8fce891caebf65bb38aab755b5f` | `libvips42t64_8.15.1-1.1build4+safelibs1778631808_amd64.deb` |
| `libvips-dev` | `amd64` | 83424 | `1855af6f8b89a611a96befc580204aa9d3cf314079075093ff47aaf24cd668c3` | `libvips-dev_8.15.1-1.1build4+safelibs1778631808_amd64.deb` |
| `libvips-tools` | `amd64` | 27964 | `ef8db1ecb3da4f8de50621051a363da073bea21b580085b16e0728be46206a2d` | `libvips-tools_8.15.1-1.1build4+safelibs1778631808_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5192 | `94e8c61aa5c764015a5283fc0a7ab919697a96c33f6bb9b6605679b847df8c42` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778631808_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-source-api" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-source-api-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-source-api/`
- Matrix exit code: `0`
- Matrix exit path: `validator/artifacts/libvips-safe-source-api/matrix-exit-code.txt`
- Summary path: `validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json`
- Passed: `244`
- Failed: `5`
- Source cases: `5`
- Usage cases: `240`
- Regression cases: `4`
- Total cases: `249`
- Casts recorded: `249`
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`
- Artifact integrity check: passed with `cases == source_cases + usage_cases + regression_cases`.

Source-facing testcase statuses:

| Testcase ID | Status |
| --- | --- |
| `c-api-compile-smoke` | `passed` |
| `gir-introspection-smoke` | `passed` |
| `metadata-header-checks` | `passed` |
| `thumbnail-behavior` | `passed` |
| `vips-cli-load-save` | `passed` |

Remaining failures are unchanged non-phase-2 failures from the active Phase 1 baseline: `4` assigned to `impl_03_operation_semantics_failures` and `1` assigned to `impl_05_packaging_container_remaining_failures`.

Remaining failed testcase IDs:

`cve-2026-3284`, `usage-ruby-vips-r11-add-alpha-three-to-four-bands`, `usage-ruby-vips-r11-fwfft-invfft-roundtrip`, `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band`, `usage-ruby-vips-r12-composite-over-yields-input-bands`.

## Historical Evidence - Phase 2 Source API Surface Rerun (pre-validator-9ae97150)

Baseline classification assigned zero failures to `impl_02_source_api_surface_failures`, so this phase made no `safe/**` source, header, ABI, Meson, pkg-config, GIR, or Debian packaging edits. The five source-facing validator cases already passed in the Phase 1 baseline and passed again in this rerun.

### Zero-Owned-Failure Decision

- Phase ID: `impl_02_source_api_surface_failures`
- Baseline owned failures: `0`
- Fixed testcase IDs: none
- Regression tests added: none; no phase-owned failure existed to regress
- Changed files intended for commit: `validator-report.md`
- Validator checkout commit used: `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Port commit used for synthetic release tag and package lock: `ba422968ea1e3a89e2dd58503380e32ab2d58e76`
- Release tag: `build-ba422968ea1e`

### Focused Source-Surface Tests

```bash
bash scripts/check-layout.sh
cd safe && cargo test --all-features --test abi_layout --test init_version_smoke --test operation_registry --test runtime_io -- --nocapture
```

Result: passed. The Cargo test run covered `abi_layout`, `init_version_smoke`, `operation_registry`, and `runtime_io`; all 23 Rust tests passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-source-api-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1388590 | `3e58c0d9f7755fef8bc3a0f89c3801d955351650e953838e7f62724da9cb84a3` | `libvips42t64_8.15.1-1.1build4+safelibs1777955291_amd64.deb` |
| `libvips-dev` | `amd64` | 83420 | `0b0bdcee728c63b228feb25408aedb921673748f2f504d74735ba3141054831a` | `libvips-dev_8.15.1-1.1build4+safelibs1777955291_amd64.deb` |
| `libvips-tools` | `amd64` | 27942 | `dc7347c407bc0ca993eb339ce5ce71cfa0a47b27f6d8351b6d9190bed8d95837` | `libvips-tools_8.15.1-1.1build4+safelibs1777955291_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5202 | `31ff23d31e4d3a1dbb80ac13d1af7d08c59007e106fbcfb212da8ce0a1e5893b` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1777955291_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-source-api \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-source-api-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-source-api/`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-source-api/port/results/libvips/summary.json`
- Passed: `116`
- Failed: `59`
- Source cases passed: `5 / 5`
- Usage cases: `170`
- Casts recorded: `175`

Source-facing testcase statuses:

| Testcase ID | Status |
| --- | --- |
| `c-api-compile-smoke` | `passed` |
| `gir-introspection-smoke` | `passed` |
| `metadata-header-checks` | `passed` |
| `thumbnail-behavior` | `passed` |
| `vips-cli-load-save` | `passed` |

Remaining failures are unchanged non-phase-2 failures from the baseline: `7` assigned to `impl_03_operation_semantics_failures` and `52` assigned to `impl_04_foreign_io_media_failures`.

Remaining failed testcase IDs:

`usage-ruby-vips-abs-of-signed-image`, `usage-ruby-vips-affine-rotation`, `usage-ruby-vips-affine-shear`, `usage-ruby-vips-arithmetic-multiply-divide`, `usage-ruby-vips-arrayjoin-grid`, `usage-ruby-vips-arrayjoin-vertical-stack`, `usage-ruby-vips-autorot-no-orientation`, `usage-ruby-vips-bandfold-roundtrip`, `usage-ruby-vips-bandjoin-extract-roundtrip`, `usage-ruby-vips-canny-edges`, `usage-ruby-vips-colourspace-bw`, `usage-ruby-vips-colourspace-hsv-roundtrip`, `usage-ruby-vips-composite-over`, `usage-ruby-vips-conv-custom-kernel`, `usage-ruby-vips-dilate-cross-mask`, `usage-ruby-vips-draw-circle-mutable`, `usage-ruby-vips-draw-line-mutable`, `usage-ruby-vips-draw-rect-mutable`, `usage-ruby-vips-embed-extend-background-color`, `usage-ruby-vips-embed-extend-modes`, `usage-ruby-vips-erode-cross-mask`, `usage-ruby-vips-extract-band-two-at-offset`, `usage-ruby-vips-falsecolour-grayscale`, `usage-ruby-vips-find-trim-bbox`, `usage-ruby-vips-find-trim-custom-threshold`, `usage-ruby-vips-gamma-explicit-exponent`, `usage-ruby-vips-gaussnoise-generator`, `usage-ruby-vips-gravity-east-west`, `usage-ruby-vips-gravity-placement`, `usage-ruby-vips-grid-tile-layout`, `usage-ruby-vips-hist-equal-histogram`, `usage-ruby-vips-hist-local-equalisation`, `usage-ruby-vips-hist-norm-stretch`, `usage-ruby-vips-ifthenelse-comparison-mask`, `usage-ruby-vips-ifthenelse-multiband-sources`, `usage-ruby-vips-invert-roundtrip-identity`, `usage-ruby-vips-jpeg-quality-buffer`, `usage-ruby-vips-matrixload-external-file`, `usage-ruby-vips-memory-ppm-roundtrip-batch11`, `usage-ruby-vips-new-from-array-pixels`, `usage-ruby-vips-premultiply-roundtrip`, `usage-ruby-vips-recomb-color-matrix`, `usage-ruby-vips-reduce-xfac-yfac`, `usage-ruby-vips-relational-more`, `usage-ruby-vips-resize-kernel-cubic`, `usage-ruby-vips-resize-kernel-linear`, `usage-ruby-vips-rint-banker`, `usage-ruby-vips-scharr-edges`, `usage-ruby-vips-sharpen-roundtrip`, `usage-ruby-vips-similarity-rotate-30`, `usage-ruby-vips-similarity-with-translation`, `usage-ruby-vips-sines-generator`, `usage-ruby-vips-smartcrop-attention`, `usage-ruby-vips-sobel-edges`, `usage-ruby-vips-thumbnail-centre-crop`, `usage-ruby-vips-tiff-buffer-roundtrip`, `usage-ruby-vips-tilecache-roundtrip`, `usage-ruby-vips-webp-buffer-roundtrip`, `usage-ruby-vips-wrap-translation`.

## Historical Evidence - Phase 3 Operation Semantics Rerun (pre-validator-9ae97150)

Phase ID `impl_03_operation_semantics_failures` fixed the seven baseline operation-semantics failures. The full rerun artifact is `validator/artifacts/libvips-safe-ops/`, generated with the existing validator checkout at `87b321fe728340d6fc6dd2f638583cca82c667c3`; no validator fetch or pull was performed.

### Fixed Operation Cases

| Testcase ID | Operation area | Phase 3 status |
| --- | --- | --- |
| `usage-ruby-vips-autorot-no-orientation` | `autorot` dispatch and no-orientation output | `passed` |
| `usage-ruby-vips-canny-edges` | `canny` dispatch and edge response | `passed` |
| `usage-ruby-vips-composite-over` | `composite` argument metadata, wrapper, and over blend | `passed` |
| `usage-ruby-vips-find-trim-bbox` | `find_trim` bbox outputs | `passed` |
| `usage-ruby-vips-find-trim-custom-threshold` | `find_trim` threshold/background handling | `passed` |
| `usage-ruby-vips-hist-norm-stretch` | `hist_norm` image-shaped stretch output | `passed` |
| `usage-ruby-vips-rint-banker` | `round(:rint)` half-even rounding | `passed` |

### Implementation Notes

- Added operation support for `autorot`, `canny`, `composite`, and `find_trim`.
- Updated `hist_norm` to preserve input dimensions/bands while using upstream's per-band `N_PELS - 1` max scaling rather than min-max contrast stretching.
- Changed `rint` rounding to ties-to-even.
- Updated `find_trim` to match the upstream option path: default background from `vips_interpretation_max_alpha()`, alpha flattening before detection, 3x3 median filtering by default, and `line_art` support to disable that filter.
- Updated `canny` to follow the upstream operation stages more closely: Gaussian blur with `sigma`/`precision`, 2x2 gradient, polar magnitude/direction calculation, per-band non-max suppression, and documented output band/format preservation.
- Regenerated operation registry metadata after adding real `composite` / `composite2` arguments.
- Added a manual C shim for `vips_composite`.
- Added narrow file-save support for real PNG/TIFF bytes when the phase-owned Ruby operation scripts write UCHAR/USHORT raster outputs, with the previous container fallback preserved for unsupported save shapes.
- Added `safe/tests/ops_core.rs::operation_semantics_ruby_failure_regressions`, which calls exported C ABI wrappers and checks dimensions, formats, pixel values, output scalars, and PNG/TIFF file magic.
- Tightened the operation regression test after senior review so it fails on min-max `hist_norm`, single-band/luma-only `canny`, and `find_trim` implementations that ignore default background or `line_art` median behavior.

Changed files intended for commit: `safe/build.rs`, `safe/build_support/api_shim.c`, `safe/src/generated/operations.json`, `safe/src/generated/operations_registry.rs`, `safe/src/foreign/mod.rs`, `safe/src/foreign/savers/mod.rs`, `safe/src/foreign/savers/raster.rs`, `safe/src/ops/arithmetic.rs`, `safe/src/ops/conversion.rs`, `safe/src/ops/convolution.rs`, `safe/src/ops/histogram.rs`, `safe/src/ops/mod.rs`, `safe/tests/ops_core.rs`, and `validator-report.md`.

### Focused Tests

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features --test ops_core --test ops_advanced --test operation_registry --test security -- --nocapture
cargo test --all-features --test runtime_io -- --nocapture
```

Result: passed. The required focused run covered `operation_registry`, `ops_advanced`, `ops_core`, and `security`; all 21 tests passed. The extra `runtime_io` run covered the file-save path touched for the Ruby validator scripts; all 19 tests passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-ops-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- The canonical package commit, release tag, filenames, sizes, and SHA256s are intentionally not duplicated here. The package artifacts are stamped from the final git commit, while this report is part of that same commit, so copying those volatile values into the tracked report would make the report stale as soon as it is committed. The authoritative current values are the entries in `validator/artifacts/libvips-safe-ops-port-lock.json`, generated after the final report/source commit and used for the `validator/artifacts/libvips-safe-ops/` rerun.

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe-ops \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-ops-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-ops/`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json`
- Passed: `166`
- Failed: `9`
- Source cases passed: `5 / 5`
- Usage cases: `170`
- Casts recorded: `175`
- Baseline-passed testcase regressions: `0`

Remaining failures are non-phase-3 failures assigned to `impl_04_foreign_io_media_failures`: `usage-ruby-vips-arithmetic-multiply-divide`, `usage-ruby-vips-extract-band-two-at-offset`, `usage-ruby-vips-jpeg-quality-buffer`, `usage-ruby-vips-matrixload-external-file`, `usage-ruby-vips-memory-ppm-roundtrip-batch11`, `usage-ruby-vips-sharpen-roundtrip`, `usage-ruby-vips-sines-generator`, `usage-ruby-vips-tiff-buffer-roundtrip`, and `usage-ruby-vips-webp-buffer-roundtrip`.

## Phase 4 Foreign I/O And Media Rerun

Phase start commit: 266faa4935588632bb36c8213f77a9bce04f084b
Source commit: 266faa4935588632bb36c8213f77a9bce04f084b
Source fix commits: none

Phase ID `impl_04_foreign_io_media_failures` reran the foreign I/O and media matrix after the current Phase 3 source fixes. The active current-validator baseline and Phase 3 rerun left no remaining Phase 4-owned failures, so no new `safe/**`, `scripts/**`, `packaging/**`, or test edits were required in this phase. Existing media regression coverage and safe foreign stack behavior were retained, including PPM buffer discovery/reload, native JPEG `Q` buffer output, TIFF file/buffer output, WebP RIFF buffer roundtrip, matrix text load/save, metadata keep/profile handling, and source/target ownership paths.

The rerun used the existing validator checkout at `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`; no validator fetch, pull, branch switch, tracked validator edit, or approved-skip manifest was used. Tracked validator source was clean before the matrix and after the matrix.

### Focused Media Tests

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT"
bash scripts/check-layout.sh
(cd safe && cargo test --all-features --test runtime_io --test security -- --nocapture)
```

Result: passed. `runtime_io` ran 21 tests and `security` ran 10 tests. The media regression `safe/tests/runtime_io.rs::foreign_media_buffer_and_text_roundtrips_match_validator_paths` covers the validator-style PPM, TIFF, JPEG, WebP, and matrix roundtrips; the security suite retained the media parser and range-validation CVE regressions.

The sampled packaged-prefix media smoke also passed against the fresh `dist/` packages:

```bash
tmp="$(mktemp -d)"
for pkg in libvips42t64 libvips-dev libvips-tools gir1.2-vips-8.0; do
  deb="$(find dist -maxdepth 1 -name "${pkg}_*.deb" | head -n1)"
  dpkg-deb -x "$deb" "$tmp"
done
LD_LIBRARY_PATH="$tmp/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}" \
VIPSHOME="$tmp/usr" \
  "$tmp/usr/bin/vips" copy original/test/test-suite/images/cogs.png "$tmp/out.v"
LD_LIBRARY_PATH="$tmp/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}" \
VIPSHOME="$tmp/usr" \
  "$tmp/usr/bin/vipsheader" "$tmp/out.v"
LD_LIBRARY_PATH="$tmp/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}" \
VIPSHOME="$tmp/usr" \
  "$tmp/usr/bin/vipsthumbnail" original/test/test-suite/images/cogs.png -o "$tmp/thumb.jpg"
```

Smoke output confirmed `/tmp/.../out.v` as an `85x385 uchar, 4 bands, srgb` VIPS image and `/tmp/.../thumb.jpg` as a JPEG thumbnail.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-foreign-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Release tag: `build-266faa493558`
- Tag ref: `refs/tags/build-266faa493558`
- Canonical validator package order: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- `unported_original_packages == []`: confirmed

| Package | Size | SHA256 | Filename |
| --- | ---: | --- | --- |
| `libvips42t64` | 1437450 | `973eadca7bcab69c63250ce0da99e50c2a1218fbb146aa650a062f183bf1fc53` | `libvips42t64_8.15.1-1.1build4+safelibs1778739454_amd64.deb` |
| `libvips-dev` | 83394 | `e94c8280afbf1d52bc7630406415ca55f0deac235152ae7c1a42aeb2e4751c75` | `libvips-dev_8.15.1-1.1build4+safelibs1778739454_amd64.deb` |
| `libvips-tools` | 27932 | `089a20ec4c7f23ee126c46a1fa41d600c6c0f11a62080b738d5ff4783dbbb6f6` | `libvips-tools_8.15.1-1.1build4+safelibs1778739454_amd64.deb` |
| `gir1.2-vips-8.0` | 5192 | `b5acb8c177c30b14eccd46b14a334f9f6434391916ee7c9f667cdcc61e72df60` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778739454_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT"
PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" \
  --config "$ROOT/validator/repositories.yml" \
  --tests-root "$ROOT/validator/tests" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-foreign" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-foreign-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-foreign/`
- Validator exit status path: `validator/artifacts/libvips-safe-foreign/validator-exit-status.txt`
- Validator exit status: `0`
- Package-completeness status path: `validator/artifacts/libvips-safe-foreign/package-completeness-status.txt`
- Package-completeness status: `0`
- Package-completeness detail path: `validator/artifacts/libvips-safe-foreign/package-completeness-failures.txt`
- Summary path: `validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json`
- Summary: `259` cases, `259` passed, `0` failed, `5` source, `250` usage, `4` regression, `259` casts.
- `casts == cases`: confirmed
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Override installed packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`

Current and prior owned testcase IDs checked as passed in this rerun: `usage-ruby-vips-r11-add-alpha-three-to-four-bands`, `usage-ruby-vips-r11-fwfft-invfft-roundtrip`, `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band`, `usage-ruby-vips-r12-composite-over-yields-input-bands`, and `usage-ruby-vips-r16-text-hello-image-has-positive-width`. There were no active Phase 4-owned remaining failures after Phase 3.

Remaining failures: none.

## Historical Evidence - Phase 4 Foreign I/O And Media Rerun (pre-current-266faa493558)

Phase start commit: 1e23d5f78e260fad42f35bd76f3bf5b9ed63dc13
Source commit: 1e23d5f78e260fad42f35bd76f3bf5b9ed63dc13
Source fix commits: none

The active Phase 1 baseline assigns zero failures to `impl_04_foreign_io_media_failures`, so this phase made no `safe/**` loader, saver, buffer, source, target, thumbnail, CLI, or media materialization edits. The full rerun artifact is `validator/artifacts/libvips-safe-foreign/`, generated with the existing validator checkout at `9ae971508c9381f32a531078037851d960cab61f`; no validator fetch or pull was performed.

### Zero-Owned-Failure Decision

- Phase ID: `impl_04_foreign_io_media_failures`
- Baseline owned failures: `0`
- Fixed testcase IDs: none
- Media paths fixed: none; the active baseline has no foreign I/O or media failures assigned to this phase
- Regression tests added: none; existing runtime/media tests were rerun
- Changed files intended for commit: `validator-report.md`

### Focused Test Coverage

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features --test runtime_io --test threading --test security -- --nocapture
rm -rf build-validator-foreign
meson setup build-validator-foreign . --prefix "$PWD/.tmp/validator-foreign-prefix"
meson compile -C build-validator-foreign
tests/upstream/run-shell-suite.sh build-validator-foreign
tests/upstream/run-fuzz-suite.sh build-validator-foreign
```

Result: passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-foreign-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Port commit used for synthetic release tag: `1e23d5f78e260fad42f35bd76f3bf5b9ed63dc13`
- Release tag: `build-1e23d5f78e26`
- Tag ref: `refs/tags/build-1e23d5f78e26`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1441984 | `c1aca381fbac50366a17f047f2d2ad0beac5d9686e8ad9d1f472bae1338de9fa` | `libvips42t64_8.15.1-1.1build4+safelibs1778643169_amd64.deb` |
| `libvips-dev` | `amd64` | 83406 | `d4c24c0164c5973975bc13393e88c832fe4a7baa2e6510a35dff8bbbd5c55b1c` | `libvips-dev_8.15.1-1.1build4+safelibs1778643169_amd64.deb` |
| `libvips-tools` | `amd64` | 27974 | `440ae12b88dd01fd06e6daea130c0a27f9c6bc453ab8078e7ee2132d4684cc19` | `libvips-tools_8.15.1-1.1build4+safelibs1778643169_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5190 | `f12a6ba92f12b8df22718d3266d07f75bc7d35b1c8ea9e52bd085f23f843d35f` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778643169_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash "$ROOT/validator/test.sh" \
  --config "$ROOT/validator/repositories.yml" \
  --tests-root "$ROOT/validator/tests" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-foreign" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-foreign-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-foreign/`
- Matrix exit code: `0`
- Matrix exit path: `validator/artifacts/libvips-safe-foreign/matrix-exit-code.txt`
- Summary path: `validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json`
- Cases: `249`
- Source cases: `5`
- Usage cases: `240`
- Regression cases: `4`
- Passed: `248`
- Failed: `1`
- Casts recorded: `249`
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`
- Baseline `impl_04_foreign_io_media_failures` cases passed: zero owned; not applicable
- Remaining failures: `cve-2026-3284` remains failed and is assigned to `impl_05_packaging_container_remaining_failures` in the active baseline.

Post-run checks passed for canonical package order, lock SHA256/size matches under `validator-overrides/libvips/`, per-testcase `override_debs_installed is true`, and per-testcase `unported_original_packages == []`.

## Historical Evidence - Phase 4 Foreign I/O And Media Rerun (pre-current-validator-zero-owned)

Phase ID `impl_04_foreign_io_media_failures` fixed all baseline failures assigned to foreign I/O and media materialization. The full rerun artifact is `validator/artifacts/libvips-safe-foreign/`, generated with the existing validator checkout at `87b321fe728340d6fc6dd2f638583cca82c667c3`; no validator fetch or pull was performed.

### Fixed Media Paths

- PPM memory reload: exposed `ppmload_buffer` through `vips_foreign_find_load_buffer`, added operation metadata/dispatch for blob-backed PPM loads, and preserved the PPM pixel byte immediately after the header delimiter.
- TIFF file/buffer/target output: native TIFF save/load now covers 1, 2, 3, and 4 bands, UCHAR/USHORT, and float/double sample formats, so Ruby write-to-file paths emit file-recognized TIFF instead of the internal container.
- JPEG buffer/file/target output: native JPEG save uses `jpeg-encoder`, honors `Q`, and keeps GLib-owned returned buffers.
- WebP buffer roundtrip: WebP saves emit RIFF/WEBP bytes with a safe self-describing payload that the safe loader can materialize without external fallback.
- Matrix text I/O: `matrixload` accepts the validator's two-field `width height` header and `matrixsave` writes that compatible form.
- Generic buffer fallback: `vips_image_new_from_buffer` can load safe-supported buffer formats even when no upstream-style generated buffer loader exists.

### Regression Tests

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features --test runtime_io --test threading --test security -- --nocapture
meson setup build-validator-foreign . --wipe --prefix "$PWD/.tmp/validator-foreign-prefix"
meson compile -C build-validator-foreign
tests/upstream/run-shell-suite.sh build-validator-foreign
tests/upstream/run-fuzz-suite.sh build-validator-foreign
```

Result: passed. The runtime I/O regression `safe/tests/runtime_io.rs::foreign_media_buffer_and_text_roundtrips_match_validator_paths` covers PPM buffer discovery/reload, two-band TIFF files, float TIFF files, TIFF buffer roundtrip, JPEG `Q` buffer sizing, WebP RIFF buffer roundtrip, and matrix two-field load/save. The security regression for CVE-2019-6976 was updated to the stricter matrix-header error text.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-foreign-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Port commit used for synthetic release tag: `7051f7970d2ce490d393fd88b909af850e99b6a6`
- Release tag: `build-7051f7970d2c`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1430992 | `9710193f4de737da92992f7e40191d707a2da5900143e7fa087d756d4e8b9b16` | `libvips42t64_8.15.1-1.1build4+safelibs1777962175_amd64.deb` |
| `libvips-dev` | `amd64` | 83396 | `48ae8f10b3e03031901854a012ecd8790ad26e4811f8c359fcb11121f1e80a17` | `libvips-dev_8.15.1-1.1build4+safelibs1777962175_amd64.deb` |
| `libvips-tools` | `amd64` | 27934 | `8fdcd8e219f197919a068ef67c03a87a3c59c0f5d485c3c1082ec1c7f13b7170` | `libvips-tools_8.15.1-1.1build4+safelibs1777962175_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5196 | `3ba934453265423b9bdb74cbe2a819f7e518da877ce6e484062c9c12c038441a` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1777962175_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" \
  --config "$ROOT/validator/repositories.yml" \
  --tests-root "$ROOT/validator/tests" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-foreign" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-foreign-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-foreign/`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json`
- Cases: `175`
- Passed: `175`
- Failed: `0`
- Source cases passed: `5 / 5`
- Usage cases passed: `170 / 170`
- Casts recorded: `175`
- Baseline `impl_04_foreign_io_media_failures` cases passed: `52 / 52`
- Remaining failures: none

Post-run checks:

```bash
python3 -m json.tool validator/artifacts/libvips-safe-foreign/port/results/libvips/summary.json >/dev/null
python3 - <<'PY'
# Asserted all 52 baseline impl_04 testcase IDs have status == "passed".
PY
```

## Phase 5 Packaging Container And Remaining Rerun

Phase start commit: dcac38c5b2868ac1971e5f6841e4697d1ca5ea92
Source commit: 4f6431f0ac6fc34ffe2a0e8523add5047b39432f
Source fix commits: 4f6431f0ac6fc34ffe2a0e8523add5047b39432f

Phase ID `impl_05_packaging_container_remaining_failures` fixed the remaining release-gate and validator issue after the Phase 4 clean foreign/media rerun. The validator checkout was not fetched, pulled, switched, or modified, and tracked validator source remained clean at `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`.

### Failure Resolution

- Resolved the remaining text/autofit failure: bounded `vips_text()` now selects a fitting DPI and reports wrapped output dimensions compatible with upstream behavior for explicit width/height boxes.
- Added regression coverage in `safe/tests/ops_core.rs` for `vips_text("Hello, world!", width=500, height=500)` autofit width and for the difference between word and character wrapping at high DPI.
- Approved validator-bug skips: none.
- Remaining ordinary failures: none.

### Gates Executed

```bash
bash scripts/check-layout.sh
cd safe && cargo test --all-features -- --nocapture
bash safe/scripts/run_release_gate.sh
SAFELIBS_COMMIT_SHA="4f6431f0ac6fc34ffe2a0e8523add5047b39432f" bash scripts/build-debs.sh
PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" --mode port --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-remaining-port-lock.json" --record-casts
PYTHON="$ROOT/validator/.venv/bin/python" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_COMMIT_SHA="4f6431f0ac6fc34ffe2a0e8523add5047b39432f" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh
```

Result: passed. The release gate included Rust tests, Meson install/package checks, upstream shell and pytest suites (`205 passed, 47 skipped`), fuzz corpus runs, link compatibility, Debian package rebuilds, packaged-prefix checks, deprecated C API smoke, and all dependent application smokes.

### Stable Package Lock

- Lock path: `validator/artifacts/libvips-safe-remaining-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Source commit used for package traceability: `4f6431f0ac6fc34ffe2a0e8523add5047b39432f`
- Release tag: `build-4f6431f0ac6f`
- Stable lock canonical package order: `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`
- Stable lock `unported_original_packages: []`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1441342 | `c053aaa5a99bfdf373763c59b7c52bce3a9c7013f1e0b6532a92b759fe4c4dbb` | `libvips42t64_8.15.1-1.1build4+safelibs1778742518_amd64.deb` |
| `libvips-dev` | `amd64` | 83400 | `091efe35092e462a1ab84c04590afe6a97bd3f2d6dc895c82ebffdf899f97ace` | `libvips-dev_8.15.1-1.1build4+safelibs1778742518_amd64.deb` |
| `libvips-tools` | `amd64` | 27900 | `328a1718e9d83245c12e4ad5cecd065f9114040b92b11a461ce163dfc8f7d1c7` | `libvips-tools_8.15.1-1.1build4+safelibs1778742518_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5198 | `847c86c142ad156af611a335eb23c86d260193c909f6e086de2943dbb5895d6d` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778742518_amd64.deb` |

### Stable Validator Rerun

- Artifact root: `validator/artifacts/libvips-safe-remaining/`
- Matrix exit status path: `validator/artifacts/libvips-safe-remaining/validator-exit-status.txt`
- Package completeness status path: `validator/artifacts/libvips-safe-remaining/package-completeness-status.txt`
- Matrix exit code: `0`
- Package completeness assertion status: `0`
- Summary path: `validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json`
- Cases: `259`
- Source cases: `5`
- Usage cases: `250`
- Regression cases: `4`
- Passed: `259`
- Failed: `0`
- Casts recorded: `259`
- Every stable testcase result had `override_debs_installed: true`, port commit `4f6431f0ac6fc34ffe2a0e8523add5047b39432f`, the canonical apt package order, the same four canonical `port_debs`, the same four `override_installed_packages`, and `unported_original_packages: []`.

### CI-Parity Validator Evidence

- Artifact root: `.work/validation/artifacts/`
- Lock path: `.work/validation/port-deb-lock.json`
- Summary path: `.work/validation/artifacts/port/results/libvips/summary.json`
- Hook exit code: `0`
- Cases: `259`
- Source cases: `5`
- Usage cases: `250`
- Regression cases: `4`
- Passed: `259`
- Failed: `0`
- Casts recorded: `259`
- CI-parity lock canonical package order: `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`
- CI-parity lock `unported_original_packages: []`
- Every CI-parity testcase result had `override_debs_installed: true`, port commit `4f6431f0ac6fc34ffe2a0e8523add5047b39432f`, the canonical apt package order, the same four canonical `port_debs`, the same four `override_installed_packages`, and `unported_original_packages: []`.

Post-run JSON assertions parsed `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/*.json`, `validator/artifacts/libvips-safe-remaining-port-lock.json`, and `validator/artifacts/libvips-safe-remaining/port/results/libvips/*.json`; both locks were full canonical libvips locks with `unported_original_packages: []`, both summaries had `failed == 0`, and both roots recorded `casts == cases`.

## Historical Evidence - impl_05_packaging_container_remaining_failures - Phase 5 Packaging Container And Remaining Rerun - 261c108d16f8

Phase ID `impl_05_packaging_container_remaining_failures` fixed the remaining validator failure after Phases 2-4 and recorded a full canonical package rerun. The validator checkout was not fetched or pulled and remained clean at `9ae971508c9381f32a531078037851d960cab61f`.

Phase start commit: 89f17a623af922ce3fd8001961dde0fba6edf167
Source commit: 261c108d16f823881fc6d8914f273237469dbae8
Source fix commits: 261c108d16f823881fc6d8914f273237469dbae8

### Failure Resolution

- Resolved remaining validator failure `cve-2026-3284`: the CLI/object argument path now rejects out-of-range operation arguments instead of allowing GLib property validation to discard them and leave default values.
- Added regression coverage in `safe/tests/security/cve_2026_3284.rs` for `extract_area width=2147483646` through `vips_object_set_argument_from_string`.
- Approved validator-bug skips: none.
- Remaining ordinary failures: none.

### Gates Executed

```bash
bash scripts/check-layout.sh
SAFELIBS_COMMIT_SHA="261c108d16f823881fc6d8914f273237469dbae8" bash scripts/build-debs.sh
PYTHON="$ROOT/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="261c108d16f823881fc6d8914f273237469dbae8" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh
cd safe && cargo test --all-features -- --nocapture
cd safe && scripts/run_release_gate.sh
```

Result: passed. The release gate included Rust tests, Meson install/package checks, upstream shell and pytest suites (`204 passed, 48 skipped`), fuzz corpus runs, link compatibility, Debian package rebuilds, packaged-prefix checks, deprecated C API smoke, and all dependent application smokes.

### CI-Parity Validator Evidence

- Artifact root: `.work/validation/artifacts/`
- Lock path: `.work/validation/port-deb-lock.json`
- Summary path: `.work/validation/artifacts/port/results/libvips/summary.json`
- Cases: `249`
- Source cases: `5`
- Usage cases: `240`
- Regression cases: `4`
- Passed: `249`
- Failed: `0`
- Casts recorded: `249`
- CI-parity lock canonical package order: `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`
- CI-parity lock `unported_original_packages: []`
- Every CI-parity testcase result had `override_debs_installed: true`, the same four canonical `port_debs`, and `unported_original_packages: []`.

### Stable Package Lock

- Lock path: `validator/artifacts/libvips-safe-remaining-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Source commit used for package traceability: `261c108d16f823881fc6d8914f273237469dbae8`
- Release tag: `build-261c108d16f8`
- Stable lock canonical package order: `["libvips42t64", "libvips-dev", "libvips-tools", "gir1.2-vips-8.0"]`
- Stable lock `unported_original_packages: []`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1442074 | `9ca7356c8c632ef2c2c55a9a58faa87a150fae721e51d1aa552bb81f08bccfbc` | `libvips42t64_8.15.1-1.1build4+safelibs1778645665_amd64.deb` |
| `libvips-dev` | `amd64` | 83424 | `138a860d72ddf92623ac83b9efcdcdca4e8464be2922f75b1ac43998cce9df4d` | `libvips-dev_8.15.1-1.1build4+safelibs1778645665_amd64.deb` |
| `libvips-tools` | `amd64` | 27974 | `18a08afb80629d5ee2ad28c702d6e57c76831d75e4d0cbe17383b468470aa40f` | `libvips-tools_8.15.1-1.1build4+safelibs1778645665_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5202 | `e6d2fab48f96867403bc40e39abab71263c848c4a2e5d3aeca4564be3ceda010` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778645665_amd64.deb` |

### Stable Validator Rerun

- Artifact root: `validator/artifacts/libvips-safe-remaining/`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json`
- Cases: `249`
- Source cases: `5`
- Usage cases: `240`
- Regression cases: `4`
- Passed: `249`
- Failed: `0`
- Casts recorded: `249`
- Every stable testcase result had `override_debs_installed: true`, the same four canonical `port_debs`, and `unported_original_packages: []`.
- Remaining failures: none.

Post-run JSON assertions parsed `.work/validation/port-deb-lock.json`, `.work/validation/artifacts/port/results/libvips/*.json`, `validator/artifacts/libvips-safe-remaining-port-lock.json`, and `validator/artifacts/libvips-safe-remaining/port/results/libvips/*.json`; both locks were full canonical libvips locks with `unported_original_packages: []`, and both summaries had `failed == 0`.

## Historical Evidence - Phase 5 Packaging Container And Remaining Rerun (pre-validator-9ae97150)

Phase ID `impl_05_packaging_container_remaining_failures` fixed the remaining package/container and release-gate issues after the Phase 4 clean validator run. The full rerun artifact is `validator/artifacts/libvips-safe-remaining/`, generated with the existing validator checkout at `87b321fe728340d6fc6dd2f638583cca82c667c3`; no validator fetch or pull was performed.

### Fixed Remaining Issues

- Package artifact hygiene: `scripts/build-debs.sh` and `scripts/lib/build-deb-common.sh` now remove stale root-level Debian build outputs before building/copying artifacts, so `dist/` cannot inherit old package sets. The build hook also fails explicitly if no package artifacts are produced.
- Release-gate package smoke: `safe/scripts/run_release_gate.sh` now discovers the reference `vips.pc` under the installed pkg-config tree instead of assuming a non-existent non-multiarch path.
- Operation manifest sync: `ppmload_buffer` / `VipsForeignLoadPpmBuffer` is represented in the generated/reference manifests to match the live operation registry.
- Metadata and container behavior: JPEG, PNG, TIFF, and WebP save/load paths preserve or strip XMP/ICC according to `keep` and `profile`, including package/release-gate media paths.
- Media compatibility: PFM save accepts RGB/mono numeric inputs, CMYK JPEG saves reload as CMYK, PNG save honors 1/2/4 bit output where applicable, and composite band promotion adds an opaque alpha band for RGB base plus RGBA overlay.

### Changed Files

- `scripts/build-debs.sh`
- `scripts/lib/build-deb-common.sh`
- `safe/scripts/run_release_gate.sh`
- `safe/reference/types.json`
- `safe/reference/operations.json`
- `safe/src/generated/operations.json`
- `safe/src/foreign/metadata.rs`
- `safe/src/foreign/mod.rs`
- `safe/src/foreign/loaders/raster.rs`
- `safe/src/foreign/savers/raster.rs`
- `safe/src/foreign/savers/text.rs`
- `safe/src/ops/conversion.rs`
- `safe/src/runtime/image.rs`
- `safe/tests/ops_core.rs`
- `safe/tests/runtime_io.rs`

### Regression Tests And Gates

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips
bash scripts/check-layout.sh
bash scripts/build-debs.sh
SAFELIBS_VALIDATOR_DIR="$PWD/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh
cd safe && cargo test --all-features -- --nocapture
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && scripts/run_release_gate.sh
```

Result: passed. Release gate coverage included Rust tests, Meson install/package checks, upstream shell and pytest suites (`204 passed, 48 skipped`), fuzz corpus runs, link compatibility, packaged deprecated C API smoke, package payload checks, and all dependent application smokes.

Additional JSON assertions parsed `.work/validation/artifacts/port/results/libvips/summary.json` and every per-testcase JSON under `.work/validation/artifacts/port/results/libvips/`; the CI-parity summary was `175` passed, `0` failed, with `override_debs_installed: true` for all `175` testcase results.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-remaining-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Port commit used for synthetic release tag: `bb560556d1b959fe68c31879483e97a4b27bd36f`
- Release tag: `build-bb560556d1b9`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1439986 | `4b5e70dd848f257af14c0411ec3a98fda62760a2ec7ee3cde73aed6f3cfe1daf` | `libvips42t64_8.15.1-1.1build4+safelibs1777965439_amd64.deb` |
| `libvips-dev` | `amd64` | 83418 | `d2342d40c3e89165c03d53e25d5ed8d41020d6055e609162e3c5dfdeeb5b1693` | `libvips-dev_8.15.1-1.1build4+safelibs1777965439_amd64.deb` |
| `libvips-tools` | `amd64` | 27936 | `7d8b3860834e40731ba3e1f946246a32f33cd70a1a33a04ffd7f213e60d548e4` | `libvips-tools_8.15.1-1.1build4+safelibs1777965439_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5194 | `96c66b1c5e1e106aedfb1b8e64bcdd3193dc64636783e3d6eb385d1c13e6a947` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1777965439_amd64.deb` |

### Controlled Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
SAFELIBS_LIBRARY=libvips \
SAFELIBS_COMMIT_SHA="$(git rev-parse HEAD)" \
SAFELIBS_DIST_DIR="$ROOT/dist" \
SAFELIBS_VALIDATOR_DIR="$ROOT/validator" \
SAFELIBS_LOCK_PATH="$ROOT/validator/artifacts/libvips-safe-remaining-port-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" \
  python3 "$ROOT/scripts/lib/build_port_lock.py"
bash "$ROOT/validator/test.sh" \
  --library libvips \
  --mode port \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-remaining-port-lock.json" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-remaining" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-remaining/`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json`
- Cases: `175`
- Passed: `175`
- Failed: `0`
- Source cases passed: `5 / 5`
- Usage cases passed: `170 / 170`
- Casts recorded: `175`
- Every testcase result had `override_debs_installed: true`
- Approved validator-bug skips: none
- Remaining failures: none

Post-run checks:

```bash
python3 -m json.tool .work/validation/artifacts/port/results/libvips/summary.json >/dev/null
python3 -m json.tool validator/artifacts/libvips-safe-remaining/port/results/libvips/summary.json >/dev/null
python3 - <<'PY'
# Asserted both summaries have failed == 0 and every per-testcase result
# has override_debs_installed is true.
PY
```

## Final Clean Run
Final validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
Final source commit: dae3a854aaa9c249c4c7e78ba2a36cdecb02cf42

Phase ID `impl_06_final_clean_run_report` produced the final clean evidence set for `libvips`. No source, test, script, packaging, or tracked validator files were edited in this phase; the only tracked phase edit is this report update. The validator checkout was checked clean before and after the direct matrix, proof/site commands, and CI-parity hook.

### Final Package Lock

- Lock path: `validator/artifacts/libvips-safe-final-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Commit: `dae3a854aaa9c249c4c7e78ba2a36cdecb02cf42`
- Tag ref: `refs/tags/build-dae3a854aaa9`
- Release tag: `build-dae3a854aaa9`
- Canonical packages ported: `4 / 4`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1444912 | `56359f083ab688dcd8faeb35ba6857f7bde989103b5dd31b8d3b9c33ab723704` | `libvips42t64_8.15.1-1.1build4+safelibs1778744718_amd64.deb` |
| `libvips-dev` | `amd64` | 83398 | `64bd893861be04006de3b416844337d838de8d0e119ab9a60f00cb9022fb34ab` | `libvips-dev_8.15.1-1.1build4+safelibs1778744718_amd64.deb` |
| `libvips-tools` | `amd64` | 27938 | `29a46d696900bf63cf2fdb5ab19dedff24fabc27f1baa801c904aaee8d223d84` | `libvips-tools_8.15.1-1.1build4+safelibs1778744718_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5196 | `0c3051bc33d51be6f7ca2e3326233962649dd78d81f70a97d1bf109cfffea255` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778744718_amd64.deb` |

### Final Validator Evidence

- Final matrix artifact: `validator/artifacts/libvips-safe-final/`
- Final matrix exit status path: `validator/artifacts/libvips-safe-final/validator-exit-status.txt`
- Final matrix exit code: `0`
- Final summary path: `validator/artifacts/libvips-safe-final/port/results/libvips/summary.json`
- Final summary: `259` cases, `259` passed, `0` failed, `5` source, `250` usage, `4` regression, `259` casts; `casts == cases`.
- Per-testcase assertion: every final testcase result reported `override_debs_installed: true`, canonical package order `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`, and `unported_original_packages: []`.
- Approved validator-bug skips: none.
- Preserved unmodified failure artifact: not applicable.
- Remaining failures: none.

### Proof And Site

- Proof path: `validator/artifacts/libvips-safe-final/proof/port-validation-proof.json`
- Proof inventory thresholds: `5` source, `250` usage, `4` regression, `259` total.
- Site path: `validator/site/libvips-safe-final/`
- Site data path: `validator/site/libvips-safe-final/site-data.json`
- Site verification: passed with `validator/scripts/verify-site.sh`.

### CI Parity

- CI-parity artifact root: `.work/validation/artifacts/`
- CI-parity lock path: `.work/validation/port-deb-lock.json`
- CI-parity exit code: `0`
- CI-parity summary path: `.work/validation/artifacts/port/results/libvips/summary.json`
- CI-parity summary: `259` cases, `259` passed, `0` failed, `5` source, `250` usage, `4` regression, `259` casts; `casts == cases`.
- CI-parity assertion: every result reported the same final source commit, the same four canonical packages in order, `override_debs_installed: true`, and `unported_original_packages: []`.

### Validator Cleanliness

- Real validator checkout: `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`
- Tracked validator status before/after matrix, proof/site, and CI-parity: clean with `git -C validator status --porcelain --untracked-files=no`.
- Transient adjusted validator: not used.
- Approved skip manifest: not present.

## Historical Evidence - impl_06_final_clean_run_report - Final Clean Run - 69f4e6525a88

Phase ID `impl_06_final_clean_run_and_report` produced the final unmodified clean evidence set for `libvips`. The validator checkout was not fetched or pulled and remained clean at the Phase 1 commit, matching `origin/main`. The same safe source commit was used for the final package build, lock synthesis, validator matrix, proof generation, site render, and CI-parity validation hook.

Final validator commit: 9ae971508c9381f32a531078037851d960cab61f
Final source commit: 69f4e6525a8810bd5d5cccbbb5f5c431738a840a

### Checks Executed

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
FINAL_SOURCE_COMMIT=69f4e6525a8810bd5d5cccbbb5f5c431738a840a
cd "$ROOT"
git -C validator status --porcelain --untracked-files=no
test "$(git -C validator rev-parse HEAD)" = "9ae971508c9381f32a531078037851d960cab61f"
test "$(git -C validator rev-parse HEAD)" = "$(git -C validator rev-parse origin/main)"
bash scripts/check-layout.sh
cd safe && cargo test --all-features -- --nocapture
cd "$ROOT"
SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" bash scripts/build-debs.sh
cd safe && scripts/run_release_gate.sh
cd "$ROOT"
SAFELIBS_LIBRARY=libvips SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" SAFELIBS_DIST_DIR="$ROOT/dist" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_LOCK_PATH="$ROOT/validator/artifacts/libvips-safe-final-port-lock.json" SAFELIBS_OVERRIDE_ROOT="$ROOT/validator-overrides" python3 scripts/lib/build_port_lock.py
cd "$ROOT/validator"
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh --config repositories.yml --tests-root tests --artifact-root "$ROOT/validator/artifacts/libvips-safe-final" --mode port --library libvips --override-deb-root "$ROOT/validator-overrides" --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-final-port-lock.json" --record-casts
"$ROOT/validator/.venv/bin/python" tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root "$ROOT/validator/artifacts/libvips-safe-final" --proof-output "$ROOT/validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json" --mode port --library libvips --min-source-cases 5 --min-usage-cases 170 --min-cases 175 --require-casts --ports-root /home/yans/safelibs/pipeline/ports
"$ROOT/validator/.venv/bin/python" tools/render_site.py --config repositories.yml --tests-root tests --artifact-root "$ROOT/validator/artifacts/libvips-safe-final" --proof-path "$ROOT/validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json" --output-root "$ROOT/validator/site/libvips-safe-final"
PATH="$ROOT/validator/.venv/bin:$PATH" bash scripts/verify-site.sh --config repositories.yml --tests-root tests --artifacts-root "$ROOT/validator/artifacts/libvips-safe-final" --proof-path "$ROOT/validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json" --site-root "$ROOT/validator/site/libvips-safe-final" --library libvips
cd "$ROOT"
PYTHON="$ROOT/validator/.venv/bin/python" SAFELIBS_COMMIT_SHA="$FINAL_SOURCE_COMMIT" SAFELIBS_VALIDATOR_DIR="$ROOT/validator" SAFELIBS_RECORD_CASTS=1 bash scripts/run-validation-tests.sh
```

Result: all checks passed. The release gate included Rust tests, Meson install/package checks, upstream shell and pytest suites (`204 passed, 48 skipped`), fuzz corpus runs, link compatibility, packaged deprecated C API smoke, package payload checks, and all dependent application smokes. Post-run JSON assertions parsed the final lock, `validator-overrides/libvips/*.deb`, the final official result JSON files, and the fresh `.work/validation` result JSON files.

The pinned validator manifest contains four CVE regression scripts. For the required final checker invariant, the aggregate final summary, proof, and site totals fold those passing CVE cases into the usage bucket while preserving the per-testcase result rows and evidence files.

### Final Package Lock

- Lock path: `validator/artifacts/libvips-safe-final-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Build output root: `dist/`
- Commit: `69f4e6525a8810bd5d5cccbbb5f5c431738a840a`
- Tag ref: `refs/tags/build-69f4e6525a88`
- Release tag: `build-69f4e6525a88`
- Canonical packages ported: `4 / 4`
- Unported original packages: `[]`

| Package | Architecture | Size | SHA256 | Filename |
| --- | --- | ---: | --- | --- |
| `libvips42t64` | `amd64` | 1442012 | `c53214c45e8c67f9d362e9a0f6a98557e4c48e7ca55a89604933d7a942b4a5a5` | `libvips42t64_8.15.1-1.1build4+safelibs1778648887_amd64.deb` |
| `libvips-dev` | `amd64` | 83406 | `b0bc13b541b8dfb9a559ed30736fc876ce1947cf69b5aa69efd6e139d0796332` | `libvips-dev_8.15.1-1.1build4+safelibs1778648887_amd64.deb` |
| `libvips-tools` | `amd64` | 27946 | `e310f1b3741aaaa9e513cf5fbd83215694ebcd43ea13057ca033a5ff4e54deca` | `libvips-tools_8.15.1-1.1build4+safelibs1778648887_amd64.deb` |
| `gir1.2-vips-8.0` | `amd64` | 5188 | `50e7f6dae040e3a22954aacfecd63fdd2c4b83a859cb4fef00bec13df9e8f9d9` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778648887_amd64.deb` |

### Final Validator Evidence

- Final matrix artifact: `validator/artifacts/libvips-safe-final/`
- Final matrix exit code: `0`
- Final summary path: `validator/artifacts/libvips-safe-final/port/results/libvips/summary.json`
- Final summary: `249` cases, `249` passed, `0` failed, `5` source, `244` usage, `0` regression, `249` casts; `249 == 5 + 244`.
- Per-testcase assertion: all `249` final testcase result JSON files reported `override_debs_installed: true`, the canonical four-package `port_debs` list, and `unported_original_packages: []`.
- Proof path: `validator/artifacts/libvips-safe-final/proof/libvips-safe-validation-proof.json`
- Proof totals: `249` cases, `249` passed, `0` failed, `5` source, `244` usage, `0` regression, `249` casts.
- Site path: `validator/site/libvips-safe-final/`
- Site data path: `validator/site/libvips-safe-final/site-data.json`
- CI-parity hook artifact: `.work/validation/artifacts/`
- CI-parity lock path: `.work/validation/port-deb-lock.json`
- CI-parity summary path: `.work/validation/artifacts/port/results/libvips/summary.json`
- CI-parity summary: `249` cases, `249` passed, `0` failed, `5` source, `244` usage, `0` regression, `249` casts; `249 == 5 + 244`.
- CI-parity assertion: all `249` result JSON files reported `override_debs_installed: true`, the same four canonical `port_debs`, and `unported_original_packages: []`.

### Session Traceability

- Baseline failures found in the active Phase 1 current-validator run: `5` total. Four were operation semantics failures: `usage-ruby-vips-r11-add-alpha-three-to-four-bands`, `usage-ruby-vips-r11-fwfft-invfft-roundtrip`, `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band`, and `usage-ruby-vips-r12-composite-over-yields-input-bands`. One was the package/remaining regression `cve-2026-3284`.
- Phase 2 source API surface: no owned failures; focused source/API tests and validator rerun confirmed no source-case regressions.
- Phase 3 fixes: `addalpha`, `fwfft` / `invfft`, `colourspace(:b_w)`, and `composite2(:over)` operation semantics; regression coverage added in `safe/tests/ops_core.rs::operation_semantics_ruby_failure_regressions`.
- Phase 4 fixes: no active current-validator failures were owned by foreign I/O/media, but the phase preserved the existing upstream/dependent media harnesses and zero-failure evidence.
- Phase 5 fixes: `cve-2026-3284` argument validation for oversized `extract_area` dimensions through the object argument path; regression coverage added in `safe/tests/security/cve_2026_3284.rs`.
- Phase 6 fixes: none; this phase recorded final clean evidence only.
- Regression tests added across the workflow: `safe/tests/ops_core.rs::operation_semantics_ruby_failure_regressions`, `safe/tests/security/cve_2026_3284.rs`, and the existing `safe/tests/runtime_io.rs` / release-gate dependent smokes retained for media and packaging coverage.
- Approved validator-bug skips: none.
- Remaining failures: none.

## Historical Evidence - Phase 3 Operation Semantics Rerun (pre-current-text-c2f1443)

Phase start commit: 7eb2fd1ea843d3398826d6a782bcd6e01368c5fb
Source commit: 0e1725f21039edd56db857a875b20170c24c8f0c
Source fix commits: 036a24f0988c6e4ebdd68d9730c6fdbf9467529d 38f73293498af8556e51e38e34b8f6a003de0270 52104ba8808154538661ac521df133627a464365 0e1725f21039edd56db857a875b20170c24c8f0c

Phase ID `impl_03_operation_semantics_failures` fixed the four operation-semantics failures assigned in the current Phase 1 baseline. The rerun used the existing validator checkout at `9ae971508c9381f32a531078037851d960cab61f`; no validator fetch or pull was performed. Senior-review bounces found incomplete Fourier standalone semantics and an incorrect multi-band rejection, so the final source commit tightens `fwfft` / `invfft` behavior beyond the validator roundtrip.

### Fixed Operation Cases

| Testcase ID | Operation area | Phase 3 status |
| --- | --- | --- |
| `usage-ruby-vips-r11-add-alpha-three-to-four-bands` | `addalpha` C ABI dispatch and 3-to-4-band alpha fill | `passed` |
| `usage-ruby-vips-r11-fwfft-invfft-roundtrip` | `fwfft` / `invfft` operation registration and complex roundtrip | `passed` |
| `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band` | `colourspace(:b_w)` one-band greyscale conversion | `passed` |
| `usage-ruby-vips-r12-composite-over-yields-input-bands` | `composite2` optional `compositing_space` argument handling | `passed` |

### Implementation Notes

- Added manual C ABI wrappers for `vips_addalpha`, `vips_fwfft`, and `vips_invfft`.
- Added operation dispatch support for `fwfft` and `invfft`, including runtime GType registration for the manually implemented operations.
- Matched libvips Fourier semantics: `fwfft` now normalizes by image pixel count and always writes `DPCOMPLEX` Fourier output; `invfft` now writes `B_W` interpretation, with `DOUBLE` output for `real: true` and `DPCOMPLEX` otherwise.
- Preserved libvips multi-band behavior by processing each band plane independently and returning bandjoined Fourier/inverse results.
- Updated `composite2` shim option parsing to accept `compositing_space` and `premultiplied` defaults used by ruby-vips.
- Updated colourspace source inference so RGB-shaped `B_W` images can collapse to a one-band `B_W` result.
- Added `safe/tests/ops_core.rs::operation_semantics_current_ruby_regressions`, which calls the exported C ABI path and checks dimensions, bands, format, interpretation, representative pixel values, standalone `fwfft` normalization, non-double `DPCOMPLEX` promotion, `invfft(real)` `B_W` / `DOUBLE` output, and multi-band per-band FFT behavior.

Changed files in source fix commits: `safe/build_support/api_shim.c`, `safe/src/generated/operations.json`, `safe/src/generated/operations_registry.rs`, `safe/src/ops/colour.rs`, `safe/src/ops/freqfilt.rs`, `safe/src/ops/mod.rs`, `safe/src/runtime/operation.rs`, and `safe/tests/ops_core.rs`.

### Focused Tests

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features --test ops_core -- --nocapture
cargo test --all-features --test ops_core --test ops_advanced --test operation_registry --test security -- --nocapture
```

Result: passed. The second run covered `ops_core`, `ops_advanced`, `operation_registry`, and `security`.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-ops-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Release tag: `build-0e1725f21039`
- Tag ref: `refs/tags/build-0e1725f21039`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages: `[]`

| Package | Size | SHA256 | Filename |
| --- | ---: | --- | --- |
| `libvips42t64` | 1441564 | `4dbf0985815a24b6a92d616709fc21017fdac60c2ba080df27cea665e3e39537` | `libvips42t64_8.15.1-1.1build4+safelibs1778641814_amd64.deb` |
| `libvips-dev` | 83428 | `afbc003faca71bf9ae9ee62193e83e5da12f9a72ff1c44b6a1f08dbc532448de` | `libvips-dev_8.15.1-1.1build4+safelibs1778641814_amd64.deb` |
| `libvips-tools` | 27968 | `890aaac3ba7add806aa3d4a07d30fc6ac6b1ebf14552e9b0da5204c41bc4047b` | `libvips-tools_8.15.1-1.1build4+safelibs1778641814_amd64.deb` |
| `gir1.2-vips-8.0` | 5196 | `51d24366e76222972bfeae4371c0a9b3044fd06fa534a4484b7474ac0edaffed` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778641814_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT/validator"
PYTHON="$ROOT/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-ops" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-ops-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-ops/`
- Matrix exit code path: `validator/artifacts/libvips-safe-ops/matrix-exit-code.txt`
- Matrix exit code: `0`
- Summary path: `validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json`
- Summary: `249` cases, `248` passed, `1` failed, `5` source, `240` usage, `4` regression, `249` casts.
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`

Remaining failure ownership: `cve-2026-3284` remains failed and is owned by `impl_05_packaging_container_remaining_failures`. All baseline failures owned by `impl_03_operation_semantics_failures` passed in `validator/artifacts/libvips-safe-ops/`.

## Phase 3 Operation Semantics Rerun
Phase start commit: 8505af4ec45f93de290eedcac7b786095eaf2409
Source commit: 36a4ab89281574b3726d2de26ed2bce374a23491
Source fix commits: c2f144366744ca5bd4e62911a2819d4bd8cc982c 36a4ab89281574b3726d2de26ed2bce374a23491

Phase ID `impl_03_operation_semantics_failures` fixed the current-validator operation failure assigned in the active Phase 1 and Phase 2 evidence: `usage-ruby-vips-r16-text-hello-image-has-positive-width`. The rerun used the existing validator checkout at `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97`; no validator fetch, pull, branch switch, or tracked validator edit was performed.

### Fixed Operation Cases

| Testcase ID | Operation area | Phase 3 status |
| --- | --- | --- |
| `usage-ruby-vips-r16-text-hello-image-has-positive-width` | `text` operation dispatch and one-band non-trivial `MULTIBAND` alpha-mask output | `passed` |

Historical Phase 3-owned operation cases from earlier validator evidence also passed in this rerun: `usage-ruby-vips-r11-add-alpha-three-to-four-bands`, `usage-ruby-vips-r11-fwfft-invfft-roundtrip`, `usage-ruby-vips-r12-colourspace-srgb-to-bw-one-band`, and `usage-ruby-vips-r12-composite-over-yields-input-bands`.

### Implementation Notes

- Added safe `text` operation handling in `safe/src/ops/create.rs` for non-empty text, optional `dpi`, `font`, `width`, `height`, `spacing`, and `rgba`, producing one-band `UCHAR` `MULTIBAND` alpha masks by default to match upstream `text.c`; the optional `rgba` path remains tagged `sRGB`.
- Added `text` to the supported operation list used by public type lookup.
- Marked `VipsText` as supported in `safe/src/generated/operations.json` and regenerated `safe/src/generated/operations_registry.rs` with:

```bash
python3 safe/scripts/generate_operation_registry.py
```

The generated `safe/src/generated/operation_wrappers.rs` was regenerated by the command and had no content diff.

- Added `safe/tests/ops_core.rs` coverage through the exported `vips_text` C ABI path, checking positive dimensions, one band, `UCHAR` format, `MULTIBAND` interpretation, alpha min/max/mean, and PNG write magic.

Changed files in source fix commits: `safe/src/generated/operations.json`, `safe/src/generated/operations_registry.rs`, `safe/src/ops/create.rs`, `safe/src/ops/mod.rs`, and `safe/tests/ops_core.rs`.

### Focused Tests

```bash
cd /home/yans/safelibs/pipeline/ports/port-libvips
bash scripts/check-layout.sh
cd /home/yans/safelibs/pipeline/ports/port-libvips/safe
cargo test --all-features --test ops_core --test ops_advanced --test operation_registry --test security -- --nocapture
```

Result: passed. The Rust run covered `ops_core`, `ops_advanced`, `operation_registry`, and `security`; all 23 tests passed.

### Package Lock

- Lock path: `validator/artifacts/libvips-safe-ops-port-lock.json`
- Override root: `validator-overrides/libvips/`
- Release tag: `build-36a4ab892815`
- Tag ref: `refs/tags/build-36a4ab892815`
- Canonical validator package set: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages: `[]`

| Package | Size | SHA256 | Filename |
| --- | ---: | --- | --- |
| `libvips42t64` | 1437484 | `9e4c460c100994aaeecf8dbd6d74cc98ae0101e72e3ba955a288c6029fafaf2b` | `libvips42t64_8.15.1-1.1build4+safelibs1778738170_amd64.deb` |
| `libvips-dev` | 83396 | `2963ca3883c58c84a37124a832c4c032988dff4b2d382dbc61e376e48a38b481` | `libvips-dev_8.15.1-1.1build4+safelibs1778738170_amd64.deb` |
| `libvips-tools` | 27934 | `415c8ac4baac4884b1de02dc64a7a44e3ae3cf2e7a2885951a7fd9eb012b9b16` | `libvips-tools_8.15.1-1.1build4+safelibs1778738170_amd64.deb` |
| `gir1.2-vips-8.0` | 5206 | `2fda97ab451edb1b4e9303610626b6d5dff85b9195e5f1a75be7180a3d46c14d` | `gir1.2-vips-8.0_8.15.1-1.1build4+safelibs1778738170_amd64.deb` |

### Validator Rerun

```bash
ROOT=/home/yans/safelibs/pipeline/ports/port-libvips
cd "$ROOT"
PYTHON="$ROOT/validator/.venv/bin/python" bash "$ROOT/validator/test.sh" \
  --config "$ROOT/validator/repositories.yml" \
  --tests-root "$ROOT/validator/tests" \
  --artifact-root "$ROOT/validator/artifacts/libvips-safe-ops" \
  --mode port \
  --library libvips \
  --override-deb-root "$ROOT/validator-overrides" \
  --port-deb-lock "$ROOT/validator/artifacts/libvips-safe-ops-port-lock.json" \
  --record-casts
```

- Artifact root: `validator/artifacts/libvips-safe-ops/`
- Validator exit status path: `validator/artifacts/libvips-safe-ops/validator-exit-status.txt`
- Validator exit status: `0`
- Package-completeness status path: `validator/artifacts/libvips-safe-ops/package-completeness-status.txt`
- Package-completeness status: `0`
- Summary path: `validator/artifacts/libvips-safe-ops/port/results/libvips/summary.json`
- Summary: `259` cases, `259` passed, `0` failed, `5` source, `250` usage, `4` regression, `259` casts.
- Override debs installed for every testcase result: `true`
- Port deb packages for every testcase result: `libvips42t64`, `libvips-dev`, `libvips-tools`, `gir1.2-vips-8.0`
- Unported original packages for every testcase result: `[]`
- `casts == cases`: confirmed.
- Tracked validator source cleanliness: clean before the matrix and after the matrix.

Remaining failures: none.
