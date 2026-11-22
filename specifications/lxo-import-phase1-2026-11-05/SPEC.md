# LXO import, phase 1 (modelling subset) — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05. REA-assisted: no.

Each statement below is tagged:
- **[doc]**: from the public page;
- **[obs]**: seen in the test files and confirmed against Modo's own
  readback;
- **[doc+obs]**: both.

## Summary

Phase 1 reads a Modo scene file (`.lxo`, file type LXOB) and builds the
modelling content:
- mesh items and their names;
- points, polygons and polygon types;
- material tags;
- UV maps;
- subdivision edge weights;
- item position;
- parenting;
- visibility.

Everything else in the file (cameras, lights, render and shader-tree items,
animation, action layers beyond what is listed here) is skipped without
failing.

## Container

- **Byte order.** All numbers are big-endian. [doc+obs]
- **File header.** `FORM`, a 4-byte size, then the 4-byte file type `LXOB`.
  In the test files the size equals file length − 8. [doc+obs]
- **Chunks.** Inside FORM, each chunk is: 4-character ID, 4-byte unsigned
  size, data. Odd sizes are followed by one pad byte that the size does not
  count. [doc+obs]
- **Sub-chunks** (inside ITEM and ACTN): 4-character ID, 2-byte unsigned
  size, data, same pad rule. [doc+obs]
- **Strings.** NUL-terminated, then padded with one more NUL if needed so
  the total length is even. [doc+obs]
- **Index values** (point and polygon references, "VX"):
  - 2 bytes when the first byte is not 0xFF.
  - Otherwise 4 bytes, with the top byte masked off.
  - Every index in the test files used the 2-byte form. [doc; 4-byte form
    not observed]
- **Unknown chunks** must be skipped using their size. The test files
  contain IASS (empty), ENVL, AANI and others that phase 1 ignores. [obs]

### Chunk order seen in Modo 17.1 files [obs]

1. File header chunks: VRSN, APPV, ENCO, IASS.
2. TAGS. Absent when no mesh has polygons (LF01).
3. CHNM.
4. Per mesh layer, in layer order: LAYR, PNTS, BBOX, two VMPA+VMAP pairs,
   POLS, then VMAD, PTAG (MATR then PART) and VMED where present.
   - VMAD came right after POLS (LF05); VMED after the PTAGs (LF06).
   - A mesh with no geometry has only its LAYR (LF01).
5. All ITEM chunks.
6. ENVL chunks.
7. Three ACTN chunks (edit, scene, setup).
8. AANI.

The page says a chunk must come before any chunk that refers to it, with
item links as the exception. The LAYR → ITEM link (below) points forward to
a later ITEM, so a reader must resolve item references after reading the
whole file. [doc+obs]

## Header chunks

- **VRSN:** two numbers and the saving application's name. Modo 17.1v1
  wrote 16, 0, "Modo by The Foundry". [obs]
- **APPV:** four numbers and a version string. Modo 17.1v1 wrote 17, 4001,
  1, a build number, "Modo 17.1v1". [obs]
- **ENCO:** text encoding. Code 2 means UTF-8 (the doc lists the codes);
  Modo 17.1v1 wrote 2. Phase 1 decodes strings as UTF-8. [doc+obs]

## Mesh layers

### LAYR [doc+obs]

Fields in order:
1. Layer index: 0, 1, 2 … in file order.
2. Flags (below).
3. Rotation pivot: three floats.
4. Name.
5. Legacy parent: always −1; ignore.
6. Subdivision refinement level: 2.0 in all files.
7. Curve refinement angle: 0.087266 rad in all files.
8. Scale pivot: three floats.
9. Six unused 4-byte values.
10. An **item reference**: the reference ID of the mesh ITEM that owns this
    layer.
11. A patch refinement level (2 bytes) and three spare 2-byte values.

Notes:
- **Linking layers to items.** Use the LAYR item reference. In every test
  file it matched the owning mesh ITEM's reference ID. The first field of
  the ITEM's own LAYR sub-chunk did not equal the layer index (0, 1 or 4
  for a file's first layer, 0xFFFFFFFF for LF03's second), so phase 1 must
  not rely on it. [obs]
- **Layer name.** Equals the mesh item's name. It is empty for an unnamed
  item (LF01). [obs]
- **Pivots.** Zero in all files. The page says point positions are relative
  to the layer pivot; phase 1 adds the rotation pivot (Q-LX-04). [doc]

**Layer flags** are bits [doc+obs]:

| Value | Meaning |
|---|---|
| 1 | visible |
| 2 | hidden |
| 4 | foreground (active layer) |
| 8 | background |

Observed:
- Visible active layer: 5.
- Visible inactive layers: 9.
- The hidden layer in LF08: 6 (hidden + active).

The ITEM's LAYR sub-chunk repeats the same flags in its second field, then
four colour bytes (200, 200, 200, 255 in all files). [obs]

### PNTS [doc+obs]

- Three floats per point; count = size / 12.
- **Coordinate conversion.** The file is left-handed (per the page; +Z
  points away). Modo's scene coordinates are the same values with Z
  negated.
  - Every test mesh matched Modo's own readback exactly after negating Z.
  - Example (LF02): the file stores point 0 as (−0.5, −0.5, 0.5); Modo
    reports (−0.5, −0.5, −0.5).

### BBOX

Min and max corners in file coordinates. Optional; phase 1 may ignore it.
[doc+obs]

### POLS [doc+obs]

- A 4-character polygon type, then the polygons.
- Each polygon: a 2-byte count whose top 6 bits are flags (mask with
  0x3FF), then that many point indices.
- Flags were 0 in every test file.

**Corner order.** The file lists corners in the opposite direction from
Modo. To get Modo's order, keep the first corner and reverse the rest:
- File `[0, 3, 2, 1]` → Modo `[0, 1, 2, 3]` (LF02).
- File `[7, 11, 10, 9, 8]` → Modo `[7, 8, 9, 10, 11]` (LF07).

Applied together with the Z negation, this matched Modo's polygons in every
test file, and facing is preserved. [obs]

**Polygon types:**

| ID | Modo type |
|---|---|
| FACE | face [doc+obs] |
| SUBD | Subdivs [doc+obs, LF06] |
| PSUB | Catmull-Clark [doc only] |

The page also lists curve, patch, text, line and one-point types; phase 1
skips them and reports how many it skipped (Q-LX-02).

A layer may have more than one POLS chunk (Q-LX-03). Polygon indices used
by PTAG and VMAD refer to the most recent POLS chunk. [doc]

### TAGS and PTAG [doc+obs]

- **TAGS** is a list of strings, indexed from 0.
- **PTAG** gives a tag type, then pairs of (polygon index into the most
  recent POLS, index into TAGS).
- **MATR** gives each polygon's material tag. LF04: polygons 1 and 4 →
  "Red", others → "Default". [obs]
- **PART** is also present in every file and gives the part name; phase 1
  keeps it as the polygon's part (Q-LX-05).
- **Resolve tags by index, never by string.** TAGS may hold the same string
  twice: in most files it is ["Default", "Default"], with PART pointing at
  one and MATR at the other. LF06 also had an unused "subdiv" entry. [obs]

### UV maps: VMAP and VMAD [doc+obs]

- **VMAP:** a type ID, a dimension (2 bytes), a name, then (point index,
  dimension floats) pairs.
- **VMAD:** the same per (point index, polygon index).
- **Corner UV rule.** For each polygon corner, use the VMAD value for
  (point, polygon) if there is one, otherwise the point's VMAP value.
  - This reproduced Modo's per-corner UVs for all 24 corners of LF05.
  - UV values are used as stored (no flip).
- **Empty "Texture" map.** Every mesh in the test files carries an empty
  TXUV map named "Texture", and Modo lists a "Texture" map on each. Phase 1
  creates the UV map even when it has no values. [obs]
- **Maps phase 1 skips:**
  - A VMAP of type BSPL named "B-Spline", dimension 1, always empty. Not in
    the page's type list.
  - The VMPA chunk before each VMAP. It held the two values 1 and 6 every
    time; meaning unknown. [obs]

### Edge weights: VMED [doc+obs]

- A type ID, a dimension, a name, then (point A, point B, value) entries.
- Type SUBV, name "Subdivision", dimension 1, holds subdivision edge
  weights. Endpoint order is not significant.
- LF06 stored edges 6–7 = 1.0, 4–7 = 0.25, 5–6 = 0.5 and 4–5 = 0.5,
  matching Modo's weights exactly.
- Edges not listed have weight 0.
- Modo lists a "Subdivision" weight map on every mesh even when no VMED
  is written. [obs]

## Items

### ITEM chunk [doc+obs]

- Fields: item type name, item name, a 4-byte reference ID unique in the
  file, then sub-chunks.
- **Display name:** the item name. When it is empty (LF01's default mesh),
  use the VNAM sub-chunk ("Mesh"). [obs]
- **Item types phase 1 uses:** `mesh`, `translation`, `rotation`, `scale`.
  Others are skipped (camera, sunLight, polyRender, environment,
  shaderFolder, advancedMaterial, defaultShader, renderOutput, materials,
  masks, …).
- **Other identity sub-chunks.** UNIQ holds an internal unique name; UIDX
  and VNAM are naming aids. Phase 1 only needs VNAM as the fallback.

### Channel values [doc+obs]

- **CHAN:**
  - Fields: an index into the CHNM name list, a 2-byte type, the value.
  - The low 4 bits of the type give the kind: 1 integer, 2 float, 3 string
    (a text-hint choice).
  - Modo 17.1 sets 0x20 on most mesh channels and leaves it clear on
    others; the meaning is unknown. Ignore the high bits. [obs]
- **CHNV (vector channel):**
  - Fields: base name, type (same rule), component count, then per
    component a name ("X", "Y", "Z") and its value.
- **CHNM:** a count followed by the channel-name strings. [doc+obs]

### Parenting [doc+obs]

- A LINK sub-chunk with graph name `parent` gives the parent item's
  reference ID and the child's position among its siblings.
- LF03: Child → Parent, index 0.
- Items with no such link are top-level.

### Transforms [doc+obs]

- **Separate items.** Position, rotation and scale are each their own item
  (types `translation`, `rotation`, `scale`).
- **Which item they move.** Each has a LINK in graph `xfrmCore` to the item
  it transforms, with an order index (scale 0, rotation 1, translation 2 in
  all files).
- **Values:**
  - Translation: a CHNV `pos`, metres.
  - Rotation: a CHNV `rot`, radians, plus a string channel `order` (`zxy`
    on every item seen).
  - Scale: a CHNV `scl`.
- **Not mirrored.** These values are stored as Modo shows them. LF03's
  Child position (1, 2, 3) is stored as 1, 2, 3 even though point
  coordinates are mirrored. [obs]
- **Source of the current value.** Use the transform item's own channel
  value. The same value may also appear in an ACTN layer: in LF03 the
  Child's position appears in the `edit` action and the camera's in
  `setup`. Phase 1 reads the item channels. [obs]
- **Untested cases.** Rotation and scale of a mesh were not tested, so
  whether they need a handedness conversion is Q-LX-01.

### Visibility [obs]

- The mesh channel `visible` (string) reads `default` normally and `off`
  for the hidden mesh (LF08).
- The layer flags agree: hidden bit set, visible bit clear.
- Phase 1 maps `off` to hidden. Other values are Q-LX-06.

## Results expected for the test files

`cases.json` lists, per file:
- each mesh's name, parent, position and visibility;
- points (Modo coordinates) and polygons (Modo order);
- polygon type, per-polygon material tag, UVs per corner, edge weights.

All values except LF08's geometry were checked against Modo's own readback.
LF08's geometry matches the build description (cubes at x 0, +2, −2 by
bounding box).
- **LF01:** default empty scene. One empty mesh named "Mesh"; no TAGS
  chunk.
- **LF02:** one cube "Box", 8 points, 6 faces, all "Default".
- **LF03:** "Parent" and "Child" cubes; Child parented to Parent, at
  (1, 2, 3).
- **LF04:** cube with "Red" on polygons 1 (+Y) and 4 (+Z).
- **LF05:** cube from the Cube primitive with UVs (VMAP + 7 VMAD entries).
- **LF06:** cube with Subdivs polygons (SUBD) and four edge weights.
- **LF07:** one triangle, one quad and one 5-gon in one mesh.
- **LF08:** three cube layers; "Hidden" has visible = off and layer flags 6.

## Refusals

- **Not an LXO:** reject a file that doesn't start with FORM … LXOB.
- **Truncated file:** reject a chunk size that runs past the end.
- The wording of these messages is Operandi's own; Modo's behavior on bad
  files was not observed here (Q-LX-07).

## Open questions

See `QUESTIONS.md`.

## Open questions and decisions

- **Q-LX-01** Rotation and scale of a mesh item: are `rot` and `scl` stored
  as Modo shows them, like `pos`, or do they need a handedness conversion
  to match the mirrored points? Needs a test file with a rotated and scaled
  mesh.
- **Q-LX-02** Polygon types other than FACE and SUBD: confirm PSUB for
  Catmull-Clark with a Modo-saved file; decide how to treat curves, patch,
  text, line and one-point polygons (skip and warn?).
- **Q-LX-03** Meshes with mixed polygon types: does Modo write one POLS
  chunk per type, and do the PTAG/VMAD polygon indices then restart per
  chunk? Needs a test file.
- **Q-LX-04** Non-zero layer pivots: are points stored relative to the
  pivot? Needs a test file with a moved pivot. What is the scale pivot
  field's role (it held 1, 0, 0 in LF02)?
- **Q-LX-05** PART tags: does Operandi have parts in phase 1? The files
  always tag every polygon with part "Default".
- **Q-LX-06** Values of the `visible` channel other than `default` and
  `off`, the `render` channel, and how a hidden parent affects its children.
- **Q-LX-07** How Modo reacts to corrupt or truncated files, and whether
  Operandi should load what it can or refuse the whole file.
- **Q-LX-08** The meaning of the ITEM LAYR sub-chunk's first field, the
  VMPA chunk (1, 6), the 0x20 bit in channel types, and the empty
  "B-Spline" map. None is needed for phase 1 as far as the tests show.
- **Q-LX-09** Large meshes: confirm the 4-byte index form with a file of
  more than 65,280 points.
- **Q-LX-10** Point selection sets, polygon selection sets (PICK), vertex
  normals, weight maps and morphs: phase 2?
- **Q-LX-11** Files from other Modo versions: phase 1 targets Modo 17.1
  output only. Earlier formats are covered separately.
- **Q-LX-12** (decision pending) Can the eight Modo-saved test files go to the build
  side as fixtures, or should Operandi build its own test files from this
  spec? They hold only our own simple geometry and Modo's default scene
  items.
