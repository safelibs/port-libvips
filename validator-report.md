# libvips-safe Validator Report

## validator checkout
- Validator URL: https://github.com/safelibs/validator
- remote main commit: dc9f47b6054e9a51afde8a437a2e5e5562cc946a
- active validator commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- active validator reason: remote main manifest unusable for libvips; using last known runnable validator commit
- Selection artifact: validator/artifacts/libvips-safe-validator-selection.txt
- Inventory artifact: validator/artifacts/libvips-safe-inventory.json
- Manifest SHA-256: c44346195fbfa8dd5de2c29b14ac9474eb3b91802d4fc2e7a5325141f9ee6140
- Python: validator/.venv/bin/python imports PyYAML 6.0.3; host python3 also imports PyYAML 6.0.3.
- Validator input model: README.md, repositories.yml, tests/libvips/testcases.yml, and test.sh confirm validation installs `.deb` packages from `<override-deb-root>/<library>/*.deb`; it does not validate a raw library path.
- Bootstrap note: origin/main was fetched and inspected first. Its libvips manifest no longer contains a non-empty `testcases` list, so the checkout was moved to the pinned fallback commit with 85 runnable libvips cases.

## Package Inputs
- Package-source commit: fc7c6a5171b9bccb3aad6ecb503b9d70a7c612c5
- Safe source edits in this phase: none.
- Package rebuild command: `cd /home/yans/safelibs/pipeline/ports/port-libvips/safe && dpkg-buildpackage -b -uc -us`
- Package staging command: refreshed validator-overrides/libvips from the rebuilt root-level `.deb` files for the four canonical packages.
- Port lock: validator/artifacts/libvips-safe-port-lock.json

| Package | Override path | Architecture | Size | SHA-256 |
| --- | --- | --- | --- | --- |
| libvips42t64 | validator-overrides/libvips/libvips42t64_8.15.1-1.1build4_amd64.deb | amd64 | 1300482 | 1d22c27d7893a2d510d8d5e3cce68a03258a485f86df1c9df5ab7803049b7567 |
| libvips-dev | validator-overrides/libvips/libvips-dev_8.15.1-1.1build4_amd64.deb | amd64 | 83304 | baa99134376d9bd7f0ebe33ab98a879a3c5555d6a57304c223871ec388e6ef98 |
| libvips-tools | validator-overrides/libvips/libvips-tools_8.15.1-1.1build4_amd64.deb | amd64 | 27852 | c6d324c9d891bacd7b096d51052dcb88f467eb0f71ccac01e783fb43337a48be |
| gir1.2-vips-8.0 | validator-overrides/libvips/gir1.2-vips-8.0_8.15.1-1.1build4_amd64.deb | amd64 | 5104 | 362d0824adb9f58e64c4d0932175ac976330db5fb0f74ddbf96a1020ac790c82 |

## Manifest Counts
- Source cases: 5
- Usage cases: 80
- Total cases: 85
- Source IDs: vips-cli-load-save, thumbnail-behavior, c-api-compile-smoke, gir-introspection-smoke, metadata-header-checks

## Baseline Command
```bash
ARTIFACT_NAME=libvips-safe
cd /home/yans/safelibs/pipeline/ports/port-libvips/validator
PYTHON="/home/yans/safelibs/pipeline/ports/port-libvips/validator/.venv/bin/python" RECORD_CASTS=1 bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libvips-safe \
  --mode port-04-test \
  --library libvips \
  --override-deb-root /home/yans/safelibs/pipeline/ports/port-libvips/validator-overrides \
  --port-deb-lock /home/yans/safelibs/pipeline/ports/port-libvips/validator/artifacts/libvips-safe-port-lock.json \
  --record-casts
```

## Result Summary
- Artifact root: validator/artifacts/libvips-safe
- Matrix exit status: 0
- Matrix exit artifact: validator/artifacts/libvips-safe/matrix-exit-code.txt
- Summary artifact: validator/artifacts/libvips-safe/port-04-test/results/libvips/summary.json
- Result JSON records: 85 testcase records plus summary.json
- Cast records: 85
- Passed: 81
- Failed: 4
- Source cases: 5
- Usage cases: 80
- Override package install failures: none

## failure classification
| Testcase ID | Kind | Status | Owner phase | First artifact | Root cause | Regression test | Resolution |
| --- | --- | --- | --- | --- | --- | --- | --- |
| vips-cli-load-save | source | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/vips-cli-load-save.json | JPEG load/save materialization falls back to external `convert` inside the validator container, which is absent; `copy` fails before PNG output. | Add phase 4 coverage for JPEG file load and save materialization without external decoder fallback. | Open for `impl_04_foreign_io_buffer_failures`. |
| thumbnail-behavior | source | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/thumbnail-behavior.json | `vipsthumbnail` hits the same JPEG materialization path and fails when external `convert` is absent. | Add phase 4 coverage for thumbnailing a JPEG fixture without external decoder fallback. | Open for `impl_04_foreign_io_buffer_failures`. |
| usage-ruby-vips-crop-sample-jpeg | usage | open | `impl_04_foreign_io_buffer_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-crop-sample-jpeg.json | Ruby crop of a JPEG fixture fails during materialization through the missing external `convert` path before `extract_area` can complete. | Add phase 4 coverage for ruby-vips JPEG fixture load, crop, and materialization through file or buffer APIs. | Open for `impl_04_foreign_io_buffer_failures`. |
| usage-ruby-vips-gravity-generated | usage | open | `impl_03_ruby_usage_operation_failures` | validator/artifacts/libvips-safe/port-04-test/results/libvips/usage-ruby-vips-gravity-generated.json | Generated-image `gravity` dispatch reaches ruby-vips but the safe operation implementation returns `gravity: operation not implemented`. | Add phase 3 coverage for generated-image gravity crop behavior and centered pixel payload. | Open for `impl_03_ruby_usage_operation_failures`. |

## Later Owner Phases
- `impl_02_source_surface_failures`: no baseline failure is assigned here because the source-case failures depend on JPEG decode, save, or materialization rather than command, header, package identity, or metadata surface alone.
- `impl_03_ruby_usage_operation_failures`: owns generated ruby-vips operation behavior failures.
- `impl_04_foreign_io_buffer_failures`: owns file, buffer, loader, saver, lazy materialization, and external decoder fallback failures.
- `impl_05_packaging_container_and_remaining_failures`: no baseline packaging or container setup failure was observed; all override packages installed and matrix exit was 0.
