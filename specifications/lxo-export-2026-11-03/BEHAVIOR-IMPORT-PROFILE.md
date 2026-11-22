# Importing the LXE profile (answers LXE-IMP-Q01)

**Decision: the importer admits the LXE profile as a third producer
profile. The exporter stays minimal.** Adding Modo's gradient, envelope,
curve-profile or edit-layer records to exports would mean writing records
that were never tested in the stripped layout, and Modo crashes on layouts
it doesn't expect (LXE-00). Admitting a narrow, exactly defined profile on
the import side carries no such risk. Round-tripping Operandi's own files
is then a requirement of both specs together.

Facts below marked *observed* come from Modo 17.1v1 opening LXE files
(BEHAVIOR-WRITER.md and ORACLES.json). The admission rules themselves are
Operandi policy for its own files, not Modo behaviour.

## Recognizing the profile

**LXE-IMP-01 — A file is in the LXE profile when all of these hold:**

- VRSN (16, 0) and APPV words (17, 4001, 1, any build). The producer texts
  are free; don't rely on them.
- ENCO 2.
- The top-level chunks are only VRSN, APPV, ENCO, at most one CHNM, at most
  one TAGS, the LAYR/PNTS/VMAP/POLS/VMAD/PTAG geometry groups, ITEM chunks
  of type `mesh`, `translation`, `rotation` or `scale`, at most one bare
  `polyRender` ITEM (no subchunks; ignore it), and at most one ACTN,
  which must be named and typed `setup`.
- Every LAYR matches LXE-04 exactly (flags 5, zero pivot, parent −1, the
  refinement values, identity block and 22-byte tail).
- Mesh ITEMs carry only LAYR and at most one `parent` LINK; transform ITEMs
  carry only one `xfrmCore` LINK each, with the ordinals in LXE-11.

A file that matches some of this but not all is not in the LXE profile; the
existing Modo 16/17.0 profile rules apply to it unchanged (and will usually
refuse it). Never mix rules from the two profiles in one file.

## Meaning of what is present or absent

**LXE-IMP-02 — The setup action is the only source of channel values.**
Read pos/rot/scl, `type`, `order` and `visible` from the ACTN `setup`
records for each item. There is no ITEM-versus-action agreement check in
this profile, because ITEMs carry no channel values. Each channel appears at
most once per item; a repeat refuses the file.

**LXE-IMP-03 — Missing pieces have fixed meanings** (*observed* in Modo
17.1v1 unless noted):

| Absent | Meaning |
|---|---|
| All three transform ITEMs of a mesh | Identity transform (*observed*, LXE-O-02) |
| A pos/rot/scl component in setup | 0 for pos and rot, 1 for scl (Operandi rule; the exporter always writes all three) |
| `order` on a rotation | Refuse: the exporter always writes `zxy` |
| `visible` on a mesh | Visible (*observed*: reads "default") |
| `parent` LINK | Top-level item |
| Any VMAP for a mesh | No UV maps |
| VMAD records | Every corner takes its point's VMAP value |
| PTAG MATR / PART for a polygon | No material / no part label (Modo displays "Default" and no part; keep Operandi's "no label" distinct, as the import spec already requires) |
| CHNM | Only legal when there is no ACTN |
| TAGS | Only legal when there is no PTAG |

**LXE-IMP-04 — Ordering rules the importer may rely on:** VMAD follows its
layer's POLS; PTAG follows POLS; all ITEMs follow all geometry; the single
ACTN is last. Item ids are unique but need not be contiguous or follow layer
order (LXE-O-12).

**LXE-IMP-05 — Coordinate rules are the import spec's:** negate Z of PNTS,
reverse each polygon's corners keeping the first, carry corner attributes
with their corners; transform values are Modo XYZ (not negated), radians,
ZXY.

## Round trip

**LXE-IMP-06 — Exporting then importing gives back the same scene.** For
every LXE oracle's input scene: export, import, and compare names, points,
polygon corner order, transforms, parents, visibility, UV per polygon corner
and material/part labels with the original, exactly (tolerance 1e-6 for
floats). Byte equality of a second export is also expected, since the
exporter's output is fully determined by the scene; check it.

## Not decided here

Files that Modo 17.1v1 itself saved (including an LXE file opened and saved
again by Modo, LXE-O-RT-07/09) are a different, much larger profile:
Modo 17.1's full records, the APPV words (17, 4001, 1, build) and an extra
PAKG subchunk on items. Admitting those needs its own corpus check against
the import spec's rules. *(open: LXE-Q10)*
