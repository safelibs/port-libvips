#!/usr/bin/env bash

run_case() {
  log "Building and testing nip2"
  apt-get build-dep -y nip2

  rm -rf /tmp/nip2-src
  mkdir -p /tmp/nip2-src
  register_cleanup /tmp/nip2-src
  (
    cd /tmp/nip2-src
    apt-get source nip2
  )

  local src_dir
  src_dir="$(find /tmp/nip2-src -maxdepth 1 -mindepth 1 -type d -name 'nip2-*' | head -n 1)"
  if [[ -z "${src_dir}" ]]; then
    echo "failed to locate nip2 source tree" >&2
    exit 1
  fi

  (
    cd "${src_dir}"
    ./configure --disable-silent-rules
    make -j"${JOBS:-$(nproc)}"
    chmod +x test/test_all.sh
    mkdir -p /tmp/nip2-home
    prepare_vips_module_overlay "${src_dir}"
    VIPSHOME="${src_dir}" vips --version >/dev/null
    HOME=/tmp/nip2-home VIPSHOME="${src_dir}" run_manifest_smoke_command nip2 "${src_dir}"
  )
}
