#!/bin/sh
# Builds liboperandi_cycles (Cycles + Operandi's C bridge) into
# development/cycles/install/lib, ready for `--features cycles`.
#
#   scripts/build-cycles.sh            # Blender's precompiled libraries (macOS, Windows, Linux x64)
#   scripts/build-cycles.sh --system   # libraries already installed on this machine (Homebrew, apt...)
#
# Cycles is pinned to one commit so every machine builds the same renderer,
# plus the patches in crates/operandi-cycles/cycles/patches.
# Enabled: CPU, Metal on macOS, OpenImageDenoise. Switched off for now:
# CUDA/OptiX/HIP/oneAPI, OSL, OpenVDB, OpenSubdiv, USD and Alembic.
set -eu
cd "$(dirname "$0")/.."
CYCLES_URL="${CYCLES_URL:-https://projects.blender.org/blender/cycles.git}"
CYCLES_COMMIT=3eb35f85996baa6c1b2e19bf297b22745c85e321 # main, 2026-10-26 (Cycles 5.3.0 alpha)
# Git Bash on Windows: CMake needs C:/... paths, not /c/...
HERE="$PWD"
command -v cygpath >/dev/null 2>&1 && HERE="$(cygpath -m "$PWD")"
ROOT="$HERE/development/cycles"
# python.org's Windows installer adds `python`, not `python3`, and Windows'
# own `python3` may be a Microsoft Store shortcut that doesn't run Python.
PYTHON=python3
"$PYTHON" -c "" >/dev/null 2>&1 || PYTHON=python
SRC="$ROOT/src"
# OPERANDI_CYCLES_OUT (a folder under the repo) keeps a second build apart
# from the normal one, e.g. the older-Linux container build; the Cycles
# download in development/cycles/src is shared.
OUT="$ROOT"
[ -n "${OPERANDI_CYCLES_OUT:-}" ] && OUT="$HERE/$OPERANDI_CYCLES_OUT"
BUILD="$OUT/build"
INSTALL="$OUT/install"
SYSTEM_LIBS=OFF
[ "${1:-}" = "--system" ] && SYSTEM_LIBS=ON

mkdir -p "$ROOT"
if [ ! -d "$SRC/.git" ]; then
    git clone --filter=blob:none "$CYCLES_URL" "$SRC"
fi
git -C "$SRC" fetch --quiet origin "$CYCLES_COMMIT" 2>/dev/null || true
git -C "$SRC" -c advice.detachedHead=false checkout --quiet "$CYCLES_COMMIT"
# Operandi's few changes to Cycles, applied once on top of the pinned commit
# (skipped when already there). Each patch says what it is for.
for patch in crates/operandi-cycles/cycles/patches/*.patch; do
    [ -e "$patch" ] || continue
    if ! (cd "$SRC" && git apply --reverse --check -p1 "$HERE/$patch") 2>/dev/null; then
        (cd "$SRC" && git apply -p1 "$HERE/$patch")
    fi
done

if [ "$SYSTEM_LIBS" = OFF ]; then
    # Downloads only this platform's library pack (large: several GB, once).
    git lfs version >/dev/null 2>&1 || { echo "git-lfs is required (brew install git-lfs)"; exit 1; }
    (cd "$SRC" && "$PYTHON" src/cmake/make_update.py --no-cycles)
fi

GENERATOR=""
command -v ninja >/dev/null 2>&1 && GENERATOR="-G Ninja"
# On Windows, Ninja only finds the compiler inside a Visual Studio developer
# prompt; otherwise CMake's default Visual Studio generator finds it itself.
case "$(uname)" in
    MINGW* | MSYS* | CYGWIN*) [ -n "${VCINSTALLDIR:-}" ] || GENERATOR="" ;;
esac
# shellcheck disable=SC2086
cmake -S crates/operandi-cycles/cycles -B "$BUILD" $GENERATOR \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$INSTALL" \
    -DCYCLES_SOURCE_DIR="$SRC" \
    -DWITH_LIBS_PRECOMPILED=$([ "$SYSTEM_LIBS" = ON ] && echo OFF || echo ON) \
    -DWITH_CYCLES_ALEMBIC=OFF \
    -DWITH_CYCLES_OPENIMAGEDENOISE=ON \
    -DWITH_CYCLES_OPENSUBDIV=OFF \
    -DWITH_CYCLES_OPENVDB=OFF \
    -DWITH_CYCLES_NANOVDB=OFF \
    -DWITH_CYCLES_OSL=OFF \
    -DWITH_CYCLES_USD=OFF \
    -DWITH_CYCLES_HYDRA_RENDER_DELEGATE=OFF \
    -DWITH_CYCLES_PATH_GUIDING=OFF \
    -DWITH_CYCLES_DEVICE_CUDA=OFF \
    -DWITH_CYCLES_DEVICE_OPTIX=OFF \
    -DWITH_CYCLES_DEVICE_HIP=OFF \
    -DWITH_CYCLES_DEVICE_METAL=$([ "$(uname)" = Darwin ] && echo ON || echo OFF) \
    -DWITH_CYCLES_DEVICE_ONEAPI=OFF \
    -DWITH_CYCLES_EMBREE=ON \
    ${OPERANDI_CYCLES_CMAKE_ARGS:-}
cmake --build "$BUILD" --config Release --target operandi_cycles
cmake --install "$BUILD" --config Release

# The precompiled pack ships some libraries (Embree, TBB, OpenImageIO,
# OpenColorIO, OpenEXR...) as shared libraries the bridge loads through
# @rpath. Copy them, and what they load in turn, next to the bridge so tests,
# examples and app bundles find them in one folder. Linux and Windows use
# scripts/gather-cycles-libs.py below.
if [ "$SYSTEM_LIBS" = OFF ] && [ "$(uname)" = Darwin ]; then
    pending="$INSTALL/lib/liboperandi_cycles.dylib"
    while [ -n "$pending" ]; do
        next=""
        for lib in $pending; do
            for dep in $(otool -L "$lib" | awk '/@rpath\//{sub("@rpath/","",$1); print $1}'); do
                [ -e "$INSTALL/lib/$dep" ] && continue
                found=$(find "$SRC/lib/macos_arm64" -name "$dep" -type f | head -1)
                [ -n "$found" ] || { echo "Missing $dep in the library pack"; exit 1; }
                cp "$found" "$INSTALL/lib/$dep"
                next="$next $INSTALL/lib/$dep"
            done
        done
        pending="$next"
    done
    # OpenImageDenoise loads its GPU back ends (Metal) at run time, so
    # otool doesn't list them; copy them too.
    find "$SRC/lib/macos_arm64" -name 'libOpenImageDenoise_device_*.dylib' -type f \
        -exec cp {} "$INSTALL/lib/" \;
fi

if [ "$SYSTEM_LIBS" = OFF ]; then
    case "$(uname)" in
        Linux)
            "$PYTHON" scripts/gather-cycles-libs.py "$INSTALL/lib/liboperandi_cycles.so" \
                "$SRC/lib/linux_x64" "$INSTALL/lib" ;;
        MINGW* | MSYS* | CYGWIN*)
            # Windows puts DLLs in bin/ (lib/ holds the import library).
            "$PYTHON" scripts/gather-cycles-libs.py "$INSTALL/bin/operandi_cycles.dll" \
                "$SRC/lib/windows_x64" "$INSTALL/bin" ;;
    esac
fi

# GPU devices compile Cycles' kernels from source on first use, so ship the
# kernel and util headers (about 4 MB), plus Cycles' licence texts.
RES="$INSTALL/share/cycles"
rm -rf "$RES/source"
for dir in kernel util; do
    (cd "$SRC/src" && find "$dir" \( -name '*.h' -o -name '*.metal' \) -print) | while read -r f; do
        mkdir -p "$RES/source/$(dirname "$f")"
        cp "$SRC/src/$f" "$RES/source/$f"
    done
done
mkdir -p "$RES/license"
cp "$SRC"/src/doc/license/*-license.txt "$SRC"/src/doc/license/readme.txt "$RES/license/"
echo "Installed $(ls "$INSTALL"/lib/*operandi_cycles* | head -1)"
