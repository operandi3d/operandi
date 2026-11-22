#!/bin/sh
# Make a worktree for a branch with a warm build cache, instead of a cold
# one that spends minutes recompiling every dependency.
#
#   scripts/new-worktree.sh <branch> [base]     (base defaults to main)
#
# Creates ~/Developer/Operandi-<branch> (AGENTS.md, "Git"), on <branch> if it
# exists or as a new branch from <base>. Then it copies the main checkout's
# Cargo build folder and its Cycles libraries into the new worktree's own.
# On APFS the copy is a clone: it takes seconds and no extra disk space
# until files change. Cargo then rebuilds only Operandi's own crates (their
# build ids include the checkout's path), not the ~270 dependencies: about
# 110 s for the first test build instead of 180 s (2026-11-15).
#
# OPERANDI_SEED_FROM=<checkout> seeds from another checkout instead of the
# main one. Run it from any checkout of this repo. git worktree needs write
# access to the main checkout's .git folder, so agents run it outside the
# sandbox.
set -eu

[ $# -ge 1 ] || { echo "usage: $0 <branch> [base]" >&2; exit 2; }
BRANCH=$1
BASE=${2:-main}
CARGO_BIN="${CARGO_BIN:-cargo}"

# The main checkout is the first entry `git worktree list` prints.
MAIN=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
NAME=$(printf '%s' "$BRANCH" | sed 's#^claude/##; s#^codex/##; s#/#-#g')
DEST="$(dirname "$MAIN")/Operandi-$NAME"

if [ -e "$DEST" ]; then
    echo "$DEST already exists; reuse it (AGENTS.md: one worktree per branch)." >&2
    exit 1
fi
if git -C "$MAIN" show-ref --verify --quiet "refs/heads/$BRANCH"; then
    git -C "$MAIN" worktree add "$DEST" "$BRANCH"
else
    git -C "$MAIN" worktree add -b "$BRANCH" "$DEST" "$BASE"
fi

build_dir() {
    "$CARGO_BIN" metadata --format-version 1 --no-deps --manifest-path "$1/Cargo.toml" |
        python3 -c 'import json, sys; print(json.load(sys.stdin).get("build_directory") or "")'
}
FROM=$(build_dir "${OPERANDI_SEED_FROM:-$MAIN}")
TO=$(build_dir "$DEST")
if [ -z "$FROM" ] || [ -z "$TO" ] || [ "$FROM" = "$TO" ]; then
    echo "No separate build folder to seed (FROM='$FROM' TO='$TO'); the first build starts cold."
elif [ ! -d "$FROM" ]; then
    echo "The main checkout has no build folder yet ($FROM); the first build starts cold."
elif [ -e "$TO" ]; then
    echo "$TO already exists; leaving it as it is."
else
    mkdir -p "$TO"
    # -c clones on APFS. Skip incremental/: those caches are keyed by the
    # crate's build id, which differs in the new checkout, so they'd never
    # be used.
    for item in "$FROM"/* "$FROM"/.[!.]*; do
        [ -e "$item" ] || continue
        case "$(basename "$item")" in
            debug | release | qa)
                mkdir -p "$TO/$(basename "$item")"
                for sub in "$item"/* "$item"/.[!.]*; do
                    [ -e "$sub" ] || continue
                    [ "$(basename "$sub")" = incremental ] && continue
                    cp -c -R "$sub" "$TO/$(basename "$item")/" 2>/dev/null ||
                        cp -R "$sub" "$TO/$(basename "$item")/"
                done
                ;;
            *) cp -c -R "$item" "$TO/" 2>/dev/null || cp -R "$item" "$TO/" ;;
        esac
    done
    echo "Seeded $TO from $FROM."
fi

# Cycles and USD libraries (git-ignored, built by build-cycles.sh and
# build-usd.sh). Copied from the main checkout: the new branch starts from
# main, so they match. Rerun those scripts if the branch changes their C++.
for lib in cycles usd; do
    if [ -d "$MAIN/development/$lib/install" ] && [ ! -e "$DEST/development/$lib/install" ]; then
        mkdir -p "$DEST/development/$lib"
        cp -c -R "$MAIN/development/$lib/install" "$DEST/development/$lib/" 2>/dev/null ||
            cp -R "$MAIN/development/$lib/install" "$DEST/development/$lib/"
        echo "Copied $lib libraries from the main checkout."
    fi
done

printf '%s\n' "$DEST"
