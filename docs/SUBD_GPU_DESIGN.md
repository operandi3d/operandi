# Subdivision surfaces at Modo-class scale (design note)

Status: prototype working, branch `claude/project-thread-pj3a5s`, 2026-11-13.
Results and what's provisional: "Prototype status" at the end.

## Goal

Jason's reference: in Modo on a Mac Studio M1 Max, a 4.5M-quad sphere at
Subdivision level 2 (144.7M triangles drawn) orbits at about 25 fps.
Operandi today can't draw it at all.

## Where we are

- `subdivision.rs` refuses any surface over 4M polygons, and the viewport
  silently draws the cage instead. Old "10M SubD at 108 fps" numbers were
  cage draws.
- With the cap lifted, a real 10M-triangle surface orbits at 79 fps but
  costs about 1.1 KB per triangle:
  - the CPU surface is a full `Mesh`: f64 points, one `Face` with its own
    `Vec` per piece, plus `sources`, `traced`, `cage_edges` (a `Vec` per
    piece each) and the point-to-piece table;
  - the GPU fill is unshared 44-byte corners (three per triangle), and a
    second CPU copy of it sits in `ItemGeometry`;
  - picking projects that whole surface again.
- 145M triangles that way would need about 160 GB.

## The idea: evaluate patches on the graphics card

Modo Subdivision (Tab, SDV-09 v2) is one Catmull-Clark step, then a
biquadratic Bézier patch per resulting quad, sampled on an L × L grid.
Every patch is a quad, and its nine control points are:

- 4 corner controls, one per point of the once-refined net,
- 4 edge controls, one per edge of that net,
- 1 centre, one per quad of that net.

Neighbouring patches share their corner and edge controls, so the surface
is watertight by construction. So instead of storing the sampled surface,
we store the controls once and let a vertex shader evaluate each sample
(position, and the normal from the patch's two derivatives) every frame.
This is the "evaluate patches on the fly" idea of Pulli & Segal (1996),
and what OpenSubdiv's GPU patch drawing does for Catmull-Clark limit
patches.

### Data, for the 4.5M-quad sphere at level 2

Once-refined net: 18M points, 36M edges, 18M quads (= 18M patches).

| Buffer (GPU) | Size |
|---|---|
| Controls, f32 xyz: 18M corner + 36M edge + 18M centre | 864 MB |
| Per patch: 4 corner + 4 edge control ids (u32) | 576 MB |
| Per patch: owning cage polygon (u32) | 72 MB |
| Per cage polygon: colour, map colour, material program | 54 MB |
| Wire: per net edge on a cage edge, its 3 control ids | 216 MB |
| **Total** | **≈ 1.8 GB ≈ 12 bytes per drawn triangle** |

That's about 90× smaller per triangle than today, and it doesn't grow with
the level: level 3 or 4 costs the same memory, only more drawing. The
per-patch ids can later shrink to ~12 B (derive them from the cage's
corner/edge tables), taking the total near 1 GB.

The CPU keeps the cage, the net's topology and the controls (f32) for
drags and picking: roughly the same again.

### Drawing

- **Fill**: one indexed, instanced draw. A small index buffer holds the
  triangles of a batch of K patches (K × L² × 2 triangles over K × (L+1)²
  grid samples); each instance is one batch. The vertex shader turns its
  vertex number into (patch, i, j), reads the patch's nine controls,
  evaluates the Bernstein sums, and hands the existing fragment shader
  (`fs_fill`) the same inputs a stored vertex would. Shared grid samples
  inside a patch are evaluated once (post-transform cache), so at level 2
  that's 9 evaluations per patch, not 24.
- **Normals**: the true patch normal from its derivatives, not the average
  of neighbouring pieces' normals. Patches meet with matching tangents
  except along creases, so shading is smooth and creases stay crisp. This
  will differ slightly from today's look; I'll show Jason both.
- **Wire**: Modo draws the cage edges traced over the surface. Each cage
  edge is two net edges, and each net edge is a quadratic Bézier (its two
  corner controls and its edge control). One instanced draw strokes them in
  L segments each with the existing `edge_out` code.
- **Per-polygon state**: selection, rollover, locked and hidden colours,
  and the material program come from a per-cage-polygon table, so a
  selection change rewrites a few entries instead of rebuilding geometry.
- **Drags**: a moved cage point changes the controls within two rings of
  it. The CPU recomputes those and uploads only the changed ranges.

### What stays on the old path (for now)

- Catmull-Clark polygons (Shift+Tab) and mixed regions (SDV-14). See
  "OpenSubdiv" below for their plan.
- Material programs that read UV maps, and the Vertex Map style, until
  the per-patch UV corners are added (SDV-15 linear UVs are bilinear per
  patch, so they fit the same scheme).
- Freeze (it needs the real polygons; it uses the CPU evaluator as now).

Items on the old path keep working; only the scale limit differs.

### Picking

Picking projects the whole drawn surface today, which won't scale. For
items on the GPU path it will use the patch corners (each patch as two
triangles through its corner samples, which lie on the surface). At level
2 that's a quarter of the drawn triangles; still large, so a later step
is a GPU id-buffer pick.

## Memory guards

- The fixed 4M-polygon cap goes. The CPU path (Catmull-Clark, Freeze)
  estimates its bytes from the polygon count it would make and refuses
  when that's over free memory (`Editor::free_memory`, as Subdivide does).
- Tab / Shift+Tab gets the same check: before the toggle, estimate what
  the surface will need (GPU path or CPU path rate) and refuse with a
  message when it's over free memory. Today Tab has no check at all.

## OpenSubdiv

OpenSubdiv (Pixar, Apache-2.0, C++) is the standard engine, and Blender's
GPU subdivision uses it: OpenSubdiv analyses topology and builds stencil
and patch tables on the CPU, and Blender's own shaders evaluate them on the
GPU. Three options:

1. **OpenSubdiv for Catmull-Clark (Shift+Tab), our own evaluator for
   Tab.** OpenSubdiv can't do Tab at all: Modo Subdivision is its own
   scheme (one Catmull-Clark step then biquadratic patches), not one of
   OpenSubdiv's (Catmull-Clark, Loop, bilinear). So we need our own
   evaluator for Tab whatever we choose. For Shift+Tab it would give
   fast, robust uniform refinement as stencil tables.
   - Costs: a C++ library built with CMake, like Cycles, on three OSes;
     Rust bindings exist (`opensubdiv-petite`, CPU side) but are thin, and
     its Metal backend can't share buffers with wgpu without unsafe
     interop, so we'd use only its CPU tables and run them in our own
     WGSL, as Blender does.
   - Behaviour risk: our Catmull-Clark rules come from the spec (SDV-05..07
     boundary rules, semi-sharp creases at 10 × weight, negative weights
     EW2-04, mixed regions SDV-14). OpenSubdiv's semi-sharp creases are
     close but negative weights and some boundary choices have no
     OpenSubdiv option, so some meshes would come out different from
     Freeze and from the spec.
2. **Our own path for both.** Catmull-Clark refinement is also expressible
   as stencils (each new point a weighted sum of old ones), which our
   existing `catmull_clark_step` already computes. Run L-1 steps as
   stencils and evaluate the last step per quad on the GPU, or keep the
   net on the GPU and apply stencil tables in compute. One renderer, one
   set of rules, exact match with Freeze, no C++ dependency.
3. **OpenSubdiv for both.** Not possible: no Tab scheme.

**Decision (Jason, 2026-11-13): option 1.** Tab gets our own GPU patch
evaluator; Shift+Tab (Catmull-Clark) uses OpenSubdiv. Shape of it:

- OpenSubdiv's CPU side only (`Far::TopologyRefiner`, uniform refinement
  `StencilTable`), linked like Cycles and recorded in DEPENDENCIES.md.
  Its stencils run in our own WGSL compute pass into the same control
  buffers, and the refined quads draw through the same per-polygon table,
  wire and picking code as Tab. One renderer; two ways of making controls.
- Freeze of Catmull-Clark polygons should then use OpenSubdiv too, so what
  you see is what Freeze makes.
- Things to settle when we get there (spec questions, not blockers):
  negative edge weights (EW2-04) and our Boundary Rule choices map onto
  OpenSubdiv options only approximately; a mixed region (SDV-14) becomes
  Catmull-Clark and so goes to OpenSubdiv as well.
- Tab comes first: it's the reference test and needs no new dependency.

## Plan

1. Core: `subdivision_patches.rs`, the compact net (f32 controls, patch
   ids, owners, wire records), built in parallel and matching
   `modo_surface` sample for sample (test).
2. App: `subd_gpu.rs`, buffers and the fill and wire shaders, hooked into
   the resident draw for Tab items. GL counter counts patch triangles.
3. Measure on the 4.5M-quad level-2 sphere (orbit fps, memory, Tab time).
4. Memory guards: drop the 4M cap, add the Tab check.
5. Then: selection/rollover table, drag patches, picking on patch corners,
   UVs.
6. Shift+Tab through OpenSubdiv stencils into the same GPU path.

## Prototype status (2026-11-13)

Built: `operandi-core/src/subdivision/patches.rs` (controls, patches, wire
records, culling clusters; matches `modo_surface` sample for sample in the
oracle, crease, n-gon and level 1–4 tests) and
`operandi-app/src/subd_gpu.rs` (buffers, fill and wire shaders, compute
culling with indirect draws). The app asks the adapter for its own buffer
limits (`gpu::wgpu_configuration`), so one storage buffer can hold ~4 GB.

Reference sphere (globe 3000 × 1500 = 4.5M polygons, Tab, level 2, 144.0M
triangles on the GL counter), headless 1600 × 1000, M1 Max:

| | before | prototype |
|---|---|---|
| What draws | the cage (4M cap) | the real surface |
| Orbit, whole frame | – | 35 ms, 28.5 fps |
| GPU per frame | – | 33 ms (fill 16, wire 18) |
| Memory for the surface | ~160 GB if uncapped | +1.6 GB |
| Tab to first frame | – | ~9 s (patch build 3 s) |

Steps that got there: patch fill + per-segment wire 11 fps; wire as one
strip per cage edge 15 fps; culling clusters that face away 28.5 fps.

Decisions in the prototype:

- **Which surfaces use patches:** Tab polygons from 500k pieces
  (`subdivision::PATCH_MIN_PIECES`, ~1M triangles). Smaller surfaces and
  Catmull-Clark keep the stored path unchanged, so editing small and
  medium SubD meshes (fast incremental drags, image materials, locked
  colours) is exactly as before.
- **Memory guards:** the 4M-polygon cap is gone. The stored path is built
  only when it fits in half the free memory, else picking falls back to
  the cage as before. Tab, Shift+Tab and Subdivision Settings refuse a
  surface needing more than the free memory, with a message
  (`Editor::check_display_memory`).
- **Normals:** the patch's own normal from its slopes, not the average of
  the pieces around a point. Pictures match the stored path to within
  24/255 on lit pixels (`subd_gpu_tests.rs`).

## Pass 2 (2026-11-13): drags, textures, wire fade

- **Drags:** `PatchState` keeps the step's topology and points. Moving
  cage points recomputes the step points, controls, clusters and wire side
  normals within two rings of them (bit for bit what a full build gives,
  `drag_updates_match_a_full_build`). Controls, clusters and sides live in
  copy-on-write chunks (`Chunks`, 16k elements), so the cache's and the
  viewport's copies share every untouched chunk and the card gets only
  the changed ones. 4.5M-polygon sphere, level 2: moving 1 point costs
  0.8 ms in the core (was a 3.5 s rebuild); a 1-point drag frame is ~42 ms
  (24 fps), a 1000-scattered-point drag frame ~72 ms.
- **Textures:** a patch polygon's material program draws per pixel, with
  its UVs bilinear over the patch's corner, side middles and polygon middle
  (SDV-15, as Freeze makes them): 8 bytes per patch, only when a program
  exists. Pictures match the frozen surface's.
- **Wire fade:** Fade Wireframe on Small Polygons fades patch wire by the
  cage edge's length on screen, keeping outline, open and sharp edges from
  the surface normals on each side of the edge (`wire_sides`). The stored
  path still never fades a subdivided cage edge.

Still provisional on the patch path:

- Procedural patterns (per-corner colours) draw their base colour.
- Locked polygons' wire keeps the plain colour; Transparency overrides
  (see-through) and Advanced shadows don't draw the patches.
- Picking and rollover use the cage once the stored surface doesn't fit.
- Tab time (~10 s at 4.5M polygons): the patch build and the cage's own
  geometry pass are not yet tuned.
