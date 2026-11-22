# Proposed build order and rough size

Sizes are relative to past Operandi build batches (one "batch" ≈ one
builder thread's merged chunk, like one command-registry stage).

## Stage 0 — the plumbing (biggest; ~3 batches)
- Op stack data, evaluated cache, evaluator with SAFE-01..04, 10..15.
- The `.mesh` reader audit (base vs evaluated) + lint.
- Mesh Ops tab: list, Base Mesh row, eye, reorder, delete, Add Operator via
  the preset browser, Properties, ghosted view + Hide, red failure rows.
- Edit Base Mesh, the SAFE-05 guard, Freeze and Duplicate and Freeze.
- Save/load (new format version), undo, stress harness (SAFE-16).
- First ops, chosen because their kernels already exist and are pure:
  Cube, Sphere, Cylinder (as ops), Polygon Bevel, Smooth Shift, Mirror,
  Array, Subdivide, Triangulate.

## Stage 1 — selection ops (~1 batch)
- Auto-capture Select by Index, Select by Range, Pattern, Border,
  Connected, Selection Set + Assign Selection Set, Convert, blend modes.
- Select by Previous Operation with output groups for the Stage 0 ops.
- Select by Falloff + Tool Pipe falloffs.

## Stage 2 — more ops + cross-item (~2 batches)
- Wrap remaining existing kernels in groups (Edge, Vertex, Curves, UV).
- Merge Meshes, Boolean op (uses the booleans engine), SAFE-07..09 tests.
- Defer / Pause, per-mesh Deferred Evaluation.

## Stage 3 — deformers in the stack (~1–2 batches)
- Deformer items (Locator, Bend, Twist, Taper…), Enable Deformers toggle.

## Later
- MeshOp Stack grouping and nesting, aliases/assemblies, stand-in meshes,
  Duplicate Procedural Stack options, animation result caching, schematic.

## Order vs the current roadmap
Current order: Booleans → MeshFusion → Paint. Recommended: **Booleans
(direct tool) → MeshOps Stages 0–1 → MeshFusion**. MeshFusion-style live
booleans need the same evaluate-from-recipe engine and safety rules; build
that engine once, under the simpler feature, then put MeshFusion on it.
