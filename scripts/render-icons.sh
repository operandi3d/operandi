#!/bin/sh
# Render the icon studio's icons with Cycles (tools/icon-studio/README.md).
#
#   scripts/render-icons.sh                 every icon in tools/icon-studio/icons
#   scripts/render-icons.sh <icon.opscene>  just these
#
# Each icon is rendered straight at 16, 20, 24 and 32 px and at 2× each,
# into development/icon-studio/<slug>/, with a contact sheet at
# development/icon-studio/contact-sheet.png. Extra options go to the
# renderer: --samples <n>, --no-outline, --out <folder>.
#
#   scripts/render-icons.sh --install
#
# also copies every render into
# crates/operandi-app/assets/rendered-icons/<set>/<slug>-<pixels>.png. The
# app draws only the ones listed in RENDERED and FILES
# (crates/operandi-app/src/rendered_icons.rs): install first, then add an
# icon's lines there (the build needs the files to exist).
#
# Needs the Cycles libraries (scripts/build-cycles.sh once).
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-cargo}"
STUDIO=tools/icon-studio/studio.opscene
OUT=development/icon-studio

INPUTS=""
OPTIONS=""
INSTALL=0
while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT=$2; shift 2 ;;
        --samples) OPTIONS="$OPTIONS --samples $2"; shift 2 ;;
        --install) INSTALL=1; shift ;;
        --*) OPTIONS="$OPTIONS $1"; shift ;;
        *) INPUTS="$INPUTS $1"; shift ;;
    esac
done
[ -n "$INPUTS" ] || INPUTS=tools/icon-studio/icons

# The same optimized, incremental profile as QA builds (bundle-dev.sh).
"$CARGO_BIN" build --locked --profile qa -p operandi-app --features cycles
BIN=target/qa/operandi-app
rm -rf "$OUT"
# shellcheck disable=SC2086
"$BIN" --icon-studio render $INPUTS --studio "$STUDIO" --out "$OUT" $OPTIONS

[ "$INSTALL" = 1 ] || exit 0
ASSETS=crates/operandi-app/assets/rendered-icons
SLUGS=$(for d in "$OUT"/*/; do basename "$d"; done)
for set in dark-outline light-outline; do
    mkdir -p "$ASSETS/$set"
    for slug in $SLUGS; do
        from="$OUT/$slug/$set"
        cp "$from/$slug-16.png" "$ASSETS/$set/$slug-16.png"
        cp "$from/$slug-20.png" "$ASSETS/$set/$slug-20.png"
        cp "$from/$slug-24.png" "$ASSETS/$set/$slug-24.png"
        cp "$from/$slug-32.png" "$ASSETS/$set/$slug-32.png"
        cp "$from/$slug-20@2x.png" "$ASSETS/$set/$slug-40.png"
        cp "$from/$slug-24@2x.png" "$ASSETS/$set/$slug-48.png"
        cp "$from/$slug-32@2x.png" "$ASSETS/$set/$slug-64.png"
        cp "$from/$slug-32@4x.png" "$ASSETS/$set/$slug-128.png"
    done
done
echo "Installed $(echo $SLUGS | wc -w | tr -d ' ') rendered icons into $ASSETS"
