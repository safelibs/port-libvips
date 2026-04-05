#!/usr/bin/env bash

run_case() {
  log "Testing imgproxy"

  local src_dir=/tmp/imgproxy-src
  clone_git_ref imgproxy "${src_dir}"
  register_cleanup "${src_dir}"

  (
    cd "${src_dir}"
    run_manifest_smoke_command imgproxy "${src_dir}"
  )
}
