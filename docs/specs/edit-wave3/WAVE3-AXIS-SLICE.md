# Wave 3: Axis Slice section — behaviour spec v0

Provenance: the reference application's user guide (Axis Slice, Julienne),
its plain-text UI configuration (button order, preset values). Polygon Axis Slice measured 2026-11-08 (§2); Julienne/Dicer still docs-only.
REA-assisted: no. Supersedes the Axis Slice paragraphs of
`toolbox-panels-2026-11-06/EDIT.md` §2.

## 1. Panel

**AX-01.** Section **Axis Slice** below Slice, three large icon + label
buttons: **Polygon Axis Slice**, **Dicer**, **Julienne**.

## 2. Polygon Axis Slice (build in wave 3) — v1, measured 2026-11-08

17 scripted cases (a flat 4 × 2 m quad, an off-centre quad, a 2 m cube;
oracles in `cases-wave3.json`) plus the user guide and Jason's hands-on
use of the reference. Viewport handle details are from Jason's use and
the guide (not scriptable).

**AX-10 What it cuts (Observed).** Polygon mode. Works on the selected
polygons; with none selected, on every polygon of the active mesh. Adds
straight cuts through those polygons along planes perpendicular to world X,
Y and/or Z, placed inside the **bounding box of the polygons it works on**.
Nothing moves; polygons are split. Unselected neighbours that share a cut
edge take the new point on that edge (no T-junctions). With a selection,
exactly the resulting pieces are selected afterwards; with none, nothing
is.

**AX-11 Options and starting values (Observed from the tool's settings).**

| Option | Kind | Starts at |
|---|---|---|
| Slice Mode | Number / Size | **Number** |
| Number X, Number Y, Number Z | whole numbers, minimum 0 | **0, 0, 0** |
| Size X, Size Y, Size Z | distances | **1 m, 1 m, 1 m** |
| Offset X, Offset Y, Offset Z | distances | **0, 0, 0** |
| Hold Slice | on/off | off |

Number fields are greyed out in Size mode and Size fields greyed out
in Number mode. Because all Numbers start at 0, **activating the tool
in Number mode makes no cuts** until a Number is raised. That's expected,
not a fault.

**AX-12 Number fields step (Jason, hands-on; UIC-51..54).** Number X/Y/Z are
ordinary whole-number fields: the step arrows and a drag on the field (or
mouse wheel, as Operandi's other integer fields) go up and down by 1,
never below 0, and every change re-cuts live. Raising Number X adds one
more cut across X. (Operandi build reported these fields don't step; they
must.)

**AX-13 Number mode spacing (Observed).** On an axis with Number *N* > 0 and
box extent min…max (length L): spacing s = L / (N + 1); cuts at
min + k·s + w for k = 1…N, where w = Offset wrapped into one spacing
(w = Offset mod s, taken in 0…s). So there are always exactly N cuts
inside the box; the Offset slides the whole group, and a cut pushed past
one end reappears at the other end of its spacing.
Examples (quad x −2…2): N 1 → x 0; N 2 → ±0.667; N 3 → −1, 0, 1;
N 2 with Offset 0.3 → −0.367, 0.967; N 1 with Offset 2.5 → 0.5.
An axis with zero extent (a flat quad's Y) gets no cuts.

**AX-14 Size mode spacing (Observed).** On each axis: cuts at
min + Offset + k·Size for every whole number k (positive or negative)
that lands **strictly inside** the box; positions exactly on the box's
faces are skipped. The count changes as Offset or Size changes.
Examples (quad x −2…2): Size 1 → −1, 0, 1; Size 1.5 → −0.5, 1.0;
Size 0.7 → −1.3, −0.6, 0.1, 0.8, 1.5; Size 1 with Offset 0.3 → −1.7,
−0.7, 0.3, 1.3. Off-centre quad x 1…5, Size 1 → 2, 3, 4. With the starting
values (1 m everywhere) a 4 × 2 flat quad gets X cuts at −1, 0, 1 and a Z
cut at 0.

**AX-15 Handles (Docs + Jason hands-on; Provisional details).** While the
tool is active the viewport shows a handle for the cut group. Dragging it
moves **all cuts on an axis together** (it changes that axis's Offset),
re-cutting live; the cuts keep their spacing. Horizontal and vertical
drags in a view move the cuts along the two world axes that view shows
(Provisional: the axis closest to the drag direction on screen; in a
perspective view, the two world axes nearest to screen right and screen
up). Only axes with cuts move visibly. Hold Slice on: dragging and typing
update the handle and values but the mesh isn't re-cut until Hold Slice is
turned off (for dense meshes).

**AX-16.** Tool stays live; each change re-applies; one undo step when the
tool is dropped. Cut edges follow the slice engine's corner-data rules.

**AX-17 How to use it (for Jason and testers).**
1. Polygon mode; select the polygons to cut (or nothing, for the whole
   mesh).
2. Click Polygon Axis Slice. Nothing is cut yet (Numbers start at 0).
3. In Tool Properties, raise Number X (or Y, Z) with the step arrows or by
   dragging the field: that many evenly spaced cuts appear across the
   selection on that axis.
4. Drag in the viewport to slide the cuts as a group.
5. For fixed spacing instead, switch Slice Mode to Size and type Size X/Y/Z.
6. Press Q (or pick another tool) to drop it.

## 3. Julienne and Dicer (need the solid cutter from wave 4)

Both cut a mesh into **separate closed pieces** (inner faces are made),
so they need the closed-volume cutter of the Booleans work. Specify now,
build after wave 4's Boolean core exists (or move them into wave 4).

**AX-20 Julienne (Docs/Config).** A row of parallel cutting planes. First
click sets the origin (on the work plane); dragging sets the offset to the
next plane; dragging off the handles changes the planes' angle. Options:
**Number of Clones** (planes; starts 8), **Offset X/Y/Z** (starts 0.2, 0.2,
0), **Size X/Y/Z** (plane size), **Rotate X/Y/Z** (plane angle), **Between**
(planes spread evenly between origin and Offset instead of repeating by
Offset), **Angle Snap** (on) with **Angle** (45°). Cutter options:
**Preview Only** (on while editing: draws the cuts without making them;
must be turned off to cut), **Gap** (0.001: thickness removed between
pieces), **Inner Material** (tag for the new inside faces). Random Seed /
Randomness are ignored when a row generator drives the planes.

**AX-21 Dicer (Docs/Config).** Same cutter with **random** planes: smashes
the mesh into chunks. **Random Seed** (40), **Randomness** (0), **Gap**
(0.001), **Inner Material**, **Max. Cutting Planes**, **Fragment Scale
X/Y/Z** (relative spacing: long thin vs. roughly cubic chunks), Preview
Only. Same seed, same result (Provisional).

**AX-22 Result (Docs + Provisional).** Each piece is a closed shell; pieces
stay in the same mesh item as separate connected parts, Gap apart; inner
faces get Inner Material. Open meshes: Provisional, cut faces only, no caps
(AX-Q3).

## 4. Open questions (planned probe batch)

- **AX-Q2** Handle drag: exact mapping from screen drag to axes, drag distance to Offset.
- **AX-Q3** Julienne/Dicer on open meshes; piece placement with Gap.
- **AX-Q4** Dicer plane distribution for a given seed (Operandi will not
  match the reference's random sequence; same-seed repeatability is the
  goal).
