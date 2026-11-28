#!/bin/sh
# Packs a kit folder into the zip people install with
# `operandi.kit.install path:<zip>` (docs/KITS.md, "Packaging").
#
#   scripts/package-kit.sh <kit folder> [output folder]
#
# Runs the kit's own build-kit.sh first when it has one (to build its
# helper), checks kit.toml names an id and version, and writes
# <id>-<version>-<os>-<arch>.zip (default output: development/kits/). The
# zip holds the kit at its top; bin/ holds only this machine's helper, so
# build one zip per platform. Leaves out build folders and dot files.
set -eu
[ $# -ge 1 ] || { echo "usage: $0 <kit folder> [output folder]" >&2; exit 2; }
KIT=$(cd "$1" && pwd)
OUT=${2:-$(cd "$(dirname "$0")/.." && pwd)/development/kits}
[ -f "$KIT/kit.toml" ] || { echo "$KIT has no kit.toml" >&2; exit 1; }
if [ -x "$KIT/build-kit.sh" ]; then
    "$KIT/build-kit.sh"
fi
field() {
    sed -n "s/^$1 *= *\"\(.*\)\"/\1/p" "$KIT/kit.toml" | head -n 1
}
ID=$(field id)
VERSION=$(field version)
[ -n "$ID" ] && [ -n "$VERSION" ] || { echo "kit.toml needs id and version" >&2; exit 1; }
[ "$(basename "$KIT")" = "$ID" ] || echo "note: folder $(basename "$KIT") isn't named after the kit id $ID; it installs as $ID" >&2
os=$(uname -s); arch=$(uname -m)
case "$os" in Darwin) os=macos ;; Linux) os=linux ;; *) os=windows ;; esac
case "$arch" in arm64) arch=aarch64 ;; amd64) arch=x86_64 ;; esac
mkdir -p "$OUT"
ZIP="$OUT/$ID-$VERSION-$os-$arch.zip"
rm -f "$ZIP"
(cd "$KIT" && zip -q -r -X "$ZIP" . \
    -x '.*' -x '*/.*' -x 'target/*' -x '*/target/*' -x 'helper/Cargo.lock')
echo "Packed $ZIP"
