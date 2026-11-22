# Importing Modo 8–10 era LXO files (format 4.0 and 4.1)

Observed on 2026-11-03 from the structure of 102 sample scenes shipped for
Modo 8–10, from Modo 17.1v1 opening those scenes and reading every mesh back
(six decimals), and from original test files written in the old layout and
opened in Modo 17.1v1. REA-assisted: no. Everything not mentioned here
follows the import spec `lxo-import-phase1-2026-11-05` (clean re-derivation;
replaces the removed phase-1 spec) unchanged; this document only lists what differs for the older files.

## Which files

**LEG-01 — Admit format versions 4.0 and 4.1; refuse 2.x and 3.x with a
specific message.** VRSN (4, 0) and (4, 1) cover 98 of the 102 samples. The
four others are (2, 1), (3, 0) and (3, 1); refuse them with "made with Modo
older than 8; not supported yet" rather than the generic version error.

**LEG-02 — APPV is optional and its words vary.** Observed APPV words:
(8, 800, 0, build), (9, 900, 0, build), (10, 1001 / 1010 / 1011 / 1020, 0,
build). Eleven 4.0 files have no APPV at all; some of these, and a few
others, carry a STAG chunk (an S0 string such as "APPV800.0.0") right after
ENCO. Treat APPV and STAG as information only; never use them to accept or
refuse.

**LEG-03 — Encoding.** 96 files have ENCO 2 (UTF-8); one has ENCO 1. Keep
the import spec's strict rule: only ENCO 2 is accepted; refuse the other
with the existing encoding message. *(open: LEG-Q04)*

## Layer records

**LEG-04 — LAYR has the same layout up to the item reference; only the bytes
after it differ.** Up to offset k+50 (k = end of the name) the fields are as
in the import spec, and the mesh item id is at k+46 in every observed file.
The bytes after the item id (the "rest") come in these observed forms:

| Rest length | Observed bytes (hex) | Count |
|---|---|---|
| 18 | `001000000000000000010002000200020001` | 74 |
| 18 | `001000000000000000010002000200050001`, `…010003000300030001` | 3 |
| 20 | `0010000000000000000100020002000200010001` | 789 |
| 20 | same with `0004` in place of either middle `0002` | 21 |
| 24 | `001000000000000000010002000000000002000200020001` (and one variant with `0000` for the first `0001`) | 46 |
| 70 | starts `00100000000000000000000200000000000200020002000100000000` followed by a layer label string and settings | 3 |

Their meaning is not established. Accept these rest forms without
interpreting them. Also accept any value in the legacy parent field
(observed −1, 0, 10, 36) and a first refinement value of 2.0 or 5.0;
neither affects geometry. **Refuse** a layer whose LAYR is far longer than
these (observed 50 KB to 45 MB): it carries sculpt or multiresolution data.

## Polygons and maps

**LEG-05 — The polygon corner count can need 11 bits.** A FACE polygon with
1,442 corners exists in the samples; Modo reads it with exactly 1,442
corners. Read the count from the low 11 bits of the U16 and require the
top five bits to be zero. (An original 1,100-corner polygon also loads
exactly: LEG-O-02.)

**LEG-06 — SUBD and PSUB polygons import as plain polygons for now
(Jason's decision, 2026-11-03).** Read SUBD and PSUB blocks with the FACE
record layout (they share it) and import their polygons as ordinary
polygons with the same points and corner order: the control cage. Keep the
original type ("subdivision" or "Catmull-Clark") on each polygon so a later
subdivision feature can use it, and show one note per file such as "N
subdivision polygons were imported as their control cage; the smooth surface
is not shown yet". Subdivision weights (SUBV maps) and edge creases are not
applied; keep them only if Operandi can store them, otherwise list them in
the same note. The same rule is recommended for the 16/17 profile, which
currently refuses these polygons. Curve types (HCRV, BCRV, BSPL, LINE)
still follow the import spec's refusal. TXUV VMAP/VMAD and MATR/PART PTAG
decode exactly as in the import spec.

**LEG-07 — Don't invent maps Modo adds by itself.** When Modo 17.1 opens an
old file, it adds an empty "Texture" UV map to meshes that have none
(LEG-O-01, the child mesh). That is Modo's own behaviour on load; the file
has no such map, and the importer must not create one.

## Channels and transforms

**LEG-08 — Unflagged channel types mean the same as flagged ones.** Old
files use CHAN types 0x01 / 0x02 / 0x03 alongside 0x21 / 0x22 / 0x23, and
CHNV type 0x02 alongside 0x22. For static values, treat each pair as the
same type. Types with the 0x10 bit (0x11, 0x12, 0x13, 0x212) are
envelope-driven, as in the import spec, and refuse the dependent group.

**LEG-09 — Transform values come from the action layers, not from the
transform ITEM.** Use the `edit` action's value if present, otherwise the
`setup` action's value. The values stored in the transform ITEM (its CHNV)
are not reliable in these files: 1,206 of 34,498 transform values differ
from the setup action, and Modo always used the action values (LEG-O-01,
where the ITEM holds a deliberately wrong position, and six sample meshes).
For this profile, replace the import spec's ITEM-versus-action agreement
check with this precedence. A channel present in neither action layer
takes the ITEM value; if that is missing too, the default (0 for position
and rotation, 1 for scale, ZXY order).

**LEG-10 — The same transform-stack limits apply.** Import a mesh only when
its placement uses at most one translation, one rotation and one scale item
(xfrmCore ordinals as in the import spec) and its parents do the same. In
the samples, meshes outside this (pivot `transform` items or `xfrmLocal`
links, two scale or three translation items, constraint links (`chanMods`),
deformers, or animated channels) were exactly the ones whose world
positions the simple rule did not reproduce. Refuse those meshes with the
existing reasons.

## Data that isn't modelling

**LEG-11 — Skip these top-level chunks with an information note:**

| Chunk | What it is in these files |
|---|---|
| BAKE, and the VVEC, 3GRP, 3SRF, VRTS chunks that follow it | Cached dynamics and particle results (per-frame matrices, particle positions, velocities) |
| DATA | Index-range records for items (e.g. "+intrange" lists) |
| STAG | Application version string (LEG-02) |

None of them changes a mesh's points, polygons, maps or tags.

**LEG-12 — Items.** Rigs, schematic nodes, procedural and channel-modifier
items are common (schematic nodes alone number about 17,000 in the
samples). Handle them with the import spec's support matrix: omit
non-modelling items with a note; refuse a mesh only when its own placement
or geometry depends on them (LEG-10). Mesh instances follow the import
spec's instance policy.

## Answers to the build side (v3)

**LEG-13 — Mesh channels and curve gradients (LEG-IMPL-Q01): leaving them out
is right.** The 4.x mesh items carry render, display and curve settings,
none of which changes a FACE/SUBD/PSUB mesh's points, polygons, maps or
tags (every "matches" row in LOCAL-CORPUS.md was reproduced without reading
them). Read only `visible` from a 4.x mesh item, and ignore its other
channels without a channel profile. The radius and rotation gradients (GRAD
records and the envelopes they point to) shape generated curves only; with
curve polygons refused (LEG-06), skip them silently. If curve support is
added later, revisit this.

**LEG-14 — Bad geometry refuses the mesh, not the file (LEG-IMPL-Q02).** In
the 4.x samples, 22 of 920 meshes (in 14 of 98 files) contain edges shared
by more than two polygons (30,803 such edges in total), zero-area polygons
(530) or polygons that repeat a point (81). Modo opens all of them. Don't
repair them silently, and don't refuse the whole file: refuse only the
affected mesh, import everything else, and list each refused mesh with its
reason and counts, for example: "Mesh 'Rock' skipped: 12 edges
shared by more than two polygons". A later option may offer "import anyway,
leaving out N bad polygons"; that's a product decision, not covered here.

## Writing these files

Not in scope. Note for the record: an old-format file without a render item
crashed Modo 17.1v1; with a bare `polyRender` item it opened (LEG-O-03).
