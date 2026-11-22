# Interchange file formats

Branch `file-formats`, 2026-11-02. General-purpose formats with public
descriptions; not Modo behavior, no parity claim. Core:
`crates/operandi-core/src/interchange/`; menus: `operandi-app/src/interchange_ui.rs`.

## Native scenes (.opscene) and big meshes

Pretty-printed JSON (`persistence.rs`). Since version 34 (2026-11-09,
branch big-scene-save) a mesh's points, polygons or a UV map with 1024 or
more entries is written as a packed object of base64 little-endian numbers
(`mesh_pack.rs`), about 160 bytes a UV-mapped quad instead of about 600.
Smaller meshes stay plain arrays; readers take either form at any version.
Writing has no size limit (it was 64 MiB); reading checks each packed
array's length and string indices and runs `Scene::validate`, and refuses
files over 4 GiB. OBJ/glTF import reads files up to 4 GiB (was 512 MiB).
Each mesh still holds at most 5 million points and polygons.

## What works

- **File > Open… / File > Import… / drag and drop** read OBJ, STL (ASCII or
  binary), glTF, GLB, PLY, FBX, 3MF, Alembic and VideoScape GEO (3DG1) into a
  new unsaved scene,
  behind the usual unsaved-work prompt. A refused file leaves the current
  scene untouched; anything skipped is listed in a result dialog.
- **File > Export ▸** writes OBJ, STL (Binary), STL (ASCII), glTF (one file,
  data embedded), GLB, PLY (Binary), PLY (ASCII), Alembic, FBX, 3MF, VideoScape GEO,
  and an SVG front view. `scene.saveAs filename:{x.ply}` exports by the
  extension (`format:` ply, plyAscii, abc and the others); the scene keeps
  its own name. Export doesn't
  change the scene; writes go through a temporary, so failures leave no file.
- Export bakes each visible item's world transform into its points; mirrored
  items are re-wound so normals stay outward. Hidden items are skipped and
  the status line says so.
- OBJ keeps n-gons, objects (or groups, if a file has no objects), `usemtl`
  names, `s` smoothing and the Texture UV map; polylines export as `l` lines.
- glTF import walks the default scene, bakes node transforms, welds points
  with identical positions and keeps per-corner UVs and material names.
- PLY reads ASCII and both binary byte orders: positions, per-vertex
  (`s`/`t`, `u`/`v`) or per-corner (`texcoord`) UVs and n-gon faces; other
  elements are skipped with a note. It writes every visible item as one mesh
  with per-corner `texcoord` lists, so UV seams survive.
- FBX import (ufbx) reads binary and ASCII FBX, converts to Y-up metres,
  bakes node transforms, keeps n-gons, the first UV set, smoothing and
  material names with their diffuse colour.
- FBX export (own writer, 2026-11-17) writes binary FBX 7.4: one model per
  visible item (transform baked in), n-gons, corner normals as the
  viewport shades them, the Texture UV map, smoothing flags and one Phong
  material per polygon tag with its colour. Metres with UnitScaleFactor
  100, Y up. Checked by reading back with ufbx; other apps' readers not yet
  tried (QA checklist). Polylines, hierarchy, cameras and lights aren't
  written.
- 3MF (2026-11-17) reads mesh objects, components and build items (placed
  copies become items, transforms baked), the model's unit and base
  material colours; colour groups and textures are skipped with a note.
  Export writes one triangulated object per visible item in millimetres,
  Z up, with a base material per polygon tag.
- Alembic (Ogawa) reads polygon and subdivision meshes at their first
  sample with transforms baked and `uv` as Texture; it writes one polygon
  mesh per visible item with UVs. Alembic's clockwise winding is reversed
  both ways.
- Materials (Set Material dialog): native scenes and glTF keep the settings.
  glTF writes Color × Diffuse as the base color, Specular as
  KHR_materials_ior, and Operandi's exact settings in `extras`; a glTF
  material from elsewhere imports with its base color at 100 % Diffuse.
  LXO and OBJ keep only the tag names for now (MAT-Q06, MAT-Q07).
- Non-manifold or mismatched-winding edges import as protected exceptions
  (tools refuse to edit them) and zero-area polygons as recorded degenerate
  faces, instead of failing the import.

## Provisional

- No axis or unit conversion: Operandi is Y-up, which matches OBJ and glTF;
  STL and GEO coordinates are taken and written as-is.
- Imported scenes are new documents; Import doesn't merge into the open scene.
- Import runs on the main thread (no progress bar or Cancel, unlike LXO).
- glTF smooth flags come from whether a triangle's stored normals differ.
- SVG is a fixed front view, not the current viewport camera.
- Not carried: OBJ normals and lines/points on import, textures, vertex
  colors, skins, morphs, animation, glTF lines/points, FBX cameras/lights/
  deformers, Alembic animation/curves/points/cameras/materials, PLY
  colours and normals. PDB not started.
- PLY and Alembic carry no unit, so numbers are metres; FBX units come
  from the file. Alembic HDF5 archives (pre-2013) are refused.

## LXO export

File > Export > Modo Scene (LXO) writes the layout in
`specifications/lxo-export-2026-11-03` for Modo 17.1v1: meshes, names, FACE
polygons, position / ZXY rotation / scale in the setup action, parenting,
hidden state, UV maps with seams, material and part labels. Unlike the flat
formats it keeps item transforms and writes hidden items hidden. Tests check
the bytes against the spec's examples and decode the file back (oracles
LXE-O-02, 05, 07, 09, 12, 14). Operandi reopens its own LXO files: the
importer admits this layout as a third producer profile (spec v2,
BEHAVIOR-IMPORT-PROFILE.md), and tests check export, import, same scene,
byte-identical second export (LXE-IMP-06). Provisional: only Modo 17.1v1 was
tested (LXE-Q01); files Modo has re-saved are still refused (LXE-Q10);
uncovered content is left out with notes (SPECIFICATION_QUESTIONS.md).

## LXO import of Modo 8–10 files

Format 4.0/4.1 files open too (spec `lxo-legacy-import-2026-11-03`): APPV
optional, the observed LAYR variants, 11-bit polygon counts, unflagged
channel types, transforms from the edit then setup action, cache chunks
skipped. A mesh with non-manifold edges, zero-area polygons or repeated
corners is skipped on its own with its counts (LEG-14). SUBD/PSUB polygons import as plain polygon cages in every profile,
with one note per file (Jason, 2026-11-03). Formats 2.x/3.x get "made with
Modo older than 8". Tests use the spec's four Operandi-made fixtures; the
local sample-pack check (LOCAL-CORPUS.md) hasn't been run here, since those
files are Modo content outside the repo. Provisional readings are in
SPECIFICATION_QUESTIONS.md.

## USD export (stages 1–3, branch usd-io, 2026-11-11)

File > Export ▸ USD (.usd), USD ASCII (.usda), USD Binary (.usdc) and USDZ
(.usdz), or
`scene.saveAs filename:<x>.usd` (the extension names the format;
`format:usd|usda|usdc` works too). Written through OpenUSD 26.08
(`crates/operandi-usd`, built with `scripts/build-usd.sh` and
`--features usd`); a build without it says so. Not Modo behaviour; the
layout follows Blender's exporter.

- `/root` (the default prim) holds the scene. Each item is an Xform with
  translate, rotate (in the item's own order: `rotateZYX`, or `rotateZXY`
  with ZXY on) and scale ops, its children below it, so the hierarchy and
  transforms survive. Tests compare USD's world matrices with Operandi's.
- A mesh item's polygons are a Mesh prim inside its Xform: n-gons as they
  are, corner normals (plain polygons), the Texture map as `primvars:st`
  and other UV maps by name (indexed, corners of one point with the same UV
  share a value), material tags as `materialBind` geometry subsets (the
  tag's own spelling in `customData`), `doubleSided`, extent.
- Sub-D and Catmull-Clark polygons are a `catmullClark` mesh: no normals,
  edge weights as creases (sharpness = 10 × weight, Operandi's rule), the
  Boundary Rule as `interpolateBoundary` and UV smoothing as
  `faceVaryingLinearInterpolation`. A mesh mixing plain and subdivided
  polygons becomes two prims (`<name>` and `<name>_subd`).
- An item whose eye is off is written with its mesh invisible; with Auto
  Hide Child Items the Xform is invisible, hiding its children too.
- Materials (stage 2): each surface the shader tree gives a polygon
  context (item, material tag, part, selection sets) is a Material with a
  UsdPreviewSurface under `/root/_materials`, named after its tag; equal
  surfaces share one. The first polygon's material is bound to the whole
  mesh and each tag subset gets its own. Inputs: diffuseColor, metallic,
  roughness, ior, clearcoat, clearcoatRoughness, emissiveColor (Luminous
  Color × Intensity, left out when dark), opacity (Alpha × (1 − Transparent
  Amount), left out when opaque) and normal (normal maps, read −1..1).
- Image layers become a UsdUVTexture reading the mesh UV primvar (through
  a UsdTransform2d for repeat and rotation), with sRGB or raw colour space
  as the renderer reads them, wrap from the layer's tiling, and a constant
  tint, blend or `1 −` as the texture's scale and bias. Pictures are copied
  into `<name>_textures/` beside the file and named relative to it; a
  .usdz holds them inside (OpenUSD's packager files them as `0/<name>`).
  Missing picture files are left out with a note.
- Cameras and lights (stage 3) are Camera and UsdLux prims inside their
  item's Xform, so they sit and point where the item does (both look and
  shine down local −Z). Cameras: focal length, film size and offset in
  USD's tenths of a metre, clipping, f-stop and focus distance with Depth
  of Field on, orthographic too (cylindrical ones go out as perspective).
  Lights: directional → DistantLight, point and spot → SphereLight (spot
  cone as ShapingAPI), area → RectLight or DiskLight (elliptical ones as
  round disks), cylinder → CylinderLight, portal → PortalLight, dome →
  DomeLight, photometric → SphereLight with its IES file; colour, Affect
  Diffuse/Specular and Dissolve carried. Power in Blender's USD units from
  the Cycles quantities (USD-Q08). Mesh lights are left out with a note.
- The environment that lights the scene becomes a DomeLight in
  `/root/Environment`, its lat-long picture baked from the environment's
  colours at 256 × 128 into `environment.hdr` (USD-Q09).
- Up axis: Y or Z as the scene's; an X-up scene is written Y-up with a turn
  on `/root`. Metres per unit 1.
- Writes go to a hidden temporary in the same folder, renamed when
  complete; a failed export leaves the old file. `.opscene` names refused.

Provisional (USD-Q01..Q04 in SPECIFICATION_QUESTIONS.md): Crease Edges
boundary written as `edgeOnly`; UV Boundary Rule mapping; Sub-D written as
Catmull-Clark; negative edge weights and polylines left out; USD-Q05..Q07
for materials (an image's alpha blend taken as opaque, procedural or
layered inputs written as their average, one material per tag subset).
Not in UsdPreviewSurface, so left out with a note: bump, displacement,
subsurface, sheen, transmission. Not yet: shared
meshes as instances (stage 4), import (stages 5–6), Windows/Linux and
release builds (stage 7). Subdivision levels aren't written (USD has none).

## GUI checklist (QA app: Operandi File Formats QA)

Samples are in `~/Developer/Operandi-builds/file-formats-qa-samples/`.

1. File > Import… `cube-and-pentagon.obj`: two items (Cube, Pentagon), the
   pentagon is one 5-sided polygon, the cube's faces carry material Blue.
2. File > Open… `cube-ascii.stl` and `cube.geo`: each opens as a closed cube.
3. Drag `cube.geo` onto the window: same result. With unsaved edits, the
   Save/Discard/Cancel prompt appears first.
4. Make a cube, move and rotate it, then File > Export ▸ each format to a
   scratch folder. Reopen each file (except SVG): the cube sits where it was.
5. Open the exported .glb or .gltf in macOS Quick Look or a web glTF viewer:
   shaded cube, no inside-out faces.
6. Open the exported .svg in a browser: a gray front view of the cube.
7. Hide an item and export: the status line says the hidden item was skipped.
8. Export a scene with a parent, a child and a hidden item as Modo Scene
   (LXO) and open it in Modo 17.1: same hierarchy, positions, rotations,
   hidden state, UVs and material labels. Then File > Open the same file in
   Operandi: the scene comes back as exported.
9. File > Open a Modo 8–10 sample scene: meshes appear where Modo shows
   them; subdivision meshes appear as their cages; a result dialog lists
   anything left out.
10. Rename a text file to `broken.stl` and open it: an error dialog, and the
   current scene is unchanged.
11. Make a cube and a sphere with UVs, File > Export ▸ PLY (Binary), PLY
   (ASCII) and Alembic. Reopen each: same shapes where they were (PLY as
   one item, Alembic as two), UVs intact in the UV view, nothing inside out.
12. Open the exported .abc in Blender (File > Import > Alembic) and the .ply
   in Blender or MeshLab: same shapes, faces outward.
13. File > Import an FBX exported from Blender or another app: items named
   after its objects, at the right size and place, material colours shown.
14. USD (QA app built with `OPERANDI_USD=1`): make a cube, a Sub-D cage with
   an edge weight (a child of the cube) and a second item moved and
   rotated. File > Export ▸ USD (.usd), then press Space on the file in
   Finder: Quick Look shows the items in place, the cage smoothed. Export
   USD ASCII (.usda) and open it in a text editor: `def Xform "root"`, one
   Xform per item, `subdivisionScheme = "catmullClark"` on the cage. If
   Blender is installed, File > Import > USD: same hierarchy, positions,
   smooth cage with its crease, UVs in `st`. Then give the cube a red
   material on one face and an Image Map layer, and export USDZ (.usdz):
   Quick Look shows the red face and the picture on the rest.
