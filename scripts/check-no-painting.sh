#!/bin/sh
# Checks that a build without the `painting` feature carries no painting.
# Painting is on by default (2026-11-19); this checks the opt-out build
# (`--no-default-features`, or `OPERANDI_PAINTING=0 scripts/bundle-dev.sh`).
#
#   scripts/check-no-painting.sh            runs the no-painting test
#   scripts/check-no-painting.sh <binary>   also looks inside a built app
#                                           binary (or .app bundle)
#
# The test (crates/operandi-app/src/no_painting_tests.rs) finds no
# operandi.image.* / operandi.paint.* command and no painting menu entry.
# The binary check finds none of painting's command names in the file.
set -eu
cd "$(dirname "$0")/.."

cargo test --locked -p operandi-app --no-default-features -- no_painting_tests::

if [ $# -ge 1 ]; then
    BIN=$1
    if [ -d "$BIN" ]; then
        BIN=$(find "$BIN/Contents/MacOS" -type f -perm -u+x | head -n 1)
    fi
    for name in operandi.paint.addTexture operandi.image.new operandi.image.saveAll; do
        if grep -q -a "$name" "$BIN"; then
            echo "FAIL: $BIN contains painting command $name" >&2
            exit 1
        fi
    done
    echo "OK: $BIN has no painting commands"
fi
