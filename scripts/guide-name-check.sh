#!/bin/sh
# Clean-room name check for the User Guide (docs/guide/RULES.md, "Clean
# room"): the guide never names other 3D applications or their makers.
# Scans every text file under the given folders (default docs/guide),
# skipping build output, and prints each hit with its file and line.
# Exit 1 on any hit.
#
#   scripts/guide-name-check.sh [folder ...]
set -eu
cd "$(dirname "$0")/.."
[ $# -gt 0 ] || set -- docs/guide

# Whole words, case-insensitive. Add a name here when a new one turns up.
NAMES='modo|foundry|luxology|blender|maya|autodesk|3ds ?max|cinema ?4d|maxon|houdini|sidefx|zbrush|pixologic|maxon|lightwave|newtek|sketchup|rhino(ceros)?|wings ?3d|plasticity|nuke|katana|mari|substance (painter|designer)|marvelous designer|unreal engine|softimage|metasequoia|hexagon|silo 3d'

HITS=$(grep -rniE --binary-files=without-match \
    --exclude-dir=book --exclude='*.png' --exclude='*.opscene' \
    "(^|[^[:alnum:]_])($NAMES)([^[:alnum:]_]|$)" "$@" || true)
if [ -n "$HITS" ]; then
    echo "Clean-room name check failed: the guide names another application or its maker." >&2
    echo "$HITS" >&2
    exit 1
fi
echo "Clean-room name check passed ($*)."
