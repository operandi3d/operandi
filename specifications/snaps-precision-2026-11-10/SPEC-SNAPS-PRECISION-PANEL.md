# Snaps and Precision panel — behavior spec v1

Part 2 of the Snaps and Precision spec (2026-11-10; v1.2 adds doc answers and build guesses in §8). Part 1,
`SPEC-ABSOLUTE-SCALING.md`, covers the Absolute Scaling window and the
Scale Absolute / Size Absolute presets in depth; this part covers the panel
itself and the other presets.

Provenance: the reference help pages "Snaps and Precision Palette",
"Applying Snapping", "Specifying Action Centers", "Axis Rotate", "Using
Standard Tool Controls" and the Display preferences page, for names and
intent; the reference app's plain-text UI, menu and preset configuration
files (17.1v1) for layout, labels, preset composition, tooltips and command
names. The drag behavior of these presets is pointer-only and was not
observed (scripts cannot drive snapping or handles). REA-assisted: no.

Tags: **[docs]**, **[cfg]**, **[chosen]** (Operandi's pick, revisable),
**[open]**.

---

## 1. What it is

A small floating panel that gathers the precision tools in one place: a row
of **preset** tools that each switch on a ready-made combination of a
transform, geometry snapping, an action center/axis and sometimes a falloff,
plus a toggle for **Advanced Handles** and a button that opens the
**Absolute Scaling** window. [docs][cfg] Each preset is just a shortcut; it
creates nothing the user couldn't set up by hand.

## 2. Opening it

- History: it used to sit on the main modelling toolbox; in 17 it moved to
  the Edit menu (Jason). The Snapping submenu holds, in order: Snaps and
  Precision, Mesh Constraints, Snapping Options (F11), a divider, Snap
  Enable. [Jason's screenshot]

- Menu **Edit ▸ Snapping ▸ Snaps and Precision**. The menu item toggles a
  floating palette titled "Snaps", about 150 × 320 px; pressing it again
  closes it. It does not reopen by itself after restart (not persistent).
  [cfg]
- The same set of buttons also exists as a popover reached from the
  modelling toolbox ("Snaps and Precision" button). [cfg]
- The floating-palette visibility toggle key (backtick) shows/hides it with
  all other floating palettes. [docs]
- **Operandi placement [chosen, Jason 2026-11-10]:** a **Snaps and
  Precision** button on the **Measure** shelf toggles the panel open/closed.
  Follow the UI bible for the panel's look and cite rule numbers.

## 3. Layout (top to bottom) [cfg]

Vertical toolbar, each button showing icon and label, large icons.

| # | Control | Kind | Action |
|---|---|---|---|
| 0 | Snap Modes | popup (Grid, Geometry, Guides) | in the layout but switched off, and **not shown** in the open panel [Jason's screenshot]; leave it out |
| — | **Presets** | section divider (full-width label) | — |
| 1 | **Drag Snap** | button | activates preset Drag Snap, turns snapping on |
| 2 | **Drag Snap Rigid** | button | activates preset Drag Snap Rigid, snapping on |
| 3 | **Planar Rotate Snap** | button | activates preset Planar Rotate Snap, snapping on |
| 4 | **Planar Rotate Snap Axis** | button | activates preset Planar Rotate Snap Axis, snapping on |
| 5 | **Scale Absolute** | button | activates preset Scale Absolute, snapping on (part 1, B) |
| 6 | **Size Absolute** | button | activates preset Size Absolute, snapping on (part 1, C) |
| 7 | **Advanced Handles** | button | sets the tool handle draw style to Advanced (§5); shown **greyed out** in Jason's screenshot (panel open, no tool active), see Q5 |
| — | **Commands** | sub-section, small icons | — |
| 8 | **Absolute Scaling** | button | toggles the Absolute Scaling window (part 1, A2) |

- A preset button behaves like any tool button: pressing it makes that preset
  the active tool; pressing another replaces it; dropping the tool (Space /
  Esc / choosing another tool) leaves it. Each press also turns global
  snapping **on**, and it stays on after the tool is dropped [open Q2].
- Tooltips (paraphrased from the reference's own tool help, use as Operandi
  tooltips): Drag Snap "drag any vertex onto any other to snap single
  vertices"; Drag Snap Rigid "drag any vertex onto another to snap all
  connected vertices"; Planar Rotate Snap "rotate around one axis, snapping
  to a start and an end point"; Planar Rotate Snap Axis "rotate around an
  axis defined by three snapped points, then snap to a start and an end
  point"; Scale Absolute "scale by snapping to a start and an end point";
  Size Absolute "scale by snapping to a start point and setting a size".
  [cfg]

## 4. The presets

Each preset is a stack of standard parts. Parts and their settings come from
the preset configuration; the behavior of each part is the part's own spec
(snapping, action center, falloff, Element Move, transform) already handed
off or to come. [cfg]

### 4.1 Drag Snap [docs][cfg]

Parts: **Geometry Snap** (Mode = second option, Auto Center [Q3]; Tightness
15; Layers = Both) + **Action Center: Element** + **Action Axis: Auto**
(axis Z, handle mode 0) + **Element Falloff** (Element Mode Manual [Q3],
Range 0, Connected Elements = Use Connectivity, Mix 0) + **Transform** with
Translate only.

Behavior (help): this is Element Move with geometry snapping. Click and drag
any vertex, edge or polygon, whatever the current selection mode (Element
Move's Auto mode), and it moves with the pointer and snaps onto other
components in the scene. Only the clicked element moves (Range 0).

### 4.2 Drag Snap Rigid [docs][cfg]

Same parts as Drag Snap with two differences: Element Falloff **Connected
Elements = Rigid Connections**, and Action Axis handle mode 1. Behavior:
drag one component (vertex, edge or polygon) and snap it onto another
component; everything connected to it moves along rigidly, without
deforming.

### 4.3 Planar Rotate Snap [docs][cfg]

Parts: **Geometry Snap** (same mode as 4.1; Tightness 15; Layers Both) +
**Action Center: Element** + **Action Axis: Auto** (handle mode 1) + **Axis
Rotate** with **Advanced Handles** on (Angle 0, start direction +X).
Reference tool name: "Rotate Planar Snap 2D".

Behavior (help), three steps:
1. Click any component: the tool's handles center on it (and its axis follows
   that element's normal, per Element action center).
2. Drag the **outer handle** onto any component (snaps to vertices, edges or
   polygons): this sets the rest (zero) direction of the rotation.
3. Drag the **inner round handle** to rotate, freehand or snapped to another
   component, which gives an exact alignment.

### 4.4 Planar Rotate Snap Axis [docs][cfg]

As 4.3, but Geometry Snap Mode is the first option (Auto [Q3]) and the
Action Axis handle mode is 2, which adds a **second axis handle** (start and
end handles that define the plane of rotation). Reference tool name: "Rotate
Planar Snap 3D". The first click sets center and axis from the component
under the pointer, as in 4.3; the extra handle then sets the rotation plane
from snapped points before you rotate. For complex, exact rotations. Exact
order of the three snapped points: [open Q4].

### 4.5 Scale Absolute and 4.6 Size Absolute

See part 1, sections B and C.

## 5. Advanced Handles [docs][cfg]

- The button sets the global tool handle **draw style** to **Advanced**. The
  styles are Invisible, Basic, Advanced and Default (Default returns to the
  factory style); also in View ▸ Tool Handles ▸ Draw Style and in the
  Display preferences (Selected Tool Handles). [docs][cfg]
- Advanced adds extra in-viewport controls where a tool has them (e.g. Axis
  Rotate's angle wheel and value bar whose handle can snap to the angle
  marks; a camera's focal distance and f-stop handles). Basic shows the
  minimum handles. [docs]
- The panel button only sets Advanced; it does not toggle back. Pressing it
  when already Advanced changes nothing [open Q5]. (The help calls it a
  visibility toggle.)
- `-` and `=` shrink and grow handles; unrelated to the style. [docs]

## 6. Command names (for the command registry) [cfg]

| UI | Invocation |
|---|---|
| Edit ▸ Snapping ▸ Snaps and Precision | `layout.createOrClose` with cookie `SnapPal`, layout `SnapsPalette_layout`, title "Snaps", width 150, height 320, not persistent, palette style |
| Drag Snap | `tool.set SnapDrag 1 snap:true` |
| Drag Snap Rigid | `tool.set SnapDragRigid 1 snap:true` |
| Planar Rotate Snap | `tool.set PlanarRotateSnap 1 snap:true` |
| Planar Rotate Snap Axis | `tool.set PlanarRotateSnapAxis 1 snap:true` |
| Scale Absolute | `tool.set ScaleAbsolute 1 snap:true` |
| Size Absolute | `tool.set SizeAbsolute 1 snap:true` |
| Advanced Handles | `tool.handleStyle advanced` (style: invisible, basic, advanced, default) |
| Absolute Scaling | `layout.createOrClose` cookie `AbsolutePalette`, layout `absolute_layout`, title "Absolute", 250 × 350, persistent, palette style; modifier-click alternative: `layout.create` same layout, 150 × 250, popover pointing at the button |
| Snap Modes (disabled) | Grid `tool.set snap.grid on`, Geometry `tool.set snap.element 1`, Guides `tool.set op.guide 1` |

Preset parts by tool name: Geometry Snap `snap.element` (attrs `mode`:
auto, autoCent, vertex, edge, edgeCent, polygon, polyCent; `size`
Tightness; `fglayer` Layers: background, foreground ("Active"), both;
plus `twoD`, `fixed`, `prehighlight`, `onlySelected`, `depth`,
`depthEnable`, `useConstraint`); Action Center Element `center.element`;
Action Axis Auto `axis.auto` (`axis`, `mode` = Handle mode, start/end
points); Element Falloff `falloff.element` (`mode`: automatic, manual;
`dist` Range; `connect`: element ("Ignore"), connect ("Use
connectivity"), rigid ("Rigid Connections"), loop ("Edge Loop"); `mix`);
Transform `xfrm.transform` (`T`/`R`/`S` switches, `H` haul mode); Axis
Rotate `xfrm.rotate` (`angle`, `mode` = Advanced Handles, `startX/Y/Z`).

Preset display names: SnapDrag "Drag Snap Tool", SnapDragRigid "Drag Snap
Rigid Tool", PlanarRotateSnap "Rotate Planar Snap 2D Tool",
PlanarRotateSnapAxis "Rotate Planar Snap 3D Tool".

## 7. Acceptance (behavior-level; no numeric oracles yet)

1. Measure shelf button opens the panel; pressing again closes it.
2. The panel shows, in order: Presets divider, the six preset buttons,
   Advanced Handles, Commands divider, Absolute Scaling.
3. Each preset button activates its preset and turns snapping on; the tool
   properties show its parts with the settings in §4.
4. Drag Snap: in any selection mode, dragging one vertex near another lands
   it exactly on it (snap rule as in the Snapping spec); nothing else moves.
5. Drag Snap Rigid: same drag moves the whole connected part by the same
   offset; an unconnected part in the same mesh stays.
6. Advanced Handles sets the handle style to Advanced (Axis Rotate then
   shows its angle wheel).
7. Absolute Scaling opens/closes the Absolute window (part 1).

## 8. Open questions → answered from the docs, with build guidance (v1.2)

Jason, 2026-11-10: research answers what the docs can; the builder fills
remaining gaps with sensible guesses, and Jason corrects later. Each answer
says which it is: **[docs]** = the reference help says so; **[guess]** = our
recommended default, not confirmed; change freely if real use says otherwise.

**Q1. Snap Modes row.** Answered: not shown (Jason's screenshot). Leave it
out.

**Q2. Snapping state and repeat presses.**
- [docs] Snapping is one global switch, off by default, toggled by the
  Snapping button or **X**; holding X during a drag flips it for that drag
  only (on if off, off if on).
- [guess] A preset button turns global snapping **on** and leaves it on when
  the tool is dropped (the preset sets the switch, it doesn't save and
  restore it). The user turns it off with X / the Snapping button.
- [guess] Pressing the active preset's button again drops the tool (like any
  toolbox tool button); pressing a different preset swaps tools.

**Q3. What the stored option numbers mean.**
- [docs] Element Move's Element Falloff has a **Mode** list: Auto, Auto
  Center, Vertex, Edge, Edge Center, Polygon, Polygon Center. *Auto* picks
  whatever element is under the pointer, regardless of selection mode, and
  centers the move where the pointer hits the surface; the *Center* variants
  measure from the element's center point. The help says Drag Snap's Element
  Move is set to **Auto**. Range 0 = only the clicked element moves.
- [docs] **Connected Elements**: Ignore (only that element), Use
  Connectivity (connected elements within Range only), **Rigid
  Connections** (every connected element moves by the same offset), Edge
  Loops. Drag Snap = Use Connectivity with Range 0 → effectively just the
  element; Drag Snap Rigid = Rigid Connections → the whole connected part.
- [guess] Geometry Snap **Mode**: Drag Snap, Drag Snap Rigid and Planar
  Rotate Snap = **Auto Center** (snap to the nearest vertex, or to the
  center of the nearest edge/polygon); Planar Rotate Snap Axis, Scale
  Absolute, Size Absolute = **Auto** (snap to the nearest vertex, or the
  nearest point on an edge/polygon). Tightness 15 = the capture radius in
  screen pixels. Layers = Both (active and background meshes).

**Q4. Planar Rotate Snap Axis sequence.**
- [docs] First click on a component sets the center and the axis from that
  component (Element action center: a polygon gives its normal; an edge or
  vertex gives that element's normal). The extra **axis handle** controls
  the plane of rotation. Then, as in Planar Rotate Snap, the outer handle
  sets the rest direction and the inner round handle rotates; all handles
  snap to vertices, edges and polygons.
- [docs, tool tooltip] The axis is defined by snapped points: a start and an
  end handle define the plane (the action-axis handles are "start" and
  "end"; dragging them redefines the axis).
- [guess] Sequence: (1) click a component → center + initial axis; (2) drag
  the axis **start** handle to snap point A and the **end** handle to snap
  point B: the rotation axis becomes A→B and passes through the center;
  (3) drag the outer handle to snap the rest direction; (4) drag the inner
  handle to rotate, snapping to the target. Angle shows in the tool
  properties during the drag.

**Q5. Advanced Handles.**
- [docs] Draw styles: Invisible, Basic, Advanced, Default (Default = back to
  factory). Also in View ▸ Tool Handles ▸ Draw Style and the Display
  preferences.
- [guess] The panel button is a **one-way set** to Advanced (it is not a
  toggle) and shows greyed out when the style is already Advanced, which
  matches Jason's screenshot. To get back, use View ▸ Tool Handles ▸ Draw
  Style. Operandi alternative, if it feels better in use: make it a toggle
  between Advanced and the previous style (Jason to decide later).

**Q6. Drag details.**
- [docs] Element Move: hovering highlights the element that a click would
  pick; click-and-drag on a vertex/edge/polygon moves it; a plain click
  shows axis handles for moving along an axis instead. Right-drag in the
  viewport sets the falloff Range (shown as a sphere).
- [docs] Snapping: inside **Inner Range** (screen pixels) the moving
  element jumps onto the target; inside **Outer Range** the target is
  pre-highlighted. *Fixed Snap* (off in these presets) would make it jump
  only between snap positions. Snapping does not change typed values.
- [docs] Element action center: clicking an element sets the center there
  and aligns the handles to that element's normal; it need not be the
  selected element.
- [guess] Drag Snap / Rigid: the clicked element's anchor point (vertex
  itself, or edge/polygon center in Auto Center) is what snaps onto the
  target; the rest follows by the same offset. Releasing ends one action.
- [guess] Undo: one step per drag (per mouse-down…mouse-up), as with other
  transforms; dropping the tool adds nothing.
- [guess] Scale Absolute / Size Absolute handles: per axis, a base handle
  and a scale handle on the axis line from the center; drag the base handle
  onto point A (snaps), then the scale handle onto point B; factor =
  distance(center, B) / distance(center, A) along that axis. Center = the
  clicked element (Element action center).
