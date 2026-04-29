#!/usr/bin/env bash
# libvips: stage upstream meson build for libvips*.so symlinks the safe
# debian rules expect, then dpkg-buildpackage with nocheck.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

# shellcheck source=/dev/null
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

if [[ -d "$HOME/.cargo/bin" ]]; then
  case ":$PATH:" in
    *":$HOME/.cargo/bin:"*) ;;
    *) export PATH="$HOME/.cargo/bin:$PATH" ;;
  esac
fi

rm -rf -- "$dist_dir"
mkdir -p -- "$dist_dir"

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
export DEB_BUILD_OPTIONS="${DEB_BUILD_OPTIONS:+$DEB_BUILD_OPTIONS }nocheck"
dpkg-buildpackage -us -uc -b
cp -v ../*.deb "$dist_dir"/
