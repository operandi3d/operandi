#!/bin/sh
# Builds a release of Operandi with the Cycles renderer on a Linux or
# Windows machine, and packages it:
#
#   Linux:   ~/Developer/Operandi-Builds/Operandi <branch> <commit> Linux.AppImage
#   Windows: ~/Developer/Operandi-Builds/Operandi <branch> <commit> Windows.zip
#            (run from Git Bash)
#
# macOS uses scripts/bundle-dev.sh instead. Machine setup is in
# docs/RELEASE_BUILDS.md. The first run downloads Cycles' library pack and
# builds Cycles (about an hour); later runs reuse both.
#
# OPERANDI_RELEASE_OUTPUT changes the output folder. OPERANDI_PAINTING=0
# leaves 3D painting out.
set -eu
cd "$(dirname "$0")/.."

case "$(uname)" in
    Linux) PLATFORM=Linux ;;
    MINGW* | MSYS* | CYGWIN*) PLATFORM=Windows ;;
    *) echo "This script builds Linux and Windows releases; use scripts/bundle-dev.sh on macOS."; exit 1 ;;
esac
if [ "$PLATFORM" = Windows ] && ! command -v cmake >/dev/null 2>&1; then
    # Visual Studio Build Tools ship CMake but don't put it on the PATH.
    VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    if [ -x "$VSWHERE" ]; then
        VS="$(cygpath -u "$("$VSWHERE" -latest -products '*' -property installationPath | tr -d '\r')")"
        PATH="$VS/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin:$PATH"
    fi
fi
for tool in git cargo cmake; do
    command -v "$tool" >/dev/null 2>&1 || { echo "$tool not found; see docs/RELEASE_BUILDS.md"; exit 1; }
done

COMMIT="$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo +)"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = HEAD ] && BRANCH=detached
BRANCH="$(printf '%s' "${BRANCH#claude/}" | sed 's#^codex/##; s#/#-#g; s#[^A-Za-z0-9._-]#-#g')"
NAME="Operandi $BRANCH $COMMIT"
OUTPUT="${OPERANDI_RELEASE_OUTPUT:-$HOME/Developer/Operandi-Builds}"
mkdir -p "$OUTPUT"
# OPERANDI_CYCLES_OUT and CARGO_TARGET_DIR let a second build (the
# older-Linux container, scripts/build-release-container.sh) sit beside the
# normal one.
INSTALL="${OPERANDI_CYCLES_OUT:-development/cycles}/install"
TARGET="${CARGO_TARGET_DIR:-target}"

echo "== Cycles"
scripts/build-cycles.sh

echo "== Operandi"
# Packaged builds keep settings in the user's own folders, not paths from
# this machine (app_environment.rs).
if [ -n "${OPERANDI_CYCLES_OUT:-}" ]; then
    OPERANDI_CYCLES_LIB_DIR="$PWD/$INSTALL/lib"
    export OPERANDI_CYCLES_LIB_DIR
fi
# Releases carry 3D painting (in the first release, Jason 2026-11-19);
# OPERANDI_PAINTING=0 leaves it out. USD is macOS only for now.
FEATURES=cycles
[ "${OPERANDI_PAINTING:-1}" = 1 ] && FEATURES="$FEATURES,painting"
echo "   features: $FEATURES"
OPERANDI_MILESTONE1_PACKAGE=1 cargo build --locked --release -p operandi-app --features "$FEATURES"

STAGE="development/release/$NAME"
rm -rf "development/release"
mkdir -p "$STAGE"
LICENSES="$STAGE/licenses"
[ "$PLATFORM" = Linux ] && LICENSES="$STAGE/usr/share/licenses/operandi"
mkdir -p "$LICENSES"
cp LICENSE docs/DEPENDENCIES.md "$LICENSES/"
cp -R docs/licenses/. "$LICENSES/"
cp -R "$INSTALL/share/cycles/license" "$LICENSES/cycles-build"

if [ "$PLATFORM" = Windows ]; then
    cp "$TARGET"/release/operandi-app.exe "$STAGE/Operandi.exe"
    # Cycles and the libraries it loads (gather-cycles-libs.py put them all
    # in bin/).
    cp "$INSTALL"/bin/*.dll "$STAGE/"
    # The Microsoft C++ runtime, so the PC doesn't need the Visual C++
    # Redistributable installed (Microsoft allows shipping these files).
    VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    VS="$("$VSWHERE" -latest -products '*' -property installationPath | tr -d '\r')"
    VS="$(cygpath -u "$VS")"
    CRT="$(ls -d "$VS"/VC/Redist/MSVC/*/x64/Microsoft.VC14*.CRT 2>/dev/null | sort | tail -1)"
    [ -n "$CRT" ] || { echo "Visual C++ runtime folder not found under $VS/VC/Redist"; exit 1; }
    cp "$CRT"/*.dll "$STAGE/"
    ZIP="$OUTPUT/$NAME Windows.zip"
    rm -f "$ZIP"
    # Windows' own tar writes a standard zip (Compress-Archive in Windows
    # PowerShell 5 writes backslash paths other systems misread).
    (cd development/release && /c/Windows/System32/tar.exe -a -c -f "$(cygpath -w "$ZIP")" "$NAME")
    rm -rf development/release
    printf '%s\n' "$ZIP"
    exit 0
fi

# Linux: an AppImage, one file that runs on most distributions with this
# machine's glibc version or newer.
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/lib"
cp "$TARGET"/release/operandi-app "$STAGE/usr/bin/operandi"
cp -P "$INSTALL"/lib/*.so* "$STAGE/usr/lib/"
cat > "$STAGE/AppRun" <<EOF
#!/bin/sh
HERE="\$(dirname "\$(readlink -f "\$0")")"
export OPERANDI_DEV_TITLE="\${OPERANDI_DEV_TITLE:-$NAME}"
exec "\$HERE/usr/bin/operandi" "\$@"
EOF
chmod +x "$STAGE/AppRun"
cat > "$STAGE/operandi.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Operandi
Comment=Polygon modeler
Exec=operandi
Icon=operandi
Categories=Graphics;3DGraphics;
Terminal=false
EOF
cp assets/branding/operandi-dock-icon.png "$STAGE/operandi.png"
ln -s operandi.png "$STAGE/.DirIcon"
# The AppImage runtime (the small program at the front of the file that
# mounts the rest), downloaded once.
RUNTIME=development/appimage/runtime-x86_64
if [ ! -s "$RUNTIME" ]; then
    mkdir -p development/appimage
    curl -fsSL -o "$RUNTIME" \
        https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64
fi
command -v mksquashfs >/dev/null 2>&1 || { echo "mksquashfs not found (install squashfs-tools)"; exit 1; }
mksquashfs "$STAGE" development/release/fs.sqfs -root-owned -noappend -comp zstd -no-xattrs -quiet
IMAGE="$OUTPUT/$NAME Linux.AppImage"
cat "$RUNTIME" development/release/fs.sqfs > "$IMAGE"
chmod +x "$IMAGE"
rm -rf development/release
printf '%s\n' "$IMAGE"
