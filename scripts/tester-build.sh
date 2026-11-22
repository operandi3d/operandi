#!/bin/sh
# Jason's tester app (2026-11-21): one Mac app of the newest local `main`
# that always lives at the same place,
#
#   ~/Developer/Operandi-Builds/Operandi Test.app
#
# so tutorials and bug hunting can go on while feature branches build.
# It builds in its own locked worktree, ~/Developer/Operandi-tester, with
# its own build folder, so no other checkout or branch build is touched.
# launchd runs it every 20 minutes (scripts/tester-install.sh); a run with
# nothing new to do takes well under a second.
#
#   scripts/tester-build.sh           build if main moved, then install
#   scripts/tester-build.sh --force   rebuild even if main didn't move
#
# The app is never swapped while it is open: the new build waits, a
# notification says so, and it goes in on the first run after you quit.
# Status: ~/Developer/Operandi-Test-Results/tester/status.md (build.log
# holds the last build's output).
set -u
PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export PATH
NAME="Operandi Test"
REPO="${OPERANDI_REPO:-}"
if [ -z "$REPO" ]; then
    REPO=$(cd "$(dirname "$0")/.." && git rev-parse --path-format=absolute --git-common-dir)
    REPO=$(dirname "$REPO")
fi
TREE="${OPERANDI_TESTER_TREE:-$HOME/Developer/Operandi-tester}"
BUILDS="${OPERANDI_DEV_OUTPUT_ROOT:-$HOME/Developer/Operandi-Builds}"
APP="$BUILDS/$NAME.app"
STATE="$HOME/Developer/Operandi-Test-Results/tester"
STAGE="$STATE/staged"
NIGHTLY_LOCK="$HOME/Developer/Operandi-Test-Results/.running"
mkdir -p "$STATE" "$BUILDS"

notify() {
    /usr/bin/osascript -e "display notification \"$1\" with title \"$NAME\"" >/dev/null 2>&1 || true
}
status() {
    printf '# %s\n\n%s\n\nUpdated %s.\n' "$NAME" "$1" "$(date '+%b %d %H:%M')" >"$STATE/status.md"
}
running() {
    pgrep -f "$APP/Contents/MacOS/" >/dev/null 2>&1
}
# Put a staged build in place, unless the app is open.
install_staged() {
    staged=$(ls -1d "$STAGE"/*.app 2>/dev/null | head -1)
    [ -n "$staged" ] || return 0
    commit=$(cat "$STAGE/commit" 2>/dev/null)
    if running; then
        if [ ! -f "$STAGE/told" ]; then
            notify "A newer build ($commit) is ready. Quit Operandi Test and it updates within 20 minutes."
            : >"$STAGE/told"
        fi
        status "New build $commit is waiting for Operandi Test to be quit."
        return 0
    fi
    rm -rf "$APP.old"
    [ -e "$APP" ] && mv "$APP" "$APP.old"
    if ditto --norsrc --noextattr "$staged" "$APP" && codesign --verify --deep --strict "$APP"; then
        rm -rf "$APP.old"
        cp "$STAGE/commit" "$STATE/installed"
        subject=$(cat "$STAGE/subject" 2>/dev/null)
        rm -rf "$STAGE"
        status "Installed: main at \`$commit\` ($subject)."
        notify "Updated to main $commit."
    else
        rm -rf "$APP"
        [ -e "$APP.old" ] && mv "$APP.old" "$APP"
        status "Could not copy the new build into place; kept the old one."
    fi
}

# One run at a time, and never during the nightly test run.
LOCK="$STATE/.running"
if ! mkdir "$LOCK" 2>/dev/null; then
    # A lock older than 3 hours is from a run that died.
    if [ -n "$(find "$LOCK" -maxdepth 0 -mmin +180 2>/dev/null)" ]; then
        rmdir "$LOCK" && mkdir "$LOCK" || exit 0
    else
        exit 0
    fi
fi
trap 'rmdir "$LOCK"' EXIT
[ -d "$NIGHTLY_LOCK" ] && exit 0

install_staged
MAIN=$(git -C "$REPO" rev-parse --short main) || exit 1
INSTALLED=$(cat "$STATE/installed" 2>/dev/null)
STAGED=$(cat "$STAGE/commit" 2>/dev/null)
FAILED=$(cat "$STATE/failed" 2>/dev/null)
if [ "${1:-}" != --force ]; then
    [ "$MAIN" = "$INSTALLED" ] && exit 0
    [ "$MAIN" = "$STAGED" ] && exit 0
    [ "$MAIN" = "$FAILED" ] && exit 0
fi

if [ ! -f "$TREE/Cargo.toml" ]; then
    git -C "$REPO" worktree add --detach "$TREE" main || exit 1
    git -C "$REPO" worktree lock --reason "Operandi Test app (scripts/tester-build.sh)" "$TREE" 2>/dev/null || true
fi
git -C "$TREE" checkout --quiet --force --detach main || exit 1
# The Cycles and USD libraries are built once in the main checkout.
for lib in cycles usd; do
    if [ -d "$REPO/development/$lib/install" ] && [ ! -e "$TREE/development/$lib" ]; then
        mkdir -p "$TREE/development"
        ln -s "$REPO/development/$lib" "$TREE/development/$lib"
    fi
done
SUBJECT=$(git -C "$TREE" log -1 --format=%s | tr -d '"\\')
status "Building main at \`$MAIN\` ($SUBJECT)..."
OUT="$STATE/out"
rm -rf "$OUT"
if (cd "$TREE" && OPERANDI_CYCLES=1 OPERANDI_DEV_NAME="$NAME" OPERANDI_DEV_OUTPUT="$OUT" \
    sh scripts/bundle-dev.sh) >"$STATE/build.log" 2>&1; then
    rm -rf "$STAGE"
    mkdir -p "$STAGE"
    mv "$OUT"/*.app "$STAGE/" && printf '%s\n' "$MAIN" >"$STAGE/commit"
    printf '%s\n' "$SUBJECT" >"$STAGE/subject"
    rm -rf "$OUT" "$STATE/failed"
    install_staged
else
    printf '%s\n' "$MAIN" >"$STATE/failed"
    rm -rf "$OUT"
    status "main at \`$MAIN\` did not build (see build.log); kept the old app."
    notify "The newest main ($MAIN) didn't build. Kept the old app."
    exit 1
fi
