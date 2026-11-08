#!/usr/bin/env bash
# Gets what building CutZone needs on Ubuntu or Debian, WSL2 included, and prepares THIS shell for
# cargo. It installs what the build needs, downloads FFmpeg 7+ and ONNX Runtime at the versions
# scripts/ci/deps.lock pins (checked against their digests) into ~/cutzone-deps, and installs the
# pinned Rust toolchain. Run it with `source` so the variables stay in your shell:
#
#   source scripts/linux-dev-env.sh
#
# Ubuntu 24.04 ships FFmpeg 6.1, which is too old, which is why FFmpeg is downloaded and not apt-got.
_cz_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
_cz_deps=${CUTZONE_DEPS:-$HOME/cutzone-deps}
_cz_sudo=""
[ "$(id -u)" -ne 0 ] && _cz_sudo=sudo

(
  set -euo pipefail
  $_cz_sudo apt-get update -qq
  $_cz_sudo apt-get install -y -qq --no-install-recommends build-essential pkg-config cmake \
    libclang-dev libfontconfig1-dev libfreetype6-dev libxkbcommon-dev libgl1-mesa-dev libgtk-3-dev \
    libasound2-dev libssl-dev libvulkan1 mesa-vulkan-drivers xz-utils curl ca-certificates git \
    python3 ffmpeg
  mkdir -p "$_cz_deps"
  if [ ! -d "$_cz_deps/ffmpeg/lib" ]; then
    "$_cz_root/scripts/ci/fetch.sh" ffmpeg-linux64 "$_cz_deps/ffmpeg.tar.xz"
    mkdir -p "$_cz_deps/ffmpeg" && tar -xf "$_cz_deps/ffmpeg.tar.xz" -C "$_cz_deps/ffmpeg" --strip-components=1
    rm -f "$_cz_deps/ffmpeg.tar.xz"
  fi
  if [ ! -f "$_cz_deps/onnxruntime/lib/libonnxruntime.so" ]; then
    "$_cz_root/scripts/ci/fetch.sh" onnxruntime-linux-x64 "$_cz_deps/ort.tgz"
    mkdir -p "$_cz_deps/onnxruntime" && tar -xzf "$_cz_deps/ort.tgz" -C "$_cz_deps/onnxruntime" --strip-components=1
    ln -sf "$(cd "$_cz_deps/onnxruntime/lib" && ls libonnxruntime.so.* | head -1)" "$_cz_deps/onnxruntime/lib/libonnxruntime.so"
    rm -f "$_cz_deps/ort.tgz"
  fi
  if ! command -v rustup >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/rustup" ]; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
  (cd "$_cz_root/src" && rustup toolchain install "$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)" \
    --profile minimal -c rustfmt -c clippy)
) || { echo "linux-dev-env: setup failed, see above" >&2; return 1 2>/dev/null || exit 1; }

# shellcheck disable=SC1091
. "$HOME/.cargo/env"
export FFMPEG_DIR="$_cz_deps/ffmpeg"
export PKG_CONFIG_PATH="$FFMPEG_DIR/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export ORT_LIB_LOCATION="$_cz_deps/onnxruntime/lib" ORT_PREFER_DYNAMIC_LINK=1
export LD_LIBRARY_PATH="$FFMPEG_DIR/lib:$ORT_LIB_LOCATION${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# Two compile jobs and no debug info keep the build inside an ordinary machine's memory.
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2} CARGO_PROFILE_DEV_DEBUG=${CARGO_PROFILE_DEV_DEBUG:-0}
# No graphics card (WSL2 without GPU support, a container): name the software Vulkan driver, or the
# renderer can wait for minutes on a driver that never answers.
if [ ! -d /dev/dri ] && [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan
fi
echo "linux-dev-env: ready. Next:  cd src && cargo build -p concat-cli"
unset _cz_root _cz_deps _cz_sudo
