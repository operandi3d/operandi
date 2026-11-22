# Drop Action: behaviour spec v1 (2026-11-06)

## 1. The control

- **DA-1** A dropdown button in the top bar, placed right after **Work Plane**
  and before **Render**. Its face reads `Drop Action: <current>` with a
  down-arrow, for example `Drop Action: (none)`.
- **DA-2** Clicking it opens a plain list with a check mark on the current
  entry. Entries, in this order, no separators:
  `(none)`, `Parent`, `Parent in Place`, `Match`, `Match Position`,
  `Match Rotation`, `Match Scale`, `Insert`, `Insert at Parent`,
  `Insert in Place`, `Place`, `Place and Align`.
- **DA-3** Default is `(none)`. The choice is an app-wide setting that stays
  until changed (see DA-Q1 for whether it survives a restart). Changing it is
  not an undo step and does not touch the scene.

## 2. When it applies

- **DA-4** A *drop* happens in a 3D viewport in **Item mode with no tool
  active**: press on an item (the **source**), drag, and release over another
  item (the **destination**). The source does not follow the pointer during
  the drag; nothing changes until release.
- **DA-5** With `(none)`, nothing new happens: dragging keeps whatever
  Operandi does today. With any other entry, a press-drag that starts on an
  item becomes a drop drag.
- **DA-6** While dragging, the item under the pointer is highlighted as the
  drop target (pick: the same pre-highlight used for hover selection).
  Releasing over empty space, over the source itself, or after Esc does
  nothing.
- **DA-7** If the source is part of a multi-item selection, every selected
  item is a source and the action is applied to each in selection order
  (pick, DA-Q4; the Insert family uses only the pressed item). The destination is never a source.
- **DA-8** One drop = one undo step, named after the action
  (`Undo Parent`, `Undo Match Position`, ...), following the Edit-menu rule.
- **DA-9** Actions that change the hierarchy refuse (no change, status-bar
  message) when they would make a loop: for Parent / Parent in Place, the
  destination is the source or one of its descendants; for the Insert family,
  the destination is a descendant of the source.
- **DA-10** The Items list keeps its own row drag-and-drop (reorder / parent).
  Drop Action is a viewport gesture (DA-Q3).

## 3. The actions

Terms: **world** = the item's final placement in the scene; **local** = its
Position/Rotation/Scale fields relative to its parent. "Keeps its world"
means the item does not move on screen; its local fields are recomputed.

| ID | Entry | Hierarchy change | Transform change |
|---|---|---|---|
| DA-11 | (none) | none | none |
| DA-12 | Parent | source becomes the **last child** of destination (leaving its old parent) | source keeps its **local** fields, so it jumps on screen |
| DA-13 | Parent in Place | same as Parent | source keeps its **world**; local fields recomputed |
| DA-14 | Match | none | source's **world** position, rotation and scale set equal to the destination's world ones |
| DA-15 | Match Position | none | world position only; source keeps its world rotation and scale |
| DA-16 | Match Rotation | none | world rotation only; keeps world position and scale |
| DA-17 | Match Scale | none | world scale only (per axis); keeps world position and rotation |
| DA-18 | Insert | source is placed **between** destination and destination's parent (see below) | source's world position + rotation = destination's; source keeps its own world scale |
| DA-19 | Insert at Parent | same as Insert | source's world position + rotation = destination's **parent's**; if destination has no parent, same as Insert |
| DA-20 | Insert in Place | same as Insert | source keeps its world |
| DA-21 | Place | none (pick, DA-Q2) | source's position moves so its centre sits on the destination's surface point under the pointer; rotation and scale unchanged |
| DA-22 | Place and Align | none (pick, DA-Q2) | as Place, then rotate the source so its +Y axis points along the surface normal at that point |

### Insert family detail (DA-18..20, measured)

- **DA-23** The source leaves its old parent. It becomes a child of the
  destination's old parent (or a root item if the destination had none). The
  reference app adds it at the **end** of that parent's child list; the pick
  for Operandi is the destination's old slot (DA-Q5).
- **DA-24** The destination becomes the source's child. The destination
  **keeps its world** transform exactly; its local fields are recomputed under
  the source (for Insert: position 0, rotation 0, scale = destination world
  scale ÷ source world scale). The destination's own children come along
  unchanged.
- **DA-25** The source's own previous children stay its children.

### Match detail (DA-14..17, measured)

- **DA-26** All matching is in world space, so it works the same whatever
  parents the two items have. The source's parent never changes; its local
  fields are solved so the world result is right.
- **DA-27** Match Scale copies the destination's **world** scale (its own
  scale times its parents'), not its Scale fields.
- **DA-28** Match = Position + Rotation + Scale in one step (one undo).

### Place detail (DA-21..22)

- **DA-29** The surface point is the first hit of a ray from the eye through
  the pointer against the destination's polygons. If the ray misses the
  destination's surface at release, nothing happens.
- **DA-30** "Centre" is the item's pivot/origin, so a user who wants an object
  to sit on a surface first moves its centre to the bottom of its bounding
  box. This is the intended workflow for Place and Align.
- **DA-31** Destination must be a mesh item for Place/Place and Align;
  otherwise the drop does nothing.

## 4. Later (not in v1)

- **DA-32** Pause-over popup: holding still over a destination opens a small
  list of these actions for that one drop; modifier keys can pick an entry.
- **DA-33** Per-item drop overrides for presets ("when this preset is dropped
  into the scene / dropped onto / receives a drop, do X") and user drop
  scripts. Needs a preset browser first.

## 5. Test cases (measured numbers)

Rig: **P** at (2,0,0), rotation Y 90°, scale 2. **D** child of P, local
(1,0,0), rotation X 30°, scale 1 → world (2,0,−2). **DChild** child of D,
local (0,1,0). **Q** at (−3,0,0). **S** child of Q, local (0,3,0), rotation
Z 45°, scale 0.5 → world (−3,3,0). Rotation order XYZ. Source S, destination D.

- **DA-T1 Parent:** S under D (D children: DChild, S). S local unchanged
  (0,3,0)/(0,0,45)/0.5.
- **DA-T2 Parent in Place:** S under D. S local position (−1, 0.049, −2.915),
  scale 0.25; world still (−3,3,0), rotation Z 45°, scale 0.5.
- **DA-T3 Match Position:** S still under Q; local position (5,0,−2), world
  (2,0,−2); rotation and scale fields unchanged.
- **DA-T4 Match Rotation:** S world rotation = (30,90,0); position and scale
  unchanged. With Q rotated Z 90°, S local rotation is solved so the world
  rotation still equals D's.
- **DA-T5 Match Scale:** S local scale 2 (= D's world scale 2, Q scale 1).
- **DA-T6 Insert:** S under P (reference app: last in P's child list). S local (1,0,0)/(30,0,0)/
  scale 0.25 → world (2,0,−2), rotation like D, world scale 0.5. D under S,
  local (0,0,0)/(0,0,0)/scale 4; D and DChild unchanged in world.
- **DA-T7 Insert at Parent:** as T6 but S local (0,0,0)/(0,0,0)/0.25 → world
  = P's (2,0,0), rotation Y 90°. D under S, local (4,0,0)/(30,0,0)/4, world
  unchanged.
- **DA-T8 Insert in Place:** S under P, world unchanged (−3,3,0), Z 45°, 0.5.
  D under S, world unchanged.
- **DA-T9 Insert / Insert at Parent, destination at root:** T at (4,0,0),
  Y 90°, scale 2, no parent; S root at (0,3,0). Both give S at root with
  (4,0,0)/(0,90,0)/0.5 and T under S with local scale 4.
