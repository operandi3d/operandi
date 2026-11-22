#!/bin/sh
# Fast tier: the checks to run while working on a small fix (AGENTS.md,
# "Checks"). The full tier (the three commands in AGENTS.md) still runs
# before a merge.
#
#   scripts/check-fast.sh            fmt, clippy and tests for what changed
#   scripts/check-fast.sh <filter>   ... running only tests whose name
#                                    contains <filter>
#
# "What changed" is everything on this branch since it left main, plus
# uncommitted files. Changes only in operandi-app check and test that crate;
# anything else (core, the other crates, Cargo files, vendor/) checks and
# tests the whole workspace, because most crates are built on core.
#
# It never builds a library crate on its own (`-p operandi-core` alone):
# Cargo then picks different dependency features than the workspace does
# and recompiles core and its dependencies a second time (~80 s).
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-cargo}"

"$CARGO_BIN" fmt --check

BASE=$(git merge-base HEAD main 2>/dev/null || git rev-parse HEAD)
CHANGED=$({
    git diff --name-only "$BASE"
    git ls-files --others --exclude-standard
} | sort -u)
CRATES=$(printf '%s\n' "$CHANGED" | sed -n 's#^crates/\([^/]*\)/.*#\1#p' | sort -u)
BUILD_FILES=$(printf '%s\n' "$CHANGED" | grep -E '^(Cargo\.(toml|lock)|rust-toolchain\.toml|\.cargo/|vendor/)' || true)

if [ -z "$CRATES" ] && [ -z "$BUILD_FILES" ]; then
    echo "check-fast: no Rust changes since main; fmt was the only check."
    exit 0
fi
if [ "$CRATES" = operandi-app ] && [ -z "$BUILD_FILES" ]; then
    SCOPE="-p operandi-app"
else
    SCOPE="--workspace"
fi
echo "check-fast: changed crates: $(printf '%s ' $CRATES)-> $SCOPE"

# Clippy does everything `cargo check` does, so there's no separate check.
# shellcheck disable=SC2086
"$CARGO_BIN" clippy --locked $SCOPE --all-targets -- -D warnings
if [ $# -ge 1 ]; then
    # shellcheck disable=SC2086
    "$CARGO_BIN" test --locked $SCOPE -- "$1"
else
    # shellcheck disable=SC2086
    "$CARGO_BIN" test --locked $SCOPE
fi
