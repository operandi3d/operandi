# Array (v1, 2026-11-08)

Behaviour-only spec. Read BEHAVIOR-DUPLICATE-SHARED.md first. Placer:
**Grid**. Copier: **Copies**.

## 1. What it does

- AR-01 CONFIRMED (docs). Copies the source into a **3D grid**: so many
  along X, along Y and along Z. Tiles, shelving, windows on a building,
  a crate stack. Counts multiply fast: 4 × 4 × 4 = 64 pieces.

## 2. Properties

Section **Grid** (placer):

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Count X / Y / Z | integers, ≥ 1 | **2, 1, 2** CONFIRMED (probe) | pieces along each axis, original included, AR-10 |
| Offset X / Y / Z | vector, length | **1, 1, 1 m** CONFIRMED (probe); first click resets it, AR-20 | spacing between neighbours |
| Jitter X / Y / Z | vector, length | **0** CONFIRMED (probe) | random extra shift per piece, AR-13 |
| Scale X / Y / Z | vector, percent | **100 %** CONFIRMED (probe) | same size change on every copy, AR-14 |
| Between | checkbox | **off** CONFIRMED (probe) | AR-12 |

Section **Copies**: Replace Source off, Flip Polygons off, Merge Vertices
off, Distance (DS-22).

## 3. Result

- AR-10 CONFIRMED (docs: 4×4×4 makes 64). Count is the number of grid
  **cells** along each axis, and the original fills the first cell. So
  Count 3 × 1 × 1 = the original plus 2 copies. Count 1 on an axis means
  no copies along it.
- AR-11 CONFIRMED (docs). Cell (i, j, k), counting from 0, sits at the
  source's position plus (i × Offset X, j × Offset Y, k × Offset Z).
  Negative offsets go the other way.
- AR-12 CONFIRMED (docs). **Between** on: Offset is the distance from the
  first cell to the last, and the cells in between are spread evenly
  (step = Offset / (Count − 1)). With Count 1 on an axis, that axis's
  Offset does nothing.
- AR-13 CONFIRMED (docs). **Jitter**: each copy is shifted by a random
  amount between −J and +J [sign range OP] on each axis, on top of its
  grid spot. Fixed seed (as CL-14). The original moves only if Replace
  Source is on (CONFIRMED, docs).
- AR-14 CONFIRMED (docs). **Scale** sizes every copy by the same percentages
  about its own bounding-box centre [pivot OP]. The original keeps its size
  unless Replace Source is on. An odd number of negative scale values turns
  copies inside-out; their point order is reversed so normals still face
  outward [OP], as in MR-41.

## 4. Viewport interaction

- AR-20 CONFIRMED (docs). The **first click** sets Offset to the source's
  **bounding-box size** on each axis, so the pieces sit side by side with
  no gaps. (A flat source gets 0 on its flat axis.)
- AR-21 CONFIRMED (docs). **Left-drag** changes spacing: dragging left/right
  changes the Offset of the work plane's horizontal axis, up/down the
  Offset of its other axis. Which world axes these are follows the work
  plane, which follows the view.
- AR-22 CONFIRMED (docs). **Right-drag** the same way adds or removes
  pieces: changes **Count** on those two axes (one per 20 px [OP]). On a
  Mac without a right button, Ctrl+drag is *not* used for this (it's the
  constrain modifier, DS-36); a two-finger trackpad drag-click acts as
  right-drag [OP].
- AR-23 CONFIRMED (docs). **Ctrl** held before pressing keeps the drag to
  the first direction moved (only one axis changes).
- AR-24 CONFIRMED (docs: "each handle's colour matches its axis"). Three
  arrow handles from the original's centre, one per axis, in axis
  colours, each with a grip at the first neighbour's spot: drag a grip
  to change that axis's Offset.

## 5. Edge cases to test

- AR-30 1 m cube, Count 3 × 1 × 2, first click: six cubes touching, Offset
  (1, 0, 1).
- AR-31 Same with Between on, Offset X 4: cubes at x = 0, 2, 4.
- AR-32 Merge on, cubes touching: shared points weld and the touching
  faces are removed (DS-25), leaving one closed block.
- AR-33 Count 200 × 200 × 30 on a 1000-polygon mesh: refused with the
  too-many message (DS-51).
- AR-34 Scale X −100 %: mirrored copies, normals outward.
