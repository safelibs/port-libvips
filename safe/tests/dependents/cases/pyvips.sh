#!/usr/bin/env bash

run_case() {
  log "Testing pyvips from the vendored workspace snapshot"

  local src_dir=/tmp/pyvips-src
  copy_workspace_source pyvips "${src_dir}"
  register_cleanup "${src_dir}"

  cat >"${src_dir}/safe-vips-smoke.py" <<'PY'
import pyvips

image = pyvips.Image.black(8, 6)
if image.width != 8 or image.height != 6:
    raise SystemExit("unexpected pyvips image dimensions")
if image.avg() != 0.0:
    raise SystemExit("pyvips average should be 0.0 for a black image")
buffer = image.pngsave_buffer()
if not buffer:
    raise SystemExit("pyvips pngsave_buffer() returned no data")
PY

  (
    cd "${src_dir}"
    PYTHONPATH="${src_dir}" run_manifest_smoke_command pyvips "${src_dir}"
  )
}
