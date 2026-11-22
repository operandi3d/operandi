# Absolute Scaling, Scale Absolute, Size Absolute — behavior spec v1

Part 1 of the Snaps and Precision panel spec (2026-11-10). Part 2 (the rest
of the panel) follows in `SPEC-SNAPS-PRECISION-PANEL.md`.

Provenance: observed in the reference app (17.1v1) through scripted probes on
2026-11-10 (45 cases); the reference help pages "Absolute Scaling" and "Snaps
and Precision Palette" for names and intent; the reference app's plain-text
UI and preset configuration files for layout, labels, defaults and command
names. REA-assisted: no.

Tags: **[obs]** observed by probe (case id in brackets), **[docs]** from the
help, **[cfg]** from the plain-text UI/preset config, **[open]** not
observed, see Open questions.

---

## A. Absolute Scaling (window)

### A1. Summary

A small floating window for resizing a selection to exact numbers instead of
by dragging. You first read the selection's current size into the fields
(**Grab Size**), type the size you want, and press one of three scale
buttons. **Uniform Scale** scales all three axes by the same factor so that
one axis reaches the typed length. **Explicit Scale** scales each axis
separately so the selection's bounding box becomes exactly X × Y × Z.
**Reference Scale** measures the factors on the selection but applies them to
the whole mesh (e.g. scale a whole car so that its cockpit has a known
size). [docs] It moves points only; topology never changes. [obs, all cases]

### A2. Opening it

- Snaps and Precision panel → **Commands** section → **Absolute Scaling**
  button. [docs][cfg]
- Also from the Scale tool button's hold-down popup menu. [docs]
- The button toggles: press opens the window, press again closes it. It is a
  floating palette titled "Absolute", about 250 × 350 px, that remembers it
  was open (persistent). [cfg]
- A modifier-click on the same button opens it as a smaller popover (about
  150 × 250 px) pointing at the button instead of a floating palette. [cfg]
  Which modifier: [open Q1].

### A3. Layout and fields (top to bottom) [cfg]

| Row | Control | Kind | Starting value | Notes |
|---|---|---|---|---|
| 1 | **Grab Size** | button | — | tooltip: fills X Y Z with the selection's bounding box |
| Uniform section | **Uniform Scale** | button | — | |
| | **Uniform** | distance field | 0 | |
| | **Uniform Axis** | popup: Longest Axis, X Axis, Y Axis, Z Axis | Longest Axis | label reads "Uniform Axis" on screen [Jason's screenshot]; the form config calls it "Axis" |
| | **Orient To Selection** | checkbox | off | |
| Explicit section | **Explicit Scale** | button | — | |
| | **Reference Scale** | button | — | |
| | **X**, **Y**, **Z** | three distance fields on one row | 0, 0, 0 | |
| Scaling Center section | **X Center**, **Y Center**, **Z Center** | three popups on one row | Center | options per axis: Center, High, Low (in that order); menu items read "Center X / High X / Low X" etc. |

- The field values are shared by all four buttons and stay between uses
  within a session; they are not saved between sessions (reset to the
  starting values on restart). [cfg]
- Every scale button refreshes the fields afterwards (see A6), so the window
  always shows the current size after a scale. [obs]

### A4. What is measured (the working set and its box)

- **Working set** = the vertices the selection touches in the current
  component mode: selected polygons → their vertices; selected edges → their
  end points; selected vertices. [obs 02, 12, 16, 25]
- Nothing selected in a component mode → the whole mesh. [obs 03]
- Item mode with a mesh item selected → the whole mesh. [obs 24]
- Several mesh items selected → one combined box over all of them; every item
  scales about that shared box (not each item about its own). [obs 38]
- Sizes and positions are in the mesh's **own local coordinates**: the item's
  position and scale are ignored (a box 2 long in its mesh reads 2 even when
  the item is scaled ×2; Explicit 4 makes it 4 in local units). [obs 19, 20]
- **The box** is the axis-aligned bounding box of the working set, unless
  **Orient To Selection** is on (A8).

### A5. Scaling center (per axis, independent) [obs 06, 07, 09, 10, 27, 35]

For each axis separately, the point the scale is about:

- **Center**: the middle of the box on that axis, i.e. (min + max) / 2. It is
  **not** the average of the vertex positions, although the help describes it
  that way: an unevenly spaced strip from x = 0 to 2 with most points near 0
  scaled about x = 1. [obs 27]
- **Low**: the box's minimum on that axis (that side stays put).
- **High**: the box's maximum on that axis.
- For Uniform and Explicit the box is the working set's; for Reference it is
  the **whole mesh's** box (A7.3).

### A6. Buttons

#### A6.1 Grab Size [obs 01, 02, 03, 37]

- Sets **X**, **Y**, **Z** to the box's size on each axis.
- Sets **Uniform** to the box size along **Axis** (Longest Axis → the largest
  of the three).
- Changes no geometry.
- Example: box 2 × 1 × 0.5 → X 2, Y 1, Z 0.5, Uniform 2. Top face only (flat,
  2 × 0 × 0.5) → X 2, Y 0, Z 0.5, Uniform 2.

#### A6.2 Uniform Scale [obs 04–07, 11, 17, 22, 25, 26, 31, 34]

- Factor f = **Uniform** ÷ (box size along **Axis**; Longest Axis → the
  largest size). All three axes scale by f about the scaling center.
- Examples on box x 1…3, y 0…1, z 0…0.5 (2 × 1 × 0.5):
  - Uniform 4, Longest → ×2 → x 0…4, y −0.5…1.5, z −0.25…0.75. [04]
  - Uniform 3, Y Axis → ×3 → 6 × 3 × 1.5. [05]
  - Uniform 1, Z Axis, all Low → ×2 growing from the low corner → x 1…5,
    y 0…2, z 0…1. [06]
  - Uniform 1, X Axis → ×0.5 (shrinks). [26]
- **Uniform 0** collapses the working set to the scaling-center point. [22]
- **Flat selections collapse (observed reference behavior):** when the box
  has zero size on any axis (a single flat face, a planar mesh, one straight
  edge), the reference collapses the whole working set to the
  scaling-center point, on all axes. [11, 17, 25, 34]
  **Operandi [DECIDED, Jason 2026-11-10]: skip zero-size axes** — compute f
  from the chosen axis as usual and leave any axis whose current size is 0
  unchanged (a flat face scales in its plane and stays flat). If the chosen
  axis itself has size 0 (or Longest finds all zero), do nothing and say so
  in the status line. Keep cases 11/17/25/34 as reference-behavior notes;
  their expected points are the reference's, not Operandi's.
- Afterwards the fields are refreshed as by Grab Size (Uniform therefore shows
  the typed value, X Y Z the new sizes).

#### A6.3 Explicit Scale [obs 08–10, 12, 15, 16, 21, 28, 33, 36]

- For each axis: factor = target (**X**/**Y**/**Z**) ÷ current box size on
  that axis, about that axis's scaling center. Axes are independent.
- Box 2 × 1 × 0.5, targets 1, 2, 3, Center → x 1.5…2.5, y −0.5…1.5,
  z −1.25…1.75. [08]
- **Target 0 leaves that axis unchanged** (it does not flatten). Targets
  0, 1, 0.5 on a 2 × 1 × 0.5 box change nothing; 4, 0, 1 scales X and Z and
  leaves Y. [15, 33, 28]
- **An axis whose current size is 0 is left unchanged** whatever the target
  (top face, target Y 5 → Y unchanged). [12]
- **Negative target mirrors:** target X −4 on a box 2 wide → factor −2 about
  the center (x 1…3 → points at 1 go to 4, at 3 go to 0); X then reads 4
  afterwards. [21]
- With **Orient To Selection** on, sizes are measured in the oriented box
  (A8): a box 2 × 1 × 1 turned 45° about Z with targets 2, 1, 1 does not
  move (its axis-aligned box would be 2.12 × 2.12 × 1). [36]
- Afterwards the fields are refreshed: X Y Z = new sizes, Uniform = new size
  along Axis (Longest → largest; Y Axis → the Y size). [08, 23]

#### A6.4 Reference Scale [obs 13, 29, 30, 35]

1. Factors per axis are measured on the **selection's** box exactly as in
   Explicit Scale: target ÷ selection size.
2. They are applied to **every vertex of the mesh**, including parts not
   connected to the selection (a separate box in the same mesh scaled too).
   [13, 29, 30] (The help says "connected geometry"; the observed scope is
   the whole mesh layer.)
3. The scaling center is measured on the **whole mesh's** box: Center = its
   middle, Low/High = its sides. [30, 35]
- Example: mesh = box A (x 0…2, y 0…1, z 0…0.5) + separate box B
  (x 5…6, y 0…1, z 0…1). All of A selected, targets 4, 2, 1 (×2 on every
  axis), Center → whole mesh ×2 about (3, 0.5, 0.5): A → x −3…1, B → x 7…9.
  With all Low → ×2 about (0, 0, 0): B's x 5 → 10. [30, 35]
- **Zero-size selection axis collapses the whole mesh on that axis**, to the
  whole mesh's center on that axis, whatever the target (A's flat top face
  selected, target Y 0 or 5 → every point of A and B gets y = 0.5). [13, 29]
  Differs from Explicit, which leaves such an axis alone.
  **Operandi [DECIDED, Jason 2026-11-10]: skip that axis** (whole mesh
  unchanged on it), like Explicit. Cases 13/29 record the reference's
  collapse, not Operandi's expected result.
- Afterwards the fields show the **selection's** new box (e.g. 4, 0, 1 for
  the top face). [13]

### A7. Selection and state after [obs]

- Geometry topology is never changed; only point positions. [all]
- The original component selection stays. In addition, the working set's
  vertices end up selected in vertex selection (selecting a face and pressing
  any button, Grab Size included, leaves its corner vertices vertex-selected).
  [01, 12, 25] Which selection mode is active afterwards: [open Q3].
- No message or refusal was seen in any case, including empty selections,
  zero and negative values. [all]

### A8. Orient To Selection [obs 18, 31, 32, 36, 37]

- Off: the box is axis-aligned in local space.
- On: the box is fitted to the selection's own orientation. For a 2 × 1 × 1
  box turned 45° about Z, Grab Size reads X 2, Y 1, Z 1 (instead of
  2.12, 2.12, 1), and Uniform 4 (Longest) scales by exactly ×2 about the box
  center. [37, 32] Without it the same Uniform 4 scales by 4 ÷ 2.12 =
  1.8856. [31]
- After an oriented scale, X is the oriented box's longest side. [32] How
  the oriented frame is chosen in general (which side becomes X/Y/Z, ties,
  irregular shapes): [open Q4].
- On a flat square turned 45° (zero thickness) with Orient on, Uniform did
  nothing and Grab Size read the axis-aligned sizes (1.414, 1.414, 0). [18]
- The help's intended use: scaling edges or faces that aren't aligned to a
  world axis without skewing them. [docs]

### A9. Undo [open Q5]

Not observable by script. Expected: each button press is one undo step
restoring all point positions; Grab Size changes no geometry.

---

## B. Scale Absolute (preset tool) [cfg][docs][obs]

- Panel button **Scale Absolute**. It turns on a preset that combines: the
  **Scale** transform with **Absolute Handle** on (base length 1.2 on each
  axis), **Geometry** snapping on (element snap), the **Element** action
  center, and the automatic action axis. [cfg]
- Use: a two-step drag. First drag the **absolute (base) handles** to set
  the starting length (they snap to geometry, e.g. one end of a known
  feature). Then drag the **scale handles**; the scale factor is the new
  length ÷ the base length, so you can snap the second handle to another
  component and get an exact relative size. [docs][cfg tooltips: "drag here
  to define base size along X/Y/Z"]
- Scripted check: the base length values do not change the result of typed
  values; Scale X 2 gives ×2 with Absolute Handle on or off. The base only
  changes how a drag maps to the factor. [obs scaleabs_01, scaleabs_02]
- Absolute Handle in the Scale tool (help): it lets you resize the handle
  itself without changing the transform amount. [docs]
- Exact handle layout, snapping targets and the drag-to-factor mapping are
  pointer behavior: [open Q6].

## C. Size Absolute (preset tool) [cfg][obs]

- Panel button **Size Absolute**. Preset = the **Size** tool + **Geometry**
  snapping + **Element** action center. [cfg] Help says only that it opens
  an absolute sizing option. [docs]
- The Size tool has six values: **Absolute X/Y/Z** (the base size, start 1)
  and **Relative X/Y/Z** (the size you want, start 1). Per axis the factor is
  Relative ÷ Absolute, about the action center. [obs size_01–04]
  - All 1 → no change. [size_01]
  - Absolute X 2, Relative X 4 → ×2 on X only. [size_02]
  - Absolute X 1, Relative X 3 → ×3 on X. [size_03]
  - Absolute 2, 1, 0.5, Relative 4, 3, 1 → ×2, ×3, ×2. [size_04]
- Handles: one resize handle per axis and one base-size handle per axis
  (tooltips: "drag to resize positions along X" and "drag to define base
  size along X"). [cfg] Interactive use like B: snap the base handle to
  geometry to set the base, then the resize handle to set the new size.
  [open Q6]
- Note the naming: "Absolute" is the **base**, "Relative" the **target**.
  Keep these names for script compatibility. [cfg][obs]
- The probes ran with the action center at the origin; with the preset's
  Element action center the clicked element is the center [open Q7].

---

## D. Command names (for the command registry) [cfg][live command list]

Interface names only; arguments listed as `name: type`.

| UI | Invocation |
|---|---|
| Absolute Scaling button (panel) | toggles the "absolute_layout" floating window; the window runs the command below |
| Grab Size | `scale.absolute mode:0` |
| Uniform Scale | `scale.absolute mode:1` |
| Explicit Scale | `scale.absolute mode:2` |
| Reference Scale | `scale.absolute mode:3` |
| Scale Absolute button | `tool.set ScaleAbsolute 1 snap:true` |
| Size Absolute button | `tool.set SizeAbsolute 1 snap:true` |

- `scale.absolute` arguments: `mode: integer` (required; 0 Grab, 1 Uniform,
  2 Explicit, 3 Reference; option names grab, uniform, explicit,
  reference), and optional `absolute_size_uniform: distance`,
  `absolute_size_uniform_pref: string`, `orient_to_selection: boolean`,
  `absolute_size_x/y/z: distance`, `absolute_size_center_x/y/z: integer`.
  The window's buttons pass only `mode`; the values come from the window's
  stored fields. Whether the optional arguments override those fields:
  [open Q8].
- The window's stored fields (user values; all transient): `absolutescale.mode`
  (grab; uniform; explicit; reference), `absolutescale.uniform` (distance, 0),
  `absolutescale.uniformaxis` (longestAxis; x; y; z — start longestAxis),
  `absolutescale.orient` (boolean, false), `absolutescale.size_x/size_y/size_z`
  (distance, 0), `absolutescale.center_x` (center; highX; lowX — start
  center), likewise `center_y` (center; highY; lowY) and `center_z`
  (center; highZ; lowZ).
- Size tool: `xfrm.size`, attributes `absX/absY/absZ` (Absolute X/Y/Z,
  distance, 1), `relX/relY/relZ` (Relative X/Y/Z, distance, 1), `lockUV`
  (Slip UVs, boolean), `morph` (integer).
- Scale Absolute uses the Transform tool `xfrm.transform` with `S` on,
  `absHandle` (Absolute Handle, boolean) on, `absX/absY/absZ` (Absolute
  Size X/Y/Z, distance; 1.2 in this preset), scale amounts `SX/SY/SZ`
  (percent, 1 = 100%).
- Preset display names: ScaleAbsolute "Scale Absolute Tool", SizeAbsolute
  "Size Absolute Tool".

## E. Acceptance cases

`cases.json` (44 entries). Absolute Scaling: `AS-01` … `AS-38` (input mesh,
selection, the button and every field value, expected points and the fields
afterwards). Size and Scale Absolute (typed values, center at the origin):
`SZ-size_01` … `SZ-size_04`, `SZ-scaleabs_01`, `SZ-scaleabs_02`.

Key ones: 04 (Uniform basics), 06/07 (Low/High/mixed centers), 08
(Explicit), 15/28/33 (target 0 leaves axis), 21 (negative mirrors), 27
(center = box middle), 30/35 (Reference scope and center), 13/29 (Reference
flat collapse), 11/34 (Uniform flat collapse), 19/20 (local space), 32/37
(Orient), 38 (two items share one box).

## F. Open questions

- Q1. Which modifier opens Absolute Scaling as a popover instead of a
  floating window? (hands-on)
- Q2. Answered (Jason, 2026-11-10): skip zero-size axes in Uniform and
  Reference, as Explicit does; revisit after real use.
- Q3. Which selection mode is active after a button press (the vertex
  selection is added; does the mode switch)?
- Q4. How Orient To Selection picks the oriented box and assigns X/Y/Z in
  general (only a turned box was measured).
- Q5. Undo granularity (expected one step per press).
- Q6. Handle layout and drag behavior of Scale Absolute and Size Absolute,
  including snapping (hands-on).
- Q7. Size/Scale Absolute center with the Element action center (the
  clicked element) — hands-on.
- Q8. Do `scale.absolute`'s optional arguments override the window fields
  when given?
- Q9. Hidden or locked geometry: is it included in the working set or the
  whole-mesh box for Reference? Not probed.
