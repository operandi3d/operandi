# Render item: behaviour spec v1.1 (2026-11-11)

v1.1: new-scene defaults read off Jason's panel added (`[S]`), Preview tab
added, Render Camera placement settled.
v1.2: path-tracer defaults read off Jason's panel: Maximum Quality 64,
Noise Threshold 10 %, Diffuse / Reflection / Refraction Depth 8 / 4 / 8,
engine CPU. Tab list confirmed the same with the path tracer selected.

What the Render item is, how you reach it, and every row on its tabs:
type, default, range, what it does, what greys it out.

Tags: `[D]` public user guide (paraphrased), `[O]` measured earlier in
the reference app (from the cameras-lights spec), `[S]` new-scene
value Jason read off the panel (with the legacy renderer, which a new
scene starts on, or with the path tracer where the row says so), `[pick]` our sane
choice where the guide is silent, `[?]` unknown.
`Op` column: **Have** = Operandi has it today (only via the MCP bridge),
**No** = not built. Tier: see README.

## 1. The item

- **RI-01** The Render item holds the scene's global render settings. There
  is exactly one per scene, it cannot be deleted, renamed away, or moved:
  it is always the top row of the Shader Tree, and every shader layer,
  material group and render output sits under it `[D]`.
- **RI-02** Clicking the Render row selects it like any other shader tree
  item (UIC-30) and the Properties panel shows the Render item's tabs
  (UIC-101). Today the click does nothing; this is the fix Jason asked for.
- **RI-03** Selecting it does not change the viewport selection mode or
  the selected mesh items `[pick]` (same as selecting a material).
- **RI-04** Every change is one undo step and is saved in the scene file.
  Changing any row restarts an open Preview (RP spec) `[pick]`.
- **RI-05** Settings are per scene. A new scene gets the defaults below.
  Old scenes without the new rows load with the defaults, except Samples /
  Noise Threshold / Denoise, which keep the values they already had
  (RI-Q03, RI-Q04, RI-Q05).

## 2. Panel shape

- **RI-10** Vertical tabs on the right edge (UIC-100), top to bottom:
  **Frame**, **Settings**, **Preview**, **User Channels**, **Tags** `[S]`.
  The reference also has a Global Illumination tab between Settings and
  Preview, used only by its legacy renderer; Operandi leaves it out
  (RI-Q01). User Channels and Tags are the shared generic item tabs already
  built for lights and cameras; they start empty `[S]`. No Display or
  Assembly tab `[S]`.
- **RI-11** Each tab starts with a **Name** row (UIC-104), "Render",
  greyed (not renamable) `[pick]`, then **Apply Preset** (tier 2, RI-S01).
- **RI-12** Sections per tab are listed below and start open, except
  sections marked *collapsed* (UIC-106). Rows the guide marks as "hidden
  under More" go in a closed **More** section at the end of their group
  `[D]` `[pick]`.
- **RI-13** Rows that don't apply grey out in place, never hide (UIC-117),
  e.g. Subdivision Rate greys when Adaptive Subdivision is off.
- **RI-14** Rows Operandi does not build yet (tiers 3 / park) are left off
  the panel entirely, not shown greyed `[pick]`; greyed means "turn
  something on", not "not written yet".

## 3. Frame tab

**What it's for:** which camera renders, and the size and shape of the
picture.

### Camera

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-F01 | Render Camera | dropdown of scene cameras, each with the camera icon | the new scene's "Camera" `[S]` | Which camera the final render (F9) and the Preview's "(Render Camera)" look through. Same setting as item list right-click > Set as Render Camera (CL-07); either place updates the other. Sits at the top of the Frame tab `[S]`. | Part (CL-07) | 0 |

### Frame Options

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-F02 | Resolution Unit | dropdown: Pixels, Inches, Centimeters `[D]` | Pixels `[S]` | Unit for Width / Height. Inches and Centimeters work out pixels from DPI. | No | 2 |
| RI-F03 | Frame Width / Height | 2 × integer px, 1…16384 `[pick]`, linked pair (UIC-56, link off by default) | 1920 × 1080 `[O][S]` | Size of the rendered picture. Replaces today's fixed frame. A camera with Override Render Resolution on (CP-V17) wins over this. Camera-view frame bands (CL-19) follow it. Value presets arrow (UIC-55) with common sizes `[pick]`: 1280×720, 1920×1080, 2560×1440, 3840×2160, 1080×1080, 1080×1920, 2048×2048. | No | 0 (presets 1) |
| RI-F04 | DPI | number, 1…9600 `[pick]` | 300 `[pick]` | Dots per inch. Only matters when Resolution Unit is Inches or Centimeters: greyed otherwise. Also written into saved images so print size comes out right `[D]`. | No | 2 |
| RI-F05 | Pixel Aspect Ratio | number, 0.1…10 `[pick]` | 1.0 `[D][S]` | Width of one pixel relative to its height. 1 = square. The Preview and render window still show square pixels, so non-1 values look stretched there `[D]`. | No | 1 |

- **RI-F05b QuickCam Resolution** (dropdown, 100 % `[S]`): sits between
  Height and Pixel Aspect Ratio. The guide doesn't describe it `[?]`; from
  the name, a scale for quick camera renders. Not built (tier 3, RI-Q11).
- **RI-F04 note:** DPI is not on the panel in a new scene `[S]`; it only
  appears when Resolution Unit is Inches or Centimeters `[D]`. Operandi
  shows it greyed instead (UIC-117).

Picking a camera Film Back Preset also sets Frame Width / Height (CP-V36);
it now writes these rows instead of "film only" (closes CAM-Q04).

### Region Options

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-F06 | Render Region | toggle | off `[S]` | When on, the next render only computes the area between the four edges below; the rest stays empty `[D]`. | Part (render window Region) | 1 |
| RI-F07 | Left / Right / Top / Bottom | 4 × percent, 0…100 % | 0 %, 100 %, 0 %, 100 % `[S]` | Edges of the region as a share of the frame. Left < Right and Top < Bottom; typing a value that crosses the other edge clamps it `[pick]`. Greyed while Render Region is off. | No | 1 |
| RI-F08 | Edit Region | button | – | Lets you drag the region box in the Preview or render window. Dragging a box there sets the four edges **and** turns Render Region on; a single click on the picture turns it off `[D]`. Wire this to the existing render-window Region tool so both use the same stored region `[pick]`. | Part | 1 |

### Sequence Options (*collapsed*, park until animation)

First Frame 1, Last 150, Step 1 (negative = reverse), Frame Range "..."
(= all frames; also takes commas and dashes), Output Pattern (file name
built from optional pass name and output name tokens, dot-separated, then
frame number) `[D][S]`. Then **Frame Passes** 1 `[S]` (never, see below).
Not built; listed so the Frame tab's final order is known. Order on the
tab: Render Camera, Sequence, Frame Passes, Frame Options, Buckets,
Region `[S]`.

### Bucket Options: never

Bucket size 32 × 32, order Hilbert (of Rows, Columns, Spiral, Hilbert,
Random), write buckets to disk off `[D][S]`. Cycles renders progressively with its
own tiles; these rows would do nothing.

### Baking Options (tier 3)

Bake Width / Height (pixels, separate from frame size) and Bake
Directional Effects (include view-dependent shading in bakes) `[D]`. Only
when Operandi gets baking.

## 4. Settings tab

**What it's for:** how clean the picture is, how far light bounces, and how
detailed subdivided and displaced surfaces get at render time.

### Engine

| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S01 | Apply Preset | preset control: pick, Store Preset, Remove Preset | – | Stores chosen render settings as a named preset; built-in presets can't be removed `[D]`. Pick: ship three built-ins, Draft / Medium / Final (Maximum Quality 16 / 64 / 512, Noise Threshold 20 / 10 / 2 %). | No | 2 |
| RI-S02 | Ray Tracing Engine | dropdown: CPU, Metal (GPU), NVIDIA OptiX (GPU) `[pick]` | CPU `[S]` (path tracer's CPU engine) | Which device renders `[D]`. An option the machine can't run shows a warning icon but can still be chosen (so a scene can be set up for another machine); rendering then falls back to CPU with a status-bar note `[D]` `[pick]`. | No | 2 |

### Sampling

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S03 | Maximum Quality | integer samples per pixel, 1…65536 | 64 `[S]` (path tracer) for new scenes; old scenes keep their value (RI-Q03) | How many samples each pixel gets at most. Low = fast and noisy, high = clean and slow `[D]`. This **is** today's scene Samples (16), renamed and moved here. For reference, the legacy renderer's edge-only Antialiasing setting starts at 16 samples per pixel `[S]`; the path tracer's own default is 64 `[S]`. Value presets arrow (UIC-55) with powers of two `[pick]`. | Have (bridge only) | 0 |
| RI-S04 | Noise Threshold | percent, 0…100 %, 2 decimals | 10 % `[S]` (path tracer) for new scenes (RI-Q04) | Stop sampling a pixel once its noise falls below this. Higher = faster, noisier; 10–20 % can be fine if denoising runs after `[D]`. 0 % = let Cycles choose. This **is** today's Adaptive Threshold (0). | Have (bridge only) | 0 |
| RI-S05 | Antialiasing Filter | dropdown: Gaussian, Box, Triangle `[D]` | Gaussian `[D][S]` | Shape used to blend samples into a pixel. Gaussian is soft, Box is sharpest. Triangle has no exact Cycles twin (see mapping). | No | 2 |

### Ray Depth

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S06 | Diffuse Depth | integer, 0…128 | 8 `[S]` (path tracer) | Most times light may bounce off matte surfaces before the path stops `[D]`. 0 = direct light only. The legacy renderer's GI Bounces starts at **1** `[S]`, but that renderer also has irradiance caching; Operandi follows the path tracer's 8. | No | 1 |
| RI-S07 | Reflection Depth | integer, 0…128 | 4 `[S]` (both renderers) | Most mirror / glossy bounces `[D]`. | No | 1 |
| RI-S08 | Refraction Depth | integer, 0…128 | 8 `[S]` (both renderers) | Most bounces through glass and liquids. Too low makes stacked glass go black `[D]`. | No | 1 |

### More (collapsed)

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S09 | Path Threshold | percent 0…100 % | 0.1 % `[pick]` (legacy Ray Threshold starts at 0.1 % `[S]`) | Paths that would add very little are stopped early; higher = faster, less accurate `[D]`. | No | 3 |
| RI-S10 | Maximum Radiance | number ≥ 0, 0 = off | 10.0 `[D][S]` | Caps how bright a bounced ray can be, to kill fireflies. Never touches what the camera sees directly `[D]`. Keep above 1 `[D]`. | No | 1 |
| RI-S11 | Indirect Caustics | dropdown: None, Reflection Only, Refraction Only, Both `[D]` | Both (RI-Q08) | Whether light focused by mirrors and/or glass (bright patterns under a glass, light off a polished floor) is traced. None = faster, cleaner, less real. | No | 1 |

### Geometry (subdivision at render time)

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S12 | Adaptive Subdivision | checkbox | off `[S]` | When on, every subdivision-surface mesh is split at render time until its edges are about Subdivision Rate pixels long on screen, overriding each mesh's own subdivision level. Close things get more detail, far things less `[D]`. Off = each mesh uses its own render level. | No | 2 |
| RI-S13 | Subdivision Rate | number px, 0.5…1000 | 10 px `[D][S]` | Target on-screen edge length. 5 or less = no visible facets; very high = few polygons `[D]`. Greyed unless RI-S12 on. | No | 2 |

### Micropolygon Displacement

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-S14 | Micropoly Displacement | checkbox | on `[S]` | Master switch for real displacement from displacement layers in the Shader Tree. Off = displacement layers do nothing at render time `[D]`. | No | 2 |
| RI-S15 | Displacement Rate | number px, 0.5…100 | 1.0 px `[S]` | How finely surfaces are split before displacing, measured on screen (so it adapts to distance and frame size) `[D]`. Greyed unless RI-S14 on. | No | 2 |
| RI-S16 | Displacement as Bump | checkbox | off `[S]` | Also uses the displacement map as bump, so fine detail shows without as much splitting `[D]`. | No | 2 |
| RI-S17 | Displacement Ratio | number, in More | 1.0 `[D]` | Higher = less splitting outside the camera's view; may show in reflections `[D]`. | No | 3 |
| RI-S18 | Minimum Edge Length | distance, in More | 0.1 mm `[pick]` | Edges this short stop splitting; caps polygon count `[D]`. | No | 3 |
| RI-S19 | Smooth Positions | checkbox, in More | on `[pick]` | On: new points follow the smooth surface. Off: they stay on the flat original faces (needed for some baked displacement maps) `[D]`. | No | 3 |
| RI-S20 | Indirect LOD | checkbox, in More | off `[pick]` | Keeps a lighter copy of displaced geometry for bounced light; faster, more memory `[D]`. | No | 3 |
| RI-S21 | Render Output Masking | checkbox | on `[pick]` | Lets group and layer masks in the Shader Tree limit render outputs `[D]` (guide gives no detail). | No | 3 |

## 4b. Preview tab

**What it's for:** when the Preview window counts as "finished".
One section, Progressive Rendering `[S]`.

| ID | Label | Type / range | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| RI-P01 | Convergence Target | percent, 0…100 % | 97.5 % `[D][S]` | How far the Preview refines before it says Finished and stops. Operandi: Preview stops at this share of Maximum Quality samples (or earlier if Noise Threshold is met) `[pick]`. Links to the RP spec's sample target. | No | 1 |
| RI-P02 | Max Render Time | minutes, decimals allowed, 0 = no limit | 5.0 `[S]` | The Preview (and saving a frame from it) stops after this long even if not converged `[D]`. Does **not** cut F9 final renders `[pick]` (RI-Q09). | No | 1 |

## 5. What lives elsewhere (don't duplicate here)

- **Exposure, gamma, tone mapping, white level:** Final Color Output panel
  (already built).
- **Denoise:** moves to the Final Color output's **Denoising** row (RI-Q05).
- **Environment light and its importance sampling:** the Environment item.
  The path tracer does importance sampling automatically `[D]`, so there is
  no switch.
- **Per-light samples, shadows:** each light (light-properties spec).
- **Render subdivision level per mesh:** the mesh item.

## 6. Never (legacy renderer only)

Renderer dropdown (new scenes start on the legacy "Default" renderer `[S]`), Shading Engine, buckets, Field Rendering, Frame
Passes (1 `[S]`), Antialiasing sample count list, Shading Rate,
Refinement Shading Rate / Threshold / Refine Bucket Borders, Ray
Threshold, Boost Importance, Reflection / Refraction / Subsurface /
Spectral sample multipliers (Reflection / Refraction / Subsurface 64 `[S]`), Shading Rate 0.5 px `[S]`, Direct Light Samples (64 `[S]`), MIS, Shadows switch (on `[S]`),
the whole Global Illumination tab (ambient intensity and colour, indirect
enable / samples / bounces / range, indirect SSS and volumetrics,
irradiance caching and its file save / load, photon caustics). `[D]`

For the record, the legacy Global Illumination tab's new-scene values
`[S]`: Ambient Intensity 0 W/sr/m² (so Ambient Color 1, 1, 1 is greyed);
Indirect Illumination on, Indirect Samples 64, Bounces 1, Range 0 m (no
limit), Indirect SSS "Indirect Affects SSS", Indirect Volumetrics off;
Environment Importance Sampling on, 256 samples; Irradiance Caching on,
Usage "First and Second Bounce", 256 samples, Rate 2.5 px, Ratio 6.0,
Walkthrough off; Direct Caustics off, Total Photons 100 000, Local
Photons 32, Walkthrough off. Settings tab mirrors: Environment Samples
256, Irradiance Samples 256.

## 7. GUI checks for the builder's QA list

1. Click Render in the Shader Tree: Properties shows Frame, Settings,
   Preview, User Channels, Tags.
2. Click a material, then Render again: tab you were on is kept (UIC-21).
3. Change Frame Width to 1000: Camera view bands change; F9 renders 1000
   wide. Turn on the camera's Override Render Resolution: camera wins.
4. Set Maximum Quality 4: F9 is visibly noisy. Undo: back to before.
5. Old scene saved before this change: renders exactly as before.
6. Render Region on, Right 50 %: only the left half renders. Drag a box in
   the render window: the four edges update and Render Region turns on.
7. Refraction Depth 1 on a glass of water: goes dark; 8 restores it.
8. Indirect Caustics None vs Both on glass on a floor: caustic patch goes.
9. Save, close, reopen: every value kept.
10. Convergence Target 50 %: Preview says Finished at about half the
    samples. Max Render Time 0.1: Preview stops after about 6 s; F9 doesn't.
