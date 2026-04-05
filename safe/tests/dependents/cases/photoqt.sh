#!/usr/bin/env bash

run_case() {
  log "Building and testing photoqt"
  apt-get build-dep -y photoqt

  rm -rf /tmp/photoqt-src
  mkdir -p /tmp/photoqt-src
  register_cleanup /tmp/photoqt-src
  (
    cd /tmp/photoqt-src
    apt-get source photoqt
  )

  local src_dir
  src_dir="$(find /tmp/photoqt-src -maxdepth 1 -mindepth 1 -type d -name 'photoqt-*' | head -n 1)"
  if [[ -z "${src_dir}" ]]; then
    echo "failed to locate photoqt source tree" >&2
    exit 1
  fi

  patch_photoqt_for_libvips_smoke_test "${src_dir}"

  (
    cd "${src_dir}"
    cmake -S . -B build -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DDEVIL=OFF \
      -DFREEIMAGE=OFF \
      -DGRAPHICSMAGICK=OFF \
      -DIMAGEMAGICK=OFF \
      -DLIBVIPS=ON \
      -DPOPPLER=OFF \
      -DRESVG=OFF \
      -DTESTING=ON
    cmake --build build --parallel "${JOBS:-$(nproc)}"
    mkdir -p /tmp/photoqt-home /tmp/photoqt-config
    HOME=/tmp/photoqt-home XDG_CONFIG_HOME=/tmp/photoqt-config run_manifest_smoke_command photoqt "${src_dir}"
  )
}
