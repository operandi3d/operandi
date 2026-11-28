#!/bin/sh
# Builds this kit's helper for this machine and puts it where kit.toml
# expects it (bin/<os>-<arch>/). scripts/package-kit.sh runs it first.
set -eu
cd "$(dirname "$0")"
cargo build --release --manifest-path helper/Cargo.toml
os=$(uname -s); arch=$(uname -m)
case "$os" in Darwin) os=macos ;; Linux) os=linux ;; *) os=windows ;; esac
case "$arch" in arm64) arch=aarch64 ;; amd64) arch=x86_64 ;; esac
exe=off-import-helper
[ "$os" = windows ] && exe=$exe.exe
mkdir -p "bin/$os-$arch"
cp "helper/target/release/$exe" "bin/$os-$arch/$exe"
echo "Built bin/$os-$arch/$exe"
