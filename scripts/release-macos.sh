#!/bin/sh
# Builds a signed, notarized macOS release of Operandi with the Cycles
# renderer, packed in a disk image:
#
#   ~/Developer/Operandi-Builds/Operandi <version> <commit>.dmg
#
# Steps: build (release, Cycles on, packaged paths), make the .app, sign
# every library and then the app with the Developer ID certificate, send the
# app to Apple for notarization and staple the ticket to it, pack the .dmg,
# sign it, notarize and staple the .dmg too, then ask Gatekeeper to check
# both. docs/RELEASING.md explains it all in plain words.
#
# Settings (environment variables override the defaults below):
#   OPERANDI_BUNDLE_ID      the app's bundle identifier
#   OPERANDI_SIGN_IDENTITY  the signing certificate's name in the keychain
#   OPERANDI_NOTARY_PROFILE the notarytool keychain profile
#   OPERANDI_CYCLES_LIB_DIR a finished Cycles build's lib folder to use
#                           instead of running scripts/build-cycles.sh
#   OPERANDI_RELEASE_OUTPUT output folder
#   OPERANDI_SKIP_NOTARIZE=1 sign and pack only (for a quick local test)
#   OPERANDI_PAINTING=0     leave 3D painting out (it's in releases since
#                           Jason's call of 2026-11-19)
#   OPERANDI_USD=0          leave USD import/export out
set -eu
cd "$(dirname "$0")/.."

# From the project's domain, operandi3d.org (Jason, 2026-11-08). Never
# change it: macOS would treat the app as a different program (preferences
# and permissions start fresh).
BUNDLE_ID="${OPERANDI_BUNDLE_ID:-org.operandi3d.Operandi}"
SIGN_IDENTITY="${OPERANDI_SIGN_IDENTITY:-Developer ID Application: Jason Morris (PNA7R84ZP6)}"
NOTARY_PROFILE="${OPERANDI_NOTARY_PROFILE:-operandi}"
OUTPUT="${OPERANDI_RELEASE_OUTPUT:-$HOME/Developer/Operandi-Builds}"
CARGO_BIN="${CARGO_BIN:-$HOME/.cargo/bin/cargo}"
# notarytool and stapler come with Xcode; xcode-select may point at the
# Command Line Tools instead, so use Xcode.app directly when it's there.
if [ -z "${DEVELOPER_DIR:-}" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
    DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
    export DEVELOPER_DIR
fi

[ "$(uname)" = Darwin ] || { echo "This script builds macOS releases; see scripts/build-release.sh for Linux and Windows."; exit 1; }
security find-identity -v -p codesigning | grep -qF "\"$SIGN_IDENTITY\"" || {
    echo "Signing certificate \"$SIGN_IDENTITY\" not found in the keychain; see docs/RELEASING.md."
    exit 1
}

# The version is the app crate's own (crates/operandi-app/Cargo.toml).
VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' crates/operandi-app/Cargo.toml | head -1)"
[ -n "$VERSION" ] || { echo "No version in crates/operandi-app/Cargo.toml"; exit 1; }
COMMIT="$(git rev-parse --short HEAD)"
if ! git diff --quiet HEAD; then
    echo "warning: uncommitted changes; the name gets a + and this isn't a real release" >&2
    COMMIT="$COMMIT+"
fi
# Build number: commits so far, so every release counts up.
BUILD_NUMBER="$(git rev-list --count HEAD)"
NAME="Operandi $VERSION $COMMIT"

echo "== Cycles"
if [ -n "${OPERANDI_CYCLES_LIB_DIR:-}" ]; then
    CYCLES_INSTALL="$(cd "$OPERANDI_CYCLES_LIB_DIR/.." && pwd)"
    export OPERANDI_CYCLES_LIB_DIR
else
    scripts/build-cycles.sh
    CYCLES_INSTALL="$PWD/development/cycles/install"
fi

# Releases carry Cycles, 3D painting (in the first release, Jason
# 2026-11-19) and USD (macOS only for now), like the dev build
# `OPERANDI_CYCLES=1 OPERANDI_PAINTING=1 scripts/bundle-dev.sh`.
FEATURES=cycles
[ "${OPERANDI_PAINTING:-1}" = 1 ] && FEATURES="$FEATURES,painting"
USD_INSTALL=""
if [ "${OPERANDI_USD:-1}" = 1 ]; then
    FEATURES="$FEATURES,usd"
    [ -f development/usd/install/lib/liboperandi_usd.dylib ] || scripts/build-usd.sh
    USD_INSTALL=development/usd/install
fi

echo "== Operandi $VERSION ($COMMIT), features $FEATURES"
# OPERANDI_MILESTONE1_PACKAGE=1: settings live in the user's own folders,
# not in paths from this machine, and the MCP bridge is off by default
# (app_environment.rs, mcp.rs).
OPERANDI_MILESTONE1_PACKAGE=1 "$CARGO_BIN" build --locked --release -p operandi-app --features "$FEATURES"

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/operandi-release.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
APP="$STAGE/dmg/Operandi.app"
mkdir -p "$STAGE/dmg"
. scripts/macos-app-bundle.sh
USD_INSTALL="$USD_INSTALL" assemble_app "$APP" Operandi "$BUNDLE_ID" "$VERSION" "$BUILD_NUMBER" "$STAGE" "$CYCLES_INSTALL"
# Packaged builds title their window from OPERANDI_DEV_TITLE.
/usr/libexec/PlistBuddy -c 'Add :LSEnvironment dict' "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:OPERANDI_DEV_TITLE string Operandi" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :NSHumanReadableCopyright string Operandi is free software under the GNU GPL v3." "$APP/Contents/Info.plist"

# The linker also left an rpath to this machine's Cycles build folder; a
# release must only look inside its own Frameworks folder.
EXE="$APP/Contents/MacOS/operandi-app"
otool -l "$EXE" | awk '/cmd LC_RPATH/ {getline; getline; print $2}' | while read -r rpath; do
    case "$rpath" in
        @*) ;;
        *) install_name_tool -delete_rpath "$rpath" "$EXE" ;;
    esac
done

echo "== Signing"
# Inside out: every library and nested program first, then the app. Apple's
# notary service rejects anything without a secure timestamp or the
# hardened runtime (--options runtime).
sign() {
    codesign --force --timestamp --options runtime --sign "$SIGN_IDENTITY" "$@"
}
find "$APP/Contents" -type f \( -name '*.dylib' -o -name '*.so' -o -perm +111 \) \
    ! -path "$EXE" -print | while read -r f; do
    # Only Mach-O files need signatures (skip scripts and data that happen to
    # be marked executable).
    if file -b "$f" | grep -q 'Mach-O'; then
        sign "$f"
    fi
done
sign "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

notarize() {
    [ "${OPERANDI_SKIP_NOTARIZE:-0}" = 1 ] && { echo "(notarization skipped)"; return 0; }
    log="$STAGE/notary-$(basename "$1").json"
    xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" --wait \
        --output-format json >"$log" || true
    cat "$log"
    id="$(sed -n 's/.*"id" *: *"\([^"]*\)".*/\1/p' "$log" | head -1)"
    if ! grep -q '"status" *: *"Accepted"' "$log"; then
        echo "Notarization of $(basename "$1") failed. Apple's log:" >&2
        [ -n "$id" ] && xcrun notarytool log "$id" --keychain-profile "$NOTARY_PROFILE" >&2
        exit 1
    fi
    echo "Notarized $(basename "$1"): submission $id"
}

echo "== Notarizing the app"
# notarytool takes a zip, dmg or pkg, not a bare .app.
ditto -c -k --keepParent "$APP" "$STAGE/Operandi.zip"
notarize "$STAGE/Operandi.zip"
# Stapling attaches Apple's ticket to the app itself, so it opens without
# asking Apple first even after it's copied out of the disk image.
[ "${OPERANDI_SKIP_NOTARIZE:-0}" = 1 ] || xcrun stapler staple "$APP"

echo "== Disk image"
ln -s /Applications "$STAGE/dmg/Applications"
# Made, stapled and checked in the temporary folder, then copied out:
# stapler can't validate a ticket it writes on some external drives
# (error 73, "the written data did not validate").
DMG_NAME="$NAME.dmg"
STAGED_DMG="$STAGE/$DMG_NAME"
hdiutil create -volname "Operandi $VERSION" -srcfolder "$STAGE/dmg" -fs APFS -format UDZO -ov "$STAGED_DMG" >/dev/null
codesign --force --timestamp --sign "$SIGN_IDENTITY" "$STAGED_DMG"
echo "== Notarizing the disk image"
notarize "$STAGED_DMG"
[ "${OPERANDI_SKIP_NOTARIZE:-0}" = 1 ] || xcrun stapler staple "$STAGED_DMG"

echo "== Gatekeeper check"
if [ "${OPERANDI_SKIP_NOTARIZE:-0}" != 1 ]; then
    spctl -a -vv -t install "$STAGED_DMG"
    MOUNT="$STAGE/mount"
    mkdir -p "$MOUNT"
    hdiutil attach -nobrowse -readonly -mountpoint "$MOUNT" "$STAGED_DMG" >/dev/null
    spctl -a -vv "$MOUNT/Operandi.app" || { hdiutil detach "$MOUNT" >/dev/null; exit 1; }
    xcrun stapler validate "$MOUNT/Operandi.app"
    hdiutil detach "$MOUNT" >/dev/null
fi
mkdir -p "$OUTPUT"
DMG="$OUTPUT/$DMG_NAME"
rm -f "$DMG"
cp "$STAGED_DMG" "$DMG"
printf '%s\n' "$DMG"
