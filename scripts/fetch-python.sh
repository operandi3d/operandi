#!/bin/sh
# Fetches the CPython that Python scripting embeds and the app bundle
# carries (docs/DEPENDENCIES.md, "Python scripting"): a relocatable
# "install_only" build from python-build-standalone, pinned below, checked
# against the release's SHA256SUMS, unpacked into development/python/python
# (git-ignored).
#
# Afterwards build with it:
#   PYO3_PYTHON=development/python/python/bin/python3 cargo build -p operandi-app --features python
# scripts/bundle-dev.sh does that by itself once the folder exists.
#
# The library's install name is set to its absolute path here, so
# development builds and tests find it where it is; the app bundle copies
# it into Contents/Frameworks and points the binary there
# (scripts/macos-app-bundle.sh).
set -eu
cd "$(dirname "$0")/.."
# The pinned release: python-build-standalone tag and CPython version.
PBS_TAG="${OPERANDI_PBS_TAG:-20261111}"
PY_VERSION="${OPERANDI_PY_VERSION:-3.13.9}"
case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) TRIPLE=aarch64-apple-darwin ;;
    Darwin-x86_64) TRIPLE=x86_64-apple-darwin ;;
    Linux-x86_64) TRIPLE=x86_64-unknown-linux-gnu ;;
    Linux-aarch64) TRIPLE=aarch64-unknown-linux-gnu ;;
    *) echo "fetch-python: no pinned build for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
NAME="cpython-$PY_VERSION+$PBS_TAG-$TRIPLE-install_only.tar.gz"
BASE="https://github.com/astral-sh/python-build-standalone/releases/download/$PBS_TAG"
DEST=development/python
mkdir -p "$DEST"
if [ ! -f "$DEST/$NAME" ]; then
    curl --fail --location --proto '=https' -o "$DEST/$NAME.part" "$BASE/$NAME"
    mv "$DEST/$NAME.part" "$DEST/$NAME"
fi
curl --fail --location --proto '=https' -o "$DEST/SHA256SUMS" "$BASE/SHA256SUMS"
WANT=$(grep " $NAME\$" "$DEST/SHA256SUMS" | cut -d' ' -f1)
GOT=$(shasum -a 256 "$DEST/$NAME" | cut -d' ' -f1)
if [ -z "$WANT" ] || [ "$WANT" != "$GOT" ]; then
    echo "fetch-python: checksum mismatch for $NAME (want ${WANT:-none}, got $GOT)" >&2
    exit 1
fi
rm -rf "$DEST/python"
tar -xzf "$DEST/$NAME" -C "$DEST"
printf '%s\n%s\n' "$NAME" "$WANT" > "$DEST/RELEASE"
PY="$(cd "$DEST/python" && pwd -P)"
MINOR=$("$PY/bin/python3" -c 'import sys; print(f"{sys.version_info[0]}.{sys.version_info[1]}")')
if [ "$(uname -s)" = Darwin ]; then
    LIB="$PY/lib/libpython$MINOR.dylib"
    install_name_tool -id "$LIB" "$LIB"
    codesign --force --sign - "$LIB"
fi
echo "Python $("$PY/bin/python3" -c 'import platform; print(platform.python_version())') in $PY"
