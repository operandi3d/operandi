# Lathe (v1, 2026-11-08)

Behaviour-only spec. The reference calls this tool **Radial Sweep**; we
call it **Lathe** (DS-61). Read BEHAVIOR-DUPLICATE-SHARED.md first. It
shares its placer (the **Circle**: centre, axis, start/end angle, rise)
with Radial Array; build that placer once.

## 1. What it does

- LT-01 CONFIRMED (docs). Spins a **profile** round an axis and joins the
  positions into one continuous surface: bottles, glasses, plates, vases,
  chess pieces. With a rise per step it makes springs and coiled cords.
- LT-02 CONFIRMED (docs). The profile can be a **curve** or the outline of
  **polygons**. The docs warn that sweeping closed solids (a cube, a
  sphere) gives poor results; we don't block it.
- LT-03 Beginner case: draw a half-silhouette of a wine glass as a curve
  or a strip of edges next to the Y axis, Lathe, click on the axis. Result:
  one closed-round, welded, outward-facing glass.

## 2. What counts as the profile

- LT-10 CONFIRMED (docs). **Polygon mode:** the selected polygons. Each
  curve, or line polygon (two-point or multi-point open polygon), sweeps
  as a line. Each face polygon sweeps its **outline** (its edges). Nothing
  selected: the whole active mesh.
- LT-11 [OP] (beginner-friendly; not described in the docs). **Edge mode:**
  the selected edges sweep directly, one band of polygons per edge. Edges
  that join end to end make one connected band. **Vertex mode:** the edges
  joining selected points sweep (same as selecting those edges).
- LT-12 [OP]. Curves are turned into straight segments first, using each
  curve's own display step count, so what you see is what sweeps.
- LT-13 OPEN (DQ-7). What happens to the source profile. Provisional:
  **curves and line polygons are removed** (the swept surface replaces
  them), **face polygons are kept**. Edges in Edge mode are part of
  existing polygons and are of course kept.
- LT-13a [OP] (answers DUP-Q03). **Where the surface starts.** Lines and
  curves, and border edges on a part turn, join straight onto the
  profile's own points (they are used up or stay as the open edge). Face
  outlines, and edges on a full turn, start on new points beside the
  profile, so the source faces stay untouched. A selection of faces
  sweeps the outline of the region they cover: an edge shared by two
  selected faces sweeps once, not twice.
- LT-13b [OP]. A profile that **branches** (three or more swept edges at
  one point) is refused: nothing is made and the status line says
  "Lathe needs a profile without branches."
- LT-14 later. Preset profile shapes (Size, Pivot, Offset, Reverse rows),
  and **Square** mode (sweep round a square instead of a circle, with
  Start Segment). Left out of the panel (UIC-117).

## 3. Properties

Section **Circle** (placer, shared with Radial Array):

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Count | integer, ≥ 1 | **8** CONFIRMED (probe) | number of segments, LT-21 |
| Axis | dropdown X / Y / Z | **Y** CONFIRMED (probe) | the axis the profile spins round |
| Center X / Y / Z | vector, length | **0, 0, 0** CONFIRMED (probe) | a point on the spin axis; set by the first click, LT-30 |
| Axis X / Y / Z | vector, length | **0, 1, 0** (follows Axis) CONFIRMED | the axis direction, for tilted axes; LT-32 |
| Start Angle | number, degrees | **0°** CONFIRMED | |
| End Angle | number, degrees | **360°** CONFIRMED | may be above 360 for coils |
| Offset | number, length | **0 m** CONFIRMED (probe) | rise along the axis per segment |

Section **Sweep** (copier):

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Flip Polygons | checkbox | **off** | DS-21 |
| Cap Start | checkbox | **on** CONFIRMED (probe) | LT-25 |
| Cap End | checkbox | **on** CONFIRMED (probe) | LT-25 |
| UVs Around | dropdown U / V | **V** LIKELY (probe: second choice) | which UV direction runs round the axis, LT-27 |

Merge Vertices is not shown for Lathe: the seam and the axis are welded
by the rules below, always.

The reference's docs list Axis X/Y/Z as "where the axis handle ends
sit". We read it as the axis direction (LIKELY, DQ-8); the axis line
goes through Center along that direction.

## 4. Result

- LT-20 CONFIRMED (docs). The profile is placed at Start Angle, then turned
  round the axis step by step to End Angle. Each step joins to the next
  with one ring of quads (one quad per profile edge per step).
- LT-21 OPEN (DQ-9). **Count** = number of steps over the whole Start→End
  range. Provisional: angle per step = (End − Start) / Count, so Count 8
  over 360° gives 8 segments of 45°.
- LT-22 [OP], needed for a clean result. **Full turn** (End − Start is a
  whole multiple of 360° and Offset is 0): the last ring lands on the
  first, so it is not made; the last step joins straight back to the
  first ring. The surface is **closed round, with no seam points
  doubled**. With Offset ≠ 0 the ends don't meet and nothing joins.
- LT-23 [OP]. **Points lying on the axis** (within 1e-6 m) are not
  copied: every step shares the single original point. The quads that
  would touch it become **triangles** meeting at that point (a pole at the
  base and the rim centre of a glass). No zero-area polygons are made.
- LT-24 [OP]. A profile edge lying **along** the axis (both ends on it)
  makes nothing.
- LT-25 CONFIRMED (docs). **Caps** only matter when the sweep is not a full
  turn. Cap Start fills the open face at Start Angle, Cap End at End Angle,
  with one polygon each per closed profile; open profiles are capped
  between the profile and the axis [OP]. On a full turn the cap rows are
  greyed (UIC-117).
- LT-26 [OP]. **Facing.** New polygons face **away from the axis** on
  average (outward), whichever way the profile was drawn and whichever
  way the sweep turns. Flip Polygons reverses that, for interior surfaces
  such as the inside of a cup drawn as a separate profile. (The docs say
  results can come out inside-out and Flip is the fix; we make the common
  case right on its own.)
- LT-27 LIKELY (docs: Sweep UVs picks the UV direction). New polygons get
  UVs: one direction runs 0→1 round the sweep, the other 0→1 along the
  profile's length.
- LT-28 [OP]. New polygons take the material, part and smoothing of the
  first selected source polygon, or the mesh defaults if the source was
  a curve.
- LT-29 [OP]. Profile already crossing the axis: swept as is; the
  surface passes through itself. No warning.

## 5. Viewport interaction (shared Circle handle)

- LT-30 CONFIRMED (docs, Radial Array). The **first click sets Center**
  where the pointer meets the work plane. With snapping on it snaps like
  any point (so it can land exactly on the origin or a grid line).
- LT-31 CONFIRMED (docs, Radial Array). Handle: the **axis line** through
  Center, with a small **cube at each end**; an **arc** round the axis
  showing the swept range, with a cube at the **start** and at the **end**
  angle.
  - drag the start cube round the arc: Start Angle;
  - drag the end cube: End Angle (dragging past a full turn keeps going,
    for coils);
  - drag an axis end cube: tilts the axis (Axis X/Y/Z);
  - drag the axis line itself [OP]: moves Center along the work plane;
  - click away from the handle: new centre (DS-34).
- LT-32 [OP]. Changing the Axis dropdown resets Axis X/Y/Z to that world
  axis.
- LT-33 [OP]. Count has no drag gesture; type it or use the field's drag
  arrows (UIC-53).

## 6. Edge cases to test

- LT-40 A 4-point open polyline from (0,1,0) to (1,0.5,0) to (0.5,0,0) to
  (0,0,0), Lathe round Y, Count 8, full turn: 24 polygons (the top and bottom rings are
  triangles meeting at single pole points), 18 points, no doubled seam
  points, normals outward.
- LT-41 Same, End 180°, caps off: open half shell, seam not joined.
- LT-42 Same, End 180°, Cap Start and Cap End on: two flat half-discs close
  the shell.
- LT-43 Single edge (0.5,0,0)-(0.5,1,0), full turn, Count 16: an open
  tube of 16 quads, welded round.
- LT-44 Offset 0.1, End 1080°, Count 48, a small square face away from the
  axis: a three-turn spring; ends not joined.
- LT-45 Profile drawn top-to-bottom versus bottom-to-top: identical facing
  (LT-26).
