# Specification questions and deferred combinations

Open questions and product decisions, oldest first. Entries from before
2026-11-05 that quoted specs since removed were taken
out on 2026-11-20; the tools they covered were re-checked against the clean
specs in `specifications/`. Route behaviour questions to the separate research
side and ask for observable setup, actions and results only.

## M1.5 selection continuation and fill — 2026-10-30 (provisional)

- SEL-Q-M1: Select Between with two equally short routes (e.g. opposite faces of
  a cube, or opposite sides of a closed ring) currently selects every shortest
  route. Guide shows only the single-row and rectangle cases. Decision needed:
  pick one route, refuse, or keep the union.
- SEL-Q-M2: Polygon Selection Fill refuses an area that reaches an open mesh
  border, and on a closed mesh refuses a selection that doesn't split the
  unselected polygons. Decision needed if Modo fills such areas anyway.
- SEL-Q-M3: To make the Cmd+Shift double-click reach Fill on macOS, a single
  Cmd+Shift click in Polygon mode no longer adds to the selection (Ctrl+Shift
  click is unchanged). Decision needed on the macOS modifier for Fill (SEL-U01).

## Point lasso visibility — 2026-10-31 (Jason decision, provisional vs. reference)

SEL-040..044 / SEL-U03: right-drag lasso in Vertex mode on an open box viewed so
every polygon at a corner faces away. Facing-only skipped the unobstructed
corner. Jason chose: points are eligible unless a surface hides them (click
test). Edges/polygons still facing-only. Research: confirm reference lasso
visibility for points, edges and polygons.

## Jason decisions — 2026-11-01 (product policy, provisional vs. reference)

Interview answers from Jason. These are Operandi product decisions; they do not
claim reference behavior. Items marked "Research:" still need a spec.

| ID | Decision | Code today |
|---|---|---|
| PM-Q01 / BOX-Q07 | Ctrl before drawing: Cube is a perfect cube; Cylinder has equal X/Y/Z proportions (diameter = height) along the tool's Axis; Sphere unchanged. | Already implemented (`initial_draw_bounds`). |
| PM-Q02 | Ctrl (or other modifiers) pressed or released mid-drag is ignored; only the state at press counts. | Done 2026-11-01. |
| BOX-Q04 | No Cube corner handles for now; Show Corner stays unavailable. | Matches. |
| BOX-Q09 / SP-Q08 / CYL-Q11 | Revised by Jason the same day: primitives keep their last settings like Modo (the Cone spec CA-CON-003 records retention), with Reset Tool Attributes to restore the starting values. | Done 2026-11-01 for Cube/Sphere/Cylinder segment settings; Cone and Ellipsoid already kept theirs. |
| BOX-Q05 | With a primitive live, Shift-drag off its handles finishes the current one and starts a new one with the same settings. | Done 2026-11-01. |
| SEL-Q-M1 | Select Between with tied shortest routes keeps the union (selects every shortest route). | Matches. |
| SEL-Q-M2 | Fill keeps refusing open-border / non-splitting areas for now. Research: confirm reference behavior. | Matches. |
| SEL-Q-M3 / SEL-U01 | Shift+click adds to the selection in every component mode. A single Shift+Cmd click is Fill (no double-click). Cmd+click opens a pop-up menu of modeling operations at the cursor. Research: the menu's contents per mode. | Shift+click and single-click Fill done 2026-11-01 (Shift+Ctrl off macOS). Cmd+click menu not started. |
| SEL-040..044 / SEL-U03 | Lasso visibility for edges and polygons works like points: eligible unless a surface hides them. | Done 2026-11-01 (edges tested at their midpoint, polygons at their center). |
| SQ-005 / SQ-M11-001 | Ship Modo's default shortcuts for all tools. Research: map the full default key set (macOS). Tools stay unassigned until that spec arrives. | Matches the interim. |
| PM-Q04 (Mac) | "Ctrl" in primitive gestures is the physical Control key on macOS. | Matches (`Modifiers::ctrl`). |
| SQ-004 / SQ-M11-005 | Positive scale factors only; no negative/mirror scale or percent entry for now. | Matches. |
| SQ-007 / SQ-008 | Wanted soon: Symmetry, Snapping, Item hierarchy (parenting, groups, reorder, multi-item). Falloff later. Research: specs for each. | Not started. |
| SP-Q13 / CYL-Q16 | New primitives generate a basic default UV map, provisional original layout. Research: confirm the reference layout. | Done 2026-11-01: on by default, "Texture" map, Make UVs kept between uses. Sphere/Cylinder record counts match R16/R17-SP-01 and CY-01; layouts provisional. Edge Bevel/Chamfer/Extrude and Vertex Bevel/Extrude now carry UV maps (SQ-UV-01 below). |

## Row selection by click — 2026-11-01

- SEL-Q-M4: Jason selected one polygon on a sphere, then Shift+Cmd clicked
  another polygon several faces along the same row, expecting the whole row
  between them to be selected (as he remembers it). Operandi's Shift+Cmd click
  is Polygon Selection Fill, which only fills an area enclosed by the
  selection, so it refused. The row is available today as Select Between
  (Shift+G) after picking both ends. Research: what does the reference do
  for a Shift+Cmd (or other modified) click on a second polygon: fill, a row
  between picks, or something else; and which gesture selects a row/loop
  between two picks? No change until answered.

## UVs on new points — 2026-11-01 (provisional)

- SQ-UV-01 (ME-T01/T02/T07, MV-T02/T03): a UV-mapped mesh (e.g. a new Cube),
  then Edge Bevel/Chamfer/Extrude or Vertex Bevel/Extrude. The specs don't say
  what UVs new points get. Provisional: inside each polygon, a new point's UV
  is interpolated from that polygon's own corner UVs as far along as the point
  was cut (seams stay intact); bevel/chamfer strips blend between the two
  neighbours' UVs; Edge Extrude walls map onto the strip each neighbour gave
  up and its raised points reuse the edge ends' UVs; Vertex Bevel caps and
  Vertex Extrude triangles reuse the cut corners. Added the same day: Grow
  Quads continues UVs outward by one row's step; Fill Quads takes the border
  points' UVs from the polygons facing like the fill (completing a
  parallelogram where a corner has none); Edge Bridge blends from one
  border's UVs to the other's; Chamfer Edit keeps existing corners and blends
  new profile points along their edge. Corner weights, colors,
  normals and typed corner maps are still refused. Research: what the
  reference does with UVs (and other corner maps) for these tools.

## Symmetry — 2026-11-02 (provisional readings)

Built from specifications/symmetry-2026-11-01. Open items the spec already
lists (SYM-Q01..Q11) are not repeated; these are the readings Operandi chose.

- SYM-Q04 (tolerance measure): matching uses 1.5×10⁻⁶ of the bounding-box
  diagonal (6.2×10⁻⁶ m on F1, between the measured 5 and 8×10⁻⁶ m). Points
  closer than that to the plane count as on it for transforms too.
- SYM-SEL-01 (order): counterparts are appended walking the selection from the
  most recent pick back, which reproduces 9, 8 → 9, 8, 6, 5 and 9, 7 → 9, 7, 5.
  Research: confirm with three picks, e.g. 9, 8, 6 (expect 9, 8, 6, 5?).
- SYM-TR-04 for Rotate/Scale: a point on the plane is transformed, then put
  back on the plane (only Move was measured).
- SYM-Q09 (Symmetrize): partners are mutual nearest mirrored points within 1%
  of the bounding-box diagonal; points on the plane stay; unmatched points
  stay.
- SYM-PR-01 beyond Cube: Sphere and Cylinder are doubled by the same rule.
  Ellipsoid and Cone don't follow the shared setting yet. The Ellipsoid's own
  older Numeric Symmetry (EA-011) reverses every corner ([a,b,c,d] →
  [d',c',b',a']), while SYM-PR-02 keeps the first corner; Research: does the
  Ellipsoid use the Cube's order?
- Primitives that straddle the plane are still doubled (copies overlap);
  Research: what happens to a Cube centred on the plane?
- Not built: Topology matching (SYM-SEL-06), Arbitrary axis, Work Plane
  option, hidden counterparts, UV symmetry, the Symmetry Tool.

## Falloff — 2026-11-02 (provisional readings)

Built from specifications/falloff-2026-11-01; its own open list is
FAL-Q01..Q09. Operandi's readings:

- FAL-M-02 for Rotate/Scale: only Move was measured. Rotate scales its
  angles by the weight; Scale moves its factor toward 1 by the weight.
- With Symmetry on, a point's weight is taken at its own position before
  mirroring.
- Radial with a zero Size axis: only points on that axis's plane through
  Center get a weight.
- Extrude (FAL-T-04, FAL-Q04) and Smooth Shift/Thicken (FAL-Q06) refuse an
  active falloff until the topology rule is known. Every other tool except
  Move/Rotate/Scale/Transform, Bevel, Edge Bevel/Chamfer and Vertex Bevel
  refuses it too (unmeasured).
- FAL-O-09 and FAL-O-10 bevel border points/edges, which Operandi's Edge
  Bevel and Vertex Bevel don't support yet (edge-bevel-chamfer topic). The
  weight rule is tested on a closed box edge and an interior point instead.

## Selection — 2026-11-02 (provisional readings)

Built from specifications/selection-2026-11-02.

- SEL-Q-M1 is answered and Operandi now follows it: a tie selects one route,
  replacing the earlier "keep the union" decision (Jason's standing rule for
  this round: follow the measured behaviour, log the change). The route is a
  breadth-first search from the second pick with neighbours in winding /
  index order; it reproduces all nine measured ties, but the rule behind it
  is inferred (SEL-Q-B2).
- Rectangle (SEL-BT-02): picks first, then the rest; the reference's order of
  the rest isn't reproduced (it changes with pick order, no rule yet).
- Previous/Next Loop (SEL-LP-02..04) not built: from the x = 1 edge loop both
  Next and Previous gave the border, so the side rule isn't clear. Research:
  Previous/Next from a loop with an interior loop on both sides (e.g. x = 2 on
  Grid4).
- Multi-item component editing (SEL-IT-04/05) not built: Operandi selects one
  item at a time. Building it means multi-item selection first; Jason to
  decide when.
- Parent menu: no default for Keep Position was measured, so the item's
  right-click menu offers both "Parent To" and "Parent To, Keep Position".
  A parent with shear (non-uniform scale under rotation) refuses Keep
  Position.

## Polygon Bevel — 2026-11-02

Built from specifications/polygon-bevel-2026-11-02. 20 measured cases; the
geometry of every grouped and ungrouped case already matched.

Now following the reference:
- PB-O-01: with nothing selected Bevel works on every polygon and leaves
  nothing selected (it used to refuse).
- PB-C-02/03: with Auto Weld, a cap whose corners weld into one point or a
  line is removed, leaving triangles and quads around it.

Deliberate departures, for Jason to confirm or reverse:
- M17-Q04: insets past the polygon's center without Auto Weld (zero-size
  or inverted caps, self-crossing L) are still refused, as Jason chose
  earlier. Following the reference would mean allowing self-intersecting
  polygons in Operandi meshes.
- PB-O-04: Shift 0 and Inset 0 leave the mesh unchanged; the reference
  builds zero-size walls.
- Index order: Operandi keeps each cap in its own polygon's slot (live
  identity tracking relies on it), numbers a lone polygon's new points by
  point index and orders walls polygon by polygon. The reference reverses
  multi-face cap slots, uses corner order and walks the border. Same
  geometry. PB-Q06: is the cap reversal driven by selection order or index
  order? Re-run box faces 1 + 3 picked as 3 then 1.

## Polygon Extrude — 2026-11-02

Built against specifications/polygon-extrude-2026-11-02 (21 measured cases).
15 already give the reference's geometry, including Auto/Thick/Shift,
non-planar polygons, polygons touching at a point or facing different ways,
segments, zero shift and the world-space shift on a rotated item.

Departures, for Jason:
- Index order: the reference lists walls first and caps last; Operandi keeps
  the source polygons in their slots (live identity tracking relies on it).
  Same geometry.
- PE-O-02 not built: a shift in the polygon's own plane should merge the
  points that land on existing ones (two walls become triangles). Operandi
  keeps separate coincident points. Doing it needs the extrude
  correspondence to allow merged points.
- PE-N-01/02: Operandi refuses meshes with an edge shared by three
  polygons, so these cases can't be loaded.
- PE-O-03: with nothing selected the reference extrudes every polygon; the
  core operation does, but the app still asks for a selection (the live
  Extrude routing is keyed to the selection). Not built yet.

## Edge Bevel / Edge Chamfer — 2026-11-02

Built against specifications/edge-bevel-chamfer-2026-11-02 (21 cases; 7
single-edge cases now match).

Now following the reference:
- EBC-P-01: profile rows sit at equal angles on the arc (they were equal
  steps of a rational curve, which bunches them).
- EBC-P-03: Depth scales a row's distance from the chord and keeps its
  position along it.
- Round Level L now gives 2^L segments (was L + 1); provisional beyond the
  measured levels 0 and 2, capped at level 6 (64 segments).

Not built yet (each is a real piece of geometry work, for Jason to order):
- Junctions: edges sharing a point (EBC-J-02), three edges at a corner
  (EBC-J-03, J-05), closed loops (EBC-J-04).
- Square and Sharp shapes (EBC-P-02).
- Width mode (EBC-P-04): the measured 90° case can't tell 1/sin from 1/cos
  of the half angle, so EBC-Q02 should come first.
- Edges whose ends lie on an open border (seen while testing the falloff
  case FAL-O-09).

## Add Loop / Edge Slide — 2026-11-02

Built from specifications/add-loop-edge-slide-2026-11-02 (v2). Edge Slide:
all 15 measured cases pass (ES-01..08).

Now following the reference:
- ES-03: a Distance interpolation beside Percent.
- ES-04: sliding all the way merges the points with their neighbours (the
  polygons between are removed and the loop stays selected).
- ES-05: no clamping; values past ±1 (or past the rail length) are allowed
  and fold the mesh, with no message. Operandi used to stop at ±100%.
- ES-06: a border point with no rail on the chosen side uses its other
  side (it used to stay put).

Still open: ES-Q01 (which side is positive in general; Operandi uses the
left of each oriented chain, which matches the measured loops), ES-Q02..Q04.
Add Loop has no measured cases yet (AL-Q01), so nothing changed there.

## Primitives — 2026-11-02 (checked, not yet built)

Checked against specifications/primitives-2026-11-02 (32 cases). The 8
cases using options Operandi already has (default and segmented Cube, flat
and negative-size Cube, 8×4 Globe, 8-sided Cylinder and Cone, 8×4
Ellipsoid) give the reference's geometry; only the Ellipsoid also matches
point and polygon order. Operandi's own order and UV layouts differ.

Not built (each is new primitive work, for Jason to order):
- Cube: rounding (Radius, Radius Segments, Sharp, Axis), Bulge, UV Gaps and
  Compensation, the reference's cross UV layout (PRIM-C-02..07).
- Sphere: Axis, Quad Ball and Tessellated modes (PRIM-S-02).
- Cylinder: Axis, caps None/Quads/Radial (PRIM-Y-01); Edge Weight refusal
  with Face polygons (PRIM-Y-02).
- Cone: Quads caps on odd sides (PRIM-K-02); zero/negative height outside
  the one tested setup (PRIM-K-03).
- Ellipsoid: compensated UVs at zero/negative size (PRIM-E-01).
- Index order and UV layouts matching the reference (would renumber every
  primitive; worth deciding once, for all tools).

## UVs on new points — 2026-11-02 (checked)

Checked against specifications/new-point-uvs-2026-11-02 (18 cases).
SQ-UV-01's provisional rule holds where the geometry matches: Grow Quads,
Vertex Bevel and Vertex Extrude give the reference's UVs corner for corner
on the continuous map, and Vertex Extrude on the seam too.

Found while checking (for Jason; not changed yet):
- **Continuous point UVs aren't carried.** When a map stores one UV per
  point (as imported scenes and the reference do) rather than per corner,
  new points copy the old point's UV (Vertex Bevel) or get none (Grow
  Quads) instead of being interpolated. Operandi's own primitives store
  per-corner UVs, so this only shows on imported meshes. Worth fixing early.
- Geometry differs before UVs can be compared: Edge Bevel/Chamfer ends at
  interior points (the reference slides the point along the edge beyond and
  makes an end triangle; Operandi's cap is degenerate and refused), Edge
  Extrude (13 vs 15 polygons), Fill Quads, Bridge Segments (the reference's
  Segments 2 makes 2 polygons, Operandi's 3), and Chamfer Edit on a flat
  strip (refused as a degenerate profile).
- Seam choices: the reference's Vertex Bevel cap takes the left frame on a
  shared edge and the original point's continuous UV inside the seam
  polygon; its Grow Quads keeps the border points' continuous values. Not
  matched yet.

## Mesh commands — 2026-11-02

Checked against specifications/mesh-commands-2026-11-02 (34 cases); 27
now match, run through the same editor commands as the panel.

Now following the reference:
- CMD-V-04 Vertex Remove drops the points from their polygons (quads become
  triangles) instead of deleting those polygons.
- CMD-E-02 Edge Remove on border edges removes their points the same way
  (it used to refuse).
- CMD-V-01 Vertex Merge with nothing selected works on every point and puts
  each group at its average.
- CMD-V-06 Vertex Align keeps each point's place along the line's dominant
  axis (it used to project onto the line).

Not matching (logged): duplicate or mixed-winding polygons can't be loaded
(Operandi refuses non-manifold meshes, so Unify and Align cases can't run);
Merge refuses a region with a hole (the reference makes a keyhole polygon
that touches itself); aligning every point of a polygon onto a line is
refused as degenerate; Vertex Collapse (CMD-Q07) and Subdivide Percent
(CMD-Q04) follow other readings.

## Decisions — 2026-11-02 (Jason)

Deliberate departures from the reference, confirmed by Jason:
- **Numbering.** Where Operandi gives the same geometry as the reference but
  numbers points and polygons differently (Bevel caps and walls, Extrude
  walls and caps, primitives), Operandi keeps its own numbering: live
  identity tracking and attribute correspondence rely on it.
- **Bevel insets past the centre without Auto Weld** (M17-Q04, PB-C-01/04)
  stay refused rather than building inverted or self-crossing caps.
- Next build: Edge Bevel junctions (EBC-J-02..05).

## Edge Bevel junctions — 2026-11-02

Built: edges sharing points (EBC-J-02), three edges at a corner (J-03, a
triangle), closed loops (J-04) and two-row corners (J-05, quads around one
centre point) now match all 9 measured junction cases, and an edge ending
at an interior point slides the point along the edges beyond and closes
with an end triangle (UVN-T-01; its UVs match too). Edge Bevel selects the
strips' edges afterwards, Edge Chamfer nothing (EBC-J-01).

Provisional:
- The two-row corner centre is 2·mean(row points) − mean(corner points),
  which reproduces EBC-J-05's (0.460948, …); three or more rows at such a
  corner close with one polygon instead of a patch.
- A corner between two selected edges in Inset mode sits Inset along both
  edges (only 90° corners were measured).
- New centre points get no UV yet.

## Batch 2 corrections — 2026-11-02

From the v2 specs (source ad7a553), replacing earlier provisional readings:
- SYM-B2-02: Symmetry's matching tolerance follows the largest absolute
  coordinate (just under 3×10⁻⁶ of it, equality not accepted), not the
  bounding-box diagonal. All 58 measured offsets match.
- FAL-B2-02/03: a weighted transform moves each point to p + w·(T(p) − p);
  for Rotate that blends positions instead of scaling the angle. Scale and
  Move were already equivalent.
- FAL-B2-04: with X Symmetry, a point's falloff weight is read at its
  positive-side position.
- FAL-B2-05: a Radial Size of zero on an axis ignores that axis.
- SYM-B2-05: a Cube centred on the symmetry plane is made once.

The rest of batch 2 (Topology matching, Work Plane symmetry, Ellipsoid/Cone
mirroring, Extrude under falloff, Selection, new-point UVs, Add Loop and the
new Edge Slide cases) is committed as specs and not built yet.
- SYM-B2-10: with Symmetry on, a viewport pick also selects its mirror
  counterpart (measured for polygons; points, edges and deselection follow
  the same rule, provisional). A move of a one-sided selection still leaves
  the partner alone, as measured; picking normally selects both, which is
  the everyday mirrored workflow Jason expects.

## Batch 3 corrections — 2026-11-03

From the batch 3 specs (source a4880b2), replacing earlier readings:
- EBC-B3-02: Round Level L gives 2·L segments (0 → 1, 1–4 → 2, 4, 6, 8),
  not 2^L. Levels above 4 follow the same rule (provisional), up to 32.
- CMD-B3-01: Edge Subdivide Count n makes n intervals (n − 1 points), at
  least two; Count 1 with a Percent puts one point that far along. The panel
  row is Count 2; other counts are on the command line
  (`edge subdivide <count> [percent]`) until the panel gets a field.
- CMD-B3-02: Vertex Collapse merges the point and its edge neighbours at
  their mean (it used to dissolve the point's fan into one polygon).
- FAL-B3-02/04: Vertex Extrude and Edge Slide now work with a falloff
  (weights scale each point; weight 0 leaves a point unextruded).

Matching now: EBC-B3-03 (non-90° corners), the two-row corner UV case's
geometry, all of CMD-B3-01/02/03 and the gap-0 closed-loop Split.

Not built yet:
- CMD-B3-04 Align repairing a flipped polygon in a strip: Operandi refuses a
  mesh whose windings disagree, so the input can't be loaded.
- CMD-B3-05 Edge Split with a Gap or Caps on a closed loop.
- EBC-B3-01 Width mode, EBC-B3-04 corner patches past two rows, EBC-B3-05
  centre UVs, EBC-B3-06 edges ending on a border, EBC-B3-07 Edge End, and
  EBC-B3-08 Square/Sharp.
- FAL-B3-01/03/06/07 Edge Extrude, Spike, Bridge and Push under a falloff.
- The Polygon Bevel and Primitives batch 3 cases.

## Batch 2 Symmetry options and Jason's item-list requests — 2026-11-03

Built from symmetry v2: the Arbitrary axis (SYM-B2-09), Work Plane
(SYM-B2-08) and Topology (SYM-B2-07) options, and mirrored Ellipsoid and
Cone (SYM-B2-04). Provisional: with Work Plane on, typed values apply as
entered on the positive side, which fits the single measured Move but isn't
a rule the spec states; Topology's matching walk is Operandi's own.

Jason-specified, provisional until measured (questions for research):
- Cone and Ellipsoid draw in the viewport: press sets the centre on the
  construction plane, the drag sets the radius; the Ellipsoid is round and
  the Cone stands on the plane 1.5 radii tall (EA-Q01 and the Cone's own
  handles remain open). PRIM-Q04: how does the reference draw these, and
  with which handles afterwards?
- ITEM-Q01: drag an item row onto another to parent it (keeping its world
  position), onto empty list space to unparent. Does the reference keep the
  position by default, and does a modifier switch it? Can rows be reordered
  by dropping between them?
- ITEM-Q02: Ctrl-click on an item's eye gives its descendants the same
  visibility. Does the reference hide children with a hidden parent (in
  the view, or only the eyes), and how is that chosen?
- ITEM-Q03 (2026-11-03): the scene is the Items list's root row, named
  after its file ("Untitled" before saving), with every item under it;
  dropping an item on it unparents the item. Several open scenes aren't
  built: Operandi holds one document at a time, so that waits for the
  workspace layout work. The scene row has no icon yet (one would come from
  the icon generator).

## Pass 1 specs and what's still deferred — 2026-11-03

Built:
- KEY-01/02: B is Polygon Bevel in Polygon mode and Edge Bevel in Edge
  mode; Y is Transform; Edge Chamfer has no default key. Stored bindings
  that still hold the old defaults are moved over once. KEY-03/04/05 (S,
  L, [ ], Q) already matched.
- IT-01/03/04/05/06/07: typed item Move, Rotate and Scale on main, parent
  and child items. Negative Scale (Item mode, Scale and Transform) allows
  a mirrored item; off, a negative entry is refused with a message.
- ACT-AUTO-01/02: Automatic centres on the bounding box of what the tool
  moves (the whole mesh with nothing selected).
- ES-B2-01..03 and AL-01..12 geometry: 9 of 14 slide cases and every Add
  Loop case except Split give the reference's points and polygons.

Not built, or departing on purpose:
- ES-Q01: positive slide goes to the chain's left side, which follows
  winding and point order. The reference keeps its side under reversed
  winding and renumbering and flips it under a reflection, so five ES2
  cases slide the other way. The rule isn't pinned down yet.
- AL-Q02: how a click or drag becomes Add Loop's percent. The measured
  values round to 0.5% or 1% inconsistently; Operandi uses the exact
  fraction. Split (AL-04) isn't built.
- ACT-LOCAL-01 (Local, separate centres per island), item handle drags
  (IT-02/08/09) and the handle gains (HND-*: e.g. a 100 px drag giving
  0.435 m against 0.4329 m projected) are deferred; handle drags keep
  Operandi's projected distance.
- Snapping (SNP-*): Operandi has no snapping yet. Typed values are never
  snapped (SNP-ST-03), which holds trivially.
- Falloff on Edge Extrude, Bridge and Push (FAL-B3-01/06/07): Push doesn't
  exist, and how Bridge and Edge Extrude split topology under a weight
  isn't in the specs.
- CMD-B3-04: Align repairing a flipped strip. Operandi refuses to load a
  mesh whose windings disagree, so this stays a departure (logged earlier).

## Primitive options — 2026-11-03

Built from specifications/primitives-2026-11-02: Cylinder caps None,
Single, Quads and Radial per end and Axis X/Y/Z (PRIM-Y-01); Edge Weight
stays unavailable for Face polygons (PRIM-Y-02); Cone Quads cap on odd
sides (PRIM-K-02); Sphere Axis and the Tessellated mode, Level 0–6
(PRIM-S-02). 18 of 32 measured cases match by geometry.

Provisional: Quads caps on odd sides other than 5 fan their quads from the
last inner point; Tessellated UVs are a spherical projection (no UVs were
measured); the Cylinder's inner ring is at half radius for every side count.

Not built yet: rounded and bulged Cube (PRIM-C-02..07, PRIM-B3-02), Quad
Ball (PRIM-B3-01: the measured cage is 1.309182 × Size, and no rule for
other levels), Cube UV Gaps and Compensation, compensated Ellipsoid UVs,
and the zero/negative-height Cone cap cases (PRIM-B3-03/04).

## Hands-on specs (source 30eee87) — 2026-11-03

Built:
- POP-PHY-01…10: Command-click opens Modeling Operations (Polygon, Vertex,
  Edge) or Item Operations (Items) in the measured order, with separators.
  Entries run the same actions as their rows; Triple, Split Vertex, Edge
  Collapse and Duplicate give the measured results. Departure: the other
  root entries (materials, shaders, selection sets, lasso style, recording)
  are left out, and operations Operandi lacks are listed disabled. Edge
  Weight runs Edge Crease (provisional match).
- POP-PHY-10: Duplicate makes a coincident copy named "Name (2)", alone
  selected; one undo step.
- IT-PHY-01: changing the rotation order (Properties) keeps the angles, so
  the item turns. Only XYZ and ZXY exist in Operandi.
- IT-PHY-04: Ctrl+Shift dragging an item handle moves a copy and leaves the
  original. Provisional: the copy is independent, not a linked instance
  (Operandi has no instances).
- SEL-PHY-03: Shift+Cmd on a polygon outside a selected perimeter selects
  the row between it and the last pick; inside a perimeter it still fills.
- AL-PHY-02…04: Add Loop Shift-click is an ordinary click, Ctrl-click the
  middle, Command-click does nothing and opens no menu. Clicks and slides
  land on 0.5% steps (AL-Q02 provisional).
- Cone Quads cap on odd sides now follows the older 9-sided case
  (CA-O-BATCH-QUAD_CAP_S9) and PRIM-K-02 together: the grid for one more
  side with one corner cut to a triangle.

Not built:
- SEL-PHY-01/02 item-list Shift ranges: Operandi selects one item at a time.
- KEY-PHY-01…03 Shift+C Polygon Slice, Option+C Loop Slice, Ctrl+E Bend:
  those tools don't exist yet. Command+E stays unbound (KEY-PHY-04).
- IT-PHY-02/03 handle drag distances and the middle-button clone: no change.

## Free transform handles — 2026-11-03 (Jason-specified, provisional)

- HND-Q03: Move, Rotate and Transform now have a center square. Dragging it
  moves the selection freely in the view plane (Move, Transform), or turns it
  about the frame axes nearest the view's up (left/right drag) and right
  (up/down drag) at half a degree per pixel (Rotate). Scale's center already
  scaled uniformly. How do the reference's center handles map pointer motion?
- VAC-Q02: with Move, Rotate, Scale or Transform live, a press on empty
  viewport space keeps what was done so far, puts the tool's center there
  (at the current center's depth) and drags from it like the center handle.
  A press on the model (within 12 px of a point in Vertex mode, on an edge
  or polygon in those modes) now drags freely too, keeping the center where
  it is (Jason, 2026-11-03). A press on a selected element puts a moved
  center back where the Action Center puts it for the selection (Jason,
  2026-11-15). Does the reference also relocate on the model, and onto
  which depth or surface?
- HND-Q04: each Move/Rotate/Scale/Transform drag is its own undo step
  ("Move", "Rotate", ...). Cmd+Z and Redo keep the tool live; it restarts
  on the restored scene with its center from the restored selection (a
  center placed by clicking away is not remembered). After each drag the
  tool's fields restart at zero. A typed value not yet followed by a drag
  is recorded as a step when Undo is pressed. Cancel discards only the
  change since the last drag. If Undo leaves nothing to transform (e.g. it
  removes the mesh), the tool drops. Do the reference's fields restart or
  accumulate after each drag, and does Cancel revert the earlier drags?

## Jason-specified (QA, 2026-11-02), provisional until measured

- Symmetry handle: after a pick, the transform handle sits on the picked
  side's points, and a handle drag moves them the way it's dragged, the
  mirror side following mirrored. Typed values keep SYM-TR-02 (as entered on
  the negative side). The mirror half of the selection draws in a dimmer
  selection colour.
- Falloff handles: Linear shows its Start/End handles and line, Radial its
  Center and per-axis Size handles with outlines; points of the active mesh
  are tinted by weight. Dragging a handle moves it in the view plane. How
  the reference's falloff handles behave wasn't measured.
- Add Loop / Add Point: after the click that places it, holding the button
  slides the new loop along the clicked edge with a live percent beside the
  pointer; releasing keeps it. One undo step. (AL-Q01 is still open for
  the reference's own pointer behaviour.)

## Preferences and navigation (Jason-specified, 2026-11-02), provisional

- NAV-Q01: the Operandi (native) mouse preset keeps the chords already built
  (Alt + left drag orbit, + Shift pan, + Ctrl zoom; wheel zooms; middle and
  right buttons lasso). The reference's full native mouse map (Alt with the
  middle/right buttons, trackpad gestures, zoom direction and speed) isn't
  specified. The Blender and Maya presets follow those apps' widely known
  defaults, as Jason asked.
- PREF-Q01: Undo/Redo use Cmd+Z / Shift+Cmd+Z and Preferences Cmd+, (macOS
  standard, Jason's request; undo is outside the default-shortcuts spec).
  Which Preferences sections and options the reference offers is open.

## LXO export (lxo-export-2026-11-03), provisional

- LXE-IMP-Q01: answered by spec v2 (BEHAVIOR-IMPORT-PROFILE.md). The
  importer admits the LXE profile; the exporter stays minimal. Built.
- LXE-Q04 handling: empty meshes are left out (refused if they parent other
  items), polylines and zero-area polygons are left out, subdivision
  polygons are written as FACE, unused points are dropped. All with notes.
- XYZ-rotation items are converted to the same rotation in ZXY angles,
  since only order "zxy" is verified (LXE-12, LXE-Q03).
- UV corners with no value on a point that has one take the point's value in
  LXO: VMAP/VMAD can't say "unmapped corner".

## LXO legacy import (lxo-legacy-import-2026-11-03), provisional

- LEG-IMPL-Q01: answered by v3 (LEG-13): only `visible` is read from a
  4.x mesh item; curve gradients and their envelopes are skipped silently.
- LEG-04, 20-byte rest: "0004 in place of either middle 0002" is read as
  0004 in any one of the three 0002 positions.
- Unknown item kinds in 4.x files are omitted with a note. A non-modelling
  item linking to a mesh or its transforms (other than as a parent target)
  refuses that mesh (LEG-10/12).
- LEG-IMPL-Q02: answered by v3 (LEG-14): a 4.x mesh with non-manifold
  edges, zero-area polygons or repeated corners is refused on its own, with
  counts; the rest of the file imports.
- LEG-06 is applied to the Modo 16/17 profile as well (Jason's decision):
  the M16/M17 subd, psub and crease corpus cases now import as cages with a
  note instead of refusing.

## Subdivision (subdivision-2026-11-03), provisional

- SDV-Q01: answered by spec v2 and built (all non-weight SubD oracles pass).
- SDV-Q02: answered by the edge-weights hand-off (EW-05, Operandi choice);
  built, see below.
- SDV-Q07: Face/smooth mixes follow SDV-13 (v3). Subdivision/Catmull-Clark
  mixes follow SDV-14's Operandi choice: a connected smooth region (joined
  by shared edges) that holds both kinds is subdivided as all Catmull-Clark
  at the Catmull-Clark level, so it stays closed. Regions of one kind keep
  their own rule and level. Not tested against SDV-O-MIX-subd3-psub3-L1.
- SDV-Q06: Freeze keeps UVs as a linear subdivision per polygon (SDV-15);
  multi-quad island smoothing, Linear UVs and UV Boundary Rule are open.
  Freeze drops weights on frozen edges; Face polygons' edges keep theirs
  (EW-10).

## Edge weights (edge-weights-2026-11-05), provisional

- EW-05 patch stage, beyond the spec's wording: a patch corner on exactly
  two still-sharp edges uses the average of their midpoints (as written); a
  corner on one sharp edge keeps the normal control; a corner on three or
  more stays where the Catmull-Clark step put it. Each blends by the
  edges' carried sharpness (capped at 1). Questions for Research only if
  someone sees a bad shape.
- EW-06 (replaced by edge weights v2, Jason 2026-11-05): negative weights
  do nothing on Catmull-Clark but soften Subdivision (Tab) polygons by
  EW2-04's interim rule: the step-1 edge point and the patch edge controls
  blend by the weight itself (−1..0) away from the edge. Edge mode draws
  them like positive weights of the same size. Values beyond ±100 % are
  stored as typed and act as ±100 % (EW2-01). Additive (EW2-05) adds to
  each edge's weight; the field then starts at 0.
- EW-07: Operandi's Edge Crease tool is now the Edge Weight tool (input-map
  name "Edge Weight"; the command id stays edge.crease). An empty selection
  refuses with "Select edges to weight", like the other edge tools.
- EW-09: Bridge and the other tools that make new edges give them no
  weight; Add Loop and Edge Insert split a weighted edge into two weighted
  halves. Extrude, Bevel and Smooth Shift keep weights on the moved copies
  of weighted edges (their earlier specs); only their new side edges start
  at 0 %.
- LXO: Modo's Subdivision weight map is still not read (LEG-06 note).

## Vertex tool options (vertex-tool-options-2026-11-03), provisional

Built: Use Material / Material Name on Vertex Bevel and Vertex Extrude, and
Vertex Slide's Interpolation (Percent/Distance) and Merge Vertices. Every
other option in OPTIONS-TOOLS.md is shown greyed at its recorded default
(or "not recorded") with a hover note naming its open question.

- VTO-T02 Use Material on Vertex Bevel is guide-only (G); built the same way
  as Polygon Bevel's measured rule (BV-R16): the new cap gets the name as
  typed, empty included. Confirm with the VTO-Q01 settings dump.
- VTO-T07 Distance: the range wasn't recorded. Operandi refuses a Distance
  that would carry a point past its neighbour (same limit as ±100 %).
  Edge Slide lets points pass (ES-05); is Vertex Slide the same?
- VTO-T07 Merge Vertices: a point at exactly ±100 % (or the full edge
  length) merges with the neighbour it reached, as Edge Slide does (ES-04).
  Two selected neighbours sliding onto each other don't merge. Unmeasured
  on points (VTO-Q07).
- VTO-Q03: Width 0 still builds nothing; Width 0 with a non-zero Extrude is
  refused with a message until Modo's result is measured.
- VTO-Q04: unchanged. Merge (and the Merge Tool) put a group at the most
  recently selected point when points are selected, and at the midpoint
  only when nothing is selected, which is the case CMD-V-01 measured. The
  three-point selection-order check would settle both.
- VTO-C07 Set Position (World Space / Work Plane) and VTO-C02 Make Polygon
  Type are shared with the Polygon panel and weren't touched here.

## Polygon tool options (2026-11-03)

Tool options that have no measured behavior are shown greyed out at a fixed value with a
tooltip, not guessed: Pen Merge, Make Quads, Wall Mode/Offset/Inset/Segments,
Show Angles/Handles/Numbers, Make UVs, Project To, other Types and Tool
States; Chamfer Edit Mode (other than Inset), Absolute Size, Swap Editing
Side, Reverse Direction/Depth, Profiles; Sketch Extrude Move by Path,
Uniform, Align to Path, Scale, Spin, Make UVs, U/V Repeat, U Rotate,
Profiles; Bridge and Polygon Reduction Tool States.

- PEN-Q-I01 (interactive Pen, Type/Select New): Type Polygon/Vertices/Polyline,
  Flip and Close now work in the interactive Pen with the numeric Pen's
  measured meanings (PEN-N02/N03). Not measured for mouse drawing: the
  selection after Drop with Select New on (Operandi selects the new polygon
  or line; for Vertices, the new points), and the fewest points the
  reference accepts for Polyline (Operandi: 2) and Vertices (1). Setup: empty
  mesh, Polygon mode, Pen Type X, click N points, Drop. Wanted: element
  counts, representation and selection per type and N.
- BRI-Q-CH01 (Bridge Continuous / Close Bridges, BRI-R10): the twelve
  S-chain oracles don't follow one corner-matching rule for the spans after
  the first. Checked with S-chain_012_1_0 (sections 0,1,2): span 1→2 pairs
  4↔8, 5↔11, 6↔10, 7↔9 (polygon order, no reversal) although reversing
  section 2 gives smaller distances and a zero shape mismatch; with order
  0,2,1 the same pair of sections is matched reversed; with 2,1,0 span 1→0
  pairs [4,5,6,7]↔[2,1,0,3], neither the nearest nor the stored order.
  Rules tried and rejected: nearest corners (raw or centroid-relative),
  reverse iff normals oppose, stored polygon order. Also, the first span of
  S-chain_012 (two same-facing parallel caps) is matched without reversal,
  while Operandi's two-section Bridge always reverses the second loop: an
  unmeasured two-section case that may differ today. Wanted: the rule (or
  more cases) for how each later span picks its direction and starting
  corner, and a two-section case with same-facing caps. Continuous and
  Close stay greyed until then.

## Identity on moves — 2026-11-03 (Jason decision)

Position-only changes (Move/Rotate/Scale of components, any edit that keeps
the same points, polygons and element kinds) keep every point, polygon,
corner and edge identity instead of issuing new ones; topology changes still
do. This amends the I2 gap plan's frozen `otherMove*` states (see its
README). It is an Operandi design decision, not a reference observation.
## Tweak — 2026-11-03 (provisional readings, slice TWK-1)

Built from specifications/tweak-2026-11-03 (v0, guide-only: nothing in it is
measured yet). The spec's own questions TWK-Q01..Q15 aren't repeated; these
are the readings Operandi chose for TWK-1. Each is easy to change once the
hands-on checks are in.

- TWK-Q02 (Auto priority): a point within 12 px of the pointer wins, else an
  edge within 12 px, else the polygon under the pointer (the selection
  pre-highlight's radius). Hidden points and edges don't count. A press over
  empty space does nothing.
- TWK-Q04 (drag plane): the grab point (the point itself, the nearest point on
  the edge, or where the pointer meets the polygon) moves in the plane facing
  the camera through it, so it stays under the pointer at its own depth.
- Only the active mesh can be tweaked; elements of other meshes aren't picked.
  Research: can Tweak grab an element of a mesh that isn't the active one, and
  does that change the selection?
- TWK-Q05/Q06: a click without moving does nothing (no handle yet, TWK-3) and
  records nothing.
- TWK-Q13 (Symmetry, built 2026-11-03 on Jason's report): the grabbed
  points' counterparts (matched like Select Symmetrical, found once per drag)
  move by the mirrored offset; points on the plane stay on it; the side the
  element's centre is on moves as dragged (an element centred on the plane
  counts as the positive side). Hover shows the counterpart in the mirror
  colour. Research: confirm against TWK-Q13's check.
- TWK-Q15: Esc during a drag isn't special-cased (as for Move); the drag
  still ends on release and can be undone.
- A Falloff from the context bar refuses Tweak (TWK-17 Mix Mode not built).
- No tool.set ID is in the spec; Operandi uses `tweak`. No toolbox button yet
  (T, Tools > Tweak and the command line start it): the toolbox layout and a
  Tweak icon are Jason's call.
- Not built: Always Raycast, Mix Mode, Slip UVs, the handle, Position X/Y/Z,
  snapping (TWK-3). Right-drag Range is built (Tweak part 3). The panel shows the options greyed.

## Tweak part 2 — 2026-11-04 (TWK-2, spec v1)

Mode, Range with the shape presets and Connected Elements are built from
specifications/tweak-2026-11-03 v1 and pass its 13 movement oracles
(relative weights). Provisional readings:

- TWK-Q16 (3D vs screen distance): falloff distance is the straight 3D world
  distance to the nearest point of what's grabbed, and only the frontmost
  element is grabbed. In the C2 oracles both overlapping corners got weight
  1; Operandi's tests reproduce that by grabbing both (spec reading a). A
  single corner grabbed in Operandi gives its hidden twin 1 − 1/Range, not 1.
- Range is measured in world metres (a scaled item's points are measured
  after its scale).
- TWK-Q11 (Edge Loops): the loop runs straight through each point (the next
  edge is the one there sharing no polygon with the last) and stops where
  that isn't unique (poles, triangles, cube corners) or at its start. With
  Range > 0 the falloff is measured from every point of the loop (spec: treat
  it as Ignore plus the loop for now). Vertex and polygon picks move only
  their own points.
- Use Connectivity and Rigid count pieces joined through polygons. With
  Symmetry on, the counterpart's piece takes part too.
- Symmetry with Range: the falloff spreads from the grab and from its
  counterpart; each point takes the mirrored offset on the far side and stays
  on the plane when it's on it.
- Ease-In, Ease-Out and Custom use the falloff spec's curves (TWK-Q09).
- Center modes put the grab point (and so the drag plane) at the element's
  center; falloff is still measured from the element's points (TWK-12).
- The range sphere showed on hover and during a drag; Tweak part 3
  (TWR-12) replaces that: it shows only during a right-drag. Shape is greyed
  with Rigid too, like Range.
- Options last while Tweak stays active and reset to the defaults (Auto,
  0 m, Ignore, Linear) when it's chosen again, like the other tools.
## Tweak part 3: right-drag Range — 2026-11-04 (Tweak Range spec v1)

Built from the Tweak Range spec v1 (TWR-06..14). Provisional readings:

- TWR-Q01 (rate): Range changes by 2 screen points of sphere radius per
  point of horizontal travel, using the view's scale at the sphere centre
  at the press. At the default view in an 800-point-high viewport, 100
  points of travel give about 1.9 m (spec target ~2 m per inch).
- TWR-Q02 (centre): the surface point under the pointer (any visible
  mesh), else the work plane under it, else the camera target's depth.
- TWR-08: the change adds up move by move, so after Range hits 0 reversing
  grows it straight away (no dead zone back to where 0 was reached).
- TWR-10 vs navigation: Shift, Ctrl and Cmd change nothing; Alt + right-drag
  stays the navigation zoom of the Operandi preset (and other presets' right
  buttons stay theirs). Research: does the reference let Alt + right-drag
  resize Range, or zoom?
- Rigid greys Range (TWK-15), so a right-drag with Rigid does nothing and
  says so. Research: does the reference still resize Range with Rigid?
- TWR-Q03 (smallest non-zero Range): not limited; any positive value the
  drag reaches is kept.
- Range field units (TWR-02: 500 mm, 100 µm) are not part of this piece; the
  field shows metres as before.

## Edge Bevel / Edge Chamfer options — 2026-11-03

Built from specifications/edge-bevel-chamfer-2026-11-02: all 21 first-batch
cases and 20 of 31 batch-3 cases match (tests/edge_bevel_measured.rs); the
options have their own tests (tests/edge_bevel_options.rs).

Now following the reference: Width mode (EBC-B3-01), Square and Sharp
(EBC-P-02, B3-08), edges ending on an open border (B3-06), corner patches
past two rows (B3-04: the face layouts and the even centre match), the
even centre's UV (B3-05), Edge End Triangle Fan (B3-07, geometry of the
ends) and Edge Chamfer keeping an interior end point (B3-06).

Provisional, for Jason to take back:
- EBC-Q02: Edge Chamfer Width uses Width·sin(θ/2), which fits the 90° and
  135° samples but not the capped 60° prism (0.136819 measured, 0.1 here).
- EBC-Q08: inner corner-patch points for 3+ rows are Operandi's own rule
  (grids with straight far sides; odd rows' central polygon 1/n the corner
  size); measured inner points differ by up to about 0.1 of the width, and
  odd-row UVs don't match. Junctions other than three edges, and the planar
  T's inner points, are open.
- EBC-B3-06 / B3-07 departure: where Edge Chamfer keeps an interior end
  point, the reference makes zero-area end polygons (two coincident
  triangles on G3; a pentagon and triangles on the T). Operandi refuses
  zero-area polygons, so their points join the strip's end polygon instead
  (10 polygons instead of 12; 25 instead of 33).
- EBC-Q10 (new): Stop At Edge, Auto Weld, Parallel off and Use Material
  aren't measured. Operandi's readings: Stop At Edge stops a side point at
  the far end of its edge, but only with Auto Weld on, which merges it
  there; without Auto Weld (or for a corner point) a width that long is
  refused, since the polygons would fold (off: it runs past, like Edge
  Slide's ES-05); Square strips meeting at a corner keep the original
  corner point;
  Auto Weld merges new points that land on another point; Parallel off
  moves the Offset along the side edges; Use Material names the new
  polygons. Options and Segments/Round Level/Depth are kept between uses of
  each tool; the width starts at zero. Square/Sharp and Width at junctions
  follow the single-edge rules.
- EBC-Q11 (new, Jason 2026-11-03): on a 2×2-segment cube, Edge Bevel of
  an edge from a face centre to an edge midpoint is refused by the
  reference rather than sliding the midpoint along the cube's edge into a
  triangle. Operandi now refuses an end slide where the polygons around
  the end point bend more than 20° (provisional threshold; gentle curves
  still work);
  the measured flat-grid ends (UVN-T-01, EBC-B3-06) still work. Edge
  Chamfer with Profile keeps the point, so it isn't affected; Triangle Fan
  is refused the same way. Open: the reference's actual threshold.
- EBC-Q12 answered (v1, measured 2026-11-10; project file
  edge-bevel-shape-2026-11-10/EBC-Q12.md, rules EBS-01..05): Square and
  Sharp keep the original edge at every Level (Level 0 too) and ignore
  Level, Segments and Depth; Depth blends Round's middle points linearly
  from flat to the arc and does nothing at Level 0. Built. Still open:
  EBS-05, Square vs Sharp on a non-90° edge (identical at 90°).
- EBC-Q03 still open: Mitering, Coplanar, Flatness, Sharp, Sharp Corner,
  Boundary Extend, Boundary Coplanar and Reverse X/Y are shown greyed out at
  their Reset values. Each needs at least one measured case where it
  changes the result.
- EBC-Q13 (new, forum report 2026-11-22): Edge Bevel / Edge Chamfer on a
  mesh with a vertex normal map (any mesh with hard edges has one) used to
  refuse every edge. *Provisional:* it now carries the map: a rebuilt
  polygon's corners keep the normal of the corner they came from, the new
  strip and end polygons' corners stay unassigned (as Add Loop leaves its
  new corners). HardEdges entries for edges the bevel removes or splits are
  dropped on Drop, so a later Recalculate treats the strip as soft. What
  does the reference do with the normal map and with HardEdges on a
  bevelled hard edge?
- EBC-Q14 (new, forum report 2026-11-22): Edge Bevel of a border edge (an
  edge with one polygon, e.g. a seam of unwelded points) is refused ("supports
  edges between two polygons"); EBC-B3-06 covers edges that *end* on a
  border, not border edges themselves. What does the reference make when a
  selected edge is a border edge (strip on one side only? refused?)?

## Undo and Auto-Save preferences — 2026-11-04 (provisional readings)

Spec: specifications/preferences-undo-autosave-2026-11-04/UNDO-AUTOSAVE.md.
Max Undo Level 100 is measured; every other default below is provisional.

- Defaults: Group Selection Commands off, Max Sub-Commands Recorded 100,
  Auto-Save off, Time Interval 10 minutes, Backup Directory `autosave` in
  Operandi's data folder, Number of Revisions 5 (AS-Q1). Ranges: undo levels
  and sub-commands 1–10,000, interval 1–1,440 minutes, revisions 1–100.
- UND-Q1: a group ends at any other recorded step, Undo, Redo, or any scene
  change that wasn't a selection click; a pause doesn't end it. Turning
  grouping on lets the selection step just made keep growing.
- UND-Q2: lowering Max Undo Level trims on the next recorded step, so
  nothing is lost just by changing the number.
- PREF-Q2: no Apply/OK or reset-to-defaults; settings save as they change,
  like the other pages.
- AS-Q2: the countdown is a small panel at the top centre of the 3D view. An
  unchanged scene (matching its file or the last backup) gets no countdown
  and no backup. A countdown that ends while a tool is open waits and backs
  up once the tool is dropped (an armed Move/Rotate/Scale with nothing
  pending doesn't hold it up).
- AS-Q3: backups are named `<scene>.autosave-0001.opscene` (`Untitled` for a
  new scene), counting up per scene name; the lowest number is removed past
  the revision count. On quit only files named like that and older than 31
  days are deleted; nothing else in the folder is touched.

## Navigation preferences — 2026-11-04 (provisional readings)

Spec: specifications/preferences-2026-11-04/NAVIGATION.md. Built on
Preferences > Input > Remapping: Tooltip Delay (0.8 s, measured), Mouse
Wheel Acceleration (100 %), Wheel Zoom at Mouse Cursor (on), Invert Mouse
Wheel Zoom (off), Zoom Drag Axis (Bottom to Top) and Invert Mouse Wheel
Scrolling (off, lists and panels only). Those defaults keep Operandi's
earlier behaviour and are provisional (NAV-P-Q1).

- Not built: Input Device (tablet), Allow Modifier Chording, Numeric Enter
  as Tab (egui doesn't tell keypad Enter apart), Open Dialogs under Mouse,
  Reverse Pan Widget Direction (no pan widget), Trackball Rotation and its
  options (the camera has no roll yet), Invert Drag Scrolling.

## Selection and tool handle preferences — 2026-11-04 (provisional readings)

Specs: specifications/preferences-2026-11-04/SELECTION.md and
HANDLES-SNAPPING.md. Every default except OpenGL Point Size 3 keeps
Operandi's earlier behaviour and is provisional (SEL-P-Q1, HND-P-Q1).

- Input > Selection: Initial Selection Type (Polygons; applies to the new
  scene at launch), Selection Rollovers None/Manual (Manual), OpenGL Point
  Size (3), Selected Point Size (1.6×), Selection Hit Size (12 px; a bigger
  drawn point is clickable across its size), Polygon Selection (Frontface &
  Wireframe), Lasso: Ignore Shading Style (off), Lasso: Allow Partial
  Element Selection (off; it's the same switch as the lasso's Partial
  toggle, now remembered).
- SEL-P-Q4 (new): what "3 px" means on a Retina screen. Operandi maps 3 to
  its earlier 2.5-point dot radius and scales from there. Polygon Selection
  applies to the lasso only; click and paint picking still never pick hidden
  elements. Frontface only and Always Raycast behave the same, as do
  Frontface & Backface and No Raycast; the middle-button lasso reverses
  only the default mode.
- Display > Tool Handles: Handle Line Width (2.5), Handle Line Hit Width
  (10), Handle Points Hit Size (14, one size for all handle points), Handle
  Scale: Standard (1× = the earlier 72-point arrows), Selected Tool Handles
  (Basic; Basic and Advanced look the same because Operandi's tools have one
  level of handles; Invisible hides them).
- Not built: Closest rollovers, Mouse Regions Trigger Selections, Selection
  and Handle transparency, Double-Sided Selection, Point Selection Priority
  (type unknown, SEL-P-Q2), separate Large/Small points, Small and Plane
  handle scales, Unselected Tool Handles, the -/= keys, snapping ranges and
  Rotation Snap Angle (Operandi has no snapping or Ctrl-constrained
  rotation yet).


## Multiple open scenes — 2026-11-04 (provisional readings)

Spec: MULTI-SCENE.md v0.1 (settled: File > Current, "Untitled (2)" names,
New/Open add a scene, Close Scene prompts, last close leaves a blank scene,
Close All confirms, undo/auto-save preferences app-wide). Open items and
what Operandi does meanwhile (scenes.rs):

- MS-Q1/Q9 Items list (v0.2): every open scene is a root row; the current
  one is bold; each scene is a parent with its own items under it (Jason:
  a scene is the one object that cannot be parented), and
  a click on one makes it current. "(new scene)" is the last row. Right-click
  offers Save, Save As…, Export As, Close Scene. Order is opening order (a
  new scene goes after the current one). Unsaved scenes show " •" in File >
  Current only. Not built: scene rows' visibility/edit/reference columns and
  scene settings in Properties.
- MS-Q2 Per scene: geometry, selection and mode, undo/redo, path, saved
  state, view cameras, collapsed Items rows, active tool with its unconfirmed
  fields. Shared: tool presets, symmetry, falloff, action center, layout,
  display settings.
- MS-Q3 Switching mid-tool: the tool stays live in its scene and is there on
  return; switching is refused mid-drag or mid-selection gesture.
- MS-Q4 Open fills an untouched new scene (never saved, unchanged, no
  history; settled in v0.3); opening a file that is already open switches to it. Import still
  replaces into a new scene like Open (the spec's "append into the current
  scene" is not built).
- MS-Q5 Prompt "Save <name> before closing it?" Save/Discard/Cancel; Cancel
  leaves that scene current. The next tab to the right (else left) becomes
  current. A clean scene closes silently. Cancelling the Save As panel
  cancels the close.
- MS-Q6 Quit asks one scene at a time in tab order; Cancel on any prompt
  aborts the quit and keeps every scene's changes (Discard is only applied
  when the app actually quits).
- MS-Q7 Max Undo Level limits each scene's history separately. Auto-save
  backs up each changed scene under its own name ("Untitled (2)"), and
  switching away from a changed scene backs it up then. Each open scene has
  its own recovery journal (recovery.opscene, recovery-2.opscene, …); Recover
  restores all of them into tabs.
- MS-Q8 Settled in v0.3 (measured): one more than the highest open
  Untitled number; gaps are never reused.
- MS-Q10 Dragging items between scenes (v0.3: allowed, materials go along;
  the rest open): a plain drop moves the item and its children; Option
  (Alt) while dropping copies them (Jason confirmed, 2026-11-04;
  `scenes::DRAG_COPIES`).
  The copy keeps its world position, lands under the row it's dropped on
  (the scene row: top level), gets a free name, and the receiving scene
  becomes current. Undo is per scene: the receiving scene gets "Insert
  Items"; a move also leaves "Move Items Out" in the scene it left.

## Units and grid preferences — 2026-11-04 (provisional readings)

Spec: specifications/preferences-2026-11-04/UNITS-GRID.md. Measured: Inch
Scale Cutoff 36, Viewport Fit Factor 1.5. Provisional (UNI-Q1): Unit System
SI, Default Unit m, Meters per Game Unit 1, Grid Type Dynamic, fixed Size 1 m.

- Distance Units: every distance field and the command line show and read
  the chosen units; scenes stay in metres. SI, Metric and Game Units show
  values in the Default Unit (the reference may pick a unit by size; to
  check). English shows inches up to the cutoff, then feet and inches
  (3' 6"), and reads ', ", in, ft, yd, mi, mil. A number typed with no unit
  anywhere is in the Default Unit; once a term has a unit, plain numbers are
  factors (1 m/4 = 25 cm). Unitless: 1 = 1 m. Game units read as gu.
- Grid: a Fixed grid keeps its size until its lines would be closer than
  the dynamic grid's, then spaces dynamically (GRD-Q2: it doesn't drive
  rounding; Operandi has no coordinate rounding yet).
- Fit Factor: Fit frames the bounding sphere so that at 1 it touches the
  view's shorter side; Operandi's earlier framing was about 2.
- Not built: Input As Current Unit, Color Unit System, Factor or Range,
  Coordinate Rounding / Fixed Increment, Up Axis, dynamics and light units,
  Preferred Plane / Bias / Lock / Offset, Plane and Grid Increments,
  Flatness of Perspective, a Grid Visibility default for new views, the
  grid-size readout in the view's corner.

## Tool defaults, overlays and file info — 2026-11-04 (provisional readings)

Specs: specifications/preferences-2026-11-04/MESH-TOOL-DEFAULTS.md and
OVERLAYS-UI.md. Measured: Author and Copyright empty. Provisional (MTD-Q1,
OUI-Q1): Default Action Center None and Axis World in both component and
item modes (Operandi's start-up context so far). Overlays: Current
Selection, OpenGL Polygon Count and Grid Size on by default, the rest off
(provisional, Jason 2026-11-06); a preferences file saved under the old
all-off defaults is upgraded once.

- Operandi has one action context, not one per mode: the component or item
  default is applied at launch and for each new scene, by the scene's
  starting mode. "Last Selected" isn't offered yet.
- 3D Information Overlays: Current Selection, OpenGL Polygon Count (reads
  "GL: n", the triangles drawn for visible items, subdivision included:
  quad 2, n-gon n-2; Jason 2026-11-13),
  Grid Size, Polygon Type (face, Catmull-Clark or mixed, from the
  polygons' subdivision flag) and Current Tool (with falloff, action centre
  and axis), at the lower right of each 3D view (the lower left overlapped
  the axis gizmo and view labels).
- Current Selection reads "X Polygons", or "All Polygons" when nothing is
  selected (likewise Vertices, Edges); in Item mode the item's name, "N
  Items" or "No Items". Drawn in the selection colour. No totals (Jason
  2026-11-06: "n of N" was unwieldy on big meshes).
- Author and Copyright go into glTF/GLB exports (asset.copyright, and
  asset.extras.author since glTF has no author field). Other formats have
  nowhere standard to put them.
- Not built: all Mesh Items rows (item transforms, child hiding, UV map,
  vertex-map selection, new subdivision levels, primitive polygon type,
  symmetric primitives, flatness limit, copy/paste selection rules), Names
  (Default Material, Texture Map, Item Index Style), Auto Activate, Select
  Through, Snapping State overlay, the UI rows (Tidy Layouts, popups,
  proficiency, icon size, tool properties layout), Use OS Color Picker,
  Reset "In the future…" Messages, Text Encoding, Sync Display Refresh, Max
  GL Polygon Count.


## Backdrop items — 2026-11-04 (provisional readings)

Spec: specifications/backdrop-2026-11-04 (BD-01..BD-52). The spec's own open
questions BD-Q01..Q17 are built with these defaults, each easy to change:

- BD-Q01: clicking a picture in a viewport never selects it and passes
  through to meshes; select Backdrops in the Items list.
- BD-Q03 (answered v2): Auto Fit for Back, Bottom and Left uses the smallest Z, Y, X face.
- BD-Q04: Auto Fit resets the out-of-plane scale to 1 (answered v2,
  BD-33). Box (BD-61, 2026-11-09, provisional): the combined box of the
  active mesh items, meaning every selected mesh, hidden ones included;
  with a Backdrop selected on its own, the mesh that stays active (VP-04;
  hiding that one stops it being active, so it no longer counts). The
  "every visible mesh" stand-in is gone.
- BD-Q06: (2026-11-09, BD-12 built) a Camera Backdrop draws only in a view
  looking through a camera, filling that camera's render frame exactly
  (stretched to its shape); its position, scale and Pixel Size don't
  apply. Provisional: Flip and the colour options apply; the camera is the
  one being looked through; it draws over the grid like other Backdrops
  (the reference draws the grid over it); with no picture it shows no
  placeholder.
- BD-Q07: new Backdrops take Use Uniform Size and Pixel Size from the
  Display preferences (factory: off, 10 mm); still unconfirmed in the
  reference.
- BD-Q08: Backdrop Size uses Width and Height fields on the panel and turns
  Keep Aspect off to hit both exactly; Backdrop Size Keep Aspect uses Width
  and turns Keep Aspect on.
- BD-Q09: the Image list offers (none), the scene's loaded pictures and
  Load Image… (PNG or JPEG). Pictures stay in the scene's list after no
  Backdrop uses them; there's no way to delete an entry yet.
- BD-Q12: every change is one undo step; a number-field drag is one step.
- BD-Q13: a missing file opens silently in the scene, draws nothing, and the
  panel and status bar say "Backdrop picture not found".
- BD-Q14 (v2, one sample): where Backdrops overlap in one view, the first
  created is on top (BD-11b).
- BD-Q15 / BD-11a (decided, Jason 2026-11-04): the reference draws the
  picture over meshes in Perspective (BD-O-60); Operandi deliberately keeps
  meshes on top there, as in the orthographic views.
- BD-Q16 (answered v2, BD-27): a PNG's alpha mixes linearly with what is
  behind.
- BD-Q10, BD-Q11: Gamma/Blur values are kept if a file has them, never shown;
  the older per-view Background Imagery isn't built.

New questions:

- **BD-Q18** Operandi's free-rotating Orthographic view: does it show
  Backdrops like Perspective (Show In Perspective) or like the fixed views
  (only when looking straight along the projection)? Built: like Perspective.
- **BD-Q19** Command names for Create Backdrop, picking the image, the
  options, Auto Fit and the two size buttons. The spec gives none, so they
  follow Operandi's `<domain>.<verb>` style: `backdrop.create`, `.image`,
  `.attr <name> <value>`, `.autoFit`, `.size <w> [<h>]`. Research: please
  record the reference command names and arguments so we can match them.
- **BD-Q20** (answered v2) An image entry is named after its file without
  the extension ("front.png" becomes "front"); the same file again reuses
  its entry; another file with a taken name gets " (2)", " (3)".
- **BD-Q19** (answered 2026-11-04) Command names are Operandi's own choice;
  built as `backdrop.*` to match poly.*, edge.*, prim.*.

### Backdrop gaps BD-60..62 — 2026-11-09 (provisional readings)

- **BD-Q21** (BD-60) A picture dropped into a view makes a Backdrop facing
  that view: Front/Back/Top/Bottom/Left/Right give their own projection;
  Perspective, the free Orthographic view and a camera view give Front. The
  drop (image entry + Backdrop) is one undo step. Only the main window's
  views count as 3D views; a drop on a torn-off view, a panel or the
  toolbar only loads the image entry. Still pictures are recognised by
  extension (png, jpg/jpeg, tif/tiff, exr, hdr, bmp, tga). Scripted with
  the map's names (UIC-165): `backdrop.add image:<path> project:<view>`
  (project defaults to Front, BD-02) and `clip.addStill filename:<path>`.
- **BD-Q22** (BD-62) Pen Make UVs (Project To: Backdrop Item) uses the
  Backdrop drawn in the axis view the points are drawn in (its projection
  is that view's); if several, the topmost (first created, BD-11b). Hidden
  Backdrops, Backdrops with no picture, and views with Show Backdrop off
  don't count. Flip mirrors U (u → 1 − u). Perspective, the free
  Orthographic view and camera views keep world X/Y even when they show a
  Backdrop. The view is the one of the most recent captured point.

## New primitives: Torus, Capsule, Arc, Tube — 2026-11-04 (provisional readings)

Spec: specifications/new-primitives-2026-11-03 (NPR). All 36 oracles pass
(NPR-O-TUB-05 within 6 mm, see below). Readings built where the spec is open:

- NPR-Q03 Torus Bulge: Ellipsoid's superellipse (exponent 2/(2+bulge)),
  Bulge Side around the axis, Bulge Top on the cross-section. Bulge Side
  matches TOR-05 exactly; Bulge Top's oracle has only cardinal rows, so it
  doesn't test the formula.
- NPR-Q04 Capsule Quad Grid: Sides a multiple of 4 get an (S/4)×(S/4) grid
  patch inside the last end ring, interior points by a Coons patch lifted
  onto the rounded end (exactly the 8-side oracle). Other Sides counts use
  Polar ends. With more End Segments the end rings stay and the grid sits
  inside the innermost one.
- NPR-Q05 Capsule Bulge: the same superellipse around the axis (matches
  CAP-04-bulge0.5 exactly).
- NPR-Q06 Arc links: moving Center keeps the plane through the new Center
  and the old Start/End, points Start from the new Center toward the old
  Start, keeps Radius and Angle (matches ARC-05 when Center is entered as
  one vector). Entering Start keeps End's direction and sets Radius;
  entering End keeps Start and Radius and sets the plane and Angle. The
  panel enters one component at a time, so typing End 0,0,-1 field by field
  passes through End = Center (ignored). Question: are Center/Start/End
  entered as whole vectors or per component?
- NPR-Q08 Tube curve: cubic Hermite per span; tangent at a point =
  (|out|·in + |in|·out)/(|in|+|out|) of its two chords; open ends
  extrapolate a parabola through the end three points (P−1 = 3P0−3P1+P2).
  Exact for TUB-02/03/04. First ring's reference direction (ty², −tx·ty,
  −tx) fits all planar oracles; for tangents with a Z component it is a
  guess. Rings follow rotation-minimizing frames. By Length: even arc-length
  spacing; the reference spacing differs by up to 5 mm in TUB-05, so the
  reference's length estimate is something else. Twist: ring k turns
  Twist·k/(rings−1) (closed: k/rings).
- NPR-Q09 Capsule with End Size equal to the half-length: the body collapses
  to one ring (matches CAP-04-axisX).
- NPR-Q02: Make UVs for Torus and Capsule Polar match the default oracles;
  Capsule Quad Grid ends and Tube caps get no UVs (sparse corners); Tube
  walls are unrolled (U around, V along). Polygon Type, Flip, Distortion
  Compensation and Gaps aren't offered.
- NPR-Q01: no viewport drawing or handles; values come from the panel or
  MCP. Arming previews straight away (Ellipsoid waits for a value) so Drop
  alone makes the shape. Tube waits for two points.
- NPR-Q11: the target mesh must have no transform (as Ellipsoid).
- Clean-room note (2026-11-04): build thread was shown a reference-app
  screenshot of panel layout (section grouping only); no other details used.
  The attachment was relayed to this thread but not opened; only Jason's
  words ("Tube and Arc belong … under Draw") were used.

## Undo while a tool is active — 2026-11-04 (provisional readings)

Jason asked for Cmd+Z to work mid-tool, primitives first (Cube, Sphere,
Cylinder, Ellipsoid, Cone, Torus, Capsule, Arc, Tube). No spec yet; these
readings are built and marked provisional (`history_policy.rs`,
`primitive_session`).

- MTU-Q1: with a preview that isn't a step yet, one Cmd+Z throws the preview
  away and keeps the tool active; it uses no history step and Redo doesn't
  bring it back. Does the reference discard it, record-then-undo it (as
  Operandi's transform tools do, HND-Q04), close the tool, or do nothing?
- MTU-Q2: after that Undo a drawn Cube/Sphere/Cylinder restarts empty, as
  after activation; numeric tools keep the values typed in their panels.
- MTU-Q3: each further Cmd+Z removes one applied shape with the tool still
  active; numeric panel values stay as they are (Cone still puts back the
  values stored with its own steps, as before).
- MTU-Q4: Cmd+Z keeps going past the tool's own steps to earlier work, with
  the tool still active.
- MTU-Q5: Redo works mid-tool, but is refused while a preview shows.
- MTU-Q6: when Undo/Redo removes the tool's mesh (or makes it unusable,
  e.g. a transform), the tool closes.
- MTU-Q7: Cmd+Z during a drag is not blocked: it discards the preview and
  the drag draws it again.
- MTU-Q8: the Edit menu shows "Undo <tool name>" while a preview shows.
- MTU-Q9: other tools (surface, edge, vertex) still refuse Undo while
  active; next in line. Do they follow the same answers?
- MTU-Q10: Cancel after mid-tool Undos only closes the tool.

### Mid-tool undo spec v1 (2026-11-04) — built and still open

Jason decided (2026-11-04) that inside a tool each tweak is its own undo
step and the dropped tool is one step. That replaces the spec's separate
document steps for Move drags (U7) and Cone value changes (U2: after
Apply, Sides 12, Apply, the second Undo now removes the first cone with
Sides 8 back, instead of only reverting Sides). Built from the spec:
activation is an undo step (MU-MID-02/03), Escape/Q clear Redo (MU-MID-04).

- MTU-Q11: once a numeric primitive Applies, its earlier tweaks belong to
  that Apply step; Undo past it doesn't revisit them. Provisional.
- MTU-Q12: does activating a tool clear Redo (activation being a step)?
  Built: no.
- MTU-Q13: Shift-drag a second Cube (U5b): the first cube's step is undone
  with the tool closed, not reactivated.
- MTU-Q14: Undo of "Drop Cone" re-arms Cone; from there Undo keeps going
  past Cone's steps with Cone armed (its activation point isn't known).
- MU-MID-05 (plain "Undo"/"Redo" menu labels) is not applied: Operandi's
  menu still names the step (MTU-Q8). Jason's call whether to switch.

## Smooth shading (smooth-shading v0, 2026-11-03)

Built readings, all *provisional* until they are observed:

- SS-Q03 Each neighbour is compared with the corner's own polygon directly
  (no chains), weight 1, angle measured between world-space normals; an
  exact 40° step blends.
- SS-Q04 Amount is a linear blend from the flat normal toward the smooth
  one, then normalized; the field accepts -100 % to 200 % (Operandi
  budget). Cycles renders get the angle rule but not the amount.
- SS-Q05 Material boundaries: there is only one default material, so
  polygons with different material tags still blend.
- SS-Q07 Two untagged polygons blend by angle; an untagged and a tagged one
  never blend (Crease and Force).
- SS-Q09 The subdivision preview keeps its fully smooth normals; the angle
  rule only applies to cage polygons.
- SS-Q-OP01 (Operandi) Polygons carry an older per-polygon `smooth` flag (from
  OBJ `s` groups and glTF import). It no longer changes shading, since the
  material settings apply to every polygon. Should OBJ `s` groups become
  smoothing group labels on import?

## Drag anywhere (Jason's rule, 2026-11-04)

Jason: a click-drag anywhere in the viewport drives the live tool, not only
a drag on its handle; up/down acts like the move arrow, left/right like the
scale widget. Dragging a handle still edits only that handle. Each
press-to-release is one undo step; the tool stays active. Built readings,
all *provisional* until the reference's off-handle rules are specified:

- DRAG-Q01 Which value each tool's drag anywhere drives:
  - Bevel: up = Shift, right = bigger cap (less Inset, like dragging the
    Inset square out).
  - Smooth Shift / Thicken: up = Offset, right = Scale.
  - Edge Extrude, Vertex Extrude: up = Extrude, right = Width.
  - One size-like value from left/right: Chamfer, Edge Bevel, Edge Chamfer
    (Offset/Inset), Edge Relax (Offset), Edge Crease, Vertex Merge
    (Distance).
  - Edge Slide, Vertex Slide: as if the rail handle were dragged.
  - Edge Extend, Vertex Tear: free move in the view plane.
  - Poly Haul: up/down hauls from anywhere (was: only over a selected
    polygon).
  - Sphere, Cylinder after drawing: the nearest side handle, as the Cube
    already does (BOX-Q02).
  - Spike: up = longer spikes (Strength, 0.01 per pixel).
  - Bridge: right adds a Segment per 12 px, left removes (drag stops at 64).
  - Polygon Reduction: the press initializes; left lowers Number, right
    raises it (0.5 % of the source per pixel, up to the source count).
  - Torus, Capsule, Arc, Tube: drawn from the press on the construction
    plane like Cone and Ellipsoid (size from the drag, proportions kept;
    Tube: each click adds the next control point and a drag places it;
    changed 2026-11-04, Jason: a click used to restart a two-point tube).
    Replaces NPR-Q01's "no viewport
    drawing", and the "dragging isn't available" refusals of BRI-Q05,
    SA-Q01 and RED-002.
  Unchanged: Move/Rotate/Scale/Transform, Extrude, Vertex Bevel, Tweak,
  Sketch Extrude, Cube, Cone, Ellipsoid already drove from anywhere; Create
  places points by clicking. Modified drags off the handles stay blocked.

## Oracle replay fixes — 2026-11-04

From the spec-side replay of 2026-11-03 (behaviour note), rerun in
`crates/operandi-mcp/tests/oracle_replay.rs`. Open items, for Jason:
- ORQ-01 Polygons the mesh rules refuse. The reference builds an inverted
  or self-intersecting inner polygon when Bevel's Inset passes the centre
  (PB-C-01/04: PB-O-B4-square-inset0.5/0.6, rect-inset0.6, B5-L-inset0.6),
  a keyhole polygon that repeats two points when merging a ring with a
  hole (CMD-P-05), and two coincident zero-area triangles at a chamfered
  edge's interior end (EBC-B3-06). Operandi refuses all three because its
  meshes don't allow self-intersecting polygons, repeated corners or
  duplicate faces. Allowing them touches validation, triangulation and
  every tool. **Decided (Jason, 2026-11-04): no, keep refusing them for
  now; revisit if it breaks compatibility with LXO files.** The oracle
  replay keeps these cases pinned as known departures.
- ORQ-02 PE-O-02 sideways Extrude still keeps separate coincident points
  (PE-O-E1-nonplanar-shiftX, PE-O-E5-moved-item); needs the extrude
  correspondence and the live drag fast path to allow merged points.
- ORQ-03 Order only: Select Between with three picks and Symmetry picks
  (mirrored elements after the picked ones) give the right elements in a
  different list order.
- ORQ-04 Provisional: Merge now selects the merged polygon (newer oracles)
  where the basic Merge spec of 2026-10-28 cleared it.
- ORQ-05 Provisional: Edge Bevel with more than one row keeps selecting
  every strip edge; the oracles select only the strips' outer sides
  (EBC-O-E3-02/04), not yet replayed through the bridge (Round Level).
- ORQ-06 Provisional: opening a native scene whose meshes have mixed winding
  or three-polygon edges records them as the importers do. Tools that
  refuse such edges still refuse them (Align and Quadruple work).

## Extrude starting Width (2026-11-04)

- EXW-Q01 Edge Extrude (ME-T02) and Vertex Extrude (MV-T03): what Width does
  a fresh tool start at? Start: a cube, one edge (or point) selected, tool
  just activated. Action: drag only the Extrude arrow. Width 0 refuses any
  Extrude, so the tool did nothing. *Provisional*: Width starts at a tenth
  of the shortest edge touching the selection (`START_WIDTH_SHARE`). Is the
  reference's start a fixed value, the last-used Width, or something else?
- EXW-Q02 Edge Extend (ME-T03) on interior edges. Jason: it works with more
  than one neighbouring polygon. *Provisional*: the new strip is a fin wound
  like the first neighbour, and the edge ends up shared by three polygons.
  Which way should the fin face, and is the source edge kept shared?
- EXW-Q03 Edge Extrude end caps. Jason's correction: on a cube edge the
  old corner points go away, each end polygon runs straight between the two
  slid points, and the cap is one triangle. Built where an end has exactly
  one other polygon holding both side edges. Elsewhere (more polygons
  around the end, or a border) the old corner stays with a quad cap
  (*provisional*). What does the reference do there?


## Selection Sets (2026-11-04, provisional readings)

Built:
Assign, Use (Replace/Select/Deselect), Delete (one or all of the mode),
Create Selection Set Group and Delete Group on the active mesh, for vertex,
edge and polygon sets. Start for every question: a cube in the matching
selection mode with some components selected.

- SET-Q01 Assign Selection Set with a name that already exists for this
  mode. *Provisional*: the selection is added to the set; nothing is
  removed. Does Assign add, replace the set's members, or refuse?
- SET-Q02 What does the Assign dialog offer by default: an empty name, a
  generated name, or the last name used? *Provisional*: empty; OK stays
  greyed until a name is typed. Are names trimmed, case-sensitive, limited?
  (*Provisional*: trimmed, case-sensitive, at most 256 characters.)
- SET-Q03 Use Selection Set in Replace: does it also clear the other
  modes' selections or switch mode? *Provisional*: it only replaces the
  current mode's list and keeps the mode (the older `select set <name>`
  route still switches to Polygon mode).
- SET-Q04 Hidden or locked components that belong to a set: does Use
  select them? *Provisional*: yes (component hide/lock is not built).
- SET-Q05 Create Selection Set Group: which sets go in? *Provisional*: the
  ticked sets of the current mode, or all of that mode's sets when none
  are ticked; an existing group name gains the sets. Can one group mix
  vertex, edge and polygon sets, and can a set be in two groups?
  (*Provisional*: one mode per command, a set may be in several groups.)
  What can you do with a group afterwards (use it, collapse it in a list)?
- SET-Q06 Deleting a group's last set leaves the empty group.
  *Provisional*. Should the group go too?
- SET-Q07 Edits that change topology. *Provisional*: an edge that an edit
  removes leaves its edge sets; points renumbered by Cleanup, Collapse,
  Smooth Shift and Reduction carry their edge sets; new edges never join a
  set. Polygon Reduction refuses a mesh with a non-empty edge set. Extrude,
  Spike and Polygon Bevel still refuse any mesh that has a selection set
  (audit X2); what should new geometry from those tools inherit?
- SET-Q08 Item selection sets (SEL-081 lists items) and Convert to Group
  (SEL-083) are not built. Is the Select tab meant to cover items too?

## Modifier-key button variants (2026-11-04, provisional readings)

From specifications/modifier-buttons-2026-11-04.

- MB-Q8 Primitive buttons. The older PM spec routes Ctrl-click to a unit
  primitive in the existing mesh and Shift-click to one in a new mesh, takes
  the keys at mouse *release*, and does nothing while a text field has
  focus. The MB spec says Shift → "{Name} Item", Ctrl → "Unit {Name}", keys
  fixed at mouse *press*, swaps while typing. *Provisional*: primitives keep
  the PM behaviour. Move them onto the MB table (and which Shift meaning)?
  **Decided (Jason, 2026-11-05): yes.** Shift "{Name} Item" = 1 m primitive
  as a new mesh item; Ctrl "Unit {Name}" = 1 m primitive in the current
  mesh; MB rules apply (keys at press, swap while typing).
- MB-Q9 Edges button: Shift → "Bounds" and Ctrl → "Boundary" are both
  described as boundary selects with no stated difference. Start: a cube in
  Polygon mode with two adjacent polygons selected; hold Shift, click Edges;
  then Ctrl. *Provisional*: both select the border edges of the selected
  polygons (Operandi's Boundary). What does each select?
  **Decided (Jason, 2026-11-05, option A):** Bounds replaces the selection
  with the border edges; Boundary adds them to the existing edge selection.
  Both switch to Edges.
## Set Material dialog (2026-11-04, Jason's description)

Built from Jason's description of the dialog (no research spec yet).
Fields: Name (editable, with a list of the scene's materials), Type
(Default Material only), Color (linear RGB 0–1, swatch opens the macOS
color picker), Diffuse 80 %, Specular 4 %, Smoothing on, Make Default off,
Add to Library off. OK assigns the tag and stores the settings in one undo
step; an existing material of that name is updated.

- MAT-Q01 Make Default and Add to Library are stored with the material and
  saved, but do nothing yet. What should each do: Make Default (does it
  change the Default material, or the dialog's starting values, or the
  material new polygons get?) and Add to Library (where is the library,
  and what is saved there)?
- MAT-Q02 *Provisional*: "Default" is the default material. Assigning it
  clears the polygons' tag (empty), and its settings are stored only when
  they differ from the factory values, so untouched scenes keep the theme's
  viewport colour. Is that right, or should Default be an ordinary tag?
- MAT-Q03 *Provisional*: the Color numbers are linear (render) values and
  the macOS picker shows them converted to sRGB. Are the reference's
  numbers linear or display values?
- MAT-Q04 *Provisional*: Cycles uses base colour = Color × Diffuse, and
  Specular as the straight-on reflectance (4 % is IOR 1.5). Roughness stays
  at the render's default 0.5. The viewport shows Color alone. What does
  Diffuse do to the viewport colour in the reference?
- MAT-Q05 *Provisional*: Smoothing off shades the material's polygons flat
  (viewport and Cycles). The angle and amount still come from the scene's
  smoothing settings. Does the reference keep smoothing angle and amount per
  material?
- MAT-Q06 LXO saves material tags (as before) but not the material's colour,
  diffuse, specular or smoothing: the LXO export spec covers no material
  items, and LXE-00 says not to improvise items Modo hasn't been checked to
  read. Research needed: the material item/mask layout and channels Modo
  17.1 writes for a basic material, for both export and import.
- MAT-Q07 *Provisional*: OBJ keeps tag names (usemtl) only; no .mtl file is
  written or read yet. glTF saves the colour (× Diffuse) and Specular (as
  KHR_materials_ior), with Operandi's exact settings in `extras` for a
  round trip. STL, SVG and GEO have no materials.
- MAT-Q08 *Provisional*: the name-only command `poly.setMaterial {Name}`
  assigns the tag and keeps any stored settings; it no longer refuses
  "Default", which now returns polygons to the default material.
- MAT-Q09 *Provisional*: Solid, Gooch, Cel and Reflection display styles
  also use the material colour, since they all tint the same fill colour.
  Should any of them stay neutral?

## Cameras and lights (CL-1, spec v0, 2026-11-04)

Built from the docs-only v0 spec. Every value below lives in
`crates/operandi-core/src/render/camera_light_defaults.rs`; change it there.

- CL-Q01 New-scene contents. Every *new* scene gets "Mesh 1", then
  "Camera", then "Directional Light"; the mesh stays selected. Opened and
  imported files are left as they are. *Provisional guesses*: the camera
  sits 4 m from the origin, tilted down 15° (at (0, 1.04, 3.86)), aimed at
  the origin with no roll; the light sits at (2, 3, 2) rotated (−50°, 35°, 0°)
  XYZ, so it shines down and away from the default view. What are the
  reference's camera height/pitch and light rotation (and its position)?
- CL-Q02 Defaults. Focal Length 50 mm is a guess. Film 36 × 24 mm, offset 0,
  Target Distance 4 m, Clipping off are from the spec. Near/Far Clip when
  Clipping is on (0.01 m / 1000 m) are guesses. Orthographic size:
  *provisional*, the frame a perspective camera with the same lens sees at
  the Target Distance. Render frame: Cycles renders from the scene camera
  at the 720 × 480 film-back frame (the viewport camera keeps the viewport
  size); whether the render item uses that frame is unknown.
- CL-Q04 Intensity. Directional Intensity defaults to 3 W/m² (guess).
- CL-Q07 Film Fit defaults to Horizontal (guess). Fill matches widths when
  the picture is wider than the film, else heights; Overscan the opposite.
- CL-Q08 Axis. Cameras look and lights shine down local −Z with +Y up, as
  in Cycles. Is that the reference's convention?
- CL-Q09 Shapes. Camera: a box body behind the lens and a frustum out to
  the film frame with a small "up" triangle (0.3 m scale). Directional light:
  a disc with a 1 m arrow and four short rays along its direction. White,
  selection colour when selected (theme colour "Cameras and Lights").
  Clicking a shape in a view doesn't select it yet; use the Items list.
  Items rows show placeholder line glyphs until generated icons exist.
- CL-Q11 Camera view / undo. The view menu's "Camera" looks through the
  render camera (the one chosen with Make Render Camera, else the first
  camera). Orbit, pan and dolly use the normal mappings and move the camera
  item; each press-to-release is one undo step written on release (each
  wheel step is its own step). Dolly also changes Target Distance so the
  orbit point stays put. Roll and focal-length drags are not handled; the
  view is always upright (a rolled camera loses its roll when navigated).
  Film Offset is not shown in the viewport (Cycles uses it).
- CL-Q14 Brightness. Cycles sun strength = Intensity × 1 (W/m²), × colour,
  × (1 − Dissolve). Spread Angle → sun angle. Shadow Type None → shadows
  off. Render Off, or Render Default on a hidden item, skips the light.
  Partial Affect Diffuse/Specular are not passed on (0 % turns that ray type
  off). A scene with light items no longer gets the studio sun; one without
  keeps it.
- CL-Q16 Which views show them. Every view (each has Show Cameras / Show
  Lights in its Display menu, on by default); the camera a Camera view looks
  through isn't drawn there. The spec suggests perspective views only.
- Deferred: the light's colour and Affect Diffuse/Specular are plain light
  settings until a shader tree exists. LXO import/export of cameras and
  lights is CL-3 (export leaves them out; import adds none).

## Point, Spot and Area lights (CL-2, spec v0, 2026-11-04)

Values in `camera_light_defaults.rs`.

- CL-Q03 Creation. Item > Create Camera and Item > Create Light >
  Directional / Point / Spot / Area (also under Add Item): the new item sits
  at the origin with an identity transform, is the only selected item, and
  is one undo step. Names "Point Light", "Spot Light", "Area Light"
  (" (2)" and so on when taken). All guesses.
- CL-Q04 Intensity defaults (guesses): Point and Spot 10 W/sr (Radiant
  Intensity), Area 10 W/sr/m² (Radiance). Point/Spot Radius 0 (a true
  point). Area 1 × 1 m Rectangular; the item's scale multiplies Width and
  Height.
- CL-Q05 Spot cone. Cone Angle is the *full* cone (default 40°); Soft Edge
  (default 10°) lies inside it: a 40° cone with a 10° edge is fully lit
  within a 20° inner cone and fades over the outer 10° on each side. Spots
  shine down local −Z (CL-Q08).
- CL-Q14 Cycles conversions (guesses): point W = I·4π, radius as is; spot
  the same, spot size = full cone, blend = clamp(edge / (cone/2), 0, 1);
  area W = L · area · π (area = w·h, or π/4·w·h for an ellipse), as a Cycles
  rectangle or ellipse. Falloff None / Inverse Distance render as inverse
  square; the render's status line says so.
- CL-Q15 Affect Diffuse/Specular between 0 % and 100 % render as 100 % (0 %
  turns that ray type off), with a status note. Shadow Color is not built.
- CL-Q09 shapes (CL-2): point = three-axis star with a diagonal cross, plus
  three Radius circles when Radius > 0; spot = cone lines to a ring 1 m out
  with the inner full-strength ring; area = its rectangle or ellipse with a
  short normal along −Z. Each light also shows a dot in its own colour.

## Cameras and lights: observed defaults (spec v1, 2026-11-04)

Spec v1 observed these; they replace the CL-1/CL-2 guesses above and are now
the values in `camera_light_defaults.rs`:

- New scene: Mesh, Camera, Directional Light in that list order. Camera at
  (0, 0.75, 4), rotation X −5°; Directional Light at (−2, 2, 2), rotation
  X −30°, Y −45°, order ZXY (was CL-Q01).
- Camera: Perspective, Focal Length 50 mm, Film 36 × 24 mm, Film Offset 0,
  Film Fit **Fill** (was Horizontal, CL-Q07), Target Distance 4 m, Clipping
  off with Clipping Distance 4 m, Lens Squeeze 1, Film Roll 0, Display Size
  0, Overscan 100 %. Angle of View checked at 35/50/85 mm (54.432223°,
  39.597753°, 23.913168°).
- Render resolution 1920 × 1080 (was 720 × 480). Fill at that frame matches
  the gate's width so the 3:2 gate covers the 16:9 frame.
- All lights: Intensity 3.0 (was 10 for point/spot/area, CL-Q04), Falloff
  Inverse Distance Squared, Visible to Camera / Reflection / Refraction off,
  Samples 64, Importance 100 %, Display Size 1, directional Shadow Map Size
  2 m, Point and Spot Radius 0. Light material: colour white, Shadow Color
  black, Affect Diffuse/Specular/Caustics/Subsurface 100 %, enabled, opacity
  100 %. The material's channels are stored on the light, so it belongs to
  its light and goes when the light is deleted.
- Spot: Cone Angle 45°, Soft Edge 5° (were 40°/10°, CL-Q05). Area 1 × 1 m
  Rectangular.
- Create Camera / Create Light: at the origin, identity transform, only
  selected item (CL-Q03).

Built but not used yet (stored, shown read-only under "Not used yet"):
Lens Squeeze, Film Roll, camera Display Size, Overscan, Visible to
Reflection/Refraction, Samples, Importance, light Display Size, Shadow Map
Size, Shadow Color, Affect Caustics/Subsurface, material Enabled, Opacity.
Clipping Distance is used as the near clip plane when Clipping is on (far
1e5 m), a provisional reading.

Still guesses (as logged above): one undo step per creation (CL-Q03), the
intensity unit labels (CL-Q04), Cone Angle as the full cone and Soft Edge
inside it, spread angle as a full angle (CL-Q05), Film Fit details for
Fill/Overscan (CL-Q07), the −Z facing for cameras and every light (CL-Q08;
spec v1 infers one shared −Z convention, needs an eye check), viewport
shapes, handles and icons (CL-Q09), which camera Camera view uses and the
frame region (CL-Q11), Cycles brightness conversions (CL-Q14, Q15), and
whether every view shows cameras and lights (CL-Q16).

- CL-Q16 decided (Jason, 2026-11-05): modeling views don't draw cameras or
  lights by default, even the selected one. Each view's Display menu has
  Show Cameras / Show Lights to turn them on for that view
  (`SHOW_CAMERAS`, `SHOW_LIGHTS` in camera_light_defaults.rs). Camera view
  never draws the camera it looks through. This replaces the earlier
  provisional "every view shows them".

## Cameras and lights: spec v2 answers (observed, 2026-11-05)

- CL-Q08 answered: cameras and every light face local −Z at identity, as
  in Cycles; the export uses no axis flip or negative scale. (The mirrored
  renders fixed in 075692a were a wrongly rebuilt right axis, not a
  convention difference.)
- CL-Q05 answered: Cone Angle and directional Spread Angle are full angles
  (Cycles spot size = cone, sun angle = spread). Soft Edge is an angle
  inside the cone, measured in from the edge; strength is a smoothstep in
  cos(angle) from cos(h) (zero) to cos(h − e) (full), h = cone/2. Cycles'
  own spot falloff is that smoothstep with blend = (cos(h − e) − cos h) /
  (1 − cos h), now used (`LightSettings::spot_blend`). Check values for a
  40° cone, 10° edge: 1.0 at 10°, 0.62 at 15° (observed 0.60), 0.16 at 18°
  (observed 0.14), 0 at 20°; no blend matches the observed pair more
  closely while keeping full to 10° and zero at 20°.
- CL-Q07 answered: Film Fit as observed (36 × 24 film, 1920×1080 /
  1080×1920 cover 36 × 20.25 / 13.5 × 24 Fill, 36 × 20.25 / 36 × 64
  Horizontal, 42.7 × 24 / 13.5 × 24 Vertical, 42.7 × 24 / 36 × 64
  Overscan). The existing rules already gave these; tests now pin them.
  Camera view and Cycles share the rule.
- CL-Q02 (clipping) answered: Clipping Distance is the near plane → Cycles
  clip start.
- CL-Q14 answered for point, spot and sun: pixel = albedo/π · irradiance,
  irradiance I/d² (point/spot, W/sr) or I (sun, W/m²). Cycles render test
  (albedo 0.6 plane, I = 3 at 2 m, linear): point and spot 0.1434 (target
  0.1426), sun 0.5735 (target 0.5703), both within 0.6 %, with the existing
  conversions (point/spot power I·4π, sun 1:1). Falloff None / Inverse
  Distance (targets 0.5703 / 0.2852) still render as inverse square, with
  the status note. Area brightness (L·area·π) is still unmeasured.
- CL-Q16 answered: the model perspective view shows no cameras or lights by
  default, matching Jason's decision.

## Set Part upgrade (specifications/set-part-2026-11-04)

Built on the spec's provisional answers unless noted.

- SP-Q01 Nothing selected names every surface polygon of the active mesh,
  in one undo step. Line polygons are left out of that case until curve
  tagging (SP-07) is built.
- SP-Q03 *Decided by Jason (2026-11-04)*: the Name field opens with the
  part the target polygons already share, otherwise the Default Part
  preference. This is a deliberate Operandi choice.
- SP-06 Operandi has one active mesh at a time, so "several active meshes"
  reduces to that mesh. Revisit when multi-item selection exists.
- SP-14 Set Part now sits directly above Set Material. Both still sit
  between Collapse and Spin Quad, not after Align as the spec says;
  moving the pair waits until the toolbox layout work settles.
- SP-15 Geometry > Polygon > Set Part... is added after Bridge; Assign
  Material Group... and Set Smoothing Group... don't exist yet.
- SP-16, SP-20 Not built in this slice: part export to OBJ/FBX, and the
  Select tab's Polygon Tag dropdown. The shared helpers it needs are in
  `operandi-core/src/polygons/polygon_tags.rs` (`PolygonTagKind`,
  `Scene::polygon_tag_names`, `faces_with_tag`, `tag_target_faces`).
- Set Material still refuses an empty selection (MAT-001); applying the
  shared "nothing selected = whole mesh" rule to it is a follow-up once
  both branches are in main.

## Create panel v2 (create-panel-2026-11-04, draw-tools-2026-11-05)

Built from the behaviour hand-off. Jason decided (2026-11-05) to take the
sane option over the reference's quirks: Paste Tool in a moved item uses
the world Center, and Merge keeps a deleted mesh's children where they are
in the world. Everything below is an Operandi guess unless it says
otherwise; the draw-tools supplement's **choice** items are research's
guesses, not Jason's, so they are listed here too.

- Units: Ctrl-click (this mesh) and Shift-click (new mesh) on Ellipsoid,
  Cone, Torus and Capsule now make a unit shape, like Cube/Sphere/Cylinder.
  Sizes are guesses: Cone fills a 1 m box; Torus ring 0.375 m, cross
  section 0.125 m; Capsule 0.5 m wide, 1 m tall, end size 0.25 m. Each
  keeps the last Sides/Segments used.
- Tube: each click adds a point at the end of the path; pressing within
  8 px of a point drags it. No insert-after-current, delete or close yet.
- Text: fonts are Ubuntu Light (default) and Hack (both bundled), then the
  fonts installed on the Mac. Letters with holes (o, e, B…) become several
  hole-free polygons, because an Operandi polygon can't have a hole. Not
  built: editing the text again after Drop, and viewport handles for size
  and tracking. Make UVs is off by default.
- Copy is not an undo step; Cut and Paste are one each. Paste goes into the
  active mesh, or a new mesh when there is none, and selects what it
  pasted. Copied lone edges paste as two-point lines.
- Paste Tool: each click centres the clipboard's bounding box on the
  construction plane under the pointer, scaled by Size, plus Bias.
- Merge needs several meshes selected: Ctrl-, Shift- or Cmd-click rows in
  the Scenes list (the first one selected survives). With fewer than two,
  the Merge button shows a hint in the status bar, where the spec (MRG-09)
  shows nothing. Unmerge names new meshes "<name> (2)", "(3)"…
- Ground Align (supplement choices): Connected Geometry always on unless By
  Selection; Rotate To Ground turns by the shortest arc about the piece's
  centre, uses the average normal for vertex/edge selections, and does not
  drop (GAL-03 vs GAL-09). Right-click is Rotate To Ground.
- Quick Align (supplement choices): releasing away from every arrow does
  nothing but a status hint; the arrow pointing out of the target face
  means same facing (QAL-07); no Adjust after an apply (QAL-08); Target
  Mode Local Axis / Origin, Rotate Before Flatten and Tear Off UVs are not
  built. Arrows are 5 % of the view distance long. One undo step per
  release. Shortcut Shift+R.
- Ruler: starts 200 mm long along X at the middle of the view; ends snap to
  mesh points within 10 px; Angle Snap rounds the direction to 15°.
  Measurements live only while the tool is on and are not saved.
- Dimensions: Orient lines the box up with the selection's main
  directions (principal axes) instead of the world axes.
- The Edit, Merge, Align and Measure groups start folded, so the Tool
  Properties panel keeps room on smaller screens. The Cone panel test
  window is now 1100 px tall because the extra headers pushed its Apply
  button below 1000 px.
- Draw tools (supplement choices, built as written unless noted): new
  points land on the construction plane (DRW-13); Ctrl-drag picks the world
  axis with the larger motion after 4 px (CRV-05); one undo step per click
  or drag for Curve/B-Spline (DRW-Q07), and a curve's first point waits in
  the tool (no undo step) until the second; Drop removes a curve with too
  few points (Curve 2, B-Spline 4, Bezier 2 anchors; DRW-Q08); Start/End
  Control hidden for B-Spline; B-Spline Weight not built.
- Curve shape: Operandi uses uniform Catmull-Rom with a "natural" end
  tangent. The hands-on oracle's span-1 midpoint is (0.5, 0.702254, 0); ours
  is (0.5, 0.6875, 0), so the reference's end rule is something else. Not
  copied exactly; tell me if it matters.
- Curves are line elements (they select, save and copy like polylines) and
  draw smooth in the viewport. LXO export writes them as their straight
  control cage for now (not as CURV/BEZR/BSPL polygons). Filled and Make
  UVs (Curve/Bezier) are not built. Picking a curve uses its control cage.
- Bezier: plain click = sharp anchor; press-drag pulls both handles, the
  handle end following the pointer on the plane (BEZ-07); Tangent Handle
  Keep Tangent / Uniform Length / Break; Shift while dragging a handle
  re-links. Not built: Insert mode, Break and Smooth options (BEZ-Q02),
  reversing with F (BEZ-Q03).
- Sketch: spacing = Precision pixels at the stroke's start (SKT-Q01);
  Curve type fits with Douglas-Peucker within half a spacing; Bezier and
  B-Spline types use the same fitted points (B-Spline adds midpoints up to
  4). Changing any option rebuilds the last stroke while it's still the
  last undo step. Not built: right-drag Width/Precision (SKT-05), stroke
  handles, Uniform Spans, Curve Interpolation, Connect, Flip, Make UVs, the
  Start/End/Axis fields (SKT-Q02).
- Draw Patch: ends within 0.1 mm count as shared, because Operandi's drawn
  curves have their own end points (the reference needs merged points).
  The surface is a Coons patch spaced by arc length (PCH spacing choice),
  faces the viewer when built (PCH-05), and is always frozen into quads;
  there is no live spline patch, so editing a boundary curve afterwards
  doesn't rebuild it. Not built: modes other than Define Patch, Knots,
  Save Boundaries off, 3-curve patches (PCH-Q01/Q02).
- Pen: Shift-click ends the polygon and starts the next (seen). Drop with
  too few points: a two-point polygon leaves two points, one point is
  discarded (seen); this replaced the old "refuse and keep the draft" rule,
  and three Pen tests were updated to match. Not built: Merge (drag a point
  onto another), Make Quads, Inset/Segments, snap aids, Topology Pen. Pen
  also has a button in Create > Draw now.
- Pen Wall Mode (built after Jason's QA, 11-05; choices are mine, not seen):
  the clicked path becomes a flat strip of quads in the drawing plane, with
  mitred bends (capped for very sharp ones). Both Sides centres it on the
  path (2 x Offset wide); Inner and Outer put an Offset-wide strip on one
  side (Inner = right of travel, seen from the plane's normal). Turning
  walls on with Offset 0 sets it to 0.1 m. Two points are enough for a wall.
  Right-drag left/right sets Offset live. Walls need Type Polygon. Not
  built: Inset, Segments, standing (extruded) walls. Open: is "wall" meant
  to stand up out of the plane? If so, say so and it becomes an extrude.
- Torus, Capsule and Arc now draw out like Cone and Ellipsoid (Jason QA
  11-05): arming the tool shows nothing until a drag or a panel edit. The
  instant shape is only under Unit (Ctrl/Shift-click). Text still shows its
  remembered lettering at once.
- The Create layout follows the hand-off except Merge has its own group
  (the README also lists Unmerge/Merge as a row under Edit).


## Snapping (snapping-2026-11-05), provisional

- SN-08: no stickiness; the handle lets go as soon as it leaves the Inner
  Range. With Vertex and Grid both on, the nearer on screen wins, and a
  100 % grid always offers a crossing.
- SN-09: points of other visible meshes are targets; Operandi has no hidden
  points yet.
- SN-11: Grid Distance below 100 % snaps within that fraction of half a
  square of a crossing, on both axes.
- SN-12: the Create panel's Draw tools don't snap yet (follow-up).
- SN-07: only the Default preset; named presets wait.

## Cameras and lights: spec v3/v4 (Jason's hands-on observations, 2026-11-05)

Built from v3/v4; values in `camera_light_defaults.rs`.

- CL-Q16 confirmed: hidden by default in the Model view (already built).
- CL-13 glyphs (shapes observed, sizes guessed): light grey unselected,
  selection colour when selected. Camera: movie-camera body with two reels
  and a lens on −Z, a constant ~48 points on screen (Display Size 0);
  selected: dotted frustum and frame out to Focus Distance. Directional:
  cylinder along Z (1 m across, 1 m long at Display Size 1), solid line
  and filled arrowhead 1 m along −Z. Point: 1 m star (three axes, four
  diagonals). Spot: lamp housing 0.5 m long opening to −Z; selected: dotted
  cone 2 m long with Cone Angle spread and the Soft Edge's inner ring.
  Area: Width × Height rectangle (or ellipse) in local XY, dotted centre
  cross, solid line and filled arrowhead half the smaller side along −Z.
  *Guesses*: every size, line count, arrow length, the reels, the dotted
  centre being a cross (one line or two?) and the frustum length.
- CL-14 tool channels: with Move, Rotate or Transform active on the selected
  camera or light in Item mode, Tool Properties adds its own channels: area
  Width/Height, spot Cone Angle/Soft Edge, camera Focal Length (and Focus
  Distance, spec v4), point Radius; directional none. Each change is a tweak
  of the tool (its own Undo inside the tool, kept on Drop, undone by
  Cancel). Scale doesn't show them (not in the observation).
- CL-14 handles (positions observed, drags *guessed*, spec v4): area square
  grabs mid +X side and mid top, resizing about the centre; spot solid
  circle at the cone edge and dotted circle at the Soft Edge start, 90
  points down −Z on screen (the distance follows the view; *guess*), grab
  crosses on radius lines at +X (Cone Angle) and +Y (Soft Edge); camera
  grab cross at Focus Distance on the axis (drag along the axis edits it;
  the cross on the camera body is the transform tool's own centre handle,
  spec v4.1, so nothing extra is built there); point Radius as three great circles with crosses at +X and +Y,
  or one centre cross at Radius 0 that grows it. Circles light purple,
  crosses and squares cyan (theme colours "Light Handle Circles/Grabs").
  One tweak (undo step) per drag.
- Focus Distance added to cameras (default 1 m, observed); only the frustum
  and its handle use it until depth of field exists.
- CL-17/18 Camera view looks through the selected camera, or else the newest;
  the header shows its name plus "Render Camera" only for the render camera.
  The render frame's darker bands were already built.
- CL-19 Light view (look along the selected spot): *not built*, follow-up.
- CL-21 Items list: own glyph per kind; the render camera row is bold (drawn
  twice, half a point apart, since the app has no bold face) with a render
  marker; "Set as Render Camera" moves both. Order is creation order.
- Intensity labels: Radiant Exitance (directional), Radiant Intensity (point,
  spot), Radiance (area); the unit reads "W/srm²" for every kind, as
  observed, even though a directional light's exitance is W/m² (odd; kept).

Spec v4.1 answers: CL-Q03 answered, one Undo removes a just-created camera
or light (as built); one Undo reverts a whole Alt+drag orbit in Camera view
(as built).

Still open: what the spot/camera/point handle drags really do, area-light
brightness (CL-Q14).

## Shader tree skeleton (ST-1, 2026-11-05)

Built on the hand-off's placeholders (specifications/shader-tree-2026-11-05/
QUESTIONS.md). Operandi-side choices, all *provisional*:

- ST-Q01 Set Material names the new group after the tag and its material
  "<tag> Material". The hand-off doesn't say what the rows are called.
- ST-Q02 (Shader: superseded by Tier 1 G-20, a new Shader with nothing
  selected goes to the very top) Add Layer puts every new layer under the Base Shader as "the lowest
  shader directly under Render". If Jason moves Base Shader, new layers
  follow it.
- ST-Q03 A tree that only differs from the default by layers added and
  deleted again saves like a new scene (version 13 or lower).
- ST-Q04 Tree edits are refused while a tool is live or a Pen draft is open,
  like Set Material.

## Shader tree first render (ST-4, 2026-11-05)

Hand-off v0.2 §8 rules are built as measured. Operandi-side choices for the
first version, all *provisional* ("close enough and sane"):

- ST-Q05 Channels onto Cycles' principled shader: Base Color = Diffuse Color ×
  Diffuse Amount; Metallic = Metallic + Reflection Amount (Reflection Color
  unused); Specular Amount sets the IOR as before when the surface isn't
  transparent, else IOR = Refractive Index and Specular IOR Level scales the
  reflectance; Alpha = 1 − Dissolve (× Alpha Value for Constant alpha);
  emission = Luminous Color at Luminous Intensity. Sheen Tint, Flatness,
  Fresnel, Transparent Color and Absorption aren't used yet.
- ST-Q06 Coverage = opacity × texture alpha × the Layer Mask under it (Q-17);
  a Layer Mask's own opacity fades it toward "no mask" (Q-11).
- ST-Q07 Two materials at partial opacity mix per channel (Q-16).
- ST-Q08 Normal Multiply and Normal Map Blend act as Normal (Q-12; still
  open now that normal maps render, see ST-Q22). Hard Light is Overlay with
  the sides swapped.
- ST-Q09 Every projection renders as UV Map for now; images use the polygons'
  UVs.
- ST-Q10 Not rendered yet: drivers, stencil, fur, Object Normal, Vector
  Displacement; Shader effects other than Full Shading; Receive Shadows
  (Cycles has no per-object switch). (Selection-set masks render since Tier 1
  G-01; Bump, Normal and Displacement since G-07, ST-Q21..Q26.) Answered for
  bump/normal/displacement by the G-07 audit.
- ST-Q11 Polygons whose result equals the default tree's keep the studio clay
  render, as before. A scene whose tree was never edited renders its
  material table mapped into the default tree, and that mapping becomes the
  tree on its first tree edit or Set Material (one undo step).
- ST-Q12 (superseded 2026-11-06 by ST-Q13) The viewport still shows the
  material table, not the tree.
- ST-Q13 Viewport colours come from the shader tree (2026-11-06,
  provisional): each polygon shows Diffuse Color × Diffuse Amount of the
  tree folded for its item, material and part, the same fold renders use,
  so item, item-group and tag masks, group and layer opacity and blend
  modes all show. Polygons that come out at the untouched tree's colour
  keep the theme's plain fill. Image Map layers show as the picture's
  average colour until the viewport draws image maps; a missing picture
  is mid grey. GL Display off on the winning Shader hides the textures in
  the viewport; GL Display on a Material isn't used yet. Only the colour
  reaches the viewport (no specular, roughness, luminous or transparency
  there yet). Smoothing still comes from the material table, as in
  renders. Because Diffuse Amount now counts, a stored material looks a
  little darker than before (80 % by default).

## Shader tree Tier 1 (specifications/shader-tree-audit-2026-11-06), provisional

- ST-Q05 (answered 2026-11-06, research hand-off "shader settings -> Cycles
  translation map v0.1", fixes 1-4; branch material-brightness) Principled:
  IOR 1.5 and Specular IOR Level = Specular Amount, so 50 % reflects 0.04
  straight on and 100 % 0.08, a straight line (the old curve to 0.9 is gone
  from renders). Base Color = Diffuse Color; Diffuse Amount is ignored
  (Physically Based: × Diffuse Amount). Transparent Amount mixes the
  principled surface with a Transparent BSDF (Refractive Index ≤ 1.0) or a
  Glass BSDF (IOR = Refractive Index, roughness = Refraction Roughness),
  both in Transparent Color; no Principled Transmission; the surface keeps
  reflecting at IOR 1.5. Dissolve/alpha wraps the mix as an outer mix with
  a clear Transparent BSDF. With Match Specular on, Reflection Amount,
  Color and Fresnel have no effect; Metallic = Metallic only. Specular
  Fresnel still isn't used. Measured: a calibration render matches all 11
  reference values within 0.4 %.
- ST-Q27 Match Specular off: Reflection Amount still splits by Reflection
  Fresnel into Metallic and the Coat (the earlier G-02 mapping; Reflection
  Color unused). What does the reference do with Match Specular off?
- ST-Q28 Shading Model Traditional and Energy Conserving render exactly as
  Principled. How do they differ in the reference?
- ST-Q29 Not covered by the translation map, kept as they were: glTF export
  and reading pre-version-15 files still use the old Specular curve
  (`material::specular_reflectance`); a Material layer's Shading Model and
  Match Specular are not blended: the topmost applied Material's win,
  whatever its opacity. Should glTF export follow the new linear mapping?
- ST-Q13 G-04 two-zone block (Zone 1/2 Value, Color, Alpha, Bias, Gain) is in
  core (`shader_tree::TwoZone`, Perlin bias/gain curves, 50 % = identity) but
  no layer carries it yet: Image Map has no zones and Constant keeps its one
  colour/value. It joins the first procedural texture. Are the reference's
  bias/gain curves Perlin's?
- ST-Q14 Image Map adjustment order: swizzle, alpha mode, per-channel invert,
  gamma (out = in^(1/gamma), so 2.2 brightens), brightness (multiply),
  contrast (about 0.5), clamp to 0..1; scalar effects then map luminance from
  Low (black) to High (white). Is the order and the gamma direction right?
- ST-Q15 Swizzling: RGBA/RGB use the colour, Alpha/Red/Green/Blue a grey of
  that channel; RGB also leaves the image's alpha out (full coverage). Alpha
  Only: the alpha is the layer's value and the layer covers fully. Ignore:
  full coverage. Antialiasing, its strength and Minimum Spot are stored only.
- ST-Q16 G-06 Divide uses the v0.4 fit (1 − B + 2L) / (3 (1 + B)); against
  the measured points it is within 0.004 (e.g. B 0.8, L 0.5: 0.222 vs 0.219).
- ST-Q17 G-06 Soft Light uses the v0.4 formulas (B + (2L − 1)(√B − B) above
  0.5); B 0.2, L 0.75 gives 0.324 against the measured 0.320.
- ST-Q20 G-20: Size 1 m and Repeat aren't stored yet (Texture Locators are
  Tier 2); only the X axis start is. A Shader added with a layer selected
  still follows Q-2 (lands in the selected group).
- ST-Q18 Files before native version 15 stored Specular as the reflectance;
  reading them converts every stored Specular (material table, tree
  Materials, Constant layers on Specular Amount) to the amount that renders
  the same, and older glTF exports likewise. Stored values otherwise stay;
  only Reflection Fresnel (0 %), Clearcoat Bump (100 %) and the Image Map
  adjustments (defaults) are new. A Base Material still at the old starting
  values keeps rendering as the studio clay (ST-Q11).
- ST-Q19 G-19: Material and texture layers above the topmost applicable
  Shader (panel order) are left out for that surface; Layer and Group Masks
  are not, and Groups/Folders still blend what they hold below the shader.

## Bump, normal maps, displacement (G-07, branch bump-normal-disp), provisional

- ST-Q21 Displacement dicing: a fixed rate of 1 pixel (Settings.dicing_rate),
  linear subdivision of the render triangles (Operandi's own subdivision is
  applied first and wins), at most 12 levels. The coarse first Preview pass
  dices at its own bigger pixels. Hard edges (split render points) open
  cracks when displaced, since each side moves along its own normal; smooth
  surfaces stay closed. Is a per-material or per-item dicing rate wanted?
- ST-Q22 Normal Multiply and Normal Map Blend still blend as Normal, also on
  Normal and Bump layers.
- ST-Q23 Material layers have no height, normal or displacement of their
  own: they leave those folded values alone (Groups and Folders blend them
  like any channel). Bump Amplitude, Displacement Distance and Clearcoat
  Bump come from the material fold as other channels do.
- ST-Q24 Normal maps: Cycles' Normal Map node, tangent space, OpenGL
  convention (green up); their images are read as plain numbers (no colour
  conversion), while Bump and Displacement images are read as colour like
  every other image. A mesh with UVs on any polygon counts as having UVs.
  Invert on a Normal layer inverts all three components. Are bump and
  displacement images meant to be read without the colour conversion too?
- ST-Q25 Clearcoat Bump: at 100 % the coat follows the bumped/normal-mapped
  normal, at 0 % it stays smooth; in between a second Bump/Normal Map pair
  runs at that strength (Cycles blends normals by strength, not an exact
  scale of the tilt).
- ST-Q26 A constant Displacement moves the whole surface along its normal
  (the spec's 0.5 = half distance). Displaced meshes only change in Cycles;
  the viewport shows neither bump, normal maps nor displacement yet.

## Selection on mixed meshes (select-tab, 2026-11-05), provisional

Jason: a unit sphere refused Grow ("only works on all-quad meshes"). The
all-quad, max-four-edge guard was a first-version bounded scope (selection.md
line 146 allowed "basic regular-quad results"), not a spec rule: SEL-054/055
only say "adjacent". Now:

- Grow and Shrink work on any polygon mesh (triangles, n-gons, poles).
- SEL-Q-LOOP-01 (SEL-058/060, SEL-U05): Loop and Ring no longer refuse a whole
  mesh with triangles, n-gons or poles. Starting state: default Globe sphere.
  Actions: Loop or Ring from one quad/edge. Provisional reading: an edge loop
  stops at a point that doesn't have four edges (a pole ends a meridian loop;
  the edge touching the pole is kept); a ring stops at a non-quad; a polygon
  loop stops at a non-quad and keeps it selected (as the one-seed walk already
  did). If an edge loop can't add anything, it still refuses ("nothing
  changed"). Research: confirm where the reference stops and whether the
  terminating triangle is included in a polygon loop.

## Next / Previous Loop (select-tab, 2026-11-05), provisional

SEL-Q-LOOP-02 (SEL-057, SEL-LP-02/03/04, SEL-B2-01). Built for polygons and
edges from the measured Grid4 cases in specifications/selection-2026-11-02;
they all pass as tests. Provisional readings beyond them:

- Polygons: walk the loop from its first selected polygon towards its
  earliest-selected neighbour; Next takes the right-hand neighbours of that
  walk (by polygon winding), Previous the left. If the whole side is open
  border, bounce to the other side and reverse the order (matches
  [3,7,11,15] → [14,10,6,2]). Next then Previous returns the loop.
- Edges: both commands cross the polygon whose winding runs along the first
  selected edge from its lower to higher point index (the measured X=2 loop
  went to X=1 for both, so Previous is NOT the inverse of Next); an open
  border gives the whole border loop. Research: is there a case where edge
  Previous differs from Next, and what decides it?
- Points: not built. LP-04 (row 9,8,7,6,5 → border, below) and SEL-B2-01
  (column 2,7,…,22 → Next +X, Previous −X) don't fit one direction rule
  that I could find. Refused with a message for now.
- Poles and non-quads: stepping onto a band with triangles/n-gons refuses
  ("stops at a pole"); a band whose side is open only partway refuses.
- Shift+Left/Right "add next/previous loop" (KBD-126/128) not built.
- Left/Right arrows are bound by default (KBD-125/127), like Up/Down for
  More/Less; remappable.

## Import selection mode (item-drag-fast, 2026-11-05), provisional

**IMP-Q01.** Starting state: an LXO, OBJ, STL, glTF/GLB or GEO file opened
or imported as a new unsaved scene. Operandi used to leave it in the
default Polygon mode with nothing selected, where Move moved every point of
the mesh (it looks like moving the object but edits the mesh). Jason chose
(2026-11-05) that an import opens in Item mode with the active mesh, or the
first mesh, selected. Research: what selection mode and active item does
the reference leave after importing a scene or a mesh file?

## BA-Q01 — Polygon Bevel with nothing selected (2026-11-05)

Spec IDs: bevel-activation-2026-11-05 rule 7 (BA11) vs
polygon-bevel-2026-11-02 PB-O-01 / oracle PB-O-B6-empty-selection.

Starting state: one mesh, Polygon mode, no polygon selected (a lone quad
in PB-O-B6-empty-selection). Action: Polygon Bevel, apply.

Unclear: BA11 says applying changes nothing ("Other polygon tools act on
every polygon in that case; Bevel doesn't"). The earlier measured oracle
bevels the quad (4/1 → 8/5 points/polygons). Both were observed in Modo;
is one an interactive apply and the other a scripted one? Operandi keeps
PB-O-01 (bevel every polygon) until this is answered.

## Provisional readings from the B3–B6 specs (2026-11-05)

Built as the most conservative reading; each waits on the spec's own
open question.

- **Q-ST-01** (set-position): World Space and Work Plane both on is
  refused with a message.
- **Set Position, Edge mode, nothing selected** (set-position rule 2 lists
  only Vertex/Polygon empty cases): refused ("Select edges").
- **ST15** (set-position): a rotated work plane can't be set in Operandi
  (its work plane is axis-aligned), so that case isn't checked.
- **Q-CV-04** (command-conventions): unknown argument names and extra
  arguments are refused, never ignored.
- **Q-NM-01** (new-mesh): the item list's New adds the mesh at the end of
  the list, with the selected item's parent.
- **VTO-Q03** (vertex-tool-options): Vertex Extrude still starts with a
  Width (EXW-Q01) though the recorded default is 0.

## Render button and Preview window (render-preview, 2026-11-05)

Picks from Jason's v1 behaviour brief, all provisional until confirmed by
the spec side. Starting state for each: a scene with a mesh, Cycles build.

- **RP-Q1** Keys: F9 = Render (final render into the Render window, or bring
  it to the front while one runs); F8 = Preview (open, or bring to the
  front; never closes). Both remappable in the Input Map. The two buttons
  sit in the top bar right of Work Plane… (Jason's QA, 2026-11-06). The Render
  button's icon is a sphere in a picture frame (generator recipe `render`).
- **RP-Q2** The Preview is a separate OS window titled "Preview", one at a
  time, opening paused; it remembers its size and place
  (preview-window.json). Closing it stops the render and frees the picture.
- **RP-Q3** Engine: progressive Cycles, not a raster preview. Each start is
  a coarse pass (1/8 of the pixels across, 1 sample, no denoise) then the
  full pass with adaptive sampling (threshold 0.05) to 256 samples and
  OpenImageDenoise at the end. It is a new Cycles session per start, not
  one persistent interactive session; the scene is rebuilt on each restart.
  A persistent session (no device/scene rebuild on edit) is a possible
  follow-up if restarts feel slow on big scenes.
- **RP-Q4** What restarts it: any change to geometry, transforms,
  visibility, materials, the shader tree, lights, cameras, the chosen
  camera, preview navigation, window size (unless Full Resolution or Lock),
  options or Preview Settings. Selection changes don't. The scene is
  compared 5 times a second; restarts wait 120 ms for changes to settle.
  While paused, a change marks the picture stale and Resume starts over.
- **RP-Q5** Effect: (Shading) and Final Colour both show the beauty pass;
  Alpha renders with transparent film and shows alpha as grey. Shading:
  Full and Fast render the same in v1. LUT sRGB (default) or Linear.
- **RP-Q6** Options defaults: Scene Preview, all four built light paths on,
  Automatic Geometry Updates on, Use Multiple Threads (cores − 1), Show
  Only Visible Items on. Final Render Quality = Full Resolution + every light
  path + render-level subdivision; below it, subdivision renders at the
  viewport level. Light paths off: Shadows → no light casts shadows;
  Global Illumination / Reflections / Refractions → that bounce kind 0.
  Options aren't saved between launches; Preview Settings are.
- **RP-Q7** Picture: letterboxed in the render frame's aspect (720 × 480 with
  a scene camera; the viewport's shape without one). Shift+wheel zooms in
  25/50/100/200/400 % steps, double-click fits again.
- **RP-Q8** Navigation: dragging in the picture (or on the Pan/Orbit/Zoom
  icons) moves a preview-only camera, starting from the chosen camera;
  the scene's camera item is not changed. Plain drag orbits, Shift pans,
  Ctrl zooms, wheel zooms; camera roll is not kept once you navigate.
  Choosing a camera in the popup resets it. Trackball Rotation offers Off
  only (viewports have no trackball yet).
- **RP-Q9** F9 while the Preview renders pauses the Preview until the final
  render ends, then resumes it; the Preview doesn't restart while a final
  render runs (it catches up afterwards).
- **RP-Q10** Not built in v1, shown greyed ("Not available yet"): the other
  Geometry modes, Render All Outputs, Displacements, Fur, Volumetrics,
  Motion Blur, Render Proxy, Load/Save Progressive Render, Save Layered
  Image, Update Under Mouse, Tool Activation, Render Animation, Heat Map,
  Bake Mode, layer overlay. Save Image writes PNG or JPEG; EXR needs the
  `image` crate's exr feature (a new dependency) and is left out.
  Right-click material pick is not built. The Preview's small controls
  (padlock, pan/orbit/zoom, maximise, cog, triangle) are drawn shapes, not
  icon artwork, until icons are made and approved.

## Inset and Shift tools (inset-shift, 2026-11-06), provisional

- **TA1-Q08 Shift.** The gap list says only "extrudes each selected
  polygon along its own normal" (Shift distance, default 0). Starting
  state: cube, polygons 0 and 1 selected (neighbours). Action: Shift 0.5.
  Built: each polygon moves 0.5 along its own normal on new points (no
  sharing between neighbours, like Inset rule 1), with a side quad per
  edge; caps keep their slots, tags and UVs; caps stay selected; nothing
  selected shifts every polygon; Shift 0 builds the degenerate quads as
  Inset 0 does. Unclear: whether neighbours share points (group extrude),
  the side-quad order and UVs, and what nothing selected does.
- **TA1-Q09 Inset/Shift interaction.** Built: Inset's handle and
  drag-anywhere act like Bevel's Inset square (right/out = bigger inner
  copy); Shift's handle points along the first selected polygon's normal
  and drag-anywhere up/down sets it. Distances are world metres (item
  scale divided out). Unclear: the reference's handles for both.

## Viewport header buttons (viewport-buttons, 2026-11-06), provisional

Built from the coordinator's summary of BEHAVIOR-VIEWPORT-BUTTONS v1 (the
spec file itself isn't in `specifications/`). Open questions, with the
reading built:

- **VB-Q1. Answered by Jason, 2026-11-06:** the Maximise button is always
  shown: orange whenever one view fills the frame (maximised in its group,
  or alone in its layout), white when the group shows all its views. In a
  lone view, the button (or Numpad 0 / Alt+0) builds a 2 x 2 quad: Top,
  Perspective / Front, Right. The view keeps all its state and takes the
  slot matching its type (Top, Front, Right; Perspective's slot for
  Perspective, free Orthographic, Camera and Bottom/Back/Left); the other
  three are new default views. After that it is an ordinary group.
  Provisional: the view keeps its world scale as it shrinks into its slot
  (as Maximise does), and the new views start unframed.
- **VB-Q2.** A plain click on Pan, Rotate or Zoom does nothing.
- **VB-Q3.** Every mouse button drags the same; modifiers are ignored.
- **VB-Q4.** Ortho views are not linked in Operandi: every view has its
  own camera and nothing ties Top/Front/Right together. Not built; waiting
  on Jason (v1.1).
- **VB-Q5.** Zoom Drag Axis defaults to Left to Right. Operandi already had
  this preference (preferences-2026-11-04 NAVIGATION, default Bottom to
  Top, unmeasured); the Zoom button uses the same setting, so the in-view
  drag-zoom (Ctrl+Alt drag) now also zooms in left to right by default.
  Research: is the default measured, and does the in-view drag-zoom follow
  the same preference?

Other readings marked provisional:

- Rotate shows in Perspective, the free Orthographic view (what orbiting an
  axis view turns into) and Camera view; the six axis views omit it.
  Operandi has no Light view yet.
- Spec v1.1: the Pan button moves the view the way you drag (the scene
  slides the other way) at 3x the pointer's travel (`PAN_BUTTON_RATE`,
  tuned by feel); Reverse Pan Widget Direction gives grab feel at the same
  rate. Rotate and Zoom keep the in-view rates. Zoom counts only
  horizontal movement with the default Left to Right.
- During a Pan/Rotate/Zoom drag the pointer is hidden and put back on the
  icon after every move (each move is read relative to the icon), so the
  drag never runs out of screen. There was no pointer-lock code to reuse;
  this uses window pointer warps. If macOS delays events after a warp the
  drag may feel sticky: check in the GUI pass (VB-T5).
- Spec v1.1: Maximise and Restore keep world scale. The projection scales
  with the view's height times its lens, so the view's lens takes the
  height change (perspective: a wider field of view, same eye). The lens
  now survives changing the view type; Frame All allows for it. Camera
  view keeps its camera's lens and is not rescaled.
- Revert to default resets the view's display settings (every popover row
  except View Type); the camera is left alone.
- Show Viewport Controls and Information off hides the corner buttons and
  the two lines of view information at the top left; the view type and
  Display menus, the axis badge and tool hints stay.
- Show Grid offers Always and Never only.
- Show Vertices and Show Verts are view-wide, so they sit under Active
  Meshes only.
- Gradient Top is a new theme colour (Preferences > Display > Colors).

Demoted to later (each needs a feature Operandi doesn't have yet): Camera
Item and Light Item lists (no per-view camera choice, no Light view);
Trackball Rotation (no trackball orbit) and Orbit Selected (no orbit about
the selection); Show Locators (no locator items); Show Selections, Show
Selection Normals, Show Selected Filling, Show Selected Outline (selection
is baked into each view's GPU geometry); Use Shader Tree (the views don't
draw from the shader tree); Wireframe Colour (the overlay colour is a theme
colour, not per group); Show Cages/Guides (no subdivision cage display).
Skipped per the brief: HUD, presets beyond Revert, viewport linking.

### Viewport Properties layout (LAYOUT-PROPERTIES-PANEL v1.2), provisional

Specs: specifications/viewport-buttons-2026-11-06/ (the behaviour file is a
summary relayed by the coordinator; the layout file is the v1.2 addendum).

- View Type also lists Orthographic (Operandi's free ortho view, which
  orbiting an axis view turns into), after Perspective. Camera shows only
  when the scene has a camera.
- Show UV Boundaries and Show Vertices are view-wide in Operandi, so they
  appear once, under Active Meshes > Shading, not again under Inactive.
- Show Grid offers Always and Never (Inactive Work Plane isn't built).
- In All Tabs the two Shading bars read "Shading (Active Meshes)" and
  "Shading (Inactive Meshes)" so they can be told apart.
- "Show Viewport Controls and Information" is wider than the control
  column at 420 px; it is cut short with "…" and the tooltip has it in full.
- One Tab ⏷ and All Tabs ⏷ list the tabs: picking one switches mode and
  shows (or scrolls to) that tab; clicking the name only switches mode.
- Folded sections save to viewport-properties.json (per user, all views).
- When the window is too short for the full tab names, the tab font
  shrinks (to 7 pt at least) so the strip fits.

Left out (needs a feature Operandi doesn't have): Light view type,
Orientation, Overlay Drawing, Selected Item Drawing, Camera Item, Light
Item, the Viewport Independence section, Trackball Rotation and Orbit
Selected (so no Mouse Control section), Show Locators, Show Work Plane,
Draw Faded Grid, Faded Grid Color Axes, Show Selections, Show Selection
Normals, Show Selected Filling, Show Selected Outline, Use Shader Tree, Use
Texture, Ambient Intensity, Ambient Color, Wireframe Color, Show Geometry
Boundaries, Edge Thickness, Show Cages, Show Guides. The popover's Show
Verts row is gone (Show Vertices = Always covers it); the Display menu
keeps its own Show Verts toggle.
- The popover's tab strip is Operandi's own vertical tabs (side_tabs.rs, as
  on the toolbox and Properties panel): white text on the chosen tab with
  the orange edge, the others dimmed, at the toolbox's size (Jason,
  2026-11-06). It runs the popover's full height, title rows included.- Floating header (Jason, 2026-11-06): the view header's buttons float
  over the top of the 3D view, each on a translucent backing (new colour
  Header Button Backing, drawn at 75 % opacity); no strip. Heights: the
  strip was 22 points and its corner buttons 18; the floating row is 19
  and its corner buttons 16. The top-left information text moved down 22
  points to clear it. Presses on a button stay the button's; presses in
  the gaps reach the view.- Header round 3 (Jason, 2026-11-06): Navigate, Frame All, Frame
  Selection and Split View left the header for a new View menu (with
  Maximise, Viewport Properties, Tear Off View, Close View and Navigation
  Help; Navigation Help shows the current mouse preset's gestures, which
  the old Navigate tooltip didn't follow). Buttons fill with the view's
  background colour at 92 % opacity (pure see-through was not tried: text
  over a light mesh needs the fill) with a 1-point edge (new colour Header
  Button Border; Header Button Backing is gone). Views have no tab bars:
  dragging a view's tab to rearrange or dock is gone, and Tear Off and
  Close View moved to the View menu; docking a torn-off window puts its
  view back as a split, not a tab.
- Show View Information (per view, gear > Drawing and Control): the two
  top-left lines; off by default. No toggle existed before (they followed
  Show Viewport Controls and Information only). A new field, so every
  saved view and preset loads with it off.


## Panel corner dot (panel-corner-dot, 2026-11-06), provisional

Built from the coordinator's summary of PANEL-DOT v1 (the spec file isn't in
`specifications/`); docs/PANEL_CORNER_DOT.md. Open questions, with the
reading built:

- **PANEL-Q01.** The Toolbox, Items and Properties are fixed side panels,
  not panes in the workspace layout, so they can't swap with or dock beside
  views. Their dots activate and outline; dragged outside every panel (or
  Detach) the panel floats at the release point; a floating one dropped on
  the main window docks back in its own side panel; Duplicate, Copy,
  Delete, Maximize and the splits are greyed for them. Moving them into the
  layout is a separate step.
- **PANEL-Q02.** Drag cursor: egui has no hollow-rectangle cursor, so the
  system closed hand (Copy cursor with Shift) is shown, with a small hollow
  rectangle drawn beside the pointer in the window the drag started in.
- **PANEL-Q03.** Shift+drag onto a view's centre zone does nothing (status:
  drop a copy on an edge); on an edge it docks a copy there.
- **PANEL-Q04.** Ctrl+drag: the copy goes on the side the drag went (right,
  left, below or above) and becomes the active view.
- **PANEL-Q05.** The dot menu's Maximize is greyed for a view alone in its
  group (View > Four Views still builds the quad from a lone view).
- **PANEL-Q06.** Detach is greyed for a view alone in a floating window (it
  has its own window already).
- **PANEL-Q07.** Duplicate docks the copy at the right of the panel's own
  window's layout; the original stays active. Copy and Detach open their
  window 30 points right of and below the panel's top-left.
- **PANEL-Q08.** Closing a floating view window discards its views; closing
  a floating side panel's window still docks the panel back (there would be
  no way to get the Toolbox back otherwise).
- **PANEL-Q09.** A floating window's saved position is where it opened;
  moving it by hand isn't saved.
- **PANEL-Q10.** A press inside a view makes the views active again; a press
  inside a side panel's body doesn't make it active (only its dot does).
- **PANEL-Q11.** Dropping a view on a side panel does nothing.
- **PANEL-Q12.** Dot drags across OS windows rely on the system sending the
  drag's pointer moves to the window it started in; to be checked on macOS
  (PD-07).

## Add Loop and Slide options (add-loop-slide, 2026-11-06), provisional

Tool audit Tier 1 items 1–2. The gap list gives option names and defaults
only; these readings are Operandi's own until measured.

- **TA1-Q01 Absolute Distance.** Starting state: G3, Add Loop armed,
  Absolute Distance on, Distance 1 = 0.2 m. Action: click edge 1–2. Built:
  every ring edge is cut 0.2 m from its end on the clicked edge's
  lower-numbered point's side (Offset from End: from the other side),
  clamped to 1 %..99 % of each edge; Position is ignored; dragging after
  the click sets Distance 1. Unclear: which end is "start" for a click,
  whether the distance is per edge or along the clicked edge only, and
  what a drag does.
- **TA1-Q02 Both Sides.** Built: a second loop at 1 − Position (with
  Absolute Distance: Distance 2 from the opposite end) in the same edit;
  one loop when the two coincide (consistent with AL-05). Where the two
  absolute distances would cross on a short edge, the second point stays
  just past the first. Unclear: the reference's handling of crossing
  distances and of Both Sides with Middle.
- **TA1-Q03 Preserve Curvature / Tension.** Built: each new point sits on
  a cubic Hermite curve along the crossed edge; each end tangent points
  along the edge that continues the run past that end (the one neighbour
  sharing no polygon with the edge), with the edge's length, times
  Tension; no such neighbour → the straight edge. Straight, even runs give
  the plain cut. Unclear: the reference's curve type and what Tension
  0 % / 200 % do. The same curve is used by Slide.
- **TA1-Q04 Stop Negative Slide.** Built: negative amounts do nothing and
  each point stops at its rail's end (and merges there). Unclear whether
  it also stops at 100 %.
- **TA1-Q05 Loop Slide.** ES-05 measured a Percent 1.5 edge slide passing
  the next loop with Loop Slide on (the default), so "on keeps the slide
  inside the rails" can't be right for edges. Built: Edge Slide's Loop
  Slide is shown fixed on (greyed); on Vertex Slide, on keeps ±100 % (as
  before) and off lets a point carry on past its edge's end along the
  edge's line. Unclear: what Loop Slide off does to an edge slide.
- **TA1-Q06 Slip UVs.** Built: slid points get UVs interpolated along the
  rail (polygons holding the rail) or extrapolated along the opposite edge
  (polygons on the other side), on every UV map, since Operandi has no
  active UV map. Unclear: which map the reference uses and what happens at
  UV seams.
- **TA1-Q07 Edge Slide Merge Vertices.** ES-04 says the reference refuses
  Merge off for a Radial edge slide and merges anyway; shown fixed on
  (greyed). Duplicate is greyed (not built).

## Drop Action (drop-action, 2026-11-06), provisional

Spec `specifications/drop-action-2026-11-06`. Built with the spec's picks
for DA-Q1..Q6 (saved choice; Place moves only; viewport only; every
selected item, Insert family the pressed one; Insert takes the
destination's old slot; Place and Align by the smallest turn of +Y).
Readings of my own, for Jason to confirm:

- **DA-Q7** Place / Place and Align ray-test the destination's cage
  polygons, not its subdivided surface, so on a smooth mesh the point can
  sit slightly off the drawn surface.
- **DA-Q8** When a solve would need shear (a non-uniformly scaled, rotated
  parent), Parent in Place, Match, Insert and Place refuse the whole drop
  with a status message, as Parent To, Keep Position already does.
- **DA-Q9** Pressing an unselected item selects it (as a click does); a
  press-release without movement (under 3 points) is an ordinary click
  and selects only that item, even inside a multi-item selection.
- **DA-Q10** Several sources with Place / Place and Align all land on the
  same surface point.
- **DA-Q11** If one source of a multi-item drop is refused (e.g. a loop),
  the whole drop is refused; the spec doesn't say whether the others
  should still apply.
- **DA-Q12** The dropdown has no icon yet (icons go through the generator
  and Jason's approval); its face is text with a ⏷.

## Weld drag (weld-drag, 2026-11-06), provisional

Spec WD-01..05 (drag a point onto another to weld). Start: Vertex mode,
Weld armed, a mesh with points in view.

- **WD-Q01.** Points hidden behind the surface can't be pressed or snapped
  to (the same visibility rule as point picking). Should snapping see
  through the surface, or follow the display mode?
- **WD-Q02.** Dragging onto a point that shares a polygon but not an edge
  (a quad's opposite corner) is refused as Join refuses it ("would fold a
  polygon onto itself; nothing changed"). Should it split the polygon
  instead?
- **WD-Q03.** Selection: a selected dragged point drops out of the selection
  after a merge; other selected points stay (renumbered). Edge and polygon
  selections are cleared when polygons change, as for every Vertex edit.
- **WD-Q04.** The drag is previewed as an overlay (the point and its edges
  follow the pointer; the target gets a ring); the scene changes only on
  release. Esc mid-drag cancels the drag and keeps Weld armed.
- **WD-Q05.** A press and release without moving does nothing (no undo
  step). A move-only release is labelled "Weld" in the undo history.
- **WD-Q06.** The press uses the existing point-pick radius (12 points);
  snapping uses 15. With several points inside 15, the nearest wins.

## Light properties Tier 1 (light-properties, 2026-11-06), provisional

Spec `specifications/light-properties-2026-11-06`. Built with the spec's
picks LPQ-06 (Set Temperature stores RGB only), LPQ-07 (Affect Diffuse /
Specular on/off; a loaded partial value counts as on, flagged in the panel
and the render status notes) and LPQ-08 (portals have no light material).
Readings of my own, for Jason to confirm:

- **LPROP-Q01 Kelvin colour.** Set Temperature uses a public black-body
  curve fit (Helland, from Charity's table), converted to linear RGB and
  scaled so the brightest channel is 1 (brightness stays with Intensity).
  6600 K comes out white; the reference's white point and scaling unknown.
- **LPROP-Q02 Default for Select and Lock.** Read as "selectable" and
  "unlocked"; nothing inherited from parents.
- **LPROP-Q03 Visible Yes vs the eye.** Yes overrides Show Cameras / Show
  Lights, but the Items-list eye still hides the item (and the eye still
  drives Render Default, LP-10).
- **LPROP-Q04 Select No.** Only Item-mode clicks in 3D views ignore the
  item (a click on it picks nothing; it doesn't pick through to the item
  behind). Component modes on the active mesh still pick.
- **LPROP-Q05 Visible to Reflection / Refraction in Cycles.** Cycles can't
  separate the two: either one turns the light's multiple importance
  sampling on, so every bounced ray can hit its shape. With both off (the
  default), highlights still come from direct sampling, as before.
- **LPROP-Q06 Portal.** Points into the room along its −Z (arrow in the
  viewport). Environment Only is stored and shown but renders the same on or
  off (Cycles portals only gather world light). The render adds Cycles'
  world background light whenever a portal exists. No Targeting rows (not
  built for any light yet) and no Samples row.
- **LPROP-Q07 What Lock refuses.** Typed Item Transform fields (greyed),
  Move/Rotate/Scale/Transform drags (no handles, drags change nothing),
  Camera view navigation of a locked camera, Backdrop Auto Fit / Size and
  Drop Action moves. Parenting from the Items list (keeping the world
  position) is still allowed. Of the display rows only Visible exists to
  lock (Pivot, Center, Size not built).
- **LPROP-Q08 Multi-light edits (LP-51).** A light row edited with several
  lights selected applies to every selected light, including kinds that
  don't show that row (the value is kept, unused, as on a kind change).
- **LPROP-Q09 Label look.** Drawn 6 points up-right of the item's centre
  plus Label Offset (points, +Y up), 12 pt, locator colour (selection
  colour when selected); hidden when the item isn't drawn.

### Light properties Tier 2 (light-properties-t2, 2026-11-06), provisional

Built with the spec's picks LPQ-02 (cylinder tube along local X, ends
dark), LPQ-03 (Physical Sun rows grey while it's off) and LPQ-04 (our own
city list, default San Francisco 37.53° N, 122.30° W, UTC−8). Light
Linking (LP-30..32) is **not built**: Operandi has no item groups yet
(LPQ-05), and inventing them is out of scope.

- **LPROP-Q10 Targeting maths.** The aimed item points −Z at the target's
  centre with +Y as near world up as possible, then Roll about the aim. It
  keeps its own Scale; a parent's scale or shear doesn't reach it. Aiming
  straight up or down uses world −Z as the up reference.
- **LPROP-Q11 Targeting limits.** Set Target refuses the item itself and
  anything under it. Targets aiming at each other stop after 8 lookups.
  Lock refuses target edits. A targeted camera's Camera-view navigation
  moves it, but the aim wins, so the view turns back to the target. Time
  Offset is parked (no animation); Set Focus Distance isn't shown on lights.
- **LPROP-Q12 Physical Sun strength.** Below the horizon the light is left
  out of renders. None = absolute clear-sky sunlight (Kasten–Young air mass,
  Meinel transmittance, about 1000 W/m² overhead); Clamp = that curve
  relative to the sun overhead × Radiant Exitance; Replace = Radiant
  Exitance. Haze 3 is the clear sky; higher thickens the air (less, never
  warmer, light: colour isn't changed).
- **LPROP-Q13 Physical Sun rows.** Time is decimal hours (with an hh:mm
  read-out); Day is a month dropdown plus Day of Month. When Physical Sun
  and a target are both on, the sun wins.
- **LPROP-Q14 Location.** Our list of 12 cities; picking one fills Latitude,
  Longitude and Time Zone (one undo step). Numbers that match no city show
  "(custom)".
- **LPROP-Q15 EULUMDAT.** .ldt files aren't read (Cycles reads IES only);
  Browse... lists .ies.
- **LPROP-Q16 Photometric mapping.** Loading a file sets Radiant Intensity
  = the file's peak candela ÷ 177.83 lm/W (the D65 efficacy Cycles itself
  uses) and Width/Height = the file's opening width/length. In Cycles the
  strength is Intensity ÷ that, times the file's own shape, so a just-loaded
  light renders exactly the file. A Cone Angle under 180° renders as a spot
  (Cycles spots stop at 180°); 180°–360° as an unlimited point light. The
  size softens it as a sphere of half the larger side. The file is read
  when rendering; one that can't be read renders as a plain point light,
  with a status note.
- **LPROP-Q17 Cylinder in Cycles.** Cycles has no cylinder lamp: it renders
  as a hidden glowing open tube (32 sides) with the light's radiance, and
  casts no shadow itself. Visible to Camera applies; Visible to Reflection
  follows Affect Specular (Cycles can't separate them on a surface),
  refraction always sees it. Falloff is ignored as for every light. No
  viewport handles yet; Radius and Length sit in the Move/Rotate/Transform
  tool panel.
- **LPROP-Q18 Mesh light.** Any mesh light that renders hides its
  prototype from the render (it's still drawn in the viewport); the
  prototype's own materials are replaced by the glow. Use Prototype
  Transform off places the prototype's shape by the light's transform. The
  prototype renders at its render subdivision level.

### Light properties Tier 3, lights (light-properties-t3, 2026-11-06), provisional

- **LPROP-Q19 Volumetric beam shape and density.** The beam is a closed
  volume: a cylinder down a directional light's −Z (Radius × Height), a cone
  from a spot (Base, widening by the Cone Angle, over Height), a sphere
  round a point or photometric light (Sphere Radius). Density 100 % = 1 per
  metre in Cycles' volume scatter; Light Attenuation fades it by (1 −
  Attenuation) per metre from the light. Every light in the scene lights the
  beam, not only its own. Samples (LP-41) has no Cycles meaning and isn't
  shown.
- **LPROP-Q20 Light Shift.** Stored and shown, not rendered (Cycles' volume
  scatter has one colour; a distance tint would need a colour ramp we can't
  check against the reference).
- **LPROP-Q21 Importance.** Cycles has no per-light importance: stored,
  shown, saved, not rendered. A Dome light keeps it stored but has no row.
- **LPROP-Q22 Affect Subsurface.** Cycles lights have no subsurface ray
  switch (subsurface uses diffuse rays): stored, shown, saved, not rendered.
- **LPROP-Q23 Physical Sun Gamma.** Stored, shown, saved, not rendered:
  without the daylight sky (the environment spec's job) there's nothing it
  can honestly change.
- **LPROP-Q24 LPE Label passes.** Each label is a Cycles light group: the
  render saves one extra PNG per label beside the picture ("Render 3
  (Key).png"); the render window still shows the full picture only. Dome
  lights are part of the world, not a group.
- **LPROP-Q25 Light material.** Enable off = no light; Opacity scales the
  light (one material, LM-04); Blend Mode is stored and shown but changes
  nothing (nothing under the material). Image layers (gobos) blend over
  white with their own Blend Mode and Opacity (new layers: Multiply, 100 %)
  and render on spot lights only, projected through the cone and clipped
  outside it; other kinds keep the layers but render without them.
- **LPROP-Q26 Dome light.** An even world light of colour × Radiance
  added to the world for every ray except camera rays (so it lights without
  showing in the background). Shadow Type None can't apply to world light in
  Cycles (shadows stay on); its hidden 10 m radius isn't used.
- **LPROP-Q27 Store Location.** Places are saved in the user's
  sun-locations.json (not an undo step); a name already stored is replaced.
  They're picked from a "Stored Location" row under Location.

### Shared item tabs Tier 3 (light-properties-t3, 2026-11-06), provisional

- **LPROP-Q28 Pivot / Center.** Default reads as No (no marker); Yes draws a
  ring (Pivot) or a cross (Center) at the item's origin. Operandi items have
  no separate pivot point yet, so both mark the origin. Lock refuses them.
- **LPROP-Q29 Draw Options on meshes.** Enable, Wireframe and Wireframe
  Color recolour camera and light glyphs now. Meshes keep the view's wire
  colour: a per-item wire colour needs the GPU wire shader to take one per
  item (not built). Background Light / Dark use the view's light and dark
  line colours.
- **LPROP-Q30 Dimensions.** v1 rows only: Visibility (Off / On / Selected),
  Opacity, Show X/Y/Z. The readout is the world bounding box's size per
  axis, as dashed edges with their lengths. Min/max warning sizes and
  link/chain children are not built.
- **LPROP-Q31 Notes.** Title plus nine lines; drawn next to the item while
  it's selected. Trailing empty lines are dropped.
- **LPROP-Q32 Redirect.** One step only (a redirect's own redirect isn't
  followed); a missing target is ignored. Applies to Item-mode clicks.
- **LPROP-Q33 Pick Walk defaults.** Up = parent, Down = first child, Left /
  Right = previous / next item with the same parent in Items-list order; a
  Pick Walk item overrides. The arrow keys walk only in Item mode with no
  tool active (they keep Select More/Less etc. in component modes).
- **LPROP-Q34 Tooltip.** Shown next to the pointer while hovering the item
  in a 3D view, in Item mode.
- **LPROP-Q35 Per-item Drop Action.** The destination's Destination action
  wins, then the source's Source action, then the top bar; with either set,
  a drag starts even when the top bar says (none). Assembly is stored only
  (there are no presets to drop yet).
- **LPROP-Q36 User Channels.** v1 types Float, Distance, Angle,
  Percentage, Integer, Boolean, String, Divider (scalars only, no vector
  modes); Channel Name unique per item (letters, digits, spaces, _);
  numbers clamp to the limits, Integers round; Divider is a heading row.
  Stored and saved; nothing links them. No Edit User Channel rename yet.
- **LPROP-Q37 Tags.** Built-in Comment (multi-line) plus custom types
  typed in Add Tags; one tag per type; no tag-definition preferences.

## Item groups and Light Linking (item-groups, 2026-11-06), provisional

No behaviour spec for groups exists yet (Jason decided to build a small
generic version). Light Linking follows LP-30..32 with the picks LPQ-03
(Item Group and Mode greyed while Control is off) and LPQ-05 (default
none). Every choice below is ours, to be checked against a future spec:

- **GRP-Q01 What a group is.** A named, ordered list of items of any kind;
  an item may be in several groups; groups hold no transform or display
  settings and aren't items (they don't appear in the Items list).
- **GRP-Q02 Names.** Unique per scene; a new group is "Group", then
  "Group (2)"…; renaming to a taken name gets the next free suffix.
  Names are trimmed, 1–256 bytes.
- **GRP-Q03 Deleting.** Deleting an item takes it out of every group (one
  undo step with the delete); deleting a group never deletes its items; a
  light pointing at a deleted group behaves as if it had none.
- **GRP-Q04 Selecting.** Clicking a group in the Groups tab selects its
  existing members (first one primary), like clicking items: no undo step.
  Clicking a member row selects that item.
- **GRP-Q05 Where.** A Groups tab beside Scenes and Shading in the Items
  panel (New Group, Delete Group, Add Selected, Remove Selected, rows with
  a member count, ▸ to expand, double-click to rename) and Item > New Group
  from Selection. No drag-and-drop into groups yet.
- **GRP-Q06 Light Linking in Cycles.** Each linked light becomes a Cycles
  light-set rule; items get the receiver set of the rules that leave them
  out. Shadows follow the same sets (shadow linking): an item a light
  doesn't light also doesn't cast that light's shadow. Cycles allows 64
  sets; scenes with more distinct combinations share set 0 for the rest.
- **GRP-Q07 Empty cases.** Include with no group or an empty group lights
  nothing; Exclude with no group lights everything (as the brief asked).
- **GRP-Q08 Native version 18** for groups and Light Linking (17 at first; renumbered at merge because the Material Shading Model took 17).

## Preferences v2 Tier 1 (specifications/preferences-2026-11-06), provisional

Every default the spec leaves as "—" is marked provisional where it is
defined; most keep Operandi's behaviour so far. Readings to confirm:

- **PREF-Q6 Allow Modifier Chording, Invert Drag Scrolling: left out.**
  Operandi's input map binds keys only, so there are no mouse-button
  chords for modifiers to join; its lists scroll by wheel or trackpad
  (both covered by Invert Mouse Wheel Scrolling), and egui's drag-to-scroll
  (touch screens) has no hook to reverse. What do these change in the
  reference on a Mac with a mouse and trackpad?
- **PREF-Q7 Per-view Trackball Rotation.** The view's own choice sits in
  Viewport Properties > General Visibility as Use Preference / On / Off.
  Where does the reference keep it? Camera view keeps two-axis orbit (a
  render camera has no roll). Trackball speed: Mouse multiplier only until
  Input Device exists.
- **PREF-Q8 Flatness of Perspective curve.** 60 % keeps Operandi's lens;
  every 40 points doubles or halves it (0 % ≈ 115° view, 100 % ≈ 31°).
  Lens and distance change together so the target keeps its size. The
  reference's mapping isn't known.
- **PREF-Q9 Default Scene.** Only Operandi scene files are used (an LXO
  needs the background import); a file that won't load is skipped with a
  status line. Does the reference's Default Scene also apply when opening
  the app with a file?
- **PREF-Q10 Numeric Enter as Tab** uses macOS's keypad Enter key code
  (egui reports both Enter keys alike); other platforms have no keypad
  detection yet.
- **PREF-Q11 Point Selection Priority** is a distance in points: a point
  wins over an edge up to this much nearer. Default 12 (the hit size), so
  Tweak keeps preferring points as before. Type and range unmeasured
  (SEL-P-Q2).
- **PREF-Q12 Mouse Regions Trigger Selections.** A right-click without
  movement on a point, edge or polygon switches to its mode and selects
  just it (one undo step for the mode, one for the pick). Off, the click
  still opens the selection menu. Does the reference also select it?
- **PREF-Q13 X-ray transparency.** Transparency: Selection is the hidden
  selection's opacity, 0 % (today: hidden) by default; drawn on the
  graphics card for selected points, edges and polygon wire. Handles: the
  hidden handle's opacity, 100 % (today: always drawn) by default; judged
  at each handle's middle, not while a drag changes the scene. Which way
  does the reference's percentage run, and what are its defaults?
- **PREF-Q14 Handle points.** Large Points are the square handle ends
  (9 px), Small Points the falloff handles (12 px, hit 16 px), Handle
  Scale: Small applies to falloff handles, Plane Handle Ratio to the
  view-plane (free move) square, Unselected Tool Handles to falloff
  handles. Rotation Snap Angle 15°, with Ctrl held as the Rotate handle
  drag begins.
- **PREF-Q15 Item Drawing size** scales camera and light glyphs (not an
  Area light's real size). The current scene's value is saved in the
  scene; it is not an undo step.
- **PREF-Q16 Grid increments.** The reference's Plane and Grid Increments
  choices aren't listed (GRD-Q1); Operandi offers 1-2-5 (as before), 1-5,
  powers of 10 and powers of 2.
- **PREF-Q17 Use OS Color Picker** switches the light colour, shading and
  Preferences colour swatches. Set Material keeps the macOS picker it
  already had (its spec says so), whatever this preference says.
- **PREF-Q18 Reset "In the Future…" Messages** is left out: Operandi has
  no dialog the user can hide (the crash prompt's "Don't ask for this one"
  is per crash). Reset Preferences keeps the input map, saved colour
  schemes and viewport presets.
- **PREF-Q19 Default Material** tags new primitives' polygons (and Pen
  and the create tools that add geometry); other new polygons (extrude,
  bevel) take their neighbours' tags as before.

### Preferences v2 Tier 2, provisional

- **PREF-Q20 Copy and Paste defaults.** The spec reads the reference's
  default as "old selection kept, pasted geometry unselected"; Operandi's
  Paste so far clears the old selection and selects what it pasted, so
  Select Pasted and Deselect before Pasting start on. Which should win?
- **PREF-Q21 Max GL Polygon Count** asks after a recorded step takes the
  drawn count over the cap (default 5,000,000; 0 = off): Keep or Undo.
  Bounding Box Threshold draws the chosen items as boxes only while the
  view is navigated and the scene is over that cap (GPU path).
  **Answered (Jason, 2026-11-13): triangles.** The cap counts the same
  number as the "GL: n" overlay line: triangles drawn for visible items
  after subdivision (cached GPU fill, else counted from the polygon list
  by subdivision kind, never from the smooth-shading flag). The dialog
  says "The views now draw N triangles…"; the preference keeps its name
  and saved key (`max_gl_polygons`).
  **Superseded (Jason, 2026-11-20): a deliberate departure from the
  reference.** No fixed cap and no question. Operandi watches the graphics
  card's own memory (Metal's working-set budget on macOS; other platforms
  report none yet) and, near the budget, draws the heaviest items as box
  stand-ins and releases their geometry, with a note in the view; they
  come back when there is room. Bounding Box Threshold now applies near
  that budget. Old `max_gl_polygons` values still load and are dropped on
  save (`gpu_memory.rs`).
- **PREF-Q22 Icon and popup scales.** Default Form Icon Size scales the
  toolbox (Small 0.8×, Large 1.25×). Popup Transparency applies to menus,
  popups and the Viewport Properties popover; dialogs stay opaque.
  Popover Dismissal applies to Viewport Properties (Operandi's one
  popover).
- **PREF-Q23 Defer Smoothing During Interaction** shades every polygon flat
  while a tool drag runs, and smooth again on the mouse-up.
- **PREF-Q24 Coordinate Rounding** applies to points picked on the work
  plane by the creation tools, Pen, Tweak's plane drags and the numeric
  drags; Normal ≈ a step every 3 points, Fine every point, both from the
  grid increment series.
- **PREF-Q25 Up Axis** turns orbit, Top/Front/Right, the ground grid and
  OBJ/STL/glTF/GEO import and export (files are Y-up). Primitives keep
  their default axes, and saved scenes don't record the up axis.
- **PREF-Q26 Input Device.** Mouse puts the pointer back during header
  button drags (so they never hit the screen edge); Tablet and Hybrid let
  it stop at the edge and use the tablet trackball speed. Operandi's tool
  drags never warp the pointer, so Hybrid acts like Tablet.
- **PREF-Q27 File I/O.** OBJ writes a .mtl (colour only) by default as the
  spec says; STL's ASCII choice stays in File > Export; glTF tangents are
  averaged per vertex (not MikkTSpace); JPEG sub-sampling isn't offered.
## Deform presets, Push, Smooth (deform-presets, 2026-11-06), provisional

Spec `specifications/deform-presets-2026-11-06`. TW-Q1 answered (Twist
turns by weight × Angle; Swirl does the same). Starting values from its
STARTING-VALUES.md. Readings of my own, for Jason to confirm:

- **DF-Q3 Fit on activation.** The spec fits the falloff on the first click,
  with a Linear line running across the pointer's screen direction. Built:
  fitted when the button is pressed, to the selection's box (whole mesh with
  nothing selected); Linear along the box's longest axis with Start (full
  effect) at its high end, Radial centred on the box at half its size. Fit to
  Selection in Tool Properties refits. Pointer-based fitting and right-drag
  to redraw the falloff are not built.
- **DF-Q4 Item mode.** Presets, Push and Smooth refuse Item mode with a
  message instead of switching modes.
- **DF-Q5 Smooth starts applied.** Smooth works as soon as it starts, at
  Strength 1; fields and the drag rework that one result from the start.
  A left/right drag changes Strength by 1 per 200 px. Strength above 1 is
  split into more passes of at most 1 (SM-Q1 open). Preserve Volume is a
  Taubin inflate step after each pass (SM-Q1). Lock Sharp Edges compares
  the angle between the two polygons' normals with Sharp Angle.
- **DF-Q6 Push drag.** Up pushes out, down pulls in, at the move arrow's
  scale. Selected normals outside Polygon mode behave like Averaged. Points
  with no polygon don't move.
- **DF-Q7 Redo after Undo-closes.** Undo past a preset's start closes it and
  gives back the falloff; Redo reopens the plain transform, not the preset.
- **DF-Q8 Layout (corrected 2026-11-06, DEFORM-LAYOUT-CORRECTION).** The
  full 18-button panel is shown with unbuilt tools greyed out ("Not built
  yet"); Soft Drag, Smooth, Soft Selection Move and Bulge are flyouts. Built
  here: a flyout opens on a 0.35 s hold or a right-click (the right-click is
  Operandi's addition); the face remembers the last pick for the session
  only (not saved between launches). Every Deform button uses a placeholder
  icon until artwork exists. Tweak is renamed Element Move (Jason,
  2026-11-06): label, input action and command ID `ElementMove`
  (`tool.set ElementMove on`); `tweak` stays as a deprecated alias and a
  saved "Tweak" key binding carries over.

## Slice tools Tier 1A (slice-tools, 2026-11-06), provisional

Spec: specifications/toolbox-panels-2026-11-06 (EDIT.md "Slice section",
STARTING-VALUES-1A-1B.md 1A). Starting values are built as listed there.
Start: a mesh, the Edit tab, one of Loop Slice (Alt+C), Edge Slice (G),
Polygon Slice (Shift+C).

- **SL-Q01 (LS-Q1).** Loop Slice first activation: with a selected edge (Edge
  mode) or two neighbouring selected polygons (Polygon mode) the slice shows
  at once at 50 % (part of the activation step); with no ring it waits, and
  a click on an edge picks that ring (Shift+click adds another ring). What
  does the reference do with nothing selected?
- **SL-Q02.** Changing Count re-spaces the slices evenly, also in Free mode;
  bar Add/Remove keep the other slices where they are. Symmetry mirrors the
  dragged slice's partner; an odd middle slice stays at 50 %.
- **SL-Q03.** Slice N-gon on continues the ring through even-sided n-gons
  (to the opposite side); odd-sided ones end it like triangles (they take
  the points, uncut).
- **SL-Q04.** Keep Quads: built 2026-11-16 (branch loop-slice-options) as
  LS-B04 below. Still open: which edges the reference adds where the ring
  ends on a selected triangle or n-gon.
- **SL-Q05.** *Mostly answered* by spec section 7 (2026-11-16); Split,
  Cap Sections, Gap and the profile options are built (LS-B05..LS-B08).
- **SL-Q06.** Select New Polygon: Edge mode selects the new loop edges,
  Polygon mode the cut polygons' pieces, Vertex mode changes nothing. Bar
  end dots: the right one sets Length, the left one moves the bar's start
  (its right end stays); either also sets Slider Y.
- **SL-Q07 (ES-Q1).** Edge Slice: a path starts on an edge; an inside point
  (Inside Polygons) joins the cut once the path leaves its polygon through
  an edge, and changes nothing before that. A click on an edge of a polygon
  that isn't next to the last point is refused. Non-quads are cut the same
  way. One path per session: after closing, clicks are refused until Drop.
- **SL-Q08.** Edge Slice Offset (cuts into a polygon's interior) isn't built
  (greyed at 0).
- **SL-Q09.** Edge Slice Multi Slice only draws the dashed line (no cuts along
  it). Always Raycast off lets wireframe views pick edges behind the surface.
  Snap Value also keeps a point one step off the edge ends. A Ctrl or Split
  at Middle point stays at the middle while dragged.
- **SL-Q10.** Polygon Slice Angle Snap acts while global snapping (X) is on or
  Ctrl is held; otherwise the line is free. Angles are measured in the work
  plane from its first axis.
- **SL-Q11.** Polygon Slice middle-click repeat: the new cut starts at the
  pointer with the current cut's length and direction (not centred there).
  A plain drag away from the current line draws it again; Shift+drag adds a
  cut. End handles grab within 8 points, the line within 6.
- **PS-Q1.** Infinite off: a polygon is cut where its crossing with the plane
  overlaps the drawn line's extent along the line (a 3D reading; it matches
  the on-screen segment in a view along the work plane's normal).
- **SL-Q12 (PS-Q2).** Polygon Slice Split: each piece's side is its centre's
  side of the plane; points on the cut are copied for the positive side.
  Gap moves each side's cut points away from the plane along their side
  edge; Offset Side Center splits the Gap, Above puts it all on the
  normal's side, Below on the other (labels provisional). Cap Sections caps
  every closed loop of new edges on each side; copied points have no point
  map records.
- **SL-Q13.** Polygon Slice Start/End fields edit the current cut; before the
  first cut they show the starting line and typing a value makes that cut.
  Fast Slice draws the line during a drag and cuts on release.
- **SL-Q14.** Two Loop Slice rings that cross inside one polygon are refused
  ("Slices can't cross inside one polygon").
- **SL-Q15.** The Edit tab sits between Select and Vertex (there is no
  Duplicate tab yet). Its buttons are text rows until their icons are
  generated and approved. New theme colours: Slice Bar and Line, Slice
  Points, Slice Closing Point (Highlights).
- **SL-Q16 (forum report; Jason's hands-on check 2026-11-25).** Edge Slice
  may start inside a polygon (Inside Polygons on) and end or close on an
  edge, a point or the first point. A loose end left inside a polygon is
  never kept as a hanging chain: on an open path each loose end runs on to
  its polygon's nearest corner (as the reference's preview does); when both
  ends would meet the same corner, that stretch isn't cut. Closed, the loop
  runs back through the leading inside points. A loop closed wholly inside
  one polygon becomes a polygon of its own, and the rest is joined to it
  by two links from the loop points nearest two different corners (no
  holes, as PS-13's builder note). Replaces SL-Q07's "a path starts on an
  edge".
- **SL-Q17 (forum report, 2026-11-25).** A click within 8 pt of an existing
  point puts the cut through that point (it wins over the edges meeting
  there); a path can end or close on a point. Two points on one edge (or
  a point and an edge point on the edge it ends) make no cut. Does the
  reference snap to points, and from how far?
- **SL-Q18 (Jason 2026-11-25, Operandi's own option).** Edge Slice **Add
  Support Edges** (off): every polygon holding a new cut point with more
  than four sides is split into quads (one triangle for an odd count) by
  edges between existing corners; no point is added. Plan: fewest pieces,
  squarest corners, convex pieces, new edges inside the polygon, cheaper
  from a new point, never doubling an existing edge. Polygons over 32
  sides are left as they are.

## Items list header (item-list-header, 2026-11-06)

Decided 2026-11-06 (settled defaults, not provisional):

- **IL-Q1 Find field.** The top row of the Add Item menu is a find field:
  typing lists every entry whose name contains the text (any case); Enter
  makes the first one Operandi can make.
- **IL-Q2 Add Item label.** The label opens the same menu as the ▾ arrow.
  The item browser is later work.
- **IL-Q3 Unsupported entries.** Shown disabled with the tooltip "Not yet
  in Operandi". Categories with nothing enabled are hidden unless "Show
  unsupported" (bottom of the menu, off by default, remembered) is ticked.
  Today only Lights has enabled entries (all nine kinds). Replaced by
  IL-Q4 (2026-11-23).
- **IL-Q4 Only what Operandi makes (Jason, 2026-11-23; deliberate
  departure from IL-02).** Add Item lists only the types Operandi can
  make: Backdrop Item, Camera, Mesh, Lights (nine kinds) and Locators
  (Group Locator). IL-02's other entries and the categories left empty
  (Deformers, Dynamics, Falloffs, Forces, Particles, Procedural, Volumes,
  Other) are gone, and so is Show unsupported. An entry returns when
  Operandi can make its type.

Provisional:

- **IL-CMD Script command names and arguments.** The spec gives none.
  Following `<domain>.<verb>`: `item.add <type>` with types `mesh`,
  `camera`, `backdrop`, `light.<kind>` (all nine kinds), and
  `item.pattern <select|unselect|hide> [pattern]` (MCP: `type`; `action`,
  `pattern`, `limit_to_children`). The list filters are view state and
  have no command. Which names and arguments should these have?
- **IL-P1 Placement with a selected item that has children.** The new item
  goes after the selected item in its parent's list, so it's drawn below
  the selected item's children.
- **IL-P2 Pattern with no match.** Select / Unselect / Hide refuse with a
  status message and no undo step; Unselect with no selected match does the
  same. Select with matches makes the first match (list order) active.
- **IL-P3 Absent features.** Groups (tick and Group action), By Color,
  Color, Has Notes and the Search-notes toggle are shown disabled. Item
  groups and notes arrived on main while this was built (merge 2026-11-06)
  but aren't wired into the Items list yet; groups aren't items, so they
  get no type tick. All nine light kinds are in Add Item.
- **IL-P4 Sorting.** Sort by Name is case-insensitive; with Sort by Type
  too, types come first (Backdrops, Cameras, Lights, Meshes), names within.
  Flatten sorts the whole flat list.
- **IL-P5 Text Flat without Flatten.** Matches show at the base level with
  no nesting dots (dots are Flatten's).
- **IL-P6 Header toggles.** Text labels "Sel", "Notes", "Text" until the
  icon generator has recipes for them.

## Pie menus, stage 1 (pie-menus, 2026-11-07), provisional

- **PM-Q1 Ctrl+Space on macOS.** Ctrl+Space is the factory chord (real
  Control, not Cmd). macOS uses Ctrl+Space to switch input sources when
  more than one is enabled, and then Operandi never sees the key. Ctrl+1 /
  Ctrl+2 are macOS "Switch to Desktop 1/2" shortcuts when those are turned
  on (off by default). Keep these chords, or pick others for macOS?
- **PM-Q2 Plain Space.** Space alone stays reserved (mode cycle / Drop), so
  only Space with a modifier can be bound. The input map's "Space is
  reserved" rule was relaxed to plain Space only.
- **PM-Q3 Viewport toggle mapping.** Lights, Cameras, Grid and Vertices map
  to the view's Show Lights, Show Cameras, Show Grid and Show Verts.
  "Wireframe overlay" flips the active meshes' overlay between None and
  Uniform. "Background shading" flips the view background between None and
  Gradient. Texture locators and Locators are blank (no such items). Are
  the two readings right?
- **PM-Q4 Which view.** View direction, shading and toggle entries act on
  the active view, like the existing Top View / Shaded View shortcuts, not
  necessarily the view under the pointer.
- **PM-Q5 View entries set, not toggle.** The pie's Top/Front/Right set
  that view. The existing Top View / Front View / Right View shortcuts
  flip to the opposite view when already there; the pie uses new
  "View Top" … "View Camera" commands instead. Camera is disabled when the
  scene has no camera.
- **PM-Q6 Disabled entries.** An entry is disabled where its shortcut
  would not run (`input::available`, as the Cmd-click modeling menu does):
  e.g. Bevel outside Polygon mode, Extrude in Edge/Vertex mode. Falloffs
  are disabled while a tool without falloff support is live; action
  centres while the context can't be validated. A disabled slot still
  highlights (greyed) so the user sees why releasing does nothing.
- **PM-Q7 Blank slots.** Falloffs: Cylinder, Airbrush, Screen, Element,
  Vertex map. Action centres: Pivot (VAC-Q04). Modeling: Edge knife, Soft
  drag (Edge Slice and Soft Move exist but aren't the same tools). Toggles:
  Texture locators, Locators. Shading: none blank.
- **PM-Q8 Clicks.** A click runs the slot whose label it hits, or the
  highlighted wedge within about 236 px of the centre; a click on an empty
  or disabled slot, in the dead zone or farther out closes with nothing.
  Release picks by direction at any distance. Any other mouse button
  closes. Losing window focus closes.
- **PM-Q9 Not while dragging.** A pie key does nothing while a mouse
  button is held (mid-drag), so a pie can't run a tool in the middle of
  another drag.
- **PM-Q10 Icons.** Entries show labels only; icons are optional and not
  drawn yet.
- **PM-Q11 Choice marks.** Falloff entries also show which falloff is
  active (the spec lists view, shading, selection mode and action centre).
- **PM-Q12 Letting go of a chord (2026-11-15).** Hold, point, release
  runs a slot with no click (Jason). Letting go of the chord's modifier
  first (Ctrl before Space) counts as letting go, as does the key no
  longer being down when its key-up never arrives. A press, move and
  release batched into one frame still runs the slot.
- **PM-Q13 Tight layout (Jason, 2026-11-15: "a lot tighter").** Buttons
  of 120×36 px in five rows 46 px apart (10 px gaps), sides level with
  the centre dot, diagonals tucked between; about 400×220 px in all.
  Longer labels widen their button outward. The pointer over a button
  picks it even where a diagonal's corner reaches into a side's wedge.
  The greyed-entry reason shows under the whole pie.
- **PM-Q14 Pies as forms (pie engine, 2026-11-27).** A pie is a form
  with style `pie` (UIC-308); its first eight controls are the slots,
  clockwise from the top, and an empty slot is a `space` control. Shipped
  pies keep icon mode Text (labels only, as before); a New Pie starts with
  eight empty slots and Both. The Layouts pie stays generated (its slots
  are the layouts), opened through the same engine, and can't be exported
  or used as a sub-pie.
- **PM-Q15 Sub-pie gesture.** Picking a sub-pie slot (releasing the key
  over it, or clicking it) opens the sub-pie in place at the same centre
  instead of closing (UIC-310). The key is up by then, so the sub-pie
  waits for a click. Esc, or a click in the centre's dead zone, goes back
  one level; in the outermost pie both still close it, as before.
- **PM-Q16 Loops.** Setting a sub-pie that leads back to the pie is
  refused (`operandi.form.slot`), and the Form Editor only offers pies
  that don't. A loop that arrives another way (a hand-edited file, an
  import over existing pies) greys the slot naming a pie that is already
  open, with the reason.
- **PM-Q17 Any line's state.** A slot whose line isn't one of the pies'
  own view, shading, toggle, falloff or action-centre lines or an Input
  Map action is greyed from its command's answer and, when the answer is
  "on", shows a filled dot. A line no command has is red and does
  nothing (UIC-221); an `@macro` line counts as known, as on toolbox
  buttons (UIC-222), until macros exist.
- **PM-Q18 The name in the centre (UIC-313).** One line, at most 130 px,
  ending in "…" when longer, on a small plate that replaces the dot; the
  arrow toward the highlighted slot sits just outside the plate.
- **PM-Q19 Input Map rows.** Shipped pies keep their old row names (Pie:
  View Direction ...); the saved map stores the line `operandi.pie.open
  <id>`, and a map or layout key saved with the old names moves to the
  new lines. A user pie's row comes after the factory actions, named
  "Pie: <its name>". Assign Key opens the Input Map on that row, waiting
  for its keys; nothing is saved until Save Input Map.
- **PM-Q20 Pie files (UIC-312).** Export Pie writes JSON (`.oppie`):
  version, the pie's id and its form with every sub-pie form it reaches.
  Without a file panel it saves to the data folder's Saved Pies folder,
  as Save Form does (LAY-Q67). Import Pie with an id that is taken asks
  Rename / Replace / Cancel; Rename gives `<id>.2` (then .3 ...) and
  points slots at the renamed sub-pies; Replace saves over the user's
  copy (Revert still brings a shipped pie back).
- **PM-Q21 Dragging slots.** In the Form Editor's ring, dragging a slot
  onto another swaps the two (`operandi.form.swap`), so every other slot
  keeps its place, rather than moving the rest along.
- **PM-Q22 Sub-pie slots.** A sub-pie slot shows its own label or else
  the sub-pie's name, with a small triangle at its outer end pointing
  away from the centre (a tiny flat glyph, not an icon).
- **PM-Q23 Pick (Form Editor, 2026-11-27).** Pick sits beside History
  under a Command field (pie slots and every button control). The next
  click on a button drawn in the window (toolbox, toolbar, workbench, the
  in-window menus) fills the control: its line, the icon the clicked
  button wears (its own, else its tool's), and its label only when the
  control has none. The control is found by its line among all forms, so
  a line on several buttons takes the first form's look. Lines the Form
  Editor itself runs (operandi.form.*, operandi.pie.*, workbench edits)
  still run while picking, and a key's line is never picked. The native
  macOS menu bar's entries can't be picked (they aren't clicks in the
  window). The pointer is the system pointing hand while picking (Jason,
  2026-11-27), for Pick to Add too.
- **PM-Q24 Icons (UIC-314).** Browse Icons lists the icon studio's icons
  (rendered set), stored by the name forms already use; a custom file
  (PNG, JPEG or SVG, drawn at up to 64 px) replaces a built-in icon on
  that control and the other way round. Choosing a file copies it into
  the data folder's Layouts/Icons (the same picture again reuses the copy;
  another picture with the same name becomes name-2). Clear removes both.
- **PM-Q25 Icons in files.** Export Pie embeds each custom file (base64,
  by name) and Import Pie writes them into the importer's Layouts/Icons.
  Save Form, which has no Import Form yet, copies them into a
  "<form> icons" folder beside the saved file and names those copies.
  A missing file draws a dashed box with "?" in the pie and the editor,
  and the editor says "Missing file".

## Texture Locators, stage 1 (texture-locators, 2026-11-07), provisional

Spec: docs/specs/BEHAVIOR-TEXTURE-LOCATOR.md.

- **TL-Q01 Image Map without UVs.** The spec starts an Image Map on UV Map
  "when the mesh has UVs". With no UVs it starts on UV Map too (renders once
  UVs exist). Should it start on another projection (Planar?) then?
- **TL-Q02 Option lists.** Settled by the full spec (2026-11-07): Front
  Culling, Random Offsets, Falloff and Tangent use its names; files saved
  with the stage 1 placeholders read through aliases (Front Only → Cull
  Backfacing, Item → Particles and Replicas, Part → Mesh Parts, Linear →
  Linear X).
- **TL-Q03 Auto Size details.** Applies to every visible mesh polygon the
  layer's enclosing Material Groups match (item mask and polygon tag).
  World Coordinates off: the box of those points in each item's own space
  (one union across items). A flat side (zero extent) keeps the locator's
  current size instead of 0.
- **TL-Q04 One UV map per surface.** A render surface exports one UV set.
  When layers on the same surface ask for different UV maps, the first one
  asked wins. A subdivided surface without the named map falls back to the
  first map.
- **TL-Q05 Planar repeat counts.** Planar u, v are multiplied by the repeat
  counts after centring: ((L + 0.5) × repeat), so tiles grow from the
  image's corner at L = −0.5, not from the locator centre.
- **TL-Q06 Viewport.** The viewport still shows an image layer as its
  average colour; only Solid (an image covers nothing) is reflected.
  Placement and tiling aren't drawn; the locator gizmo is a later stage.
- **TL-Q07 Missing locator.** A layer whose locator item was deleted from
  the Items list renders as UV Map with repeat 1, and its Locator setting
  shows "(missing)". Deleting the Texture Group item itself uses the normal
  item delete.
- **TL-Q08 Expander.** The locator expander is a small arrow after the
  type tag, separate from the arrow that opens a texture's mask children.
- **TL-Q09 Rotation order.** The sub-tab offers XYZ and ZXY, the orders
  item transforms support today.
- **TL-Q10 Mixed tiling in renders.** With different modes on the two axes
  the coordinates are tiled with shader maths first and the image read with
  Edge (or Clip for Reset); bilinear filtering can show a faint seam at
  tile borders. Same-mode axes use the image's own extension.
- **TL-Q11 Procedural UV coordinates.** Settled by the stage 2 spec:
  procedurals get (U·wrapU, V·wrapV, 0); UV Rotation and tiling don't apply.

## Procedural textures, stage 2 (procedurals-stage2, 2026-11-07), provisional

Spec: docs/specs/BEHAVIOR-PROCEDURAL-TEXTURES.md.

- **PT-Q01 Zone names.** Grid: Line / Filler; Dots: Dot / Filler (from the
  spec). Checker: Checker 1 / Checker 2; Noise: Zone 1 / Zone 2 (our
  choice). Properties rows read "<zone> Value / Color / Alpha".
- **PT-Q02 2D plane orientation.** A 2D pattern on Solid uses the plane
  perpendicular to the Projection Axis with the same in-plane axes as a
  Planar image: X (−z, y), Y (x, −z), Z (x, y). This matters for Line,
  Triangle and Hexagon grids and Hexagon dots.
- **PT-Q03 Line grid direction.** Line = lines of constant u (vertical
  lines on axis Z, at u = ±0.5…).
- **PT-Q04 Triangle diagonal.** The diagonal set runs through the square
  corners along u − v = const, same Line Width measured across the line.
- **PT-Q05 Hexagon geometry.** Grid Hexagon: cells 1 locator size across
  the flats, a cell centred on the locator, neighbours along u. Dots
  Hexagon: rows 1 cell apart (not √3/2), odd rows shifted half a cell.
- **PT-Q06 Ramps.** Line/dot edges ramp over Transition Width centred on
  the edge. Line Width 100 % with a ramp leaves the cell centres at a 50 %
  mix rather than all line.
- **PT-Q07 Noise maths.** Our own gradient noise on the CPU (tests, future
  previews); renders use Cycles' Noise Texture, one node per octave, so
  they match in scale, contrast and look, not value for value. Frequencies
  is limited to 1..8 (one node each). Turbulence: t = 2 × the weighted mean
  of |octave|, clamped to 0..1.
- **PT-Q08 Smooth Tops.** The first octave's t scales every finer octave by
  (1 − Smooth Tops × t).
- **PT-Q09 Flow Noise.** CPU: each lattice gradient turns about its own axis
  by Flow Phase full turns (one cycle per 1.0). Cycles also loops every
  1.0 (2026-11-19): each octave crossfades two 4D noises, at W = the
  phase's fraction and W = fraction - 1, with cos/sin weights. It matches
  the viewport in look, not value for value.
- **PT-Q10 Viewport.** Procedural layers show as the zone mix at t = 0.5
  (Bias/Gain ignored there); the row tooltip says so.
- **PT-Q11 Stored only.** Value / Hue / Saturation / Alpha Variation for
  Checker, Grid and Dots.
- **PT-Q12 Dot Width range.** 0 to 1000 % (dots past 100 % overlap their
  neighbours' cells; only the own cell's dot is drawn).

### Projections, stage 3 (projections-stage3, 2026-11-07), provisional

- **TL-Q12 Front Culling scope.** Culling and the Flip modes render for
  Planar only (the projection with one direction). Cylindrical, Spherical,
  Cubic, Box and Triplanar ignore it; UV Map too.
- **TL-Q13 Flips.** Flip Backfacing mirrors u and v about the locator
  centre (before repeat), Flip U / Flip V one of them.
- **TL-Q14 Box negative faces.** Each negative face mirrors u, which reads
  right way round from outside on all three axes.
- **TL-Q15 Cubic ties.** On equal normal components X wins over Y and Z, Y
  over Z (Box the same, plus the normal's sign).
- **TL-Q16 Triplanar frame.** Triplanar follows the locator's rotation, like
  Cubic.
- **TL-Q17 Falloff measure.** Measured in locator space (so it follows
  rotation and Size): per axis |L·f|; Cubic takes the largest, Spherical the
  length, Linear one axis; opacity = 1 − that, clamped. Applies to every
  projection, images, Constants and procedurals.
- **TL-Q18 Normal.** Cubic/Box/Triplanar and culling use the shading normal
  (smoothed, before bump).
- **TL-Q19 Auto Size uniform.** Chosen (2026-11-07): when the locator's
  Size is uniform, Auto Size sets all three axes to the box's largest side;
  otherwise each axis to its own side (a flat side keeps its size).

### Procedurals, stage 4 (procedurals-stage4, 2026-11-07), provisional

- **PT-Q13 Cellular measure.** Feature points: one jittered point per unit
  cell (3D). Angular: d = 1 − clamp(2 (F2 − F1)) (0 deep in a cell, 1 on
  the edge between the two nearest points); Round: d = 2 F1. Cell where
  d < Cell Width, ramped over Transition Width. Frequencies average finer
  copies like Noise; Smooth Tops scales the finer ones by (1 − st × t₀).
  Renders use Cycles' Voronoi; since 2026-11-21 the core and the viewport
  use the same Voronoi (MXG-Q11), so the cells sit where renders put them.
- **PT-Q14 Variations.** Per cell three randoms (brightness, hue,
  saturation): brightness × (1 − a·r), hue + a·(r − ½), saturation ×
  (1 − a·r); colour effects only. Cells: Checker squares, Grid cells
  (Triangle: each half square; Hexagon: each hex), Dots cells, Cellular
  cells.
- **PT-Q15 Alpha Variation.** Checker/Grid/Dots: alpha × (1 − a·r) with the
  brightness random (cells that darken also fade).
- **PT-Q16 Wood.** r = distance from the axis + Waviness × Ring Scale ×
  noise(p / Wave Scale); rings = ½ + ½ cos(2π r / Ring Scale) (filler at
  the centre, ring half a spacing out); grain = noise(p / Grain Scale)
  mixed in at Graininess / 2.
- **PT-Q17 Ripples.** Sources fixed pseudo-random points in the locator's
  unit cube (up to 32); t = ½ − ½ × mean cos(2π (d / Wavelength + Phase)),
  crests at t = 0.
- **PT-Q18 Weave.** Yarns 0.5 apart (one over-under per locator size);
  the horizontal yarn is on top where floor(2u) + floor(2v) is even;
  Roundness blends flat (1) toward a tube profile √(1 − x²) across the
  strand. No transition ramp at yarn edges.

### Textures, stage 5 (textures-stage5, 2026-11-07), provisional

- **PT-Q19 Gabor mapping.** Renders use Cycles' Gabor Texture (2D, in the
  plane perpendicular to the axis): Scale 4 kernels per metre, Frequency
  × 32 cycles per kernel, Anisotropy = 1 − Bandwidth, Orientation as set;
  Random Offset shifts the input as a seed. The core has its own sparse
  Gabor noise for tests (look, not values).
- **PT-Q20 Gabor specks.** t = 1 − smoothstep(0.5, 0.9, |noise|): mostly
  zone 2 with zone 1 specks where the noise peaks, as the spec's default
  look; clean stripes show as thin lines at the stripe crests.
- **PT-Q21 Brightness / Contrast.** Gabor and Poisson: t' = clamp((t − ½)
  (1 + Contrast) + ½ + Brightness), in place of Bias/Gain (hidden for them).
- **PT-Q22 Poisson Cells.** Scale = 2^(Level − 1) cells per metre;
  feature-point randomness 0.25 (evenly spaced); Cell Width × Density is
  the cell size; Angular/Round as Cellular; Solid 3D Cells off = 2D cells in
  the plane. Cells don't tile exactly at the locator square's edges (Cycles'
  Voronoi has no period). Use Profile Gradient ramps across the whole cell
  instead of the Transition Width.
- **PT-Q23 Viewport.** Procedural layers are evaluated at each mesh corner
  (item space) and blended across polygons: right on dense meshes, coarse
  on low-poly ones. Images still show their average. A selected polygon
  shows the selection fill.
- **TL-Q20 Gizmo.** A Texture Locator draws when its item is selected or a
  selected Shading layer uses it: its axes plus Planar's square, a
  cylinder, three rings, or a unit cube, at Size, from the locator's world
  matrix. The interactive Planar drag is not built (skipped as large).
  *Update 2026-11-09 (planar-drag):* per the Planar spec there is no
  dedicated drag (the item tools place it; PD-Q02); a World-Coordinates-off
  locator now draws through its surface item (PD-Q07), and Planar draws an
  up tick and a projection-axis line (PD-Q13).
- **TL-Q21 Stage 6 storage.** (Front, Light Probe, Implicit UV and Projection Camera render since 2026-11-17: TL-Q30..Q32.) Front, Light Probe, Implicit UV, Random
  Offsets + Texture Offset Strength, Real World Size + X/Y, Tangent, Particle
  Source, Texture Replicator, Projection Camera and World Transforms are
  stored and saved (version 23), not rendered.

- **TL-Q11 Procedural UV coordinates.** The spec gives procedurals
  (U·wrapU, V·wrapV, 0) under UV Map; UV Rotation is applied to them too.

## Shader tree remaining, stage 1 (hand-off shader-tree-remaining-2026-11-07)

Built with the hand-off's defaults SQ-1, SQ-2, SH-1 (control switches fall
through now, not in stage 7), SH-2, SH-3 (the Light Group is an item group),
SH-4, SH-5, SH-6. Provisional readings of our own:

- **SR-Q01 Indirect Illum Type entries.** Only the starting value
  (Irradiance Caching) is specified, so the menu has that one entry.
  Stored only.
- **SR-Q02 RGBA coverage.** The colour folds exactly as a Diffuse Color
  layer (the image's alpha is also its coverage, per Image Map Alpha). The
  cut folds with the layer's opacity and masks, then cuts where the folded
  value is above 50 %: an RGBA layer at 50 % opacity or less cuts nothing.
- **SR-Q03 Indirect multipliers.** Indirect Illum Multiplier and both
  Indirect Saturations are stored only: Cycles has no per-surface control
  of light arriving by bounces.
- **SR-Q04 Direct Illum Multiplier and Receive Shadows off** render through
  copies of each scene light (strength × multiplier; nothing shadows the
  copy when Receive Shadows is off) that light only those surfaces. Not
  copied: the default studio sun (scenes without lights), cylinder and
  mesh lights, and the world. Those light such surfaces at full strength
  and still cast shadows on them.
- **SR-Q05 Alpha Type Constant** holds the surface out of the alpha by
  1 − Alpha Value (Cycles Holdout). The surface still renders, its colour
  premultiplied by that alpha; Dissolve and cut-outs still lower alpha.
- **SR-Q06 Fog** applies to camera rays only: a fogged surface seen in a
  mirror isn't fogged, and fog doesn't light other surfaces. Distance is
  straight-line from the camera (SH-6).
- **SR-Q07 Visible to Occlusion Rays** is stored only until the Occlusion
  texture / AO output exist.
- **SR-Q08 Viewport.** The viewport shows none of the cut-outs or fog;
  renders (Render Viewport, Preview) do.
- **SR-Q09 Shader light linking, Include with no group** links nothing
  (spec §7: "No group chosen: no linking"), unlike a light's own Linking
  where Include with no group lights nothing. Non-light members of the
  group are ignored.

## Ctrl constrain (HM v1, 2026-11-07)

- **HM-Q01 Move plane squares.** HM-13 asks for Ctrl on a Move plane
  square to lock within that plane. Operandi's Move has only three axis
  arrows and the centre free-move square; adding plane squares is new UI
  (needs Jason's OK under the UI bible). Not built. Ctrl on the centre
  square behaves like a Ctrl haul (work-plane axis lock).
- **HM-Q02 Ctrl Tweak distance** (provisional). The locked Tweak moves the
  grabbed element by the drag's world movement projected onto the axis;
  in a view looking along the axis that is less than the pointer moved.
- **HM-Q03 Ctrl on a Scale axis handle with unequal start values**
  (provisional). All three axes are multiplied by the same factor, so
  equal values stay equal and unequal ones keep their ratios.
## MCP bridge, still-life gaps (2026-11-07, provisional)

Operandi's own interface, not a reference behaviour; recorded so Jason can
confirm the choices.

- **MCP-Q01 scene_save is Save As.** *Decided (Jason, 2026-11-07): keep.* In the running app `scene_save` now
  makes the saved file the document (title loses its `*`), so `scene_new`
  works after it. `copy: true` keeps the old save-a-copy behaviour.
- **MCP-Q02 Changing a light's type.** *Decided (Jason, 2026-11-07):* keep
  `light_set type` and add a Type dropdown to the light's Properties panel
  (done). A switch is one undo step; it keeps the shared settings (colour,
  intensity number, shadow, render, visibility, light material, linking,
  transform, name) and settings both types use (radius between Point,
  Spot, Cylinder; falloff between Point, Spot, Area; size and shape between
  Area and Portal), and resets the rest to the new type's defaults
  (`LightSettings::with_kind`). Intensity is converted (Jason,
  2026-11-07: "switching light types should convert brightness"); see
  LT-Q01. MCP reports what it measures as `intensity_measures`.
- **LT-Q01 Brightness rule on a type switch** (decided by us, 2026-11-07).
  The new Intensity gives the same irradiance at a reference point 1 m in
  front of the light, on its axis, facing it
  (`LightSettings::irradiance_per_intensity`): Directional E = I (sun
  strength); Point, Spot, Photometric E = I / 1 m²; Area E = L·A (outline
  area, ellipse π/4·w·h); Cylinder E = L·2r·length (side-on); Dome
  E = πL. Area and Cylinder use the far-field form whatever their size,
  so a big area light switched to Point is brighter close up and matches
  from a few sizes away. Not converted (number kept): Mesh (its glowing
  area belongs to another item) and Portal (emits nothing). Ignored: the
  item's scale, Falloff setting, a Photometric file's own scale, and the
  spot cone (on-axis brightness doesn't depend on it).
- **MCP-Q03 mesh_get no longer reports a per-face `smooth` flag.** *Decided
  (Jason, 2026-11-07): fine.* It never
  affected shading (SS-Q-OP01); `smoothing` (the scene-wide rule, set with
  `smoothing.set`) replaces it.
- **MCP-Q04 Capture timing.** `viewport_capture` with another view shows
  that view in the app until eframe's screenshot arrives (one or two
  frames), then puts the view back.
- **MCP-Q05 Render settings on the scene.** Samples (default 16, the
  renderer's old fixed value), Denoise and Adaptive Threshold now live on
  the scene (native version 27) and are set with `render_settings_set`.
  No panel shows them yet, and no spec describes a render settings panel.
  Exposure, white level, ISO and gamma are the Final Color Render
  Output's own settings; `render_settings_set` edits those (the per-output
  Denoising stays stored only, RO-Q07; the scene's Denoise is the one used).

## Shader tree remaining, stage 2: Gradient and Drivers (gradient family v1)

Built with GQ-1 (clamped smooth curve), GQ-2 (slope to 180°), GQ-3 (layers
below), GQ-4 (brightness), GQ-6 (new key at input 0). Our own readings:

- **GR-Q01 Renderer ramp.** Cycles reads a curve as 64 evenly spaced
  samples between its first and last key, linear between samples: a
  Stepped key softens over 1/64 of the span.
- **GR-Q02 Stored-only inputs** (Thickness, Bumpless / Locator Incidence,
  XYZ Distance to Locator, particle inputs) and **Group Mask /
  Layer Mask** inputs read 0. Masks aren't folded channels in Operandi.
- **GR-Q03 Editor.** Inline Value / Color strip (drag keys, middle-click or
  Ctrl+Alt+click adds) plus a numeric key list with shape and remove. No
  Edit Gradient window, presets or Recent group yet.
- **GR-Q04 Texture U / V** read the map the layer's UV Map projection
  names, raw 0 → 1 (its Repeat and UV Rotation are ignored). With any
  other projection (a new Gradient starts on Solid), or UV Map with no map
  named, they read the mesh's first UV map: its legacy per-polygon UVs if
  it has them, else the first map by name (2026-11-09: renders used to
  read only the legacy UVs, so every primitive's "Texture" map read 0). A
  mesh without UVs reads 0, and the render's status line says so. Is UV
  Map required, and does Repeat apply?
- **GR-Q05 Distance to Locator** is metres from the locator's origin in
  the world; from the world origin when the layer has no locator.
- **GR-Q06 Viewport** shows a Gradient's colour at incidence 0 and
  distance 0 (it has no view there). Screen X/Y/R read 0 there too.
- **GR-Q07 Screen X / Y / R ranges** (built 2026-11-09, provisional; the
  reference's ranges aren't measurable by script). Screen X: 0 at the
  render frame's left edge, 1 at its right. Screen Y: 0 at the bottom, 1 at
  the top. Screen R: 0 at the frame centre, 1 at the corners (distance in
  pixels over half the frame diagonal, so the frame's aspect counts). A
  region render keeps the whole frame's coordinates. Right ranges and
  directions? Pixels or 0..1 instead?
- **GR-Q08 One UV map per material in Cycles.** A material's layers all
  read the same UV map in a render: the first named one the tree asks for.
  A Texture U Gradient on map A beside an Image Map on map B reads A for
  both.

## Shader tree remaining, stage 3: Vertex Map and Variation textures

Built with GQ-5 (weights clamped) and GQ-7 (plain mask reading). Ours:

- **VM-Q01 Missing point values.** A point the map doesn't list reads
  weight 0 (colour black, opaque) but still counts as covered; only a mesh
  without the map at all leaves the layer out.
- **VM-Q02 Subdivided surfaces.** Smooth pieces of a subdivision surface
  get no vertex map value (the layer does nothing there); variation
  numbers still work, taken from the cage polygon.
- **VM-Q03 Colour maps on scalar effects** use the colour's brightness.
- **VM-Q04 Islands** are polygons sharing a point; each item's islands and
  each item get a stable random number from its id (SplitMix64), so
  renaming or moving an item keeps its number, duplicating gives a new one.
- **VM-Q05 Variation Particle / Texture Particle** read 0 until Operandi
  has particles.

## Shader tree remaining, stage 4: Process and Occlusion

Built with PQ-1 (reuse the two-zone Bias / Gain), PQ-2 (order), PQ-3
(nothing below: nothing), PQ-4 (other occlusion types render as Uniform).

- **PR-Q01 Bias / Gain fit.** Operandi's two-zone curves (Perlin's) land
  within 0.07 of the measured Process points, not 0.04. Schlick's form,
  t / ((1/b − 2)(1 − t) + 1), hits Bias 30 % exactly and the rest within
  0.03. Switch both the textures' and the Process curves to it?
- **PR-Q02 "Directly below"** is the next enabled sibling below in the same
  Group / Folder, with the same effect; a Process above anything else
  (material, mask, another Process, a texture of another effect) does
  nothing.
- **PR-Q03 Occlusion settings** stored only: Spread Angle (Cycles' AO
  uses the whole hemisphere), Max Cavity / Cutoff Angle, Variance, Vertex
  Cache, and Visible to Occlusion Rays (Cycles' AO sees every surface).
  Same Surface Only = only this object. Thickness fires rays inside.

## Shader tree remaining, stage 5: group Scale, Apply to Sub-Group, Material extras

Built with XQ-1 (50 % = half size), XQ-2 (nested multiply), XQ-3 (Apply
Scale resets to 100 % and bakes sizes), XQ-4 (anisotropy = 2 × (phase −
0.5)). Ours:

- **GX-Q01 Distances for "surfaces in the group"** are scaled on the
  group's result, so a Base Material outside the group is scaled too where
  the group applies; a group mask or partial opacity blends the scaled and
  unscaled values.
- **GX-Q02 Apply Scale on shared locators.** A locator used by layers
  outside the group changes for them too. UV Map locators bake into the
  repeat counts, others into the locator's Size.
- **GX-Q03 Apply to Sub-Group** applies when any tagged group anywhere
  inside matches the polygon.
- **GX-Q04 Rendered extras:** Use Refractive Index (Principled IOR =
  Refractive Index, Specular IOR Level 50 %), Subsurface Phase, Rounded
  Edge Width (Cycles Bevel, no angle limit, all surfaces), Absorption
  Distance on Glass with a constant Transparent Color (Volume Absorption).
  Dispersion is stored: Operandi's glass is a Glass BSDF, which has none.
  Everything else in the table is stored. A normal map replaces the
  rounded normal; a bump adds to it.

## Shader tree remaining, stage 6: Render Outputs (render outputs v1)

Built with RQ-1 (processing order) and RQ-2 (range 0 = unlimited). Ours:

- **RO-Q01 Specular / Reflection Shading** are both Cycles' glossy light
  (it doesn't split highlights from reflections).
- **RO-Q02 Subsurface Shading and Motion Vector** render empty: Cycles
  folds subsurface into diffuse, and motion needs motion blur data.
- **RO-Q03 Outputs inside a Material Group** *Update 2026-11-17 (RO-Q03 v2):
  built.* Each of the first 8 grouped outputs gets a coverage AOV that
  every surface its group applies to writes 1 into; the output is
  multiplied by it (anti-aliased edges, alpha too). Later grouped
  outputs still record every surface (Cycles ABI 25). Was: record every surface (RQ-3's
  zero fill isn't built).
- **RO-Q04 Saving.** Each output other than the first Final Color saves
  next to the render as "Render N (Output name).png" in the render format;
  an untouched Alpha output isn't saved apart (the picture has alpha).
  Output Filename / Format are stored only. *Update 2026-11-17 (RO-Q04 v2):*
  both now apply when a render is saved: Filename is relative to the
  render's folder (empty = "Render N (Name)"), Format any of the Save
  Image formats ("(as the render)" keeps the render's).
- **RO-Q05 Depth** is Cycles' depth pass (along the view axis), not the
  straight-line distance; they agree at the picture's centre.
- **RO-Q06 Geometric Normal** shows the shading normal (no separate pass).
- **RO-Q07 Stored only:** per-output Denoising and Antialiasing,
  contours, per-channel levels. *Update 2026-11-20:* Colorspace now
  applies (RO-Q13).
- **RO-Q08 Approximations:** Photographic exposure = ISO / 100; Bloom is a
  three-pass box blur; Vignette is a cos⁴ falloff × Amount (not from the
  focal length and film size).

Cycles built-in outputs (branch cycles-render-outputs, 2026-11-20), all
*provisional*. RO-Q09 is left for the material-value thread.

- **RO-Q10 Illumination (Direct / Indirect / Total)** are Cycles' diffuse
  light passes (light already divided by the diffuse albedo); Total is
  their sum. Surfaces with no diffuse part (pure metal, glass) show black.
  Starting state: a scene with a chrome sphere; Illumination (Total): is
  the sphere black, or should it show the light reaching it?
- **RO-Q11 UV Coordinates** is (u, v, 0) from the UV map the surface's
  image lookups use, else the polygon's own UVs, else the first map; no
  UVs = black. With the output on, every mesh sends Cycles its UVs.
- **RO-Q12 Segment ID**: a segment is one rendered surface piece (an item
  and its first material tag of a Cycles surface; tags whose surfaces
  come out identical share one). Colour: a stable hash of item id and
  material tag → hue, full saturation, value 0.5–1, the same across
  renders and reopening. Written through the Cycles object colour. Cycles'
  Object / Material Index passes are the alternative if ids are wanted
  as numbers.
- **RO-Q13 Colorspace** is Default / Linear / sRGB (no OCIO config yet; an
  OCIO list comes with the A31 / MaterialX colour work). Default: beauty
  outputs are sRGB in 8/16-bit files and linear in EXR / HDR; data outputs
  (Alpha, Depth, Normals, Object / UV Coordinates, Motion, Incidence,
  Segment ID, Cryptomatte) are always linear, so an 8-bit Depth file now
  holds its linear numbers, not sRGB-encoded ones. Linear / sRGB force the
  numbers (sRGB in an EXR = sRGB-encoded floats). The render window shows
  linear outputs raw and forced-sRGB ones through sRGB; Default beauty
  outputs follow the window's view switch. Cryptomatte ignores it (greyed).
- **RO-Q14 Material values and colour space**: the number-valued material
  outputs (amounts, coefficients, roughness, dPdu / dPdv) count as data
  (always linear at Default); Subsurface / Transparent Color act like
  Diffuse Color (beauty rules).
- **RO-Q15 Cryptomatte levels**: Cycles keeps one depth for every type, so
  a render uses the most levels any Cryptomatte output asks for (rounded
  up to even, 2–16); outputs of one type share one set of layers.
  Several outputs with different types coexist.
- **RO-Q16 Cryptomatte names**: Object = the item's name, Asset = its
  top-level ancestor's name (its own without a parent), Material = the
  polygons' material tag; clay (untouched) surfaces get a named clay
  surface of their own so the tag still counts, and the plain default
  surface is "Default". Duplicates: the same name is the same id (two
  items called "Cube" are one matte). Manifests come from Cycles; names
  Cycles writes unescaped are repaired, nameless lights / world dropped.
- **RO-Q17 OpenEXR Multilayer** is a Save Image format in the render
  window, not an output Format (it is the whole render). One part: the
  picture as R, G, B, A; every other output as "<Output name>.R/.G/.B/.A"
  (single-value outputs keep RGB too; dots in names become "_", repeated
  names get " 2"); Cryptomatte as "CryptoObject00.R/G/B/A"... with the
  standard name / hash / conversion / manifest header attributes. All 32-bit
  float, scanline ZIP (lossless). The render cache keeps each slot's raw
  Cryptomatte layers (slot-<id>-crypto.exr) so a history slot saves them.
  The window preview of a Cryptomatte output is each pixel in its top id's
  colour.

Render outputs group D (branch render-outputs-d, 2026-11-20). D-Q1..Q4
are Jason's answers (2026-11-20, source: reference behaviour); the rest are
*provisional* guesses.

- **RO-Q18 Illumination (Unshadowed)** (D-Q1, Jason): only the light items'
  direct light, with their shadows removed: no environment / sky, no
  glowing surfaces, no bounce light. A sky-only scene renders black.
  Built as the helper render's Diffuse Direct alone (`unshadowed_settings`
  in operandi-cycles/src/outputs.rs: empty world and dome, glowing
  surfaces in no light set (`lamps_only`), light shadows off, bounces 0).
  Surfaces with no diffuse part (metal, glass) show black, as RO-Q10.
- **RO-Q19 Helper render** (D-Q2, Jason): a second Cycles render after the
  main one, only when an Illumination / Diffuse Shading (Unshadowed)
  output is enabled; same camera, size, samples, seed and noise
  threshold; transparent bounces stay the main render's; Direct
  Caustics' caustics-only light copies are left out; not denoised. The
  render window shows "Unshadowed pass" while it runs. Finish early also
  ends the helper early; Cancel cancels both. *Provisional:* the Preview
  never runs it (the outputs appear after a final render only). Known
  approximation: an area light's direct light found by bounce rays is
  still blocked; point, spot and sun lights are exact.
- **RO-Q20 Diffuse Shading (Unshadowed)** (D-Q4, Jason): Illumination
  (Unshadowed) × Diffuse Color, treated as exact.
- **RO-Q21 Light Path Expressions** (D-Q3, Jason): shipped with its gaps;
  an expression the passes can't give renders black, with "Not supported
  by the renderer: <reason>" under the field and in the render window's
  status line, and the nearest exact expression in the tooltip.
- **RO-Q22 Language** (provisional): `C` first; `<RD> <RS> <TD> <TS>`
  (also `<R.>` etc. and `<RD'label'>`), `R T D S`, `L O B V`, `.`, `.*`
  (once, `*` only after `.`), `[ ]` without labels or `.`, `'label'` after
  an event; spaces are ignored; `+` is refused. Errors give the
  character position.
- **RO-Q23 Diffuse** (provisional): `<RD>` maps to Cycles' diffuse passes and
  counts as exact although translucency and subsurface sit in them; `R`
  and `T` (which take in only part of the diffuse pass) are approximate;
  `<TD>` alone is not supported.
- **RO-Q24 `CL`** (provisional): lamps the camera sees directly share the
  Emission pass with glowing surfaces, so `CL` is approximate (`CO` and
  `C[LO]` exact). `CV` reads both volume passes (approximate).
- **RO-Q25 Light groups** (provisional): `C.*L'x'` and `C.*B'x'` read lights
  labelled x plus the world when the first visible environment's LPE
  Label is x (one group). Hidden groups `__lpe_lights` (lights with no
  label, for `C.*L`), `__lpe_emitters` (glowing surfaces, for `C.*O`) and
  `__lpe_world` are made only when an expression needs them and aren't
  shown or saved as light groups. Glowing surfaces have no LPE Label, so
  `C.*O'x'` is not supported. A light's own visible geometry with no label
  counts as a glowing surface.
- **RO-Q26 End events after a bounce** (provisional): Cycles' light passes
  count lamps, glowing surfaces and the environment together, so
  `C<RD>L` reads the diffuse pass whole (exact, as the spec's table says);
  `C<RD>O` / `C<RD>B` and similar are approximate.
- **RO-Q27 Add Per-Light Render Outputs** (provisional): Render menu, command
  `operandi.render.addPerLightOutputs`; one Light Path Expressions output
  per renderable light, `C.*L'<label>'`, named "LPE: <expression>"; a
  light with an empty LPE Label gets its item name as its label; lights
  sharing a label get one output; lights whose output exists are skipped;
  labels holding a quote are skipped; one undo step; nothing to add is an
  error in the status line.
- **RO-Q28 Light Path Expressions in the Preview** (provisional): not shown;
  after a final render only, like every output.
- **RO-Q29 Shaded AA Samples** (provisional): Cycles' sample count pass,
  heat map t = (samples − adaptive minimum) / (Maximum Quality − adaptive
  minimum), hue 240° × (1 − t), blue → cyan → green → yellow → red. The
  minimum is Cycles' automatic one (16 / threshold^0.3, at least 4).
  Noise Threshold 0: no pass, solid red, and the status line says "Noise
  Threshold is off: every pixel gets every sample". A data output: Image
  Processing and Colorspace are greyed (UIC-117) and never applied.
- **RO-Q30 Shaded AA Samples in OpenEXR Multilayer** (provisional): the heat
  map as `<name>.R/G/B/A` plus the raw t as `<name>.Y` (full float).
  8/16-bit files get the heat-map colours.
- **RO-Q31 Labels** (provisional): the website's Rendering list uses
  "Illumination (Unshadowed)", "Light Path Expressions" and "Shaded AA
  Samples", which match; "Diffuse Shading (Unshadowed)" isn't on it.

## Colour field and picker; polygon tag modes (colour-nav notes 07 and 04, 2026-11-07), provisional

- **CN-Q01 Pointer wrap.** The pointer hides during a swatch drag but
  doesn't wrap at the screen edge (egui has no pointer warp here); a long
  drag stops at the edge.
- **CN-Q02 Click on a number — answered** (Jason, 2026-11-08): a plain
  click on a swatch number opens the picker, same as anywhere on the
  swatch; numbers change only by dragging or by typing in the picker's
  fields. Built: a click is press and release within 3 points, and the
  picker opens on release; no double-click inline edit, no wait.
- **CN-Q03 Display space and drags.** With Display Color Values in Display
  Space on, numbers and typed values are display-encoded, but drag rates
  still act on the stored linear values. Display space is on by default
  (Research, 2026-11-21): colour numbers show and take sRGB-encoded values
  with 2 decimals, stored values stay linear, and swatches are the stored
  colour through the display transform whatever the number display.
  Units files from before (version 1) turn it on once on load.
- **CN-Q04 Picker settings** (model, wheel type, Stops, axis, strip) are
  kept for the session only, shared by every field. The Colour Theme
  Editor (dockable picker) is not built.
- **CN-Q05 Hex RGB field** is display-encoded when Display Space is on
  (the default); otherwise it is the stored value × 255, like the swatch's Hex unit.
  Kelvin is a curve fit (within ~3 % round trip over 2000–9000 K).
- **PT-Q3 Changing Polygon Tag Type in the mode** keeps the polygons
  selected and starts the region list again (the next click replaces).
- **PT-Q4 Regions across meshes.** Operandi selects polygons on one mesh
  at a time, so a Material region is selected on the clicked mesh only,
  not on every mesh with that material.
- **PT-Q5 Tag mode isn't saved.** A reopened scene is in Polygon mode with
  the same polygons selected. Plain entry (no Alt) keeps the current
  polygon selection; Grow, Invert and other selection operations change
  polygons without updating the region list.
- **PT-Q2 answered:** key 4 was unbound in the default input map; it now
  enters the tag mode.

## Image Viewer, stage 1 (branch image-viewer, 2026-11-07), provisional

- **IV-Q10 Strip numbers.** The number on a thumbnail is its place in the
  strip (1 = newest), so it matches the key that picks it (1–9, 0 = 10th).
  The render-order number ("Render 12") is in the tooltip and is the Save
  Image name. If the spec means the badge shows render order, keys and
  badges would disagree after the 10th render.
- **IV-Q11 Region vs pan.** A **Region** toggle in the render controls
  (on by default, remembered) decides a plain drag: on, it draws the render
  region (as before); off, it pans a zoomed picture. Middle-drag always
  pans; while rendering a plain drag pans. Which default does the spec want?
- **IV-Q12 Region renders are slots.** A finished Render Region adds a new
  slot (the old picture with the region re-rendered) rather than replacing
  the slot it started from. Rendering a region over an older slot copies
  that slot into the picture first.
- **IV-Q13 Cache files.** One plain float RGBA OpenEXR file per output
  (`slot-<id>-<k>.exr`, RLE-compressed) plus an 8-bit thumbnail and
  `index.json`, in Recent Frames (default: the app data folder's
  `Recent Frames`). A 1920 × 1080 output is about 20–33 MB, so ten slots
  can use a few hundred MB. Light groups are kept as extra outputs.
  Changing the folder starts the history from that folder's own index; the
  old folder's files stay.
- **IV-Q14 Integer formats and the view.** PNG/TIFF 16-bit get the
  display transform too (sRGB or Linear), like the 8-bit formats; only EXR
  and HDR keep linear floats. Load Image... treats every non-float file as
  sRGB and linearises it. HDR drops alpha.
- **IV-Q15 Clipping stripes** are in picture pixels (8-pixel period), so
  they grow when zoomed in. Only R, G and B count (alpha over 1 doesn't).
- **IV-Q16 Statistics.** Peak memory shows "Not reported by the renderer"
  (Cycles doesn't give it to us yet). "Polygons after subdivision" is the
  triangle count sent to Cycles. Times in tooltips are UTC.
- **IV-Q17 Keys.** The viewer's T, P, I, C, S, Cmd+A and Cmd+S are
  editable Input Map entries ("Image Viewer: …", context "Render window
  focused"); digits and plain Space are fixed (plain Space can't be bound).
  F9 in the viewer renders; Alt+Shift+F9 is global. Multi-select (Ctrl or
  Cmd+click) only marks slots in stage 1: nothing acts on a multi-selection
  yet (Compare and Save All Outputs come later).
- **IV-Q18 Statistics device.** "GPU" or "CPU" is read from Cycles' device
  name (anything without "CPU" counts as GPU).

## Navigation, lock and hide (nav-lock-hide, 2026-11-07), provisional

Built from the coordinator's NAV / LCK / HID brief; see
`specifications/nav-lock-hide-2026-11-07/AUDIT.md`.

- **NLH-Q01 What a locked element covers.** Locking reaches *down*: a locked
  polygon also locks its edges and points, a locked edge its two points.
  Selection in each mode skips locked elements of that mode.
- **NLH-Q02 Shared points (LCK-20 [Q]).** No command moves a point used by
  locked or hidden geometry, even when unlocked geometry shares it. Moves,
  rotates and deformers leave those points in place; an edit that changes
  topology and would alter locked/hidden geometry (Bevel of everything with
  a locked polygon) is refused: "This would change locked or hidden
  geometry". Hidden polygons' points are protected the same way, so moving a
  shown neighbour leaves the shared corners behind.
- **NLH-Q03 Hide reaches up (HID-03).** A hidden point hides every edge and
  polygon using it, a hidden edge the polygons using it; a hidden polygon
  hides only itself.
- **NLH-Q04 Invert in each mode.** Lock/Hide Invert swaps only the current
  mode's set (polygons in Polygon mode, edges in Edge mode, points in Vertex
  mode). In Item mode it swaps the selected items' polygons and points.
- **NLH-Q05 Item mode.** J/H lock or hide all geometry of the selected mesh
  items; Shift+J/Shift+H the other mesh items. Component modes act on the
  current mesh only.
- **NLH-Q06 Item lock and delete (LCK-05 [Q]).** Deleting a locked item is
  allowed. Parenting refuses when the item being parented is locked;
  parenting *under* a locked item is allowed.
- **NLH-Q07 Show Locked Mesh Items (LCK-25)** is one app-wide View menu
  toggle for now, not a per-view option.
- **NLH-Q08 Locked colour default** is a grey-blue (0.45, 0.55, 0.72); the
  spec names the theme entry but not a value.
- **NLH-Q09 Navigation numbers [OP].** Turntable 0.6 rad/s; (the flick,
  which slowed with e^(−3t), became look around on 2026-11-22); Zoom In/Out halve/double the
  distance; Align Selected faces the selection's area-weighted average
  normal, world up kept at the top.
- **NLH-Q10 Command names.** The name map has no entries for these actions,
  so they use their menu names (Lock Selected, Hide Invert, Zoom In, ...).
- **NLH-Q11 No UV viewport.** LCK-27 and HID-06 wait for one.

## Presets, Stage 0 (presets-stage0, 2026-11-07), provisional

Spec: PRESET-BROWSER.md (UI conventions bible), PB-40..52. Decided by
Jason: PB-Q1 (a preset is a cut-down scene file, `.oppreset`), PB-Q2
(default folder: Presets in the data folder), PB-Q3 (material load with no
material group: refuse, change nothing).

- **PB-Q4 Save Item Preset with a material selected.** The shader tree
  keeps its selection while its tab is hidden, so File > Save Item Preset…
  saves a material preset only while the Shading tab is in front with a
  material or material group selected; otherwise the selected items. Right?
- **PB-Q5 Render settings preset.** Operandi has no render item. The
  preset holds the scene's render settings (samples, denoise, adaptive
  threshold), the Render Output layers and the render camera (added as an
  extra camera on load); loading replaces the settings and every Render
  Output. Picture size lives on the camera, so it comes with the camera.
- **PB-Q6 Mesh presets and materials.** A mesh preset brings the Material
  Groups that shade its polygon tags; on load they are added only when the
  scene has no material for that tag (the scene's own wins). Right, or
  should the preset's material replace it, or never come along?
- **PB-Q7 References left behind.** Material Group item masks become
  "(all)" in presets; light linking groups, a light's prototype and item
  targets outside the preset are cleared.
- **PB-Q8 Transforms by kind.** Item presets: meshes drop the item
  transform (PB-44, with a note on save); lights and cameras keep their
  world placement. Assembly presets keep every transform (an arrangement).
  Should lights/cameras in an Item preset also land at the origin?
- **PB-Q9 Environment.** Operandi has no environment items yet, so an
  environment preset is every light (no meshes). Loading removes every
  light first. Should it also carry the scene's environment meshes, or the
  Shaders?
- **PB-Q10 Material load target.** The material preset replaces everything
  inside the selected Material Group; the group keeps its name, mask, blend
  and opacity. A selected Material (not a group) is refused, as PB-Q3.
## Primitives on a moved mesh (Jason, 2026-11-07)

Primitives (drawn, numeric and unit) now go into a moved, rotated, scaled
or parented mesh: built in world space, mapped into the item's local space,
item transform kept. Only a transform with no inverse is refused.

- **PRIM-MOVED-Q01 Unit primitive placement on a moved mesh** (provisional).
  Start: a mesh moved to (1.5, −2, 3), rotated and scaled 2 × 0.5 × 1.5.
  Ctrl-click Cube (or any unit shape). Unclear: is the unit shape made at
  the world origin at world size (built), or at the item's own origin, in
  its axes and scale? The unit-primitive spec says "centred on the origin"
  and "Size X is always world X", written for untransformed meshes.

## Edge loop around an n-gon cap (2026-11-07)

- **LOOP-NGON-01** (provisional). Start: 8-sided cylinder, each cap one
  n-gon. Double-click a rim edge (or Select > Loop). Built: the loop goes
  around the cap's perimeter (8 edges), then stays on that n-gon. Only
  polygons with more than four sides count, and only edges with a polygon
  on both sides (open borders keep their rule). Unclear: should a triangle
  beside a quad (a fan in a quad grid) also start a perimeter walk? Left
  out so the existing fan behaviour (stop at the pole) stays.

## Corner Pan/Rotate/Zoom drag feel (Jason, 2026-11-07)

Jason found the corner buttons "all over the place", with drift on an axis
he wasn't moving. Cause: the drag read the pointer's place relative to the
icon it is put back on, but the put-back lands late, so a place measured
from before it counted part of a move twice or not at all, and the first
frame jumped by the press's offset from the icon's centre. Now the drag
reads the mouse's own movement (raw motion) once the platform sends it,
counts the first frame from the press, and puts the pointer back where it
was pressed. Rates are unchanged (Pan 3x, Rotate and Zoom as in-view).

- **VBF-Q1 Raw motion units off macOS** (provisional). macOS reports the
  mouse's own movement in points; Windows and Linux report device units,
  taken here as physical pixels (divided by the display scale). A
  high-resolution mouse may make the buttons faster there: check when
  Windows/Linux are validated.
- VBF-2 (Jason, first QA build): on macOS the drag shook in place and
  snapped back. The mouse's own movement there includes each put-back as a
  move the other way, cancelling the real one. On macOS the pointer is now
  locked in place for the drag (hidden, no put-backs) and unlocked on
  release or Esc, then returned to the press; Windows and Linux keep the
  put-back.


## Shared tree list and panel groups (UI bible section 10, 2026-11-08)

Items, Shader Tree and Groups are drawn by one tree-list component
(`tree_list.rs`, UIC-130). Built per the bible; where it was silent or the
feature behind a column doesn't exist yet:

- **TREE-Q01 Items world/local space column** (UIC-150). Left out until
  Operandi has reference spaces (UIC-117: unbuilt features are left out).
  The edit-mode column shows the active mesh; clicking it makes that item
  the active mesh (provisional).
- **TREE-Q02 Sibling order in the Items list.** *Built 2026-11-08
  (branch item-tree-reorder).* Sibling order is the order of the scene's
  item array, which the native file already stores (no format bump).
  Drop zones: top quarter of a row = before it, bottom quarter = after it
  (on an open parent: its first child), middle = onto it; a scene row is
  "onto" (last top-level item) except its open bottom band (first item).
  Provisional choices: (a) a locked item can be reordered among its
  siblings but not reparented (LCK-05 keeps its links); (b) while Sort by
  Name/Type is on, a between-drop still changes the stored order, which
  the sorted list doesn't show; (c) a drop into another scene lands last
  there; (d) the insert line's level follows the hovered row only, so the
  gap after a branch's last child can't target the branch's parent level
  (drop below the list or before the next row instead); (e) no MCP verb
  yet (no reference name for it in the name map).
- **TREE-Q03 Groups group-type column and "Filter field"** (UIC-153). Left
  out: there is one kind of group and nothing to filter by yet. The funnel
  gives the search row. Delete Group and Add / Remove Selected moved from
  the old tool row onto the group's right-click menu (UIC-144).
- **TREE-Q04 Mesh Stack** (UIC-151). Not built or registered: Operandi has no
  mesh operators. The component has everything it needs (value column "#",
  state eye); it is a registration once operators exist.
- **TREE-Q05 Shader Tree's "one icon"** on the second tool row (UIC-152):
  unknown which; left out.
- **TREE-Q06 Icons.** The edit-mode column header is a letter "E", the
  funnel is a letter "F", and Groups rows use a "G" tag until the icon
  generator has recipes for them.
- **TREE-Q07 One panel kind per layout.** A panel's state belongs to its
  kind, so each kind is open once: the "+" menu greys kinds already in the
  group, and adding one open in the other group moves it there (not when it
  is that group's only tab). Close Tab is on a tab's right-click menu and
  never closes a group's last tab.
- **TREE-Q08 Alt+click solo** (UIC-138) remembers what it hid; Alt+click
  the same eye again shows those again. Locked items are left alone.
- Double-click on a row now selects its children (UIC-38) instead of
  renaming (FLAGS F9); rename is click, pause, click (UIC-39) or the row
  menu's Rename….

## Viewport Properties header (UIC-103 / FLAGS F11, UIC-111 / F13)

- One Tab ▾ and All Tabs ▾ are now preset menus; One Tab / All Tabs
  *display* moved to the O key over the open popover (O elsewhere still
  opens and closes it). Presets hold every row the tab shows except View
  Type (that is the view's camera). Saved in `viewport-tab-presets.json`.
- **VPH-Q1 "(new, selected tabs …)"**: a sub-menu with a tick per tab and a
  name field (provisional; the guide doesn't show how tabs are chosen).
- **VPH-Q2 Save Default for 3D View** makes new views (splits, Shift-drag
  copies) start with those settings, over the Grid Visibility preference.
  The "… Hauling" entries wait for hauling (left out, UIC-117).
- The label column is as wide as the longest label on show (one tab, or
  all tabs in All Tabs view), capped at 45% of the panel with "…" and a
  tooltip. The status-circle column (UIC-112) isn't in this panel yet: that
  belongs with the shared form component.
## Shared form component (2026-11-08, branch dialog-component)

UIC-80..99 and UIC-100..125 applied to every options dialog and
confirmation. Where a spec or a missing case needed a reading:

- **FORM-Q01 Spec titles vs UIC-81** (Jason OK'd, 2026-11-08). Dialog titles are now
  the command's name: "Scale Offset" (was "Enter Inset Amount", spec
  polyscale-offset), "Set Part" (was "Polygon Set Part"), "Set Material",
  "Set Position", "New Vertex", "Merge". If a spec's own title should win,
  say so.
- **FORM-Q02 Spec "no retained value" vs UIC-95** (Jason OK'd, 2026-11-08). Scale
  Offset's Inset starts at 0 m every time (SO-U02) and Set Part / Set
  Material's names start from the targets (SP-10/SP-Q03), so those rows
  are not remembered; every other dialog row is.
- **FORM-Q03 Recovery prompt's Cancel** (Jason OK'd, 2026-11-08). UIC-84 needs a
  Cancel; Recover Scene had only Recover / Discard. Cancel now keeps the
  journals on disk for the next launch and turns off journaling for the
  session so nothing overwrites them.
- **FORM-Q04 Polygon cap question** (provisional). It was Undo / Keep with
  Escape = Undo. Now "Undo the change?" OK undoes, Cancel/Escape keeps it,
  so Cancel changes nothing (UIC-96).
- **FORM-Q07 Control types** (provisional). Vertex Reorder Index's Mode
  (Swap/Top/Bottom) and Merge's Range (Automatic/Fixed) are now segments
  (UIC-89: 2–4 short options); CONTROL-TYPES row 17 had Mode as a
  dropdown.
- **FORM-Q05 Errors without new rows**. A refused OK (bad number, command
  refusal) keeps the dialog open; the reason goes to the status line and
  OK's tooltip, a bad number stays in its field outlined red. No error line
  is added under the form (UIC-82/92). Save As over an existing file: the
  first OK says "File exists: OK again replaces it", the second replaces.
- **FORM-Q06 Not converted**. LXO import progress / partial-import report /
  results (long scrolling reports, not options); Preferences and Input Map
  (editor windows); Work Plane Settings, Diagnostics, Snapping, Preview
  Settings and Smooth Shift Mini Properties (floating panels: panel mode,
  later). "..." labels keep three dots where they were; new ones use "…".
## Panel mode across the property panels (2026-11-08, branch panel-mode)

Every property panel now draws its rows through the shared form's panel
mode (`form::rows`, `option_row` forwards to it). Where a rule couldn't be
followed yet, or needed a reading:

- **FORM-Q08 Unbuilt rows vs earlier "show greyed" decisions.** UIC-117
  leaves unbuilt options out, so the greyed placeholder rows went:
  Tweak (Always Raycast, Mix Mode, Slip UVs), Sketch Extrude's
  unmeasured rows, Chamfer Edit's Absolute Size and unbuilt Mode/Shape
  entries, Slide's Duplicate, Bridge's Make UVs / Curve / Profiles,
  Ellipsoid's Handle Mode / Show Corner, Path Slice's Preview Geometry and
  depths, Snapping's unbuilt targets and rounding/angle rows, Preview's
  Heat Map / Bake Mode / Layer Overlay, a light's "Not used yet" values.
  **Edge Bevel / Edge Chamfer keep theirs greyed** (Jason asked for that,
  EBC-Q03). Should any other tool keep its greyed rows?
- **FORM-Q09 Status circles where no default is to hand** (closed,
  deferred, 2026-11-08). Corrected meaning (UIC-112 rewritten): the
  circle is the channel status light (None, Constant, Animated, Key,
  Driven, Driver, Mixed, Undefined). Each channel row carries a
  `ChannelStatus` (`form/status.rs`); only None (plain ring, at the
  default) and Constant (filled, non-default) can occur until animation,
  links and drivers exist. Rows that know their default show Constant once
  changed; a multi-selection shows Constant if any value is non-default.
  Clicking the light does nothing yet (later: set/remove key); it has no
  reset menu. No defaults-wiring pass: rows whose helpers don't know a
  default keep the light's space empty, as before.
- **FORM-Q10 Tool Properties' Apply / Drop / Cancel buttons** (provisional).
  These are tool actions under the rows, left as they were (full panel
  width), not field-column buttons (UIC-116 reads as being about property
  rows).
- **FORM-Q11 Help text in Tool Properties** (provisional). The weak
  one-line hints under a tool's rows ("Nothing selected insets every
  polygon.") stay; they aren't rows. Preferences pages' "left out" notes
  stay as notes under their sections.
- **FORM-Q12 Item channel drags** (provisional). Item Transform's arrows
  step 0.01 m / 0.5° / 0.01 per point (Shift ×10, Option or physical Ctrl
  ×0.1); one drag is one undo step. The steps are a guess.
- **FORM-Q13 Viewport Properties' Wireframe Opacity** steps by dragging its
  arrows sideways (UIC-53) instead of the old click-up/click-down arrows.
- Not changed: the toolbox tabs (Select, Create, Edit… are button grids,
  not property panels), the Preferences and Input Map windows' frames and
  page list, the colour picker, and UIC-103's header row (reserved slot,
  as the brief said).

## Edit tab wave 2 path slicing (path-slice, 2026-11-08), provisional

Spec `docs/specs/edit-wave2/WAVE2-PATH-SLICE.md` (its PS-Q1..Q8 are written
here as W2-PS-Q1.. so they don't clash with Polygon Slice's PS-Q1 above).
Built readings, each in its own function in `path_slice.rs`:

- **W2-PS-Q1 (inner points).** A path point inside a polygon lands on the
  polygon's plane, moved along the axis. A crossing within 1e-6 of an edge
  end snaps onto that end's point.
- **W2-PS-Q2 (open ends).** A run that ends inside a polygon makes no edge;
  the point it added on the polygon's edge stays.
- **W2-PS-Q3 (outline inside one polygon).** The outline becomes a polygon;
  the ring around it is split in two by bridges from the outline point
  nearest a corner, and the one halfway round, to their nearest corners (no
  single-edge bridge: Operandi's polygons can't repeat a point).
- **W2-PS-Q4 (Front Face Only).** "Front" = normal pointing to the axis's
  positive side (Y: upward faces).
- **W2-PS-Q5 (Path Steps).** Even by curve parameter over the whole curve;
  the curve passes through its points (Catmull-Rom). Auto keeps a sample each
  time the curve has turned Smooth Angle.
- **W2-PS-Q6 (Tunnel, Core).** Tunnel isn't built (greyed). Core deletes the
  considered polygons outside every closed outline (box sides too); an open
  path makes Core and Stencil act like Slice.
- **W2-PS-Q7/Q8.** Selected polygons only when any are selected (Polygon
  mode), else all; pieces of selected polygons stay selected.
- **W2-PS-Q9 (Pen self-snaps).** World Axis, Straight Line and Right Angle
  snap a new segment when within 5° (strength not specified).
- **W2-PS-Q10 (Action Axis for Pen/Curve).** The work plane's normal when the
  first point was placed. Primitive's is the clicked face's normal.
- **W2-PS-Q11 (shape frame).** A placed shape's X runs along the world axis
  lying flattest in the face (X, then Z, then Y). Heading turns about the
  face normal, Pitch about the shape's X, Bank about its Y.
- **W2-PS-Q12 (not built).** Right-drag resize from the centre, Show Rotate
  Handles, Exponent, Preset profiles and Extend Open Edge are greyed. The
  Pen's points are placed on the work plane, not on the surface.
- **W2-PS-Q13 (clones).** Clone X/Y repeat the shape at its full size plus
  Clone Gap; Center off puts the first copy on the place, the rest to +X/+Y.

## Edit tab wave 3 Polygon Axis Slice (path-slice, 2026-11-08), provisional

Spec `docs/specs/edit-wave3/WAVE3-AXIS-SLICE.md`.

- **AX-Q1.** Built as the spec's provisional rule (Number 1/0/0, Size
  0.25 m). Size: cuts Size apart, centred, as many as fit strictly inside
  the box (an even count when that fits more). Note: the reference name map
  lists `poly.julienne` starts as Number 0/0/0 and Size 1 m; which is right?
- **AX-Q2.** Dragging moves Offset along the world axes nearest the screen's
  right and up directions; one metre per metre under the pointer at the
  box's centre.
- Works on the selected polygons in Polygon mode, all polygons in other
  modes; selected polygons' pieces stay selected.
## Subdivide group and Center, edit wave 1 (2026-11-08)

Spec: `docs/specs/edit-wave1/SUBDIVIDE.md` (its own SUB-Q1..Q7 stay open).
Operandi's provisional readings, built and marked as such:

- **SUBT-Q01 Script words.** The name map gives `poly.subdivide "ccsds"`,
  `"flat"`, `"smooth"` for the three buttons but no word for the dialog-only
  SDS Subdivide method; Operandi uses `sds`. The map types `type` and `bound`
  as integers without their values; Operandi takes words (`ccsds`, `flat`,
  `smooth`, `sds`; `smooth`, `crease`, `creaseEdges`) or the dialog's order
  (0 Faceted .. 3 Catmull-Clark; 0 Smooth All .. 2 Crease Edges). Which
  integers does the reference use?
- **SUBT-Q02 Edge weights after an explicit Catmull-Clark step.** Start: a
  cube with one edge at weight 1. SDS Subdivide. Built: the edge point is
  the midpoint (sharpness 10 x weight, as the subdivision spec), and each
  half keeps weight 0.9 (one sharpness step used, so a later display level
  matches). UnSubdivide adds the 0.1 back. Faceted and Smooth keep the
  weight on both halves. Unmeasured (SUB-Q2).
- **SUBT-Q03 Item mode.** Subdivide in Item mode works on every polygon of
  every selected mesh item, one undo step (closed by corrected SUB-03).
  UnSubdivide in Item mode still works on the active mesh only
  (provisional).
- **SUBT-Q04 UnSubdivide where the step can't be undone exactly.** Points
  with three edges and no solved neighbour (a cube's corners) use the
  parallelogram estimate 3P' - 2F, exact for a Catmull-Clark cube. A
  Faceted cube's corners then go to +/-7/6 (1.1667); the oracle
  `cube_flat_unsub` says exactly +/-1.15. No simple local rule found that
  gives both 0.5 (Catmull-Clark cube) and 1.15 (Faceted cube); a linear one
  would need weights 2.85 P' + 0.2 E - 2.05 F. More cases (a Faceted grid,
  a once-subdivided pyramid) would pin it down.
- **SUBT-Q05 Smooth's push (SUB-Q1).** Each new point lies on a curve
  between two points whose end tangents follow the points' smoothed
  normals (Hermite, tangents as long as the chord); a centre is the average
  of the curved diagonals. This gives exactly the spec's two cube values;
  nothing else is measured.
- **SUBT-Q06 Linear UVs off (SUB-Q6).** UVs get the same Catmull-Clark step
  with UV seams as borders; on a partial selection the outline UVs stay put
  so the neighbours' UVs still meet.
- **SUBT-Q07 Fractal (SUB-Q5).** Only the new points move, each by a random
  vector up to Fractal% of its polygon's mean edge length.
- **SUBT-Q08 UnSubdivide details (updated by corrected SUB-40/40a).**
  Unselected neighbours keep their topology: outline points they took in
  a partial Subdivide stay in them. Corner hints only rank the possible
  groupings (fewest groups breaking a hint wins); they never refuse one.
  Afterwards the merged polygons are selected when polygons were selected;
  with nothing selected nothing is (the oracle's `selected_polygons: []`).
- **SUBT-Q09 Freeze... dialog. Closed** by corrected SUB-50: ten fields
  with their starting values, only Tessellation acts (off keeps the cage),
  the rest disabled with "Not available yet". `poly.freeze` with no
  arguments opens it; the plain `freeze` phrase still runs Freeze.
- **SUBT-Q10 SDS Subdivide's angle on non-planar polygons (SUB-Q4).**
  Built from the oracle `bump3_all_sds_method_angle20`: each polygon's
  normal for the angle test is taken at its first corner, (v1 - v0) x
  (v_last - v0), not averaged. This explains all of that case's creases,
  including the ones that aren't mirror-symmetric. One case only; it
  depends on the polygons' corner order.
- **SUBT-Q11 Shift+D. Closed** by corrected SUB-03 (2026-11-08): Shift+D
  opens the Subdivide options in Vertex/Edge/Polygon mode and is Instance in
  Item mode (a status message until instances exist). Jason's report of
  linked duplicates was Item mode.
- **SUBT-Q12 Keymap, later.** Ctrl+D duplicate with children, Ctrl+Alt+D
  duplicate the item alone, Shift+Alt+D instance the item alone. Not built.
- **SUBT-Q13 Instance (SUB-03a), a future job.** Shares the source mesh,
  own transforms starting at the source's position/rotation/scale/parent,
  shading from the source, italic in the Item List with its own icon, only
  the new instances selected afterwards, one undo step. Needs an instance
  item type (file format, render, MCP) before it can be built.
- **SUBT-Q14 Dialog look.** The Subdivide, UnSubdivide and Freeze
  dialogs use an interim layout (labels right-aligned in a left column,
  fields beside them, OK/Cancel column on the right with Return/Esc) from
  one shared helper, `dialog_form.rs`. They will be restyled there when
  the conventions bible's dialog rule lands. *Closed 2026-11-08 (branch
  dialog-component): they use the shared form (UIC-80..99) and
  `dialog_form.rs` is deleted. Iteration is a whole-number field with
  arrows, kept in 0..16.*
- **SUBT-Q15 Partial UnSubdivide keeps outline points (SUB-40b, Jason).**
  Merged polygons on the outline of a partial UnSubdivide include the old
  edge midpoints their unselected neighbours still use (n-gons of 5 to 8
  sides), moved to the middle of their restored corners; neighbours keep
  their topology. Built and tested on an open grid and on closed meshes
  (every edge shared by two polygons, all facing out). A further iteration
  over those n-gons is refused (SUB-44).

### Wave 2 spec v1, measured (path-slice-fixes, 2026-11-08)

Spec v1 replaces the readings above where it measured them; the 18 oracle
cases run as `crates/operandi-core/tests/path_slice_cases.rs`. Built now:
the ribbon also cuts polygons seen edge-on (a box's sides); every path
vertex inside a polygon is kept, a straight two-point curve's step points
too; Front Face Only = normal along +axis; Tunnel deletes the inside on
every crossed surface with no walls; Core and Tunnel drop the points left
unused; a circle's first corner lies on the shape's +Y (world -Z on Axis
Y); Bank turns about the axis. Deliberate differences, kept and adjusted
in the case test:

- **Existing points reused** where the path passes through them (PS-10).
- **No slits (PS-12)** and **no single-seam keyholes (PS-13)**: a polygon
  can't visit a point twice, so a slit is left out (the crossing point on
  the polygon's edge stays) and a keyhole is two pieces.
- **No T-junctions:** a cut point on an edge also goes into the
  neighbouring polygon, even one not cut (the reference leaves the side
  quads of a Front Face Only cube untouched).
- **Bent curves:** Operandi's Catmull-Rom shape (as PS-21 asks); the
  reference's step points through three points sit a little apart, so the
  test checks counts and the middle point there.
- **W2-PS-Q14 (answered, spec v1.1).** The shape has its own right-handed
  frame with local Z as the cutter axis; Half Size X runs along local X.
  Axis Y: (local X, local Y, axis) = (world Z, world X, world Y). Built;
  other axes follow the same cycle (provisional for tilted faces).
- **W2-PS-Q15 (answered).** Heading about local X, then Pitch about local Y,
  then Bank about the axis, right-handed; Heading and Pitch signs still
  provisional. 21 cases pass.

### Polygon Axis Slice spec v1, measured (axis-slice-measured, 2026-11-08)

Spec `docs/specs/edit-wave3/WAVE3-AXIS-SLICE.md` v1; its 17 oracle cases
run as `crates/operandi-core/tests/axis_slice_cases.rs` and all pass.
Replaces the AX-Q1 readings above: Numbers start at 0 and Size at 1 m;
Number mode spaces N cuts L/(N+1) apart with Offset wrapped into one
spacing; Size mode cuts at min + Offset + k·Size strictly inside the box.
Whole-number fields now step by 1 on a click of their arrows (right half
up, left half down, Shift 10) and by whole steps when dragged (UIC-53);
other fields don't step on a click yet, since their step size is open
(UIC-53 [Q]). Mouse-wheel stepping isn't built: no Operandi field has it
yet. AX-Q2 (drag mapping) stays as built: the world axes nearest the
screen's right and up.

- **W3-AX-Q1 (Offset wrap, from Jason's test).** AX-13's wrap into 0..s
  kept a single cut from moving left of the middle, and a -s/2..s/2 wrap
  made cuts flip back a third of the way out. In the reference, Jason
  says the cuts feel like they wrap round to the other side and keep
  their spacing. Built: each cut wraps round the box's length (a cut slid
  off one end comes in at the other). This matches 16 of the 17 cases;
  quad_numX1_off25 (one cut, Offset 2.5 on a 4 m quad) records the cut at
  0.5, where this rule gives -1.5. Which is right?

### Data-driven Tool Properties (tool-props-data, 2026-11-08)

Tools now declare their Tool Properties as field lists (`tool_form.rs`);
one renderer and one confirm route serve them all. Pilot: Cube, Sphere,
Cylinder, Cone, Ellipsoid, Torus, Capsule, Arc, Tube, Text, Spike and
Polygon Reduction. Small changes where the old per-tool code differed:

- **FORM-Q14 One confirm rule for every tool** (provisional). After a
  value is taken, every field shows the accepted value in its unit (typed
  `1/2` shows `0.5 m`), and an option change takes any pending typed text
  with it; a refused field blocks the option and is marked. Cone, Ellipsoid
  and the shapes already did this; Cube, Sphere and Cylinder now do too
  (before, a typed `1/2` stayed as typed and an option ignored pending
  text). Fields nobody is editing follow values the tool changes itself
  (handles, Reset, Reduction's initialization).
- **FORM-Q15 Sphere Subdivision Level** (provisional). Now an ordinary
  whole-number field with its mini-slider arrows (UIC-53), 0 to 6; it was a
  drag value without arrows.
- **FORM-Q16 Text's Font row** (provisional). Now a dropdown row on the
  shared label column; it was a label and dropdown laid out by hand.
- **FORM-Q17 Percent fields read "%"** (provisional). Reduction's Boundary
  and Material Border Weight accept a typed `%` now; before, typing one was
  refused as an incompatible unit.

### Group Locators (group-locators, 2026-11-08)

Built from a generic design brief, not a measured spec; everything here is
provisional.

- **GL-Q1 Where Parent to Group Locator puts the locator.** Built: with
  one shared parent the locator sits under it with an identity transform
  (children keep their local transforms exactly); otherwise it is top
  level at the world origin and children are parented in place (a child
  that would need shear refuses the whole command). Should it sit at the
  selection's centre (bounding box or pivot average) instead?
- **GL-Q2 Script name for the item type.** Built: Add Item key
  `locator.group`; MCP `item_get` reports kind `groupLocator`. What are the
  reference names for the item type and for Parent to Group Locator (no
  script command is exposed yet)?
- **GL-Q3 Shortcut for Parent to Group Locator.** Built: none; it is in
  Item Operations (Cmd+click in Items mode) and the Items row right-click
  menu. Is there a default key?
- **GL-Q4 Deleting a parent** (decided by Jason, 2026-11-08). Deleting
  any item with child items asks "The item <name> has child items. Do you
  want to delete the child items too?" ("Some of the selected items have
  child items. …" for several). Yes deletes the item and everything below
  it; No deletes the item and moves its direct children to the scene root
  in place, each keeping its own hierarchy; a child whose place would need
  shear keeps its local transform and the status line says so. Each answer
  is one undo step; items without children delete as before. Was: Delete
  refused with "Dangling parent reference".
- **GL-Q6 Delete without the dialog** (provisional). Scripts and MCP: plain
  `item.delete` / `DeleteItem` moves children to the root in place; MCP
  `item.delete children:delete|root` deletes every selected item with that
  choice (`DeleteItems`).
- **GL-Q7 The question's buttons** (decided by Jason, 2026-11-08). Yes /
  No / Cancel, stacked in the dialog's right-hand button column, each with
  its key at its right: Yes = Return, No = Escape, Cancel = Shift+Escape.
  **UI bible deviation, Jason's OK:** the shared dialog gains a question
  style (`form::Dialog::question`, `Outcome::No`). It departs from UIC-83/84
  (buttons are Yes / No / Cancel, not OK / Cancel / extras), UIC-85
  (Escape answers No; Shift+Escape cancels) and UIC-94 (the button column
  is 112 wide instead of 70, to fit "Cancel  Shift+Esc"). UIC-82 (a click
  outside cancels) and UIC-96 (Cancel changes nothing) still hold. Every
  other dialog keeps OK / Cancel.
- **GL-Q8 "In the future" for the delete question** (follow-up, not
  built). A dropdown under the message (Ask Me / Always Yes / Always No)
  saved in user preferences. Left for later.
- **GL-Q5 LXO export.** LXO export can't write empty items yet, so Group
  Locators are left out and their children move up to the locator's own
  parent in place (with a note). Should they be written as LXO locators?
- Row icon (Jason, 2026-11-08): a little blue folder (painted
  placeholder, theme colour `items.group_locator_icon`); the Texture
  Group's folder stays in the row's text colour.
- Viewport picking: like cameras and lights (CL-Q09), clicking a Group
  Locator's shape in a view doesn't select it; use the Items list.

## Multiple active meshes, stage 1 (2026-11-08, branch multi-active-meshes)

Jason's direction (2026-11-08): every selected mesh item is active in
component modes. Provisional readings:

- **MAM-Q01 A selection command that fails on one mesh.** Select All,
  Grow, Loop, Convert and the other Select menu commands run on each active
  mesh in turn. Where one fails (nothing selected to grow, a polyline) that
  mesh is left as it was and the others still change; the command only
  reports an error when it fails on every mesh.
- **MAM-Q02 Plain click on another active mesh.** Clears the current mode's
  components on every active mesh, then selects; the primary item stays
  (it doesn't become the clicked mesh).
- **MAM-Q03 Tools with an empty primary.** Tools edit only the primary.
  With nothing of the current mode on the primary but components on another
  active mesh, a tool refuses (instead of treating "nothing selected" as
  the whole primary). Shape tools still add to the primary.
- **MAM-Q04 Item mode look.** Every selected mesh draws in the active
  display style, in Item mode too; only the primary shows the item
  selection fill.
- **MAM-Q05 Hidden selected meshes** keep their component selections but
  don't draw, pick or take part in selection commands until shown again.
- **MAM-Q06 Still primary-only:** Select Symmetrical and Symmetry mirror
  highlights, tag selection modes (Material/Part/Set regions), selection
  sets, Hide/Lock of selected components, Copy/Paste and every mesh-editing
  tool.
- **MAM-Q07 Cmd-click toggles items** (Jason, 2026-11-08, revising FLAGS
  F1). In every tree list (Items list and the others) Cmd-click and physical
  Ctrl-click both add or remove one row; Opt-click is a plain click. In the
  viewport in Item mode, Shift-click adds a mesh and Ctrl-click toggles it.
  Cmd-click in the viewport is still the modeling menu (and Shift+Cmd is
  Polygon Selection Fill), so it does not toggle there; open question for
  Jason.
### Every tool on field lists (tool-props-data-2, 2026-11-08)

The rest of the tools moved onto declared field lists: the live tools
(`live_fields.rs`: Transform, Move, Rotate, Scale with deform falloff rows,
Tweak, Push, Smooth, Bevel, Extrude, Inset, Shift, Poly Haul, Chamfer Edit,
Sketch Extrude, the Edge and Vertex tools, the seven Slice tools, the item
channel and Negative Scale rows), Bridge, Pen, Smooth Shift / Thicken,
Add Loop / Add Point, the Create utilities and the draw tools. The old
per-tool drawing and confirm code is gone. Changes where it differed:

- **FORM-Q18 Name fields confirm like numbers** (provisional). Material
  Name (Bevel, Edge Bevel/Chamfer, Vertex/Edge Extrude, Vertex Bevel), the
  Slice cutter's Material and Edge Selection Set now take effect on Enter
  or on leaving the field; before, each keystroke re-ran the preview.
- **FORM-Q19 Extrude Make UVs is a segments row** (provisional), like
  Bevel's. Without a selected UV map the whole row greys, label included;
  before, only the buttons greyed.
- **FORM-Q20 Push and Smooth numbers work from the panel** (fix). Push
  Distance and Smooth Strength, Iterations and Sharp Angle had no entry in
  the old confirm route, so typing in them changed nothing. Iterations
  now refuses anything but a whole number from 0 to 1000.
- **FORM-Q21 Falloff and item channel rows are number fields**
  (provisional). A deform preset's Start/End (Center/Size) and Custom
  In/Out, and a camera or light's channels in Move/Rotate/Transform, are
  ordinary fields with arrows (typed, confirmed, scrubbed) instead of drag
  values. An invalid falloff is refused and marked instead of ignored;
  channel values outside their range clamp, as before.
- **FORM-Q22 One row layout for the draw tools, Sketch, Draw Patch,
  Create utilities and Add Loop / Add Point** (provisional). They used
  their own 90 px label rows; they now sit on the shared label column with
  arrows on every number (UIC-113, UIC-53).
- **FORM-Q23 Refusals name the field by its label** (provisional). Typing
  nonsense in, say, Loop Slice Count reads "Count: …"; the old route
  called most such fields "Transform X".
- **FORM-Q24 Tool State rows** (provisional). Bevel's and Smooth Shift's
  state pickers are labelled dropdown rows ("Profile", "Tool State") and
  the state name is a "Name" row. Recalling a Bevel state now clears the
  typed name with the other fields (it used to stay).
- **FORM-Q25 Polygon Slice's first typed cut** (provisional). Typing Start
  or End before any cut makes one on the work plane; the old route stood
  it up on world Y.
- **FORM-Q26 Edge End is a segments row** with one tooltip covering
  Profile and Triangle Fan (each button had its own); Vertex Slide's UV
  Map is a fixed dropdown that says why.
- **FORM-Q27 A failed save leaves the field text as it was**; before, it
  restored the confirmed values but not the text shown.

### Viewport Properties phase 1 (viewport-props, 2026-11-08)

- **VP-Q01 Wireframe overlay keys** (provisional). The brief puts the
  active meshes' overlay on `/` None, `Shift+/` Uniform, `Ctrl+/` Colored,
  but `/` and `Shift+/` are already Turn and Turn Upright (NAV-10..18).
  Built: three input-map actions (Wireframe Overlay None / Uniform /
  Colored) acting on the view under the pointer, unbound. Which keys win?
- **VP-Q02 Esc on a pinned panel** (provisional). Esc closes an unpinned
  panel; a pinned one closes only from its circle. Should Esc close a
  pinned panel too?
- **VP-Q03 Header row and the other tabs** (provisional). Phase 1 asks for
  only Active and Inactive Meshes and no Presets / One Tab / All Tabs row.
  The panel already had working Drawing and Control, Visibility and
  Advanced Options tabs and that header row (tab presets, Save Default for
  3D View), so they were kept; the old "O over the panel shows all tabs
  in one column" mode was dropped (phase 4). Keep or hide them until
  phase 4?
- **VP-Q04 Inactive Show Vertices** (provisional). Operandi draws points
  only for the active meshes, so Inactive Meshes > Display (Show Vertices,
  default Never) is left out. Same for Show Geometry Boundaries, Edge
  Thickness, Show Cages, Show Guides (no such drawing yet) and Show Locked
  Mesh Elements (today one app-wide View-menu switch; per view it would
  rebuild GPU geometry per view).
- **VP-Q05 Last selected mesh, how far "as if still selected" goes**
  (provisional, VP-04 v2.1). With no mesh selected, the last selected one
  draws active and limits component picks (Pick Other Meshes off). In
  Vertex/Edge/Polygon mode a mesh command (Select All, Flip, Thicken, a
  tool ...) selects it again and edits it; a component click on it
  selects it with that component. Not adopted: Item mode (nothing
  selected means nothing to move), item and scene commands (Add Item,
  parent, hide, delete ...), and mode switches. Unhiding a hidden
  remembered mesh brings back its active look; a selected camera or light
  leaves it active. Right calls?
- **VP-Q06 Visibility tab, per-view GPU colours** (P2, provisional).
  Use Shader Tree and Use Texture are left out for now: the views share
  one set of GPU meshes with the materials baked in, so a per-view switch
  would rebuild every mesh each frame while two views differ. They need
  per-view material colours on the graphics card (a follow-up step).
  Show Selected Filling (and Show Selections off) re-tints only the
  selected polygons, but does so every frame while two views differ.
  Displacements stays Later (no viewport displacement). Show Scene
  Lighting defaults to off as the spec says, so the Advanced style is lit
  by the view's own light until it is turned on; that changes how
  Advanced looked before. Show Texture Locators off keeps today's
  behaviour (selected or Shader-Tree-picked locators still draw). The
  work plane counts as Active once it is fixed or centred off the origin.
  Selection normals are half the polygon's size long.
- **VP-Q07 Drawing and Control tab** (P3, provisional). Built: Camera
  Item ((Current Selection) or a scene camera; picking one switches the
  view to Camera view through it), and a Mouse Control section with
  Trackball Rotation (moved from Visibility) and Orbit Selected (orbit
  about the selection's bounding-box centre; nothing selected orbits as
  usual). Left out: Overlay Drawing (cameras, lights and locators already
  draw over the meshes, so there's no "off" yet), Selected Item Drawing
  (the selected item's tint and outline come as one; splitting them is GPU
  work, like VP-Q06), and Light Item (there is no Light view).
- **VP-Q08 Advanced Options and point size** (P5, provisional). Ambient
  Intensity defaults to 62 % and Ambient Color to white, which is the
  ambient every style already used, so nothing changes until edited;
  range 0–200 %. Independent Point Size is on the Active Meshes tab only
  (inactive meshes draw no points, VP-Q04); Point Size starts at 3 px
  (Preferences > Selection's default), range 1–20, and picking keeps
  Preferences' hit size.
- **VP-Q09 Transparency Overrides** (P5b, provisional). Built per mesh
  tab: Transparency Mode (Normal, Ghost, X-Ray), Front and Back Polygon
  Color and Opacity, Overlay (Active tab only) and Flat Shaded. Ghost draws
  polygons facing you at the front colour/opacity (default light blue-grey,
  25 %) and those facing away at the back colour/opacity (grey, 60 %);
  X-Ray drops the front ones. See-through polygons write no depth, so
  everything behind them and their own hidden edges show. Left out: front
  and back wireframe colours, Back Wireframe Opacity and Link Wire Opacity
  (Operandi's edges don't know which way they face). Defaults wanted?

### Sculpt foundation (sculpt-foundation, 2026-11-08)

The spec's own SQ-02/04/05/06/07/08 picks are built as written (selection
ignored; screen-space ring; front-facing points only; relative right-drag;
one stroke = one step "Undo <Brush> Stroke"; no default keys). Builder
picks the spec leaves open, all provisional:

- **SCQ-01 Move and Offset Mode** (SC-123, SC-102). *Decided by Jason
  2026-11-08: as built.* Spec greys only Normal Mode, Density Mode and
  Interpolation Step on Move. Offset Mode / Offset Distance have nothing to
  act on in a grab, so they are greyed too (Offset Amount scales how far
  points follow); Continuous Stroke is greyed with Interpolation Step.
- **SCQ-02 Dab dose** (SC-24, SC-41). "Distance moved = size × Offset
  Amount" is read as the full move of one sweep across a point: each dab
  moves `full × weight × step`, so a stroke's strength doesn't depend on
  Interpolation Step and a single click is a small bump. Max Stroke applies
  the full move in one dab. Is a click meant to give the full offset?
- **SCQ-03 Attenuate cap** (SC-44). A point stops at `full offset × its
  weight`, so the bump keeps the tip's profile instead of flattening into
  a plateau at the full offset.
- **SCQ-04 Shape curves** (SC-31). Linear 1−t, Smooth 1−(3t²−2t³), Bulge
  √(1−t²), Fat 1−t⁴, Sharp (1−t)², Very Sharp (1−t)⁴; Bias is Schlick's
  bias (50 % = none). Only the names were given.
- **SCQ-05 Uniform smoothing amount** (SC-62): after each dab, a smoothing
  pass at 20 % of Smooth Amount. Low Pressure is listed greyed (no pen
  pressure is read yet, SC-45: pressure is always 1).
- **SCQ-06 Volume-keeping smooth** (SC-63): Taubin (smooth, then the same
  amount ×1.04 back out).
- **SCQ-07 Continuous Stroke** (SC-42): fills large steps by laying dabs
  at no more than 10 % of the radius (the dose follows the spacing).
- **SCQ-08 Undo label with Shift** (SC-92): a Shift-smoothing stroke is
  "Smooth Stroke", not the active brush's name.
- **SCQ-09 Auto Scale** (SC-21): the world size is fixed from the first
  stroke after turning it on (Size × metres per pixel then).
- **SCQ-10 Smooth on borders**: border points (not locked) average with
  their border neighbours only, so Smooth slides them along the edge
  instead of shrinking the opening. Mirrored dabs near the symmetry plane
  overlap their originals and add up (Attenuate caps it).
- **SCQ-11 Move and back faces** (#38, Jason 2026-11-26). Starting state: a
  subdivided cube, front ortho view, Move with the tip covering the mesh.
  Every brush skips points facing away, so Move drags the front and leaves
  the back. Built: a Move setting **Move Back Faces** (off by default) that
  also grabs points facing away inside the tip. Open: does the reference
  Move grab back faces, and should the default be on? Needs a hands-on check.
- **PH-Q37 Handles on Cone, Ellipsoid, Torus, Capsule, Text** (#37; Q-CN-06,
  NPR-Q01 leave them open). Built (provisional): once drawn, move arrows at
  the centre and a radius square at each end of each axis; Ellipsoid and
  Torus resize from the centre, Cone and Capsule keep the far side; the
  Torus squares set Radius X/Y/Z as the panel does (along the axis Height,
  across it Cross Section with Hole Size kept); Text gets arrows only. Open:
  Handle Mode and Show Corner, and whether the Torus should offer ring and
  tube radius handles instead.

## Multiple active meshes, stage 2: transforms (2026-11-08, branch multi-active-transforms)

- **MAM-Q08 Several items rotate and scale about one shared centre**
  (provisional, Jason's card pending): the mean of the selected items'
  world positions, where the handle sits. `Scene::item_pivots` is the one
  place to switch to each item's own origin. One selected item still
  turns about itself.
- **MAM-Q09 Symmetry with several meshes.** Each mesh mirrors in its own
  space as before; the shared handle doesn't take Symmetry's side-of-the-
  first-pick adjustment that one mesh gets.
- **MAM-Q10 Several items move in world space.** With one item, Move adds
  to its own Position channels (a child moves in its parent's space, as
  before); with several, each moves by the drag in world space so they move
  together. Rotation and Scale channels change as one item's do.
- **MAM-Q11 Nothing selected on one mesh.** In a component mode a mesh
  with nothing of the mode selected stays put while another active mesh
  has a selection; with nothing selected on any, every active mesh moves
  whole.

## Multiple active meshes, stage 3: tools (2026-11-08, branch multi-active-tools)

Mesh-editing commands and tools run on every active mesh, each with its
own components, as one undo step (`multi_tools.rs`). Provisional readings:

- **MAM-Q12 All or nothing.** A mesh edit that fails on one mesh (Merge
  with one polygon there, Flip of a polyline) refuses on all of them, and
  the message names the mesh. Selection commands (Select Symmetrical, Use
  Selection Set, Select Polygon Set) follow MAM-Q01 instead: a mesh where
  they fail keeps its selection. A command that is a quiet no-op on one
  mesh but refuses there as "nothing to do" (Flatten Ngons on planar
  n-gons, say) therefore refuses for all; tell us if that's wrong.
- **MAM-Q13 Starting values from one mesh.** Values a tool reads from the
  mesh when it starts (Edge Weight's current weight, Edge/Vertex Extrude's
  starting Width, Vertex Slide's rail direction) come from the first mesh
  with a selection (the primary when it has one). Vertex Slide then slides
  every mesh along that one world direction.
- **MAM-Q14 Item mode.** One-shot commands in Item mode (Subdivide,
  Triple, Mesh Cleanup...) run on every selected shown mesh, whole.
- **MAM-Q15 Dialog targets.** Set Material, Set Part, Set Position and
  Mesh Cleanup show the primary's values when they open; with several
  meshes the command applies to each mesh's own selection.
- **MAM-Q16 Tweak** grabs a point, edge or polygon on any active mesh and
  moves that mesh only (one element per drag, as before).
- **MAM-Q17 Still primary-only** (they pick or join specific elements, or
  move geometry between items): Loop Slice and Edge Slice (their rings and
  edges are picked on one mesh), Poly Haul, Add Edge Point/Loop, Weld
  (point ids), Sync Vertex Order, Cut/Copy/Paste, Merge/Unmerge Items,
  Bridge, Spike, Polygon Reduction, Pen and the shape tools (they add to
  the primary), Scale Offset Detach, Center Selection, Symmetry mirror
  highlights, tag selection modes (Material/Part/Set
  region clicks), and the Duplicate tab's live tools (Mirror, Radial
  Sweep, Clone, Array, Radial Array: their Item-mode copies are new items
  and their placement handle is the primary's). Detriangulate runs on
  every active mesh like the other one-shot commands. With nothing selected on the primary but components on
  another mesh, these still refuse (MAM-Q03).

## MCP bridge gaps from the hero renders (2026-11-09, branch mcp-bridge-gaps)

- **MCPH-Q01 Second inset Bevel on a grouped round cap** (M17-Q04): with
  Group Polygons on, a second `poly.bevel` with an inset on the pole cap of
  a round shape (the ring of polygons around a pole, already inset once) is
  refused with "Inset crosses cap edges"; Extrude works there. Is the
  reference's grouped inset of such a cap expected to succeed (the group's
  outline shrinks as one polygon), or does it also stop? Unchanged until
  answered: the refusal is the M17-Q04 decision.
- **MCPH-Q02 Material channel names on the bridge** (provisional, Operandi
  choice): `material_set` and `shader_layer_set` name channels after the
  Shading panel's rows in snake_case (`diffuse_color`, `clearcoat_amount`),
  in bridge units (fractions 0–1, radians, metres). The reference name map
  has no entries for these yet; when it does, the map's names should be
  added as aliases.
- **MCPH-Q03 Subdivision on the bridge** (provisional): no map name for
  Tab / Shift+Tab or the per-mesh levels, so they are one Operandi tool,
  `subdivision_set`.
## Duplicate tools v1 (2026-11-08, branch duplicate-tools)

- **DUP-Q01 Internal walls after a weld** (provisional). Two polygons left
  on the same points by Merge Vertices are both removed for every tool, not
  only Mirror (MR-43): an Operandi edge holds at most two polygons, so
  CL-31 / AR-32's "touching faces remain back to back" can't be stored.
  Touching clones therefore join into one hollow piece.
- **DUP-Q02 Welds inside one copy** (provisional). New points of the same
  copy never weld to each other, matching DS-23's rule for the source's own
  close points; DS-22 read literally would weld them.
- **DUP-Q03 Lathe's first ring** (provisional). The swept surface starts on
  the profile's own points only where that can't put three polygons on an
  edge: lines and curves (used up), and border edges on a part turn. Face
  outlines and full-turn edges sweep from new points beside them. Faces in
  Polygon mode sweep the outline of their region (shared edges once);
  branching profiles are refused with a message.
- **DUP-Q04 Clone Scale and Rotate** (spec v1.2, DQ-13 pending a hand
  check). Built as compounding per-step factors: copy *i* is sized
  Scale^*i* and turned *i* × Rotate about its own centre, nothing random.
- **DUP-Q05 Click away (DS-34).** Lathe and Radial Array: a click away from
  the handle moves the axis of the current sweep there; no second sweep
  (Jason, 2026-11-08, after the result was swept again). Differs from
  spec v1.3's DS-34, which still says a new placement: needs the spec
  updated to Jason's call. Clone: a new drag
  starts a new row (the previous one stays as its own undo step). Mirror
  ignores later clicks; Array's left-drag is spacing (AR-21).
- **DUP-Q06 Item mode details** (provisional). Copies go right after their
  source in the Items list with the source's parent; Replace Source moves
  the source item itself into slot 0; random scale/turn and non-uniform
  scale on a rotated item are decomposed approximately (no shear).
- **DUP-Q07 Handle tooltips** (UIC-05): built to DS-38 (shown at once,
  beside the pointer, no hover delay yet).
- **DUP-Q08 Detriangulate settings** are remembered across restarts with
  the Subdivide settings file (UIC-95). It runs in any mode (whole mesh
  outside Polygon mode, DT-11) and in Polygon mode selects the quads and
  leftover triangles (DT-26), even when nothing was selected.


## Shader layer menu (shader layer menu v0.1, 2026-11-09)

- **SLM-Q01 Editor colours** (merge, 2026-11-09). The menu's Editor Color
  uses note 10's 17 named colours (editor_color.rs) rather than the spec's
  12; the colour stays the row's own and isn't linked by instances.
- **SLM-Q02 Paste placement** (spec [?]). Directly above the selected row
  in its parent; with nothing selected, below the Base Shader like Add
  Layer. Pasted names follow the Duplicate rule ("(n)").
- **SLM-Q03 Copy/Paste of an instance** (provisional). Like Duplicate: a
  plain layer with the source's values. A copy of a Library item pastes as
  an ordinary group; Duplicate of a Library item stays in the Library.
- **SLM-Q04 Paste Values** (spec [?]). Copies the Layer block (Enable,
  Opacity, Blend, Invert) and every type setting; name, position, Texture
  Locator and editor colour stay. Type match is by type name, so Noise →
  Noise but not Noise → Checker.
- **SLM-Q05 Change Type naming** (spec [?] allows it). "Layer" becomes
  "Layer (2)" as measured, but a layer still named after its type takes
  the new type's name ("Constant" → "Gradient"). Instances can't change
  type; changing a source changes its instances' type too.
- **SLM-Q06 Instance links** (provisional). Every setting is linked
  except the eye (Enable), which stays the row's own. Colour components
  are separate settings; lists of keys (gradients) are one setting. An
  instance shares its source's Texture Locator unless it overrides that
  setting.
- **SLM-Q07 What can be instanced** (provisional). Materials, Shaders and
  texture layers. Groups, Folders, Render Outputs and references can't;
  a texture's masks aren't instanced with it. Create Instance on an
  instance links to its source.
- **SLM-Q08 Revert to Source** (spec: Operandi's choice). Overridden
  settings show their label in italics; right-click the label for Revert
  to Source; "Revert All to Source" under the instance's header drops all
  overrides. Deleting an instance leaves the source (assumed).
- **SLM-Q09 Instance file storage** (provisional). Files keep the link and
  the list of overridden settings, and also the instance's current values;
  on load every non-overridden value is taken from the source again, so
  the result is what "only the overrides" would give.
- **SLM-Q10 Library masks** (spec [?]). A group keeps its mask when it
  goes into the Library; the Library row's tooltip names it.
- **SLM-Q11 Deleting a Library item with references** (spec [?]). Yes /
  No / Cancel: Yes De-References them first (they stay as real copies), No
  deletes them too, Cancel changes nothing. A plain Delete from a script
  deletes the references too.
- **SLM-Q12 Lock Preset** (provisional). Locked items refuse every edit
  to their layers (settings, names, order, adding or deleting inside) with
  "<name> is locked: Unlock Preset to edit it"; their Properties are
  greyed. Deleting the whole item and changing row colours still work.
- **SLM-Q13 Reference rows** (provisional). A reference has its own eye
  and name; turning its eye off turns the referenced layers off there.
  It can't sit inside the Library, and a group holding a reference can't
  go into the Library (De-Reference first). Loading a material preset onto
  the Library heading isn't built yet: load it into the tree, then drag
  the group onto the Library.
## Item editor colours (note 10, 2026-11-09), provisional

- **IC-Q1 LXO — answered** (research, 2026-11-09): an item tag `COLR`
  with the lower-case name; missing or "none" is none; unknown names
  ("teal", "gray") are none, also for `item.editorColor` and native
  files. Built: Operandi's LXO export writes COLR on coloured meshes and
  both LXO readers take it back; COLR on any other item type (locators,
  materials) is accepted, but those items aren't imported yet, so their
  colour is dropped with them.
- **IC-Q2 Select popover Group action** is still greyed (it was before
  this work); Select, Unselect and Hide act on the colour matches.
- **IC-Q3 "(none)" in the By Color dropdowns** filters or selects the items
  with no editor colour.
- **IC-Q4 Groups tab.** Member rows carry the mark and the Editor Color
  menu; group rows themselves have no colour (groups aren't items).
  Schematic node outlines come with the schematic.

## Channel light and Reset to Default (CH-Q1 answered, 2026-11-09)

- **CH-Q1 answered** (Jason, 2026-11-09): show the default/changed light
  now with right-click Reset to Default. Built as above.
- **CH-Q2 Coverage.** Only rows that declare a default show a light:
  Preferences, Item Display/Assembly, the camera rows that had one, and
  now Shader Tree layer channels. Light, camera and procedural texture
  rows need their defaults declared (moves with the channel registry,
  F1).
- **CH-Q3 What "default" means for a layer.** A layer's lights compare
  with the values Add Layer gives that type (a texture keeps its image or
  map); the Base Material compares with the full material defaults.

## Viewport image textures (2026-11-09, branch viewport-image-textures), provisional

- **VIT-Q01 Bicubic filtering.** Starting state: an Image Map with
  Filtering = Bicubic. The viewport samples it bilinear (with mipmaps);
  renders use Cycles' cubic. Unclear whether the realtime view should
  match cubic exactly.
- **VIT-Q02 Gradient Geometry inputs in the viewport.** Starting state: a
  Gradient on Incidence / Slope / Camera Distance / Back Facing / Screen
  X/Y/R (or a Vertex Map attribute, or Occlusion) blended with an image.
  The viewport reads them as the CPU fold does (0, 0, 1: no view, no mesh
  attributes, nothing around the point). Should the realtime view use the
  viewport camera and the mesh's vertex maps instead?
- **VIT-Q03 Several procedural patterns with an image.** Starting state:
  one surface whose colour reads an image and two different procedural
  patterns. One pattern per surface is evaluated per corner and passed to
  the GPU; with two or more the surface keeps the average-colour path.
  Is per-pixel procedural drawing wanted in the viewport?
- **VIT-Q04 Image paths in saved scenes.** Native scenes keep image paths
  as written (an absolute path, or one relative to the working folder);
  nothing resolves them against the scene file's folder. The fixture
  stores repository-relative paths, so QA uses a copy with absolute paths.
- **VIT-Q05 Command names for scene images.** Reload Image now runs
  through `image.reload <image name|file path|all>`, and `image.list`
  reports the scene's images (UIC-160..164). The reference name map has
  no entry for either, so the names are Operandi's own `<domain>.<verb>`
  (provisional). Reload changes no scene data, so it adds no undo step.

## Planar texture placement (planar-drag, 2026-11-09)

Built first from a coordinator brief, then cut back to the Planar behaviour
spec (2026-11-09) on Jason's word "just do the modo way": there is no
dedicated drag; a texture is placed with the ordinary Move / Rotate / Scale
/ Transform tools on its Texture Locator. Code: `operandi-core/src/
planar_texture.rs`, `operandi-app/src/uv/planar_texture_ui.rs`, the gizmo in
`camera_light_ui.rs`. Starting state for all: a mesh with a texture layer
whose Texture Locator is Planar, Item mode.

- **PD-Q01 Target.** The layer target is the locator of the *one* texture
  layer selected in the Shading panel (any projection); with several
  texture layers selected, none.
- **PD-Q02 Selecting the layer selects its locator (spec: the layer makes
  its locator the item tools' target, as if the locator were selected).**
  Selecting one texture layer in the Shading panel runs a plain
  `select.item` of its locator, the same as clicking it in the Items list,
  whether or not a tool is active: a live Move/Rotate/Scale/Transform
  moves to it, and one started afterwards (W) starts on it. Only on a
  *change* of that layer selection, so clicking the mesh afterwards still
  selects the mesh. Items mode only (spec: in vertex/edge/polygon modes
  the tools act on the mesh as usual; the plane still draws but isn't the
  target), so there the item selection is left alone. A hidden or locked
  locator isn't selected.
- **PD-Q03 Side grabs — removed** per Jason 2026-11-09 ("just do the modo
  way"): the spec has no dedicated drag.
- **PD-Q04 Corner grabs — removed** per Jason 2026-11-09 ("just do the
  modo way").
- **PD-Q05 Inside-square slide — removed** per Jason 2026-11-09 ("just do
  the modo way"); the Move tool moves the locator. No UIC-76 break remains,
  and the Ctrl (F1) slide test went with it.
- **PD-Q06, PD-Q09, PD-Q10 — moot** (they were about the grabs): one undo
  step per drag is now simply the item tools' own behaviour.
- **PD-Q07 World Coordinates off.** The render places such a locator in
  each surface item's own space. The plane now draws there, through the
  *host*: the first visible mesh (item order) with a polygon a layer using
  the locator applies to. With several surfaces at different places the
  texture shows on each, but the plane is drawn on the host only. No host:
  world space as before.
- **PD-Q08 Tool handle vs plane (World Coordinates off).** The spec says
  the tool handles align to the locator's rotation with World Coordinates
  off. Operandi's Move/Rotate/Scale handle sits at the locator item's own
  world position and follows the action-axis setting as for any item, so
  with the host away from the origin, or turned, the handle and the drawn
  plane are apart. Not built yet (the item tools don't know about render
  space).
- **PD-Q11 Tooltips — removed** with the grabs (per Jason 2026-11-09).
- **PD-Q12 Commands (UIC-160..165).** Names from the reference name map,
  in core's command registry (`command_registry.rs`, run by
  `command_exec.rs` for the command line, scripts and the MCP bridge
  alike): `transform.channel name:<channel> value:<v> [item:<id>]`
  (Transform Channel), e.g. `transform.channel name:scl.X value:2`; value
  `?` reads the channel (UIC-163), as `item.channel` does. While
  Move/Rotate/Scale/Transform holds that Texture Locator alone, the core
  turns it into a tweak of that tool (one undo step inside it, the tool's
  offset kept), the same step a Scale-handle drag makes; idle it is the
  ordinary Item Transform step. `item.transform` (registry-2's
  Operandi-only command, typed and bridge) takes the same route. Moving
  the tool to the layer's locator (PD-Q02) is the map's `select.item
  item:<id>` (Select Item); `item.select` stays as a deprecated alias.
  Provisional: (a) the channel names pos.X/Y/Z, rot.X/Y/Z, scl.X/Y/Z are
  the ones Operandi's LXO files already use, not a channel list from the
  map; rot is the item's stored Euler angle in radians (`90deg`
  accepted). (b) select.item's `mode` and `mask` are accepted and
  ignored; transform.channel's `mode`, `filter` and `replace` are refused
  ("not supported yet"), as item.channel's mode is; the map gives no
  values for them. (c) No macro recorder yet (UIC-161). (d) Registry-2
  refuses typed and bridge commands while a tool is active ("the person at
  the keyboard wins"). Typed at the keyboard, select.item,
  transform.channel and item.transform are let through while a
  Move/Rotate/Scale/Transform is live in Items mode (they join it); an open
  dialog or a mouse drag still refuses them. Scripts and the MCP bridge
  still wait for the tool to close, so a script can't join a live tool.
- **PD-Q13 Plane drawing (spec).** Whenever the locator or its layer is
  selected, tool or not: the outline at Size in the selected-item colour,
  1 px; a line from the centre along the projection axis, 25 % of the
  smaller in-plane side; an "up" tick on the middle of the +V edge pointing
  outward along +V. Provisional: the spec doesn't give the tick's length;
  it is 10 % of the smaller in-plane side. The locator's own three short
  axes still draw as before (TL-Q20). With Show Texture Locators on, the
  unselected ones keep the locator colour.
- **PD-Q14 Size clamp (spec: 0.001 m).** Every path stops each Size axis
  at 1 mm from zero, keeping its sign: the item tools' preview, a typed or
  scripted `transform.channel` / Item Transform, the Texture Locator
  panel (its Size fields now start at 0.001), Apply Scale and Auto Size.
- **Not built from the spec:** snapping for the item tools on a locator
  follows whatever they do for any item; nothing locator-specific was
  added.

## Environments (2026-11-09, branch environments), provisional

- **ENV-Q-OP1 Channel names.** The general value setter (`item.channel`)
  needs each environment setting's channel name. The specs give UI names
  only, so `environment.rs` uses provisional short names (`intensity`,
  `visCamera`, `zenColor`, `skyExp`, `fogType`, …; `ITEM_CHANNELS` and
  `MATERIAL_CHANNELS`). Which names does the reference use for the
  Environment item and the Environment Material?
- **ENV-Q-OP2 LXO environments.** LXO import skips lights, cameras and
  environments today (scene/shader state). Starting state: an LXO with an
  environment and envMaterial. Which of their channels should map onto
  Operandi's Environment (channel names and units)? Until then an imported
  scene gets the default environment.
- **ENV-Q-OP3 Environments in the Item List.** *Answered (Jason,
  2026-11-09; ENVIRONMENT-PLACEMENT §1):* not in the Item List under any
  filter; only in the Shader Tree's Environments section. An item type
  with no transform doesn't go in the Item List. Done on branch
  env-not-in-items (`Item::in_item_list`).
- **ENV-Q-OP4 Old files.** Native files before version 33 that have a
  camera or light get the default environment on load (they were made in
  the app); bare scenes without lights don't. A scene with no environment
  on purpose is written as version 33 so it stays without one.
- **ENV-Q-OP5 Physically-based Daylight in the CPU/viewport colour.** The
  viewport and the Rust reference draw it as the 4 Color Gradient of its
  colours until a sky model is chosen for them (Q-ENV1).
- **ENV-Q-OP6 Blend modes over an environment.** Starting state: a texture
  layer under an environment with Blend Mode Subtract, Color Dodge,
  Color Burn, Soft Light, Divide, Hard Light or a normal-map blend.
  Add, Multiply, Screen, Difference, Darken, Lighten and Overlay mix as
  Cycles' Mix Color does; the others blend as Normal for now. Should they
  match the surface blends exactly?
- **ENV-Q-OP7 Procedural and Gradient layers under an environment.** They
  can be added and saved, but renders draw only Constant and Image Map
  environment layers today. Gradient inputs that depend on a surface
  (incidence, slope…) have no meaning on the sky: which inputs apply?
- **ENV-Q-OP8 Image orientation.** Spherical about Y: u = 0.5 +
  longitude/360° with longitude from +Z toward +X, v = 0.5 +
  latitude/180°, as the locator spec. The locator's rotation turns the
  image; the environment item has no transform (ENVIRONMENT-PLACEMENT
  §2). Position and Size are ignored.
- **ENV-Q-OP9 Old environment transforms (ENVIRONMENT-PLACEMENT §2).**
  Scenes and presets saved before env-not-in-items could give an
  environment a transform, a parent or children. On load they are dropped,
  not folded into the layers [chosen]: the Environment Material's
  gradients have no placement of their own, and image layers' locators
  can be shared with other layers, so folding would turn those too. A
  child is unparented where it stands. Renders never applied anything
  else.
- **ENV-Q-OP10 Item > Create Environment placement (§3.5).** The spec puts
  it right after Create Backdrop, but Operandi's Item menu has no Create
  Backdrop (backdrops come from Add Item in the Items list). Create
  Environment stays the Item menu's first entry. Should Item get a Create
  Backdrop entry?
- **ENV-Q-OP11 Environment selection and the Item List (§3.3).** Clicking
  an Environment row makes the environment the item selection (so
  item.* commands, Delete and the Properties panel reach it); mesh items
  are deselected, as clicking an item elsewhere does. The Item List shows
  no environment rows, so nothing there is highlighted. Is "Selection
  doesn't touch the Item List" meant to keep the mesh selection instead?
  Environment Material and environment layer rows select one at a time.
- **ENV-Q-OP12 Environment row menus (§3.4).** Presets is greyed: presets of
  one environment aren't wired (File > Save Environment Preset… saves every
  light and environment). The Environments header's own right-click menu
  keeps one entry, Create Environment (item.create environment); the spec
  lists no header menu.

  latitude/180°, as the locator spec. The environment item's rotation and
  the locator's rotation both turn the image [chosen]; Position and Size
  are ignored.

## 3D mouse stage 1 (branch spacemouse-1, no spec)

- **SM-Q01 Direction of each motion.** The puck moves the scene ("object"
  feel): push right, the scene goes right; lift, it rises; pull toward
  you, it comes closer; twist and tilt turn it the same way. Confirmed by
  Jason on a SpaceNavigator through 3DxWare (2026-11-09) for every motion;
  the sideways tilt (roll) was backwards and is flipped. A camera ("fly")
  feel could come later as an option.
- **SM-Q02 Rates.** Full deflection orbits 2 rad/s, pans one view
  distance per second and zooms e^1.5 (about 4.5×) per second, times
  Speed; linear past a 5 % dead zone. No response curve yet.
- **SM-Q03 Roll.** Sideways tilt banks the view only with Roll with
  Sideways Tilt on (off by default, as a mouse orbit never rolls).
- **SM-Q04 Which windows react.** Only while Operandi is the focused app;
  the hovered main-window view, else the active view. Floating view
  windows and the Preview aren't targets yet.
- **SM-Q05 3DxWare value range.** Through the driver, full deflection is
  taken as ±1024 and rescaled to the raw-HID ±350, so both feel alike.
  Not yet measured on a live puck (`OPERANDI_SPACEMOUSE_LOG=<file>` logs
  the first states).
- **SM-Q06 How the driver's settings combine with ours.** 3DxWare applies
  its per-app speed, axis flips and disabled axes first; Operandi's Speed
  multiplies that and its Invert flips it again. The driver knows Operandi
  by its process name (the executable name), not the bundle ID: the
  client API takes a name.
- **SM-Q07 3DxWare's own button actions.** Operandi asks the driver for
  every button (button mask all), so 3DxWare should hand presses to
  Operandi instead of running its own assignment. To confirm on the live
  puck that a button with a 3DxWare action does only Operandi's shortcut.

## Slider min/max labels (ui.slider_min_max_labels, approved change to UIC-90)

- **MINMAX-Q01 Guard limits.** Some fields clamp to huge values only to stop
  runaway input (±1e6, ±1e9). Built: limits at ±1e6 or beyond count as open,
  so those fields get no labels. Provisional; is there a better cut-off?
- **MINMAX-Q02 Fields in grids.** Value boxes laid out in an egui grid (the
  colour picker's channel rows, the image viewer, the input map editor)
  draw their labels under the box, but the grid doesn't grow its row for
  them, so they may overlap the next grid row. Built as is; to check in the
  GUI pass and give those rows the extra height if it looks wrong.

## Tool activation (specifications/tool-activation/TOOL-ACTIVATION.md)

- **TA-Q01 Shift+click: does it change the selection?** Shift+click (or
  Shift+drag) with Polygon Bevel on keeps the current bevel and starts a
  fresh one, values reset, on the current polygon selection (Jason,
  2026-11-09). Built: the selection stays as it is (the caps). Provisional.
  Also provisional: the trigger is Shift+press anywhere off the handles (on
  a face or not), as the older Shift-click continue was.
- **TA-Q02 Polygon Bevel with nothing selected. Resolved (TOOL-ACTIVATION
  v1.2):** keep PB-O-01, it bevels every polygon, as the code does. With
  live panel edits that happens on the first edit. Provisional until Jason
  checks it in the GUI (BLQ-15).
- **TA-Q03 Reset, Recall and Profile before any edit.** Built: only a value
  edit (typed or dragged) makes Bevel live; Reset Tool Attributes, recalling
  a tool state and picking a profile before any edit only store, as before.
  Provisional.
- **TA-Q04 Undo after Drop. See TA-07 (TOOL-ACTIVATION v1.2, provisional).**
  Inside the tool each edit, drag and Shift+click undoes on its own (tool
  stays on). Once dropped, switched or cancelled, each bevel of the session
  is one undo step: every bevel closed by Shift+click / Shift+drag, then the
  open one (on Cancel the open one is thrown away).
- **TA-05 follow-up (not built).** Polygon Extrude's panel Apply button
  that extrudes with the panel values without a viewport click.

## Viewport shading (VS, 2026-11-09)

- **VS-Q01 Material colours in the shaded styles.** The spec gives values
  for the default grey only. Built: Solid, Shaded, Texture, Default and
  Advanced multiply the material colour (factors = target ÷ 204, so default
  grey hits the targets; Shaded therefore still shows material colours).
  Gooch, Cel and Reflection ignore the material and use their fixed colours.
  Provisional.
- **VS-Q02 Vertex Map's wire.** Not in VS-1's list of eight styles, so it
  keeps the light Wireframe colour. Provisional.
- **VS-Q03 Reflection brightness and blur. Resolved (spec v1.1):**
  Reflection is a camera-fixed matcap looked up by the view-space normal.
  Built with Blender's clay_studio matcap (bright top, warm middle, cool
  rim), diffuse + specular, 32×32, sRGB-encoded with no other exposure
  change; the world HDRI and its blur are gone. The matcap choice is by eye.
- **VS-Q04 Advanced highlight width.** "Small, tight": Blinn-Phong exponent
  100, strength 0.14 (peak 251 measured). Default peaks at 215 rather than
  ~211 (one shared 45° light, as allowed). Provisional.
- **VS-Q05 Advanced ambient default.** Ambient Intensity now defaults to 44 %
  (90 on default grey, target 89). Saved views that stored 62 keep 62.
  Provisional.
- **VS-Q06 Cel (and Gooch) in Top/Right views. Resolved (spec v1.1):**
  both are fixed to the camera, with Cel's eye the real direction to the
  eye per pixel, so Top and Right look like Front and the outline follows
  the silhouette. Cel's spot threshold is now N·H > 0.94.
- **VS-Q07 Selection over the display styles (Jason, 2026-11-10).** Built:
  the style shades first, then the highlight (selection, symmetry
  selection, locked, rollover) goes on top: the highlight colour times how
  bright the style made the default grey. In the lit styles that equals
  the old look exactly. In Gooch, Cel and Reflection the brightness only
  follows the style from 45 % up (`MARKED_FLOOR`, by eye), so dark tones
  don't bury the selection. Vertex Map keeps its own tint. Provisional.
- **VS-Q08 Reflection matcap choice (Operandi extra, Jason 2026-11-10).**
  Viewport Properties > Advanced > Reflection: Matcap (8 built-in Blender
  matcaps or Custom Image), Matcap Image (path) and Load Image…, all
  running `operandi.view3d.matcap` (Operandi-only, not an undo step like
  the other view3d settings). Saved per view; custom images by path.
  Default Full Metal (Jason picked "the metal one", 2026-11-10; one line,
  `matcap::DEFAULT`); views saved before matcaps existed load with it too.
- **VS-N01 Naming deviation: the Reflection style is called Matcap
  (Jason approved, 2026-11-10).** The reference name is "Reflection".
  Operandi shows "Matcap" in the Display menu, the Shading Style list, the
  view header and the Viewport Properties > Advanced section (rows Image,
  Custom Image, Load Image…). `view3d.shadingStyle Reflection`, the pie
  menu's "Shading Reflection" and the MCP bridge's `reflection` still work;
  saved views and presets are unchanged on disk (the style is stored as
  before).
- **VS-Q09 Show Wireframe in Item Mode, off (viewport-display spec
  2026-11-03; Jason, 2026-11-10).** The spec says only what "on" does
  (every shown mesh draws its wire in Item mode). With the default Uniform
  overlay, "off" changed nothing, and the wire always showed. Built: in
  Item mode the plain wire draws only when the box is ticked; component
  modes keep the group's overlay; Wireframe style is unaffected.
  Provisional, while the spec side checks what the reference does with the
  box off. Main had the same behaviour before this change.
  **Answered by VS-6 (spec v1.2, 2026-11-11):** off hides the plain wire
  in Item mode even with the overlay on; the box and Colored are the
  defaults. Built on branch item-mode-wire.

## Items list eye hides children — 2026-11-10 (branch group-eye-children)

Built from GV-01..06 (behaviour notes, 2026-11-10): an eye click with
Auto Hide Child Items on (now the default) hides the item and its subtree
through it ("all off", `Item::hides_children`, native v35), the children
keeping their own eyes; off, only the item. Ctrl/Cmd+click is a plain
click; any parent, not only Group Locators. Answers ITEM-Q02.

- **GEYE-Q01 Shift+click and Alt+click on a parent's eye** (provisional):
  Shift is a plain click; Alt+click solos the item with its subtree, every
  other row hidden on its own (a hidden ancestor of the soloed item turns
  to a plain off so it draws). Unverified in the reference.
- **GEYE-Q02 Child eye under a hidden parent** (GV-05, Operandi's call):
  drawn faded; clicking it toggles the child's own eye and it stays hidden
  until the parent shows. Does the reference draw or allow this?
- **GEYE-Q03 Eye sweep** (provisional): a drag down the eye column uses
  the same rule per row; hiding a parent and its child in one sweep also
  turns the child's own eye off.

## Delete an edge loop — 2026-11-10 (branch claude/project-thread-byrc95)

Jason, 2026-11-10: selecting an edge loop and pressing Delete should remove
the loop, merge the polygons on either side and remove the points left
over, leaving valid geometry. Built: Delete / Backspace over a viewport in
Edge mode run Edge Remove (`edge.remove`); with the Items list focused they
still delete items. Edge Remove itself now takes out leftover points.
Jason then widened it: Delete removes the selected elements as a general
rule. Over a viewport the keys run the name map's `delete` command. A
deleted point takes every polygon using it, leaving one hole edged by the
untouched outer ring (eight-sided around a quad-grid point, no triangles;
Jason, 2026-11-10, not Vertex Remove's diamond). Deleted polygons go and
so do the points only they used. Edges run Edge Remove; Item mode runs
Delete Item (supersedes PAR-012's Items-list-only
reading). The `delete` line still has to reach
`specifications/command-names/MODO-NAMES-SUBSET.txt` through the spec path.

- **Delete vs Backspace** (Jason, 2026-11-10, later the same day): forward
  Delete (fn+Delete on Mac laptops) runs `delete`, which also takes the
  points left unused; Backspace removes edges and polygons but keeps their
  points (Edge Remove, Remove; CMD-E-01, REM-001). Points: both keys run
  `delete`. Edge Remove itself is back to keeping points.
- **ER-Q01 Which points does Delete take out?** (provisional) A point of a
  removed edge goes when no polygon uses it any more, or when it is left
  between exactly two interior edges (as all along a removed loop). Border
  points stay (CMD-E-01/02 measured only points that keep other edges), as
  do points on a polyline and any point whose removal would leave a polygon
  under three sides. Does the reference also take out a border point left
  between two border edges (the strip's 1–4 case)? Since issue #5
  (2026-11-23, provisional): a border point left between two border edges
  of one polygon goes when they run in a straight line (a loop cut's ends);
  a corner stays. Edge Remove still keeps every point.
- **ER-Q02 Delete in Polygon mode drops orphaned points** (Jason's
  leftover-points rule): unlike Remove (REM-001 keeps every point), Delete
  also drops points only the deleted polygons used. Points that were
  already unused stay.
- **Delete leaves no orphans** (Jason, 2026-11-15): forward Delete in Edge
  mode also takes every point it leaves with nothing attached, such as the
  far corners of a lone quad or triangle that goes when its edge's points
  leave it. Backspace (Edge Remove, Remove) still keeps points.
- **ED-Q01 Delete on a polyline span (provisional).** A selected edge of a
  polyline (a stray edge with no polygons) was refused ("Remove dissolves
  interior edges"). Delete now cuts the span out: an open line splits there,
  a closed line opens there, a piece left with one point goes, and points
  left unused go. Start: an open line 0-1-2-3, Edge mode, span 1-2 selected,
  Delete. Does the reference split the line, delete the whole line, or
  refuse? Backspace (Edge Remove) still refuses on lines.

## Pen tablet (pen tablet plan stages 1–2, SC-45, SC-62)

- **PTQ-01 Pressure source on macOS.** winit 0.30 (eframe 0.33) reports no
  pen pressure, tilt or eraser on macOS, so Operandi reads them from AppKit
  in its existing `sendEvent:` observer (no new monitor, no vendor SDK, no
  new crate). Only events AppKit marks as tablet events carry pen values;
  plain mice and trackpads (including Force Touch pressure) stay at full
  pressure, so mouse sculpting is unchanged [chosen]. Windows and Linux
  always report a mouse at full pressure until they get their own reader.
- **PTQ-02 Pressure Curve shapes.** Soft = p^0.5, Linear = p, Firm = p^2
  [provisional]. The curve is applied once where samples arrive, so every
  later user of pressure sees the curved value.
- **PTQ-03 Low Pressure threshold.** With Smooth Mode = Low Pressure a dab
  under 35 % pressure smooths (at Smooth Amount, its pressure stretched over
  0..1 below the line), a harder one sculpts at its own pressure
  [provisional]. The undo label stays the brush's ("Push Stroke"), as the
  stroke is still that brush. Move ignores Low Pressure (it grabs).
- **PTQ-04 Pressure Sets Brush Size.** Lives in Preferences > Input > Pen
  Tablet with the curve and the eraser (not in the Sculpt tab), off by
  default. Size = brush size × pressure, never under 5 %; dab spacing
  shrinks with it so small dabs still join [provisional]. Strength follows
  pressure either way (SC-45).
- **PTQ-05 Low Pressure greyed.** The Smooth Mode entry greys until a pen
  has come near the tablet (a proximity event or any tablet point) in this
  session, not until "a tablet is attached": macOS has no event for an
  idle attached tablet. A setting saved as Low Pressure still loads; with a
  mouse (full pressure) it sculpts as Shift Key would without Shift.
- **PTQ-06 Eraser end.** Read at the press like the modifier keys (SC
  section 7): Invert (default) = Ctrl, Smooth = Shift; held keys still add
  their own effect. Only sculpt strokes use it; elsewhere the eraser is an
  ordinary left button.
- **PTQ-07 Pressure keys.** The preferences use the automatic group keys
  `input.pen_pressure_curve`, `input.pen_eraser_action` and
  `input.pen_pressure_size` (snake case like the other group fields), not
  the camel-case names sketched in the plan. Rename when the reference name
  map has entries for them.

## Absolute Scaling (snaps-precision-2026-11-10 part A)

- **ABS-Q01 (spec Q1) popover.** Which modifier opens the window as a
  popover isn't known, so only the floating window is built.
- **ABS-Q02 (spec Q2) flat axes.** Answered (Jason, 2026-11-10): every mode
  leaves a zero-size axis alone; Uniform with a zero-size Uniform Axis does
  nothing and says so. Cases 11, 13, 17, 25, 29, 34 have Operandi answers.
- **ABS-Q03 (spec Q3) selection after.** The reference adds the working
  set to the vertex selection. Not done: the selection is left exactly as
  it was [provisional].
- **ABS-Q04 (spec Q4) Orient frame.** The frame is the smallest-volume box
  (then smallest surface) among the world axes and frames built from pairs
  of the selection's edge directions and polygon normals; sides are named
  after the world axis they lie closest to, a 45° tie going to the longer
  side first (case 37). AS-18 (flat turned square): Operandi fits the square
  (1 × 1 × 0) and Uniform scales it; the reference did nothing
  [provisional].
- **ABS-Q05 (spec Q5) undo.** One step per press; Grab Size records none.
- **ABS-Q08 (spec Q8) script arguments.** `scale.absolute`'s optional
  arguments become the window's values for that press and after, then the
  press refreshes them as the buttons do [provisional]. Centers take
  center/high/low, an index, or the stored names (highX, lowY).
- **ABS-Q09 (spec Q9) hidden/locked geometry.** Measured like any other
  point; the usual element protection keeps it from moving [provisional].
- **ABS-Q10 window layout.** X, Y, Z and the three Centers are one row
  each (UIC-110), not three fields on one row. The open state isn't kept
  across restarts (A2 says persistent; UIC-73) [provisional].
- **ABS-Q12 Origin centre (Operandi addition, Jason 2026-11-11).** X/Y/Z
  Center also offer Origin: that axis scales about the item's origin (0 in
  the mesh's own coordinates), so the geometry doesn't slide off the
  origin. Script value `origin` (or `originX` etc.). Not in the reference.
- **ABS-Q11 item transforms with several meshes.** Each mesh's points are
  combined in its own local coordinates (A4); with differently placed items
  the shared box mixes spaces. Only same-transform items were measured
  (case 38).

## Snaps and Precision panel (snaps-precision-2026-11-10 v1.2)

All of these are the spec's [guess] defaults or my own; Jason corrects later.

- **SPQ-01 Snapping state.** A preset turns snapping on and leaves it on;
  its button again drops the tool; another preset swaps (spec Q2 guess).
- **SPQ-02 Geometry Snap mode.** Operandi's snapping has Vertex and Grid
  targets only, so every preset snaps to vertices (no edge or polygon
  centres or nearest points yet). Tightness 15 px isn't applied; the user's
  Inner/Outer Range are.
- **SPQ-03 Element centre.** The four handle presets wait for one click on
  a vertex, edge or polygon (Auto Center pick): centre = its middle, axes
  Z = its normal (a point's or edge's: the average of its polygons'). The
  click isn't snapped. The previous action centre comes back when the
  preset's tool goes.
- **SPQ-04 Scale/Size Absolute handles.** Per axis a base circle at the base
  length and a scale square at base × factor; factor = (B − centre) / base
  along the axis, at least 0.001 (no mirroring by drag). Ctrl on a square
  scales all axes alike. Where the two coincide the base is picked first,
  then the square. Scale Absolute keeps the plain Scale handles too.
- **SPQ-05 Planar Rotate.** Axis = the clicked element's normal (the frame's
  Z). Rest handle starts at the frame's +X, 96 px out; the rotate handle 60
  px. Moving the rest handle keeps any turn so far. Ctrl steps by Rotation
  Snap Angle.
- **SPQ-06 Planar Rotate Snap Axis.** Start/end handles start on the axis
  either side of the centre; once both are dragged the axis runs start →
  end through the centre. Setting an axis after a turn keeps the turn as
  its own step.
- **SPQ-07 Advanced Handles.** One-way set of the selected tool handle
  style (greyed while Advanced). Advanced now means: plain Rotate also
  shows the planar rest/rotate handles (about its Z). No View ▸ Tool
  Handles menu yet (Preferences > Tool Handles has the setting).
- **SPQ-08 Menu.** Edit ▸ Snapping order: Snaps and Precision, Snapping
  Options…, divider, Snapping. Mesh Constraints isn't built, so it isn't
  listed. The panel doesn't reopen after a restart (§2).

## Text fields keep their keys — 2026-11-10 (branch text-field-keys)

- **TFK-Q01 Tab in a lone text field.** UIC-51 covers numeric fields; Jason
  asked (2026-11-10) for Tab to reach text fields too. A field with no other
  field in its panel (search boxes, the command line, an inline rename)
  keeps focus on Tab, and Tab does nothing there [provisional]. Should Tab
  leave the field instead?
- **TFK-Q02 Cmd shortcuts while typing.** Every shortcut is off while a
  field has focus, Cmd+S/Cmd+W included, as before for listed fields; the
  field's own Cmd+A/C/V/Z still work [provisional]. Should Save work while
  typing?

## Booleans stage 1 — 2026-11-11 (branch booleans; spec booleans/BOOLEANS.md)

- **BOOL-Q1 Last Selected** (spec Q1). Built as recommended: within the
  active mesh, every connected piece holding a selected polygon is the
  driver and the rest is the target [provisional]. Needs Polygon mode with a
  selection; Include Children is greyed there.
- **BOOL-Q3 Open input** (spec Q3). Refused with one notice naming the item
  and the problem (open edges, edges with more than two polygons, polygons
  facing opposite ways, inside out, passing through itself, curves) and a
  **Select Problem Edges** button [provisional]. **Close holes and retry**
  isn't built: Operandi has no hole-fill command yet.
- **BOOL-Q4 Engine disagreement.** Every boolean also runs boolmesh; when its
  volume or extent differs from Manifold's by more than a millionth of the
  inputs' size, nothing changes and the status line says the engines
  disagreed. Should that be a notice like the refusals instead?
- **BOOL-Q5 Not built yet.** BOOL-30 named result sets (Intersecting Edges,
  Polygons from Geometry/Driver Surface), BOOL-22 progress bar and Esc
  cancel (the boolean runs in one go), Solid Drill, Axis Drill, Curve
  Boolean and the procedural versions (later stages).
- **BOOL-Q6 Locked or hidden elements** on the target: refused ("unlock and
  unhide them first") [provisional]. Should a boolean keep them instead?
- **BOOL-Q7 Corner data on new points.** UV maps blend within the source
  polygon (barycentric in its triangle holding the point). Corner weights,
  colours, normals and legacy per-polygon UVs can't be interpolated by the
  shared rebuild yet, so a mesh with those on cut polygons is refused
  ("nothing changed").
- **BOOL-Q8 Add and other tools.** Add's result is deliberately non-manifold
  (four polygons on each crossing edge). It is recorded as an admitted
  exception, so tools that refuse non-manifold meshes refuse it until it is
  cleaned up.

## Render item settings — 2026-11-11 (branch render-item, spec render-item-2026-11-11)

- **RI defaults (spec v1.2).** New scenes: Maximum Quality 64, Noise
  Threshold 10 % (Cycles 0.1), depths 8 / 4 / 8, Preview Convergence
  Target 97.5 % and Max Render Time 5 min. Older scenes keep 16 samples,
  their threshold and Denoise, Cycles' 7 / 7 / 7, and a Preview with no
  target or time limit. Today Operandi passes Cycles its defaults (7 each)
  for final renders (RI-Q06).
- **RI-B01 Noise Threshold 0 %.** The spec says 0 % lets Cycles choose with
  adaptive sampling always on. Older scenes stored 0 meaning "adaptive
  sampling off", so 0 % stays "off" to keep them rendering as before
  [provisional]. New scenes start at 10 %.
- **RI-B02 Total bounces.** The mapping says max bounces = largest depth
  + 1. Built as the largest depth, so older scenes (Cycles' 7 / 7 / 7,
  total 7) render exactly as before [provisional]. Transparent bounces
  follow Refraction Depth as the mapping says (7 for older scenes, Cycles'
  own value).
- **RI-B03 Denoising.** RI-Q05 moves Denoise onto the Final Color output.
  Built as one switch: every Final Color output's Denoising row shows and
  sets it, with None or Intel Open Image Denoise only; other outputs keep
  their stored choice. Per-output denoising would need the render to
  denoise each output on its own [provisional].
- **RI-B04 Frame size presets and link.** RI-F03 asks for a value presets
  arrow (UIC-55) and a link (UIC-56). The shared form has neither yet, so
  Width and Height are two plain rows; the presets and link come with the
  form widgets.
- **RI-B05 Left for later.** Tier 1 Pixel Aspect Ratio, Render Region (and
  tying it to the render window's Region) and the frame presets; all Tier
  2 rows, Apply Preset (built-ins 16 / 64 / 512, 20 / 10 / 2 %) and QuickCam
  Resolution (RI-Q11). Film Back Preset still sets film only (CAM-Q04
  stays open).
- **RI-B06 User Channels and Tags tabs.** RI-10 lists them, but those
  tabs store their data on scene items and the Render item isn't one. Left
  off until they can hold the Render item's own data (UIC-117 exception:
  not built, so not shown).
- **RI-B07 Convergence Target.** RI-P01 says the Preview stops at this
  share of Maximum Quality. Built as a share of the Preview's own Samples
  (Preview Settings, the RP sample target), so the Preview keeps its own
  quality setting [provisional]. Max Render Time stops the Preview through
  Cycles' time limit and keeps the picture so far; F9 never uses either.

## Sculpt toolbox brushes — 2026-11-11 (branch claude/project-thread-09ebuw, spec sculpt-toolbox-2026-11-11 v1.1)

Built on the Blender-derived maths in `operandi-core/src/sculpt_blender`.
The spec's own open questions SCT-Q01..Q16 stand; these are the builds
chosen for them.

- **SCB-01 Fold profile (SCT-Q05).** Tip weight × a signed profile: −1 at
  the centre, 0 at half the radius (`FOLD_CROSSOVER`), +1 at the rim, where
  the tip fades it to nothing. Additive, along the Average normal
  [provisional].
- **SCB-02 Clay (SCT-Q06, Q07).** The plane is the surface at the press
  (sampled with Blender's plane sampler from the press positions) moved
  Level × radius along the Normal Mode direction (Average). Points below
  move toward it by Offset Amount × weight per dab and never past it, so a
  longer stroke doesn't build higher. Ctrl: the plane sits Level below and
  points above it are cut down [provisional]. Density Mode is shown but the
  plane is already the cap.
- **SCB-03 Smudge (SCT-Q08).** Each dab moves points by the pointer's step
  since the last dab, flattened into each point's tangent plane, × weight ×
  Offset Amount (Blender's Nudge). Ctrl reverses [provisional].
- **SCB-04 Scar (SCT-Q09).** Draw inward along the Average normal plus a
  pinch toward the stroke line (the axis across the stroke), at the same
  strength [provisional]. Ctrl: outward with the same pinch.
- **SCB-05 Flatten (SCT-Q10).** Brush Center: the plane through the dab
  centre with the surface normal, re-taken each dab; Mouse Down: the press
  centre and normal for the whole stroke. Inverted Flatten off: only points
  above the plane move (below stay put); on: only points below. Each dab
  moves a point Offset Amount × weight of the way. Ctrl pushes up (Draw
  along the Average normal) [provisional].
- **SCB-06 Pinches, Spin, Emboss (SCT-Q11, Q13).** Pinches slide in each
  point's tangent plane by Offset Amount × weight × the dab spacing share.
  Spin turns Offset Amount × weight × 90° per radius of travel, clockwise
  looking down the axis toward the surface; mirrored dabs turn the other
  way. Emboss: pointer travel along the screen's right axis × Offset
  Amount × weight, along each point's own normal at the press [all
  provisional].
- **SCB-07 Move Slide (SCT-Q14).** Each point's move loses its part along
  the point's normal at the press, so it slides in its tangent plane. On a
  curved surface points can still leave it for long drags [provisional].
- **Toggles for Jason.** `ATTENUATE_ON_PLAIN_MESHES` (sculpt.rs, false):
  true makes Attenuate fade toward the shape at the stroke's start. Masks
  (SCT-Q15) aren't built yet.
- **Skipped rows.** Carve's Set Color / Color / Alpha and Inflate's Paint
  Mode (image painting only, SCT-07) aren't shown yet; Sculpt Mode and
  Curve Constraint rows show their image and background choices greyed.

## Vertex Map Tools and falloff — 2026-11-11 (branch vertex-map-tools, spec vertex-map-tools-2026-11-11 v1)

The spec's VM-Q01..Q12 stand; these are the builds chosen.

- **VMB-01 Stroke formulas (VM-Q12).** As the spec's guesses: Paint
  new = old + (1 − old) × Paint Value × weight (Ctrl: old − old × k);
  Smooth toward the edge neighbours' average, no value read as 0 (Ctrl
  sharpens); Scale Down × (1 − Scale Amount × weight × 0.1) per dab, only
  points with a value (Ctrl scales up); Erase removes values; Set Value
  writes Value under the whole brush with no fade (Ctrl erases)
  [provisional].
- **N32 Overlapping Paint dabs (VMT-05, VMT-10, VM-Q12).** Start with an
  empty weight map and sweep the full-pressure Paint brush over one point
  at 20%. A centred dab gives 0.20, as specified. The viewport samples
  every 5% of the brush radius by default and feeds each sample to the
  same formula. Twenty full-tip dabs give `1 − 0.8^20 ≈ 0.988`. The
  existing core test also requires repeated dabs in one stroke to build
  toward 1. Should interpolated samples share a strength budget per pass,
  or should VMT-05 cap the change per press-to-release stroke, even when
  the brush passes over the same point again?
  Owner, 2026-11-19: a whole stroke at X% strength adds about X%; overlapping dabs within one stroke must not stack toward full.
  Paint now blends from the press-time value toward 1 (Ctrl: toward 0)
  using the strongest dab at each point. This supersedes VMB-01 for Paint.
- **N32 Subdivision weight display (VMT-30, VMT-31).** Show Weightmaps
  already interpolates per-point weights on ordinary faces. Both the CPU
  tessellation and GPU patch paths use one average per cage polygon.
  Should evaluated surface vertices use weights interpolated in cage
  coordinates (like linear UVs) or follow subdivision's smooth evaluation?
  Live weight edits currently rebuild the full GPU geometry, so a display
  fix also needs a local update path for affected faces during painting.
  The active paint tool's bright dots come from a separate viewport
  overlay. The GPU vertex dots already use their normal colours.
  N32 removes the separate dot overlay. *Provisional:* both subdivision
  paths still use the cage polygon average. Smooth evaluation is deferred
  until both paths can update the weights of affected faces locally.
- **VMB-02 Normalize (VM-Q04).** Only lowers the other weight maps at an
  edited point, and only when the total passes 100 %; does nothing with a
  single map [provisional].
- **VMB-03 Selected map.** Operandi had no selected vertex map, so it is
  tool state (`Editor::selected_weight_map`), picked in the Falloff menu
  and the Vertex Map Tools section; not saved with the scene. A Vertex Map
  falloff on a mesh without that map weighs 0 there [provisional].
- **VMB-04 Names (VM-Q01).** New Weight Map starts as "Weight Map"; a
  stroke with nothing selected makes and selects "Weight" (VMT-18), as its
  own undo step before the stroke's.
- **VMB-05 Display (VM-Q09, Q10).** While a Vertex Map Tool is on, each
  point with a value is drawn in the weight colours (VMT-30, matches
  VM-CASE-16); points with no value aren't drawn. Show Weightmaps / Show
  Weight Values options and the Vertex Map shading style reading point
  maps aren't built yet.
- **VMB-06 Command names.** From the registry reply: tools
  `sculpt.vmapPaint`, `sculpt.vmapSmooth`, `sculpt.vmapScale`,
  `sculpt.vmapErase`, `sculpt.vmapSetVal`; `vertMap.new`,
  `select.vertexMap`, `sculpt.liveWeightEdit live:<bool>`;
  `falloff.vertexMap mode index mix` and `falloff.invert` (also `tool.set
  falloff.<kind> on|off`, the Add Falloff route). Added to the name map
  (MODO-NAMES-SUBSET.txt) with Jason's OK, 2026-11-11.
  `tool.attr` on the tools' shared attributes (oval, sval, normalize, ...)
  isn't wired yet.
- **VMB-07 Symmetry and the falloff.** Transforms read a Vertex Map
  falloff at the point's own value, not its positive-side mirror's
  (FAL-B2-04 samples Linear / Radial at the mirror) [provisional].


## Mesh Stack v1 (specifications/meshops-2026-11-11)

- **MOQ-01 Auto-capture (MO-20).** Polygons selected on the mesh become the
  new op's Select by Index only when the op lands directly above the Base
  Mesh (its input is the Base Mesh). Anywhere else the op works on the
  whole mesh, because selection numbers the Base Mesh, not the result
  below the op (SAFE-06) [provisional].
- **MOQ-02 Selection on a mesh with a stack.** Outside Edit Base Mesh, the
  mesh shows its result and keeps no component selection (SAFE-06);
  components are picked in Edit Base Mesh only. Select by Index Set/Add
  /Remove read the Base Mesh selection [provisional].
- **MOQ-03 Direct tools (Q1).** A command that would change the Base Mesh
  of a mesh with a stack is refused with the Edit Base Mesh / Freeze
  message; a failed command on such a mesh gets that hint appended. A
  command that changes nothing (Flip on an empty Base Mesh) runs.
- **MOQ-04 Time limit (SAFE-04, SAFE-10).** Not enforced: a timed failure
  would make results depend on machine speed (SAFE-11). Evaluation runs off
  the UI thread and newer edits cancel it between ops. Output limits are
  fixed at 4 million polygons and points, not yet preferences.
- **MOQ-05 Freeze and export wait.** Freeze, render, preview and export
  evaluate the current recipe on the UI thread when the background result
  isn't ready (SAFE-14 needs a current result). A very heavy stack can
  pause the UI there.
- **MOQ-06 Op values.** Ops store the reference attributes Operandi builds
  (registry MESHOPS-NAMES rev 2); unbuilt ones (Cube radius, axis, UV
  options, Bevel segments...) aren't stored and `item.channel` refuses
  them. Mirror and Array save as a Clone Effector (`effector.clone.item`:
  merge, dist) with a `pmodel.mirror` / `pmodel.array` generator in its
  Tool Pipe (registry rev 3, confirmed); v1 files with the flat
  `mirror.clone.item` / `array.clone.item` form still load. Triangulate has
  no channels of its own.
  Subdivide `type` and `boundary` use the dialog order (SUBT-Q01).
  Subdivide's Fractal isn't an op value (fixed seed).
- **MOQ-07 Ops as script targets.** Ops aren't scene items in Operandi, so
  `item.delete`, `item.parent`, `item.channel`, `select.item` and the
  `deformer.*` commands take `op:<item>:<op>`; `op:<item>:base` selects the
  Base Mesh row. `item.parent position:` counts in evaluation order (0 runs
  first).
- **MOQ-08 Tab and layout.** The Mesh Stack tab is the last tab of the upper
  side panel (MO-10 puts it beside the Items list) and shows only the
  selected mesh (MO-28). No Item List inside it, no Ghosted/Hide/Single/
  Filter buttons, no Defer/Pause, no viewport handles on ops (MO-19) and
  no Tab/Shift+S/Shift+F/backtick shortcuts (Q4) in v1. Add Operator is a
  menu (the preset browser isn't merged). The selected op's values show
  in the Properties panel (one "Mesh Op" category) rather than under
  the list.
- **MOQ-09 Ghost.** Edit Base Mesh draws the full result as a light-blue
  see-through wire (up to 200,000 edges); the colour isn't a preference
  yet. Mid-stack ghosting (MO-26) isn't built.
- **MOQ-10 What still reads the Base Mesh.** Viewport drawing, picking,
  polygon counts, Cycles render and preview, and OBJ/STL/glTF/LXO export
  use the result. Frame/Fit bounds, measuring, snapping to a procedural
  mesh, MCP mesh queries and LXO stack round-trips still see the Base
  Mesh (the spec's full `.mesh` audit is later work).
- **MOQ-11 Bevel on a whole closed mesh.** With no Select by Index and
  Group Polygons on, a closed mesh has no region edge, so the op bevels
  each polygon on its own instead of failing [provisional].
- **MOQ-12 Loop found when opening a file (SAFE-07).** The op that closes
  the loop is turned off; Operandi has no load-warnings list yet, so the
  only sign is its eye being off [provisional].
- **MOQ-13 A linked item still evaluating.** In the viewport, an op whose
  linked item is still being evaluated in the background waits and keeps
  showing its last result, instead of failing for a moment [provisional].
- **MOQ-14 Links ignore the source's Edit Base Mesh.** An op reading an
  item that is in Edit Base Mesh reads that item's full result, not its
  Base Mesh [provisional].
- **MOQ-15 Boolean op driver (BOP-01/30).** The op links one Driver
  Surface; Operandi also lets that be a group, which then drives with all
  its meshes [provisional].
- **MOQ-16 Adding a Boolean op (BOP-10).** No create dialog yet: a second
  selected mesh becomes the driver; with none, the op shows "choose its
  Driver Surface" until one is picked in Properties [provisional].
- **MOQ-17 Union All (BOP-31).** Every driver already acts as one solid
  (BOOL-07), so Union All gives the same result as Union [provisional].
- **MOQ-18 Boolean on an empty mesh below.** Passes through unchanged
  [provisional].
- **MOQ-19 Merge Meshes switches (MM-Q1, MM-Q5).** Polygon Tags off drops
  part tags and keeps materials; the second channel named `curve` isn't
  kept (Operandi's values are named once); Spline Patch, Text and Curve
  Fill switch polygon types Operandi doesn't have yet [provisional].
- **MOQ-20 Links in files.** Operandi saves an op's links in its values
  (`driver`, `sources`) and Merge Hierarchy as `hierarchy`; reference files
  use links and an item tag (BOP-01, MM-02/03), so the LXO reader and
  writer will have to translate [provisional].
- **MOQ-21 Selection ops pick polygons only.** Edge and point groups
  (`newEdges`, `loop`, `intersecting` ...) and Convert wait for edge and
  vertex ops [provisional].
- **MOQ-22 How groups follow polygons.** A group stays on a polygon an op
  above leaves untouched (same corners); a polygon an op rebuilds leaves
  its groups. Bevel and Smooth Shift `sides` = their new polygons that
  aren't fronts; no `back` yet [provisional].
- **MOQ-23 Range and Pattern values.** Range is Start, End (0 = last,
  inclusive), Step; Pattern is the 1,0 text. Channel names and the Select
  by Selection Set type id (`operandi.selset.selop`) are Operandi's own
  [provisional].
- **MOQ-24 A Selection input that picks nothing.** The op leaves its
  input unchanged (PG-04 leaves it to the op) [provisional].
- **MOQ-25 Edge and vertex ops' elements.** Until selection ops pick
  edges and points (MOQ-21), an edge op works on every edge or the edges
  of the polygons its Selection input picked; a vertex or deform op on
  every point or those polygons' points [provisional].
- **MOQ-26 Channels kept but not built.** Some ops save reference channels
  their engine doesn't use yet; they are kept and saved but not shown
  (hidden fields): Edge Bevel sharp/mitering/coplanar/boundExtend, most of
  Edge Chamfer's and Edge Relax's extras, Vertex Bevel Use Material,
  Vertex Merge allowSelfMerge/mergeToClosest, Slice axis/snap/fast, Axis
  Drill selectEdges/selectionSetName/frontOnly, Polygon Reduce Legacy
  Mode, Jitter's rigid and UV options, UV Unwrap's projection, symmetry
  and seam choices, most Pack UVs region options, Transform UV tile and W
  [provisional].
- **MOQ-27 Jitter op.** Operandi has no direct Jitter tool; the op moves
  each point by a seeded random amount within +/- Range per axis
  [provisional].
- **MOQ-28 Ops not built yet.** Loop Slice, Edge Slide, Bridge, Chamfer
  Edit, Curve/Pen/Primitive Slice, Curve and 2D primitive ops, Cone,
  Capsule, Ellipsoid, Toroid and Text as ops, Thicken (a preset of Smooth
  Shift), Create Polygons, Set Polygon Type, Freeze op (needs the item's
  subdivision settings), Surface Constraint, Scatter/Curve/Pen/Particle
  Clone, Radial Sweep, Create UV Map, UV Fit, UV Rectangle, UV Flip,
  Convert UV to Mesh, vertex map ops, Transfer Vertex Map [open].
- **MOQ-29 Add browser (AB-xx).** Entries use their direct tool's
  generator icon (Cube, Bevel, Edge Relax ...); ops with none (Flip
  Polygons, Polygon Unify, Delete, Mesh Cleanup) use the default cube
  (Jason 2026-11-17); "+" and
  "(add path…)" are greyed (these lists are built in); the funnel shows or
  hides the search row; no gear button; selection ops are added from an
  op's Selection input (Add Selection…, a plain list) rather than a
  Selection folder in Add Operator; Add Item's label opens the browser and
  its arrow the IL-02 menu; Deformers and Assemblies roots stay hidden
  while empty [provisional].
## Nodes N1 (specifications/nodes-n1-2026-11-11)

- **NM-Q01 NM-T1 reference.** No reference render of the Noise -> Color
  Ramp graph made in Blender's Cycles exists yet. The test renders the
  graph and checks every pixel lies on the ramp and the noise varies; a
  pixel diff needs a reference picture (and its camera) made in Blender.
- **NM-Q04 Ramp interpolation.** Color Ramp Linear, Constant and Ease are
  baked as Blender's colour band does; Cardinal and B-Spline are read as
  Linear [provisional].
- **NM-Q05 Several Node Materials.** Only the topmost Node Material that
  reaches a polygon renders; one below it is hidden, not mixed
  [provisional].
- **NM-Q06 What "ignores layers below" covers.** The whole layered result
  (including layers above the Node Material in its group that drive
  ordinary channels) is the Mix Shader's first input; the graph's surface
  replaces it by the layer's coverage (opacity x masks x groups).
- **NM-Q07 Viewport (NM-40).** The background bake isn't built: the
  viewport shows the spec's "while baking" colour always (the principled
  Base Color when it is a plain value, an Emission/Glass/Transparent
  colour likewise, else mid grey), mixed over the layers by coverage.
- **NM-Q08 Generated coordinates.** Meshes using a graph get Blender's
  Generated coordinates: 0..1 across the mesh's own bounding box (a flat
  axis spans 2 m), in the mesh's space.
- **NM-Q09 Displacement.** A graph linked into the Material Output's
  Displacement replaces the layers' displacement and is true displacement
  (diced), as the tree does today; Bump-only isn't offered.
- **NM-Q10 Preset badge.** `Preset::has_nodes` is there for the "Nodes"
  badge; the preset browser isn't on main yet, so nothing draws it.

## Blender materials converter, Lane A (BMC, 2026-11-11, branch blender-materials-starter)

Built from the coordinator's brief "Step 1: Layers lane + CC0 starter pack"
(no behaviour spec). Readings marked here are provisional.

- **BMC-Q01 Credits field.** Licence, source URL and credit live in the
  preset header as `source: {licence, url, credit}`; a preset carrying one
  needs native version 37. Older builds refuse such a preset (unknown
  header field); older presets load unchanged. Is that the field the
  preset browser should show and copy?
- **BMC-Q02 Mapping offset.** A UV Map locator has repeat and rotation but
  no offset, so a Mapping node's Location is dropped with a note
  [provisional]. None of the 32 starter materials use one.
- **BMC-Q03 Generated coordinates.** Images on Texture Coordinate >
  Generated are read with the mesh's UVs [provisional]; there is no
  bounding-box projection to match it exactly.
- **BMC-Q04 Transmission.** Principled Transmission becomes Transparent
  Amount with the base colour as Transparent Color, the IOR and the
  roughness [provisional, approximate]. The forward translation never
  feeds the principled Transmission.
- **BMC-Q05 Which material.** A .blend may hold several materials
  (ambientCG ships _NORMAL and _DISPLACEMENT variants). The converter
  takes the one using the most of the folder's pictures, a normal or bump
  breaking ties [chosen]; `--material` overrides.
- **BMC-Q06 Maps the graph leaves out.** With a graph, the graph decides
  which maps are used; the folder's other maps (ambientCG's DX normal,
  displacement and AO) are left out with a note [chosen]. Without a graph
  the loader's CD-21..25 rules apply.
- **BMC-Q07 Strength above 1.** Normal Map strength becomes the layer's
  opacity, so strengths above 1 are used as 1 (noted). Bump amplitude is
  Distance × Strength [provisional].
- **BMC-Q08 Data maps in Cycles.** Found while building: the Cycles bridge
  read every picture except normal maps with the sRGB curve, so roughness,
  metal, height, opacity and similar maps rendered darker than meant. Now
  only colour inputs (base, subsurface radius, specular tint, emission
  colour) read as sRGB; the rest read as plain data, as Blender's
  Non-Color does. The realtime viewport still uploads every image as sRGB
  (`material_gpu.rs`); not changed here.
- **BMC-Q09 Packed AO order (loader).** The texture-set loader put a packed
  ARM map's AO multiply below the base colour, where it was covered and
  did nothing. Fixed in the shared module (preset/texture_set.rs) the same
  way as the browser's fix on main (20c52bee): only the packed AO multiply
  goes above the colour; its roughness and metal stay below. The browser now
  loads sets through that module (its copy in library.rs is gone).


## Viewport split and tear-off v1 — 2026-11-11 (branch viewport-split-tearoff, spec viewport-split-tearoff-2026-11-11)

The spec's own questions VST-Q02..Q07 stay open (VST-Q01 answered: four
directions). Build choices on top of them:

- **VSB-01 List minimum.** Toolbox / Items / Properties minimum is
  100 × 80 pt ("header plus two rows" read as 80) [provisional]. Side
  panels aren't in the layout yet (PANEL-Q01), so it only matters later.
- **VSB-02 Detach place.** Detach puts the new window's top-left (title bar
  included) at the view's old top-left, so the content sits one title bar
  lower than the view did [provisional].
- **VSB-03 Torn-off views on top.** Always on top, as palettes (VST-Q05
  answered by Jason 2026-11-11: yes). They still sit anywhere on any
  screen, outside the main window too.
- **VSB-04 Window too small for every minimum.** Dividers still move as long
  as no panel already under its minimum gets smaller (VST-26 overflow rule)
  [provisional].
- **VSB-05 Split preview colours.** Insert line and fill use the drop
  indicator colour; refused splits use the error text colour (VST-Q07)
  [provisional].

## 2026-11-11 — UV Unwrap and Pack UVs (spec uv-unwrap-pack-relax v1.1, branch claude/project-thread-iwegbz)

Spec questions UV-Q2..Q16 are the spec's own; these are the readings
Operandi built while they're open, plus its own gaps.

- **UV-Q4 Method.** Built to the option help: Conformal runs least squares
  conformal maps, Angle Based runs ABF++ and then LSCM [provisional].
- **UV-Q16 Gap values.** Relative Gap Size 0.2 gives about 0.005 at the tile
  edge (UVW observation), so the island margin is `gap × 0.025` of the
  layout. Pack Gaps 0.2 gives 0.01 at the edge and about 0.004 between
  islands (UVP-08): edge `gaps × 0.05`, between `gaps × 0.02`. Both fields
  show the stored number (0.2) [provisional].
- **UV-Q17 Iterations.** Iterations 0 leaves the Initial Projection fitted
  into the tile; any other value runs the solver. The direct solvers don't
  use the count, so 1 and 100 give the same result. Is the count meant to
  run more passes of something (a relax after the solve)?
- **UV-Q18 Island orientation.** After solving, each island is turned so
  its edges run along U and V as far as they can, then its longer side is
  laid along the Pinning Axis, then Offset Angle is added. (The solver's own
  minimum-area turn set a cube's cross of faces at 45°.) Start: a cube cut
  at the cap and the four verticals; is the cross square to the tile with
  its longer side along U?
- **UV-Q19 Stretch.** Pack's Stretch scales each moving island uniformly so
  its UV area matches its 3D area relative to the others; it doesn't
  reshape islands. Does Stretch also change an island's proportions?
- **UV-Q20 Unwrap as a dialog.** The spec makes UV Unwrap an interactive
  tool with live Tool Properties and viewport drags for Iterations. With no
  UV viewport yet, Operandi has UV Unwrap... as a dialog with the same
  fields, in order; OK runs it as one undo step. The tool comes with the UV
  viewport.
- **UV-Q21 Unwrap layout.** Islands are laid side by side without rotation
  (bounding boxes), the layout preferring wider than tall, then scaled
  together so the longer side spans the tile from (0, 0). UVW-11's 1 × 0.75
  holds. With Update Layout off the layout is still fitted (UV-Q3).
- **UV-Q22 Planar projection frames.** Planar along X is seen from +X (U =
  -Z, V = Y), along Y from +Y (U = X, V = -Z), along Z from +Z (U = X, V =
  Y); Cylindrical and Spherical take U around the axis from its first side
  axis. Which way does the reference face each projection?
- **UV-Q23 Several meshes.** Unwrap runs on every active mesh in one step;
  Pack packs the active mesh's islands only (the spec packs several meshes
  together).
- **UV-Q24 UV view as a view mode.** View > UV View switches the active
  view between 3D and UV space (a "3D" button in its header switches back),
  so any view of a quad can be the UV view. The reference has a UV layout
  with its own viewport instead. Not saved with the layout yet.
- **UV-Q25 What the UV view draws.** Only the active mesh's UVs, in the
  view's chosen map (the mesh's UV map by default); no background items yet
  (Show Inactive UVs). Grid every 0.1 over 0-1, labels every 0.5 inside,
  the axis lines, the UDIM number 1001 at the tile's centre; no backdrop
  image or checker yet (UV-Q15), no coverage readout.
- **AC rulebook pass 2026-11-21 (built).** AC-22 Auto Axis keeps World
  axes (the X/Y/Z main-axis buttons aren't built yet); AC-21 the menu's
  Selection Border takes Selection axes; AC-30/31 plain Element picks a
  polygon (centre + normal as Y), edge (middle, normal, its direction as
  X) or point; AC-40 Screen takes the view's axes when chosen (new axis
  rule "Screen"), its click isn't snapped and its depth runs through the
  selection's box centre. Not yet: G7 item mode (Origin/Parent/Pivot
  centres for item transforms) and G8-G10 (menu lists, label, Automatic's
  axis handle).
- **AC-75 / AC-Q2 Local handle (Jason, 2026-11-21, built).** The one
  Local handle sits on the last selected element's cluster, at its box
  centre, with its frame. Clusters join only through the selection
  (AC-03): polygons a row apart stay two clusters. *Provisional (AC-Q5):*
  a polygon cluster's normal goes on the item axis nearest it (Y facing
  up, as ACS-11/12; Z on a wall facing Z, with Y up the wall and X around
  it; X on a wall facing X), as a line takes the axis nearest it (AC-73).
  Start: a polygon on a wall facing X, Local, Rotate Y 30. Does it turn
  about world-up or about its normal?
- **UV-Q26 Overlap and Distortion.** Overlap tints whole polygons whose UV
  triangles overlap another's, not only the overlapping area (UVV-02).
  Distortion colours each polygon by log2 of its UV/3D area share against
  the map's, full red at half and full blue at double (UVV-03).
- **UV-Q27 UV view keys.** Alt+drag pans, Ctrl/Cmd+Alt+drag zooms (left to
  right zooms in), the wheel zooms about the pointer, A and Shift+A frame
  all UVs or the selection's, . and , zoom. G stays Edge Slice (the spec's
  "centre on the cursor" would take it), and the U/V-only pans and box zoom
  aren't built. A click selects (Shift adds, Ctrl or Cmd removes; an empty
  plain click clears); no paint or lasso selection in the UV view yet.
- Not built yet, greyed or refused with a reason: Symmetrize, the UV Seam
  Map source (Operandi has no seam maps), Pack Regions Nearest, Polygon Tag
  and By Item, Background Constraint, Pack UVs to New Map, Unwrap & Relax,
  UV Relax, Cut Map, the UV viewport.

## Freeze v2 (specifications/freeze-2026-11-10, v2)

v2 replaces the v1 guesses (FRZ-B01..B09) where it observed the reference:
selection limits scope, Tessellation off freezes nothing, on-curve points
are kept, 36 steps per span always. Still provisional:

- **FRZ-B01 (Q1)** Polygon mode with nothing selected, and Edge mode, act
  on the whole mesh, like Item mode. Vertex mode with points selected
  freezes nothing and says "Nothing to freeze: Freeze doesn't act on
  selected points"; Vertex mode with none selected acts on the whole mesh.
- **FRZ-B02** Straight polylines are left as they are (observed).
- **FRZ-B03** Frozen curves are added after the other polygons, not in the
  curve's old place. B-Spline points go unless another polygon uses them;
  a B-Spline end shared with another polygon is not reused as a sample.
- **FRZ-B04 (Q5)** Decided (Jason, 2026-11-11: "go with the guide"):
  Fixed Spans off cuts adaptively by Curve Refinement Angle (Option A). A frozen Curve
  (through-points kind) follows Operandi's drawing, so its span-1 midpoint
  is (0.5, 0.6875, 0) against the reference's 0.702254 (the open Curve
  end-tangent question above).
- **FRZ-B05** Curve Refinement Angle runs 0.01°..180°.
- **FRZ-B07** Nothing is selected afterwards in Polygon mode (observed);
  Item mode keeps the item selected.
- **FRZ-B08** `face` 0 = Faces, 1 = Polylines (reference), 2 = 2 Point
  Polygons (Operandi-only), names faces / polylines / twoPointPolygons;
  other values are rejected.
- **FRZ-B09** A straight or S-shaped open curve frozen as Faces is kept as
  an explicit degenerate face (the reference makes the flat Face too). An
  adaptive Faces result with only two samples becomes a two-point line. A
  line that shares a point with a subdivision cage keeps its own copy of
  that point after the surface freezes.
- **FRZ-B10** Polygon mode with some subdivision polygons selected freezes
  them as a patch of their own: along the border with unselected smooth
  polygons the result follows the patch's boundary rule, so it can differ
  slightly from the whole surface's shape there.
- **FRZ-B11 (Q3)** Closed curves: the closing span is sampled too and a
  Polylines result is closed; anchors are kept as for open curves.

## Solid Drill and Axis Drill — 2026-11-11 (branch drills, spec drills-2026-11-11)

Built with the spec's Q1–Q8 defaults (Q1 two hole-free polygons per hole; Q2 numbers 0 Core 1 Tunnel 2 Stencil 3 Slice, dialog Core, Stencil, Tunnel, Slice; Q3 Y; Q4 Core; Q5 Material starts "Stencil"; Q6 on-surface pieces inside; Q7 walls cut by the prism; Q8 Material greyed, UIC-92).

- **DRILL-Q1 Surface Item row.** The spec says shown only for Specific Surface Item; built greyed in place instead (UIC-92/117, as Boolean). [provisional]
- **DRILL-Q2 Slice along existing edges** (D3): nothing changes, so no undo step and a status note ("the cut lines lie on existing edges"). Same for Tunnel/Stencil/Slice when the driver doesn't reach the target (DRILL-25). [provisional]
- **DRILL-Q3 Not built yet.** DRILL-22 named result sets (Intersecting Edges, Stencil Polygons, Polygons from Geometry), as BOOL-Q5; DRILL-09 progress and Esc cancel; DRILL-30 procedural versions.
- **DRILL-Q4 Per-point maps on new points** (weights, morphs, point colours) aren't interpolated; corner maps (UVs etc.) are, within the source polygon (as BOOL-Q7).
- **DRILL-Q5 Several Solid Drill drivers** are unioned first (Manifold), so cuts inside an overlap don't appear. Axis Drill shadows the same way (DR-A5).
- **DRILL-Q6 Pieces touching themselves at one point** (a cut loop meeting a polygon's outline at a single corner from inside) are refused with nothing changed rather than split further. [provisional]

## Viewport materials — 2026-11-11 (branch claude/project-thread-szcspe)

Advanced style now approximates the Cycles principled surface (Blender
EEVEE maths). Built readings, all *provisional*:

- **VM-Q01 Base colour.** Start: a material with Diffuse Amount 50 %.
  Advanced uses Diffuse Color alone (the render's base colour); the other
  styles keep Diffuse × Amount. A textured Amount stays multiplied in
  Advanced too. **Decided (Jason, 2026-11-11): keep as built.**
- **VM-Q02 Highlight of smooth surfaces.** The key light is a point, so a
  mirror would show no highlight; the highlight uses Roughness of at least
  10 %. Should the view light have a size?
- **VM-Q03 VS-3's tight Advanced highlight.** VS-3 measured a small, tight
  highlight on the default grey. With the default material (Roughness
  40 %) it is now broader, still peaking at white halfway between light
  and eye. **Decided (Jason, 2026-11-11): keep the principled highlight
  everywhere.**
- **VM-Q04 Normal map tangents.** Tangents come from how UVs change on
  screen, not stored (MikkTSpace) tangents; seams and mirrored UVs may
  differ slightly from the render.
- **VM-Q05 Reflections.** Reflections see a flat environment equal to the
  Advanced ambient, not the scene's environment or backdrop.
- **VM-Q06 Left out.** Dissolve and Transparent Amount (needs a sorted
  see-through pass), subsurface, refraction, anisotropy, Diffuse Roughness
  (Oren-Nayar), coat tint and coat bump, displacement; cut-outs still cast
  shadows; bump/normal from procedural patterns (they are per corner).
  *Update 2026-11-17 (shader-tree-complete):* Dissolve and Transparent
  Amount were added by viewport transparency; refraction, subsurface,
  coat tint and coat bump now draw (VM-Q08, VM-Q09).
- **VM-Q07 Which styles.** Only Advanced draws the principled surface,
  bump, normal map and cut-outs. Should Texture or Default show cut-outs?

## Preset browser Stage 1 (preset-browser-2026-11-11) — 2026-11-11 (branch claude/project-thread-ljnnlt)

Built on the spec's defaults: BQ-01 F6, BQ-02 thumbnails, BQ-03 highest
resolution found, BQ-04 AO multiplies base colour, BQ-06 roots reorder by
drag. BQ-05 (starter pack) not built. Provisional readings:

- **PB1-Q01** PB-107 "file watcher": the shown folder and every root are
  looked at every 1.5 s (modified time and entry count), not an OS watcher,
  so no new dependency. Is a second or so of delay fine?
- **PB1-Q02** PB-110 material dropped on geometry. **Changed by Jason
  (2026-11-11, branch browser-views):** the drop targets the material tag
  of the polygon under the pointer: every polygon of that mesh with that
  tag. While dragging, only those polygons highlight (replaced, not added
  to, as the pointer crosses into another tag; cleared off the views or on
  Esc), the views don't select or run tools under the drag, and the scene
  doesn't change. On release the material goes INTO that tag's existing
  material group, replacing the group's contents (the group keeps its
  name, mask, blend and opacity), one undo step; polygons keep their tags
  and no new group is made. A tag with no group yet gets one, named after
  the tag. A second drop on the same group replaces the first. Same for
  material presets and texture sets, and the same replace rule as loading
  into a selected group (`preset.do target:`, Shading panel). The old rule
  (selected polygons else whole mesh, new tag per drop) is gone; a mixed or
  empty polygon list from a script still gets a new tag.
- **PB1-Q15** (provisional) A drop replaces EVERYTHING in the target group:
  layers added by hand can't be told from ones an earlier drop or load put
  there. A group's tag shared with other meshes changes them too (it's the
  same material group).
- **PB1-Q03** CD-21 opacity map: drives Dissolve, inverted (white = solid).
- **PB1-Q04** CD-25 height map: Displacement effect, with the set's material
  displacing 1 cm (new materials displace 0). Or Bump instead?
- **PB1-Q05** CD-13 HDR: the camera-visible environment's top Image Map layer
  takes the picture, else a new Spherical Image Map layer goes on top; other
  layers stay. A scene with no environment gets one.
- **PB1-Q06** CD-26 Save as Material Preset… saves next to the texture set
  (numbered when taken), not into a chosen folder. Should it ask for a
  folder, or always use the Presets root?
- **PB1-Q07** CD-21 a packed ARM/ORM map together with separate AO,
  roughness or metal maps: the separate maps win; the packed one fills only
  what's missing.
- **PB1-Q08** PB-113 thumbnails rendered for presets found in a folder go to
  the cache only; the picture is written into the file only for presets
  saved through the browser, CD-26 saves, and Replace Thumbnail.
- **PB1-Q09** PB-26 search: names only (Stage 1 has no tags), in the shown
  folder and everything below it (up to 20,000 entries), case-insensitive.
- **PB1-Q10** CD-12 double-clicking a picture opens it in the Render window's
  image viewer.
- **PB1-Q11** PB-120 drag to save: from Item List and Shader Tree rows. The
  3D viewport has no drag-out gesture for items, so that route isn't built.
- **PB1-Q12** Icons: kind letters on placeholders and list rows until the icon
  generator has recipes for the browser's kinds (needs Jason's review).
- **PB1-Q13** PB-14 (collapsing the tree with Back/Forward/Up/New Folder
  buttons) isn't in Stage 1's PB-102 layout and isn't built; New Folder… is
  on the folder right-click menu.
- **PB1-Q14** PB-114 ball: hole 35° half-angle, wall 10% of the radius,
  textures repeat 3 times around and 1.5 times hole to pole; 160 px renders
  at 24 samples, sun from the upper left, grey world.
  **Changed by Jason (2026-11-11, branch browser-views):** the big
  camera-facing hole made thumbnails stare into the cavern. Now a glossy
  ball fills 88% of the picture height with a small opening (19°
  half-angle) up and to the right, turned partly away from the camera, so
  only the cut edge and a sliver of the inside show. Rendered over a clear
  background with a shadow-catcher floor, laid over a soft grey vertical
  gradient (no floor line); a big soft area key from the upper left and a
  dimmer fill from the right. Environment presets share the ball. Cached
  renders carry a look number (`r2-`), so old ones are redone; files with
  an embedded picture keep it until re-rendered
  (`operandi-app --rerender-thumbnails <folder>`). Light strengths are
  tuned by eye, provisional.
- **PB1-Q15** CD-21 a map named only `_mask` (Poly Haven `*_mask_1k`):
  provisional, not read as opacity and left unused. Poly Haven uses it
  for weave cut-outs (hessian_230, rough_linen) and for paint/wear masks
  (wood_peeling_paint_weathered), and wiring the paint mask to Dissolve
  made the wood see-through. Should `mask` ever mean opacity (e.g. only
  in fabric sets), or should a converter flag pick it?
- Command names: the registry thread's (2026-11-11); `pbview.*`, `file.*`
  and `layout.window` are recorded in the registry test until their lines
  reach `specifications/command-names` (waiting on Jason).

## USD export stage 1 (branch usd-io, 2026-11-11)

Not Modo questions: USD is Operandi's own format choice, following
Blender's exporter. Recorded so they get a look before import maps them back.

- **USD-Q01** Boundary Rule → `interpolateBoundary`: Crease → `edgeAndCorner`,
  Smooth and Crease Edges → `edgeOnly`. Crease Edges has no USD equivalent;
  is `edgeOnly` the right fallback?
- **USD-Q02** UV Boundary Rule → `faceVaryingLinearInterpolation`: Linear
  UVs → `all`; Crease → `cornersPlus1`, Smooth → `none`, Crease Edges →
  `boundaries`. A guess at the closest OpenSubdiv rule.
- **USD-Q03** Sub-D (Subdivs) polygons are written as Catmull-Clark, the only
  smooth scheme USD has, so they look slightly different elsewhere.
- **USD-Q04** Negative edge weights (softening, EW2-04) and polylines/curves
  aren't written; polylines could become BasisCurves later.
- **USD-Q05** An image layer blended by its own alpha over a constant below
  it is written as the plain image (exact for opaque pictures).
  UsdPreviewSurface has no per-pixel blend.
- **USD-Q06** Inputs UsdPreviewSurface can't express (procedural patterns,
  gradients, Texture Locator projections other than UV, mixes of several
  images) are written as their average value (`Expr::eval`, patterns at
  0.5), with a note. Baking them to pictures would be a later step.
- **USD-Q07** One Material per tag subset: when parts or selection sets shade
  polygons of one tag differently, the subset takes its first polygon's.
- **USD-Q08** Light power in USD units, from what Operandi gives Cycles: a
  point/spot/photometric light of power 4πI is written as intensity 4I with
  `normalize` on (Blender writes P / π); area lights as their radiance with
  `normalize` on; directional lights as the Cycles sun strength / 4 and
  angle = Spread Angle / 2 (both as Blender writes them); cylinder and dome
  lights as their radiance. Checked only against the formulas, not against
  another renderer's picture.
- **USD-Q09** The environment is baked to a 256 × 128 lat-long `.hdr`
  (OpenEXR's lat-long convention, which UsdLux names: longitude 0 at +Z,
  +π/2 at +X, top row +Y), from the environment rays of kind Indirect see.
  Image layers on it lose detail at that size. USDZ viewers may not read
  `.hdr`.

## Remesh (branch remesh, 2026-11-11; no behaviour spec yet)

Built from the coordinator's brief and the registry thread's names; every
default below is provisional until the reference is measured.

- **REM-Q01** `sculpt.remesh density` (0–1, higher finer): mapped to 16 voxels
  along the mesh's longest side at 0 and 640 at 1, growing geometrically
  (default 0.5, about 101). What does the reference's density mean, and its
  default?
- **REM-Q02** `mesh.retopo` defaults: Polygon Target 5000, Relative 25 %,
  Crease Angle 0 (off), Boundary Align on, Vertex Normals on, New Mesh off.
  EDIT.md lists Poly Count Mode Relative / Absolute / **Adaptive**; Adaptive
  isn't built (no reading of what it does).
- **REM-Q03** Retopo Edge Lock / Retopo Edge Flow and the density weight map
  (EDIT.md): QuadriFlow only follows creases and borders, so these guides
  aren't built. `enableSymmetry` and `quadsOnly:false` are refused;
  `scalingStrength` and the other symmetry arguments aren't accepted.
- **REM-Q04** What remesh keeps: material = the mesh's most used one; UV maps
  kept by name but empty; parts, selection sets, morphs and corner data
  dropped. Voxel Remesh carries point weight and colour maps from the
  nearest old point (`keepMaps`, Operandi only). Locked or hidden components
  refuse the command.
- **REM-Q05** Blocks / Smooth / Sharp (`operandi.remesh`, no reference
  command) run on Operandi's grid engine with octree sizing (2^depth cells
  over the longest side / scale); Blender's dualcon was tried and dropped
  (broken surfaces; Jason agreed 2026-11-11). Defaults: Smooth, depth 6,
  scale 90 %, sharpness 1, Remove Disconnected off, threshold 100 %.
- **REM-Q06** Button places: Edit tab after a divider (EDIT.md §1):
  Automatic Retopology... and Remesh...; Sculpt tab: Remesh... (opens on
  Voxel); Geometry menu: both.

## OpenPBR and MaterialX (branch openpbr-materialx, 2026-11-11)

Spec: `docs/specs/openpbr-materialx/` (the slot check overrides the
behaviour spec). Sockets checked against the pinned Cycles
(3eb35f85, 5.3 alpha) `scene/shader_nodes.cpp` and `kernel/svm/closure.h`.

- **OPM-Q01** (slot check) subsurface has its own `subsurface_color`.
  Cycles scatters with Base Color, so a new "Subsurface Albedo" channel
  leans Base Color toward it by Subsurface Amount (exact at 0 and 100 %).
  The existing "Subsurface Color" channel is Cycles' Subsurface Radius, so
  its OpenPBR identity is `subsurface_radius_scale`.
- **OPM-Q02** (slot check) dispersion: the pinned Cycles has
  "Transmission Dispersion Scale" and "Transmission Dispersion Abbe
  Number" sockets, the same two inputs as OpenPBR, so no conversion; the
  spec's "Dispersion" socket doesn't exist. Material "Dispersion" (stored
  only until now) is the scale and now renders on see-through glass;
  scenes that set it will look different (the only OPM-T1 exception;
  accepted by Jason 2026-11-11 as a deliberate look change).
- **OPM-Q03** (slot check) also built: `transmission_scatter_anisotropy`
  (Volume Scatter anisotropy), `coat_roughness_anisotropy` (stored; no
  Cycles form), `geometry_coat_tangent` (no Cycles form). coat_color is
  Cycles' "Coat Tint".
- **OPM-Q04** (slot check) thin film: micrometres in Operandi and OpenPBR,
  × 1000 to Cycles' nanometres.
- **OPM-Q05** (slot check) gltf_pbr: Operandi's own table from the glTF 2.0
  material model (materialx/import.rs `gltf_inputs`). Choices: sheen
  colour's brightest channel is fuzz weight; thickness 0 with transmission
  is thin walled; without a volume, transmission is tinted by base colour;
  occlusion multiplies the base colour like a texture set's AO;
  `alpha_mode` OPAQUE ignores alpha, MASK is read as plain opacity.
- **OPM-Q06** (slot check) target OpenPBR 1.1.2; the bundled node
  definition is checked against Operandi's table by a test.
- **OPM-Q07** Spec corrections from the socket check: Cycles has "Thin
  Wall" (an int socket), so `geometry_thin_walled` isn't an approximation:
  it sets Thin Wall, and a see-through part becomes a thin principled glass
  with no interior volume. "Coat Normal" exists but isn't driven: the coat
  follows the surface's relief by Clearcoat Bump, so only a coat normal
  equal to the geometry normal imports; a separate coat normal map has no
  layer (refused as textured). Coat Roughness default is 0.03 in Cycles
  (always given explicitly, as before).
- **OPM-Q08** `thin_film_weight` between 0 and 1: Cycles has no film
  coverage, so any weight above 0 is the whole film (note on import).
- **OPM-Q09** *Decided (Jason, 2026-11-11):* 1 nit of
  `emission_luminance` = Cycles Emission Strength 1
  (`openpbr::STRENGTH_PER_NIT`), as MaterialX's reference implementation
  emits colour × luminance. Light-bulb examples (10 000 nits) burn out in a
  default studio; that's expected.
- **OPM-Q10** `coat_darkening` below 1 is stored only: Cycles always
  darkens under the coat (note on import).
- **OPM-Q11** *Decided (Jason, 2026-11-11):* new materials start at
  OpenPBR 1.1.2's own defaults (Coat IOR 1.6, Albedo from Diffuse off with
  the 0.8 grey albedo), so a scene with an edited shader tree saves as
  native v41. Materials in older files read the pre-v41 values (Coat IOR
  1.5, albedo from Diffuse Color) and render as before; an older file's
  untouched default tree still counts as the default tree (not written).
- **OPM-Q12** Index of refraction and Specular Amount: Cycles reflects
  with IOR 1.5 at level Specular Amount, or with the Refractive Index at
  level 0.5 when Use Refractive Index is on. Import: `specular_ior` 1.5 →
  Amount = weight / 2; another index → Use Refractive Index on (weight
  other than 1 noted); see-through with weight ≠ 1 → refract with the
  index and reflect at the level with the same reflectance straight on,
  unless that's more than IOR 1.5 can give (diamond). Export writes the
  reverse. Refraction Roughness has no OpenPBR input (exports note it when
  it differs from Roughness).
- **OPM-Q13** Units: OpenPBR lengths without a `unit` are centimetres
  (its spec); Operandi stores metres. standard_surface lengths and the
  `displacement` node's scale have no unit either and are read as
  centimetres too, so ambientCG's scale 1 is 1 cm (what a texture set
  gets). Exports write centimetres without a unit.
- **OPM-Q14** ACEScg (and other wide-gamut) colour constants are converted
  to linear Rec.709; values outside it are clamped to 0..1 with a note (7
  of the 83 OpenPBR examples, all metals). Pictures aren't converted: a
  colour map whose `colorspace` isn't sRGB-encoded gets a gamma
  approximation and a note.
- **OPM-Q15** Thin-walled or filmed see-through surfaces with index 1.0
  (the soap bubble) render through the principled glass so the film
  shows; in the flat studio light the bubble is very faint.
- **MX-Q01** Simple chains, read wider than MX-03 so exports read back
  (MX-06): `multiply` by a second picture (a Multiply layer), `mix` of a
  picture over the rest by a constant (a layer at that opacity), `invert`,
  and multiply by ±1 then add 0/1 per channel (a DirectX normal's green).
  Anything else is refused whole with "needs Node Material (not yet
  supported): <node> (<category>)".
- **MX-Q02** Import notes don't refuse: UV offsets (place2d offset,
  tiledimage uvoffset) are left out; normal map scale other than 1 is left
  out; UDIM files are refused.
- **MX-Q03** *Decided (Jason, 2026-11-11):* a .mtlx beside a texture set
  with an AO map (or a packed AO) gets the set's AO Multiply layer on the
  base colour, by the texture-set loader's own rule
  (`TextureSet::add_ambient_occlusion`, `ao_maps`), in the Preset Browser,
  its thumbnails and the starter-pack converter. ambientCG materials now
  have the same layers as their texture sets.
- **MX-Q04** Export losses (listed, the Material's value stands in):
  procedural, gradient, constant and other non-image layers; blends other
  than Normal (any opacity) and Multiply (100 %); masks and nested groups;
  image gamma/contrast/low-high/clamp and Alpha Only; projections other
  than UV are written as UV. A float picture is written as `image` type
  float (MaterialX reads its first channel; Operandi used luminance).
- **MX-Q05** Multiple materials in one .mtlx: the first is imported (the
  browser and File > Import). A chooser isn't built.

## Material row greying (branch material-greying, 2026-11-12)

Jason's UIC-92 calls (2026-11-12): older Material rows grey in place like
the OpenPBR ones, and a row a texture layer drives stays live. The rules
are one table (`openpbr::GATES`), read from the render translation
(`shader_eval`), not from a spec. Provisional:

- **MG-Q01** Rows whose controlling row sits *below* them stay live
  (UIC-93 needs the control above): Specular Color (Specular Amount),
  Transparent Color (Transparent Amount), Subsurface Color (Subsurface
  Amount), Luminous Color (Luminous Intensity), Specular Amount and
  Refractive Index (Use Refractive Index). Should the rows move, or stay
  live?
- **MG-Q02** "Driven by a texture" reads as: an enabled texture layer
  with that Effect in the material's group or any group around it,
  wherever it sits there. Both the row's own channel and its controlling
  row's channel count, so a texture on Clearcoat Amount keeps every coat
  row live, and a texture on Clearcoat Roughness keeps that row live.
  Masks and layer opacity aren't considered.
- **MG-Q03** The coat rows grey only when nothing makes a coat: Clearcoat
  Amount 0 % and (Match Specular on, or Reflection Amount or Reflection
  Fresnel 0 %), since Reflection feeds the coat when Match Specular is off.
- **MG-Q04** Stored-only rows (Exit Color, Subsurface Depth/Samples,
  Rounded Edge Angle, Round Same Surface Only, Clip Value, Matte Color, GL
  Color) grey by their switch though nothing renders them yet. Blurry
  Reflection, the Volume rows and Thin Walled aren't gated.

## Operandi Library pack (F1, 2026-11-11, branch library-pack)

Not a Modo behaviour: Jason's own feature. These are choices for Jason.

- **LP-Q01** Basics sit after the user's roots (not first) so the Presets
  folder stays the first root (PB-Q2); a fresh install shows Basics first.
  They're written to the data folder's Cache/Basics at each start, not into
  the app bundle, so they work the same on every platform. *Provisional.*
- **LP-Q02** Packs found in the library folder are added as ordinary roots
  once each; removing one keeps it removed (like the Presets root). Should a
  pack instead always show, like Basics? *Provisional.*
- **LP-Q03** With two versions installed, the older root's name gets
  "(newer: <version>)" and its tooltip says where the newer one is; nothing
  is hidden or removed. *Provisional.*
- **LP-Q04** The "Get the free Operandi Library…" / "Locate Library
  Folder…" offer shows above the contents of whatever folder is shown while
  no pack root is there, not only in an empty folder. *Provisional.*
- **LP-Q05** (2026.11.5) Starter meshes carry real-world UVs (one UV unit
  is one metre) so tiling materials keep one texel size on every shape;
  islands overlap, so they aren't ready for baking. A packed 0–1 layout
  instead, or as a second UV map? *Provisional.*
- **LP-Q06** (2026.11.5) Mesh presets live in the pack's own top folder
  Meshes/<Category>/<Name>.oppreset (one file each, thumbnail inside), not
  under Starter/, so the browser shows them apart from materials. The mug
  handle's two open ends sit hidden inside the wall (the only open edges).
  *Provisional.*

## Original-compatible A1/A7 — 2026-11-11 (branch original-compat)

- **OC-Q01 (A1)** Handles with the switch on, viewport resized at the
  default view: provisional, the handle keeps today's screen size in any
  viewport height (only zoom changes it). A true world size would also
  shrink it in a shorter viewport (e.g. quad view). Which?
- **OC-Q02 (A1)** The brief says the `-`/`=` keys multiply the handle size;
  Operandi has no such keys yet, only the Handle Scale preference. Add
  them (on or off the switch)?
- **OC-Q03 (A7)** Ctrl+Shift+click on a Mac with the switch on: provisional,
  a right-click like Ctrl+click (macOS convention), not a range select.

## HDRI environments and environment presets — 2026-11-12 (branch hdri-environment)

- **HDRI-Q01** Several enabled Image Map layers under one environment: the
  view draws (and lights with) only the top one, as Normal blend; Cycles
  renders them all with their blend modes. *Provisional.*
- **HDRI-Q02** Glossy reflections in the Advanced style read one blurred
  copy of the picture for every roughness (no per-roughness mips).
  *Provisional.*
- **HDRI-Q03** Layer Opacity scales the picture's light in the Advanced
  style (over no light, not over the gradient's). *Provisional.*
- **EP-Q01** A .png/.jpg dropped on the Shader Tree's Environments section
  should become an environment image; not built yet (the Shader Tree
  doesn't take Finder drops). Today it is a Backdrop / image entry.
- **EP-Q02** Replace removes an earlier preset's item group with
  everything in it; a hand-made item the user parented inside that group
  goes too. Items parented under it from outside move to the root where
  they stand. *Provisional.*
- **EP-Q03** `preset.saveAs kind:environment filename:…` (no copy option on
  the command) keeps picture paths as they are (relative only when
  beside or under the preset); the dialog's Copy Images is the only way
  to copy. *Provisional.*

## Material editing (branch material-editing, 2026-11-12)

- **MAT-ED-Q01 Dispersion preset units.** *Decided by Jason 2026-11-12:*
  the preset values (Diamond 0.044, Zircon 0.038, Quartz 0.013, Silica
  Glass 0.01, Flourite 0.007) are physical dispersion, the gemological
  B−G figure (n at 430.8 nm − n at 686.7 nm). Picking one sets Dispersion
  (OpenPBR transmission_dispersion_scale) to 100 % and Abbe Number to
  (IOR − 1) / (k × figure), with the material's current Refractive Index
  and k = (n_F − n_C) / (n_B − n_G) ≈ 0.585 from a two-term Cauchy fit
  (`operandi_core::dispersion`; diamond gives Abbe ≈ 55, quartz ≈ 72).
  The list shows the figures; (new) saves the material's figure computed
  back from IOR, Abbe Number and Dispersion. With an index of 1 or less a
  pick is refused with a status message.

## LXO legacy (format 4.x) leniencies — 2026-11-12 (branch lxo-legacy-relax)

Asked for by Jason; these relax LEG-10/LEG-12 and are *provisional* until
the spec side confirms them.

- **LXO-LEG-Q01** Rotation orders other than ZXY: provisional, the order's
  letters are read as the order the axes apply ("zxy" = Z, then X, then Y,
  as for ZXY today) and converted once to ZXY angles giving the same
  orientation. Is that the reading Modo uses? Every converted mesh gets a
  per-mesh note ("rotation order XYZ converted to ZXY (same orientation)").
- **LXO-LEG-Q02** Pivot / Zero transform items in a mesh's xfrmCore
  stack. *Answered by the spec side 2026-11-15* (a pivot is a "Pivot
  Position" / "Pivot Position Inverse" pair wrapping rotation and scale; a
  Zero layer is an extra item holding the real offset while the core item
  reads 0), and rebuilt: the whole stack is composed in stack order as
  matrices (lowest ordinal applies first, as core scale 0, rotation 1,
  translation 2), every scale, rotation, translation and transform item
  included, each rotation in its own order. A core-only stack reads exactly
  as before. A pivot pair ("N" and "N Inverse") cancels when the pivot is
  zero or nothing between them rotates or scales. A result that is an exact
  TRS becomes the item transform (note "placed by its whole transform
  stack"); one with shear keeps its position as the transform and bakes the
  rest into the points (note). An item of another type, an unknown channel
  or a non-numeric value refuses the mesh's group with a plain reason, and a
  stack item's own problems (e.g. animation) now refuse it too. *Still
  provisional:* the inverse item is composed as its channels read (they
  hold the inverse offset); a `transform` item reads only pos/rot/scl
  channels (T·R·S, empty = identity). Confirm both.
- **LXO-LEG-Q03** A LINK whose target is above 0x7FFFFFFF: provisional, a
  link to nothing, left out with a count. `xfrmLocal` links still refuse.
- **LXO-LEG-Q04** Procedural modelling (`deformers` / `chanMods` links
  between a modeled item and a non-modelling one): the mesh's group is left
  out with a plain reason naming the ops; a file with no other mesh to
  import is refused with one plain sentence. Item-group membership
  (`itemGroups`) no longer blocks a mesh.

## Matcaps in the preset browser (MATCAPS MC-01..11) — 2026-11-12 (branch claude/project-thread-i94glh)

Spec: preset-browser-gaps-2026-11-12/MATCAPS.md. *Provisional* readings:

- **MC-Q01 (MC-02)** One click with Alt to preview a matcap until the
  pointer leaves its tile (the spec's nice-to-have) is not built.
- **MC-Q02 (MC-08)** "The view's status line" is the app's status line:
  it says "Matcap not found: name" once when the active view's matcap file
  can't be read. The view draws Clay Studio; the saved choice keeps the path,
  so the matcap comes back if the file does.
- **MC-Q03 (MC-02/03)** Double-click and drag switch only the view's
  *active* group to the Matcap style (as `view3d.shadingStyle Matcap`), not
  the inactive meshes' style. Right?
- **MC-Q04 (MC-06)** Use in All 3D Views runs
  `operandi.view3d.matcap name:custom image:<path> all:1`; `all:1` also
  switches every view's active group to the Matcap style. The `all:`
  argument is new on an Operandi-only command (registry thread told).
- **MC-Q05 (MC-05)** The Matcap menu is Viewport Properties ▸ Matcap ▸
  Image: built-ins, then a Library submenu (one submenu per folder below a
  "Matcaps" folder, "Matcaps" for pictures right in one), then Custom Image
  with its path field and Load Image… (the spec's Load File…). The list is
  found in the background from every root, again when the roots change,
  after Refresh and every 30 s.
- **MC-Q06 (MC-01)** In a Matcaps folder every picture is a matcap, HDR
  and EXR ones too, and none are read as texture-set maps. A `name.txt`
  beside a picture is its note (MC-11) and isn't listed as a file.
- **MC-Q07 (MC-07)** Pictures are read at 256×256 (the size the Matcap
  style has always used), so the 1024 cap never comes into play.
- **MC-Q08 (PG-01/MC-09)** The five starter folders are made only when the
  Presets folder doesn't exist or holds nothing, so an existing install
  doesn't get them and a folder the user deletes isn't put back.

## Preset browser gaps P1, meshes (PG-01..08) — 2026-11-12 (branch claude/project-thread-i94glh)

Spec: preset-browser-gaps-2026-11-12/GAPS.md. *Provisional* readings:

- **PG-Q01 (PG-02)** Settled (Jason, 2026-11-12): saving a mesh is a drag
  from the Item List onto a browser folder only; no drag out of a 3D view
  (a drag in the view moves things). The menu says Save Mesh Preset… when
  every selected item is a mesh.
- **PG-Q02 (PG-04)** Centre on Ground is a check box in the save dialog
  (`preset.saveAs centerOnGround:`), not in the small drop menu, which keeps
  its one-click entries. It applies to item and assembly presets.
- **PG-Q03 (PG-06)** Shift-drop rests the bottom middle of what arrives
  (box centred on X/Z, lowest Y) on the first mesh surface under the
  pointer, else on the work plane. It isn't turned to the surface's normal.
  The drop isn't yet replayable as its own command (the map's
  `preset.dropMesh` / `preset.dropScene` would carry the position): which
  arguments do they take?
- **PG-Q04 (PG-06/07)** Double-clicking a model file now adds it at the
  origin (it used to open it as its own scene); right-click ▸ Open does
  that. Double-clicking a scene still opens it.
- **PG-Q05 (PG-07)** `scene.importReference` now adds to the current scene
  (registry ruling), so File ▸ Import… does too; Open… still opens a tab.
  An imported Operandi scene brings its camera and light too unless
  `cameras:false lights:false`. A Modo scene's groups the reader refuses
  are left out with the report in the status line, without the partial
  import question.
- **PG-Q06 (PG-05)** Scene and model thumbnails render the file's meshes
  alone in the default grey material; files over 256 MB keep the 3D glyph.
- **PG-Q07 (PG-08)** A mesh dropped on a mesh is always a new item; nothing
  merges (as before).

## Modo content folder — 2026-11-12 (branch claude/project-thread-ssvwc2)

Spec: BEHAVIOR-MODO-CONTENT-FOLDER (MC-01..40), stages 1 and 2. Black-box
only (Q-MC1): nothing below was worked out from a Modo file.

- **MC-Q01** `DESC` byte layout isn't published beyond "primary item type
  and a user-friendly description". Provisional: two zero-terminated UTF-8
  strings (LWO2's S0), type then description; anything else is ignored.
  The kind word comes from plain words in the type ("material",
  "environment", "assembly", "mesh"…), else from the FORM type. What are
  the item type names, so "Material preset" etc. are exact?
- **MC-Q02** `PRVW` is "compressed image data, flag 1 = PNG" but its header
  isn't published. Provisional: the PNG is found by PNG's own 8-byte
  signature within the chunk's first 64 bytes; no signature, no picture.
  What is the header (size, flags) before the image?
  *Update 2026-11-14 (lxoxide, see docs/DEPENDENCIES.md):* the header is
  u16 width, u16 height, u32 pixel format, u32 flags (1 = PNG); the PNG
  starts at byte 12. The signature search stays as a fallback.
  Also: a mesh preset's layers with no mesh ITEM now import as meshes in
  place, named after the layer (provisional).
- **MC-Q03** `THUM` ("uncompressed 8-bit RGBA") has no published width or
  height, so it isn't used: MC-30's order is PRVW → kind icon for now.
  Where are THUM's dimensions?
- **MC-Q04** `.lxl` FORM type is marked `[?]` (LXPM?). Provisional: a mesh
  preset may be `LXPM` or `LXOB`; anything else (e.g. `LXPR`) is refused
  with its type named. Its body is read exactly like a scene; unknown
  chunks are skipped by size, and a preset with no readable mesh is
  refused.
- **MC-Q05** MC-01 route 2 (drop into the Library folder): provisional,
  every folder in the library folder becomes a root once, like packs, so
  one the user removes doesn't come back. Symlinks are followed; Finder
  aliases and Windows shortcuts are not (they need OS APIs) — needed?
- **MC-Q06** "Under the pointer" for a dragged mesh preset: provisional,
  PG-06's rule as for model files: a plain drop lands at the origin, a
  Shift drop on the surface or work plane under the pointer. MC-11 says a
  drag lands under the pointer; is the Shift rule fine for .lxl?
- **MC-Q07** MC-13 "kit plumbing": only the listed extensions (.lxm .py
  .pl .lua .cfg) are hidden; folders are never hidden, since kit folders
  aren't defined. Should any folder names be hidden?
- **MC-Q08** Copy Into Library copies into `<library>/<folder name>`; a
  folder of that name there asks first (overwrite replaces files of the
  same name, keeps others). Hidden OS files are left behind.

## Environment Texture Locator — 2026-11-13 (branch env-texture-locator), provisional

From the coordinator's brief (no spec folder yet).

- **ETL-Q01 XYZ composition.** *Decided (Jason, 2026-11-13).* XYZ turns X
  first (Rz·Ry·Rx on column vectors), matching IT-O-H3's measured matrix,
  so all six orders turn their first letter first. Native v44: items saved
  in the old Z-first XYZ with two or more non-zero angles open in ZYX,
  which turns the same way (one angle or none turns alike in both and stays
  XYZ). LXO and USD use the corrected XYZ.
- **ETL-Q02 Light Probe.** *Answered (research, 2026-11-13).* Spherical is
  latitude-longitude; Light Probe is a mirror-ball photo unwrapped to the
  whole sphere. Environment image layers start Spherical about Y in ZXY;
  pre-v44 environment locators saved as Light Probe (drawn as
  latitude-longitude then) open as Spherical. Light Probe draws in the
  viewport and Cycles through `environment_probe_uv`.
- **ETL-Q02b Mirror ball or angular map.** *Open, provisional.* Light
  Probe is built as a true mirror ball (the photo taken along -Z of the
  locator frame, centre = +Z, rim = -Z, +Y up). Is it that, or the angular
  (Debevec) map, where the radius is linear in angle? And which way does
  the ball face by default?
- **ETL-M1 Mirrored sky.** *Decided (Jason, 2026-11-13).* The
  latitude-longitude sky read mirrored (text backwards) in the viewport
  and in Cycles alike; u now grows toward -X from +Z, so looking down -Z
  the picture reads left to right. Every scene's sky flips to the correct
  orientation.
- **ETL-Q03 Mapping on the sky.** Horizontal/Vertical Repeat, Repeat U/V,
  U/V Tile Offset, UV Rotation, Texture Offset Strength and Falloff are
  saved but don't change the sky in the viewport or Cycles (Cycles would
  need a mapping step and an ABI change). Should they apply to the
  latitude-longitude coordinates, and in which order (repeat, offset,
  rotation)? Falloff and Position have no meaning for a picture at
  infinity; Auto Size is greyed.
- **ETL-Q04 Starting order.** New environment image locators start in ZXY
  (brief). Locators from older files keep their order (XYZ), so a picture
  turned about X or Z doesn't move when it opens.

## SPEC-EDGE-EXTEND-ROTATE-SCALE v1 (2026-11-13, branch claude/project-thread-js1l2r)

- **EXT-RS-Q1 Order and centre.** *Provisional.* Offset (global and
  Local) first, then Rotate (X, then Y, then Z, as Transform) and Scale
  along the action axes about the action centre moved with the Offset, so
  EXT-RS-02 gives a 1 m square 1 m lower. Local offsets are scaled and
  turned with the edge. With Segments > 1 each row takes its share of every
  value (Rotate sweeps round in steps). Is that the order, and do the
  middle rows interpolate like this?
- **EXT-RS-Q2 Scale drag.** *Provisional.* Scale squares use Transform's
  feel (1 % per pixel, Ctrl scales all three) and stop at 0.1 %; typed
  Scale may be any finite value, negative included. Is negative scale
  allowed in the original?
- **EXT-RS-Q3 Local offsets on fins.** *Provisional.* An interior edge's
  Inset/Shift use its first neighbouring polygon (the one the fin is built
  off, EXW-Q02). Positive Inset moves the new edge outward, away from its
  polygon, in the polygon's plane; where two selected edges meet the point
  is mitred. Is outward the positive direction?
- **EXT-RS-Q4 Make UVs.** *Provisional.* U or V runs 0..1 outward across
  the new strip (row k at k/Segments); the other direction runs 0..1 along
  each connected run of selected edges by length. The values go into the
  "Texture" map (made if the mesh has no map; else the first map). Other
  maps get unassigned corners. What are the real values and map?
- **EXT-RS-Q5 Defaults.** *Answered (research, spec v1.1, 2026-11-13).*
  Move handles on; Rotate, Scale and Local off; Plane None; Haul Global;
  Make UVs U (the reference name map was right).
- **EXT-RS-Q6 Integer values.** *Answered (research, spec v1.1).*
  planeHandle none=0, move=1, scale=2; hauling global=0, local=1; uvs
  none=0, u=1, v=2. tool.attr takes the integers and the readable names.
- **EXT-RS-Q7 Shift+click.** *Provisional.* Shift+click (or Shift+drag)
  keeps the extension and starts one from the new outer edges with the same
  values, shown at once; the action frame is resolved again on the new
  edges. Each extension is its own undo step after Drop (as Bevel's TA-07).
  Shift+click before any extension does nothing and says why.

## Selection falloff and Flex (tutorial-gaps-2026-11-13, SPEC-SELECTION-FALLOFF-FLEX)

Built from the spec's reading of the guide; nothing measured yet. Each is
the provisional choice in `falloff.rs` / `deform.rs`.

- **SF-Q1 Ring weights.** *Open, provisional.* Points carry the weight;
  ring r (edges in from the border) weighs `shape(r / Steps)`, so with
  Steps 2 and Linear the border is 0, the next ring 0.5, the next 1
  (SF-01). Is the ramp offset by half a ring, or carried by polygons?
- **SF-Q2 Edges and points.** *Open, provisional.* Polygon mode: the border
  is the selected polygons' points shared with an unselected polygon (a
  mesh's open edge is not a border). Vertex and Edge modes: the border is
  the selected points (points of selected edges) with an unselected
  neighbour along an edge. A single selected edge loop is all border and
  doesn't move. How does the reference count rings for edges and points?
- **SF-Q3 No border.** *Open, provisional.* A selection with no border
  (a whole closed mesh, or nothing selected) weighs 1 everywhere.
- **SF-Q4 Inward or outward.** *Open, provisional.* The ramp runs inward
  from the border (the guide), not outward from the selection.
- **SF-Q5 Defaults.** *Registry, 2026-11-13.* Steps 2, Shape Smooth (shape
  index 3), In/Out (p0/p1) 0. The spec's table said Linear; the registry's
  attribute default wins.
- **SF-Q6 Flex's axis outside Polygon mode.** *Open, provisional.* Flex sets
  Selection Border centre and Selection axes (`center.border` +
  `axis.select`). Selection axes need selected polygons, so in Vertex and
  Edge modes Flex keeps the current axis. Dropping Flex gives back the
  action centre (and falloff) from before it started, even when the user
  changed it while Flex was open.
- **SF-Q7 Right-drag.** *Open, provisional.* With a Selection falloff under
  a live tool, a right-drag anywhere changes Steps (one per 20 px right,
  fewer to the left, never below 1) instead of the tool's own right-drag
  (Transform's scale). Flex blends positions like other falloffs
  (FAL-M-02), not angles like Twist (TW-Q1).

## Bend (tutorial-gaps-2026-11-13, SPEC-BEND)

Built from the spec's reading of the guide; nothing measured yet.

- **BD-Q1 Item mode.** *Open, provisional.* Bend refuses Item mode, like
  Push and Smooth.
- **BD-Q2 Beyond the spine's end.** *Open, provisional.* Points past the
  end turn rigidly with it (they continue along the end's tangent).
- **BD-Q3 Arc or ramp.** *Open, provisional.* A true arc: the spine keeps
  its length and curls onto a circle of radius Length / Angle; points
  beside it keep their offset along the bend axis and move in or out with
  the radius.
- **BD-Q4 Bend direction.** *Open, provisional.* The bend leans toward the
  action axis most square to the spine (ties X, then Z, then Y), so a spine
  up Y bends toward +X with World axes; Element or Work Plane axes steer it.
  The registry's Two Handles isn't a reference attribute and isn't built.
- **BD-Q5 Starting spine and base.** *Open, provisional.* On activation the
  base sits at the bottom of the target's box along the work plane's up
  axis (centred across it) and the spine runs to its top. A press off the
  model moves the base to the work plane there; a press on the model only
  bends. Drag left/right = 0.5° per pixel; the spine's end square drags in
  the view plane (Ctrl: one world axis). Base X/Y/Z are Operandi fields
  (`operandi.base0..2`); the reference places it with the action centre.
- **BD-Q6 Falloff and Symmetry.** *Open, provisional.* A falloff blends
  positions (FAL-M-02). Symmetry isn't applied yet.

## Constrain to Background (SPEC-BACKGROUND-CONSTRAINT v1, tutorial gaps 2026-11-13)

Built with Screen Axis, Vector and Point; the readings below are
provisional until the measured v2.

- **BG-Q1 Hidden polygons.** Hidden polygons of a background mesh don't
  constrain (locked ones do).
- **BG-Q2 Default mode.** Settled by MC-06 (BEHAVIOR-MESH-CONSTRAINT v1):
  Screen Axis when first turned on; later turns, and later launches,
  bring back the last options.
- **BG-Q3 Double Sided.** Off by default; off, only polygons facing the
  search count (normal against the search direction).
- **BG-Q4 Miss.** A point that meets nothing stays where the tool put it
  (BG-03), each frame afresh.
- **BG-Q5 Subdivided background.** Answered (Jason, 2026-11-15): the
  displayed smooth surface, never the cage. Smooth polygons count as the
  pieces the viewport draws at the mesh's display level; Face polygons as
  they are.
- **BG-Q6 Screen Axis line.** The whole line of sight through the point is
  searched and the hit nearest the viewer wins (orthographic: along the
  view direction; perspective: from the eye through the point). So a point
  drawn on the work plane inside the base mesh lands on its front, as the
  tutorial's Pen strips do. Is the search really one-way from the point?
- **BG-Q7 Offset direction.** Along the surface normal, toward the side the
  search came from.
- **BG-Q8 Which tools.** Live tools apply it only when they move existing
  points without adding or removing any (Move, Rotate, Scale, Transform,
  Element Move, Slide, deformers); Extrude, Bevel and other point-adding
  tools are left alone, as is Item mode. Pen constrains every point it
  places or drags. Primitives and Draw tools aren't constrained.
- **BG-Q9 Vector mode.** The line through the moved point along its own
  motion; the hit nearest where the tool put it.
- **BG-Q10 Esc.** Esc over a view clears it when no tool edit is in progress
  (no applied live tool, empty Pen draft, no Draw tool or brush) and does
  nothing else; an armed tool stays for the next Esc.
- **BG-Q11 Names.** *Answered (registry thread, 2026-11-13).* The tool is
  `const.bg` (`tool.set const.bg on`, which the Edit ▸ Snapping entry
  runs); attrs geometry, bkgOffset, negative, dblSided, handle, target.
  geometry 0–3 = Off, Screen Axis, Vector, Point (list order, spec v1.1,
  to confirm). The reference lists geometry=0 and negative=1 as defaults;
  Operandi starts at Screen Axis (BG-Q2); Negative Offset now starts on
  (MC-06, settles spec v1.1 Q9).
- **BG-Q12 Target Item.** None (every background mesh) by default; a
  target that is active or hidden constrains nothing.
- Handle Constraint (MC-05/06): a stored, saved toggle, on by default,
  shown greyed with its reason. The only curve handles are the Draw
  tools' Bezier handles, and Draw tools aren't constrained at all
  (BG-Q8), so it has nothing to act on yet.

## Mesh Constraint button and popover (BEHAVIOR-MESH-CONSTRAINT v1, 2026-11-15)

Built on branch bg-constraint-subd. The answers below are the spec's
provisional Operandi choices.

- **MC-Q1 Button label.** Provisional: while on it shows the mode's name
  ("Background"); off, "Mesh Constraint"; with Option held, "Options".
- **MC-Q2 Pipe order.** Provisional: snapping first, constraint after.
  Snapping steers the tool's input (its handle), the tool moves the
  points, then the constraint pushes the moved points onto the
  background. The tool card's `layers.constraint` says which tools take
  part.
- **MC-Q3 Snapping and constraint.** Provisional: independent switches;
  turning one on or off never changes the other.
- **MC-Q4 Esc.** Provisional: only with no tool edit in progress (no
  applied live tool, empty Pen draft, no Draw tool or brush), as BG-Q10;
  the settings are kept for the next turn-on.
- Saved settings: everything but Target Item (it names an item of one
  scene) is kept in `mesh-constraint-preferences.json`; the on/off state
  starts off each launch. Is the on state meant to survive a relaunch?
- Popover header: following UIC-181 (as the Snapping popover does) it has
  no "Mesh Constraint" title, only the pin circle; the toolbar button
  names it. The spec's row 1 lists a header title.
- Row order: the spec lists Geometry Constraint last; Operandi keeps it
  first in both the popover and Tool Properties because it greys the rows
  under it (UIC-93, controlling field first). Needs Jason's call.
## Pen Make Quads (SPEC-PEN-MAKE-QUADS, 2026-11-13)

Built on branch claude/project-thread-irzez8. Provisional readings:

- **PMQ-Q1 Wall Mode together.** *Open, provisional.* Make Quads and Wall
  Mode can't both be on: each greys the other, and the core refuses the
  pair.
- **PMQ-Q2 Facing.** *Open, provisional.* Each quad runs
  `[previous left, previous right, right, left]`, where left is the
  clicked point and right its partner. On Drop the whole strip turns to
  face the viewer, like the ordinary Pen polygon (sum of its polygons'
  normals); Flip Polygon reverses that.
- **PMQ-Q3 Undo while drawing.** *Open, provisional.* Cmd+Z takes back one
  click: a point with its partner, the triangle's point, or one
  starting-edge point. The dropped strip is one undo step.
- **PMQ-Q4 Partner when turning.** *Open, provisional (changed
  2026-11-15).* The partner is the click plus the strip's last rung (the
  last pair's left → right, as it stands, so a dragged partner carries
  on), turned by as much as the strip's left side turns at the last left
  point. The first click after the starting edge has no turn yet and takes
  the edge as drawn, so on a steady curve every rung lags by one click's
  turn. A near U-turn keeps the rung unturned. The drag that follows a
  click guesses its partner again; a later drag (after picking the point)
  doesn't move partners already made. Was: always the starting edge's
  vector, which folded quads once the strip curved (Cartoon Kid eye).
- **PMQ-Q5 Constrain to Background.** *Built (2026-11-13, changed
  2026-11-15).* The click lands on the background. Its partner's rung is
  laid flat on the surface under the click and settled onto the nearest
  surface keeping the rung's length (plus the constraint's offset), not
  pushed along the view: along the view each rung stretched over a curved
  eye and the next guess copied it. The Ctrl+click end point lands like
  any new point.
- **PMQ-Q6 After the triangle.** *Open, provisional.* A Ctrl+click
  triangle ends the strip: further clicks are refused until Shift+click
  starts a new strip or the tool is dropped. Ctrl+click right after the
  starting edge makes a lone triangle.
- **PMQ-Q7 Other polygon types.** *Open, provisional.* Make Quads works
  for Subdivision and Catmull-Clark too (the strip's polygons take that
  type); the spec names Polygons only. Vertices, Polyline and B-Spline
  ignore it (greyed).
- **PMQ-Q8 Changing mid-draft.** *Open, provisional.* Make Quads, or a
  Type change that turns the strip on or off, is refused once points are
  captured (the pairs would read as a different polygon).
- **PMQ-Q9 Numeric route.** Unchanged: four numeric points with Make
  Quads make one quad (PN06), more are refused (Q-PEN-04).
- **PMQ-Q10 Weld on drag.** The spec says dragging a strip point onto
  another welds them "as for the Pen", but the Pen's Merge isn't built
  (PEN-R02); not built here either.

## Solid Sketch phases 1–4 (spec solid-sketch v0.3) — 2026-11-13 (branch claude/project-thread-xg0zvq)

Built to the most conservative reading; each is provisional and easy to change.

- **SS-Q01** (spec Q13) Answered in spec v0.4 (2026-11-14): Level 2 bakes one Catmull-Clark step into the cage (4× the quads), marked SubD; built on branch sketch-subdiv-level2.
- **SS-Q02** (spec Q2) A node takes at most six links (root) or five (others). Past that, an extra child gets a separate closed box instead of the spec's "split the nearest face". Where should a sixth link go, and where do an end node's loops sit?
- **SS-Q03** (spec Q8b) Auto Merge merges a link's two facing loops into one (at their midpoint) when the child's facing face lies behind the parent's along the run; that gives 18 quads for three big close nodes, as counted. If merging would make a bad polygon, the cage is built unmerged. What is the real "too close" distance?
- **SS-Q04** (spec Q5) A create drag sets the half-size to the pointer's distance from the node on the work plane; right-drag scales by ×e per 100 points to the right (uniform). How does Modo map drag distance to size?
- **SS-Q05** (spec Q4) Pressing on any node selects it and a drag moves it straight away.
- **SS-Q06** (spec Q8) In Destroy mode an empty-space click does nothing.
- **SS-Q07** (spec Q9) Negative is stored (panel greyed, scripts can set it) and changes nothing.
- **SS-Q08** (spec Q10) Picking a sketch up again makes the node that was current at Drop current.
- **SS-Q09** (spec Q11) "No" starts a separate sketch and leaves the layer's polygons (the old sketch's become plain polygons once the new one is dropped). Starting on a layer with geometry but no sketch keeps that geometry.
- **SS-Q10** (spec Q12) Answered by Jason, 2026-11-13: rotate rings use the usual axis colours (red about X, green about Y, blue about Z), not the documented reverse.
- **SS-Q11** (spec Q18) Auto Align and Auto Merge ship off and remember the last values used; Subdiv Mode comes from the preset each time.
- **SS-Q12** Node frame: X along the run from the parent (the root faces its first child, else the work plane's X), Y carried from the parent by the smallest turn, then Rotate and Twist. A node's own Rotate turns its box; for the root it only spins the box about the run, since the root always faces its first child.
- **SS-Q13** Ctrl-press on a node inserts on its first link out; on a leaf it adds a child there. Ctrl+Shift is treated as Ctrl.
- **SS-Q14** "Don't ask again" on the pick-up question is kept with the dialog settings (UIC-95 memory), not yet in the Preferences window. Clearing it needs a Preferences row.
- **SS-Q15** (spec Q15) Hidden args `tension`, `material`, `part`, `uvmap`, `uvProj`, `cenU`, `cenV`, `rotU` are accepted by `tool.attr solid.sketch`, stored with the sketch and unused.
- **SS-Q16** (spec Q16) Symmetry is ignored.

## Draw point modes, repeat transform, drag snap (specs draw-edit-repeat-snap-2026-11-13) — 2026-11-13 (branch draw-edit-repeat-snap)

Built to the most conservative reading; each is provisional.

- **RPT-Q5** (RPT-Q1..Q3) The repeat memory lasts for one tool session (dropped, cancelled or re-activated: gone); the pivot follows the current action centre each repeat; the fields restart at zero after each apply as before (HND-Q03), so they show neither step nor total. Element Move isn't covered (it has no "last amount").
- **DPM-Q5** (DPM-Q2) Bezier Delete keeps the neighbours' handles as they were (no refit).
- **DPM-Q6** (DPM-Q3) Delete may take a curve down to one point; it then waits in the tool as a first point, and Drop removes it (DRW-Q08).
- **DPM-Q7** (DPM-Q4) Mode comes back as last used, per tool; a curve reopened from the selection still opens in Edit (CV-20). Not saved between app runs.
- **DPM-Q8** Insert (Bezier) needs a click within 12 px of the curve; a click on a point or handle in Insert mode picks and drags it, as in Edit.
- **DPM-Q9** Pen Add now inserts after the current point (DPM-09) instead of always appending; Make Quads strips still grow at the end. Pen Undo steps back through adds, drags (one step each) and deletes, not only the last point.
- **DSN-Q6** (DSN-Q2) No hysteresis: leaving Inner Range lets the point go.
- **DSN-Q7** (DSN-Q4) Pen has no Straight Line / Right Angle / World Axis targets in Operandi yet, so DSN-11 has nothing to act on.
- **DSN-Q8** With snapping off, a dragged or placed Curve/B-Spline/Bezier point still joins another mesh point within 8 px (the existing join for Draw Patch). Should it, given DSN-T03/T06?
## Direct Caustics and Absorption Distance — 2026-11-13 (branch whiskey-caustics), provisional

- **DC-Q01 Light kinds.** Spot, point, area and directional lights all cast
  direct caustics (Cycles' MNEE takes all four); cylinder and mesh lights
  don't. Is that the reference's set?
- **DC-Q02 Photon rows.** Operandi has no photon map: Total Photons, Local
  Photons and Walkthrough Mode are saved and shown but don't change the
  render. Cleaner caustics come from Maximum Quality. Fine, or should
  Total Photons scale samples somehow?
- **DC-Q03 Shader visibility.** A Glass surface whose Shader has Visible to
  Refraction off casts no direct caustic. Correct?
- **DC-Q04 Caustics on see-through surfaces.** A surface that casts
  (see-through Glass) receives no direct caustics; every other mesh does.
  Which surfaces receive in the reference?
- **DC-Q05 Indirect Caustics row.** The spec lists it in the Global
  Illumination tab's Caustics section, so it moved there from Settings >
  More (render item spec v1.2). Is it in both places, or only here?
- **DC-L1 Limits (not a question).** MNEE needs smooth shading on the
  glass, follows at most 6 surfaces (glass, liquid and glass again use 4
  to 6) and only refraction (no mirror caustics from lights).
- **AD-Q4 Absorption Distance 0 on a solid.** Starting state: a closed
  slab, Transparent Color 0.5 grey, index 1, Absorption Distance 0. The
  spec (AD-02) expects 0.5 through it; Operandi tints at each surface the
  ray crosses, so 0.25 (0.5 in, 0.5 out), whatever the thickness. Kept as
  it was (it changes every existing transparent material). Should a solid
  tint once?
  **Answered 2026-11-13 (render fix list F4): tint once.** Built: a
  non-thin-wall see-through surface at distance 0 passes the square root of
  a constant Transparent Color at each crossing, so a closed solid passes
  the colour once.
- **AD-Q7 Distance 0 on an open sheet or a textured colour (provisional).**
  After F4, a single-sided open sheet (not Thin Wall) crossed once passes
  only the square root of the colour (0.71 for 0.5), and a textured
  Transparent Color still tints at each crossing (0.25 for 0.5). Should an
  open sheet be treated as a thin wall, and is a textured colour worth a
  square-root node?
- **AD-Q5 Transparent Amount below 100 %.** Absorption applies to the
  light that goes through (the see-through share); the rest is the opaque
  surface. Confirm.
- **AD-Q6 Overlapping volumes.** Where liquid overlaps glass both absorb
  (Cycles adds their volumes). Confirm.

## Viewport transparency (spec: tutorial-gaps-2026-11-13 SPEC-VIEWPORT-TRANSPARENCY v1, branch `viewport-transparency`)

Built as an Operandi improvement (Jason, 2026-11-14), not a reference match.

- **VT-Q01 Which styles.** Decided by Jason 2026-11-14: Advanced only.
  Every other style (Shaded, Texture, Default, Solid, Vertex Map, Gooch,
  Cel, Matcap) draws see-through materials solid. Transparency Override
  groups (VP-80) keep their own drawing.
- **VT-Q02 Keeping clear glass readable (provisional, by eye).** A
  see-through surface reflects 6 % of a light grey facing the eye, rising
  to 60 % at its silhouette, plus a small key-light highlight (gpu.rs
  `GLASS_*`). No refraction, IOR or absorption.
- **VT-Q03 Overlaps (provisional).** Items sort back to front by the centre
  of their bounds; inside an item the far side draws before the near side.
  Interleaved surfaces of two items can sort wrong (no per-pixel sorting).
  See-through surfaces don't write depth, so the wire and selection of what
  is behind still draw.
- **VT-Q04 Setting off, or a style that draws solid (provisional).** A
  see-through surface draws opaque but leans toward its Transparent Color
  (× 0.75) by its Transparent Amount, so Diffuse Amount 0 glass is light
  grey, never black. Subdivision surfaces big enough to draw as GPU patches
  (500k+ pieces) also draw this way for now.
- **VT-Q05 The setting.** Preferences > Display > Show Material
  Transparency, on by default (spec suggestion, undecided).
- **VT-Q06 Selection.** Selected see-through polygons draw solid in the
  selection colour, like a whole selected item, so what is selected stays
  clear. Picking still hits the nearest active polygon; to pick liquid
  inside glass, make the liquid the active mesh.
- **VT-Q07 Shadows.** Decided by Jason 2026-11-14: lighter by
  transparency. With Show Material Transparency on, a see-through surface
  casts a shadow lighter by how much shows through (98 % clear ≈ faint,
  50 % ≈ half); opaque casters and the setting off keep solid shadows.
  Built as a dithered shadow-map pass for items with see-through polygons
  only, which the lookup's 3×3 filter averages; the shadow is uncoloured
  (Transparent Color doesn't tint it).
- Dissolve folds in as an untinted see-through share.

## Symmetry for the modelling tools (symmetry-2026-11-13, phase 1)

- **SYM3-Q11 Solid Sketch twins with Symmetry off (provisional).** Q8's
  recommended answer (stay linked, nag) is built as: twins keep mirroring
  every edit even with Symmetry off; only new nodes get no twin. Confirm,
  or should edits go one-sided until Symmetry is back on?
- **SYM3-Q12 First Solid Sketch node off the plane.** **Answered (spec
  v3.1, SYM-B3-10 seen):** the first node always lands on the plane. Built.
- **SYM3-Q13 Edge Extend Rotate/Scale with Symmetry.** Offset is mirrored
  for the partner side; Rotate and Scale aren't yet (provisional).
- **SYM3-Q14 Loop Slice ring that is its own mirror.** A ring whose band
  crosses the plane along the axis cuts each edge from its walk direction;
  at positions other than 50 % such a cut can land off-symmetric.
- **SYM3-Q15 Symmetrize Polygons with loose points on the replaced side.**
  Refused with a message (provisional). Delete them, or keep and mirror?
- **SYM3-Q16 Symmetry Tool details.** Built from spec section 9 and v3.1.
  Open: the reference tool ID and argument names (Operandi uses
  `operandi.symmetry.tool` for now); the drag gain (right raises Threshold
  by the drag's world length); points near the plane (built: within half of
  Threshold, with no other partner, they snap onto it); whether it acts on
  the selection only or the whole mesh (built: whole mesh).
- **SYM3-Q17 Phase 2 provisional readings.** An Edge Slice path that crosses
  the plane is cut once (its mirror can't be placed on the first cut's
  pieces); Axis Slice's mirrored Offset assumes the cut box is centred on
  the plane; Clone/Array/Radial Array with nothing selected copy everything
  with the placement as entered; a polygon across the plane goes with the
  primary side's pass (Sketch Extrude, Clone).
- **SYM3-Q18 Toggle Symmetry's default key.** Section 8 asks for one key
  (Q2: the reference's key if it has one). Built unbound and remappable
  until research names the key.
- **SYM3-Q-OP01 Seam snap with the plane facing the view (provisional,
  2026-11-14).** With X Symmetry in the Left or Right view, every point
  sits on the seam on screen, so SYM3-23's screen test snapped every
  placed point to X = 0. Now the seam snap is skipped when the way to the
  plane runs within about 11° of the view ray. Is that the intended rule?
- **DRAW-Q-OP01 Depth of a placed point (Jason, 2026-11-14; provisional
  edges).** In a view without perspective, a new Solid Sketch node or
  Curve/B-Spline/Bezier point keeps the view-depth coordinate of the
  point it extends (the current one); only the two in-view axes come from
  the pointer. Kept on the work plane: Perspective views, the first point,
  a fixed work plane and Lock Preferred Plane. The Pen keeps its draft's
  plane (a polygon is flat). Should Perspective follow the previous point
  too?

## X-Ray / Ghost quick toggle (xray-toggle-2026-11-13)

- **XR-Q1 Default key for Toggle X-Ray.** Unbound (spec Q1); set it in the
  Input Map. Confirm none, or pick one.
- **XR-Q2 Picking in X-Ray (provisional).** Clicks still pick the nearest
  surface under the pointer, even where its front polygons are clear, as
  the spec says until measured; the middle-button lasso reaches hidden
  elements. Should a click in X-Ray pick through the clear front faces to
  what's behind them (the "select a joint inside a character" use case)?
- **XR-Q3 Styles (provisional).** The buttons work in every style but
  Wireframe, the same styles the Viewport Properties row allows (spec Q3).
  Confirm, or limit Ghost/X-Ray to the solid and advanced styles.
- **XR-Q4 Alt+click with Make Inactive Same as Active / Invisible
  (provisional).** Alt greys the buttons then (the inactive meshes follow
  the active ones, or aren't drawn) and a click says why. Confirm.

## Light view (view menu Light entry, coordinator brief 2026-11-14, branch `claude/project-thread-bywy7w`)

- **LV-Q1 Orbit point (provisional).** A light has no Target Distance, so
  orbit and dolly turn about a point straight ahead of it: on entering,
  where the light's aim passes the previous view's centre (1 m ahead when
  that's behind it); after that, as far ahead as navigation left it. Is
  there a stored distance, or another rule?
- **LV-Q2 Field of view (provisional).** A spot light's view fits its whole
  cone across the view's shorter side; every other light (and spots wider
  than 160°) uses a new view's lens. Confirm, or should every light use one
  fixed lens?
- **LV-Q3 Roll (provisional).** As in Camera view, the view keeps world up
  at the top, so navigating writes a light with no roll about its aim
  (visible on area and cylinder lights). Should the view take the light's
  own roll?
- **LV-Q4 (Current Selection) (provisional).** The view follows whichever
  light is selected and keeps the last one while something else is
  selected; the entry is greyed when no light is selected. Confirm, or
  should it fix the light selected at the moment of picking?

## Vertex Bevel batch 1 (vertex-tools-2026-11-14 v1.1, branch `vertex-bevel`)

- *VXB-Q1 and VXB-Q2 answered in spec v1.2 (VX-B08, VX-B09) and built.*
- **VXB-Q3 Inside points next to a clamped cut (provisional).** In the
  VX-B08 clamped case at L2, Operandi's inside rule matches one of the four
  check points within 0.01; the others are 0.011, 0.012 and 0.04 out (the
  sector beside the clamped cut sits 0.04 further out than the reference's
  (-0.4514, -0.5)). The reference pulls the inside points toward a clamped
  cut much more than the border does. Probe: the same case at L3 and L4,
  and a cube corner with one short edge at L2, listing every inside point.
- **VXB-Q1 Clamped fillets (answered).** When a clamp leaves a side's
  two cuts at different distances from the point (one edge shorter than
  Inset), no circular arc is tangent to both edges there. Operandi
  stretches the arc to meet both cuts (still tangent to both edges). Grid
  point 5 with point 6 moved to make one 0.1 m edge, Inset 0.2, Round
  Level 2: what does the fillet on that side look like?
- **VXB-Q2 Cap UVs (answered).** Fillet points take the UVs their place
  has inside their own polygon; cap points inside blend the corners'
  UVs (the centre the average of the original corners). Interior grid
  point, UV map on, Round Level 2: how are the cap's UVs laid out?
- *VXB-Q3 answered in spec v1.3 (VX-B11) and built.*

## Vertex batch 2: Merge and Join (vertex-tools-2026-11-14 v1.3, branch `vertex-merge`)

- **VXM-Q1 Merge / Join Discontinuous UV Values (provisional).** On (the
  dialogs' default), corners at a merged point whose UVs are now equal share
  one UV; corners with different UVs keep theirs, so seams stay. Off, every
  corner keeps its own UV even when the values match. Probe: two quads with
  separate coincident points on a shared side, UVs equal there; Merge with
  the option on and off, then move one UV in the UV view: does the other
  corner follow? And with different UVs: are they averaged?
- **VXM-Q2 One-point polygons (provisional).** Keep 1-Vertex Polygons keeps
  a polygon squashed to two corners as a two-point line. A polygon squashed
  to one corner is still removed, since Operandi's mesh has no one-point
  polygons yet (Create's Point Polygon, VX-CR01, will need them too). Which
  of the two does the reference keep?
- **VXE-Q1 Vertex Extrude starting Width (open, Jason).** Spec v1.3 VX-E
  says Width starts at 0; Operandi starts it at a tenth of the shortest
  edge (EXW-Q01) so dragging the Extrude arrow alone does something. Left
  as is until Jason decides.
- **LXO-LEG-Q05** (2026-11-14, branch lxoxide-borrow) Format 4.x records
  Operandi meets but doesn't use are skipped with a named note instead of
  refusing the file: sub-scene/reference records (IASS, AANI, REFS, SUBS,
  XREF), unknown ITEM/ACTN records, a repeated ACTN parent (first kept),
  a missing or non-UTF-8 ENCO (strings still read as strict UTF-8, except
  names: changed 2026-11-15 from the spec side's review, a name (item,
  VNAM, layer, polygon tag, vertex map) that isn't UTF-8 is decoded as
  Windows-1252 with a note "Name '…' decoded as ANSI (Windows-1252)"
  instead of refusing the file; with ENCO=2 it still refuses).
  Provisional: a channel stored twice keeps its value when both agree;
  disagreeing values, unreadable CHNV vector types and unreadable ACTN value
  types refuse only a modeled owner's group. Are any of these unsafe?
- **LXO-OLD-Q01** (2026-11-14, branch lxoxide-borrow) LXO formats 2.x and
  3.x (Modo older than 8) are read by the format 4.x reader, mesh content
  only, using lxoxide (MIT, ea7ede6) as the reference for record layouts.
  *Provisional* rules, all built black-box (no Modo-saved file inspected):
  - LEG-OLD-01: 2.x/3.x are accepted instead of refused; every refusal now
    names the file's format (`[LXO format 2.1]`).
  - LEG-OLD-02: their LAYR settings after the item id (subdivision level,
    curve angle, tail length) aren't checked, since they don't change the
    polygon cage; the layer matrix must still be identity and sculpt data
    still refuses the mesh.
  - ITEM.CHNL (a scalar channel named by a string, no CHNM index) reads
    like CHAN.
  - LEG-OLD-03: an ITEM repeated with the same id, kind and name (common
    in presets) is one item, its records merged; channels that disagree
    refuse its group. The same id on a different kind or name still
    refuses the file.
  - LEG-OLD-04: links and action values aimed at items the file doesn't
    contain (the render item's `shadeLoc`, the scene's setup action) are
    left out with a note. Applies to format 4.x too.
  - LEG-OLD-05: PRST/PPRT item tags on any item (preset markers) are left
    out with a note.
  - LEG-OLD-06: one- and two-point polygons in FACE/SUBD/PSUB blocks are
    left out with a count, their tags and corner UVs with them; the other
    polygons keep theirs.
  - LEG-OLD-07: in a mesh preset (LXPM) of format 2.x–4.x, a layer whose
    item id names no mesh item imports as a mesh of its own (named after
    the layer, at the root), and a mesh item with no layer is left out.
    Scenes (LXOB) still refuse both.
  - A mask or other shader item's `shadeLoc` link to a mesh no longer
    refuses the mesh (LEG-12: shading, not placement or geometry).
  - A UV map giving one corner two values is left out, not the file.
  Still refused (counts from Jason's Modo Content, 914 files, 724 open):
  LEG-14 bad geometry 95, rig/schematic/dynamics/instance links 40,
  procedural modelling 32, offset layer pivot 8 (is a nonzero LAYR pivot
  an offset to add to the points, or only the pivot?), format 4.x layer
  settings or sculpt data 8, unknown item tags 4. Are any of the rules
  above unsafe, and what does a LAYR pivot do?
  - **LAYR pivot: answered by the spec side 2026-11-15.** Stored points are
    absolute; a nonzero LAYR pivot doesn't move them. Formats 2.x–4.x and
    the phase-1 reader (Q-LX-04, which added the pivot to the points) now
    import points as stored and note "layer pivot (x, y, z) kept as
    information only, not applied" per mesh. The Modo 16/17 strict reader
    still refuses one. Checked against the spec's world boxes for four
    local presets (Sofa_c683, Pendant Heart 01, Hand 01, Shelving_Enam):
    all match to 1e-4, axis for axis. Open: the legacy reader keeps Z as
    stored while phase 1 negates it; which is right for each format?

## Modo material presets (.lxp), BEHAVIOR-LXP-MATERIALS v1.1 (branch `lxp-materials`, 2026-11-15)

Builder questions, all *provisional*:

- LXP-B01 Stage A: Incidence, Driver A–D, Process and Occlusion already
  existed. Incidence now also draws per pixel in the viewport (it was one
  flat value there, the head-on one). Reflection Color still reaches no
  render input (ST-Q05): a gradient on it folds into the channel but
  doesn't change the picture. Occlusion stays 1 in the viewport (Cycles AO
  only).
- LXP-B02 / LXP-B02b *Answered* (Research, 2026-11-15): OPM-Q09 stands,
  1 nit = 1 render unit; the spec's × 683 was a mistake. A `.lxp`
  Luminous Intensity is render-unit radiance taken 1:1 as
  emission_luminance, so White LED is 1.7 × lumiCol 0.98. The earlier
  `.lxp`-only ÷ 683 path is gone; glTF/USD are unchanged.
- LXP-B02a *Answered* (Research, 2026-11-15): any surface of radiance L
  renders pixel L whatever made it. Mesh and cylinder lights now feed
  their Emission with strength L, not π·L (a radiant exitance E would be
  E/π), so they are 1/π as bright as before (about 0.32×, −1.65 stops) at
  the same Intensity. Area lights keep power L·area·π, which Cycles shows
  as radiance L. Pixel tests: material, mesh light and area light, each
  pixel 1.0 at radiance 1 (`emission_units_tests`). Found on the way: a
  lamp (area, point, spot) with Visible to Camera but neither Visible to
  Reflection nor Refraction doesn't show to the camera, because Cycles
  only intersects lamps with MIS on and the bridge turns MIS on from
  those two rows only. Not changed here (it's a C bridge change).
- LXP-B03 Render-only settings (rays, clipping, exit colour, Specular
  Fresnel) go in the note but don't light the tile's info dot; only
  look-changing drops and approximations do.
- LXP-B04 A material with no stored `brdfType` reads as Physically Based;
  `blinn` and `ashikhmin` are the legacy models (§3.3). In the user's
  content: 394 none, 610 ashikhmin, 231 blinn, 37 gtr.
- LXP-B05 Layer order: siblings sort by their `parent` link ordinal,
  turned round when that puts the material on top. Nested masks are
  flattened into one group; a preset with several materials keeps the
  first ("only the first material kept").
- LXP-B06 LXP-05 "every bump/diffColor layer dropped" read as every Bump
  layer or every Diffuse Color layer.
- LXP-B07 *Answered* (Research, 2026-11-15): each gradient record is one
  number curve keyed by channel name (`value`, `color.R/G/B/A`) with an
  interpolation flag (0 curve → Smooth, 1 Linear, 2 Stepped). Built:
  colour effects build keys at the union of the R/G/B(/A) curves' inputs,
  a missing component holds the item's static channel value or 0, any
  stepped component steps the colour; a lone record on a colour effect is
  read as `value` and goes grey with "colour gradient had only a value
  curve"; unknown flags read Smooth with "gradient curve approximated".
  Provisional: a missing alpha curve with no static value is 1 (opaque),
  not 0.
- LXP-B07a *Answered* (published spec 7.6.7 and 10.6, 2026-11-15):
  ITEM-level GRAD = S0 name (empty = `value`), VX envelope, U4 flags;
  action GRAD = VX CHNM index, VX envelope, U4 flags, S0 name used when
  the index is 0. Built: actions apply parents first (PRNT) so a child
  wins, `setup` and `edit` only (the content's 69 `scene` actions and any
  `anim` are left out and counted), action GRADs replace item-level ones
  by channel name, and action CHAN values reach every item. Content
  survey: 0 colour-effect gradients go grey now.
- LXP-B07b *Provisional*: an action `CHNN` is read as CHAN with an S0
  name in place of the index (U2 type, VX envelope, value). None occur in
  the content, so it's untested against real files.
- LXP-B07c *Answered* (Research, 2026-11-15): percent-typed gradient
  values (34 of 170 records in the content) are 0–1 fractions like
  `float` ones, and aren't clamped: over 100 % is legitimate. Before,
  these 456 layers were dropped as "gradient with percent keys".
- LXP-B08 Out-of-range values (frequencies past 8, ratios, IOR) are
  clamped with a note; a locator scale of 0 keeps a 1 m texture size with
  a note. Colour space: Operandi reads colour maps as sRGB and data maps
  linear, so a `(none)` colour map is noted as read sRGB.
- LXP-B09 The "partial" badge is a small "partial" caption at the tile's
  bottom left (like MC-10's "geometry only") in the warning colour; the
  info dot is an accent dot at the top right whose tooltip is the note
  and whose click puts it in the status bar. The thumbnail stays Modo's
  PRVW (LXP-Q4: until we render our own). The UI bible wasn't on this
  machine, so no UIC numbers were checked.

## Item transform Reset + Freeze (freeze-transforms-2026-11-14, stage 1, branch `freeze-transforms`)

- **FRZT-Q01 Pivot (provisional).** Operandi items have no pivot offset
  channel: the pivot is the item's centre. So the bake is exact with no
  pivot term (rule 4.2.6 holds trivially). After Freeze Position or All the
  centre is at the parent's origin, as §4.1 says; it can't "keep its place
  on the mesh" because there is nothing to hold it. Revisit when a pivot
  channel lands (stage 2?).
- **FRZT-Q02 Animated channels (provisional).** Operandi has no keyframes
  yet, so no channel is animated and the "Position is animated, Freeze
  skipped" refusal (rule 4.2.8) has nothing to fire on. No keyed-channel
  test was possible. To build with animation.
- **FRZT-Q03 Instances (provisional).** Operandi has no mesh instances. The
  closest thing is a Mesh light whose prototype is the frozen mesh. With Use
  Prototype Transform on (default) it stays put. With it off the light draws
  the prototype's local points in its own frame, so a Freeze changes its
  shape; not compensated (that would rewrite the light's channels). Starting
  state: mesh light, Use Prototype Transform off, prototype rotated; Freeze
  All on the prototype. Should the light's shape be kept?
- **FRZT-Q04 Stored Solid Sketch (provisional).** A mesh that holds a Solid
  Sketch drops the stored sketch on Freeze, because its nodes are in the old
  local space and picking it up would rebuild in the wrong place. Option:
  transform the sketch nodes instead (only exact for similarity bakes).
- **FRZT-Q05 Children that would shear (provisional).** Baking a rotation
  under non-uniform scale is exact for the points (B = S⁻¹RS), but a rotated
  child would need shear to keep its place, which Operandi channels can't
  hold. That item's Freeze is skipped with a note ("… would shear under its
  non-uniform scale, Freeze Rotation skipped; try Freeze All on both"); the
  rest of the selection still freezes. Same for Scale/All with non-uniform
  scale and a rotated child. Start: parent scale 1,3,1 rotated Z 30°, child
  rotated Z 20°; Freeze Rotation on the parent. Is skipping right, or should
  the child be allowed to move?
- **FRZT-Q06 Freeze on mixed selections (provisional).** The panel shows
  Freeze when the active item is a mesh; a selection whose active item is a
  camera shows only Reset even if meshes are also selected. Item > Transform
  offers Freeze whenever any selected item is a mesh.

## Selection inference (selection-inference-2026-11-15 v1.1) — 2026-11-15 (branch selection-inference)

Defaults SI-Q03 (Crease and Split Normals from polygons: outline), SI-Q04
(plain Join lands on the last point in SI-07 order) and SI-Q05 (show the
inferred elements) are built as the spec gives them. Operandi readings:

- **SI-B01 Highlight look (provisional).** While a point or edge tool works
  on an inferred set, the viewport draws that set as the tool's own mode's
  selection (selected dots or edges, on the smooth surface through the
  shared path), not a separate lighter colour; the polygon tint of a
  Polygon-mode selection is not drawn until the tool ends.
- **SI-B02 Weld drag (not built).** The Weld drag tool still arms only in
  Vertex mode: it drags one picked point, so there is no set to infer.
- **SI-B03 Other-mode lists (provisional).** The inferred set is written
  into the tool's own element list (selected points or edges), which stays
  as the tool leaves it, also after Cancel; that list was ignored in the
  starting mode anyway (SI-04).
- **SI-B04 Kept selection after the edit (provisional).** The selection
  made in the starting mode is kept by index where the index still exists;
  for the tools tested (Vertex Bevel, Edge Bevel, Join, Align, Split) the
  index still names the same element. A tool that renumbers polygons
  would keep the wrong ones.
- **SI-B05 Selection falloff (provisional).** A Selection falloff reads the
  tool's own mode (the inferred set), not the starting selection.
- **SI-B06 Empty selection message.** A tool that refuses an empty set in
  its own mode refuses it from another mode with its own-mode message
  ("Vertex Bevel needs selected points in Vertex mode").


## Bridge in Edge mode — 2026-11-15

From Jason's report (two open border edges, Apply greyed). Spec:
`bridge-2026-11-05` (Edge mode BR16/BR17).
- **BRI-Q05 (reworked)** Bridge no longer needs default work plane, action
  center, falloff or bevel profile settings. It builds from the two
  selections alone in the mesh's own space, with no handle or drag for those
  settings to act on, so they are ignored. If the reference does use any of
  them for Bridge, which and how?
- **BRI-Q07 Curve and Smooth for edges (provisional).** The spec measures
  Curve only on facing polygons (section normals) and BR17 on coplanar
  edges (even spacing). For edges, the curve leaves each side across its
  edge in the plane of that side's border polygons: Curve uses one averaged
  direction per side, Smooth each point's own. A side whose polygons don't
  lead towards the other side (a lid's rim seen from straight below) leaves
  straight. Start: a flat quad's border edge and a wall quad's bottom edge
  one unit across and up; Segments 3. What path does the reference take?
- **BRI-Q08 Flip, Reverse Direction and Remove in Edge mode
  (provisional).** An edge bridge always runs against both borders' corner
  order so it faces like the polygons it joins; Operandi's mesh rules need
  that (no inside-out joins, no edge with three polygons). So Flip Polygons
  and Reverse Direction are greyed and ignored in Edge mode, and Remove
  Polygons has nothing to remove. When the two sides face opposite ways the
  only bridge would cross itself, so it is refused with "Flip one side's
  polygons, then bridge". Edges inside the surface (polygons on both sides)
  are refused too. Does the reference build an inconsistent or non-manifold
  bridge in these cases?

## Falloffs batch 1 — 2026-11-15

Spec: falloff-2026-11-15 BEHAVIOR-FALLOFFS v1.1 (project files), batch 1.
- **FO-Q04 (provisional default).** Selection Steps by mouse: with a tool
  that takes falloff open, a right-click (moving under 4 px) adds one Step;
  a right-drag adds one per 20 px right and removes one per 20 px left. It
  acts on the first Selection layer in the stack. Does the reference use a
  click, a drag, or both, and which way adds?
- **FO-Q09 (answered, Jason 2026-11-15: "ok for deviation").** The row is
  **Shape** on every falloff and on Tweak, where the reference says Shape
  Preset on the geometric ones. Approved bible deviation.
- **FO-17 not reachable yet.** A tool that ignores falloff refuses to start
  while a falloff is active (FAL-T, `falloff_refusal`), and the menu is greyed
  while such a tool is open, so the greyed "This tool ignores falloff"
  section never shows. Should such tools start and ignore the falloff instead?
- **FO-16.** With no tool open the falloff sections show under Tool
  Properties' "Select a tool" line (no separate empty header row).
- **Unbuilt rows left out (UIC-117):** Use World Transforms (batch 3).
- **FO-40 default End.** Factory End is now 0,1,0 (measured); FO-04 still
  fits it to the selection when a tool is open.

## Falloffs batch 2 — 2026-11-17

Spec: falloff-2026-11-15 v1.1, batch 2 (G12 Element, G13 Soft Selection,
G14 Coplanar).
- **FO-Q05 (provisional default).** Soft Selection Radius by mouse: with a
  tool that takes falloff open, a right-drag changes Radius, one metre of
  view per pixel at the selection's centre, right for more. With Use
  Connectivity, distance runs along polygon sides from the selection;
  without, straight-line. A Selection layer in the stack takes the
  right-drag first (Steps).
- **FO-Q06 (provisional default).** Coplanar: the selection's polygons (the
  selected ones in Polygon mode, otherwise those whose corners are all
  selected) set one facing, their summed area-weighted normal. Polygons
  facing within Angle of it take part (spreading across shared sides with
  Use Connectivity), each weighing Shape(angle / Angle); Angle 0 takes
  polygons within 0.01°. A point takes the most of its polygons. Is the
  angle measured against the selection's facing or against the neighbour
  it spreads from, and does "coplanar" also need the same plane offset?
- **FO-Q11.** Element: Range is measured from the picked element's points
  in every mode, as Tweak does (TWK-12); FO-45 says from its centre for the
  Center modes. Which? (The Center modes do put the action centre at the
  element's centre.)
- **FO-Q12.** Element press: with Move/Rotate/Scale/Transform and an
  Element layer, a press off the handles picks the element under the
  pointer (the falloff's Element Mode decides which kinds), puts the action
  centre on it and drags freely; each press is its own undo step. A press
  on the handles drags the last pick. Nothing moves before the first pick.
  Is the selection ignored entirely (as built), and should the handles jump
  to the element on hover?
- **FO-Q13.** Reaching outside the selection (FO-47) is built for the
  transform tools (Move, Rotate, Scale, Transform and their presets). The
  other tools that take falloff (Bevel, Smooth, Push, Slide, Bend, edge and
  vertex tools) still work on the selection only, with these falloffs
  weighting inside it. Should any of them reach too?
- **FO-Q14.** Command names `falloff.softSelection`, `falloff.element`,
  `falloff.coplanar` and their keyed options (radius, connectivity, angle,
  mode, range, connect, shape, p0, p1, mix) are our guesses; the reference
  name map has none for them.
- **Always Raycast** is stored and shown; Operandi always picks on the
  surface it draws, so it changes nothing (provisional).
- **Show Vertex / Show Edge** draw for the first layer only (greyed on
  added layers); Show Weights (proposed) is the View ▸ Show Falloffs toggle.

## Falloffs batch 3 — 2026-11-17

Spec: falloff-2026-11-15 v1.1, batch 3 (G07 World Transforms, G08 draw by
drag, G09 per-axis handles, G10 Cylinder).
- **FO-05 (provisional reading).** Choosing Linear, Cylinder or Radial in
  the Falloff menu, or the section's **Draw in View** button, arms
  drawing: the next unmodified press off the handles sets the origin (on
  the plane facing the view through the tool's centre), the drag sets the
  size, the release accepts, and drawing disarms. Otherwise presses select
  or drive the tool as before. Linear runs press → release point; Radial is
  a sphere and Cylinder a tube of the dragged radius. Does the reference
  draw on every press while the falloff is the active tool, and does Radial
  get one size or three?
- **FO-35 default off (measured) changes old behaviour on moved meshes.**
  Falloffs are now read in each mesh's own space unless Use World
  Transforms is ticked; Auto Size and the deform presets (Soft Move, Taper,
  Twist…) fit in the active mesh's own space. For meshes at the origin
  nothing changes.
- **FO-40/41/42 per-axis handles.** Short lines in the axis colours, 34 px
  long, from Linear's Start and End and from Radial's and Cylinder's
  Center; each moves that one coordinate (in the falloff's own space).
- **FO-42 Cylinder outline (proposed):** rings across the Axis at the
  centre and two radii either side, joined by four lines; Size handles for
  the two cross axes only. Choosing Cylinder with a tool open fits it to
  the selection (FO-04); Linear and Radial keep their last settings
  (FAL-M-03), as before.

## Falloffs batch 4 — 2026-11-17

Spec: falloff-2026-11-15 v1.1, batch 4 (G11 Screen, G15 Lasso, G16
Airbrush).
- **FO-Q08 (default taken: yes).** Screen, Lasso and Airbrush weights are
  worked out from the view when drawn or painted and stick to the points
  ("stamps", per stack layer and mesh); moving the camera afterwards
  changes nothing. Editing Screen's or Lasso's options works the stamp out
  again in the current view. Topology changes drop a stamp (weight 0).
- **Drawing.** Choosing Screen or Lasso arms drawing (as FO-05), and so
  does their Draw in View button: Screen's press sets Center, the drag its
  Size; Lasso draws its Style (Lasso freehand, Rectangle corner to corner,
  Circle from its centre, Ellipse in a box) and stamps on release. Screen
  has no on-screen handles yet (Draw again or type Center/Size).
- **Airbrush.** With a tool that takes falloff open (or none), an
  unmodified press off the handles paints; each pointer sample adds
  Strength × Smooth(d / Size), clamped to 1, and the live tool re-applies
  with its current values (so set the tool's values first, then scrub).
  Right-drag resizes the brush, 1 px per px. "Clear Paint" empties it.
  The paint stays until the falloff type changes.
- **Transparent** (Screen, Airbrush): off, a point counts only when a
  polygon around it faces the view. Lasso always projects through.
- **Lasso Use World Transforms** is stored only: the shape's weights are
  fixed when drawn.

## Falloffs batch 5 — 2026-11-17

Spec: falloff-2026-11-15 v1.1, batch 5 (G17 Noise, G18 Vertex Map colour
and morph maps, G20 Image).
- **FO-50 (proposed shape).** Noise is our own gradient noise of position
  ÷ Scale with a fixed seed, mapped −1..1 → 0..1 (0.5 on every lattice
  point). Reads own-space or world positions per Use World Transforms.
- **FO-49 colour and morph maps.** The falloff menu's map picker (still
  labelled Weight Map) now lists morph and colour maps after the weight
  maps, marked "(morph)" / "(colour)"; the sculpt tab's picker lists weight
  maps only. A colour map's point value is the mean of its corners'
  colours; Magnitude = mean of R, G, B; Component Index 0..3 = R, G, B, A.
  Morph: Magnitude = offset length, Component 0..2 = X, Y, Z. A name used
  by more than one kind reads the weight map, then the morph, then the
  colour map. With two Vertex Map layers each reads its own Mode/Index.
- **FO-54 Image.** The picture is chosen with Browse… (any format the app
  reads; turned to grey) and kept for the session, not saved with the
  scene. Laid over the view, it spans the view's height, centred, at
  Scale 100 %; Pos is in picture widths/heights. Weights are stamped when
  chosen or edited (FO-Q08), like Screen. Soft Border fades each tile's
  edges over that per cent of its half-size. Through the UV map, each
  point uses the mean of its corners' UVs in the mesh's selected UV map.
  No on-screen handles yet (FO-54's centre/rotate handles); Mix Mode is
  shown (Operandi adds it).

## Falloffs batch 6 — 2026-11-17

Spec: falloff-2026-11-15 v1.1, batch 6 (G19 Path, G21 Curvature).
- **FO-53 Path (provisional gestures).** Choosing Path turns on point
  adding (the section's Add Points / Stop Adding Points button toggles
  it): each unmodified click off the handles adds a control point on the
  plane facing the view through the last point (the tool's centre for the
  first). Points are falloff handles: drag one to move it. Clear Path
  removes them. The curve is our own Catmull-Rom through the points (not
  the curve tool's); weight Smooth(distance / Size). Points live in the
  mesh's own space unless Use World Transforms is on. They are tool
  context, not saved. Should clicks add points only with no tool open?
- **FO-Q07 Curvature (provisional formula).** Each point's bend is how far
  its neighbours' average sits below it along its normal, divided by their
  mean distance (positive on bumps, negative in dips); weight = 0.5 +
  bend × Scale, clamped to 0..1. Flat areas weigh 0.5. This doesn't yet
  match "low Scale favours concave, high Scale convex"; it needs the
  reference's formula or measured cases.


## Join with Symmetry — 2026-11-15

From Jason's report (cartoon kid tutorial: edge Join with Symmetry on
refused "needs exactly two selected edges", because symmetric picking
selects four). No spec covers Join with Symmetry.
- **SYM-JOIN-Q1 (provisional).** Edge Join / Join Averaged: with a pair of
  edges on each side and none on the centre line, each side's pair joins by
  itself (first pick onto second, or halfway), in one weld, so points the
  pairs share on the plane weld once. Two edges (one side, on or across the
  centre line, an edge and its mirror) join as without Symmetry; any other
  count refuses and says how many edges sit on each side and on the centre
  line. Vertex Join: two or more points on each side and none on the plane
  weld per side; otherwise all the points weld into one as before. Start:
  Symmetry X, two facing quad gaps mirrored on ±X, both edges of each gap
  picked. Does the reference join each side separately, and what does it
  do with three edges, or a pair plus a centre-line edge?
## Twist Bevel (twist-bevel-2026-11-15 v1.1) — 2026-11-15 (branch twist-bevel)

The spec's own open questions TB-Q01..Q12 are built with its defaults
(Segments 4, Normal/Action Axis, the Profile, bias and spacing formulas,
+X-flattened Bend Direction 0°, falloff on Shift/Inset/Rotation/Bend, Bend
Direction locked for a group across the symmetry plane, Bend clamp ±360°,
drag anywhere = Shift, twist about the moving tangent). TB-Q09: Tool States
are left out entirely (a later shared job), not even Reset. TB-Q10: the
credit line is used as written. Build readings of our own, all provisional:

- **TB-B01 Action Axis direction.** Rotation Axis = Action Axis twists about
  the action frame's Y axis (its "main direction"), mirrored for a group on
  the mirror side. Start: a polygon facing +X, Action Axis World, Rotation
  90°. Which axis of the action frame is meant?
- **TB-B02 Readout.** Operandi has no readout next to the pointer yet, so a
  handle drag shows "Twist Bevel · Rotation 42.5°" on the status line.
- **TB-B03 Closed selection.** A group whose normals cancel can't reach the
  tool (a closed region has no outline, which the region finder already
  refuses), so the whole Twist Bevel refuses rather than one group.
- **TB-B04 Folded walls.** A wall that twists or bends so far that it
  crosses itself (a sharp twist over a short Shift, or a bend radius under
  the tube's half width) refuses the preview, like Bevel's folded bands:
  "a wall twists or folds too far; add Segments or Shift, or lower
  Rotation". The last good shape stays until it's fixed.
- **TB-B05 Falloffs that read points.** A Vertex Map or Selection falloff
  has no value at the base centre, so a tube takes the mean of its points'
  weights. Position falloffs (Linear, Radial) are read at the centre.
- **TB-B06 Handle increments.** The bend, twist and inset handles don't
  step by Rotation Snap Angle or value increments yet (spec section 5's
  "if increment snapping is on"); Operandi has no increment-snapping
  switch for tool values. Only the Shift arrow snaps (to targets).
- **TB-B07 Continue keeps the shape.** Shift+click starts a new twist bevel
  on the caps with Shift and Inset at 0 and Rotation, Bend, Profile and
  spacing kept, so a chain of segments can share one curl.
- **TB-B08 One mesh.** Twist Bevel runs on the primary mesh only, not on
  every active mesh as Bevel does.
- **TB-B09 Icon.** The generator's `twist-bevel` icon (Jason approved it,
  2026-11-15).
- **TB-B10 Scripts.** `tool.attr poly.twistBevel rotation|bend|bendDirection`
  take radians, as every tool.attr angle does; the panel shows
  degrees. Field keys follow the spec's names: shift, inset, segments,
  group, rotation, rotationAxis, profile, profileBias, segmentSpacing,
  centerBias, bend, bendDirection.


## Selection action centre in Edge and Vertex mode (2026-11-15)

- **ACS-Q01 Selection axes without polygons.** Specs: action-center-2026-11-01
  (D18-A), action-center-scale-2026-11-06, selection-inference-2026-11-15.
  Start: a mesh in Edge or Vertex mode with edges or points selected; choose
  Action Center ▸ Selection. Unclear: which axes the reference takes from
  edges or points. *Provisional:* the equal-vote average normal of the
  polygons that use the selected edges (Edge) or touch the selected points
  (Vertex), same maths as polygons; World axes when nothing is selected or
  the selection touches no polygon (wires); opposing normals still refuse
  with the existing "indeterminate" message. Selection Center Auto Axis
  uses the same axes and falls back to World instead of refusing.

## Nits batch 1 (branch `nits-batch-1`, 2026-11-15)

- **BD-Q7 Bend Action Axis.** Spec: deform-presets-2026-11-06 (Bend lists
  only "Action Axis Auto"). Start: Bend on an upright column. Jason asked
  for an Action Axis choice. *Provisional:* Auto / X / Y / Z (field
  `operandi.actionAxis`); X/Y/Z is the action axis the bend turns around,
  so it leans along spine × axis; an axis along the spine falls back to
  Auto. Unclear: whether the reference offers this and how it names it.
- **LOOP-HOLE-01 Edge loop on a hole's border.** Spec: selection-2026-11-02
  SEL-LP-03 (an open grid's border loops). Start: a grid with an opening
  whose corner points carry two spokes as well as the two border edges;
  double-click a border edge of the opening. *Provisional:* where the
  straight walk has two ways on and the edge is a border, it follows the
  only other border edge, so the whole opening's rim is selected.
- **SYM-MAKE-Q1 Make Polygon with Symmetry.** Start: Symmetry X, points
  picked on one side (each pick also selects its mirror). *Provisional:*
  four or more picks make that polygon and its mirror, each wound like its
  neighbours; two picks make one polygon across the plane (picks, then
  their mirrors back). A selection that doesn't alternate pick, mirror is
  made as is. Unclear: the reference's rule, and three picks spanning the
  plane (made as two mirrored triangles here).
- **EBC-Q11 update.** Edge Bevel's end slide is now refused only where the
  polygons the slide runs over face more than 45° away from the strip's
  polygons at that point (was: every polygon around the point within 20°),
  so ends on curved organic surfaces bevel (Jason, 2026-11-15). The 90°
  box/fold case is still refused. Open: the reference's threshold.

## Tool presets stage 1 (2026-11-15, spec tool-presets v0.2)

Jason decided TP-Q1 (restore on drop), TP-Q3 (axis dialog for now) and
TP-Q4 (readable data file in the app) on 2026-11-15; TP-Q2 is still open,
and today's behaviour (tools start from the values last finished with)
stays. Builder's readings:

- **TP-B01 Twist and Swirl action layers.** Section 6a gives Twist an Auto
  action axis and Swirl an Auto action centre and axis. Today they leave the
  user's action centre alone, and the migration must not change behaviour,
  so their rows name only the falloff. Adding the layers is a one-line data
  change in `presets/factory-tool-presets.toml` once confirmed.
- **TP-B02 No Auto action axis.** Operandi has Automatic as an action
  centre but no Auto action axis, so Flare's row names an Automatic centre
  and no axis slot (Push moves along normals, so the axis doesn't change
  its result).
- **TP-B03 Flare's falloff direction.** Section 8 says "the Linear falloff
  end gets the full push and the start gets none". Operandi's Linear
  falloff has full effect at its Start, and Flare is fitted like Shear and
  Taper (Start at the selection's high end, DF-Q3), so the top of a column
  gets the full push and the foot none. *Provisional.*
- **TP-B04 Reset under Thicken (TP-15).** The spec says Reset under Thicken
  already gives Thicken's values; on main it gave Smooth Shift's (a test
  said so on purpose). It now gives Thicken's, as TP-15 asks, and so does
  starting Thicken again after that Reset. Reverting is one line in
  `smooth_shift_editor.rs`.
- **TP-B05 Flip It and shared points.** Flip It mirrors the points of the
  polygons it flips. Points shared with polygons it doesn't flip move too
  (as Scale −1 would); when that would squash a neighbour flat, Flip It
  refuses and nothing changes. Best on separate pieces. Vertex and Edge
  mode refuse, as Flip does; nothing selected or Item mode flips the whole
  mesh. *Provisional.*
- **TP-B06 Rows without layers.** Element Move, Push, Thicken, the
  Transform tools and the app modes (Weld, Quick Align, Ground Align) are
  rows too, so the table lists every preset button, but their buttons still
  start them as before. Weld's snapping layer is its own fixed 15 px point
  snap, separate from the user's Snapping settings, so it takes nothing
  from the user and gives nothing back. Uniform Scale (`TransformUScale`)
  has no Operandi tool yet and has no row.

## Drag layer and Loop Slice (loop-slice-2026-11-15)

Builder's provisional readings; the spec's own questions are in
`specifications/loop-slice-2026-11-15/QUESTIONS.md`. Feel numbers are all in
`operandi_core::haul::feel`.

- **DL-B01 One-value buttons lock at once.** DL-04 says the first clear
  direction picks the value. When a button drives only one value (Loop
  Slice right = Count), a drag that starts a little up or down still
  changes it, by its sideways part, instead of doing nothing. With two
  values the first 4 px decide (DL-Q01). *Provisional.*
- **DL-B02 Trackpad keys (DL-Q04).** Cmd+left-drag = middle-drag,
  Cmd+Shift+left-drag = right-drag, only for tools whose row lists that
  button and only off the handles. Not Ctrl: on a Mac Ctrl+click is the
  real Ctrl and never a right-click (BIBLE Keys, FLAGS F1), and Ctrl
  already constrains drags. Cmd and Cmd+Shift do nothing to a live tool
  off its handles today (the Cmd menu and Cmd+Shift Fill only run with no
  tool). *Provisional.*
- **DL-B03 Left button kept for most tools.** Polygon Bevel, Edge Bevel,
  Chamfer edit, Vertex Bevel/Extrude and Smooth Shift already drive the
  spec's left values with a drag anywhere (DRAG-Q01) without an axis lock.
  Their table rows leave the left button out, so that drag is unchanged;
  moving it onto the layer is one line each in
  `presets/factory-hauls.toml`. Jason to decide.
- **DL-B04 Hauls go through the panel field.** Each table value is a Tool
  Properties field key; a haul writes the field's own setter, so limits
  and refusals match typing. A greyed field isn't hauled (the status says
  why). A value past a limit stops at the last one the tool took.
- **LS-B01 Inset with no profile (LS-Q10).** Every new loop point is pushed
  Inset metres against the mean normal of its edge's polygons (positive =
  groove). Works with Preserve Curvature. Profiles aren't built, so Inset
  is always live. *Provisional.*
- **LS-B02 Left drag only with Edit = Move (LS-21).** With Edit Add or
  Remove, a press off the bar picks a ring as before (LS-Q06 open).
- **LS-B03 Ring pick on release.** A left press off the bar that moves
  less than 4 px picks a ring when it comes up (it used to on the press).
- **LS-B04 Keep Quads (SL-Q04, LS-72).** Greyed until Slice Selected is on.
  It only touches selected polygons the slice points land on without being
  cut across (where the ring stops at a selected triangle or n-gon): each
  gets edges from a slice point three corners on, cutting off quads while
  it has an even number of sides above four. Odd ones (a pentagon after
  the cut) stay as they are: edges alone can't make them quads, and no new
  points are added. *Provisional.*
- **LS-B05 Split with no Gap where the cut stops inside the mesh.** With
  Slice Selected the last edge (next to an uncut polygon) keeps one point:
  the split closes there, so the neighbour stays 5-sided as LS-72 and the
  two sides meet at that point. With a Gap that edge takes both loops and
  the neighbour spans the gap. *Provisional.*
- **LS-B06 Cap Sections.** Only a ring that runs all the way round (closed)
  is capped: one polygon per side per slice, wound to match its
  neighbours, material and part from the piece next to it. *Provisional.*
- **LS-B07 Gap.** Each side moves half the Gap along every ring edge (in
  that edge's own length, so a 0.1 m Gap on a 1 m edge gives 45 %/55 %,
  LS-73). A Gap wider than the ring is refused. *Provisional* for edges of
  different lengths.
- **LS-B08 Profile cut (LS-40..LS-43, LS-74).** Operandi's own shapes from
  the shared Profile layer (`operandi_core::profile`): Round Groove, V
  Groove, Square Slot, T Slot, Saw Notch. Each profile point is one loop;
  x across the ring is the shape's own x (0..1 across each ring edge, so it
  is scaled to the ring's width and the symmetric shapes come out
  centred); depth is pushed against the edge's mean normal. Keep Aspect on:
  depth = shape depth x the ring edge's length; off: the deepest point
  goes Inset deep. Reverse Direction mirrors x; Reverse Inset flips in and
  out. Count and Position are set aside and greyed (LS-Q13, approved). Corner data along the edge is
  interpolated by an even spread between the first and last points' x,
  which keeps undercuts (T Slot) in order. *Provisional.*
- **LS-Q13 Count/Position with a profile.** *Approved (Jason,
  2026-11-16: "yeah you can grey them out"):* they change nothing while a
  profile is picked (Count stays 1, LS-74), so Current, Count, Position and
  the Uniform button grey in place with "The profile sets the loops"
  (UIC-92), like Inset. Noted in the spec's greying table on branch
  loop-slice-options.
- **LS-Q14 (new) Split with a profile.** Not measured. Built: every
  profile loop is split like a slice (so the groove comes apart along each
  loop, with Gap strips between). Is that what the reference does, or does
  Split separate only once across the whole profile?
- **LS-Q15 Slice Selected with unselected polygons between selected
  ones.** *Answered (measured 2026-11-20, spec section 8):* the polygon
  between isn't cut; it takes one point on each side it shares with a cut
  polygon. Each connected group of selected polygons is cut on its own; a
  lone selected polygon is cut across its first side (point 0 to 1) and
  the opposite one. Built.
- **LS-Q16 Slice Selected turning at corners (LS-32).** *Answered
  (measured 2026-11-20):* an L is cut down the middle of the strip, the
  corner polygon from the middle of its entry side to the middle of its
  exit side (a triangle and a five-sided rest), the same with Slice
  Selected off (the ends then run on round the ring). A 2x2 block gets a
  closed loop round its centre, each polygon cut corner to corner. Built
  (`selection_rings` in slice.rs).
- **LS-Q17 (open) 2x2 block with Slice Selected off.** The reference's cut
  leaves the selection and depends on pick order; no rule yet. Unchanged:
  the block's two directions cross and the tool waits for a click
  (SL-Q14).
- **LS-Q18 (new) Slice Selected on a group that branches.** A selected
  polygon sharing three or more sides with the rest (a T, a 2x3 or 3x3
  block) has no single middle line. Built *provisional:* that group is left
  uncut. What does the reference do?
- **LS-Q19 (new, forum report 2026-11-22) A lone polygon with Slice
  Selected off.** LS-01 needs two neighbouring polygons; a lone selected
  polygon (the default, Slice Selected off) gave no cut, which read as
  "Loop Slice refuses to cut on faces". Built *provisional:* each selected
  polygon that shares no side with another names the ring through its
  first side (point 0 to 1, as LS-75 does with Slice Selected on), and the
  cut runs on round that ring. Does the reference cut there, and across
  which side? Also open: a click on a polygon's middle (not near an edge)
  while Loop Slice is live only says "click an edge" (LS-03 covers edge
  clicks only).
- **LS-Q20 (new, GitHub #23) Slice Selected with edges selected.** LS-32
  names selected polygons only, so in Edge mode Slice Selected did nothing
  and the whole ring was cut. Built *provisional:* the cut crosses only the
  selected edges. A polygon is cut when both sides the cut crosses are
  selected; a selected edge with no such polygon cuts just the polygons on
  either side of it. Unselected polygons take the new points (LS-72). What
  does the reference do with edges selected?
- **LS-B09 Slices keep creation order (LS-71).** Free mode no longer sorts
  positions; Current, the bar's highlighted triangle and the middle-drag
  stepping follow the order the slices were added. Uniform and Symmetry
  still place theirs left to right.


## Bend v2 handles (bend-handles-2026-11-15, branch `bend-v2`, 2026-11-16)

Readings built for BH-Q01..Q11; all *provisional* until measured.
Start for each: Bend live on an upright target (capsule or strip).

- **BH-Q01..Q03** *Answered* by spec v2.1 section 5 (measured, BN-40..43):
  true arc, rigid past the end, +Angle leans toward spine × axis (a turn of
  -Angle about the axis). The v1 readings already matched; no maths
  changed. Tests: `bend_v21.rs` (BD-01..05).
- **BH-Q04** Starting spine: from the action centre up the work plane's Y,
  as long as the target's box along it. A target long along a ground axis
  still gets an upright spine.
- **BH-Q04 / BH-Q05 ANSWERED 2026-11-16** by BN-24 (revised): every centre
  click refits the spine along the work-plane axis with the larger
  far-side distance; the Axis goes to the plane's normal (BN-30b). The
  entries below are kept for history.
- **BH-Q05** After a click moves the centre, the spine keeps its vector, so
  its end moves with the centre.
- **BH-Q06** Ctrl while dragging the ring steps Angle by Rotation Snap
  Angle, like the Rotate ring (HM-20).
- **BH-Q07** The spine's end snaps to targets when snapping is on (not
  during a Ctrl one-axis drag). It and the centre may snap to the target's
  own points; points the bend has already moved are left out (SN2-17).
- **BH-Q08** A press off the handles moves the centre, and the rest of that
  drag keeps moving it (snapping each sample). It never bends.
- **BH-Q09** Deform Space: shown and kept (on by default), no effect.
- **BH-Q10** Starting Axis: the axis the old Auto turned around, from the
  spine: Z for a spine up Y, Y for a spine along X or Z.
- **BH-Q11** Move unchanged (still keeps its centre on a press on the
  model).
- **BH-Q12** (BN-45) Axis along the spine: the bend can't turn the spine
  round it, so the mesh stays unmoved and the status bar reads "The bend
  axis runs along the spine; pick another axis". *Provisional* until Jason
  decides.
- **BH-Q13** (BN-47) Values remembered between uses: Angle, Spine, axis and
  Both Sides come back (a remembered Angle bends at once); the centre still
  comes from the action centre, because Bend's pivot is the shared action
  centre (BN-02) and a remembered point could sit far from a new
  selection. Spec lists the centre as remembered too; which centre?
  *Provisional.*
- **BH-B01 Action Axis Auto.** Operandi has no separate Auto axis; the row
  sets World axes and Bend's own X/Y/Z row picks one. Replaces N01's
  Auto/X/Y/Z (BD-Q7).
- **BH-B02 Not built yet** (left out of the panel, UIC-117): Slip UVs and
  Morph (G9; Operandi has no shared transform Slip UVs / Morph setting to
  use yet) and the Action Axis Handle option (G10, shared Action Axis
  work). Both are later batches.
- **BH-B03 Labels.** The centre rows read "Center X / Y / Z" under the
  "Auto Action Center" heading (UIC-114 first-row naming) where the spec
  lists "X, Y, Z".

## Nits batch 2 (branch `nits-batch-2`, 2026-11-16)

- **EBC-Q11 update 2 (N08).** Edge Bevel no longer refuses an open strip
  whose end meets a sharp fold (Jason, 2026-11-16, from a reference result
  he described: the strip splits along its whole length and at each open
  end the two new edges close back to the end point). Where the end point
  can't slide (a slide edge leads over polygons more than 45° from the
  strip's), it is kept and both sides meet at it, as Edge Chamfer's kept
  point does; flat ends still slide as measured (UVN-T-01, EBC-B3-06).
  Open: whether the reference ever slides a bent end, and its threshold.

## UV editing stage 1 (branch `uv-stage-1`, 2026-11-16)

Specs: `specifications/uv-editing-2026-11-16` (UVE-1, UVT, UVS-1). Every
reading below is built and marked *provisional* in code.

- **UVE-1 Q1** New Map's default name when "Texture" exists: "Texture 2",
  "Texture 3", ... *Provisional.*
- **UVE-1 Q2** Deleting a map leaves Image Map layers and Texture
  Locators naming it as they are (they show the renderer's missing-map
  fallback). Renaming doesn't follow into them either, since locators name
  maps across meshes. *Provisional.*
- **UVE-1 Q3** Paste works on any mesh whose polygons have the copied
  mesh's corner counts (same topology). *Provisional.*
- **UVE-1 Q4** Fit UVs: Keep Proportion starts on, Gaps 0 %, the block sits
  at the region's lower-left corner, Gaps is a margin on every side.
  Alt+click (per Polygon, "as a Barycentric layout") stretches to fill.
  *Provisional.*
- **UVE-1 Q5 / UVT-52** Mirror Center starts at 0.5 and is remembered.
  *Provisional.*
- **UVE-1 Q6** Positive UV angles turn counter-clockwise. *Provisional.*
- **UVT-Q05** Snapping in a UV view: grid only, the action centre lands on
  0.1 steps while Snapping is on. *Provisional.*
- **UVT-Q03 (applied to UV transforms)** A position falloff (Linear,
  Radial) is read at (U, V, 0) in a UV view; Vertex Map and Selection
  falloffs read their points as in 3D. The falloff handles aren't drawn in
  the UV view yet. *Provisional.*
- **UVT-08** Ctrl/Cmd+click opens the dialog first for Flip, Mirror and
  Align as well as Fit (the guide says it for Fit). *Provisional.*
- **UV-B01 Action centre in UV space.** Origin = (0, 0); Local = each
  island's own centre; every other mode the target's bounding-box centre.
  A clicked centre (Element, Screen) isn't read in a UV view yet.
  *Provisional.*
- **UV-B02 Edge rows outside Edge mode** (Align's Alt+click): runs of
  selected points joined by mesh edges in Vertex mode, each island in
  Polygon mode. *Provisional.*
- **UV-B03 Rename — settled (UVT-200, 2026-11-16).** Double-click on a map
  in the UV Maps list opens the UV Editor on it; rename stays
  click-pause-click or right-click > Rename… (UIC-39).
- **UV-B04 Selecting in the UV view** selects mesh points, edges and
  polygons (shared with 3D), so clicking a point on a seam selects all its
  UV positions (UVT-03 "selects the UV positions under the cursor" needs a
  per-corner selection Operandi doesn't have yet). *Provisional.*
- **UV-B06 Layout bar** (UVT-200/201, UIC-11..18): only the Model and UV
  tabs; "+", reordering, stars, collapse, layout sets and the quick
  switcher aren't built. The Model layout is the user's own arrangement
  (no default to reset to); the UV layout isn't saved between launches
  (the app opens in Model). The toolbox's UV tab shows only in the UV
  layout, so the Model layout's nine tabs keep their full length. A
  separate layouts effort replaces this. *Superseded by layouts stage 1
  (LAY-Q1..Q10 below), 2026-11-16.*
- **UV-B07 UV Editor window** (UVT-200): one at a time; its tool strip has
  Move, Rotate, Scale, Transform, Unwrap, Pack and the island buttons
  ("the common UV tools", our pick). *Provisional.*
- **UV-B05 Checker** (UVT-114, approved Operandi extra, Jason 2026-11-16):
  plain polygons show it through the selected map; a polygon whose
  material reads an image shows it through the map that material names.
  *Provisional.*

## Nits batch 3 (branch `nits-batch-3`, 2026-11-16)

- **N11 Ctrl+G.** Jason asked for Ctrl+G grouping. *Provisional:* Ctrl+G
  runs Parent to Group Locator (GL-04) on the selected items. Unclear:
  whether the reference's Ctrl+G makes a group locator (hierarchy) or a
  Groups-tab group.

## Layouts stage 1 (layouts-2026-11-16)

- **LAY-Q1 Three columns only.** The file's frame tree holds the side
  panels (LAY-61), but this build draws only the three-column shape
  (toolbox over Tool Properties | views | two tab groups, UIC-196). A file
  that places panels another way is refused and the shipped default opens.
  *Provisional* until a later stage draws any tree.
- **LAY-Q2 When the open command runs.** When a layout opens from its file
  (first time in a session, at launch, after a reset), not when switching
  back to it: LAY-33 "exactly as left" wins, so a toolbox tab you picked in
  UV stays. *Provisional.*
- **LAY-Q3 Side widths as fractions.** They apply when a layout opens
  (launch, switch, reset). Resizing the window inside a session keeps the
  panels' widths in points as before, and the stored fraction follows. A
  fresh install at a width other than 1440 gets panels at the fraction
  (1280 wide: 217 pt instead of 244). *Provisional.*
- **LAY-Q4 Cameras.** Kept per view while switching in a session; not
  written to layout files, so a relaunch opens every view with the launch
  camera, as before. Views a layout opens fresh get the current camera in
  Perspective (as the UV layout did). *Provisional.*
- **LAY-Q5 Floating panels.** Window > Float side panels stay app-wide
  (panel-layout.json, LAY-60's separate file). Torn-off view windows,
  including the UV Editor, belong to the layout they were opened in.
  *Superseded by stage 6* (LAY-52 and the spec's LAY-Q2 answer: every
  floating window belongs to the app; see "Layouts stage 6" below).
- **LAY-Q6 Layout menu.** A new top-level Layout menu, after View, with a
  Layouts submenu; View > Layout's two lock entries stay where they are.
  *Provisional.*
- **LAY-Q7 Reset.** Double-clicking any layout tab asks, then resets it to
  its shipped file and deletes the user's copy. That now includes Model
  (before, Model had no default and refused). *Provisional.*
- **LAY-Q8 Per-layout panels.** Each layout has its own tab groups, Items
  filter, split fractions and side widths. UV opens with the default tabs
  plus UV Maps in front and the default widths, not the user's Model tabs
  and widths as the stopgap did. *Provisional.*
- **LAY-Q9 Missing kinds.** An unknown kind in a tab group or the view area
  is an empty panel showing its name ("(not in this version)") and is
  written back with its name; one in the left column makes the file
  refused (LAY-Q1). *Provisional.*
- **LAY-Q10 Reserved fields.** A split's `locked` and a tab group's
  `minimized` are read and written but unused; View > Layout's divider
  lock stays app-wide. A view's display settings are stored only when they
  differ from the new-view default. *Provisional.*

## Default colours (colors-2026-11-16 v2, branch `colour-defaults`, 2026-11-16)

Built: section 2 defaults (panels, buttons, fields, lists, selection,
tabs, view header) with new tokens (Tab Strip, Tab Text, Tree Lines, Items Header Text, Items Selected Text, view header button /
text / on); the interface sections listed first in Preferences > Colors
with Reset Section per section (per-swatch reset was already there);
section 3's stock Perspective / Orthographic / UV schemes as per-view-kind
colours (new "Orthographic Views" and "UV View" sections); D1-D4.

- **COL-B1 not built yet:** tooltip colours (#F0F0B4 / black / #FFAA00),
  scroll bar track and handle, field-being-edited colour and the tree's
  expand-arrow colour. egui draws tooltips, scroll bars and focused text
  fields from shared window and widget colours, so each needs its own
  drawing hook first. The arrow is a generator icon.
- **COL-B2 Perspective background:** Operandi has no "Deformers Active"
  state, so the Perspective view uses Background #383838 (D6 not chosen),
  not the #303030 the reference shows with deformers on.
- **COL-B3 Primary selection:** UIC-34 keeps the primary row a shade
  darker (#353535) than other selected rows (#3B3B3B, D4).
- **COL-B4 Per-view schemes:** each view kind has fixed scheme colours
  (editable); picking a different named scheme per view is not built.
- **COL-B5 State columns and header (Jason, 2026-11-16):** the narrow
  state columns and the column header row use the list background and
  stripe like the rest of the tree, not the spec's darker #303030 / #3A3A3A
  (they read as a near-black gutter). Header = list colour, editable.

Layouts stage 2 (BEHAVIOR-LAYOUTS-STAGE2.md, branch layouts-stage-2):

- **LAY-Q11 What a blank layout and the content arrow cover.** Arbitrary
  arrangements are built in the view area (middle column) only. A new
  blank layout (copy box off) and Clear Layout keep the side columns
  (toolbox / Tool Properties, the two tab groups) and make the view area
  one empty panel. The content arrow on a view-area panel offers 3D View,
  UV View and Empty; side-column kinds (Items, Properties, ...) are listed
  by category but greyed ("Opens in the side columns in this build").
  Tabbed Viewport is greyed. Spec: UIC-193/LAY-73/LAY-74 say any kind in
  any slot and "a blank window". *Provisional* until the side columns are
  part of the frame tree.
- **LAY-Q12 Rename, delete, revert by kind.** Only layouts you made can be
  renamed or deleted; only shipped layouts can be reverted (a layout you
  made has no default; Revert/Reset is greyed for it). Spec says Rename
  without limiting it. *Provisional.*
- **LAY-Q13 Shared side columns.** UIC-192/LAY-72: "Shipped layouts share
  one left column and one right column." Not built: each layout keeps its
  own tab groups as in stage 1 (LAY-Q8). Changing it would change what
  stage 1 users see when switching. **Decided (Jason 2026-11-16): keep
  the Model and UV side columns separate**, not shared, a deliberate
  deviation from UIC-192/LAY-72. Nothing to build.
- **LAY-Q14 Pieces.** A piece is a view-area panel (Save Viewport As) or
  the whole view area (Save Viewport Group As), saved under a name and a
  category in the data folder's `Pieces` folder. Saving it again updates
  every layout that shows it: slots tagged with that piece (their kind and
  settings) and layouts whose whole view area is that group piece.
  Switched-away layouts reopen from their updated files (their in-session
  cameras are dropped). A group piece placed into a slot that isn't the
  whole view area is a copy (not updated later). No pieces ship.
  *Provisional.*
- **LAY-Q15 Layouts button.** UIC-18's "Layouts button in the bottom
  toolbar": Operandi has no bottom toolbar, so the switcher opens from
  Layout > Recent Layouts…, Ctrl+Shift+Tab and the Layouts pie (Ctrl+`)
  only. *Not built.*
- **LAY-Q16 The switcher and the pie.** Ctrl+Shift+Tab opens the switcher
  with the next layout outlined; each further Tab moves on; releasing
  Ctrl or Shift switches. From the menu it waits: click, Return, or Esc;
  Tab/Shift+Tab and arrows move. The Layouts pie lists the bar's layouts
  (starred-only respected), first eight. Both keys are in the Input Map.
  *Provisional.*
- **LAY-Q17 Stars and the bar.** Model and UV ship starred (their files);
  a new layout isn't. With "starred only" on, the current layout's tab
  shows even when unstarred. The collapse control is a "‹" at the bar's
  left; the collapsed strip is a 6-point bar. Right-click on a tab: Rename,
  Delete, Reset. *Provisional.*
- **LAY-Q18 Locks per layout.** Layout > Lock Thumb / Divider Dragging
  belong to the layout showing (in layouts.json). The old app-wide
  layout-locks.json is read once as the starting value of every layout
  without its own; it isn't written any more. *Provisional.*
- **LAY-Q19 Content arrow on views.** The arrow is a small triangle at the
  view header's far right (the corner buttons move left by 14 points) and
  at the top right of UV views and empty panels; a click or right-click
  opens its menu. *Provisional* (it changes Model's header slightly).

### Layouts stage 3 (toolbox as forms, batch 1)

- **LAY-Q20 Button looks.** The old tabs draw buttons three ways (the
  toolbox's flat cells and rows, and standard framed buttons). To keep a
  converted tab paint-identical, a form control has a `look`: `toolbox`
  (default) or `framed`. A framed button in a vertical form shows its
  shortcut after its label; in a horizontal form it shows the label only.
  The spec names style and icon mode per form, not a per-button look.
  *Provisional*; unifying the looks would change what the tabs show and
  needs Jason's OK (UIC bible).
- **LAY-Q21 Proficiency level of the toolbox.** LAY-86's filter works on
  forms (Basic < Standard < Advanced, tested), but there is no Base Form
  Proficiency preference yet, so the toolbox shows everything (Advanced).
  *Provisional.*
- **LAY-Q22 Variant lines.** UIC-197's Alt/Ctrl variants are per button:
  a control may name a `shift`, `alt` and `ctrl` line (Ctrl is Cmd on
  macOS); a click with Ctrl/Cmd runs the ctrl line first, then Shift, then
  Alt (the island buttons' old order), else the plain line. The set-button
  row above the toolbox (Click = show, Alt+click = popover, Ctrl+click =
  palette) is not built; the tabs stay as they are. *Provisional.*
- **LAY-Q23 Island buttons as a command.** The UV island buttons had no
  command line, so they get `operandi.uv.preset <id> [plain|shift|alt|options]`
  (Operandi name, class undoable; `options` opens the dialog). Name is a
  placeholder for the reference name map (NAMES-NEEDED). *Provisional.*
- **LAY-Q24 Where user forms live.** One file per changed form,
  `Layouts/Forms/<form id>.json`, in the same Layouts data folder (a
  subfolder so it is never read as a layout). A copy equal to the shipped
  form isn't written. Revert is `operandi.form.revert <id>`; the editor
  that makes copies is stage 4. *Provisional.*
- **LAY-Q25 Sections.** A labelled divider starts a section that runs to
  the next labelled divider or the form's end; its fold state is kept by
  its `id` (default `<form id>/<label>`), so the UV tab keeps its old
  "uv-tools" fold key. Unlabelled dividers are plain separators.
  *Provisional.*
- **LAY-Q26 Gaps.** The old Select tab spaces its groups 6 points apart
  with no line. LAY-80 lists buttons, sub-forms and dividers only; forms
  get a fourth kind, `"space": <points>`, to keep the look. Or should a
  gap be a divider style? *Provisional.*
- **LAY-Q27 Flyouts.** UIC-197's flyouts (Select tab's Convert) are a
  sub-form with style `menu`: one button, labelled by the sub-form's
  control, opening a menu of its lines with their shortcuts. It greys
  (with the first reason) while none of its lines can run. *Provisional.*
- **LAY-Q28 Placeholders.** Rows for commands Operandi doesn't have are
  the line `operandi.notBuilt "<name>"` (class quiet), whose state answer
  is always "<name> · not in Operandi yet", so they stay grey without UI
  logic. A red unknown line (UIC-221) would read as a broken file.
  *Provisional.*
- **LAY-Q29 Select tab tooltips.** Tooltips hold `{shortcut}`, replaced
  by the line's current binding or "unassigned". While a tool is live
  the old rows added " · drop the active tool first" to the tooltip; now
  the command's reason ("Drop or Cancel the active tool first") shows
  instead, as on every form button. The Selection Sets rows run
  select.editSet / convertGroup / useSet / deleteSet since layouts-stage-3c
  (Jason: "make them work"). *Provisional.*
- **LAY-Q30 Tool flyouts (batch 3, Deform tab).** The Deform tab's
  hold-to-open flyouts are sub-forms of a new style, `flyout`: one
  button wearing one of its lines (the face, `face` on the control picks
  the first), click runs the face, hold (0.35 s) or right-click opens the
  list, a pick runs and becomes the face (kept for the session). Lit
  while any of its lines is on; greyed with the first reason only while
  none can run (UIC-223), so Soft Selection Move, whose four tools are
  all unbuilt, is now greyed (it was drawn live and said "Not built yet"
  on click). Every other row paints as before. *Provisional.*
- **LAY-Q31 Modifier-clicks and wording on Deform.** The old Deform rows
  refused Shift/Alt/Ctrl-clicks with "Deform tools don't support
  modifier-clicks yet"; as form buttons with no variants a modifier-click
  runs the plain line, like Element Move and Push already did. Unbuilt
  rows now say "<name> · not in Operandi yet" (LAY-Q28) instead of "Not
  built yet". *Provisional.*
- **LAY-Q32 Headings and flat tool rows (batch 3, Duplicate tab).** The
  Duplicate tab's Extend / Clone / Array / Duplicate titles are a new
  `heading` control (a title that doesn't fold). Its tool rows keep the
  flat look (no border) with `look: flat`; Deform's Element Move and Push
  keep the bordered tool button. The Align row's half-width buttons
  draw no shortcut (`noShortcut`), as before. *Provisional.*
- **LAY-Q33 New commands for the Vertex and Edge tabs (batch 4).** The
  rows that called the app directly now run commands: `Add Loop` and
  `Add Point` (Operandi actions; arm, or disarm when armed, like the old
  rows; `tool.set edge.addLoop on` still only arms) and `Drop Weld`
  (turns Weld off; unavailable while Weld is off, which is what shows the
  tab's Drop Weld button, LAY-85). The Edge tab's Set Position... runs the
  shared `Set Position` action, so it now shows that action's binding if
  one is set (the old row's key had no binding). Names provisional until
  the reference name map has entries. *Provisional.*
- **LAY-Q34 Greyed rows' tooltips (batch 4).** New Vertex... and Reorder
  Index... grey outside Vertex mode from their commands' answers, with the
  same reason ("Switch to Vertices mode first"). Hovered while greyed they
  now show the row's tooltip under the reason, as every form button does
  (the old rows showed the reason alone). *Provisional.*
- **LAY-Q35 Checkbox and plain-button looks (batch 4).** Show Vertices
  stays a checkbox: `look: check`, ticked from the command's lit answer
  (new: Show Vertices lights while the setting is on), a click runs the
  line. Drop Weld stays a natural-width button with Drop's shortcut:
  `look: button`. Tooltips that ended in the binding as buttons draw it
  ("  [Ctrl+L] ") use the new `{binding}` placeholder. *Provisional.*
- **LAY-Q36 Labelled variants (batch 5, Polygon tab).** Set Material...
  shows Set Part... while Alt (Option) is held, as before (MB). In a form
  a variant may now be `{"line", "label", "tooltip"}` instead of a bare
  line: while exactly its key is held the button wears that label, lights
  and greys from that line and shows that tooltip; a click runs what was
  shown at the press. Keys follow the MB rules (physical Control is Ctrl,
  Cmd never counts), not the bare variants' Ctrl-or-Cmd. `{variants}` in
  a tooltip lists them ("Option: Set Part..."). The MB table no longer
  has a Set Material entry. *Provisional.*
- **LAY-Q37 Modifier-clicks and lit rows on Polygon (batch 5).** The old
  rows refused Shift/Alt/Ctrl-clicks ("... doesn't support modifier-clicks
  yet"); as form buttons with no variants a modifier-click runs the plain
  line (LAY-Q31's rule). Pen, Spike, Polygon Reduction, Set Position,
  Scale Offset, Flip It, Flip It Origin, Set Part and Set Material light
  from new answers. Because the Edge tab's Set Position... runs the same
  `Set Position` line, it now also lights while Set Vertex Position is
  open. Flip, Spin Quad, Unify..., Split, Quadruple and Align keep their
  natural-width framed look as `look: natural`. *Provisional.*
- **LAY-Q38 Pen tooltip (batch 5).** The Pen row's tooltip was "Polygon
  Pen · Unassigned" (or "Polygon Pen ·   [P] "); it is now
  "Polygon Pen · {shortcut}", the binding as text or "unassigned", like
  the other form tooltips. The only visible change on the tab.
  *Provisional.*
- **LAY-Q39 Edit tab (batch 6).** The Slide button is two show-when
  sub-forms on a new quiet query command, `operandi.mode.is <modes>`
  (available only in those selection modes), so it wears Vertex Slide's
  icon and key in Vertex mode and Edge Slide's otherwise. Both run the new
  `Edit Slide` (class tool), which greys outside Vertex and Edge mode with
  the old reason. A control's new `shortcut` names whose binding it shows.
  Alt on SDS Subdivide runs the new `SDS Subdivide 2X` (undoable); Alt on
  Center X / Y / Z runs `vert.center xy|yz|zx`. Freeze... runs bare
  `poly.freeze` (its dialog). The UV operations are one shared form,
  `toolbox.uv.ops`, in both the UV and Edit tabs, so a user copy changes
  both. Dicer and Julienne are `operandi.notBuilt` placeholders and now say
  "<name> · not in Operandi yet" (they gave the wave 4 reason). *Provisional.*
- **LAY-Q40 Sculpt tab (batch 6).** Three escape hatches (in
  `command_form::WIDGETS`): the Tangent / Center Pinch slot (its face is a
  setting, not a session memory like a tool flyout's), the Weight Map
  drop-down and the New Weight Map… in-menu form. Widgets may now sit in a
  horizontal row. Vertex Map Tools and Tips are `look: weak` headings.
  Brush tooltips use the new `{chord}` placeholder (the binding as the
  input map displays it, or "Unassigned"). *Provisional.*
- **LAY-Q41 Create tab left for its own batch (batch 6).** Its primitive
  cells wear MB variants that draw an Item / Unit corner tag and run unit
  primitives that have no command lines yet (about eight new commands),
  Cube / Sphere / Cylinder keep their variants inside the tool button, and
  the Tube cell has its own hold popup with Solid Sketch. Each needs a new
  form piece; Create stays drawn by its code until then.
- **LAY-Q42 Unit primitive commands (batch 7, Create tab).** The
  primitives' Shift and Ctrl variants run twelve new lines named like the
  existing Ellipsoid ones: `Cube Unit` / `Cube New Unit`, and the same for
  Sphere, Cylinder, Cone, Torus and Capsule (class undoable). `<Name> Unit`
  adds a 1 m primitive to the current mesh, `<Name> New Unit` as a new mesh
  item, as the old variant route did. Names provisional until the reference
  name map has entries. `Item Merge` now says "Merge: select two or more
  meshes..." itself when fewer are selected (the old button checked first),
  so a typed or scripted line says it too. *Provisional.*
- **LAY-Q43 Corner tags and tool-button variants (batch 7).** A labelled
  variant may carry a `tag`, drawn in the cell's top-left corner while it
  shows ("Item" for Shift, "Unit" for Ctrl). Cube / Sphere / Cylinder are
  tool buttons with labelled variants: the bordered tool cell wears the
  shown variant (its tag, its line's binding) and the form runs what was
  shown at the press. The MB variant table has no primitive entries any
  more. Like the old variant buttons, a labelled-variant click does
  nothing while a modal, menu, navigation drag or layout gesture owns the
  input; this now applies to the Polygon tab's Set Material... too.
  *Provisional.*
- **LAY-Q44 Hold cells (batch 7, Tube).** A sub-form of the new style
  `hold`: a cell whose click runs its first line; right-click, or holding
  the press 0.8 s (the old Tube cell's time; tool flyouts use 0.35 s),
  opens a menu of all its lines, each lit from its answer. It wears the
  icon of its first lit line, lights while any is on, has the small corner
  cue, and may sit in a horizontal row. Unlike a tool flyout it doesn't
  remember a pick: a click always runs Tube, as before. *Provisional.*
- **LAY-Q45 Row columns, tall rows and binding placeholders (batch 7).** A
  horizontal form's `columns` shares the width as if it had that many
  controls (Draw's Arc / Tube / Pen row keeps the grid's quarter cells).
  `look: tall` is a text row at the icon rows' height (Snaps and
  Precision). Tooltips gain `{binding:<line>}` (another line's binding as
  buttons draw it: Tube names Solid Sketch's key, each variant names its
  primitive's key, as before) and `{andBinding}` (" · " and the binding,
  or nothing when unbound). *Provisional.*
- **LAY-Q46 Create tab changes (batch 7).** Paint-identical to the old tab
  in 1196 comparisons plus the Tube menu open. Visible only with a user
  binding: the Pen cell's tooltip ends in the binding as the input map
  shows it ("P", via `{chord}`) instead of "  [P] "; and a user-bound unit
  line (e.g. `Cube Unit`) shows its key in the cell while its variant
  shows (there was no such line before). Text and Arc no longer refuse
  modifier-clicks; they run the plain line (LAY-Q31's rule). The
  Primitives and Draw fold states sit with the other sections' (neither is
  saved between runs, as before). No escape hatches. *Provisional.*

### Layouts stage 4: workbench edit mode (BEHAVIOR-LAYOUTS-STAGE4.md)

- **LAY-Q47 The top workbench and the new tab.** Start: the shipped Model
  layout. The spec puts an empty workbench in the toolbox and one "along
  the top". Built: the toolbox has a Workbench tab, last in the strip,
  that joins the strip the first time it's opened (Layout > Workbench,
  Edit Workbench or `operandi.toolbox.tab workbench`) and stays for the
  session, as the UV tab does. Always showing it made the nine Model tabs
  shorter in any window up to about 1080 pt tall (the strip only just
  fits nine; the PAR-014 strip tests caught it), which the "looks
  identical" build rule forbids. The top one (`toolbar.workbench`) is a
  slim (26 pt) row under the menu bar, hidden until Layout > Top
  Workbench (`operandi.workbench.topBar`) shows it. Unclear: whether
  either should show from the start. *Provisional; Jason to choose.*
- **LAY-Q48 Workbench undo and where edits live.** Start: a workbench
  with items. Built: every edit writes the user's copy of the form at
  once (LAY-87; Revert drops it). Workbench edits aren't scene undo
  steps (Cmd+Z undoes the scene, as with layouts); `operandi.workbench.undo`
  (Undo Edit in the edit tools) takes them back for this session.
  Revert also forgets that undo. *Provisional.*
- **LAY-Q49 Popup face.** Built as a stored, saved choice (Default,
  Pop-up, Buttons) on any button; forms have no choice control yet, so
  every face draws as a button. Unclear: what each face draws for a
  command with a list of values. *Provisional.*
- **LAY-Q50 Alignment, picked items and the edit view.** Built: a
  workbench lays its buttons in rows of three units, Normal one, Wide two,
  Full Width three (a row breaks when the next one doesn't fit). Picked
  items start Full Width so they read like toolbox rows; Normal is the
  file default. While editing, items show one per row (chips along the
  top) so they can be dragged and right-clicked; the units show again
  when editing ends. A picked toolbox button takes the label, icon and
  tooltip of the first form button running that line. Add Divider with
  Label names it "Section N"; Label in its right-click menu renames it.
  *Provisional.*
- **LAY-Q51 Tool pipe preset.** Start: a tool open with an action centre,
  a falloff and symmetry set. Built: Add Tool Pipe ▸ As Preset adds one
  button running `cmds.batch` with, in pipe order, the action centre
  (`action center <name>`, when it is one the menu sets directly), the
  falloff type (`Falloff linear|radial|none`, which uses the falloff's
  remembered preset, not its exact handles), the symmetry axis, then the
  tool's line. Weight-map, Invert and Selection falloffs, Element and
  Screen centres, the tool's own values, snapping and the constraint
  aren't recorded yet. As Macro and As Script are greyed, "not in
  Operandi yet" (no macro recorder or script runner). *Provisional.*
- **LAY-Q52 Pick to Add sources.** Built: a click on any button or menu
  item whose line runs through `execute` (the toolbox, menus, forms), a
  Tool Properties row (added as a property control that shows the live
  row while that tool is open, its name greyed otherwise), a Preset
  Browser entry (a `preset.do` button) or a Command History line. Keys,
  viewport clicks, the macOS menu bar and the workbench's own commands
  still run. A picked
  button doesn't run. Buttons whose action isn't a line (a few older
  menus that call the app directly, panel widgets) aren't picked; their
  clicks run as usual. *Provisional.*
- **LAY-Q53 Customizable Workbench flag.** Built as `"workbench": true` in
  a form file; any such form draws with Edit Workbench and its edit
  tools. No UI sets it until the Form Editor (stage 7); Show in Form
  Editor is greyed. *Provisional.*
- **LAY-Q54 Icon-only workbench buttons (UIC-212).** Start: a workbench
  button, editing, right-click. The Label field starts from the label
  shown (the tool's or command's name by default). Built: clearing it
  (empty, Enter) makes the button icon-only (`"noLabel": true`); icon-only
  buttons take half a slot, so six fit a row at Normal, Wide takes two,
  Full Width the whole row. The tooltip still names the button. **Reset
  Label** brings back the default name; typing the default name does the
  same. A button with no icon to show (no tool icon, slug or custom
  picture) or drawn as a checkbox keeps showing its name, never a blank
  button. A divider's Label edits its section text. Unclear: whether the
  reference has an icon-only state, how it sizes it, and whether Reset
  Label is offered. *Provisional.*

### Layouts stage 5: per-layout key maps (BEHAVIOR-LAYOUTS-STAGE5.md)

- **LAY-Q55 Lookup order (LAY-20, UIC-201).** Start: a layout sets a key
  for one action. Built: the dispatcher's existing conditions (selection
  mode, tool active, pointer over a view, Items list focused, Render
  window focused) are checked as before, on one map: the global map with
  the layout's keys on top. A layout key on action A replaces A's global
  key, and in that layout hides every global action with the same chord
  that can apply at the same time as A (`input::contexts_overlap`).
  Global actions sharing the chord in other contexts keep it (Edge Bevel's
  B beside a layout's Polygon Bevel B; the Render window's keys). So
  "layout, then the per-mode/tool contexts, then global" reads as: within
  each context the layout's key wins; a layout never reaches into a
  context its action can't apply in. The layout's context is the `keys`
  map in its file (LAY-71), named after the layout; the reserved
  `key_context` name field stays unused. Unclear: whether the reference
  lets a layout context shadow keys across modes. *Provisional.*
- **LAY-Q56 Revert and the layout's keys.** Start: a shipped layout with
  a changed arrangement and its own keys; Layout > Reset Layout (or a
  double-click on its tab). Built: Revert puts the arrangement back and
  **keeps** the keys; the question has an unticked "Also clear its N
  layout keys" box, and the status line says the keys were kept.
  `operandi.layout.revert keys:true` clears them too. The Input Map's
  **Clear this layout's keys** and `operandi.layout.clearKeys` remove them
  on their own. Unclear: what the reference's layout reset does with a
  layout's key context. *Provisional.*
- **LAY-Q57 New, Save As and copies.** Start: a layout with keys; Layout >
  New (blank) or Save Layout As / New with copy. Built: a copy carries the
  keys; a blank new layout starts with none. Rename keeps them; Delete
  removes them with the file. *Provisional.*
- **LAY-Q58 The Input Map's two scopes.** Start: Input Map, "Keys for".
  Built: Global (as before) or "This layout (<name>)", always the layout
  showing. In the layout scope a row shows the key that fires there;
  Clear makes it unbound *in this layout* (an empty chord); **Use Global**
  drops the layout's key. Rows the layout sets show ◆, rows whose global
  key it hides show ◇, in both scopes; tooltips name the layout and the
  global key. Reset All to Defaults (global scope only) leaves layouts'
  keys alone. Save writes both maps. Two layout keys on one chord in one
  context are a clash, as in the global map. Unknown actions and chords
  that don't read in a layout file are kept but do nothing. Unclear: the
  reference editor's presentation. *Provisional.*
- **LAY-Q59 Project input-map presets.** Start: File > Project, save or
  load the Input Map preset. Built: the preset is the global map only; a
  layout's keys travel with its layout file. *Provisional.*

### Layouts stage 6: palettes as layouts (BEHAVIOR-LAYOUTS-STAGE6.md)

- **Spec LAY-Q2 (torn-off views on a layout switch): ANSWERED 2026-11-17**
  (behaviour only, via the coordinator). This is the stage 6 spec's
  LAY-Q2, not this file's stage 1 LAY-Q2 above. A floating (torn-off) view
  belongs to the app, like a palette: a switch never touches it (same
  place, size, camera and shading); the layout switched to shows its own
  views; switching back to the layout it was torn from opens that layout's
  saved version, so the slot gets a fresh view (shading from the file,
  camera as it was at the tear-off) and the floating one stays: two views.
  Closing the floating window is permanent. Built that way
  (`palettes.rs`, `carry_floating` = `take_floating` / `put_back_floating`,
  `open_torn_from`); no switch left to flip. Floating views now live in
  palettes.json, not in layout files (a file that lists some still opens
  them, once).
- **LAY-Q60 "The saved version" after a tear-off.** Start: Model with two
  views; tear one off; work on in Model; switch away and back. Built: the
  version that opens is Model's file as it was the frame before the
  tear-off, so changes made in Model after the tear-off (and before
  leaving) are not kept. Several tear-offs from one layout: the earliest
  file opens (it has every slot); only the first tear-off's slot gets the
  torn view's camera, the others the layout's fresh camera. Origins are
  not kept over a restart: after a relaunch, switching back opens the
  layout's current file. Docking the floating view back into the main
  window drops its origin. Unclear: whether "saved version" means the file
  before the tear-off or the latest one. *Provisional.*
- **LAY-Q61 Toggle Palettes and opening one.** Start: palettes hidden with
  ` (LAY-53). Built: opening any palette (menu, `layout.window`, the
  double-click in UV Maps) shows them all again and brings that one
  forward; toggling a hidden palette's menu item shows them rather than
  closing it. ` with nothing open says so and does nothing. The small
  pop-up palettes (Snaps, Absolute, Work Plane, Preview's) hide with the
  rest. Torn-off views are not palettes and stay. *Provisional.*
- **LAY-Q62 Images and Channels palettes.** LAY-51 gives them sizes
  (400 x 600) but Operandi has no Images or Channels panel yet, so they
  are not shipped (no empty windows). Adding the panel kind and a palette
  file is all they need. *Open: needs the panels.*
- **LAY-Q63 Extra palettes and names.** Built: a Command History palette
  (400 x 600, not in LAY-51); "Shading" shows the Shader Tree panel;
  "Vertex Maps" shows the UV Maps list (the only vertex map list so far);
  the "UV View palette" is a plain UV view without the UV Editor's tool
  strip (900 x 600). Panel palettes draw a second copy of the panel; the
  docked one stays. Toolbox / Items / Properties take the panel out of the
  side column while they float, as Float did. *Provisional.*
- **LAY-Q64 The UV Maps double-click.** Built: it opens the UV Editor (or
  brings it forward); it never closes it. Texture > Open UV Editor and
  `layout.window hash:uvEditor` toggle. *Provisional.*
- **LAY-Q65 Palette ids.** `layout.window hash:` takes presetBrowser (as
  before), uvEditor, uvView, toolbox, items, properties, shading, groups,
  vertexMaps, commandHistory (build names). Toggle Palettes is the Input
  Map action "Toggle Palettes" (`) and `operandi.layout.togglePalettes`
  (placeholder, NAMES-NEEDED.md). The Window menu's Float entries and
  Preset Browser moved to Layout > Windows/Palettes (UIC-184).
  *Provisional.*

### Layouts stage 7: menus as forms and the Form Editor (BEHAVIOR-LAYOUTS-STAGE7.md)

- **LAY-Q66 Menus converted, menus left hard-coded.** Built: Texture,
  Render and Help are forms (`menu.texture`, `menu.render`, `menu.help`)
  and System (new) is `menu.system`; each was checked identical to the
  old code (titles, shortcuts, greying reasons, ticks, no native key
  equivalents; `form_editor_tests.rs`) and the old code deleted. Their
  greying and ticks now come from the commands' own answers (UIC-223), so
  the lines are recorded in the Command History as menu lines (UIC-224).
  Left hard-coded, logged per the spec: File (recent files, open scenes,
  project presets and fixed File key equivalents), Edit (undo/redo
  reasons from the history, snapping popovers), Select (modes, tag kinds,
  lasso styles and Partial, which has no command), Item and Geometry
  (submenus: a menu's title is a fixed string, so a sub-form can't be a
  submenu yet), Tools (tool lists built from tool tables), View (per-view
  answers under the pointer, the Light submenu), Layout (the layout and
  palette lists change at run time) and Window (Compact / Standard have no
  commands). The in-window menu bar on Windows and Linux draws Help and
  System from the same model; its other menus stay as they were.
  Toolbars: the top workbench row was already a form; the top toolbar is
  hard-coded (left as is). *Provisional.* (2026-11-17, menus as forms:
  every menu is a form now, LAY-Q71..Q80.)
- **LAY-Q67 Save Form.** "Save Form to its own file" writes the form's
  JSON (`operandi.form.export`) to filename:, else to the data folder's
  `Saved Forms/<id>.json` (no file panel yet). Edits themselves are saved
  as the user's copy as they are made (LAY-87). *Provisional.*
- **LAY-Q68 Groups.** A form's group is its id's first word (toolbox →
  Toolbox, toolbar → Toolbars, menu → Menus, else Forms) unless the user
  moved it with the Group property or made it in a group; user groups are
  kept in `Layouts/form-groups.json`. Groups never change behaviour.
  *Provisional.*
- **LAY-Q69 Find Form.** The next primary click on a button (a toolbox
  button, a workbench button, an in-window menu item) selects the form
  and control that runs its line, preferring the toolbox tab showing,
  instead of running it. Clicks on headings, gaps, panels or the native
  macOS menu bar aren't caught (they run no line through the app's click
  path). Esc stops. *Provisional.*
- **LAY-Q70 Form Editor properties.** A control shows the workbench
  right-click's properties (label, command, tooltip, icon file, width,
  toggle, pop-up, level) plus icon, shortcut and, for a divider, show only
  when; a form shows label, style, icons, icon size, Customizable
  Workbench, show only when, slot (form, head/tail, sort), columns and
  group. Text fields set on Enter or when they lose focus. Edits share the
  workbench's Undo Edit; Revert drops the user's copy (on a form you made,
  it deletes the form). Preview Form draws the form at toolbox width
  under the properties; its buttons work. *Provisional.*

### Menus as forms (follows layouts stage 7, LAY-Q66; UIC-10, UIC-210, UIC-213)

LAY-Q66's hard-coded menus are all forms now; nothing in the menu bar is
hand-written (menu_forms.rs).

- **LAY-Q71 The bar and its forms.** Built: the bar is the form
  `menu.bar`, each of its sub-forms one menu (`menu.file` ...
  `menu.help`) titled with the form's label; each submenu is a form of
  its own (`menu.file.export`, `menu.layout.palettes`, ...), so the Form
  Editor shows and edits it. Changing a menu form's label renames the
  menu. Menus show every level (Advanced); a form's showWhen hides a
  whole menu or submenu; a divider in a menu is a plain separator (a
  labelled divider's showWhen isn't read there). *Provisional.*
- **LAY-Q72 Live lists.** Built: a `list` control fills a menu section
  from a named source: `scenes` (File > Current), `recentProjects`,
  `layouts` (Layout > Layouts) and `palettes` (Layout > Windows/
  Palettes). Each gives every entry its label, line, tick and greying.
  No recent projects: one greyed "No Recent Projects" entry, as before.
  An unknown source shows one greyed "<source> (live)" entry. In the
  Form Editor a list is one item, "Recent Projects (live)", that moves
  and deletes as a whole and has no properties beyond Level.
  *Provisional.*
- **LAY-Q73 Run-time titles.** Built: `{title}` in a label is the line's
  answered title, `{title:<line>}` another line's (a command's answer
  gained a title, `CommandState::title`); a line that answers none shows
  the line. Used by Edit > Undo / Redo ("Undo Cube") and View > Maximise
  (Four Views / Maximise Viewport / Restore Viewport Layout); submenu
  labels take `{title:<line>}` too. *Provisional.*
- **LAY-Q74 New command names.** Built: every menu entry runs a registry
  command (UIC-164 has no menu gaps left), so every menu click is a
  Command History line (UIC-160, UIC-224). 18 placeholders (NAMES-NEEDED.md)
  plus two name map names: `viewport.split` (View > Split View) and
  `viewport.detach` (Tear Off View); their direction:/keep:/fromMouse:
  aren't read yet. The phrase lines "Split View" and "Tear Off View" only
  reached the Split and Tear tools by prefix, so the menus don't use
  them; a test checks every menu line names its command exactly.
  *Provisional.*
- **LAY-Q75 Bare lines open dialogs (CMD-12).** Built: bare `mesh.cleanup`
  opens Mesh Cleanup (it was an error); bare `operandi.layout.create`,
  `.saveAs` and `.rename` open their dialogs. `operandi.layout.delete`
  and `.revert` keep their bare behaviour (scripts use it) and take
  `ask:true` for the menu's confirmation, rather than CMD-13's `+`
  prefix, which is unconfirmed. Bare `preset.saveAs` follows the Preset
  Browser's gear toggle for `mtlx:` and bare `operandi.materialx.import`
  goes above the selected shader layer, as the menu lines did.
  *Provisional.*
- **LAY-Q76 Greying from the commands' answers (UIC-223).** Built, and
  visible: Boolean, Solid Drill, Axis Drill, Remesh and Automatic
  Retopology grey in Geometry > Polygon while a tool is active (their
  toolbox buttons already did; the old menu didn't). Cone in Item >
  Primitive ticks while Cone is armed. The other way round, the
  menus' rule became the commands': the Select tab's selection rows,
  Edit's Lock and View's Hide now also grey while an Ellipsoid, shape,
  Spike or Bridge session is open, and Center's Item-mode reason is
  `vert.center`'s own. *Provisional.*
- **LAY-Q77 The in-window bar (Windows, Linux).** Built: it draws all
  thirteen menus from the same forms as the macOS bar, with submenus;
  its short hand-written File/Edit/Select/Item/Geometry/Tools copies and
  the Window menu's own Float checkboxes are gone (the stage 6 loose
  end: Float is Layout > Windows/Palettes, as on macOS). Its entries no
  longer grey themselves while their own menu is open (stage 7's
  in-window System and Help entries did nothing for that reason). About
  Operandi still heads Help there. *Provisional.*
- **LAY-Q78 Small differences in what an entry does.** Item > Create
  Environment runs `item.create environment`: it also lands in Add
  Item's recent list and says "New Environment" in the status line.
  *Provisional.*
- **LAY-Q79 The macOS bar.** Built: it updates titles, ticks and greying
  in place, and rebuilds the menus (never while one is open) when their
  shape changes: a live list grows, a menu form is edited. Native key
  equivalents are a control's `keyEquivalent` (File's Save, Close Scene,
  Close All only, as before). Quit and About stay AppKit's app menu.
  *Provisional.*
- **LAY-Q80 Escape hatches.** None needed: `command_form::WIDGETS` is
  unchanged and no menu entry is custom-drawn.

## Click to start (tool-click-start-2026-11-16)

- **CS-Q01 DECIDED 2026-11-16 (Jason: consistency over the reference).**
  Polygon Bevel, Edge Bevel, Vertex Bevel and Edge Chamfer wait for the
  first click like the other tools (CS-07), although the reference starts
  them at once. An approved deviation, marked in the flag table. Vertex
  Extrude still starts at once. Once live, their drag anywhere and re-drag
  work as before.
  - **Revisited 2026-11-23, approved 2026-11-26 (UIC-77):**
    Polygon, Edge and Vertex Bevel start at once again, as the spec's
    table has it. Pressing B showed the Tool Properties but no gizmo on the
    selection until a click in the view, which a tester reported as a bug.
    Vertex Bevel also gets its Inset arrow (see ACS-Q-HANDLE1), so all
    three show a gizmo when picked. Edge Chamfer still waits for the click.
- **CS-Q02** The first click lands on the mesh surface under the pointer
  even with snapping off (snapping on: the snapped point; off the mesh:
  the work plane). *Provisional.*
- **CS-Q03** Apply on an armed tool keeps the panel's values; the tool
  stays armed on the result, so a later click (or Apply) runs those values
  again on the new mesh. Panel edits and `tool.attr` on an armed tool only
  store values, from scripts too. *Provisional.*
- **CS-Q04** The drag that follows the activating press is the tool's
  normal off-handle drag (Bevel: drag anywhere; Bend: moves the centre and
  refits; Taper/Shear: the Scale/Move free drag). *Provisional.*
- Also provisional on this branch:
  - CS-11: only Bend has its own Auto axis row, so it's the only tool
    whose axis the click sets; other tools keep their action axis.
  - CS-13: Linear/Radial falloffs keep the fit made when the row was
    picked (TP-17); the target can't change while armed.
  - CS-20: a later click away from the handles refits Bend only; other
    tools keep today's off-handle press.
  - "Magnet" is read as row `xfrm.magnet` (its button says Soft Move) and
    waits for a click.
  - Not moved onto the shared layer: primitives (they draw from the
    press), the Duplicate tools (DS-30/31: their Apply makes the copies
    and then values edit them live, unlike CS-21) and the slices (they cut
    where clicked). Their flags are off in the table; their own waiting
    is unchanged. Not in Operandi yet or not pipe tools: Soft Drag,
    Element Push, Jitter, Quantize, Bridge (a session of its own), Pen
    (draft of its own), Snap Drag, Drag Weld (app mode).

## Command History, stage H (ui-architecture-2026-11-16; CMD, HIS, UIC-224..229)

- **CH-Q01** A line's outcome: refused when it fails through the shared
  run path (`run_ok`), a parser's `fail`, or an unknown name. A few older
  parsers set the status text without saying they failed; those lines
  are recorded as succeeded with their status as the message.
  *Provisional.*
- **CH-Q02** `history.max` defaults to 1000 lines and
  `history.resultsMax` to 200; both last for the session only (not
  preferences). *Provisional; reference defaults unknown.*
- **CH-Q03** Argument names: `history.max count:`, `history.resultsMax
  count:`, `history.clear results:` (Operandi's way to clear Results),
  `cmds.undoTo index:` (the undo length to leave), `cmds.batch label:
  c1:..c9:` (nine lines at most). *Provisional; the reference's names
  aren't in the local docs.*
- **CH-Q04** A tool session records `tool.set <tool> on`, one `tool.attr
  <tool> <field> <value>` per Tool Properties value that changed (field
  keys as the panel names them, script units), the line that ended it as
  fired (`tool.drop`, `Drop`), then `tool.set <tool> off`. Values are
  compared when a mouse button is released, a key is pressed and around
  each command, never per mouse move. Does the reference write
  `tool.doApply` on Drop as well? *Provisional.*
- **CH-Q05** Class marks in the Undos, History and Results rows are the
  tree list's letter tags (U undoable, R redoable, I UI, S side effect,
  Q quiet) until the icon generator has recipes for them.
- **CH-Q06** The Undos tab lists document steps. Tweaks inside a live
  tool (mid-tool undo) aren't rows of their own; undoing a tweak takes
  its `tool.attr` lines back out of the session. Pen points, Solid Sketch
  edits and Polygon Reduction steps keep no lines of their own; their
  dropped step keeps the session's lines. *Provisional.*
- **CH-Q07** "Show UI Only Commands" / "Show Side Effect Commands" in the
  undo list (CMD-01) aren't built: UI and side-effect commands make no
  undo steps here, so they show only in History.
- **CH-Q08** The Commands tab shows internal names; the option to show
  user names instead (HIS-30) waits for user names in the registry.
- **CH-Q09** Lines double-clicked in the panel are recorded with source
  "typed" (they go to Results, HIS-50); the drag row's `cmds.undoTo` is
  "button".
- **CH-Q10** MCP bridge calls other than `command_run` are recorded as
  `<tool> <JSON arguments>`, which is not a registry line (they can't be
  rerun from the History yet).
- **CH-Q11** Undo, Redo and `cmds.undoTo` are class quiet (CMD-02: never
  undo steps; they show in History only).

## UV transform pipe (branch `uv-transform-pipe`, 2026-11-16)

Specs: `specifications/uv-2026-11-16` (BRIEF-TRANSFORM-PIPE-IN-UV,
ADDENDUM-GENERAL-TOOLS-IN-UV, UVT-130..139). The three guesses the brief
names are one constant each in `operandi-core/src/uv/uv_pipe.rs`.

- **UVT-Q11** Range in a UV view is in UV units (`RANGE_UNITS`; `Pixels`
  is built too: Range then reads as UV-view pixels). *Provisional.*
- **UVT-Q12** Use Connectivity / Rigid mean the UV island
  (`CONNECTIVITY`; `Mesh` reads 3D connectivity instead). *Provisional.*
- **UVT rotate direction** Positive angles turn counter-clockwise, U right
  and V up (`ROTATE_COUNTER_CLOCKWISE`, as UVE-1 Q6). *Provisional.*
- **UVT-Q13** UVT-131 "a click with no drag shows the handles on it":
  not built; a click on an element with Element Move does nothing yet.
  *Provisional.*
- **UVT-Q14** Soft Drag, Jitter and the Screen falloff aren't built in 3D
  yet (the Deform tab shows them greyed), so they don't run on UVs either.
  Once built they need only `uv_pipe::runs_on_uvs`. *Open: next batch.*
- **UVT-Q15** Action centres (UVT-133): Selection, Automatic and Border
  are worked out on the UV proxy as in 3D (Selection = the selected UV
  positions' average); Local turns each island about its own centre;
  Screen falls back to the selection centre (build guess). Element takes a
  UV point or edge middle clicked in the UV view while the tool runs
  there. Axes are always U, V and the view axis. *Provisional.*
- **UVT-Q16** Slide (UVT-137, edge mode): started in a UV view it slides
  the current map's UVs along their UV edges. The Slide panel's UV Map
  field (sliding a map's UVs from a 3D view) isn't built. *Provisional.*
- **UVT-Q17** Element Move's Range field keeps its " m" unit text in a UV
  view, though the value is UV units. *Provisional.*
- **UVT-138** Element Move has no Slip UVs option in Operandi, and Slide's
  UV-keeping option isn't touched in UV space (the proxy has no UVs of its
  own), so nothing needed greying.

## UV tools finish (branch claude/project-thread-ll5iqr, 2026-11-18)

Specs: `specifications/uv-editing-2026-11-16` (BEHAVIOR-UV-TOOLS, UVS-1)
and `specifications/uv-2026-11-16`. All *provisional*.

- **UVT-Q13 answered here (build):** a click with Element Move on a UV
  element puts Move handles on it (U teal, V yellow, centre square);
  dragging an arrow moves the element along that axis only.
- **UVT-Q16 (build):** Slide's UV Map field: (none) slides points; a map
  slides that map's UVs from any view (`UvToolSpace::pinned`).
- **UVT-Q17 (build):** Range reads in UV units in a UV view (no "m").
- **UVT-P01** Project starting values (UVT-Q02): Planar, Axis Y,
  Automatic. Atlas and Atlas 2 wait for the xatlas port.
- **UVT-P02** Project Automatic (UVT-Q09): Planar, Cubic and View keep the
  aspect, the longer side spanning the tile from its lower-left corner;
  Cylindrical and Spherical fill 0-1 in V over the selection's height.
  Manual: Size spans the tile, Center lands on its middle.
- **UVT-P03** Project measures in the item's own space; Project from View
  in world space as the view sees it, the view's height being the tile
  when Auto Scale is off. No 3D handle for Manual's frame (UVT-Q01).
- **UVT-P04** Barycentric: quads to the tile's corners from corner 0
  counter-clockwise; triangles (0,0) (1,0) (0.5,1); larger polygons on the
  inscribed circle.
- **UVR-P01** Relax Conformal / Angle Based: each Iteration closes 10 % of
  the way to the re-solved island (fitted back onto where it was by one
  turn, scale and move); Spring and Adaptive count passes.
- **UVR-P02** Boundary Maintain: border points move only along their
  border; Straighten: corners where the border turns more than 30°,
  points between them on the straight line (Uniform Spans evens them).
- **UVR-P03** Relax's falloff weighs mesh points in 3D in either view
  (UVT-Q03).
- **UVR-P04** Relax with no UV map starts and does nothing until
  Iterations is raised (then says why).
- **UVR-P05** Drag anywhere with Unwrap, Relax and Unwrap & Relax: one
  Iteration per 2 px, right for more (UV-Q6). Not built: Interactive
  pins, Pause, Save Constraints, UV Symmetry, Show Transform Handles.
- **UVO-P01** Quick Align turns the whole island about the reference
  edge's middle (UVT-Q06); the reference is the first selected edge, or
  the first two selected points.
- **UVO-P02** Split sizes (UVT-Q07): plain 0.00001 (invisible, but two UV
  positions), Ctrl 0.01, Shift 0.03 UV units; each side moves half the
  distance square away from the cut.
- **UVO-P03** Sew / Move and Sew's "selected side": the side whose island
  holds more selected polygons, else the larger island. Shift moves the
  selected side onto the other, Alt the other onto the selected.
- **UVO-P04** Rotate Polygons (UVT-Q10): each corner takes the next
  corner's UV.
- **UVO-P05** Island Align's toolbox shows U Min / Center / Max and V Min /
  Center / Max (the other axis unchanged) instead of the 4 x 4 grid; the
  command takes any pair.
- **UVO-P06** Paste Connectivity walks both selections from their first
  polygons across shared edges; Select Error UVs' overlap test compares
  the triangles of different polygons.

- **UVS-P01** Seam maps (UVT-101) live in `MeshAttributes::edge_sets`
  under the key prefix `#uvseam:`, so every edit that renumbers edges
  carries them and files need no new version; selection set lists leave
  them out.
- **UVS-P02** The first seam map is "UV Seam"; Cut Map's UV Seam... picks
  or names the one its buttons use. Seam Marker grows the Conditional Loop
  with its starting stop rules into the map (the selection stays).
- **UVY-P01** UV Symmetry's Center starts at 0.5; partners are UV
  positions within 0.00001 of the mirrored place; when both partners moved,
  the one that moved further drives; points on the line stay on it.
- **UVY-P02** Center welds positions within 0.005 of the line (each mesh
  point's ones to their average along it). Symmetrize's Align to Symmetry
  Axis, Relax's UV Symmetry and Unwrap's Symmetrize aren't built.
- **UVV-P01** Coverage samples the 0-1 tile on a 128 x 128 grid; the
  Flipped tint starts off; tile numbers show once the view goes past tile
  1001.
- **UVV-P02** UDIM Indicator is built as buttons, not a tool: Select
  Tile... (Select UVs in the Region) and four arrows moving the selected
  islands a tile; no see-through frame, Manual frame or coverage inside it.
- **UVP-P01** Pack Nearest: islands are grouped by the tile their bounding
  box's centre is in, each group packed into that tile.
- **UVP-P02** Pack Polygon Tag: each tag's polygons pack into their own
  tile from 1001 along U (ten to a row), tags in name order; Smoothing
  Group splits smooth and flat polygons; Selection Set uses each polygon's
  first set by name. By Item: each active mesh to its own tile in list
  order, its new map named after it.
- **UVP-P03** Background Constraint: the other visible meshes' islands in
  a map of the same name stay put inside the region and are packed round.

- **UVA-P01** Atlas / Atlas 2 charting is Operandi's own D-Charts style
  region growing (as xatlas's ClusteredCharts works, no xatlas code):
  Atlas lets a polygon turn up to about 35° from its chart's mean normal,
  Atlas 2 about 70°, roundness weight 0.15.
- **UVA-P02** Atlas 2 starts at Segments 8, Iterations 4 (UVT-Q02); seeds
  spread by facing, the largest charts reseed each round.
- **UVA-P03** Orientation turns each chart so world up (+Y; +Z for charts
  facing up or down) runs up V; charts are never mirrored.
- **UVA-P04** Atlas charts pack by their bounding boxes (quick enough for
  the live tool).
- **UVE5-P01** Peeler: Uniformity starts at 0 % (UVT-Q04); every grid
  region is stretched to fill the Min/Max U/V box; the walk starts at the
  lowest-numbered quad touching the cut. Peeler is a live tool like
  Project.
- **UVE-P01** Export UVs draws each UV edge once, black 1-unit lines on no
  background, the tile 1024 units square (V up).
- **UVE-P02** Convert UVs to Mesh: a new item "<name> UVs" with U to X and
  V to Y in metres, keeping the map; it becomes the selected item.
- **UVE-P03** Transfer UVs: the primary active mesh's selected map onto
  every other active mesh with the same polygons (corner counts and
  edges); the spec's "two neighbouring polygons" picking isn't needed for
  meshes numbered alike, so it isn't built.

- **UVB-P01** Box Transform: the pivot starts at the box's centre; each
  drag ends a step of the session (as Move's), so the next drag's box is
  the moved UVs'; right-dragging a corner scales evenly.
- **UVB-P02** Box Transform's Shift snap lands the box's lower-left corner
  on the 0.1 grid (UVT-Q05).
- **UVB-P03** Box U/V and Gaps aren't built (Offset moves the box).
- **UVBR-P01** The Split and Sew brush is a UV view mode (UV tab > Brush,
  Alt+click for Sew), not a separate tool: a stroke runs once, when it
  ends, on the points within the brush (20 px to start); Split moves each
  side 0.01 apart; Sew joins every copy of a brushed point at their
  average. Move Part isn't built.
- **UVR-P06** Relax Interactive: pins last for the tool session; Pause and
  Save Constraints aren't built; with pins the solve goes all the way.

## Shader tree complete (branch shader-tree-complete, 2026-11-17)

The hand-off folders `shader-tree-remaining-2026-11-07` and
`procedural-textures-2026-11-07` aren't in the repo; built from
`docs/specs/BEHAVIOR-TEXTURE-LOCATOR.md`, the earlier questions below and
the code. All *provisional*.

- **TL-Q30 Front projection.** The picture of the Projection Camera (the
  render camera when none is chosen or it's gone) is laid on the surface:
  u, v run 0 → 1 from its bottom-left to top-right corner at the render
  resolution, Film Fit and Film Offset as the render frames it (Offset
  ignored for orthographic cameras), then × Repeat; UV Rotation doesn't
  apply. Surfaces behind the camera read the mirrored projection. Front
  Culling doesn't cull Front. With no camera in the scene it renders as
  UV Map, and the panel says so.
- **TL-Q31 Light Probe on a surface.** The direction from the locator's
  centre to the point is read off the mirror ball exactly as an
  environment Light Probe reads a ray direction (ETL-Q02b): Projection
  Axis is the picture's up, the ball's centre is the axis after it (Y:
  +Z). Then × Repeat.
- **TL-Q32 Implicit UV without fur.** Operandi has no fibres, so each
  polygon is its own 0..1 patch: a triangle (0,0) (1,0) (0,1), a quad the
  unit square in its winding order, larger polygons evenly round the
  circle inside the square. A material reads one UV map in a render
  (GR-Q08), so Implicit UV beside a UV Map layer in one material reads
  whichever comes first.
- **VM-Q05 update (2026-11-17).** An environment without an image
  (gradients, constant colours, Daylight's colours) now lights and
  reflects in Advanced through a 256 × 128 picture made from its own
  colours, as an image environment does. Scenes with no environment keep
  the flat ambient.
- **VM-Q08 Subsurface in the viewport.** Subsurface Weight wraps the key
  light past the terminator by up to 0.5 (in N·L), per channel by
  Subsurface Radius (largest channel = full wrap), so skin-like
  materials glow red at the shadow line. No blur or light through thin
  parts. Coat Tint colours what is under the coat by Coat Weight; the
  coat's highlight follows the bump and normal map by Clearcoat Bump.
- **VM-Q09 Refraction in the viewport.** Each see-through surface's near
  side shows what is behind it shifted against its normal by
  (IOR − 1) × 6 % of the view's height (no thickness, no ray tracing,
  only opaque things behind are shifted; a second glass behind shows
  straight). Strength by eye.
- **VM-Q10 Cut-out shadows in the viewport.** A material with cut-outs
  (Stencil, an RGBA cut) casts a dithered shadow lighter by the share its
  cut-outs remove on average (images as their average), like see-through
  materials (VT-Q07), in Advanced. The shadow isn't cut to the
  pattern's shape: that needs the material's per-pixel program in the
  shadow pass (follow-up).
- **VM-Q11 Still left out of the viewport (2026-11-17).** Displacement
  (needs a new per-corner value in the fill vertices and drag-time
  evaluation) and bump / normal from procedural patterns per pixel
  (needs every pattern ported to the GPU). Both still render in Cycles.
- **GR-Q03 v2 / GR-Q09 Edit Gradient window (provisional).** "Edit
  Gradient…" under a Gradient layer's strip opens a resizable window: the
  Input Parameter, a large strip of the curve the effect reads, its key
  list, and Presets ▾ (Ramp Up / Down or Black to White / White to Black,
  Bell, Four Steps; colour adds Rainbow, Heat, Cool to Warm) with a Recent
  group (the last 8 curves replaced or closed, this session only), Reverse
  (mirror across the keys' span), Distribute Evenly and Reset. Presets
  fill the keys' span, or the input's usual range (0..90°, Slope 0..180°,
  0..10 m, 0..1) when the keys span nothing. Each change is one undo step.
- **PQ-5 Occlusion types (provisional).** Every type renders now, in
  Cycles (the viewport still reads them as open, as before). Uniform, Up,
  Down Slope and Reflection fire their rays round the normal, world +Y,
  halfway between the normal and straight down, and the view's mirror
  direction; Thickness fires into the surface. Concavity: rays out,
  this object only (dark in creases). Convexity: rays in, this object only
  (dark on ridges). Concavity & Convexity and Curvature:
  ½ + ½ (inside − outside), so creases read high, ridges low, flat ½;
  Curvature always this object only. Spread Angle, Max Cavity Angle and
  Cutoff Angle stay stored (Cycles' AO is a whole hemisphere). Cycles ABI 23.
- **SH-2b Shader Effect (provisional, shader-tree-complete).** A Shader set
  to an Effect other than Full Shading makes the surfaces it governs
  render only that part of their light, by switching the rest of the
  principled surface off: Diffuse (no reflections, emission or
  see-through part), Specular / Reflection (the same glossy light, RO-Q01;
  the base colour kept only where metallic), Transparent (only the
  see-through part), Subsurface (no reflections, sheen, emission or
  see-through; the share Subsurface Amount leaves stays diffuse),
  Luminous (only emission), Fog (black under its fog). Alpha and cut-outs
  stay. The Advanced viewport draws the same.
- **GR-Q10 More Gradient inputs (provisional).** Bumpless Incident =
  Incidence (renders don't bend the normal a Gradient reads by the bump).
  Locator Incidence: degrees between the normal and the direction to the
  layer's locator (world origin without one), 0 facing it. XYZ Distance to
  Locator: the largest of |x|, |y|, |z| in the locator's own space, so its
  Size scales it (a box-shaped distance). Thickness: 1 m × the share of
  rays fired into the surface that leave this object within 1 m (paper
  thin reads near 0, 1 m or more reads 1 m); the viewport reads 1 m.
  Still stored only: Group Mask and Layer Mask (masks aren't folded
  values in Operandi) and the four particle inputs (no particles).
  Cycles ABI 24.
- **VM-Q11 v2 (2026-11-17, built).** Procedural patterns now draw per
  pixel in the viewport (colour, bump, normal), from pattern.wgsl, a port
  of procedural.rs that matches it on the GPU. Displacement shows in the
  Advanced style: each corner moves along its normal by the material's
  displacement, worked out in the vertex shader every frame, so it
  follows drags at no extra CPU cost. Provisional limits: corners of
  faceted polygons (no smooth normal) don't move; a box's corners carry
  each face's normal, so its faces part where Cycles would dice and stay
  closed; the wireframe, shadows, picking and subdivision patches use
  the undisplaced surface.

## Path tools and curve commands (modelling-parity groups 2 and 3, 2026-11-17)

Specs: CURVE-EXTRUDE-PEN-CLONE v1 (CX-xx), SPEC-CURVE-CLONE v1,
CURVE-COMMANDS-2D-SHAPES v1 (CC-xx). Built provisionally as below.

- **CX-Q1 Pen Extrude (hands-on 2026-11-17, CX-30..32, built)** — one
  section per pen point, no in-between steps; sections turn like Pen Clone.
  The button: Start at Source off, Cap Start off, Cap End on, Invert on.
  Corner Scale (on) widens an inner section across the bend by
  1 / cos(half the bend) so both walls keep their thickness (build guess,
  CX-Q1). Not built: Type Open/Closed as a choice (Closed is the path's
  own Closed), Load Selected Edges, Current Point X/Y/Z, Sweep UVs,
  Curve Cage and the Profiles sections.
- **CX-Q2 Pen Clone Corner on** — not built; the Pen Clone button's preset
  has Corner off, and the option isn't shown.
- **CX-Q3** Section orientation at the first point when the polygon isn't
  perpendicular to the path: the polygon stays as it is (it *is* the
  first section); later sections carry its normal along the path with
  the least twist from the polygon's own normal.
- **CX-Q4 Bezier / B-Spline Extrude** — same rules with those curve kinds.
- **CX-Q5 Invert (provisional)** — the walls follow the selected
  polygons' winding. Invert on (all three extrude buttons) keeps that;
  off reverses a free-standing selection. Polygons joined to the rest of
  the mesh always keep the mesh's winding and get no start cap (the walls
  start from the hole they leave). Size is the scale of the last section
  (Clone: the last copy), spread evenly; Pivot, Angle, Tolerance and Sweep
  UVs aren't shown (new walls get no UVs yet).
- **CX-Q6 Ids** — the buttons' tool.set ids are Operandi's own
  (`CurveExtrude`, `BezierExtrude`, `BSplineExtrude`, `CurveClone`,
  `BezierClone`, `BSplineClone`, `PenClone`); the name map has no entries.
- **CX-Q7 Mesh Stack** — Curve Extrude, Curve Clone and Pen Clone ops
  (`curve.extrude.item`, `curve.clone.item`, `pen.clone.item`) keep the
  path as their own points; the reference's ops read a curve item through
  `pmodel.pathGenCurve` + `gen.pathsteps.item`, which Operandi doesn't
  have yet.
- **CX-Q8 Curve end tangent** — CX-13 measures the last section facing
  (1,1)/√2; Operandi's Curve (the "natural end" rule, see the Draw tools
  note above) ends at about 38.7° on the same points. CX-10 says use
  Operandi's curve, so the section follows Operandi's tangent.
- **CC-Q3 (Curve Clone)** — the source counts as the first copy: Steps 3
  makes the source plus 2 copies; with Start at Source off the copies
  still skip the first position. Scale Pivot Bottom is the bounding box's
  lowest Y, centred in X and Z. Copies turn from the path's first
  direction (so a path drawn flat keeps them upright). Background curves
  as paths (SPEC-CURVE-CLONE Q1) not built.
- **Bank / Twist** — Bank is one roll for every section (not per point);
  Twist is spread from 0 at the first section to its value at the last.
  Closed sweeps end where they began, with the caps as set.
- **Path points** land on the smooth surface of another mesh under the
  pointer, else on the work plane, then snap. Bezier clicks get smooth
  handles (the last two anchors are reshaped); a Sketch stroke is fitted
  with the VectorCraft fit to about 3 px.
- **CC-40..43 Curve offsets (hands-on, built)** — Enter Offset Amount
  (Offset, starts 0 m). Each control point moves along its corner's mitre
  so every segment stays Offset away; positive goes inside a closed curve
  whichever way it winds (CC-Q2); the copy is a new curve in the same item,
  same kind and point order, selected after. Trio adds both sides. Open
  curves use the same rule with their ends moved square to their one
  segment (build guess); copies are never joined to the original.
- **CC-50..54 Conversions (hands-on, built)** — the result is a new item
  Converted_Curve / Converted_Bezier / Converted_Bspline placed like the
  source, the source kept; the target curve tool then opens on it in Edit
  mode (Q drops it). Curve→Bezier and B-Spline→Bezier are exact (each span
  is a cubic; a Curve's handles come out ± (next − previous) / 6).
  Bezier→B-Spline: first knot then every handle (closed: 2n + 1 control
  points; open: ends on the last knot). Curve→B-Spline goes through the
  Bezier first; Bezier→Curve and B-Spline→Curve keep the span ends only
  (build guesses). Greyed without a curve of the source type (CC-54).
- **CC-Q3 Bezier Tangent Sharp** — each affected handle aims at its
  neighbouring knot and keeps its own length (the observed Out shortening
  to 0.167 and the In handle's direction aren't copied). Smooth: both on
  the line from the In handle's reverse to the Out handle, own lengths;
  Uniform: that line, both at the average length. Out / In change one
  handle (it lines up with the other); Set Length gives the affected
  handles Length. Works on selected knots in Vertex mode.
- **CC-Q4 Align Winding Axis** — only Automatic (the world axis the
  curves face along most). Merge Curves joins selected open Curves and
  polylines that share end points (by point), closing a chain whose ends
  meet.
- **CC-Q5 2D shape planes** — Axis Y is observed (XZ plane, Position Y
  along +Z). Axis X puts the shape in YZ (Position X → Y, Position Y →
  Z); Axis Z in XY. Rectangle's first corner is top-left (−X, +Z),
  turning toward +X.
- **CC-11 Curve Fill** — each closed curve fills on its own; nested curves
  don't make holes yet. Curves and polylines fill on their own points
  (MF1); Bezier and B-Spline curves fill on new points along their
  outline (12 per span), since they don't pass through their handles.
- **CC-30 Set Curves Color** — a session setting of the app (all views),
  not per view yet.

## Generators and on-hold ops (modelling-parity batch 5, 2026-11-17)

Spec: SCATTER-UVRECT-TRANSFER-CONSTRAINT v1 (GO-xx). Built provisionally:

- **GO-06 Scatter variation** — each copy's scale is 1 ± Scale (per axis,
  below 100%) and its turn ± Rotate degrees per axis, from the Seed.
  Operandi's own random numbers (GO-05).
- **GO-Q4 Scatter op spread** — Mitchell's best candidate (each point the
  farthest of up to 200 seeded tries); the source counts as slot 0 and
  copies take the other slots. Saved as `scatter.clone.item` (one op, not
  the reference's generator + effector pair).
- **GO-Q2 Half Ray** — built as Directional. Save Surface Normal isn't
  built (kept in the file, not shown).
- **GO-Q1 UV Rectangle size** — without Pack the island keeps its total UV
  area; its first corner and its average turn (Procrustes fit, mirrored
  islands stay mirrored). Pack uses Pack UVs with Gaps. The "warning" for
  islands that aren't full quad grids is a status note; with none
  straightened the command refuses. Command name `operandi.uv.rectangle`
  (GO-Q5: the reference's command name isn't in the name map).
- **GO-Q3 Transfer Vertex Map** — Distance sampling only (the nearest spot
  of the source surface), in world space; types Position, Relative Morph,
  Weight, RGBA and UV (UV writes every corner of the point, so no seams;
  UV Completion and Flip UVs aren't built). The command reads the first
  shown background mesh; the op reads its Source Item link. Command name
  `operandi.vertMap.transfer`.
- **UnSubdivide op** — not registered here: the Mesh Stack branch
  (claude/project-thread-fcjo2t) already has `poly.unsubdiv.item`.
- **JD-Q1 Julienne / Dicer (EDIT.md v0)** — built as commands with a
  dialog (`operandi.julienne`, `operandi.dicer`; Edit tab rows Julienne…
  and Dicer…) on the Boolean engine: closed pieces, inner faces take Inner
  Material (empty: the mesh's own), Gap between pieces. Julienne's planes
  run through the mesh's centre, Offset apart along Offset's direction;
  the click-to-place origin, drag offset, Size, Rotate, Angle Snap and
  Preview Only aren't built. Dicer draws each plane's direction in a box
  shaped by Fragment Scale and its position within half the mesh's size
  of its centre; Randomness isn't built. Open meshes are refused, as
  Boolean refuses them.
- **Curve Boolean** — not built here: the Mesh Stack branch's Boolean batch
  has the solid Boolean op; a curve boolean needs its own spec pass.
## Deform finish (modelling parity batch 1, DEFORM-PROBES 2026-11-17)

- **DFF-Q01 Jitter Seed (provisional).** The bare tool starts at 137 and the Deform button at 666 (DP starting values). Operandi starts every way of picking the tool at 666 (the button's); the MeshOp starts at 137 (TYPE-IDS).
- **DFF-Q02 Rigid Rotate / Rigid Scale amounts (provisional).** Not probed. Each connected piece turns about its centre by up to the angle whose arc at the piece's edge is Range (per enabled axis), and scales by up to the factor that grows its edge by Range. Same seed, same result.
- **DFF-Q03 Radial Align anchor (DP-Q1, provisional).** A closed loop keeps its point farthest from the centre at its angle (matches DP-RA-1/2); an open run keeps its first point. Start Angle turns the result from there in every mode; N-Sided's Rotate adds to it.
- **DFF-Q04 Radial Align N-Sided with fewer points than Sides (provisional).** Points are spread over the corners. With more points than Sides each corner gets one and the extra points go on the last sides from the anchor, evenly (DP-13 case).
- **DFF-Q05 Linear Align Curve mode (provisional).** The run is smoothed with its ends fixed (Smoothing Iterations passes), then respaced evenly along itself when Uniform is on; Preserve Original Curve skips the smoothing. Exponent: a superellipse, 0 round.
- **DFF-Q06 Linear Align on closed loops (provisional).** A closed loop or a polygon border has no ends, so it is refused with a message (Radial Align takes those). Typed Radius > 0 replaces the measured one (DP-Q2 open).
- **DFF-Q07 Align handles and Center (not built).** The Align tools have no viewport handles or Center X/Y/Z rows yet; drag sets Weight. Relax Mesh Constrain = solve against the fixed border until settled (DP-21, RM-Q1); off = 5 gentle passes of 0.2 (fitted to DP-RM-1).
- **DFF-Q08 MeshOp ids (provisional).** TYPE-IDS names only Jitter. Quantize, Radial Align, Linear Align and Relax Mesh ops use Operandi's ids `xfrm.quantize.item`, `xfrm.radialAlign.item`, `xfrm.linearAlign.item`, `sculpt.relaxMesh.item`; Quantize's op grid is in the mesh's own space. Vortex's rotation axis is the tool's own (DP-Q3).

## Selection tools (modelling parity batch 4, SELECTION-TOOLS v1)

- **SL-Q1 Coplanar Edges Flatness (provisional).** The pair's points' largest distance from the plane through the shared edge (average of the two normals), over the pair's longest point-to-point span. Passes SL-CE1/CE2.
- **SL-Q2 Diagonal pairing (provisional).** Diagonal selects exactly the edges Detriangulate would remove with the same options (same pairing, same starting values as SL-44). Passes SL-DI1; SL-DI2 checked as a valid pairing.
- **SL-Q3 Coplanar Polygons Angle (provisional).** 5° on the button. The tool is a dialog that stays open: each change re-runs from the start polygons (one undo step each); Cancel takes them back; no Tool Properties panel or viewport clicks yet.
- **SL-Q4 Conditional Loop stops (provisional).** Built: Loop Both Directions, Max Loop, Shared Polygon, Sharp Angles (a turn over 45°, an Operandi value), Length Difference (longer over shorter minus 1 above the tolerance), Selection (stop at the Nth already selected edge). Selection Set and Seam Map stops aren't built (no edge sets or seam maps). The button grows from the last selected edge (Loop From: Last). Conditional Loop Tool is a dialog that runs once.
- **SL-Q5 Lazy Selection reach (provisional).** Three times the pick radius for points and edges; polygons pick as before. Saved with the selection preferences.
- **SL-Q6 Shortest Path Mode (provisional).** Toggle `Shortest Path Mode`: a Shift+click that adds an element also adds the path to it from the element selected before (a second undo step); Ctrl+Shift+click adds without a path. No hover preview yet.

## Topology and clean-up (modelling parity batch 6, TOPOLOGY-CLEANUP-TOOLS v1)

- **TC-Q2 Triple Fan (provisional).** The pivot is the last selected vertex (the selection keeps vertices across modes); every selected polygon with four or more corners that uses it fans from it, keeping its winding. No vertex, or none on such a polygon: a message, no change. The probe's triangle list (0,2,1)… reads wound the other way; Operandi keeps each polygon's own facing (TC-01 "facing unchanged").
- **TC-Q3 Collapse Selection (provisional).** One new point per connected region (polygons joined by edges). A region whose outline isn't one loop (a hole, or a whole closed mesh) is refused. The centre's UVs and corner maps are the outline corners' average; point maps aren't set on it.
- **TC-30 Axis Rotate (provisional).** Operandi's preset id `AxisRotate` (the name map gives none): Rotate with only the ring on the work plane's normal (XZ → Y, YZ → X, XY → Z), Alt+Shift+E, in the Tools menu. Advanced Handles, lever handle, Item-mode T/R/S switches and middle-click repeat are Rotate's as they are.
- **Not built in batch 6 (Jason, 2026-11-17: hands-on batch later):** Topology Pen, Tack Tool and Retopo Sketch (all pointer-driven, TC-Q1); Retopo Edge Lock / Flow wait with Retopo Sketch, since Automatic Retopology doesn't read edge sets yet.

## Hands-on Deform batch (2026-11-17, observed in Jason's session)

- **SD-Q1 answered.** Soft Drag: Move with a Screen falloff, circle 20 px, right-drag resizes; weight 1 − screen distance / radius (linear; the Screen falloff is linear now, was Smooth). Nothing selected: whatever is under the circle moves; with a selection only selected points, weighted. Each left press centres the circle on the pointer (Operandi's reading of "around the pointer").
- **DP-Q5 answered.** Element Push: hover highlights the element; the drag moves it along its normal, a polygon as one flat piece along its polygon normal; Range (0) spreads the move to neighbours by 1 − 3D distance / Range, along the same direction. Distance per pixel is Push's own (unchanged).
- **DP-Q6 answered.** Soft Selection: linear 1 − d / Radius; Use Connectivity on: d along mesh edges and disconnected parts never move; off: straight-line d, so near disconnected parts move. The handle's axes follow the selection in any component mode (new action-axis layer `selection_all`; Flex keeps polygon-only `selection`).

## Topology batch (TC-22, TC-45, TC-52; observed hands-on 2026-11-17)

- **TC-22 Tack (built).** A left click on a polygon's front side of another shown mesh puts the selection there: the selection's centre on the polygon's centre, its face flat against it facing the other way (Align on), everything connected carried along (Move Connected Vertices on). Offset, Size and Rotate (a turn about the target's normal) apply. Not built yet: Copy Geometry / Shift+click, Make Bridge, Mode (TC-Q4). TC-Q5 (provisional): polygons of the active mesh are refused.
- **TC-45 Topology Pen (built, partly).** Preset `mesh.topology` on Element Move: a plain left-drag moves an element and isn't snapped (TC-Q6 left open; the Background constraint, if the user has it on, still applies as for Element Move). Shift+left-drag on an edge pulls out one new quad when released (no live preview yet); its two new points land on the nearest background surface. The other modes and mouse shortcuts (TC-42) wait for their hands-on check.
- **TC-52 Retopo Sketch (built).** A left-drag across a background mesh (shown, not active) makes one open Curve in the active mesh, a point every 3 cm along the stroke, each on the background's front surface. Selection sets (Retopo Edge Lock / Flow) and the TC-51 options aren't built yet.

## Toolbox Topology and Curve tabs (TOOLBOX-TOPOLOGY-CURVE v1, TB-xx, 2026-11-17)

- **TB-01** Topology and Curve now follow Polygon in the tab strip. The
  rest of Operandi's strip is unchanged (Create, Select, Deform, Sculpt,
  Duplicate, Edit, Vertex, Edge, Polygon; UV after Edit in the UV layout);
  Fusion isn't built. The Curve tab was missing from the strip on main
  before this (its form existed): fixed.
- **TB-Q1 Topology presets (provisional)** — each Tools button runs
  `operandi.topology.tool <tool>`: it turns Constrain to Background on
  (and snapping, for Topology Pen, Pen and Topology Sketch), then starts
  the tool. Both stay on after the tool drops (the Settings section's
  Constrain to Background button turns it off); TP-12's give-back isn't
  built for these app-run tools. Grow Quads and Subdivide don't project
  onto the background yet (TB-12).
- Not built, rows say why: Contour, Instance Mirror, Merge Instance,
  Topology Mode, Offset, Selection Hit Size, Foreground Opacity, Hair
  Guide. Soft Selection runs `tool.set falloff.softSelection on`.
- **TB-20 Curve tab** — laid out as the spec: Tools, Commands (offset
  flyouts), the Convert Curves flyout, Bezier Tangent. The six 2D shapes
  are no longer on this tab (they are Mesh Stack, Create › Curves in Add
  Operator).
- **CC-43 Open Curve Offset (v9 spec)** — matches: one new open curve in
  the same item, mitred inner corners, positive to the left of travel
  seen from the plane's positive axis; the ends sit beside the original
  ends (the spec's build rule; the observed 0.1 × (2 + √5) slide isn't
  copied). Trio does both sides.
## Curve Boolean (CURVE-BOOLEAN v1, CB-xx, 2026-11-17)

Built as written for CB-10..19 (the six acceptance cases pass). Provisional:

- **CB-Q1** Open curves, Close / Trim Open Curves, Self Intersection and the
  Constraint Target / Offset aren't built; only closed curves take part.
- **CB-Q2** Bezier and B-Spline curves are cut as their control polygons
  (straight segments) and the results are polylines.
- **Driver Mesh** (Driver Curve = Driver Mesh Item, Include Children) isn't
  built: the drivers are the mesh's own unselected closed curves (with none,
  the other selected curves, used up as CB-18 says).
- Edges lying along each other aren't split specially; points left in the
  middle of a straight side are dropped from Union/Intersect/Subtract
  results.
- Command name `curve.boolean` (Edit tab, Curve Boolean...).

## 3D painting stage 1 (PNT-01..39, PNT-300..309, 2026-11-17)

Built from the coordinator's summary of the spec (the spec file wasn't in
this repo). Provisional readings:

- **PNT-Q01** Command names are Operandi's own (`operandi.image.*`,
  `operandi.paint.*`): the reference name map has no painting commands.
- **PNT-Q02** New Image's default name is "Image" ("Image (2)" …); a
  layer from Add … Texture is named after its image.
- **PNT-Q03** Add Mask Texture with no layer selected is refused ("Select
  the layer to mask first"). Add Color/Displacement with no Material Group
  goes where Add Layer puts a layer (above the Base Material).
- **PNT-Q04** Flatten Layers keeps the flattened layers but turns them off
  (so the look doesn't change), includes hidden selected layers, and makes
  an RGBA image at the deepest source depth.
- **PNT-Q05** Bake UV Border Size starts at 8 px.
- **PNT-Q06** Preferences > Images & Painting isn't drawn yet; Auto Create
  Textures, Bake UV Border Size and "Also save changed images" are
  commands (`operandi.paint.autoCreate`, `.uvBorder`, `.alsoSaveImages`),
  not saved between sessions. "Also save changed images" is a File menu
  tick, not a checkbox in the save panel.
- **PNT-Q07** Only RGB and Grayscale Photoshop files are read (CMYK, Lab,
  Indexed are refused).
- **PNT-Q08** A rebuilt Composite uses the layers' blend modes where the
  shader tree has the same mode (Normal for the others) and ignores group
  opacity and blend.
- **PNT-Q09** The Images list marks Photoshop images with a text glyph,
  not a generated layer-stack icon (needs an icon and Jason's approval).
  The paint target's paintbrush reuses the generator's vertex-map paint
  icon.
- **PNT-Q10** The viewport shows a Photoshop image's Composite on every
  Image Map using it, whichever PSD Layer the map picks.
- **PNT-Q11** Auto Create Textures on: painting with no texture asks "No
  texture selected. Add a new texture?" (Add makes a Color texture with
  New Image's last values); off: the refusal only.
- **PNT-Q12** New images have no file until saved; Save All asks for a
  file for each new image in turn. Auto-save writes changed images beside
  its backup as `<backup> <image>.<ext>`.

## 3D painting stage 2: brush engine (PNT-40..89, 2026-11-18)

Built from the coordinator's copy of the stage 2 spec. Provisional
readings (numbering continues the spec's PNT-Q list where it named one):

- **PNT-Q06** Stroke undo label "<Tool> Stroke" ("Airbrush Stroke").
- **PNT-Q10** Sculpt and paint share one tip implementation and one
  spacing routine (brush_engine.rs), but not one set of values: sculpt
  keeps a tip per brush (changing that would change sculpt results, which
  the regression gate forbids), and the three paint tools share one paint
  tip. Changing Size while sculpting doesn't change the paint tip.
- **PNT-Q11** Symmetry: the mirror dab is the stroke's surface point
  mirrored through the mesh's symmetry plane, projected to the screen,
  with the same screen radius.
- **PNT-Q12** Density: dab strength = tip weight × Density × pressure;
  Airbrush accumulates m + (1 − m) × dab, Paintbrush and Eraser keep
  max(m, dab); the image shows mask × Opacity over the pixels at the press.
- **PNT-Q13** Attenuation: linear, dab k of N gets 1 − k / N (dab N and
  later lay nothing).
- **PNT-Q14** BG Color starts black.
- **PNT-Q15** Ctrl+Shift+click colour picking isn't built yet; FG/BG
  Color rows use the colour button.
- **PNT-Q16** Bias is the sculpt tips' Schlick bias.
- **PNT-Q17** Custom tip profile: a cubic Bézier from the centre (1) to
  the rim (0), handles set by In / Out Weight (length) and Slope (rise).
  Paint's starting shape is Smooth (sculpt's stays Fat).
- **PNT-Q18** Maximum Brush Size is 500 px, a constant (no preference page
  yet); right-drag and the Size row stop there.
- **PNT-Q19** Live display re-sends the whole image to the GPU at most
  about 30 times a second during a stroke, not only the changed tiles
  (the stroke tracks the changed rectangle for that later).
- **PNT-Q20** Only the paint target's mesh is painted, and only it hides
  texels (other meshes in front don't block the paint); every polygon of
  the mesh takes paint, not only those the target's material group covers.
- **PNT-Q21** The Default Brush preference isn't built: picking a paint
  tool keeps the current tip (Smooth to start).
- **PNT-Q22** The Paint tab has no tool-choice (flyout) buttons yet: each
  later tool is its own greyed button. The Current Image row opens the
  Images window, which holds the Texture picker. The Paint tab joins the
  tab strip once opened (Texture > Paint Tools > Paint Tab, or a paint tool
  turned on), like UV and Workbench, rather than as a Sculpt/Paint sub-tab.
- **PNT-Q23** Painting a Photoshop layer shows the layer itself in the
  views while the stroke goes on; the Composite is rebuilt on release.
- **PNT-Q24** Tool Properties for paint tools are drawn by the painting
  module (plain rows running `operandi.paint.set`), not the shared
  ToolFields lists, so the field-list tests don't cover them yet.

## 3D painting stage 3: more paint tools (PNT-90..129, 2026-11-18)

Provisional readings (the spec's own PNT-Q20..Q27 numbers clash with stage
2's list, so these continue as PNT-Q25+ where noted):

- **Smudge** carries the colour one dab step back along the drag into each
  texel, mixed by tip weight × Density × pressure × Opacity (no separate
  pickup buffer; the image's own pixels are the "finger").
- **Blur / Sharpen** (spec PNT-Q20): Gaussian of radius 2 texels × Density
  (at least 0.5), mixed by tip weight × Opacity; Sharpen moves away from the
  same average (unsharp mask).
- **Clone** (spec PNT-Q21, 22): the offset is kept until a new source is
  set; only the screen-matched copy is built (Use View Projection shown
  greyed). The source is sampled from the image as it was at the stroke's
  start. Ctrl+click (Cmd on macOS keeps its menu otherwise; with Clone Cmd
  isn't blocked).
- **Line** (spec PNT-Q23, 24): outlines stroked with the tip, Shape starts
  Line; pressure, symmetry don't apply to shapes.
- **Fill** (spec PNT-Q25): the whole image within tolerance, per RGBA
  channel; Opacity × Density × FG Alpha. Clicking in the UV Editor isn't
  built (3D view only).
- **Gradients** (spec PNT-Q26): the ends are the pixels under the press and
  release; Radial's radius is their UV distance. Alt+click on the Linear
  Gradient button for Radial isn't built (separate buttons instead).
- **Tool-choice buttons** (PNT-93, 111): not built; each tool has its own
  button.
- **PNT-Q31** Use Falloff with Fill and the gradients weighs only pixels
  the view reaches (others aren't changed).
- **Lasso** (spec PNT-Q27): a falloff of the painting tools only, not yet
  on Operandi's falloff stack: Use Falloff on a paint tool applies it (mixed
  with the stack's falloff by its Mix Mode). It's held in screen space, so
  moving the view moves its volume. Handles to adjust it aren't built:
  drawing again replaces it; Clear Lasso removes it. Use World Transforms is
  stored (it only matters to mesh-point weights, `Lasso::point_weights`).

## 3D painting stage 4: tips, Tablet Nozzle, brush presets (PNT-130..169, 2026-11-18)

- **PNT-Q28** Procedural tip: the Shader Tree's Noise / Cellular / Dots
  pattern (their starting settings) at the texel's UV × 8 / Scale, Bias and
  Gain as the two-zone curves, times the Smooth tip's round profile;
  Amplitude 0 % gives the plain profile. W uses a fixed depth (no per-dab
  W yet).
- **PNT-Q29** Keep Aspect: the longer side of the picture spans the dab.
- **PNT-Q30** Speed: half strength (or size) at 1000 screen points a second.
- **PNT-Q31** Rotation starts Align.
- **PNT-Q32** Jitter 100 % = up to ± the full value: Spacing ± one gap,
  Position up to one tip radius (uniform over the disc), Size ± 100 %,
  Rotation ± 180°, Strength up to − 100 %. Pressure Sensitivity: Low p²,
  Medium p, High √p, on top of the pen pressure curve preference.
- **PNT-Q33** Sphere tip: FG shaded by the ball's height (Height Shading),
  Flatness lifting it toward flat, a fixed highlight up-left (Specular);
  painted as colour, so it suits colour and bump images alike.
- **PNT-Q34** Roller: the picture's U runs along the stroke by the distance
  travelled (in tip radii), turned with the stroke when the nozzle aligns.
- **PNT-Q35** Procedural / Sphere share no tool-choice button: separate
  buttons, as in stage 3.
- **PNT-Q36** Text tip fonts: Operandi's own (Ubuntu Light, Hack), not the
  installed system fonts; Bold and Italics are always faked.
- **PNT-Q37** Brush presets are their own `.opbrush` JSON files in
  Presets/Brushes, applied from Texture > Paint Tools > Brush Presets, not
  yet Preset Browser items or the tool-preset recipe format.
- **PNT-Q38** Sculpt with the new tips, the nozzle and brush presets
  (PNT-151, 241, 246, 247) isn't built: painting builds show the Sculpt
  tab's buttons greyed; sculpt brushes behave exactly as before.
- **PNT-Q39** Without the Nozzle button, dabs aren't rotated (stage 2's
  look); Stamp and Text take the dragged angle either way.

## 3D painting stage 5: inks and Curve Constraint (PNT-170..200, 2026-11-18)

- **PNT-Q40** (from the "painting does nothing" fix) A press paints the
  textured mesh under the pointer even when it isn't selected, and with no
  Image Map layer selected the top Image Map layer with an image is the
  target. A curve or untextured mesh in front doesn't take the press.
- **PNT-Q41** The ink stack is shared by the paint tools that take inks
  (Airbrush, Paintbrush, Eraser, Clone, Smudge, Blur, Sharp, Line), like the
  tip, not kept per tool. Fill and the gradients ignore it. On Eraser,
  Clone, Smudge, Blur and Sharp an ink only changes strength, not colour.
  One button per ink (no tool-choice flyouts yet).
- **PNT-Q42** Image Ink's Fit View stretches the picture over the whole
  view (width and height); Auto Scale keeps its size on the model as the
  view zooms in or out (scale × the view distance when shown ÷ now).
  Soft Border fades only the picture's edge with Repeat off.
- **PNT-Q43** Use Color as Mask: the picture's brightness is a stencil
  (white lets the tool's own colour through, black holds it back). UV
  Mask: its brightness at the texel's UV multiplies the paint, blurred
  over Mask Blur Radius pixels (up to 8), Invert Mask flips it.
- **PNT-Q44** E/R turn the widget 5° (E clockwise), D/F size it by 5 %.
  They clash with the global Rotate (E), Scale (R), Subdivide (D) and Flip
  (F) and win only while the widget shows with an ink-taking tool on. The
  rotate handle turns half a degree per point dragged left/right.
- **PNT-Q45** Projection Ink's camera is a selected camera, else the render
  camera. Show Image draws the camera's background at Image Distance as a
  flat picture (no perspective correction across it). Exposure is 2^EV.
  Stamp fills the selected polygons of the painted mesh in Polygon mode,
  else every visible texel.
- **PNT-Q46** Parametric Ink's Slope and Incidence Angle use the flat
  polygon normal (no smoothing); Displacement and Tangent Pressure are
  greyed (no displaced surface; the pen reports no barrel pressure).
  In/Out trim the 0..1 parameter range before the shape curve.
- **PNT-Q47** Random Ink: Hue 100 % = ± half a turn round the colour
  wheel; Saturation and Brightness 100 % = ± 100 % of the value; one draw
  per dab (the mirror dab shares it).
- **PNT-Q48** Curve Constraint: every visible, unselected mesh's curves
  (polylines and smooth curves, drawn as sampled) are background curves.
  With none, the stroke paints freely and says so. Background 2D snaps
  each pointer position to the nearest on-screen point of the nearest
  curve; Background 3D snaps the surface point to the nearest curve point
  in space and paints only surface within the tip's radius of it.
- **PNT-Q49** Inks on sculpt brushes (PNT-243) stay greyed ("Inks work
  with the paint tools only").

## 3D painting stage 6: layout, toolbox set, menus, preferences, UV painting, polish (PNT-210..239, 2026-11-18)

Settled by this stage: PNT-Q06 (stage 1: the page is drawn now, still not
saved), PNT-Q15 (colour pick built), PNT-Q18 (Maximum Brush Size is a
preference), PNT-Q21 (Default Brush built), PNT-Q22 / Q35 (flyouts built),
stage 1's PNT-Q10 (the viewport shows a picked PSD layer).

- **PNT-Q50** The brush preset strip is drawn over the bottom 64 points of
  the 3D view (layouts can't put a strip panel in the view column yet).
  A thumbnail is the tip's falloff as a soft disc with the tip kind's
  initial and the preset's name, not a painted sample.
- **PNT-Q51** The Sculpt/Paint set replaces the toolbox's tabs while the
  Paint layout shows (Sculpt Tools, Paint Tools, Vertex Map Tools,
  Utilities; Hair and Particle Tools left out). Vertex Map Tools repeats
  the Sculpt tab's vertex map section. Other layouts keep the usual tabs.
- **PNT-Q52** Utilities: Relax Mesh runs `sculpt.relaxMesh`; Geometry to
  Brush, Copy to Morph, Convert to Multiresolution and Background to
  Multiresolution are greyed ("Not in Operandi's sculpting yet"), as are
  Add Fur Texture and Convert to Vector Displacement.
- **PNT-Q53** The Image Viewer for painted images is a window of its own
  (not the Render window), showing the image's live pixels and, for a
  Photoshop image, a PSD Layer picker.
- **PNT-Q54** Images & Painting / Default Tools aren't saved between
  sessions yet. Enable Image Antialiasing by Default starts on and sets
  Antialiasing on Add Texture's new layer (stored; Cycles filters by
  Filtering alone). Default Foreground Color sets every paint tool's FG
  when changed. Default Brush is picked when a paint tool turns on with
  none on; Last Selected keeps the tip. Always Display Images as Icons
  shows the Images window as a thumbnail grid.
- **PNT-Q55** Ctrl+Shift+click picks the painted image's colour under
  the pointer (unlit), not the shaded pixel on screen.
- **PNT-Q56** UV view painting: the image shows behind the UV map only
  while a paint tool is on; while one is on, the UV view's selection and
  UV tools don't take left drags (Alt still navigates). Symmetry, the
  Curve Constraint and Projection Ink have no surface there and do
  nothing.
- **PNT-Q57** The paint Lasso is not put on the falloff stack (the stack's
  Lasso falloff stamps one weight per mesh point, too coarse for pixels);
  it stays painting's own, with a move handle (centre square) and a size
  handle (corner circle) usable with any paint tool on. With Use World
  Transforms on it stays on the model through the view it was drawn in;
  once the view moves its outline and handles hide and a note says it's
  there.
- **PNT-Q58** Only changed tiles going to the GPU (PNT-51) isn't built:
  the whole image goes about 30 times a second (as PNT-Q19).
  *Built 2026-11-18:* in the 3D views a stroke writes only its changed
  rectangle (and the mip pixels over it) into the texture, every frame; a
  UV-view stroke still sends whole pictures ~30 a second, and the Images
  viewer window updates at the release.
- **PNT-Q59** Tool-choice flyouts: Fill / Linear / Radial Gradient, Blur
  / Sharp, Procedural / Sphere, and one Inks flyout (its items add or
  remove that ink). Line, Smudge and Lasso stand alone. Hold or
  right-click opens a flyout; it keeps its last pick for the session.
  Alt+click on the face showing Linear Gradient turns on Radial.
- **PNT-Q60** The viewport shows a Photoshop image's picked layer when an
  Image Map using the image picks one (the first such map decides for
  every map of that image).
- **PNT-Q61** Starter brushes with pictures (PNT-245) aren't built.
- **PNT-Q62** Sub-D (Tab) meshes are painted on the smooth surface the
  view draws (the subdivided surface at the item's level, with its
  interpolated UVs), so paint lands under the brush. With Use Falloff on,
  the falloff's weights belong to the cage points, so the cage is painted
  instead.
## Spec v8 checks (2026-11-17)

- **GO-30..35 Transfer Vertex Map (fixed to match).** Type now lists the reference's nine in order (Position, Weight, Subdivision Weight, UV, Relative Morph, Absolute Morph, RGB, RGBA, Vertex Normal), so the op's saved `type` index matches the reference; Subdivision Weight, Absolute Morph and Vertex Normal are refused (no such maps in Operandi). New Sampling Space (Local ignores both items' placements, World uses them) and Sampling Method (Raycast along the target point's normal, a miss leaves the point unset; Distance takes the nearest spot, past the edge the edge's value). Values interpolate across the source polygon. **GO-Q1 (provisional):** the starting Space and Method aren't recorded; Operandi starts World and Distance (what it did before). Transfer ops saved before this change with type 1..4 read under the new order.
- **SL-64 Conditional Loop (fixed to match).** Row, closed ring, Max Loop 1 (nothing added) and Stop at Selection (stops on reaching the other selected edge, which stays) already matched. Sharp Angles stopped an octagonal ring after one edge: the sharp turn is now over 60° (was 45°), so 45° ring corners carry on and a 90° fold stops (provisional, SL-Q4).
## Spec v9 (final parity specs, 2026-11-17)

- **TC-33 Axis Rotate (built).** Picking it arms it: no ring until a left click in the view; the click is the rotation centre (Automatic centre layer) and the ring is the world axis the view looks along (the work plane's normal). Tool Properties: Angle, Apply, and a note pointing at Snap Angle (15°, Preferences ▸ Handles; Ctrl+drag snaps). Item mode keeps the user's centre (items don't take an Automatic centre yet). TC-Q7 (other views) as provisional.
- **DP-46 Soft Drag Transparent (already built).** Off: only points facing the view; on: hidden and back-facing points under the circle too (the Screen falloff's Transparent check in Tool Properties). DP-Q7 open.
- **TC-46 Topology Pen (built).** Moves never conform, even with Constrain to Background on; only new points land on the surface. TC-42 shortcuts built: left-drag Move, Shift+left-drag Duplicate (edge → strip), Shift+middle-click Add Loop (an edge loop across the clicked edge, at its middle), Ctrl+middle-click Remove (the element under the pointer; border edges refused). Not built yet: right-click Move Edge Loop, middle-click Split, Shift+right Duplicate Loop, Ctrl+left Slide (Ctrl+left keeps Element Move's axis lock), the Smoothing shortcuts.
- **TC-22 Tack (built).** Copy Geometry (and Shift+left-click) tacks a copy and leaves the original; Make Bridge onto another mesh does nothing at all (status says so); no Mode drop-down.
- **TC-53 Retopo Edge Lock / Flow (built).** Edit tab ▸ Commands: the selected polygons (curves) join the RetopoEdgeLock / RetopoFlow selection sets; other sets stay.
- **TC-54 Dicer placement (built).** Edit tab's Axis Slice flyout holds Polygon Slice, Polygon Axis Slice (its face), Pen Slice, Primitive Slice and Dicer. Julienne is gone, button and command (`operandi.julienne`, `Command::Julienne`): EDIT.md v0 had specced it as its own closed-slice tool, but TC-54 says Julienne is Axis Slice (Jason, 2026-11-17: remove it).

## Nits batch 4 (branch `nits-batch-4`, 2026-11-17)

- **Q-DC-04 Move arrows on drawn primitives (N21).** Spec:
  drag-creation-2026-11-06 (no rule for positioning). Start: a Sphere or
  Cylinder drawn by a drag. Jason asked for move arrows at the centre.
  *Provisional:* once drawn, three axis arrows at the shape's centre move
  it along X/Y/Z (as the Cube's arrows already do); the radius squares
  still shape it. Unclear: the reference's handle layout for the round
  primitives, and for Cone/Torus/Capsule/Ellipsoid (not changed).
- **N25 Edge Extend first click.** Spec: tool-click-start CS-14 ("Others:
  the click just shows the handles"). Jason (2026-11-17): the click makes
  the extension at zero offset, so the next tool (Scale) works on the new
  edges at once. *Built:* the click builds the rows on the source edges
  (zero-area polygons, allowed as degenerate until dragged open).
- **N27 Handle side with Symmetry.** Jason (2026-11-17): handles sat
  halfway between the two halves. *Built:* every tool's handle sits at the
  centre of the selection on the working side: the side the active view
  looks at (its orbit target, else its eye) when the selection has
  something there, else the first-selected element's side (the selection's
  order alone made the handle jump after lassos and tool results). Edge Extend's Rotate and Scale
  then run per side (two passes): each half about its own centre, the far
  half mirrored. A strip that crosses the plane is split where it leaves
  the plane, as the other two-pass tools do.

## Behaviour-check fixes (branch `check-fixes-1118`, 2026-11-18)

- **Selection echo (MCP).** Each mode keeps its own selection list. The
  bridge's echo now gives every mode's list (it showed only the current
  mode's), so scripts and the oracle checks see Make Polygon's new
  polygon, and Edge Subdivide's and Edge Bevel's new points, as the specs
  record them.
- **SYM-Q-ORDER Select Symmetrical's added points (provisional).** Spec:
  symmetry-2026-11-01 SYM-B2-01. Start: F1, Symmetry X, points picked by
  script. *Built:* the picks stay in pick order and the missing
  counterparts follow in descending index, which fits all six cases
  (9, 6 → 9, 6, 8, 5). Unclear: whether descending index is the rule
  beyond these cases, and the order for edges and polygons (one addition
  measured; they still walk back from the latest pick).
- **SYM-SEL-07 script picks.** The bridge's `select` no longer adds the
  mirror with Symmetry on; pointer picks still do (SYM-B2-10).
- **SEL-B2-04 rectangle order.** Built as measured: line by line from the
  first pick, each line running across the later of the first pick's two
  sides into the rectangle (the SEL-B2-03 "later side edge" rule). Only
  Grid4 is measured; on other topology it is a reading of the same rule.
- **MP06 one-point polygon (provisional).** Spec: make-split-2026-11-05
  rule 2. *Built:* Make Polygon (Auto) from one point makes a polygon of
  that point, stored as an open straight line of one point: no spans, no
  area, no edges; surface tools skip it as they skip lines. It survives
  undo, save/reopen, every export, subdivision and every command (tests
  `one_point_polygon`). Unclear: whether the reference draws it (Operandi
  draws only the point), and what tools such as Extrude do with it.
- **CMD-Q-HOLE Merge with holes (provisional bridge choice).** Spec:
  mesh-commands-2026-11-02 CMD-P-05. *Built:* a selected region with holes
  merges into one keyhole polygon: the outer border, then each hole joined
  by a doubled bridge along one removed interior edge (its two points
  repeat). Of the candidate edges the highest-numbered is the bridge,
  which gives the measured 10–14. Unclear: the reference's rule for the
  bridge and for several holes (only one hole measured). The viewport
  fills the keyhole and leaves the hole open.
- **M17-Q04 reversed for plain Bevel (needs Jason's OK).** Spec:
  polygon-bevel-2026-11-02 PB-C-01/04. Jason chose earlier to refuse
  insets past the centre; this batch's brief asked for the reference's
  result. *Built:* plain Bevel (and its drag) no longer refuses or clamps:
  a zero-size cap is an admitted degenerate face, an inverted or
  self-crossing cap or wall is kept as asked and drawn with a simple fan.
  Twist Bevel still refuses a fold. Commit "Fix 3b" alone holds the change,
  so it can be reverted on its own.

## Behaviour-check fixes — 2026-11-18 (branch check-fixes-1118b)

The behaviour check on main ce1290f5, replayed in
`crates/operandi-mcp/tests/oracle_replay.rs` (now drives every
extrude, smooth-shift-thicken, split, quadruple and flip case).
- **VE04 / VE08 lone corner (provisional).** Spec: extrude-2026-11-05
  (Q-EX-03's vertex part is open). *Built:* Vertex Extrude on a point with
  one polygon cuts the corner off (two points replace it), raises nothing
  and leaves it unselected, as measured. Edge and Vertex Extrude with
  nothing selected now turn on and act on everything (EE07, VE08); they
  start with a Width from the mesh's shortest edge.
- **Not changed, the oracle conflicts with an earlier decision:**
  EE04/EE07 border edges (Q-EX-03 decided: proper ridge); SP04/SP10/SP13
  (Q-MS-03 decided: cut each consecutive pair); QD09 and QD18 (Q-QD-01
  decided: never join points of one row; QD09's triangle 2,3,4 has zero
  area); SS06/TH10 Scale centre (**Q-SS-04 DECIDED** (Jason 11-18: scale
  follows the selected Action Center; intentional difference from the
  reference, SS06/TH10 known differences). No action centre, Selection,
  Selection Center Auto Axis and Automatic: the shifted selection's box
  middle; Selection Border: its border points' average; Origin: the
  item's origin; Element/Screen: the picked point; Local: each piece's
  own middle. With Symmetry a point centre is mirrored for the other
  side's points);
  EBC-O-E3-06-one-chamfer and PE-O-E5-moved-item (ORQ-01 decided: no
  duplicate or zero-area faces; E5's result holds two face pairs that are
  each other reversed); PE-O-E1-nonplanar-shiftX (its result puts three
  polygons on edge 0-1, which the mesh rules refuse; ORQ-02).
- **Not changed, the spec doesn't pin it:** NPR-O-TUB-05 By Length (NPR-Q08:
  the reference's length estimate is unknown; even arc length is within
  5 mm); SS10/TH12 selection after with nothing selected (Q-SS-10).
- **Already passing here:** TH11 (needed `thicken` on the bridge's
  `smooth_shift` params, added with `sharp`, `angle_weight`,
  `both_sides`), MP19, FL07, QD16.
- **New-point UVs at seams (SQ-UV-01; UVN-T-01, T-04, T-06), provisional.**
  Spec: new-point-uvs-2026-11-02. The seam cases of Edge Bevel, Vertex
  Bevel and Grow Quads now match
  (`crates/operandi-core/tests/new_point_uvs_measured.rs`). *Built:* Edge
  Bevel's strip takes its F2 side's frame on every corner (the polygon
  where the selected edge runs from its higher to its lower point; the
  measured "right neighbour" in both UVN-B2-02 layouts), carried across the
  edge from the other side by the shift/turn that lines the edge's UVs up.
  An end polygon's point slid along an edge from the removed point keeps the
  interpolated UV only when every polygon on that edge carries the point's
  continuous UV, otherwise it takes that continuous UV; Vertex Bevel's cap
  (Round Level 0) needs only one such polygon on the cut's edge, and one
  choice serves every UV map. Grow Quads' border corners take their points'
  stored continuous UVs. *Open:* Operandi meshes (and LXO import) keep only
  corner UVs, so "continuous" there is the point's UV in its
  lowest-numbered polygon, and Grow Quads keeps the border polygon's UV;
  the frame choice for other corner orders (UVN-B2-02's open part);
  Vertex Bevel Round Level 1+, Edge Chamfer seams and anything at a fold
  (polygons not in one plane, e.g. a cube's edge, where one frame would put
  UVs off the island) keep the per-polygon rule.

## Behaviour-check fixes 2026-11-18c (branch `check-fixes-1118c`)

- **AC-Q20 Selection's box space, provisional.** Check case
  AC-rotate-Selection (action-center-2026-11-18, not in this repo; behaviour
  from the coordinator's brief). Start: two islands in an item moved and
  turned 30° about Y; Action Center Selection; Rotate. Selection and
  Selection Center Auto Axis now centre on the middle of the selection's
  bounding box, not the average of its points (this replaces an earlier
  unique-vertex average).
  *Provisional:* the box is taken in the item's own space and its middle
  placed in the world, so it turns with the item; several meshes at once
  use a world box. With Symmetry on, the picked side's box. Automatic and
  None are unchanged (Automatic: world box; None: average). Unclear: which
  space the reference's box is in.
- **AC-Q21 Local's centre and axes, provisional.** Check cases
  AC-scale-Local and AC-rotate-Local (same source). Start: the same two
  islands, Action Center Local, Scale or Rotate. Each connected piece now
  turns and scales about the middle of its own item-space box (as AC-Q20)
  and with its own axes: D18-A's frame from its polygons' averaged normal
  (the selected polygons, or those around the selected edges or points, or
  every polygon with nothing selected). Like Selection Center Auto Axis,
  this overrides the chosen axes; a piece with no polygon, or whose normals
  cancel, keeps them. With Symmetry on, a point brought over from the other
  side uses its piece's mirror image. Unclear: whether the reference's
  piece axes follow D18-A's X/Z choice, and box vs average for the piece
  centre (equal in every measured case).
- **MOQ-30 Merge Meshes' Use World Transforms, provisional.** Check cases
  MOP-MM-M03_useWorld0 and MOP-MM-M04_both0
  (`specifications/checks-ac-meshops-2026-11-18`). Start: target T (empty,
  at (0, 2, 0)) and source S1 (unit cube at (3, 0, 0)), a Merge Meshes op
  on T. The cases place M03 like the default (by the source's world
  transform) and M04 like Source World Transform off (the source's own
  points), so the switch (`useWorldXfrm`, default on, a row under Source
  World Transform) is stored and saved but changes nothing here; placement
  follows Source World Transform alone. (Batch 3 first read it as
  ignoring the item transforms; the exact cases replaced that reading on
  2026-11-18.) Unclear: what the switch does change in the reference,
  perhaps falloff sampling as FAL-B2-07's Use World Transforms.
- **MOQ-31 Grouped Bevel op on regions it can't group (GitHub #25, #29).**
  A Polygon Bevel op with Group Polygons on whose selection (from
  selection ops) has islands touching only at a corner, a whole closed
  part, or a pinched outline used to fail and pass its input through.
  It now bevels each edge-connected region as a group, and a region it
  still can't group polygon by polygon [provisional]. Unmeasured: what
  the reference does with corner-touching islands (one shared new point,
  as Extrude's PE-N-03, or one each).
- **MOQ-32 Bridge op with no Selection, provisional.** TYPE-IDS "Bridge".
  A Bridge op whose Selection picks nothing in particular bridges the
  mesh's border edges, so two loose open pieces join. Unclear: what the
  reference bridges with no selection, and its Mode option order.
- **MOQ-33 Thicken op's type id, provisional.** The reference saves
  Thicken as `poly.smshift.item` named "Thicken"; Operandi gives it its own
  id `poly.thicken.item` (same values, Thicken on). LXO import should map
  the reference form to it.
- **MOQ-34 Set Polygon Type op on lines, provisional.** Lines in the op's
  selection are left as they are. Unclear: the reference's option list
  past Face (Operandi: Face, Subdivision, Catmull-Clark).
- **MOQ-35 Loop Slice op's Count and Position, provisional.** With Count
  above 1 the loops are spaced evenly and Position is set aside (as the
  tool's Count field does). No Selection cuts every ring. Unclear: how the
  reference places several loops from its one `pos` channel.
- **MOQ-36 Edge Slide op picks edges, provisional.** The reference's Sel is
  VRTX; Operandi's Edge Slide works on edges, so the op picks edges.
  Unclear: what the reference slides from a point selection.
- **MOQ-37 Hard Edge op's Mode and Selection, provisional.** The op picks
  edges (reference Sel VRTX). Mode 0 Selection hardens the chosen edges,
  1 By Angle those sharper than Angle; the reference default "auto" reads
  as Selection. Hard Edge Convert uses default smoothing settings (an op
  can't read the scene's). Unclear: the reference's Mode options.
- **MOQ-38 Part Tag op's type id, provisional.** Not in TYPE-IDS.md;
  Operandi uses `pmodel.partTag.item` with one channel `partName`, after
  Material Tag. Needs the reference's id and channel name.
- **MOQ-39 Batch 2 selection ops, provisional.** Select Connected (whole
  parts the selection below touches), Select by Polygon Tag (Material or
  Part), Select Coplanar Polygons (grows across edges while within Angle
  of the polygon it grew from), Select by Hard Edges and Select Vertex
  Map Value (weight maps; a point with no value is 0). Type ids and
  channels are Operandi's own. Not built: Convert (selection ops already
  convert between polygons, edges and points), Select by Falloff and
  Conditional Loop (need specs). Select by Border is Select Border.
- **MOQ-40 Assign Selection Set stores polygons, provisional.** Reference
  Sel is VRTX; Operandi stores the chosen polygons as a polygon set.
- **MOQ-41 Curve Convert op converts in place, provisional.** TYPE-IDS
  "Curve Convert". The buttons make a new item; the op replaces the curves
  (new ones at the end of the list). "Curve to Bezier" = its defaults.
- **MOQ-42 Curve Winding Aligner Axis, provisional.** Only Automatic is
  built (CC-Q4); Axis is kept, not shown.
- **MOQ-43 Curve Fill op, provisional.** One flat polygon per closed curve
  (CC-10); Fill Steps, Edge Weight, UVs, curve range kept, not shown; no
  holes (CC-11). Most hidden reference channels aren't kept.
- **MOQ-44 Curve Boolean op driver, provisional.** Driver = unselected
  curves; no Selection = all cut each other. Close/Trim/Offset/Self
  Intersection kept, not built.
- **MOQ-45 Edges to Curves op lists, provisional.** Curve Type Polyline /
  Curve / Bezier, Mode Continuous / Separate, Delete Geometry removes the
  polygons the edges belong to; Bezier Tangent Weight kept, not shown.
- **MOQ-46 Curve Rebuild op, provisional.** Evenly spaced by length along
  the drawn shape; result a Curve (lines stay lines). Rebuild Spans not
  built.
- **MOQ-47 Radial Sweep op id, provisional.** `radial.sweep.item`, one op
  (reference: Sweep Effector + Helix). Unused Sweep Effector channels not
  kept. Mesh space.
- **MOQ-48 Curve Sweep op profile, provisional.** 8-sided round profile,
  diameter Size; End Caps None/Start/End/Both; curves stay. Preset shapes,
  start/end percent, normal finding and gradients not built or kept.
- **MOQ-49 Text op choice order, provisional.** Location Top/Middle/Bottom,
  Justification Left/Center/Right by position; Operandi's font default.
- **MOQ-50 Arc op channels, provisional.** From the help table, not
  confirmed; Arc tool defaults; Start/End give direction and plane only.
- **MOQ-51 Sweep Effector and Create Polygons ops not built.** A bare Sweep
  Effector needs a generator in its Tool Pipe (Curve Extrude and Radial
  Sweep already are generator + sweep); Create Polygons' Positions are a
  runtime list that isn't saved, so a stack can't hold them.
- **MOQ-52 Create UV Map op, provisional.** Projection Type by position
  in the tool's list; World Space kept, not built (mesh space).
- **MOQ-53 UV Relax op, provisional.** Mode/Boundary by position ("lscm"
  = Conformal); transform-handle channels kept, not built; unchosen
  polygons hidden from the relax.
- **MOQ-54 UV Unwrap and Relax op, provisional.** Unwrap step as the UV
  Unwrap op (MOQ-26), then Relax; Relax Mode default Adaptive.
- **MOQ-55 Convert UV to Mesh op, provisional.** Result is the layout
  alone (the command makes a new item).
- **MOQ-56 Create Vertex Map types, provisional.** Weight, UV, Morph, RGB,
  RGBA by position; blank name adds nothing.
- **MOQ-57 Set Weight blank map, provisional.** Blank Map Name = first
  weight map, else "Weight"; Map Type kept, not built.
- **MOQ-58 Flood Weight High/Low, provisional.** 100% / 0%.
- **MOQ-59 Set RGB, provisional.** Blank name = first colour map, else
  "Color"; RGB alpha 1; Force Discontinuous etc. kept, not built.
- **MOQ-60 Geodesic Weight, provisional.** Shortest edge-path distance
  from the op's chosen points (Input Array isn't saved); no Selection adds
  nothing; Triangulate and advanced kept, not built.
- **MOQ-61 Set Seam blank map, provisional.** "UV Seam" (UVS-P02).
- **MOQ-62 Set Vertex Normals channels, provisional.** Tolerance, Method,
  Angle, Power, Color Map Name kept, not built; Vertex Normal Options
  defaults. UV Transform (`uvtransform`, help table only) not built.
- **MOQ-63 Path slice ops' cutter, provisional.** Curve Slice, Pen Slice
  and Primitive Slice (Operandi ids `curve.slice.item`, `pen.slice.item`,
  `prim.slice.item`; the reference saves generator + `effector.cutter.item`
  pairs) cut in mesh space with the tools' engine. Operation lists the
  built cuts by position (Core, Tunnel, Stencil, Slice); Axis by position
  X, Y, Z, Custom, Action Axis; depth0/depth1/preview kept, not shown.
  Select Sliced Edges adds the new edges to the named edge set. Slice
  Effector alone (needs a generator) and Lace Geometry (no channels in
  TYPE-IDS) not built.
- **MOQ-64 Curve/Pen Slice path, provisional.** Curve Index / Polyline
  Index count the mesh's own curve elements in order (no curve items
  yet); the curve isn't cut and stays; Action Axis = world Y; no curves =
  pass-through, an index past the last curve fails. Curve Slice samples
  Steps even steps by length over [Start+Slide, End+Slide]; Automatic
  keeps a step per Smoothing Angle turned. Alignment channels not kept.
- **MOQ-65 Primitive Slice shape, provisional.** Type = Rectangle,
  Circle, Ellipse (Preset not built, Content Preset channels not kept);
  the shape's axis is always the Axis X/Y/Z vector (default Z); Action
  Axis = that vector. Handles, Lazy Apply, Exponent, Extend Open Edge not
  kept.
- **MOQ-66 Tool pipe steps, provisional.** Symmetry (`symmetry.tool.item`)
  and Set Work Plane (`pmodel.workplane.item`) pass the mesh through and
  set state for every op above until replaced; an off step sets nothing.
  Symmetry adds the mirror partners (matched by position across the
  mesh-space axis = 0 plane, tolerance 1e-6 of the mesh size) of each
  op's resolved Selection; it doesn't mirror ops without a Selection or
  the moves of transform ops. Set Work Plane moves the new points of the
  Create primitives and curve shapes by Offset and Angle X, then Y, then
  Z (R = Rz·Ry·Rx).
- **MOQ-67 Falloff step, provisional.** `falloff.meshop.item` (Operandi's
  own id; TYPE-IDS has no stack falloff entry) keeps a copy of the active
  falloff layers, taken when added, as the primitive ops do (#15). Ops
  above that keep the point count have each moved point scaled by the
  weight at its old place (mesh space, clamped 0..1); ops that change the
  point count run unshaped with a note. Only positional falloffs (Linear,
  Radial, Cylinder, Noise); selection, element and vertex-map falloffs and
  per-op falloff override not built. An empty step clears the one below.
  2026-11-26 (Jason): one rule for the stack. The primitive ops (#15/#35)
  no longer keep their own falloff copy; they build with the Falloff
  step's falloff through their Falloff Scale/Offset/Twist rows (an old
  scene's copy is read and ignored). The step's falloff points are its
  view handles.
- **MOQ-68 Deformer steps, provisional.** No deformer channels in
  TYPE-IDS, so ids (`deform.bend.item`, `deform.twist.item`), values and
  defaults are Operandi's own, in a "Deformers" Add folder. A deformer
  moves only points (Selection by points; none = all) by Strength, in mesh
  space; shaping comes from a Falloff step below (MOQ-67), not a falloff of
  its own. Bend uses the Bend tool's kernel (Center = its base; an Axis
  along the Spine leaves the points). Twist turns by Angle x clamp(distance
  along Axis / Length, 0, 1). Angle starts at 90°. Plan:
  /mnt/project-files/meshops-deformers-2026-11-26/DEFORMERS-PLAN.md.
  Batch 10: Transform Effector (`deform.transform.item`: scale, then
  turn X/Y/Z, then move, about Center; starts as no change), Rotate
  Effector (`deform.rotate.item`), Magnet (`deform.magnet.item`: Offset
  times (1 - (d/Radius)^2)^2).
  Batch 11: Weight Influence (`deform.weightInfluence.item`: Offset x
  map weight, unmapped points stay), Push Influence
  (`deform.pushInfluence.item`: along averaged polygon normals x Distance
  x optional map weight), Morph Influence (`deform.morphInfluence.item`:
  relative morph offsets x Strength). A missing map fails the step.
  Batch 12: Vortex (`deform.vortex.item`: turn Angle x (1-(d/Radius)^2)^2
  by distance from the line), Lattice (`deform.lattice.item`: Bernstein
  free-form deformation in the Min..Max box, Divisions 2-16 per axis,
  offsets set by dragging its control points in the view; outside points
  stay).
  Batch 13: Spline Deformer (`deform.spline.item`), Bezier Deformer
  (`deform.bezier.item`, Bezier curves only) and Curve Constraint
  (`deform.curveConstraint.item`, nearest spot, Radius 0 = all) read the
  mesh's own curves by index (no curve items yet); frames by parallel
  transport from the world axis least in line with the start; curve
  points never move; past the ends the mesh carries on straight.
  Batch 14: Wrap (`deform.wrap.item`): each point is bound to its nearest
  spot on the Target (polygon, triangle, two weights, offset in the
  triangle's frame), saved with the step (Jason 2026-11-26); Rebind is a
  Properties button that measures it again. A changed point count below
  or missing target polygons pass the mesh through and ask for Rebind.
  Unclear: whether Rebind needs its own command for scripts.
- **UVN-O-grow-quads-seam with corner UVs only, provisional.** Spec:
  new-point-uvs-2026-11-02 (UVN-T-06). Start: the seam grid, read as corner
  UVs only (how Operandi and imported meshes keep UVs); Grow Quads on edges
  13-14, 14-15. The two border corners now match: where the border polygon
  is an island moved by one constant shift from a lower-numbered polygon
  across a shared edge (polygon 8 vs 7 and 5, u+1), its border corners take
  its UV plus that shift. A stored point UV still wins. Unclear: seams that
  aren't a pure shift (scaled or turned islands) keep the border polygon's
  UV, and chains of shifted islands take one step only.
- **Edge Extrude at an interior end (UVN-O-edge-extrude-cont/-seam).**
  Spec: extrude-2026-11-05 (EE06's end rule), new-point-uvs-2026-11-02.
  Start: a 3×3 grid, Edge Extrude Width 0.2, Extrude 0.2 on the middle
  edge 5-9, whose ends each have two more polygons. Operandi kept the end
  point and closed the end with one quad through it (three corners on a
  line): 13 polygons against 15. Now the edge between those two polygons
  slides too, both drop the corner, and each gets its own end triangle, as
  EE06 does for a cube's one: 15 polygons, and the continuous case matches
  point for point. Seam case (2026-11-18, provisional): on a flat selected
  edge whose two polygons' UVs differ by one constant shift, both ridge
  quads take the higher-numbered polygon's island (the −X wall's outer
  corners u = x/3 + 1, the ridge about 1.667); floor points and end
  triangles keep their own side. -seam now matches too. Unclear: which
  island wins when the lower-numbered polygon is the shifted one, and at
  a fold.
- **Edge Chamfer interior ends (UVN-O-edge-chamfer-cont/-seam): known
  difference, ORQ-01.** The reference's 14 polygons are Operandi's 10 plus
  two coincident zero-area triangles at each end ([5,18,17] ×2, [9,16,19]
  ×2), which ORQ-01 (decided 2026-11-04) keeps refusing. Not changed.
- **FQ-Q01 Fill Quads shape (UVN-O-fill-quads-cont/-seam), provisional.**
  Spec: new-point-uvs-2026-11-02 UVN-T-07, plus Jason's measurements of
  2026-11-18 (Grid off, border selected counter-clockwise from the hole's
  lower-left corner): a 2×1 hole gets two squares split by the middle rung;
  a 3×1 hole a square at the start end, then a diagonal from the far end's
  start-side corner to the top of that rung (two 4-sided pieces with three
  corners on a line); a 2×2 hole fans both cuts from the middle of one
  side. Built: an 8-edge loop takes three border edges at a time from a
  start point `s`, walking the way the selection runs (cuts `s`–`s+3`,
  `s+3`–`s+6`, a fan from `s+3`); `s` makes the two cuts shortest in all,
  ties go to the shorter last cut, then to the first from the selection's
  start. That gives the 3×1 result and, on UVN-T-07's loop, the
  reference's fan from point 13 (fill-quads-cont now matches exactly).
  Other loops keep the ladder of rungs (a 2×1 hole's single rung is the
  middle one; 4×1 and longer holes stay squares). UVs on an 8-edge flat
  loop: a new polygon's first two corners (its border edge into the fan
  point) keep the UVs of the polygon across that edge; the others take
  their point's continuous UV, recovered across a seam island's constant
  shift on corner-only meshes. Held back: on fill-quads-seam the reference
  also moves the untouched polygon [2, 3, 8, 7]'s corner at point 8 across
  the seam (u 1 to 2); Operandi leaves existing polygons alone. Unclear:
  which side the 2×2 fan takes when the selection starts elsewhere or runs
  clockwise, how loops of 10+ edges are cut, and whether Symmetry should
  make a hole on the axis fill as mirror images.

## USD import U1/U2 (branch usd-import, 2026-11-18)

Spec: `docs/BEHAVIOR-USD-IMPORT.md` (Operandi's own format choice; Blender
5.2.2 exports are the test files). All provisional; each is what was built.

- **USD-IQ01** The `st` UV primvar becomes Operandi's primary map
  ("Texture", the one tools look for first); every other UV primvar keeps
  its name. Round trips with the exporter, which writes "Texture" as `st`.
- **USD-IQ02** Operandi's shading doesn't read stored normals, so authored
  normals are kept as a corner normal map named `normals`, and each
  polygon whose corner normals all agree is marked flat (others smooth).
  Subdivision meshes ignore authored normals (USD's rule).
- **USD-IQ03** A Z-up file is turned by seeing every transform through the
  turn (`turn · local · turn⁻¹`, points and normals turned too), so items
  get the rotations a Y-up file would give them; rotation orders follow
  the axes (Blender's XYZ arrives as XZY with the same angles). The
  alternative, a -90° X on every top item, was rejected as noise.
- **USD-IQ04** One top-level Xform with no transform of its own (Blender's
  and Operandi's `/root`) is dropped; its children are the top items.
- **USD-IQ05** An Xform whose Mesh children have no transform of their own
  (and nothing below them) is one mesh item named after the Xform, holding
  all of them; other Xforms are Group Locators. A Mesh with its own
  transform is its own item.
- **USD-IQ06** Operandi has no instance items, so native instances and
  PointInstancer instances arrive as copies sharing one mesh until edited
  (the report counts them). The dialog's Instances row is greyed at
  Copies with that reason.
- **USD-IQ07** A material named "Default" (how Operandi's exporter names
  untagged polygons) tags its polygons with the default material; its
  Material Group still comes in, for the "Default" tag.
- **USD-IQ08** Unauthored `subdivisionScheme` means Catmull-Clark (USD's
  fallback); `loop` is read as Catmull-Clark and `bilinear` as plain
  polygons, with a note. `interpolateBoundary` edgeAndCorner → Crease,
  edgeOnly and none → Smooth; `faceVaryingLinearInterpolation` all →
  Linear UVs, none → Smooth, boundaries → Crease Edges, the corners rules →
  Crease. Crease sharpness ÷ 10 is the edge weight (the exporter's 10 ×
  weight), capped at 1; corner sharpness has no Operandi equivalent and is
  reported.
- **USD-IQ09** A missing picture keeps its layer, pointing at the written
  path made absolute against its layer, with a one-pixel size until the
  file is found, so it shows the missing-file look (USD-T5).
- **USD-IQ10** Both networks go through Operandi's MaterialX reader. Before
  that, a surface shader's lobes switched off by a zero weight drop their
  connected inputs (Blender wires the base colour picture into
  `subsurface_color`), and `geometry_normal` / `geometry_tangent` inputs
  fed only by nodes that rebuild the geometric directions (`normal`,
  `tangent`, `normalize`, `rotate3d`...) are left unset, which is the same.
  A network the reader still refuses falls back to the other one, then to
  a plain tag; the report says which and why.
- **USD-IQ11** Pictures inside a .usdz are unpacked into the project's
  `sourceimages/<file name>/`, or `<file>_textures/` beside the file
  without a project.
- **USD-IQ12** Cameras, lights and the dome light (U3) and the graphs
  sidecar (U4) are counted in the report; their dialog rows are greyed
  with that reason until those stages.
- **USD-IQ13** Constant-interpolation colour primvars (one colour for the
  whole mesh, how many exporters write `displayColor`) are not made into
  colour maps; the report names them. Guide and proxy purpose meshes are
  left out and named; `resetXformStack` is read as an ordinary local
  transform, with a note.
- **USD-IQ14** (fixed in U6) Found while testing USD-T3: the exporter writes the
  imported roughness picture layer of a MaterialX material as a plain
  value ("Procedural or layered textures written as plain values"); base
  colour and normal pictures round-trip. Export-side, not changed here.
- **USD-IQ15** (U3) A camera or light looks down its local −Z in the file
  and in the scene, so its item takes `turn · local` (not seen through the
  turn on its own side) and its children `local · turn⁻¹`; the dome's
  picture keeps its local +Y pole (`turn · world` on its image layer's
  Texture Locator). Checked against Blender's render of USD-T1: the same
  framing pixel for pixel.
- **USD-IQ16** (U3) Light units, the export's read backwards (Blender's):
  a normalized light's power is π × intensity whatever its size, an
  unnormalized one's π × intensity × its own surface area (item scale not
  counted); × 2^exposure; colour temperature tints when enabled; a
  ShapingAPI cone under 90° makes a spot (cone = 2 × the half-angle,
  softness = Cycles' spot blend); DistantLight intensity × 4 is the
  Cycles sun strength, Spread Angle = 2 × angle. Mesh lights are left out.
  Only the first DomeLight is used; its colour tint is left out (noted).
  Fixed in the export at the same time: area lights were written as their
  radiance, too dim by their area; they are now radiance × area,
  normalized, which reads back the same.
- **USD-IQ17** (U3) USD camera lens and film values are tenths of a scene
  unit; Film Fit is Horizontal (the horizontal aperture spans the
  picture); a film not 16:9 turns on the camera's resolution override at
  the same width and the film's shape; USD's fallback clipping range
  (1 to 1,000,000) means Clipping stays off; an fStop turns depth of field
  on.

## 3D painting stage 7: sculpt extras (PNT-151, 241..247, 260..264, 2026-11-18)

- **PNT-Q63** Sculpt brushes keep their own tip choice and tip values
  (PNT-42/Q10); the Tablet Nozzle and the ink stack are one for paint and
  sculpt. Picking Smooth or Hard on the Sculpt tab takes the extra tip off.
- **PNT-Q64** A Texture/Stamp tip on a sculpt brush is signed around its
  Offset only with Use as Mask on (white pushes out, black in, the zero
  point not at all); otherwise its alpha is the weight. A colour (vector
  displacement) picture is read as brightness: moving points by its RGB
  vector isn't built.
- **PNT-Q65** Stamp and Text tips on sculpt brushes act as ordinary dab
  tips; the press-drag-release "stamp once" gesture is paint-only so far.
- **PNT-Q66** On sculpt brushes the nozzle's Rotation and speed aren't
  used (dabs don't turn; speed reads 0); strength (per-dab Offset Amount),
  size and position jitter are.
- **PNT-Q67** Random Ink on sculpt scales each dab's strength by its
  brightness; Hue and Saturation do nothing. Image Ink scales by the
  picture's brightness through the view; Parametric by its mask.
  Projection Ink is greyed ("Projection Ink works with the paint tools
  only").
- **PNT-Q68** The mask is one value per point per mesh, kept for the
  session (undoable, never saved). Mask brushes use the Vertex Map Tools'
  formulas on a hidden map the meshes lend for the stroke; Ctrl on Erase
  and Set Value does what it does on those tools. The red display is drawn
  over front-facing polygons without a hidden-surface test. The mask
  brushes wear the Vertex Map icons (no new artwork yet). With Sub-D the
  mask is on the cage points.
- **PNT-Q69** The starter brushes are built in, not preset files; a crumb
  brush's picture becomes a scene image named after the brush the first
  time it's applied. Dynamic Smooth only differs from Smooth with a pen.
- **PNT-Q70** Geometry to Brush relaxes the patch by neighbour averaging
  with its border held (Iterations passes, stopping early once settled),
  scales the largest offset to white (or full colour), and lays the
  picture out in the plane facing the Projection Axis. The picture is a
  new image "Brush from Geometry"; it becomes the sculpt Texture tip (and
  the paint tools' when one is on), Use as Mask on, Offset 50 %.
- **PNT-Q71** Copy to Morph stays greyed: Operandi has no morph maps.
- **PNT-Q72** Background Constraint on sculpt brushes is the app's
  Constrain to Background (on/off, target, offset, double-sided) with the
  vertex tools' search along the last view pressed in.
- **PNT-Q73** Clear Mask and Bake Falloff act on the active mesh.
- **PNT-Q74** (test drive 2026-11-26) Zoomed in on a small image the
  texels sit further apart on screen than the dabs (a couple of points),
  and Smudge changed nothing. Smudge now pulls colour only once the drag
  has gone one texel spacing (every dab when texels are dense, as
  before), and Smudge and Clone look up to 8 points (was 2) for the
  texel under a screen point. Provisional: the spec doesn't say how
  sampling works between texels.
- **USD-IQ18** (U4) A sidecar graph replaces a material only when its
  name is exactly the USD material's prim name (Blender writes both from
  the material's name; names Blender made safe for USD, with spaces or
  dots, don't match). It replaces the whole Material Group's contents
  (the MaterialX or Preview Surface layers and their Texture Locators)
  with one Node Material layer named "<material> Node Material". A graph
  Node Materials can't render is refused whole and the flattened material
  stays (reported).
- **USD-IQ19** (U5) Mesh Instances (`Item::instance_of`, native version
  51), built provisionally ahead of IT-PHY-04's Control+Shift drag: an
  instance shares its source's mesh (one copy-on-write mesh) and keeps
  its own transform, name, parent and visibility. After every recorded
  edit the scene re-shares them; an edit made on an instance reaches its
  source and the other instances; deleting the source leaves the
  instances as ordinary meshes. Item-list marking, making instances by
  hand and the Instance action are not built. The import makes the first
  item showing a prototype's mesh the source and the rest its instances
  (Instances option; Copies keeps separate items).
- **USD-IQ20** (U6) The export writes each Material Group's MaterialX
  network (Operandi's own .mtlx writer: open_pbr_surface and image nodes,
  nodedefs ND_<category>_<type>, extract/convert named after their input,
  normalmap as ND_normalmap_float) as Shader prims under the Material on
  `outputs:mtlx:surface`, beside the UsdPreviewSurface on
  `outputs:surface`; the base material (no group) is Preview Surface only.
  Creases are written on every mesh, not only Catmull-Clark ones (Blender
  writes and reads them on plain meshes). Mesh Instances are written as
  separate meshes (no USD instancing yet). USD-IQ14 is fixed: a data map
  read through one channel and blended by its own alpha is written as
  that channel.
- **USD-IQ21** (render-camera-picks, 2026-11-18, provisional) Which camera
  an imported USD scene renders through. Starting state: a stage with
  several cameras. Built: the `camera` relationship of the RenderSettings
  prim the stage's `renderSettingsPrimPath` metadata names, else of the
  first RenderSettings prim with one, becomes the render camera; with
  none, the render camera stays unchosen (the first camera renders).
  Blender 5.2.2's export of the chess set writes no RenderSettings and no
  camera metadata, so its active camera (Camera_002) can't be recovered
  and that scene renders through its first camera. Unclear: should an
  import pick a camera some other way (the last one, one named in
  customLayerData), or ask?
- **USD-IQ25** (usd-pointinstancer-export, 2026-11-19, provisional) When
  the export writes Mesh Instances as a UsdGeomPointInstancer
  (crates/operandi-usd/src/point_instancer.rs). Built: at least 16
  Mesh Instances of one source under one parent, each with no items
  below it, the source's materials and subdivision, and a positive
  scale, become one PointInstancer `<source>_instances` under that
  parent: positions, orientations (quath, plus quatf orientationsf for
  readers since OpenUSD 24.11), scales, ids 0..n, invisibleIds for eyes
  that are off, and one prototype, an instanceable reference to the
  source's `/root/_prototypes` meshes. customData `operandi:scatter` and
  `operandi:names` let the import bring the same items back under the
  parent, named, as Mesh Instances of the source. Fewer, or the ones
  that differ, stay instanceable Xforms. A 1,000-copy scatter: 63 KB
  .usdc and 0.017 s to import, against 116 KB and 0.097 s as Xforms
  (debug build). Lossy: positions are float (Xforms keep doubles),
  Euler angles and rotation order come back as an equivalent rotation,
  and other programs see points, not named prims. Unclear: is 16 the
  right threshold, should it be a preference, and should instances with
  children still scatter (their children as separate items)?
- **USD-IQ22** (usd-export-gaps, 2026-11-19, provisional) How the USD
  export writes mesh data USD has no Operandi equivalent for, and how the
  import reads it back. Built: weight maps as float primvars; typed maps
  as float/float2..4 primvars (6-number kinds as float with elementSize 6;
  rgb3/rgba4 as colours); colour maps as color4f primvars; the first
  colour map of a mesh as displayColor (+ displayOpacity when any alpha
  isn't 1). Point maps are `vertex`, corner maps `faceVarying`; a map
  that misses points or corners is indexed with unauthoredValuesIndex 0.
  Morph maps are UsdSkelBlendShape prims bound through UsdSkelBindingAPI
  (no Skeleton; Blender 5.2.2 reads them as shape keys). Parts and polygon
  sets are face GeomSubsets, point sets point GeomSubsets (families
  operandi:part / operandi:polygonSet / operandi:pointSet); edge sets go
  in the mesh's customData as point pairs (USD 0.26 also has edge
  subsets; unused). Each carries customData "operandi" (its Operandi name,
  map kind, level). Mesh Instances (supersedes USD-IQ20's "separate
  meshes"): each item of an instance group gets a child Xform, no
  transform, instanceable, referencing `/root/_prototypes/<source>` (a
  class); one that shades or subdivides differently from its source is a
  copy (reported). The import takes such a holder's meshes as the item's
  own, so instance_of comes back. Still not written, each reported:
  selection set groups, smoothing group names, Solid Sketch nodes, normal
  maps beyond the shading normals, kit items, locators with nothing
  below. Unclear: should typed vector maps be turned with the up axis on
  export too (they are on import), and should rgb3/rgba4 prefer
  displayColor over colour maps?
- **USD-IQ23** (usd-import-foreign, 2026-11-19, provisional) What the
  import makes of other programs' mesh data (no Operandi customData).
  Built: a float primvar per point (vertex/varying) becomes a weight map,
  per corner (faceVarying) a corner weight map; a uniform (per-polygon)
  primvar goes on each corner of its polygon. Ints and vectors become
  typed maps: 1 value weight1, 2 uv2, 3 by role (normal3f normal3,
  point3f absolute_position3, vector3f or no role vector3, ints rgb3
  unturned), 4 rgba4; ints are stored as floats. Colours stay colour
  maps. A float2 with no texCoord role is a UV map only when named like
  one (st, st0, st_*, uv*, UV*, UVMap, map1). Constant primvars,
  elementSize > 1 and `skel:` skinning weights are left out (constant
  ones noted). GeomSubsets of any family but materialBind and Operandi's
  own: face subsets are polygon sets, point subsets point sets, edge
  subsets edge sets, named after the subset prim. Every bound
  UsdSkelBlendShape is a morph map (Blender shape keys); inbetweens are
  left out. Blender 5.2.2 writes vertex groups as plain float primvars,
  so they come in as weight maps. Unclear: should ints have their own map
  kind, should a no-role float3 be a direction (turned) or plain numbers,
  and should two subsets of one name in different families merge (they
  do now)?

## Action Center Local axes and UV bridge (2026-11-18, branch uv-relax-bridge)

ACL-01..04 (Local per-island axes) replace AC-Q21's axis choice (D18-A's
normal Y with projected world Z) with the principal-axis rule. Checks
AC-move-Local-islands and AC-move-Local pass
(`tests/action_center_local_axes.rs`).

- **ACL-Q1 Spread weighting, provisional.** Each moved point of the island
  weighs the same (not by polygon area). Start: an island of one big and
  several small quads, Local, Move along X. Does area change the axis?
- **ACL-Q2 Local Y and Z, provisional.** Local Y is the principal
  direction nearest the island's averaged polygon normal (pointed with it;
  the least-spread one when the island has no polygon), X is the other
  one nearest item X, and Z = X × Y. This keeps D18-A's Y = normal, so
  Local Rotate/Scale about Y still turn about the normal; the brief's
  suggestion was Z = normal. X is chosen among the two in-plane
  directions only, so a wall-like island never takes its normal as X.
  Start: a quad in the YZ plane, Local, Move along X. Which way does it go?
- **ACL-Q3 Ties, provisional.** Two in-plane directions equally near X:
  the one of more spread (the L of three quads takes −45°). Equal spread
  (a square): the item's X laid into the island's plane. Ties are within
  1e-4 relative. Tests avoid other ties.
- **ACL-Q4 Turned items, provisional.** "Nearest X" uses the item's own X,
  so axes turn with the item; AC-move-Local's square island (a tie, not
  checked by the case) moves along the item's X. With Symmetry, a mirrored
  island's X is pointed along +X again, so the pair moves as mirror images.
- **ACL-Q5 Straight-line islands: superseded** by AC-73 (action-centers
  rulebook 2026-11-21): the tool axis nearest the line is turned onto it
  (built; which axis is AC-Q1, provisional), and AC-75: the Local handle
  takes the last selected element's cluster frame (built; handle position
  stays the selection's box centre, AC-Q2).
- **UV-Q26 Seamless cube unwrap layout (not built, needs a decision).**
  Start: a 1 m cube, no seams, UV Unwrap with Reset values. The reference
  turns two of the six one-face islands 45° and fills about 0.88 × 0.99;
  Operandi keeps all six square in a 3 × 2 grid, 0.99 × 0.66. Matching it
  goes against UV-Q18 (islands turned square to U and V) and UV-Q21
  (islands laid out without rotation, which gives UVW-11's 1 × 0.75), so
  it waits for a decision. Needed: the reference's UVs per face for this
  case, and whether UVW-11's 1 × 0.75 cross must still hold.

## Viewport header toggles (branch `viewport-header-toggles`, 2026-11-18)

- **HT-D1 XR-1 / UIC-24 deviation, Jason's call.** XR-1's Normal / Ghost /
  X-Ray radio group is one Transparency pop-up button (HT-1): glyph of the
  active meshes' mode, lit when not Normal, corner triangle (UIC-25). A
  quick click toggles Normal and the view's last non-Normal mode (X-Ray
  until one is used) [OP]; hold or right-click opens the list; Alt for the
  inactive meshes (UIC-27). Then a small separator (HT-2) and three toggles
  running the existing `view3d.showLocators`, `view3d.inactiveInvisible`
  and `view3d.sameAsActive` (HT-3), lit from their `?` answer (UIC-223).
  The last mode used is per view for the session only, not saved.
- **HT-Q2 Hold time.** 400 ms assumed (provisional).
- *Provisional:* the toggles' and Transparency button's glyphs are drawn
  shapes like the other header buttons, not generator icons yet.
- **HT-5/HT-6 Show Weightmaps / Show UV Distortion (spec v1.1, branch
  `viewport-weights-distortion`).** `view3d.showweights state:` and
  `view3d.showDistortion enabled:`, registered with the other viewport
  commands (UIC-160/165, class Ui), per-view rows in Viewport Properties
  (Active Meshes) and saved with the view; first after the separator.
  Greyed with a tip in Wireframe, like the Transparency button.
- **HT-Q3 Weight tint strength.** The weight colour replaces the colour it
  covers before lighting (so VMT-30's colours show, lit); distortion mixes
  at the UV view's own opacity (150/255). Provisional.
- **HT-Q4 Smooth surfaces.** Subdivision pieces and Tab patches take their
  cage polygon's average weight (flat per polygon), not per-point blending.
  Provisional.
- **HT-Q5 Name map. Answered (Research, 2026-11-18):** both are genuine
  reference names. Added to `MODO-NAMES-SUBSET.txt` as
  `view3d.showweights(state:boolean)` and
  `view3d.showDistortion(enabled:boolean, bgmesh:integer)`; the
  Operandi-only marks are off. *Provisional:* `bgmesh:1` is accepted but
  treated as 0. UV Distortion is one setting per view, so honouring it
  for inactive meshes (as the Transparency button's Alt does) would need a
  separate inactive setting through the saved view, presets and GPU tint.
- *Speed:* with UV Distortion on, any mesh change rebuilds the item's
  geometry (the whole map's ratio can recolour every polygon); with
  Weightmaps on, a weight edit does. Both off by default.

## Collapsible top bar and side bars (branch `top-bar-collapse`, 2026-11-18)

- **TB-01 UIC-16 deviation, Jason's call.** UIC-16 puts a collapse control
  at the bar's left end. Here the Operandi logo is that control: a click
  on it collapses the whole top row (logo, in-window menus, file state,
  layout tabs) to a thin line across the window. The line takes the
  theme's hover colour under the pointer; a single click on it does
  nothing, a double-click or a drag down past a few points brings the bar
  back. Tooltips: "Collapse bar" on the logo, "Double-click to show bar"
  on the line. No separate collapse button.
- **TB-02 Layout tabs' "‹" button removed, Jason's call.** The small
  arrow right of "Saved / Untitled" that hid only the layout tabs is gone,
  with its state: an old layouts.json `collapsed` key is ignored, so the
  tabs show. Its command `operandi.layout.collapse` had no other user and
  is replaced by `operandi.bar.collapse bar:top|left|right on: width:`.
- **TB-03 Side bars collapse, Jason's call.** A double-click on a docked
  side bar's resize line collapses it to a 6 pt line at that window edge,
  with the same look and code as the top bar's line (collapse_line.rs).
  A double-click on the line reopens it at its old width; a drag out
  reopens it where the drag lets go, or at the old width when that is
  under the bar's minimum; a single click does nothing. Tooltip
  "Double-click to show panel"; resize cursor. With the right bar
  collapsed the Command History entry line goes with it.
- All three states are kept in layouts.json between launches (app-wide,
  not per layout). *Provisional:* a fresh line ignores clicks for 0.35 s,
  so a double-click on the logo doesn't collapse and reopen at once.

### Docking J2 (owner's docking brief, D-02)

Start with two views in the centre and Tool Properties docked under the
toolbox. Drag the Tool Properties grip onto an edge of one view.

**Answered (owner, 2026-11-19 11:00 CT).** Split that view's slot and dock
Tool Properties beside it. The other view keeps its slot (UIC-72).

### Docking J3 (owner's docking brief, D-02, D-04, D-12)

Start in Model with Shader Tree selected in the upper right group and
Properties selected in the lower group. Drag the upper group's corner dot
onto the lower group's tab row or compass centre. Which tabs join the
destination, in what order, and which comes to the front?

*Answered by the owner, 2026-11-19 (D-02, D-04):* A drop into a
group joins its tabs. Insert new kinds in source order at the tab-row
drop position, or append them at compass centre. Keep any matching
destination tab in place and activate it if it was the dragged front
kind. Compass arrows, edge strips and dividers create separate panes
and never join their tabs. Keyboard routing follows each group's
visible active tab (D-12).

### Mixed docking tab rows (owner's docking brief, D-04)

Start with Item List and Shader Tree in a sidebar row. Drop a viewport
grip between those two tabs. Should the viewport appear inline between
them, with the sidebar controls in the same row, or should the sidebar
row remain together when it shares the slot with the viewport? Inline
insertion requires a common tab model for the registered panels and the
dock tree panes.

*Provisional:* Keep the registered row together as one entry when it
shares a slot with a tree pane. A drop between registered groups uses
the ordered merge above. A drop on a dock tree tab row uses that row's
exact insertion position.

### Leaving the source row (owner's docking brief, D-05)

Start with several tabs in one row, then drag one past the edge of the
row or through work-area space between targets. How much room should a
reorder gesture have, and what should happen on a release without a target?

*Provisional:* The source row has a six-point margin for reordering. A
release inside the main window without a docking target leaves the layout
as it was. Moving the last registry tab to a different slot removes its
empty source group.

### Floating dock windows (owner's docking brief, D-06 and D-10)

Float Tool Properties or a mixed group, then try to move or close its
window, or switch layouts. Where should docking begin? Where should a
closed panel return, and which layout owns a floating group?

*Provisional:* Drag the title strip inside the window to dock. The system
title bar moves the window. Closing a window holding a dock panel returns
its contents to the main window's right edge. Windows holding dock panels
belong to the layout that owns their panes. Existing palette and
view-only window behaviours follow LAY-50 through LAY-53.

### Tool Tabs docking (owner's docking brief, D-07 and D-12)

Open a stock or older layout, or move Tool Tabs into a tab group or a
floating window. Which orientation and size should Tool Tabs use where
there is no dock side, and what should the arrows do when the row overflows?

*Provisional:* Project the original vertical strip beside the toolbox
until a docking move needs separate tiles. A tab-group join or floating
window keeps the last orientation. An edge, arrow or divider drop sets
the orientation from its side. A horizontal edge drop fixes the strip
to one row, including its grip and padding.
Each scroll arrow advances most of a row without changing the active
category. Scroll position is transient. A short standalone vertical
strip scrolls vertically.

### Tool set bar (owner's docking brief, D-08 and D-12)

Open a customized layout and choose UV or Paint. Which existing category
opens, and should the Paint set also show the Paint layout's brush strip?
Where do the switches from the retired layout tab row go?

*Provisional:* Model opens Create, UV opens the existing UV form and
keeps the UV-enabled category strip, and Paint opens Paint Tools in the
existing Sculpt/Paint set. Its preset strip stays on the layout that
opened it. Ordering and stars remain available through their commands,
with the starred-only filter applied to the Layouts pie. Window >
Layouts reuses Layout > Layouts (intentional menu snapshot change).

### Layout-owned viewport windows and the shipped Model (D-10 and D-11)

Tear off a viewport from the dock tree, then switch layouts. How should
this interact with the earlier LAY-Q2 provisional behavior? The default
image also names Item List, Display and Channels while the current
registry has Items and item-specific categories inside Properties.

*Provisional:* Dock tree layouts own their torn-off viewports. Older
column layouts keep the existing app-wide viewport tear-off behavior,
and viewport windows already embedded in a saved layout stay with it.
Named palettes keep their LAY-52 behavior. Use the registered Items and
Properties panels, including the existing Mesh Stack tab and the
item-dependent Properties categories. The horizontal Tool Tabs row fits
its contents. The lower row keeps the previous Model column and divider
proportions.

The legacy Toolbox palette removes the combined toolbox column for as
long as it is open. In Model's dock tree, Tool Properties is a separate
dock panel and stays docked when the Toolbox palette opens.

### Panel recovery (D-04, D-10)

If the original sidebar row is missing or both rows are floating, where
should Window > Panels put a recovered panel?

*Provisional:* Panel recovery uses the docked Items row first, then the
docked Properties row. It creates a row on the right if one is absent.
If both rows float, it opens the recovered panel on the right of the
main window.

### Short side strips (D-03, D-04)

With multiple side panes stacked beside the viewport, which pane owns the
short strip?

*Provisional:* The short strip follows the closest visible side pane. For
equal horizontal gaps, it follows the upper pane. The narrow divider
remains an insertion target.

### Mode bar and message line (owner, 2026-11-19)

The owner asked for the mode bar (Selection, Action Center, Falloff,
Symmetry, constraints, snapping, Work Plane, Drop Action, Render, Preview)
to dock top or bottom only, saved in layouts, default bottom above the
command line; and for the message line to become a thin strip at the
bottom of the Tool Properties column.

*Provisional:* The mode bar is a full-width bar at the window's top or
bottom, moved by dragging its grip into the other half of the window or by
its right-click menu (Esc cancels the drag). It is not a dock-tree leaf and
doesn't float. Layout files store `mode_bar_bottom`; files without it keep
the bar at the top. The message line moves under Tool Properties only in
dock-tree layouts (every shipped layout); older fixed-column layouts, a
floating or collapsed Tool Properties, and long Help text keep it under the
views.

### Companion panels and layout highlight (owner, 2026-11-19)

*Provisional:* Picking the UV tool set splits the active view and makes
the new view a UV view, unless a UV view already shows; leaving the set
closes it only if the set opened it. This supersedes D-08's "a set button
never changes the layout" for companion views only. While open, the
companion view is part of the layout like any view (it saves with it);
switching layouts forgets which view was the companion. A layout's entry
(pie, Layout and Window menus) is lit when it is the open layout, with its
own tool set and no companion view open.

## Material-value render outputs (branch `material-value-outputs`, 2026-11-19)

Thirteen Render Output effects that write a material's own values, not
light (material_values.rs). Starting state for all: a mesh with the Base
Material, one of these outputs added under Add Layer > Special > Render
Output > Material Values, then Render.

- **MV-Q01 Diffuse Coefficient.** *Provisional:* Diffuse Color × Diffuse
  Amount, for every shading model (Principled renders Base Color =
  Diffuse Color alone; this output still multiplies).
- **MV-Q02 Diffuse Energy Conservation.** Provisional (2026-11-20): yes,
  specular and coat take their share (straight-on reflectance, no angle
  dependence); awaiting Jason. Diffuse Coefficient × (1 − Transparent
  Amount) × (1 − Subsurface Amount) × (1 − Metallic as rendered: the
  channel, plus Reflection Amount × (1 − Reflection Fresnel) with Match
  Specular off) × (1 − F0) × (1 − Coat Weight as rendered × 0.04). F0 is
  the specular layer's straight-on reflectance: 0.08 × Specular Amount
  (0.04 at 50 %), or ((n − 1) / (n + 1))² with Use Refractive Index on.
- **MV-Q03 Specular Coefficient.** *Provisional:* Specular Color ×
  Specular Amount.
- **MV-Q04 Reflection Coefficient.** *Provisional:* Reflection Color ×
  Reflection Amount; with Match Specular on (the default) it equals
  Specular Coefficient, since reflections follow specular then.
- **MV-Q05 Where the values come from.** *Provisional:* the folded
  per-surface values (texture layers count) at the first camera hit,
  averaged over the pixel's samples like other passes; nothing hit (the
  background) is 0. Grey values are shown as grey (all three channels).
  A surface seen through a transparent one shows the front surface's
  values.
- **MV-Q06 dPdu / dPdv.** *Provisional:* unit world-space directions from
  the mesh's first UV map (Cycles' mikktspace tangent, perpendicular to
  the shading normal); dPdv = normal × dPdu. Raw −1..1 values, no remap,
  so negative components show black. A mesh without UVs writes 0. Unclear:
  whether the reference uses the named texture's UV map, the
  unnormalised derivative length, or a 0..1 remap.
- **MV-Q07 Studio clay.** *Provisional:* while any material-value output
  is enabled, no surface is swapped for the studio clay (its shader writes
  no values), so an untouched scene's Diffuse Amount output shows the
  default material's value, not black. The picture itself looks the same.
- **MV-Q08 AOV names (master plan A31).** Each value is a colour AOV named
  `operandi_value_<name>` (`diffuse_amount`, ..., `dpdu`, `dpdv`), so a
  future MaterialX/OpenPBR surface graph only has to write the same named
  AOVs. The table in material_values.rs gives each value's OpenPBR meaning.

## USD curves (2026-11-19, branch usd-curves)

- **USD-IQ24** (provisional) How line elements map to USD curves, both
  ways (crates/operandi-usd/src/curves.rs). Export, exact: polylines are
  linear BasisCurves; Beziers cubic bezier; closed B-Splines cubic
  bspline periodic, open (clamped) ones cubic NurbsCurves with clamped
  uniform knots; Curves cubic catmullRom (periodic when closed), an open
  end without Start/End Control getting a made-up control point with the
  same end tangent, listed in customData `operandi:kinds` so the import
  drops it. One prim per schema/type/basis/wrap in the item's Xform;
  every curve has its own points (shared points come back unwelded); a
  weight1 typed map `widths` is the widths. Import: linear polylines,
  bezier Beziers, periodic bspline closed B-Splines, other bspline
  converted exactly to Beziers, catmullRom Curves with Start and End
  Control (`pinned` ends get USD's 2·P0−P1 point), NURBS of order 2
  polylines, cubic unweighted clamped-uniform NURBS B-Splines, other
  NURBS sampled into polylines (12 steps a knot span, reported),
  HermiteCurves polylines through their points (reported). Widths
  (constant, uniform or per point) come in as the `widths` typed map, in
  metres. Lossy: NURBS other than the above, hermite tangents, widths
  that aren't per point/curve/constant, curve primvars other than widths,
  and the order of line elements among polygons. Unclear: should
  imported widths be a native curve thickness once Operandi has one, and
  should a closed Curve export with its point order rotated to match
  other programs' start point?

### Shared pivot service (ac-symmetry-rulebook-2026-11-19, branch shared-pivot-service)

Built from RULEBOOK.md v1.2 and cases.json. Matrix results and the cases
each question holds back are in `tests/ac_symmetry_rulebook.rs`
(`KNOWN_GAPS`).

| ID | Starting state and actions | What's unclear / what Operandi does |
|---|---|---|
| ACS-Q3 (rulebook) | Fixture pair (or pair + straddle, or one quad), Action Center Selection, Selection Border, Element or Local, Symmetry X/Y/Z or off; Scale 2, 1.5, 1 typed | The reference's custom axis frame (tool Z along the axis, tool X world X made perpendicular, in 11 of 16 cases) differs from Operandi's D18-A Selection axes (Y along the normal, Z near world Z), Element's frame (`PickedFrame::from_z`) and Local's ACS-12 frame. Centres match in all 26 hands-on cases; 14 cases' points differ only by the frame and are known gaps. *Provisional:* Operandi's frames unchanged. Adopting the reference frame would change D18-A, a user-approved rule: the owner's call. |
| ACS-Q4 (rulebook) | Item moved +0.7 X, Symmetry X, centre typed at world X 0.2 or 0.7 | The reference picked the primary side from the centre's world X. *Provisional:* Operandi tests the centre against the item's own plane (ACS-08), so these 10 cases are known gaps. |
| ACS-Q5 (rulebook) | Symmetry X, a selection with points on the plane (a piece across it, a centre loop) | Do points on the plane count toward the + half's centre (ACS-01)? *Provisional:* no; only points strictly on + count (on-plane counts as −, ACS-03). If nothing is strictly on + or −, the whole selection is used. |
| ACS-Q6 (rulebook) | Parent, Pivot, Pivot Center Parent Axis with a turned parent | *Provisional:* Parent = the parent's world position (the world origin without a parent) with the parent's axes; Pivot = the mesh item's world position with its axes; Pivot Center Parent Axis = Pivot's centre with the parent's axes. Only unturned items were recorded. |
| ACS-Q8 (rulebook) | Symmetry with Work Plane on, typed Move (SYM-B2-08 oracle) | Where the scripted centre sat relative to the moved plane isn't recorded. *Provisional:* no Work Plane special case any more; the primary side is the centre's side as everywhere (ACS-03). The SYM-B2-08 oracle passes with Operandi's own centre. |
| ACS-B1 | Symmetry X, select a − element first, then its mirror (or look at the − half in the view); open a tool | N27 (Jason, 2026-11-17) put the handle on the first-picked side, or the side the view looked at. ACS-01 says selection order and pointer position don't matter: the handle is on the + half, and a click (ACS-07) or typed centre moves the work to the − side. *Built:* ACS-01; the view-side and first-pick rules are gone. Tools without a pivot of their own (Edge Slide, Vertex Slide, Jitter, Relax, Bend's side, Twist Bevel, the two-pass Duplicate/Sketch tools) take the same primary side. |
| ACS-B2 | Symmetry X, a connected piece across the plane selected (N30) | N30 kept such a piece's handle on the plane; ACS-01 takes the + half's points (seen with the straddling quad). *Built:* ACS-01. |
| ACS-B3 | Symmetry X, Move typed with no centre placed (SYM-TR-02) | SYM-TR-02's "typed values as entered on the − side" was recorded by script with the centre at the origin. ACS-03 makes it the centre's side; Operandi's centre is on the + half (ACS-01), so typed values now apply as entered on +, like a drag. The old oracle tests now place the centre at the origin to keep checking SYM-TR-02. |
| ACS-B4 | Action Center None, any tool | The rulebook table makes None "like Automatic": the bounding-box centre. VAC-T12 had the vertex average. *Built:* the box centre. |
| ACS-B5 | Selection Border, Symmetry X, a whole row of quads across the plane (Jason's hand check) | "The handle sat on the + half's border, at its outer edge." *Provisional:* the average of the border points strictly on + (for a row from x −2 to 2, x = 1.5). If the hand check meant the outermost edge itself, this needs a correction. |
| ACS-B6 | Linear Falloff, Symmetry X, centre on the plane or on − | ACS-13 (hand check with the handle on +) and FAL-B2-04 (recorded with the centre on the plane). *Built:* every point reads its weight at its + side position, so both sides fade as mirror images whichever side is primary. |
| ACS-B7 | Automatic or None, click away from the model | The table says the click gives X and Z on the work plane and keeps the selection centre's height. *Built* for the ground work plane in every view; other work planes keep the earlier rule (the ray at the current centre's depth). Screen's ~1 mm unsnapped difference isn't modelled. |
- **PF-Q08 / PF-Q09 (BEHAVIOR-PARITY-FIXES), 2026-11-19: answered
  2026-11-20.** PF-08: Newell normal through the bounding-box centre
  (built). PF-09: chord-weighted Hermite tangents with mirrored open ends
  (built).



- **Q-CE-05 (chamfer-edit-2026-11-05 CE12), 2026-11-20.** Start: the
  2-segment round chamfer (polygons 1, 2 selected), Source Segments 2,
  Segments 4. The reference's six new points sit off the box's end caps
  (z 0.966 / 0.034 instead of 1 / 0) and off the arc. *Operandi:* the two
  strips merge into one between the outer polygons (rule 6) and are
  rebuilt on the decided arc (Q-CE-01) with every point on its cap;
  counts and polygons match the reference, positions within 0.04 in x, y.
  Unclear: whether the reference's z drift is intended. CE16/CE17 stay
  refused (Q-CE-03 still open).

  Answered 2026-11-20: keep Operandi's behaviour. Q-CE-03 answered the
  same day and built (pf-bridge-2): CE16/CE17 slice the run.
- **PF-Q08 / PF-Q09 (BEHAVIOR-PARITY-FIXES), 2026-11-19: answered
  2026-11-20.** PF-08: Newell normal through the bounding-box centre
  (built). PF-09: chord-weighted Hermite tangents with mirrored open ends
  (built).


### Service holdouts (2026-11-20, branch service-holdouts)

| ID | Starting state and actions | What's unclear / what Operandi does |
|---|---|---|
| ACS-Q-HANDLE1 | Edge or vertex tool, several edges or points selected, picked in different orders; Symmetry on or off | **Answered (spec side, 2026-11-21):** Edge Bevel, Edge Extrude, Edge Chamfer and Vertex Extrude put the handle at the selection's bounding-box centre (pick order doesn't matter; under Symmetry, the primary side's box), arms starting about 25 px out. Edge Slide: one arrow across the first-picked edge at its midpoint; Vertex Slide: arrows along each edge leaving the first-picked point (pick order matters). Edge Relax and Vertex Bevel draw no handle (drag anywhere). *Revisited 2026-11-23, approved 2026-11-26:* Vertex Bevel shows one Inset handle on the handle point along the selection's normal, drawn as a blue scale handle (a square end) rather than an arrow (the maintainer), so every Bevel shows a gizmo when picked; its drag off the arrow is unchanged. *Built:* all of it except Vertex Slide still shows one rail arrow (from the first-picked point), not one per edge. Other vertex tools (Merge, Tear) keep the point nearest the box centre. |
| SNH-Q01 | Pen or Pen Slice, Snapping on with World Axis / Straight Line / Right Angle ticked | **Answered (spec side, 2026-11-21):** a dragged point locks when it is within Inner Range screen pixels of a lock line through the previous point, whatever the segment length; Snap Angle has no effect; locks act only while a point is dragged (press-drag to place, or moving one), never on a plain click; the point is projected onto the line; a point target in range still wins (SN2-37). *Built* (`snap_kinds::lock_segment`). |
| SNH-Q02 | Solid Sketch, Symmetry on, a new node near the plane and near a snap target | **Answered (spec side, 2026-11-21):** the seam pulls within about 8 px (none with Symmetry off), snap targets pull within 8 px on node creation, and when both are in range the nearer on screen wins, the seam on a tie. *Built.* Moving a node still snaps with Inner Range and has no seam pull. |

### MaterialX graphs, slice 1 (2026-11-20, branch claude/project-thread-93alht)

Built from the public MaterialX and OpenPBR specifications and Cycles'
node definitions (no behaviour spec; the plan is Jason's A31 approval).

| ID | Starting state and actions | What's unclear / what Operandi does |
|---|---|---|
| MXG-Q01 | Import a `.mtlx` with `place2d` rotate 30 | Which way the picture turns. *Provisional:* MaterialX's rotate2d turns the coordinates clockwise, so Cycles' Mapping gets −30°. Needs one render compared with MaterialX's own viewer. |
| MXG-Q02 | `tiledimage` with uvoffset (0.25, 0) | *Provisional:* coordinates × uvtiling − uvoffset (picture moves right). Same check as Q01. |
| MXG-Q03 | `noise2d`/`noise3d`/`fractal3d`/`cellnoise` in a graph | Cycles has no MaterialX noise. *Provisional:* Cycles Noise (or White Noise of the floored cell) remapped to MaterialX's −1..1 × amplitude + pivot; a note says so. Same look, different pattern. |
| MXG-Q04 | `UsdUVTexture` with sourceColorSpace auto | *Provisional:* sRGB for the rgb output of 8-bit pictures, data for single channels and for EXR/HDR linear. |
| MXG-Q05 | Graph with OpenPBR transmission_color, coat_darkening, transmission depth/scatter | Cycles' Principled BSDF has no slots. *Provisional:* transmission colour mixed into Base Color by the weight; the others left out with a note. |
| MXG-Q06 | Graph material in a scene saved with project-relative paths | The graph's pictures stay relative to the folder it was imported from (absolute), not rewritten to `project:` paths or copied into the project. Stage 2. |
| MXG-Q07 | Advanced viewport, a graph layer with a node the viewport doesn't draw (Camera, Window or Reflection coordinates; Geometry ▸ Incoming; an object-space normal map) | *Provisional (stage 5):* that input uses the socket's own value (or Cycles' default), the rest of the graph draws. Renders are unaffected. (Voronoi draws since 2026-11-21.) |
| MXG-Q08 | Advanced viewport, Fresnel or Layer Weight in a graph | *Provisional:* the angle to the eye per pixel, as Incidence gradients use; the node's own Normal input and back faces (Cycles swaps the index there) are left out. |
| MXG-Q09 | Advanced viewport, a Bump node on a Principled Normal | *Provisional:* drawn as the tree's bump with height × Strength × Distance; Cycles blends the bent normal by Strength instead, so strong, short bumps look softer in the viewport. Displacement from a graph isn't drawn. |
| MXG-Q10 | Advanced viewport, a Mix Shader of a Principled and an Emission (or Glass, Transparent) | *Provisional:* each channel (base colour, emission, transmission...) is mixed by the factor, as one surface; Cycles mixes the two lights. Close for most mixes; differs where one side is see-through. |
| MXG-Q11 | Shader tree Cellular or Poisson Cells layer, viewport vs render | Resolved 2026-11-21: the core's cells (`procedural::cells_with`) and the viewport's (`pattern.wgsl`) are Cycles' Voronoi F1/F2 (`cycles_voronoi`), and per-cell variation uses the Voronoi cell colour, as renders do. Poisson Cells' low randomness offsets each point from its cell's corner (Cycles' Randomness), not round the centre as the old reference did. |
| MXG-Q11 | Node editor (A3): Material Group > Show as Nodes, then the user wants to change something in the window | No spec covers editing (Nodes N1 puts add/wire/delete in N2). *Provisional (slice 1):* the window is read-only for every graph; edits go through the Shader Tree, and the window follows them (UIC-244). Needs a spec for: which edits the node view allows on a tree material (values only? adding nodes, which become which layers?), how a node with no layer form is kept, undo granularity per drag/wire. |
| MXG-Q12 | Open a scene whose saved graph differs from what its layers now give (an older build's sums, or an outside edit) | *Provisional (stage 4):* renders and exports use the saved graph for those polygons until their layers are edited; the viewport draws the layers, and nothing in the Shader Tree marks the material. Needs: should the tree mark a pinned material, should the viewport draw the graph, and should opening show a note. |
| MXG-Q13 | A texture placed by a locator on several items (each item sees the locator differently) | *Provisional (stage 4):* the saved graph holds placement as reference numbers filled in per item, so items share one graph; a surface whose sums run through a placement number (none in the tests) is saved per item. Cleaner later: a placement node naming the locator, if outside readers ever need the `.opscene` graphs directly. |

## Clean-room leftovers — 2026-11-20 (Operandi's own rules, pending research)

- **Q-CE-06 Chamfer Edit, borders at different distances from the corner.**
  The clean spec only probed equal distances. Operandi builds the decided
  arc (Q-CE-01) for distances of 1 and stretches it along each face to that
  border's distance, so it still meets both faces tangentially. Research
  question: a 90° chamfer with insets 0.2 and 0.4, Segments 3, Depth 100 %,
  0 % and −100 %; wanted: the new points.
- **Q-SS-03 Sharp with split smoothing groups.** Operandi: a face split off
  by the smoothing angle moves exactly Offset (SS11b moved 0.3; not copied).
- **Q-SS-05 Both Sides on a closed corner.** Operandi moves original points
  back along the selected faces' own direction (SS11d not copied).
- **Q-SS-10 Selection after, nothing selected.** Operandi: the shifted
  polygons weren't selected, so they stay unselected; the bridge quads are
  selected. Matches SS10 and TH12.

## Node editing — 2026-11-20 (NE-01..NE-19, provisional readings)

- **NE-Q01 Which layer inputs take wires.** *Closed 2026-11-21 (Jason:
  texture layers too).* A Material layer's colour and amount channels
  (node_view.rs `CHANNELS`), a texture layer's plain settings (a
  Constant's Color or Value, a procedural's Color/Value 1 and 2;
  node_view_settings.rs) and free nodes' inputs. Mixing nodes' opacity and
  blend take no wires (NE-05). Since branch node-wires-more-layers also:
  an Image Map's Gamma, Brightness, Contrast and (scalar effects) Low and
  High Value; a Gradient's Input Parameter (the number its curves look
  up); a weight Vertex Map Texture's and an Occlusion's zone colours or
  values; and any of these on a layer used as a mask. See NE-Q07.
- **NE-Q02 Compatible types (NE-08).** Built: same type, a float into any
  number, colour3 and vector3 into each other. Shader outputs drive nothing.
- **NE-Q03 Free nodes in material presets.** A material preset saves the
  layers only; the group's free nodes stay in the scene (copy/paste
  between materials is "later" in the spec).
- **NE-Q04 Which channels a Material box lists.** Built: Diffuse Color and
  Amount, Roughness, Metallic, Specular Amount, Luminous Color, plus any
  channel that differs from a new material or is driven.
- **NE-Q05 "Inline" values (NE-01).** *Closed 2026-11-21 (Jason: edit on
  the boxes).* Values are fields on the boxes: drag or click and type; a
  colour is its swatch; a mixing node shows only Opacity and Blend
  (NE-05). A drag is one undo step on release (UIC-224/226). Zoomed out
  below 60 % the values show without fields. The side panel stays for the
  selected box.
- **NE-Q06 A float free node into a colour input** writes `nodename` with
  the colour type in exported MaterialX (no `convert` node yet).
- **NE-Q07 Texture inputs that take no wires** (provisional, branch
  node-wires-more-layers). An Image Map's picture (a file, not a value)
  and its switches (swizzle, alpha, invert, clamp, filtering); a
  Gradient's curve keys (a list) and its Input choice; a colour Vertex
  Map's colours (they are the map); Occlusion's rays, distance and angles
  (Cycles' AO node reads them as fixed settings); zone alphas, bias, gain;
  Variation and Process layers. A wired Image Map Gamma below 0 renders
  as 1 / gamma regardless (the own value skips gamma ≤ 0). Which of these
  should take wires?

## Duplicate panel, part A — 2026-11-20 (duplicate-panel-2026-11-20 DX-xx)

- **DXQ-1 Transform order.** *Provisional:* items go to spots in item-list
  order, top to bottom; the first item is the placer's source, so it keeps
  the first spot (its own place for Array, Radial Array and Scatter, the
  path's first place for paths). Hands-on check pending.
- **DX-23 reading.** A spot's turn and size go on top of the item's own,
  about the item's position and in its own axes, so a non-uniform size never
  makes shear. Curve paths' Size (a copier option) doesn't apply: items keep
  their size.
- **DX-25.** A locked item stays put and doesn't use up a spot; the panel
  says how many stayed.
- **DXQ-4 answered (build side).** Duplicate Item copies the first-selected
  item alone; children are not copied. So Duplicate stays as it is and
  Duplicate Hierarchy (new) copies each selected item's whole branch.
- **Handles.** The Array / Radial Array handles still centre on the
  selection's bounding box, not the first item's position; values are right,
  the handle can sit off the items. *Provisional.*

## Instance duplicates and Symmetrize (BEHAVIOR-DUPLICATE-PANEL part B, 2026-11-20, branch dup-instance-symmetrize)

- **DX-Q1 Parent Off.** *Provisional:* copies stay at the source's level
  (top level for a top-level source), which is what Item mode did before
  DX-32. The spec says "top level"; they differ only for a source that
  has a parent.
- **DX-Q2 Hierarchy with Instance.** *Provisional:* mesh children of an
  instance copy are instances too; other children are plain copies.
- **DX-Q3 Bounding Box and the instance tint (DX-31, DX-35)** aren't built:
  items have no per-item box display, and the viewport's wire colour is
  per look, not per item. Left out (UIC-117) until a small display spec.
- **DXQ-2** checked in code, not by render: the viewport flips winding for
  a negative-determinant item and Cycles gets the item matrix as the
  object transform, so Instance Mirror uses a negative X scale (DX-34).
  GUI check 7 confirms it.
- **DXQ-3 answered (the maintainer, 2026-11-20: OK):** Symmetrize uses the
  whole mesh and clears a component selection (DX-43).
- **DX-Q4 Symmetrize plane** is in the item's own space (like Symmetry's
  Symmetrize), not world space.

## Topology Pen v2 (TP-xx) 2026-11-20

Built from BEHAVIOR-TOPOLOGY-PEN (2026-11-20) on branch topology-pen. These are the provisional readings, marked in the code (`topology_pen.rs`, `topology_pen_ui.rs`):

- **TPQ-1 Modifier gestures in other modes.** The spec lists them for Move mode. Operandi lets them work in every Mode; the Mode only changes what a plain left-drag does.
- **TPQ-2 Duplicate on a border vertex.** Makes a triangle with the border neighbour whose edge points nearest the drag direction. The new point lands on the background.
- **TPQ-3 Remove on a border vertex.** Refused ("border points can't be removed"). An interior vertex is dissolved, so its polygons join into one. A lone point goes with its one-point polygon.
- **TPQ-4 Which loop.** Remove, Smoothing and Shift+right Duplicate use the Select Loop rule (stops at a triangle, n-gon, pole or border). A Move of a loop uses Tweak's Edge Loops rule (TWK-16), which stops at the same places.
- **TPQ-5 Fill.** "Average edge length near the click" is the average of the edges at the 8 nearest points. With no edges there (only one-point polygons), the nearest points' average spacing is used instead. "Non-overlapping" is checked in the plane facing the viewer: no side crosses an existing edge, no existing point is inside, no side already has two polygons, and the click isn't on a polygon. A Fill polygon that touches existing polygons takes their winding (Make Polygon's rule); otherwise it faces the viewer.
- **TPQ-6 Smoothing.** One pass on the press, then another every 4 px of pointer travel. The radius is centred where the pointer meets the mesh. Border points relax along the border. A corner (Lock Corner) is a border point used by only one polygon. Re-conform puts each moved point on the nearest background point.
- **TPQ-7 Smart snapping and the fields.** The dragged point nearest another point within the pick radius jumps onto it; one weld per drag. Right-drag in empty space sets Radius with Tweak's range sphere, not a circle. The Add Loop Position field shows where the last loop went; typing it doesn't move that loop. Typing an Offset moves the last moved element by the change, as one undo step.
- **Not built (left out of the panel, UIC-117 exception):** Backface, Raycast Inactive Surfaces, Slip UVs. **Symmetry:** Move mirrors as Tweak does. The other modes don't apply to the mirrored element yet.
| SNH-Q03 | Pen or Pen Slice, Straight Line ticked, dragging the third or later point | Is the Straight Line lock the continuation of the previous segment through the previous point? *Provisional:* yes (the line through the previous point along the previous segment). |
| ACS-Q-HANDLE2 | Edge Weight, several edges selected in different orders | Where does Edge Weight's handle sit? Unchecked by the spec side. *Provisional:* unchanged: the selected edge nearest the selection's box centre. |

## Node View key and menu — 2026-11-20 (branch node-view-open, provisional)

- NV-Q01: Window ▸ Node View (also under Layout ▸ Windows/Palettes) and Cmd+Shift+N open the node view of the selected Material Group, or the group the selected layer sits in; with nothing suitable selected, the group already shown, else the first Material Group; with none, a status message. Key and menu open the same view, though UIC-184 has keys open popovers (the node view has no popover form). Confirm the key and the fallback.

## Missing tools group 2 (G2-PLANE-TEAPOT v1) — 2026-11-21 (branch missing-tools-2, provisional)

- MT6-08: Operandi has no primitive list in its Geometry menu; Teapot, Teapot Item and Plane Item went into Item ▸ Primitive (where the other primitives are). Command names "Teapot", "Teapot Item", "Plane Item" are Operandi's own (no tool id in the name map).
- MT6-04: 6 × 6 quads per patch; the lid and the body stay separate pieces of the one mesh (the 1987 data doesn't join them), as do the handle and spout ends (they run into the body). Faces are turned outward per piece (the polygons round each piece's highest point face up).
- MT6 tool: a drag in the viewport moves the teapot's box centre, as Text does; there's no drawn size.
- Data: the site publishes the control points only inside its viewer; they were recovered exactly from the viewer's own resolution-3 patch tessellation (4 × 4 samples solve back to the 16 control points; all reproduce to 2e-5 and match the classic values).
## Missing tools group 1 (G1-SMALL-COMMANDS v1) — 2026-11-21 (branch missing-tools-1, provisional)

- Q-FB-1: Fix Black Dots keeps the pinch points, unused (as the reference). Q-FB-2: a bent pair (FB-C) still merges into one non-flat polygon. One pass: a merge that leaves a new pinch (a cube corner) isn't merged again in the same run.
- Q-EZ-1/Q-EZ-2: Edge Zip refuses open-border edges and edges beside a non-quad ("Edge Zip needs edges with a quad on each side") instead of welding.
- Q-DR-1: Diagonal Re-Direct refuses (the whole command, "Select two diagonal polygons.") when a side polygon isn't a quad, C isn't surrounded by exactly four polygons, or a selected quad has no diagonal partner. With several pairs (DR-F), C is selected for each pair whose side quads were both rebuilt. M's UVs blend the side quad's corners equally.
- Names: no reference names known for these three; Operandi's own `operandi.mesh.fixBlackDots`, `operandi.edge.zip`, `operandi.poly.diagonalRedirect` until the name map has them. All three run on the primary mesh only (not each active mesh).

## Missing tools group 3 (G3-TOPOLOGY-TAB v1) — 2026-11-21 (branch missing-tools-3)

- Q-MI-1 decided (Jason, 2026-11-21): **flip.** Merge Instance turns the added polygons round when the instance's placement mirrors (negative determinant), so both halves face the same way (`FLIP_MIRRORED` in merge_instance.rs). Q-MI-2 (provisional): two or more instances are refused.
- MT7 (provisional): Topology Mode draws the active meshes' faces with the Transparency Override's Overlay fill (Ghost: front and back, flat, Topo Face at the Foreground Opacity), and their plain wire and points without the depth test. An edge whose polygons all face away draws in Topo Back Edge. Selected-element highlights and Sub-D cage wire are still depth-tested. Hide back-facing points is a CPU test per point (the polygons round it), rebuilt as the view turns. The view-menu entry is in the viewport's display menu; the spec's 0–1 RGB defaults are stored as 0–255 (51,51,51 / 28,64,105 / 97,158,235).
- MT8 (provisional): Topology Offset is a preference and a Topology tab field, but because the foreground already draws over everything it has no visible effect yet.

## Missing tools group 5: Contour (G5-CONTOUR v1) — 2026-11-21 (branch missing-tools-4)

- Q-CT-1 decided (Jason, 2026-11-21): Lines, Curve and Face all take MT13-05's spacing and counts. Lines closes with Operandi's closed-polyline flag (a polyline's closure adds a span, never a repeated point), the same shape as repeating the first point.
- CT-1 (provisional): the cut is of the background's faceted surface (CT-7's 0.4868 end is exactly that), so evenly spaced points between the sphere's vertices sit on its flat polygons, up to 0.5 % inside radius 0.5, not within 0.0001 of it.
- MT13-06/07/13 as written (provisional). Open pieces run from the end nearer the line's end (CT-7, CT-8). With Cull Occluded Open Curves, the open piece furthest along the slice direction (toward the viewer) stays. A typed zero Slice Direction is refused ("draw it in a view or set another direction"). Select New Polygons selects the new polygons in the current mode. Angle Snap snaps the line's angle on screen.
- Icon: Contour has its rendered icon (icon studio, 2026-11-21).

## Missing tools group 6: Mesh Paint (G6-MESH-PAINT v1) — 2026-11-21 (branch missing-tools-5, provisional)

- Q-MP-1 readings as built: Strokes scatters, once the pointer has moved a brush radius, Brush Density ÷ 10 copies (at least one) at random in the brush circle, falloff 1 at the centre to 0 at the edge, heading along the stroke; Density effects keep a point with that chance. Surface Align turns the source's +Y onto the normal and its +Z along the stroke (Strokes, Linear). Random size: one factor within Size X's variation of 1 (Adaptive + Random times the brush size). Drop: the drag's length scales (1 + length/100 pt) and its angle turns about the normal. Slide with Ctrl first: the drag sizes the copy.
- Mesh Paint is a Create-tab utility (like Retopo Sketch), not a live tool: each click or stroke is one undo step (MP-16) and its random choices come from a seed stored with the step. Slide shows the brush circle, not a live copy, while dragging.
- Not built (provisional): the Target Mesh setting (the selected mesh is always the target), 3D Brush (stored only), tablet tilt (the tablet modes fall back to Surface Align / the view's up), snapping to Vertex / Polygon Center (MP-17's second half). Symmetry mirrors each copy across the target's symmetry plane.
- Icons: both buttons wear their rendered icons (the Alt variant its own). Mesh Paint is a utility, not a `Tool`, so it has no row in tool_speed_guard_tests (a stroke commits once, on release; drag frames only ray-cast the target).

- MT8 Topology Offset dropped (Jason, 2026-11-21): no effect while foreground draws on top.

## Missing tools group 7: Patch Curves (G7-PATCH-CURVES v1) — 2026-11-21 (branch patch-curves, provisional)
- PC-Q1 (provisional): with three curves, the corner opposite the first picked curve collapses (the second and third curves' shared end); the row of cells touching it is triangles and the corner is one shared point.
- PC-Q2 (provisional): Knot spacing = divisions evenly spaced in sample index of the frozen curve sampling (the same steps per span), so each span between control points gets an equal share. Read from the option name, not measured.
- PC-Q3 (Jason, 2026-11-21: build live patches; branch patch-curves-2): Freeze is off by default. Closing the loop leaves one `Patch` polygon on the corners (each corner's two curve ends are joined into one point). The evaluated scene shows it tessellated at the mesh's Patch Subdivisions (Subdivision section of the item properties, default 16), so the viewport, Cycles and exports read the cells; saves keep the patch (native version 57); Freeze makes the cells real. With Freeze on, the frozen grid is built as before. Either way the closing click is one undo step and an option change rebuilds the last patch while it's still the last step.
- PC-7 provisional readings: a live patch always spaces by Length and has no UVs of its own (cells blend the patch's corner UVs); Perpendicular, Parallel, Type 1/2 and Make UVs are greyed while Freeze is off. The first cell takes the patch polygon's number, the others follow the mesh's polygons; picking and component editing see the one patch polygon (as for Mesh Stack results), while the viewport draws the cells, so in Vertex mode the cells' points show too. Flip on a three-corner patch changes which corner collapses.
- PC-Q5 (provisional): a patch whose boundary curve was deleted (no open curve joins two neighbouring corners, by point or by position) is broken: it draws, renders and exports nothing, stays in the scene, and Freeze leaves it as it is.
- Kept from main: ends within 0.1 mm count as shared, so drawn curves (which have their own end points) still patch; the spec's "ends that only touch build nothing" is not applied. Each corner reuses the start point of the curve that begins there. Save Boundaries isn't an option: the curves always stay.

## Missing tools group 8: Replica family (G8-REPLICA-FAMILY v1) — 2026-11-21 (branch replicas, provisional)
- RP-Q1 (provisional): Snap Rotation rounds each random angle to whole 90° steps.
- RP-Q2 (provisional): Freeze Replicator puts one Mesh Instance per copy (Density's share) into the Replicator's group, named "proto (2)", "proto (3)", …; the Replicator and its point cloud (when it's in the same group) go; the group is selected.
- RP-Q3 (provisional): Point Source reads points only (Vertices); Surface mode isn't built.
- RP-Q4 (provisional): Replica Source builds Active Item only; Specific Item, All Background Meshes, Mesh Layer Preset and Random Presets are listed greyed. Parent Selected puts the group under the replicated item. The path tools (Curve/Bezier/B-Spline Replica, Pen Replicate) have no Replica section yet and always use Active Item / Scene.
- Storage: each point is a copy's pivot; its rotation (XYZ angles, radians) and scale (mirror = negative X) are vector maps "Replica Rotation" / "Replica Scale" on the cloud. The placement's own rotation and scale go into the point, the prototype's own rotation and scale don't; Use Source Transforms on multiplies them in. Random values are uniform in ±range per point and channel from the Seed (Blender's Instance on Points + Random Value), applied on top of the point's transform; Uniform Scale uses Random Scale X for all axes. Default Seed 401.
- The evaluated scene shows copies as Mesh Instances (ids out of the document's range); picking ignores them. Viewport: wire boxes by default; the view's Replicators option (None / Selected / All) shades them. Cycles builds one render mesh per prototype and places copies by matrix (the session then shares one Cycles mesh); USD export always writes a Replicator's copies as a PointInstancer (mirrored copies stay Xforms). Native version 58 (live patches took 57).
- Mesh Paint Replica: one Replicator per stroke; with several background meshes each dab's pick is kept per point ("Replica Member" map, Replicator `members`). It's the Ctrl variant of the Mesh Paint button and a "Create Replicator" switch in its panel; Parent doesn't apply.
- Icons: the Replica buttons have no icons of their own yet (the Instance buttons in those flyouts have none either; Mesh Paint Replica borrows mesh-paint-instance). For the icon studio: replica-clone, curve-replica, bezier-replica, bspline-replica, pen-replicate, replica-scatter, replica-array, replica-radial-array, replica-mirror, mesh-paint-replica.

## Q and Esc as general keys (SC-078, SC-080) — 2026-11-21 (branch claude/project-thread-s4394q)
- SCQ-ESC-1 (provisional): SC-080 says Esc "cancels the current operation; else drops the selection", the bible (UIC-24, UIC-66) says Esc drops the active tool. Built as one step per press: a field edit cancels (UIC-51), menus/drags/background constraint/falloff as before, then over a view the active tool drops keeping its edit (Create cancels its scratch points), then the selection of the selection mode drops (`select.drop`, input-map "Drop Selection", one undo step). Esc acts only over a 3D view (UIC-70). Unclear: whether "cancel the current operation" should undo an applied tool edit instead of keeping it.
- SCQ-ESC-2 (provisional): `select.drop` takes no arguments yet; the name map's `type` and `mask` values aren't given.

## Missing tools group 9: Vertex Normal Tools + Guides (G9-VERTEX-NORMALS-GUIDES v1) — 2026-11-21 (branch vertex-normals-guides, provisional)
- VN-Q1 (provisional): Recalculate weights each polygon by area × corner angle (either off with Area / Angle Weighting), modelled on Blender's Weighted Normal; Sum Co-Planar Areas groups polygons within Coplanar Tolerance and adds their weights. With Full Weighting off, a corner whose largest coplanar group has more than 3× the next group's area takes that direction alone (Face Influence); on, the plain weighted average. High Quality Normals has no effect. Area and Full Weighting are linked as the spec says.
- The drawn and rendered normal map is the mesh's first (`normal_map_order`); the viewport and Cycles now read it where it sets a corner, so the tools' results show. Recalculate makes the map "Vertex Normal" (or the custom name) and puts a new map first.
- Harden modes as built: Vertices (Connected Verts: both ends selected), Edges, Polygons (outer border), By Angle (selected edges, all when none), UV Auto/Edge/Polygon Boundaries from the current UV map's seams (edges whose two polygons give an end different UVs). Harden with nothing picked is refused. Soften's plain button softens every edge.
- VN-8 Transfer Normals reads the source's shading normals (its map, else its smoothing) at the nearest surface point and writes every corner of each target point; World Space (on by default) turns the source's own map with it. Vertex Map Transfer accepts Vertex Normal now.
- VN-10 Spin's Offset defaults to 1; corner data (UVs, normals, weights, colours) moves with its points. Morph to Normals points each corner along the morph offset at its point (points that don't move keep theirs). Rename takes the custom name from Options (or `name:` on the line).
- VN-0: the palette is its own window (`layout.window hash:vertexNormalTools`), from Geometry ▸ Vertex Normal Tools... and an Edge-tab button beside Split Normals. A Model-toolbar button didn't fit without breaking the mode bar's layout (mode bar test), so it's on the Edge tab instead; text-only buttons until the icon studio makes them. Options save to vertex-normal-preferences.json; changing one re-runs Recalculate when the active mesh has a map.
- VN-11 Cage tab: not built (lowest priority); the tab says so.
- GD-Q1 (provisional): the Guides tool is a utility (Edit ▸ Snapping ▸ Guides, or `Guides`), so it takes the viewport while on; guides draw and snap with any tool. A drag draws the current guide as Two Points (start/end on the work plane, or snapped to a point) and turns Enable and Show on; a locked guide refuses. Planar Ray: Angle from the plane's first axis to its second (YZ: +Y→+Z), Distance 1 m by default. Guides draw as thin light-blue lines. Snap to Guide makes the guide a snapping target for every role (nearest point along it on screen). Ruler Array guides aren't in Operandi yet.
- GD-3: guides save with the scene (native version 59; replicas took 58).
- Icons for the icon studio: vertex-normal-tools, recalculate-normals, harden-normals, soften-normals, guides.

## Soft Move's own falloff (TP-19, Mug tutorial) — 2026-11-21 (branch soft-move-falloff, provisional)
- TP-19 (from Jason's tutorial walk-through, behaviour only): a tool row's falloff layer can say `at_tool = true` (factory-tool-presets.toml). The falloff keeps its fitted size (TP-17) but its centre goes to the tool's centre when the tool starts and on the first click (CS-10), so the falloff and the transform handle begin at one spot and are then edited apart. Radial and Cylinder only; Soft Move is the only row that uses it. Bulge (Radial, also waits for a click) keeps its fit to the selection's centre until Jason says otherwise.
- TP-19 view (provisional): a Radial or Cylinder falloff's Size handles draw as filled boxes in their axis colour (the Scale handles' shape), on top of the tool's handles, and take the press where they overlap a tool handle. The falloff's Centre and its per-axis handles give way to the live tool's handle under the pointer (with no live tool they behave as before), since a tool that brings its own falloff puts both centres on one spot. Unclear: whether later clicks away from the handles (CS-20) should move the falloff with the tool; built: only the first click does.

## Animation phases 1-2 (BEHAVIOR-TIME-KEYS, BEHAVIOR-TIMELINE) — 2026-11-22 (branch animation-time-keys, provisional)
- TK-Q01 Smooth Flat rule: inner slope = 1.5 × the slope through both neighbours, zero at a peak or dip, capped at 3 × either side's slope, then Fritsch-Carlson scaled if both ends of a segment lean in. Matches every TK-13 value; near-equal neighbours unprobed.
- TK-Q02 Auto / Auto Flat end keys point straight at their one neighbour (Smooth Flat ends stay flat).
- TK-Q03 Time Snap also snaps typed and scripted times (`operandi.time.set time:0.51` lands on a frame); off, any time is kept.
- TK-Q04 Auto Key Animated: an edit that the curve already gives at the key time adds no key (a transform tool's Drop after its drags were keyed). Auto Key All keys the selected items' changed channels only.
- TK-Q05 Discard (TK-41) is an undo step; Apply keys at the key time.
- TK-Q06 Reset puts transforms back to 0/1 and other channels to their rest value (the value before the first key).
- TK-Q07 Keyable channels: transforms, and number / on-off / choice / colour-component channels whose setter is a light, camera, item-common or transform edit; camera focalLength, focusDistance, targetDistance and fStop channels added (names provisional). Shader-layer channels are not keyable yet.
- TK-Q08 The Wrap end behaviour plays to the end, then stops back at the frame playback started on.
- TK-Q09 Shortcuts: `/`, `,` `.`, Shift+R and Shift+Y already belong to Turn, Zoom In/Out, Quick Align and Flex globally, so they are animation keys only in the Animate layout's keys (UIC-201). Ctrl+arrows are macOS Mission Control keys by default and may never reach the app.
- TL-Q01 The timeline is a bottom strip opened by the bottom bar's clock button and by the Animate layout, not yet a dockable panel kind (UIC-190..195).
- TL-Q02 Timeline pan is Shift+middle-drag, zoom the wheel over the ruler, until the reference gestures are probed (TL-15).
- TL-Q03 Paste of keys copied from one item into a scene where other items are selected goes to the first selected item's same channels; otherwise back to the same item ids. Paste Insert pushes later keys by the pasted span plus one frame.
- TL-Q04 Remove Frame never deletes or merges keys: a channel whose next key would land on the one before it doesn't move.
- TL-Q05 Playblast writes a PNG sequence (no movie encoder yet) and opens its first frame; the HUD isn't hidden yet. Render Frame Range writes `<name>_0000.png` into the project's renders/ folder, else ~/Pictures/Operandi Renders.
- TL-Q06 USD: TL-83's Animation option is a `scene.saveAs animation:on|off|current` argument; the File > Export dialog has no control for it yet (always on). Lights export intensity and colour samples; cameras focal length, apertures, f-stop and focus distance.
- TL-Q07 The Animate layout uses the Model toolbox set; an animation toolbox set (UIC-197) waits for its icons.
- Command names are Operandi's own (`operandi.time.*`, `operandi.key.*`, `operandi.marker.*`, `operandi.channel.select`, `operandi.anim.*`, `operandi.render.frameRange`, `operandi.view.playblast`): the name map has none (UIC-165).

## Forum gizmo report (2026-11-22)

- FG-Q01 Plane circles under a non-World action axis: the arrows follow the action axis but the circles stay flat in world planes (PH-21, provisional), placed at the arrows' corners, so they look skewed or edge-on away from the gizmo (forum screenshot). Options: (a) circles lie in their two arrows' plane and drag along those arrows; (b) world circles at the world-axis corners; (c) hide the circles unless the axis is World.
- FG-Q02 Selection and Local axes on a ring around a hole (a tunnel loop): the polygons' normals nearly cancel, and the small leftover picks the in-plane axes, so with uneven segments the gizmo spins around the tunnel direction by an arbitrary angle (an arch ring: 125°). Should a vote whose normals mostly cancel count as "no common normal" (Local: axes from the ring's shape; Selection: its current "indeterminate" message), and below what share?

## Graph editor, phase 3 (BEHAVIOR-GRAPH-EDITOR) — 2026-11-22 (branch animation-held, provisional)
- GE-Q01 Weighted handles: a weight scales the handle to `weight × a third of the segment` in time; weight 1 is exactly the plain (Hermite) curve. Weights run 0.01-3.
- GE-Q02 Slopes in commands and the query are per frame (`operandi.key.handle slope:`), as the graph shows them; curves store them per second.
- GE-Q03 Constant Smooth, Oscillate Smooth and Offset Repeat Smooth behave as their plain versions until the spec says how they differ. Linear extends along the end key's slope.
- GE-Q04 Cycle (GE-62) forces the last key to the first key's value and gives it the first key's out slope as its in slope.
- GE-Q05 Key Keeping Shape sets the new key and its two neighbours to Manual slopes (today's values), which keeps a cubic curve exactly.
- GE-Q06 Set Frame (absolute) moves the earliest selected key to the frame and keeps the others' spacing; ripple and per-channel linking aren't built.
- GE-Q07 Shown curves: the selected channels; with none selected, the listed items' animated channels. Normalised and Stacked map each curve's key range; Isolate shows the first selected key's curve.
- GE-Q08 Built 2026-11-22: Box Scale (GE-44), Ctrl+Shift+drag copies (GE-31), Ctrl+Alt+right-drag box zoom (GE-20), Speed view (GE-81), envelope presets (GE-83), Ctrl/Shift+wheel rows in Stacked (GE-82), clamped/overshoot hint (GE-X4), retime strip (GE-X5), In/Out slope pop-ups (GE-53), Animation Editors (GE-01).
- GE-Q09 Provisional: Box Scale Ctrl+corner drag scales time and value together about the opposite corner (no warp). Envelope presets are `.openvelope` files in Presets/Envelopes, listed in the graph's Presets menu; not yet shown in the Preset Browser and no drag-drop onto the graph. Overshoot hint is a ring on keys whose curve passes beyond both neighbours. Retime marks live in the editor only (not saved).

## Rig phase 4 (2026-11-23, branch graph-editor-resize)

- RG-Q01 Rig pieces are scene data (`Animation::rig`, native v62), not items; their channels key like any (`<piece id>:influence`). Settled 2026-11-23: they show in the Items list as rows under their item (rig-items-list), and in the Schematic.
- RG-Q02 The evaluated scene places each rigged item at its evaluated world as a root item (parent links are only in the document), so every reader (viewport, renders, export, picking) sees the rig; Item-mode handles sit at the evaluated place, and a transform tool on a driven channel is refused with "Driven by <piece>". Provisional.
- RG-Q03 RG-04 [OP]: a driven field greys with "Driven by <piece>" and shows the evaluated value (the rig's result under the item's parent).
- RG-Q04 RG-31 Affect X/Y/Z on rotations works on world XYZ Euler angles; Influence blends position and scale linearly and rotation by slerp. Provisional.
- RG-Q05 Direction with Up parallel to the aim falls back to world +X (+Z when the aim lies along X); measured RG-44 cases hit exactly. Axis X aims X with Y towards up; Axis Y aims Y with Z towards up. Provisional.
- RG-Q06 Path: Percent and Offset are fractions (1 = 100%); Wrap takes Percent modulo 100% (an exact lap lands on the end); smooth curves are sampled 24 steps per span for arc length. Provisional.
- RG-Q07 Ctrl/Shift+click forms (RG-26/27) are menu entries for now ("… (multiple)", Constrain > Remove); the Rig toolbox tab with live Ctrl/Shift labels comes with the rest of the Rig UI. Selection pick order: the other selected items in order, the active item last.
- RG-Q08 The "0 targets weigh nothing" and "Use Pivots" cases use the target's pivot point in its own space (`pivot.position`).
- RG-Q09 RG-50/52/53: the Channel Links window holds the relationship keys (driver value, driven value, Key, Delete Key) and the User Channels list and Add row; the in-viewport panel, prev/next arrows, the Graph button, the item properties' User Channels tab and the label drag of RG-54 (the command `operandi.rig.bindUserChannel` does it) come with the Rig UI pass. Provisional.
- RG-Q10 RG-52 relationship curves key the driver's value as the curve's time (seconds); smooth-flat and hold-outside as TK-12/13.
- RG-Q11 RG-60 Expression: inputs are named wires (`in.<name>`), `time` (seconds) and `frame` built in; an expression with a syntax error is refused when typed and, if loaded, outputs 0 with its error in `operandi.rig.query`. The error's red field and position marker come with the Rig UI pass.
- RG-Q12 RG-61/62: Blend, Min, Max and Index are piece channels, overridden by a wired input of that name; Switch picks the inputs in name order.
- RG-Q13 RG-63: every item has read-only `world.pos/rot/scl.X..Z` and `local.*` channels for links and modifiers to read; worlds are the document's (before the rig), so reading them never loops. Matrix-typed compose/decompose nodes aren't built (Operandi has no matrix channels yet).
- RG-Q14 §8 Bake keys pos/rot/scl on every frame of the range (angles kept continuous) and disables every piece on the baked items; driven non-transform channels aren't baked.
- RG-Q15 RG-70..72: the Schematic is Operandi's own small node view (the shader node view is built on the shader tree's board and doesn't fit items): automatic columns (drivers, pieces, driven), nodes dragged by hand aren't saved, items appear when a piece uses them or with Add Selected Item. Constraints are wired from Animate > Constrain; the Schematic wires links and modifiers and deletes wires and pieces.
- GE-Q08 Not built yet: Box Scale tool (GE-44), Ctrl+Shift+drag copies (GE-31), Ctrl+Alt+right box zoom, Link related keys (GE-42), Speed view (GE-81), envelope presets (GE-83), stacked-row wheel stepping, clamped hint (GE-X4), retime tool (GE-X5), the in/out slope pop-ups (the menu and presets cover them).

## Statistics panel (BEHAVIOR-STATISTICS, development/specs/statistics-panel) — 2026-11-23 (branch statistics-panel, provisional)
- STAT-Q01 Drawn (GPU) Active/Background show triangles, the GL readout's own unit (the spec says polygons; reusing the readout was asked for).
- STAT-Q02 "Flat" edges and "Flat with a neighbour": the two polygons' Newell normals within 0.01° (they share the edge, so that puts them in one plane). Not flat uses the Flatten tools' rule (1e-12 of the extent, Q5); no preference added, since Flatten has a rule, not a setting.
- STAT-Q03 Filter row dropped (Jason, 2026-11-23: Modo-style panel); acceptance check 7 not built.
- STAT-Q04 Open groups are saved per user (statistics-panel.json in the data folder), not per layout.
- STAT-Q05 Polygon and edge counts: an edge's polygon count is its distinct surface polygons (lines don't count; a two-point polygon counts once); one-point polygons have no edge. Vertex polygon counts include lines.
- STAT-Q06 With no selected mesh in a component mode every shown mesh is counted, and + makes the meshes it selects on active. Items count every item; hidden items too.
- STAT-Q07 Rename and Delete of parts and materials reach every mesh and the Shader Tree's group masks; sets, maps and smoothing groups only the counted meshes. Renaming onto a name in use is refused; "Default" can't be renamed or removed. Clear works on weight, UV, morph and colour maps; Normal maps refuse.

## Python scripting preview (2026-11-22, branch python-scripting)

- PY-Q01 A whole script is one undo step (UIC-227), named "Script <file>" / "Console: <line>"; `undo_group` names a step inside it. Provisional: the script's lines show in the History tab as their own lines (source script), not yet as sub-commands of the script's line.
- PY-Q02 `operandi.run` reports what the line's Command History record says (`ok` and its message). Commands that report a failure only on the status line (not `last_failure`) read as success; the hardening branch's undo-reporting work covers the same gap.
- PY-Q03 In the running app, `python_run` (MCP) runs on the bridge's headless path (headless commands, all-or-nothing like other bridge calls); the console and Run Script… use the app's full typed-line path.
- PY-Q04 The Script Console is a floating window (UIC-183) from System > Script Console; no shortcut yet. Console lines are one statement (echoes its value) or a block.

## Rig phase 5 (2026-11-23, branch graph-editor-resize)

- SK-Q01 Setup mode is a view state of the scene (`Animation::setup`, never saved); undo and redo keep it as it is, like the current time. Setup values are each animated channel's `rest`; unanimated channels' own values are their setup values. Constraints still evaluate in setup mode; the skin shows the rest shape (SK-51).
- SK-Q02 SK-03 Restore Setup throws away the channel's keys (its edit values) and puts the setup value back; Item to Setup copies the value shown now into the setup value. Provisional.
- SK-Q03 SK-15 numbering: the number counts per name and side (Joint1, Joint1_L, Joint1_R); a mirrored pair shares its number. A joint on the mirror plane gets no suffix and no mirror. Provisional.
- SK-Q04 SK-16 the Axis (roll) is chosen as near as it can be to perpendicular to world up (Z up when the bone is vertical); a leaf keeps its orientation. Provisional.
- SK-Q05 SK-10 a drag after the click moves the joint in the plane through it facing the view; SK-18 shape: three rings a tenth of the bone long (a fixed screen size for leaves) and a four-sided bone. Provisional.
- SK-Q06 The skin's MeshOp id is Operandi's own (`operandi.skin.item`); the name map has none. Provisional.
- SK-Q07 Bind pose = the joints' setup pose, read from their channels' setup values (bound joints get setup channels with no keys); constraints and targets don't count in it; the mesh item's own placement is today's, so moving the mesh item alone doesn't deform it. World off is stored but acts as on. Provisional.
- SK-Q08 Sharpness raises each weight (relative to the point's strongest) to the power 2^Sharpness before normalising; a Smooth influence curves its weight with 3w²−2w³. Provisional.
- SK-Q09 Bind: Distance weights are (nearest distance / this bone's distance)^k with k 2/4/8 for Low/Medium/High; ties go to the child bone; a point seeing no bone in Visible takes its nearest; Heat's joints-inside test counts surface crossings along one ray; map names are the joints' names; the skin goes to the bottom of the stack. Provisional.
- SK-Q10 Weight tools: Mirror copies the + side to the − side by default, matching within 0.0001; Split gives on-plane points half to each side and removes the original; Merge adds capped at 100% and removes the source; Transfer takes the nearest source point's weights (not the nearest surface point); Adjust Weights is a command (additive or set) until its drag tool lands. Provisional.
- SK-Q11 UsdSkel: joints aren't written as Xforms (the Skeleton stands for them); the stage's /root becomes the SkelRoot; on import the mesh's own placement stands for geomBindTransform, joints' setup pose is the bind transforms, and every joint channel that changes across the SkelAnimation samples gets a key at each sample. Provisional.
- IK-Q01 Apply IK outside Setup is refused ("Apply IK needs Setup mode…"), like Bind (SK-02); the spec says to apply in Setup but not that it's refused. Provisional.
- IK-Q02 IK-02 hierarchy: `<root>_IKRoot` takes the chain root's place under its parent and holds the root joint and the goal. Provisional (spec [probe]).
- IK-Q03 IK-07 Soft is a fraction of the chain's reach (0.1 = the last 10% eases in, exponential); IK-08 stretch ratio 1 + Scale Factor·(distance/reach − 1), shared by Base and Mid amounts; End amount is stored but a two-bone chain has no third bone. Provisional.
- IK-Q04 IK-06 the created pole sits one first-bone length out from the middle joint on the bend side, under the IK root group. Provisional (spec [probe]).
- IK-Q05 FABRIK: a straight chain first leans its inner joints 0.1% of the chain length towards the pole (else it can't pick a side); with Root Locked off only the backward pass runs; Twist Spread shares the goal's turn about the end bone (against its rotation at Apply) evenly over the bones. Provisional.
- IK-Q06 Limits are on the joint's local rotation channels (its rotation order, relative to its parent), so a two-bone knee's limit adds to its rest bend; the soft wall is the last 10%×strength of the range (two-bone and chain use strength 1 until full-body's Limits Strength); Limit FK clamps after each recorded edit; the fan is drawn about the parent's axes. Provisional.
- IK-Q07 Switch: when Blend has no keys yet, its old value is keyed one frame before the switch too, so earlier frames keep their pose; to FK keys every chain joint's position and rotation; Select Switch Channels selects the items (no channel sets yet); Disable IK needs Setup mode. Provisional.
- IK-Q08 Full-body IK: the solver is a rig piece on the root (no separate solver item); goals made by Add Goal are top-level items; pins (tasks with no goal) take priority and goals act in what the pins leave free, so a far goal can't drag a planted foot; each step's error is clamped to 10% of the hierarchy's size and the whole step scaled so no joint turns more than 0.2 rad (or moves 5%) at once; Tolerance is in millimetres (×0.001 m, and ×0.001 rad for orientations); Precision sets the damping (λ = 0.1/Precision). Provisional.
- IK-Q09 Pose tool: other IK solvers are left out of the drag's solve (the hierarchy moves as plain FK, the answer written into its channels); "screen space" drags in the plane through the joint facing the view, Ctrl+drag on the work plane; Shift twist is 0.5° per pixel; each frame of a drag re-solves from the press. Provisional.

## Character rig, ACS milestone 1 (2026-11-23, branch claude/project-thread-cezhnf)

BEHAVIOR-ACS-FEATURES v1 (ACS-xx there; ACSF here so they don't clash with the action centre's ACS-Qn). Piece internals are the builder's design until spec v2 (Jason, 2026-11-23).

- ACSF-Q01 ACS-01 what each context shows: Guide shows the rig's guides and hides its joints; every other context shows the joints and hides the guides. Animate shows the bind joints too until controllers exist. Meshes are never hidden by a context. Provisional.
- ACSF-Q02 ACS-02 Setup mode: every context but Animate turns Setup mode on (guides and the bind skeleton are setup state); guide, option and build commands run in Setup mode and then put it back as the context wants. Provisional.
- ACSF-Q03 ACS-31 guide symmetry: after any edit that moved guides, the side whose guides moved leads (the left when both did) and the other side's guides become its mirror across the rig root's YZ plane; centre guides are put on that plane. Live drags follow in the preview too. Provisional.
- ACSF-Q04 ACS-22 module options (vertebrae, neck joints, twist joints) are one set for the rig, shared by both sides of a mirrored pair. Lowering a count that would delete bound joints is refused ("unbind it…"). Provisional.
- ACSF-Q05 ACS-23/25 serial pieces: vertebrae and twist joints sit evenly in line along the line between two guides, as joints of the chain (so they take weights), not yet twist-spreading drivers; that waits for controllers. Provisional.
- ACSF-Q06 ACS-34 Fit Guides to Mesh: the rig root goes to the mesh's floor at its bounds centre, Reference Size becomes its height, guides go to the Biped template, then each is centred inside the mesh by rays: spine and head front to back, arm joints up-down and front to back, leg joints across (from their own side) and front to back, the head top, fingertip and toe tip to the mesh's surface. Assumes a T pose facing +Z with the character's left on +X. Provisional.

- RG-Q90 Items list rig rows (rig-items-list): a piece row selects its item and stays marked; renaming stores the piece's own name (`operandi.rig.rename`, empty = the kind's label); the eye column is the piece's Enable; IK pieces delete with Remove IK. A skin row (SK-36) opens to its joint influences; its menu has Enable, Bake Weights, Duplicate and Freeze, Delete (Unbind, Setup only); Rename, Duplicate and plain Freeze (which must ask first) aren't there yet. Provisional.
- RG-Q91 Flaky under load: `rig_phase4::rg06_only_items_downstream_of_a_change_are_evaluated` asserts a debug-build frame time (200 constrained items under 41.7 ms); with the Mac busy (load average ~53, 2026-11-23) it measured 45.9 and 56.6 ms and failed, and passes when the machine is quiet. Left as it is for now; the fix is to time the release build (or count evaluations instead of milliseconds). Open.
- RG-Q92 Pick order (rig-pick-order): the last picked item is the primary and `also` holds the earlier picks oldest first (`Scene::picked_items`); rig commands read that order. Visible changes: Ctrl/Shift-click makes the clicked item active (Properties, gizmo, active mesh); Items-list Shift range makes the clicked end primary (was the anchor); removing the primary promotes the most recent pick (was the oldest); bridge `select` mode item add:true now adds (it replaced). Merge, Parent to Group Locator and New Group keep first-pick order; a light edit spreads to the selected lights whichever of them is edited. Provisional.

## LXO scene import: hierarchy, animation, rig (2026-11-23, branch lxo-animation-import, provisional)

From a real Modo 17.1 scene that wouldn't open (a user's file, never in the repo). Written from the file layout only; no Modo behaviour was observed.

- LXO-AN-Q01 Modo `locator` and `groupLocator` items both import as Group Locators (Operandi's one transform-only item); their draw shape, size and inherit channels aren't read. `meshInst` becomes a Mesh Instance of the mesh its `source` link names. Provisional.
- LXO-AN-Q02 A transform stack with more than one item of a kind (a "Zero" rotation beside the core one): the `type core` item gives the value; others are left out, noted only when not identity. Shear items aren't read (noted when any shear is over 1e-6). Provisional.
- LXO-AN-Q03 Polygon types other than FACE, SUBD and PSUB (B-spline curves, curves, text, bones…): the whole POLS chunk is skipped by name, its records unread (a BCRV record holds more than corner indices). Edge weights on edges no polygon has are dropped with a note; zero-area polygons and non-manifold edges are kept and marked as every importer does (`interchange::finish_mesh`). Provisional.

## LXO import: animation, cameras, lights and rigs (2026-11-24, spec lxo-import-anim-rig-2026-11-24, branch claude/project-thread-ahtmlr, provisional)

The spec's own defaults (Q-LXA-01..11) are used as written. Found while testing against its fixtures:

- LXO-AR-Q01 LXA-72 scale constraint `offset`: the file stores 0 for no offset. Read as an offset added to a scale of 1 (Operandi offset = 1 + stored). Only 0 was seen.
- LXO-AR-Q02 The spec's cases for `constraints.lxo` disagree with themselves about Dst_expr: its world matrices leave it unmoved, its modifier results move it by the Expression. The test follows the modifier results. Which is right?
- LXO-AR-Q03 LXA-71 says a direct link copies the driver's value, but the spec's cases show a distance driving a rotation in display units (1 m gives 1°). A link between a rotation and another channel imports as an Expression `A * π/180` (or `180/π`). Angles other than transform rotations (cone angle, user channels) are not converted yet. Confirm in the spec.
- LXO-AR-Q04 The spec's world matrices for `constraints.lxo` hold the targets where the edit layer pins them, not where their keys move them (the import drops those edits, LXA-03). The test pins those six targets to compare the constraint maths alone.
- LXO-AR-Q05 Q-LXA-08: Operandi's two-bone solver bends `ik-and-stacked-constraints.lxo`'s chain to the side its setup pose bends; the spec's matrices have the knee on the other side. The end joint, bone lengths and chain plane agree. Does the solver's `plane`/`sense` setting pick the side?

- LXO-AR-Q06 `HCRV` polygons (the path in `path-blend-shader.lxo`): read as per curve a count, a 4-byte flags word and that many point indices, imported as open Curve polylines with no end controls. The flags aren't decoded. Confirm the layout.
- LXO-AR-Q07 Planar IK three bars imports as chain IK (Q-LXA-14): the end reaches the goal within the solver's tolerance (a few mm in `planar-ik.lxo`); `weightA` (the bend share) and `orient2/3` have no Operandi setting and are left out.

## Multiresolution sculpting (2026-11-23, branch claude/project-thread-0cmp4g, provisional)

Built without a behaviour spec (Jason approved the feature; none of this was observed in the reference). Every answer below is Operandi's own provisional reading.

- MR-Q01 Detail flows up, not down: sculpting at level k changes only level k's detail (stored per point in a frame of the smoothed level below, so it rides along when lower levels change). Lower levels are not reshaped to follow a high-level stroke; stepping down after sculpting big forms at the top shows the lower level as it was. Open: should strokes at a high level also update the levels below?
- MR-Q02 Every level is plain Catmull-Clark of the whole cage (creases and the mesh's Boundary Rule at the first Add Level), whatever kind its polygons are; level meshes above 0 are Face polygons (no further display subdivision), keeping each base polygon's material, part and smoothing.
- MR-Q03 Budgets: at most 6 levels and 8,000,000 polygons per level (Operandi's numbers).
- MR-Q04 Above level 0, only UV maps (subdivided linearly per polygon), materials and parts carry; weight maps, morphs, colour maps, selection sets, vertex normals and edge weights stay on the base, and edits to them at a higher level are lost on stepping.
- MR-Q05 Locks and hidden elements are cleared on every level change.
- MR-Q06 Topology changes (any tool adding or removing polygons) while the mesh is multiresolution leave the detail unable to follow: stepping and Add Level refuse with a reason, the read-out says "polygons changed", Apply Multiresolution keeps the mesh as it is; undo restores.
- MR-Q07 Attenuate (SCT-43) fades the current level's detail toward the smoothed level below (the level with no detail of its own); greyed at level 0 and on plain meshes. Its Amount (25 %) is the share of the remaining detail removed per full-weight dab.
- MR-Q08 Command names are Operandi's own (`operandi.multires.*`) until the name map has the reference's; Convert to Multiresolution and Background to Multiresolution on the painting Utilities tab stay greyed (their behaviour is unknown). Render uses the level you are at (no separate render level). No sculpt layers, no default shortcuts for stepping.

## Edge Bevel: Segments, Mitering, Loop Slide (2026-11-24 to 26, branch bevel-gizmos-mitering, proposed)

A Windows tester's reports and requests, built from their screenshots; no
Modo material. **Approved by the maintainer 2026-11-26** as built, with the
Segments counting, Vertex Bevel's handle and Free Orbit/Free Cursor's place
changed as noted.

- **EBC-Q-CAP1 (changes N08 for one case).** At a strip's single end where
  more polygons meet than its two sides and one end polygon, the edges
  between them now end where they cross the end's profile and the old point
  goes (it spiked back before). Concave corners, where they miss the
  profile, keep N08. Edge Chamfer unchanged.
- **EBC-Q-SEG1 (changes EBC-B3-02's panel name; approved 2026-11-26).**
  Round Level is shown as Segments: the rows of polygons across the strip,
  1 to 64, 1 being flat, any count (odd counts aren't measured). Edge
  Chamfer's Segments always counted this way (EBC-D-02) and are unchanged.
  Scripts: `segments`; `level` still sets 2 x level rows. Vertex Bevel and
  the Mesh Stack's Edge and Vertex Bevel ops show Segments too; Vertex
  Bevel's cap needs an even count above 1 (VX-B02's 2 x Round Level), so
  an odd count steps on to the next even one, and `level` still works.
  Mesh Stack ops saved with `level` load as 2 x level Segments, and ones
  saved before Loop Slide load with it on (as they were built).
- **EBC-Q-MO1 (fills EBC-Q03's Mitering for Edge Bevel).** Mitering Offset:
  an edge this far beyond the bevel on each side, with a second handle.
- **EBC-Q-LS1 (new option).** Loop Slide, Edge Bevel only, on by default
  (2026-11-26: on is the measured result, EBC-B3-03, at corners that
  aren't square; off keeps the full width through them).
  Off: points of the polygons around the strip that it would reach move out
  of its way (along their edge, or across the polygon beyond), so it keeps
  its full width. On: nothing moves; the strip slides along the edges beside
  it and narrows. `tool.attr edge.bevel loopSlide`.
- **EBC-Q-CP1 (new option).** Mitering Style, Edge Bevel only, for corners
  where rounded strips meet: Default (the default), Neutral, Uniform, Pinch.
  Patch and Radial are built but hidden until finished.
  `tool.attr edge.bevel miteringStyle` (`cornerPattern` still works).

## MeshBlend B1 (2026-11-24, branch claude/project-thread-20fok5, provisional)

Spec meshblend-2026-11-24. Readings where the spec is silent:

- MB-Q01 A Trim added or switched to with no Primary named acts on every Primary of the item (SB-11 sets the relationship by pick order; scripts and the panel's Add rows have no second pick). Untrim narrows it.
- MB-Q02 Keep Cage Points uses the cage at Subdivision 0 (SB-22 says the result sits on the cage; how strips then meet it is B2's question).
- MB-Q03 Duplicate and Convert makes the frozen copy but doesn't hide the MeshBlend Item or its sources (SB-53 says it hides them); Convert to Mesh is the existing Freeze of the item's whole stack.
- MB-Q04 When the polygons rebuilt from provenance break a mesh rule, the result falls back to the engine's triangles for that rebuild (seen once in random sequences); the cause is open.
- MB-Q05 A rebuild that fails shows the failure on the op's row and passes the (empty) Base Mesh through, as every Mesh Stack op does; ST-04's "keep the last good result" isn't built yet.
- MB-Q06 Sources are read as the viewport shows them (SB-23): subdivision polygons as their smooth surface at the source's Subdivision level, plain Face polygons as they are. SB-20 says every source is smoothed; read literally, a plain 1-segment cube blended as a ball and a cylinder's n-gon cap came to a point (Jason's QA, 2026-11-24).

## Schematic workspaces (2026-11-24, branch claude/project-thread-mna1o9, provisional)

Spec schematic-2026-11-24 (BEHAVIOR-SCHEMATIC, LXO-SCHEMATIC-FORMAT). Readings where it is silent:

- SC-Q10 LXO places are scaled 6 points per unit, not the suggested 4: Operandi's nodes are 170 points wide with a row per linked channel, so at 4 columns and stacked nodes overlapped. Nodes that still overlap are moved down (each column keeps its order). Check HC-02's CONTROLS page.
- SC-Q11 SC-16: a new piece on no page shows (unsaved) on the first saved page holding an item it touches, beside that item; it is saved when moved or arranged. Pieces on no page and touching nothing placed show on "Unplaced".
- SC-Q12 Not built yet: marquee select (SC-69; left-drag on empty space still pans), multi-node selection, collapse (SC-29), backdrops/notes/elbows (imported as counts only), expanded vector rows (VXPD), hover magnify (SC-67). Fit Selected frames the node clicked last.

- MB-Q07 New MeshBlends use the Closed, clean output (SB-50 lists Closed as the default); Closed keeps the cut faces as triangles.
- MB-Q08 Relative Strip Width is a share of the average cage edge length of all enabled sources (SB-31b says the two sources at that join).
- MB-Q09 A strip's own Width is always a distance, whatever Width Units says; Rows stay shared so corner patches always fit. Overrides apply only while Strip Width is above 0.

## Primitive MeshOps with the tools' options (2026-11-25, GitHub #14, branch claude/project-thread-8fccr9, provisional)

- PRIM-OP-Q01 The eight primitive ops (Cube, Sphere, Ellipsoid, Cone, Torus, Cylinder, Capsule, Teapot) run their tools' generators (primitive_ops.rs). Value names the map doesn't give are Operandi's own: `makeUVs`, `axis`, `method` (Sphere Mode), `level`, `capTop`/`capBottom`, `polType`, `edgeWeight`, `compensation`, `gaps`, `uvMap`, `endSegments`, `endSize`, `ends`, `bulge`, `bulgeTop`/`bulgeSide`, `ringRadius`, `crossSection`, `height`, `size`. Rename when the map has the reference's.
- PRIM-OP-Q02 Not op values: the tools' handle rows (Handle Mode, Min/Max, Show Corner), Torus Hole Size (derived) and Ellipsoid Numeric Symmetry (a Symmetrize op does it). Sphere Mode offers Globe and Tessellated only (Quad Ball isn't built). The op form can't grey a row yet, so Subdivision Level shows in Globe mode too and is ignored there.
- PRIM-OP-Q03 An op's primitive keeps the "Texture" UV map name and no material; the tools rename to the Default Texture Map and add the Default Material (user preferences, which would make a saved stack evaluate differently per machine).

## Falloffs on primitives (2026-11-25, GitHub #15, branch claude/project-thread-8fccr9, provisional)

- PRIM-FO-Q01 The global falloff shapes the Cube, Sphere and Cylinder while they're made (shape_falloff.rs); only Linear, Radial, Cylinder, Noise and Invert layers can weigh new points, and the others grey the Falloff section with a reason. The weight scales (about the axis), moves and turns each ring; at weight w a value changes by w of its full amount.
- PRIM-FO-Q02 Follow Falloff spacing re-spaces rings by 1 + 4 × |slope| of the spine weight (Operandi's own number); greyed on the Sphere.
- PRIM-FO-Q03 (2026-11-26, #35) Cone, Capsule, Torus and Ellipsoid take the falloff too, tools and ops alike. Follow Falloff spacing only on the Cone (straight-sided); greyed elsewhere. The Torus's spine is its axis, so Spine sampling reads one weight per height. Their tools have no Fit Falloff to Shape yet. Supersedes the earlier "not yet".
- PRIM-OP-Q04 A primitive op's falloff copy reads the op's own positions (the item's local space) whether its layers have Use World Transforms on or off.
- PRIM-FO-Q04 Fit Falloff to Shape puts the Linear Start on the top of the shape, where the tool's top size handle is; there the tool's handle wins the press (TP-19), so the falloff's Start is moved with its axis arrows or by typing.

## Mesh Stack selections of edges and points (2026-11-25, GitHub #19, branch claude/project-thread-8fccr9, provisional)

- SEL-Q01 Edge ops pick edges (numbered as `Mesh::edges`, the sorted unique point pairs) and Vertex ops points; other ops polygons. Supersedes MOQ-21/MOQ-25. Selection ops picking in another kind convert: edge if one of its polygons, polygon if one of its edges, point if on a picked polygon or edge.
- SEL-Q02 Rule ops are Operandi's own (`operandi.edgeAngle.selop`, `operandi.border.selop`, `operandi.sides.selop`, `operandi.facing.selop`); Edges by Angle measures between the two polygons' normals (0° flat), an edge with one or three+ polygons is never picked by it.
- SEL-Q03 A captured Select by Index keeps the element count it was made on (`select_by_index_count` in the file); a different count below shows a hint on the op row, and the op still runs. Indexes typed into Select by Index have no count.
- SEL-Q04 Selection Set reads a set of the op's own kind first, then a polygon set of that name.
- SEL-Q05 Batch 1 rules (Vertex Count with Tris/Quads/N-gons presets, Area, Edge Length, Convex/Concave, Normal, Plane, Distance, N Randomly, Random Parts, Vertices by Edge Count, Symmetric, UV Seam) follow Steve Hill's own descriptions of his kit in #19; they measure in the mesh's own space only (no World Coordinates option or transform input yet). Polygon Sides and Facing were replaced by Vertex Count and Normal.
- SEL-Q06 Converting points to edges or polygons needs all their points picked; polygons and edges to points take any. Random Parts joins polygons through shared points.

## Live MeshOp falloffs and primitive op handles (2026-11-25, Steve Hill via Jason, branch claude/project-thread-8fccr9, provisional)

- MOP-H-Q01 A primitive op's falloff is its own: its Falloff Type (None, Linear, Radial, Cylinder, Noise) and that type's values are op rows, and changes evaluate at once. A new one starts fitted to the shape (Linear from top to base along its Axis; Radial and Cylinder at its centre and size). Combining several falloffs in the panel is parked (Jason, 2026-11-25).
- MOP-H-Q03 (2026-11-26, Jason) The selected primitive op also shows the tools' Move handles: an arrow per axis (72 px, Handle Scale) and a plane square per axis pair, in the op's item axes; a square on an arrow's shaft wins the press.
- MOP-H-Q02 With a primitive op's row selected and no tool on, the view shows its centre (moves in the view plane), one size square per axis and its falloff's points (Linear Start/End, Radial/Cylinder Center). Drags move by the view-plane delta like the falloff handles (falloff_view.rs); the size square takes the delta along its own axis. Other op types' handles come later, a family at a time.
## Tool tab cycling (2026-11-25, tool-tab-cycling v1.1, branch claude/project-thread-rjq1h8, provisional)

- TTC-Q01 (spec Q1) Ctrl+Tab in a multi-line text field pages the tool tabs; the field types no tab. Spec default.
- TTC-Q02 (spec Q2) A hidden or collapsed tool panel (no tab strip drawn) isn't reopened; the keys do nothing. Spec default.
- TTC-Q03 T1 reads "Create → Edit"; Operandi's Model strip has Select right of Create, so Ctrl+Tab on Create opens Select (rule 1, on-screen order).
- TTC-Q04 Ctrl+Shift+Tab was the Layout Switcher's key (UIC-18). The switcher now starts with no key (Layout > Recent Layouts… still opens it); a saved key map still holding its old factory key hands it to Previous Tool Tab. UIC-18 updated with Jason's OK (2026-11-25: Ctrl+` already switches layouts).
- TTC-Q05 Floating panels in their own OS windows don't take the keys (eframe runs no raw-input hook there); the main window does.
- TTC-Q06 Spec 5 (announce the tab): the switch sends a "<Tab> tab" value-changed event and puts the same text on the message line; a full screen-reader announcement waits for Q4's accessibility work.

## Older LXO scenes (old-lxo-2026-11-25, GitHub #20, 2026-11-25)

- **OLD-B01 Plug-in items still have their links read (OLD-04).** An item
  of a kind the importer doesn't know is never imported and its records
  aren't counted, but its LINK records are still read: a deformer,
  constraint or rig item stores its link to the mesh it changes, and
  LEG-10 refuses that mesh. Skipping the item whole would import such a
  mesh unchanged without saying so. Built *provisional:* links only.
- **OLD-B02 What counts as "unknown" (OLD-04).** Every kind other than the
  modelled ones (mesh, translation, rotation, scale, meshInst, transform)
  and the scene/shader list (camera, light, material...), so locators and
  rig items are skipped the same way; none of them was imported before.
  The note reads "'<kind>' items not imported (not modelling, or plug-in
  data)".
- **OLD-B03 Memory (OLD-06).** The old reader charged points and polygons
  twice against the import budget. Now it charges what the 16/17 reader
  does: T7 (1.21 M polygons) costs 1.1 GiB against 0.97 GiB for the same
  file as format 16.0. The app's budget is half the free memory (at least
  1 GiB), so T7 opens on any machine with about 2.2 GiB free.


## Action Center rule sheet v6, step 1 (2026-11-27, branch ac-rewrite-1, provisional)

Built: A-SEL (Selection axes = principal axes of the moved points),
C-BORDER (box middle of the edges shared with unselected polygons), S-01..S-04
for both, Element in force when chosen. Where the sheet left room:

- ACR-Q01 A-SEL with nothing selected: the sheet says "the points the tool moves", which is then every point of the mesh. Built that way (it was World before); a whole cube still gets World through the tie rule.
- ACR-Q02 A-SEL's Z sign, "the moved polygons' summed normal": built as the unit normals of the selected polygons; in Edge and Vertex mode the polygons using the selected edges or points; under Symmetry only polygons with a point on the side that votes. A polygon across the plane therefore votes on the + side.
- ACR-Q03 A-SEL with a centre placed on the − side (S-07 meets S-02): the axes come from the selected points on the side the centre was placed on, so centre, axes and the driving side agree. The sheet only says "+ side". Reset Action Center brings the + side's frame back.
- ACR-Q04 A-SEL ties: "within 1e-6 (relative)" is measured on the spreads' variances, relative to the largest one.
- ACR-Q05 C-BORDER's box is taken in world space (the sheet says item space only for C-SEL); with no border edge it is C-SEL's item-space box.
- ACR-Q06 A selection with points only on the − side gets World axes under Symmetry (S-02 "none there → World"), though its centre comes from the − side (S-01). Built as written; it looks odd on a one-sided selection.
- ACR-Q07 Polygon Bevel and Twist Bevel read the old "normal on Y" frame. Bevel's Shift arrow now follows the selection's average normal under Selection axes too, and its Inset arrow the frame's X laid square to it. Twist Bevel's Rotation Axis = Action Axis reads the frame's Z under Selection axes (the flat-patch normal), Y under every other axis rule.
- ACR-Q08 Smooth Shift and Thicken's Scale about Selection Border uses the same C-BORDER (box middle of the shifted border points; it was their average, over edges used by one selected polygon).
- ACR-Q09 **Sheet against a recorded case.** Rulebook case ACS-H-select-x-pair-rotate (Selection, Symmetry X, a tilted pair, Rotate Y 30° typed) was recorded turning about world Y. The sheet puts every tool's typed values in the one A-SEL frame, whose Y there is world Z. Built as the sheet says; the case is listed as a known gap in `ac_symmetry_rulebook.rs`. The nine Selection and Selection Border Scale cases of the same rulebook now pass with that frame (they were known gaps before).
- ACR-Q10 A-SEL under Symmetry with a selected polygon lying across the plane (rulebook cases ACS-H-select-x-PC-scale and -pairstraddle-scale, still known gaps): the sheet doesn't say whether such a polygon's + side points and its normal count. Built: its points strictly on the + side count, and its normal too.

## Action Center rule sheet v6, step 2: presses (2026-11-27, branch ac-rewrite-2, GitHub #32 and #39, provisional)

Built: what a press off the handle does under each Action Center (`ActionContext::press`), Element picking on every press (C-ELEM, A-ELEM with Z on the normal), C-AUTO and C-SCREEN placing on every press, every other centre staying put. Where the sheet left room:

- ACR-Q11 Screen before any press ("as C-AUTO"): the centre is Automatic's at the moment Screen is chosen and doesn't follow a later selection change until the next press. `CenterMode::Screen` holds a point; an unplaced Screen needs the data-model step.
- ACR-Q12 A-ELEM, an edge's X: "the edge's own direction" has two ways round; built from its lower-numbered point to the other.
- ACR-Q13 Presses pick or place while Move, Rotate, Scale or Transform is on. With no tool on a press still selects, and other tools keep their own press. The old one-shot click also worked before a tool was on; Element chosen then waits at the world origin for the first press with a transform tool.
- ACR-Q14 The Snaps and Precision presets keep their own set-up click and their own axes (`PickedFrame::from_z`); while a preset runs, presses don't re-pick. That one click is still taken before the handles are tested.
- ACR-Q15 C-AUTO's "work plane" is the plane the view's grid uses (XZ, YZ or XY through the work plane's centre, chosen per view unless fixed). A press that never meets it (edge-on, or behind the view) lands at the centre's own depth, facing the view.
- ACR-Q16 A press that moves the centre first keeps what the tool did so far as its own step, as the old empty-space press did; section 7's "re-apply in the new frame" is for menu changes.
- ACR-Q17 A placed centre snaps to the selection's own points, edges and polygons too (SN2-17 leaves what the tool moves out of a handle drag's targets; nothing moves while a centre is placed), and the search is at the pointer, not at the old centre's height above it.
- ACR-Q18 Gone with VAC-Q02: a press on the selected element no longer puts a moved centre back (under None and Automatic every press places; under the others nothing moves).
- ACR-Q19 Item mode is unchanged; its presses come with the Item-mode step.
- ACR-Q20 In a UV view with a UV tool on, Element picks the UV point or edge under every click (UVT-133 was one click).

