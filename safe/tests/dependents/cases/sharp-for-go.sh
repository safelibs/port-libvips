#!/usr/bin/env bash

run_case() {
  log "Testing sharp-for-go"

  local gopath=/tmp/sharp-for-go-gopath
  local src_dir="${gopath}/src/github.com/DAddYE/vips"
  rm -rf "${gopath}"
  mkdir -p "$(dirname "${src_dir}")"
  register_cleanup "${gopath}"
  clone_git_ref sharp-for-go "${src_dir}"

  (
    cd "${src_dir}"
    GOPATH="${gopath}" GO111MODULE=off run_manifest_smoke_command sharp-for-go "${src_dir}"
  )
}
