#!/bin/sh
# The nightly test run (testing plan item 1, Jason 2026-11-18). Plain
# shell and cargo: it uses no Claude and no network beyond what cargo
# needs. launchd starts it (scripts/nightly-install.sh); it can also be run
# by hand.
#
#   scripts/nightly.sh
#
# It tests the newest local `main` in its own detached worktree,
# ~/Developer/Operandi-nightly, so it never touches a checkout someone is
# working in. Results go to ~/Developer/Operandi-Test-Results/:
#
#   latest.md               the last run: one line per step, failing tests,
#                           and failures that are new since the run before
#   runs/<date>.md          the same, kept per night
#   runs/<date>/<step>.log  full output of each step
#
# Claude reads latest.md only when someone asks.
set -u
# Keep the Mac awake (not the display) until the run ends; caffeinate
# lets it sleep again as soon as this script exits.
if [ -z "${OPERANDI_NIGHTLY_AWAKE:-}" ] && [ -x /usr/bin/caffeinate ]; then
    OPERANDI_NIGHTLY_AWAKE=1 exec /usr/bin/caffeinate -i /bin/sh "$0" "$@"
fi
PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export PATH
CARGO_BIN="${CARGO_BIN:-cargo}"

# The repo this script belongs to (any checkout of it) finds main.
REPO=$(cd "$(dirname "$0")/.." && git rev-parse --path-format=absolute --git-common-dir)
REPO=$(dirname "$REPO")
TREE="${OPERANDI_NIGHTLY_TREE:-$HOME/Developer/Operandi-nightly}"
OUT="${OPERANDI_NIGHTLY_OUT:-$HOME/Developer/Operandi-Test-Results}"
STAMP=$(date +%Y-%m-%d)
LOGS="$OUT/runs/$STAMP"
REPORT="$OUT/runs/$STAMP.md"
mkdir -p "$LOGS"

# Only one run at a time (a late run can overlap the next night's).
LOCK="$OUT/.running"
if ! mkdir "$LOCK" 2>/dev/null; then
    echo "nightly: another run holds $LOCK; remove it if that run died." >&2
    exit 1
fi
trap 'rmdir "$LOCK"' EXIT

if [ ! -d "$TREE" ]; then
    # The first night builds from cold (a few minutes); later nights reuse
    # this worktree's build folder.
    git -C "$REPO" worktree add --detach "$TREE" main
fi
if ! git -C "$TREE" checkout --quiet --detach main || [ ! -f "$TREE/Cargo.toml" ]; then
    echo "# Nightly $STAMP: could not set up $TREE" >"$OUT/latest.md"
    exit 1
fi
# The Cycles and USD libraries are built once in the main checkout.
for lib in cycles usd; do
    if [ -d "$REPO/development/$lib/install" ] && [ ! -e "$TREE/development/$lib" ]; then
        mkdir -p "$TREE/development"
        ln -s "$REPO/development/$lib" "$TREE/development/$lib"
    fi
done
cd "$TREE"
COMMIT=$(git log -1 --format='%h %s')

PASSED=0
FAILED=0
LINES=""
# Run one step; record pass/fail and the time it took.
run() {
    name=$1
    shift
    start=$(date +%s)
    if "$@" >"$LOGS/$name.log" 2>&1; then
        result="pass"
        PASSED=$((PASSED + 1))
    else
        result="**FAIL**"
        FAILED=$((FAILED + 1))
    fi
    took=$(($(date +%s) - start))
    LINES="$LINES| $name | $result | ${took}s |
"
}
skip() {
    LINES="$LINES| $1 | skipped: $2 | |
"
}

run fmt "$CARGO_BIN" fmt --check
run clippy "$CARGO_BIN" clippy --locked --workspace --all-targets -- -D warnings
run tests "$CARGO_BIN" test --locked --workspace --no-fail-fast
# Monkey-user sessions (monkey_tests.rs): 2,000 random sessions, ~10 min.
run monkey env OPERANDI_MONKEY_SEEDS=2000 "$CARGO_BIN" test --locked -p operandi-app monkey_sessions_long -- --ignored
if [ -d development/cycles/install/lib ]; then
    run cycles "$CARGO_BIN" test --locked --workspace --no-fail-fast --features operandi-app/cycles
else
    skip cycles "Cycles library not built (scripts/build-cycles.sh)"
fi
if [ -d development/usd/install/lib ]; then
    run usd "$CARGO_BIN" test --locked --workspace --no-fail-fast --features operandi-app/usd
else
    skip usd "USD library not built (scripts/build-usd.sh)"
fi

# Failing test names from every log ("test foo::bar ... FAILED").
grep -h '^test .* FAILED$' "$LOGS"/*.log 2>/dev/null | sed 's/^test //; s/ \.\.\. FAILED$//' | sort -u >"$LOGS/failing.txt"
PREVIOUS=$(ls -1d "$OUT"/runs/*/ 2>/dev/null | grep -v "/$STAMP/\$" | tail -1)
NEW=""
if [ -n "$PREVIOUS" ] && [ -f "$PREVIOUS/failing.txt" ]; then
    NEW=$(comm -23 "$LOGS/failing.txt" "$PREVIOUS/failing.txt")
fi

{
    if [ "$FAILED" = 0 ]; then
        echo "# Nightly $STAMP: all passed"
    else
        echo "# Nightly $STAMP: $FAILED step(s) failed"
    fi
    echo
    echo "main at \`$COMMIT\`. Logs: \`$LOGS\`."
    echo
    echo "| Step | Result | Time |"
    echo "|---|---|---|"
    printf '%s' "$LINES"
    if [ -s "$LOGS/failing.txt" ]; then
        echo
        echo "## Failing tests"
        echo
        sed 's/^/- /' "$LOGS/failing.txt"
    fi
    if [ -n "$NEW" ]; then
        echo
        echo "## New since the run before"
        echo
        printf '%s\n' "$NEW" | sed 's/^/- /'
    fi
} >"$REPORT"
cp "$REPORT" "$OUT/latest.md"
[ "$FAILED" = 0 ]
