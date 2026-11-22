# Action Center × Symmetry rulebook (ACS-xx) — v1.2 (final for building)

Behaviour only, for one shared pivot service. Recorded 2026-11-19 in the
reference app: 234 scripted transform cases with a typed centre, 48 scripted
cases for the other tools, and 29 hands-on cases (real key presses, clicks and
drags). REA-assisted: no. Builds on the earlier Action Center specs (ACT-xx,
ACL-xx) and the Symmetry spec (SYM3-xx); where they differ, this file is newer.

Tags: **[seen]** observed by hand in the UI; **[typed]** observed with values
typed into the tool (no drag); **[script]** observed only in scripted runs;
**[open]** not settled.

## 1. The pivot service

One service answers, for the active tool:

- **inputs**: Action Center mode, the selection (with its mesh item's
  transform), the Symmetry setting (axis, offset), the last viewport click
  (if the mode uses one), the item hierarchy.
- **outputs**: a list of *pivots*. Each pivot is a centre point, an axis frame
  (three directions), the set of points it drives, and a flag *primary* or
  *mirrored*. Without Symmetry there is one pivot (or one per island under
  Local). With Symmetry there is a primary pivot and its mirror image.

Every transform tool (Move, Rotate, Scale, Transform, Stretch) asks the service
and applies its values once per pivot. Tools in section 5 don't use it.

## 2. Rulebook table

"Centre" is where the handle appears and the pivot is. "Click" says whether a
viewport click changes it. With Symmetry on, the rules of section 3 apply on top
of every row.

| Mode | Centre | Axis | Click? | What a click sets | With Symmetry on |
|---|---|---|---|---|---|
| None | Like Automatic | World | Yes | Same as Automatic | Same as Automatic **[seen]** |
| Automatic | Bounding-box centre of the selection (no selection: of all active geometry) | World (Y up) | Yes, optional | X and Z (in Top view: the two screen axes) from the click on the work plane; the third coordinate stays at the selection centre's height | Before a click: centre of the + half (ACS-01). After a click: the click point, unmirrored; its side becomes the primary side **[seen]** |
| Selection | Bounding-box centre of the selected points (not the vertex average) **[seen]** | From the selection's normals (custom frame) | No | — | Centre of the + half; axis still from the whole selection **[seen]** |
| Selection Center Auto Axis | As Selection | World | No | — (a click doesn't move it, earlier spec) | Centre of the + half **[seen]** |
| Selection Border | Border centre (earlier spec: projected border centre) | From the selection | No | — | Centre of the + half's border **[seen]** (whole quads, and a full top row of a segmented cube) |
| Element | Centre of the element clicked | Normal of the element clicked | Yes, needed | Centre and axis from the clicked polygon/edge/vertex; it need not be selected | The clicked element's side becomes the primary side; a click on a − side element gives a − side pivot **[seen]** |
| Screen | Like Automatic | Screen frame | Yes | Same as Automatic; not snapped (results differ from Automatic by about 1 mm on the same click) | Same as Automatic **[seen]** |
| Origin | World origin | World | No | — | On the plane: the − side is primary (ACS-03) **[seen]** |
| Parent | The parent item's pivot position | Parent's axes | No | — | Same rules; tested only with the pivot on the plane **[seen]** |
| Local | Per island: the island's bounding-box centre **[seen]** | Per island: Y = island normal, X = principal spread axis nearest world X (ACL-02) | No | — | Each island keeps its own pivot; mirrored islands mirror automatically **[seen]** |
| Pivot | The item's pivot | Item axes | No | — | The item pivot sits on the item's own plane, so the − side is primary **[seen]** |
| Pivot Center Parent Axis | The item's pivot | Parent's axes | No | — | As Pivot **[open]** (not separately recorded) |

The Action Center menu also lets Center and Axis be chosen separately; each
preset above is just one centre rule plus one axis rule. The service should
keep them as two independent parts.

## 3. Symmetry rules

**ACS-01 Half selection for the centre.** With Symmetry on, every
selection-based centre (Automatic before a click, Selection, Selection Center
Auto Axis, Selection Border) is computed from the selected points on the
**+ side** of the plane only. Example: the mirrored pair of quads gives the
centre of the + quad (1.5, 0.25, 0.5), not (0, 0.25, 0.5) **[seen]**. With a
straddling quad also selected, the + half's points (the + quad plus the
straddling quad's + corners) give the box centre (1.25, 0.25, 1.5) **[seen]**. Selection Border follows the same
rule: with the whole top row of a cube segmented in X selected, the Rotate
handle sat on the + half's border, at its outer edge **[seen]** (Jason,
2026-11-19).
It holds for X, Y and Z **[seen]**. If nothing selected is on the + side, the
− side is used (only the − quad selected gives (−1.5, 0.25, 0.5)) **[seen]**.
Selection order and the pointer position when the tool starts don't matter
**[seen]**.

**ACS-02 Axis from the whole selection.** The axis frame of a
selection-based mode still comes from the whole selection, not the half:
the + quad plus the straddling quad gave the same axis with and without
Symmetry **[seen]**.

**ACS-03 Primary side.** The *primary side* is the side of the plane the
centre is on. A centre exactly on the plane counts as the **− side**.
Typed values (and the drag) apply as entered to selected points on the
primary side **[typed]**, 234/234 cases.

**ACS-04 Mirrored pivot.** Selected points on the other side are transformed
by the mirror image: the pivot is mirrored across the plane, and so is the
axis frame; a move's component along the mirror axis flips, rotations turn the
other way about the mirrored axes, scale factors are the same. Equivalently:
mirror the point, apply the primary transform, mirror back **[typed]**.
Example (X symmetry, centre typed at (−1.5, 0.25, 0.5), Scale X 2): the
− quad scales about x = −1.5, the + quad about x = +1.5, so both results are
the same as with the centre at +1.5 **[typed]**.

**ACS-05 Seam points.** A selected point on the plane takes the primary
side's transform, then its coordinate along the mirror axis is put back on
the plane **[typed]**.

**ACS-06 One-sided selections.** Only selected points move. Symmetry doesn't
add partners at transform time (ACS-03/04 decide only *how* each selected
point moves). Example: only the + quad selected and the centre on the plane:
the + quad gets the mirrored edit (moves −0.25 in X for Move X 0.25) **[typed]**.

**ACS-07 Click picks the side.** For Automatic, None and Screen, a click sets
the centre where it lands, without mirroring it; the click's side becomes the
primary side. Click on −X: the − quad moves +0.25, the + quad −0.25; click on
+X: the reverse **[seen]**. Element works the same with the clicked element's
centre **[seen]**.

**ACS-08 Plane, offset and moved items.** The mirror plane lives in the mesh
item's own space and moves with the item **[typed]**; a Symmetry offset moves
the plane along its axis, and a centre on the offset plane counts as − side
**[typed]**. One oddity **[script]**: with the item moved +0.7 in X, a centre
typed at world X 0.2 (item X −0.5) still made the + side primary, as if the
side test used the centre's world coordinate against world zero. Treat as
**[open]** (ACS-Q4).

**ACS-09 Topology option.** On an exactly symmetric mesh, Topology on gave
the same results as plain X symmetry in all 45 cases **[script]**.

**ACS-13 Falloff with Symmetry.** With Linear Falloff, Symmetry X and
Automatic, a Move of a mirrored pair of faces fades the same way on both
sides, as mirror images of each other **[seen]** (Jason, 2026-11-19,
segmented cube; the falloff's start and end sat on opposite sides of the
cube, running across it, and the Move handle sat on the + side). So each
point takes its falloff weight at its mirror position, as SYM3-12 says. Other
falloff types weren't checked **[open]**.

## 4. Local, Scale and Rotate (answers the hold from ACL-xx)

**ACS-10 Local Scale.** Each island scales about its own **bounding-box
centre** along its own local X (the ACL-02 axis), with the same factor for
every island. Hand drag on the X cube, factor 6.08, eight islands: every
island stretched 6.08 along its own axis, the fixed point the box centre
(the triangle: box 14.706, vertex average 14.594 along that axis; the
tilted strip: 6.873 vs 6.850) **[seen]**.

**ACS-11 Local Rotate.** Each island rotates about its bounding-box centre;
a rotation entered about Y turns each island about its own normal (its local
Y): Rotate Y 30 turned the flat islands about world Y and the tilted ones
about their tilted normals (for example (0, 0.866, −0.5)) by 30° each
**[typed]**. A free hand drag on the handle gave one 113.8° rotation, applied
in each island's own frame (the same angle, axes turned with each island)
**[seen]**.

**ACS-12 Local Y and Z.** Local Y is the island normal; Z completes a
right-handed frame **[typed]**, which settles ACL-Q2 for flat islands.

## 5. Tools that don't use the pivot (and one deliberate exception)

**ACS-20 Smooth Shift and Thicken.** Operandi rule: their Scale pivots on
the selected Action Center, through the pivot service like a transform tool
(Jason's ruling 2026-11-18, a deliberate deviation from the reference app).
Observed in the reference app, for the record only: the Scale is about the
mesh item's own origin whatever the Action Center (centre typed at three
positions, Symmetry off or X, identical results **[script]**; the 11-18 UI
check gave the same **[seen]**). The 24 Smooth Shift / Thicken cases in
cases.json record that reference behaviour and are marked "not an Operandi
oracle".

**ACS-21 Bevel and Extrude.** Results don't depend on the centre **[script]**.
In the UI, dragging the Extrude handle sideways along X with Symmetry X
mirrors the shift: the − side extrudes the opposite way in X **[seen]**
(Jason, 2026-11-19, segmented cube, Selection). Operandi follows the UI.
By script the shift was not mirrored (both quads moved +0.3 in X)
**[script]**; that's a scripting artefact, and the 6 scripted Extrude cases
with Symmetry X are marked "not an Operandi oracle".

## 6. Fixtures

All in one mesh item at identity unless the case says otherwise; coordinates
written for X symmetry (for Y and Z symmetry the case's points are rotated so
the mirror axis comes first: (a, b, c) becomes (c, a, b) for Y and (b, c, a)
for Z).

- **P** (+ side), quad: (1,0,0) (1,0,1) (2,0.5,1) (2,0.5,0) — tilted.
- **N** (− side), its mirror: (−1,0,0) (−2,0.5,0) (−2,0.5,1) (−1,0,1).
- **C**, straddling quad: x −0.5…0.5, y 0, z 2…3.
- **F** and **G**, unselected flat quads at x 3…4 and −3…−4, z −2…−1.
- **S**, two quads sharing a seam edge on x = 0 (x −1…1, z 4…5, far edge
  raised 0.3).
- **Islands** (Local cases): the eight-island mesh of ACL-xx, listed in full
  in cases.json.

Selection sets used in cases.json: pair = P and N; oneside = P; straddle = C;
pairstraddle = P, N, C; seam = both S quads; seamhalf = the + S quad.

## 7. Test matrix

`cases.json` holds 311 cases:

- 234 transform cases with the centre typed (Automatic, Tool Properties
  centre fields): Move (0.25, 0.1, 0.05), Rotate Y 30, Rotate Z 30, Scale
  (2, 1.5, 1), across centre on +, on and − side; Symmetry off, X, Y, Z,
  X+Topology, X with offset 0.5, item moved +0.7 in X; selections pair,
  oneside, straddle, pairstraddle, seam, seamhalf. Expected points in item
  space, tolerance 1 mm. These test ACS-03..09 without any click.
- 48 Smooth Shift, Thicken, Bevel, Extrude cases (centre-independence, ACS-20/21); the 24 Smooth Shift / Thicken ones are reference behaviour only, not Operandi oracles (ACS-20).
- 26 hands-on centre placement cases: the centre the reference app showed for each mode
  and the points after typed values (ACS-01, 02, 07 and the table).
- 3 Local cases (ACS-10/11).

A builder can run every typed-centre case against the pivot service alone.

## 8. Open questions

- **ACS-Q3** The axis frame of custom-axis modes: "tool Z = axis direction,
  tool X = world X made perpendicular" fits 11 of the 16 hands-on custom-axis
  cases; Rotate under Selection, the Y-symmetry case and the three cases
  whose axis leaves the XY plane (with the straddling quad) don't fit. The centre rules hold in
  all of them.
- **ACS-Q4** With a moved item, is the primary side decided by the centre's
  world coordinate (as scripted) or against the item's plane?
- **ACS-Q5** Do selected points exactly on the plane count toward the + half
  centre (ACS-01)?
- **ACS-Q6** Pivot Center Parent Axis and Parent with a turned parent.
- **ACS-Q8** Topology symmetry on a mesh that isn't exactly symmetric;
  Arbitrary axis; the Work Plane option.

## 9. Hands-on checks

All done. Jason's three hand checks on 2026-11-19 are recorded in ACS-01
(Selection Border), ACS-13 (falloff) and ACS-21 (Extrude). The Smooth Shift /
Thicken pivot follows Jason's ruling (ACS-20). What's left is listed in
section 8, and none of it blocks building the pivot service.
