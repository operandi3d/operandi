# Duplicate tools: builder questions (2026-11-08)

Behaviour questions from the Operandi build (branch duplicate-tools,
spec v1.2). Each has the reading built today, marked provisional.

- **DUP-Q01 Internal walls after a weld.** When Merge Vertices leaves two
  polygons on the same points (two touching cubes from Clone or Array, a
  ring of wedges from Radial Array), are both polygons removed, as MR-43
  says for Mirror? Built: removed for every tool. (Operandi can't keep an
  edge shared by more than two polygons, so CL-31's "shared faces stay"
  can't be stored either way.)
- **DUP-Q02 Welds inside one copy.** Two new points of the same copy that
  lie within Distance of each other: do they weld? Built: no, matching
  DS-23's rule that the source's own close points never weld.
- **DUP-Q03 Radial Sweep and the profile's own points.** Does the swept surface
  start on the profile's points (joined to them), or on new points beside
  them? Built: joined for lines and curves (they are used up) and for
  border edges on a part turn; new points for face outlines and full
  turns of edges. Faces sweep the outline of their region (an edge two
  selected faces share sweeps once). A profile that branches (three
  edges at one point) is refused.
- **DUP-Q04 Clone Scale and Rotate.** Built to v1.2: compounding per step
  (copy i sized Scale^i and turned i × Rotate about its own centre).
  Waiting on DQ-13.
- **DUP-Q05 Clicking away after the first placement (DS-34).** Built: Radial Sweep,
  Radial Array and Clone start a new placement (the previous result stays
  as its own undo step). Mirror ignores later clicks (a second mirror of
  the result would double it). Array's left-drag changes spacing (AR-21),
  so a click there doesn't restart. Is that how the tools behave?
- **DUP-Q06 Item mode.** Where do new items go in the Items list, and with
  which parent? Built: straight after the source, same parent. With
  Replace Source the source item itself moves into slot 0. A rotated item
  given a non-uniform size change (Array Scale, Clone Scale) keeps no
  shear: the nearest plain scale and rotation is used.
- **DUP-Q07 Handle tooltips (UIC-05).** What does each handle's tooltip
  say? Built: grips light up under the pointer; no tooltip text yet.
- **DUP-Q08 Detriangulate.** Built: runs in any mode (whole mesh outside
  Polygon mode, DT-11); in Polygon mode it selects the new quads and the
  leftover triangles (DT-26), even when nothing was selected before.
  Settings are remembered across restarts. Does the reference select the
  result, and does it run outside Polygon mode?

## Answers (research side, spec v1.3, 2026-11-08)

All of these are Operandi's own choices [OP], except where noted: the
reference couldn't be checked by script for these (scripted applies of
the line and circle placers don't behave like a real click). Each one
endorses what's built, now written into the spec.

- **Q01** Yes, both removed, for every tool: new rule **DS-25**. CL-31 and
  AR-32 rewritten to match.
- **Q02** No weld inside one copy: new rule **DS-26**.
- **Q03** As built: **LT-13a** (where the surface starts) and **LT-13b**
  (branching profile refused, with the status message).
- **Q04** As built (v1.2 compounding). Still waiting on Jason's hand
  check, DQ-13.
- **Q05** As built: **DS-34** rewritten with the Mirror and Array
  exceptions.
- **Q06** As built: added to **DS-12**.
- **Q07** Tooltip text for every grip: new rule **DS-38**.
- **Q08** Keep as built (**DT-11**, **DT-26**). The reference wasn't
  checked; if Jason sees otherwise in QA, it's a one-line change.

## v1.5 name changes for the builder (2026-11-08)

Jason: names follow the reference's docs. None of the spec's names came
from any other project; these were our own plain-English renames, now
reverted to the docs' names. Labels only, no behaviour change:

| Was (v1.4) | Now (v1.5) | Where |
|---|---|---|
| Flip Polygons | **Invert Polygons** | every copier |
| section "Mirror Plane" | **Mirror Generator** | Mirror |
| Snap to World Axis in the plane section | own section **Mirror Generator Snap** | Mirror |
| section "Line" | **Linear Generator** | Clone |
| section "Grid" | **Array Generator** | Array |
| section "Circle" | **Helix Generator** | Radial Array, Radial Sweep |
| section "Copies" | **Clone Effector** | Mirror, Clone, Array, Radial Array |
| section "Sweep" | **Sweep Effector** | Radial Sweep |
| UVs Around | **Sweep UVs** | Radial Sweep |
| Keep Material Boundaries | **Keep Material Boundary** | Detriangulate |
| Keep Part Boundaries | **Keep Part Boundary** | Detriangulate |
| Keep Smoothing Boundaries | **Keep Smoothing Group Boundary** | Detriangulate |
| Keep Selection Set Boundaries | **Keep Selection Set Boundary** | Detriangulate |
| Keep UV Seams | **Keep UV Boundary** (not in the docs; our label) | Detriangulate |
| Lathe (v1.4 already) | **Radial Sweep** | tool, button, menu |

Already matching the docs, unchanged: Mirror, Clone, Array, Radial Array,
Detriangulate; Axis, Angle, Mode (Axis / Free Rotation / Three Points),
Center X/Y/Z, Left X/Y/Z, Up X/Y/Z, Snap to World Axis, Replace Source,
Merge Vertices, Distance, Number of Clones, Offset X/Y/Z, Scale X/Y/Z,
Rotate X/Y/Z, Count X/Y/Z, Jitter X/Y/Z, Between, Count, Axis X/Y/Z,
Start Angle, End Angle, Offset, Cap Start, Cap End, Flatness, Max Angle.
Helix Generator's Center X/Y/Z row isn't in the docs' table but the tool
has it, so it stays.
