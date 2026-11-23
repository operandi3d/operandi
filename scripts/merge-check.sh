#!/bin/sh
# The check before proposing a merge (testing plan item 1, Jason
# 2026-11-18): the full tier from AGENTS.md on this branch with main
# merged in, plus the Cycles and USD tests when their libraries are built.
# Those tests only compile with `--features cycles` / `usd`, so plain
# `cargo test` and CI never run them.
#
#   scripts/merge-check.sh
#
# Exits non-zero on the first failing step. It changes nothing in git: if
# main isn't merged into this branch yet, it says so and stops.
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-cargo}"

if ! git merge-base --is-ancestor main HEAD; then
    echo "merge-check: main has commits this branch lacks; merge main in first (git merge main)." >&2
    exit 1
fi
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "merge-check: note, uncommitted changes are included in this check."
fi

step() { echo; echo "== $*"; }

step "fmt"
"$CARGO_BIN" fmt --check
step "clippy"
"$CARGO_BIN" clippy --locked --workspace --all-targets -- -D warnings
step "tests"
scripts/test-all.sh

# The feature-gated tests, each only where its library is built.
if [ -d development/cycles/install/lib ]; then
    step "Cycles tests (--features cycles)"
    "$CARGO_BIN" test --locked --workspace --features operandi-app/cycles
else
    echo; echo "== Cycles tests skipped: no development/cycles (scripts/build-cycles.sh)"
fi
if [ -d development/usd/install/lib ]; then
    step "USD tests (--features usd)"
    "$CARGO_BIN" test --locked --workspace --features operandi-app/usd
else
    echo; echo "== USD tests skipped: no development/usd (scripts/build-usd.sh)"
fi
# Python scripting: the bundled CPython (scripts/fetch-python.sh), else a
# python3 on PATH for development.
PY_FOR_TESTS=""
if [ -x development/python/python/bin/python3 ]; then
    PY_FOR_TESTS="$(cd development/python/python/bin && pwd -P)/python3"
elif command -v python3 >/dev/null 2>&1; then
    PY_FOR_TESTS="$(command -v python3)"
fi
if [ -n "$PY_FOR_TESTS" ]; then
    step "Python tests (--features python, $PY_FOR_TESTS)"
    PYO3_PYTHON="$PY_FOR_TESTS" "$CARGO_BIN" test --locked --workspace --features operandi-app/python
else
    echo; echo "== Python tests skipped: no Python (scripts/fetch-python.sh)"
fi
echo; echo "merge-check: all passed."
