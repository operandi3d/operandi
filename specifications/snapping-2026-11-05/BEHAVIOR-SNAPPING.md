# Snapping, vertex and grid: build spec v1, 2026-11-05

Behaviour only. Scope: the snapping switch and settings, Vertex targets and
Grid targets. Other target types (Edge, Edge Center, Polygon, Polygon
Center, Pivot, World Axis, Straight Line, Right Angle, Intersection, Box)
appear in the settings list but are greyed out for now. **Operandi choice**
marks a sane default we picked because Modo's value is unmeasured.

## Switch and keys

**SN-01 — One on/off switch, off on a fresh install.** A toggle button
above the viewport and key **X** turn it on and off. The switch is separate
from which targets are ticked.

**SN-02 — Hold X to flip snapping for one drag.** Holding X during a drag
turns snapping off if it is on, or on if it is off; releasing X restores it.
A tap (press and release with no drag) toggles as in SN-01.

**SN-03 — Typed values are never snapped.** A transform entered as numbers
lands exactly where typed, even next to a target. Tests SNP-O-01 and
SNP-O-02 in ORACLES-SNAPPING.json.

## Settings

**SN-04 — Snapping Options popup.** Edit > Snapping > Snapping Options, and
F11, which opens it under the pointer. Settings persist per user and apply
at once.

**SN-05 — Targets are ticked per mode: Global, Component, Items.** Each mode
keeps its own ticks. Global ticks apply in every selection mode; Component
ticks are added to Global in Vertex/Edge/Polygon modes; Items ticks are
added to Global in Items mode. Example: Global Grid + Component Vertex means
both are active while editing vertices, only Grid in Items mode. Several
targets can be on at once; the nearest one wins (SN-08).

**SN-06 — Defaults (Operandi choice where marked).**

| Setting | Default |
|---|---|
| Snapping | off |
| Global targets | Vertex only (seen in a real session; **Operandi choice** as the default) |
| Component / Items targets | none |
| Inner Range | 24 px (snap distance) |
| Outer Range | 40 px (pre-highlight distance) |
| Coordinate Rounding | Fine (existing Units setting; reuse it) |
| Grid Distance | 100 % |

Note: an earlier preferences hand-off (HANDLES-SNAPPING.md, HND-P-Q1)
suggested 3 px and 15 px. Those were the distances of two landing tests,
not the ranges. Use 24 and 40.

**SN-07 — Presets (light version).** "Default" restores SN-06. Saving named
presets per mode can wait (**Operandi choice**).

## What snaps

**SN-08 — The tool handle snaps, not the pointer.** The handle sits where
the action centre puts it (e.g. Selection Center). While dragging, take the
handle's would-be screen position; find the nearest active target in screen
pixels.

- Within Outer Range: pre-highlight that target (a theme colour marker).
- Within Inner Range: the handle jumps exactly onto the target in 3D, and
  the selection moves by the same offset.
- Leaving the Inner Range releases it (**Operandi choice**: no extra
  stickiness).

Ranges are screen pixels, so they cover less world distance when zoomed in.

**SN-09 — Vertex targets.** Every visible point of the edited mesh except
the ones being moved, plus visible points of other visible meshes. Hidden
points never count. **Operandi choice** for other meshes and hidden ones
(unmeasured).

**SN-10 — Vertex landing is exact and doesn't weld.** Move one point onto
another: it lands on the same coordinates to six decimals, the two points
stay separate (no merge), counts and polygon lists are unchanged, and the
selection stays as it was. Released 3 px and 15 px short of the target,
both land exactly. Tests SNP-O-SN-03 and SNP-O-SN-03-15px (Front ortho,
288.3 px per metre).

**SN-11 — Grid targets.** Grid line crossings on the current work plane, at
the grid square size the viewport is showing (fixed or dynamic grid, as in
the Units/Grid spec). Grid Distance below 100 % snaps only when the handle
is within that fraction of a square from a crossing (**Operandi choice** for
the exact rule). The snapped point keeps its height off the work plane
unchanged (only the in-plane coordinates snap).

**SN-12 — Snapping and other tools.** Applies to Move, Rotate's centre,
Scale's centre, the Draw tools' point placement and Tweak. Rotate and Scale
amounts themselves don't snap (that is the separate angle/step increment).
Snapping respects Symmetry: the mirrored side follows the snapped result.

## Feedback

**SN-13 — Status.** The viewport's Current Tool overlay shows that
snapping is on and which targets are active.

**SN-14 — Undo.** Toggling snapping is not an undo step. A snapped drag is
one undo step like any drag.

## Not covered (later)

Edge, Polygon, Box and the other target types; Fixed Snap; Layers
(Background/Active); Only Selected; 2D Snap; Depth Limitation; pivot snap
size; exact Modo engage and release distances; factory values.
