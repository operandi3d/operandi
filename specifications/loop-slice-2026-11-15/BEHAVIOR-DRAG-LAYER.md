# Shared drag layer ("hauling"): behaviour spec v0.1, 2026-11-15

Behaviour only. Jason (11-15): "a lot of the mouse gesture stuff should be
the same from one tool to the next." It is, in the reference app: one
shared mechanism, one table of which value each mouse button drives per
tool. This file is that mechanism plus the table. It extends BIBLE
**UIC-76** (Hauling) and belongs in the shared tool pipe as its own layer,
after the tool's handles get first pick of the click.

Tags as in `BEHAVIOR-LOOP-SLICE.md`. REA-assisted: no.

## 1. Rules

- **DL-01 Handles win.** A press on a tool handle (or on a tool's own
  on-screen control, such as Loop Slice's strip) is the handle's. Only a
  press that misses every handle becomes a haul. [docs, UIC-76]
- **DL-02 Screen drag, not surface drag.** A haul reads pointer movement
  in screen pixels. On or off the mesh makes no difference. [settings: each
  haul is defined only by a button and a screen axis]
- **DL-03 Each button drives up to two values**: one by sideways motion,
  one by up/down. Left, middle and right buttons each have their own
  pair, so a tool can expose up to six values to the mouse. [settings]
- **DL-04 One axis per drag** (axis does not auto-switch mid-drag): every
  default in the table is set that way, so the first clear direction of
  the drag picks the value and the other axis is ignored for the rest of
  that drag. [settings: auto-switch is off in every default; "first clear
  direction wins" is our reading, DL-Q01]
- **DL-05 Whole-number values step.** Counts, sides and segments change by
  1 per fixed number of pixels; distances and percents change smoothly.
  Step size and speed: DL-Q02.
- **DL-06 Right = more.** Moving right (or up) increases the value.
  [guess, DL-Q03]
- **DL-07 Live.** The mesh updates during the drag; release keeps it; the
  tool stays live so you can haul again.
- **DL-08 The user can change the table** per tool ("Load / Save Default
  Hauling", already in the BIBLE near UIC-76). In the reference app each
  tool property has a small haul icon: drag on that icon with a button,
  sideways or up/down, to assign that button and axis to the property
  (Ctrl while dragging = replace instead of add); a plain click opens a
  menu to clear it. [settings] The table below is the starting set.
- **DL-09 Left button with no entry** = the tool's own left-drag action
  (draw a primitive, move the slice, sculpt...). The layer only claims the
  buttons the table lists.

## 2. The starting table (reference app defaults) [settings]

The pattern, which makes it learnable: **left = the main amount** (size,
inset, offset, shift), **right = how many** (segments, sides, count),
**middle = shape tweaks** (depth, scale, twist, rounding). Sideways is the
main value of each pair.

| Tool | Left: side / up | Middle: side / up | Right: side / up |
|---|---|---|---|
| **Loop Slice** | (moves current slice) | Current / Inset | Count / — |
| Add Loop | — | — | Gap / — |
| Polygon Slice | — | — | Gap / — |
| Polygon Bevel | Inset / Shift | — | Segments / — |
| Polygon Extrude | — | — | Segments / — |
| Edge Bevel (chamfer) | Offset / Mitering | Depth / — | Segments / — |
| Chamfer edit | Offset / — | Depth / — | Segments / Source Segments |
| Vertex Bevel | Inset / — | — | Level / — |
| Vertex Extrude | Inset / Shift | — | — |
| Smooth Shift | Shift / — | Scale / Max Angle | — |
| Sketch Extrude | — | Scale / Spin | Precision / — |
| Solid Sketch | — | Width / — | Precision / Slice |
| Tube | — | Radius / Twist | Segments / Sides |
| Cube | (draws) | Rounding Radius / Rounding Segments | Segments X, Y, Z together / — |
| Sphere | (draws) | — | Sides (and Order) / Segments |
| Ellipsoid | (draws) | Bulge Top / Bulge Side | Sides / Segments |
| Cylinder, Cone | (draws) | — | Sides / Segments |
| Capsule | (draws) | End Segments / End Size | Sides / Segments |
| Torus | Ring Radius / — | Hole / Cross-section | Sides / Segments |
| Arc | — | Angle / Radius | Segments / — |
| Helix | — / Offset | End / Start | Sides / — |
| Curve circle | Radius / — | — | — |
| Curve rectangle, ellipse, diamond | Size X / Size Y | — | — |
| Curve star | Size / Scale | — | Sides / — |
| Curve n-sided | Size / — | — | Sides / — |
| Linear clone | — | Scale / — | Number / — |
| Radial Align | Radius / Angle | Weight / — | Smooth / — |
| Smooth | — | Strength / — | — |

Move, Rotate, Scale and other plain transforms aren't in the table: they
use their handles, and a drag away from the handle uses the handle's main
axis (already built).

## 3. Profile layer (also shared)

Several tools carry the same "pick a 1D profile" piece in their pipe:
**Loop Slice, Polygon Bevel, Edge Bevel/Chamfer, Chamfer edit, Edge
Slide, Bridge, Sketch Extrude/Sweep, Tube**. [settings] Build it once: a
small preset list of profile shapes (Operandi's own), plus the common
switches where the tool uses them (Reverse Direction, Reverse Inset, Keep
Aspect). Loop Slice is the first user; Bevel's Profile option can move onto
it later.

## 4. Mac note

Mac trackpads have no middle button and right-click is a two-finger click
or Ctrl+click. Which keys stand in for middle-drag and right-drag on a
trackpad is DL-Q04 (Jason is on a Mac; one minute at the mouse).
