# Render item settings: hand-off (2026-11-11)

v1.2 (same day): new-scene defaults from Jason's panels filled in
(path tracer: 64 samples, 10 % noise, depths 8 / 4 / 8), Preview tab
added. See the top of BEHAVIOR-RENDER-ITEM.md.

Behaviour-only spec for the **Render** item, the top node of the Shader
Tree. Clicking it shows the scene's render settings in the Properties
panel. Clean room: built from the public user guide plus new-scene default
values Jason read off the panel, all written as plain facts in our own
words. No reference-app images, no internals, no bridge readback.
REA-assisted: no.

Files:
- `BEHAVIOR-RENDER-ITEM.md`: every row, by tab and section (IDs RI-xx).
- `CYCLES-MAPPING.md`: which Cycles setting each row drives, and how the
  three settings Operandi already has move into this panel.
- `QUESTIONS.md`: open points RI-Q01.., each with a pick to build on.

## The big call (read this first)

The reference app has **two renderers** and the Render item's tabs change
with the choice:

- a legacy renderer (irradiance caching, shading rate, buckets) with three
  tabs: Frame, Settings, Global Illumination;
- a modern path tracer, where the Global Illumination tab disappears.

Operandi renders with Cycles, which is a path tracer. So this spec copies
the **path-tracer layout**: tabs Frame, Settings, Preview, User Channels,
Tags; no Renderer dropdown, no Global Illumination tab. Legacy-only rows are listed once
under "Never" so nobody wonders where they went (RI-Q01).

## Build tiers

- **Tier 0, wiring (do first):** clicking Render selects the Render item
  and shows its panel; move the three existing settings (Samples, Denoise,
  Adaptive Threshold) into it; Frame Width / Height become real render
  settings (today the frame is fixed unless a camera overrides it);
  Render Camera picker. Save/load in the scene file.
- **Tier 1, small and worth it now:** Maximum Quality, Noise Threshold,
  Diffuse / Reflection / Refraction Depth, Maximum Radiance, Indirect
  Caustics, Pixel Aspect Ratio, Preview tab (Convergence Target, Max
  Render Time), Render Region (L/R/T/B, tied to the
  existing render-window Region), resolution presets.
- **Tier 2, nice to have:** Ray Tracing Engine (CPU / GPU), Antialiasing
  Filter, Resolution Unit + DPI, Adaptive Subdivision + Subdivision Rate,
  Micropoly Displacement + Displacement Rate + Displacement as Bump,
  Apply Preset.
- **Park until Operandi has animation:** First / Last Frame, Step, Frame
  Range, Output Pattern.
- **Tier 3, later:** Path Threshold, Displacement Ratio, Minimum Edge
  Length, Smooth Positions, Indirect LOD, Render Output Masking, Bake
  Width / Height, Bake Directional Effects.
- **Never:** Renderer dropdown, Shading Engine, buckets (size, order,
  reverse, write to disk), Field Rendering, Frame Passes, Shading Rate and
  Refinement, irradiance caching, ambient light, photon caustics, MIS,
  Spectral Samples, per-ray-type sample multipliers.

## Related specs

- Camera Properties (CP-V17 Override Render Resolution, CP-V36 Film Back
  Preset): the camera can override this frame size.
- Cameras and lights (CL-07 render camera, CL-19 frame region).
- Render Preview (RP-xx): its Final Render Quality and Full Resolution use
  these settings.
- Render Outputs (shader-tree-remaining): Denoising, exposure, gamma and
  tone mapping live on each output, not here.
