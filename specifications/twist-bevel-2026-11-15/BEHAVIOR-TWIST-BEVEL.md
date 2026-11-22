# Twist Bevel: behaviour spec v1.1 (2026-11-15)

Behaviour-only. A builder can follow this without seeing any other program.
Provenance: written from a screen recording of a third-party tool in use
(kept on the research side) and Jason's description ("it lets you bevel out,
twist, taper and bend"). No code, files or internals of any program were
read. v1.1 adds three spacing/bias controls, ideas taken (in our own words)
from a feature write-up of "Modest", a Blender add-on by Christian, which
Jason shared. Numbers marked **[default]** are our own choices, not measured; they
are listed again under Open questions.

## 1. What it is

A polygon tool that pushes the selected polygons out into a **tube of
segments**, and in the same step can:

- **bevel out**: move the cap out along the polygon normal (Shift);
- **taper**: shrink or grow the cap's outline (Inset), with a curve along the
  length (Profile);
- **twist**: spin the cap, and every segment in proportion, about the tube's
  own axis (Rotation);
- **bend**: curve the tube in an arc (Bend), in a chosen direction around the
  normal (Bend Direction).

Think of it as Polygon Bevel with Segments, where each new ring is placed
along a curved, twisting spine instead of a straight line. Typical uses:
horns, tentacles, handles, pipes, curled leaves, cartoon hair tufts.

Name in Operandi: **Twist Bevel**. Command `poly.twistBevel` (plain name,
not under `operandi.*`, since it's a normal modelling tool). Lives on the
Polygon tab next to Bevel. Polygon mode only.

## 2. Properties (tool panel, top to bottom)

| Row | Type | Default | Range | What it does |
|---|---|---|---|---|
| Shift | distance | 0 m | any; negative pushes inward | Length of the tube, measured **along the spine** (so a bent tube keeps the same length as a straight one). |
| Inset | distance | 0 m | any; negative grows | How far the cap's outline moves inward (taper). Positive = cap smaller, negative = cap bigger. |
| Segments | whole number | 4 **[default]** | 1..256 **[default cap]** | Number of rings between base and cap. 1 = a plain bevel. |
| Group Polygons | checkbox | on | | On: each connected patch of selected polygons makes one tube. Off: each polygon makes its own tube. Same meaning as Bevel's Group Polygons. |
| Rotation | angle | 0° | any (beyond ±360° keeps twisting) | Total twist at the cap. Each ring gets its share (see 3.4). |
| Rotation Axis | dropdown | Normal | Normal, Action Axis **[default list]** | What the twist spins around. Normal = the tube's own spine at that ring. |
| Profile | number, no unit | 0.0 | -1.0..1.0 **[default clamp]** | Bends the taper curve along the length. 0 = straight taper. Negative = pinched waist; positive = bulge (see 3.3). |
| Profile Bias | number, no unit | 0.0 | -1.0..1.0 | Slides the fattest (or thinnest) point of the Profile toward the base (negative) or the cap (positive). No effect when Profile is 0 (see 3.3). |
| Segment Spacing | number, no unit | 0.0 | -2.0..2.0 | Bunches the rings toward the base (negative) or the cap (positive). 0 = evenly spaced (see 3.7). |
| Center Bias | number, no unit | 0.0 | -2.0..2.0 | Bunches the rings toward the middle (positive) or toward both ends (negative). Works together with Segment Spacing (see 3.7). |
| Bend | angle | 0° | -360°..360° **[default clamp]** | How far the tube curls in total. 90° = the cap ends up facing sideways. |
| Bend Direction | angle | 0° | any (wraps at 360°) | Which way it curls, as an angle around the normal (see 3.2). |

Below these, a **Tool States** section (section 7).

The order matters: rows are grouped as size (Shift, Inset, Segments, Group),
twist (Rotation, Rotation Axis), shape (Profile, Profile Bias), ring spacing
(Segment Spacing, Center Bias), bend (Bend, Bend Direction).
Keep the order; panel rows follow the bible's property-panel rules (UIC-100..).

## 3. Geometry

### 3.1 The base frame (per tube)

For each group (or each polygon with Group off):

- **Base centre C**: the average of the group's points.
- **Normal N**: the average of the group's polygon normals, normalised. If it
  comes out zero (a closed shell selected), refuse that group and say so on
  the status line ("Twist Bevel: can't find a direction for a closed
  selection").
- **Side axis S**: the reference for Bend Direction 0°. **[default]** Take
  world +X projected onto the plane at right angles to N; if that's too short
  (N almost along X), use world +Z instead. Normalise. Then rotate S about N
  by Bend Direction to get the **bend direction B**.
- **Up axis U** = N × B (completes a right-handed frame).

### 3.2 The spine

A curve of length L = Shift starting at C and leaving along N.

- Bend = 0: a straight line, P(t) = C + N·L·t, for t from 0 to 1.
- Bend = θ (radians, not 0): a circular arc in the plane of N and B, turning
  toward B. Radius R = L / θ. At t:
  - angle a = θ·t
  - P(t) = C + N·R·sin(a) + B·R·(1 − cos(a))
  - tangent T(t) = N·cos(a) + B·sin(a)
  The frame at t is the base frame rotated by a about U.
- Negative Bend curls the opposite way (toward −B). It is the same as Bend
  Direction + 180°.
- Negative Shift with Bend: the spine runs backward along −N; the same
  formulas work with L negative.

### 3.3 Taper (Inset and Profile)

Each ring is a copy of the group's **boundary outline**, made smaller or
bigger.

- Ring k (k = 1..Segments) sits at t = space(k / Segments) (3.7; with both
  spacing controls at 0 that's just k / Segments). The cap is ring Segments
  (t = 1).
- Inset at t: `inset(t) = Inset · t − Profile · W · bump(t)`, where W is
  the group's **half width**: the average distance from C to its boundary
  points, measured in the base plane.
  - Profile 0: straight taper from the base outline to the cap.
  - Profile −0.5: the middle ring (t = 0.5) is pulled in by an extra half
    of W, a clear waist. Profile +0.5 pushes it out by the same (bulge).
  - Base (t = 0) and cap (t = 1) are not affected by Profile.
- **bump(t)** is a hump that is 0 at both ends and 1 at its peak. With
  Profile Bias b = 0 the peak is the middle: `bump(t) = 4·t·(1 − t)`.
  With b not 0, the peak moves to `p = 0.5 + 0.45·b` (so p stays between
  0.05 and 0.95): first remap `t' = t^(ln 0.5 / ln p)` (which sends p to 0.5
  and keeps 0 and 1 fixed), then `bump = 4·t'·(1 − t')`.
- Profile only shows if there are rings in between (Segments 2 or more).
- The outline is moved inward by `inset(t)` the **same way Bevel's Inset
  moves it** (edge-offset, corners mitred, Bevel's collapse rules). Reuse that
  code. Negative values grow the outline.
- If an inset collapses a ring past its centre, apply Bevel's collapse
  behaviour (BEHAVIOR-INSET-COLLAPSE in the Polygon Bevel spec).

### 3.4 Twist (Rotation)

- Ring k is spun by `Rotation · t` about the axis set by Rotation Axis,
  through that ring's spine point P(t):
  - **Normal**: the spine tangent T(t). The tube twists like a towel.
  - **Action Axis** **[default list]**: the action axis's main direction,
    through P(t). For tubes that should twist about a fixed world direction.
- The cap therefore ends up turned by the full Rotation.
- Positive Rotation is anticlockwise looking back down the tangent at the
  base (right-hand rule about T).

### 3.5 Placing each ring

For ring k at t:

1. Start from the base outline, as offsets from C (points in the base plane).
2. Inset it by `inset(t)`.
3. With Rotation Axis = Normal: spin the offsets about N by `Rotation · t`
   (twist, done in the flat base plane so it is exact).
4. Rotate the offsets about U by `θ · t` (bend), so the ring's plane is
   square to T(t).
5. Add P(t) to every offset.
6. With Rotation Axis = Action Axis: skip step 3 and instead, after step 5,
   spin the ring about the action axis direction through P(t).

The cap is ring Segments. The **original polygons** become the cap (they
move; they are not copied), exactly like Bevel. Interior points of a grouped
patch move with the cap (same as Bevel with Group on).

### 3.5a Ring spacing is only sampling

Shift, Inset, Profile, Rotation and Bend all describe the **shape** as a
function of t. Segments, Segment Spacing and Center Bias only decide **where
along that shape the rings sit**. So changing spacing never changes the
base, the cap or the overall silhouette; it only moves detail toward where
it's needed (for example, rings bunched at the tight end of a curl). More
Segments just samples the same shape more finely.

### 3.6 New polygons

- Between consecutive rings, one quad per boundary edge, as Bevel makes with
  Segments. Winding faces outward (same as Bevel's walls).
- Wall polygons take the material/part of the polygon they grew from, as
  Bevel does.
- Polygon type follows the source: if the source polygons are SubD
  (Catmull-Clark) the walls are SubD too.
- Shared boundary points between two tubes with Group off: each tube makes its
  own walls (same as Bevel with Group off).
- After the tool: the **cap polygons stay selected**, the walls do not.

### 3.7 Spacing the rings (Segment Spacing, Center Bias)

`space(u)` turns the even ring positions u = k / Segments into the bunched
positions t. It always keeps 0 → 0 and 1 → 1 and never reorders rings.

1. Segment Spacing s: `u1 = 1 − (1 − u)^(2^s)`. Positive s bunches rings
   toward the cap, negative toward the base, 0 leaves them even.
2. Center Bias c, applied to u1: let `d = 2·u1 − 1` (−1..1), then
   `t = 0.5 + 0.5 · sign(d) · |d|^(2^c)`. Positive c bunches rings toward
   the middle, negative toward both ends.

Both at 0 gives t = u. Both can be used at once (spacing first, then centre).

## 4. Handles in the viewport

All handles sit on the **first** (or primary) group's frame. Dragging a
handle changes the property for every group at once. While dragging, the
property's name and value show in the usual readout next to the pointer.
Handle colours and sizes follow Operandi's handle conventions (viewport
handles spec); the list below is what each one does, not how it looks.

| Handle | Where | Drag does |
|---|---|---|
| Shift arrow | at the cap, along the tangent T(1) | Shift (length along the spine). |
| Inset handle | at the cap, pointing out sideways in the cap plane | Inset. Dragging out = negative (grow), in = positive (shrink). |
| Twist ring | around the cap | Rotation (spins the cap; readout in degrees). |
| Bend arrow | at the base, pointing along B | Bend. Dragging toward B curls more; away curls back. Readout in degrees. |
| Bend Direction ring | around the base, in the base plane | Bend Direction (swings which way the tube curls). |

- The handles ride on the current shape: when the tube bends, the cap
  handles move and tilt with the cap.
- Segments, Profile, Profile Bias, Segment Spacing and Center Bias have no
  handle; panel only. **[default]**
- Click-drag on empty space (not on a handle): changes Shift, like Bevel's
  plain drag. **[default]**
- Live editing follows Bevel's rules (tool activation spec TA-01..07): the
  first drag makes a twist bevel, a later plain drag or a panel edit adjusts
  the **same** one, and Shift+click or Shift+drag starts a new one on the
  current cap. One undo step per twist bevel after the tool drops.

## 5. The shared tool pipe

Twist Bevel is the "tool" layer at the end of the shared pipe (action
centre, falloff, symmetry, snapping, then the tool).

- **Action centre / axis**: not used to build the frame; each tube uses its
  own base frame (3.1), like Bevel. Used only by Rotation Axis = Action Axis.
- **Falloff**: weight is read at each group's base centre C and multiplies
  Shift, Inset, Rotation and Bend for that group (so a linear falloff over a
  row of polygons gives a row of tubes from short and straight to long and
  curled). Segments, Profile and Bend Direction are not weighted.
  **[default]**
- **Symmetry**: creation path of the shared symmetry service. A group and its
  mirror partner get mirrored results: Shift, Inset, Profile, Segments the
  same; Bend Direction mirrored across the symmetry plane; Rotation reversed
  in sign so the twist is a mirror image. A group that **crosses** the
  symmetry plane is treated as one group and keeps its own frame; its Bend
  Direction is snapped to lie in the symmetry plane so the result stays
  symmetric, and the status line says so. **[default]**
- **Snapping**: only the Shift arrow snaps. With snapping on, dragging Shift
  can land the cap centre P(1) on a snap target (cage positions, per the
  snapping rule). Bend, twist and inset handles use the usual angle/value
  increments if increment snapping is on.
- **SubD surface picking**: selecting polygons before or during the tool goes
  through the shared surface-pick path (`item_pick_candidates` /
  `shown_points`), never the raw cage. The geometry itself (3.1–3.6) is
  computed on the cage.
- **Constraints** (Constrain to Background): not applied. **[default]**

## 6. Odd input

- Nothing selected: same as Bevel (PB-O-01: acts on all polygons). **[default,
  following Bevel]**
- Shift = 0 and Inset = 0: no change, nothing is made (same as Bevel).
- Shift = 0 with Bend not 0: the spine has no length, so Bend has no effect.
- Segments = 1 with Bend: one ring at the arc's end; the walls are straight
  quads (a coarse bend). That's correct, not an error.
- Bend of 360° or more: the tube can run into itself. Allowed; no
  self-collision check.
- Non-planar group: use the averaged frame (3.1); the outline is projected
  for insetting the same way Bevel does it.

## 7. Tool States (tool presets)

A small section under the properties that saves and recalls named sets of
this tool's values. It matches the value-presets pattern (UIC-55) but for the
whole tool:

- **States** dropdown: "(none)" plus saved names. Picking one loads all its
  values into the panel (and updates a live result).
- **Save**: asks for a name (prefilled "Twist Bevel 1", "… 2", following the
  app's name-numbering style), saves the current values.
- **Delete**: removes the chosen state. Greyed when "(none)" is chosen.
- **Rename**: renames the chosen state. Greyed when "(none)" is chosen.
- **Reset**: puts every value back to the factory default in section 2.
- States are saved in user config and survive restarts.

Build note: Operandi has no tool presets yet (preset-browser gap P3). Build
this as a **shared** "Tool States" section any tool can show, not something
private to Twist Bevel. Twist Bevel is just the first user. If that's too big
for this build, ship Twist Bevel with Reset only and leave the rest for the
shared piece (TB-Q09).

## 8. Reuse from existing Operandi tools

- **Polygon Bevel**: group finding (Group Polygons), inset/offset of the
  outline including collapse rules, wall making with Segments, material and
  polygon-type inheritance, selection after, live re-drag and undo rules.
  Twist Bevel = Bevel's ring builder with a different "place this ring"
  step (3.5). The cleanest build is to pull Bevel's "make N rings and stitch
  them" into a shared function that takes a per-ring transform, and have
  Bevel pass a straight-line transform.
- **Polygon Extrude**: nothing extra beyond Bevel; Extrude is Bevel with
  Inset 0.
- **Bend / Twist deformers** (if present): not reused. They bend existing
  points about a fixed axis; here each ring is placed exactly on an arc,
  which is simpler and keeps ring sizes exact.
- **Handles**: reuse the existing arrow, ring and readout handle pieces.

## 9. Tests the builder should write (minimum)

1. Unit cube, top face, Shift 1, Segments 4, all else 0: four rings 0.25
   apart straight up, same size as the face; 16 new quads; top face selected.
2. Same with Inset 0.25 (on a face of side 1): cap side 0.5, ring k side
   1 − 0.5·k/4.
3. Same with Bend 90°, Bend Direction 0: cap centre at C + N·R + B·R with
   R = 1/(π/2); cap normal equals B.
4. Same with Rotation 90°: cap is the top face turned 90° about N; ring k
   turned 22.5°·k.
5. Profile −0.5 vs +0.5: middle ring smaller vs bigger than the straight
   taper; base and cap unchanged.
6. Group off on two adjoining faces: two separate tubes.
7. Symmetry X on, one face at +X: mirror face at −X gets a mirror-image tube
   (bend mirrored, twist reversed).
8. Linear falloff across a row of 5 faces: Shift scales by each face's
   weight.
9. SubD cube: walls come out SubD; picking the face uses the surface.
10. Shift 0, Inset 0: mesh unchanged.
11. Profile 0.5, Profile Bias +1: the widest ring sits near t = 0.95 (toward
    the cap); base and cap unchanged.
12. Shift 1, Segments 4, Segment Spacing +1: rings at t = 1 − (1 − k/4)^2,
    i.e. 0.4375, 0.75, 0.9375, 1. Spacing never moves base or cap.
13. Center Bias −1 on the same: rings bunched toward both ends,
    symmetric about the middle.

## 10. Credit

Jason wants the original idea credited. The builder adds this line to the
app's credits/acknowledgements (not to the tool panel):

> Twist Bevel: idea inspired by the "Twist Bevel" tool for Modo by FUSION.
> Profile Bias, Segment Spacing and Center Bias: ideas from Modest, a
> Blender add-on by Christian.

TB-Q10 tracks both authors' preferred names and links. Until it's answered, use
the line as written. (The existing "Not affiliated with or endorsed by
Foundry" notice already covers the Modo mention.)

## 11. Open questions (each has the default used above)

- **TB-Q01** Segments default: 4. Range cap 256.
- **TB-Q02** Rotation Axis choices other than Normal: we offer Normal and
  Action Axis. The recording only shows Normal.
- **TB-Q03** Profile formula and range: our formula in 3.3 and a −1..1 clamp.
  Matches the recording's look (negative pinches the middle) but not
  measured.
- **TB-Q04** What Bend Direction 0° points at: we use world +X flattened onto
  the face (else +Z). Could instead follow the face's first or longest edge.
- **TB-Q05** Falloff: weights Shift, Inset, Rotation, Bend per group.
- **TB-Q06** Symmetry for a group crossing the plane: Bend Direction locked
  into the plane.
- **TB-Q07** Bend clamp −360..360°.
- **TB-Q08** Empty-space drag = Shift; no Segments/Profile handle.
- **TB-Q09** Tool States as a shared piece for every tool, or Reset-only for
  now.
- **TB-Q10** Credit: the author's preferred name and link (Jason to supply
  where he got the tool).
- **TB-Q12** Profile Bias formula (peak at 0.5 + 0.45·b) and the spacing
  formulas in 3.7 are ours; the source only gave names, ranges and intent.
- **TB-Q11** Whether the original tool keeps the twist exact through a bend
  (twist about the moving tangent, as specced) or about the original normal.
  We chose the moving tangent; it looks right in the recording.

## 12. Later ideas (not in this build)

From the Modest write-up, worth a later stage. Each one needs checking
against the bible before it's built:

- **Stages / chains**: Shift+click already starts a new twist bevel on the
  cap (section 4). Later: remember the chain of stages in one session, step
  back into an earlier stage, remove the latest whole stage, and a "repeat
  last stage" command that copies its settings.
- **Span editing**: pick several stages at once and adjust their Profile,
  bias or twist together, with twist shared out by segment count; and a
  "smooth across stages" mode that treats the stage joins as a control cage
  for one flowing shape.
- **All-stages adjust**: change every stage in the chain by the same amount.
- **Roll back one stroke**: undo just the last drag inside the tool. Fits
  the mid-tool undo design rather than a new key.

Clashes with Operandi, so NOT copied:

- **Hold-a-key-and-move-the-mouse controls** (one letter per property,
  double-tap to reset), **right-drag for segments** and **hold Space for
  all stages**. Operandi drives tools with handles plus the panel, and those
  keys and the right button already have jobs (shortcuts, menus). Any
  keyboard scrub mode would be a bible deviation and needs Jason's OK.
- **Drag direction picks the property** (sideways = inset, up = shift).
  Operandi uses explicit handles; empty-space drag stays Shift, like Bevel.
- **Enter / right-click to apply**: Operandi's tools drop the usual way
  (tool activation rules).
