#!/bin/sh
# Makes an Operandi release on GitHub: builds macOS (signed and notarized,
# here on the Mac), Linux and Windows (on Jason's Linux box and Windows VM,
# through the "Release builds" workflow), then uploads all of them to the
# repo's Releases page in one go.
#
#   scripts/release.sh v0.1.0             build all three and publish
#   scripts/release.sh v0.1.0 --dry-run   only run the checks, change nothing
#   scripts/release.sh v0.1.0 --draft     upload as a draft (only visible to
#                                         you until you press Publish)
#   scripts/release.sh v0.1.0 --platforms mac,linux
#                                         only some platforms
#
# This is the ONLY thing that publishes a release. Normal builds
# (scripts/bundle-dev.sh, scripts/release-macos.sh, scripts/build-release.sh)
# just leave files in ~/Developer/Operandi-Builds and never upload.
#
# Before it builds anything it checks: you're on main, nothing tracked is
# uncommitted, main matches GitHub's main, the version matches
# crates/operandi-app/Cargo.toml, the tag and release don't exist yet, the
# signing certificate and notary login are there, and which build machines
# are online (a platform whose machine is off is skipped, and it says so).
# Then it runs scripts/test-all.sh, tags the commit and pushes the tag.
# If anything fails after that, it deletes the tag again so you can rerun.
#
# Needs the GitHub CLI, signed in: `brew install gh`, then `gh auth login`.
# Plain-words guide: docs/RELEASING.md, "Releasing all three".
set -eu
cd "$(dirname "$0")/.."

WORKFLOW=release-builds.yml
NOTARY_PROFILE="${OPERANDI_NOTARY_PROFILE:-operandi}"
SIGN_IDENTITY="${OPERANDI_SIGN_IDENTITY:-Developer ID Application: Jason Morris (PNA7R84ZP6)}"
if [ -z "${DEVELOPER_DIR:-}" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
    DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
    export DEVELOPER_DIR
fi

say() { printf '%s\n' "$*"; }
stop() { printf '\nSTOPPED: %s\n' "$*" >&2; exit 1; }
usage() { sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit "${1:-0}"; }

# ---- Arguments
TAG=""
PLATFORMS=mac,linux,windows
DRAFT=0
DRY_RUN=0
while [ $# -gt 0 ]; do
    case "$1" in
        -h | --help) usage 0 ;;
        --dry-run) DRY_RUN=1 ;;
        --draft) DRAFT=1 ;;
        --platforms) [ $# -ge 2 ] || usage 1; PLATFORMS="$2"; shift ;;
        --platforms=*) PLATFORMS="${1#--platforms=}" ;;
        -*) say "Unknown option $1"; usage 1 ;;
        *) [ -z "$TAG" ] || stop "Give only one version (got $TAG and $1)."; TAG="$1" ;;
    esac
    shift
done
[ -n "$TAG" ] || { say "Which version? For example: scripts/release.sh v0.1.0"; say; usage 1; }
printf '%s' "$TAG" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$' ||
    stop "\"$TAG\" isn't a version like v0.1.0 (or v0.1.0-beta.1)."
VERSION="${TAG#v}"

WANT_MAC=0; WANT_LINUX=0; WANT_WINDOWS=0
for p in $(printf '%s' "$PLATFORMS" | tr ',' ' '); do
    case "$p" in
        mac | macos) WANT_MAC=1 ;;
        linux) WANT_LINUX=1 ;;
        windows) WANT_WINDOWS=1 ;;
        *) stop "Unknown platform \"$p\" (use mac, linux, windows)." ;;
    esac
done

[ "$(uname)" = Darwin ] || stop "Run this on the Mac (it signs the Mac build there)."

# ---- Checks (nothing is changed until they all pass)
say "== Checks for $TAG"
command -v gh >/dev/null 2>&1 || stop "The GitHub CLI isn't installed. Run: brew install gh   then: gh auth login"
gh auth status >/dev/null 2>&1 || stop "The GitHub CLI isn't signed in. Run: gh auth login"
REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner)" || stop "Couldn't find this repo on GitHub."

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = main ] || stop "You're on \"$BRANCH\". Releases come from main: git checkout main"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    git status --short --untracked-files=no >&2
    stop "There are uncommitted changes (listed above). Commit or stash them first."
fi
UNTRACKED="$(git status --porcelain | grep -c '^??' || true)"
[ "$UNTRACKED" -eq 0 ] || say "   note: $UNTRACKED untracked file(s) lying around; they aren't part of the build, so that's fine."

git fetch --quiet origin main
LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse origin/main)"
if [ "$LOCAL" != "$REMOTE" ]; then
    AHEAD="$(git rev-list --count origin/main..HEAD)"
    BEHIND="$(git rev-list --count HEAD..origin/main)"
    [ "$AHEAD" -eq 0 ] || stop "Your main has $AHEAD commit(s) GitHub doesn't have yet. Push first: git push"
    stop "GitHub's main has $BEHIND commit(s) you don't have. Get them first: git pull"
fi

CARGO_VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' crates/operandi-app/Cargo.toml | head -1)"
[ "$CARGO_VERSION" = "$VERSION" ] || stop "The app says it's version $CARGO_VERSION (crates/operandi-app/Cargo.toml) but you asked for $VERSION. Change the version line in that file to \"$VERSION\", commit, push, and run this again."

git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && stop "Tag $TAG already exists here. Pick a new version."
[ -z "$(git ls-remote --tags origin "refs/tags/$TAG")" ] || stop "Tag $TAG already exists on GitHub. Pick a new version."
gh release view "$TAG" >/dev/null 2>&1 && stop "A release called $TAG already exists on GitHub."

if [ "$WANT_MAC" = 1 ]; then
    security find-identity -v -p codesigning | grep -qF "\"$SIGN_IDENTITY\"" ||
        stop "Signing certificate \"$SIGN_IDENTITY\" isn't in the keychain. See docs/RELEASING.md, \"The certificate\" (double-click your .p12 to restore it)."
    xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 ||
        stop "The notary login \"$NOTARY_PROFILE\" isn't saved (or Apple refused it). See docs/RELEASING.md, \"The notary login\"."
    say "   Mac: certificate and notary login OK"
fi

SKIPPED=""
skip() { SKIPPED="$SKIPPED
  - $1: $2"; say "   SKIPPING $1: $2"; }

# Linux and Windows need the workflow on GitHub and their machine online.
if [ "$WANT_LINUX" = 1 ] || [ "$WANT_WINDOWS" = 1 ]; then
    if ! gh workflow view "$WORKFLOW" >/dev/null 2>&1; then
        [ "$WANT_LINUX" = 1 ] && skip Linux "the $WORKFLOW workflow isn't on GitHub's main yet"
        [ "$WANT_WINDOWS" = 1 ] && skip Windows "the $WORKFLOW workflow isn't on GitHub's main yet"
        WANT_LINUX=0; WANT_WINDOWS=0
    elif RUNNERS="$(gh api "repos/$REPO/actions/runners" --paginate \
        --jq '.runners[] | select(.status == "online") | [.labels[].name] | join(" ")' 2>/dev/null)"; then
        for os in Linux Windows; do
            [ "$os" = Linux ] && [ "$WANT_LINUX" = 0 ] && continue
            [ "$os" = Windows ] && [ "$WANT_WINDOWS" = 0 ] && continue
            if printf '%s\n' "$RUNNERS" | grep -w self-hosted | grep -w "$os" | grep -qw X64; then
                say "   $os: build machine online"
            else
                skip "$os" "no $os build machine is online on GitHub (Settings > Actions > Runners; see docs/RELEASING.md)"
                [ "$os" = Linux ] && WANT_LINUX=0
                [ "$os" = Windows ] && WANT_WINDOWS=0
            fi
        done
    else
        say "   note: couldn't ask GitHub which build machines are online; if one is off, its build waits until it's on."
    fi
fi
[ "$WANT_MAC$WANT_LINUX$WANT_WINDOWS" != 000 ] || stop "Nothing left to build."

INCLUDED=""
[ "$WANT_MAC" = 1 ] && INCLUDED="$INCLUDED macOS"
[ "$WANT_LINUX" = 1 ] && INCLUDED="$INCLUDED Linux"
[ "$WANT_WINDOWS" = 1 ] && INCLUDED="$INCLUDED Windows"
say
say "Plan: release $TAG of $REPO from main at $(git rev-parse --short HEAD), with:$INCLUDED"
[ -z "$SKIPPED" ] || say "Left out:$SKIPPED"
[ "$DRAFT" = 1 ] && say "As a draft (you press Publish on GitHub)."
if [ "$DRY_RUN" = 1 ]; then
    say
    say "Dry run: all checks passed, nothing was changed."
    exit 0
fi

# ---- Tests
say
say "== Tests (scripts/test-all.sh)"
scripts/test-all.sh || stop "Tests failed, so no release. Nothing was tagged or uploaded."

# ---- Tag (undone again if anything below fails)
OUT="${OPERANDI_RELEASE_OUTPUT:-$HOME/Developer/Operandi-Builds}/release-$TAG"
ASSETS="$OUT/upload"
rm -rf "$OUT"
mkdir -p "$ASSETS"
TAG_PUSHED=0
RUN_ID=""
RELEASED=0
cleanup() {
    status=$?
    [ "$RELEASED" = 1 ] && return
    if [ -n "$RUN_ID" ]; then
        gh run cancel "$RUN_ID" >/dev/null 2>&1 || true
    fi
    if [ "$TAG_PUSHED" = 1 ]; then
        say "Removing tag $TAG again (here and on GitHub), so you can rerun." >&2
        git push --quiet origin ":refs/tags/$TAG" || say "  couldn't remove it on GitHub: git push origin :refs/tags/$TAG" >&2
        git tag -d "$TAG" >/dev/null 2>&1 || true
    fi
    [ "$status" -eq 0 ] || say "Nothing was published." >&2
}
trap cleanup EXIT
trap 'exit 130' INT TERM

say
say "== Tagging $TAG"
git tag -a "$TAG" -m "Operandi $VERSION"
TAG_PUSHED=1
git push --quiet origin "refs/tags/$TAG"

# ---- Linux and Windows start on their machines while the Mac builds here
if [ "$WANT_LINUX" = 1 ] || [ "$WANT_WINDOWS" = 1 ]; then
    say
    say "== Starting the Linux/Windows builds on GitHub"
    REQUEST="$(date +%Y%m%d%H%M%S)-$$"
    bool() { [ "$1" = 1 ] && echo true || echo false; }
    gh workflow run "$WORKFLOW" --ref main -f tag="$TAG" \
        -f linux="$(bool "$WANT_LINUX")" -f windows="$(bool "$WANT_WINDOWS")" \
        -f request_id="$REQUEST" >/dev/null
    tries=0
    while [ -z "$RUN_ID" ]; do
        tries=$((tries + 1))
        [ "$tries" -le 30 ] || stop "Started the workflow but couldn't find its run on GitHub."
        sleep 3
        RUN_ID="$(gh run list --workflow "$WORKFLOW" --limit 20 --json databaseId,displayTitle \
            --jq ".[] | select(.displayTitle | contains(\"$REQUEST\")) | .databaseId" | head -1)"
    done
    say "   run: $(gh run view "$RUN_ID" --json url -q .url)"
fi

# ---- macOS
if [ "$WANT_MAC" = 1 ]; then
    say
    say "== macOS (scripts/release-macos.sh)"
    # The script's last line is the disk image's path; its log is kept too.
    OPERANDI_RELEASE_OUTPUT="$OUT/mac" scripts/release-macos.sh 2>&1 | tee "$OUT/mac.log"
    DMG="$(tail -1 "$OUT/mac.log")"
    case "$DMG" in *.dmg) ;; *) DMG="" ;; esac
    [ -n "$DMG" ] && [ -f "$DMG" ] || stop "The Mac build failed (its log: $OUT/mac.log)."
    cp "$DMG" "$ASSETS/Operandi-$VERSION-macOS.dmg"
fi

# ---- Wait for Linux and Windows, then fetch their files
if [ -n "$RUN_ID" ]; then
    say
    say "== Waiting for the Linux/Windows builds"
    if ! gh run watch "$RUN_ID" --exit-status --interval 30; then
        gh run view "$RUN_ID" --json jobs --jq '.jobs[] | "   \(.name): \(.conclusion)"' >&2 || true
        stop "A Linux/Windows build failed (above). Its log: gh run view $RUN_ID --log-failed"
    fi
    gh run download "$RUN_ID" -D "$OUT/ci"
    if [ "$WANT_LINUX" = 1 ]; then
        f="$(find "$OUT/ci" -name '*.AppImage' | head -1)"
        [ -n "$f" ] || stop "The Linux build finished but left no AppImage."
        cp "$f" "$ASSETS/Operandi-$VERSION-Linux-x86_64.AppImage"
    fi
    if [ "$WANT_WINDOWS" = 1 ]; then
        f="$(find "$OUT/ci" -name '*.zip' | head -1)"
        [ -n "$f" ] || stop "The Windows build finished but left no zip."
        cp "$f" "$ASSETS/Operandi-$VERSION-Windows-x64.zip"
    fi
fi

# ---- Publish
say
say "== Uploading to GitHub Releases"
(cd "$ASSETS" && shasum -a 256 Operandi-* > SHA256SUMS.txt)
NOTES="$OUT/notes.md"
{
    say "Operandi $VERSION."
    say
    say "Downloads:"
    [ "$WANT_MAC" = 1 ] && say "- **macOS** (Apple silicon): \`Operandi-$VERSION-macOS.dmg\`, signed and notarized. Open it and drag Operandi to Applications."
    [ "$WANT_WINDOWS" = 1 ] && say "- **Windows** (64-bit): \`Operandi-$VERSION-Windows-x64.zip\`. Unzip anywhere and run \`Operandi.exe\`. It isn't signed yet, so Windows may warn first (More info > Run anyway)."
    [ "$WANT_LINUX" = 1 ] && say "- **Linux** (x86_64): \`Operandi-$VERSION-Linux-x86_64.AppImage\`. Make it runnable (\`chmod +x\`) and open it."
    say
    say "\`SHA256SUMS.txt\` lists each file's checksum."
    say
    say "Operandi is free software under the GNU GPL v3. Not affiliated with or endorsed by Foundry."
} > "$NOTES"
set -- --title "Operandi $VERSION" --notes-file "$NOTES" --verify-tag
case "$VERSION" in *-*) set -- "$@" --prerelease ;; esac
[ "$DRAFT" = 1 ] && set -- "$@" --draft
gh release create "$TAG" "$ASSETS"/* "$@" >/dev/null
RELEASED=1

URL="$(gh release view "$TAG" --json url -q .url)"
say
say "Done: $URL"
say "Files are also in $ASSETS"
[ -z "$SKIPPED" ] || say "Left out:$SKIPPED"
if [ "$(gh repo view --json visibility -q .visibility)" != PUBLIC ]; then
    say "The repo is private, so only its collaborators can see or download this release."
fi
