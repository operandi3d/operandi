# Vertex Map Tools and Vertex Map falloff — behaviour spec v1

For: plain-mesh sculpt masks (sculpt toolbox SCT-53, SCT-Q15; the product decision is
weight-map painting plus a Vertex Map falloff). Also answers FAL-Q09 for
the Vertex Map falloff. Rules for the paint tools are **VMT-nn**, rules for
the falloff are **VMF-nn**, open questions are **VM-Qnn**.

Provenance: reference user guide 17.1 (vertex map tools, weighting tools,
vertex maps, vertex map falloff, display preferences) for names, intent and
modifier keys; the reference's plain-text UI configuration for toolbox
order, form labels and the tip each tool starts with; scripted probes of
2026-11-11 for the falloff's exact numbers (15 cases), the tools' starting
values and the weight display colours. Brush strokes were not measured
(they need pointer input), so stroke shapes are documented intent or a
marked **build guess**. REA-assisted: no.

---

## 1. Weight maps (what the tools edit and the falloff reads)

**VM-01** A weight map stores at most one number per mesh point. A point
can also have **no value** at all; that is different from 0 (VMF-04, VMT-14).
Values are signed and not limited to 0…1 (the falloff reads −1 and 2 as
given, VMF-03).

**VM-02** A mesh can hold several weight maps, each with a name. One of
them is **selected** (the reference picks it in its vertex map list). The
paint tools write to the selected map and the falloff reads the selected
map (VMF-05). Operandi's existing saved per-point weights are this storage;
if it holds only one map today, that map is "the selected map".

**VM-03 New Weight Map.** A button at the bottom of the Vertex Map Tools
section opens a Create New Vertex Map form: Name (text), Type (Weight Map
for this purpose), Init Value (checkbox) and Value (number, shown when Init
Value is on). With Init Value off the new map has no value on any point;
with it on every point gets Value. The new map becomes the selected map.
*Starting name: unobserved* (VM-Q01).

---

## 2. Vertex Map Tools (paint, smooth, scale down, erase, set value)

### 2.1 Placement

**VMT-01 Toolbox section.** In the Paint layout's toolbox, a section titled
**Vertex Map Tools**, laid out as a vertical list with large icons and
labels, in this order:

1. Paint
2. Smooth
3. Scale Down
4. — divider —
5. Erase
6. Set Value
7. Live Deformers (toggle, starts on; VMT-20)
8. — divider —
9. the brush tip buttons (same tip row as the sculpt toolbox) and Save
   Brush Preset
10. — divider —
11. Nozzle (pen tablet settings, as in the sculpt toolbox)
12. — divider —
13. New Weight Map (button, VM-03)

**VMT-02 Second home.** The animation/setup layouts have a Weighting tab
with the same tools, plus a **Smooth Selected** command right after Smooth
(VMT-17). Operandi may put the section wherever its sculpt toolbox lives;
the order above is what matters.

**VMT-03 Form titles.** Each tool's properties form is titled
"Vertex Map: Paint", "Vertex Map: Smooth", "Vertex Map: Scale Down",
"Vertex Map: Set Value". Erase has no form.

### 2.2 Fields and starting values

Only the fields below are shown; anything else the brush machinery has is
fixed for these tools (same rule as SCT-06).

| Tool | Field | Kind | Starts at |
|---|---|---|---|
| Paint | Paint Value | percent | 50 % |
| Paint | Normalize Weights | checkbox | on |
| Paint | Set Color / Color / Alpha | checkbox / colour / percent | off / — / 100 % |
| Paint | Blend Mode | choice | Normal |
| Paint | Channel Mask R, G, B, A | four toggles | — |
| Smooth | Smooth Amount | percent | 50 % |
| Smooth | Normalize Weights | checkbox | on |
| Scale Down | Scale Amount | percent | 50 % |
| Scale Down | Normalize Weights | checkbox | on |
| Erase | (none) | | |
| Set Value | Value | percent | 0 % |
| Set Value | Normalize Weights | checkbox | on |
| Set Value | Set Color / Color / Alpha | checkbox / colour / percent | off / — / 100 % |

**VMT-04 Colour-only fields.** Set Color, Color, Alpha, Blend Mode and the
Channel Mask act only on colour maps. Operandi paints weight maps only:
show them greyed with a status note, as SCT-07 does for image-only fields,
or leave them out (VM-Q02).

**VMT-05 Fixed density.** Paint and Scale Down build up with tip
attenuation (the sculpt "Attenuated" density); Smooth, Erase and Set Value
use "Additive". These are not shown.

**VMT-06 Tips.** Paint, Smooth, Scale Down and Erase start with the
**Smooth** tip, like the sculpt brushes; the user can pick another tip and
right-drag in the viewport to resize it, as for every sculpt brush. Set
Value has **no tip**: every point the brush touches gets the full effect,
with no fade toward the rim (guide). Brush size is in screen pixels, as in
the sculpt foundation.

**VMT-07 Starting values are not factory-confirmed.** They were read from a
reference install whose tool settings could have been changed; they match
the tools' built-in defaults (VM-Q03).

### 2.3 What each stroke does

All tools act on the **selected weight map** of the active mesh, only on
mesh points (the map has no finer resolution than the points), and only on
points under the brush. "Tip weight" below is the tip's 0…1 profile at the
point, as for the sculpt brushes.

**VMT-10 Paint.** Raises the selected map toward 100 % at touched points;
Paint Value sets the rate: high values reach full weight in a few dabs,
very low ones nudge it. *Build guess:* per dab, new = old + (1 − old) ×
Paint Value × tip weight, with a point that had no value treated as 0.
**Ctrl:** lowers values instead (toward 0). **Shift:** smooths, exactly as
the Smooth tool does (VMT-12).

**VMT-11 Normalize Weights (Paint, Smooth, Scale Down, Set Value).** When
on and the mesh has **more than one** weight map, the other weight maps at
each edited point are reduced so the total over all weight maps there is
100 %. Guide example: a point at 100 % in map A, then painted to 50 % in
map B, ends with A at 50 %. It is meant for skinning. *Build guess:* with a
single weight map it does nothing, and it only ever lowers the other maps
when the total exceeds 100 % (VM-Q04).

**VMT-12 Smooth.** Blurs the map's values across the surface under the
brush. Smooth Amount sets strength: above 50 % gets increasingly soft,
below 50 % gives a slight softening. *Build guess:* per dab, each touched
point moves toward the average of its edge-connected neighbours by Smooth
Amount × tip weight. **Ctrl:** sharpens (moves values away from the
neighbour average, raising contrast).

**VMT-13 Scale Down.** Pulls touched values toward 0 a little each dab;
Scale Amount sets how much. Think dodge/burn. *Build guess:* new = old ×
(1 − Scale Amount × tip weight × k) with a small per-dab k so it builds
slowly; negative values move toward 0 too. **Ctrl:** scales up (away from
0) instead.

**VMT-14 Erase.** **Removes** the value from touched points entirely (they
become "no value", VM-01), rather than setting 0. No fields. *Ctrl:
undocumented* (VM-Q05). For the falloff a removed value reads as 0
(VMF-04), so on a falloff the visible result equals painting 0.

**VMT-15 Set Value.** Writes exactly Value to every touched point, at full
strength, with no fade (no tip). **Ctrl:** erases (as VMT-14).

**VMT-16 Polygon mask.** The guide says colour painting and smoothing are
confined to the selected, unlocked, visible polygons. *Build guess:* apply
the same confinement to weight painting when polygons are selected
(VM-Q06).

**VMT-17 Smooth Selected** (Weighting tab command, no brush). Smooths the
selected map over the selected points once per click; repeated clicks keep
smoothing. Works with the active falloff. Optional for Operandi.

**VMT-18 No map selected.** The guide requires a mesh and a selected
weight map before painting. What a stroke does without one was not
observed (VM-Q07). *Build guess:* the first stroke creates a weight map
named "Weight" with no values, selects it, then paints.

**VMT-19 Undo.** *Build guess:* each stroke (press to release) is one undo
step named after the tool, as for sculpt strokes (VM-Q08).

**VMT-20 Live Deformers.** A toggle in the section, starting **on**. On:
anything driven by the map (deformers, and in Operandi a Vertex Map
falloff shown on screen) updates during the stroke. Off: it updates once on
release, which is faster on big meshes.

### 2.4 Seeing the weights

**VMT-30 Weight shading.** A viewport shading style "Vertex Map" colours the
surface by the selected weight map, interpolated across polygons, flat
(unlit). Measured colours (8-bit sRGB):

| Weight | Colour |
|---|---|
| ≤ −1 | (0, 0, 255) blue |
| −0.5 | (64, 70, 191) |
| 0 | (128, 140, 128) grey-green |
| 0.25 | (159, 105, 96) |
| 0.5 | (191, 70, 64) |
| ≥ 1 | (255, 0, 0) red |

So: a straight blend from grey-green at 0 to red at +1 and to blue at −1,
clamped beyond ±1 (2 shows the same red as 1). Colour of a point with no
value: VM-Q09.

**VMT-31 Weight overlay.** A per-viewport option, **Show Weightmaps**,
draws the same colouring as an overlay on the normal shaded view. Another,
**Show Weight Values**, prints each point's number next to it in a
light grey-green (0.74, 0.82, 0.74), with a preference for decimal places.
Both start off. *Build guess:* while a Vertex Map tool is active, Operandi
shows the weight overlay even if the option is off, so the user sees what
they paint (VM-Q10).

---

## 3. Vertex Map falloff

### 3.1 Form and activation

**VMF-01** Listed in the Falloff menu as **Vertex Map**; form title
"Vertex Map Falloff". Fields:

| Field | Kind | Choices | Starts at |
|---|---|---|---|
| Mode | choice | Magnitude, Component | Magnitude |
| Index | integer | 0… | 0; greyed unless Mode = Component (setting it in Magnitude mode is refused) |
| Mix Mode | choice | Multiply, Add, Subtract, Max, Min | Multiply |

There is **no map field and no Invert field**: the falloff always reads the
selected weight map (VMF-05), and inversion is a separate falloff
(VMF-12).

**VMF-02 No handles.** The falloff has nothing to drag in the viewport (no
handle is defined for it, and it has no position or size). It shows only
its effect.

### 3.2 Weight per point

**VMF-03 Raw value.** Each point's falloff weight is its value in the
selected weight map, **signed and unclamped**. Measured with a 1 m Move:
values −1, −0.5, 0, 0.25, 0.5, 1, 1.5, 2 moved the points −1, −0.5, 0,
0.25, 0.5, 1, 1.5, 2 m. So negative weights push the opposite way and
weights above 1 overshoot. (VM-CASE-01)

**VMF-04 No value = 0.** A point with no value in the map gets weight 0
(it does not move). (VM-CASE-01, column 0)

**VMF-05 Which map.** The selected weight map. With two maps and the
second selected, the second drives it. (VM-CASE-03)

**VMF-06 No map selected → 1.** With no vertex map selected the falloff
gives every point weight 1 (the tool acts as if there were no falloff).
(VM-CASE-04)

**VMF-07 Mode on a weight map.** Magnitude and Component give the same
signed value on a weight map; Index 0 and 1 also give the same (a weight
map has one value, so any index reads it). Magnitude does **not** take an
absolute value of a weight. (VM-CASE-02, VM-CASE-05)

**VMF-08 Other map types (guide only, out of scope now).** On colour maps
Magnitude gives the colour's luminosity and Component picks R, G, B or A;
on morph maps Magnitude gives the length of the offset and Component picks
X, Y or Z. Operandi needs weight maps only.

### 3.3 Combining with other falloffs

**VMF-10 Stacking.** Choosing a falloff from the menu replaces the active
one (FAL-M-01). The menu also has an **Add** route that stacks another
falloff on top of the active ones instead.

**VMF-11 Mix Mode.** Each added falloff combines with the result of the
falloffs before it using **its own** Mix Mode:

| Mix Mode | Result |
|---|---|
| Multiply | before × this |
| Add | before + this |
| Subtract | before − this |
| Max | larger of the two |
| Min | smaller of the two |

The combined result is **not clamped** (Add reached 2, Subtract reached −2,
and the points moved that far). Measured with Linear (1 at x = 0 to 0 at
x = 8) then Vertex Map added, all five modes, and with Vertex Map first
then Linear added (Multiply, Add, Min). (VM-CASE-06..13) The first
falloff's own Mix Mode had nothing to combine with in these runs
(VM-Q11).

**VMF-12 Invert.** The Falloff menu has an **Invert** falloff. Added after
a Vertex Map falloff it gave 1 − w with w first clamped to 0…1: weights
−1, −0.5, 0, 0.25, 0.5, 1, 1.5, 2 and "no value" became 1, 1, 1, 0.75,
0.5, 0, 0, 0 and 1. (VM-CASE-14)

**VMF-13 With sculpt brushes.** The sculpt foundation already multiplies
the active falloff's weight into each brush's per-point weight; a Vertex
Map falloff plugs straight into that. Points with weight 0 or no value do
not move; weight 1 moves fully.

### 3.4 Using it as a sculpt mask

**VM-40** Because weight 1 means "full effect", a painted area is where
sculpting **works**. For a mask in the usual sense (painted = protected),
stack Vertex Map then Invert (VMF-12): painted 100 % → 0 (frozen),
unpainted or erased → 1. *Build suggestion:* SCT-53a's Enable Mask / Invert
Mask controls can be wired to exactly this stack, with the mask brushes
being the Vertex Map Tools on a map named for the mask.

---

## 4. Acceptance cases (cases.json)

- **VM-CASE-01** Magnitude, raw signed value, no value = 0.
- **VM-CASE-02** Component, index 0: same as 01.
- **VM-CASE-03** Two maps, second selected: second drives it.
- **VM-CASE-04** No map selected: all 1.
- **VM-CASE-05** Component, index 1 on a weight map: same as 01.
- **VM-CASE-06** Linear alone (reference for 07..13).
- **VM-CASE-07..11** Linear, then Vertex Map added with Multiply, Add,
  Subtract, Max, Min.
- **VM-CASE-12..13** Vertex Map, then Linear added with Multiply, Add; and
  **VM-CASE-15** with Min.
- **VM-CASE-14** Vertex Map, then Invert added.
- **VM-CASE-16** Weight shading colours.
- Hands-on: VMT-10..15 stroke behaviour, Ctrl/Shift, display while
  painting (not measured; check by eye against the descriptions).

## 5. Open questions

- **VM-Q01** Starting name in the New Weight Map form.
- **VM-Q02** Show colour-only fields greyed, or hide them? (UI choice.)
- **VM-Q03** Factory starting values (a reset was not done).
- **VM-Q04** Normalize Weights with a single map, and whether it only
  lowers other maps when the total exceeds 100 %.
- **VM-Q05** Ctrl with Erase.
- **VM-Q06** Polygon-selection confinement for weight painting.
- **VM-Q07** Stroke with no weight map selected.
- **VM-Q08** Undo granularity of a stroke.
- **VM-Q09** Display colour of points with no value.
- **VM-Q10** Whether the reference turns on weight display by itself when a
  Vertex Map tool is picked.
- **VM-Q11** Whether the first falloff's Mix Mode does anything.
- **VM-Q12** Exact per-dab formulas for Paint, Smooth, Scale Down
  (VMT-10/12/13 are build guesses; measuring needs pointer-driven strokes).
