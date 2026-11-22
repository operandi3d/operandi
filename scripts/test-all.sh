#!/bin/sh
# Full-tier tests, faster: the same tests as `cargo test --locked
# --workspace`, but every test program runs at once instead of one after
# another (test speed, Jason 2026-11-16). `cargo test` waits for the app's
# tests (~35 s) before it starts core's (~20 s); here they overlap.
#
#   scripts/test-all.sh            build, then run every test program
#   scripts/test-all.sh <filter>   ... passing <filter> to each program
#
# Each program runs in its own package folder, as `cargo test` does. The
# workspace has no doc tests, so none are run. Output of each program goes
# to a log; failing programs' logs are printed at the end.
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO_BIN:-cargo}"

LIST=$(mktemp)
LOGS=$(mktemp -d)
trap 'rm -rf "$LIST" "$LOGS"' EXIT

"$CARGO_BIN" test --locked --workspace --no-run --message-format=json 2>/dev/null |
    python3 -c '
import json, os, sys
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
        name = m["target"]["name"] + "-" + m["target"]["kind"][0]
        print(name, os.path.dirname(m["manifest_path"]), m["executable"], sep="\t")
' >"$LIST"
if [ ! -s "$LIST" ]; then
    # Show the build error.
    "$CARGO_BIN" test --locked --workspace --no-run
    exit 1
fi

# As `cargo test` does through .cargo/config.toml: the app state is big,
# so tests get the main thread's 8 MB stack.
RUST_MIN_STACK="${RUST_MIN_STACK:-8388608}"
export RUST_MIN_STACK

START=$(date +%s)
PIDS=""
while IFS="$(printf '\t')" read -r name dir exe; do
    (
        status=0
        cd "$dir" && "$exe" "$@" >"$LOGS/$name.log" 2>&1 || status=$?
        echo "$status" >"$LOGS/$name.status"
    ) &
    PIDS="$PIDS $!"
done <"$LIST"
# shellcheck disable=SC2086
wait $PIDS

FAILED=0
PASSED=0
for status in "$LOGS"/*.status; do
    name=$(basename "$status" .status)
    if [ "$(cat "$status")" != 0 ]; then
        FAILED=$((FAILED + 1))
        echo "=== FAILED: $name"
        cat "$LOGS/$name.log"
    else
        PASSED=$((PASSED + $(sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' "$LOGS/$name.log" | head -1)))
    fi
done
echo "test-all: $PASSED tests passed in $(($(date +%s) - START)) s; $FAILED test programs failed."
[ "$FAILED" = 0 ]
