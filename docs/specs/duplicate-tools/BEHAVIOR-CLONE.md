# Clone (v1, 2026-11-08)

Behaviour-only spec. Read BEHAVIOR-DUPLICATE-SHARED.md first. Placer:
**Line**. Copier: **Copies**.

## 1. What it does

- CL-01 CONFIRMED (docs). Makes copies of the source **in a row**, each one
  a fixed step further on from the one before: fence posts, stair steps,
  a string of beads.
- CL-02 LIKELY (v1.2 correction). Each step can also **scale and turn**
  the copy relative to the one before, so a row can shrink, grow or curl:
  a tapering string of beads, a spiral of steps. This is **not random**.
  (The reference's docs call these rows "random fluctuation", but that
  page's text is borrowed from Scatter Clone: its Replace Source line even
  mentions "the scatter clone effect". The tool's own starting values,
  Scale 100 % and Rotate 0°, only make sense as a per-step factor.)

## 2. Properties

Section **Line** (placer):

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Number of Clones | integer, ≥ 1 | **1** CONFIRMED (probe) | copies made, not counting the original |
| Offset X / Y / Z | vector, length | **0, 0, 0** CONFIRMED (probe) | step from one copy to the next; set by dragging, CL-20 |
| Scale X / Y / Z | vector, percent | **100 %** CONFIRMED (probe) | size change per step, CL-12 |
| Rotate X / Y / Z | vector, degrees | **0°** CONFIRMED (probe) | turn per step, CL-13 |

Section **Copies**: Replace Source off, Flip Polygons off, Merge Vertices
off, Distance (DS-22).

## 3. Result

- CL-10 CONFIRMED (docs). Copy *i* (counting from 1) sits at the source's
  position plus *i* × Offset. The original stays put.
- CL-11 CONFIRMED (docs). With **Replace Source** on, the original is
  replaced by a copy in its own place (slot 0). Number of Clones still
  counts the extra copies [LIKELY].
- CL-12 LIKELY (DQ-13). **Scale compounds per step.** Copy *i* is sized
  by Scale^*i* per axis, about its own bounding-box centre: Scale 50 % with
  3 clones gives copies at 50 %, 25 % and 12.5 %. 100 % = no change.
  If DQ-13 shows the reference sizes every copy the same instead, only
  this rule changes; the builder's current reading ("every copy at
  Scale") is the fallback and is fine to ship until then.
- CL-13 LIKELY (DQ-13). **Rotate compounds the same way.** Copy *i* is
  turned by *i* × Rotate per axis, about its own bounding-box centre,
  X then Y then Z. [OP for pivot and order.]
- CL-14 [OP]. The Offset step is not scaled or turned by the copies'
  scale or rotation: positions stay on the straight line of CL-10.
- CL-15 [OP]. Scale is limited to above 0 % so no copy collapses; negative
  values are not allowed (use Mirror for flipped copies).

## 4. Viewport interaction

- CL-20 CONFIRMED (docs). **Press and drag** in a view: the drag sets
  Offset. The press point is the start, the pointer's spot on the work
  plane is the end, and Offset = end − start, so the first copy follows
  the pointer and the rest repeat the step.
- CL-21 [OP]. After the drag a handle shows at the first copy's position
  (an arrow from the source's centre with a grip at its tip) to adjust
  Offset in the work plane; Ctrl constrains to the first direction moved
  (DS-36). Snapping applies.
- CL-22 [OP]. Right-drag left/right changes **Number of Clones** (one per
  20 px), the same gesture Array uses for counts.

## 5. Edge cases to test

- CL-30 Cube, 3 clones, Offset (2,0,0): cubes at x = 0, 2, 4, 6.
- CL-31 Offset (1,0,0) on a 1 m cube with Merge on: four touching cubes
  weld into one strip; each pair of touching faces is removed (DS-25),
  leaving one closed box.
- CL-32 Rotate Y 45°, 3 clones: copies turned 45°, 90°, 135°.
- CL-34 Scale 50 %, 3 clones, Offset (2,0,0) on a 1 m cube: copies at
  x = 2, 4, 6 sized 0.5, 0.25, 0.125 m (DQ-13).
- CL-33 Item mode: three new items, positions stepped, source untouched.
