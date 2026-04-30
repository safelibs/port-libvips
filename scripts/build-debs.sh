#!/usr/bin/env bash
# libvips: stage upstream meson build for libvips*.so symlinks the safe
# debian rules expect, then dpkg-buildpackage with nocheck.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=/dev/null
. "$repo_root/scripts/lib/build-deb-common.sh"

prepare_rust_env
prepare_dist_dir "$repo_root"

cd "$repo_root/safe"
sudo mk-build-deps -i -r -t "apt-get -y --no-install-recommends" debian/control
cd "$repo_root"

rm -rf build-check build-check-install
meson setup build-check original --prefix "$repo_root/build-check-install"
meson compile -C build-check
meson install -C build-check
multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
cp -a "build-check-install/lib/$multiarch"/libvips*.so* build-check-install/lib/

cd "$repo_root/safe"
stamp_safelibs_changelog "$repo_root"
export DEB_BUILD_OPTIONS="${DEB_BUILD_OPTIONS:+$DEB_BUILD_OPTIONS }nocheck"
dpkg-buildpackage -us -uc

shopt -s nullglob
artifacts=(
  ../*.deb
  ../*.ddeb
  ../*.dsc
  ../*.tar.gz ../*.tar.xz ../*.tar.bz2 ../*.tar.zst
  ../*.buildinfo
  ../*.changes
)
shopt -u nullglob
cp -v "${artifacts[@]}" "$repo_root/dist"/
