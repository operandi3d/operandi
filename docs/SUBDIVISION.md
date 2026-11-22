# Subdivision surfaces

Branch `subdivision`, 2026-11-03, from `specifications/subdivision-2026-11-03`.
Core: `crates/operandi-core/src/subdivision.rs`; drawing:
`operandi-app/src/subdivision_view.rs`; settings: Item properties.

## What works

- **Tab / Shift+Tab** (input map "Toggle Subdivision" / "Toggle
  Catmull-Clark", also Geometry > Subdivision) flip the selected polygons,
  or every polygon of the active mesh when none are selected, exactly as
  SDV-01/02 (toggle oracle passes). One undo step each.
- **Catmull-Clark** matches every PSUB oracle: cube, asymmetric cube,
  tetrahedron, pentagonal prism, impulse and open grids, levels 1–3, all
  three Boundary Rules and the semi-sharp edge weights (Edge Crease weight,
  sharpness = 10 × weight).
- **Modo Subdivision** (spec v2, SDV-04/09/10) matches every oracle
  without edge weights: closed and open meshes, levels 1–3 (one
  Catmull-Clark step, then a biquadratic patch per quad on an L × L grid).
- **Edge weights** (specifications/edge-weights-2026-11-05) work on both
  kinds with one rule. Catmull-Clark matches all 14 hand-off targets.
  Subdivision polygons use Operandi's EW-05 rule: the first step creases
  like Catmull-Clark, then edges still sharp act like open edges in the
  patch stage. A 20 % edge stays hard for 2 levels, 30 % for 3.
  Edge weights v2 (2026-11-05): any value is stored, the effect stops at
  ±100 %. Negative weights do nothing on Catmull-Clark (as Modo). On
  Subdivision polygons they run the same rule with negative fractions
  (EW2-04, interim Operandi choice): the edge sinks and softens. Not
  matched point for point; snapshot tests only.
- **Edge Weight tool** (Edge mode right-click > Modeling Operations > Edge
  Weight, or the Edge tab): drag the handle or type Weight (%), −100 % to
  100 %. Weighted edges are drawn in the theme's Edge Weight colour,
  brighter and wider as the weight rises.
- In Edge mode, selected, weighted and pre-highlighted edges are drawn
  along the smooth surface (the cage edges' traced path), not on the
  floating cage (Jason's QA, 2026-11-05).
- **Mixed kinds (SDV-14, Operandi choice):** a connected region holding
  both Subdivision and Catmull-Clark polygons is subdivided as all
  Catmull-Clark, so it stays closed.
- The viewport draws smooth polygons as their surface at the mesh's display
  level, with the cage edges on top; selection and rollover follow the cage.
- **Item properties > Subdivision**: Subdivision Level, Catmull-Clark Level
  (0–6), Boundary Rule, and **Freeze** (also Geometry > Subdivision > Freeze).
  Settings save with the scene (native version 10).

## Provisional

- Edge weights on Subdivision polygons follow Operandi's rule (EW-05), not
  Modo's, which isn't known. Expect a similar look, not the same points.
- Face polygons in a smooth mesh cut the smooth region off and keep their
  points, so a gap can open (SDV-13, matches the oracles).
- Freeze keeps UV maps as a linear subdivision of each polygon's UVs:
  exact for one-quad islands (SDV-15), provisional for larger islands
  (SDV-Q06). Freeze drops weights on frozen edges (Face polygons keep
  theirs).
- Picking uses the cage, not the smooth surface. Render levels, UV rules and
  Linear UVs are stored defaults only. Exports write the cage.
- Level drags record one undo step per change (SDV-Q09 open).

## GUI checklist (QA app: Operandi Subdivision QA)

1. Make a cube, hover the viewport and press Shift+Tab: it turns into a
   smooth rounded shape with the cube's edges still drawn around it.
2. Press Tab: back to a cube. Press Tab again: smooth again, a little
   rounder (Modo Subdivision). At level 3 a cube has 218 points after
   Freeze (Catmull-Clark: 386).
3. Polygon mode, select one face, Shift+Tab: only that face changes kind.
4. Item properties > Subdivision: change the levels and Boundary Rule; the
   surface updates. Undo steps back through each change.
5. Freeze: the surface becomes real polygons (96 for a level-2 cube).
   Undo restores the cage.
6. Save, close and reopen: polygon kinds and levels come back.
7. Click in a numeric field and press Tab: the field moves focus as before
   and nothing changes kind.
8. Edge weights: cube, Shift+Tab, Edge mode, select one edge, right-click >
   Edge Weight. Drag the handle right: the edge turns red and the surface
   pulls toward it. Type 100 %: the edge goes sharp. Type -50 %: no change
   from 0 %. Drop; one Undo removes it.
9. Same with Tab (Subdivision kind): weighted edges sharpen the same way.
10. Polygon mode: select three faces of a cube, Tab; the rest Shift+Tab.
    No gap opens along the seam.
