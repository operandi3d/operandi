# Bend: behaviour spec v2 (from scratch), 2026-11-16

Behaviour only. Replaces SPEC-BEND v1 (tutorial-gaps-2026-11-13) for
everything about handles, the tool pipe and the panel. v1's maths readings
(BD-Q2 beyond the end, BD-Q3 true arc) stand until measured (BH-Q01..Q03).

Sources, in order: the reference app's user guide (Bend, Deforming Items,
Action Centers and Falloffs pages), the reference app's plain-text config
(tool pipe recipe, attribute names, handle tooltips, option lists), and
Jason's own observation of the running app (2026-11-16, five views of a
capsule with Bend live). Bridge not used yet. REA-assisted: no.

Tags: [docs] user guide, [cfg] plain-text config, [obs] Jason's
observation, [guess] our reading, open until measured.

Why v2: Jason, 2026-11-16: "the bend tool is still not right. The tool
handles are not correct." Today's Operandi Bend has no ring, has its own
Base instead of using the action centre, bends by a sideways screen drag,
and picks its own bend direction. The reference does none of those.

---

## 1. What Bend is in the tool pipe

- **BN-01 Its own engine, on the shared pipe.** The Bend button is a recipe
  of three layers: **Action Center Auto**, **Action Axis Auto**, then the
  **Bend** tool. No falloff layer is part of the recipe (unlike Twist,
  which is Rotate + Linear falloff). [cfg] In the tool-presets table this
  is a row: `center.auto + axis.auto + xfrm.bend`.
- **BN-02 The bend's pivot is the action centre.** There is no separate
  "base". Where the action centre is, the bend starts. The panel shows the
  action centre's X/Y/Z under its own heading (**Auto Action Center**),
  and those fields move the ring. [cfg, obs]
- **BN-03 The bend turns around the action axis.** The Action Axis row
  (X / Y / Z, one always lit) sets the axis the geometry curls around.
  [docs: "an alternate axis if the initial selection is incorrect for the
  desired bending direction"; obs: the ring turns with it, BN-11]
- **BN-04 Shortcut Ctrl+E** (already built). [cfg]
- **BN-05** Snapping, Symmetry and user-added falloffs stack on it like any
  other tool on the pipe (they are not part of the recipe, but the pipe
  takes them). [docs: the panel lists Snapping; obs: Snapping on and
  Geometry Snap in the pipe readout]

## 2. Handles in the view

There are three things on screen while Bend is live. [docs, cfg, obs]

### 2.1 Angle ring (handle 1)

- **BN-10 A single circle** centred on the action centre. It is a plain
  ring, like one ring of the Rotate handle, not a filled disc. Colour: the
  handle blue used for rotate rings. [obs]
- **BN-11 The ring's plane is square to the action axis.** Axis Y: the
  ring lies flat (in the ground plane). Axis X: it stands up, facing along
  X. Axis Z: it stands up, facing along Z. Changing the Axis buttons turns
  the ring at once; the centre and the spine do not move. [obs]
- **BN-12 Constant size on screen**, like every other tool handle (Jason's
  rule: handles shrink with zoom). Radius about the Rotate ring's. [obs,
  existing Operandi rule]
- **BN-13 Drag on the ring sets Angle** the way a Rotate ring does: the
  angle swept by the pointer round the ring's centre, in the ring's plane,
  is added to Angle. Going further round gives more bend; dragging with the
  pointer further out from the ring gives finer control (the same pointer
  travel sweeps less angle). Tooltip: "Angle handle. Drag here to bend
  positions." [docs, cfg]
- **BN-14 Ctrl while dragging the ring** snaps the angle in steps, as the
  Rotate ring does in Operandi already (HM-20). [guess, BH-Q06]
- **BN-15 Hover highlight** as other handles (UIC rules for hover). [docs:
  handles turn yellow on hover]

### 2.2 Spine line and spine end (handle 2)

- **BN-20 The spine is a thin line** from the action centre to the spine's
  end: the "area of influence" Jason pointed at. Its colour is a pale
  violet, thinner than the ring. [obs]
- **BN-21 Spine X/Y/Z is a vector from the action centre to the spine's
  end** (not an absolute point). [cfg: "Spine vector on X/Y/Z"] It is in
  world (work-plane) space: changing the action axis did not move the
  spine's end and did not change the Spine values. [obs]
- **BN-22 The spine's end has a small marker** you can grab (the guide
  calls it a small square; the current app draws a small cross in the
  handle's light-cyan colour). Hover highlights it. Tooltip: "Spine
  handle. Drag here to move spine vector." [docs, cfg, obs]
- **BN-23 Dragging the spine's end moves it on the work plane's axes**;
  Ctrl while dragging keeps it on one axis. Snapping applies to it like
  any point handle. [docs; snapping is a guess, BH-Q07]
- **BN-24 Spine fit (revised 2026-11-16; replaces the old starting-spine
  rule and BN-32/BH-Q05 "spine keeps its vector").** Each click that
  places Bend's centre refits the spine: the first click, and every later
  click away from the handles.
  1. Take the bounding box of what Bend acts on (the selection, or the
     whole mesh when nothing is selected) and the clicked centre c.
  2. The candidates are the two world axes lying in the work plane (front
     view: X and Y; looking down: X and Z). Never use the axis pointing at
     the viewer, and ignore the action axis setting.
  3. For each candidate, measure the distance from c to the FAR side of
     the box along that axis.
  4. The spine runs along the candidate with the larger far-side distance,
     toward that far side, exactly that long.
  Acceptance (front view, XY work plane), box x -1.4..1.4, y 0..2,
  z -0.5..0.5:
  - click (1.37, 0.95) gives spine (-2.77, 0, 0)
  - (0.67, 1.86) gives (-2.07, 0, 0)
  - (0.06, 1.88) gives (0, -1.88, 0)
  - off the box at (-1.89, 0.96) gives (3.28, 0, 0)
  - a 1 x 2 x 2.8 m box (x 1, y 2, z 2.8) seen from the front, clicked
    mid-face, gives -Y 1.08 m, not Z.
  The old fit (base at the bottom of the box, spine up the work-plane
  normal) is gone.
- **BN-30a** The same click puts the centre on the mesh surface under the
  pointer (front-face clicks on that box give z = 0.5 exactly). Off the
  mesh, it lands on the work plane. With snapping on, it lands on the
  snapped point. "Surface hit even with snapping off" is provisional
  (CS-Q02).
- **BN-30b** The click also resets Bend's Axis to the work plane's normal
  (Z in a front view, Y looking down). The X/Y/Z buttons change it
  afterwards.
- **BN-25 The spine stays straight** while bending; it shows the
  rest-pose region, not the bent shape. [obs: after a strong bend the
  line and its end marker were still straight, outside the curled mesh]

### 2.3 Action centre and axis (shared layers, not Bend's own)

- **BN-30 Press away from the handles = new centre.** With Automatic
  (Action Center Auto) the centre is not fixed: a press that misses the
  ring and the spine's end moves the centre to where the press meets the
  work plane. [docs, Automatic action centre]
- **BN-31 Snapping decides where that press lands.** With snapping on and
  a point under the pointer, the centre lands on that point, in any
  selection mode (Jason was in Polygon mode). This is the snapping layer
  acting on the action-centre handle, not Bend picking a vertex. [obs;
  Jason agrees it comes from Automatic, 2026-11-16]
- **BN-32** The ring moves with the centre and the bend now starts there;
  the spine refits on every centre click (BN-24 revised 2026-11-16,
  answers BH-Q05).
- **BN-33 Action Axis Handle option: None / Single Handle / Two Handles.**
  Default None. Single Handle draws one extra handle (a line with a dot
  from the centre) that you drag to tilt the axis freely ("Axis single
  handle. Drag here to change axis vector."). Two Handles draws a start
  and an end handle that together set the axis plane ("Start handle /
  End handle. Drag here to define axis plane..."). This is what the guide
  calls Two Handles for off-axis bending. It belongs to the shared
  Action Axis layer, so every tool on Automatic gets it, not just Bend.
  [docs, cfg]
- **BN-34 Deform Space** checkbox under Auto Action Center, on in Jason's
  view. Config text: "Compute axis from deform space." Only matters for
  meshes shown deformed (morphs, deformers); Operandi shows the row and
  keeps the value. [cfg, obs; effect is BH-Q09]

## 3. Mouse map (drag layer)

Bend has **no entry in the reference's default hauling table** [cfg: it is
not listed], so under DL-09 the drag layer claims no button for it and
every press goes to Bend's own handles and the action centre:

| Press | What happens |
|---|---|
| Left on the ring | Angle (BN-13) |
| Left on the spine's end | Move the spine's end (BN-23) |
| Left anywhere else | New centre (BN-30, BN-31); see BH-Q08 for what the rest of that drag does |
| Ctrl + left on a handle | Constrained version of that handle (BN-14, BN-23) |
| Middle, right | Nothing from Bend (user can assign hauls per DL-08) |

So **a sideways screen drag does not bend.** Today's Operandi "drag left or
right to bend, 0.5° a pixel" is wrong and goes.

## 4. Panel (Tool Properties), in this order

Matches Jason's view of the reference panel; labels exact. [obs, cfg]

**Bend**
| Row | Control | Default | Notes |
|---|---|---|---|
| Angle | number, degrees | 0° | Bend at the spine's end; points nearer the centre get less, fading to none at the centre. [docs] |
| Spine X / Y / Z | three distance fields, label "Spine" on the first | from the first click (BN-24) | Vector from the centre (BN-21). |
| Slip UVs | checkbox | off | Shared with the other tools (one setting for all). [cfg] |
| Morph | dropdown: None, Transform, Keep Positions | None | Shared with the other tools. [cfg, docs] |
| Both Sides | checkbox | off | Also bends the negative side of the spine (behind the centre), mirrored. [docs] |

**Auto Action Center**: X, Y, Z (distance), Deform Space (checkbox).
**Action Axis Auto**: Axis X / Y / Z (three buttons, one lit), Handle
(None / Single Handle / Two Handles).
Then the usual Snapping and Geometry Snap sections when snapping is on.

There is **no Base row** and **no Auto choice on the Axis row** in the
reference. [obs]

## 5. Result (v2.1, measured)

Let c = action centre, S = spine vector, L = |S|, u = S/L, a = action axis,
s = (p - c)·u (distance along the spine).

- **BN-40 True arc.** Points with 0 <= s <= L follow a circular arc of
  constant curvature Angle/L. The spine keeps its length, a cross-section
  at s turns by Angle·s/L, and the width across the strip is kept.
- **BN-41 Past the end: rigid.** Points with s > L turn by the full Angle
  and carry on along the end's tangent.
- **BN-42 Behind the centre (s < 0):** with Both Sides off, they don't
  move. With it on, they are the exact mirror (same lean, same amounts).
- **BN-43 Direction and sign.** Positive Angle leans the spine toward
  S × a (spine cross axis). Spine +Y with Axis Z leans toward +X. Spine +Y
  with Axis X leans toward -Z. In right-hand terms, the bend turns by
  -Angle about a. (N01 already uses spine × axis.)
- **BN-44** A spine off the world axes works the same way, measured along
  the spine (spine (1,1,0): the region ends at s = sqrt2).
- **BN-45** Axis along the spine: open question BH-Q12, leave as is.
- **BN-46 Units:** `tool.attr xfrm.bend angle <n>` takes degrees; querying
  it returns radians.
- **BN-47** Tool values (Angle, Spine, axis, centre) are remembered between
  uses, as for other tools. Reset sets Angle 0 and Spine (0, 1 m, 0).
- **BN-48** Scripted runs honour the Auto Action Center X/Y/Z and the Axis
  values typed in the panel.

Acceptance cases:

- **BD-01** Strip along +Y from 0 to 2 m, centre at the origin, spine
  (0,2,0), Axis Z, Angle 90°: the top end lands at (1.273, 1.273, 0) and
  points along +X. The centre line is a quarter circle of radius 1.273 m.
- **BD-02** Same at 180°: the top end is at (1.273, 0, 0), pointing down
  (-Y).
- **BD-03** Points at y < 0 don't move with Both Sides off; with it on,
  they mirror.
- **BD-04** Axis X instead of Z: the same curve in the YZ plane, leaning
  -Z.
- **BD-05** Centre at (0,1,0): nothing below y = 1 moves.

---

## 6. Gaps in Operandi today (fix list for the builder)

Checked against `main` plus `nits-batch-1` (N01) on 2026-11-16.

| # | Today | Should be | Spec |
|---|---|---|---|
| G1 | No ring at all. | Blue Angle ring at the action centre, square to the action axis, constant screen size; dragging it sets Angle like a Rotate ring. | BN-10..15 |
| G2 | A sideways drag anywhere bends (0.5°/px). | No haul for Bend. Off-handle presses move the centre instead. | §3 |
| G3 | Bend has its own Base (and Base X/Y/Z rows). | Base is the action centre. Remove Base; show Auto Action Center X/Y/Z (+ Deform Space). Bend's button sets Automatic + Auto axis for the life of the tool (tool-presets row), restored on drop. | BN-01, BN-02, §4 |
| G4 | A press off the model moves the base; a press on the model does nothing. | Any press that misses the handles moves the centre, through the snapping layer, so a press on a point with snapping on lands on that point in any selection mode. | BN-30, BN-31 |
| G5 | N01 Action Axis has Auto/X/Y/Z; Auto = "axis most square to the spine". | Axis X/Y/Z only, one always lit; the bend turns around it; the ring turns with it at once. Starting axis BH-Q10. | BN-03, BN-11 |
| G6 | Spine end is a filled square in the selection colour; drag is in the view plane. | Small marker at the spine's end with hover highlight; drag on the work plane's axes, Ctrl = one axis, snapping applies. | BN-22, BN-23 |
| G7 | Spine line drawn in the selection colour, plus a drawn bent arc. | Thin pale-violet straight spine only; no extra arc overlay (the mesh itself shows the bend). | BN-20, BN-25 |
| G8 | Starting spine: base at the bottom of the box along the work-plane up axis. | Centre comes from Automatic (selection centre, or the click); spine fitted from there (BN-24, BH-Q04). | BN-24 |
| G9 | No Slip UVs, Morph rows. | Rows present, shared settings. | §4 |
| G10 | Action Axis Handle option missing. | None / Single Handle / Two Handles on the shared Action Axis layer (can be a later batch; it's shared work, not Bend's). | BN-33 |
| G11 | Status/panel note describes the old drag. | Rewrite: "Drag the ring to bend. Drag the spine's end to set the bent region. Click elsewhere to move the centre." | — |

Keep: the bend maths (BD-Q2/Q3 readings), Both Sides, falloff blending,
`tool.attr xfrm.bend angle/spineX/Y/Z/bothSides` names, Ctrl+E.

Note for the build side: Operandi's Move/Rotate/Scale already move the
centre on a press off the model (VAC-Q02) but keep it on a press on the
model. Bend should not copy that on-model rule (BN-31). Whether Move
should also jump onto a snapped point is BH-Q11; don't change Move in
this batch.

## 7. Open questions

Bridge (needs Jason's "go"; scripted, no hands):
- **BH-Q01** Arc shape: does the spine keep its length (true arc)?
- **BH-Q02** Points past the spine's end: turned rigidly, or left?
- **BH-Q03** Angle sign: with Axis Y and spine +Z, does +90° curl toward
  +X or -X?
- **BH-Q04** Starting spine direction and length on a target long along a
  ground axis.
- **BH-Q09** Deform Space: any effect on a plain mesh?
- **BH-Q10** Starting Axis button when Bend is picked.

At the mouse (Jason, about 3 minutes, Bend on a capsule):
- **BH-Q05** After clicking a new centre, does the spine's end stay put
  (spine vector changes) or move with the centre (vector kept)?
- **BH-Q06** Ctrl while dragging the ring: does it step the angle?
- **BH-Q07** With snapping on, does the spine's end snap to points?
- **BH-Q08** Click away from the handles and keep dragging: does the drag
  bend, move the spine's end, or only move the centre?
- **BH-Q11** (Move, not Bend) with snapping on, does a press on a point
  move the Move widget onto it too?
