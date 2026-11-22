# One-step Blender scene import via USD (build spec v1), 2026-11-18

Approved by Jason 2026-11-18 ("sure"). Saved as given. Provisional
readings are USD-IQ01..IQ14 in `docs/SPECIFICATION_QUESTIONS.md`; the GUI
check is `docs/USD_IMPORT_GUI_QA.md`.

**Route:** USD import, using the OpenUSD library the USD export already
links. **Licence:** OpenUSD is Apache-2.0 with a trademark clause, checked
2026-11-11 as GPL v3 compatible. Ship Pixar's LICENSE and NOTICE, and no
Pixar branding. Code adapted from Blender's USD importer
(`io/usd/intern/usd_reader_*`) is allowed (Blender reuse rule). It goes in
the GPL-2.0-or-later bridge folder (`operandi-usd/bridge/blender/`), with
SPDX headers and an "Adapted from Blender `<file>` at `<commit>`" line.

**Why USD, not glTF:** glTF forces triangles. USD keeps quads and n-gons,
hierarchy, instancing, cameras, lights, UV sets, colours and custom normals,
and Blender can write materials as UsdPreviewSurface and MaterialX. Our
MaterialX/OpenPBR reader already exists.

**Blender export settings we target** (4.2+/5.x, File > Export > USD):
.usdc (or .usdz), Mesh + Lights + Cameras + Materials on, UV Maps on,
Normals on, Color Attributes on, Instancing on, Evaluation = Render,
Materials: USD Preview Surface on + MaterialX on, Export Textures on,
Relative paths on. We read upAxis and metersPerUnit.

## Stages

**U0 Ship USD.** Turn on the `usd` feature for QA builds on Mac (Windows and
Linux later), and bundle the OpenUSD libs + plugInfo + Pixar LICENSE/NOTICE
(Resources/licenses). Report the app size change.

**U1 Geometry + scene graph.** UsdGeomMesh becomes mesh items, keeping
quads/n-gons (faceVertexCounts), orientation/winding, and
subdivisionScheme (catmullClark = Operandi subdivision on, none = off). All
UV primvars (st, st1…) become named UV maps. displayColor and colour
primvars become vertex colour maps. Normals become custom normals if
Operandi supports them; otherwise smoothing, and report it. Bring creases
and corners in if present. The Xform hierarchy becomes parented items with
local transforms (no baking), named after the prims. Instanceable prims and
PointInstancer become Operandi instance items if those exist, otherwise
copies (report the count). Convert upAxis/metersPerUnit to Y-up metres.
Invisible prims come in hidden. The whole import is one undo step.

**U2 Materials.** Bound materials (including GeomSubset per-face bindings)
become Material Groups plus polygon tags named after the material. Prefer
the MaterialX network (via our existing MaterialX reader,
OpenPBR/standard_surface). Otherwise use UsdPreviewSurface (diffuseColor,
roughness, metallic, normal, emissive, opacity/opacityThreshold, clearcoat,
ior) with UsdUVTexture (file, sourceColorSpace, wrap, scale/bias, the st
primvar via UsdPrimvarReader, UsdTransform2d as a Texture Locator). Resolve
texture paths relative to the layer. Extract usdz textures into the scene's
project folder. Anything that doesn't map gets reported.

**U3 Cameras + lights (later).** UsdGeomCamera becomes camera items, UsdLux
becomes lights (intensity × 2^exposure, colour, temperature, size, spot
cone via ShapingAPI), and a DomeLight texture becomes the environment
image.

**U4 Node graph sidecar (later).** A `<scene>_graphs.json` from
`tools/blender-export` next to the USD auto-replaces matching material
names with Node Material layers in the right groups.

## Import options dialog (UIC rules)

Units/up-axis override, Import cameras, Import lights, Import materials
(MaterialX / Preview Surface / none), use the graphs sidecar, instances as
instances or copies. Default: all on.

## Report after import

Counts of items, polys (quads/n-gons/tris), materials by kind, cameras,
lights and instances, plus a list of what was dropped and why.

## Acceptance

- **USD-T1** is a Blender test scene: a 3-level hierarchy with a rotated
  parent, an instanced chair ×10, a cube with 2 UV sets + vertex colours, a
  sub-d mesh with creases, a Principled material with image base colour +
  normal map + roughness map, emission, glass, a 35 mm camera, area + point
  + sun lights, and an HDRI world. Made with Blender 5.2.2 headless (it
  needs the sandbox off for Metal). It must import with hierarchy, local
  transforms, quads kept, 2 UV maps, colours and textures in the right
  colour space.
- **USD-T2:** the same scene as .usdz gives an identical result.
- **USD-T3:** Operandi's USD export re-imported gives the same
  items/tags/materials.
- **USD-T5:** a missing texture shows the missing-file look, the import
  still succeeds, and it's reported.

Also fix the `scene_import` MCP tool description, which wrongly claims
"hierarchy and transforms kept" for the other formats, and add
usd/usdc/usda/usdz to it.
