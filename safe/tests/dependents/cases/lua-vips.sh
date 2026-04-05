#!/usr/bin/env bash

run_case() {
  log "Testing lua-vips"

  local src_dir=/tmp/lua-vips-src
  clone_git_ref lua-vips "${src_dir}"
  register_cleanup "${src_dir}"

  cat >"${src_dir}/safe-vips-smoke.lua" <<'EOF'
package.path = table.concat({
  "./src/?.lua",
  "./src/?/init.lua",
  package.path,
}, ";")

local vips = require "vips"
local image = vips.Image.black(8, 6)
assert(image:avg() == 0.0, "lua-vips average should be 0.0")
local out = image:pngsave_buffer()
assert(#out > 0, "lua-vips pngsave_buffer() returned no data")
EOF

  (
    cd "${src_dir}"
    run_manifest_smoke_command lua-vips "${src_dir}"
  )
}
