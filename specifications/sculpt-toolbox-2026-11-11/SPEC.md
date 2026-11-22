# Sculpt toolbox (remaining brushes) — behaviour spec v1.1

Builder ref: **SCB-Q01**. Extends the sculpt foundation (SC-01..123: Smooth,
Push, Inflate, Move, tips, stroke machine, modifiers, symmetry, locks).
Rules here are **SCT-nn**; open questions are **SCT-Qnn**.

Provenance: reference user guide 17.1 (sculpting chapter: brush tools,
masks, locks, utilities) for names, intent and modifier keys; the
reference's plain-text UI configuration for toolbox order, labels and the
exact field list of every Tool Properties form; starting values from the
read-only name pass of 2026-11-06 (same pass as the command-name registry),
cross-checked against the reference's saved tool cache (only Push had ever
been used, and its cached values match). No new probes. No geometry was
measured, so every "how far / what shape" detail below is either a
documented intent or a marked **build guess**. REA-assisted: no.

Exact tool and argument names are in `NAMES.txt` (registry addendum, same
format as the command-name registry). This spec uses UI labels only.

---

## 1. What the toolbox holds

**SCT-01 Toolbox order.** The reference's Sculpt Tools panel lays the
brushes out two per row, in this order (Operandi already has the four
marked ✓):

| Row | Left | Right |
|---|---|---|
| 1 | Push ✓ | Inflate ✓ |
| 2 | Fold | Clay |
| 3 | Smooth ✓ | Smudge |
| 4 | Carve | Scar |
| 5 | Flatten | Tangent Pinch / Center Pinch (one tool-choice button, see SCT-02) |
| 6 | Emboss | Attenuate |
| 7 | Spin | Move ✓ |

Below the brushes, in order: a **Mask** section (Paint, Smooth, Scale Down,
then Erase, Set Value; Enable Mask, Invert Mask, Mask Opacity; Clear Mask;
Bake Falloff), a **Brush** section (tips: Smooth, Hard, Procedural,
Texture, Sphere), Nozzle, Inks, Settings, and a Background Constraint tool
button. Lock Borders / Lock Corners sit at the bottom of every brush form
(foundation SC-72/73).

**SCT-02 Pinch pair.** Tangent Pinch and Center Pinch share one slot as a
tool-choice button: one face shows the last-picked pinch, and the other is
reached from it. *Build guess:* a split button that remembers the last
choice, per UIC rules for grouped tools (SCT-Q01).

**SCT-03 Menu order.** The same tools also appear in a Sculpt Tools menu
(under the Texture menu in the reference) in this order: Push, Smooth,
Carve, Flatten, Fold, Inflate, Smudge, Move, Tangent Pinch, Center Pinch,
Scar, Emboss, Attenuate, Spin; then a Mask group: Paint, Smooth,
Scale Down, Erase, Set Value.

**SCT-04 Names.** Toolbox label → form title → undo label:

| Toolbox label | Form section title | Hover tooltip (own words) |
|---|---|---|
| Fold | Sculpt: Fold | Folds the surface into a hill and a valley. |
| Clay | Sculpt: Clay | Builds the surface up to a set level. |
| Smudge | Sculpt: Smudge | Drags points along with the brush, smearing the surface. |
| Carve | Sculpt: Carve | Pushes points sharply inward. |
| Scar | Sculpt: Scar | Cuts a pinched gash into the surface. |
| Flatten | Sculpt: Flatten | Flattens the surface to a plane. |
| Tangent Pinch | Sculpt: Tangent Pinch | Pinches points toward the stroke line, sliding along the surface. |
| Center Pinch | Sculpt: Center Pinch | Pinches points toward the brush centre, sliding along the surface. |
| Emboss | Sculpt: Emboss | Moves the area under the brush in or out along its normals by a set amount. |
| Attenuate | Sculpt: Attenuate | Fades stored sculpt detail back toward the unsculpted shape. |
| Spin | Sculpt: Spin | Swirls points around the brush centre. |

Undo label follows the foundation: "<Toolbox label> Stroke"; Shift-smoothing
strokes are "Smooth Stroke" (SCQ-08). The reference's own long name for
Attenuate is "Sculpt Erase"; the toolbox and form say Attenuate. Use
Attenuate in the UI.

**SCT-05 Shortcuts.** None of these tools has a default key. Hover text
shows "Unassigned", as the foundation does.

---

## 2. Shared fields (meaning is the foundation's)

These fields already exist in the foundation and mean the same thing on the
new brushes. Values are listed per tool in §3.

| Field | Kind | Choices / unit |
|---|---|---|
| Sculpt Mode | choice | Automatic, Mesh, Image Map (Fast), Image Map (Details), Curves |
| Offset Mode | choice | Adaptive, Absolute |
| Offset Distance | distance | greyed unless Offset Mode = Absolute |
| Offset Amount | percent | greyed when Offset Mode = Absolute; middle-drag edits it |
| Smooth Amount | percent | strength of Shift smoothing |
| Smooth Mode | choice | Shift Key, Low Pressure, Uniform |
| Normal Mode | choice | Brush Center, Vertex, Average, Screen, Mouse Down, X, Y, Z |
| Density Mode | choice | Additive, Attenuated, Maximum Stroke, Maximum Map |
| Interpolation Step | percent of tip size | dab spacing |
| Continuous Stroke | checkbox | |
| Curve Constraint | choice | None, Background 2D, Background 3D |
| Set Color / Color / Alpha | checkbox / colour / percent | image painting only |

**SCT-06 Missing field = fixed behaviour.** A field a brush's form does not
show cannot be changed for that brush. Where the hidden value matters it is
given in §3 as "(fixed)".

**SCT-07 Image-only fields.** Sculpt Mode choices other than Mesh/Automatic,
Maximum Map, Set Color/Color/Alpha, Paint Mode, Soft Filter and Rigid UVs
act only on image, vertex-map or multiresolution sculpting. Operandi
sculpts meshes only for now: show them greyed with a status note, as the
foundation does for its image-only fields (SCT-Q02).

**SCT-08 Ranges.** The guide gives no numeric ranges. *Build guess:*
percent fields 0–100 % with typing past 100 % allowed for Offset Amount and
Level; Offset Distance ≥ 0; Interpolation Step 1–200 % (the guide shows a
200 % example). SCT-Q03.

**SCT-09 Interpolation Step default.** The starting value read from the
reference is **5 %** for every brush. The guide's prose says 8 % and a
picture caption says 10 %; go with the observed 5 % (SCT-Q04).

---

## 3. The brushes

Values are starting values. "Ctrl" and "Shift" are the stroke modifiers;
Alt stays navigation (foundation). Shift smooths on every brush that shows
Smooth Amount (and on those that don't, see each entry).

### 3.1 Fold — SCT-10..13

**SCT-10 Effect.** Makes a hill-and-valley fold: points near the centre of
the brush go **in** (against the normal), points near the rim come **out**.
The tip's profile shapes the fold strongly.
**SCT-11 Ctrl** reverses it: centre out, rim in. **Shift** smooths.
**SCT-12 Form** (in order): Sculpt Mode (Automatic) · divider · Offset Mode
(Adaptive), Offset Distance (0.1 m), Offset Amount (**25 %**), Smooth Amount
(50 %), Smooth Mode (Shift Key) · divider · Interpolation Step (5 %),
Continuous Stroke (off), Curve Constraint (None) · divider · Lock Borders,
Lock Corners. Fixed: Normal Mode Average, Density Additive.
**SCT-13 Build guess for the profile.** Signed weight: inward at the centre,
crossing zero part-way out, outward toward the rim, zero at the edge. The
crossover radius is not documented (SCT-Q05).

### 3.2 Clay — SCT-14..18

**SCT-14 Effect.** Raises the surface along the normal up to a **plane**
that sits Level above the brush plane; once a point reaches it, it stays
there (no further rise however long you brush). Level is a percent of the
brush **radius**, so a larger brush builds higher. At Level 0 % nothing
happens, because all points already lie on the brush plane.
**SCT-15** The plane's direction follows Normal Mode. The guide recommends a
small Interpolation Step for clean results.
**SCT-16 Ctrl** — not documented. *Build guess:* the plane sits Level
**below** and points above it are pushed down to it (SCT-Q06). **Shift**
smooths.
**SCT-17 Form:** Offset Amount (**50 %**), Level (**10 %**), Smooth Amount
(50 %), Smooth Mode (Shift Key), Normal Mode (**Average**), Density Mode
(**Attenuated**) · divider · Interpolation Step (5 %), Continuous Stroke
(off), Curve Constraint (None) · divider · locks. No Sculpt Mode, Offset
Mode or Offset Distance row (Clay is always adaptive, mesh or auto).
**SCT-18** The guide says Clay takes its normal from the brush centre "by
default", but the observed starting Normal Mode is Average. Use Average
(SCT-Q07).

### 3.3 Smudge — SCT-19..21

**SCT-19 Effect.** Points under the brush are carried along in the stroke's
direction for a short while, then let go and stay where they were dropped as
the brush moves on. Good for organic smears.
**SCT-20 Ctrl** slides the surface the opposite way to the stroke.
**Shift** smooths.
**SCT-21 Form:** Sculpt Mode (Automatic) · divider · Offset Amount
(**50 %**), Smooth Amount (50 %), Smooth Mode (Shift Key) · divider · locks.
No stroke fields: Smudge follows the pointer continuously (fixed:
Interpolation Step 5 %, Density Additive). *Build guess:* each frame, a
point moves by the pointer's travel since the last frame × weight × Offset
Amount, along the surface (SCT-Q08).

### 3.4 Carve — SCT-22..24

**SCT-22 Effect.** Pushes the surface **inward** to cut a valley, with a
fixed maximum depth per stroke (so going back and forth in one stroke does
not dig deeper). For scoring and scratches; a Sharp tip shape makes
wrinkles.
**SCT-23 Ctrl** raises a ridge instead. **Shift** smooths.
**SCT-24 Form:** same rows as Push: Sculpt Mode (Automatic) · Offset Mode
(Adaptive), Offset Distance (0.1 m), Offset Amount (**50 %**), Smooth Amount
(50 %), Smooth Mode (Shift Key), Normal Mode (**Average**), Density Mode
(**Attenuated**) · Interpolation Step (5 %), Curve Constraint (None),
Continuous Stroke (off) · Set Color (off), Color, Alpha (100 %) · locks.
(The "per-stroke cap" is the Attenuated density default.)

### 3.5 Scar — SCT-25..27

**SCT-25 Effect.** Pinches points in toward the stroke line **and** presses
them in at the same time, leaving a gash. Useful for wrinkles, especially
followed by smoothing.
**SCT-26 Ctrl** pushes outward, a raised welt instead of a cut. **Shift**
smooths.
**SCT-27 Form:** Offset Amount (**25 %**), Smooth Amount (50 %), Smooth Mode
(Shift Key) · locks. No Sculpt Mode row. Fixed: Normal Mode Average, Density
Additive, Interpolation Step 5 %. *Build guess:* pinch share and depth share
equal (SCT-Q09).

### 3.6 Flatten — SCT-28..32

**SCT-28 Effect.** Draws the surface under the brush toward one plane.
With the default Normal Mode (Brush Center) the plane is re-taken from the
centre of the brush at each dab, so it turns as the brush travels; slow,
deliberate or repeated strokes give a consistent flat.
**SCT-29 Plateau.** With Normal Mode = Mouse Down, the plane is taken once,
where the stroke starts, and the whole stroke flattens to it.
**SCT-30 Inverted Flatten** (checkbox): off = points **above** the plane are
lowered to it; on = points **below** the plane are raised to it. *Build
guess:* with it off, points below the plane stay put (SCT-Q10).
**SCT-31 Ctrl** pushes the surface up instead of flattening. **Shift**
smooths.
**SCT-32 Form:** Sculpt Mode (Automatic) · Inverted Flatten (off), Offset
Mode (Adaptive), Offset Distance (0.1 m), Offset Amount (**50 %**), Smooth
Amount (50 %), Smooth Mode (Shift Key), Normal Mode (**Brush Center**) ·
Interpolation Step (5 %), Continuous Stroke (off), Curve Constraint (None)
· locks. Fixed: Density Additive.

### 3.7 Tangent Pinch — SCT-33..35

**SCT-33 Effect.** Pulls points sideways toward the **line of the stroke**
(the axis across the stroke direction): a vertical stroke pulls in from
left and right. Points slide along the surface. Makes and sharpens grooves
and creases.
**SCT-34 Ctrl** pushes the middle out (spreads away from the line).
**Shift** smooths.
**SCT-35 Form:** Sculpt Mode (Automatic) · Offset Mode (Adaptive), Offset
Distance (0.1 m), Offset Amount (**25 %**), Smooth Amount (50 %), Curve
Constraint (None) · locks. No Smooth Mode row (Shift only). Fixed: Normal
Mode Vertex, Density Additive, Interpolation Step 5 %.

### 3.8 Center Pinch — SCT-36..38

**SCT-36 Effect.** Pulls points toward the **centre of the brush**; they stay
on the original surface as they gather, so it reads as a slide.
**SCT-37 Ctrl** pushes them out from the centre. **Shift** smooths.
**SCT-38 Form:** identical rows and values to Tangent Pinch (Offset Amount
25 %).

### 3.9 Emboss — SCT-39..42

**SCT-39 Not a stroke brush.** On press, the patch under the brush is
captured (which points, and their weights from the tip). Then horizontal
pointer travel sets how far that patch moves along each point's own
normal: right = out, left = in. Releasing ends it as one undo step. The tip
profile modulates the amount, so the Smooth tip gives one dimple or bump.
**SCT-40 Ctrl** reverses the direction (and, with an image tip, inverts the
image). No Shift smoothing is documented.
**SCT-41 Form:** Sculpt Mode (Automatic) · Offset Amount (**25 %**) · locks.
Fixed: Normal Mode Brush Center (observed start). Direction is each
point's own normal, per the guide.
**SCT-42 Build guess:** distance = horizontal pixels × world-per-pixel ×
Offset Amount × weight (SCT-Q11). The guide says Emboss is at its best
with image tips (Texture, made with Geometry to Brush), which Operandi
doesn't have yet.

### 3.10 Attenuate — SCT-43..45

**SCT-43 Effect.** Fades stored sculpting (image displacement, vertex-map
or multiresolution detail) back toward the unsculpted shape, like melting
it away. In the reference it **does nothing to plain mesh sculpting**.
**SCT-44 Shift** smooths. No Ctrl behaviour documented.
**SCT-45 Form:** Sculpt Mode (Automatic) · **Amount** (25 %), **Smooth**
(50 %) (these are the shared Offset Amount / Smooth Amount fields, relabelled
here), Smooth Mode (Shift Key) · locks.
*Recommendation:* show the button, greyed with "Needs a sculpt layer or
multiresolution mesh", until Operandi has sculpt layers (SCT-Q12).

### 3.11 Spin — SCT-46..48

**SCT-46 Effect.** Rotates points around the brush centre in a vortex,
strongest where the tip weight is high. Normal Mode picks the axis: Brush
Center (default) spins around the surface normal at the centre, so points
mostly stay on the surface; Screen spins around the view direction, which
can fold the surface over itself for stronger swirls.
**SCT-47 Ctrl** spins the other way. **Shift** smooths.
**SCT-48 Form:** Sculpt Mode (Automatic) · Offset Mode (Adaptive), Offset
Distance (0.1 m), Offset Amount (**25 %**), Smooth Amount (50 %), Normal
Mode (**Brush Center**) · locks. Fixed: Density Additive, Interpolation
Step 5 %. *Build guess:* rotation per dab = Offset Amount × weight × a fixed
angle; spin direction without Ctrl is clockwise as seen looking down the
axis (SCT-Q13).

### 3.12 Additions to existing brushes — SCT-49..51

**SCT-49 Move: Move Mode** (choice, after Sculpt Mode): **Normal**
(default) or **Slide**. Normal is the foundation's screen-plane grab.
*Build guess:* Slide keeps moved points on the original surface (SCT-Q14).
Move's Offset Amount starts at **100 %**. Rigid UVs (off) is UV-only.
**SCT-50 Smooth: Soft Filter** (checkbox, off) is image-only (SCT-07).
**SCT-51 Inflate: Paint Mode** row (image painting only, SCT-07).

---

## 4. Masks (reference scope; decision needed)

**SCT-52** Five mask brushes paint a per-point shield that sculpting
respects: masked points move less in proportion to mask strength, and fully
masked points don't move. Masked areas show bright red over the surface.

| Brush | Effect | Ctrl | Field (start) |
|---|---|---|---|
| Paint | adds mask, building up each stroke | erases | Value (100 %) |
| Smooth | blurs the mask's edge | — | Smooth Amount (100 %) |
| Scale Down | weakens the mask each stroke | strengthens | Scale Amount (100 %) |
| Erase | removes mask | acts as Set Value | none |
| Set Value | writes Value exactly, hard-edged | acts as Erase | Value (100 %) |

Controls: Enable Mask (on), Invert Mask (off; flips the effect, not the
red display), Mask Opacity (display only), Clear Mask (button), Bake
Falloff (button: turns the active falloff into mask, or into a
displacement with a Displacement Offset).

**SCT-53 Where masks work.** All the brushes in §3 sculpt plain meshes
(the guide's "Mesh sculpting", which moves the cage points directly).
Masks are narrower: the guide says in two places that masking works only
with multiresolution sculpting; the mask's resolution follows the
subdivision level; there is one mask at a time; and masks are not saved.
For a mask that keeps, the guide points to a falloff instead, because every
sculpt brush respects the active falloff. So on a plain mesh the reference's
working route is: paint a weight map (its separate Vertex Map Tools: Paint,
Smooth, Scale Down, Erase, Set Value, the same five verbs as the mask
brushes), then sculpt with a Vertex Map falloff on that map. Operandi
already multiplies the falloff into the brush weight (foundation).

**SCT-53a Recommendation.** Give Operandi's mask brushes a per-point mask on
plain meshes, with the behaviour in SCT-52, kept for the session and not
saved, like the reference's. This goes beyond the reference's mask scope,
but it matches what the reference already lets you do on plain meshes
(weight map plus falloff) in one step instead of two. This is a product
decision (SCT-Q15).

---

## 5. Pressure, tips, nozzle

**SCT-54** Tips (Smooth, Hard) and pen pressure are already the
foundation's and the pen-tablet stage's. The tablet nozzle starts with
pressure → strength on, pressure → size off, and jitter off. Low Pressure
Smooth Mode behaves as the guide describes: light pressure smooths, and the
effect blends to the brush's own effect as pressure reaches full.
**SCT-55** Procedural, Texture and Sphere tips, Inks, and Background
Constraint are out of scope for v1 (SCT-Q16).

---

## 6. Mapping to generic brush types (for wiring)

"Generic" means the common brush families in open sculpting maths. This
is a guide for picking a starting kernel; the behaviour rules above win.

| Brush | Closest generic type | Notes for wiring |
|---|---|---|
| Fold | Draw with a signed (two-lobe) profile | No direct twin. Draw along Average normal, weight = signed profile (SCT-13). Ctrl flips the sign. |
| Clay | Clay / Fill with a height cap | Plane = brush plane + Level × radius along the normal; points move toward it, never past it. Attenuated density. |
| Smudge | Smear / Smudge (drag-along) | Per-frame drag along the pointer motion, tangent to the surface. Ctrl = reverse direction. |
| Carve | Draw Sharp / Layer, negative | Inward Draw with a per-stroke depth cap (Attenuated). Ctrl = positive. |
| Scar | Crease | Inward Draw plus a pinch toward the stroke line. Ctrl = outward. |
| Flatten | Flatten, with Scrape / Fill sides | Inverted Flatten off ≈ Scrape (lower what's above); on ≈ Fill (raise what's below). Mouse Down normal ≈ "plane locked at stroke start". |
| Tangent Pinch | Pinch restricted to the axis across the stroke | Like Crease's pinch part without the push. Slide on surface. Ctrl = Magnify. |
| Center Pinch | Pinch (radial) | Slide on surface. Ctrl = Magnify. |
| Emboss | Draw with an Anchored / drag-dot stroke | Captured patch, amount from horizontal drag, along vertex normals. |
| Attenuate | Erase displacement / layer erase | Needs a stored base shape; greyed until sculpt layers exist. |
| Spin | Twist / Rotate (continuous) | Axis from Normal Mode; per-dab rotation, not a grab. |
| Mask brushes | Mask (paint, smooth, scale, erase, fill) | Per-point weight multiplied into every brush's weight. |

---

## 7. Acceptance checks (behavioural, hands-on)

1. The Sculpt tab shows the rows in SCT-01's order; the pinch slot switches
   between Tangent and Center Pinch and keeps the last one.
2. Each new brush opens its form with exactly the rows and starting values
   in §3 (Clay and Scar have no Sculpt Mode row; Tangent/Center Pinch have
   no Smooth Mode row; Attenuate's rows read Amount and Smooth).
3. On a subdivided sphere: Fold leaves a dent ringed by a raised rim; Ctrl
   gives a bump ringed by a trough.
4. Clay on a flat grid with Level 10 % stops rising at 10 % of the brush
   radius; brushing longer in one stroke doesn't go higher; Level 0 %
   changes nothing.
5. Carve: back-and-forth within one stroke reaches the same depth as one
   pass; a second stroke goes deeper.
6. Flatten with Mouse Down on a bumpy surface leaves a plateau at the
   height and tilt of the press point. Inverted Flatten on fills dips
   instead of cutting peaks.
7. Center Pinch gathers grid lines toward the centre without leaving the
   surface; Tangent Pinch draws them toward the stroke line.
8. Spin swirls grid lines; Ctrl swirls the other way; Screen normal mode
   can fold the surface.
9. Emboss: press and drag right raises the patch under the press, left
   lowers it, one undo step; the patch doesn't follow the pointer.
10. Every brush: Shift smooths (Undo "Smooth Stroke"); locked borders and
    corners never move; symmetry mirrors the stroke.

---

## 8. Open questions

- **SCT-Q01** Exact look of the Tangent/Center Pinch tool-choice slot
  (split button vs. drop-down face).
- **SCT-Q02** Show image-only fields greyed (recommended) or hide them?
- **SCT-Q03** Numeric ranges and soft limits for all fields.
- **SCT-Q04** Interpolation Step: observed 5 % vs. the guide's 8 %/10 %.
- **SCT-Q05** Fold's profile: where the in/out crossover sits, and the
  rim's height relative to the centre's depth.
- **SCT-Q06** Clay with Ctrl (undocumented).
- **SCT-Q07** Clay's normal: guide says brush centre, observed start is
  Average.
- **SCT-Q08** Smudge: how long points are carried and how strength scales
  the drag.
- **SCT-Q09** Scar: the balance between its pinch and its depth.
- **SCT-Q10** Flatten with Inverted Flatten off: do points below the plane
  rise at all?
- **SCT-Q11** Emboss: drag-to-distance scale, and whether vertical drag
  does anything.
- **SCT-Q12** Attenuate on plain meshes: greyed (recommended) or "fade
  toward the shape at stroke start"?
- **SCT-Q13** Spin: rotation per dab and default handedness.
- **SCT-Q14** Move's Slide mode.
- **SCT-Q15** Mask brushes on plain meshes (product decision). Docs say masks are multiresolution-only; the plain-mesh route is a weight map plus a Vertex Map falloff. Unconfirmed by probe: whether the mask brushes do anything on a plain mesh.
- **SCT-Q16** Procedural, Texture and Sphere tips, Inks, Background
  Constraint: later spec.

Q05, Q06, Q08, Q09, Q10, Q11, Q13 and Q14 can be answered by measured
probes (a flat grid, one stroke each, read the point positions back)
through the reference, in a later research pass.
