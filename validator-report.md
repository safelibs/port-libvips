# Validator Report

## Phase 1 Current Validator Baseline

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

## Phase 3 Operation Semantics Rerun

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
