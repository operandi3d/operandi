# Viewport display options, tier 1 (2026-11-03)

Operandi's own design, written from Jason's brief (via the coordinator
session). No Modo material was used. Colors, band counts and the reflection
environment are Operandi's choices.

## Scope

Each 3D view has its own display settings. Changing one view leaves the
others alone. Display changes are not scene edits: they add no undo step and
are not saved in the scene. A new view (Split View) starts from the active
view's settings.

## Groups

- **Active meshes**: the selected item.
- **Inactive meshes**: visible, unselected items.

Each group has a **shading style**, a **wireframe overlay** (None, Uniform
light lines, Colored dark lines) with an **opacity** (0–100 %), and
**smoothing** (on/off).

Overrides:

- **Make Inactive Same as Active**: inactive items draw with the active
  group's settings.
- **Make Inactive Invisible**: inactive items aren't drawn in this view
  (they stay visible in other views and in the scene).

## Styles

| Style | Look |
|---|---|
| Wireframe | Edges and points only, no fill; back edges visible. No separate overlay (its wire is the overlay, full opacity). |
| Solid | Flat, unlit fill. Usually used with an overlay. |
| Shaded | Lit fill (the default; matches Operandi's look before this work). |
| Vertex Map | Faces colored by the mesh's vertex map. A color map blends its corner colors across faces. A weight map shows positive as red, zero or unassigned as green, negative as blue. A color map wins over a weight map. No map: neutral grey. |
| Gooch | Shadowless tone shading: faces toward the light warm, away from it cool, everything readable. |
| Cel | Lighting in four flat bands. |
| Reflection | Mirror-like: a built-in procedural studio environment (sky, floor, hard horizon, light strips) reflected in the surface, with a little base color and shading, so bumps and dents show. |

## Quick toggles

- **Show Grid**: the ground grid on/off.
- **Show Wireframe in Item Mode**: in Item selection mode, every shown mesh
  draws its wire (Uniform, full opacity), overriding the group overlay.
- **Show Verts**: on draws every point of the active item in every mode.
  Off follows the **Show Vertices** setting: Always / In Component Modes /
  In Vertex Mode / Never. Selected and pre-highlighted points, and points no
  polygon uses, always draw. Default Never (Jason's MV-C03 call: off shows
  only selected and hovered points). The Vertex tab's Show Vertices checkbox
  is this toggle.
- **Show UV Seam**: draws the edges where the current UV map's islands
  break.
- **Smoothing**: off makes every polygon faceted; on shades per the
  existing smoothing rule (a polygon marked smooth averages the normals of
  the smooth polygons around each point; subdivision surfaces are smooth).

Defaults: Shaded active and inactive, Uniform overlay at 100 %, Grid on,
Smoothing on, everything else off.

## Display menu

One **Display** menu in the view header (it replaces the Shaded/Wireframe
combo and the old Display menu):

1. The styles. A check marks the active group's style, "○ inactive" marks
   the inactive group's. Click sets the active style; Alt-click the
   inactive one.
2. Active Meshes ▸ and Inactive Meshes ▸ (overlay, opacity, smoothing), then
   the two overrides.
3. The toggles, with Show Vertices ▸.
4. The presets.
5. Save Viewport Preset…, Delete Preset….

## Presets

Saved by the user, kept with the preferences (`viewport-presets.json` in
the data folder), not in scenes.

- **Save** asks for a name and which groups to include: Drawing (toggles
  and overrides), Active Meshes, Inactive Meshes. Saving with an existing
  user preset's name replaces it. The saved preset becomes the view's
  checked preset.
- **Choosing** a preset applies its groups to the view and checks it. If any
  setting it holds changes afterwards, it shows a "*". Choosing the checked
  preset again unchecks it and restores the view's settings from before it
  was applied. Choosing another preset keeps that first restore point.
- **Delete** picks one user preset to remove. Built-ins can't be deleted.
- Built-ins: Shaded Wire, Solid Wire (Colored lines), Wireframe.

## Commands

`shaded view` and `wireframe view` (and the Numpad4 / Numpad7 shortcuts)
set the active view's active style.

## Provisional

- Vertex Map uses the first color map, else the first weight map (by
  name); Operandi has no "selected vertex map" yet. Typed maps are matched
  by kind name (contains rgb/color, or weight/wght). Subdivision pieces show
  their cage polygon's average map color.
- UV seams: Operandi keeps no seam data, so the seam is the border of the
  first UV map's islands (an edge whose polygon sides disagree, or only one
  of which is mapped).
- Live primitive previews and Cone/Ellipsoid previews draw on the
  projected (CPU) path. They follow the view's style with one colour per
  face (Solid, Shaded, Gooch, Cel, Reflection; the same formulas as the
  shader, display_shading.rs); Vertex Map previews stay neutral grey.
- Gooch's highlight (Blinn-Phong, exponent 32, strength 0.5) and dark
  silhouette rim (where the normal turns 0–0.25 from edge-on to the eye)
  are Operandi's own tuning. Gooch and Reflection colours are editable in
  Display > Colors > Display Styles.
