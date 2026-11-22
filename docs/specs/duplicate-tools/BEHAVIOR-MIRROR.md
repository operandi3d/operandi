# Mirror (v1, 2026-11-08)

Behaviour-only spec. Read BEHAVIOR-DUPLICATE-SHARED.md first; this file
adds to it. Tags as in README.md.

## 1. What it does

- MR-01 CONFIRMED (docs). Makes a **mirror-image copy** of the source on
  the other side of a flat **mirror plane**. The plane can be moved and
  turned, live, with a handle or by typing.
- MR-02 CONFIRMED (docs, probe). Changing **Angle** away from 0° hinges the copy
  open or shut, so you can make a V shape (wings, a book, a roof) instead
  of a flat-on reflection. MR-21 says exactly how.
- MR-03 Beginner case (this must be rock solid): half a model in polygon
  mode, open seam lying on X = 0, Mirror, click in the view. Result: a
  whole, symmetrical, **single connected** mesh with no seam and no
  inside-out polygons.

## 2. Properties

Section **Mirror Plane** (placer):

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Axis | dropdown X / Y / Z | **X** CONFIRMED (probe) | the plane's facing direction: X means the plane is the Y-Z plane |
| Angle | number, degrees | **0°** CONFIRMED (probe; the docs say 180, see MR-21) | MR-21 |
| Mode | dropdown: Axis, Free Rotation | **Axis** (LIKELY: first listed) | Three Points: later |
| Center X / Y / Z | vector, length | **0, 0, 0** CONFIRMED (probe) | a point on the plane |
| Left X / Y / Z | vector, unitless | follows Axis (MR-22) | greyed in Axis mode (CONFIRMED, docs note) |
| Up X / Y / Z | vector, unitless | follows Axis (MR-22) | greyed in Axis mode (CONFIRMED) |
| Snap to World Axis | checkbox | **on** CONFIRMED (probe) | MR-33 |

Section **Copies** (copier): Replace Source **off** CONFIRMED (preset), Flip Polygons
**off**, Merge Vertices **on** in Vertex/Edge/Polygon mode (CONFIRMED:
the reference's plain-text tool preset turns it on for the component
versions), **off** and greyed in Item mode (DS-24), Distance (OPEN,
**0 m** CONFIRMED (probe): exact overlaps only, DS-22).

Labels and layout per UIC-110..117. Vector rows per UIC-114.

## 3. The plane

- MR-20 CONFIRMED (docs). The plane passes through **Center**. **Axis**
  picks which world axis the plane faces: Axis X = plane is the Y-Z
  plane through Center, copies flip in X.
- MR-21 CONFIRMED (probe). **Angle turns the mirror plane about its
  hinge line**, the line through Center along **Left**, by exactly Angle.
  Angle 0° is the plain reflection. Probe: a flat square on the X-Z
  floor, Axis X, Angle 90°: the plane turned a quarter turn about the
  vertical Left line and became the X-Y plane, so the copy was flipped
  in Z (it landed beside the source, sharing the edge on the hinge).
  The reference's docs describe a starting value of 180 meaning "plain
  flip"; the tool itself starts at 0 and 0 is the plain flip. Build to
  the tool: 0 = plain mirror. Range −360..360 [OP].
- MR-22 CONFIRMED for X (probe: starting Left = (0, 1, 0), Up =
  (0, 0, 1)); Y and Z by the same rotation [LIKELY]. In **Axis** mode
  Left and Up come from Axis: Axis X → Left = Y, Up = Z. Axis Y →
  Left = Z, Up = X. Axis Z → Left = X, Up = Y. The rows show these
  values, greyed. The hinge line of MR-21 runs along Left.
- MR-23 CONFIRMED (docs). **Free Rotation** mode un-greys Left and Up so
  the plane can face any direction, and adds a second handle to turn it
  (MR-31). The plane's facing direction is Left × Up. When a typed Left
  or Up isn't at right angles to the other, the other is corrected to be
  [OP], and both are kept unit length.
- MR-24 later. **Three Points** mode (a triangle handle that sets the plane
  from three dragged corners). Leave it out of the dropdown for now.

## 4. Viewport interaction

- MR-30 OPEN (DQ-5). **First click.** Provisional: the plane stays where
  Center says (world origin at first use), and the first click in a view
  simply shows the result and the handle. It does not move the plane to
  the click point. Reason: the beginner case (MR-03) is "mirror across 0",
  and a stray click must not shift the plane off the seam.
- MR-31 CONFIRMED (docs). The **handle** is the plane drawn as a see-through
  square around Center, sized to the source's bounds [OP], with:
  - a centre grip: **drag it to slide the plane** along its facing
    direction (changes Center). [Drag constrained to the facing direction
    is OP; the docs say "drag the centre".]
  - in Axis mode, a hinge-angle grip on the plane's edge: drag round the
    hinge (the Left line) to change **Angle**;
  - in Free Rotation mode, a second grip that turns the plane about its
    Up line, so with the Angle grip any facing can be reached.
  Reuse the corner-circle plane handle look from the slice tools (DS-35).
- MR-32 [OP]. Axis is changed from the panel only; the preview updates
  at once.
- MR-33 CONFIRMED (docs). With snapping on (X) and **Snap to World Axis**
  on, dragging the centre grip snaps Center to the world axes' zero
  (the plane through the origin) as one of its snap targets.

## 5. Result

- MR-40 CONFIRMED (docs). Component modes: new mirrored polygons are added
  to the **same mesh** as the source (DS-13).
- MR-41 CONFIRMED (probe: half-cube mirror, every polygon faces out). A reflection turns a shape
  inside-out, so the copy's polygon point order is **reversed
  automatically**; the copies' normals then point outward like the
  source's. **Flip Polygons** reverses them again on top of that, for
  sources whose normals were already wrong. (The docs' warning about
  inverted faces is what this rule prevents.)
- MR-42 CONFIRMED (docs + preset + probe: half-cube seam welded, 12
  points and 10 polygons). With **Merge Vertices** on, a new point
  within Distance of an original point is welded to it (DS-22, DS-23).
  For Mirror the welded point is placed **exactly on the plane** (the
  midpoint of the pair) [OP], so a seam drawn slightly off zero still
  closes cleanly and ends up exactly symmetric.
- MR-43 OPEN (DQ-6). **Polygons lying flat in the plane** (e.g. the cut face
  of a half-cylinder that was capped): their copy uses the same points
  after the weld, facing the other way. Provisional: remove **both** the
  original and its copy, because the pair would be an internal wall.
  Only exact matches (same points after welding) are removed.
- MR-44 [OP]. **Geometry crossing the plane** is copied as is; the copy
  overlaps the original. Nothing is cut. (Cutting is Symmetrize's job,
  a separate tool.) Points from different sides that land within Distance
  of each other still weld, which can pinch overlapping geometry: that is
  expected, and the user can turn Merge off.
- MR-45 [OP]. **Edges on the seam.** After welding, an edge whose two
  points were both welded is shared by the original polygon and its copy:
  the mesh is connected across the seam. Subdivision then smooths
  straight across it (this is the whole point for subdivision tutorials).
- MR-46 [OP]. **Symmetry mode** on or off makes no difference to Mirror.
- MR-47 DS-12 applies in **Item mode**: one new mesh item per selected item,
  placed and oriented as the mirror image. [OP] The new item must not
  carry a negative scale: mirror its geometry instead, and give it a
  normal (positive) transform, so later tools behave normally on it.
  Children are not copied (Hierarchy option is later).

## 6. Edge cases to test

- MR-50 Half a cube cut at X = 0 with the seam open (5 polygons, 8
  points, 4 of them on X = 0). Mirror on X, merge on: one closed mesh of
  10 polygons and 12 points (the 4 seam points are shared, not doubled).
  Normals all outward.
- MR-51 Same with seam at X = 0.0004, Distance 1 mm: seam still welds,
  welded points end at X = 0 exactly. With the starting Distance (0) it
  does not weld.
- MR-52 Same with Distance 0: nothing welds; two touching open halves.
- MR-53 Probe case: unit square (0..1 in X and Z, Y = 0), Axis X, Center 0,
  Angle 90°, merge off: copy covers X 0..1, Z −1..0 (flipped in Z), and
  shares the source's edge along Z = 0.
- MR-54 Replace Source on: only the mirrored half remains.
- MR-55 Flip Polygons on: copy's normals point inward; original untouched.
- MR-56 Item mode: original item untouched, new item "<name> 2" (or
  Operandi's next number), looks mirrored, has positive scale.
- MR-57 Undo after drop: one step removes the copy and un-welds the seam.
