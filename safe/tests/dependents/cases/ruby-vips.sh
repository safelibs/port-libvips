#!/usr/bin/env bash

run_case() {
  log "Building and testing ruby-vips"
  apt-get build-dep -y ruby-vips

  rm -rf /tmp/ruby-vips-src
  mkdir -p /tmp/ruby-vips-src
  register_cleanup /tmp/ruby-vips-src
  (
    cd /tmp/ruby-vips-src
    apt-get source ruby-vips
  )

  local src_dir
  src_dir="$(find /tmp/ruby-vips-src -maxdepth 1 -mindepth 1 -type d -name 'ruby-vips-*' | head -n 1)"
  if [[ -z "${src_dir}" ]]; then
    echo "failed to locate ruby-vips source tree" >&2
    exit 1
  fi

  patch_ruby_vips_for_reference_metadata_surface "${src_dir}"

  (
    cd "${src_dir}"
    run_manifest_smoke_command ruby-vips "${src_dir}"
  )
}
