# Blender reference notes

Parts of Blender we want to consult later but are not porting yet. Status as
of 2026-11-11. The full ranked survey is the "Blender code reuse survey" doc
(https://claude.ai/artifact/XF3s7b3uHMB7LsgyB3Rtja).

Blender's core is GPL-2.0-or-later. Anything ported keeps Blender's notice and
SPDX line, adds "Adapted from Blender `<path>` at `<commit>`", sits under the
CONTRIBUTING.md GPL exception and is listed in `docs/DEPENDENCIES.md`. Don't
take anything in Blender that imitates Modo (keymaps, add-ons). Blender's
icons, themes and splash art stay out; other bundled assets only after that
file's licence is checked (the studio lights are CC0, see
`docs/licenses/blender-studiolights`).

Paths are from memory; check them against the Blender tree at the commit you
use.

## Already ported (on main)

| Area | Where in Operandi |
| --- | --- |
| UV unwrap (LSCM, ABF++, SLIM) and packing | `crates/operandi-uv` |
| Sculpt brush maths, Kelvinlets | `crates/operandi-core/src/sculpt_blender/` |
| Viewport principled material (EEVEE approximation) | `crates/operandi-core/src/blender_principled.rs` |
| Cycles scene sync (Apache-2.0 reference) | `crates/operandi-cycles` |

## Being ported (own branches, 2026-11-11)

| Area | Branch |
| --- | --- |
| Remesh: QuadriFlow quads, dualcon Blocks/Smooth/Sharp, voxel | `remesh` |
| USD export/import via OpenUSD, Blender `io/usd` as the map | `usd-io` |
| FBX (ufbx), Alembic, PLY | `fbx-abc-ply` |

## Reference later

| Area | Where in Blender | Use it when |
| --- | --- | --- |
| Cycles glue | `intern/cycles/blender` (Apache-2.0) | Adding instancing, motion blur, hair, volumes, light linking or baking to `operandi-cycles`. |
| Node viewport shaders | `source/blender/gpu/shaders/material/` (one GLSL file per node) | Each new node in Nodes N2 and later: translate to WGSL, compare with Cycles. |
| Geometry helpers | `source/blender/geometry/`: fillet curves, resample curves, subdivide/trim curves, split edges, merge by distance, mesh to curve, convex hull; 2D Delaunay in `source/blender/blenlib` | A tool needs one of these; port that one function. |
| Bevel junctions | `source/blender/bmesh/tools/bmesh_bevel.cc` (vertex mesh patterns) | Operandi's junction code hits a case it can't solve. Behaviour still follows Operandi's specs. |
| More sculpt brushes | `source/blender/editors/sculpt_paint/brushes/` | Adding brushes beyond the 11 built. Skip dyntopo (BMesh-based). |
| Subdivision | Blender wraps OpenSubdiv (`intern/opensubdiv`) | Exact limit surfaces or GPU subdivision; use OpenSubdiv directly. |
| UX ideas | Blender user manual | Any tool, alongside Operandi's own specs. |

## Skipped

Curves and text (Operandi has its own), Blender's bevel as a whole, BMesh,
DNA/RNA, dyntopo, multires, Blender icons and themes.
