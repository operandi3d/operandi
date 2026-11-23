#!/bin/sh
# Builds the Mac and Windows test apps and puts them on one rolling GitHub
# pre-release, "Test build" (tag `test-build`), replacing the files that
# were there. Testers keep one link that always has the newest build:
#
#   https://github.com/<owner>/<repo>/releases/tag/test-build
#
#   scripts/test-build.sh                 build GitHub's main, upload
#   scripts/test-build.sh <branch>        build that branch from GitHub
#   scripts/test-build.sh --platforms mac only the Mac (or: windows)
#   scripts/test-build.sh --no-upload     build and check, upload nothing
#
# OPERANDI_WINDOWS_SSH=user@host is the Windows machine. Windows builds
# there over ssh, in its checkout (OPERANDI_WINDOWS_SSH_REPO, default
# Developer/Operandi under its home folder), the same way scripts/release.sh
# does, while the Mac builds here.
#
# It builds what is on GitHub, never a local-only commit, so each file's
# source is public (the GPL). The Mac app is a QA build
# (scripts/bundle-dev.sh with Cycles), not signed or notarized. The build
# happens in its own worktree, ~/Developer/Operandi-test-build, so it
# never switches branches in your main checkout. It never touches real
# releases such as v0.1.0, only the `test-build` tag and its release.
# Files that look like Foundry content or a private model stop the upload.
#
# Needs the GitHub CLI, signed in (gh auth login). Plain-words guide:
# docs/RELEASING.md, "Test builds".
set -eu

say() { printf '%s\n' "$*"; }
stop() { printf '\nSTOPPED: %s\n' "$*" >&2; exit 1; }
usage() { sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit "${1:-0}"; }

TAG=test-build
TITLE="Test build"
MAC_FILE=Operandi-Test-macOS.zip
WIN_FILE=Operandi-Test-Windows-x64.zip
# File names that must never be in a public download: Sylvain's model,
# Foundry content, Modo scenes and presets.
FORBIDDEN='sylvain|hc02|foundry|modo|\.lx[op]$'

# ---- Arguments
BRANCH=main
PLATFORMS=mac,windows
UPLOAD=1
GOT_BRANCH=0
while [ $# -gt 0 ]; do
    case "$1" in
        -h | --help) usage 0 ;;
        --no-upload) UPLOAD=0 ;;
        --platforms) [ $# -ge 2 ] || usage 1; PLATFORMS="$2"; shift ;;
        --platforms=*) PLATFORMS="${1#--platforms=}" ;;
        -*) say "Unknown option $1"; usage 1 ;;
        *) [ "$GOT_BRANCH" = 0 ] || stop "Give only one branch (got $BRANCH and $1)."; BRANCH="$1"; GOT_BRANCH=1 ;;
    esac
    shift
done
BRANCH="${BRANCH#origin/}"
WANT_MAC=0; WANT_WINDOWS=0
for p in $(printf '%s' "$PLATFORMS" | tr ',' ' '); do
    case "$p" in
        mac | macos) WANT_MAC=1 ;;
        windows) WANT_WINDOWS=1 ;;
        *) stop "Unknown platform \"$p\" (use mac, windows)." ;;
    esac
done
[ "$WANT_MAC$WANT_WINDOWS" != 00 ] || stop "Nothing to build."
[ "$(uname)" = Darwin ] || stop "Run this on the Mac."

# The main checkout, wherever this copy of the script lives.
REPO="${OPERANDI_REPO:-}"
if [ -z "$REPO" ]; then
    REPO="$(cd "$(dirname "$0")" && git rev-parse --path-format=absolute --git-common-dir)"
    REPO="$(dirname "$REPO")"
fi
TREE="$(dirname "$REPO")/Operandi-test-build"
OUT="${OPERANDI_TEST_BUILD_OUTPUT:-$HOME/Developer/Operandi-Builds/test-build}"
WIN_SSH="${OPERANDI_WINDOWS_SSH:-}"
WIN_REPO="${OPERANDI_WINDOWS_SSH_REPO:-Developer/Operandi}"

# ---- Checks
say "== Checks"
if [ "$UPLOAD" = 1 ]; then
    command -v gh >/dev/null 2>&1 || stop "The GitHub CLI isn't installed. Run: brew install gh   then: gh auth login"
    gh auth status >/dev/null 2>&1 || stop "The GitHub CLI isn't signed in. Run: gh auth login"
    ORIGIN_URL="$(git -C "$REPO" remote get-url origin)" || stop "The checkout has no \"origin\" remote."
    GH_REPO="$(gh repo view "$ORIGIN_URL" --json nameWithOwner -q .nameWithOwner)" || stop "Couldn't find the repo on GitHub."
    export GH_REPO
fi
if [ "$WANT_WINDOWS" = 1 ]; then
    [ -n "$WIN_SSH" ] || stop "Windows builds over ssh: set OPERANDI_WINDOWS_SSH=user@host (or use --platforms mac)."
    ssh -o ConnectTimeout=10 -o BatchMode=yes "$WIN_SSH" "cd $WIN_REPO && git rev-parse --git-dir" >/dev/null 2>&1 ||
        stop "Can't reach the Windows machine $WIN_SSH over ssh, or it has no checkout at $WIN_REPO."
    say "   Windows: $WIN_SSH is reachable"
fi

git -C "$REPO" fetch --quiet origin "$BRANCH" ||
    stop "There's no branch \"$BRANCH\" on GitHub. Test builds only come from pushed branches."
COMMIT="$(git -C "$REPO" rev-parse FETCH_HEAD)"
SHORT="$(git -C "$REPO" rev-parse --short=8 "$COMMIT")"
SUBJECT="$(git -C "$REPO" log -1 --format=%s "$COMMIT")"

# One run at a time.
mkdir -p "$OUT"
LOCK="$OUT/.running"
mkdir "$LOCK" 2>/dev/null || stop "Another test build is running (or one died: then remove $LOCK)."
WIN_PID=""
cleanup() {
    [ -z "$WIN_PID" ] || kill "$WIN_PID" 2>/dev/null || true
    rmdir "$LOCK" 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 130' INT TERM

# ---- The build worktree, on the commit to build
if [ ! -f "$TREE/Cargo.toml" ]; then
    say
    say "== Making the build worktree $TREE (first run only)"
    # Seeds the worktree's build cache from the main checkout. It has to
    # start on a branch; the worktree then sits on the commit itself
    # ("detached"), and the branch is deleted again.
    (cd "$REPO" && scripts/new-worktree.sh test-build "$COMMIT")
    git -C "$TREE" checkout --quiet --detach
    git -C "$TREE" branch -q -d test-build 2>/dev/null || true
fi
git -C "$TREE" checkout --quiet --force --detach "$COMMIT"
# The Cycles and USD libraries are built once, in the main checkout.
for lib in cycles usd; do
    [ -d "$REPO/development/$lib/install" ] || continue
    [ -L "$TREE/development/$lib" ] && continue
    mkdir -p "$TREE/development"
    rm -rf "$TREE/development/$lib"
    ln -s "$REPO/development/$lib" "$TREE/development/$lib"
done

say
say "Plan: test build of $BRANCH at $SHORT ($SUBJECT)"
rm -rf "$OUT/upload" "$OUT/windows"
mkdir -p "$OUT/upload"

# ---- Windows over ssh, in the background while the Mac builds
if [ "$WANT_WINDOWS" = 1 ]; then
    say
    say "== Starting the Windows build on $WIN_SSH (log: $OUT/windows.log)"
    mkdir -p "$OUT/windows"
    # Send only what the Windows checkout doesn't have yet when we can.
    THERE="$(ssh -o BatchMode=yes "$WIN_SSH" "cd $WIN_REPO && git rev-parse HEAD" 2>/dev/null | tr -d '\r' || true)"
    if [ -n "$THERE" ] && git -C "$TREE" merge-base --is-ancestor "$THERE" HEAD 2>/dev/null && [ "$THERE" != "$COMMIT" ]; then
        git -C "$TREE" bundle create "$OUT/windows/build.bundle" HEAD "^$THERE" 2>/dev/null
    else
        git -C "$TREE" bundle create "$OUT/windows/build.bundle" HEAD 2>/dev/null
    fi
    # cmd quoting mangles inline bash, so the steps go over as a file.
    cat > "$OUT/windows/operandi-test-build.sh" <<EOF
#!/bin/bash
set -eu
rm -f ~/operandi-test-build.zip
# Visual Studio's CMake and Python aren't on the ssh PATH by default.
export PATH="/c/Program Files/Python312:\$HOME/.cargo/bin:\$PATH"
cd ~/$WIN_REPO
git fetch -q ~/operandi-test-build.bundle HEAD
git checkout -q --force --detach $COMMIT
rm -f ~/operandi-test-build.bundle
OUT=~/operandi-test-build-out
rm -rf "\$OUT" && mkdir -p "\$OUT"
OPERANDI_RELEASE_OUTPUT="\$OUT" scripts/build-release.sh
# A fixed name to copy back: scp from Windows doesn't expand wildcards.
cp "\$OUT"/*.zip ~/operandi-test-build.zip
rm -rf "\$OUT"
EOF
    (
        set -e
        scp -q "$OUT/windows/build.bundle" "$WIN_SSH:operandi-test-build.bundle"
        scp -q "$OUT/windows/operandi-test-build.sh" "$WIN_SSH:operandi-test-build.sh"
        ssh "$WIN_SSH" '"C:\Program Files\Git\bin\bash.exe" -l operandi-test-build.sh'
        scp -q "$WIN_SSH:operandi-test-build.zip" "$OUT/windows/$WIN_FILE"
    ) > "$OUT/windows.log" 2>&1 &
    WIN_PID=$!
fi

# ---- macOS
if [ "$WANT_MAC" = 1 ]; then
    say
    say "== macOS (scripts/bundle-dev.sh with Cycles; log: $OUT/mac.log)"
    if ! (cd "$TREE" && OPERANDI_CYCLES=1 OPERANDI_DEV_NAME="Operandi Test Build" sh scripts/bundle-dev.sh) > "$OUT/mac.log" 2>&1; then
        tail -30 "$OUT/mac.log" >&2
        stop "The Mac build failed (above; whole log: $OUT/mac.log). If it's about Cycles or USD, rebuild them in the main checkout (scripts/build-cycles.sh, scripts/build-usd.sh) and run this again."
    fi
    APP="$(tail -1 "$OUT/mac.log")"
    case "$APP" in *.app) [ -d "$APP" ] ;; *) false ;; esac || stop "The Mac build finished but left no app (log: $OUT/mac.log)."
    BAD="$(find "$APP" | grep -iE "$FORBIDDEN" || true)"
    [ -z "$BAD" ] || { say "$BAD" >&2; stop "The Mac app has files that must not be shared (above). Nothing was uploaded."; }
    ditto -c -k --sequesterRsrc --keepParent "$APP" "$OUT/upload/$MAC_FILE"
    say "   $(du -h "$OUT/upload/$MAC_FILE" | cut -f1) $MAC_FILE"
fi

# ---- Wait for Windows
if [ -n "$WIN_PID" ]; then
    say
    say "== Waiting for the Windows build"
    if ! wait "$WIN_PID"; then
        WIN_PID=""
        tail -30 "$OUT/windows.log" >&2
        stop "The Windows build failed (above; whole log: $OUT/windows.log). Nothing was uploaded."
    fi
    WIN_PID=""
    [ -f "$OUT/windows/$WIN_FILE" ] || stop "The Windows build finished but no zip came back (log: $OUT/windows.log)."
    BAD="$(unzip -Z1 "$OUT/windows/$WIN_FILE" | grep -iE "$FORBIDDEN" || true)"
    [ -z "$BAD" ] || { say "$BAD" >&2; stop "The Windows zip has files that must not be shared (above). Nothing was uploaded."; }
    mv "$OUT/windows/$WIN_FILE" "$OUT/upload/"
    say "   $(du -h "$OUT/upload/$WIN_FILE" | cut -f1) $WIN_FILE"
fi

if [ "$UPLOAD" = 0 ]; then
    say
    say "Built $SHORT, not uploaded (--no-upload). Files: $OUT/upload"
    exit 0
fi

# ---- Upload
say
say "== Uploading to the \"$TITLE\" pre-release"
# The release notes remember which commit each file came from, so a Mac-only
# run still says where the Windows file it leaves in place was built.
OLD_BODY="$(gh release view "$TAG" --json body -q .body 2>/dev/null || true)"
was() { printf '%s\n' "$OLD_BODY" | sed -n "s/^<!-- $1: \([0-9a-f]*\) -->.*/\1/p" | head -1; }
MAC_SHA="$(was mac)"; WIN_SHA="$(was windows)"
[ "$WANT_MAC" = 1 ] && MAC_SHA="$COMMIT"
[ "$WANT_WINDOWS" = 1 ] && WIN_SHA="$COMMIT"
URL_BASE="https://github.com/$GH_REPO"
line() { # platform, file, sha
    [ -n "$3" ] || return 0
    say "- **$1**: \`$2\`, built from [\`$(printf '%.8s' "$3")\`]($URL_BASE/tree/$3) ($(git -C "$REPO" log -1 --format='%s, %cd' --date=format:'%b %d %H:%M' "$3" 2>/dev/null || echo "commit $3"))."
}
NOTES="$OUT/notes.md"
{
    say "The newest test build of Operandi. It isn't a release: the files here are replaced every time a new test build goes up, so this page always has the latest."
    say
    say "Built from \`$BRANCH\` on $(date '+%b %d %Y, %H:%M %Z')."
    say
    line macOS "$MAC_FILE" "$MAC_SHA"
    line Windows "$WIN_FILE" "$WIN_SHA"
    say
    say "**macOS** (Apple silicon): unzip and drag the app to Applications. It isn't notarized, so the first time macOS refuses to open it: open System Settings > Privacy & Security, scroll down and click **Open Anyway**. Or run \`xattr -dr com.apple.quarantine \"/Applications/Operandi Test Build\"*.app\` in Terminal."
    say
    say "**Windows** (64-bit): unzip anywhere and run \`Operandi.exe\`. It isn't signed, so Windows may warn first (More info > Run anyway)."
    say
    say "Found a bug? Use Help > Report Bug or Idea… in the app."
    say
    say "Operandi is free software under the GNU GPL v3; each file's source is the commit linked above. Not affiliated with or endorsed by Foundry."
    say
    [ -z "$MAC_SHA" ] || say "<!-- mac: $MAC_SHA -->"
    [ -z "$WIN_SHA" ] || say "<!-- windows: $WIN_SHA -->"
} > "$NOTES"

if gh release view "$TAG" >/dev/null 2>&1; then
    gh release upload "$TAG" "$OUT/upload"/* --clobber
    # Move the tag to this commit, so GitHub's source download matches.
    gh api -X PATCH "repos/$GH_REPO/git/refs/tags/$TAG" -f sha="$COMMIT" -F force=true >/dev/null
    gh release edit "$TAG" --title "$TITLE" --notes-file "$NOTES" --prerelease --latest=false >/dev/null
else
    gh release create "$TAG" "$OUT/upload"/* --target "$COMMIT" --title "$TITLE" \
        --notes-file "$NOTES" --prerelease --latest=false >/dev/null
fi

say
say "Done: $(gh release view "$TAG" --json url -q .url)"
