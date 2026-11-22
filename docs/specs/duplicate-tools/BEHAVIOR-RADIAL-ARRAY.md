# Radial Array (v1, 2026-11-08)

Behaviour-only spec. Read BEHAVIOR-DUPLICATE-SHARED.md first. Placer:
**Circle** (the same one Lathe uses, BEHAVIOR-LATHE.md section 3 and 5).
Copier: **Copies**.

## 1. What it does

- RA-01 CONFIRMED (docs). Copies the source **round a circle**, each copy
  turned to face the same way relative to the centre: chair legs, bolts
  round a wheel, petals, clock marks. With a rise per copy it makes a
  **spiral**: spiral stairs.

## 2. Properties

Section **Circle**: exactly Lathe's rows (Count, Axis, Center X/Y/Z, Axis
X/Y/Z, Start Angle 0°, End Angle 360°, Offset 0), with these differences:
- Count start value **8** CONFIRMED (probe), the same as Lathe; it
  means pieces, RA-10.
- Offset = rise along the axis **per copy**.

Section **Copies**: Replace Source off, Flip Polygons off, Merge Vertices
off, Distance (DS-22).

## 3. Result

- RA-10 PARTLY CONFIRMED (probe: Count 4 on one cube gave 4 cubes in
  total, so Count includes the original). Spacing still OPEN (DQ-10).
  Provisional:
  - **Count** is the total number of pieces in the ring, **the original
    included**. Slot *i* (0 to Count − 1) is at angle Start + *i* × step.
  - **Full turn** (End − Start a whole multiple of 360°): step =
    (End − Start) / Count, so the last copy does **not** land on top of
    the first. 6 over 360° = every 60°.
  - **Part turn**: step = (End − Start) / (Count − 1), so the first and
    last pieces sit exactly on Start and End. 3 over 0..90° = 0°, 45°, 90°.
  - The original is slot 0 and is **not moved**, whatever Start Angle is.
    Copies are placed relative to it: copy *i* is the original turned by
    *i* × step about the axis (and raised *i* × Offset). So Start Angle
    only matters with Replace Source on, where slot 0 is a copy turned by
    Start.
- RA-11 CONFIRMED (docs: "Replace Source puts a clone at the 0° position").
  Replace Source on: the original is removed and every slot, including
  slot 0, gets a copy turned by its own angle.
- RA-12 CONFIRMED (docs). Each copy is **turned** about the axis, not just
  moved, so all copies face the centre the same way.
- RA-13 [OP]. Merge Vertices welds copies that touch, as in DS-22: a ring
  of wedges that meet makes one connected ring.
- RA-14 DS-12 applies in **Item mode**: new items, each with its position
  and rotation turned about the axis (the docs confirm item copies get
  their transforms set).

## 4. Viewport interaction

- RA-20 CONFIRMED (docs). Same Circle handle as Lathe (LT-30..33): the first
  click sets Center on the work plane; start and end cubes on the arc set
  the angles; axis end cubes tilt the axis; clicking away from the handle
  moves the centre (DS-34).
- RA-21 [OP]. Right-drag left/right changes Count (one per 20 px), as in
  Array and Clone.

## 5. Edge cases to test

- RA-30 Cube at (2,0,0), axis Y, centre 0, Count 4, full turn: cubes at
  (2,0,0), (0,0,−2), (−2,0,0), (0,0,2), each turned with the ring.
- RA-31 Count 3, End 90°: copies at 0°, 45°, 90°.
- RA-32 Offset 0.25, End 720°, Count 24: a two-turn spiral rising 0.25 per
  step.
- RA-33 Wedge of a ring (a 60° slice touching its neighbours), Count 6,
  Merge on: one closed, connected ring.
- RA-34 Replace Source on, Start 30°: the original is gone; the first copy
  sits turned 30°.
