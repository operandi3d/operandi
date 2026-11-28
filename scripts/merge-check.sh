#!/bin/sh
# The check before proposing a merge (testing plan item 1, Jason
# 2026-11-18): the full tier from AGENTS.md on this branch with main
# merged in, plus the Cycles, USD and Python extras where they're built.
# Their code only compiles with `--features cycles` / `usd` / `python`, so
# plain `cargo test` and CI never build it. Each extra is compiled on its
# own, then the tests run twice: Cycles and USD on, then Python on
# (Jason, 2026-11-26).
#
#   scripts/merge-check.sh
#
# Exits non-zero on the first failing step. It changes nothing in this
# checkout or its branch: if main isn't merged in yet, it says so and stops.
#
# Where it builds (merge-check cache, 2026-11-26). Every branch is checked
# in one shared merge checkout, ../Operandi-merge-check next to the main
# checkout, with its own warm build folder, instead of in the branch's
# worktree. Each run checks out this branch's commit there (plus any
# uncommitted changes to tracked files) and runs the steps in it. Git
# rewrites only the files that differ from the last branch checked, so
# Cargo rebuilds only what those files touch, and the Cycles/USD/Python
# builds are warm too. A fresh worktree's first merge check otherwise
# rebuilds every Operandi crate, its C++ and every feature set.
# - Correct: the merge checkout's files are exactly this commit's, and a
#   file whose content changed gets a new time, so Cargo's usual checks
#   (file times) can't reuse another branch's results for it.
# - One run at a time: a lock in its .git folder; a second run waits.
# - Files that git doesn't track (new files you haven't `git add`ed) are
#   not part of the check, as they wouldn't be part of the merge.
# - OPERANDI_MERGE_CHECK_DIR=<folder> uses another merge checkout;
#   OPERANDI_MERGE_CHECK_DIR=here checks in this checkout, as before.
# - The build folder is deleted and starts over when it grows past
#   OPERANDI_MERGE_CHECK_MAX_GB (default 150).
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-cargo}"
SRC=$(pwd -P)

if [ -z "${OPERANDI_MERGE_CHECK_LOCKED:-}" ]; then
    if ! git merge-base --is-ancestor main HEAD; then
        echo "merge-check: main has commits this branch lacks; merge main in first (git merge main)." >&2
        exit 1
    fi
    if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
        echo "merge-check: note, uncommitted changes to tracked files are included in this check."
    fi
fi

# The spec-backed tests (oracles, cases) build only when the private specs
# checkout is found (crates/operandi-core/build_specs.rs). A merge must run
# them, so a missing checkout stops here instead of skipping them quietly.
SPECS="${OPERANDI_SPECS:-../Operandi-specs}"
if [ ! -d "$SPECS/specifications" ]; then
    echo "merge-check: no private specs at $SPECS; clone them there or set OPERANDI_SPECS." >&2
    exit 1
fi

# The main checkout is the first entry `git worktree list` prints.
MAIN=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
MC="${OPERANDI_MERGE_CHECK_DIR:-$(dirname "$MAIN")/Operandi-merge-check}"

if [ "$MC" != here ] && [ "$SRC" != "$(cd "$MC" 2>/dev/null && pwd -P)" ]; then
    if [ -z "${OPERANDI_MERGE_CHECK_LOCKED:-}" ]; then
        if [ ! -d "$MC/.git" ]; then
            # Its own small git folder, borrowing the main repository's
            # objects (every worktree's commits are there), so checking out
            # a commit never touches the main repository.
            echo "merge-check: making the merge checkout $MC (once)."
            git clone -q --shared --no-checkout "$MAIN" "$MC"
        fi
        # Hold the lock for the rest of the run (it's dropped when this
        # process ends, however it ends). perl's flock is on macOS and Linux.
        OPERANDI_MERGE_CHECK_LOCKED=1 exec perl -MFcntl=:flock -e '
            $^F = 255; # keep the lock open across exec
            open(my $lock, ">>", shift) or die "merge-check: lock: $!\n";
            unless (flock($lock, LOCK_EX | LOCK_NB)) {
                print "merge-check: another merge check is running; waiting for it.\n";
                flock($lock, LOCK_EX) or die "merge-check: lock: $!\n";
            }
            exec @ARGV or die "merge-check: $!\n";
        ' "$MC/.git/merge-check.lock" sh "$0"
    fi

    COMMIT=$(git rev-parse HEAD)
    PATCH=$(mktemp)
    trap 'rm -f "$PATCH"' EXIT
    git diff --binary HEAD >"$PATCH"
    echo "merge-check: checking $(git rev-parse --abbrev-ref HEAD) ($(git rev-parse --short HEAD)) in $MC"
    # -f also undoes the last run's uncommitted changes; clean removes
    # untracked files left from other branches (ignored ones, like the
    # build output and development/, stay).
    if ! git -C "$MC" checkout -q -f --detach "$COMMIT" 2>/dev/null; then
        # The commit it had checked out may have been pruned since. Start
        # its files over (the build folder stays; it's keyed by the path).
        echo "merge-check: refreshing the merge checkout's files."
        find "$MC" -mindepth 1 -maxdepth 1 ! -name .git ! -name target ! -name development -exec rm -rf {} +
        rm -f "$MC/.git/index"
        git -C "$MC" update-ref --no-deref HEAD "$COMMIT"
        git -C "$MC" reset -q --hard
    fi
    git -C "$MC" clean -q -f -d
    if [ -s "$PATCH" ]; then
        git -C "$MC" apply --whitespace=nowarn "$PATCH"
    fi
    # The built Cycles, USD and Python this checkout tests with (git-ignored).
    for lib in cycles/install usd/install python; do
        if [ -d "development/$lib" ]; then
            mkdir -p "$MC/development/$lib"
            rsync -a --delete "development/$lib/" "$MC/development/$lib/"
        else
            rm -rf "${MC:?}/development/$lib"
        fi
    done
    OPERANDI_SPECS="$(cd "$SPECS" && pwd -P)"
    export OPERANDI_SPECS
    cd "$MC"
    SPECS="$OPERANDI_SPECS"

    # Bound its disk use: start the build folder over past the limit.
    BUILD_DIR=$("$CARGO_BIN" metadata --format-version 1 --no-deps --offline |
        python3 -c 'import json, sys; m = json.load(sys.stdin); print(m.get("build_directory") or m["target_directory"])')
    MAX_GB="${OPERANDI_MERGE_CHECK_MAX_GB:-150}"
    if [ -d "$BUILD_DIR" ]; then
        USED_GB=$(($(du -sk "$BUILD_DIR" | cut -f1) / 1048576))
        echo "merge-check: build folder $BUILD_DIR, ${USED_GB} GB (limit ${MAX_GB} GB)."
        if [ "$USED_GB" -gt "$MAX_GB" ]; then
            echo "merge-check: over the limit; starting the build folder over (this run builds cold)."
            rm -rf "${BUILD_DIR:?}"
        fi
    fi
fi

step() { echo; echo "== $*"; }

# The extras whose libraries are built here, and the Python settings (the
# bundled CPython from scripts/fetch-python.sh, else a python3 on PATH).
# Every step that builds with Python gets the same settings, so its build
# stays warm.
FEATURES=""
add_feature() { FEATURES="${FEATURES:+$FEATURES,}operandi-app/$1"; }
PY_LABEL=""
HAVE_PYTHON=""
if [ -f development/python/pyo3-config.txt ]; then
    PYO3_CONFIG_FILE="$(cd development/python && pwd -P)/pyo3-config.txt"
    OPERANDI_PYTHON_BUILD_HOME="$(cd development/python/python && pwd -P)"
    PY_LABEL="bundled CPython"
elif [ -x development/python/python/bin/python3 ]; then
    PYO3_PYTHON="$(cd development/python/python/bin && pwd -P)/python3"
    PY_LABEL="$PYO3_PYTHON"
elif command -v python3 >/dev/null 2>&1; then
    PYO3_PYTHON="$(command -v python3)"
    PY_LABEL="$PYO3_PYTHON"
fi
with_python() {
    if [ -n "${PYO3_CONFIG_FILE:-}" ]; then
        PYO3_CONFIG_FILE="$PYO3_CONFIG_FILE" OPERANDI_PYTHON_BUILD_HOME="$OPERANDI_PYTHON_BUILD_HOME" "$@"
    else
        PYO3_PYTHON="$PYO3_PYTHON" "$@"
    fi
}

step "fmt"
"$CARGO_BIN" fmt --check
# No new area link against the flow (docs/AREA_DEPENDENCIES.md); the same
# strict check CI runs.
step "area dependencies"
python3 scripts/area-deps.py --self-test
python3 scripts/area-deps.py --check --strict --no-report
# The plain build (no extras), with clippy's warnings as errors.
step "clippy"
"$CARGO_BIN" clippy --locked --workspace --all-targets -- -D warnings

# Each extra on its own must compile, tests included (a build with two
# on can't show that one alone is broken). Compile only; tests run below.
if [ -d development/cycles/install/lib ]; then
    step "Cycles compiles (check --features cycles)"
    "$CARGO_BIN" check --locked --workspace --all-targets --features operandi-app/cycles
    add_feature cycles
else
    echo; echo "== Cycles skipped: no development/cycles (scripts/build-cycles.sh)"
fi
if [ -d development/usd/install/lib ]; then
    step "USD compiles (check --features usd)"
    "$CARGO_BIN" check --locked --workspace --all-targets --features operandi-app/usd
    add_feature usd
else
    echo; echo "== USD skipped: no development/usd (scripts/build-usd.sh)"
fi
if [ -n "$PY_LABEL" ]; then
    step "Python compiles (check --features python, $PY_LABEL)"
    with_python "$CARGO_BIN" check --locked --workspace --all-targets --features operandi-app/python
    HAVE_PYTHON=1
else
    echo; echo "== Python skipped: no Python (scripts/fetch-python.sh)"
fi

# Every test, twice: with Cycles and USD on (or no extras when neither is
# built), then with Python on. Not all three in one build: then Python
# fails to start in the app's tests ("Failed to import encodings
# module", Homebrew Python 3.15, 2026-11-26), while every pair passes.
step "tests (${FEATURES:-no extras})"
OPERANDI_TEST_FEATURES="$FEATURES" scripts/test-all.sh
if [ -n "$HAVE_PYTHON" ]; then
    step "tests (operandi-app/python, $PY_LABEL)"
    OPERANDI_TEST_FEATURES=operandi-app/python with_python scripts/test-all.sh
fi
echo; echo "merge-check: all passed."
