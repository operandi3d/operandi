#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-$HOME/.cargo/bin/cargo}"
# QA apps are always optimized: debug builds draw large meshes many times
# slower, which reads as a bug when someone is testing a tool. They use the
# `qa` profile (Cargo.toml): optimized like release but incremental, so a
# rebuild after a small app change takes seconds instead of about a minute.
# OPERANDI_DEV_PROFILE=release builds exactly what a release ships; pass
# OPERANDI_DEV_PROFILE=debug only on purpose. Either way other than `qa`,
# the app name says which.
PROFILE="${OPERANDI_DEV_PROFILE:-qa}"
# OPERANDI_CYCLES=1 adds the Cycles renderer (File > Render Viewport);
# run scripts/build-cycles.sh once first.
FEATURES=""
[ "${OPERANDI_CYCLES:-0}" = 1 ] && FEATURES="cycles"
# USD import and export (File > Open/Import a .usd*, File > Export ▸ USD)
# are on by default on macOS (docs/BEHAVIOR-USD-IMPORT.md, U0); they need
# scripts/build-usd.sh's development/usd/install, which this builds once
# when it's missing. OPERANDI_USD=0 leaves USD out.
[ "$(uname)" = Darwin ] && : "${OPERANDI_USD:=1}"
if [ "${OPERANDI_USD:-0}" = 1 ]; then
    FEATURES="${FEATURES:+$FEATURES,}usd"
    [ -f development/usd/install/lib/liboperandi_usd.dylib ] || scripts/build-usd.sh
fi
# 3D painting is on by default (the maintainer, 2026-11-19);
# OPERANDI_PAINTING=0 builds without it.
NO_DEFAULT=""
[ "${OPERANDI_PAINTING:-1}" = 0 ] && NO_DEFAULT="--no-default-features"
[ -n "$FEATURES" ] && FEATURES="--features $FEATURES"
CARGO_PROFILE="$PROFILE"
[ "$PROFILE" = debug ] && CARGO_PROFILE=dev
# shellcheck disable=SC2086
"$CARGO_BIN" build --locked --profile "$CARGO_PROFILE" -p operandi-app $NO_DEFAULT $FEATURES
STAGE=$(mktemp -d /tmp/operandi-rust-bundle.XXXXXX)
trap 'rm -rf "$STAGE"' EXIT
# Every QA build from every branch lands in one folder,
# ~/Developer/Operandi-Builds, as "Operandi <branch> <commit>.app" (or
# "<OPERANDI_DEV_NAME> <commit>.app"). A new build replaces the older builds
# with the same name, so there is one app per branch. Each name gets its own
# bundle id so two branches can run side by side.
COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)$(git diff --quiet HEAD 2>/dev/null || echo +)"
BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
[ "$BRANCH" = HEAD ] && BRANCH=detached
BRANCH="$(printf '%s' "${BRANCH#claude/}" | sed 's#^codex/##; s#/#-#g; s#[^A-Za-z0-9._-]#-#g')"
DEV_BUNDLE_NAME="${OPERANDI_DEV_NAME:-Operandi $BRANCH}"
[ "$PROFILE" = qa ] || DEV_BUNDLE_NAME="$DEV_BUNDLE_NAME $PROFILE"
ID_SUFFIX="$(printf '%s' "$DEV_BUNDLE_NAME" | tr 'A-Z' 'a-z' | sed 's#[^a-z0-9]\{1,\}#-#g; s#^-##; s#-$##')"
DEV_BUNDLE_ID="${OPERANDI_DEV_ID:-org.operandi.dev.$ID_SUFFIX}"
APP_NAME="$DEV_BUNDLE_NAME $COMMIT"
STAGED_BUNDLE="$STAGE/$APP_NAME.app"
OUTPUT_ROOT="${OPERANDI_DEV_OUTPUT:-$HOME/Developer/Operandi-Builds}"
BUNDLE="$OUTPUT_ROOT/$APP_NAME.app"
mkdir -p "$OUTPUT_ROOT"
# Shared with scripts/release-macos.sh: binary, Info.plist, icons, Cycles
# libraries and licence texts.
. scripts/macos-app-bundle.sh
CYCLES_INSTALL=""
[ "${OPERANDI_CYCLES:-0}" = 1 ] && CYCLES_INSTALL=development/cycles/install
USD_INSTALL=""
[ "${OPERANDI_USD:-0}" = 1 ] && USD_INSTALL=development/usd/install
PROFILE="$PROFILE" USD_INSTALL="$USD_INSTALL" assemble_app "$STAGED_BUNDLE" "$DEV_BUNDLE_NAME" "$DEV_BUNDLE_ID" 0.1.1 1 "$STAGE" $CYCLES_INSTALL
PLIST="$STAGED_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :LSEnvironment dict' "$PLIST"
# The window title names the build (commit and time) so QA can tell
# rebuilds of the same app apart.
BUILD_LABEL="$COMMIT $(date '+%b %d %H:%M')"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:OPERANDI_DEV_TITLE string $DEV_BUNDLE_NAME · build $BUILD_LABEL" "$PLIST"
if [ -n "${OPERANDI_DEV_DATA:-}" ]; then
    /usr/libexec/PlistBuddy -c "Add :LSEnvironment:OPERANDI_DEV_DATA string $OPERANDI_DEV_DATA" "$PLIST"
fi
if [ -n "$FEATURES" ]; then
    for lib in "$STAGED_BUNDLE"/Contents/Frameworks/*.dylib; do
        codesign --force --sign - "$lib"
    done
fi
codesign --force --sign - "$STAGED_BUNDLE"
codesign --verify --deep --strict "$STAGED_BUNDLE"
# Only replace this script's own generated bundles, never an installed app:
# the same name followed by a single commit word (hex, maybe "+").
for old in "$OUTPUT_ROOT/$DEV_BUNDLE_NAME "*.app; do
    [ -e "$old" ] || continue
    rest="${old#"$OUTPUT_ROOT/$DEV_BUNDLE_NAME "}"
    case "$rest" in
        *" "*) continue ;;
        [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]*.app|unknown*.app) rm -rf "$old" ;;
    esac
done
rm -rf "$BUNDLE"
ditto --norsrc --noextattr "$STAGED_BUNDLE" "$BUNDLE"
xattr -dr com.apple.FinderInfo "$BUNDLE" 2>/dev/null || true
xattr -dr com.apple.ResourceFork "$BUNDLE" 2>/dev/null || true
codesign --verify --deep --strict "$BUNDLE"
# A stable link in this checkout's development/ folder points at the newest
# build. Skip it when the output folder IS development/: the link would
# replace the app it points to (the old self-pointing symlink trap).
mkdir -p development
LINK="development/$DEV_BUNDLE_NAME.app"
if [ "$(cd "$OUTPUT_ROOT" && pwd -P)" != "$(cd development && pwd -P)" ]; then
    if [ -L "$LINK" ] || [ ! -e "$LINK" ]; then
        rm -f "$LINK"
        ln -s "$BUNDLE" "$LINK"
    fi
fi
printf '%s\n' "$BUNDLE"
