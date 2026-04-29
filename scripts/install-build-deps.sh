#!/usr/bin/env bash
# Install apt packages and rust 1.78 (pinned by the port) for libvips.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  curl \
  devscripts \
  dpkg-dev \
  equivs \
  fakeroot \
  file \
  git \
  jq \
  python3 \
  rsync \
  xz-utils

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain 1.78 --no-modify-path

# shellcheck source=/dev/null
. "$HOME/.cargo/env"
rustup default 1.78
rustc --version
cargo --version

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi
