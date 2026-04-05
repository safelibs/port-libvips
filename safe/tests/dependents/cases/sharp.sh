#!/usr/bin/env bash

run_case() {
  log "Testing sharp"

  local src_dir=/tmp/sharp-src
  rm -rf "${src_dir}"
  mkdir -p "${src_dir}"
  register_cleanup "${src_dir}"

  (
    cd "${src_dir}"
    npm init -y >/dev/null
    SHARP_FORCE_GLOBAL_LIBVIPS=1 npm_config_build_from_source=true npm install sharp@0.32.6
    cat > safe-vips-smoke.mjs <<'EOF'
import sharp from 'sharp';

const input = Buffer.alloc(8 * 6 * 3, 32);

const { data, info } = await sharp(input, {
  raw: {
    width: 8,
    height: 6,
    channels: 3
  }
})
  .resize(4, 4)
  .png()
  .toBuffer({ resolveWithObject: true });

if (sharp.versions.vips !== '8.15.1') {
  throw new Error(`sharp linked against unexpected libvips version ${sharp.versions.vips}`);
}
if (info.width !== 4 || info.height !== 4 || data.length === 0) {
  throw new Error('sharp failed to produce the expected resized image');
}
EOF
    run_manifest_smoke_command sharp "${src_dir}"
  )
}
