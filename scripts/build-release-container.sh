#!/bin/sh
# Builds the Linux release inside an older Linux (Rocky Linux 8, glibc 2.28)
# with podman, so the AppImage also runs on older distributions. Same output
# as scripts/build-release.sh on Linux. Needs podman (sudo apt install podman).
#
# The container's Cycles build goes in development/cycles-rocky8 and its
# Rust build in target-rocky8, beside the normal ones; the Cycles download
# (development/cycles/src) is shared.
set -eu
cd "$(dirname "$0")/.."
IMAGE=operandi-linux-build:rocky8
podman build -t "$IMAGE" scripts/linux-build-container
OUTPUT="${OPERANDI_RELEASE_OUTPUT:-$HOME/Developer/Operandi-Builds}"
mkdir -p "$OUTPUT" "$HOME/.cache/operandi-container-cargo" "$HOME/.cache/operandi-container-rustup"
# The repo is mounted at /src and the output folder at /out. Cargo's
# download cache stays on the host between runs.
podman run --rm \
    -v "$PWD:/src:Z" -v "$OUTPUT:/out:Z" \
    -v "$HOME/.cache/operandi-container-cargo:/root/.cargo/registry:Z" \
    -v "$HOME/.cache/operandi-container-rustup:/root/.rustup:Z" \
    -w /src \
    -e OPERANDI_CYCLES_OUT=development/cycles-rocky8 \
    -e CARGO_TARGET_DIR=/src/target-rocky8 \
    -e OPERANDI_RELEASE_OUTPUT=/out \
    "$IMAGE" scripts/build-release.sh
