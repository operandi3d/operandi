# LXO export: what a writer must produce for Modo 17.1v1

Observed in Modo 17.1v1 on 2026-11-03 by opening generated test files and
reading every fact back by script (six decimals). REA-assisted: no. The
encoding of each chunk follows the import spec
(`lxo-import-phase1-2026-11-05`, a clean re-derivation that replaces the removed phase-1 spec); this document says
which pieces an exporter must write, in what order and with what values, so
that Modo opens the file with the intended meaning. Coordinates are Modo XYZ
in metres unless a line says "file space".

## Safety rule first

**LXE-00 — Modo 17.1v1 does not refuse a bad file; it crashes.** Every
malformed or under-specified candidate in this probe either opened correctly
or terminated Modo without any message. An exporter must therefore write only
the verified layout below and must never emit an experimental variant to a
user's file. Known crash triggers: a file without a render item whose APPV is
missing or older than LXE-02's value (fixed by LXE-16), a VMAD chunk placed
before its layer's POLS, and a LAYR payload whose tail deviates from LXE-04.

## File header

**LXE-01 — The file is one FORM of type LXOB.** `FORM`, U32 length
(= file size − 8), `LXOB`, then the top-level chunks. No trailing bytes.

**LXE-02 — Write VRSN, APPV and ENCO first, in that order.**

| Chunk | Payload to write | Verified |
|---|---|---|
| VRSN | U32 16, U32 0, S0 producer text | Any producer text works (e.g. "Operandi") |
| APPV | U32 17, U32 4001, U32 1, U32 build, S0 producer text | **Required.** Second word 4001 opens; 4000 or a missing APPV crashes a stripped file. Build word is free (0 works). Producer text is free |
| ENCO | U32 2 (UTF-8) | All strings are then UTF-8 |

A full scene saved by Modo still opens with APPV second word 4000, so the
crash is specific to the stripped layout this spec uses. Keep the stripped
layout and the 4001 value together. *(open: LXE-Q01, other Modo versions)*

**LXE-03 — Chunk order after the header:** CHNM (if any channel is
written), TAGS (if any polygon tag is written), then for each mesh in turn
its geometry group (LXE-04 to LXE-08), then all ITEM chunks (LXE-09 to
LXE-11 and the render item of LXE-16), then one ACTN `setup` (LXE-12).
Nothing else is required.

## Geometry, one group per mesh

**LXE-04 — LAYR opens each mesh's group.** Payload:

| Offset | Value to write |
|---|---|
| 0 | U16 layer ordinal (0, 1, 2… in file order) |
| 2 | U16 flags = 5 (the value Modo writes for an ordinary mesh) |
| 4 | F32 × 3 = 0, 0, 0 (legacy pivot) |
| 16 | S0 mesh name (may be empty) |
| k | I16 −1 (legacy parent; hierarchy comes from LXE-10) |
| k+2 | F32 2.0, then F32 0.08726646 (bytes `3db2b8c2`) |
| k+10 | F32 × 9 identity: 1,0,0, 0,1,0, 0,0,1 |
| k+46 | U32 mesh ITEM id (LXE-09) |
| k+50 | 22 constant bytes `00100000000000000001000200020002000100010000` |

These are the exact bytes Modo 17.1v1 writes for an ordinary mesh; a
generated LAYR was byte-identical to Modo's own for the same mesh. A tail
with different bytes crashed Modo.

**LXE-05 — PNTS holds the points in file space.** F32 triples, no count.
File space negates Z: write (x, y, −z).

**LXE-06 — POLS holds the polygons as FACE.** `FACE`, then per polygon U16
corner count (3…1023, no flag bits) and that many VX point indices. Because
file space is mirrored, write each polygon's corners **reversed, keeping the
first corner**: Modo corners [a, b, c, d] are written [a, d, c, b]. Modo then
reads back the original order exactly (LXE-O-02, LXE-O-09). Indices of 65280
and above use the long VX form; 66,049 points and 65,536 quads load exactly
(LXE-O-14).

**LXE-07 — UV maps: VMAP before or after POLS, VMAD only after POLS.**
For each UV map: VMAP `TXUV`, U16 2, S0 map name, then one record per point
(VX point, F32 u, F32 v) for the map's continuous value. For corners that
differ from their point's value (seams), VMAD `TXUV`, U16 2, S0 same name,
then records (VX point, VX polygon, F32 u, F32 v). VMAD must come **after**
the layer's POLS; placed before it, Modo crashed. Several maps per mesh
work; values outside 0…1 are kept. U and V are written as they are (no flip,
and no change for the mirrored file space). No VMPA chunk is needed. A mesh
written without any VMAP has no UV map in Modo (Modo adds only its own
empty "Subdivision" weight map).

**LXE-08 — Material and part labels: TAGS once, PTAG per mesh.** TAGS is the
file's list of label strings (S0 each, no count). After a mesh's POLS, write
PTAG `MATR` and PTAG `PART`, each a list of (VX polygon, U16 index into
TAGS). Modo shows the labels per polygon exactly. It creates **no** material
or mask items for them; a polygon without a MATR entry reads as "Default",
without a PART entry as no part.

## Items

**LXE-09 — One `mesh` ITEM per layer.** ITEM payload: S0 `mesh`, S0 name,
U32 id, then subchunk LAYR: U32 layer ordinal, U32 0, four colour bytes
(`c8 c8 c8 ff` used). Ids are file-local, need not be contiguous or follow
layer order, and ITEM chunks may come in any order (LXE-O-12).

**LXE-10 — Parenting is a `parent` LINK on the child.** Add subchunk LINK:
S0 `parent`, I32 parent item id, I32 0. Modo reads the hierarchy and world
positions exactly (LXE-O-07).

**LXE-11 — Non-identity transforms use three transform ITEMs.** For a mesh
with id M, write ITEM `scale`, `rotation` and `translation` (empty names,
own ids), each with subchunk LINK: S0 `xfrmCore`, I32 M, I32 ordinal
(scale 0, rotation 1, translation 2). Their values go in the setup action
(LXE-12), not in the ITEM. A mesh with identity transform may omit all
three; Modo adds them itself.

**LXE-12 — Channel values go in one ACTN `setup` layer.** ACTN payload:
S0 `setup`, S0 `setup`, U32 id, then for each item whose channels are set:
subchunk ITEM (U32 item id) followed by CHAN subchunks (VX index into CHNM,
U16 type, VX 0, value). Types: 2 = F32, 3 = S0. CHNM lists every channel name
used. Write:

| Item | Channels |
|---|---|
| translation | `type` = "core" (S0); `pos.X`, `pos.Y`, `pos.Z` (F32, metres, Modo XYZ: **not** negated) |
| rotation | `type` = "core"; `order` = "zxy" (S0); `rot.X`, `rot.Y`, `rot.Z` (F32 radians) |
| scale | `type` = "core"; `scl.X`, `scl.Y`, `scl.Z` (F32) |
| hidden mesh | `visible` = "off" (S0) |

With this, Modo reads position (3, −2, 5), rotation (30°, 45°, 60°) ZXY and
scale (1, 2, 0.5) exactly, and the world points match Modo's own save of the
same scene to six decimals (LXE-O-05). Modo's own files also carry `edit`
and `scene` action layers; they are not needed. Putting the values in an
`edit` layer alone is **not** acceptable: the world positions are right but
the item's channels read back as zero (LXE-O-05c).

**LXE-16 — Also write one bare render item (v3).** After the mesh and
transform ITEMs, write ITEM: S0 `polyRender`, S0 "" (empty name), U32 id,
and no subchunks. Modo fills in every render setting itself and names it
"Render". This is what makes the file safe when the declared version is
older than the running Modo: a stripped file declaring APPV (17, 4000, …)
crashed Modo without this item and opened correctly with it (LXE-O-15), and
the same holds for format 4.x files (see the legacy import spec). Keep
writing the APPV of LXE-02 as well (LXE-O-16).

## What Modo fills in

**LXE-13 — Everything else is recreated by Modo on open.** A file with only
the pieces above opens with Modo's own Scene, Environment, Render and the
five standard shader folders (Bake Items, Environments, Library, Lights,
Nodes). There is no camera, light, Base Material or Base Shader unless the
file has them. Leaving out camera, light, render, shading, scene, envelope,
item metadata and bounding-box data did not change any modelling fact.

**LXE-14 — Names.** An empty mesh name displays as "Mesh". Unicode names
are kept exactly. Two meshes with the same name display as "Dup" and
"Dup (2)" (the second in layer order gets the suffix); the stored names are
whatever the file says.

**LXE-15 — Modo keeps everything when it saves the file again.** The
parent/child/hidden file and the two-map/seam/tag file were opened, saved by
Modo and reopened: every point, polygon, transform, parent, visibility, UV
and label was unchanged (LXE-O-RT-07, LXE-O-RT-09).

## Not covered (see QUESTIONS.md)

Other Modo versions (16.x, 17.0) opening these files; pivots and centre
offsets; other rotation orders; instances; SUBD/PSUB polygons; weight,
colour, normal, morph and selection-set maps; subdivision settings; empty
meshes; LXL presets.
