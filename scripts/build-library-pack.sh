#!/usr/bin/env bash
# Build the Operandi Library pack: the CC0 starter materials as a versioned
# zip people download and unzip into their library folder.
#
#   scripts/build-library-pack.sh [version] [starter] [output]
#
# version: calendar version, default 2026.11.5
# starter: the converted materials (scripts/build-starter-pack.sh writes
#          them), default ~/Developer/Operandi-Content/Starter
# output:  default ~/Developer/Operandi-Content (never inside the repo)
#
# Writes <output>/Operandi-Library-<version>.zip holding one folder,
# "Operandi Library": manifest.json, CREDITS.txt, LICENSES/CC0-1.0.txt and
# Starter/<Category>/<Name>/ (<Name>.oppreset, its maps, <Name>.png
# thumbnail); the starter folder is laid out the same way, and a material
# straight in it (no category folder) is copied without one. The
# manifest is made from each preset's own licence header, and the build
# stops (exit 3) if any Starter item isn't CC0 or has no source URL.
#
# Matcaps (MATCAPS MC-10): MATCAPS=<folder> (default
# ~/Developer/Operandi-Content/Matcaps) is copied in as Matcaps/, with its
# subfolders, CREDITS.txt and licence notes, minus the Blender matcaps the
# app already builds in (crates/operandi-app/assets/studiolights; listed in
# BUILT_IN_MATCAPS below).
#
# Environments (2026.11.4): ENVIRONMENTS=<folder> (default
# ~/Developer/Operandi-Content/Environments) is copied in as
# Environments/<Category>/<Name>.hdr|.exr with its CREDITS.txt. The browser
# lists HDR pictures as environments and draws their thumbnails itself.
#
# Both picture folders need CREDITS.txt (exit 4 without) and SOURCES.tsv:
# one tab-separated line per picture (path, licence, source URL, credit,
# optional date checked) that the licence gate turns into manifest items.
# The gate refuses (exit 3) a picture without a CC0 line. SOURCES.tsv and
# the _download/ folders (licence proof) never ship.
#
# Meshes (2026.11.5): MESHES=<folder> (default
# ~/Developer/Operandi-Content/Meshes) is copied in as
# Meshes/<Category>/<Name>.oppreset. Make it with
# `operandi-app --write-starter-meshes <folder>` (a Cycles build): our own
# shapes from operandi-core preset/starter_meshes.rs, CC0 in their headers,
# thumbnails rendered inside. The gate reads their headers like Starter's.
#
# LIBRARY_INSTALL=<folder> also unzips the pack there (for QA).
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-2026.11.5}"
STARTER="${2:-$HOME/Developer/Operandi-Content/Starter}"
MATCAPS="${MATCAPS:-$HOME/Developer/Operandi-Content/Matcaps}"
ENVIRONMENTS="${ENVIRONMENTS:-$HOME/Developer/Operandi-Content/Environments}"
MESHES="${MESHES:-$HOME/Developer/Operandi-Content/Meshes}"
# The app's own matcaps (src/matcap.rs BUILT_IN), by their library names.
BUILT_IN_MATCAPS=(
  "Clay/Clay Studio.png" "Clay/Basic Grey.png" "Glossy/Pearl.png" "Glossy/Resin.png"
  "Glossy/Ceramic Lightbulb.png" "Metal/Metal Carpaint.png" "Metal/Full Metal.png"
  "Diagnostic/Check Reflection Horizontal.png"
)
OUT="${3:-$HOME/Developer/Operandi-Content}"
CHECKED="${LICENCE_CHECKED:-2026-11-11}"
NAME="Operandi Library"

case "$OUT" in
  "$REPO"|"$REPO"/*) echo "output must be outside the repo: $OUT" >&2; exit 1 ;;
esac
[ -d "$STARTER" ] || { echo "no starter materials at $STARTER (run scripts/build-starter-pack.sh)" >&2; exit 1; }

cargo build --offline --release -p operandi-preset-convert --manifest-path "$REPO/Cargo.toml" >&2
TARGET="$(cargo metadata --offline --format-version 1 --no-deps --manifest-path "$REPO/Cargo.toml" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
CONVERT="$TARGET/release/operandi-preset-convert"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
PACK="$WORK/full/$NAME"
mkdir -p "$PACK/Starter" "$PACK/LICENSES"
cp "$REPO/assets/library/LICENSES/CC0-1.0.txt" "$PACK/LICENSES/"

# One folder per material: its preset and every picture next to it. A
# folder without <its name>.oppreset is a category holding materials.
copy_material() { # source folder, destination below Starter/
  local dir="$1" rel="$2"
  mkdir -p "$PACK/Starter/$rel"
  find "$dir" -maxdepth 1 -type f ! -name '.*' -exec cp {} "$PACK/Starter/$rel/" \;
}
for dir in "$STARTER"/*/; do
  name="$(basename "$dir")"
  if [ -f "$dir/$name.oppreset" ]; then
    copy_material "$dir" "$name"
    continue
  fi
  found=0
  for sub in "$dir"*/; do
    [ -d "$sub" ] || continue
    item="$(basename "$sub")"
    if [ -f "$sub/$item.oppreset" ]; then
      copy_material "$sub" "$name/$item"
      found=$((found + 1))
    else
      echo "skipping $name/$item: no $item.oppreset" >&2
    fi
  done
  [ "$found" -gt 0 ] || echo "skipping $name: no materials in it" >&2
done

# A picture folder: copied whole, minus licence proof and hidden files.
copy_pictures() { # source folder, folder name in the pack
  local src="$1" top="$2"
  [ -f "$src/CREDITS.txt" ] || { echo "no CREDITS.txt in $src" >&2; exit 4; }
  [ -f "$src/SOURCES.tsv" ] || { echo "no SOURCES.tsv in $src" >&2; exit 4; }
  mkdir -p "$PACK/$top"
  rsync -a --exclude '_download/' --exclude '.*' "$src/" "$PACK/$top/"
}

# MC-10: the matcaps the app doesn't already have.
if [ -d "$MATCAPS" ]; then
  copy_pictures "$MATCAPS" Matcaps
  for f in "${BUILT_IN_MATCAPS[@]}"; do rm -f "$PACK/Matcaps/$f"; done
  printf '\nIn this pack: the Blender matcaps the app already builds in (%s) are left out; find them in the view'"'"'s Matcap menu.\n' \
    "$(printf '%s, ' "${BUILT_IN_MATCAPS[@]}" | sed 's/, $//')" >> "$PACK/Matcaps/CREDITS.txt"
else
  echo "no matcaps at $MATCAPS; the pack has none" >&2
fi
if [ -d "$ENVIRONMENTS" ]; then
  copy_pictures "$ENVIRONMENTS" Environments
else
  echo "no environments at $ENVIRONMENTS; the pack has none" >&2
fi

if [ -d "$MESHES" ]; then
  mkdir -p "$PACK/Meshes"
  rsync -a --exclude '.*' --include '*/' --include '*.oppreset' --exclude '*' "$MESHES/" "$PACK/Meshes/"
  echo "Meshes: $(find "$PACK/Meshes" -name '*.oppreset' | wc -l | tr -d ' ') presets, $(du -sh "$PACK/Meshes" | cut -f1)" >&2
else
  echo "no meshes at $MESHES; the pack has none" >&2
fi

# The licence gate: exits 3 (and the script with it) on any bad item,
# preset or picture.
"$CONVERT" pack-manifest "$PACK" --name "$NAME" --version "$VERSION" --checked "$CHECKED"
rm -f "$PACK/Matcaps/SOURCES.tsv" "$PACK/Environments/SOURCES.tsv"
for top in Matcaps Environments; do
  [ -d "$PACK/$top" ] || continue
  printf '\n%s/ sources and licences are also in %s/CREDITS.txt.\n' "$top" "$top" >> "$PACK/CREDITS.txt"
  echo "$top: $(find "$PACK/$top" -type f \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.exr' -o -iname '*.hdr' \) | wc -l | tr -d ' ') pictures, $(du -sh "$PACK/$top" | cut -f1)" >&2
done

mkdir -p "$OUT"
ZIP="$OUT/Operandi-Library-$VERSION.zip"
rm -f "$ZIP"
(cd "$WORK/full" && zip -q -r -X "$ZIP" "$NAME" -x '*/.DS_Store')
echo "wrote $ZIP ($(du -h "$ZIP" | cut -f1))"

if [ -n "${LIBRARY_INSTALL:-}" ]; then
  mkdir -p "$LIBRARY_INSTALL"
  rm -rf "${LIBRARY_INSTALL:?}/$NAME"
  (cd "$LIBRARY_INSTALL" && unzip -q "$ZIP")
  echo "installed into $LIBRARY_INSTALL/$NAME"
fi
