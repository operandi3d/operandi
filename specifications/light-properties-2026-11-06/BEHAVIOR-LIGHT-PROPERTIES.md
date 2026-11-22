# Light properties panel: behaviour spec v1 (2026-11-06)

Every row on the light-specific Properties tab of all nine light kinds, plus
the light material (shader tree) rows, what each does, its type, default,
units, and what greys it out. Builds on `cameras-lights-2026-11-04` (CL-xx),
which already specs (and built) the core channels and maths of Directional,
Point, Spot and Area. This spec does not repeat that maths; it points to it.

The other four tabs (Display, Assembly, User Channels, Tags) are the same on
every item kind and are specced once in
`camera-properties-2026-11-06/BEHAVIOR-ITEM-COMMON-TABS.md` (IDs CP-D, CP-A,
CP-U, CP-T). A plain-English summary of them is in this folder's README.

Tags: `[D]` public user guide (paraphrased), `[C]` read from a shipped
plain-text configuration file (panel layout and labels), `[O]` observed by
read-only scripted readback, `[S]` seen on Jason's panel, `[pick]` our sane
choice, `[?]` unknown / to confirm. `Op` = what Operandi has per the CL spec
(this thread did not read Operandi code: **Have** = specced and built in
CL-1/CL-2, **No** = not built). Tier: see README.

Units: intensity is shown with the unit "W/srm2" on every kind `[O]`
(CL table). Percent rows are stored 0–1 and shown 0–100 %.

## Panel shape

- **LP-01** A light's Properties panel has five vertical tabs: **<Kind
  name>** (e.g. "Directional Light"), **Display**, **Assembly**, **User
  Channels**, **Tags** `[S]`. Only the first is light-specific. Panel chrome
  (Presets, One Tab, All Tabs, collapsible sections, greyed rows keep their
  value) is as camera CP-01..CP-05.
- **LP-02** The light tab, top to bottom: Name, Notes (collapsed), Transform
  section, Targeting rows (some kinds only, LP-04), then a section named after
  the kind with the light rows, then collapsible sections **Light Linking**
  (all kinds), **Physical Sun** (directional only), **Volumetrics**
  (directional, point, spot, photometric). Light Linking, Physical Sun and
  Volumetrics start **collapsed** `[C]`. A thin divider splits the light rows
  into "brightness and shadows" and "size, sampling and visibility" `[C]`.
- **LP-03** Name, Notes and Transform are identical to the camera (CP-V01,
  CP-V02, CP-V03..). Scale also multiplies a light's size rows (area
  Width/Height etc.) `[D]` (CL-10).
- **LP-04** **Targeting rows** (Set Target, Remove Target, Enable, Set Focus
  Distance, Roll, Time Offset) appear on Directional, Spot, Area, Cylinder,
  Photometric and Portal, **not** on Point, Dome or Mesh Light `[C]`
  (pointless for things with no facing). Behaviour = camera's Set Target
  spec. Set Focus Distance is a camera DOF thing and does nothing useful on
  a light `[D]`; pick: hide it on lights.
- **LP-05** Skip the **Octane Light** section (third-party renderer plugin).

## Shared light rows

Rows present on most kinds; the per-kind table says which kinds have which.
Labels differ by kind only for intensity (LP-12).

| ID | Label | Type / range | Default | Behaviour | Greyed when | Op | Tier |
|---|---|---|---|---|---|---|---|
| LP-10 | Render | dropdown: Default, Yes, No | Default `[O]` | Default = the light renders only while visible in the item list (eye on); Yes = always renders; No = never renders, whatever the eye says `[D]`. | – | Part (CL-22: eye controls render) | 1 |
| LP-11 | Dissolve | percent, 0–100 % | 0 % `[O]` | Dims the light: strength × (1 − dissolve); 100 % = off `[D]`. | – | Have? | 1 |
| LP-12 | Radiant Exitance (directional) / Radiant Intensity (point, spot, photometric) / Radiance (area, cylinder, dome, mesh) | float, ≥ 0, "W/srm2" | 3.0 `[O]` | Brightness. Same stored value, label depends on what it physically means for that kind (CL-09c). | – | Have | 0 |
| LP-13 | Light Color | RGB colour | 1, 1, 1 `[O]` | **Shortcut** to the light material's Color (LM-10): editing either edits the same value `[C][D]`. | – | Have | 0 |
| LP-14 | Falloff Type | dropdown: None, Inverse Distance, Inverse Distance Squared | Inverse Distance Squared `[O]` | How brightness drops with distance: none / 1/d / 1/d² (CL-09c) `[D]`. Not on directional, dome or portal `[C]`. | – | Have | 0 |
| LP-15 | Shadow Type | dropdown: None, Ray Trace, Deep Shadow Map | Ray Trace `[O]` | None = casts no shadows. Ray Trace = normal shadows. Deep Shadow Map = a faster approximate shadow for fur/volumes `[D]`. | – | Have (None/Ray Trace) | 0; Deep Shadow Map never |
| LP-16 | Shadow Map Res | integer, pixels | 1024 `[O]` | Resolution of the deep shadow map `[D]`. Directional, point, spot, photometric only `[C]`. | Shadow Type ≠ Deep Shadow Map `[D][S]` | No | never |
| LP-17 | Samples | integer, ≥ 1 | 64 `[O]` | Per-light sample count for soft light/shadows. Only used when the render's global adaptive light sampling is off; otherwise greyed `[D]`. | Render setting "Direct Illumination light samples" > 0 (the default) `[D]` | No | never (Cycles has no per-light samples) |
| LP-18 | Importance | percent, ≥ 0 | 100 % `[O]` | Multiplier on how much of the shared sample budget this light gets; doesn't change brightness, only noise `[D]`. | – | No | 3 |
| LP-19 | Visible to Camera | checkbox | off `[O]` | Light's own shape shows up in the render (as a glowing disk/rectangle) `[D]`. Size from Radius / Width×Height / Spread Angle by kind `[D]`. | – | Have? (CL table) | 1 |
| LP-20 | Visible to Reflection Rays | checkbox | off `[O]` | The light's shape appears in mirror-like reflections `[D]`. | – | No | 1 |
| LP-21 | Visible to Refraction Rays | checkbox | off `[O]` | The light's shape is seen through glass `[D]`. | – | No | 1 |
| LP-22 | LPE Label | text | empty `[O]` | A name used to split this light's contribution into its own render pass ("light path expression" output) `[D]`. | – | No | 3 |

**LP-23 Order on the panel** `[C]`: Render, Dissolve, intensity, Light Color,
Falloff Type, Shadow Type, Shadow Map Res, (kind-specific rows), divider,
(size rows), Samples, Importance, Visible to Camera, Visible to Reflection
Rays, Visible to Refraction Rays, LPE Label, then kind extras (spot/photometric
cone rows) after another divider. Exact per-kind order is in the per-kind
tables below.

### Light Linking (section, all kinds incl. portal)

**Plain English:** pick a group of objects and say "this light lights ONLY
these" or "this light lights everything EXCEPT these".

| ID | Label | Type | Default | Behaviour | Greyed when | Op | Tier |
|---|---|---|---|---|---|---|---|
| LP-30 | Control Light Linking | checkbox | off `[O]` | Turns this light's own linking on; overrides the shader-level linking for this light `[D]`. | – | No | 2 |
| LP-31 | Item Group | dropdown of the scene's item groups, plus none | none `[?]` | The group of items the rule applies to. Groups are made in the Groups panel from selected items `[D]`. Operandi needs item groups first. | Control Light Linking off `[?]` | No | 2 |
| LP-32 | Mode | dropdown: Include, Exclude | **Exclude** `[O]` | Include = light affects only the group. Exclude = light affects everything but the group `[D][C]`. | Control Light Linking off `[?]` | No | 2 |

### Volumetrics (section; directional, point, spot, photometric)

**Plain English:** make the light beam visible in the air, like sunbeams
through fog ("god rays"). The look (colour, thickness) is on the light
material (LM-30..34); the switch and shape are here.

| ID | Label | Type | Default | Behaviour | Greyed when | Op | Tier |
|---|---|---|---|---|---|---|---|
| LP-40 | Volumetrics | checkbox | off `[O]` | Turns the visible beam on `[D]`. | – | No | 3 |
| LP-41 | Samples | integer | 40 `[O]` | Quality of shadows cast through the beam; more = smoother, slower `[D]`. | Volumetrics off `[D]` | No | 3 (or never with Cycles) |
| LP-42 | Height (directional, spot) | distance | 10 m (both) `[O]` | How far the beam reaches out in front of the light `[D]`. | Volumetrics off | No | 3 |
| LP-43 | Radius (directional) / Base (spot) | distance | 0.5 m directional, 0 m spot `[O]` | Width of the beam where it starts at the light `[D]`. | Volumetrics off | No | 3 |
| LP-44 | Sphere Radius (point, photometric) | distance | 1 m `[O]` | Size of the glowing ball of air around the light `[D]`. | Volumetrics off | No | 3 |

## Per kind

Every default below was read back from a freshly created light of each kind
`[O]`. All kinds: intensity 3.0, Inverse Distance Squared, Ray Trace, Samples
64, Importance 100 %, Visible rows off, Display Size 1.0 `[O]`.

### Directional Light (sun-like parallel rays)

Rows `[C]`: Render, Dissolve, **Radiant Exitance**, Light Color, Shadow Type,
Shadow Map Res, **Shadow/Photon Size**, ─, **Spread Angle**, Samples,
Importance, Visible ×3, LPE Label. No Falloff Type (rays never fade) `[D]`. Physical Sun and Volumetrics defaults below
were read on a new directional light `[O]`.
Targeting rows yes. Sections: Light Linking, **Physical Sun**, Volumetrics.

| ID | Label | Type | Default | Behaviour | Greyed when | Op | Tier |
|---|---|---|---|---|---|---|---|
| LP-DIR-01 | Shadow/Photon Size | distance | 2 m `[O][S]` | Size of the square the shadow map (and caustic photons) cover, drawn as an outline around the light `[D]`. Only matters for shadow maps and caustics. | – (shown enabled with Ray Trace `[S]`) | No | never |
| LP-DIR-02 | Spread Angle | angle, ≥ 0° | 0° `[O]` | Apparent size of the sun: 0 = razor-sharp shadows; 2–5° = realistic soft sun shadows; full angle (CL-09b) `[D][O]`. | – | Have | 0 |

#### Physical Sun (section, directional only)

**Plain English:** instead of aiming the light by hand, type a place on
Earth, a date and a time, and the light points where the real sun would be.
Works with the environment's "physically based daylight" sky, which is
where you pick which light is the sun `[D]`.

| ID | Label | Type / range | Default | Behaviour | Greyed when | Op | Tier |
|---|---|---|---|---|---|---|---|
| LP-SUN-01 | Physical Sun | checkbox | off `[O]` | Makes this light the sun: its rotation is computed from the rows below `[D]`. No sun position at night `[D]`. | – | No | 2 |
| LP-SUN-02 | Time | time of day, hours | 12:00 `[O]` | Local clock time `[D]`. | Physical Sun off `[?]` | No | 2 |
| LP-SUN-03 | Day | date | 21 June `[O]` (year stored, ignore) | Day of year `[D]`. | same | No | 2 |
| LP-SUN-04 | Daylight Savings Time | checkbox | off `[O]` | Shifts Time by one hour `[D]`. | same | No | 2 |
| LP-SUN-05 | Haze | float 1–10 | 3.0 `[O]` | Atmosphere murk: higher = warmer, oranger horizon and sun `[D]`. Same value as the daylight sky's Haze `[D]`. | same | No | 2 |
| LP-SUN-06 | Clamp Intensity | dropdown: None, Clamp, Replace | Clamp `[O]` | None = physically real (very bright) sun; Clamp = capped to a pleasant level; Replace = use this light's Radiant Exitance instead `[D]`. | same | No | 2 |
| LP-SUN-07 | Gamma | float | 2.2 `[O]` | Extra brightness/colour tweak of the sun and sky `[D]`. | same | No | 3 |
| LP-SUN-08 | North Offset (from −Z) | angle | 0° `[O]` | Which way is north, measured from the scene's −Z axis, so you don't have to rotate your model `[D][C]`. | same | No | 2 |
| LP-SUN-09 | Location | dropdown of named cities | `[?]` (lat/long below matches the San Francisco Bay Area) | Fills Latitude/Longitude/Time Zone from a preset list `[D]`. | same | No | 2 |
| LP-SUN-10 | North Latitude | angle, −90..90° | 37.53° `[O]` | Negative = southern hemisphere `[C]`. | same | No | 2 |
| LP-SUN-11 | East Longitude | angle, −180..180° | −122.30° `[O]` (i.e. 122.3° W) | Negative = west `[C]`. | same | No | 2 |
| LP-SUN-12 | Time Zone | float, hours from UTC | −8 `[O]` | | same | No | 2 |
| LP-SUN-13 | Store Location | button | – | Saves the current lat/long/zone as a named location in user prefs `[D]`. | same | No | 3 |

Pick: Operandi computes sun direction with a standard solar-position formula
(public, e.g. NOAA) from these inputs; the sky itself is the environment
spec's job (Cycles Sky Texture).

### Point Light (bulb)

Rows `[C]`: Render, Dissolve, **Radiant Intensity**, Light Color, Falloff
Type, Shadow Type, Shadow Map Res, ─, **Radius**, Samples, Importance,
Visible ×3, LPE Label. No Targeting. Sections: Light Linking, Volumetrics
(with Sphere Radius).

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-PT-01 | Radius | distance, ≥ 0 | 0 m `[O]` (CL) | Bulb size: 0 = hard shadows; larger = softer shadows and highlights, and the size seen when Visible to Camera `[D]`. | Have | 0 |

### Spot Light (cone)

Rows `[C]`: Render, Dissolve, **Radiant Intensity**, Light Color, Falloff
Type, Shadow Type, Shadow Map Res, ─, **Radius**, Samples, Importance,
Visible ×3, LPE Label, ─, **Cone Angle**, **Soft Edge**. Targeting yes.
Sections: Light Linking, Volumetrics (Height, Base).

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-SP-01 | Radius | distance, ≥ 0 | 0 m `[O]` | As point. | Have | 0 |
| LP-SP-02 | Cone Angle | angle | 45° `[O]` | Full cone angle; 360° = like a point light `[D]`. Maths CL-09. | Have | 0 |
| LP-SP-03 | Soft Edge | angle | 5° `[O]` | Fade measured inward from the cone edge (CL-09) `[D][O]`. | Have | 0 |

### Area Light (soft box)

Rows `[C]`: Render, Dissolve, **Radiance**, Light Color, Falloff Type, Shadow
Type, ─, **Shape**, **Width**, **Height**, Samples, Importance, Visible ×3,
LPE Label. No Shadow Map Res. Targeting yes. Section: Light Linking.

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-AR-01 | Shape | dropdown: Rectangle, Ellipse | Rectangle `[O]` | Outline of the emitting panel `[D][C]`. | Have | 0 |
| LP-AR-02 | Width, Height | distance (one row pair) | 1 m × 1 m `[O]` | Absolute size, × item scale; bigger = brighter and softer (CL-10) `[D]`. | Have | 0 |

### Cylinder Light (fluorescent tube)

Rows `[C]`: Render, Dissolve, **Radiance**, Light Color, Falloff Type, Shadow
Type, ─, **Radius**, **Length**, Samples, Importance, Visible ×3, LPE Label.
Targeting yes. Section: Light Linking. No volumetrics.

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-CY-01 | Radius, Length | distance pair, ≥ 0 | 0.2 m, 1 m `[O]` | Tube size; emits sideways from the curved surface `[D]`. Tube axis `[?]` (LPQ-02). | No | 2 |

### Dome Light (overcast sky)

Rows `[C]`: Render, Dissolve, **Radiance**, Light Color, Shadow Type, ─,
Samples, Importance. Nothing else: no falloff, no size row, no Visible rows,
no LPE Label, no Targeting. Section: Light Linking.

- **LP-DM-01** A dome light is an even light from a huge sphere around the
  scene, like an overcast day `[D]`. The dome has a hidden radius (10 m on
  the one read back `[O]`), not shown on the panel. The guide itself says
  global illumination with a bright environment usually looks better `[D]`.
  Pick: map to a uniform environment light in Cycles. Tier 3.

### Mesh Light (glowing object shape)

Rows `[C]`: Render, Dissolve, **Radiance**, Light Color, Falloff Type, Shadow
Type, ─, **Prototype**, **Use Prototype Transform**, Samples, Importance,
Visible ×3, LPE Label. No Targeting. Section: Light Linking.

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-MS-01 | Prototype | dropdown of mesh items | none `[O]` (a new mesh light emits nothing until one is picked `[pick]`) | The mesh whose shape becomes the light; that mesh is hidden in renders. Several mesh lights may share one mesh `[D]`. | No | 2 |
| LP-MS-02 | Use Prototype Transform | checkbox | on `[D][O]` | On = the light sits wherever the mesh is (move the mesh to move the light). Off = the light has its own position `[D]`. | No | 2 |

Note: a mesh light gives less noise than just making a material glow, because
the renderer can aim samples at it `[D]`. In Cycles an emissive mesh already
gets that treatment, so this is "set an emission shader + hide the source".

### Photometric Light (real fixture from an IES file)

Rows `[C]`: Render, Dissolve, **Radiant Intensity**, Light Color, Falloff
Type, Shadow Type, Shadow Map Res, ─, **Width**, **Height**, Samples,
Importance, Visible ×3, LPE Label, ─, **Cone Angle**, **Soft Edge**, ─,
**Photometry File** + **Browse...**. Targeting yes. Sections: Light
Linking, Volumetrics (Sphere Radius).

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-PH-01 | Photometry File | file path + Browse... | empty | Loads a manufacturer's `.ies` or `.ldt` file describing how bright the fixture is in each direction. Kept external, like an image `[D]`. | No | 2 |
| LP-PH-02 | (intensity) | | 3.0 before a file is loaded `[O]` | Loading a file **sets** Radiant Intensity and Width/Height from the file; editing them afterwards makes it no longer accurate `[D]`. | No | 2 |
| LP-PH-03 | Width, Height | distance pair, ≥ 0 | 0 m × 0 m until a file sets them `[O][D]` | Size of the glowing part `[D]`. | No | 2 |
| LP-PH-04 | Cone Angle, Soft Edge | angles | 360°, 0° `[O]` (i.e. no cone limit) | Same meaning as spot (LP-SP-02/03), applied on top of the file's shape `[D]`. | No | 2 |
| LP-PH-05 | Viewport shape | – | – | Drawn as a sphere pushed out in each direction by the file's brightness; scaled by Display Size `[D]`. | No | 2 |

### Portal (window helper, not really a light)

Rows `[C]`: Render, ─, **Shape**, **Width**, **Height**, Samples,
**Environment Only**. No intensity, colour, falloff, shadow, Dissolve,
Visible or LPE rows; no light material `[D][C]`. Targeting yes. Section:
Light Linking.

- **LP-PO-01** **Plain English:** a rectangle you put over a window. It
  makes no light itself; it tells the renderer "sky light comes in through
  here, aim your samples at it", so interiors lit by the sky render with far
  less noise `[D]`. Use few, slightly larger than the opening `[D]`.

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LP-PO-02 | Shape | dropdown: Rectangle, Ellipse | Rectangle `[O]` | As area light `[D]`. | No | 1 |
| LP-PO-03 | Width, Height | distance pair, ≥ 0 | 1 m × 1 m `[O]` | Size of the opening `[D]`. | No | 1 |
| LP-PO-04 | Environment Only | checkbox | off `[O]` | On = only gather environment light through it, ignore direct lights `[D]`. | No | 1 (Cycles portals are environment-only anyway) |

## Light material (shader tree item)

Every light has one, as a child of the light (CL-01); a new portal gets one
too `[O]` even though the guide says portals take their colour from the
environment and have no light material settings `[D]`. Pick: Operandi gives
portals none. Its
panel `[C]`:

**Plain English:** the light item says how bright and what shape; the light
material says what colour, and which kinds of shading the light is allowed
to take part in.

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| LM-01 | Enable | checkbox | on `[O]` | Off with only one material = light off `[D]`. | Have | 0 |
| LM-02 | Invert | checkbox | off `[O]` | Inverts the colour `[D]`. | No | never |
| LM-03 | Blend Mode | dropdown (Normal, Add, Subtract, …) | Normal `[O]` | Only matters when textures are layered on the light `[D]`. | No | 3 |
| LM-04 | Opacity | percent 0–100 | 100 % `[O]` | With one material, also dims the light `[D]`. | No | 3 |
| LM-10 | Color | RGB | 1, 1, 1 `[O]` | Light colour (= LP-13). | Have | 0 |
| LM-11 | Shadow Color | RGB | 0, 0, 0 `[O]` | Tints this light's shadows (CL-Q15) `[D]`. | No | never (no Cycles equivalent) |
| LM-12 | Affect Diffuse | percent | 100 % `[O]` | 0 = specular-only light `[D]`. | No | 1 (0/100 only, CL-Q15) |
| LM-13 | Affect Specular | percent | 100 % `[O]` | 0 = no highlights from this light `[D]`. | No | 1 (0/100 only) |
| LM-14 | Affect Caustics | percent | 100 % `[O]` | Contribution to caustics `[D]`. | No | never |
| LM-15 | Affect Subsurface | percent, may exceed 100 | 100 % `[O]` | Contribution to subsurface scattering `[D]`. | No | 3 |
| LM-16 | Set Temperature (Kelvins) | button → number dialog | – | Sets Color from a colour temperature (1500 K candle … 5500 K noon sun … 9000+ K blue sky) `[D]`. One-shot; stores RGB, not K `[?]`. | No | 1 |
| LM-30 | Scattering Color (Volumetrics) | RGB | 1, 1, 1 `[O]` | Colour of the visible beam `[D]`. | No | 3 |
| LM-31 | Density | percent | 50 % `[O]` | Thickness of the beam `[D]`. | No | 3 |
| LM-32 | Light Attenuation | percent | 10 % `[O]` | How fast the beam fades away from the light `[D]`. | No | 3 |
| LM-33 | Light Shift | percent, −100..100 | 0 % `[O]` | Tints the fading beam: negative bluer, positive redder; needs Attenuation > 0 `[D]`. | No | 3 |

A light material can also hold texture layers (e.g. an image to make a
"slide projector" gobo) `[D]`. Tier 3.

## Cross-cutting

- **LP-50** Shared rows are one setting per light, even when the label
  changes by kind (LP-12). Changing a light's kind (CL-05) keeps shared rows.
- **LP-51** Multi-selection of several lights: panel shows the rows of the
  first selected kind; edits apply to all selected lights that have the row
  `[pick]` (same rule as camera spec cross-cutting).
- **LP-52** Hidden channels exist that no panel shows (e.g. dome radius,
  a sun azimuth/elevation pair, a volumetric dissolve) `[O]`. Ignore.
