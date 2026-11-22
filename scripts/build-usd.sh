#!/bin/sh
# Builds liboperandi_usd (Operandi's C bridge to OpenUSD) into
# development/usd/install/lib, ready for `--features usd`.
#
#   scripts/build-usd.sh
#
# OpenUSD comes prebuilt from Blender's library pack, the same download
# scripts/build-cycles.sh makes (development/cycles/src/lib/<platform>).
# A worktree without its own download uses the main checkout's.
# OPERANDI_LIBRARY_PACK=<folder> points somewhere else.
#
# macOS only for now (stage 7 of the USD plan adds Windows and Linux).
set -eu
cd "$(dirname "$0")/.."
HERE="$PWD"
case "$(uname)" in
    Darwin) PLATFORM=macos_$(uname -m | sed 's/x86_64/x64/') ;;
    *) echo "build-usd.sh supports macOS for now"; exit 1 ;;
esac

PACK="${OPERANDI_LIBRARY_PACK:-}"
if [ -z "$PACK" ]; then
    for root in "$HERE" "$(cd "$(git rev-parse --git-common-dir)/.." && pwd)"; do
        if [ -d "$root/development/cycles/src/lib/$PLATFORM/usd" ]; then
            PACK="$root/development/cycles/src/lib/$PLATFORM"
            break
        fi
    done
fi
[ -n "$PACK" ] && [ -d "$PACK/usd" ] || {
    echo "Blender's library pack wasn't found: run scripts/build-cycles.sh once, or set OPERANDI_LIBRARY_PACK"
    exit 1
}

ROOT="$HERE/development/usd"
BUILD="$ROOT/build"
INSTALL="$ROOT/install"
GENERATOR=""
command -v ninja >/dev/null 2>&1 && GENERATOR="-G Ninja"
# shellcheck disable=SC2086
cmake -S crates/operandi-usd/bridge -B "$BUILD" $GENERATOR \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$INSTALL" \
    -DLIBRARY_PACK="$PACK"
cmake --build "$BUILD" --config Release --target operandi_usd
rm -rf "$INSTALL"
cmake --install "$BUILD" --config Release

# OpenUSD and what it loads in turn (TBB, OpenSubdiv, MaterialX, OpenVDB,
# OpenImageIO...), next to the bridge so tests and app bundles find them in
# one folder.
pending="$INSTALL/lib/liboperandi_usd.dylib"
while [ -n "$pending" ]; do
    next=""
    for lib in $pending; do
        for dep in $(otool -L "$lib" | awk '/@rpath\//{sub("@rpath/","",$1); print $1}'); do
            [ -e "$INSTALL/lib/$dep" ] && continue
            found=$(find "$PACK" -name "$dep" -type f | head -1)
            [ -n "$found" ] || { echo "Missing $dep in the library pack"; exit 1; }
            cp "$found" "$INSTALL/lib/$dep"
            next="$next $INSTALL/lib/$dep"
        done
    done
    pending="$next"
done

# Local symbols only serve debuggers; dropping them roughly halves OpenUSD
# and OpenVDB (OpenVDB comes along only because Blender's OpenUSD links
# it: 40 symbols, a hard dependency). strip -x keeps every symbol the
# libraries link by. About 58 MB less in each app (2026-11-18).
strip -x "$INSTALL"/lib/*.dylib

# OpenUSD's plugin descriptions (file formats, schemas). The library also
# finds them on its own in usd/ next to libusd_ms.
cp -R "$PACK/usd/lib/usd" "$INSTALL/lib/usd"
echo "Installed $INSTALL/lib/liboperandi_usd.dylib"
