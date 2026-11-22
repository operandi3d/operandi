#!/usr/bin/env bash
# Build the CC0 starter material pack from the local test library.
#
#   scripts/build-starter-pack.sh [source] [output]
#   scripts/build-starter-pack.sh --picks <sources.tsv> [output]
#
# source: ~/Developer/Operandi-Content/Test-Library (Poly Haven and ambientCG,
#         all CC0; see its CREDITS.txt)
# output: ~/Developer/Operandi-Content/Starter (never inside the repo: the
#         pictures are big)
#
# Each material gets its own folder: <Name>/<Name>.oppreset with its maps
# copied next to it (paths relative to the preset). ambientCG assets ship a
# .blend, which tools/blender-export/export.py turns into a node graph JSON
# first (Blender runs headless; it's a separate GPL-2.0-or-later tool, never
# linked into Operandi). Poly Haven folders are converted from their maps
# alone. CREDITS.txt lists every material with its source, licence and the
# date its licence was checked. Refused materials are listed, not written.
#
# --picks converts the downloads scripts/fetch-library-materials.py lists
# in its sources.tsv into <output>/<Category>/<Name>/ instead, and adds
# them (and any refused ones) to <output>/CREDITS.txt.
set -euo pipefail

PICKS=""
if [ "${1:-}" = --picks ]; then
  PICKS="$2"; shift 2
  [ -f "$PICKS" ] || { echo "no picks file at $PICKS" >&2; exit 1; }
  set -- "" "$@"
fi

REPO="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"; SRC="${SRC:-$HOME/Developer/Operandi-Content/Test-Library}"
OUT="${2:-$HOME/Developer/Operandi-Content/Starter}"
BLENDER="${BLENDER:-/Applications/Blender.app/Contents/MacOS/Blender}"
CHECKED="${LICENCE_CHECKED:-2026-11-11}"

cargo build --offline --release -p operandi-preset-convert --manifest-path "$REPO/Cargo.toml" >&2
TARGET="$(cargo metadata --offline --format-version 1 --no-deps --manifest-path "$REPO/Cargo.toml" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
CONVERT="$TARGET/release/operandi-preset-convert"

# Poly Haven picks (fabric_pattern_07 has no colour map in the library;
# the aerial sets are left out: they're scanned from
# above at landscape scale).
POLYHAVEN=(asphalt_02 bark_brown_02 beige_wall_001 brown_mud_leaves_01
  coast_sand_rocks_02 concrete_floor_worn_001 factory_wall
  laminate_floor_02 metal_plate plywood red_brick rusty_metal_04)

mkdir -p "$OUT"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
CREDITS="$WORK/CREDITS.txt"
REFUSED="$WORK/REFUSED.txt"
: > "$REFUSED"
{
  echo "CREDITS - Operandi starter materials"
  echo "Every material below is CC0 1.0 Universal (public domain): no attribution required."
  echo "Licence pages checked $CHECKED: https://polyhaven.com/license and https://docs.ambientcg.com/license/"
  echo "Thumbnails are each material's own base colour map (no site renders are included)."
  echo
  echo "name | source URL | licence | credit | date checked"
} > "$CREDITS"

converted=0
convert_one() { # name folder url credit [graph] [category]
  local name="$1" folder="$2" url="$3" credit="$4" graph="${5:-}" category="${6:-}"
  local dest="$OUT/${category:+$category/}$name"
  local stage="$WORK/stage-$name"
  rm -rf "$stage"; mkdir -p "$stage"
  local args=("$folder" --out "$stage/$name.oppreset" --copy-maps
    --licence CC0-1.0 --url "$url" --credit "$credit")
  [ -n "$graph" ] && args+=(--graph "$graph")
  local log status=0
  log="$("$CONVERT" "${args[@]}" 2>&1 </dev/null)" || status=$?
  if [ "$status" -eq 0 ]; then
    rm -rf "$dest"; mkdir -p "$(dirname "$dest")"; mv "$stage" "$dest"
    echo "$name | $url | CC0-1.0 | $credit | $CHECKED" >> "$CREDITS"
    converted=$((converted + 1))
    echo "$log"
  else
    echo "$name: $log" | tee -a "$REFUSED" >&2
  fi
}

# ambientCG's .blend -> node graph JSON in $WORK/<name>.json (GRAPH is ""
# without one).
blend_graph() { # name folder
  local blend
  blend="$(ls "$2"/*.blend 2>/dev/null | head -1 || true)"
  GRAPH=""
  { [ -n "$blend" ] && [ -x "$BLENDER" ]; } || return 0
  GRAPH="$WORK/$1.json"
  "$BLENDER" -b "$blend" -P "$REPO/tools/blender-export/export.py" -- "$GRAPH" \
    --licence CC0-1.0 --url "https://ambientcg.com/a/$1" --credit "ambientCG" </dev/null >/dev/null 2>&1
}

if [ -n "$PICKS" ]; then
  # Keep the credits already there, minus their old "Not converted" list.
  if [ -f "$OUT/CREDITS.txt" ]; then
    awk '/^Not converted:/{exit} NF{for(;b>0;b--)print "";print;next}{b++}' "$OUT/CREDITS.txt" > "$CREDITS"
  fi
  while IFS=$'\t' read -r category site name url _licence fetched folder; do
    [ "$category" = category ] && continue
    graph=""
    credit="Poly Haven"
    if [ "$site" = ambientCG ]; then
      blend_graph "$name" "$folder" || { echo "$name: Blender export failed" >> "$REFUSED"; continue; }
      graph="$GRAPH"
      credit="ambientCG"
    fi
    CHECKED="$fetched" convert_one "$name" "$folder" "$url" "$credit" "$graph" "$category"
  done < "$PICKS"
  if [ -s "$REFUSED" ]; then
    { echo; echo "Not converted:"; cat "$REFUSED"; } >> "$CREDITS"
  fi
  cp "$CREDITS" "$OUT/CREDITS.txt"
  echo "converted $converted materials into $OUT ($(wc -l < "$REFUSED" | tr -d ' ') not converted)"
  exit 0
fi

for d in "$SRC"/ambientCG/*/; do
  name="$(basename "$d")"
  [ "$name" = HDRIs ] && continue
  blend="$(ls "$d"*.blend 2>/dev/null | head -1 || true)"
  graph=""
  # MX-08: ambientCG's .mtlx is preferred; the converter picks it up, so
  # Blender is only needed for folders without one.
  if ls "$d"*.mtlx >/dev/null 2>&1; then
    blend=""
  fi
  if [ -n "$blend" ] && [ -x "$BLENDER" ]; then
    graph="$WORK/$name.json"
    "$BLENDER" -b "$blend" -P "$REPO/tools/blender-export/export.py" -- "$graph" \
      --licence CC0-1.0 --url "https://ambientcg.com/a/$name" --credit "ambientCG" >/dev/null 2>&1 \
      || { echo "$name: Blender export failed" >> "$REFUSED"; continue; }
  fi
  convert_one "$name" "$d" "https://ambientcg.com/a/$name" "ambientCG" "$graph"
done

for name in "${POLYHAVEN[@]}"; do
  convert_one "$name" "$SRC/PolyHaven/Textures/$name" "https://polyhaven.com/a/$name" "Poly Haven"
done

if [ -s "$REFUSED" ]; then
  { echo; echo "Not converted:"; cat "$REFUSED"; } >> "$CREDITS"
fi
cp "$CREDITS" "$OUT/CREDITS.txt"
echo "converted $converted materials into $OUT ($(wc -l < "$REFUSED" | tr -d ' ') not converted)"
