# MeshOps: procedural mesh stack — behaviour spec v1 (2026-11-11)

Behaviour only, in our own words, from public documentation and public
release notes. No screenshots, no binaries, no file internals. Tool and
option names follow the reference app's public docs (naming rule).

Rule ids are `MO-nn`. "Provisional" means the public docs don't say and we
chose; Jason can overrule. Safety rules live in `SAFETY.md` (`SAFE-nn`) and
win over anything here.

Scope split: this spec is the stack itself. MeshFusion (Fusion items,
strips, trims, Fusion assemblies) is the MeshFusion spec. A Boolean is one
op in this stack; its options are in the booleans spec.

---

## 1. The idea

- **MO-01** A mesh item can carry a **stack of operations** on top of its
  **Base Mesh**. What you see, render and export is the stack's result. The
  Base Mesh is untouched by the ops.
- **MO-02** The stack evaluates **bottom to top**: Base Mesh first, then each
  op in turn, each one taking the result of the op below it.
- **MO-03** Change anything (an op's value, the Base Mesh, the order, an
  input mesh) and only the ops **at and above** the change re-evaluate. Ops
  below keep their cached results. Editing the top op is as cheap as running
  that one tool.
- **MO-04** Order matters and is shown: Bevel then Chamfer is not the same
  as Chamfer then Bevel.
- **MO-05** The stack is the saved truth. The result is a cache: rebuilt on
  load, never edited directly (see SAFE-03).
- **MO-06** A stack may start from an **empty** Base Mesh. A procedural
  primitive (Cube, Cylinder, Star curve, Text…) is just an op at the bottom
  that makes geometry. Swapping it (Star → N-Sided) keeps every op above.

## 2. The Mesh Operations list

### 2.1 Where it lives
- **MO-10** A **Mesh Ops** tab sits beside the Item List in the right-hand
  panel (Model layout). It shows the Item List on top, the **Mesh Operations
  list** below, and the selected op's **Properties** below that.
- **MO-11** Rows: each mesh item, expandable. Expanded, it lists its ops
  top-down in evaluation order (top = last evaluated) with **Base Mesh** as
  the bottom row.
- **MO-12** Each op row has: an **eye** (enable/disable; disabled = the op
  passes its input straight through), its name, and an expand arrow that
  reveals its **inputs**.
- **MO-13** Inputs depend on the op: **Selection** (most edit ops),
  **Tool Pipe** (modifiers, falloffs), and op-specific ones such as **Curve**,
  **Geometry** or **Sources** (meshes the op reads from). A Cube has only a
  Tool Pipe; a Polygon Bevel has Selection and Tool Pipe.
- **MO-14** Each input row offers an add button (Add Selection, Add Tool
  Pipe, Add Falloff…). An entry in an input has, on hover, a remove (x) and
  a swap control (replace with another type).

### 2.2 Adding ops
- **MO-15** **Add Operator** button above the list opens the procedural
  preset browser (Operandi's preset browser, Stage 1 spec, in a procedural
  mode). A small arrow beside it opens the same choices as a dropdown menu.
- **MO-16** With a procedural mesh selected, **Tab** over the list opens Add
  Operator. Provisional: Tab only does this while the pointer is over the
  Mesh Operations list, so the viewport's Tab (Sub-D toggle) is unaffected.
- **MO-17** Browser folders: **Deformers**, **Mesh Operations** (sub-folders
  Create › Primitives / Curves, Curves, Deform, Duplicate, Edge, Edit,
  Polygon, Selection, UV, Vertex, Vertex Map, Other), **Tool Pipe**,
  **Assemblies**. Search box with Search All / Search View. Double-click adds.
- **MO-18** Where a new op lands: directly **above the selected row**. With
  the mesh item or nothing in its stack selected, it lands on top.
  (Provisional; the docs show "select Base Mesh, add op" and "link out of an
  op inserts above it".)
- **MO-19** Adding an op selects it, shows its Properties and its viewport
  handles. Dragging in the viewport straight away hauls the op's main values
  (no separate tool activation, no haul UI drawn).
- **MO-20** **Auto-capture of selection:** if components are selected on the
  mesh when you add an op that takes a selection, the op gets a
  **Select by Index** holding those components. No selection = the op
  works on the whole mesh.

### 2.3 Editing the list
- **MO-21** Drag rows up/down to reorder; an insertion line shows where it
  will land. Delete key removes the selected op(s) with their private inputs.
- **MO-22** Shift-click range-select, Cmd/Ctrl-click toggle (same as Item
  List). Several selected ops show their Properties together.
- **MO-23** Trying to make an op an input of itself (directly or through a
  chain) is refused with a **dependency loop** warning; nothing changes.
- **MO-24** Context shortcuts while an op is selected: **Shift+S** Add
  Selection, **Shift+F** Add Falloff, **` (backtick)** open the op's input
  tree. Builder must check these against Operandi's map (QUESTIONS Q4).
- **MO-25** List header buttons: **Ghosted view**, **Hide**, **Single item
  view**, **Filter** (name search).

### 2.4 Looking at the middle of the stack
- **MO-26** **Ghosted view** (on by default): select an op that isn't on top
  and the viewport shows the result **up to that op** normally, with the
  full result drawn over it as a tinted see-through ghost. Hide (also on by
  default) hides the full result instead of ghosting it. Ghost tint/style is
  a preference.
- **MO-27** Selecting the Base Mesh (or **Edit Base Mesh** from the Item
  List right-click menu) puts you on the Base Mesh with the ghost of the
  full result over it. Direct modelling tools then edit the Base Mesh and
  the stack re-evaluates on top (see SAFE-05 for the guard rail).
- **MO-28** **Single item view** (default on): the list shows only the
  selected mesh's stack. Off: every procedural mesh in the scene.

### 2.5 Performance controls
- **MO-29** **Defer**: heavy stacks update on mouse-up instead of during the
  drag (the selected op still previews live in ghosted view). **Pause**:
  no stack re-evaluates at all until unpaused.
- **MO-30** Both are global buttons in the list header. Each mesh item has a
  **Deferred Evaluation** setting: Default (follow global) / Never / Custom
  (During Interaction, During Playback checkboxes) / Always.
- **MO-31** **Stand-In Mesh**: a mesh item may name a lighter mesh to draw
  in its place while deferred or paused. (Later stage.)

## 3. Which tools can be MeshOps

- **MO-40** Most direct modelling tools have a procedural twin with the same
  name and core options. Some options differ between the two versions (the
  docs say so; e.g. Jitter's handles read Rigid Translate/Rotate/Scale). Each
  op's spec lists its own options; default = same as the direct tool.
- **MO-41** Groups (from the public docs, not exhaustive):
  - **Create › Primitives:** Cube, Sphere, Cylinder, Cone, Capsule,
    Ellipsoid, Torus/Toroid, Text.
  - **Create › Curves (2D):** Circle, Ellipse, Rectangle, Diamond, N-Sided,
    Star.
  - **Curves:** Curve Fill, Curve Rebuild, Curve Sweep / Sweep Effector,
    Curve Boolean, Edges to Curves, Curve Clone.
  - **Polygon:** Polygon Bevel, Smooth Shift, Polygon Extrude, Polygon
    Merge, Flip Polygons, Symmetrize, Triangulate, Subdivide, Unsubdivide,
    Polygon Unify, Material Tag, Part Tag, Create Polygons.
  - **Edge:** Edge Bevel, Edge Chamfer, Edge Slide, Edge Split, Edge
    Extrude, Edge Relax, Edge Remove.
  - **Vertex:** Vertex Bevel, Vertex Extrude, Create Vertex.
  - **Edit:** Boolean, Axis Drill, Solid Drill, Slice, Axis Slice, Loop
    Slice, Curve Slice, Pen Slice, Bridge, Curve Effector, Merge Meshes,
    Freeze (the op — see §6.3), Polygon Reduce.
  - **Duplicate:** Array, Radial Array, Mirror, Clone, Scatter Clone, Curve
    Clone, Pen Clone.
  - **Deform:** Jitter, Push, Smooth, Surface Constraint, UV Transform.
  - **UV / Vertex Map:** Create UVs, UV Unwrap, Pack UVs, UV Fit, UV
    Rectangle, Transfer Vertex Map.
- **MO-42** Things that are **not** ops: selecting, item transforms, the
  interactive drawing tools (Pen, Sketch) — they act on the Base Mesh.
- **MO-43** For Operandi: an op exists only once its direct tool exists and
  its registry entry is in (UIC-165, registry thread). See `SCOPE.md` for
  the proposed first batch.

## 4. Selection operations

- **MO-50** A **selection op** is an input that answers "which elements does
  this op work on?", re-answered every evaluation. It lives in an op's
  Selection input; several can stack there, bottom to top.
- **MO-51** Common settings on every selection op: **Enable**, **Invert**,
  **Blend Mode** — **Override** (default; replaces what's below it),
  **Add**, **Subtract**, **Multiply** (keep only elements both select).
- **MO-52** Element type must match the op. An edge op fed a polygon
  selection does nothing (no error, no geometry change) until a **Convert**
  is added. Provisional for Operandi: show a small warning on the op row
  ("Selection is Polygons, this op needs Edges").
- **MO-53** Types:
  - **Select by Index** — stored element numbers (ranges like `0-9,12,18-30`),
    with Set / Clear / Add / Remove buttons that use the current viewport
    selection. Warn in Properties: numbers can point at different elements
    once something below changes topology.
  - **Select by Range** — Start, End, Step.
  - **Select by Pattern** — a repeating True/False list (`1,0` = every
    other). Accepts True/False, 1/0, Yes/No.
  - **Select by Previous Operation** — Source Item (an op lower in the
    stack) + Name (a group that op produced: e.g. a primitive's Caps / Top /
    Bottom / Sides / All; Split's Caps / Cap Side A / Cap Side B; Subdivide /
    Triangulate / Axis Slice New Edges; Unify's Unified Polygons; clone ops'
    Cloned Polygons or Clone n; 2D primitives' Curve; Boolean's
    Intersecting Edges). **The robust one**: follows topology changes.
  - **Select by Falloff** — a falloff input; elements whose weight is above
    **Threshold** are selected.
  - **Select by Border**, **Select Connected**, **Select Coplanar Polygons**
    (Angle, Use Connectivity, Height Range), **Select by Hard Edges**
    (Hard/Soft), **Select Vertex Map Value** (Map Type, Map Name, Method,
    Component, Min Value), **Select By Polygon Tag** (Material / Part /
    Selection Set + Value), **Select by Selection Set** (Name), **Convert**
    (Convert From; keeps the source and adds the converted elements).
- **MO-54** **Assign Selection Set** is an op (not an input): it writes a
  named set onto the mesh at that point in the stack, so ops higher up can
  read it with Select by Selection Set. It must sit below its readers.

## 5. Tool Pipe (modifiers)

- **MO-60** An op's **Tool Pipe** holds modifiers that change how it acts,
  as the direct tool pipe does: falloffs (Linear, Radial…), Symmetry, Path
  Generator (e.g. Text along a curve). Falloff items in the scene can be
  wired in and are evaluated like falloff tools.
- **MO-61** The add panel splits **Existing** (already in the scene,
  shareable) and **New**.

## 6. Deformers vs MeshOps, Freeze

### 6.1 Deformers
- **MO-70** A **deformer** only moves points; a **MeshOp** may change
  topology. Both live in the **same list** and mix in any order (a Twist can
  sit below a Bevel or above it).
- **MO-71** Deformers can be animated and have falloffs/influences; a plain
  Locator deformer gives a procedural, revisitable transform.
- **MO-72** The viewport's **Enable Deformers** toggle (O key) is off in
  modelling layouts by default and on in Animate/Render. Provisional for
  Operandi: it hides **deformers only**; MeshOps always show (Q3).
- **MO-73** Operandi has no deformer items yet (its Deform tab tools are
  destructive). Deformers in the stack are a later stage.

### 6.2 Freeze (collapse the stack)
- **MO-74** Right-click an op › **Freeze**: that op and everything below it
  are baked into a new Base Mesh; ops above stay live on top of it. Freezing
  the top op = plain mesh, no stack.
- **MO-75** **Duplicate and Freeze**: same, but on a new mesh item; the
  original keeps its full stack.
- **MO-76** Freeze is one undo step. Undo gives the exact stack and Base
  Mesh back.
- **MO-77** Freezing keeps UVs, vertex maps, selection sets, tags and
  materials that the evaluated result had (a public bug lost UVs here).

### 6.3 The Freeze *op* (different thing, same name)
- **MO-78** A **Freeze** MeshOp converts non-face polygons (text, curves,
  B-splines) into faces, 2-point polygons or polylines (Freeze Curves as,
  Make Holes, Drill Axis, Subdivide Curves with Fixed Spans, Curve
  Refinement Angle), so later ops get real polygons.

## 7. Procedural items, duplicates and cross-item links

- **MO-80** **Merge Meshes** op pulls other mesh items' results into this
  stack. **Boolean** takes a second mesh (the booleans spec covers options).
  These are the cross-item links; they must follow SAFE-07..09.
- **MO-81** **Duplicate Procedural Stack** (Ctrl+D on a procedural mesh or
  op): copies the mesh with its whole stack and private inputs. Options, all
  off by default: Clone Connected Meshes, Clone Merge Mesh Inputs, Clone
  Connected Channel Links.
- **MO-82** **Instances** of a procedural mesh show its result; they are
  not editable themselves.
- **MO-83** Later stage: **MeshOp Stack** (a grouping op holding ops, can
  nest), reusable stack **aliases/assemblies**, schematic view of stacks,
  animation result caching.

## 8. Saving

- **MO-90** The file stores: Base Mesh, ops in order, every op's **full set
  of values** (never "default"), inputs and their links by item id, enable
  states. Not the evaluated result (optional speed cache only, always
  re-derivable; SAFE-03).
- **MO-91** Saves reopen to the identical result (determinism, SAFE-11).
