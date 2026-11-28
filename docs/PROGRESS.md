# Milestone progress ledger

- **Kit framework, slice 1 (2026-11-27, branch kits-framework-1):** kit
  API 1 (docs/KITS.md). `kit.toml` manifest with strict checks, a Kits
  folder in the data folder, `kits.toml` for which kits are on (off after
  install), install from zip or folder, and importers run as helper
  programs (protocol 1: `<helper> import <in> <out>`, glTF/PLY/FBX/USD
  back, `note:` lines, stderr tail on failure, time limit). Kit file types
  join File > Import and drag-and-drop (KIT-1). `operandi.kit.list`,
  `.install`, `.enable`, `.disable`, `.reload`, `.openFolder`; example kit
  `examples/kits/off_import`, `scripts/package-kit.sh`. Not yet: Kits
  window and trust prompt (commands only), import progress and Cancel
  (the import blocks while the helper runs), exporters, kit commands, MCP
  sidecar imports through kits. Guide: scripting/kits.md.
- **Action Center rule sheet v6, step 2: presses (2026-11-27, branch ac-rewrite-2, GitHub #32, #39):**
  with a transform tool on, a press off the handle follows the Action
  Center's rule (`ActionContext::press`, `element_center.rs`): Element
  picks the polygon, edge or point under the pointer on every press, on
  any shown mesh, without touching the selection (A-ELEM: Z on the
  normal); None, Automatic and Screen place the centre on the view's work
  plane on every press, snapped at the pointer (Screen never); every
  other centre stays put; a handle always wins. Element and Screen are
  in force when chosen, with no waiting click. Not yet: Item mode, the
  menu table, the preview through the pivot list. Provisional:
  ACR-Q11..Q20.
- **Action Center rule sheet v6, step 1 (2026-11-27, branch ac-rewrite-1):**
  Selection and Selection Border axes are the principal axes of the moved
  points (A-SEL, `selection_axes.rs`): X the longest direction, Z the
  thinnest; they never refuse a selection. Selection Border's centre is
  the box middle of the edges shared with unselected polygons (C-BORDER;
  Smooth Shift's Scale too). Under Symmetry the + side gives centre and
  axes and drives the edit, the other side mirrors (S-01..S-04). Element
  is the world origin with World axes until a pick. Not yet: the preview
  through the pivot list, press routing, the menu table, Item mode,
  falloff side sampling. Guide: interface/action-centers.md.
  Provisional: ACR-Q01..Q08.
- **Bug batch: grid depth, Text tip, tooltips (2026-11-25, branch claude/project-thread-mnzzuz):**
  the grid draws after the solid meshes, depth-tested (Perspective and
  orthographic depth both match the meshes'), so it shows over geometry
  below or behind its plane; with backdrops the old order stays (BD-10,
  A5). The paint/sculpt Text tip field keeps a draft while typing and
  commits on Enter or focus loss. Toolbox and form-row hover text drops
  command lines (UIC-04) and spec codes (`ui_text::hover_text`). Vertex
  Collapse on a cube corner (5 points, 3 quads) is CMD-B3-02, not a bug.
- **Twist Bevel options from the TwistBevel notes (2026-11-25, branch
  claude/project-thread-cy3fzd):** opt-in Stop Before Fold (a folding cap
  stops just short instead of being refused), Even Interior (harmonic
  interior, straight runs keep their share) and Keep Cap UVs, all off by
  default and after the spec's rows. Twist Bevel now carries morphs and
  typed maps (plain Bevel still refuses them, BV-Q04). Side axis (TB-Q04)
  and the Action Axis fallback are world axes on rotated items, in the
  geometry and the handles. Plain Inset and Bevel unchanged.
- **Live MeshOp falloffs and primitive op handles (2026-11-25, branch claude/project-thread-8fccr9):**
  primitive ops edit their own falloff (type plus its values) in the op
  panel, live; selecting a primitive op shows its centre, size squares and
  falloff points in the view (`viewport/meshop_handles.rs`), and a drag
  edits the op with one undo step. Provisional: MOP-H-Q01, Q02.

- **Mesh Stack selection rules batch 1 (2026-11-25, GitHub #19, branch claude/project-thread-8fccr9):**
  thirteen rules from Steve Hill's list in `meshop_selection.rs`: Vertex
  Count (Triangles/Quads/N-gons), Area, Edge Length, Convex/Concave
  Edges by Angle, Normal, Plane, Distance, N Randomly, Random Parts,
  Vertices by Edge Count, Symmetric, UV Seam. Mesh Stack only; no Select
  menu commands until Jason polls users. Provisional: SEL-Q05, SEL-Q06.

- **Mesh Stack selections of edges and points (2026-11-25, GitHub #19, branch claude/project-thread-8fccr9):**
  ops say what their Selection input picks (`SelElement`; Edge ops edges,
  Vertex ops points); every selection op answers in that kind;
  Select by Index captures from Edge/Vertex mode with the count it was
  made on and a row hint when that changes; four rule ops (Edges by Angle,
  Border, Polygon Sides, Facing); Move Up/Down reorders selection ops.
  Guide: panels/mesh-stack-selections.md. Provisional: SEL-Q01..Q04.

- **Falloffs on primitives (2026-11-25, GitHub #15, branch claude/project-thread-8fccr9):**
  the global falloff (Linear, Radial, Cylinder, Noise) shapes the Cube,
  Sphere and Cylinder inside their shared generator. The Falloff menu
  and handles stay usable while the tool is live, and the preview follows.
  The folded Falloff section has Scale, Offset, Twist, Sample At (Spine
  or Points), Spacing (Follow Falloff) and Fit Falloff to Shape. The
  primitive ops keep their own copy of the falloff layers (taken when
  added; Use Active Falloff, Clear Falloff). Not yet: Cone and Capsule.
  Provisional: PRIM-FO-Q01..Q04, PRIM-OP-Q04.

- **Primitive MeshOps match the tools (2026-11-25, GitHub #14, branch claude/project-thread-8fccr9):**
  one generator per primitive. Cube, Sphere and Cylinder became shapes
  (`primitives/primitive_shapes.rs`) that the tools and ops both build
  from; the other primitives already had one parameter struct each.
  `meshops/primitive_ops.rs` has eight ops (adds Ellipsoid, Cone, Torus,
  Capsule, Teapot) with every tool option; op values convert to the
  tool's parameters with every field named, so a new option doesn't
  build until its op has it. No old-scene upgrade (nothing released).
  Provisional: PRIM-OP-Q01..Q03.
- **MeshBlend strips, round 2 (2026-11-24, branch claude/project-thread-20fok5):**
  flat faces cut beside strips are re-meshed (`meshblend/tidy.rs`, spade
  constrained Delaunay + seeded points, kept only if ST-10 passes), then
  edge flips and pairing; output modes (SB-50; Closed, clean is the new
  default, MB-Q07); Width Units with Strip Width % (MB-Q08; greyed rows via
  `OpDef::field_unavailable`); stable strip ids (ST-07) listed in Properties
  with per-strip Width/Profile overrides and `meshblend.strip` (MB-Q09).
  Not yet: tapering width, viewport strip picking, curved-face re-meshing.

- **MeshBlend B2, strips v1 (2026-11-24, branch claude/project-thread-20fok5):**
  rounded strips on every join with the B0 strip builder (now
  `meshblend/strips.rs`): Strip Width, Profile 0–200 %, Rows, Design Mode
  in the op form; three-patch corners; ST-10 gate (`meshblend/check.rs`)
  before a strip result is used; a width that doesn't fit is halved up to
  three times with a note (room and volume checks), else sharp joins with
  a note; cut surface triangles paired back into quads. Not yet: relative
  width, output modes, per-strip edits, quad cleanup beyond pairing.

- **MeshBlend B1 (2026-11-24, branch claude/project-thread-20fok5):**
  Geometry > New MeshBlend makes a MeshBlend Item (a mesh whose stack
  starts with the `meshblend.item` op) from the selected meshes. Recipe
  (`meshblend/recipe.rs`) holds sources, roles, relationships, per-source
  Subdivision / Keep Cage Points / In Result and the strip settings for
  B2; evaluator (`meshblend/evaluate.rs`) is a plain function; eight
  undoable `meshblend.*` commands; Properties rows per source with Add and
  Convert. Sharp joins only. Sources blend as the viewport shows them
  (Tab polygons smooth, plain polygons as they are, MB-Q06). Bad sources are left out with a red line on
  the op row (new `OpStatus::Noted`). Provisional: MB-Q01..Q05.

- **MeshBlend B0 strip spike (2026-11-24, branch claude/project-thread-20fok5):**
  no UI. `meshblend_spike/` in core: a 25-scene corpus (BLD-C01..C27
  inputs plus FAILURES cases), the ST-10 topology gate and ST-09
  repeatability fingerprint, the shared sharp stage (smooth sources,
  pre-flight with reasons, canonical source order, one Manifold boolean
  per branch with provenance), and two strip prototypes. D (own strip
  builder: cut at distance Width from the join, matched stations, profile
  loft, three-patch corners) passes ST-10 and ST-09 on every scene; 16
  cutters rebuild in about 0.2 s. B (Edge Chamfer on the join) refuses
  most joins on smooth sources. Pictures: the ignored
  `meshblend_spike::b0_comparison` test. Jason picks the method (Q07).

- **USD animation import (2026-11-24, branch usd-animation-import):**
  opening or importing a USD file brings its animation: transform time
  samples and camera and light attribute samples become linear keys on
  the channels that change (import_anim.rs; bridge reader writes
  `xformSamples`, `attributeSamples` and the stage timing). The scene
  takes the file's frame rate and authored range; time codes convert by
  framesPerSecond / timeCodesPerSecond (UsdSkel keys too). Export then
  import keeps the motion, one key per frame. Corner sharpness is still
  left out: Operandi's subdivision has nothing to hold it. Animated mesh
  points stay first-sample only. docs/USD_STATUS.md corrected.

- **STEP and IGES export (2026-11-24, branch claude/project-thread-losplx):**
  File ▸ Export ▸ STEP/IGES and `scene.saveAs format:step|iges` (.stp,
  .igs too). Tab polygons leave as exact degree-2 B-spline patches with
  shared edges (closed pieces as solids, open ones as surfaces); Face
  polygons as planes (bent ones as triangles); Catmull-Clark as
  render-level facets. Millimetres, Z up, fixed (provisional: no prefs).
  Checked in OpenCascade 7.8 (FreeCAD 1.1): valid, closed, volume equals
  the patches' exact volume. IGES is untrimmed surfaces only. Not yet:
  packed faces, import, seam-continuity measurement.
- **Materials stage 4, slice 1: MaterialX is the saved master copy
  (2026-11-24, branch materialx-master-copy, AR-32):** scene files keep
  every surface's MaterialX graph and which polygons use it (native
  version 65, `materialx/saved.rs`); the Shader Tree is saved beside them
  as the view's layout. On open, a saved graph that differs from what the
  layers now give is pinned to its polygons for renders and exports until
  those layers are edited. Old scenes open from their layers and save
  their graphs from then on. The direct layers-to-Cycles route is gone.
  Items with the same material share one graph (texture placement saved
  by reference; 1,000 textured cubes: 5 graphs, +0.5 MB, 0.5 s).
  Provisional: MXG-Q12 (pinned graphs aren't drawn by the viewport or
  marked in the tree), MXG-Q13 (placement by reference). Not yet: one shared graph per material, graph-to-layers
  recognition, node editing (MXG-Q11). Guide: Materials > How materials
  are saved.

- **Mini Properties resize (2026-11-24, branch mini-properties-resize,
  UIC-66 amended with the maintainer's OK):** the bottom-right corner drags
  to resize it; it keeps its top-left corner while you drag and opens at
  that size until quit (not saved across launches yet). Guide: Tool
  Properties page, "Mini Properties".

- **Bug reports #6, #7, #8 (2026-11-24, branch fix-bug-reports-batch1):**
  #6 Texture Locator (Shader Tree and Environment) Position, Rotation,
  Size, Falloff and Real World Size are one row per axis (UIC-114), so Y
  and Z no longer run off the panel. #7 Mesh Stack op ids start at 1 (0
  is Source Item's "(none)"); older saves give an op 0 a fresh id and
  move its Select by Previous links with it. #8 Mini Properties grows to
  its rows (up to 480 pt) instead of staying as short as it first opened.
  #5 was already fixed on main.

- **Help mode, What's This? (2026-11-24, branch help-mode, UIC-06):** F1
  or Help ▸ What's This? (`operandi.help.whatsThis`), question-mark
  pointer; the next click opens /guide/<version>/go/<help-id>/ for the
  button, menu entry, tool's Tool Properties, panel or 3D view, and does
  nothing else; Esc or right-click leaves. Provisional: floating palettes
  in their own OS windows and the in-window menu bar (Linux, Windows)
  still act on the click.

- **LXO export label (2026-11-24, branch lxo-export-label, Jason's
  choice):** File ▸ Export's "Modo Scene (LXO)…" is now "LXO Scene…", and
  the format's name in the export file dialog is "LXO Scene". The command
  (`scene.saveAs format:lxo`) is unchanged.

- **User Guide, panels and menus (2026-11-24, branch guide-panels):**
  Items list, Tool Properties, Properties panel, Shader Tree and
  Preferences pages with shots; one page per menu written from the menu
  model by guide_menus_tests.rs (OPERANDI_BLESS_GUIDE=1 rewrites them).

- **User Guide, Topology and Curve tabs (2026-11-23, branches
  guide-model-tools-11 and -12):** retopology workflow page, Contour,
  Topology Mode/Constrain/Merge Instance; Curve tab, curve commands,
  offsets/conversions/Bezier Tangent. Hair Guide is still a not-built
  placeholder.

- **User Guide, Polygon tab (2026-11-23, branch guide-model-tools-10):**
  overview plus Twist Bevel, Extrude, Smooth Shift/Thicken, Spike,
  Polygon Reduction, more tools, triangles, Flip/Align, Merge/Remove,
  Scale Offset, Part/Material. Harness: OPERANDI_QA_FINAL_COMMAND (Apply
  after typed fields).

- **User Guide, Edge tab (2026-11-23, branch guide-model-tools-9):**
  overview plus Edge Bevel/Chamfer, Edge Extrude, Extend, Edge Relax, Add
  Loop/Point, Edge Weight, Grow/Fill Quads, Split, Edge commands. Harness:
  OPERANDI_QA_SELECT selects by index through the MCP select tool
  (mcp_answer now built with the qa feature too).

- **User Guide, Vertex tab (2026-11-23, branch guide-model-tools-8):**
  overview plus Create, Vertex Bevel, Vertex Extrude, Merge/Join/Weld,
  Tear, Vertex Slide, Place points, Make Polygon and Split, Remove and
  Collapse, Index order.

- **User Guide, Edit tab (2026-11-23, branch guide-model-tools-7):**
  overview plus Slide, Slice tools, Dicer, Center, Subdivide, Freeze,
  Booleans and drills, Retopology and Remesh, Clean-up commands.

- **Online libraries in the Preset Browser, A34 (2026-11-23, branch
  claude/project-thread-zmlbjh):** Poly Haven (HDRIs, textures, models)
  and ambientCG (HDRIs, materials), all CC0, as read-only roots after
  Basics. Off by default, one switch each in Preferences > Defaults >
  Application > Online Libraries; a library that's off is never contacted
  and isn't shown. Catalogues cached (weekly / monthly / only when asked,
  plus Refresh Catalogue), thumbnails fetched only for tiles on screen,
  files only on use at the Download Resolution (or Download At), cached
  in `<data>/Online Libraries` and working offline. Credits: "Powered by
  Poly Haven" / "Assets from ambientCG.com" over their tiles, licence and
  authors in tooltips, both in About. System `curl` with an Operandi
  User-Agent; no accounts. `operandi.online.refresh|download|clearCache`,
  `pref.value online.*`, MCP `online_library`. Guide page
  materials/online-libraries.md; GUI: PRESET_BROWSER_GUI_QA.md section
  A34. Live test (opt-in, `online_library_tests::live`) loads one of each
  kind from both sites. Provisional: model assets are glTF only; textures
  fetch JPG maps with a PNG displacement; ambientCG materials are the JPG
  sets; drops of a not-yet-downloaded asset land where the pointer was
  when the download finishes. Not yet: AMD MaterialX Library and
  Physically Based (A34 sources 3-4), tag search in the browser (the MCP
  list searches tags). Poly Haven asked to be told before launch.
- **Deform tab Sculpt button (2026-11-23, branch deform-sculpt-button,
  Jason's choice):** the button in Scale Tools now runs
  `operandi.toolbox.tab sculpt` (Brush tool set, Sculpt tab) instead of
  the not-built placeholder; same name and icon. The guide's Deform page
  says so (guide-model-tools-6).
- **User Guide, Duplicate tab (2026-11-23, branch guide-model-tools-6):**
  overview of the four versions (copies, Instance, Instancer, Transform)
  and pages for Clone, Array, Radial Array, Scatter Clone, Mirror, the path
  tools, Radial Sweep, Symmetrize, Bridge, Tack Tool and Duplicate; the
  Deform overview says Sculpt opens Brush > Sculpt (branch
  deform-sculpt-button).

- **User Guide, rest of the Deform tab (2026-11-23, branch
  guide-model-tools-5):** Twist, Bend, Vortex, Swirl, Push, Taper, Bulge,
  Flare, Radial/Linear Align and Relax Mesh, on column.opscene and
  grid.opscene; Flex, Shear and Soft Selection Move pictures now show the
  change. The Deform tab's Sculpt button is still a not-built placeholder.

- **User Guide, Deform tab Move Tools (2026-11-23, branch
  guide-model-tools-4):** Deform tab overview plus Element Move, Element
  Push, Flex, Soft Drag, Soft Move, Shear, Smooth, Jitter, Quantize and the
  Soft Selection tools, each with a harness shot (sphere-patch.opscene).

- **User Guide, Select tab (2026-11-23, branch guide-model-tools-3):**
  Selection modes, Select tab, Loops and rings, Between and shortest path,
  Select by shape, Selection sets, plus Snaps and Precision and Mesh Paint;
  harness: OPERANDI_QA_AFTER_COMMAND and Shift-clicks in OPERANDI_QA_CLICKS.

- **User Guide, rest of the Create tab (2026-11-23, branch
  guide-model-tools-2):** pages for Solid Sketch, Topology Pen, Pen, Curve,
  Bezier, B-Spline, Sketch, Patch, Cut/Copy/Paste (with Paste Tool),
  Merge/Unmerge, Quick Align, Ground Align, Ruler, Protractor and
  Dimensions; harness: OPERANDI_QA_STROKE and OPERANDI_QA_LATE_COMMAND.

- **User Guide, Create tab primitives (2026-11-23, branch user-guide):**
  tool pages for Cube, Sphere, Ellipsoid, Cone, Cylinder, Torus, Capsule,
  Text, Arc and Tube, each with a harness shot; Reference > Keyboard
  shortcuts is written from `input::ACTIONS` by guide_shortcuts_tests.rs
  (OPERANDI_BLESS_GUIDE=1 rewrites it). New QA harness settings for the
  shots: OPERANDI_QA_PRIMITIVE_HEIGHT (height drag), OPERANDI_QA_FIELDS
  (type into Tool Properties), OPERANDI_QA_CLICKS (clicks at world points).
- **Scripting guide pages (2026-11-23, branch scripting-guide):**
  User Guide section Scripting and the test bridge: overview, Python
  scripting (switch, Script Console, Run Script, mesh data, animation,
  undo, limits), Python reference (written from operandi.py, the
  registry's script-askable queries and the operandi.python.* commands by
  guide_python_tests.rs; bless with OPERANDI_BLESS_GUIDE=1), Kits and
  extensions (scripts on menus via operandi.form.add, kit items kept,
  the two Rust register hooks, what isn't built). Every ```python block
  in those pages runs in the app as a test (python feature). Fix found on
  the way: in the app a script couldn't run any command without a
  headless form (tools, select.*, operandi.form.*, command.list): the
  query check refused them first (operandi-python answer_editor).

- **Help > User Guide (2026-11-23, branch guide-help-menu):** Help >
  Documentation is now User Guide and always lit; it opens
  operandi3d.org/guide/<version>/ (test-channel builds: /guide/test/).
  Same command, `operandi.help.documentation`. `scripts/guide-build.sh`
  builds the guide and writes a go/<help-id>/ redirect per id in the page
  headers (tools/guide_redirects.py, plus go/help-ids.json); an unclaimed
  id lands on the guide's search. Not built: F1 help mode (UIC-06), so
  nothing opens go/ links yet. The guide link answers once the guide is
  published.

- **Auto Rig, milestone 1 (2026-11-23, branch
  claude/project-thread-cezhnf):** native build of Auto Character System's
  first steps (BEHAVIOR-ACS-FEATURES; based on Auto Character System by
  Lukasz Pazera, MIT, no code copied; credited in About). Auto Rig section
  of the Rig tab and `operandi.acs.*`: New
  Biped Rig (25 guides, Guide context), guide drags in the views with live
  left/right symmetry, Fit Guides to Mesh (size and place from the mesh,
  guides centred in the limbs by rays), Mirror and Reset Guides, contexts
  (Assemble/Guide/Meshes/Form/Animate; not undo steps, saved), options
  (vertebrae, neck, twist joints; rebuilds), Build Skeleton (28 joints by
  default, keeps joint ids so weights stay) and Bind Selected Meshes (the
  shared Heat bind, in Form). Every command has an MCP bridge test.
  Native v63. Provisional: ACSF-Q01..Q06 (context visibility, symmetry
  lead, shared options, in-line twist joints, fit rays). Not yet:
  controllers, IK/FK switch and match (next milestone), guide presets,
  rig clay, controller shapes. GUI: AUTO_RIG_GUI_QA.md.

- **Brand renames (2026-11-23, branch brand-renames, Jason's names):**
  what users see changes; ids, file names, saved keys and LXO keys stay.
  - Tool Pipe → Tool Chain (Workbench ▸ Add Tool Chain, docs). Command
    `operandi.workbench.addChain`; `addPipe` still works as an alias.
  - Replicator → Instancer in labels, tooltips, status text, item names
    and undo labels: Instancer Clone/Array/Radial Array/Mirror/Scatter,
    Curve/Bezier/B-Spline Instancer, Pen Instancer, Mesh Paint Instancer,
    Freeze Instancer, the Instancer properties tab and Duplicate section,
    Viewport Properties ▸ Instancers, Texture Instancer. The old phrases
    (Replica Clone, Freeze Replicator, Replicators, Mesh Paint Replica …)
    still work. Tool preset ids, icon files, `Item::replicator` and the
    vertex maps "Replica Rotation/Scale/Member" (saved names) are unchanged.
  - Retopo Edge Flow's set is RetopoEdgeFlow (was RetopoFlow); in an older
    scene the next Retopo Edge Flow moves RetopoFlow's polygons into it, in
    the same undo step.
  - Add Item lists only what Operandi makes: Backdrop Item, Camera, Mesh,
    Lights, Locators ▸ Group Locator. Show unsupported is gone (IL-Q4, a
    deliberate departure from IL-02).
- **Mesh Stack rename (2026-11-23, branch mesh-stack-rename):** Mesh Ops
  renamed Mesh Stack in code, docs and bridge names. Saved files keep the
  key `mesh_ops` (scene items and saved panel layouts), so they open in
  older and newer builds. Bridge: panel name `mesh_stack` (`mesh_ops` still
  accepted); `item_get` field `mesh_stack` and `mesh_get`
  `mesh_stack_status` replace `mesh_ops` / `mesh_ops_status`. Command and
  type ids (`meshop.*`, `*.meshop.item`) unchanged.

- **Work plane display (2026-11-23, branch workplane-display):** the work
  plane has its own Show Work Plane switch again (Work Plane Settings >
  Display and Preferences > Display), off by default and saved with the
  display preferences. Since Viewport Properties P2 (2026-11-08) the old
  Work Plane Display check did nothing at rest, because each view's Show
  Work Plane row defaulted to Active; that row now defaults to Always and
  applies only while the switch is on. Show Grid's Inactive Work Plane
  hides the floor grid only when the work plane is actually drawn.

- **Toolsets and tool tabs rework (2026-11-23, branch toolsets-rework):**
  seven toolsets (Model, UV, Brush, Setup, Animation, Workbench, Game);
  each set lists its own tabs (`ToolSet::tabs`), a one-form set draws no
  strip. Animation is one form (`toolbox.animation.json`: Transform with
  World/Local, Add Item, Tools, Commands, Snapping; no Actors, Actions,
  Action Layers, Poses, Channel/Key Sets or Retargeting yet). New forms (existing
  commands only): Setup, Constraints, Inverse Kinematics, Weighting, Game
  Vertex Map, Baking. `operandi.toolbox.tab` opens the first set with the
  tab; "Paint" reads as Brush. Set buttons never change the layout (the UV
  companion view is gone). Tabs reorder along their row without UI build
  mode; dragging one out still needs it. Transform rows lost the
  changed-from-default dot. Not built (no commands): Locator, Work Plane,
  Match, Set, World/Local, Deformers, Controls, Rename Properties, game
  navigation, smoothing groups, morph maps, mesh tangents, Game Presets.
  GUI: TOOLSETS_GUI_QA.md.

- **Statistics panel (2026-11-23, branch statistics-panel;
  development/specs/statistics-panel):** a tree-list panel (Info category,
  Layout > Windows/Palettes "Statistics") counting the active meshes:
  vertices by edge and polygon count, straight-line points, vertex maps and
  sets; edges by polygon count, borders (part, material, open, UV seam,
  flat), crease weights and sets; polygons by type, Checks (single point,
  not flat, overlapping, flat with a neighbour, sharp angle, concave), by
  vertex count, part, material, sets, smoothing groups and Drawn (GPU, the
  GL readout's triangles); items by type and group. + / − select a row
  (switching mode, Q2), one `operandi.statistics.select` undo step; row
  menu Rename, Delete, Copy/Paste, Clear, Select Only, Hide/Unhide. Counted
  on a worker thread, topology remembered per polygon list, only open
  groups counted. Not built: the Filter row and a shortcut (Jason,
  2026-11-23), the per-element Info page (Q9). Questions STAT-Q01..Q07.

- **Update check (2026-11-23, branch update-check):** Preferences >
  Defaults > Application > Updates: Update Channel (Release / Test) and
  Check for Updates on Launch (`pref.value updates.*`). The launch check
  and Help > Check for Updates (`operandi.update.check`) ask GitHub by the
  system's `curl` on a background thread and offer the newer build in an
  Update Available confirmation (OK opens its release page, Skip This
  Build). Never downloads or installs. macOS, Windows and Linux all
  follow the chosen channel (Linux test builds, 2026-11-23). The channel is a dropdown, not joined segments (UIC-113:
  segments are for dialogs and tool properties). GUI: UPDATE_CHECK_GUI_QA.md.

- **Edge Bevel with hard edges (2026-11-22, branch forum-bevel-loopslice;
  forum report):** Edge Bevel and Edge Chamfer refused every edge of a mesh
  with a vertex normal map ("cannot yet remap ... normals"), and hardening
  an edge always makes one, so they did nothing after Harden. They now
  carry the map: rebuilt polygons keep their corners' normals, new
  polygons' corners are unassigned (provisional, EBC-Q13). Bridge with its
  defaults leaves no open edges or loose points (checked over segments,
  modes and Remove). Still refused: border edges (EBC-Q14, no spec), corner
  weights/colours.

- **Loop Slice, Slice Selected follows the selection (2026-11-22, branch
  forum-bevel-loopslice; ported from claude/project-thread-44lm6w):** with
  Slice Selected each connected group of selected polygons is cut along its
  middle (LS-75): pairs and strips as before, a lone polygon across its
  first side, an L turns its corner (a triangle and a five-sided rest), a
  2x2 block gets a closed loop round its centre; pick order doesn't matter.
  Unselected polygons are never cut and take one point per cut side (LS-72,
  LS-Q15). An L with Slice Selected off is cut the same, its ends running
  on round the ring. Provisional: a branching group is left uncut (LS-Q18);
  a 2x2 block with Slice Selected off is unchanged (LS-Q17, open). Current
  stops at the last slice instead of refusing a nudge ("from 0 to 0").
  Forum report: with Slice Selected off (the default) a lone selected
  polygon now cuts round the ring through its first side (provisional,
  LS-Q19). Rings still stop at triangles, odd n-gons and (Slice N-gon off)
  n-gons, so a lone n-gon or a pair beside one can still give no cut.

- **Colour numbers in display space (2026-11-21, branch
  colour-swatch-raw):** Display Color Values in Display Space is on by
  default, so colour numbers show and take display-encoded (sRGB) values,
  with 2 decimals; stored values stay linear, so renders and files are
  unchanged. Swatches draw the stored colour through the display
  transform, the same whatever the number display. Units preference files
  from before turn the option on once (file version 2); a later choice to
  turn it off sticks. See CN-Q03/CN-Q05.

- **Fields and views take turns (2026-11-21, branch
  claude/project-thread-uome72; Model the Mug tutorial):** shared, for
  every tool. A Tool Properties field holding the keyboard with nothing
  typed (after Tab, or a click) lets go when the pointer moves over a 3D
  view, so A and the other view keys act on the view (UIC-70); before, A
  went into the field and its bad text then refused every viewport press.
  Typed text keeps the keyboard until Enter/Tab (UIC-51) or a press in the
  view confirms it; that press then goes on to the view. Frame All and
  Frame Selection read what the views show (`Editor::shown_display`), so a
  Cone preview frames too. With a creation tool open, Frame All frames
  the shape being made (the geometry it adds), or a 1 m box at the tool's
  centre before anything is drawn, not the whole scene. Sphere and
  Cylinder move arrows answer along their whole shaft, as the Cube's do
  (they took only the tip, so a press on the arrow dragged a side).
  Provisional: the hover hand-off and the 1 m empty frame have no spec
  line. [GUI checklist](FIELD_VIEW_HANDOFF_GUI_QA.md)

- **Camera view navigation speed (2026-11-21, branch
  claude/project-thread-w0doma):** a wheel, trackpad or 3D mouse move in
  Camera or Light view moves the camera or light once the view has held
  still for 0.3 s (provisional), as one undo step; a drag still writes on
  release. Undo, Redo and leaving the view write a waiting move first.
  Moving an item with no geometry under it (a camera, a light) no longer
  rebuilds the picking surface. Camera-view wheel zoom on a 1M-polygon mesh:
  345 to 8 ms a frame. Bench: `performance_tests::camera_view_navigation`.

- **Closing the Node View (2026-11-21, branch node-view-close):** a ×
  and right-click Close Panel on every docked registry panel's header;
  Window > Node View / Cmd+Shift+N toggle it (ticked while open; when
  maximised the first press restores the layout); Esc over the maximised
  Node View restores the layout; closing its floating window closes it.
  What it shows is kept, so reopening shows the same material.
  [GUI checklist](NODE_VIEW_PANEL_GUI_QA.md)

- **Node View panel (2026-11-21, branch node-view-dock):** the node view
  is a registry panel ("Node View", Window > Panels) instead of a floating
  window: Show as Nodes, Show Graph, a driven input's marker and
  `operandi.panel.show kind:{Node View}` dock it under the views (40 % of
  their height) the first time, then bring it forward. It docks, tabs and
  floats like any panel (UIC-71..73, UIC-157) and is saved with the
  layout. It follows the Shader Tree's selection (a layer shows its
  material); provisional: a selection that isn't a material keeps the last
  one shown. The old window's place file (node-graph-window.json) is no
  longer read. [GUI checklist](NODE_VIEW_PANEL_GUI_QA.md)

- **Instance duplicates and Symmetrize (2026-11-20, branch
  dup-instance-symmetrize; BEHAVIOR-DUPLICATE-PANEL DX-30..45):** Instance
  Clone, Array, Radial Array, Mirror, Scatter and Curve / Bezier / B-Spline
  / Pen Instance make Mesh Instances of the selected items (any selection
  mode); the Item section (Instance, Parent Off/Hierarchy/Root/Primary,
  Hierarchy) is shared with the plain tools in Item mode. Instance Mirror
  uses a negative scale. Item ▸ Instance (Shift+D) works (DX-38). New
  Symmetrize tool on the Duplicate tab: cuts polygons crossing the plane,
  keeps one side, mirrors it, welds the seam; Axis, Direction, Merge,
  Offset, Topology, live; it ignores a component selection (DXQ-3, the
  maintainer's OK). Not built: Bounding Box and the instance wire tint.
  [GUI checklist](DUPLICATE_INSTANCE_SYMMETRIZE_GUI_QA.md)

- **Dock guide and UI build mode (2026-11-20, branch claude/project-thread-1ngxi1):**
  the orange drop strips are replaced by a five-way cross over the panel
  under the pointer (centre = tab) and four arrows at the work area's
  outer edges, with a translucent landing preview; tab rows still take a
  between-tabs drop; a drop anywhere else floats the panel (UIC-72
  redesign, Jason's OK via the coordinator). Divider strips are gone.
  Panels, tabs and the mode bar move only in Layout > UI Build Mode
  (`operandi.layout.buildMode`, Ctrl+Shift+L; Jason with Sylvain): there a
  press anywhere on an accent-outlined panel moves it, its controls and the
  3D view don't respond, tabs still click and drag, and a bottom bar offers
  Done. Lock Thumb Dragging left the Layout menu (the command stays for
  scripts, with no effect); double-click float is gone (Float is in the
  panel menu). The mode bar (top or bottom only) shows the guide's
  top/bottom arrows and a landing strip. Provisional: outer-edge docks
  still split the window in half. [GUI checklist](DOCK_GUIDE_GUI_QA.md)

- **Scene item budget (2026-11-20, branch scene-budget):** a scene holds
  up to 1,000,000 items (`MAX_SCENE_ITEMS`; was 10,000, or 4,096 in some
  tools), loading included; LXO read/write limits follow it. Free names
  ("Mesh (n)") take one pass over the list, item lookups no longer refill
  per added item, and undo steps are shared (`Arc`) instead of copied with
  the editor on every command. Undo keeps at most 4 GiB of item copies
  (`UNDO_MEMORY_BUDGET`, provisional): about 12 steps at 100,000 items.
  USD import that fills the scene brings in what fits and reports the rest
  as left out. ALab (16,960 items, 13.9M polygons) round-trips: import
  10.5 s, export 9.1 s, re-import 9.6 s, about 11 GB peak. Known limits:
  every command costs about 1 s at 100,000 items (each one makes several
  whole-scene copies and checks), and a frame about 200 ms (Item List rows
  and viewport passes over every item).

- **Handle tooltips (2026-11-19, branch mn09-handle-tooltips, MN-09):**
  every handle kind has a hover tooltip from one table (`handle_tips.rs`):
  the other live layouts (Bevel-type pairs, Edge Extend, Twist Bevel's
  spine...), falloff handles, Tweak's element, an armed Click to Start
  tool, camera and light handles. None show during a drag. A test lists
  every kind and fails if one has no text. Provisional: the wording is
  Operandi's own. [GUI checklist](MN09_HANDLE_TOOLTIPS_GUI_QA.md)

- **Selection parity fixes (2026-11-19, branch pf-selection, PF-01..PF-07):**
  polygon Grow/Shrink use every polygon sharing a point; Shrink no longer
  counts the open border as unselected (points, polygons); an edge Loop
  from a border edge turns the border's corners and runs all round; Edge
  mode Boundary with nothing selected takes every open-border edge; Close
  Loop with polygons along a closed loop completes the ring; Next/Previous
  Loop work on point loops. PF-07: Close Loop on an open edge run already
  answers at once in core and through the bridge's dispatch (measured
  < 1 ms); the minute-long stall seen is the background-window slowdown
  in the bridge request (pf-bridge). Edge-mode Shrink keeps its border rule.


- **Drag and drop gaps (2026-11-19, branch mn07-drag-drop, MN-07):** a
  picture (HDR/EXR too) dropped on a mesh becomes an Image Map layer at
  the top of the material group masking the polygon's tag
  (`ShaderTreeEdit::AddImageForTag`, one step); several dropped files are
  all handled at the same point (scene files queue and open one by one);
  a selected item dragged from a 3D view onto a Groups row joins that
  group. Provisional: a tag with no material group gets the layer at the
  tree's top level; with no Drop Action set, dragging from a selected item
  is now an item drag rather than a lasso. [GUI checklist](MN07_DRAG_DROP_GUI_QA.md)

- **Bridge request batch 2 (2026-11-20, branch pf-bridge-2):** every
  tool_run resets the tool first and turns Click to Start off for the call
  (PF-12 in a fresh app and after other runs). Chamfer Edit through the
  bridge runs every recorded case: the start-up probe takes a several-
  segment chamfer; Width mode, Absolute Size (0 closes the chamfer),
  Swap Side and the Reverse switches (no effect without profiles) are
  built and take `mode`, `absSize`, `swapSide`, `reverseX`, `reverseY`;
  Q-CE-03 (answered): a selection with no unselected polygon beside it on
  both sides is a run whose open ends Offset pushes out and Segments slices
  straight across (Depth unused). Q-CE-05: Operandi's behaviour kept.

- **Bridge request, first batch (2026-11-19, branch pf-bridge):** headless
  `command_run` entries vert.set (axis, value, workPlane, worldSpace),
  poly.unify (force), poly.haul (offset) and chamfer.edit (offset,
  segments, sourceSegs, depth as a factor); `tool_run` gains poly.inset
  (inset) and xfrm.quantize (step [x, y, z]). command_help describes them.
  Not done yet: the rest of the request (chamfer.edit's Mode/Absolute/
  Reverse/Swap, Poly Haul modes, vert.merge range, Pen JSON options,
  Scale Offset amount, Detriangulate options, the selection commands, the
  other tool_run tools and options, and full speed with the window in the
  background).


- **Chamfer Edit cases (2026-11-20, branch pf-chamfer-edit):** Source
  Segments above 1 merges the selected strips into one before rebuilding
  (CE12, CE14), a mismatch is refused (CE13, Q-CE-02), and the selection
  stays when no polygons were added or removed (CE02, CE10; rule 11).
  CE04-CE06 keep the decided arc (Q-CE-01, within 0.01 of the reference).
  Still open: CE12's z drift (Q-CE-05), CE16/CE17 (Q-CE-03); Width mode,
  Absolute Size and Swap Side (CE03, CE07-CE09, CE11) aren't built.

- **Curve Extrude section frames (2026-11-20, branch cx-frame, CX-Q8):**
  sections sit on the PF-09 curve at equal parameter steps; each is the
  previous one turned by the smallest rotation between their tangents,
  carried from the source polygon as it is, so the end cap faces the
  source normal turned by the accumulated tangent turn (path (0,0,0),
  (0,1,0), (1,2,0), 4 steps: (0.9502, 0.3115, 0)). The old CX-13 rule
  (source turned to face along the path first) is gone. Includes
  pf-mesh-tools (merged) for the PF-09 curve.

- **Mesh tool parity fixes (2026-11-19, branch pf-mesh-tools):** PF-10
  Edge Split all round a closed loop leaves nothing selected; PF-12 Edge
  Extend at its Reset values still adds the zero-width row and selects
  its new edge (also through the bridge's tool_run). PF-08 v2: Flatten
  Ngons moves each 5+ point polygon's points along its Newell normal onto
  the plane through its bounding-box centre, nothing selected after
  (`flatten_normal` holds the normal rule). PF-09: smooth Curves use
  chord-weighted Hermite tangents with mirrored open ends; USD exports
  them as cubic Beziers so they round-trip.

- **Chamfer Edit cases (2026-11-20, branch pf-chamfer-edit):** Source
  Segments above 1 merges the selected strips into one before rebuilding
  (CE12, CE14), a mismatch is refused (CE13, Q-CE-02), and the selection
  stays when no polygons were added or removed (CE02, CE10; rule 11).
  CE04-CE06 keep the decided arc (Q-CE-01, within 0.01 of the reference).
  Still open: CE12's z drift (Q-CE-05), CE16/CE17 (Q-CE-03); Width mode,
  Absolute Size and Swap Side (CE03, CE07-CE09, CE11) aren't built.


- **Render outputs group D (2026-11-20, branch render-outputs-d):**
  Illumination (Unshadowed) (lamps' direct light, shadows off, from a
  second lamps-only, zero-bounce Cycles render shown as "Unshadowed pass"),
  Diffuse Shading (Unshadowed) (that × Diffuse Color), Light Path
  Expressions (parser, mapping onto Cycles passes and light groups,
  presets, status line; unsupported ones render black with a note) with
  Render > Add Per-Light Render Outputs, and Shaded AA Samples (sample
  count heat map, raw share as .Y in OpenEXR Multilayer). Cycles ABI 28
  (rerun scripts/build-cycles.sh), native file version 55 only when used.
  Provisional: RO-Q18..RO-Q31. [GUI checklist](RENDER_OUTPUTS_D_GUI_QA.md)

- **Cycles built-in render outputs (2026-11-20, branch cycles-render-outputs):**
  Render Output types Illumination (Direct / Indirect / Total), UV
  Coordinates, Segment ID and Cryptomatte (Object / Material / Asset,
  2–16 levels) from Cycles' own passes, AOVs and cryptomatte, with Cycles'
  manifests; a per-output Colorspace (Default / Linear / sRGB) for saving
  and the render window; Save Image > OpenEXR Multilayer (every output,
  Cryptomatte layers and metadata, full float). Cycles ABI 27 (rerun
  scripts/build-cycles.sh), native file version 53 only when used.
  Provisional: RO-Q10..Q17. [GUI checklist](CYCLES_OUTPUTS_GUI_QA.md)

- **Handle style in the View menu (2026-11-19, branch mn01-handle-style-menu, MN-01):**
  View > Tool Handles > Draw Style: Invisible, Basic, Advanced, Default,
  the current one ticked (Default puts back Basic and is never ticked).
  Each runs `tool.handleStyle` and saves the preference; Invisible hides
  the active tool's handles and hauling still works.
  [GUI checklist](MN01_HANDLE_STYLE_MENU_GUI_QA.md)

- **Key toggles (2026-11-19, branch mn02-key-toggles, MN-02):** hold a
  tool's key and drag in a 3D view to use it; releasing the key drops it
  (after the drag ends). A tap keeps today's behaviour. Holding another
  tool's key swaps to it for the hold and returns to the first tool
  (`key_toggles::RETURN_TO_PREVIOUS_TOOL`). A key whose tool was already on
  starts no hold. Preferences > Input > Keys > Key Toggles (default on).
  [GUI checklist](MN02_KEY_TOGGLES_GUI_QA.md)

- **Mouse rule gaps (2026-11-19, branch mn03-mouse-rule-gaps, MN-03):**
  middle-drag with a drawn Cube/Sphere/Cylinder copies it at the pointer
  (same settings) and the drag adjusts the copy; a transform tool's
  right-drag sets up a Linear/Radial/Cylinder/Screen falloff when its
  hauls row leaves the right button free; under the Screen falloff Bevel
  and Twist Bevel follow the transform rule (plain = fresh, Shift =
  continue); Airbrush strokes add, lower (Shift) or erase (Ctrl/Cmd).
  Provisional: MN-03.3 covers the tools that can start a fresh edit and
  take falloffs (Bevel, Twist Bevel); the copy keeps the first one's
  height above the work plane. [GUI checklist](MN03_MOUSE_RULES_GUI_QA.md)

- **Reset Tool Attributes everywhere (2026-11-19, branch mn05-reset-tool, MN-05):**
  every tool resets through one command (`tool.reset`, Edit > Reset Tool
  Attributes, Ctrl+D / Cmd+D). Tools with their own reset keep it; the rest
  restart on a throwaway copy with remembered values forgotten and take its
  starting values. One undo step "Reset <tool>"; the tool stays active.
  Provisional: no per-tool saved defaults exist yet, so the target is
  always factory; only tools that had one show a Reset button in Tool
  Properties (the menu and key work for all). Pen and Solid Sketch have no
  values and say so. [GUI checklist](MN05_RESET_TOOL_GUI_QA.md)

- **Number field gaps (2026-11-19, branch mn06-number-fields, MN-06):**
  mini-slider detents (UIC-54) on Tool Properties, form and Item Transform
  arrows; click-to-step on every field (ten drag steps, MN-06.4); Item
  Transform's Position, Rotation and Scale groups have the UIC-56 link
  button and UIC-57 Enter modifiers (Cmd for Ctrl on a Mac). Provisional:
  distance light stops are 2..9 × each power of ten; stops closer than 12
  points apart at the drag's speed are skipped; the link button is a text
  glyph until generator icons are approved; link modes are per session.
  Not done: link buttons on Tool Properties X/Y/Z rows, and detents on the
  plain `numeric_tab::DragValue` fields (dialogs).
  [GUI checklist](MN06_NUMBER_FIELDS_GUI_QA.md)

- **Link buttons, follow-up (2026-11-20, branch mn06b-link-buttons,
  MN-06b):** one link button per X/Y/Z group, on the first row, as tall as
  the mini-slider, showing a rendered chain icon per state (icon studio:
  `link-independent`, `link-copy`, `link-proportional`, `link-relative`;
  awaiting Jason's approval). Every Tool Properties group built with
  `tool_form::vector` has it (Move/Rotate/Scale/Transform, primitive
  Position/Size/Segments, deform centres, duplicate offsets, falloff
  shapes) with the UIC-57 Enter keys; whole-number groups round. Tooltip
  says "Link: off / same value / scale together / add together", what a
  click picks and the Enter keys. Not done: Tool Properties scrub drags
  don't spread (Item Transform's do); work plane, Action Center and
  Backdrop have no X/Y/Z fields; environment and texture locators use
  inline drag values. [GUI checklist](MN06B_LINK_BUTTONS_GUI_QA.md)

- **Maximise any panel (2026-11-19, branch mn08-maximise-any-panel, MN-08):**
  checked first: Numpad 0 / Alt+0 only acted on 3D views. Now, with the
  pointer over a docked panel, the panel fills its group (the side column,
  or in a docked layout the split holding it or its tab group); the key
  again restores. Text fields keep the key. Provisional: per session, not
  saved with the layout. [GUI checklist](MN08_MAXIMISE_PANELS_GUI_QA.md)

- **Material-value render outputs (2026-11-19, branch material-value-outputs):**
  13 new Render Output effects under Add Layer > Special > Render Output >
  Material Values: Diffuse Amount / Coefficient / Energy Conservation /
  Roughness, Specular and Reflection Coefficient, Roughness, Subsurface
  Amount / Color, Transparent Amount / Color, dPdu, dPdv. Each writes a
  Cycles colour AOV `operandi_value_<name>` from the folded material
  (textures count). Cycles ABI 26 (rerun scripts/build-cycles.sh), native
  file version 52 only when one is used. Provisional: MV-Q01..Q08.
  [GUI checklist](MATERIAL_VALUE_OUTPUTS_GUI_QA.md)

- **Shared pivot service (2026-11-19, branch shared-pivot-service):** one
  service (`pivot_service.rs`) gives every tool its pivots under the
  Action Center × Symmetry rulebook (ACS-01..13, 20, 21): the + half's
  centre, axes from the whole selection, the primary side from the
  centre's side, the other side mirrored, seam points back on the plane,
  a click or typed centre picks the side. Move/Rotate/Scale/Transform,
  the polygon stage handles, Smooth Shift/Thicken (scale about the Action
  Center), Edge Extend, the slide and deform tools all ask it; the
  first-pick and view-side rules are gone. Parent, Pivot and Pivot Center
  Parent Axis now work. Matrix: 287 of 311 cases pass, 24 known gaps
  (ACS-Q3 axis frames, ACS-Q4 moved item). Questions ACS-Q3..Q8,
  ACS-B1..B7. [GUI checklist](SHARED_PIVOT_GUI_QA.md)

- **USD export gaps (2026-11-19, branch usd-export-gaps):** USD export now
  writes weight, typed and colour maps (displayColor for the first), morph
  maps as blend shapes, parts and point/polygon/edge selection sets, and
  Mesh Instances as native USD instances; the import reads each back
  (round-trip tests in operandi-usd `export_gaps_tests.rs`). Whatever USD
  still gets nothing of is a line in the export report. Mesh Stack results
  were already exported (every entry point passes the evaluated scene;
  now tested). Bridge ABI 6: rerun scripts/build-usd.sh. Provisional:
  USD-IQ22.

- **Layout cameras (branch codex/view-camera-persist).** Each 3D view's
  navigation camera and each UV view's pan and zoom survive restart
  (LAY-40, UIC-195). Older slots open at defaults.
  [GUI checklist](LAYOUTS_STAGE1_GUI_QA.md)

- **Panel hover grips (2026-11-19, branch codex/hover-grips).** Corner
  overlays, Cmd/Ctrl+Shift+middle-drag, and header menus run the existing
  docking operations. 3D, UV and empty views keep an always-visible corner
  dot for their frame menu (UIC-71/72/193). [GUI checklist](LAYOUTS_STAGE2_GUI_QA.md)

- **Side-panel docking J2 (branch codex/dock-j2).** The existing grips
  move the toolbox, Tool Properties and both tab groups in the dock tree.
  Divider sizes and tab state persist (UIC-191, UIC-199). Targeting one
  split view follows the provisional D-02 reading.
  [GUI checklist](LAYOUTS_STAGE2_GUI_QA.md)

- **Polygon handles under Symmetry (2026-11-18, branch codex/symmetry-handle):**
  with Action Center None, Bevel, Extrude, Smooth Shift and Thicken place
  their handles on the picked side and continue editing both sides.

- **3D painting stage 7: sculpt extras (2026-11-18, branch painting-stage7
  on painting-icons, PNT-151, 241..247, 260..264; painting builds only):**
  painting tips, nozzle and inks on sculpt brushes; the sculpt mask (five
  mask brushes, red display, controls, one undo step each, never saved);
  seven starter brushes; Geometry to Brush; Constrain to Background on
  sculpt. Not built: vector-displacement tips, stamp-once and nozzle
  rotation on sculpt, Copy to Morph. Provisional: PNT-Q63..Q73. GUI check:
  docs/PAINTING_STAGE7_GUI_QA.md.
- **Preview grain fix (2026-11-18, branch fix-preview-grain):** the
  Preview's kept Cycles session never denoised: its 1-sample coarse pass
  ran without denoising, and a Cycles session that first renders that way
  hands back raw pictures afterwards. The coarse pass now denoises like
  the full one, and the kept session starts afresh when Denoise changes.
  Chess scene through Camera_002 (640 × 400, 256 samples): RMS grain
  0.0261 → 0.0027, firefly share 0.025 % → 0. Stop rule (sample target,
  Convergence Target, Max Render Time) and indirect clamp (10) unchanged;
  a paused Preview still shows the raw picture.

- **Render camera picks (2026-11-18, branch render-camera-picks):** Item
  List right-click on a camera has Set as Render Camera (greyed on the
  render camera; CL-21 / CL-G12). USD import makes the camera named by
  the stage's RenderSettings the render camera (USD-IQ21, provisional);
  Blender's chess export names none, so it still renders through its
  first camera.

- **Behaviour-check fixes d (2026-11-18, branch check-fixes-1118d, on
  1118c):** check cases checks-ac-meshops-2026-11-18 admitted and replayed
  against their inputs (AC-rotate-Selection, AC-scale-Local,
  AC-rotate-Local already matched); Merge Meshes' Use World Transforms no
  longer changes placement (M03 = M01, M04 = M02; MOQ-30). Fill Quads cuts
  8-edge holes three edges at a time from the selection's start (3×1 as
  measured; UVN-O-fill-quads-cont matches, -seam's new polygons match;
  FQ-Q01). Edge Extrude puts both walls of a seam edge on the
  higher-numbered island (UVN-O-edge-extrude-seam matches). All
  provisional. Held: the seam case's change to an untouched Fill Quads
  neighbour; Pivot cases (not built).

- **Behaviour-check fixes c (2026-11-18, branch check-fixes-1118c):**
  Action Center Selection and Selection Center Auto Axis centre on the
  selection's item-space bounding box (AC-Q20); Local turns and scales each
  piece about its own box with its own polygon axes, mirrored under
  Symmetry (AC-Q21); the Merge Meshes op has Use World Transforms
  (`useWorldXfrm`, MOQ-30); Grow Quads recovers continuous UVs across a
  constant-shift seam on corner-UV meshes; Edge Extrude closes an interior
  end with one triangle per other polygon (15 polygons, as the
  reference). All provisional. Held: MC31 (Q-MC-01 decided), Edge
  Chamfer's extra zero-area triangles (ORQ-01), Fill Quads' shape (FQ-Q01,
  one case). MC15 didn't reproduce; it has a bridge regression test.

- **UV tools finish, batches 1-4 (2026-11-18, branch
  claude/project-thread-ll5iqr; specs uv-editing-2026-11-16 and
  uv-2026-11-16):** pipe leftovers: Jitter, Quantize, Radial and Linear
  Align run on UVs (Range Z / Step Z greyed, UVT-135), Soft Drag in a UV
  view (its circle stamps the UV proxy, `uv_pipe::UV_STAMP_LAYER`),
  Element Move's Range shows no "m" there (UVT-Q17), a click puts U/V
  handles on a UV element (UVT-Q13), Slide's UV Map field (UVT-Q16).
  Sub-D UV drags no longer re-evaluate the surface (60 ms to 9.5 ms a
  frame on the 16k grid, `uv_tool_frames_subdivided`). New live tools:
  Project (Planar, Cylindrical, Spherical, Barycentric, Cubic; Automatic
  or Manual frame; `uv_project.rs`), Project from View (a click in a 3D
  view), UV Unwrap as a tool, UV Relax (Spring, Conformal, Angle Based,
  Adaptive; Smooth, Locked, Maintain, Straighten borders; falloff),
  Unwrap & Relax (`uv_relax.rs`). Stage 3 buttons (`uv_ops.rs`, one
  `Command::UvOp`): Quick Align, Island Align, Orient Pieces, To Origin,
  Texel Density, Real World Size, Split, Sew, Move and Sew, Select
  Border / Error UVs / Unmapped, Copy / Cut / Paste, Delete, Set Value,
  Rotate Polygons, Merge UV Maps, with option dialogs (`uv_ops_ui.rs`,
  `operandi.uv.dialog`). Provisional: UVT-P01..P04, UVR-P01..P05,
  UVO-P01..P06, UVS-P01/P02, UVY-P01/P02, UVV-P01/P02, UVP-P01..P03,
  UVA-P01..P04, UVE5-P01, UVE-P01..P03, UVB-P01..P03, UVBR-P01, UVR-P06. The UV tools use the UV map glyph until their icons are
  approved. Batch 5: seam maps (`uv_seams.rs`, stored as prefixed edge
  sets) with the Cut Map buttons, Seam Marker and Unwrap cutting at a seam
  map; UV Symmetry layer, Center and Symmetrize (`uv_symmetry.rs`); UV view
  Flipped tint, Coverage, tile numbers, UDIM tile select and arrows; Pack
  Nearest, Polygon Tag, By Item, Background Constraint and Pack to New
  Map. Batch 6: Atlas and Atlas 2 projections (`uv_charts.rs`), Peeler
  (`uv_peel.rs`, live tool), Texture > Export UVs to SVG / EPS, Convert UVs
  to Mesh, Transfer UVs (`uv_export.rs`), Box Transform (`uv_box.rs`, UV
  view box handles), the Split and Sew brush, Relax Interactive pins. The
  26 UV icons (batch 22) are on the tools and buttons. Tests: core
  `uv_atlas_peel_export.rs`, `uv_seams_symmetry.rs`, `uv_project_relax.rs`, `uv_ops.rs`,
  `uv_transform_pipe.rs`; app `uv_editing_tests.rs`. GUI check:
  docs/UV_FINISH_GUI_QA.md.
- **Test bridge requests (2026-11-18, branch bridge-requests, spec
  BRIDGE-REQUESTS BRQ-xx; bridge only, no tool or UI change):**
  `command_run` takes poly.subdivide (type, fractal, maxangle, linear,
  bound, noEdgeWeight, iterations), poly.unsubdiv and vert.center, options
  not given at their Reset values (BRQ-10). `tool_run` takes Curve Slice,
  Primitive Slice and poly.julienne (Dice) with `params.slice`, and
  Linear, Radial, Cylinder, Noise and Curvature falloffs (BRQ-20). Not
  reachable: Extrude refuses falloffs (a tool gap, not a bridge one); Add
  Loop and Spikey aren't `tool_run` tools; view-drawn falloffs (Screen,
  Lasso, Airbrush, Path, Image) need the view. Batches 3-5: Backdrops
  in `scene_info` (every item now has `kind`) and `item_get`, with a new
  `backdrop_set` (BRQ-30); `mesh_get subdivided` (BRQ-40) and `uvs`
  (BRQ-70); `tool_run` `slide` options for edge.slide/vert.slide (BRQ-50;
  Linear mode, Reverse and Duplicate aren't built and are refused);
  `mesh.cleanup` headless with its twelve switches (BRQ-60); `tool_run
  edge.bridge` with segments, mode, twist, tension, remove, flip, reverse
  (BRQ-80; the rest of Bridge's panel isn't built and is refused).
  Batch 6: `mesh_get evaluated` (the Mesh Stack result, with polygon types
  and op statuses), `item_get` lists `mesh_stack`, new `meshop_set` for
  op values and item inputs, `command_help meshop.create` lists every op
  type (BRQ-91..94; no Freeze or Set Polygon Type op exists yet).
  `tool_run action_center` per call (BRQ-100; Parent and Pivot aren't
  built). MCP `modified` compares scene contents, so a full undo is clean.
  `command_run uv.relax` (mode, iter, seal, boundary, area, uniform,
  noEdge, name; Reset values, iter 0 changes nothing) and headless
  `hide.sel` / `hide.unsel` / `hide.invert` / `unhide` (BRQ-110).
  The bridge's `scene_load` (sidecar and app) opens meshes with 1/2-point,
  repeated-point or zero-area polygons as given (open lines and
  degenerate marks; File > Open still refuses them), and Mesh Cleanup
  now carries degenerate marks through the polygons it removes.
  Tests:
  `operandi-mcp/tests/bridge_requests.rs`.
- **Behaviour-check fixes (2026-11-18, branch check-fixes-1118):** the MCP
  selection echo lists every mode's own list; Select Between's rectangle
  and Select Symmetrical keep the reference's order; script picks don't
  mirror (SYM-SEL-07); Make Polygon from one point makes a one-point
  polygon; Merge of a ring makes one keyhole polygon; Bevel builds an Inset
  past the centre (reverses M17-Q04). oracle_replay grades all of them
  exact. Provisional: SYM-Q-ORDER, CMD-Q-HOLE, MP06 storage.
- **3D painting stage 6: layout, set, menus, preferences, UV painting,
  polish (2026-11-18, branch painting-stage6 on painting-stage5,
  PNT-210..239; painting builds only):** Paint layout (switcher 90) with
  the Sculpt/Paint toolbox set and a brush preset strip, Utilities tab,
  Texture > Image Processing, Images & Painting / Default Tools
  preferences, painting in the UV view (image space, Fill and gradients
  too), Images list resolution / format / unsaved, Image Viewer with PSD
  layer picker; flyouts, Alt+click Radial, Ctrl+Shift+click FG pick,
  lasso handles and fixed to the model, viewport shows a picked PSD layer.
  Not built: changed-tile uploads, starter brushes, lasso on the falloff
  stack (PNT-Q57, Q58, Q61). Provisional: PNT-Q50..Q61. GUI check:
  docs/PAINTING_STAGE6_GUI_QA.md.
- **3D painting stage 5: inks and Curve Constraint (2026-11-18, branch
  painting-stage5 on painting-stage4, PNT-170..200; painting builds
  only):** Image Ink (widget with move / size / turn handles, E/R/D/F,
  Flip, Stamp, Repeat, Soft Border, Pixel Blending, Fit View, Auto Scale,
  Use Color as Mask, UV Mask), Parametric Ink (Slope, Altitude, Incidence
  Angle, Pressure; Use as Mask, shape, In/Out), Random Ink, Projection Ink
  (camera background, Show Image, Stamp, Exposure), stacking in order;
  Curve Constraint Background 2D / 3D. Not built: sculpt inks (greyed,
  PNT-Q49), Displacement / Tangent Pressure (greyed). Provisional:
  PNT-Q40..Q49. GUI check: docs/PAINTING_STAGE5_GUI_QA.md.
- **3D painting stage 4: tips, nozzle, brush presets (2026-11-18, branch
  painting-stage4 on painting-stage3, PNT-130..150; painting builds
  only):** Procedural, Sphere, Texture (Normal / Roller / Stamp), Stamp and
  Text tips; the Tablet Nozzle (pressure curve, strength and size by
  pressure or speed, Align, jitter); Save / apply brush presets (.opbrush).
  Not built: sculpting with the new tips (PNT-Q38). Provisional: PNT-Q28..Q39.
  GUI check: docs/PAINTING_STAGE4_GUI_QA.md.
- **3D painting stage 3: more paint tools (2026-11-18, branch
  painting-stage3 on painting-stage2, PNT-90..120; painting builds only):**
  Smudge, Blur, Sharp(en), Clone (Ctrl+click source, kept offset,
  crosshair), Line / Circle / Rectangle, Fill (whole image within
  tolerance), Linear and Radial Gradient (UV space), and the Lasso falloff
  (Lasso / Rectangle / Circle / Ellipse, Soft Border, Mix Mode). Each is one
  undo step. Provisional: see the stage 3 section of
  SPECIFICATION_QUESTIONS.md. GUI check: docs/PAINTING_STAGE3_GUI_QA.md.
- **3D painting stage 2: brush engine (2026-11-18, branch painting-stage2
  on painting-stage1, PNT-40..89; painting builds only except the shared
  engine):** sculpt's tips and dab spacing moved to `brush_engine.rs`
  (sculpt results unchanged, held by tests/sculpt_brush_engine.rs);
  projected texel painting (`paint_stroke.rs`: seams, back faces, hidden
  surface); Airbrush, Paintbrush, Eraser; Smooth (with Custom profile) and
  Hard tips; 14 blend modes, Opacity ceiling, Density, Attenuation Steps,
  falloff, symmetry; live in the views, one undo step per stroke; Paint
  toolbox tab and Texture > Paint Tools; status line. Provisional:
  PNT-Q06, Q10..Q24. GUI check: docs/PAINTING_STAGE2_GUI_QA.md.
- **3D painting stage 1 (2026-11-17, branch painting-stage1, PNT-01..39,
  PNT-300..309; behind the `painting` feature, off in release and tester
  builds):** new `operandi-paint` crate (images at 8/16-bit/float, Invert,
  Clear, Set Resolution, Flatten's compositor, PNG/EXR/TGA/TIFF/JPEG, PSD/PSB
  on vendored craft-psd); core `painting` feature (pixels in the scene,
  Arc-shared, never in the scene file; images aren't undone); app: New
  Image panel, Add Color/Mask/Displacement Texture, paint target with the
  paintbrush, refusals, Images window, Expand UV Seams, Flatten Layers,
  Save/Save As/Save All, scene save also saves images, quit and auto-save,
  outside-change reload, PSD Layer picker and Paint into which layer?.
  Provisional: PNT-Q01..Q12. Not built: brushes (stage 2), the Paint
  toolbox tab, the Images & Painting preferences page, the Image Viewer's
  layer picker, per-layer PSD pictures in the viewport. GUI check:
  docs/PAINTING_STAGE1_GUI_QA.md.
- **Add browser (2026-11-17, branch claude/project-thread-fcjo2t, spec
  add-browser-2026-11-17 AB-xx):** one pop-over widget (`add_browser.rs`)
  for Add Operator, Add Selection and Add Item: 450x600, pin, Back /
  Forward / Up, Search All / Search View with recent searches, Directories
  tree at 36 % with a draggable divider, Name list grouped by folder path
  and sorted, size slider, four view modes (List default), double-click or
  Enter adds, closes on click outside unless pinned. Ops are filed in the
  AB §5 Mesh Operations tree. Provisional: MOQ-29.
- **Mesh Stack suite batch 4: tools as ops, 7 groups (2026-11-17, branch
  claude/project-thread-fcjo2t, TYPE-IDS.md types/channels/defaults):**
  Polygon (Extrude, Merge, Flip, Symmetrize, Detriangulate, UnSubdivide,
  Unify, Material Tag, Spike, Delete), Edge (Bevel, Chamfer, Split,
  Extrude, Relax, Subdivide), Vertex (Bevel, Extrude, Create, Merge,
  Split, Set Position), Duplicate (Radial Array, Clone, saved as Clone
  Effectors), Edit (Solid Drill, Axis Drill, Slice, Axis Slice, Polygon
  Reduce, Mesh Cleanup), Deform (Jitter, Push, Smooth), UV (Unwrap, Pack
  UVs, Transform UV Map). Each runs its direct tool's kernel
  (mesh_stack_tools.rs, drills in mesh_stack_items.rs). Reduce uses our own
  quadric reducer (SolidCraft's is the same method, triangles only, so
  nothing taken); no new Blender code (UV Unwrap already runs our Blender
  port). Provisional/open: MOQ-25..28. GUI check:
  docs/MESH_STACK_SUITE_GUI_QA.md (batch 4).
- **Mesh Stack suite batch 3: selection ops (2026-11-17, branch
  claude/project-thread-fcjo2t):** an op's Selection input is its Select by
  Index plus a stack of selection ops (Index, Range, Pattern, Previous
  Operation, Selection Set) with Enable, Invert and Blend Mode (MO-50..53),
  saved as native v50. Select by Previous Operation reads short group ids
  (PG-xx): primitives all/caps/capA/capB/sides, Bevel/Smooth Shift
  front/sides, Array/Mirror clone and clone1.., Boolean geometry/driver.
  Groups ride as hidden polygon sets kernels never see and never reach the
  result. Scripts: `selop.create`, `selop.edit`. Provisional: MOQ-21..24.
- **Mesh Stack suite batch 2: Boolean and Merge Meshes ops (2026-11-17,
  branch claude/project-thread-fcjo2t, specs meshops-specs-2026-11-17):**
  Boolean (`boolean.meshop.item`, BOP-xx) runs the direct Boolean's engine
  (`boolean_gathered`, shared) on the mesh below with a linked Driver
  Surface; Union/Intersect/Subtract/Add/Union All, Use World Transform
  (BOP-22). Merge Meshes (`pmodel.meshmerge`, MM-xx) adds linked meshes'
  results by world transform or own points, with Index, Merge Hierarchy,
  polygon type and map switches; deleted sources contribute nothing.
  Provisional: MOQ-15..20 (no create dialog: a second selected mesh
  becomes the Boolean's driver). GUI check: docs/MESH_STACK_SUITE_GUI_QA.md.
- **Mesh Stack suite batch 1: framework (2026-11-17, branch
  claude/project-thread-fcjo2t):** op types come from one table built-ins
  and kits share (`meshop_types.rs`, KIT-1/K7); the 9 v1 ops moved to
  `mesh_stack_builtin.rs`, and the Properties form is drawn from each type's
  fields (KIT-4). Ops can link other mesh items (`Item`/`Items` fields):
  linked results are evaluated first and placed in the reader's space,
  loops are refused when made and turned off on load (SAFE-07), deleted
  sources fail only their op (SAFE-08), hidden items evaluate (SAFE-09).
  A placeholder op runs once a kit registers its type. File format
  unchanged. Guide: docs/MESH_STACK.md. Provisional: MOQ-12..14. No op uses
  links yet; Boolean and Merge Meshes (batch 2) are the first.
- **Phase 0 doors (2026-11-17, branch claude/project-thread-bjattn, master
  plan §4 / ARCHITECTURE-RULES):** (1) `Item::kit` (kit_item.rs, KIT-1):
  an item of a kit kind loads without its kit, shows a "?" badge and
  tooltip in the Items list, draws as its dashed bounding box (Show
  Locators), and saves back byte for byte; scripts read its values,
  can't set them. (2) `Scene::time` (scene_time.rs): fps, start, end,
  current as exact ticks and a fractional frame rate (time_tick.rs,
  adapted from craft-libs; frames never drift at 29.97 etc.), saved when
  changed (native v49 with kit items), set and read by `item.channel ...
  scene:true` (seconds, fps, or `frame`). (3) `channels::Held` (constant only)
  that every channel read goes through at the current time. (4)
  `evaluate.rs`: `Editor::evaluated(Purpose)` is the one evaluated scene
  for viewport, Cycles, preview, export and picking. (5)
  `CommandRecord::scene_time`. (6) `click_select.rs`: every viewport and
  UV selection click resolves there. Provisional: time defaults 24 fps,
  0-5 s; picking stays on the Base Mesh (AR-04); kit items aren't
  pickable in the viewport (select them in the Items list); v49 may need
  renumbering if another branch merges a v49 first. GUI check:
  docs/DOORS_GUI_QA.md.
- **TextureCraft pieces (2026-11-17, branch claude/project-thread-rccgyy):**
  FBX export (own binary FBX 7.4 writer, read back by ufbx; other apps
  not yet tried), 3MF import/export, Texture > Atlas Unwrap…
  (`operandi.uv.atlas`, one undo step), Bake Curvature… / Bake
  Thickness… (16-bit PNG into the UVs, with UV padding and background
  fill) and Pad Texture to UVs… (`operandi.bake.*`, side effect). Code
  adapted from TextureCraft/craft-libs (MIT) is credited per file
  (docs/DEPENDENCIES.md). Painting engine parked unbuilt in
  `parked/texture-painting/`. Provisional: bakes run on the main thread;
  command names are placeholders (docs/NAMES-NEEDED.md). GUI check:
  docs/TEXTURECRAFT_PIECES_GUI_QA.md.
- **Loop Slice options (2026-11-16, branch loop-slice-options, spec
  loop-slice-2026-11-15 sections 4 and 7):** Keep Quads (with Slice
  Selected), Split with Cap Sections and Gap, and the router Profile with
  Reverse Direction, Reverse Inset and Keep Aspect are built and greyed per
  the spec table (UIC-92/117). New shared Profile layer
  `operandi-core/src/scene_items/profile.rs` with five Operandi shapes; only Loop
  Slice uses it so far (Bevel can move onto it later). Free-mode slices
  keep creation order (LS-71). Provisional readings LS-B04..B09; new
  questions LS-Q13 (grey Count/Position with a profile?) and LS-Q14 (Split
  with a profile). Tests: `tests/loop_slice_options.rs` (the measured
  numbers), app `slice_tools_tests.rs` (greying). GUI check:
  docs/SLICE_TOOLS_GUI_QA.md, Loop Slice 9-12.
- **Project file paths (2026-11-16, branch project-file-paths, Jason's
  design docs/PROJECT_SYSTEM.md phase 2):** files inside the scene's
  project are saved as `project:<rel>` (native v48) and found again in a
  copied project; outside files and the recovery journal stay absolute.
  Pictures, IES files and imported materials' images chosen or dropped
  from outside are copied into `references/` or `sourceimages/`
  (Preferences > File I/O, on by default). Render/Preview saves default to
  `renders/`, Export to `exports/`, MaterialX and material presets to the
  new `materials/`; `scripts/` added. Presets applied from the Preset
  Browser copy their files too (Jason, 2026-11-16); `preset.do` and other
  command lines don't. GUI check: docs/PROJECT_FILE_PATHS_GUI_QA.md.
- **Command History, stage H (2026-11-16, branch command-history, spec
  ui-architecture-2026-11-16):** every line `execute` runs leaves a
  command record (canonical line with arguments by name, class, source,
  outcome, sub-commands, time; `operandi-core/src/commands/command_record.rs`,
  app `command_history.rs`). Registry classes: `side_effect` and `quiet`
  added, every command has one (test). Undo steps keep their records
  (`Checkpoint::records`, `Editor::pending_records`); tool sessions
  record tool on, `tool.attr` per changed value at mouse-up, the drop and
  tool off; tweak Undo takes lines back. `cmds.batch` (one step, lines
  as sub-commands, capped by Max Sub-Commands Recorded), `cmds.undoTo`,
  `history.clear/max/resultsMax`. Command History panel
  (`command_history_ui.rs`, panel kind Command History): Undos (drag row,
  click/double-click, greyed redoable rows, sub-command arrows), History,
  Commands, Results, F filters, all on the shared tree list. In Model it
  is the right column's bottom, collapsed to the command entry (Up/Down
  recall). Prefs renamed Max Undo Levels; Max Sub-Commands no longer
  greyed. MCP bridge calls are recorded too. Not built (later stages):
  Scripts tab, Map Command to Key, Macro from History, Fire Again.
  Provisional: CH-Q01..Q11; FLAGS: viewport gestures don't run lines yet.
  Tests: `command_history_tests.rs`, core `command_record` unit tests.
  GUI check: docs/COMMAND_HISTORY_GUI_QA.md.

- **Layouts stage 3, batch 1 (2026-11-16, branch layouts-stage-3):** spec
  layouts v1.4 stage 3 (LAY-80..87, UIC-210..223). Toolbox forms: JSON
  files in `crates/operandi-app/assets/forms` (built in by build.rs), the
  user's copies in `Layouts/Forms` winning, `operandi.form.revert <id>`
  dropping them (`form/command_form.rs`). Command buttons, sub-forms,
  dividers (labelled = foldable section), head/tail slots, show-only-when,
  proficiency levels, Shift/Alt/Ctrl variant lines. Buttons read lit and
  greyed state from their command (new registry state answers,
  `command_runtime::CommandState`); an unknown line is drawn red and does
  nothing. The UV tab is the first tab drawn from a form
  (`toolbox.uv.json`, `toolbox_form.rs`), paint-identical to the old tab;
  the other tabs are unchanged. Escape hatch: one custom widget,
  `uv.mapField` (listed in `command_form::WIDGETS`). New
  `operandi.uv.preset` runs the island buttons. Tests find form buttons by
  command line (`toolbox_form_tests::button_by_line`). Provisional:
  LAY-Q20..Q25. GUI check: docs/LAYOUTS_STAGE3_GUI_QA.md.
- **Menus as forms (2026-11-17, branch menus-as-forms):** every menu is
  a form (UIC-10, UIC-210, UIC-213; `menu_forms.rs`). The bar is
  `menu.bar`; File, Edit, Select, Item, Geometry, Tools, View, Layout and
  Window and their submenus are `menu.*` forms, each checked against a
  snapshot of the old code in eight app states and each entry against
  the old action before the old code went. Live lists (open scenes,
  recent projects, layouts, palettes), `{title}` labels from a command's
  answer (Undo/Redo, Maximise), 20 new commands so every menu click is a
  Command History line (UIC-160); no UIC-164 menu gaps left. The
  in-window bar on Windows/Linux shows the same menus (its Window menu's
  Float checkboxes are gone). Deliberate changes and provisional
  choices: LAY-Q71..Q80. Tests: `menu_forms_tests.rs` (20, 13 of them menu
  snapshots). GUI check: docs/MENUS_AS_FORMS_GUI_QA.md.
- **Layouts stage 7 (2026-11-17, branch layouts-stage-7):** menus as
  forms and the Form Editor (LAY-83, LAY-87, LAY-Q53, UIC-10/210/213..215;
  `form_editor.rs`). Texture, Render and Help are forms (`menu.*`),
  checked identical to the old code before it went; a new System menu
  holds Form Editor. The Form Editor is a floating window (the
  `formEditor` palette): groups > forms > controls with search and Find
  Form; form and control properties (the workbench right-click's, plus
  Customizable Workbench, head/tail slot, show only when, columns,
  group); New Group, New Form, Add Command (or from the Command History),
  Add Sub-form, Add Divider, drag to reorder, Preview Form, Save Form,
  Undo Edit, Revert. Workbench right-click Show in Form Editor works. New
  `operandi.form.*` commands. Hard-coded menus (File, Edit, Select, Item,
  Geometry, Tools, View, Layout, Window) logged in LAY-Q66. Tests:
  `form_editor_tests.rs` (14). Provisional: LAY-Q66..Q70. GUI check:
  docs/LAYOUTS_STAGE7_GUI_QA.md.
- **Layouts stage 6 (2026-11-17, branch layouts-stage-6):** palettes as
  layout files (LAY-50..55, UIC-205..207; `palettes.rs`,
  `assets/palettes`). One toggle opens and closes each (menu, Float,
  `layout.window hash:`); opening an open one brings it forward. Sizes and
  places remembered (palettes.json, LAY-51 defaults). Palettes and every
  floating view belong to the app: a layout switch carries them over
  unchanged; switching back to a view's layout opens its pre-tear-off
  version with a fresh view in the slot (spec LAY-Q2 answer). ` hides and
  shows them. UV Editor (Texture menu, UV Maps double-click) is the
  uvEditor palette. Layout > Windows/Palettes lists them with Toggle
  Palettes; the Window menu's Float entries and Preset Browser moved
  there. Float Toolbox / Items / Properties are palettes with the same
  look. Not built: Images and Channels palettes (no panels yet, LAY-Q62);
  LAY-56 out of scope. Tests: `palettes_tests.rs` (13) + layout crate
  adopt/take. Provisional: LAY-Q60..Q65. GUI check:
  docs/LAYOUTS_STAGE6_GUI_QA.md.
- **Layouts stage 5 (2026-11-17, branch layouts-stage-5):** per-layout
  key maps (LAY-20, LAY-71, UIC-201; `layout_keys.rs`). A layout file
  holds its own `keys` (action -> chord, empty = unbound there); keys it
  doesn't set come from the global map. `bindings` is now the map that
  fires in the current layout, so every hint (buttons, menus, toolbox
  `{binding}`, tooltips) follows a switch. Input Map: "Keys for" Global or
  This layout (<name>), ◆/◇ marks with the layout named in the tooltip,
  Use Global per row, Clear this layout's keys. Revert keeps the keys
  unless its new box (or `keys:true`) says otherwise. New
  `operandi.layout.setKey` / `clearKeys`. Shipped Model and UV have no
  keys: bindings equal the factory map and paint is unchanged. Tests:
  `layout_keys_tests.rs` (12). Provisional: LAY-Q55..Q59 (lookup order:
  within each mode/tool context the layout's key wins). GUI check:
  docs/LAYOUTS_STAGE5_GUI_QA.md.
- **Layouts stage 4 (2026-11-17, branch layouts-stage-4):** workbench
  edit mode (LAY-82, LAY-86, LAY-87, UIC-212; `workbench.rs`). Two empty
  shipped workbenches: a Workbench tab last in the toolbox (in the strip
  once opened, Layout > Workbench) and a slim row along the top (Layout >
  Top Workbench). Edit Workbench shows the
  tools: Pick to Add (toolbox and menu buttons, Tool Properties rows,
  Preset Browser entries, Command History lines; green under the
  pointer; Esc stops), drag to reorder, right-click for Label, Tooltip,
  Popup Face, Alignment, Proficiency, Command, Toggle Style, Custom
  Icon..., Duplicate, Delete and a greyed Show in Form Editor. Labelled
  dividers fold in use, not while editing. Add Tool Pipe ▸ As Preset
  makes a one-click custom tool (`cmds.batch`); As Macro / As Script
  greyed. Undo Edit and Revert. Thirteen `operandi.workbench.*` commands.
  Paint check: every Model tab and the top bar paint identically at 1400
  and 900 pt tall (only font-atlas coordinates differ). Form
  file gains `workbench`, and controls `width`, `toggle`, `popup`,
  `iconFile` and `property`. Escape hatches: four `workbench.*` rows in
  `command_form::WIDGETS`. Provisional: LAY-Q47..Q54. Tests:
  `workbench_tests.rs`. GUI check: docs/LAYOUTS_STAGE4_GUI_QA.md.
  Nits round 1: the right-click menu stays open while its text fields are
  typed in (Enter or a click away sets, Esc cancels); Label starts filled
  in; a cleared Label makes an icon-only button, six to a row, with
  Reset Label (LAY-Q54); a tool button's own label now shows.
- **Layouts stage 3, batch 7 (2026-11-17, branch layouts-stage-3g, on
  3f):** the Create tab is a form (`toolbox.create.json`), so every
  toolbox tab is one. Checked paint-identical in one process (1196
  comparisons and the Tube menu open) before the old code went, with a
  test showing the check fails on a changed form. New form pieces:
  variant corner tags, labelled variants on tool buttons, `hold` cells
  (Tube / Solid Sketch), row `columns`, `look: tall`, `{binding:<line>}`
  and `{andBinding}`. Twelve new unit-primitive commands (`Cube Unit`,
  `Cube New Unit`...). No escape hatches. Changes only with user
  bindings (Pen tooltip), and Text / Arc modifier-clicks. Provisional:
  LAY-Q42..Q46.
- **Layouts stage 3, batch 6 (2026-11-16, branch layouts-stage-3f, on
  3e):** the Edit and Sculpt tabs are forms (`toolbox.edit.json`,
  `toolbox.sculpt.json`), each checked paint-identical in one process
  before its old code went (Edit 215 comparisons, Sculpt 99), with a test
  showing the check fails on a changed form. One deliberate change:
  Dicer / Julienne's greyed reason. The UV operations are a shared form
  (`toolbox.uv.ops`). New form pieces: `look: centered` and `look: weak`,
  a control's `shortcut`, widgets in rows, `{chord}`. New commands `Edit
  Slide`, `SDS Subdivide 2X`, `operandi.mode.is`. Escape hatches:
  `sculpt.pinchSlot`, `sculpt.weightMap`, `sculpt.newWeightMap`. Create
  stays on its own code for a later batch (LAY-Q41). Provisional:
  LAY-Q39..Q41.
- **Layouts stage 3, batch 5 (2026-11-16, branch layouts-stage-3e):** the
  Polygon tab is a form (`toolbox.polygon.json`), paint-identical to the
  old tab in 175 comparisons (each mode, nothing selected, every tool
  live, Pen / Spike / Bridge / Reduction armed, each dialog open, user
  bindings, folded, small icons, Alt / Shift / Ctrl held, and hovers over
  all 34 rows and titles), checked in one process before the old code
  went, with a test showing the check fails on a changed form. One
  deliberate change: the Pen tooltip (LAY-Q38). New in forms: `look:
  natural` and labelled variants with `{variants}` (Set Material's Alt:
  Set Part..., LAY-Q36). Nine rows light from new answers; modifier-clicks
  run the plain line (LAY-Q37). No new commands, no new escape hatches.
- **Layouts stage 3, batch 4 (2026-11-16, branch layouts-stage-3d):** the
  Vertex and Edge tabs are forms (`toolbox.vertex.json`,
  `toolbox.edge.json`), paint-identical to the old tabs in 33 states
  (each selection mode, tools live, Weld / Add Loop / Add Point / Bridge
  armed, Show Vertices on and off, folded, and hovers over rows, tool
  buttons and greyed rows), checked in one process before the old code
  went, except one deliberate change: a greyed New Vertex... / Reorder
  Index... shows its tooltip under the reason (LAY-Q34). New form
  controls: `look: check`, `look: button`, the `{binding}` tooltip
  placeholder. New commands `Add Loop`, `Add Point`, `Drop Weld`; Weld,
  Bridge, Add Loop, Add Point and Show Vertices light from new answers,
  New Vertex / Reorder Index grey from theirs. No new escape hatches.
  Provisional: LAY-Q33..Q35.
- **Layouts stage 3, batch 3 (2026-11-16, branch layouts-stage-3c):** the
  Deform and Duplicate tabs are forms (`toolbox.deform.json`,
  `toolbox.duplicate.json`), paint-identical to the old tabs at rest, with
  Push / Smooth / Element Move / Mirror / Radial Array live, with Soft
  Move, Flex, Bend and Flare open, with Quick Align on, in Item mode and
  with each section folded (checked in one process before the old code
  went), except one deliberate change: the Soft Selection Move flyout,
  whose tools are all unbuilt, is now greyed (LAY-Q30). New form
  controls, built in the form component rather than as escape hatches:
  `flyout` sub-forms (hold or right-click, the pick becomes the face),
  `heading` titles, `look: flat` tool rows and `noShortcut`. Preset rows
  and Quick/Ground Align light from new state answers. No new commands
  and no new escape hatches. Provisional: LAY-Q30..Q32.
- **Selection Sets rows (2026-11-16, branch layouts-stage-3c):** the Select
  tab's Assign / Create Group / Use / Delete Selection Set rows run bare
  `select.editSet` / `select.convertGroup` / `select.useSet` /
  `select.deleteSet`, which open the commands' dialogs (CMD-12). They grey
  from the commands' answers (UIC-223) with the dialogs' own refusals: tool
  live, not Vertex/Edge/Polygon mode, no visible mesh, nothing selected
  (Assign), no sets (the rest). Bare Assign now refuses with nothing
  selected instead of opening a dialog that OK would refuse.
- **Layouts stage 3, batch 2 (2026-11-16, branch layouts-stage-3b):** the
  Select tab is a form (`toolbox.select.json`), paint-identical to the old
  tab at rest, with a tool live, with Symmetry on and folded. Its rows are
  the selection commands' lines; they grey from new state answers (tool
  live; Symmetrical also needs Symmetry). New in forms: `space` gaps,
  `menu` sub-forms as flyouts (Convert), `{shortcut}` in tooltips.
  Placeholder rows are `operandi.notBuilt "<name>"` (new, quiet). Escape
  hatch: the three drop-downs (Lasso Style, greyed Select Falloff and
  Polygon Tag). Duplicate and Deform stay as they were (not plain rows).
  Provisional: LAY-Q26..Q29.
- **Layouts stage 2 (2026-11-16, branch layouts-stage-2):** spec
  BEHAVIOR-LAYOUTS-STAGE2.md (UIC-12..18, UIC-192..200, LAY-42..48,
  LAY-60, LAY-72..74). Layout menu: New (dialog, copy box off), Save
  Layout, Save Layout As, Rename / Delete (layouts you made), Reset (asks),
  Clear, Layouts, Recent Layouts, the two locks (now per layout). The bar:
  collapse, stars, drag to reorder with an insert line, "+", starred only,
  double-click / right-click Reset (asks). Quick switcher on Ctrl+Shift+Tab
  (MRU, release switches) and the Layouts pie on Ctrl+`. Saved viewports
  and groups (pieces) in `Pieces/`; saving again updates every layout that
  shows them. The content arrow on view-area panels (3D View, UV View,
  Empty, saved pieces, Save Viewport items, Layout items); an empty panel
  has the corner dot and the arrow, so a layout builds from one empty
  panel and reopens as built. Every action is a command (`operandi.layout.*`,
  `layout.lock*`, `operandi.viewport.*`, `operandi.panel.kind`); the reset
  question now runs `operandi.layout.revert` (FLAGS UIC-220 gone).
  Provisional: LAY-Q11..Q19 (view area only for building; shared side
  columns and the bottom Layouts button not built). Tests:
  `layouts_edit_tests.rs`, `operandi-layout` graft/replace. GUI check:
  docs/LAYOUTS_STAGE2_GUI_QA.md.

- **Click to start (2026-11-16, branch click-start, spec
  tool-click-start-2026-11-16):** Auto-Activate per tool (CS-01/05): a
  column on preset rows plus `[[tool]]` rows in
  `factory-tool-presets.toml`, read by `tool_preset::auto_activate`. A
  tool flagged off is armed when picked (`click_start.rs`): values edit,
  nothing drawn, mesh unchanged; Apply runs once and stays armed (CS-21);
  Drop/Cancel change nothing (CS-22); status CS-06. The first press in a
  3D view (`Command::ToolClick`) puts an Automatic centre at the click
  (snap > surface > work plane), runs Bend's spine fit, shows the result,
  then goes on as the tool's own press (CS-15). Waiting: Bend, Push,
  Flare, Smooth, Thicken, Extrude, Smooth Shift, Edge Extend, Element
  Move, Shear, Taper, Pole, Magnet, and Polygon/Edge/Vertex Bevel + Edge
  Chamfer (approved deviation CS-07). Primitives, Duplicate tools and
  slices keep their own waiting. The app honours the flags; the test app
  and bare core editors don't unless asked. Provisional: CS-Q02..Q04.
  Tests: core `tests/click_start.rs`, app `click_start_tests.rs`. GUI
  check: docs/CLICK_START_GUI_QA.md.

- **Bend spine fit (2026-11-16, branch click-start, BN-24 revised,
  BN-30a/b):** each centre click in a view (`Command::ToolClick { point,
  plane }`) refits the spine over the target's box (selection, else whole
  mesh): of the two work-plane axes, the one with the larger distance to
  the box's far side, toward it, that long; never the view axis. The click
  lands on a snapped point, else the mesh surface (provisional CS-Q02),
  else the work plane, and sets Axis to the plane's normal. Script
  `SetActionCenter` still only moves the centre. Tests: core
  `tests/bend_spine_fit.rs`.


- **Layouts stage 1 (2026-11-16, branch layouts-stage-1):** spec
  layouts-2026-11-16 (LAY-70/71/75, LAY-01..61, UIC-190..221). One layout
  file describes the whole main window (`operandi-layout` `WindowLayout`:
  frame tree with fractions, tab groups, grids, panel kinds by registry
  name with their settings, name/order/star, open command, reserved key
  map context, icon). Shipped Model and UV are files in
  `crates/operandi-app/assets/layouts`, built in by build.rs; a third is
  added by adding a file (tested). `layout.restore <name>` behind the
  layout bar and the new Layout > Layouts; each layout keeps its state
  while another shows (views, cameras, view settings, toolbox tab, widths,
  tabs) and writes `Layouts/<name>.json` once it differs from the shipped
  one. The app opens on the layout open at quit. The first run reads the
  old view-layout.json / view-displays.json / layout.json once as Model;
  they aren't written any more. Unknown panel kinds show as named empty
  panels; other file versions fall back to the shipped file. The UV
  stopgap `uv_layout.rs` is gone (UV Editor code now `uv_editor.rs`); new
  `operandi.toolbox.tab` is the layouts' open command. Provisional:
  LAY-Q1..Q10; UIC-220 exception in docs/FLAGS.md. Tests:
  `layouts_tests.rs`, `operandi-layout` `window.rs`. GUI check:
  docs/LAYOUTS_STAGE1_GUI_QA.md.
- **UV editing stage 1 (2026-11-16, branch uv-stage-1):** specs
  uv-editing-2026-11-16 (UVE-1, UVT, UVS-1 section 4). UV map list (E1,
  `uv_maps.rs`): New (default "Texture", then "Texture 2"...), select,
  rename, delete several, copy / paste a whole map, all undoable; the
  selected map is saved (native v47) and is what the UV view, UV tools and
  Unwrap use. New "UV Maps" panel (Data Lists, shared tree list) and the UV
  view's map button. Move / Rotate / Scale / Transform started in a UV
  view edit UVs (E6, `uv_edit.rs`) through the same tool session: flat
  handles (U, V, centre, ring, scale boxes), drag anywhere, Ctrl axis lock
  and Rotation Snap steps, Ctrl/Cmd+Shift Tear Off (and Transform's Tear
  Off field), grid snap with Snapping on, falloff, action centre, one undo
  step per drag; shared UVs move together, seams move apart, 3D never
  moves, hidden polygons stay. One island affine engine (E7) with a rule
  table and Entire / Island / Polygon / Edge row grouping: Fit UVs, Flip
  U/V, Mirror U/V, Align Up/Down/Left/Right as tool preset rows on the
  Edit tab's UV group (plain, Shift, Alt; Ctrl/Cmd opens the dialog with
  remembered values). Paint (left-drag) and lasso (right-drag) selection
  in the UV view. Checker overlay (UVT-114, an approved Operandi extra,
  Jason 2026-11-16): UV view header toggle and the 3D view's Display menu,
  numbered 8 × 8 per tile, drawn per pixel in the fill shader, off by
  default. Provisional: UVE-1 Q1-Q6, UVT-Q03/Q05/08, UV-B01..B05.
  Tests: core `tests/uv_editing.rs`, app `uv_editing_tests.rs`,
  `uv_checker.rs`. GUI check: docs/UV_EDITING_GUI_QA.md.
  Add-on (UVT-200..203, `uv_layout.rs`): a layout bar under the menu bar
  with Model and UV tabs (UIC-11; double-click asks, then resets, UIC-15);
  the UV layout is the toolbox at a new UV tab (Map field, transform
  tools, Unwrap, Pack, island buttons), a UV view beside a Perspective
  view 50/50, and the UV Maps list in the side tabs. Texture > Open UV
  Editor, or double-clicking a map in the UV Maps list, opens a UV view in
  its own OS window (800 × 600, UIC-183) with the UV tools down its right
  side. View > UV View stays the per-view switch. Provisional: UV-B06/B07;
  UV-B03 settled.
- **Drag controls and Loop Slice drags (2026-11-15, branch drag-layer):**
  spec loop-slice-2026-11-15 v0.1. One shared drag layer ("hauls" in
  code, `operandi-core/src/tools/haul.rs`, table
  `presets/factory-hauls.toml`, viewport `operandi-app/src/view/haul_view.rs`):
  a left, middle or right drag away from the handles changes the value the
  tool's row lists for that button and direction, through that value's
  Tool Properties field, live, one undo step per drag; the first 4 px lock
  the axis, whole numbers step every 12 px, right/up = more. Loop Slice:
  left-drag off the bar moves the current slice (Edit = Move), right-drag
  sets Count, middle-drag sideways Current and up/down Inset; the bar works
  as before and a click still picks a ring. New Loop Slice Inset (no
  profile). Also wired: right-drag Polygon Bevel Segments, Polygon Extrude
  Sides, Edge Bevel Round Level, Edge Chamfer / Chamfer edit Segments
  (Chamfer edit up: Source Segments), Vertex Bevel Round Level, Polygon
  Slice Gap, Sphere/Cylinder Sides (up: Segments); middle-drag Edge Bevel /
  Chamfer Depth and Smooth Shift / Thicken Scale (up: Max Angle). Mac
  trackpad: Cmd+drag = middle, Cmd+Shift+drag = right. Provisional: DL-B01..
  B04, LS-B01..B03. Not built (stay hidden as unbuilt): Loop Slice Keep
  Quads, Split / Cap Sections / Gap, Profile and its Reverse / Keep Aspect
  switches. Tests: core `tests/haul.rs`, app `haul_tests.rs`. GUI check:
  docs/DRAG_LAYER_GUI_QA.md.
- **Tool presets stage 1 (2026-11-15, branch tool-presets):** spec
  tool-presets v0.2. One table of recipe rows,
  `crates/operandi-core/presets/factory-tool-presets.toml`, read at start
  (`tool_preset.rs`): each row names its pipe layers (action centre, action
  axis, falloff, snapping) and its tool or command. Starting a row
  remembers the user's layer in every slot it names and gives all of them
  back on drop, Cancel, another tool or Undo (TP-10..14), and row-to-row
  switches give the first row's back first. The Deform presets (Soft Move,
  Shear, Twist, Swirl, Taper, Bulge, Flex), Thicken, Element Move, Push,
  the Transform tools, Weld and the Align tools are rows; a golden file
  recorded on main proves the Deform presets give identical points,
  falloffs, action centres and history. New rows: Flare (Bulge flyout,
  `tool.set Flare on`) and Flip It / Flip It Origin (Polygon tab ▸
  Commands, axis dialog, `flip it x`). Reset under Thicken now gives
  Thicken's values (TP-15). Provisional: TP-B01..B06. Tests:
  `tests/tool_presets.rs`, `tests/tool_preset_golden.rs`, app
  `tool_presets_tests.rs`. GUI check: docs/TOOL_PRESETS_GUI_QA.md.
- **Join with Symmetry (2026-11-15, branch join-symmetry):** edge Join /
  Join Averaged with Symmetry on join each side's pair (four picked edges)
  in one weld, so pairs meeting at the centre line weld their shared points
  once; two edges join as before, other counts refuse with how many sit on
  each side. Vertex Join welds each side by itself when both sides have two
  or more points. Shared side-sorting helper `symmetry_sides::by_side`.
  Provisional: SYM-JOIN-Q1. Tests: `tests/join_symmetry.rs`.
- **Twist Bevel (2026-11-15, branch twist-bevel):** spec twist-bevel v1.1.
  New polygon tool (Polygon tab next to Bevel, Tools menu,
  `tool.set poly.twistBevel on`, input action "Twist Bevel", no default
  key): bevels the selection out into a tube of rings that tapers (Inset,
  Profile, Profile Bias), twists (Rotation about the spine or the action
  axis), bends (Bend, Bend Direction) and spaces its rings (Segment
  Spacing, Center Bias). Built on Bevel's own ring builder with a
  per-ring placement step (`bevel::RingPlacer`, `twist_bevel.rs`), so
  walls, materials, SubD type, UVs and the cap selection are Bevel's.
  Handles: Shift arrow (snaps the cap centre), Inset square, twist ring,
  bend arrow, Bend Direction ring; drag anywhere = Shift; Shift+click
  continues on the caps; one undo step per twist bevel. Falloff weights
  each tube at its base centre; Symmetry makes mirror-image tubes. Credits
  in About and docs/ABOUT.md. Provisional: TB-B01..B10 (Tool States left
  out). Tests: `tests/twist_bevel.rs`, app `twist_bevel_tests.rs`. GUI
  check: docs/TWIST_BEVEL_GUI_QA.md.

- **Selection inference (2026-11-15, branch selection-inference):** spec
  selection-inference v1.1. Point tools (Vertex Bevel, Extrude, Merge,
  Slide) and edge tools (Edge Bevel, Chamfer, Extrude, Extend, Slide,
  Crease, Relax) start from any component mode: edges and polygons give
  their points, polygons their outline (every edge with no outline),
  points the edges between them. One shared step first in the tool pipe
  (`selection_inference.rs`, tool card `takes`); the mode and the
  selection you made stay; one undo step; status line says what it works
  on; the inferred elements show on the SubD surface while the tool is
  open. Join, Join Averaged, Align, Merge, edge Split and Split Normals
  infer too; polygon tools refuse Vertex/Edge mode with a reason. Set
  Position in Edge mode with nothing selected moves every point (M05).
  Provisional: SI-B01..B06. Tests: `tests/selection_inference.rs`. GUI
  check: docs/SELECTION_INFERENCE_GUI_QA.md.

- **SubD elements on the surface (2026-11-15, branch subd-pick):** on a
  subdivided mesh, points, edges and polygons pick, pre-highlight and draw
  where the smooth surface shows them, not on the cage (Jason: the default
  for every tool). Points whose polygons are all smooth sit where their
  traced edges meet (`SubdivisionCache::shown_points`); selection clicks,
  Closest rollovers, lasso, point dots, Tweak (hover and grab) and the
  edge clicks (Add Loop, Add Point, Poly Haul, action-centre click) use
  them, and Edge Slice / Loop Slice picking (Edge Slice points show on
  the surface too). Snap targets stay on the cage by Jason's call
  (2026-11-15: snapping to the cage, constraints to the SubD surface).
- **Stuck zoom (2026-11-14, branch fix-stuck-zoom):** Ctrl+Alt drag zoom
  (toward the press), `.` / `,`, the header Zoom button and 3D mouse
  push/pull (toward the view centre) now use the wheel's Zoom to Surface
  step (`zoom_toward_surface` in `view_nav.rs`), so they no longer stall
  3 cm from a far orbit centre. Over empty space, ortho views or with the
  option off: as before. Symmetry has no part in navigation (tested); the
  wheel already reached the surface with Symmetry on in full-frame tests,
  so a wheel-only stall in Jason's scenes is still unexplained
  (docs/STUCK_ZOOM_GUI_QA.md SZ-01/06).
- **Registry handlers (2026-11-14, architecture review pick E, branch
  claude/project-thread-n8l909):** registry entries carry their handler;
  run-time `Registry` with namespaced `register()` (core
  `command_runtime.rs`); `command_exec::SPECS` folded into the entries;
  the app's typed `match` and benchmarks run as registry handlers
  (`command_handlers.rs`). No behaviour change. Still on the old chain:
  `canonical_command` and its sub-parsers, sculpt, pie and attribute
  lines (docs/COMMAND_REGISTRY.md, stage 3).
- **Snapping spec v2.3 (2026-11-14, same branch):** Box snaps to the 8
  corners of each item's own box plus one box around all meshes; an
  axis-arrow drag hunts with a free reticle and lands at the target's
  position along its axis (Constraint Snapping on or off, Q-14); a small
  Unsnapped-colour reticle shows where the point would be unsnapped
  (SN2-43b); nearest target wins, exact ties within 2 px by type order.
- **Snapping spec v2.2, stage 2 (2026-11-14, branch claude/project-thread-ofjqsa,
  spec snapping-2026-11-14 BEHAVIOR-SNAPPING-V2.md, project files):** all
  twelve snap types in the shared snap layer (`snap_kinds.rs` rules,
  `snap_service.rs` search): Edge, Edge Center, Polygon (under the handle,
  only when nothing is in Inner Range), Polygon Center, Intersection (edge
  through polygon), Box (27 points of the item's own box), Pivot (Snap
  Size), ties within 2 px by type order (SN2-15); Pen locks to World Axis /
  Straight Line / Right Angle within 5°. Grid wins only with nothing in
  Inner Range; Use Fixed Grid; Layers, Only Selected, Depth Limitation, Pre
  Highlight, Constraint Snapping (axis arrows snap along their axis).
  Per-Mode presets with (New Preset) / (Remove Preset). F11 opens an
  in-app popover at the pointer (UIC-181/185: no title bar, pin circle,
  closes on leave/Esc/F11 again, one unpinned popover at a time); rows
  have no status lights (UIC-112); Coordinate Rounding, Fixed Increment
  and Snap Angle are the Units and Rotation Snap Angle settings. New
  Element Snap key Ctrl+Shift+T (Drag Snap). Unsnapped theme colour for
  the pre-highlight marker. 1M points: 0.14 ms per move vertex-only,
  1.7 ms with every point and edge type on.
  Jason 2026-11-14: Coordinate Rounding's factory default stays None (the
  spec's Fine is not used), and the option groups appearing only while
  their type is ticked is an approved departure from UIC-92. Left for
  stage 3: Show Grid overlay, 2D Snap, Fixed
  Snap, Snap Angle stepping rings and angle handles while snapping, World
  Axis on Mirror, Item Snap (Shift+X in Items mode), per-type highlight
  looks, the generic panel Presets menu, and a Window-menu floating
  Snapping window (UIC-184).

- **One snapping layer (2026-11-14, branch claude/project-thread-ofjqsa,
  architecture review pick B):** every point snap (tool handles and Tweak,
  Curve/Pen/Tube points, Draw's join-a-point with snapping off, Ruler ends,
  Weld drag, symmetry seam) goes through `snap_service.rs`, one of the
  tool-pipe layers; radii live in its `Role` table, unchanged. Targets are
  gathered once per scene and sorted into a screen grid once per view:
  0.12 ms per pointer move on a 1M-point mesh (15 ms grid rebuild after the
  view moves). The 200k-point skip in Draw and the Ruler is gone. Behaviour
  change (brief): points hidden behind the surface never snap anywhere;
  current-scene snaps hide by the picking surface, as Weld did (Draw and
  Pen snaps used the cage), tool handles keep the cage. Angle snaps share
  core `angle_snap.rs`. Tests: `snap_service_tests.rs`; GUI check:
  docs/SNAP_SERVICE_GUI_QA.md.
- **One drag, one undo step (2026-11-14, branch claude/project-thread-pyqiyz):**
  drawing a Box (and its right-drag Segments) and a Sketch Extrude stroke
  recorded a step per mouse move, because their updates reached the editor
  without the "pointer held" flag (there since mid-tool undo, 2026-11-04).
  Every viewport press now holds the tweak in the editor until release
  (`hold_pointer` / `release_pointer`), as does a field scrub, so no tool
  sets it on its own. `drag_undo_guard_tests.rs` drags every tool on empty
  space, the mesh and each handle, both buttons, and fails above one step.

- **X-Ray / Ghost buttons (2026-11-14, branch xray-buttons, spec
  xray-toggle-2026-11-13):** Normal / Ghost / X-Ray buttons in each 3D
  view's header (click: active meshes, Alt+click: inactive), greyed in
  Wireframe; they run `view3d.TransparencyOverride`, so Viewport Properties
  and the buttons agree. Input Map phrases for all six, plus `Toggle X-Ray`
  (`operandi.view3d.toggleXRay`, unbound). Provisional: picking doesn't go
  through clear faces (XR-Q2); glyphs are drawn shapes like the other
  header buttons, not generator icons yet.

- **Big SubD surfaces, pass 2 (2026-11-13, branch claude/project-thread-pj3a5s,
  docs/SUBD_GPU_DESIGN.md):** drags update only the patches near moved
  points and re-send changed chunks (1-point drag on the 4.5M sphere ~24 fps,
  was a 3.5 s rebuild a frame); image materials draw per pixel on patches
  with SDV-15 UVs; Fade Wireframe on Small Polygons fades patch wire, keeping
  outline, open and sharp edges. Provisional: procedural patterns, locked
  wire colour, see-through and shadows on patches.
- **Big subdivision surfaces on the graphics card (2026-11-13, branch
  claude/project-thread-pj3a5s, design docs/SUBD_GPU_DESIGN.md):** Tab
  surfaces of 500k+ pieces draw as biquadratic patches sampled in a vertex
  shader (~12 bytes per triangle instead of ~1 KB), wire as one strip per
  cage edge, clusters facing away culled on the card. The 4.5M-polygon
  sphere at level 2 (144M triangles) orbits at 28.5 fps (Modo reference
  ~25), surface memory +1.6 GB. The 4M-polygon cap is gone; Tab, Shift+Tab
  and Subdivision Settings now refuse surfaces bigger than free memory.
  Provisional on patches: image materials draw their base colour, no
  drag patches yet (whole rebuild), no see-through or shadows. Shift+Tab
  via OpenSubdiv is a later stage.
- **Many objects and the wire fade (2026-11-13, branch claude/project-thread-sbitxm):**
  perf task 2. Item lookups by id go through a remembered index
  (`Scene::item`, `scene_index.rs`; 334 call sites that searched every
  item); the Items list lays out only rows on screen; small items' fill
  and wire live in shared vertex buffers (`gpu_arena.rs`) and neighbours
  drawn alike join one draw call; items drawn at the same matrix share one
  placement uniform (a drag of thousands writes one); set lookups in
  multi-item selection and transforms. 10k spheres, M1 Max, real device:
  still frame 184 → 31 ms, drag one 534 → 81 ms, drag all 1,373 → 190 ms,
  undo 930 → 190 ms; select all (1.3 s) and the first click (1.1 s, the
  picking surface) are still slow. New preference **Fade Wireframe on
  Small Polygons** (Preferences > Display, on by default, Jason): plain
  mesh wire on edges under 6 points fades, under 2 points is not drawn
  (thresholds provisional; edges seen end-on fade too). Refined with Jason
  the same day: Wireframe-only groups never fade; outline (one side facing
  away), sharp (over 40°, provisional) and open-border edges never fade,
  nor a subdivided polygon's traced cage edges (`EdgeVertex::sides`,
  `always`; shader `kept`); an inactive mesh of 10,000+ polygons
  (provisional) too small on screen for its wire also shows its bounding
  box's corners in the new theme colour Dense Mesh Box (Display > Colors >
  Geometry, blue-grey). Benchmarks:
  `perf_view_tests.rs` (`perf_objects`, `perf_view`), scene `subcage70m`.
  Found: subdivided surfaces over 4 million polygons are refused and the
  cage is drawn instead (subdivision.rs, since 2026-11-02), so the earlier
  10M/50M subdivided timings were cage drawings; the real 10M surface
  orbits at 79 fps but uses ~11 GB. Checks: picture capture identical to
  main with the fade off; `drawing_equivalence_tests.rs`
  (`wire_fades_only_where_polygons_are_tiny`), `gpu_arena` and
  `scene_index` unit tests.

- **Draw point modes, repeat transform, drag snap (2026-11-13, branch draw-edit-repeat-snap):**
  specs draw-edit-repeat-snap-2026-11-13. *Repeat:* middle-click with
  Move/Rotate/Scale/Transform re-applies the last amount as a new undo
  step (HM-04 was there); a typed amount still showing now counts as the
  first apply, and dropping or re-activating the tool forgets it (RPT-08).
  *Point modes:* Mode is a segment row (UIC-89) on Curve, B-Spline, Bezier
  (adds Insert: split a span by de Casteljau, shape unchanged), Pen (D1)
  and Tube, remembered per tool; Delete-mode hover shows in the error
  colour; middle/Shift+middle-click deletes on Pen and Tube too. Pen and
  Tube add after the current point; a Make Quads delete takes the pair
  (D2); Pen Undo now steps back through adds, drags and deletes. *Drag
  snap:* draw-tool points, Bezier handle ends, Pen and Tube points run the
  snap targets (Inner/Outer Range, pre-highlight, X hold) on every move,
  own points are targets, Ctrl snaps on the axis line; with snapping off
  curve points still join a mesh point within 8 px (as before). RPT-11 not
  built (D3). Provisional DPM-Q5..Q9, RPT-Q5, DSN-Q6..Q8. Tests:
  `draw_point_mode_tests.rs`, `ctrl_constrain_tests.rs` rpt_*. GUI check:
  docs/DRAW_EDIT_REPEAT_SNAP_GUI_QA.md.
- **Edge Extend Rotate/Scale (2026-11-13, branch claude/project-thread-js1l2r):**
  SPEC-EDGE-EXTEND-ROTATE-SCALE v1. Edge Extend now has Rotate X/Y/Z and
  Scale X/Y/Z about the action centre (which follows the Offset), Local
  Inset/Shift, Handles (Move/Rotate/Scale/Local; Move only by default,
  spec v1.1), Plane circles (Move or Scale), Haul (Global or Local Offsets),
  Make UVs (None/U/V, U by default) and Shift+click to extend again with the same values.
  Core `edge_extend_shaped` works in world space; the handles reuse
  Transform's layout (`viewport/extend_handles.rs`). `tool.attr edge.extend`
  takes the reference names (offX, rotY, sclZ, segs, planeHandle,
  hauling, uvs…). Provisional: order, Make UVs values, Inset direction
  (EXT-RS-Q1..Q4, Q7); defaults and integer order answered (Q5, Q6). Tests: `edge_extend_rotate_scale.rs` (EXT-RS-01..04),
  `extend_handles_tests.rs`. GUI check: docs/EDGE_EXTEND_ROTATE_SCALE_GUI_QA.md.

- **Bend v2 (2026-11-16, branch bend-v2, spec bend-handles-2026-11-15):**
  Deform tab ▸ Bend, Ctrl+E, `tool.set xfrm.bend on` / "Bend" all go through
  the tool-presets row `xfrm.bend` (Automatic centre + Auto axis, given back
  on drop). Blue Angle ring at the centre square to the Axis row (X/Y/Z),
  drag it to bend, Ctrl steps; spine-end marker drags the spine (Ctrl one
  work-plane axis, snaps); any other press moves the centre through
  snapping. Thin straight violet spine, no arc overlay, no haul. Panel:
  Angle, Spine X/Y/Z, Both Sides; Auto Action Center X/Y/Z + Deform Space;
  Action Axis Auto. Provisional BH-Q01..Q11, BH-B01..B03; not built: Slip
  UVs, Morph, Action Axis Handle. Tests: `bend.rs`, `tests/bend.rs`, Deform
  tab UI tests. GUI check: docs/BEND_GUI_QA.md.
  v2.1 (measured section 5): maths already matched (BH-Q01..Q03 answered,
  BD-01..05 in `tests/bend_v21.rs`). Added: script Angle in degrees
  (BN-46), values remembered between uses + Reset button / `tool.reset`
  (BN-47; centre from the action centre, BH-Q13 provisional), axis along
  the spine moves nothing and says so (BN-45, BH-Q12 provisional).

- **Selection falloff and Flex (2026-11-13, branch claude/project-thread-ii6xy3):**
  Falloff menu ▸ Selection (also Add Falloff, `falloff.selection steps shape
  p0 p1 mix`, MCP `tool_run` falloffs) weighs 0 on the selection's border,
  rising over Steps rings inward; points outside weigh 0. Rings are
  worked out once per drag (`item_selection_rings` memo). Flex (Deform tab,
  Shift+Y, `tool.set "Flex" on`) is Transform + Selection falloff (Steps 2,
  Smooth) + Selection Border centre and Selection axes, given back on drop;
  right-drag changes Steps. All provisional (SF-Q1..Q7): nothing measured.
  Tests: `tests/selection_falloff.rs` (SF-01, SF-02, FX-01), Deform tab UI
  tests, Flex speed guard. GUI check: docs/SELECTION_FALLOFF_FLEX_GUI_QA.md.

- **Cheaper plain-mesh drawing and Item mode (2026-11-13, branch claude/project-thread-sbitxm):**
  the wire was ~90% of a plain mesh's frame (fill alone: 9 ms at 10M
  triangles). An edge two plain polygons share is now stroked once with
  the opacity of two blended strokes (`EdgeVertex::uses`, `SideMates`
  topology memo), halving the strokes; pictures match within 1/255. A
  whole selected item draws its unselected geometry with the selection
  fill and highlight applied by the shader (gpu.rs `Selected`), so Item
  mode switches and item select/deselect rebuild nothing (built selected
  only with symmetry pairs). M1 Max, real-device timing (`perf_view_tests.rs`):
  1M plain orbit 18 → 10–12 ms, 10M tris (5M quads) 74 → 33–35 ms, Item
  mode orbit 42 → 18–20 / 143 → 67 ms, switch to Item mode 2.1 s → 26 ms
  (1M) and 18.7 s → 65 ms (5M). Not reached: 60 fps at 10M plain triangles
  (the blended wire strokes are still ~75% of the frame; going further
  changes the look). One record per stroke drawn instanced was tried and
  was slower on Apple GPUs. Checks: `drawing_equivalence_tests.rs` (new vs
  reference drawing, every run) and `picture_capture_tests.rs` (opt-in,
  compares pictures between builds).

- **Pen Make Quads partners follow curves (2026-11-15, branch pen-quad-guess):**
  The guessed partner is the last rung turned with the strip (was the
  starting edge copied in world space, which folded quads round an eye),
  re-guessed during the click's own drag, and on a constrained background
  laid on the surface keeping the rung's length. Provisional: PMQ-Q4/Q5
  (one-click lag on steady curves). Tests: arc/S-curve/drag cases in
  `pen_make_quads.rs`, plain and SubD eye in `background_constraint.rs`.

- **Pen Make Quads (2026-11-13, branch claude/project-thread-irzez8):**
  SPEC-PEN-MAKE-QUADS. Make Quads in the Pen's Tool Properties (was
  greyed): two clicks set a starting edge, each later click adds the point
  and its partner (click + B − A) and one quad; Ctrl+click ends the strip
  with a triangle; Shift+click starts a new strip; strip points drag; live
  preview of the quads; Drop turns the strip to face the viewer; one undo
  step per strip, Cmd+Z mid-strip takes back a click. Provisional:
  PMQ-Q1..Q10 (Wall Mode exclusive, facing, mid-draft undo, partner on
  turns, no weld/background constraint). Tests: `pen_make_quads.rs`
  (PMQ-02..04 and the criteria), app test
  `make_quads_clicks_ctrl_triangle_shift_new_strip_and_drop`. GUI check:
  docs/PEN_MAKE_QUADS_GUI_QA.md.

- **Fast drags on big meshes (2026-11-13, branch claude/project-thread-sbitxm):**
  a drag frame now costs what moved, not the whole mesh. `Mesh.faces`,
  `uv_maps` and `attributes` are shared copy-on-write (`Shared`), so a
  moved copy shares its polygons; `prove_points_moved` validates only the
  moved points and their polygons; identity, topology, picking-free
  readouts (polygon count/type, material names, `has_smooth`), world-bound
  and symmetry-tolerance checks are remembered per allocation; the GPU
  patch no longer places every point, re-reads the highlight or rebuilds
  point dots, and whole-mesh drags find their map without reading every
  point. On edit1m (1M polygons, Mac Studio M1 Max, `perf_drags_tests.rs`,
  which waits for the viewport's own GPU each frame): drag
  1 polygon 275 → 23 ms, 100 points 261 → 25 ms, 10k polygons 318 → 57 ms,
  first drag frame 1.7 s → 55 ms, drop 385 → 18–56 ms, undo/redo
  220–300 → 20–64 ms. The still frame is ~18 ms (GPU drawing the plain
  mesh), so 60 fps needs the plain-mesh draw work (next perf task). The
  first undo after opening still builds the picking surface (~0.7 s, as
  the first hover does). The snapshot harness (`perf_snapshot_tests.rs`)
  waits on a second graphics device, so its frame times leave out the
  viewport's GPU work; its edit1m CPU-side numbers went to 0.8 ms orbit,
  6.5 ms one-polygon drag, 3.5 ms undo. Tests: `drag_patch_tests.rs` (patched geometry
  equals a full build every frame, after drop, undo and redo),
  `points_moved_tests` in mesh.rs. GUI check:
  docs/DRAG_SPEED_GUI_QA.md.

- **HDRI environments (2026-11-12, branch hdri-environment):** Environment
  tab's Environment Image section (Load Image…, Clear Image, Rotation =
  image locator heading), `operandi.environment.loadImage`,
  `texture.delete`; HDRI drawn as the view's background on the GPU (same
  lookup as Cycles) and lighting the Advanced style (SH irradiance +
  blurred reflections); .hdr/.exr dropped on a view's empty space becomes
  the environment (UIC-74 deviation, Jason OK); Cycles importance-samples
  a world with a picture; environment presets (.openv, Replace/Add; lights,
  meshes and cameras land in one item group named after the preset,
  materials in a same-named Shader Tree group, both marked, native v42). Provisional: one blur level for all
  roughness; only the top image layer draws in the view; PNG/JPG onto the
  Shader Tree's Environments section isn't built. GUI check:
  docs/HDRI_ENVIRONMENT_GUI_QA.md.

- **Big meshes: viewport buffers and Subdivide memory check (2026-11-12,
  branch subdivide-crash):** a Sphere subdivided five times aborted the app
  (271 MB wireframe buffer over wgpu's 256 MiB limit); every viewport
  vertex list now goes up in pieces of at most 128 MiB (`gpu_chunks.rs`).
  Subdivide's fixed 8 million cap is gone (Jason): the polygon count is
  computed exactly (`subdivided_polygon_count`), a result over half of the
  free memory asks first, over all of it is refused. Memory per polygon
  (5 KB, `BYTES_PER_POLYGON`) is measured on one scene; provisional. Other
  tools (Bevel, Extrude, Spike) keep their 8 million guards.

- **Material editing (2026-11-12, branch material-editing):** distance
  rows (Absorption/Subsurface/Displacement Distance, Bump Amplitude,
  Rounded Edge Width, shader fog, Texture Locator, Backdrop Size) read
  units and arithmetic (`DragValue::distance`, UIC-52). Reusable value
  presets arrow (`value_presets.rs`, UIC-55; value-presets.json) on
  Refractive Index (23 built-ins) and Dispersion (5 physical B−G
  figures; a pick sets Dispersion 100 % and Abbe Number from the IOR,
  MAT-ED-Q01 decided by Jason 2026-11-12). Material layer
  Properties in six side tabs and folding sections
  (`material_layout.rs`, UIC-100..107); rows placed by judgement:
  Coat rows with Clearcoat, Luminosity and Viewport Display sections on
  Material Ref, Volume on Transparency, Clipping and Matte on Rays.
  Image Map Image list thumbnails (preset_thumbs cache) and Image Viewer
  on double-click of the field or an image layer row (a Loaded slot).
  Material preset drop on a shader tree group row replaces its contents
  (LoadPreset into the group). "Flourite" kept as given (Fluorite?).
  GUI check: docs/MATERIAL_EDITING_GUI_QA.md.

- **Material row greying (2026-11-12, branch material-greying):** the
  older Material rows grey in place, live, like the OpenPBR rows (UIC-92):
  coat, sheen, reflection, transparency, subsurface, smoothing, rounded
  edge, clip, matte and GL colour dependents, one table
  (`openpbr::GATES`) with each control above its rows (UIC-93, tested). A
  row stays live when a texture layer drives it or its controlling row
  (`openpbr::driven_rows`). Provisional: MG-Q01..Q04. GUI check:
  docs/OPENPBR_MATERIALX_GUI_QA.md 2c/2d.

- **OpenPBR and MaterialX, Stages A and B (2026-11-11, branch
  openpbr-materialx, spec docs/specs/openpbr-materialx):** every Material
  channel has an OpenPBR 1.1.2 identity (`openpbr.rs`: input table,
  channel identities, OpenPBR -> Cycles table, constant conversions both
  ways, tests holding both routes to Cycles together). New channels: Coat
  Color / IOR / Darkening / Anisotropy, Sheen Color / Roughness, Thin Film
  (weight, thickness µm, IOR), Abbe Number, Transmission Scatter / Scatter
  Anisotropy, Thin Walled, Subsurface Albedo / Uses Diffuse Color (native
  v41, written when a shader tree uses them). Cycles ABI 19: eight more Principled
  inputs, interior Volume Scatter, Thin Wall, principled glass for
  dispersion/thin wall/film. OPM-T1: 13 older scenes and starter presets
  render pixel-identical (`examples/opm_t1.rs`). Stage B: own `.mtlx`
  reader/writer (`materialx/`), bundled MaterialX/OpenPBR definitions
  (Apache-2.0); imports open_pbr_surface, standard_surface (published
  graph), gltf_pbr and UsdPreviewSurface into Material + Image Map layers,
  refuses anything needing Node Material; exports open_pbr_surface with
  image/mix/multiply nodes and a loss list; File > Import MaterialX… /
  Export Material as MaterialX…, Preset Browser lists .mtlx (MX-07),
  gear option writes .mtlx beside material presets, the preset converter
  prefers ambientCG .mtlx (MX-08). All 23 Test-Library ambientCG files and
  all 83 OpenPBR examples import. Provisional: OPM-Q07..Q15, MX-Q01..Q05.
  Command names ruled 2026-11-12 (branch mtlx-command-names):
  `operandi.materialx.import [layer:]` adds layers above the layer or at
  the top (no longer replaces a group's contents), `operandi.materialx.export
  [item:]`, and `preset.saveAs mtlx:true` (Operandi extra; the gear toggle
  sets it). GUI check:
  docs/OPENPBR_MATERIALX_GUI_QA.md. Stage C not built.

- **Packed mesh storage, stage 1 (2026-11-11, branch packed-mesh-storage,
  plan https://claude.ai/artifact/2fhQ3PHYUpZjZFfFZcKzCH):** `PackedMesh`
  (`packed_mesh.rs`): flat point, corner and tag arrays, each behind its own
  `Arc`; UV maps as a shared-value table (or per corner when smaller). The
  `MeshRead` trait reads both forms. Packing is bit-exact on every fixture
  and primitive (`tests/packed_mesh.rs`). Probe
  (`tests/packed_mesh_probe.rs`, opt-in, counting allocator): 298 → 81 B a
  quad (3.7x); a points-only undo copy 12x smaller; on 2M quads unpack
  ~160 ms, pack ~410 ms. Stage 2: `Item.mesh` is a `MeshSlot`
  (`mesh_slot.rs`): reads and writes work like a `Mesh` (Deref), `read()`
  looks without opening, a content stamp, and a counter of hidden opens.
  Nothing packs on its own yet (idle packer is stage 3); drag probe
  unchanged.
- **FBX, Alembic and PLY (2026-11-11, branch fbx-abc-ply):** File > Open,
  Import and drag-and-drop read .fbx (ufbx), .abc and .ply; File > Export ▸
  writes PLY (Binary), PLY (ASCII) and Alembic. scene.saveAs takes
  `format:` ply, plyAscii, abc and infers the format from a foreign file
  extension (registry thread's convention). Alembic is vendored C++
  (Alembic 1.8.12 + Imath 3.2.3, BSD-3) in `operandi-alembic`, built with
  `cc`, no CMake. Provisional: FBX import-only; first sample only, no
  animation; Alembic winding reversal and FBX units checked only against
  our own files and a hand-written ASCII FBX. GUI check:
  `docs/FILE_FORMATS.md` items 11–13.

- **Mesh Stack v1 (2026-11-11, branch meshops, spec meshops-2026-11-11
  Stage 0 trimmed):** a mesh item can carry a stack of ops over its Base
  Mesh (native v39); views, render and export show the result. Nine ops on
  the existing kernels: Cube, Sphere, Cylinder, Polygon Bevel, Smooth
  Shift, Subdivide, Triangulate, Mirror, Array, plus Select by Index
  (auto-captured from the Base Mesh selection). Each op runs guarded
  (panic, bad value, invalid or over-limit result = red row, input passed
  through); results are cached per op off the scene and evaluated on
  worker threads with stale ones dropped. Mesh Stack tab (list, eye, drag
  reorder, Add Operator, Properties, Select by Index buttons, row menu
  Freeze / Duplicate and Freeze / Delete, Base toggle), Edit Base Mesh with
  a ghost, direct tools refused on a stack otherwise, Freeze in one undo
  step, every edit one undo step, stress harness
  (`tests/mesh_stack_stress.rs`). Provisional: MOQ-01..10. GUI check:
  `docs/MESH_STACK_GUI_QA.md`.
- **Nodes N1: Node Material layer (2026-11-11, branch nodes-n1, spec
  nodes-n1-2026-11-11):** graphs are stored in the shader tree's Nodes
  section in the shared node graph JSON v1 shape (native v38, own
  format_version); unknown node types load, stay and are flagged.
  Add Layer > Node Material and > Import Node Material... (all or nothing,
  names the refused node). Properties: graph picker, one control per
  exposed input (layer value or the graph's shared default), Show Graph.
  Graph rows under Nodes edit shared defaults. Texture layers above drive
  exposed inputs from the Effect menu's Node Inputs. Blend is Normal only
  (others greyed). Read-only graph view in its own window, place kept.
  Cycles ABI v18: a general node list (Cycles type and socket names), the
  graph's surface mixed over the layers by coverage; layer-only scenes
  render as before. Presets and copy/paste carry the graph. Commands
  `operandi.nodeMaterial.import/showGraph/setDefault`. Provisional:
  NM-Q01, Q04..Q10; the viewport bake (NM-40) isn't built. GUI check:
  `docs/NODES_N1_GUI_QA.md`.

- **Library pack 2026.11.5 (2026-11-13, branch content-pack):** 13 mesh
  presets of our own (core `preset/starter_meshes.rs`: rounded cube, quad
  sphere, cylinder, cone, torus, capsule, tube, plane, table, stool, mug,
  stepped plinth, backdrop sweep), all quads, real sizes, resting on the
  ground, real-world UVs, default material tag, CC0 "Operandi
  contributors". `operandi-app --write-starter-meshes <folder>` writes them
  with thumbnails rendered on the preview stage; the pack's new Meshes/
  folder goes through the licence gate from their headers. Environments
  gain 15 indoor/studio CC0 2K HDRIs (13 Poly Haven, 2 ambientCG) in
  Indoor/ and Studio/. LP-Q05/Q06 provisional.
- **Library pack 2026.11.4 (2026-11-12, branch content-pack):** the pack
  now ships Matcaps/ (43 CC0: 18 Blender, 15 hecko, 10 Operandi renders;
  the 8 Blender matcaps the app builds in are left out) and Environments/
  (12 CC0 2K HDRIs: 8 Poly Haven .hdr, 4 ambientCG .exr, in Studio, Sky,
  Sunset, Night). Pictures have no licence header, so each source folder
  carries SOURCES.tsv (path, licence, URL, credit, date checked); the gate
  makes manifest items from it (kinds `matcap`, `environmentImage`) and
  refuses (exit 3) a picture without a CC0 line or a line without a
  picture. No mesh presets ship: there are none yet (PG-01 only makes the
  empty Meshes folder). Ignored test `library_pack_loads_from_its_own_folder`
  loads an unzipped pack through the browser.
- **Operandi Library pack (2026-11-11, branch library-pack, F1):** a
  downloadable "Operandi Library" folder (manifest.json, CREDITS.txt,
  LICENSES/CC0-1.0.txt, Starter/<Name>/ with preset, maps and thumbnail)
  built by `scripts/build-library-pack.sh` into
  `~/Developer/Operandi-Content/Operandi-Library-<version>.zip` (one
  zip; the 1024 px lite zip was dropped, it saved only 22 MB). The manifest comes from each preset's
  licence header; the build stops (exit 3) on a Starter item that isn't
  CC0 or has no source URL. Core `preset::pack` reads and writes the
  manifest and also takes an Extras/ (CC-BY) part, none shipped. Preset
  browser: packs in the default library folder
  (`~/Library/Application Support/Operandi/Library`; %APPDATA% and
  XDG data elsewhere; `OPERANDI_LIBRARY_FOLDER` overrides) become roots once,
  titled "<name> <version>"; entry tooltips show licence, credit and
  source; an older installed version says which is newer. With no pack
  installed the contents offer "Get the free Operandi Library…" (opens
  operandi3d.org/library in the web browser; no network code) and "Locate
  Library Folder…". Six built-in Basics (Plastic, Metal, Glass, Clay,
  Rubber, Emissive; GPL, constants only) are a read-only root after the
  user's roots, shown first on a fresh install. Commands (registry
  ruling): Operandi-only `operandi.pbview.libraryGet [url:]` only opens the
  page (not undoable); Locate… is a folder chooser that runs
  `pbview.basePathAdd`, which adds a pack (or a folder of packs) the same
  way however it's added.
  Provisional: LP-Q01..Q04. GUI check: `docs/LIBRARY_PACK_GUI_QA.md`.

- **Preset browser Stage 1 (2026-11-11, branch
  claude/project-thread-ljnnlt, spec preset-browser-2026-11-11):** Window ▸
  Preset Browser opens it in its own OS window (place and size remembered),
  F6 as a popover, and it's a registered panel kind for side panel tabs.
  Folder tree on the shared tree list with "(add path…)", root rename,
  remove, reorder, "not found" for missing drives; thumbnails or list,
  sorted by name, type, date or size; search below the shown folder.
  Content folders: presets by header, scenes, models, pictures, HDRs as
  environments, texture sets folded into one material (CD-20..23, real
  Poly Haven/ambientCG names checked). Loads: double-click, Enter, drag into
  a view (material onto geometry, model import, picture as Backdrop),
  `preset.do`; each one undo step. Saving: folder right-click Save … Here…,
  drag Item List/Shader Tree rows onto a folder, Replace/Keep Both on a
  clash. Thumbnails decode in the background with a memory cap and a disk
  cache; presets and texture sets render on the cutaway ball (PB-114) with
  Cycles. Commands from the registry thread (`pbview.*`, `file.*`,
  `layout.window`, `operandi.pbview.thumbnailSize`). Provisional:
  PB1-Q01..Q14. GUI check: `docs/PRESET_BROWSER_GUI_QA.md`.

- **Edit ▸ Center Selected (2026-11-11, branch center-selected-menu, spec
  center-selected-2026-11-11):** submenu between Snapping and the Lock
  group: All, X, Y, Z, XY, YZ, ZX, each running `vert.center` (the Edit
  tab's Center, CEN-01..04). Greyed in Item mode with Center's refusal
  (spec §5 suggestion; reference greying unknown, Q1). The reference's
  neighbours (Reset Tool Attributes, Center/Pivot to Bounding Box,
  Symmetry) aren't built, so the group sits after Snapping.

- **Snaps and Precision panel (2026-11-10, branch snaps-precision, spec
  snaps-precision-2026-11-10 v1.2):** Create > Measure > Snaps and
  Precision and Edit ▸ Snapping ▸ Snaps and Precision toggle the "Snaps"
  palette: Presets (Drag Snap, Drag Snap Rigid, Planar Rotate Snap, Planar
  Rotate Snap Axis, Scale Absolute, Size Absolute, Advanced Handles) and
  Commands (Absolute Scaling). A preset turns snapping on (left on),
  starts its tool, and its button again drops it; leaving it restores the
  action centre. Drag Snap/Rigid = Element Move, Auto, Range 0, Use
  Connectivity / Rigid. The other four wait for a click on any element
  (Element centre, axes on its normal, new `AxisOrientation::Element`).
  Scale/Size Absolute: per-axis base circle and scale square, both
  snapping; Size shows Absolute/Relative fields, Scale shows Absolute
  Size. Planar Rotate: rest and rotate handles about the element normal;
  the Axis variant adds start/end handles that set the axis. Advanced
  Handles sets Tool Handles to Advanced (then plain Rotate shows the
  planar handles too); `tool.handleStyle`. All guesses are SPQ-01..08.
  GUI check: `docs/SNAPS_PRECISION_GUI_QA.md`.

- **Text fields keep their keys (2026-11-10, branch text-field-keys,
  Jason's request; UIC-51):** while any text or number field has focus,
  every app shortcut is off (M, Tab for Sub-D, digits, pie keys), whether or
  not the field put itself on the app's list; any focused egui text edit
  counts. Tab / Shift+Tab now move between text fields as well as number
  fields of the same panel or form (forms in dialogs and panels, item and
  layer names, tags, channels), skipping buttons and checkboxes; a test
  fails on any `TextEdit` that doesn't join. Provisional: TFK-Q01..Q02.
  GUI check: `docs/TEXT_FIELD_KEYS_GUI_QA.md`. Follow-up (branch
  tab-subd-text-field): the same in panels and palettes in their own OS
  windows (Absolute, Snaps, Work Plane...), which get no raw-input hook;
  `numeric_tab::take_tab_in_window` runs at the start of each such pass.

- **.opscene file type (2026-11-10, branch claude/project-thread-1j4090):**
  macOS: the app bundle exports `org.operandi3d.opscene` and owns it, with
  a document icon (the O on a dark page, OP-DOC-1). Double-click in Finder,
  Open With and drops on the Dock/app icon open the scene, whether or not
  the app is running (Apple Event handler in `open_documents.rs`, installed
  at will-finish-launching). Several files open one per frame through the
  File > Open path, waiting while a dialog is open. A file path on the
  command line opens too. Windows/Linux registration is written up in
  `docs/FILE_ASSOCIATION.md`, not built. GUI check:
  `docs/OPSCENE_FILE_TYPE_GUI_QA.md`.

- **Absolute Scaling (2026-11-10, branch claude/project-thread-ilbzoc,
  spec snaps-precision-2026-11-10 v1.1 part A):** Create > Measure >
  Absolute Scaling (label button, toggles the "Absolute" window). Grab Size,
  Uniform Scale, Explicit Scale, Reference Scale; Uniform, Uniform Axis,
  Orient To Selection, X/Y/Z, X/Y/Z Center. One undo step per press; fields
  refresh after every press, kept for the session only. Script:
  `scale.absolute mode:… [absolute_size_* …]`. Flat axes are left alone and
  named in the status line (Q2, Jason). 31 of the 38 cases match the
  reference; the 6 flat cases and AS-18 have Operandi answers (Q2, ABS-Q04).
  Provisional: ABS-Q01..Q09. Not built: Scale Absolute / Size Absolute
  presets (parts B, C), the popover variant. GUI check:
  `docs/ABSOLUTE_SCALING_GUI_QA.md`.

- **Pen tablet pressure (2026-11-09, branch pen-tablet):** stage 1: macOS
  pen pressure, tilt, rotation and pen end read from AppKit in the existing
  `sendEvent:` observer (`tablet.rs`; winit drops them); each pointer event
  gets its own sample, dabs blend pressure between events. Preferences >
  Input > Pen Tablet: Pressure Curve (Soft/Linear/Firm), Eraser End
  (Invert/Smooth), Pressure Sets Brush Size (off). `OPERANDI_PEN_DEBUG=1`
  draws a live readout. Stage 2: pressure scales sculpt strength (SC-45),
  optionally size and the ring; eraser = Ctrl or Shift; Smooth Mode Low
  Pressure un-greys once a pen is seen and works (SC-62). Mouse and
  trackpad stay full pressure. Windows/Linux: mouse only for now.
  Provisional: PTQ-01..07. GUI check: `docs/PEN_TABLET_GUI_QA.md`.

- **Viewport shading (2026-11-09, branch viewport-shading, spec v1.1):**
  VS-1..VS-3. The active meshes' plain wire is black over every filled
  style (new theme colour Shaded Wireframe; Wireframe and Vertex Map keep
  theirs). Every style is fixed to the camera, in perspective and every
  orthographic view: Shaded, Texture, Default, Vertex Map and Advanced are
  lit by one light straight above the line of sight, 45° up; scene lights
  only with Show Scene Lighting (Advanced). Measured on the GPU, default
  grey sphere: Solid #999999; Shaded peak 213 / far side 29; Texture
  161 / 22; Default 215 / 98; Advanced 90 far side and a highlight of 251
  at 0.38 radius above centre (ambient default now 44 %). Gooch: view light
  (1,1,1), #1F1F78→#CCCC5C by |N·L|, no highlight or rim. Cel: view light,
  real eye, so the outline follows the silhouette; a cube's front face is
  #7A7A7A. Matcap (was Reflection): a Blender matcap (CC0) by view-space
  normal. Removed theme colours: Gooch Outline and the five Reflection
  colours. Selected, locked and rollover polygons draw their highlight on
  top of every style (VS-Q07). Reflection's matcap is a per-view choice
  (8 Blender matcaps or any PNG/EXR; `operandi.view3d.matcap`, VS-Q08). The
  Reflection style is named Matcap (VS-N01, Jason 2026-11-10; Reflection
  still works in scripts).
  Provisional: VS-Q01..Q05, VS-Q07, VS-Q08.

- **Item-mode wire (2026-11-11, branch item-mode-wire, spec v1.2 VS-6):**
  A new view's overlay is Colored (black; the theme's Dark Lines Overlay
  default is now black, a group's Wireframe Color still wins) and Show
  Wireframe in Item Mode starts on. Off hides every mesh's plain wire in
  Item mode whatever the overlay, on the CPU drawing path too; the
  selection highlight still draws; component modes unaffected. Saved
  views keep their stored values. Tests VS-T13..T16 (gpu.rs).

- **Bevel live on activation (2026-11-09, branch bevel-live):** Polygon,
  Edge and Vertex Bevel act on the first panel edit, no viewport click
  (TA-01); Polygon Bevel's panel edits keep adjusting the one bevel (TA-02).
  Switching on or dropping with no edit changes nothing (TA-03). Edge Bevel
  with nothing selected bevels every edge (TA-06). Extrude, Smooth Shift,
  Thicken and Edge Chamfer still wait for a click (TA-04). A second handle
  drag adjusts the same bevel (Jason's check; `BEVEL_REDRAG_STACKS` flips it
  back); Shift+click or Shift+drag keeps the bevel and starts a fresh one.
  Undo inside the tool takes back one edit at a time; after Drop each bevel
  is one undo step (TA-07, provisional). Polygon Bevel with nothing selected
  bevels every polygon (PB-O-01, kept by v1.2). Provisional: TA-Q01, TA-Q03. Not built: Extrude's Apply
  button (TA-05). GUI check: `docs/BEVEL_LIVE_GUI_QA.md`.

- **Slider min/max labels (2026-11-09, branch ui-hints-minmax):** a UI
  preference, off by default (`pref.value ui.slider_min_max_labels`,
  Preferences > Defaults > UI > Slider Min/Max Labels). When on, every
  number field with both limits finite gets a thin range bar along its
  bottom edge and its min and max, with units, in small grey text under its
  ends; those rows are 8 px taller. Off leaves every row exactly as before.
  One shared module (`form/range_labels.rs`) used by the form number field
  (dialogs, panels, `rows::number_d`), every `numeric_tab::DragValue` with a
  set range, and Tool Properties whole-number fields. An approved change to
  UIC-90 (Jason, option A). Provisional: limits at ±1e6 or beyond count as
  open (MINMAX-Q01); value boxes inside egui grids may overlap the next grid
  row (MINMAX-Q02). GUI check: `docs/SLIDER_MIN_MAX_GUI_QA.md`.

- **Original-compatible switch (2026-11-09, branch ui-hints-original):**
  one UI preference, off by default (`pref.value ui.original_compatible`,
  Preferences > Defaults > UI, which lists what it changes). A registry
  (`original_compatible.rs`) holds each deliberate difference (id + one
  line); code asks `is_original(id)`. Registered: A4 empty backdrops show a
  dotted square; A5 in Perspective, backdrop pictures cover meshes (grid in
  front still over them; depth-tested grid and a depth-claim pass); A6 Set
  Part starts with the Default Part name (provisional). A2 needed nothing:
  colour fields already open the picker on double-click (339604f4). A1/A7
  wait on checks; A3 dropped. GUI check: `docs/ORIGINAL_COMPATIBLE_GUI_QA.md`.
  2026-11-11 (branch original-compat): A1 handles keep a world size
  (`Camera::handle_zoom`, 1 at the default view; provisional: viewport
  height doesn't change it, no `-`/`=` handle keys exist yet); A4's square
  is a fixed 1 m whatever the Pixel Size; A6 now opens with the last name
  Set Part applied this run (else Default Part); A7 on a Mac only Cmd+click
  toggles tree-list rows, Ctrl+click opens the row menu.

- **Detriangulate preview (2026-11-09, branch ui-hints-detri):** while the
  Detriangulate... dialog is open, the viewport shows in red the edges OK
  would remove, updating as the options change; OK or Cancel clears it.
  The preview comes from the same pairing code as the edit
  (`detriangulate::joined_pairs`, `Editor::detriangulate_preview`), and a
  test checks it equals the edges the edit removes. It is worked out again
  only when the options, active item or scene revision change.
  Provisional: drawn over the mesh, hidden edges included. GUI check:
  `docs/DETRIANGULATE_PREVIEW_GUI_QA.md`.

- **Big scenes save (2026-11-09, branch big-scene-save):** scenes past
  64 MiB save and reopen. The fixed 64 MiB cap on writing and reading is
  gone; big mesh arrays are packed (native version 34, `mesh_pack.rs`,
  see FILE_FORMATS.md); Save over an existing file reads only its header;
  opening skips a slow per-edge map. A failed Save or Open shows a dialog
  with the reason; the title keeps "*" and the scene its name and path.
  5M polygons / 50 items: 794 MB, save 1.1 s, load 1.0 s (release).
  The recovery journal now writes big scenes too, at most every 15 s for
  every scene (was 1 s; Jason, 2026-11-09).
  A failed auto-save or recovery copy shows a dialog once, then only the
  status line until one succeeds; a failed recovery copy is retried. GUI check: `docs/BIG_SCENE_SAVE_GUI_QA.md`.

- **UI hints stage 1 (2026-11-09, branch ui-hints-1):** greyed controls
  explain themselves: hovering any greyed button, menu row, pie slot or
  field says why and what to do. One shared helper (`unavailable.rs`,
  `ui.gated` / `ui.gated_ui`) takes the reason; a source check fails on
  raw egui enable calls or `enabled_if(.., "")`. Greyed scopes now really
  show their reason (egui gave the hover to the greyed child). Form rows
  name their controlling row; shortcut actions' reasons come from
  `input::unavailable_reason` (derived from `input::available`); native
  menu items get the reason as their tooltip. Active-tool exit hint: the
  viewport's bottom-left line names the user's own Drop keys (Space, the
  Drop binding, Cube's Escape) and the way to cancel
  (`tool_exit_hint.rs`); it follows rebinds at once. Provisional: Cancel
  has no input-map entry, so the hint points at Tool Properties' Cancel.
  GUI check: `docs/UI_HINTS_GUI_QA.md`.

- **Viewport image textures (2026-11-09, branch viewport-image-textures):**
  image layers draw per pixel in the realtime viewport instead of as their
  average colour. The folded `shader_eval::Expr` is written out as WGSL
  (`operandi-core/src/materials/shader_wgsl.rs`) and run in the fill shader, so the
  viewport uses the same mapping, tiling, blend modes, opacity, alpha and
  falloff as Cycles (UV maps with the render's map rule, every Texture
  Locator projection, Repeat / Mirror / Edge / Reset). Numbers ride in a
  parameter buffer, so moving a locator or scrubbing opacity recompiles
  nothing; pipelines are cached by generated source; each picture is read
  and uploaded once (sRGB, mipmaps, ≤ 4096 px), Reload Image re-reads it.
  Nearest/linear filtering; Bicubic draws bilinear (VIT-Q01). Geometry
  gradient inputs, vertex-map attributes and occlusion read as in
  `eval_at` (0 / 0 / 1, VIT-Q02); one procedural pattern per surface
  rides in per corner, two or more fall back to the average colour
  (VIT-Q03). GL Display off, selected and locked polygons keep today's
  colours. Reload Image runs through the new `image.reload` command and
  `image.list` reports the images (UIC-160..164, VIT-Q05). Fixture
  `fixtures/viewport-image-textures/`. GUI check:
  `development/viewport-image-textures/QA.md`.

- **Shader layer right-click menu (2026-11-09, branch shader-layer-menu,
  spec shader layer menu v0.1):** the layer menu in the spec's order with
  entries that don't apply left out (UIC-07). Stage 1: Duplicate (directly
  above, "(n)", own locator), Copy / Paste / Paste Values on Operandi's own
  clipboard, Editor Color (note 10's colours), Properties, Select
  Polygons, Reload / Replace Image… / Open Image Folder. Stage 2: Change
  Type keeps the Layer block, Effect, masks and locator. Stage 3: instances
  linked per setting and per colour component, italic rows and overridden
  labels, Revert to Source, source delete takes its instances (asks).
  Stage 4: Library items, references resolved at render time
  (`Scene::shading_tree`), De-Reference, Lock / Unlock Preset. Native
  version 32. Provisional: SLM-Q01..Q13. GUI check:
  `docs/SHADER_LAYER_MENU_GUI_QA.md`.

- **MCP render returns its picture (2026-11-09, branch mcp-render-image):**
  `render` answers with the finished picture as an image, scaled to
  `max_size` (default 1024 px; `include_image: false` leaves it out), and
  `render_status` returns the last render's picture with
  `include_image: true`. The saved file keeps full size. A picture that
  can't be read is reported in `image_error`.
- **MCP bridge gaps from the hero renders (2026-11-09, branch
  mcp-bridge-gaps):** new bridge tools `shader_tree_get`, `material_get` /
  `material_set` (every Material channel by its panel name, one undo step),
  `shader_layer_add` / `shader_layer_set` (Gradient input and keys, Vertex
  Map, Constant, Image Map by PNG/JPEG path, procedurals, groups, any
  layer's channels) and `subdivision_set` (Tab / Shift+Tab and per-mesh
  levels; `item_get` reports them). All go through the panels' own edits.
  `scene_load` of the already-open file now reads it again instead of
  silently keeping the copy in memory. `light_set render` text says
  Default/Yes/No and also takes On/Off; torus `height` / `cross_section`
  documented as half sizes. Refusals and `scene_info.dialog` name the open
  dialog. Provisional: MCPH-Q01..Q03 (Q01 is the grouped cap inset, left
  refused per M17-Q04).
- **Sculpt foundation + 4 pilot brushes (2026-11-08, branch
  sculpt-foundation, spec SC-01..123):** new Sculpt toolbox tab with
  Smooth, Push, Inflate and Move (label buttons until icons are drawn),
  Smooth/Hard tip row, Lock Borders / Lock Corners. Core machine in
  `operandi-core/src/sculpting/sculpt.rs`: tips with six shapes and Bias, Hard tip
  with Soft Border, dabs at Interpolation Step spacing (Continuous Stroke),
  Additive/Attenuate/Max Stroke density, all Normal Modes, Ctrl invert,
  Shift (or Smooth Mode Uniform) smoothing, Ctrl-Smooth volume-keeping,
  Symmetry-mirrored dabs and Move grabs, locked/hidden points never move,
  active falloff multiplies the weight. Strokes use their own spatial grids
  (points and polygons) and write only changed points each frame, so the
  viewport patches instead of rebuilding (speed guard covers it). One
  stroke = one undo step "<Brush> Stroke" via `Command::SculptStroke`.
  Cursor ring + normal line + mirrored ring; right-drag size, middle-drag
  strength, plain right-click still opens the menu. Tool Properties from
  the declared-field form (Sculpt: <brush>, Stroke, Tip sections). Brush
  settings in `sculpt.json` (prefs, no file-format change). Provisional:
  SCQ-01..SCQ-10. GUI check: `docs/SCULPT_FOUNDATION_GUI_QA.md`.
- **Duplicate tools v1 (2026-11-08, branch duplicate-tools; spec
  `docs/specs/duplicate-tools/`):** a Duplicate tab (DS-60) and Geometry >
  Duplicate with Radial Sweep (was Lathe), Clone, Array, Radial Array and Mirror, built as
  placers (plane, line, grid, circle) plus a Copies or Sweep copier
  (`duplicate.rs`, `duplicate_sweep.rs`). Arming changes nothing; a click
  in the view or Apply makes the copies, values and handles update them
  live, Drop is one step. Copies carry every polygon attribute, mirrored
  copies are re-wound so they face out, Merge Vertices welds new points
  only (Mirror onto the plane), and two polygons left on the same points
  (an inside wall) are removed. Item mode makes new items named like
  Duplicate's (Mirror gives them positive scale). Radial Sweep sweeps lines,
  curves, face outlines or edges, closes a full turn with shared axis
  poles, caps part turns, faces out and writes Texture UVs. Handles: the
  mirror plane (slide, hinge, Free Rotation turn), the circle (axis, arc
  angles, tilt), Clone's arrow, Array's axis grips; right-drag sets counts.
  Detriangulate... (Polygon tab, Geometry > Polygon) joins triangle pairs
  best first with Flatness, Max Angle and Keep boundaries, one step.
  Follows spec v1.1 (probed starting values; Mirror Angle 0 = plain
  mirror, turned about Left). Drag frames 5–11 ms on the
  16k grid. Provisional: DUP-Q01..Q08. No icons yet (label rows). GUI
  check: `docs/DUPLICATE_GUI_QA.md`.

- **Multiple active meshes, stage 2: transforms (2026-11-08, branch
  multi-active-transforms):** Move, Rotate, Scale and Transform (and the
  Deform presets built on them) move every active mesh's selected
  components together about one handle at the centre of the combined
  world-space selection (all centre and axis modes read every mesh),
  falloffs in world space, Symmetry per mesh, one undo step. In Item mode
  they move every selected item about the items' shared centre, skipping
  locked items and children of selected parents. One mesh or item works as
  before. Provisional: MAM-Q08..Q11. GUI check:
  `docs/MULTI_ACTIVE_TRANSFORMS_GUI_QA.md`.
- **Group Locators (2026-11-08, branch group-locators, provisional
  design):** new item kind `Item.locator` (locator.rs), native version 30.
  Add Item > Locators > Group Locator; Item Operations and the Items row
  menu's Parent to Group Locator (one undo step; shared parent keeps local
  transforms, mixed parents keep world placement, shear refuses). Item-mode
  Move/Rotate/Scale carry children; component tools refuse it. Viewport:
  corner-bracket cube + centre cross at constant screen size, per-view
  Display > Show Locators (on). Locators tick in Filter/Select popovers;
  "Group Locator" Properties tab. OBJ/glTF/Cycles bake its transform into
  children; LXO leaves it out (children re-parented in place). Items-row
  icon is a little blue folder. Delete of any item with children asks
  whether to delete them too (Yes: whole branch; No: direct children to
  the root in place, keeping their own hierarchy; one undo step;
  GL-Q4 decided by Jason). Open: GL-Q1..Q3, Q5..Q7. GUI check:
  `docs/GROUP_LOCATORS_GUI_QA.md`.
- **Multiple active meshes, stage 1 (2026-11-08, branch
  multi-active-meshes, on top of active-mesh-pick):** every selected mesh
  is active in component modes: all draw points, edges and selected
  polygons, and click, Shift/Ctrl-click, paint and lasso select on any of
  them; the primary stays as chosen in the Items list. Non-primary meshes
  keep their own components (`Selection::others`, native version 29), per
  mode, through undo/redo and save. Select menu commands run on every
  active mesh; tools still edit the primary only. Provisional:
  MAM-Q01..Q06. GUI check: `docs/MULTI_ACTIVE_MESHES_GUI_QA.md`.
- **Tree clicks keep the transform tool (2026-11-08, branch
  tree-keeps-tool, Jason):** with Move, Rotate, Scale or Transform live,
  clicking, Ctrl-/Shift-clicking or group-selecting in the Items tree drops
  the open drag as its own undo step and restarts the same tool on the new
  selection (handle there, zero offset, Negative Scale and Tweak options
  kept). Deselecting or a selection the tool can't start on leaves the tool
  dropped. Other tools still refuse tree clicks. Item mode still moves only
  the active item of a multi-selection.
- **Items reorder by drag (2026-11-08, branch item-tree-reorder, UIC-143):**
  rows dragged between rows now reorder (top/bottom quarter of a row =
  before/after it, an open parent's bottom quarter = first child, middle =
  child, below the list = last top-level). Insert line indented to the
  landing level in the new `items.drop_indicator` colour; drops into a
  dragged row's own branch show nothing. Multi-selection keeps list order.
  New core command `MoveItems` (one undo step: Reorder / Parent /
  Unparent, world position kept); order saves as is. In the shared tree
  list, so Shader Tree gets the same zones. Provisional: TREE-Q02 (a)–(e).
- **Plane handles v1 (2026-11-08, branch plane-handles-v1):** circles lie
  flat in their world plane at the outer corner of their two arrows, about
  1/6 of an arrow across, thin axis-coloured rim and faint dark fill; hover
  lights only the rim. World-aligned under any action axis; Ctrl does
  nothing on a circle. Scale's center is now a larger circle. On Transform
  a press on a circle wins over the rotation ring through it.
- **Panel mode everywhere (2026-11-08, branch panel-mode, UIC-100..124):**
  `form::rows` draws panel-mode rows one at a time for panels that bind
  their own values; `option_row` forwards to it, so every Tool
  Properties, Properties, Preferences and floating-window row is label |
  status circle | field with one label edge per tab, 20 px rows, 2 px gaps.
  Every value box has the mini-slider arrows (UIC-53); section bars fold
  and remember (UIC-105..107); the Name row heads every item tab
  (UIC-104); Preferences pages, Snapping, Preview Settings, Work Plane
  Settings and Smooth Shift Mini Properties use sections, status lights,
  tooltips, number fields instead of sliders and dropdowns instead of
  radio groups. Unbuilt rows are left out except Edge Bevel's (FORM-Q08).
  The status light (UIC-112) shows None or Constant only until
  animation/links exist; no click action yet (FORM-Q09 closed).
  Provisional: FORM-Q10..13. Tab strips mark the active tab by its accent name
  (UIC-100). Multi-selection shows "(mixed)" on Item Transform, camera,
  light and shared-tab rows; Item Transform edits every selected item as
  one undo step, arrows moving each from its own value (UIC-56/57/118).
  GUI check: `docs/PANEL_MODE_GUI_QA.md`.
- **Polygon Axis Slice measured (2026-11-08, branch axis-slice-measured,
  spec v1):** Numbers start at 0 (activating cuts nothing), Size at 1 m;
  Number cuts are L/(N+1) apart with a wrap-around Offset, Size cuts run
  from the box's start plus Offset. Number X/Y/Z (and every other
  whole-number field) now step by 1 on a click of their arrows or a drag
  (Jason's report, UIC-53). 17 oracle cases pass as a test.
- **Wave 2 measured cut rules (2026-11-08, branch path-slice-measured,
  spec v1.1):** the path's ribbon also cuts a box's side faces, step points of a
  straight curve stay on the cut, Tunnel is built (holes, no walls), Core
  and Tunnel drop unused points, shapes use their own frame (Half Size X along
  world Z on Axis Y) with Heading, Pitch, then Bank. 21 oracle cases pass as a test, with the deliberate
  differences listed in SPECIFICATION_QUESTIONS (no slits, two-piece
  keyholes, no T-junctions, W2-PS-Q14/Q15).
- **Shared tree list and panel groups (2026-11-08, branch
  claude/project-thread-wcdyn2, UI bible section 10):** one tree-list
  component (`tree_list.rs`) now draws Items, Shader Tree and Groups: tool
  row with the funnel last, icon-headed state columns, Name, value columns,
  full-height column lines, dotted connectors, 17 px rows, selection in
  accent (primary lighter), bold "*"-marked current scene, dimmed hidden
  rows. Click / real-Ctrl+click / Shift+click ranges, double-click selects
  children, click-pause-click renames, eye drag-sweeps (one undo step),
  Alt+click solo, drag on/between rows, search row (text or `*`/`?`, Match
  Case, Flat), draggable column widths, sideways scroll. Panel groups read a
  panel-type registry (`panel_registry.rs`): "+" menu by category, cut-off
  tab names with a ▾ overflow list, drag to reorder, Close Tab, tabs saved
  with the layout. Mesh Stack waits for mesh operators (TREE-Q04). New core
  commands: `SelectItems`, `SetItemsVisible`. GUI check:
  `docs/TREE_LIST_GUI_QA.md`.
- **Viewport Properties header (2026-11-08, same branch, UIC-103/111):**
  One Tab ▾ / All Tabs ▾ are preset menus (new, rename, delete; find; Load
  and Save Default for 3D View, which new views start from); O over the
  popover switches one tab / all tabs. Labels hug their fields: the label
  column is the longest label's width (≤ 45%, then "…").

- **Shared form component (2026-11-08, branch dialog-component,
  UIC-80..99, UIC-100..125):** `form/` takes a list of rows (label, kind,
  default, unit, enabled-when) and does all layout, keys, greying and
  sizing. Dialog mode: small modal titled with the command, OK/Cancel
  column at the right with Ret/Esc, Return commits a typed field before it
  means OK, last OK'd values remembered per command in
  `dialog-values.json`. Panel mode: tab strip, foldable sections (folds
  remembered), status circles. Every options dialog and confirmation now
  uses it, including Subdivide, UnSubdivide and Freeze (their interim
  `dialog_form.rs` is gone, SUBT-Q14); Preferences > Mesh Items is the
  panel-mode proof. Provisional
  and exceptions in `docs/SPECIFICATION_QUESTIONS.md` (FORM-Q01..Q06). GUI
  check: `docs/DIALOG_COMPONENT_GUI_QA.md`.
- **Edit tab waves 2 and 3 (2026-11-08, branch path-slice; specs
  `docs/specs/edit-wave2/`, `edit-wave3/`):** one shared Slice cutter
  (`path_slice.rs`) projects a path along X/Y/Z/Custom/Action Axis onto the
  mesh and cuts it in with the slice engine (Slice, Stencil, Core; Front
  Face Only; Select Sliced Edges). **Pen Slice**: click points on the work
  plane, Closed/Open, World Axis/Straight Line/Right Angle snaps. **Curve
  Slice** (draw mode): clicked points make a curve sampled into Steps (or
  Auto by angle). **Primitive Slice**: click a face and drag to size a
  Rectangle (rounded corners), Circle or Ellipse lying on it; Edit From
  Center/Ctrl, Keep Aspect, Heading/Pitch/Bank, Flip, Clones, Lazy Apply.
  **Polygon Axis Slice**: Number or Size cuts along X/Y/Z over the
  selection's box, Offset (drag in the view), Hold Slice. Greyed: Tunnel and
  the boolean types, Preset profiles, rotate handles, Exponent, Dicer and
  Julienne. Edit tab rows are label-only until icons are approved. Also
  fixes typed Slice tool fields (Tier 1A's were confirmed but not applied).
  ~4.5 ms per drag frame on the subdivided 16k grid. Provisional readings:
  SPECIFICATION_QUESTIONS W2-PS-Q1..Q13, AX-Q1/Q2. GUI checks:
  `docs/PATH_SLICE_GUI_QA.md`.
- **Edit tab wave 1: Subdivide group, Center, Slide, Freeze... (2026-11-08,
  branch subdivide-tools, spec docs/specs/edit-wave1/SUBDIVIDE.md):** one
  command, Subdivide Polygons (`poly.subdivide`), with four methods:
  Catmull-Clark (the SDS Subdivide button, exact SUB-10 step), SDS Subdivide
  (angle creases), Faceted, Smooth; Fractal, Max Smoothing Angle, Linear
  UVs, Boundary Rules and Ignore Edge Weights work, Background Constraint is
  greyed. Partial selections follow SUB-14, the new polygons are selected
  afterwards, Alt+click SDS Subdivide runs twice as one undo step, settings
  are remembered across restarts (`subdivide.json`). D = SDS Subdivide
  outside Edge mode (in Item mode on every selected mesh item), Shift+D
  the Options dialog in component modes and Instance in Item mode (a
  status message: instances aren't built). UnSubdivide... opens its
  dialog first (corrected SUB-40/40a); UnSubdivide
  (`poly.unsubdiv`) undoes Catmull-Clark steps exactly where they can be
  undone; on a partial selection the merged polygons keep the outline
  points their unselected neighbours use (n-gons, SUB-40b). Center X/Y/Z/All (Alt:
  XY/YZ/ZX, `vert.center`) uses the world bounding box. Slide at the top of
  the tab starts Edge or Vertex Slide; Freeze... opens a dialog over the
  existing Freeze (which no longer fails when another mode's selection is
  remembered). New icons for the six buttons await Jason's approval.
  Oracle subset (docs/specs/edit-wave1/cases-subset.json): 9 of 10 match;
  cube_flat_unsub (UnSubdivide of a Faceted cube) is open, SUBT-Q04.
  Freeze... has the ten-field dialog of corrected SUB-50 (only Tessellation
  acts). The three dialogs share an interim layout (`dialog_form.rs`,
  SUBT-Q14). Provisional: SUBT-Q01..Q14. GUI check: `docs/SUBDIVIDE_TOOLS_GUI_QA.md`.
- **Main window place (2026-11-07, branch window-state, UIC-73 [OP]):** the
  main window's size, place and maximised/fullscreen state are saved on exit
  to `main-window.json` in the data folder and restored on launch; the
  normal size/place survives a maximised or fullscreen quit. A place whose
  title bar is on no screen (monitor unplugged) opens at the system default,
  and the size shrinks to fit the screen. Screens are listed on macOS and
  Windows; Linux trusts the saved place (provisional). GUI check:
  `docs/WINDOW_STATE_GUI_QA.md`.
- **Tab between numeric fields (2026-11-07, branch tab-numeric-fields,
  UIC-51):** while typing in any numeric field, Tab accepts the value and
  opens the next numeric field of the same panel with its value selected;
  Shift+Tab goes back; both wrap. Buttons, checkboxes and text fields are
  skipped, disabled fields too. Covers Tool Properties fields and every
  other value box (`numeric_tab::DragValue`; a test keeps plain
  `egui::DragValue` out). A refused value keeps focus with its error. "Same
  panel" = same window/popover layer and clip area (provisional).
- **Plane handles (2026-11-07, branch plane-handles, PH v0):** small
  circles between two arrows on Move, Scale and Transform's move part,
  outlined in the colour of the axis they don't move along. Move circles
  move in their plane (Ctrl: first-axis lock), Scale circles scale their two
  axes equally. Hidden when the plane is nearly edge-on; picked after the
  center and the arrows. Placement, fill, pick order and axes are
  provisional constants in `viewport/plane_handles.rs` (PH-07/08/11/21).
- **Ctrl extras (2026-11-07, branch ctrl-extras):** Shift-drag with
  Move/Rotate/Scale/Transform continues the last drag (HM-02; plain drags
  start fresh). Middle-drag repeats the last drag's amounts and goes on from
  there, middle-click repeats once (HM-04; a preset that navigates with
  middle wins). Ctrl+Shift on a handle in Vertex/Edge/Polygon mode copies
  the whole selected polygons and drags the copy (HM-06); the copy and the
  drag are two undo steps, like the item duplicate.
- **Ctrl constrain (2026-11-07, branch ctrl-constrain, HM v1):** Ctrl is
  read at the press and kept for the whole drag (HM-01/12a). Ctrl haul with
  Move/Transform, Ctrl on the centre square, and Ctrl Tweak lock to the
  first work-plane axis after 4 px (HM-10..12, HM-40, HM-61/71). Rotate
  snaps to Rotation Snap Angle on rings and hauls (HM-20/21/23). Ctrl on a
  Scale axis handle, and Ctrl haul Scale, scale uniformly (HM-30, HM-32
  provisional). Ctrl away from a drawn primitive moves it whole (HM-52).
  Move has no plane squares yet, so HM-13's plane-square lock isn't built
  (HM-Q01).
- **Primitives on a moved mesh (2026-11-07, branch primitives-moved-mesh):**
  drawn, numeric and unit primitives (Cube, Sphere, Cylinder, Cone,
  Ellipsoid, Torus, Capsule, Arc, Tube, Text) and Pen now work on a moved,
  rotated, scaled or parented mesh: built in world space, mapped into the
  item's local space (`Scene::world_to_local`), item transform kept, one
  undo step. Only a transform with no inverse is refused. Unit placement is
  provisional (PRIM-MOVED-Q01).
- **Command names (2026-11-07, branch command-names):** command, tool and
  argument names follow the name map; old names are deprecated aliases in
  `operandi-core/src/commands/command_names.rs`, resolved at `execute` and in the MCP
  bridge, logged once per run. `*.Slide` picks by mode (provisional: no
  Polygon/Item Slide). Script percents are 0–1 and angles radians (map
  units, Jason 2026-11-07); panels keep % and degrees. Details:
  docs/COMMAND_NAMES.md.

**Planning list (2026-11-05):** `specifications/tool-matrix-2026-11-05/`
lists every Model toolbox button and the spec that covers it. It replaces
the legacy milestone matrix as the list new work is planned from.

## Current status — checked 2026-11-01

This summary was checked against the code and tests on 2026-11-01. The dated
entries below are history and may be stale; where they disagree, this wins.

**Built and reachable:** all 73 M1–M3 tools and commands; Cube, Sphere,
Cylinder, Cone, Ellipsoid with basic UVs; M1.6 workspace layout
(PAR-004..010). Full suite: 1,206 tests pass.

**Light type conversion (branch light-type-convert, 2026-11-07):** a
light's Type switch (panel dropdown, MCP `light_set type`) converts
Intensity to keep the irradiance 1 m in front of the light (LT-Q01);
Mesh and Portal keep the number.

**MCP still-life gaps (branch mcp-still-life-gaps, 2026-11-07):** the
bridge gained `viewport_capture` (PNG of a view, current / named / through
a camera, returned as an image), `view_set` (look through a camera),
`render` / `render_status` (Cycles to a file at a chosen size and samples,
answered when done or at a timeout), `item_get` / `camera_set` /
`light_set` (units listed), `command_help`, `scene_revert` and
`smoothing.set`. Fixed: `select item` in component modes, unknown
`command_run` keys now refused, `primitive_add` schema (shape listed and
required, any case, cube/sphere/cylinder added, cone radius), `scene_save`
leaving the scene dirty, selection echo listing other modes' leftovers,
`mesh.new name`. Radians and 0–1 throughout. Follow-ups: `scene_info` document
fields, `render_settings_get/set` (scene samples, denoise, adaptive threshold,
native v27; exposure, white level, ISO and gamma on the Final Color output), light Type dropdown (keeps what still applies). Provisional:
MCP-Q02 (intensity conversion), MCP-Q04, MCP-Q05.

**Camera properties (branch camera-properties, 2026-11-07):** camera
Properties tabs Camera View, Camera Effects and (via Display > Show Camera
Overlay) Camera Overlay. Built: Reset (Position/Rotation/Scale/All), Sync
to View, Set Target from the selection + Set Focus Distance, all six
Projection Types (panoramas in Cycles only), lens / F-Stop / Film Back
presets (user presets saved in film-back-presets.json), Resolution
Override (renders, Preview and Camera view frame), Depth of Field (Enable,
Focus Distance, Autofocus, F-Stop, Iris Blades, Iris Rotation) in Cycles
renders only, camera Background Image (Camera view + F9 composite),
overlay safe areas / grids / diagonals / chevrons / horizon, Display Size
and editable Lens Squeeze / Film Roll. Stored only: Lens Distortion, Edge
Weighting. Native format v20; Cycles bridge ABI v5 (renumbered after Texture Locators took v19 / ABI v4). Not built: Stereo,
Motion Blur, foreground image, matcher, golden rectangle. Picks CAM-Q01..10
and checks CAM-G01..17 in docs/GUI_QA_HANDOFF_M1_M2_M3.md.

**Toolbox icons batch 8 (branch toolbox-icons, 2026-11-06):** 28 new
generator icons (`tools/operandi-icons/operandi_icons/deform_icons.py`): every
Deform tab entry, built or greyed (presets, Tweak, Push, Smooth, the flyout
lists, Radial/Linear Align), the three Edit tab Slice tools (now icon rows
instead of text), Shift, and Inset on its original artwork. Tests check no
two Deform entries share an icon and none borrows another tab's. Jason
approved the art 2026-11-07; merged to main.

**Items list header (branch item-list-header, 2026-11-06):** Add Item
split button (label and ⏷ open one menu), Select (Item Patterns) and
Filter (Item List Filters) popovers, Sel / Notes / Text toggles; the old
Item button and New Mesh Beside Selected are gone (Delete and Rename stay
on the row menu). Add Item makes Mesh, Camera, Backdrop Item and all nine
lights after the selected item as a sibling (end of the root list with none
selected), alone selected, one undo step; Recent keeps 8, newest first,
across restarts. The menu has the full IL-02 structure with a find field;
unsupported entries are greyed, empty categories hidden behind Show
unsupported. Patterns (`*`, `?`, any case, type ticks, Limit to children)
Select / Unselect / Hide, one undo step each. Filters are view-only:
Flatten (dots), Sort by Name / Type, type ticks, show-only-selected, text
search with Flat, greyed parents kept; saved with the layout. Core:
`Command::AddItem`, `Command::ItemPattern`, `item_list::list_rows`;
script `item.add`, `item.pattern`. Greyed: Groups / Group, By Color,
Has Notes, Search-notes. Questions IL-CMD, IL-P1..P6.
GUI checks: docs/ITEM_LIST_HEADER_GUI_QA.md.

**Add Loop and Slide options (branch add-loop-slide, 2026-11-06, tool
audit Tier 1):** Add Loop and Add Point show Tool Properties while armed:
Position, Split at the Middle, Both Sides (two mirrored loops, one undo
step; one loop at 50 %, AL-05), Absolute Distance with Offset from End and
Distance 1/2 (a drag sets Distance 1), Preserve Curvature with Tension,
Show Position / Show Distance readouts; `edge.addLoop/addPoint` use them.
Slide (edges and points) adds Stop Negative Slide, Preserve Curvature with
Tension and Slip UVs (every UV map); Vertex Slide adds Loop Slide (off lets
a point pass its edge's end). Greyed: Keep Quads, Split, Cap Sections, Gap,
Duplicate, and Edge Slide's Loop Slide / Merge Vertices (fixed on per
ES-04/ES-05). Provisional readings TA1-Q01..Q07. GUI checks:
docs/TOOL_AUDIT_TIER1_GUI_QA.md.

**Weld drag (branch weld-drag, 2026-11-06):** Weld (Vertex tab) is now
press-drag-release instead of click source, click target (WD-01..05).
Snap radius 15 logical points; release on a point merges at the
destination (destination keeps its id and point attributes, existing Join
healing), release elsewhere moves the point at its view-plane depth; one
undo step per drag, tool stays armed. Preview is an overlay; the scene
changes only on release. `vert.weld target:<n>` still welds the selection.
Provisional picks WD-Q01..Q06; GUI row in GUI_QA_HANDOFF_M1_M2_M3.md.

**Slice tools Tier 1A (branch slice-tools, 2026-11-06):** new Edit tab
(between Select and Vertex) with a Slice section: Polygon Slice (Shift+C),
Edge Slice (G), Loop Slice (Alt+C), text buttons until icons are approved.
All three are live tools on a shared cut engine (`slice_cut.rs`): per-tweak
undo inside the tool, one "Undo <tool>" step after Drop, Cancel restores.
Loop Slice: rings from a selected edge or two neighbouring polygons (or a
clicked edge), Count/Mode Free-Uniform-Symmetry/Current/Position/Uniform,
on-screen slider bar (Edit Move/Add/Remove, resizable), Slice Selected,
Slice N-gon (even n-gons), Preserve Curvature + Tension, Select New Polygon.
Edge Slice: edge-to-edge clicks, drag to place, drag points, close on the
first point (closing colour), Ctrl/Split at Middle, Split Polygons, Show,
Snap Value, Inside Polygons, Always Raycast, Multi Slice (dashed line).
Polygon Slice: drag a line (plane perpendicular to the work plane), drag
ends/line, Shift+drag adds, middle-click repeats, Start/End fields, Fast
Slice, Angle Snap, Split, Cap Sections, Gap, Offset Side, Infinite.
Greyed: Loop Slice Keep Quads, Split/Cap/Gap and profile options; Edge
Slice Offset. Provisional readings SL-Q01..Q15, PS-Q1; GUI checks:
docs/SLICE_TOOLS_GUI_QA.md. Speed (16k grid): core preview 2.5–3.1 ms per frame; a full
frame on the subdivided grid is ~4 ms for Edge/Polygon Slice and ~10 ms
for Loop Slice (was ~30 ms; branch loop-slice-speed, 2026-11-07), whose
whole 128-cell ring changes every frame. Fixes were shared: GPU patches
compute each smooth piece/point normal once, partial subdivision uses one
ring of context (proved equal to a full rebuild) and cached point→polygon
lists, and selected-edge overlays read only nearby pieces. Guard:
`loop_slice_drag_work_stays_near_the_ring`.

**Render button + Preview window (branch render-preview, 2026-11-05):**
top-bar Render (F9) and Preview (F8) right of Work Plane; Preview is a floating window with a
progressive Cycles render (coarse pass, then adaptive samples to 256 and
denoise) that restarts on scene changes, with Status/Lock/Effect/LUT/Camera
controls, the Options menu (built items work, the rest greyed) and Preview
Settings. Cycles C ABI gained light-path, adaptive, thread and
transparent-film fields. Provisional picks: RP-Q1..Q10; GUI checks RP-G01..16.

**Inset and Shift tools (branch inset-shift, 2026-11-06, tool audit
Tier 1):** two live Polygon tools without toolbox buttons (Jason,
2026-11-06: they aren't toolbox buttons in the reference); reached through
`poly.inset` / `poly.shift`, typed names, the modeling popup, the menu
bar's Geometry > Polygon > Shift / Inset and unassigned input actions. Inset uses the admitted `inset_geometry` (spec inset-2026-11-05):
one field, Inset (0 m, negative grows). Shift (provisional, TA1-Q08):
one field, Shift (0 m); each polygon moves along its own normal on new
points, joined by side quads (Inset's topology at 0). Nothing selected =
every polygon. One handle each (Inset like Bevel's Inset square, Shift
along the normal) and drag-anywhere. Their tool info reuses the Bevel and Extrude icons.
GUI checks: docs/TOOL_AUDIT_TIER1_INSET_SHIFT_GUI_QA.md.

**Interchange formats (branch file-formats, 2026-11-02):** OBJ, STL, glTF/GLB
and VideoScape GEO import and export, plus SVG export, from File > Import…,
File > Export and Open; LXO export for Modo 17.1v1 from spec lxo-export-2026-11-03.
See FILE_FORMATS.md.

**Subdivision v3 + edge weights (branch subd-edge-weights, 2026-11-05):**
all 57 v3 subdivision oracles that apply pass on main already; added the
SDV-14 mixed-kinds rule, edge weights on Subdivision polygons (EW-05,
Operandi rule), the Edge Weight tool in percent with negatives, and Freeze
keeping Face-edge weights. See SUBDIVISION.md.

**Snapping (branch snapping, 2026-11-05):** switch (button, X, hold X for
one drag), Snapping Options (F11), Vertex and Grid targets on Move, Tweak
and transform centres, 24/40 px ranges; the four snapping oracles pass.
See SNAPPING.md.

**Vertex tool options (branch vertex-tool-options, 2026-11-03):** built
Use Material on Vertex Bevel/Extrude and Vertex Slide Percent/Distance and
Merge Vertices from spec vertex-tool-options-2026-11-03; the other options
show greyed at their recorded defaults until measured (VTO-Q01..Q10).
Tests: crates/operandi-core/tests/vertex_tool_options.rs.

**Input Map editor (branch keybinding-editor, 2026-11-06):** the Input
Map window (Edit/Preferences > Input Map...) is now a real editor: search by
name or key, Find by Key, filter by selection mode, click a shortcut and
press the new keys, Clear/Reset per row, Reset All. Clashes (same key, same
context) show in red with "Remove it from ..." or Undo; Save waits until none
are left. File format unchanged. Own design, not Modo's (input_map_editor.rs).

**Panel corner dot (branch panel-corner-dot, 2026-11-06):** a small dot at
the top left of every view and side panel (amber when active). Drag a view's
dot onto another view's centre to swap, onto an edge to dock beside, outside
to tear off at the release point; Shift copies, Ctrl splits, Esc cancels.
Right-click: Duplicate, Copy, Detach, Delete, Maximize, Split H/V/Quad (quad
dividers linked). View > Layout has Lock Thumb and Lock Divider Dragging.
Side panels only float and dock home (PANEL-Q01). See PANEL_CORNER_DOT.md;
GUI checks PD-01..14; questions PANEL-Q01..Q12.

**View shortcuts follow the pointer (branch claude/project-thread-7jm5hw,
2026-11-08):** view keys act on the main-window view under the pointer,
which becomes active, with no click first (UIC-70; Jason): Maximise, the
view type and shading keys, Frame All/Selection, Zoom In/Out, Re-centre,
Turn, Turn Upright, Align/Fit and Align Selected, and the View and Shading
pies (`viewport_buttons::hovered_view_action`), and the scroll wheel's zoom. With the pointer off the
views the ungated ones still act on the active view. When a floating
panel has the keyboard its shortcuts now run too (as the Preview's do),
and the view under the mouse comes from the system mouse position, since
the main window hears no pointer moves then. Not covered: keys pressed in
a torn-off view window, and keypad 0 over non-view panels (UIC-70 says
panels and lists maximise too).

**Viewport header buttons (branch viewport-buttons, 2026-11-06):** from
BEHAVIOR-VIEWPORT-BUTTONS v1 (summary from Research; the spec file isn't in
`specifications/` yet). Each 3D view's header ends with Pan, Rotate, Zoom,
Maximise/Restore and a Viewport Properties gear (viewport_buttons.rs,
viewport_properties.rs). Maximise shows one view of a view group alone and
Restore brings the splits back exactly (numpad 0 / Alt+0, tab right-click,
saved with the layout). Pan/Rotate/Zoom drag through the in-view navigation
code; Esc restores. The gear or O opens a per-view popover whose rows are
the view's existing settings plus Show Viewport Controls and Information,
Background None/Gradient, Show Meshes and Show Selection Rollovers. Each
view's display settings now save with the layout. Provisional and demoted
items: SPECIFICATION_QUESTIONS.md "Viewport header buttons". Glyphs are
drawn shapes until generator icons are approved. The popover follows the
v1.2 layout addendum (one 420 px column, dropdowns, rotated full-name
tabs, folding sections remembered); screenshots
~/Developer/Operandi-Builds/viewport-props-*.png.

**Fast item drags (branch item-drag-fast, 2026-11-05):** Item-mode
Move/Rotate/Scale no longer rebuild and re-upload the item's GPU geometry
every frame; the cached geometry (fills, wire, dots, UV seams, shadows) is
drawn through a per-item matrix and rebuilt exactly once the tool drops.
65k-quad drag frame 65 ms → 4.6 ms (CPU, `item_move_drag_large`); guarded
by `item_transform_drags_stay_on_the_fast_path`. A component Move, Rotate
or Scale that moves every point together (Polygon mode, nothing selected)
draws the cached mesh through the same matrix: 579 ms → 4.5 ms per frame on
a subdivided 19k-quad mesh (`whole_mesh_move_drag_gpu`, guarded). Drags
update the tool once per frame, the rollover no longer rebuilds the picking
surface while the scene changes, unsaved-changes checks compare meshes by
allocation, and imports open in Item mode (IMP-Q01, provisional). In-app
benchmark: `operandi.bench.drag [frames] [anywhere]`.

**Frame Time rebuild counts (branch frame-rebuilds, 2026-11-05):** Window >
Frame Time also lists what the last 60 frames rebuilt: whole GPU meshes,
patched polygons, bytes uploaded, subdivision (full and local), picking
surface and topology results, plus the busiest frame. A drag on the fast
path reads zero after its first frames.

**Bug and crash reports (branch report-bug, 2026-11-05):** Report Bug
button (top right) opens a pre-filled GitHub issue with version/commit, OS,
GPU and recent log lines; a panic hook writes crash files and the next
launch offers Report / Not now / Don't ask; macOS DiagnosticReports `.ips`
files for native crashes go through the same prompt. Nothing is sent
without the user submitting it. Windows/Linux open and reveal untested.
See BUG_REPORTS.md.

**Colour preferences (branch color-themes, 2026-11-04):** Preferences >
Display > Colors edits every theme colour live (swatch picker + RGB 0–1,
per-colour and Reset All), saved as overrides in color-theme.json so untouched
colours follow the built-in default; Load from Scheme / Save to Scheme… in
color-schemes/. Order follows spec preferences-colors-2026-11-04; per-view
schemes and colours with no Operandi drawing yet (rollover kinds, falloff
range, topo, boundaries) are not built. Add new colours to theme.rs `colors!`;
a test fails if drawing code uses a default constant directly.

**Built-in colour schemes (2026-11-19):** Colors now offers seven embedded
schemes ahead of saved schemes. Export writes every registered colour to JSON.
Import validates entries and saves a copy under the cleaned filename. A numeric
suffix preserves any existing file with that name. Steel is the factory
default. Classic Grey keeps the previous colours, and older selected or saved
schemes retain their Classic Grey base. Explicit custom colours carry forward.

**Set Material dialog (branch set-material, 2026-11-04):** the Polygon Set
Material dialog now sets Type, Color (with the macOS colour picker), Diffuse,
Specular and Smoothing as well as the name; Make Default and Add to Library
are stored only (MAT-Q01). Materials live in `Scene::materials` by tag name
(core `material.rs`), shown in the viewport's fill colour and rendered by
Cycles. Saved in native scenes and glTF; LXO keeps tags only (MAT-Q06).
Tests: crates/operandi-core/tests/polygon_set_material.rs,
crates/operandi-app/src/materials/polygon_material_tests.rs.

**Set Material with nothing selected (branch set-material-empty,
2026-11-05):** like Set Part, Set Material tags every surface polygon of the
active mesh when no polygons are selected. Set Part... and Set Material...
moved to the end of the Polygon Commands group, below Align.

**Drag-to-scrub numeric fields (branch claude/project-thread-cf6nwx,
2026-11-04):** every Tool Properties numeric field (the shared `number`
control, so all tools) has a ◂▸ handle on its left; drag it left/right to
change the value live. Steps per point: 1 cm, 0.5°, 0.5 %, 0.01, or one per
5 points for counts; Shift ×10, Option ÷10; Escape puts the value back; a
value the tool refuses or clamps stops the scrub there. Each value goes
through the same confirm route as typing. One undo step per drag: Move,
Rotate, Scale, Transform record each scrub on release (like a handle drag,
fields then read 0); other tools' sessions stay one step at Drop.
Provisional: the steps and modifiers are Operandi's own choice, awaiting
Jason's feel check; distance steps are metric even with English units; the
handle is painted, not a generator icon. Dialog fields (Create New Vertex,
Merge distance) have no handle yet. Clean-room note: this build thread was
shown a reference-app screenshot of a numeric field with drag arrows; the
design is Operandi's own. Tests: crates/operandi-app/src/anim/scrub_tests.rs.

**Undo and Auto-Save preferences (branch preferences-undo-autosave,
2026-11-04):** Preferences > Defaults > Application sets Max Undo Level
(100) and Group Selection Commands / Max Sub-Commands Recorded; Defaults >
Auto-Save backs the scene up on a timer with a 10-second Cancel countdown,
into its own folder with a revision limit, and clears backups older than 31
days on quit. Saved in undo-autosave.json. Other defaults are provisional
(see SPECIFICATION_QUESTIONS.md). Image backups aren't built (no images yet).

**Navigation preferences (same branch, 2026-11-04):** Preferences > Input >
Remapping: tooltip delay, wheel zoom speed/direction/centre, drag-zoom
axis and list wheel scrolling; the rest of NAVIGATION.md is listed as not
built on the page.

**Selection and handle preferences (same branch, 2026-11-04):** Input >
Selection (start mode, rollovers, point size, click reach, lasso rules) and
Display > Tool Handles (line width, hit sizes, handle scale, invisible
handles). Not-built rows are listed on each page.

**Multiple open scenes (branch claude/project-thread-rjs74u, 2026-11-04):**
New and Open add scenes; every open scene is a root row of the Items list
(each a parent of its items, the current one bold; click another scene or one of its items to switch; "(new
scene)" row; right-click Save/Save As/Export As/Close Scene), also File >
Current. Each scene keeps its own history, selection, path, saved
state, cameras, live tool and recovery journal. Close Scene (Cmd+W), Close
All (Cmd+Shift+W) and Quit ask about each unsaved scene. Spec MULTI-SCENE.md
v0.1; open points are provisional (MS-Q1..Q8). Not built: Import appending
into the current scene, Save All, Revert, a "new scene" button in the Items
list. Tests: scenes_tests.rs.

**Units and grid preferences (same branch, 2026-11-04):** Input > Distance
Units (SI/Metric/English/Game/Unitless, default unit, inch cutoff, metres
per game unit) drive every distance field; Input > Grid and Work Plane has
Fixed/Dynamic grid and size, plus Viewport Fit Factor (1.5).

**Tool defaults and overlays (same branch, 2026-11-04):** Defaults > Tool
Settings (start-up action centre/axis), Display > 3D Information Overlays
(selection, polygon count, grid size, polygon type, current tool) and
Defaults > Scene Save/Export (Author/Copyright into glTF). The many
not-built rows are listed on each page and in SPECIFICATION_QUESTIONS.md.

**Render window (branch render-polish, 2026-11-05):** Render > Render Viewport
opens a Render window (its own OS window; macOS window tabbing is off app-wide) that fills in as Cycles works (about every 1–2 s),
with Pause/Resume (Space), Finish (stop adding samples, denoise, keep) and
Cancel (closing the window cancels too). Drag a rectangle on the picture and
Render Region re-renders only that area into the picture. Finished renders
still save to ~/Pictures/Operandi Renders, but no longer open in Preview
(Show in Finder instead). Camera view now frames the render picture (16:9
outline, dimmed outside) so it matches the render; film offset isn't shown
in the view yet. Operandi's own design, no spec. C API change:
rerun scripts/build-cycles.sh. Tests: render_ui.rs, operandi-cycles.

**Cameras and lights v3/v4 (branch cameras-lights-cl1, 2026-11-05):** own
glyph per kind in the viewport and the Items list (render camera bold with a
marker); Camera view follows the selected or newest camera and names it;
Move/Rotate/Transform show the selected camera or light's own channels in
Tool Properties, with viewport handles for area, spot, camera (Focus
Distance) and point lights, one tweak per drag. Light view not built.

**Point, Spot and Area lights CL-2 (same branch, 2026-11-04):** Item > Create
Light > Directional / Point / Spot / Area and Item > Create Camera; Radius,
Falloff Type, Cone Angle, Soft Edge, Shape, Width, Height with per-kind
Intensity labels; wireframe shapes per kind; Cycles point, spot and area
lights (provisional power conversions, falloff and partial Affect notes in
the status line). Guesses: SPECIFICATION_QUESTIONS.md "Point, Spot and Area
lights (CL-2)".

**Cameras and lights CL-1 (branch cameras-lights-cl1, 2026-11-04):** new
scenes get a Camera and a Directional Light (Add Item adds more); channels in
Properties (Make Render Camera); white wireframe shapes with per-view Show
Cameras / Show Lights; view menu "Camera" looks through the render camera and
navigating moves it (one undo step per press); Cycles renders through the
scene camera at 1920 × 1080 and uses the scene's directional lights as suns.
Native version 13. All defaults in camera_light_defaults.rs; guesses in
SPECIFICATION_QUESTIONS.md "Cameras and lights (CL-1)". Tests:
crates/operandi-core/tests/camera_light.rs, camera_light_ui.rs, operandi-cycles.

**Backdrop items (branch backdrop, 2026-11-04):** Item > Create Backdrop or
Add Item > Backdrop Item adds a reference picture item from spec
backdrop-2026-11-04: PNG/JPEG image entries shared by file path, all six view
projections plus Camera (stored only), size rules, Invert/Contrast/Brightness
in the measured order, Flip, Transparency, Blend, Auto Fit, Backdrop Size
buttons, per-view Show Backdrop, an orange placeholder outline and grid
until a picture is loaded, Transform/Move/Rotate/Scale from any mode,
selection outline, Frame All, save/reopen
(native version 11), exports leave Backdrops out. Drawn on the graphics card
between grid and meshes. Commands `backdrop.*` (BD-Q19). Provisional
readings: SPECIFICATION_QUESTIONS.md "Backdrop items". Tests:
crates/operandi-core/tests/backdrop.rs (oracles 01–67 where numeric; spec v2:
Auto Fit resets the out-of-plane scale, first-created on top, " (2)" names),
backdrop_ui.rs and backdrop_gpu.rs.

**Backdrop gaps (branch backdrop-gaps, 2026-11-09):** BD-61 Auto Fit fits
the active meshes' combined box (hidden selected meshes count; a Backdrop
selected alone uses the mesh that stays active). BD-60 a picture dropped on
a 3D view becomes a Backdrop facing that view (one undo step; elsewhere it
only loads an image entry; scene files still open); scripted as
`backdrop.add name: image: project:` and `clip.addStill filename:`. BD-62 Pen Make UVs maps points to the picture of the topmost
Backdrop in the axis view drawn in, Flip mirrored, unclamped; world X/Y
otherwise. BD-12 (spec review): a Camera Backdrop fills the render frame in
a camera view (was drawn nowhere). Provisional: BD-Q04, BD-Q06, BD-Q21,
BD-Q22. Tests: crates/operandi-core/tests/backdrop_gaps.rs, backdrop_ui.rs,
camera_props_tests.rs.

**Clips chooser (branch backdrop-clips, 2026-11-09):** BC-01 the Backdrop
Image field is a thumbnail, the clip name and an arrow; BC-05 (first cut)
it opens a floating "Clips" chooser (clip_picker.rs, reusable): Choose
"<current>" keeps it and closes; Add Clip > (load image)… opens the file
panel and adds one clip (a file the scene has reuses its entry); "(none)"
and the scene's clips as thumbnails, one click assigns and closes. BC-04
Esc closes the Add Clip menu, then the chooser, with no change. Each change
is one undo step through the existing commands; re-picking is no step.
Not yet: other folders, other Add Clip entries, navigation, filter,
thumbnail size, view buttons; the shader tree and camera keep their
dropdowns.

**Dark theme (branch dark-theme, 2026-11-03):** darker default chrome, panels,
Scenes list and viewport background in theme.rs; our own values, contrast-tested.
**Viewport display (branch viewport-display, 2026-11-03):** per-view Display
menu from specifications/viewport-display-2026-11-03: seven styles for
active and inactive meshes (Alt-click sets inactive), overlay kind/opacity,
smoothing, inactive overrides, Show Grid / Wireframe in Item Mode / Verts /
UV Seam / Smoothing, saved presets with modified marker and restore.
Styles shade on the graphics card, so the shared mesh geometry is unchanged
per view. Provisional: Vertex Map uses the first map; UV seams are island
borders of the first UV map; CPU-drawn previews show Wireframe or lit only.
**Tweak, slice TWK-1 (branch tweak-tool, 2026-11-03):** T starts Tweak
(Element Move) in any mode; hover shows the point, edge or polygon a press
would grab; dragging moves it under the pointer, leaves the selection alone,
and each drag is one undo step with the tool staying live. Provisional
readings (picking priority, drag plane, Esc, symmetry) are listed in
SPECIFICATION_QUESTIONS.md, Tweak. Range/falloff (TWK-2) and the handle,
numeric fields, symmetry and snapping (TWK-3) wait for measured specs.
Tests: crates/operandi-app/src/deformers/tweak_tests.rs, core tweak.rs.
**Tweak part 2, TWK-2 (branch claude/project-thread-94fqz1, 2026-11-04):**
Mode (all seven entries), Range with Linear / Ease-In / Ease-Out / Smooth /
Custom, and Connected Elements Ignore / Use Connectivity / Rigid / Edge
Loops, in Tool Properties and the MCP bridge; a yellow range sphere shows
the reach. Weights are worked out once per drag. Passes the spec v1's 13
movement oracles (tests/tweak_oracles.rs). Provisional: 3D distance
(TWK-Q16), Edge Loops at poles and with Range, Symmetry with Range (see
SPECIFICATION_QUESTIONS.md, Tweak part 2). Still to do (TWK-3): handle,
Position X/Y/Z, right-drag Range, Always Raycast, snapping.
**Tweak part 3, right-drag Range (branch claude/project-thread-dl52gd,
2026-11-04):** with Tweak, a right-drag anywhere resizes Range (right
bigger, left smaller, vertical ignored, stops at 0), at a rate that keeps
the sphere's on-screen growth the same at any zoom. The Range field and the
status line follow live; the 4-circle sphere shows only while the button is
held (no longer on hover or left-drag). One drag is one undo step and Tweak
stays. Provisional: rate constant, sphere centre, Alt still navigates,
Rigid blocks it (SPECIFICATION_QUESTIONS.md, Tweak part 3).
**Display style polish (branch display-polish, 2026-11-05):** Gooch's cool,
warm and outline tones and Reflection's sky, horizon, ground, far ground and
light strips are Display > Colors entries ("Display Styles"), applied live;
defaults keep the old look. Gooch adds a white Blinn-Phong highlight
(exponent 32, strength 0.5) and a dark silhouette rim (dot(n, eye) 0–0.25).
Live primitive and Cone/Ellipsoid previews now follow the view's Solid,
Gooch, Cel and Reflection styles per face (Vertex Map stays neutral grey);
CPU and GPU share display_shading.rs. Provisional: highlight/rim numbers
are our own choice.

**Tool Drop and switching (branch tool-drop, 2026-11-05):** Cone's Drop now
keeps the drawn Cone as one "Cone" undo step (it used to vanish), like every
other primitive. Picking any tool while another is open drops the open one
(its work is one undo step) and starts the new one; the open tool's own
button drops it; a field error refuses the switch and changes nothing. One
rule in `Editor::tool_switch` replaces the old pairwise blocks, so Cone,
Pen and Polygon Reduction no longer refuse a switch. Ctrl/Shift-click unit
insertion drops an open tool the same way (two steps: the tool's work, then
the unit). Cone's Apply is a plain "Cone" step like Ellipsoid's, so Undo
never reopens Cone. Tests: tool_switch_drop.rs.

**Mode switch drops the tool (branch mode-switch-drop, 2026-11-05):** changing
selection mode (1/2/3/4, the Select menu, the mode buttons) while any tool is
open drops that tool first, then changes mode, in one keypress. Same rule as
picking another tool: the tool's work is one undo step, and a Drop that fails
(a field error, a Pen draft with no points) refuses and changes nothing. This
covers Move/Rotate/Scale, every mesh tool, primitives, Pen, Spike, Bridge,
Reduction and an Extrude stage; before, only Smooth Shift and Thicken did this
and the rest said "Drop or Cancel the active tool first". The mode entries in
the menus stay enabled while a tool is open. Tests: tool_switch_drop.rs
(core), input_focus_tests.rs (W/E/R then 2).

**Display modes (branch display-modes, 2026-11-05):** the Display menu has
ten styles in order Wireframe, Solid, Vertex Map, Shaded, Texture, Default,
Advanced, Gooch Tone Shading, Cel Shading, Reflection (the button keeps
"Gooch"/"Cel"). Texture draws the material colour full bright, unlit.
Default draws exactly as Shaded until image maps exist. Advanced is Default
lit by the first visible, rendering scene light (the view light when there
is none) with a 2048² shadow map (3×3 filtered) cast by every filled mesh
and received by Advanced meshes; a light with Shadow Type None lights but
doesn't shadow. Display > Show Shadows (Advanced section, on) turns them
off per view. New views start in Default; Split View still copies the
active view. Styles save by name, so old presets load. Provisional: one
light only; point/spot lights are treated as directional from their
position; anti-aliasing is the existing 4× MSAA all styles already use;
live CPU previews draw Advanced unshadowed; Operandi has no per-item draw
colours or image maps yet, so those parts of the brief have nothing to act
on. Not done (later tiers): AO, reflections, Advanced settings panel.

**Unfinished, buildable now:**
- Polygon Reduction refuses meshes with UVs, as its spec requires (RED-003:
  refuse rather than lose data; maps are listed later work). Untick Make UVs
  on a primitive to reduce it. Grow Quads, Fill Quads, Edge Bridge and
  Chamfer Edit carry UVs since 2026-11-01.
- Project system (docs/PROJECT_SYSTEM.md): Phase 1 built 2026-11-01 (File >
  New/Set/Recent Project, operandi.project.toml, default folders, Open/Save
  default to scenes/, last project reopened at launch), with native macOS
  Open/Save panels and folder pickers. Phases 2–4 (project-
  relative references, shared path service) not started; Phase 3 (project
  presets: File > Project Presets, load/save, yours win) built 2026-11-01.
- Cmd+click modeling menu: the shell can be built; its contents need a spec.
- Known "basic" limits: Edge Bevel/Extrude separate interior edges only; Edge
  Relax simple strips; Polygon Reduction whole planar meshes only; viewport
  dragging for Bridge, Spike, Cone and Ellipsoid (fields only).

- Icons: the app now uses the generated set from tools/operandi-icons (49
  icons, all five batches approved by Jason 2026-11-01), replacing the
  dictionary artwork; Merge Tool, Weld, Tear, Vertex Slide, Edge Chamfer and
  Edge Crease have their own icons instead of placeholders.

- Toolbox tabs (Jason, 2026-11-01, a deliberate departure from PAR-014):
  the category tabs sit on the left, start below the shared rows
  (transforms, Mesh Cleanup, New Mesh) which now span the full width, and
  look like tabs (active joins the content with an orange edge; others are
  darker and set back).

- Symmetry (2026-11-02, specifications/symmetry-2026-11-01): context bar
  menu Off/X/Y/Z with Offset; Move/Rotate/Scale mirror the transform for
  positive-side points and keep on-plane points on the plane; Select
  Symmetrical; Cube/Sphere/Cylinder are created twice; Symmetrize
  Positive/Negative. All 11 measured cases pass (tests/symmetry.rs).
  Provisional: tolerance measure, selection order, Symmetrize matching
  (SPECIFICATION_QUESTIONS.md, Symmetry). Not built: Topology, Arbitrary and
  Work Plane axes, Ellipsoid/Cone, UV symmetry.

- Falloff (2026-11-02, specifications/falloff-2026-11-01): context bar menu
  None/Linear/Radial with shape (incl. Custom), Symmetric, Auto Size and
  Reverse; settings kept per type. Weights scale Move/Rotate/Scale, Polygon
  Bevel, Edge Bevel/Chamfer and Vertex Bevel. All 18 measured weights and both
  Bevel cases pass (tests/falloff.rs). Extrude and Smooth Shift refuse a
  falloff for now (FAL-Q04/Q06).

- Selection (2026-11-02, specifications/selection-2026-11-02): Select
  Between follows the measured ties (one route), works on points, and joins
  only the last two of 3+ picks; Loop from a single polygon; Parent/Unparent
  with or without Keep Position (item right-click menu). 22 of 29 measured cases
  pass (tests/selection_measured.rs); the other 7 are Previous/Next Loop, not built; nor is
  multi-item editing.

- Polygon Bevel (2026-11-02, specifications/polygon-bevel-2026-11-02): all
  20 measured cases checked (tests/polygon_bevel_measured.rs). Empty
  selection now bevels everything; Auto Weld removes fully collapsed caps.
  Refused collapsing insets, zero walls and index order are logged
  departures.

- Polygon Extrude (2026-11-02): 21 measured cases checked
  (tests/polygon_extrude_measured.rs); 15 match the reference's geometry,
  index order differs by design. In-plane merge, non-manifold input and
  empty selection in the app not built.

- Edge Bevel / Chamfer (2026-11-02): equal-angle round rows, measured Depth
  rule, Round Level → 2^L segments; 7 of 21 measured cases match
  (tests/edge_bevel_measured.rs). Junctions, Square/Sharp and Width mode not
  built.

- Edge Slide (2026-11-02): all 15 measured cases pass
  (tests/edge_slide_measured.rs); Distance mode, merge at 100%, no clamping,
  border points use their other side. Add Loop unchanged (no cases yet).

- Mesh commands (2026-11-02): 27 of 34 measured cases match
  (tests/mesh_commands_measured.rs); Vertex Remove, border Edge Remove,
  Merge with nothing selected and Align follow the reference now.
- Primitives and UVs on new points (2026-11-02): checked; geometry of the
  options Operandi has matches. Missing primitive options, the continuous
  point-UV gap and seam choices are logged.

- Edge Bevel junctions (2026-11-02): shared points, corners, loops and
  interior ends; 16 of 21 measured Edge Bevel/Chamfer cases match (Square,
  Sharp and Width mode remain).

- Edge Bevel / Chamfer options (2026-11-03): Mode Inset/Width, Shape
  Round/Square/Sharp, Use Material, and for Chamfer Parallel, Stop At Edge,
  Auto Weld and Edge End, in the panel and `tool.attr edge.bevel|chamfer`;
  border ends and corner patches past two rows. All 21 first-batch and 20
  of 31 batch-3 cases match. Provisional: patch inner points, Chamfer Width
  at 60°, the unmeasured options' readings (EBC-Q10). Eight unmeasured
  options are shown greyed out (EBC-Q03).

- Batch 2/3 and pass 1 (2026-11-03): Symmetry Arbitrary/Work Plane/
  Topology and mirrored Ellipsoid/Cone; falloff on Spike; Cone and
  Ellipsoid drawn by dragging; drag-drop parenting; Ctrl-click eye hides
  children; KEY-01/02 defaults; item transforms with Negative Scale;
  Automatic action center; Add Loop geometry (all but Split) and 9 of 14
  batch-2 slides. Deferred items are listed in SPECIFICATION_QUESTIONS.md
  ("Pass 1 specs and what's still deferred").

- Transform drags (2026-11-03): a drag anywhere off the handles moves
  freely, on the model too; each Move/Rotate/Scale/Transform drag is its own
  undo step and Undo/Redo keep the tool active. Provisional (HND-Q04).

**Tool-by-tool gaps:** see [TOOL_GAPS.md](TOOL_GAPS.md), the brief for each
round of spec work.

**Waiting on specs:** Modo default shortcuts; Cmd+click menu contents; Fill on
open areas (SEL-Q-M2); row selection by click (SEL-Q-M4); UVs on new points
(SQ-UV-01); Symmetry pointer picking (SYM-Q02/03), Snapping and Item hierarchy; primitive UV layout;
numeric mini-sliders/links/presets; context popup contents.

**Not yet checked by hand (GUI):** PS-07 in GUI_QA_HANDOFF_M1_M2_M3.md has
nothing visible to check yet (PS-01..06 passed in Jason's 2026-11-01 QA
session); LXO File > Open end to end; real 4K/5K and mixed-DPI displays.


## Architecture decisions

- Portable domain crate owns meshes, scene, ordered selection, workplane, commands,
  live transactions, bounded history and versioned persistence. The UI adapter
  supplies hit evidence to validated domain selection commands. CPU picking uses
  the same per-face fan triangulation as GPU drawing.
- Explicit u32 vertex references; each face owns optional corner UVs, material name
  and smoothing flag. Stable u64 item IDs survive rename/history. Component indices
  are revision-local; previews keep the source face index for the cap and clear
  invalidated edge/vertex selections. No hidden pointer-derived identity.
- Live preview recomputes from an owned activation Scene. Failure preserves prior
  valid scene and history. Drop stores one before/after checkpoint; Cancel rolls
  back. At most 100 checkpoints (development budget, not a reference limit).
- Topology edits support a single planar convex quad. Inset intersects inward edge
  offset lines; crossing or degeneracy is rejected. Source cap removed/replaced,
  four walls connect it to the old perimeter, winding retained. Cap UVs preserved;
  wall corners copy source values independently, including seam discontinuities.
  No generated UV maps or claim to general reference UV behavior.
- Domain transforms are f64, world axes/world origin, positive scale; item
  rotations XYZ Euler in radians, UI degrees. Component edits require identity
  item transform; item transforms remain separate from mesh coordinates.
- UI state (camera, layout region sizes, display toggles, fields) is distinct from
  scene state. No generic toolkit inspector; compact shared theme/fields/vector
  icon functions are original. Full candidate panels are intentionally unexposed.
- GPU resources belong to ViewportGpu, separate from the mesh. A per-frame immutable
  projected snapshot uploads explicit 28-byte vertices (3 float32 position + 4
  float32 linear color; manual byte encoding). Metal depth-rasterizes triangles
  and line-list edges into RGBA8-sRGB + Depth32Float offscreen targets. Egui samples
  the color texture; its submission follows viewport submission on the same queue.
  Physical resize replaces textures/registration; wgpu retains in-flight resources.
  No GPU handles persist in scenes. Fixed sample count 1; subpixel edge antialiasing
  and large-scene performance remain future work.
- Logical projection/picking is independent from physical target sizing. Current
  observed UI/native scale is 1.0; simulated ray checks cover 1080p through 5K and
  1×/2×. No mixed-density or other physical-display claim.
- Native coalesced press/move/release can arrive in a single frame. input.rs retains
  that gesture rather than relying only on current button state; regression tested.
- Atomic save: bounded UTF-8 JSON envelope with format/version and explicit fields,
  no memory layouts; validate, create-new same-directory temporary, write/sync,
  rename. Error cleanup never removes a preexisting temporary. Scene/item/vertex
  budgets are documented safety budgets, not Modo limits. Numeric JSON float fields
  encode f64 values, references u32, identities u64 and envelope version u32.
- Unsaved scene journal is separate from the last saved file. File Close/New/Open
  prompt; native Command-Q is recoverable via journal. Recovery restores scene,
  not live tools/history or filename. Backend errors are reported by eframe and
  startup diagnostics, not silently switched to a software viewport.

## Toolchain/dependencies and hygiene

Rust 1.99.1 pinned; eframe/egui/egui-wgpu 0.33.3, wgpu 27.0.1, glam 0.30.10,
serde 1.0.229, serde_json 1.0.154 resolved in Cargo.lock. Public crate versions,
licenses and notice texts: DEPENDENCIES.md and licenses/. Official rustup installed
with minimal profile plus rustfmt/clippy; no shell profile changed. Dependency
sources are public implementation techniques only, never another DCC's source.

Development output: `~/Library/Caches/Operandi-Rust/Builds/Operandi Rust Dev.app`,
linked from `development/Operandi Rust Dev.app`, plus the repository demonstration scene;
identifier `org.operandi.rust.milestone 1.dev`. No install, publish, commit, push,
automation or project-license selection. Build artifacts/recovery/QA logs ignored.
Original math fixtures live in Rust tests; GUI-authored demonstration also in
fixtures/VS_Box_Run01.opscene. Next milestone recommendations are in README.md.

Packaging note: Documents file-provider metadata is reapplied to .app directories
and fails strict signature checks. The script stages in /tmp, signs, copies into
the dedicated Library/Caches development build directory, verifies, and creates
the repository link. This is isolated build output; no installed app replaced.


## Before 2026-11-01

Entries from 2026-10-21 to 2026-10-23 described work against specs since removed
and were taken out on 2026-11-20.

## 2026-11-01 — primitive and selection decisions

From Jason's spec-question answers: mid-drag modifier changes are ignored for
Cube/Sphere/Cylinder; Shift-drag off a live primitive's handles draws the next
one with the same settings; those three keep their segment settings between
uses (like Cone/Ellipsoid) with Reset Tool Attributes and `tool.reset`;
Shift+click adds in every mode and one Shift+Cmd click fills; the lasso picks
edges and polygons unless a surface hides them. All provisional product
policy. GUI checks PS-01..06 in GUI_QA_HANDOFF_M1_M2_M3.md are not yet run.
Open: the Cmd+click modeling menu (contents await a spec) and basic primitive UVs.

## 2026-11-01 — basic primitive UVs

Cube, Sphere and Cylinder generate a "Texture" UV map by default (Make UVs,
kept between uses, Reset turns it on). Provisional original layouts: Sphere
uses the Ellipsoid latitude/longitude scheme, Cylinder a wall strip plus two
cap islands, Cube six tiles; Sphere/Cylinder record counts match the M1.6
reference samples. Edge Bevel/Chamfer/Extrude and Vertex Bevel/Extrude
now carry UV maps on new points (provisional interpolation, SQ-UV-01); other
corner maps are still refused. GUI check PS-07 not yet run.

## 2026-11-01 — Bevel acceptance status corrected

The ordinary/IC Bevel sweep (`bv_ordinary_oracle_sweep`) now matches 173 of
183 reference cases with no failures; Auto Weld/Weld Range landed in
`bevel_weld.rs` after the "166/183, 17 failing" note above. The other 10
(BA-036/042 crossing insets, BA-037..039/043..047 weld outputs) are cases where
the reference result is self-intersecting or folded; Operandi refuses them
atomically. Jason decision 2026-11-01: keep refusing rather than produce
invalid polygons.

## 2026-11-03 — Polygon tool options

- Pen (interactive): Type Polygon/Vertices/Polyline, Flip Polygon, Close and
  Select New work and are kept between sessions; Polyline needs 2 points,
  Vertices 1 (provisional, PEN-Q-I01). Polyline isn't turned to face the
  viewer on Drop. Every other reference Pen option shows greyed out.
- Chamfer Edit, Sketch Extrude, Bridge and Polygon Reduction now list their
  remaining reference options greyed out with a reason, in reference order.
- Bridge Continuous/Close Bridges: not built. The measured chain cases don't
  give a consistent matching rule (BRI-Q-CH01).
- Smooth Shift, Thicken, Spike and Extrude already had every listed option.

## 2026-11-04 — Torus, Capsule, Arc, Tube

- Torus and Capsule are in Create > Primitives, Arc and Tube in Create >
  Draw (Jason, 2026-11-04; labelled until icons are approved), Item > Primitive and the command line (`torus`, `prim.torus`
  etc.), with a numeric panel each: preview on arming, Apply/Drop/Cancel/
  Reset, remembered settings, one undo step per shape, nothing selected.
- Generators reproduce all 36 NPR oracles (Tube By Length within 6 mm).
  Torus linked sizes (Radius/Cross Section/Height/Ring Radius/Hole Size) and
  Arc's Center/Start/End/Radius/Angle links are in operandi-core.
- Provisional: Bulge formulas, Quad Grid beyond 8 sides, Arc Center links,
  Tube curve/frames/By Length/Twist, UVs beyond the defaults (see
  SPECIFICATION_QUESTIONS.md, NPR). Not built: viewport drawing/handles,
  Tube profiles and point editing in the viewport, Polygon Type, Flip,
  Compensation, Gaps, Text (NPR-Q10).
- MCP: `primitive_add` makes Cone, Ellipsoid, Torus, Capsule, Arc or Tube.

## 2026-11-04 — Undo while a primitive tool is active

- Cmd+Z works with Cube, Sphere, Cylinder, Ellipsoid, Cone, Torus, Capsule,
  Arc and Tube active: the first Undo throws away an unfinished preview,
  later Undos step back (also past the tool's own steps) with the tool
  still active; Redo is refused only while a preview shows; the tool closes
  if its mesh is gone. One rule in `history_policy.rs` (`primitive_session`)
  and `handle_undo`/`handle_redo`; tests in `tests/mid_tool_undo.rs`.
- Provisional throughout (MTU-Q1..Q10). Other tools still refuse Undo while
  active; they are the next phase.

## 2026-11-04 — Undo while a tool is active: tweaks, activation, Escape/Q

- Jason's rule (2026-11-04): inside a tool each tweak (a drag, a typed
  value, a scrub) is its own undo step; Undo takes back the last one and
  keeps the tool, Redo puts it back. Once the tool is dropped the whole
  session is one undo step; Cancel takes back the whole session.
  `mid_tool_undo.rs` (`Editor::tweaks`); the app marks held drags and
  scrubs (`tweak_held`, `end_tweak`) so each is one tweak.
- Covers every tool (Jason, 2026-11-04): transforms, Tweak (replaces
  TWK-24's step per drag), the primitives, Bevel, Extrude, Smooth Shift,
  Thicken, edge and vertex tools, Spike and Bridge. A press-to-release drag
  or a scrub is one tweak. Not covered: an Extrude stage (keeps its own
  Begin/Move Extrude history), Pen, Polygon Reduction (RED-004). Apply still
  records a step and keeps the tool.
- With no tweak left, Undo closes the tool and changes no geometry
  (MU-MID-02/03); Redo opens it again with its undone tweaks.
- Escape or Q after Undo keeps what remains and clears Redo (MU-MID-04).
  Document Redo is refused while the tool holds unrecorded tweaks.
- Open: MTU-Q11..Q14. Menu labels still name the step (MU-MID-05). After
  an Apply, Undo of that step follows each tool's old rule. Undo mid-drag
  unchanged.

## 2026-11-04 — Smooth shading (angle and amount)

- Every mesh shades by the default material's smoothing (smooth-shading v0,
  SS-01/02): a corner averages the polygons around its point whose normal is
  within the Smoothing Angle of its own (default 40°), then blends from flat
  by the Smoothing amount (default 100 %). Cube stays crisp, 24-side
  cylinders round out, caps stay hard.
- Settings live on the scene (`Scene.smoothing`, native version 12 only when
  changed), in item properties > Smoothing, one undo step per change or
  field drag. Crease by Smoothing Angle and Smoothing Groups (Off/Crease/
  Force over the existing smoothing labels) are built too.
- The viewport, glTF/OBJ export normals and Cycles (points split at
  creases) use the same rule (`operandi_core::shading`). The view's
  Smoothing switch still forces flat.
- Provisional: the whole spec (not observed), defaults and ranges in
  `shading.rs` constants, SS-Q03/Q04/Q07 readings. The old per-polygon
  `smooth` flag no longer affects shading (SS-Q-OP01). Not built: area/
  angle weighting, Normal Method, hard edges, vertex normal maps, Set
  Smoothing Group command, per-material settings.

## 2026-11-04 — Drag anywhere drives the tool

- Jason's rule: a click-drag anywhere in the viewport drives the live tool
  (up/down = move arrow, left/right = scale widget); dragging a handle still
  edits just that handle. Each drag is one undo step and the tool stays.
- Changed: Bevel, Chamfer, Smooth Shift, Thicken, Poly Haul, Edge Extend/
  Extrude/Slide/Crease/Bevel/Chamfer/Relax, Vertex Extrude/Merge/Tear/
  Slide, drawn Sphere and Cylinder, Spike, Bridge, Polygon Reduction and
  Torus/Capsule/Arc/Tube (drawn on the plane). Tests in
  `drag_anywhere_tests.rs`; Bridge's drag has no headless test yet.
- Provisional: the per-tool mapping and gains (DRAG-Q01).

## 2026-11-04 — Oracle replay fixes

`crates/operandi-mcp/tests/oracle_replay.rs` replays, through the MCP
bridge, the 64 spec oracles the 2026-11-03 spec-side replay flagged, and
pins the known departures. 55 now match (numbering aside).
- Selection after the operation: Extrude selects both caps after Drop or
  Apply (the drag still holds the moving caps); Edge Subdivide also selects
  its new points; Vertex Collapse/Join/Split and Edge Join leave nothing
  selected; one-row Edge Bevel selects every edge between new points, and
  their points; Polygon Merge selects the merged polygon.
- Zero distance: Bevel applied at Shift 0 / Inset 0 builds flat walls
  (PB-O-04); the bridge's `tool_run` applies Bevel/Extrude at zero.
- Extrude with nothing selected extrudes every polygon (PE-O-03), and a
  polygon on an edge shared by three polygons extrudes (PE-N-01/02).
- Cube treats a negative size as positive (PRIM-C-09); viewport side drags
  still refuse crossed sides.
- Vertex Align may leave a polygon flat on the line (kept as degenerate).
- Opening a scene with mixed winding or three-polygon edges records them
  instead of refusing; Align repairs them; Quadruple with nothing to change
  is a quiet no-op (CMD-B3-03).
- Not changed (SPECIFICATION_QUESTIONS.md, ORQ-01..03): sideways Extrude
  point reuse (PE-O-02), the one-edge Chamfer's duplicate zero-area
  triangles, Bevel insets past the centre, Merge of a ring with a hole;
  Select Between / Symmetry selection order.

## Edge and Vertex Extrude starting Width (2026-11-04)

- Fixed: a fresh Edge or Vertex Extrude had Width 0, which refuses any
  Extrude, so dragging the Extrude arrow did nothing. Width now starts at a
  tenth of the shortest side edge (provisional, EXW-Q01).
- Their handles now match Bevel: blue Extrude arrow, red Width line with a
  square end.
- Edge Extrude on a cube edge now cuts the corners away: end polygons run
  straight between the slid points and the caps are triangles (Jason's
  correction). Ends with more polygons around them keep the quad cap
  (provisional, EXW-Q03).
- Edge Extend now works on interior edges too (Jason's correction): the new
  strip is a fin off the first neighbour, so the edge has three polygons
  (provisional, EXW-Q02).


## Selection Sets (2026-11-04, branch selection-sets)

- Built: Assign Selection Set, Use Selection Set (Replace, Select, Deselect),
  Delete Selection Set (one, or all of the mode) and Create Selection Set
  Group (plus Delete Group) for vertex, edge and polygon sets of the active
  mesh. Each edit is one undo step; Use is a selection step.
- Reachable from a small dialog (`open_selection_set(SetAction::…)` or the
  `execute` names "assign selection set", "use selection set", "delete
  selection set", "create selection set group"), the command line
  (`selset.assign|use|delete|deleteAll|group|ungroup`, see `help selset.use`)
  and the MCP bridge (same `selset.*` names). No toolbox buttons yet; the
  Select tab wires those.
- Saved in native scenes. Edge sets and groups are new and need file
  version 13; scenes with only vertex/polygon sets still save as version 7.
  LXO export still leaves sets out (it says so).
- Provisional: SET-Q01..Q08. Known limit: Extrude, Spike and Polygon Bevel
  refuse a mesh that has any selection set (audit X2, already true for sets
  from imported LXO files).

## Modifier-key button variants (2026-11-04, branch modifier-buttons)

- Built the mechanism from specifications/modifier-buttons-2026-11-04: a
  button is a base action plus (Shift/Ctrl/Alt → action) variants, in one
  table (`button_variants.rs`). Templates add variants by the kind of the
  base action (every component-mode button gets Alt → Convert); buttons add
  their own on top or opt out. No per-button code.
- Live on the Vertices/Edges/Polygons header buttons (Alt → Convert; Edges
  also Shift → Bounds and Ctrl → Boundary) and on Set Material... (Alt →
  Set Part...). Labels, lit state, shortcut hint and tooltips swap the
  moment a key goes down, app-wide, also while a text field has focus.
  The command is fixed at mouse press. Cmd never swaps. An unmatched key
  pair shows the base. Keys let go in another app don't stick (MB-09).
- Tooltips list the variants on a second line (MB-Q7). Mode buttons are as
  wide as their longest label, so the bar never reflows; toolbox rows cut a
  too-long label with an ellipsis.
- Not wired yet, because their targets don't exist: Snapping/Mesh
  Constraint/Work Plane/Symmetry "Options" (no separate toggle and options
  panel), Render → Render Window (Render is a menu item), Make → Make
  Polygon... (no dialog), Set Material Ctrl → Set Smoothing Group, the
  "open as popover" variants. Each is one table line once its action exists.
- MB-Q8 (Jason, 2026-11-05): the primitive cells (Cube, Sphere, Cylinder,
  Ellipsoid, Cone, Torus, Capsule) are on the table through a
  `primitive-quick` template: Shift → "{Name} Item" (1 m primitive as a new
  mesh item), Ctrl → "Unit {Name}" (1 m primitive in the current mesh), no
  drag. Same commands as before; the old PM route (`primitive_button.rs`,
  keys at release, suppressed while typing, Ctrl+Shift refusal) is gone.
  Keys are read at press, labels swap while typing, Ctrl+Shift draws (MB-13).
  Icon cells show the swap as an "Item"/"Unit" corner tag. Clicks still do
  nothing while a menu, modal or navigation drag owns the input. Dropping an
  open tool first comes from core's unit insert (branch tool-drop).
- MB-Q9 (Jason, 2026-11-05): Shift "Bounds" replaces the selection with the
  border edges of the selected polygons; Ctrl "Boundary" adds them to the
  edges already selected (new `SelectionOperation::BoundaryAdd`). Both
  switch to Edges.
  The table lives in code; the editable config waits for the Input Editor.
## 2026-11-04 — Tube: clicks add control points

- In the viewport each Tube click appends the next control point on the
  construction plane; a drag adds one and moves it; Undo takes back one
  click. Before, every click replaced the tube with a new two-point one.
- Provisional: Add mode only; Edit/Delete point modes still open (NPR-Q01).
  Test `tube_clicks_add_control_points`.

## 2026-11-05 — Create panel v2 (branch create-panel-v2)

- Tube: clicks add path points; press on a point to drag it.
- Unit Ellipsoid, Cone, Torus and Capsule (Ctrl/Shift-click their cells).
- Text tool (Create > Primitives): flat lettering from a bundled or system
  font, Location, Justification, Axis, Flip, Make UVs; editable until Drop.
- Create > Edit: Cut, Copy, Paste (Cmd+X/C/V) and Paste Tool.
- Create > Merge: Merge and Unmerge meshes; Scenes list Ctrl/Shift/Cmd-click
  selects several items.
- Create > Align: Quick Align (Shift+R) and Ground Align.
- Create > Measure: Ruler, Protractor, Dimensions (viewport overlays).
- Create > Draw: Curve, Bezier Curve, B-Spline (point editing sessions,
  smooth display, one undo step per click/drag), Sketch (stroke built as
  Curve/Face/Lines/Quads/Bezier/B-Spline), Draw Patch (4-curve quad grid),
  Pen button with Shift-click for the next polygon and the new Drop rules.
  Curves are polylines with a `curve` kind (native format; old files load).
- After QA: Torus, Capsule and Arc draw out (unit shape only via Unit);
  Pen Wall Mode (Both Sides/Inner/Outer strip, Offset, right-drag Offset).
- Guesses are listed in SPECIFICATION_QUESTIONS.md, "Create panel v2".
- Known: `acceptance_help_scroll_reaches_end` (640 px, tool.set) already
  fails on main a7b5b46; not touched here.

## Shader tree renders in Cycles (ST-4 first version, hand-off v0.2)

- Core `operandi-core/src/materials/shader_eval.rs`: folds the tree bottom to top for
  each polygon context (item, material tag, part tag) into one value per
  material channel, with the v0.2 measured rules: masked groups, Layer Mask
  hides the one layer above, texture-only groups start from below, Group
  Masks combine by their own blend, groups fold first then blend every
  channel, topmost shader wins. Constants fold to numbers; image maps stay a
  small node program. Tests reproduce each measured case in §8.
- `operandi-cycles`: one render mesh per distinct surface; each gets a
  principled shader built from that program (Mix Color nodes for blend modes,
  Image Texture on UVs), plus the Shader layer's ray visibility, Cast Shadows
  and Shadow Catcher. C ABI grew (`OcSurface`, `OcExpr`, UVs): rerun
  scripts/build-cycles.sh.
- Older files: renders map their material table into the default tree, and
  the first tree edit or Set Material keeps that mapping. Choices and gaps:
  SPECIFICATION_QUESTIONS.md ST-Q05..Q12.
- Image Map layers pick their picture in Properties (Image: the scene's
  pictures or Load Image…), one undo step with the picture.

## 2026-11-06 — Bump, normal maps, displacement (G-07, branch bump-normal-disp)

- Core: Bump, Displacement and Normal texture layers fold into three new
  channels (Material layers leave them alone, ST-Q23). `Principled` gains
  `bump` (height × Bump Amplitude), `normal_map` (dropped on meshes without
  UVs) and `displacement` (clamp(fold, ±1) × Displacement Distance), all
  `None` when they change nothing, so old renders and the clay test stay.
- Cycles: one Bump node (Distance 1, Strength 1) fed by the folded height,
  a tangent-space Normal Map node into its Normal, the result into the
  principled Normal (and Coat Normal by Clearcoat Bump). Displacement is true
  displacement on linearly subdivided faces diced at 1 pixel (ST-Q21).
  C ABI changed: rerun scripts/build-cycles.sh.
- Provisional: ST-Q21..Q26. QA: GUI_QA_HANDOFF ST-B01..B07, fixtures
  uv-plane.obj and relief-stripes*.png.

## 2026-11-06 — Material brightness (branch material-brightness)

From the research hand-off "shader settings -> Cycles translation map
v0.1" (fixes 1-4). Native version 17; Cycles C ABI v2 (rerun
build-cycles.sh).
- Specular: IOR 1.5, Specular IOR Level = Specular Amount (100 % = F0
  0.08); the old curve no longer drives renders (glTF export and pre-v15
  file reading still use it, ST-Q29).
- Base Color = Diffuse Color; Physically Based multiplies by Diffuse Amount.
- Transparency: Mix Shader with a Transparent BSDF (index 1.0) or Glass BSDF
  in Transparent Color; no Principled Transmission; Dissolve as an outer mix.
- Match Specular (default on): Reflection channels do nothing; off keeps the
  old Metallic/Coat split (provisional, ST-Q27).
- Shading Model dropdown with four models; Traditional and Energy
  Conserving render as Principled (provisional, ST-Q28).
- Calibration render test (40 m plane, sun 3 / grey sky): all 11 reference
  values within 0.4 %. GUI checks: GUI_QA_HANDOFF_M1_M2_M3.md MB-01..07.
- Also fixed main's Cycles build: the OcLight size check still said 108
  bytes after Visible to Reflection/Refraction made it 116.

## 2026-11-06 — Shader tree Tier 1 (branch shader-tier1)

- G-01 Selection Set groups mask polygons in the named set (AND item mask);
  overlapping set groups both apply in tree order; Cycles splits render
  meshes by set, subdivided pieces included. Part/Selection Set tags pick
  from the scene's names in Properties.
- G-02 Reflection Fresnel channel (0 %) + texture effect; Cycles splits
  Reflection Amount between Metallic and a Fresnel coat (provisional).
- G-03 Base Material and Set Material start at the reference values;
  Specular is an amount on a provisional curve (50 % = F0 0.04, ST-Q05).
  Native version 15; older files convert Specular so they look the same.
- G-04 two-zone block in core only, no layer uses it yet (ST-Q13).
- G-05 Image Map adjustments (Low/High, invert, gamma, brightness, contrast,
  clamp, swizzle, alpha mode, filtering) in eval and Cycles (new Separate
  Color / Gamma ops, per-image interpolation: rerun build-cycles.sh).
  Antialiasing stored only.
- G-06 Subtract/Dodge/Burn clamp before the opacity lerp; Soft Light and
  Divide use the v0.4 formulas, built from mixes and Gamma for Cycles.
- G-20 Add Layer starts: Constant black / 100 %, Group Scope All, Material
  without bump/displacement, texture axis X, Shader (nothing selected) at
  the very top.
- G-19 layers above the winning shader don't surface it.

## Shader tree skeleton (ST-1, specifications/shader-tree-2026-11-05)

First slice of the shader tree: the data model and the panel. Nothing renders
from it yet.

- Core `operandi-core/src/materials/shader_tree.rs`: Render Output, Shader, Material,
  Material Group, Folder and texture layers (Constant, Image Map) with the
  Layer block and the hand-off's channels; ordering and parenting; the default
  tree (Alpha Output, Final Color Output, Base Shader, Base Material). Every
  edit is `Command::ShaderTree` and one undo step (an opacity or value drag
  folds into one step).
- v0.1 structure: item masks can name an item or (later) a group of items,
  tags can be "(all)", "(none)" or a name, and textures can hold mask
  textures, which can have masks of their own. Changing the tag type resets
  the tag to "(none)".
- Saved in the scene file only when it differs from the default; native
  version 14. Opening an older file doesn't add its tags to the tree yet
  (that would make an untouched file look edited); the next Set Material on
  a tag adds that tag's group. `Scene::map_materials_into_default_tree` is
  ready for an explicit "show existing materials" step (ST-2).
- Set Material adds the tag's Material Group with a Material inside, and
  copies colour, Diffuse, Specular and Smoothing into it, in the same undo
  step. The material table is still what the viewport and Cycles use.
- Shading tab next to Scenes: View (Materials is a stub showing the whole
  tree), Assign Material (opens Set Material), Filter, Add Layer (Folder,
  Group, Shader, Render Output, Material, Image Map, Constant); columns eye,
  Name, B, %, Effect; select (Ctrl/Cmd, Shift range), double-click rename,
  eye click and drag-sweep, blend menu, % drag, effect menu, drag to reorder
  or into a group, Delete/Backspace, right-click Rename/Delete. Properties
  shows the selected layer's channels.
- Provisional or not built: Library, Nodes, Bake Items, FX are empty rows;
  Lights lists the scene's lights by name; Environments is empty; no Library
  drops yet, so the masked-group guard (Q-44) has nothing to guard; no Texture
  Locators, layer-mask drops, Ctrl+G, gang editing, Select Polygons or image
  picking (ST-2/ST-3); compiling to Cycles came with ST-4 (above). Row icons are one-letter tags until icons are drawn.
- Properties follow-ups (Jason, 2026-11-05): every Properties label now sits
  right-aligned in the shared label column (`option_row`), as Tool
  Properties already did; Edge Bevel/Chamfer and Falloff rows were moved onto
  it too. Properties has vertical category tabs on its right edge, only for
  categories that exist: Mesh and Surface (smoothing) for meshes; Camera,
  Light, Backdrop for those items; one tab named for a shader layer's kind.

## 2026-11-06 — Viewport readout (branch overlay-readout)

- 3D Information Overlays now sit at the lower right (the lower left overlapped the axis gizmo and view labels); Current Selection, OpenGL Polygon Count and Grid Size are on by default (provisional, Jason 2026-11-06), with a one-time upgrade of old all-off preferences files; selection reads "X Polygons", or "All Polygons" when nothing is selected, in the selection colour (Jason 2026-11-06: totals dropped as unwieldy on big meshes).

## 2026-11-06 — Edge loop double-click (branch edge-loop-dblclick)

- Edge mode: double-click an edge selects it plus its edge loop; Shift+double-click adds the loop to the selection (kept edges first). The first click is its usual Selection undo step, the loop one "Select Loop" step (`Command::SelectEdgeLoop`, `Scene::select_edge_loop`). Where Loop can't walk (poles, lone triangles), only the clicked edge stays selected. Double-click timing and distance follow egui's click options (0.3 s, 6 px). Vertex, Polygon and Item modes are unchanged. GUI check PS-06.
- Edge Loop with several selected edges (Jason, 2026-11-06): each seed's loop is walked and joined, existing edges first and new ones after; a seed that can't walk (pole, corner, ambiguous point) stays selected as it is. Refused only when no seed adds anything. One-seed Loop is unchanged.

## 2026-11-06 — Drop Action (branch drop-action)

- Works: top-bar "Drop Action" dropdown (12 entries, saved choice, not an undo step) and the viewport drag in Item mode with no tool: press an item, release on another; target pre-highlights; Esc, empty space or the source itself cancel. Parent, Parent in Place, Match ×4, Insert ×3, Place, Place and Align, each one undo step named after the action; MCP `item.drop`. Core tests DA-T1..T9 use the spec's measured numbers.
- Provisional: DA-Q1..Q6 spec picks; DA-Q7..Q12 (cage raycast, shear refusal, click semantics, multi-source Place, all-or-nothing multi drops, no icon). Not built: hover popup, Items-list drops, presets (spec "later").

## 2026-11-06 — Light properties Tier 1 (branch light-properties)

- Works: light tab named after the kind (LP-01) with rows in LP-23 order and the per-kind intensity label (LP-12); Render Default/Yes/No (LP-10), Dissolve × strength in Cycles (LP-11), Visible to Camera / Reflection / Refraction mapped to Cycles ray visibility (LP-19..21), Light Material section with Color, Affect Diffuse / Specular switches (LM-12/13, 0 or 100 %) and Set Temperature (LM-16, Kelvin → RGB once). Every row is one undo step named after it and applies to all selected lights.
- Portal light kind (LP-PO): Item > Create Light / Add Item > Light > Portal; rows Render, Shape, Width, Height, Environment Only; area-style outline and size handles in the viewport; Cycles area-light portal plus world background light. Native file version 16.
- Shared Display and Assembly tabs on every item kind: Visible (Default/Yes/No/No (Hide Children), meshes leave the view and picking), Label + Show Label + Label Offset drawn in 3D views, Select (No = not picked in Item mode), Lock (transforms refused, no handles, transform fields greyed). One named undo step per change, all selected items, saved in the scene.
- Provisional: LPROP-Q01..Q09. Not built (later tiers): Light Linking, Targeting rows, Samples/Importance/LPE, Volumetrics, other light kinds, User Channels and Tags tabs.

## 2026-11-06 — Light properties Tier 2 (branch light-properties-t2)

- Targeting (LP-04): Set Target / Remove Target / Enable / Roll on Directional, Spot, Area, Cylinder, Photometric, Portal and the camera. The aim is applied in `Scene::world_matrix`, so the viewport, handles and renders follow when either item moves; rotation fields grey while driven. Saved; one undo step each.
- Cylinder Light (LP-CY-01, 0.2 m × 1 m along local X), Mesh Light (LP-MS-01/02, Prototype + Use Prototype Transform; the prototype glows and is hidden in renders), Photometric Light (LP-PH-01..05: IES file via Browse..., loading sets Radiant Intensity and Width/Height, Cone 360°/Soft Edge 0°, viewport web from the file, Cycles IES node). Cylinder and mesh lights render as glowing surfaces.
- Physical Sun on Directional (LP-SUN-01..06, 08..12): NOAA solar position aims the light; Location list, Clamp Intensity None/Clamp/Replace; rows grey when off.
- Not built: Light Linking (no item groups in Operandi), Gamma and Store Location (Tier 3), cylinder viewport handles, .ldt files. Questions LPROP-Q10..Q18.

## 2026-11-06 — Light properties Tier 3, lights (branch light-properties-t3)

- Works: Volumetrics section on Directional/Point/Spot/Photometric (Cycles volume beam: cylinder, cone or sphere with Scattering Color, Density, Light Attenuation); LPE Label as Cycles light groups with a saved pass per label; Dome Light (even world light not seen by the camera); light material Enable, Opacity, Blend Mode and image layers (gobos projected through spot cones); Importance, Affect Subsurface and Physical Sun Gamma stored and shown; Store Location saves places to preferences.
- Provisional / not rendered: Light Shift, Importance, Affect Subsurface, Gamma (no Cycles meaning). Questions LPROP-Q19..Q27.
- Shared item tabs Tier 3 (same branch): Display Pivot/Center markers, Add/Remove Draw Options (camera/light glyph wire colour), Dimensions (bounding-box readout, meshes), Notes; Assembly Redirect, Pick Walk (arrow keys in Item mode), Tooltip, User Tag, per-item Drop Action Source/Destination overriding the top bar; User Channels and Tags tabs (stored, saved). Each row one named undo step over every selected item. Questions LPROP-Q28..Q37.

## 2026-11-06 — Item groups and Light Linking (branch item-groups)

- Item groups (no spec; GRP-Q01..Q08): Scene holds named ordered groups of any items; New Group from selection, Rename, Delete, Add/Remove Selected, select a group's members; a Groups tab beside Scenes/Shading and Item > New Group from Selection. Deleting items prunes them from groups. Native version 18 (renumbered from 17 at merge; the Material Shading Model has 17). Each change one undo step.
- Light Linking (LP-30..32) on every light kind: Control Light Linking, Item Group, Mode (Include/Exclude, default Exclude), greyed while off; Cycles light sets with matching shadow sets. Real renders test Include and Exclude.

## 2026-11-06 — Preferences v2 Tier 1 (branch preferences-panel)

Spec `specifications/preferences-2026-11-06`. Built (provisional defaults
keep today's behaviour; questions PREF-Q6..Q19):
- Remapping: Trackball Rotation (free orbit, camera roll) with per-view
  choice, override and speed; Numeric Enter as Tab (macOS keypad); Open
  Dialogs under Mouse (every modal).
- Selection: Closest rollovers; Point Selection Priority (also Tweak);
  Mouse Regions Trigger Selections; selection x-ray on the GPU; handle
  transparency.
- Display: Grid Visibility for new views, Flatness of Perspective, Item
  Drawing size (new scenes + current scene, saved with the scene),
  Backdrop fitting / uniform size / pixel size for new Backdrops.
- Tool Handles: Large/Small points and hit sizes, Small scale, Plane
  ratio, Unselected handles, Rotation Snap Angle, snapping ranges shown
  here too. Tool Settings: Last Selected.
- Application: Default Material, Default Texture Map, Item Index Style
  (items, shader layers, Backdrops), Default Scene, Use OS Color Picker.
- Work plane Preferred Plane / Bias / Lock / Offset, grid increments;
  rollover colours by polygon kind, falloff Cool/Warm; Reset Preferences
  (Edit menu) and Reset Panel on every page.
Left out: Allow Modifier Chording, Invert Drag Scrolling, Reset "In the
Future…" (PREF-Q6, Q18).

## 2026-11-06 — Preferences v2 Tier 2 (branch preferences-panel)

Built (questions PREF-Q20..Q27): Mesh Items page (display levels, auto UV
map, item masks, primitive polygon, symmetric primitives, copy/paste
rules, auto-hide children, defer smoothing); Max GL Polygon Count with
Keep/Undo and Bounding Box Threshold; Coordinate Rounding and Fixed
Increment; Color Unit System and Factor or Range; Up Axis; Input Device;
File I/O page (OBJ, STL, glTF, image format and encoding); Rendering page
(threads, block sleep, Preview Start Paused and samples); Snapping State
overlay; UI page (popup transparency, popover dismissal, icon size).
Not built: primitives' axes under Up Axis, JPEG sub-sampling.
## 2026-11-06 — Deform tab Tier 1B (branch deform-presets)

- Works: Deform toolbox tab (after Select). Soft Move, Shear, Twist, Swirl, Taper, Bulge start Move/Rotate/Scale with a Linear or Radial falloff fitted to the selection, and give back the user's own falloff on Drop/Cancel/switch (`Command::ActivateDeform`, core `deform.rs`). Twist/Swirl turn each point by weight × Angle (TW-Q1). Push (normals, Averaged/Selected) and Smooth (Strength, Iterations, Lock Boundary/Corner/Sharp Edges, Sharp Angle, Preserve Volume) are new tools with drag-anywhere and falloff support. Undo labels use the preset's name; mid-tool undo as for transforms. Quick/Ground Align row shared with Create. GUI checks DEF-G01..G11.
- Provisional: DF-Q3..Q8 (fit on activation, Item mode refusal, Smooth starts applied, Push drag, Redo after undo-close, unbuilt buttons and placeholder icons).
- Layout correction (2026-11-06): Deform tab now shows the full 18-button order with unbuilt tools greyed, flyouts for Soft Drag (Soft Move, Shear), Smooth, Soft Selection and Bulge (hold or right-click), Radial/Linear under Align and a Commands section (Relax Mesh..., greyed). GUI checks DEF-G01, DEF-G01b.
- Tweak renamed Element Move (Jason, 2026-11-06): label, shortcut action (T, saved "Tweak" bindings carry over), command ID `ElementMove` in the command line and MCP; `tweak` kept as a deprecated alias.

## 2026-11-06 — Resizable side panels (branch resizable-panels)

Both docked side panels resize by dragging their inner edge. The grab
reaches 4 points either side of the edge (was 2) and shows a left-right
resize cursor on hover, not only mid-drag; a window floating over the edge,
another drag in progress, an open menu or dialog, or View > Layout's Lock
Divider Dragging turn it off. Limits: toolbox 200–420 (was 200–320),
Properties/Items 240–640 (was 240–380); the viewport keeps at least 500
points and the right panel gives way first. Widths keep saving to
`layout.json` and reload clamped per panel (both were clamped to 200–380).
A floating toolbox or right panel no longer leaves a stale grab edge. The
toolbox already scaled its four-column grid and full-width rows to the
panel; its tab strip stays 22 points at either end. Tests: layout_tests.rs.

## 2026-11-07 — Textures, stage 5 (branch textures-stage5)

- Works: Gabor Noise (Cycles Gabor Texture) and Poisson Cells (low-
  randomness Voronoi, Level doubles the scale) with Brightness/Contrast in
  place of Bias/Gain; the viewport evaluates procedural layers at mesh
  corners instead of the flat average; a Texture Locator gizmo (axes plus
  the projection's shape at Size) when its layer or item is selected.
- Stored only (stage 6): Front, Light Probe, Implicit UV, Random Offsets,
  Real World Size + X/Y, Tangent, Particle Source, Texture Replicator,
  Projection Camera, World Transforms (saved, tested). No interactive
  Planar drag.
- Native version 23, Cycles ABI v8 (to renumber at merge). Provisional:
  PT-Q19..Q23, TL-Q20..Q21. GUI checks: TEXTURES5_GUI_QA.md.

## 2026-11-07 — Procedurals, stage 4 (branch procedurals-stage4)

- Works: Cellular (Angular/Round, octaves), Wood, Ripples and Weave with the
  spec's starting values and zone names; per-cell Value/Hue/Saturation
  (and Alpha for Checker/Grid/Dots) variation on Checker, Grid, Dots and
  Cellular. Cycles: Voronoi, Noise, White Noise, Hue/Saturation/Value and
  Math nodes. Auto Size keeps a uniform Size uniform (largest side).
- Native version 22, Cycles ABI v7 (to renumber at merge). Provisional:
  PT-Q13..Q18, TL-Q19. GUI checks: PROCEDURALS4_GUI_QA.md.

## 2026-11-07 — Projections, stage 3 (branch projections-stage3)

- Works: Cylindrical, Spherical, Cubic (follows rotation, no mirroring), Box
  (world aligned, faces right way round), Triplanar (|n|^k, k = 1 + 31h²)
  for images, Constants and procedurals; Front Culling (Project Through,
  Cull, Flip, Flip U, Flip V) on Planar; Falloff Cubic/Spherical/Linear
  X/Y/Z into opacity. Option names from the full spec (old placeholders
  read). Full specs committed (c13c081d). Cycles renders match the core
  reference per pixel.
- Stored only: Front, Light Probe, Implicit UV (render as UV Map), Random
  Offsets + strength, Real World Size, Tangent, Particle Source, World
  Transforms.
- Native version 21, Cycles ABI v6 (to renumber at merge). Provisional:
  TL-Q12..Q19. GUI checks: PROJECTIONS_GUI_QA.md.

## 2026-11-07 — Procedural textures, stage 2 (branch procedurals-stage2)

- Works: Add Layer > Textures adds Checker, Grid (Square/Line/Triangle/
  Hexagon/Cube), Dots (Square/Hexagon/Cube) and Noise (Fractal/Simple/
  Turbulence, Smooth Tops, Flow Noise) as texture layers with their own
  Solid locator, below Base Shader. Shared two-zone block (Value/Color/
  Alpha per zone, Bias, Gain); zone alpha fades the layer; Blend/Opacity/
  Invert as for Constant. Solid, Planar (depth 0) and UV Map (repeat only)
  feed the pattern. Cycles builds every pattern from stock Math and Noise
  Texture nodes (GPU-safe); renders match the core reference per pixel.
- Stored only: Value/Hue/Saturation/Alpha Variation. Viewport shows the
  zones' average. Cellular, Wood, Ripples, Weave listed as coming.
- Native version 20, Cycles ABI v5 (new ops; both to renumber at merge).
  Tests: procedural.rs, Cycles pattern render tests, shading_panel_tests.rs.
  GUI checks: PROCEDURALS_GUI_QA.md. Provisional: PT-Q01..Q12.

## 2026-11-07 — Texture Locators, stage 1 (branch texture-locators)

- Works: every texture layer gets a Texture Locator item (in one "Texture
  Group", named "<layer> (Texture)"); Locator setting on the layer (shared
  locators allowed; an unused one is deleted with its last layer). Texture
  Locator sub-tab with transform, Auto Size and every setting; expander row
  in the Shader Tree. Solid, Planar (X/Y/Z) and UV Map (repeat counts, UV
  Rotation, named map) render in Cycles with Repeat/Reset/Mirror/Edge
  tiling per axis; World Coordinates on/off. Solid hands stage 2 a 3D
  locator point. Each locator edit is one undo step; native version 19;
  older scenes get UV Map locators and look the same.
- Stored only (render later): Cylindrical, Spherical, Cubic, Box,
  Triplanar, Front, Light Probe, Implicit UV (images render as UV Map);
  World Transforms, Front Culling, Triplanar Hardness, Random Offsets,
  Falloff, Tangent, Real World Size, Particle Source. No viewport gizmo.
- Cycles ABI v4 (OcExpr.xform; OcSurface image vectors/extensions).
  Tests: texture_locator.rs, Cycles op-list and render tests,
  shading_panel_tests.rs. GUI checks: TEXTURE_LOCATORS_GUI_QA.md.
- Provisional: TL-Q01..Q11.

## 2026-11-07 — Pie menus, stage 1 (branch pie-menus)

- Works: pies are data (name + up to 8 entries, empty placeholders keep
  their slot), bound through the input map (searchable, conflict-checked).
  Opens on key-down at the pointer (nudged on screen, closes other popups);
  hold-flick-release runs the wedge's command, a tap stays open for a
  click, Esc / click elsewhere closes. 20 px dead zone, 45° wedges, arrow
  toward the highlighted slot, toggle boxes and choice dots, disabled
  entries greyed. Uses Popup Transparency. Slots run `execute` like the
  shortcut, with that command's own undo; the pie is never an undo step.
- Factory pies: View Direction (Ctrl+Space), Selection & Transforms
  (Alt+Q), Shading Style (Ctrl+2), Viewport Toggles (Ctrl+1); Falloffs,
  Action Centers, Modeling unbound. Ctrl+Space binds now (plain Space stays
  reserved). Tests: pie_menu_tests.rs. GUI checks: PIE_MENUS_GUI_QA.md.
- Provisional: PM-Q1..Q11.

## 2026-11-07 — Shader tree remaining, stage 1 (branch claude/project-thread-w6o7ya)

- Works (renders): Dissolve (1 − v, linear), Stencil (above 50 % removes
  the whole surface), RGBA (Diffuse Color from the image, cut where alpha
  < 50 %); cut parts are gone for every ray. Shader: all control switches
  (a switched-off section comes from the shader below), visibility rows,
  Alpha Type Constant (holdout) and Shadow Catcher, Light Linking by item
  group (Include / Exclude), Linear / Exponential / Layered / Underwater
  fog, Direct Illum Multiplier, Receive Shadows off. Every other shader
  setting is stored. Shading panel shows the Shader by section, greying
  rows a switch or Fog Type turns off.
- Native version 21 when a shader uses the new settings; Cycles ABI v6
  (fog, holdout, light shadow membership).
- Provisional: SR-Q01..Q09 (indirect multipliers stored only, light copies
  for Direct / Receive Shadows, fog camera rays only, no viewport preview).
  Tests: operandi-core/tests/shader_settings.rs,
  operandi-cycles/src/shader_settings_tests.rs. GUI checks:
  SHADER_SETTINGS_GUI_QA.md.

## 2026-11-07 — Shader tree remaining, stage 2: Gradient + Drivers (branch shader-remaining-2)

- Works (renders): Gradient layer (Add Layer > Processing) with Value and
  Color curves (Smooth / Linear / Stepped keys, ends hold), inputs
  Incidence, Slope, Texture Value, Distance to Camera / Locator, Texture
  U / V, Back Facing, Bump / Displacement Height, Driver A–D, Stencil and
  the basic channels. Driver A–D effects fold into hidden channels. Panel:
  grouped input menu, inline strip, key list.
- Native version 26 for these stages; Cycles ABI v10 (geometry, distance and
  ramp ops, numbered from 40). Provisional: GR-Q01..Q06. Tests:
  operandi-core/tests/gradient.rs, gradient.rs unit tests,
  operandi-cycles/src/gradient_tests.rs. GUI checks:
  SHADER_REMAINING_GUI_QA.md.

## 2026-11-07 — Shader tree remaining, stage 3: Vertex Map + Variation (branch shader-remaining-2)

- Works (renders): Vertex Map Texture (weight maps through the two-zone
  block, colour maps as they are, no map / mesh without it = no effect)
  and Variation Texture (Mesh Part islands, Item; curves as the
  Gradient's). Both under Add Layer > Processing; maps listed by name.
  Cycles gets RGBA corner attributes. Provisional: VM-Q01..Q05.
  Tests: operandi-core/tests/vertex_variation.rs,
  operandi-cycles/src/vertex_map_tests.rs.

## 2026-11-07 — Shader tree remaining, stage 4: Process + Occlusion (branch shader-remaining-2)

- Works (renders): Process layer (Bias, Gain, Hue, Hue Shift, Saturation,
  Value over the texture layer directly below, then its own blend /
  opacity / invert); Occlusion texture, Uniform and Thickness through
  Cycles' AO with rays, distance and Same Surface Only; other types stored
  and render as Uniform (HSV and AO ops). Provisional: PR-Q01..Q03.
  Tests: operandi-core/tests/process_occlusion.rs, process_occlusion.rs
  unit tests, operandi-cycles/src/process_tests.rs.

## 2026-11-07 — Shader tree remaining, stage 5: group Scale + Material extras (branch shader-remaining-2)

- Works: Material Group Scale block (UV, Size, Bump / Absorption /
  Displacement / Subsurface Distance switches, Scale, nested groups
  multiply) with per-layer Ignore Scale Group and an Apply Scale button
  (one undo step); Apply to Sub-Group on untagged groups. Material: Use
  Refractive Index, Subsurface Phase, Rounded Edge Width and Absorption
  Distance render; the other settings in the table are stored. Provisional: GX-Q01..Q04. Tests:
  operandi-core/tests/group_material_extras.rs,
  operandi-cycles/src/material_extras_tests.rs.

## 2026-11-07 — Shader tree remaining, stage 6: Render Outputs (branch shader-remaining-2)

- Works: 18 output types in Add Layer > Special > Render Output (Final
  Color, Alpha, Depth, normals, Diffuse Shading Total / Direct / Indirect,
  Specular, Reflection, Transparent, Luminous, Diffuse Color, Ambient
  Occlusion, Object Coordinates, Shading Incidence, plus empty Subsurface
  and Motion Vector) from Cycles passes and AOVs; per-output settings with
  image processing (exposure, levels, hue / saturation / colorize, tone
  map, bloom, vignette, gamma, output levels, clamp, unpremultiply). The
  render window picks the output to show; outputs save next to the render.
  No enabled output: the render refuses. Provisional: RO-Q01..Q08.
  Tests: operandi-core/tests/render_outputs.rs, render_outputs.rs unit
  tests, operandi-cycles/src/outputs.rs and outputs_render_tests.rs.

## 2026-11-07 — Colour field and picker; polygon tag modes (branch colour-nav-tag-modes)

- Works: one shared colour field (colour_field.rs) for every colour row:
  fill + R G B in the Color Unit System; drag a number, Shift/Ctrl/Alt for
  brightness/hue/saturation; click = Colour Picker popover (HSV wheel or
  square, Kelvin 1000–12000 K, RGB, axis, Stops, dynamic strip, RGB/RYB
  wheel, RGB (HDR)/RGB/Kelvin/HSV/Hex fields, roll-off and Esc close);
  right-click Copy /
  Paste / Reset. One drag or picker press = one undo step across frames
  and selected layers/lights (`Editor::undo_fold`). Swatch drags carry to
  other selected shader layers and lights (relative for modifier drags).
  New preference: Display Color Values in Display Space (off).
- 2026-11-08 (branch colour-swatch-click, CN-Q02 answered): a click on a
  swatch number opens the picker like the rest of the swatch, at once on
  release (press and release within 3 points); the double-click inline
  edit and its 0.25 s wait are gone.
- Works: Materials (polygon tag) mode after Vertices/Edges/Polygons, key 4,
  tag type popup (Material, Part, Selection Set; Smoothing Group greyed),
  Select menu entries. Clicks and lasso select whole regions; Shift/Ctrl
  add/remove; outline drawn instead of filled polygons; Material regions
  select their Shader Tree group; Alt+click converts the selection.
- Provisional: CN-Q01..Q05, PT-Q3..Q5 (SPECIFICATION_QUESTIONS.md).
  Harmony rules and presets are listed, greyed (B6 scaffold).
  Tests: colour_math.rs, colour_field.rs, colour_picker.rs,
  value_display.rs, tag_selection.rs, tag_mode_ui.rs, tests/undo_fold.rs.
  GUI checks: COLOUR_NAV_TAG_MODES_GUI_QA.md.

## 2026-11-07 — Image Viewer, stage 1 (branch image-viewer)

- Works: the Render window is the Image Viewer. It keeps renders as linear
  floats (it already did; 8-bit only for display). History strip, newest
  first, numbered 1–10 for keys 1–9/0; click, Ctrl/Cmd+click; hover x and
  lock; right-click Save Image... / Lock / Remove / Show in Finder;
  tooltip scene + time; T hides it. Every render is cached as float EXR per
  output with a thumbnail and an index in Recent Frames and survives
  quitting; Max Render Slots (10) trims the oldest unlocked render
  (oldest locked when all are). Renders no longer auto-save to Pictures
  (new preference, off; MCP renders still write their file). Render > Open
  Render Window (Alt+Shift+F9). Zoom field, wheel / Cmd+Alt-drag zoom,
  middle-drag pan, Region toggle, Cmd+A fit, pixel readout. Header: Output,
  Pass and Stereo (greyed), Display sRGB / Linear (raw), Zoom, Channels,
  Show Clipping. Statistics tab (live and per slot). Save Image... in nine
  formats; Load Image.... Space hides everything but the picture, P pauses.
  Window place, panel widths, tab and hidden panels persist.
- Provisional: IV-Q10..Q18 (SPECIFICATION_QUESTIONS.md). Image Processing
  and Compare tabs are placeholders; the lock mark is drawn, not a
  generated icon yet. No scene-file or Cycles ABI change.
  Tests: image_viewer.rs, image_viewer_ui.rs, render_ui.rs.
  GUI checks: GUI_QA_HANDOFF_M1_M2_M3.md, IV-G01..G16.

## Navigation, lock and hide — 2026-11-07 (branch nav-lock-hide)

- **Navigation:** Alt+middle roll, Alt+right look around (the eye stays
  put and the view turns; was a flick orbit until 2026-11-22), Shift+Alt+right up/down pan, Ctrl/Cmd+Alt+left zoom about
  the press point, Ctrl/Cmd+Alt+right box zoom. Keys `.` `,` zoom steps, `/`
  turntable, `Shift+/` upright, `Cmd+A` align, `Cmd+Shift+A` fit and align.
  View > Navigate lists them. Re-centre is a View menu command with no key
  (G stays Edge Slice). Operandi preset only; Blender/Maya presets unchanged.
- **Item lock:** Items list right-click Lock/Unlock (all selected items, one
  undo step), padlock after the name (a drawn placeholder until the icon
  generator has one), eye toggle and parenting now refused while locked.
- **Geometry lock:** J / Shift+J / Cmd+J / I and Edit > Lock…; locked
  elements leave and can't join the selection, never move, draw in the theme
  colour "Locked mesh elements"; View > Show Locked Mesh Items. Saved (native
  version 28).
- **Hide:** H / Shift+H / Ctrl+H (physical Control on Mac) / U and View >
  Hide…; hidden geometry isn't drawn, picked or edited; not saved.
- Provisional readings NLH-Q01..Q11. No UV viewport yet (LCK-27, HID-06).

## 2026-11-07 — Presets, Stage 0 (branch presets-stage0)

- Works: File > Load Preset…, Save Item / Assembly / Environment Preset…;
  Item List right-click Presets > Save… / Save Assembly…; Shading panel
  right-click on a material or material group Presets > Save… (and Load…
  on a group); right-click Render > Save Preset… (render settings).
  Commands `preset.save kind:… [path:…]`, `preset.load [path:…]`; MCP
  `preset_save` / `preset_load`. Opening or dropping a `.oppreset` loads it.
- Format: a cut-down native scene with a `preset` header (kind, empty
  thumbnail slot, PB-52); same versions as scenes, no version bump. Plain
  Open refuses presets as scenes and Load Preset refuses scenes. Default
  folder: Presets in the data folder, made on first save.
- Each load is one undo step; saving changes nothing. Material presets
  carry texture layers, their locators and images; mesh presets drop the
  item transform and carry their tag material groups.
- Skipped: PB-43 profile presets (no user profile curves yet); thumbnails
  (slot only). Provisional: PB-Q4..Q10. Tests: operandi-core
  src/preset/tests.rs, operandi-app preset_ui_tests.rs, operandi-mcp
  tests/bridge.rs.

## 2026-11-07 — Corner Pan/Rotate/Zoom drag feel (branch pan-zoom-feel)

- Works: the corner drag buttons read the mouse's own movement, so the
  pointer being put back on the icon no longer adds or drops part of a
  move (no drift on the other axis); the first frame counts from the
  press; the pointer returns where it was pressed. Rates unchanged. On
  macOS the pointer is locked in place for the drag instead of put back
  (VBF-2: the put-back cancelled the movement and the view shook).
- Provisional: VBF-Q1 (raw motion units on Windows/Linux). Tablet drags
  keep reading the pointer. Tests: viewport_buttons.rs (five new).


## 2026-11-08 — Tool list / Tool Properties divider (branch tool-props-data-2)

- The left panel is split: tool list on top, Tool Properties below, each
  scrolling on its own, with a draggable divider (`tool_split.rs`). The
  split is a fraction saved in layout.json (`tool_split`).
- Activating a tool raises Tool Properties to at least 50 %
  (`TAKE_OVER_FRACTION`); more is kept, dropping a tool doesn't move it.
- Before this, Tool Properties sat at the end of one scrolling column, so
  its share depended on how long the tab's tool list was (Select's is long)
  plus a Polygon-only command cap; that cap is gone. Instant, no animation.

## 2026-11-08 — Every tool's Tool Properties from field lists (branch tool-props-data-2)

- Works: every tool kind declares its panel as a field list and the one
  shared renderer and confirm route serve them (`panels.rs` `ToolPanel`
  picks it; live tools in `live_fields.rs`). New shared row kinds: unbuilt
  dropdown entries, panel-only name drafts, action dropdowns, lifted
  sub-forms and a hook for the profile browser.
- Removed: the per-tool drawing and confirm code (generic live confirm,
  Bridge/Pen/Smooth Shift confirms, the old triple/vector row helpers,
  `unmeasured.rs`). A test checks every tool kind declares and draws a
  field list and that none of the old helpers return.
- Fixed: Push and Smooth numbers now reach the tool (FORM-Q20).
- Provisional: FORM-Q18..Q27. GUI pass: `docs/TOOL_PROPS_DATA_GUI_QA.md`.

## 2026-11-08 — Pick Other Meshes in Component Modes (branch active-mesh-pick)

- Works: new Selection preference, off by default (Jason). Off, Vertex/Edge/
  Polygon picks (click, paint, rollovers, Closest, Mouse Regions, Tweak,
  Poly Haul, edge-loop double-click, Selection Fill) see only
  `Scene::active_meshes()`; other meshes neither pick nor hide (rays and
  point/edge visibility pass through them), and a click on another mesh
  keeps the selection. No shown active mesh: picks any mesh. Item mode and
  the lasso (already active-mesh only) unchanged. On: old behaviour.
- Drawing still treats other meshes as solid (hidden points don't draw).
  Tests: active_mesh_pick_tests.rs. GUI: docs/ACTIVE_MESH_PICK_GUI_QA.md.

## 2026-11-09 — Gradient inputs: Texture U/V and Screen X/Y/R (branch gradient-inputs)

- Fixed: renders read only a polygon's legacy UVs when no map was named, so
  every primitive's "Texture" map (and any image on it without a named
  map) read 0. Now: legacy UVs, else the first map by name, as the
  viewport does. Texture U/V also follow the layer's UV Map projection's
  named map; a mesh without UVs reads 0 and the render status says so.
- Built: Screen X/Y/R from Cycles' Window coordinates (ABI v11); the
  viewport stays flat. Provisional: GR-Q04, GR-Q07, GR-Q08. Tests in
  operandi-cycles gradient_tests.rs; GUI steps 10–11 in
  docs/SHADER_REMAINING_GUI_QA.md.

## 2026-11-09 — Item editor colours (branch item-editor-colors)

- Works: Editor Color on items and Shader Tree layers (note 10): right-click
  > Editor Color in the Items list, Groups tab member rows and Shader Tree
  (17 named colours plus (none); several selected = one undo step). Rows
  show a 3 pt bar at the Name column's left edge plus a 15 % tint; the bar
  stays on selected rows. Items list Filter and Select popovers: By Color +
  colour dropdown (filter keeps greyed parents). `item.editorColor
  [color:<name>]` sets or reports. Colours are themable ("Item Editor
  Colors" in Preferences > Colors).
- Stored by name; native version 31. No render or viewport effect.
- Provisional: IC-Q1..Q4. Tests: operandi-core/tests/editor_color.rs
  (IC-01..04), editor_color_ui.rs. GUI checks: ITEM_EDITOR_COLORS_GUI_QA.md.

## 2026-11-09 — Channel light and Reset to Default (branch channel-dot)

- Works (CH-Q1, Jason 2026-11-09): the channel status light (UIC-112) is a
  grey ring at the default and a filled accent dot once changed (themable:
  Interface > Channel Light (Default/Changed)). Right-click a channel row's
  label or light > Reset to Default (greyed at the default); the change is
  one ordinary edit, so one undo step. Every row drawn with a default
  (`check_d`, `number_d`, `choice_d`, `text_d`, or `status` +
  `take_reset`) gets it.
- New: Shader Tree layer Properties (materials, shaders, constant / image
  map textures, group switches) show lights against the layer's starting
  values (`NodeKind::defaults`; Base Material against the full material
  defaults) and reset through them.
- Not yet: rows that don't declare a default (most light, camera and
  procedural texture rows) show no light (CH-Q2).

## 2026-11-09 — Planar texture placement (branch planar-drag)

- Per the Planar spec (Jason: "just do the modo way"): no dedicated drag;
  the ordinary Move/Rotate/Scale/Transform tools place a Texture Locator,
  and in Items mode selecting a texture layer in the Shading panel selects
  its locator, so a live or later item tool acts on it (PD-Q02). One undo step per drag, as for any item.
- The plane draws when the locator or its layer is selected: outline in
  the selected-item colour, projection-axis line (25 % of the smaller
  side), up tick on the +V edge (PD-Q13). World Coordinates off draws it
  through the surface item (PD-Q07, fixes TL-Q20's offset).
- Size clamps at 0.001 m on every path (PD-Q14).
- Typed: `transform.channel name:scl.X value:2` (map name; also pos.*,
  rot.*), `select.item item:<id>` (`item.select` aliased). UIC-160..165,
  PD-Q12. No macro recorder yet.
- Not yet: tool handles in render space for World Coordinates off
  (PD-Q08). GUI checks: PLANAR_DRAG_GUI_QA.md.

## 2026-11-09 — Environments (branch environments)

- Works (spec environments-2026-11-09): Environment items with their
  Environment Material; every new scene has one ("Environment" /
  "Environment Material", 4 Color Gradient). Constant, 2 Color, 4 Color
  (hard horizon) and Overcast exactly as §4, × Intensity; each ray kind
  sees the newest environment visible to it (§6). Cycles renders them;
  the Preview and Render use the scene's environments.
- Texture layers under an environment: Constant and Image Map (lat-long,
  Spherical Y; the locator's rotation turns it; HDR kept
  above 1) render; procedurals and gradients are stored only (ENV-Q-OP7).
- Physically-based Daylight with a Sun Light renders Cycles' sky; without
  one it draws its gradient. Fog is stored and shown only.
- Shader Tree Environments section, Properties (§7), Add Layer (§8), Item >
  Create Environment, rename / duplicate / delete; every row runs
  item.channel (or item.name / environment.light); UIC-164 has no new gaps.
- Viewport Background = Environment draws it (gradients exact; physical
  and image layers approximate).
- Native version 33 (older files with a camera or light get the default
  environment); Cycles ABI v15.
- Provisional: ENV-Q-OP1..OP8. GUI checks: ENVIRONMENTS_GUI_QA.md.
- Placement (ENVIRONMENT-PLACEMENT, branch env-not-in-items): environments
  are gone from the Item List (rows, filters, Select patterns, parked
  scenes, Parent To, solo, Item/Assembly presets) and have no transform:
  no Transform or Rotation Order in Properties, transform edits refused,
  renders use world axes; old files drop a stored transform on load
  (ENV-Q-OP9). Shader Tree rows: Environment icon, an eye (hidden = no ray
  sees it), Shift/Cmd selection, Rename / Editor Color / Properties |
  Duplicate / Delete | Presets (greyed). Open: ENV-Q-OP10..OP12.

## 2026-11-09 — 3D mouse stage 1 (branch spacemouse-1)

- Works: a 3Dconnexion SpaceMouse moves the view under the pointer (it
  becomes the active view, UIC-70). Twist/tilt orbit about the orbit centre
  (Orbit Selected honoured), sideways push and lift pan, push/pull zoom.
  Rates are per second, so the motion doesn't depend on frame rate; a stalled
  frame counts as 0.1 s at most. Only the focused app reacts.
- Input, behind a small `Source` trait (`space_mouse/source.rs`): on macOS
  with 3DxWare installed, its driver keeps the device exclusively (measured:
  raw HID open fails with "exclusive access"), so Operandi loads the
  driver's client framework at run time and registers as a client
  (`framework.rs`, our own declarations, nothing bundled, no SDK files).
  Otherwise raw HID via hidapi on a background thread: finds the device,
  reopens it after an unplug, quiet when there's none. macOS and Windows;
  Linux comes later (needs libudev).
- Preferences > Input > 3D Mouse: Device status, Enabled, Speed, Dead Zone,
  Motions (all / rotate only / pan and zoom only), Roll with Sideways Tilt,
  and six Invert switches. All `pref.value spaceMouse.<field>` (UIC-164),
  saved in `space-mouse-preferences.json`.
- Buttons are decoded but not bound (stage 2). No new named commands.
- Provisional: SM-Q01..Q06. GUI checks: SPACEMOUSE_GUI_QA.md.

## 2026-11-09 — 3D mouse stage 2: button shortcuts (branch spacemouse-1)

- Works: Preferences > Input > 3D Mouse > Buttons has one row per button
  (at least 2; more as the device shows them, up to 32). Click a row and
  press a shortcut (Esc cancels, Clear empties it), or press a puck button
  to pick its row. Default: none.
- A puck press goes in as that key at the start of the next frame, so the
  normal key dispatch runs it: same action, the hovered view (UIC-70),
  nothing while a dialog is open. Held buttons don't repeat. Both sources
  (3DxWare and raw HID). No new commands.
- `pref.value spaceMouse.buttons.button<n> <shortcut>`; unreadable
  shortcuts are refused. Empty string values now go through `pref.value`
  braced (`{}`), which clears.
- Provisional: SM-Q07.

## 2026-11-10 — Delete an edge loop (branch claude/project-thread-byrc95)

- Delete / Backspace with the pointer over a viewport remove the selection:
  run the name-map `delete` command: a point takes its polygons (one hole
  edged by quads), polygons go with their orphaned points, edges run Edge
  Remove, Item mode deletes items.
  Items list focus keeps item deletion.
- Edge Remove takes out leftover points (unused, or between two interior
  edges): removing a sphere's latitude or pole-to-pole loop leaves closed
  quads/triangles with no stray points. Provisional (ER-Q01, ER-Q02).
- Tests: `operandi-core/tests/edge_loop_delete.rs` (latitude, pole-to-pole,
  next-to-pole loops; undo/redo; save/reopen) and
  `operandi-app/src/edges/edge_loop_delete_tests.rs` (both keys in every mode;
  off-viewport changes nothing).

## 2026-11-10 — Delete vs Backspace (branch delete-backspace)

- Forward Delete over a viewport runs `delete` (elements plus the points
  left unused); Backspace runs Edge Remove / Remove on edges and polygons,
  keeping their points. Points: both keys delete them with their polygons.
- Edge Remove (`edge.remove`, Shift+Backspace) keeps points again
  (CMD-E-01); the leftover cleanup lives in `edge_delete_geometry`.

## 2026-11-10 — Polygon double-click selects connected (branch polygon-double-click-connected)

- Polygon mode: double-click a polygon selects every polygon connected to it (the piece `select.connect` walks, by shared points, not position); Shift+double-click adds that piece and doesn't grow other selected pieces. First click is its usual Selection step, the piece one "Select Connected" step (`Command::SelectConnectedPolygons`, `Scene::select_connected_polygons`). Works on any active mesh. Jason's request, not from a spec; Vertex mode unchanged. A press that turned into a paint drag no longer counts as a first click (edges too). GUI check PS-06c.
- Tests: `operandi-core/tests/polygon_connected_double_click.rs`, `multi_active_meshes.rs::polygon_connected_double_click_on_another_mesh`, `selection_ui_tests::polygon_double_click_selects_the_connected_piece`.

## 2026-11-10 — Edge Bevel Shape and Depth (branch bevel-edge-shape)

- Edge Shape and Depth did nothing at the default Round Level 0 (Edge Chamfer: Segments 1). Now, per EBC-Q12 v1 (measured): Square and Sharp keep the original edge as a corner point at every Level, Level 0 included, and ignore Level/Segments and Depth (EBS-03); Depth blends Round's middle points linearly from the flat strip to the arc and does nothing at Level 0 (EBS-01/02). Rows that don't apply grey in place (UIC-92/117): Round Level/Segments with Square or Sharp, Depth with Square/Sharp or at Level 0. Edge Chamfer now offers Sharp (measured the same as Square on a 90° edge). Open: Square vs Sharp on angled edges (EBS-05). GUI check BLQ-16.
- Test: `edge_tab_tests::edge_bevel_shape_and_depth_follow_ebc_q12`.

## 2026-11-11 — Boolean, stage 1 (branch booleans; spec booleans/BOOLEANS.md)

- Boolean... (Edit tab, "Booleans & Drill"; Geometry menu; `poly.boolean mode cutmesh item children`, bare opens the dialog; MCP/headless too). Union, Intersect, Subtract, Add of the active mesh with the drivers: Background (visible unselected meshes), Last Selected (provisional, BOOL-Q1) or a Specific Surface Item / group, Include Children. One undo step "Boolean <op>"; drivers untouched; result in the target's own space; nothing selected afterwards.
- Engine pipeline (stress-test plan): pre-flight on every input (closed, manifold, consistent, not inside out, finite, no self-intersection; zero-area polygons left out), inputs recentred on the origin, Manifold, a cheap result check, then a boolmesh cross-check that refuses the result when the engines disagree. About 2.5× the time of Manifold alone, by Jason's choice.
- Result polygons: untouched faces come back unchanged; cut faces become one n-gon per piece; tags, material, part, smoothing and polygon sets follow the source polygon (Subtract caps take the driver's); UV maps interpolate within the source polygon. Spec counts all hold (Union 12, Intersect 6, Subtract 9, Add 18; touching union 10; coplanar subtract 6; driver inside, no overlap, several drivers).
- Open questions BOOL-Q1..Q8 (Close holes and retry, BOOL-30 sets and Esc cancel not built). Icon not made yet (text row). GUI checklist `docs/BOOLEAN_GUI_QA.md`.
- Tests: `operandi-core/tests/booleans.rs`, `boolean.rs` unit tests, `boolean_dialog::tests`.

## 2026-11-11 — Render item settings (branch render-item)

- Clicking **Render** in the Shader Tree selects it; Properties shows Frame (Name, Render Camera, Frame Width / Height), Settings (Maximum Quality, Noise Threshold, Diffuse / Reflection / Refraction Depth, and under More: Maximum Radiance, Indirect Caustics) and Preview (Convergence Target, Max Render Time; Preview only). One undo step per change or drag; saved in native version 36. Frame Width / Height replace the fixed 1920 × 1080 frame (a camera's Resolution Override still wins). Ray depths, clamp and caustics reach Cycles (ABI 16) for F9 and the Preview. New scenes (spec v1.2): 64 samples, 10 %, depths 8 / 4 / 8, Preview 97.5 % / 5 min; older scenes render as before. Final Color's Denoising is the render's denoise switch (RI-B03).
- Provisional: RI-B01..B07. Not built yet: User Channels / Tags tabs, Pixel Aspect Ratio, Render Region, frame presets / link, Apply Preset, Tier 2 rows. GUI check docs/RENDER_ITEM_GUI_QA.md.
- Tests: `operandi-core/tests/render_item.rs`, `render_item_ui_tests.rs`.

## 2026-11-11 — Sculpt toolbox brushes on Blender maths (branch claude/project-thread-09ebuw; spec sculpt-toolbox-2026-11-11)

- Blender's per-brush sculpt maths ported into `operandi-core/src/sculpt_blender/` (GPL-2.0-or-later, see DEPENDENCIES): falloff presets, hardness, square tips, brush-plane sampling, brush frames, Kelvinlets, and the Draw, Inflate, Smooth, Clay, Clay Strips, Crease, Pinch, Plane, Layer, Grab, Thumb/Nudge, Snake Hook and Rotate rules. Push, Inflate, Smooth, Move and the tip shapes now run through it (same results).
- New brushes in the Sculpt tab in SCT-01 order: Fold, Clay, Smudge, Carve, Scar, Flatten, Tangent / Center Pinch (one pop-up slot, UIC-25), Emboss, Attenuate (greyed, SCT-Q12), Spin; Move gains Move Mode (Normal / Slide). Each has its spec form, commands (`sculpt.clay` etc., NAMES.txt), input-map actions, Shift smoothing, locks and symmetry.
- Provisional: SCB-01..SCB-07 in SPECIFICATION_QUESTIONS.md. Not built yet: mask brushes (SCT-52, waiting on SCT-Q15), Carve's image-only Set Color / Color / Alpha rows, Inflate's Paint Mode row.
- Tests: `sculpt_blender` unit tests, `sculpt_toolbox_tests.rs` (core acceptance checks 3–10), `sculpt_ui_tests.rs` (forms, pinch slot, Attenuate).
## 2026-11-11 — Cycles sync after Blender's (branch claude/project-thread-f7taxo)

- The Preview keeps one Cycles between restarts (`operandi_cycles::Session`, C `oc_session_*`, ABI 17), as Blender's interactive sync does: device, kernels, images, surfaces, meshes, world and lights stay loaded, and a restart rebuilds only what changed, by content keys. On a 128k-triangle scene a GPU restart after a camera move went from 574 ms to 134 ms, and an item move to 56 ms (`session_speed_probe`). F9 renders still start fresh. Closing the Preview frees it.
- Meshes go to Cycles in their own space with the item's world matrix, so a moved item keeps its points, and equal meshes (clones) share one copy. Each surface is built once and shared by every mesh using it.
- Smooth shading now passes the viewport's exact corner normals (angle rule and Smoothing amount) instead of splitting points, so a box keeps its 8 points. The old "Smoothing amount not passed on" provisional note is gone.
- Provisional: a render cancelled before Cycles has taken in the whole scene drops the kept renderer; an image file changed on disk starts it afresh. GUI check `docs/CYCLES_SYNC_GUI_QA.md`.
- Tests: `operandi-cycles/src/session_tests.rs`, `tests::creases_come_from_corner_normals_on_shared_points`.

## 2026-11-11 — Vertex Map Tools and Vertex Map falloff (branch vertex-map-tools; spec vertex-map-tools-2026-11-11)

- Falloff menu: **Vertex Map** (reads the selected weight map's raw signed values; no value is 0, no map selected is 1; Mode / Index, no handles) and **Invert** (1 − clamp(w)). **Add Falloff** stacks more falloffs, each with a Mix Mode (Multiply, Add, Subtract, Max, Min, unclamped); picking another type replaces the stack. A **Weight Map** picker selects the map. Transforms, Bevel and sculpt strokes use the whole stack, so Vertex Map then Invert makes a painted map a sculpt mask (VM-40). All 15 measured cases pass.
- Sculpt tab **Vertex Map Tools** section: Paint, Smooth, Scale Down, Erase, Set Value (spec forms, Normalize Weights), Live Deformers, the Weight Map picker and New Weight Map (Name / Init Value / Value). Strokes use the sculpt brush (tip, pressure, symmetry, falloff, Shift smooths the map); each is one undo step. A stroke with no map selected makes "Weight" (VMT-18).
- Commands: `sculpt.vmapPaint` … `sculpt.vmapSetVal`, `vertMap.new`, `select.vertexMap`, `sculpt.liveWeightEdit`, `falloff.vertexMap`, `falloff.invert`.
- Provisional: VMB-01..07 in SPECIFICATION_QUESTIONS.md. Not built: the Vertex Map shading style reading point weight maps, Show Weight Values, Smooth Selected (optional), colour-map fields (left out, VM-Q02).
- Tests: `tests/vertex_map_falloff.rs` (cases.json, stacking, sculpt mask), `tests/vertex_map_tools.rs`, `sculpt_ui_tests.rs` (viewport stroke, commands, colours).
- **N32 (2026-11-19):** Paint applies the strongest dab per point from its stroke-start value. Vertex dots keep their usual colours; Show Weightmaps displays the surface tint. The cage polygon average on subdivision remains provisional.

## 2026-11-11 — Blender materials, Lane A + CC0 starter pack (branch blender-materials-starter)

- `operandi-preset-convert <folder> --out x.oppreset [--graph g.json] [--copy-maps] [--licence/--url/--credit]` turns a texture-set folder into a material preset: one group, one Image Map layer per map (effect, strength, colour space), one shared Texture Locator, licence and credit in the header (native v37). Core in `preset/convert.rs`; the texture-set loader is shared as `preset/texture_set.rs` (moved from the preset browser branch, which should use it when it merges).
- With a node graph from `tools/blender-export/export.py` (GPL-2.0-or-later, Blender headless, format `NODE-GRAPH-JSON.md`) the material must be simple (a Principled tree of the listed nodes); otherwise it is refused whole with "needs Node Material (not yet supported)". Principled values map back through the Cycles translation in reverse.
- Starter pack: 32 CC0 materials (20 ambientCG via their .blend, 12 Poly Haven from maps) in `~/Developer/Operandi-Content/Starter`, built by `scripts/build-starter-pack.sh`; 0 refused. Thumbnails are the base colour map (the rendered ball is on the browser branch).
- Fixes: Cycles reads data maps (roughness, metal, height...) as plain data, not sRGB (BMC-Q08); packed AO multiply order (BMC-Q09). Picture paths in a preset may be relative to the preset.
- Provisional: BMC-Q01..Q09. GUI check `docs/BLENDER_MATERIALS_GUI_QA.md`.
- Tests: `preset/convert/tests.rs` (17), `operandi-cycles` `data_maps_are_read_without_the_colour_conversion`. Renders: `cargo run --release -p operandi-cycles --features cycles --example render_preset -- x.oppreset out.png`.


## 2026-11-11 — Viewport split and tear-off v1 (branch viewport-split-tearoff; spec viewport-split-tearoff-2026-11-11)

- **Ctrl+drag a view's dot** splits it at half toward the drag (right, left, below or above; VST-Q01 confirmed by Jason). The direction follows the pointer until release; while dragging, the panel shows an insert line where it divides and a fill over the new half, red when the halves would be too small (then release does nothing). Esc cancels; released outside every panel it still splits, never tears off. Works in torn-off windows too.
- **Minimum sizes** (VST-23, provisional): views (3D and UV) 120 × 90 pt, lists 100 × 80. Splits that would go below are refused, and Split Horizontal / Vertical / Quad and View > Split View grey with a reason. Dividers stop where a panel on either side reaches its minimum. Floating view windows can't shrink below what they hold (at least 200 × 160).
- **Delete** in a row of three or more gives the space to the two neighbours in proportion (T08: 200/300/500 → about 286/714); a split left with one panel dissolves.
- **Torn-off views** open through the same OS-window path as palettes and floating panels (`floating::os_window`), stay above the main window (VST-Q05, Jason: yes) while sitting anywhere on any screen, and come back where they were left, and take app shortcuts while focused. Detach opens the window over the view's old place; every new window is pulled fully onto the pointer's screen. A saved window whose screen is gone reopens near the main window (VST-37).
- **UV views**: `Pane::Uv(key)` is in the layout ("UV", "UV 2"); split, copy, quad, maximise and tear-off treat it like a 3D view. Drawing it is the UV viewport build's part.
- Layout changes never add undo steps or mark the scene modified (VST-39, VST-41; tested).
- Provisional: VSB-01..05 in SPECIFICATION_QUESTIONS.md. Not built (spec "later"): view presets (VST-16), Lock Width / Height (VST-26), palette-style windows and the backtick toggle (VST-30), layout undo (VST-40). GUI check docs/VIEWPORT_SPLIT_TEAROFF_GUI_QA.md.
- Tests: operandi-layout unit tests (neighbour share, nesting, minimums, divider clamp, UV panes), `panel_dot_tests.rs` (T01–T07, T11–T14, T16), `window_state.rs` (screen clamp, T15).

## 2026-11-11 — UV Unwrap and Pack UVs, stage 1 (branch claude/project-thread-iwegbz; spec uv-unwrap-pack-relax v1.1)

- New crate `operandi-uv`: Blender's UV parametrizer (charts with seams, pins and hole sealing; ABF++; LSCM; Minimize Stretch; average island scale), SLIM minimum stretch and the island packer (box pack, strip packers, optimal square tables, shape packing over a distance bitmap, margin search, fixed islands, overlap merging), ported at Blender b4eed6fd. Licences in `crates/operandi-uv/README.md`.
- UV Unwrap... (Edit tab UV group, Geometry menu, `uv.unwrap`, MCP/headless): every visible polygon of the active meshes, seams from selected edges, an edge selection set or material/part borders, closed pieces with no seam become one island per polygon, the UV map created as Texture when missing. Pack UVs... (`uv.pack`): Pack, Stretch, Orient, Keep Size, Direction, Gaps / pixel gaps, Bounding Box, Lock Stacked, Normalized / UDIM / Manual / Tiles; with polygons selected only their islands move and the others are avoided. One undo step each; the values are remembered.
- Provisional: UV-Q4, Q16..Q23 (Unwrap is a dialog until the UV viewport exists). Not built: Symmetrize, seam maps, Unwrap & Relax, UV Relax, Cut Map, the UV viewport, Pack's Nearest / Polygon Tag / By Item / Background / New Map. Shape packing of big islands can take a second or two. GUI check `docs/UV_UNWRAP_PACK_GUI_QA.md`.
- Tests: `operandi-uv` unit tests, `operandi-core/tests/uv_tools_spec.rs` (spec cases UVW-01..13, UVP-01..09), `uv_dialogs_tests.rs`.

## 2026-11-11 — UV view, stage 1 (branch claude/project-thread-iwegbz; spec uv-unwrap-pack-relax v1.1 §4)

- View > UV View (or typing "uv view") turns the active view into UV space: the 0-1 tile with a 0.1 grid, labels every 0.5, axis lines and the UDIM number; the active mesh's polygons in the chosen map (header button "UV: <map>"), selected polygons filled, selected edges and points in the selection colour, seams (discontinuous edges) pale lavender; Overlap and Distortion toggles. Alt+drag pans, Ctrl/Cmd+Alt+drag and the wheel zoom, A / Shift+A frame, . and , zoom. Clicks select in UV space, shared with the 3D views; Select Connected there selects UV islands (`Command::SelectUvIslands`).
- Provisional: UV-Q24..Q27 (a view mode rather than a UV layout, active mesh only, overlap by whole polygons, no paint/lasso yet, not saved with the layout). GUI check added to `docs/UV_UNWRAP_PACK_GUI_QA.md`.
- Tests: `uv_dialogs_tests::uv_view_selects_and_follows_islands`, `uv_view::tests`.

## 2026-11-11 — Freeze v1: curves (branch freeze-v1; spec freeze-2026-11-10)

- Freeze turns Curve, B-Spline and Bezier elements into 2 Point Polygons, one Face or one straight Polyline (dialog's Freeze Curves as). Fixed Spans on: 36 steps per span, on the drawn curve; off: cut adaptively by Curve Refinement Angle (greyed while Fixed Spans is on). A mesh with curves and subdivision surfaces freezes both in one "Freeze" undo step; Tessellation off keeps the cage and still freezes curves. Nothing to freeze is refused with no undo step. The old "doesn't handle meshes with lines" refusal is gone.
- `poly.freeze` takes the reference args face, drill, axis, disp, deform, tess, fixed, cangle (radians or "5deg"), morphSave, morphName; bare opens the dialog; unbuilt ones are accepted and ignored with a status note. The plain Freeze action uses the remembered values.
- Provisional: FRZ-B01..B09 in SPECIFICATION_QUESTIONS.md. Not built: Make Holes, Displacement, Deformation, morph save. GUI check docs/FREEZE_GUI_QA.md.
- Tests: `operandi-core/tests/freeze_curves.rs` (FRZ-C01..C10), `subdivide_tab_tests::frz_curves_freeze_from_the_dialog_and_the_command`.

## 2026-11-11 — Solid Drill and Axis Drill (branch drills; spec drills-2026-11-11)

- Solid Drill... and Axis Drill... (Edit tab "Booleans & Drill", Geometry menu; `poly.solidDrill mode material cutmesh item children` and `poly.drill mode axis ...`, numbers or words, bare opens the dialog; headless/MCP too). Core, Tunnel, Stencil (material created if missing, one undo step with the cut) and Slice; Background, Last Selected or a Specific Surface Item drives. One undo step "Solid Drill Tunnel" etc.; drivers untouched; nothing selected after.
- Own cutting core (`operandi-core/src/slicing/drill.rs`): edge cuts made once per edge and shared, each nearby polygon split in its plane by the driver's crossing segments (exact-sign triangle tests), holes joined by two bridges, untouched polygons unchanged. Axis Drill uses the outline's prism (shadows extruded, unioned with Manifold) as a Solid Drill driver. Solid Drill drivers get the Booleans pre-flight and notice.
- Stability (DRILL-19..21): two independent inside tests per piece (winding number + exact ray crossings, or + 2D outline test), per-polygon area cover, open-edge length unchanged, validate; any failure or panic changes nothing. 40 random turned boxes through a curved sheet: all exact, Core + Tunnel = Slice.
- All spec cases DR-D0..D3, S1..S5, A1..A5, F1..F3 pass. Open: DRILL-Q1..Q6 (result sets, Esc cancel, per-point maps on new points, procedural stage). No icons yet (text rows). GUI checklist `docs/DRILL_GUI_QA.md`.
- Tests: `operandi-core/tests/drills.rs`, `drill.rs` unit tests, `drill_dialog::tests`.

## 2026-11-11 — Viewport materials closer to Cycles (branch claude/project-thread-szcspe)

- Advanced style draws each material as Cycles' principled surface, approximated the way Blender's EEVEE does (GPL-2.0-or-later module `operandi-core/src/materials/blender_principled.rs`): sheen, coat, emission, metal (F82 tint), gloss (Specular IOR Level), diffuse, each dimming the layers below; GGX highlight from the key light, flat ambient as diffuse light and reflected environment; all in linear light. Calibrated so the default grey keeps VS-3's levels (90 dark side, 225 facing the light).
- Advanced base colour is Diffuse Color alone, as in the render (Diffuse × Amount in the other styles). Roughness, Metallic, Specular Amount/Color, Clearcoat, Sheen and Luminous now show.
- Image layers on those inputs, Bump, Normal and Stencil / RGBA cut-outs draw per pixel (one list function per material, `shader_wgsl::translate_list`); bump and normal map follow Blender's nodes, with a tangent frame from screen derivatives.
- Other styles unchanged. Provisional: VM-Q01..Q07. GUI check docs/VIEWPORT_MATERIALS_GUI_QA.md.
- Tests: `blender_principled` unit tests; `material_gpu_tests::principled_wgsl_matches_the_cpu` and the list half of `generated_shaders_match_eval_at_on_the_gpu` (headless GPU); `gpu_scene` `principled_values_reach_the_advanced_style`, `textured_principled_inputs_draw_per_pixel`; display_shading Advanced levels.

## 2026-11-11 — Freeze v2 (branch freeze-v2; spec freeze-2026-11-10 v2)

- Checked against the reference: Polygon mode with a selection freezes only the selected curves and polygons; Item mode everything; Vertex mode with points selected nothing. Tessellation off now freezes nothing, curves included. On-curve points (Bezier anchors, Curve points) are reused as samples and stay shared with other polygons; Bezier handles go. 36 equal-parameter steps per span, straight or not.
- Fix: Freeze in Item mode no longer deselects the item.
- Provisional: FRZ-B01..B11 (Fixed Spans off follows the guide, Option A, Jason 2026-11-11). GUI check docs/FREEZE_GUI_QA.md.
- Tests: `operandi-core/tests/freeze_curves.rs` (FRZ-C01..C13 v2).

## 2026-11-11 — USD export, stage 1 of 7 (branch usd-io)

- File > Export ▸ USD (.usd), USD ASCII (.usda), USD Binary (.usdc), and `scene.saveAs` with a USD file name or `format:usd|usda|usdc` (registry: a format implies export; extension and format must agree). Through OpenUSD 26.08 from Blender's library pack: `scripts/build-usd.sh`, `--features usd` (`OPERANDI_USD=1` for QA apps). Without the feature the export says USD isn't included.
- `crates/operandi-usd`: `plan.rs` turns the scene into prims (items as Xforms with their own rotation order, hierarchy, visibility; meshes with n-gons, normals, UV maps, material-tag subsets; Sub-D/Catmull-Clark as catmullClark with creases; mixed meshes split). `ffi.rs` writes them through a C bridge whose prim writers are adapted from Blender's USD exporter (GPL-2.0-or-later module `bridge/blender/`, in the CONTRIBUTING exception).
- Blender's OpenUSD needs Python's symbols at load; the bridge links them from the pack's static Python (never run).
- Provisional: USD-Q01..Q04. Next stages: materials, cameras/lights, instancing, import, Windows/Linux. GUI check: docs/FILE_FORMATS.md item 14.
- Tests: `operandi-usd` plan tests (no OpenUSD) and round trips through OpenUSD (`--features usd`: world matrices match Operandi's, text/binary formats, creases, subsets, safe names, failed export keeps the old file); `file_commands` USD saveAs test.

## 2026-11-11 — Remesh (branch remesh)

- Three commands, one undo step each, last values kept, bare opens the dialog: `sculpt.remesh` (Voxel Remesh), `mesh.retopo` (Automatic Retopology) and `operandi.remesh` (Blocks / Smooth / Sharp). Edit tab rows, Sculpt tab Remesh..., Geometry menu.
- Voxel Remesh and Blocks / Smooth / Sharp are Operandi's own (`operandi-core/src/sculpting/remesh_voxel.rs`): exact line crossings with winding numbers (overlaps join, inside-out shells count, holes closed by a two-of-three axis vote), checkerboard squares filled so no edge has more than two polygons, surface nets with cell-centre, crossing-mean or plane-fit points. Voxel keeps weight and colour maps (sculpt masks) by nearest point.
- Automatic Retopology through QuadriFlow (BSD-3), vendored in the new `crates/operandi-remesh` (GPL-2.0-or-later glue from Blender, Eigen MPL-2.0); crease angle made adjustable. Refuses surfaces with non-manifold edges or points.
- Dialogs run the work on a worker thread with progress and Cancel and hand the result to the same command, so typed, scripted and dialog runs give the same mesh.
- Speed (106k-polygon mesh, release): Voxel 0.05 s (47k quads) to 0.36 s (435k), Smooth 0.26 s, Retopo 2–3 s.
- Provisional: REM-Q01..Q06. GUI check docs/REMESH_GUI_QA.md.
- Tests: `remesh_voxel` and `remesh` unit tests (manifold output, volume, union, holes, corners, maps, cancel, args), `operandi-remesh` tests, `remesh_dialog::tests`; speed probe `cargo test --release -p operandi-core --lib remesh_speed -- --ignored --nocapture`.

## 2026-11-11 — USD export, stage 2 of 7: materials and USDZ (branch usd-io)

- Shader-tree surfaces become UsdPreviewSurface materials under `/root/_materials` (`operandi-usd/src/materials.rs`): diffuse, metallic, roughness, IOR, clear coat, emission, opacity, normal map; bound to each mesh and its tag subsets. Image layers become UsdUVTexture + primvar reader (+ UsdTransform2d), with tint/blend/invert as scale and bias; pictures copied into `<name>_textures/`. File > Export ▸ USDZ and `format:usdz` pack layer and pictures (UsdUtils).
- Bridge: `bridge/blender/usd_writer_material.cc` adapted from Blender's (GPL-2.0-or-later); ABI 2. The bridge now uses default symbol visibility: hidden visibility gave TfToken its own type info and broke writing tokens.
- Provisional: USD-Q05..Q07; bump, displacement, subsurface, sheen and transmission aren't in UsdPreviewSurface and are noted. GUI check: docs/FILE_FORMATS.md item 14.
- Tests: material plan tests (default material, tag binding, image layer), recogniser unit tests, OpenUSD round trips (preview surfaces and bindings, texture files and connections, USDZ contents, missing picture).

## 2026-11-11 — USD export, stage 3 of 7: cameras, lights, environment (branch usd-io)

- Cameras and lights export as Camera and UsdLux prims inside their items' Xforms (`operandi-usd/src/lights.rs`; bridge `usd_writer_camera.cc`, `usd_writer_light.cc` adapted from Blender, ABI 3). The environment is baked into a lat-long `environment.hdr` on a DomeLight in `/root/Environment`, turned to the scene's up axis.
- Provisional: USD-Q08 (light units), USD-Q09 (dome bake). Mesh lights not exported.
- Tests: lat-long convention, RGBE bytes, light units; OpenUSD round trips for a camera (placed where its item is), spot and area lights, and the dome with its picture.


## 2026-11-12 — Matcap renders (branch render-matcap)

- `operandi-app --render-matcap <preset.oppreset | look> <out.png> --hdri <file>` renders a plain smooth sphere filling a square picture: orthographic camera straight on, lit only by the HDRI, black outside the circle (`--transparent` for alpha), 512 px by default (`--size`, `--samples`, `--rotate`, `--exposure`). `--batch <folder>` renders the fixed ten looks (`preset::matcap::LOOKS`) and writes CREDITS.txt. Needs a Cycles build.
- The ten looks are simple material values in code: the library's CC0 materials are textured scans, none a fitting clay, chrome, gold or jade. Toon 2/3-step are post-processed clay renders (lightness fitted to the dominant light direction, cut into equal-area flat bands): the renderer has no stepped ramp.
- Pictures are CC0-1.0 (Jason, 2026-11-12). Clay looks are soft under a studio HDRI's even light.
- Tests: `preset::matcap` (closed outward sphere, exact framing, look names), `matcap_render` (argument parsing and errors, ortho view, toon bands, black/clear outside, credits).
## 2026-11-12 — Preset browser P1: matcaps and meshes (branch claude/project-thread-i94glh)

- Works: matcaps in Matcaps folders (round tiles, double-click/drag into a
  view, Use in All 3D Views, Viewport Properties Matcap ▸ Library, centred
  square, Clay Studio for a missing file, sidecar notes); 16 GPU matcap
  slots reused oldest first. Fresh Presets starter folders and the Meshes
  hint. Library pack ships Matcaps/. `scene.importReference` adds to the
  current scene with kind filters (File ▸ Import too); right-click Open /
  Import Into Current Scene; scenes dragged in import (Cmd opens); Shift-drop
  rests on the surface or work plane; Save Mesh Preset with Centre on Ground
  (`preset.saveAs centerOnGround:`); clay thumbnails for scenes and models.
- Provisional: MC-Q01..08, PG-Q01..07 (no drag-out of a 3D view yet; no Alt
  preview; Shift-drop isn't its own command yet).


## 2026-11-12 — Modo content folder, stages 1 and 2 (branch claude/project-thread-ssvwc2)

- Browse a Modo content folder in place (MC-01..04): drop it on the Preset Browser for Add as Location / Copy Into Library / Cancel (`pbview.basePathAdd`, `dircache.copy`), or put it in the library folder and it appears as a root. Nothing is written inside it.
- Modo files by type (MC-10..13, `preset::modo`): reads only the FORM header plus DESC and PRVW, seeking past other chunks. `.lxo` is a Modo Scene (also under Meshes; drag in = add, "geometry only"); `.lxl` mesh presets add a new mesh item at the origin (Shift-dropped: under the pointer, PG-06), one undo step (`lxo::load_mesh_preset`); `.lxp` `.lxe` `.lrl` list greyed with their own picture and kind ("Modo material preset. Not supported yet."), Show in Finder only; scripts and .cfg never show. Modo materials arrive as named material groups.
- Kind filter (MC-20/21): Show [All / Meshes / Materials / Environments / Images / Scenes] beside the search toggle, `operandi.pbview.kindFilter`; an empty Meshes says "Add a folder of models to see them here." "Show only usable files" is now `operandi.pbview.onlyUsable`.
- Provisional: MC-Q01..Q08 (DESC/PRVW layouts, THUM unused, .lxl FORM type, library-folder roots, drop point). GUI check: docs/MODO_CONTENT_FOLDER_GUI_QA.md.
- Tests: `preset::modo` (framing, DESC, PRVW, cut-off and binary files), `lxo::mesh_preset_tests`, library listing/buckets, `preset_browser::modo_tests` (MC-T1..T5, commands, copy, folder drop).


## 2026-11-13 — Environment Texture Locator (branch env-texture-locator)

- Works: an environment image (HDRI) layer's Properties have Layer / Texture Locator sub-tabs. Texture Locator: Transform (Position, Rotation, Rotation Order with all six orders, Size, Auto Size, World Transforms, World Coordinates), Projection (Projection Type, default Spherical; Projection Axis, default Y), Mapping and Falloff. Rotation X/Y/Z, Rotation Order and Projection Axis (the picture's pole) turn the sky in the viewport and in Cycles alike (both read `Scene::locator_to_image`). Edits are undo steps, one drag one step (`ShaderTreeEdit::SetLocator`).
- New image layers start Spherical about Y, ZXY, at identity. Environment layers without a locator get one when a file opens. Item transforms take the four new orders everywhere (`rotOrder` channel, Item Properties, LXO/USD export). Native version 43.
- Saved only, no effect on the sky: Position, Size, World Transforms, World Coordinates, Horizontal/Vertical Repeat, Repeat U/V, U/V Tile Offset (new field), UV Rotation, Texture Offset Strength, Falloff. Auto Size is greyed (nothing to fit at infinity). Other Projection Types draw as latitude-longitude under an environment. Texture Replicator: out of scope.
- Provisional: ETL-Q01..Q04. GUI check: docs/ENVIRONMENTS_GUI_QA.md item 15. Tests: `tests/env_texture_locator.rs`.
- Follow-up (Jason, 2026-11-13): XYZ turns X first, as measured (ETL-Q01 decided); native v44 moves older XYZ items turned about two or more axes to ZYX, so nothing moves; LXO and USD read and write the corrected XYZ. New image layers start Spherical (ETL-Q02 answered); Light Probe is a true mirror ball in the viewport (unwrapped on decode) and Cycles (Mirror Ball texture, ABI 20), one function `environment_probe_uv`; pre-v44 Light Probe environment locators open as Spherical. The latitude-longitude sky was mirrored in both the viewport and Cycles; fixed, so every scene's sky flips to the correct orientation (matcap renders too).

## 2026-11-13 — Constrain to Background (branch claude/project-thread-1560nl)

- Works: Edit ▸ Snapping ▸ Constrain to Background (also an unbound input
  map action and the command line). Screen Axis, Vector and Point modes,
  Offset, Negative Offset and Double Sided in a Tool Properties section
  under the tool's rows. Live tools that move points (Move, Element Move,
  Rotate, Scale, Transform, Slide, deformers) put the moved points on the
  shown, inactive meshes every frame; Pen puts each placed or dragged point
  there. Target Item limits the background to one mesh. Scripts:
  `tool.set const.bg on|off|?` and `tool.attr const.bg geometry|bkgOffset|
  negative|dblSided|handle|target`. Turning it on or off keeps the tool
  and re-applies. Esc over a
  view with no tool edit clears it. Not saved, not an undo step.
- Background triangles and their box tree are built once and kept while
  the background meshes don't change: about 1 ms per drag frame for 1,600
  moved points over 32k triangles (release, opt-in probe).
- Provisional: BG-Q1..Q12 (cage surface, whole line-of-sight search,
  which tools apply it, Esc). Handle Constraint not built. Pen Make Quads
  partner points and the Ctrl+click end point land on the background too.
- GUI check: docs/BACKGROUND_CONSTRAINT_GUI_QA.md.
- Follow-up (Jason, 2026-11-13): XYZ turns X first, as measured (ETL-Q01 decided); native v45 moves older XYZ items turned about two or more axes to ZYX, so nothing moves; LXO and USD read and write the corrected XYZ. New image layers start Spherical (ETL-Q02 answered); Light Probe is a true mirror ball in the viewport (unwrapped on decode) and Cycles (Mirror Ball texture, ABI 20), one function `environment_probe_uv`; pre-v44 Light Probe environment locators open as Spherical. The latitude-longitude sky was mirrored in both the viewport and Cycles; fixed, so every scene's sky flips to the correct orientation (matcap renders too).

## 2026-11-13 — Solid Sketch, phases 1–4 (branch claude/project-thread-xg0zvq)

- Core (`solid_sketch.rs`): node tree and the spec's loop-model cage (box per node, 4-quad tubes, 8n−2 quads per chain, +8 per branch); Twist, Rotate (branch swings about the node), delete/re-parent, branch delete/move/scale, Ctrl-insert, Auto Align, Auto Merge (loop merge), Subdiv Mode as subdivision polygons.
- Tool (`solid_sketch_editor.rs`): `solid.sketch` / `SS Default`, live rebuild into the active mesh; each edit (a whole drag) is a step inside the tool, Drop is one "Solid Sketch" undo step, Cancel restores; refuses without a mesh layer.
- App (`solid_sketch_ui.rs`): Tube cell's alternate (right-click or hold) and Geometry ▸ Draw ▸ Solid Sketch; Tool Properties in the spec's order; click/drag to add and size, drag to move (Shift: branch), right-drag to scale (Shift: branch; a click opens the menu), middle-click to delete (Shift: branch), Destroy mode, Ctrl-insert; red/blue Size squares, green Twist square, rotate rings behind Rotate Handles.
- Keep the sketch (`solid_sketch_store.rs`, native v45): nodes saved on the mesh; re-activating asks to pick it up (names the polygons that would go, Don't ask again); exports carry only polygons; `operandi.solidSketch.strip` (Operandi-only, undoable).
- Provisional: SS-Q01..Q16. Negative builds nothing yet. Solid Sketch icon (batch 19) shows on the Tube cell while Solid Sketch is on. GUI check: docs/SOLID_SKETCH_GUI_QA.md.
- Tests: `solid_sketch::tests` (17), `tests/solid_sketch_tool.rs` (16), `solid_sketch_ui_tests` (11).

## GL triangle count (2026-11-13)

- The OpenGL Polygon Count overlay line now reads "GL: 8,832": triangles drawn for visible items (quad 2, n-gon n-2, after SubD), under the selection line. It replaced "N polygons drawn", which counted polygons and keyed SubD off the shading flag.
- Source: each resident item's cached GPU fill (`Resident::drawn_triangles`, one lookup per item a frame); items not on the GPU fall back to `subdivision::drawn_triangles` (polygon list, both SubD kinds).
- Tests: `gl_count_tests` (6).
- Max GL Polygon Count counts triangles, the same number as the "GL:" line (PREF-Q21 answered, Jason 2026-11-13); the dialog says "N triangles". Non-SubD meshes off the GPU count from the remembered polygon summary (`FaceSummary::plain_triangles`), so the per-frame Bounding Box check never walks polygons. Mesh Stack items count their result. Preference key unchanged.

## 2026-11-13 — Direct Caustics and Absorption Distance (branch whiskey-caustics)

- Works: Render item > Global Illumination tab > Caustics: Direct Caustics, Total Photons, Local Photons, Walkthrough Mode, Indirect Caustics (moved from Settings > More). With Direct Caustics on, each light gives a caustics-only copy at its Affect Caustics share: smooth see-through Glass surfaces cast, every other mesh receives, rendered with Cycles' manifold next event estimation (MNEE). A light with Affect Diffuse and Specular 0 now casts only caustics. Light Material gets an Affect Caustics row; MCP `affect_caustics`, `direct_caustics`, `total_photons`, `local_photons`, `walkthrough`. Native v46 (only when the section differs from its defaults). Cycles ABI 21 plus a small Cycles patch (`crates/operandi-cycles/cycles/patches/caustics-only-lights.patch`, applied by build-cycles.sh).
- Absorption Distance (it existed): above 0 the surface no longer tints too (the colour came out squared, then more), and index-1 surfaces absorb as well. AD-01 and AD-03 measured in tests.
- Provisional: photons are stored only (DC-Q02); MNEE handles refraction through up to 6 surfaces and needs smooth shading on the glass, so flat-shaded glass or deep stacks (glass + liquid + several ice cubes) may show no direct caustic; DC-Q01..Q05, AD-Q4..Q6. GUI check: docs/RENDER_ITEM_GUI_QA.md items 12–13. Tests: `operandi-cycles` caustics_tests, material_extras_tests; core `tests/render_item.rs`.

## 2026-11-13 — Render fixes round 1 (branch render-fixes-round1)

- F1 shadow caustic tint: Cycles' MNEE already carries the glass's inside absorption (Absorption Distance) into Direct Caustics; measured Beer's law on an amber ball (R:G:B 1 : 0.28 : 0.020 at 0.5 m, 1 : 0.53 : 0.14 at 1 m), clear ball neutral. The planned Cycles patch (`mnee-volume-tint.patch`) tinted twice (1 : 0.08 : 0.0004), so it was dropped. The original white whiskey caustic isn't reproduced by the tests; GUI item 14 checks it.
- F3: the final render and the Preview light with the scene's lights only (`Settings::with_only_scene_lights`); no studio sun when the scene has none. Thumbnails keep it.
- F4: Absorption Distance 0 on a solid (not Thin Wall) passes the square root of a constant Transparent Color per crossing, so it tints once (0.5 slab passes 0.50, thin or thick). Textured colours and open sheets: AD-Q7.
- F5: new scenes start with Caustics Refraction Only; saved scenes keep theirs.
- F6: a scene with no polygons renders its environment (final render and Preview).
- F7: with Direct Caustics off, glass is a Transparent BSDF of its Transparent Color to shadow rays and to rays after a diffuse bounce, so its shadow is even, tinted and never black, with no focus even with refraction caustics on (measured shadow/lit 1.01 : 0.51 : 0.25 for colour 1 : 0.5 : 0.25). Provisional.
- Tests: `operandi-cycles` render_fixes_tests (F1, F3, F6, F7), material_extras_tests (F4); core `tests/render_item.rs` (F5). GUI check: docs/RENDER_ITEM_GUI_QA.md items 14–16.

## 2026-11-14 — Symmetry for the modelling tools, phase 1 (branch symmetry-tools)

Spec: research symmetry-2026-11-13 v3 (behaviour only; README phase 1, recommended answers to Q1–Q9).
- Shared service (`symmetry.rs`): creation path `Symmetry::welded` / `mirror_append` (mirror copy, seam points shared, crossing geometry made once with "Crosses the symmetry plane: not mirrored"); movement path `mirror_moved` (topology-keeping tools) and `mirror_offset` (one-vector tools); `partners_of` (grid partner lookup); `unpaired_points`, the shared "still exactly symmetric" check.
- Never silent (SYM3-40, `symmetry_support.rs`): every tool is classed (no wildcard arm); while Symmetry is on, unwired tools say "<Tool> ignores Symmetry for now" in the status bar.
- Pen (mouse and numeric, Make Quads too): mirrored and welded at the seam, live dimmed mirror preview, 8 px seam snap; one undo step.
- Solid Sketch: linked twins (real nodes pointing at each other; editing either edits both, mirrored), seam nodes with mirror-symmetric boxes, one welded symmetric cage; delete takes the twin; twins saved with the sketch. Q6–Q9 recommended answers. Provisional: with Symmetry off, a symmetric sketch's twins stay linked *and keep mirroring edits* (only new nodes go untwinned); the first node placed off the plane gets a crossing twin link.
- Selection: lasso/rectangle already sent per-pick Select (partners added); double-click edge loop, Select Connected and Selection Fill now add the mirror side.
- Extrude and Edge Extend offsets, Edge Slide and Vertex Slide: mirrored for the partner side. Provisional: Extend's Rotate/Scale aren't mirrored yet.
- Loop, Ring, Close Loop, Select More / Less / Between, Next / Previous Loop (SYM-LOOP-01..04, bug reports #3/#4, Jason 2026-11-21): with Symmetry on, only the picks on the clicked side count (a pick that mirrors an earlier pick is set aside; seam picks count), then the mirror of the result is added.
- Loop Slice: the mirror ring is cut too. Provisional: a ring that is its own mirror with cuts not at 50% on edges crossing the plane may land off-symmetric.
- Symmetrize (polygons): Symmetry menu ▸ Symmetrize Polygons Positive/Negative (delete half, mirror, merge). Merge off is in the command, not the menu yet; loose points on the replaced side are refused.
- Not yet (phase 2/3): Edge/Vertex Extrude, Bend, Poly Haul, Edge/Polygon/Pen/Curve/Primitive/Axis Slice, Clone/Array/Radial Array/Lathe, Vertex Create, Sketch Extrude, Tear; seam guide, toggle shortcut, unpaired warning, Symmetry Tool, seam snap distance in Preferences.
- Spec v3.1 (probes): Solid Sketch's first node always lands on the plane; lasso/rectangle put swept elements first, then partners. Extrude's mirrored sideways shift and Pen's seam weld differ from the reference on purpose (Jason 2026-11-14: better is fine).
- Tests: `tests/symmetry_creation.rs` (7), `tests/solid_sketch_symmetry.rs` (9), `tests/symmetry_selection.rs` (3), `tests/symmetry_movement.rs` (5). GUI check: docs/SYMMETRY_TOOLS_GUI_QA.md.

## 2026-11-14 — Symmetry for the modelling tools, phase 2 + Symmetry Tool (branch symmetry-phase2)

Spec: research symmetry-2026-11-13 v3.1, README items 12–15 and 17.
- Two-pass helper (`symmetry_sides.rs`): the primary side is done as entered, then the other side with mirrored settings (world-space mirror for drawn paths, cut planes and copy placements); the untouched side is found again by position, so a pass that renumbers doesn't lose it.
- Bend: spine fitted to the side the user works on; the other side bends with the mirrored base, spine and lean; seam points stay on the plane. Vertex Tear: torn points on the other side take the mirrored drag. Poly Haul's picked modes (Bevel, Split, Remove, Tear, Slide): the mirror element is hauled too, with the mirrored drag. Sketch Extrude: the other side follows the mirrored path.
- Edge and Vertex Extrude push along each element's own normal: symmetric from a symmetric selection (class B, tested).
- Slices: Edge Slice cuts the mirrored path; Polygon Slice adds each cut's mirror (a cut already mirrored isn't doubled); Pen/Curve/Primitive Slice cut the mirrored path in the mirrored direction; Axis Slice cuts once more with the Offset mirrored. Provisional: an Edge Slice path across the plane is cut once; Axis Slice assumes the box is centred on the plane.
- Clone, Array, Radial Array: the other side's polygons get the mirror image of each placement. Lathe: the other side's profile turns about the mirrored axis, the other way round. Mirror ignores the setting (class G). Provisional: nothing selected copies everything with the placement as entered.
- Vertex Create: each point gets a mirror point; one on the plane is made once.
- Every tool now takes part in Symmetry; none says "ignores Symmetry" any more (the rule stays for new tools).
- Symmetry Tool (Symmetry menu, command line "symmetry tool"): drag right to raise Threshold (default 1 mm); points whose partner is within it snap to match. Use Topology on, Reverse off (the negative side moves). Points still without a partner show as yellow squares and are counted in the panel. Provisional: a point within half of Threshold of the plane snaps onto it; tool ID `operandi.symmetry.tool` until the name map gives one (SYM3-Q16).
- Tests: `tests/symmetry_phase2.rs` (10). GUI check: docs/SYMMETRY_TOOLS_GUI_QA.md steps 10–16.
- Not yet (phase 3): seam guide, toggle shortcut, unpaired warning on tool start, seam snap distance in Preferences.
## 2026-11-14 — Light view (branch claude/project-thread-bywy7w)

Brief: coordinator 2026-11-14 (behaviour in words, no spec folder).
- View type menu ▸ Light (after Camera): (Current Selection), then every light by name (UIC-10 sub-menu, UIC-03 no "…", UIC-01 mark, UIC-42 filter past 12 lights). Greyed with a reason when there are no lights / no light selected.
- Looks from the light's position down its own -Z, point lights included. Navigation moves the light through Camera view's path (`sync_camera_view`, `write_camera_view`, `Command::CameraView` now also takes lights, labelled "Light View"): one undo step per press, locked lights refused, undo/redo moves the view with the light. MCP `view_set camera:<light id>` looks through a light.
- Provisional (LV-Q1..Q4): orbit point ahead of the light, spot cone sets the lens, no roll, (Current Selection) follows the selection.
- Tests: `light_view_tests.rs` (9) and the menu test in `viewport_buttons.rs`. GUI check: docs/LIGHT_VIEW_GUI_QA.md.

## 2026-11-14 — Symmetry phase 3: guide, toggle, unpaired warning (branch symmetry-phase3)

Spec: research symmetry-2026-11-13 v3.1 section 8, README item 16.
- Seam guide: while Symmetry is on, the plane is drawn across the active mesh (a square a little larger than it, from the remembered bounds): a thin axis-coloured line in orthographic views, a faint grid-coloured outline in perspective.
- Toggle Symmetry (Symmetry menu, command line "toggle symmetry", remappable): off remembers the setting (axis, offset, topology); on brings it back, X the first time. No default key yet (SYM3-Q18).
- Unpaired warning: when a tool starts with Symmetry on, the points it edits that have no mirror partner are marked with yellow squares and the status bar says "N points have no mirror partner — Symmetrize or Symmetry Tool to fix".
- Seam snap distance: a snapping preference, Symmetry Seam Snap (Preferences > Tool Handles > Handle Snapping, 8 px default, 1–60), saved with the snapping settings so the shared snap service can take it over later.
- Tests: core `tests/symmetry_phase3.rs` (2), app `symmetry_tests::toggle_seam_guide_and_snap_distance`. GUI check: docs/SYMMETRY_TOOLS_GUI_QA.md steps 17–20.
- Show Symmetry Plane (Jason, 2026-11-14): a per-view checkbox in the view's Display menu and Viewport Properties ▸ Visibility ▸ General Visibility hides the seam guide without turning Symmetry off. On by default, saved with the view's settings. GUI check step 21.
- The modes bar's Symmetry button lights in the accent colour while Symmetry is on (UIC-01, UIC-23), like Snapping's. GUI check step 22.

## 2026-11-14 — Symmetry toggles with a tool open (branch symmetry-live)

Jason's bug: Symmetry couldn't be turned on while a tool was active. Tool pipe: Symmetry is a layer under every tool.
- Solid Sketch refused every Symmetry command ("Drop or Cancel Solid Sketch first"); Symmetry and Toggle Symmetry now run with any task open, like Constrain to Background.
- The open tool follows the change at once: a transform's handle moves to the picked side (or back; a centre set by a drag or click stays), an applied live preview and an open Cone/Ellipsoid/shape preview are rebuilt, Solid Sketch re-applies its twin rule for new nodes, and the unpaired warning updates. A refused change leaves everything as it was. Not an undo step; each drag stays one.
- Finished drags keep the Symmetry they were made with (each drag is its own step); Move still moves only selected points (SYM-TR-01).
- Tests: app `tweak_tests::symmetry_toggled_*`, `symmetry_button_and_shortcut_work_with_a_tool_open`, `symmetry_toggles_with_every_session_open`; core `solid_sketch_symmetry::symmetry_toggled_with_the_sketch_open_applies_to_new_nodes`. GUI check step 23.

## 2026-11-14 — One tool card per tool (branch claude/project-thread-qiyo4d)

Brief: architecture review 2026-11-14 pick A, steps 1–2. No behaviour change except one message.
- `tool_card.rs` (core): a `ToolCard` per tool — family, modes, pipe layers (action centre Frame/Neutral/Unused, falloff, symmetry class, snap use), handles (kind, anchor, axes, plane circles, drag anywhere), multi-mesh, item transforms, per-drag undo, continue. Exhaustive match, no wildcard.
- Lists removed: `supports_falloff`, `symmetry_support::tool_support`, the multi-mesh list, the action-frame list, the edge/vertex/deform activation lists, `is_primitive`/`is_slice`/`is_duplicate`/`is_transform` bodies, the Move/Rotate/Scale/Transform lists (now `Tool::transforms_items`), the continue list; in the app `combined_handles`, `off_handle_drag`, the handle none/count/colour/anchor/axis lists, plane circles and the Move snap gate.
- The three neutral-context checks (Chamfer Edit, Poly Haul, Sketch Extrude) are one action-centre gate with the same messages, built from the card.
- Changed message: the falloff refusal now names the tools from the cards ("…Bevel, Transform, Move, Rotate, Scale, Slide, Edge Bevel, Edge Chamfer, Vertex Bevel, Vertex Extrude, Push, Smooth or Bend").
- Handle frames for edge and vertex tools moved to core `action_center.rs` (`edge_handle_frame`, `edge_slide_handle_frame`, `vertex_handle_frame`); the edge frame is now memoised per drag like the other two.
- Tests: core `tests/tool_card.rs` (old lists copied verbatim, mode refusal/acceptance), app `tool_card_view_tests.rs` (old viewport lists).
- Left for later: tool geometry on the card (step 3, with registry handlers), identity/mid-tool-undo lists, Tool Properties fields.


## 2026-11-14 — Solid Sketch symmetric legs (branch claude/project-thread-q2lzj4)

- Fixed: legs branched off a seam hip fused into one lopsided blob. Both legs wanted the hip box's bottom face; the first took it and its twin was joined to a side face. Now a twin's link takes the mirror images of its leader's faces, and a leader under a seam node leaves through a face on its own side. Tests check the cage's polygons are symmetric, not only its points (`solid_sketch_symmetry.rs`).

## 2026-11-14 — Vertex Bevel batch 1 (branch vertex-bevel)

Spec: vertex-tools-2026-11-14 v1.1 (G01, G02, G03; VX-B01..B07, VX-E05, VX-Q02).
- Round Level (0–32) on the Vertex Bevel panel, live preview and MCP (`round_level`). Each side of the cap is a fillet of 2·L segments tangent to the two edges; the fillet points also go into the polygon on that side. The cap is one L×L quad grid per cut point around a centre: flat on a flat surface, a dome on a corner. All seven oracle cases match (arc points to 1e-4, inside points to 0.01).
- Border points: a 2+-polygon border point gets a cap with its open side along the border; rounded, the inner sides are fillets and the open side stays straight (VX-Q02). A one-polygon corner is cut off with no cap (a fillet when rounded).
- Inset (Bevel) and Width (Extrude) stop at each edge's far end instead of refusing; two selected neighbours cut their shared edge from each end, so big values pass (VX-B9).
- After the bevel only the cut points are selected; Use Material tags every cap polygon.
- Spec v1.2 (VX-B08/B09): fillets come from Inset and the edge directions only, so a clamped cut kinks the border; the centre is the mean of the cut points; every new corner takes the old surface's UV at its position (per polygon across a UV seam).
- Provisional: the inside rule [OP] (spokes, Coons sectors); next to a clamped cut three of the four check points are 0.011–0.04 out (VXB-Q3).
- Tests: core `tests/vertex_bevel_round.rs` (11), rounded UVs in `tool_uvs.rs`, updated refusals in `m3_vertex_tools.rs`. GUI check: docs/VERTEX_BEVEL_GUI_QA.md.

## 2026-11-14 — Solid Sketch Subdiv Mode Level 2 (spec v0.4, branch sketch-subdiv-level2)

- Level 2 now runs one Catmull-Clark step on the cage (subdivision Freeze at level 1) and writes it as SubD polygons: 4× the quads (1 node 24, 2 nodes 56), slightly rounder, inside Level 1's bounds. none and Level 1 unchanged. Changing the mode rebuilds at once.
- Tests: core `subdiv_level1_marks_the_same_cage`, `subdiv_level2_bakes_one_smoothing_step`; app `changing_subdiv_mode_rebuilds_at_once`.

## 2026-11-14 — Vertex batch 2: short-edge caps and Merge (branch vertex-merge)

Spec: vertex-tools-2026-11-14 v1.3 (VX-B11, VX-M01..M06, VX-C01; G04, G06, G13).
- Vertex Bevel next to a clamped cut (VX-B11): the cap is built as if nothing were clamped, then each clamped cut moves every inside point by s × (a along + c across) from the measured table (flat four-polygon caps, Round Level 1–3); several clamps add up. Other cases blend the Level 3 table [OP]. Inside points take the UV of the polygon they lie in.
- Merge (tool and command) lands each group at its average and leaves nothing selected. New options: Keep 1-Vertex Polygons (tool, dialog, `keep:true`), Test Morph Map Positions (tool, dialog) and Merge Discontinuous UV Values (dialog, on).
- Join… opens a dialog with Keep 1-Vertex Polygons and Join Discontinuous UVs; Join Averaged runs with their defaults.
- Merge Tool already started at Distance 0.001 (G13).
- Provisional: Discontinuous UVs shares matching UVs at merged points (VXM-Q1); a polygon squashed to one corner still goes, Operandi has no one-point polygons yet (VXM-Q2).
- Tests: core `tests/vertex_merge.rs` (8), VX-B11 cases in `tests/vertex_bevel_round.rs`, average landing in `m3_vertex_tools.rs` (MV-T04/MV-C09 scenes regenerated). GUI check: docs/VERTEX_MERGE_GUI_QA.md.

## 2026-11-14 — Item transform Reset + Freeze, stage 1 (branch freeze-transforms)

Spec: freeze-transforms-2026-11-14 (BUILD-BRIEF-STAGE1, BEHAVIOR-ITEM-TRANSFORM-OPS §3, §4, §7, §8, §10 defaults).
- Item Properties > Item Transform ends with a Reset dropdown (every item type) and, on mesh items, a Freeze dropdown: Position, Rotation, Scale, All; Shear hidden. Item > Transform has the same eight entries. Both run `transform.reset` / `transform.freeze` (typed, MCP; no argument = all) on the item selection. The camera's own Reset moved onto the shared one.
- Reset: the part goes to identity, children follow, geometry untouched. Freeze: the part is baked into the points (P = P'·B, points take B) and nothing moves; direct children get B·C. Parents first when both are selected. Mirrored bakes reverse polygons; corner normals, normal and relative-morph maps follow; UVs, weights, sets untouched. Mesh Stack items bake into the Base Mesh, stack live. Rotation order never changes.
- Locked items skipped with one status note ("2 locked items skipped"); non-mesh items skipped by Freeze; identity is no undo step; one step per pick ("Freeze Rotation", "Reset Transform").
- Provisional: no pivot channel or animation yet, so nothing to compensate there (FRZT-Q01, Q02); mesh-light prototypes not compensated (Q03); a stored Solid Sketch is dropped by Freeze (Q04); a Freeze whose children would need shear is skipped (Q05).
- Tests: core `tests/freeze_transforms.rs` (15), `item_transform_ops` unit tests (3), app `item_transform_menu_resets_and_freezes`. GUI check: docs/FREEZE_TRANSFORMS_GUI_QA.md.

## 2026-11-15 — Mesh Constraint and SubD background (branch bg-constraint-subd)

- Works: a SubD background is its displayed smooth surface, at the mesh's display levels (BG-Q5 answered); Face polygons and hidden-polygon rules as before. Cache rebuilds on SubD toggle or level change; a 4.2M-triangle smooth background builds in ~0.36 s once per drag, ~2 ms drag frames (release probe `subd_drag_frame_cost_probe`).
- Works: the toolbar Mesh Constraint button (click toggles, orange and "Background" while on, Option-click or right-click opens the popover); the Mesh Constraint popover (Snapping popover's frame and rules, mode strip with Guide/Primitive greyed "Not built yet", Constrain to Background rows); Edit ▸ Snapping ▸ Mesh Constraints…; the constraint is tool-pipe layer 5 (`ToolCard::layers.constraint`), after snapping. New defaults: Negative Offset and Handle Constraint on. Options saved in `mesh-constraint-preferences.json` (not Target Item, not the on state).
- Provisional: MC-Q1..Q4 and the header/row-order notes in SPECIFICATION_QUESTIONS.md. Handle Constraint is stored only (Draw tools, which own the only curve handles, aren't constrained). Scope unchanged (BG-Q8). GUI check: docs/BACKGROUND_CONSTRAINT_GUI_QA.md items 8–11.

## 2026-11-15 — Falloffs batch 1: falloff in Tool Properties (branch falloff-panel)

Spec: falloff-2026-11-15 v1.1, batch 1 (G01, G02, G03, G05, G06, G04 skeleton).
- Works: Tool Properties shows one foldable section per falloff layer ("Selection Falloff", "Linear Falloff"…) under any tool, or alone with no tool open (`falloff_panel::FalloffFields`). Rows in spec order with Mix Mode last; In/Out always shown, greyed unless Custom ("Only used when Shape is Custom"), factory 0/0; Linear Start/End, Symmetric, Auto Size X/Y/Z, Reverse; Radial Center/Size, Auto Size; Vertex Map Mode/Index; Selection Steps; Invert note; added layers have Remove; deform presets keep Fit to Selection.
- Mix Mode on every layer; the first layer's is stored (`Editor::spatial_falloff_mix`, `SpatialFalloffMix`) and has no effect (FO-32). Weights stay unclamped (FO-34).
- Falloff menu only chooses ((none), Linear, Radial, Vertex Map, Selection, Invert), Adds (Invert, Linear, Radial, Vertex Map, Selection) and Clear Falloffs; Weight Map stays there. Esc over a view with no tool edit in progress clears every layer. Right-click adds a Step, right-drag changes Steps, on any Selection layer. View ▸ Show Falloffs toggles the point tints (on by default).
- "Shape" label on every falloff and Tweak (FO-Q09, Jason's approved deviation). Provisional: FO-Q04 gesture, FO-17 unreachable. Tests: app `falloff_panel_tests.rs` (7). GUI check: docs/FALLOFF_PANEL_GUI_QA.md.

## 2026-11-15 — Delete leaves no orphan points (branch delete-orphan-points)

- Works: forward Delete in Edge mode drops every point it leaves with nothing attached (a lone quad's or triangle's other corners); a stray edge (two-point polyline) goes with its points; a polyline span splits an open line or opens a closed one (provisional ED-Q01). Pre-existing loose points stay; one undo step; both sides go when Symmetry has selected the mirror partners. Polygon/Vertex Delete already cleaned up; Backspace (Edge Remove, Remove) still keeps points.
- Fix: Vertex Remove (and border-edge Delete) no longer drops every two-point line in the mesh.
- Tests: core `tests/delete_orphan_points.rs` (9). GUI check: docs/DELETE_ORPHAN_POINTS_GUI_QA.md.

## 2026-11-15 — Selection action centre in every component mode (branch action-center-edge)

- Fix: Action Center ▸ Selection was greyed in Edge and Vertex mode because its axes only took selected polygons. Axes now come from the polygons around the selected edges or points (provisional ACS-Q01); the centre is the selected points' average. Selection Center Auto Axis does the same in Edge/Vertex mode. With nothing selected it uses World axes rather than refusing.
- Fix: the Selection choice is no longer greyed on a moved mesh (availability now uses the same world-space frame as the tool).
- Other centres (Automatic, Border, Element, Screen, Origin, Local) already worked in every component mode. Parent/Pivot stay "Not available yet" (VAC-Q04). Tests: core `tests/action_centers.rs` (+2).

## 2026-11-16 — Transform pipe on UVs (branch uv-transform-pipe)

- Works: in a UV view the pipe tools run their own 3D code on a UV proxy of the current map (every UV position a point at (U, V, 0), item at the origin) and write back U and V only (UVT-130, UVT-06). Move, Rotate, Scale, Transform and their presets (Soft Move, Shear, Taper, Bulge/Pole, Twist, Swirl, Flex), Element Move, Smooth and Edge Slide (UVT-137) all work on UVs. Seams are separate points (UVT-42), connectivity and Selection falloff follow UV edges, Linear/Radial falloffs are fitted and measured in UV and drawn in the UV view, Vertex Map weights come from each UV's 3D point (UVT-132), centres are UV points (UVT-133), Symmetry is off (UVT-136), hidden polygons stay (UVT-02), one undo step per session.
- UV view: Element Move hovers/outlines and drags UV points, edges, polygons; right-drag sizes Range; Smooth and Slide drag anywhere; Element centre clicks pick UV points; handles U teal, V yellow (UVT-134).
- Not built: Soft Drag, Jitter, Screen falloff (not in 3D yet, UVT-Q14); click-without-drag handles (UVT-Q13); Slide's UV Map field (UVT-Q16). Guess switches in `uv_pipe.rs` (UVT-Q11, Q12, rotate). Tests: core `tests/uv_transform_pipe.rs` (14), app `uv_editing_tests.rs` (+2). GUI check: docs/UV_EDITING_GUI_QA.md items 28–35.
- Fix (Jason's QA): after an Element Move drag, hover picked UVs where they used to be and the next drag undid the last. Each Move / Rotate / Scale / Transform / Element Move drag in a UV view now ends a step of the session as in 3D, so the UV proxy is rebuilt from the moved UVs (fields restart from zero per drag, HND-Q03). Test: `uvt131_element_move_picks_uvs_where_they_moved_to`.

## 2026-11-17 — Falloffs batch 2: Soft Selection, Element, Coplanar (branch claude/project-thread-m0er89)

Spec: falloff-2026-11-15 v1.1, batch 2 (G12, G13, G14).
- Works: Soft Selection (selection at 1, Shape(d / Radius) beyond it, along the surface with Use Connectivity, straight-line without), Coplanar (the selection plus polygons facing within Angle, spreading across shared sides or anywhere), Element (press on the model with Move/Rotate/Scale/Transform picks a vertex, edge or polygon per Element Mode and drags it; Range, Connected Elements and Shape as Tweak's, hover highlight). All three move points outside the selection with the transform tools, stack with Mix Modes, and are one undo step per drag; Cancel restores.
- Tool Properties sections with the spec's rows (Radius, Use Connectivity, Angle, Element Mode, Range greyed under Rigid, Connected Elements, Always Raycast, Shape/In/Out, Show Vertex/Show Edge, Mix Mode). Falloff menu and Add list in spec order. Right-drag in the view sets Soft Selection Radius. Show Edge draws weighted edges.
- Commands `falloff.softSelection`, `falloff.element`, `falloff.coplanar` (command line and MCP `falloffs`). Weights are worked out once per drag (`falloff_reach.rs` memo).
- Provisional: FO-Q05, Q06, Q11..Q14; Always Raycast stored only. Tests: core `tests/falloff_reach.rs` (9), app `falloff_batch2_tests.rs` (3). GUI check: docs/FALLOFF_BATCH2_GUI_QA.md.

## 2026-11-17 — Falloffs batch 3: Cylinder, World Transforms, axis handles, drawing (branch claude/project-thread-m0er89)

Spec: falloff-2026-11-15 v1.1, batch 3 (G07..G10).
- Works: Cylinder falloff (Center, Size, Axis, Auto Size, Shape; endless tube; rings-and-lines outline; Center, per-axis and cross-axis Size handles). Use World Transforms on Linear, Radial and Cylinder (off by default as measured: each mesh reads its own space; handles drawn through the active mesh's matrix; Auto Size and deform presets fit in that space). Per-axis handles on Linear Start/End and Radial/Cylinder Center move one coordinate. Drawing: picking a geometric falloff (or Draw in View) arms it; press sets the origin, drag the size.
- Provisional: FO-05 arming reading, Radial drawn as a sphere, Cylinder outline. Tests: core `tests/falloff_batch3.rs` (3), app `falloff_batch3_tests.rs` (4). GUI check: docs/FALLOFF_BATCH2_GUI_QA.md items 9–13.

## 2026-11-17 — Falloffs batch 4: Screen, Lasso, Airbrush (branch claude/project-thread-m0er89)

Spec: falloff-2026-11-15 v1.1, batch 4 (G11, G15, G16).
- Works: Screen (press-drag draws a circle; Smooth falloff; Transparent), Lasso (Lasso/Rectangle/Circle/Ellipse drawn in the view, Soft Border), Airbrush (drag paints weight that builds up, right-drag sizes the brush, brush circle under the pointer, Clear Paint). Weights are stamped from the view when drawn (FO-Q08) and drive every falloff tool through the stack; sections, menu and Add entries in spec order; commands `falloff.screen`, `falloff.lasso`, `falloff.airbrush`.
- Provisional: drawing/painting gestures, Screen has no handles, Lasso World stored only. Tests: core `tests/falloff_screen.rs` (5), app `falloff_batch4_tests.rs` (3). GUI check: docs/FALLOFF_BATCH2_GUI_QA.md items 14–17.

## 2026-11-17 — Falloffs batch 5: Noise, Vertex Map colour/morph, Image (branch claude/project-thread-m0er89)

Spec: falloff-2026-11-15 v1.1, batch 5 (G17, G18, G20).
- Works: Noise (Scale, World; own gradient noise, fixed seed). Vertex Map falloff reads morph maps (offset length or X/Y/Z) and colour maps (brightness or R/G/B/A) as well as weight maps; the falloff menu's picker lists them. Image (Browse…, Repeat, Soft Border, Use Selected UV Map, Scale, Pos X/Y, Rotation, World, Mix): brightness laid over the view or looked up through the UV map, stamped like Screen. Commands `falloff.noise`, `falloff.image`.
- Provisional: noise shape, picture kept per session only, no Image handles. Tests: core `tests/falloff_pattern.rs` (5), app `falloff_batch5_tests.rs` (2). GUI check: docs/FALLOFF_BATCH2_GUI_QA.md items 18–20.

## 2026-11-17 — Falloffs batch 6: Path, Curvature (branch claude/project-thread-m0er89)

Spec: falloff-2026-11-15 v1.1, batch 6 (G19, G21). Every falloff type in the spec's menu is now built.
- Works: Path (clicks add curve points, points drag as handles, Size, World, Clear Path; tube weight around a smooth curve). Curvature (Scale; bumps above 0.5, dips below). Both worked out once per drag. Commands `falloff.path`, `falloff.curvature`.
- Provisional: Path gestures and curve; Curvature formula (FO-Q07). Tests: core `tests/falloff_path.rs` (4), app `falloff_batch6_tests.rs` (2). GUI check: docs/FALLOFF_BATCH2_GUI_QA.md items 21–22.


## 2026-11-17 — Path tools, curve commands, 2D shapes (branch curve-tools)

Specs: CURVE-EXTRUDE-PEN-CLONE v1, SPEC-CURVE-CLONE v1, CURVE-COMMANDS-2D-SHAPES v1.
- Works: one path engine (`path_tools.rs`) with Sweep and Clone; buttons Curve / Bezier / B-Spline Extrude and Curve / Bezier / B-Spline / Pen Clone as preset rows. Path points by click (surface or work plane, snapped), drag, Delete, and Sketch (VectorCraft curve fit, `curve_fit.rs`). Symmetry. Ops `curve.extrude.item`, `curve.clone.item`, `pen.clone.item`.
- Works: Make Open / Closed Curve, Make Curve Fill, Align Winding, Set Curves Color (view setting); Curve tab form; Circle, Ellipse, Rectangle, Diamond, N-Sided, Star Mesh Stack.
- Pen Extrude added 2026-11-17 from the hands-on check (preset `PenExtrude`). Not built yet: conversions, offsets, Bezier Tangent. Provisional: CX-Q2..Q8, CC-Q3..Q5 in SPECIFICATION_QUESTIONS.md. Tests: core `tests/path_tools.rs` (18), `tests/curve_commands.rs` (16); app `path_tool_view`, `curve_tab`. GUI check: docs/CURVE_TOOLS_GUI_QA.md.

## 2026-11-17 — Generators and on-hold ops, batch 5 (branch curve-tools)

Spec: SCATTER-UVRECT-TRANSFER-CONSTRAINT v1 (`generator_ops.rs`).
- Works: Scatter Clone tool (Duplicate tab; Count incl. source, Centre ± Range, Scale/Rotate variation, Seed, Clone effector) and `scatter.clone.item` op (even spread in a plane or box); `surfaceConstraint.meshop.item` (Directional/Half Ray/Closest, Invert, Offset, Max Distance, Strength); UV Rectangle (`uvrect.meshop.item`, `operandi.uv.rectangle`, UV tab row); Transfer Vertex Map (`vertexmaptransfer`, `operandi.vertMap.transfer`); `uvfit.meshop.item`. The UnSubdivide op (`poly.unsubdiv.item`) is left to the Mesh Stack branch, which already has it.
- Julienne… and Dicer… (Edit tab, commands `operandi.julienne` / `operandi.dicer`, `julienne.rs`): closed pieces on the Boolean engine, Gap, Inner Material (JD-Q1). Not built: Curve Boolean (no spec yet; the Mesh Stack branch has the solid Boolean op). Provisional: GO-Q1..Q5. Tests: core `tests/generator_ops.rs` (15).
## 2026-11-17 — Deform finish, modelling parity batch 1 (branch deform-finish)

Spec: deform-presets-2026-11-06 v0 + DEFORM-PROBES v1 (DP-xx, 10 cases). Engines in `deform_finish.rs`, ops in `mesh_stack_deform.rs`.
- Works: Jitter (Range X/Y/Z, Seed, per-axis switches, Rigid Translate/Rotate/Scale; drag right for Range), Quantize (Step X/Y/Z from the world origin), Radial Align (Circle, Arc, N-Sided, Weight, Start/Central Angle, Exponent, Flatten + Smoothing for polygon borders), Linear Align (Line/Curve, Uniform, Align Length, Align to First, Screen Align), Relax Mesh... (dialog and `sculpt.relaxMesh iter: constrain:`, one step), Vortex (Rotate + Cylinder falloff, click sets the centre). All live tools take falloff and Symmetry (primary side mirrored). Toolbox rows replace `operandi.notBuilt`; Radial/Linear grey with nothing to align. Mesh Stack: Jitter (`xfrm.jitter.item`), Quantize, Radial Align, Linear Align, Relax Mesh.
- Provisional: DFF-Q01..Q08 (SPECIFICATION_QUESTIONS.md). Not built here (Jason, 2026-11-17, hands-on batch later): Soft Drag, Element Push, Soft Selection x4; drafted in commit eda70e80 and taken out after it. Sculpt stays with the Sculpt milestone.
- Tests: core `tests/deform_finish.rs` (24: every DP case), app `deform_tab_tests::deform_finish_rows_start_their_tools`, speed guard rows. GUI check: docs/DEFORM_FINISH_GUI_QA.md.

## 2026-11-17 — Selection tools, modelling parity batch 4 (branch deform-finish)

Spec: SELECTION-TOOLS v1 (SL-xx, 17 cases). Core `select_tools.rs` (`Command::SelectTool`), app `select_tools_ui.rs`.
- Works: Shortest Path (vertices, edges, polygons; the reference's straight-then-turn route), Sharp..., Coplanar Edges..., Colinear Vertices..., Diagonal... (dialogs with last values; `select.edgeSharp min: max:` etc. run without one), Coplanar Polygons Tool (live dialog), Conditional Loop and its Tool dialog, Lazy Selection and Shortest Path Mode toggles. Each is one undo step that changes only the selection; Symmetry adds the mirrored elements.
- Provisional: SL-Q1..Q6. The Conditional Loop selection op is the Mesh Stack builder's.
- Tests: core `tests/select_tools.rs` (14, every case), app `select_tab_tests::selection_tool_rows_run_their_commands`. GUI check: docs/SELECTION_TOOLS_GUI_QA.md.

## 2026-11-17 — Topology and clean-up, modelling parity batch 6 (branch deform-finish)

Spec: TOPOLOGY-CLEANUP-TOOLS v1 (TC-xx). Core `topology_cleanup.rs`.
- Works: Triple Fan (Polygon tab; fans the selected polygons from the selected vertex), Collapse Selection (Polygon tab; one centre per region, inner points removed, UVs blended), Axis Rotate (Rotate preset with one ring on the work plane's normal, Alt+Shift+E, Tools menu). One undo step each; nothing selected afterwards.
- Provisional: TC-Q2, TC-Q3, TC-30 reading. Not built here: Topology Pen, Tack, Retopo Sketch, Retopo Edge Lock/Flow (hands-on batch).
- Tests: core `tests/topology_cleanup.rs` (6), app `polygon_triple_tests::triple_fan_collapse_selection_and_axis_rotate_run_from_their_lines`. GUI check: docs/TOPOLOGY_CLEANUP_GUI_QA.md.

## 2026-11-17 — Curve hands-on batch (branch curve-hands-on)

Specs: CURVE-EXTRUDE-PEN-CLONE CX-30..32, CURVE-COMMANDS-2D-SHAPES CC-40..64 (hands-on).
- Works: Pen Extrude button (Start at Source off, Corner Scale); Curve tab conversions (six, new Converted_<type> item, target tool in Edit mode); Open/Closed Curve Offset and Trio (Enter Offset Amount); Bezier Tangent… (Sharp/Smooth/Uniform, Both/Out/In, Set Length). Core `curve_convert.rs`, app `curve_hands_on.rs`.
- Provisional: CX-Q1 Corner Scale width, CC-Q2 open offsets, CC-Q3 Sharp lengths; Bezier→Curve, B-Spline→Curve, Curve→B-Spline by pattern. Tests: core `tests/curve_convert.rs` (7), app `curve_hands_on` (1). GUI check: docs/CURVE_HANDS_ON_GUI_QA.md.
## 2026-11-17 — Hands-on Deform batch: Soft Drag, Element Push, Soft Selection (branch deform-hands-on)

From Jason's hands-on session (SD-Q1, DP-Q5, DP-Q6 answered). The rows drafted in eda70e80 are back and match the observations.
- Works: Soft Drag (Screen falloff around each press, linear, 20 px, right-drag sizes), Element Push (hover highlight, press picks the element, drag pushes it along its normal, polygons flat, Range spreads), Soft Selection Move / Rotate / Scale / Transform (Selection centre and axes, Soft Selection falloff, linear, Use Connectivity path vs straight distance).
- Tests: core `tests/deform_finish.rs` (Element Push, Soft Selection connectivity), `tests/falloff_screen.rs` (linear), app `deform_tab_tests::hands_on_rows_start_their_presets`. GUI check: docs/DEFORM_HANDS_ON_GUI_QA.md.

## 2026-11-17 — Tack, Topology Pen, Retopo Sketch (branch topology-tools)

From the hands-on observations TC-22, TC-45, TC-52 (spec v7). Core `align_tools.rs` (`Command::Tack`), `topology_cleanup.rs` (`Command::TopologyDuplicate`, `Editor::background_hit`); app `topology_tools_ui.rs`.
- Works: Tack Tool (Tools menu; click a front-facing polygon of another mesh; Align, Offset, Size, Rotate, Move Connected Vertices), Topology Pen (Edit tab, Tools menu; drag moves, Shift+drag an edge makes a strip on the background), Retopo Sketch (Edit tab, Tools menu; stroke → open Curve on the background, 3 cm spacing). One undo step each.
- Provisional / not built: see the Topology batch entry in SPECIFICATION_QUESTIONS.md. Tests: core `tests/topology_tools.rs` (3), app `polygon_triple_tests::tack_topology_pen_and_retopo_sketch_start_from_their_lines`. GUI check: docs/TOPOLOGY_TOOLS_GUI_QA.md.

## 2026-11-17 — Topology and Curve tabs, open curve offset (branch curve-open-offset)

Spec: TOOLBOX-TOPOLOGY-CURVE v1 (TB-xx), CURVE-COMMANDS-2D-SHAPES CC-43 (v9).
- Works: Topology tab (`toolbox.topology.json`, `topology_tab.rs`: tool buttons with Constrain to Background, Commands, Settings, Modifiers); Curve tab rebuilt to TB-20 with flyouts and now in the tab strip (it was missing); open curve offset side rule per CC-43.
- Provisional: TB-Q1; not built rows listed in SPECIFICATION_QUESTIONS.md. Tests: app `topology_tab`, `curve_tab` strip test, core `curve_convert::cc43`.
## 2026-11-17 — Curve Boolean (branch curve-boolean)

Spec: CURVE-BOOLEAN v1 (CB-xx). `curve_boolean.rs`, Edit tab Curve Boolean... (`curve.boolean`, dialog: Operation, Axis, Clip, Merge Curves).
- Works: Slice, Union, Intersect, Subtract of selected closed curves with the unselected ones; drivers unchanged; reversed winding; Clip; Merge off; both-selected Union; nothing selected after; one undo step.
- Provisional: CB-Q1/Q2, Driver Mesh not built. Tests: core `tests/curve_boolean.rs` (7, CB-T1..T6). GUI check: docs/CURVE_BOOLEAN_GUI_QA.md.
## 2026-11-17 — Spec v8 checks: Transfer Vertex Map, Conditional Loop (branch parity-fixes-v8)

- Transfer Vertex Map (GO-30..35): reference Type order, Sampling Space Local/World and Sampling Method Raycast/Distance on the op and on `operandi.vertMap.transfer` (`space:`, `method:`); RGB type. Tests: `tests/generator_ops.rs` go31/go32_go34/go33.
- Conditional Loop (SL-64): Sharp Angles threshold 60°, so closed rings close. Tests: `tests/select_tools.rs` sl64_* (5).
## 2026-11-17 — Spec v9: Axis Rotate, Topology Pen, Tack, Retopo sets, Dicer (branch parity-v9)

- Axis Rotate armed until a click, ring at the click, Angle + Apply; Topology Pen moves never conform, Shift+middle Add Loop, Ctrl+middle Remove; Tack Copy Geometry / Shift+click, Make Bridge refusal; Retopo Edge Lock / Flow buttons (Edit ▸ Commands); Dicer in the Axis Slice flyout; Julienne (button and command) removed, it is Axis Slice (TC-54). Details and what's not built: SPECIFICATION_QUESTIONS.md "Spec v9".
- Tests: core `tests/topology_tools.rs` (copy, constraint skip), app `polygon_triple_tests::retopo_edge_lock_and_flow_add_the_selection_to_their_sets`, `path_slice_tools_tests` (flyout). GUI check: docs/PARITY_V9_GUI_QA.md.

## 2026-11-18 — Behaviour-check fixes (branch check-fixes-1118b)

- Edge / Vertex Extrude with nothing selected turn on and act on every edge
  or point (EE07, VE08), starting with a Width; a one-polygon corner is cut
  off (VE04, provisional).
- Bridge `smooth_shift` takes thicken/sharp/angle_weight/both_sides (TH11).
- Oracle replay drives 84 more cases; departures pinned with their
  decisions (SPECIFICATION_QUESTIONS, "Behaviour-check fixes — 2026-11-18").

## 2026-11-18 — USD import U0–U2 (branch usd-import)

Spec `docs/BEHAVIOR-USD-IMPORT.md`; provisional readings USD-IQ01..IQ14.

- U0: macOS QA builds include USD by default (`OPERANDI_USD=0` leaves it
  out); bundles carry OpenUSD's libraries, plugInfo, LICENSE and NOTICE.
- U1: File > Open / Import / drop of a .usd/.usda/.usdc/.usdz asks the
  Import USD options, then opens it as an unsaved scene with a report.
  Xform hierarchy as parented items with local transforms, quads and
  n-gons, winding, all UV primvars, colour maps, normals (as a map plus
  polygon smoothing), creases, Catmull-Clark, hidden prims, units and up
  axis; instances and PointInstancers as mesh-sharing copies.
  `scene.importReference` and MCP `scene_import` use the defaults.
- U2: bound materials (GeomSubsets too) become polygon tags and Material
  Groups through the MaterialX reader (MaterialX network first, else
  Preview Surface; option), with pictures in their colour space, Texture
  Locators from UsdTransform2d, .usdz pictures unpacked, missing pictures
  kept and reported.
- Tests: `cargo test -p operandi-usd --features usd` reads the Blender 5.2.2
  USD-T1 scene (`fixtures/usd`, made by `make_usd_t1.py`) as .usdc and
  .usdz (USD-T1, T2), round-trips an Operandi export (T3) and a missing
  picture (T5); the plain tier tests the scene builder on hand-made stages.
- Not yet: cameras, lights, dome (U3); graph sidecar (U4); USD-IQ14.

## 2026-11-18 — USD import U3: cameras, lights, dome (branch usd-import)

- Chess-set fix first (c7746fd6): creases written last point to first
  emptied every lathed piece's mesh; now stored lower point first.
- UsdGeomCamera → camera items (lens, film, offsets, clipping, f-stop,
  focus); UsdLux Sphere/Rect/Disk/Distant/Cylinder/Portal → lights with
  Blender's power, exposure, colour temperature and ShapingAPI spots; the
  first DomeLight's picture → the environment image, pole up. Import
  Cameras / Import Lights now work in the dialog and on
  `operandi.usd.import`. USD-IQ15..IQ17.
- Checked against Blender: `usd_t1_renders_like_blender` (opt-in, Cycles
  + USD) renders USD-T1 through its imported camera and lights and matches
  Blender 5.2.2's own Cycles render within 0.1 % overall and 1 % per
  region; with the world dark (lights alone) within 0.1 % overall and 7 %
  per lit region.
- Export: area lights were written too dim by their area; fixed.

## 2026-11-18 — USD import U4: node graph sidecar (branch usd-import)

- `<file>_graphs.json` beside a USD file (from `tools/blender-export`), or
  the Graphs File picked in the dialog / `graphsFile:` on
  `operandi.usd.import`, turns each matching material's group into a Node
  Material layer with Blender's whole graph; unmatched and refused graphs
  are reported. "Use Graphs Sidecar" now works. USD-IQ18.
- USD-T4: fixtures/usd/usd_t4 (Noise → Color Ramp "Marble", plain
  "Paint"; make_usd_t4.py + export.py) passes.

## 2026-11-18 — USD import U5: Mesh Instances (branch usd-import)

- Before: native instances and PointInstancer copies came in as separate
  items sharing one copy-on-write mesh, independent once edited.
- Now: `Item::instance_of` (core mesh_instance.rs, native version 51):
  instances share their source's mesh and follow its edits (and pass
  theirs back) through `Scene::sync_instances` after every recorded step,
  on reopening and in undo. USD import makes the 10 T1 chairs one source
  and nine instances. Dialog "Instances" (Instances / Copies) and
  `instances:` on `operandi.usd.import`. USD-IQ19.

## 2026-11-18 — USD import U6: export round trip (branch usd-import)

- Export writes MaterialX networks beside Preview Surface, creases on
  plain meshes, and imported roughness maps (USD-IQ14 fixed). USD-IQ20.
- Checks: USD-T1 and Jason's chess set (copy) imported, exported and
  re-imported give the same item, polygon, camera and light counts and
  world bounds (chess: 45 items, 471,222 polygons, 3 cameras, 3 lights);
  Blender 5.2.2 reads the chess export the same as the original (35 meshes,
  471,222 polygons, 16,928 creased edges, the same lights, cameras,
  materials and box) — `fixtures/usd/blender_readback.py`,
  `round_trip_a_named_file` (opt-in).

## 2026-11-18 — Smooth Shift / Thicken Scale follows the action centre (branch smooth-shift-ac-pivot)

- Q-SS-04 decided (Jason): Scale pivots on the selected action centre —
  none/Selection: the shifted selection's centre (as before); Origin: the
  item's origin; Element/Screen: the picked point; Selection Border: border
  average; Local: each piece. Symmetry mirrors a point centre per side.
  SS06/TH10 stay pinned in the oracle replay as intentional differences.
- Tests: core `tests/smooth_shift.rs` sst_scale_follows_the_action_center,
  sst_origin_action_center_scales_about_the_origin,
  sst_action_center_scale_keeps_symmetry.

## 2026-11-18 — Show Weightmaps and Show UV Distortion (branch viewport-weights-distortion)

- HT-5/HT-6: two view header toggles (first after the separator) and
  Viewport Properties rows, `view3d.showweights` / `view3d.showDistortion`,
  per view and saved, greyed in Wireframe. Fills carry both tints as
  vertex data (`gpu_scene::Tint`); the view's look picks which show, the
  weight winning. Weight colours per pixel from the interpolated value
  (VMT-30); distortion uses `uv_view::polygon_distortion`, the UV view's
  own function, so the two views match.
- Provisional: HT-Q3 (tint strength), HT-Q4 (smooth surfaces flat per
  polygon). Tests: weights_distortion_tests.rs (HT-12..19, real GPU
  pictures), viewport_buttons ht05/ht18/ht20. GUI: docs/VIEW_TINTS_GUI_QA.md.

## 2026-11-19 — Docking J3 (branch codex/dock-j3)

- Panel grips show compass, edge, tab-row and divider targets during a
  move. Escape cancels. Layout files retain the resulting arrangement
  and active tabs (D-02..D-04, UIC-72, UIC-155, UIC-191).
- Tab-group merge follows the provisional D-02/D-04 reading. Keyboard
  routing checks the drawn lists (D-12). GUI gestures are in
  [LAYOUTS_STAGE2_GUI_QA.md](LAYOUTS_STAGE2_GUI_QA.md).

## 2026-11-19 — Docking J4 and J5 (branch codex/dock-j4)

- D-05: Drag a single tab onto the J3 targets. Sidebar tabs can become
  standalone registry panels. Escape leaves the source row intact. The
  reorder margin and releases between targets are provisional.
- D-06: The grip floats dock panels and groups in OS windows. The title
  strip uses the J3 targets. Layout files keep floating dock panels and
  legacy palettes keep their existing state format. GUI gestures are in
  [LAYOUTS_STAGE2_GUI_QA.md](LAYOUTS_STAGE2_GUI_QA.md).

## 2026-11-19 — Docking J6 (branch codex/dock-j6)

- D-02/D-04: Drops into a registry group keep its existing tab for each
  duplicate kind and activate the dragged front kind. Beside drops keep
  the groups separate.
- D-07: Tool Tabs can move and float independently. Top/bottom drops use
  one row with overflow arrows and left/right drops use the vertical strip.
  The original layout projects the vertical strip beside the toolbox.
  Saved layouts include the standalone pane and its orientation.

## 2026-11-19 — Docking J7 (branch codex/dock-j7)

- D-08, UIC-197/198: the top row selects Model, UV or feature-gated Paint
  tools. Window > Layouts uses the same live list and restore commands as
  Layout > Layouts. The Layouts pie remains available.

## 2026-11-19 — Docking J8 (branch codex/dock-j7)

- D-10, UIC-191/194/195: the layout stores dock window geometry, Tool
  Tabs orientation, and toolbox selection. Existing column layouts
  retain their arrangement and legacy files still open.
- D-11: shipped Model places horizontal Tool Tabs over the left tool
  panels, the centre view, and the two right-hand tab groups.

## 2026-11-20 — Report Bug or Idea (branch report-bug-or-idea)

- Help > Report Bug or Idea… (`operandi.help.reportBug`, name kept) opens
  the reports repo's `bug.yml` issue form, pre-filled by field id: manual
  reports fill only Environment (build, OS, GPU, recent log); crash links
  put the panic or native summary in `what` and environment, backtrace
  frames and log in `environment`, cut to stay under 7,000 characters.
  The form links to `idea.yml`. Needs the BugReports `issue-forms` branch
  merged before the links work as intended (docs/BUG_REPORTS.md).

## 2026-11-20 — Service holdouts (branch service-holdouts)

- Pen Slice snaps through the shared snap service like the Pen (point
  targets win over direction locks, SN2-37); its locks are the Pen's one
  function, switched by the Snapping ticks (SN2-34..36). Its own World
  Axis / Straight Line / Right Angle options are removed (they were never
  saved). Engage angle stays the Pen's 5° (SNH-Q01).
- One Rotation Snap Angle rounding (`HandlePreferences::round_to_snap_angle`)
  for Rotate, Scale hauls, Bend, the precision ring, UV rotate and the
  Duplicate hinge/arc handles (no fixed 15°); Radial Array / Radial Sweep
  arc ends step by it while snapping is on (SN2-40). Arc has no viewport
  angle handle yet.
- Tweak's primary side comes from the pivot service (plane counts as −).
- Edge/vertex tool handles sit on the selected element nearest the
  selection's box centre (primary half under Symmetry), not the first
  picked (ACS-Q-HANDLE1, provisional).
- Snapping for Cube/Sphere/Cylinder corners and side hauls, Cone/Ellipsoid
  draws (SN2-41), Create's points (VX-CR02), Solid Sketch node create and
  move (5.3; the sketch's own mesh is never a target, SNH-Q02).

## 2026-11-20 — MaterialX graphs, slice 1 (branch claude/project-thread-93alht)

Design: docs/MATERIALX_GRAPHS.md (stages 1-5).
- A `.mtlx` material the tree can't show as layers (noise, maths,
  ramps, custom nodes...) now imports as one Node Material layer holding
  the material's own MaterialX text, instead of being refused (MX-04).
  The same goes for USD import's MaterialX networks and the preset
  browser's `.mtlx` entries, since they all go through
  `materialx::import::preset`.
- `materialx::cycles` translates MaterialX to Cycles nodes for the
  existing general node list (no C++ change): open_pbr_surface,
  standard_surface (through MaterialX's translation graph),
  UsdPreviewSurface, gltf_pbr, surface mixes, displacement, images,
  coordinates, maths, comparisons, remaps, noises, ramps, checkerboard,
  nodegraphs, interfaces and nodegraph-implemented custom nodes.
  Constants fold. An unknown node still refuses the whole material,
  naming it.
- Saved as MaterialX text in the shader tree's Nodes section (native
  version 54). The viewport shows the graph's base colour (NM-40).
- Provisional: MXG-Q01..Q06. Not yet: USD export of graph layers, a view
  of the graph, exposed inputs, project-relative picture paths.
- Slice 2 (same day): USD export writes a group shown by a MaterialX
  graph layer as that graph's UsdShade MaterialX network (nodegraphs
  flattened to `<graph>_<node>`, pictures copied into `<name>_textures`),
  and USD import reads it back as a graph. Multi-output and custom nodes
  are left out with the reason (Preview Surface only); displacement isn't
  written yet.
- Slice 3 (same day, branch claude/project-thread-j6dj5q): every tree
  surface has a MaterialX form (`materialx::tree`): one `operandi_*` node
  per expression node, an `operandi_surface`, numbers kept to the bit.
  `materialx::tree_library` declares the nodes, with standard-node
  implementations for constant, luma, channel and the Normal, Add,
  Subtract, Multiply, Screen, Difference, Darken and Lighten blends
  (checked against the tree's sums through `materialx::cycles`). Cycles
  renders every tree material from its form read back. Not yet: the other
  nodes' implementations, USD export through the form, retiring the
  direct route (stage 4).
- Standard versions (same day, branch claude/project-thread-j6dj5q
  restarted from main): one node per mode for blends, planar axes,
  falloff shapes and tilings; standard-node graphs for 56 of the form’s
  nodes (all blends, images, UVs, tiling, clipping, locators, planar,
  falloff, gamma, bias/gain, HSV, distance...). A test-only MaterialX
  evaluator checks them against `Expr::eval_at` at shading points, and
  every standard node against the bundled 1.39.4 `stdlib_defs.mtlx`. Not
  yet: patterns, cylindrical/spherical/probe, box/triplanar weights,
  ramps, geometry inputs, occlusion.
- USD export through the form (same day): a Material Group that the
  Material-and-Image-Map network (MX-05) can't write whole (other blends,
  masks, locators, procedural-free adjustments...) now exports its tree
  surface's standard MaterialX form onto `open_pbr_surface`, flattened to
  standard nodes only, and imports back as a graph layer that Cycles
  renders. Lossless MX-05 groups export as before (they read back as
  layers). Surfaces with a node that has no standard version (patterns,
  ramps...) fall back to MX-05 with a note. Left out of the form, with a
  note: bump, displacement, rounded edges, inside volumes, Node Material
  layers, material values, Shader settings.
- Remaining standard versions (same day): cylindrical, spherical and
  probe projections, and box, cubic and triplanar face weights, one node
  per axis or face; 77 of the form's nodes now have standard graphs, all
  checked at shading points. Left Operandi-only: patterns, ramps (too many
  stops for MaterialX's ramp), geometry inputs, occlusion.
- Export Material as MaterialX and colour spaces (same day): a group the
  simple writer can't write whole exports as its standard form; exported
  pictures that drive colours are marked sRGB (they had no colour space
  in chunk 5's USD networks, so other apps read them as linear).
- Library groundwork (same day): MaterialX's 62 example materials are a
  test fixture; 56 import (was 54), the 6 left are Disney principled and
  hair shaders. Translation now uses MaterialX's own nodegraph
  implementations (bundled `stdlib_ng.mtlx`) and supports rgbtohsv /
  hsvtorgb, so the procedural brick and glTF boombox materials import.
  `library_probe` checks any downloaded library folder.
- Stage 5, viewport from the graph (same day): graph layers (MaterialX
  graphs and Node Materials) draw per pixel from their nodes in every
  viewport style instead of one flat colour (NM-40): base colour,
  roughness, metal, emission, transmission, coat, alpha, normal maps and
  bumps, with Cycles' maths, mixes, mapping and Noise / White Noise /
  Checker / Gradient / Wave textures ported to the GPU. All 17 example
  graph layers draw. Provisional: MXG-Q07..Q10 (Voronoi and camera-based
  coordinates fall back to the input's own value). GUI check: import
  `standard_surface_brick_procedural.mtlx` and `standard_surface_marble_solid.mtlx`,
  Advanced style, compare with a Preview render.
- Voronoi in the viewport (2026-11-21): graph layers' Voronoi texture draws
  per pixel (all features, metrics and 1D-4D), a port of Cycles' own code
  checked against its numbers. GUI check: a Blender Node Material with a
  Voronoi on Base Color, Advanced style beside a Preview render.
- Cellular and Poisson Cells match the render (2026-11-21): the shader
  tree's own Cellular and Poisson Cells patterns, and their per-cell colour
  variation, now use the same Voronoi in the core and the viewport as
  Cycles renders, so the cells sit in the same places (MXG-Q11). GUI check:
  a Cellular layer with Variation, viewport beside a Preview render.

## Large-model viewport (2026-11-20, branch large-model-perspective)

- Fixed the stretched, fisheye Perspective view (Sylvain): Maximise/Restore and
  the lone-view quad kept the world scale by scaling the lens, widening the
  field of view (about 95° from a half-height view), and layouts saved it. The
  distance takes the change now; layouts from older builds open at the normal
  lens. Tests: `maximising_never_widens_the_perspective_lens`,
  `a_saved_wide_lens_opens_at_the_normal_lens`.
- Alt+Shift+wheel over a Perspective view changes its field of view (5°–150°),
  keeping the orbit centre's size; a readout shows degrees and full-frame mm.
  Provisional, not from a spec; not in the reference shortcut list (SC-039 is
  the plain wheel only). Layouts save the view's own lens.
- Max GL Polygon Count and its question retired (Jason): the graphics memory
  watch (`gpu_memory.rs`) boxes the heaviest items near the card's budget
  (macOS only for now). PREF-Q21 updated.
- Node editor slice 1 (same day, read-only): Show Graph works for
  MaterialX graph layers; Material Group > Show as Nodes shows the
  group's live MaterialX graph in the graph window. Editing needs a spec
  (MXG-Q11). GUI steps 11-12 in NODES_N1_GUI_QA.md.

## Node editing, slices 2-5 (2026-11-20, branch claude/project-thread-l14sxn)

- Material Groups keep free MaterialX nodes, wires, box places and hidden
  flags (native 56). Wires into a Material layer's channels drive renders
  (Cycles ABI 29, `OC_EXPR_NODE`) and the viewport; unwired is
  pixel-identical. Export writes every free node with xpos/ypos.
- Show as Nodes edits: layer boxes, locked mixing nodes, Add menu, wire
  drag with type checks, Delete/Hide/Show All, stubs, value panel; the
  Shader Tree's "Nodes (N)" row; driven channels greyed with a marker in
  Properties. Commands `operandi.nodes.*` (placeholders).
- Provisional: which channels take wires and which a box lists (NE-Q01,
  NE-Q04), type rules (NE-Q02), values edit in the side panel rather than
  on the box (NE-Q05), presets don't carry free nodes (NE-Q03). GUI check:
  NODE_EDITING_GUI_QA.md.

## 2026-11-20 — Topology Pen v2 (branch topology-pen)

- Works: Mode drop-down (Move, Duplicate, Remove, Split, Add Loop, Point, Fill, Smoothing) and Edge Loop; the modifier gestures (right-drag loop/Radius, middle Split, Shift Duplicate, Ctrl Slide/quad, Shift+Ctrl Smoothing); smart snapping with weld on release; Falloff (Radius, Use Connectivity, Shape, Show Vertex/Edge), Inner Snap, Copy Discontinuous UVs, Offset, Edge Slide. Options stick for the session. The Topology tab button turns snapping off while the pen is up (TP-04). One undo step per press. The icon is rendered by the icon studio.
- Provisional: TPQ-1..7 (SPECIFICATION_QUESTIONS.md). Not built: Backface, Raycast Inactive Surfaces, Slip UVs, Symmetry for the non-Move modes.

## 2026-11-21 — Service holdouts, spec answers (branch service-holdouts)

- Pen and Pen Slice direction locks: within Inner Range pixels of the
  lock line, while dragging only, projected onto the line; Snap Angle
  plays no part (SNH-Q01). Straight Line's line is provisional (SNH-Q03).
- Solid Sketch: a new node takes the nearer of the seam and a snap target
  (8 px each), the seam on a tie (SNH-Q02).
- Handles (ACS-Q-HANDLE1): box centre for Edge Bevel/Extrude/Chamfer and
  Vertex Extrude, arms 25 px out; first-picked for Edge/Vertex Slide; no
  handle for Edge Relax and Vertex Bevel. Vertex Slide still has one rail
  arrow, not one per edge. Edge Weight unchanged (ACS-Q-HANDLE2).

## 2026-11-21 — Missing tools group 2 (branch missing-tools-2)

- Plane Item: Shift-click on the Text button, or `Plane Item`; UP12 is no longer skipped (MT5).
- Teapot tool (Position, Size, Axis, Make UVs) and Teapot Item: the 1987 Utah Teapot, 32 patches from `utah_teapot.patches`, credited in the About window (MT6). In Item ▸ Primitive and on the command line; no toolbox button.
- Provisional: placement in Item ▸ Primitive, separate lid piece, drag moves only (SPECIFICATION_QUESTIONS.md).
## 2026-11-21 — Missing tools group 1 (branch missing-tools-1)

- Select tab: Polygon Tag Type is a live drop-down (Material, Part, Smoothing Group greyed, Selection Set) running `operandi.select.polygonTagType` (MT1).
- Edit tab Commands: Fix Black Dots, Edge Zip, Diagonal Re-Direct, each one undo step, with the spec's FB-A..G, EZ-A..F and DR-A..H cases as tests. Text-only buttons (no icons yet).
- Provisional: Q-FB-1/2, Q-EZ-1/2, Q-DR-1 and the command names (SPECIFICATION_QUESTIONS.md).

## 2026-11-21 — Missing tools group 3 (branch missing-tools-3, stacked on missing-tools-2)

- Topology tab: Topology Mode (`view.topologyMode`, per view, also in the view's display menu), Foreground Opacity and Selection Hit Size fields (MT8 Offset dropped, Jason 2026-11-21), Instance Mirror, and Merge Instance (MT7-MT12). Four Topo colours in Preferences > Colors; its two preferences on the Display page.
- Merge Instance flips a mirrored instance's polygons (Q-MI-1 decided). Provisional: Offset has no visible effect yet, highlights stay depth-tested (SPECIFICATION_QUESTIONS.md).

## 2026-11-21 — Contour (branch missing-tools-4, stacked on missing-tools-3)

- Topology tab Contour: drag across a background mesh (or type Start/End/Direction); the cut under the line becomes one Curve, Lines or Face polygon per piece in the active mesh, evenly spaced, with Automatic, Uniform Span, Slicing Side, Cull Occluded Open Curves, Intersect, Select New Polygons, Angle Snap and Fast Slice (MT13). CT-1, CT-4..8, the miss message, undo and cancel are tested.
- Provisional: see SPECIFICATION_QUESTIONS.md (faceted-surface radius, open-piece direction, borrowed icon).

## 2026-11-21 — Mesh Paint (branch missing-tools-5, stacked on missing-tools-4)

- Mesh Paint and Mesh Paint Instance (Duplicate and Create tabs, Alt for the Instance variant): Slide, Strokes, Drop, Drop (no Scale), Linear, Array and Radial paint copies of a background mesh onto the selected mesh, as geometry or as instance items (Parent Off / Hierarchy / Root / Primary). Size, rotation, offset, parameters and effects, Random / All BG Layers, symmetry. One undo step per click or stroke, seeded so undo, redo and reopen agree.
- Provisional and not built: see SPECIFICATION_QUESTIONS.md (Target Mesh, 3D Brush, tablet tilt, snapping).

## 2026-11-21 — Q and Esc (branch claude/project-thread-s4394q)

- Q drops any tool even while a Tool Properties number field holds typed text (it confirms the typing first; a refused value keeps the field). Esc over a view drops any tool, then the selection (`select.drop`); Esc in a field only cancels its edit. The exit hint shows "Q / Escape: drop tool" (Space opens Mini Properties). Provisional: SCQ-ESC-1/2.

## 2026-11-22 — Animation phases 1 and 2 (branch animation-time-keys)

- Time and keys: scene time settings, key curves with Smooth Flat (TK-13 values to 1e-4), slope types, keys on transforms and item channels, status lights, S / Shift+S / Shift+W/E/R/Y, right-click key menu, Auto Key Off/Animated/All, edited-not-keyed lights with Apply/Discard, playback and stepping, Cycles renders the current pose.
- Timeline: ruler, range bar, key time, markers and range markers, track row with key select/move/scale/copy/paste/slopes, Animate layout and menu, Render Frame Range and Playblast, USD export of animated transforms, cameras and lights as time samples. Every action is a command the MCP bridge can run; `operandi.anim.query` reads everything back.
- Provisional: see SPECIFICATION_QUESTIONS.md, "Animation phases 1-2". Graph editor, rigs and USD import of animation are later phases.

## 2026-11-22 — Forum wireframe fix (branch forum-wireframe-gizmo)

- Wire edges no longer vanish near the camera: an edge with one end behind the near clip is cut there instead of dropped, on the GPU wire (`edge_out_fading`) and the CPU overlay lines (`Camera::project_segment`). After a wheel zoom to a surface the near clip now scales with the view distance, so moving far away again keeps depth precision (the wire stayed sunk in its polygons before).
- Not changed: the gizmo report needs a design choice (FG-Q01, FG-Q02).

## 2026-11-22 — Animation phase 3: graph editor (branch animation-held)

- Graph Editor window: channel list with filters and Lock, curves coloured by axis, keys (broken magenta, selected accent), handles with slope and weight, cycle ghosts, shadows, Normalised, Stacked, Isolate, Velocity, time strip, toolbar (Frame/Value fields, Key, Key Keeping Shape, slope presets, Break/Unify, Pre/Post behaviours, Cycle, Set Frame/Value, Scale), right-click menu.
- Core: weighted handles, pre/post behaviours, cycle, interpolation, Key Keeping Shape, shadows (native v61), Euler filter, Simplify, key offset/value/frame edits; all as commands the MCP bridge runs, read back by `operandi.anim.query`.
- Gaps finished (branch graph-editor-resize): Box Scale, drop copies, box zoom, Speed view, envelope presets, Stacked wheel rows, overshoot hint, retime strip, In/Out slope pop-ups, Animation Editors; each a command with a bridge test.
- GE-X7: 100 curves x 1,000 keys pans at 16.5 ms/frame (qa build probe `graph_editor_speed_probe`), just inside 60 fps; dense views draw the curve and only selected keys.
- Provisional: GE-Q01..Q09 in SPECIFICATION_QUESTIONS.md.

## 2026-11-23 — Rig phase 4, steps 1-4 (branch graph-editor-resize)

- Rig pieces (constraints: Position, Rotation, Scale, Transform, Direction, Distance, Path, Path Position, Path Normal; Dynamic Parent) and pivots as scene data (native v62); unknown kinds load as missing and save back untouched.
- Evaluation (RG-01..06): parent/dynamic parent → constraints on the world, loops refused (commands) or broken (files); results held by the engine with a per-item memo keyed on its actual inputs (only what's downstream of a change is evaluated, never stale). The evaluated scene shows them everywhere.
- 16 `operandi.rig.*` commands + `operandi.rig.query`; Animate > Constrain and Hierarchy menus; Compensation pref; driven lights and greyed fields. Bridge tests replay RG-22/28/29/44. Provisional: RG-Q01..Q08.
- Step 5 (RG-50..54): direct and relationship channel links (vectors by component, text refused, loops refused), user channels (ten types, seven modes, keyable, linkable, renamed with their keys and links, deleted with them), Bind into User Channel; Channel Links window. Provisional: RG-Q09/Q10.
- Step 6 (RG-60..63): Expression (parser by Codex from a written brief, reviewed and tested: precedence, lazy ternary, short-circuit, 29 functions, noise, error positions, depth limit), Linear Blend, Math, Clamp, Switch modifiers, read-only world/local channels. Step 7 (§8): Bake Selected (`operandi.rig.bake`), USD export samples rigged items' evaluated transforms under their parents. Provisional: RG-Q11..Q14.
- Step 8 (RG-70..72): Schematic window (Animate > Schematic) from `rig_schematic::build`, `operandi.rig.schematic` query, drag-to-wire (links, modifier inputs and outputs, add-channel ports), `operandi.rig.unwire`, right-click Delete. Review fixes: handles at the evaluated place, driven drags refused, driven fields show the evaluated value. Provisional: RG-Q15.
- Provisional: GE-Q01..Q08 in SPECIFICATION_QUESTIONS.md.

## 2026-11-22 — Sculpt respects the selection (branch sculpt-selection, issue #4)

- In Vertex, Edge or Polygon mode with components selected, every sculpt brush (Shift-smooth, Ctrl, Move/Emboss, symmetry, Vertex Map brushes) moves only the selection's points; unselected points join the frozen set, so locks and hidden points work as before (SCS-01..09). Nothing selected or Item mode sculpts the whole mesh; with several active meshes, one without a selection is left alone.
- A selection of nothing but locked or hidden points starts no stroke ("nothing to sculpt on the active meshes").
- Step 9 (§9): the four milestone rigs as tests (piston rod aiming at a crank pin turned by a keyed user channel through a link; wheels from an expression on the car's distance; a camera on a bent path at constant speed aiming at a target; the robot arm hand-off with no pops either way and the same motion after Bake). The hand-off exports to USD rigged and baked, and Blender's OpenUSD reads the same positions (1e-4). Evaluated copies keep the hierarchy and carry their document scene so exports sample the rig at every frame.
## 2026-11-22 — Python scripting preview (branch python-scripting)

- Embedded CPython (PyO3 0.30.1, crate operandi-python, feature `python`), one Python thread; scripts call back into the app between frames, so the window stays live and Stop works. Errors end the script with a traceback, never the app.
- `operandi` module: run (the command's own result, CommandError on failure), query, undo_group, mesh_get/mesh_set (lists or buffers of doubles), set_time/set_channel/set_key. A script is one undo step (UIC-227).
- Script Console (floating window), System > Run Script…, Scripts folder, commands operandi.python.console/run/runScript/stop and operandi.mesh.setData; MCP `python_run`.
- Hidden behind Preferences > Experimental > Python scripting (preview) or OPERANDI_PYTHON=1 (python_switch.rs, off by default).
- Bundled CPython: scripts/fetch-python.sh + bundle-dev.sh/macos-app-bundle.sh; release signing, Windows and Linux not done (docs/PYTHON_SCRIPTING.md). Provisional: PY-Q01..Q04.

## 2026-11-23 — Rig phase 5 (branch graph-editor-resize)

- Step 1 (SK-01..03): Setup mode (`operandi.anim.setup`, Animate > Setup, timeline toggle, yellow view outline): items show setup values, keys are ignored, edits change setup values; Item to Setup and Restore Setup. Provisional: SK-Q01/Q02.
- Step 2 (SK-10..19): joints are Joint locators (skeleton.rs); `operandi.skel.add/insert/delete/move/align/query`; symmetry and _L/_R naming; Z-down-the-bone alignment; Skeleton tool (timeline toggle, options window) with Add/Edit/Insert/Delete, Intersection placement, Compensation, one step per click or drag; joints draw on top as rings plus a bone to the first child. Not yet: the item Display tab's joint shape choices (SK-18). Provisional: SK-Q03..Q05.
- Step 3 (SK-30..36): skin MeshOp `operandi.skin.item` (skin.rs): joints link into the stack, weights from the joints' point_weights maps, normalised; Local/Linear blend and Dual Quaternion; Threshold, Sharpness, Limit Sources, Smooth influences; bind pose = joints' setup pose (setup channels on bound joints), so Setup shows rest and moving joints there moves the bind pose (SK-51). `operandi.skin.set/bakeWeights/query` (deformed position at a time). SK-33 numbers exact; SK-52 7.6 ms a frame (50k points, 60 joints, release). Not yet: deformer-list expansion of influences, Freeze menu entries (SK-36 uses the generic Mesh Stack rows). Provisional: SK-Q06..Q08.
- Step 4 (SK-20..25): `operandi.skin.bind/unbind/rebind` (skin_bind.rs), Setup-only (SK-02 message); Rigid, Smooth (Distance), Smooth (Visible, BVH line-of-sight) and Heat (bone heat: cotangent Laplacian + point areas, own conjugate-gradient solve); Falloff, Segments (a joint with several children is one bone, leaves a radius), Limit Weights, Minimum Weight, Dual Quaternion; Heat refuses loose points, loose edges, non-manifold edges, overlapping shells and joints outside, changing nothing and selecting the points; Bind dialog (Rig tab Bind, Shift+click Unbind) with the Use Smooth (Distance) fallback. Provisional: SK-Q09.
- Step 5 (SK-40..45): brushes are the Vertex Map Tools; for a joint map their Normalize works across the skin's maps only (SK-43); weight edits pass the Mesh Stack guard and a skinned mesh keeps its point selection; `operandi.weight.set/adjust/isolate/smooth/normalize/mirror/merge/split/transfer/query`; selecting a bound joint selects its map (SK-40); joint renames rename their maps (SK-23); timeline Weights menu. Not yet: Adjust Weights as a drag tool with Show Weight numbers, the skin's all-joints colour view (SK-40), the delete-map question (SK-45). Provisional: SK-Q10.
- Step 6 (§7): UsdSkel export (rest mesh + UsdSkelBindingAPI weights, geomBind, skinning method; Skeleton with bind/rest transforms and SkelAnimation; /root as SkelRoot; bridge ABI 11) and import (joints, keys, weight maps, skin). OpenUSD's skinning matches Operandi's (linear and DQ); export → import is exact.
- Step 7 (§8): milestone checks 1–2 in core tests; check 3 (`blender_round_trip`) opens the export in Blender 5.2: same armature, same bend at frames 0/12/24, and Blender's own export reads back with the same joints and deformation. Blender ignores and doesn't write skinningMethod, so Dual Quaternion comes back from Blender as Linear (Operandi→Operandi keeps it). GUI checklist: ANIMATION_GUI_QA.md 36–43. Provisional: SK-Q11.

## 2026-11-23 — Rig phase 6, IK and posing (branch graph-editor-resize)

- Step 1 (IK-01..08): `ik.twoBone` rig piece on the chain root (ik.rs, analytic solve; the root's evaluation sets the middle/end joints' worlds), `operandi.ik.applyTwoBone` (Setup only; goal box under `<root>_IKRoot`, coloured by side), `operandi.ik.pole` create/assign/remove, Orient, Soft, Stretch (base/mid), Blend by slerp, `operandi.ik.query`. IK-04 numbers exact in unit, scene and bridge tests; the knee never flips. Timeline IK menu. Provisional: IK-Q01..Q04.
- Step 2 (IK-15..18): `ik.chain` FABRIK solver (Iterations, Tolerance as a fraction of chain length, Root Locked, Pole, Twist Spread, Blend), `operandi.ik.applyChain` (2–100 joints, Setup only). IK-18 cases exact in unit, scene and bridge tests. Provisional: IK-Q05.
- Step 3 (IK-40..43): per-joint IK settings and limits (`Rig::joint_ik`, ik_joint.rs; `operandi.ik.jointSet/jointQuery`, degrees in commands), soft wall turning hard at the value in both solvers, Limit FK clamps edits, the selected limited joint draws its fans. Provisional: IK-Q06.
- Step 4 (IK-30..33): Blend keyable (slerp); `operandi.ik.switch` (toggle/fk/ik; to FK keys the joints, to IK moves and keys goal and pole, Q19), `setKeys` (delete), `selectSwitch`, `selectSolvers`, `disable` (Setup only, saved), `updateRest`, `match`. No pop either way in scene and bridge tests. IK menu label follows the selection (Switch to FK / to IK / Toggle). Provisional: IK-Q07.
- Step 5 (IK-20..27): `ik.body` full-body solver (ik_body.rs): damped least squares with pins first and goals in their null space, joint settings (active, axes, weights, limits with Limits Strength, Max Joint Velocity), Translate Root / Root Weight, Task Strength, Precision, Tolerance, Steps with early stop; `operandi.ik.assign/addGoal/assignGoal/addTask/removeTask/taskSet/addToChain/removeFromChain/remove`; the query lists every task's error and met. Not yet: Solve Continuous (stored; every frame starts from FK), Draw Chains display, the Freeze button in the scale refusal. Provisional: IK-Q08.
- Step 6 (IK-50..54): `operandi.pose.drag` (ik/rigid/twist/fk, pins, root weight, pin weight/precision; a scratch full-body solve whose answer is written into the joints' channels, keyed with Auto Key) and the Pose tool (timeline Pose toggle, circles white/yellow/red, drag, Ctrl work plane, Shift twist, middle FK, right-click pins, options window); a drag is one undo step. Not yet: Ctrl+right-click persistent pins, the mouse map in the editable key map (KM). Provisional: IK-Q09.
- Step 7 (§7): Bake Selected on any joint of an IK chain bakes the whole chain's solved pose into keys and keys the solver's Blend to 0 (FK) instead of disabling it; UsdSkel export writes the solved pose as the SkelAnimation (no solver, goal or task prims) and reads back deforming the same. Every §7 MCP command and query exists, with bridge tests for IK-04, IK-18, IK-31, IK-41, full body and pose drag.
- Step 8 (§8): the five milestone checks are automated (rig_phase6_milestones.rs: leg plant with pole sweep, IK/FK switch scrub, full-body lean with planted feet, pose-tool crouch with Auto Key, 12-joint FABRIK tail on a path); GUI checklist ANIMATION_GUI_QA.md 44–51. Milestone 3 gives the foot goals Weight 10 (goals share one priority; pins come first). Icons still needed: Pose tool, Assign IK, Switch IK/FK.

## 2026-11-23 — Rig rows in the Items list (branch rig-items-list)

- Rig pieces (constraints, dynamic parents, links, modifiers, IK solvers) show under their item; a skinned mesh shows its Skin row, which opens to one row per joint influence (RG-01, IK-02, SK-36). Click selects the item (an influence its joint), double-click renames (`operandi.rig.rename`, one undo step), the eye is Enable, right-click menus: piece Rename/Enable/Delete/Select Item, skin Enable/Bake Weights/Duplicate and Freeze/Delete (Unbind), influence Enable/Select Joint. Goals and poles are items already. Provisional: RG-Q90.

## 2026-11-23 — Rig tab (branch rig-tab)

- The Animate layout's toolbox has a Rig tab (`assets/forms/toolbox.rig.json`, UIC-190..201): Setup, Skeleton and Pose tools; constraints (flyout), hierarchy, Compensation; Channel Links, Schematic, Bake; Bind (Shift: Unbind), Rebind, the weight tools, Bake Weights; IK (two-bone, chain, full body, goals, tasks, up vector with Shift: Remove, Switch IK/FK, Set/Delete Keys, selects, Match, Update Rest Pose, Disable, Remove). Every button is an existing command (tested). The temporary timeline Skeleton/Pose/Bind/Weights/IK buttons are gone; Setup and the phase-4 rig buttons stay on the timeline. Icons still needed (text buttons now): Unparent, Add Parent, Compensation, Add Joints to Bind, the six weight tools, Add Position Task, Add Orientation Task, IK/FK Set Keys, Select Switch Channels, Select IK Solvers, Match Goal, Update Rest Pose, Disable IK, Remove IK.

- Rig tab icons (branch rig-tab-icons): the 19 rendered Rig tab icons (icons-rig) set on every remaining Rig tab button; Dynamic Parent rows in the Items list use add-parent.
- Pick order (branch rig-pick-order): Ctrl/Shift-clicking an item makes it the primary; the earlier picks stay in pick order, so Parent, Constrain, Channel Links, Bind and IK go the way you picked (Passenger then Car: Car is the parent). Items-list range: the clicked end is primary. Deselecting the primary promotes the latest remaining pick. Merge, Parent to Group Locator and New Group still go by the first pick. The Rig tab's Compensation lights while on. RG-Q92.

## 2026-11-23 — Shift+double-click empty space expands the selection (branch shift-dblclick-expand)

- Vertex, Edge and Polygon modes: Shift+double-click where both presses hit no component grows the selection: Vertices and Polygons add everything connected to it (Select Connected, per piece), Edges add each selected edge's full loop (Select Loop; seeds kept first). Adds only, one undo step (the first press missed, so it records nothing); symmetry and every active mesh as those commands do. Nothing selected: status hint, no change; Item mode: nothing. Plain/Shift clicks, component double-clicks, Shift+drag unchanged. Jason's request, not from a spec (empty_expand.rs). The gesture isn't in the editable key map (no viewport mouse gesture is yet). GUI check PS-06d.

## 2026-11-23 — Preference checkboxes redraw (branch prefs-checkbox-refresh)

- Bug: Preferences > Defaults > Application > Experimental's Python scripting (and Animation when turned off) switched the feature but the box didn't tick. The page's row edits go back through `pref.value`, and its diff paired the group's JSON leaves by position; a field left out of the JSON at its default (`skip_serializing_if`) shifted the pairs, so the click was undone. Now the diff goes by path (and keeps the row's edit if the leaves still differ), the Application group writes every field, and `pref.value application.python_preview/animation_preview` applies the switch too. With OPERANDI_PYTHON / OPERANDI_ANIMATION set, the box shows the forced state, greyed, saying why (UIC-92, UIC-117). Tests click every checkbox on every Preferences page (prefs_checkbox_tests.rs) and flip every on/off leaf of every group through `pref.value`. No other box was broken.
## 2026-11-23 — Multiresolution sculpting (branch claude/project-thread-0cmp4g)

- Sculpt tab, Multiresolution section: Add Level (Catmull-Clark; the first makes the mesh multiresolution with its shape as the base), Level Down / Level Up, Delete Higher Levels, Apply Multiresolution, and a "Level 2 of 3 · polygons" read-out. Each is one undo step and a registry command (`operandi.multires.*`, also `operandi.multires.query` JSON for MCP).
- Every brush sculpts at any level (the item's mesh holds the level you're at). Detail is stored per level in surface frames, so detail sculpted high follows changes made lower down. Attenuate now works on a level above the base (fades its detail). Saves as native version 64 (detail packed like points); damaged detail is refused on load.
- Provisional: no behaviour spec; MR-Q01..Q08 (detail flows up only, maps beyond UVs stay on the base, locks cleared on stepping, topology edits stop stepping until Apply).

## 2026-11-24 — 3D mouse on signed Mac builds (branch spacemouse-signed)

- Bug: on signed (notarized) Mac builds with 3DxWare installed, Preferences said "SpaceNavigator found but could not be opened: hidapi error: hid_open_path: failed". The hardened runtime only loads libraries signed by Apple or by us, so 3Dconnexion's client framework wouldn't load, Operandi fell back to raw USB, and 3DxWare holds the device. Dev builds (ad-hoc, no hardened runtime) were fine. Fix: release-macos.sh signs the app with assets/macos/operandi.entitlements (`com.apple.security.cs.disable-library-validation`).
- When it still fails, Preferences says what to do: with 3DxWare, why the driver couldn't be reached (reopen Operandi or reinstall 3DxWare); without it on macOS, check Input Monitoring and quit other apps using the device. The raw error follows; hover the Device row for the full text.

## 2026-11-24 — LXO import: animation, cameras, lights and rigs (branch claude/project-thread-ahtmlr)

- Modo 16/17 scenes open with their animation: every keyed channel Operandi has becomes a key curve (exact Bezier shape, slope types, stepped side, weights, behaviours 0–8, Manual where Operandi's own slope would differ); keyed meshes, locators, cameras, lights and light materials; frame rate, ranges, time display and current time. All 55 curves of the spec's fixtures match Modo at every sampled frame (4435 samples).
- Cameras and all nine light types with their settings and light material colour; joints and skins (weight maps renamed to the joints, setup pose from the setup layer); position, rotation, scale and direction constraints, dynamic parents, channel links, Math and Expression modifiers, two-bone IK; morph maps as relative morphs, weight maps as weight maps. World matrices and deformed points match the fixtures (tests `lxo_anim_rig_spec`).
- Provisional: LXO-AR-Q01..Q05 (scale offset, a cases.json contradiction, links convert degrees, edit-layer pinned targets, IK knee side). Left out and reported: animated morph amounts, other deformers and modifiers, extra layers, pivot keys.
- Follow-up (2026-11-24): File > Import now keeps an imported scene's keys, rig pieces, user channels and pivots (renumbered), skins follow their joints, and a scene with no keys takes the file's frame rate and range; before, imported animation didn't play. Spec v2 (LXA-100..180): user channels with type, label, limits and keys; Channel Relationships as relationship links; planar IK (two bars two-bone, three bars chain IK); path constraints with HCRV curves read as open curves; matrix blends, mod/avg reported; schematic items and mesh-less influences skipped quietly. Non-UTF-8 text reads as Windows-1252. Provisional LXO-AR-Q06/Q07.

## 2026-11-24 — Channel panel (branch channel-panel)

- Channel sets (BEHAVIOR-CHANNEL-PANEL CP-01..70, native v66): named lists of channels from any items, saved with the scene with the current set. Commands `operandi.channelSet.create/add/remove/move/rename/delete/current`, each one undo step except current (view state).
- The current set's panel floats over the active 3D view (UIC-188, new rule with the maintainer's OK): `Item : Channel` rows with the Properties status light (click keys) and a live value field: drag hauls with the pointer locked on macOS (Shift fine, Ctrl coarse), one undo step and one Auto Key per drag; typing waits for Enter. Header drags the panel (kept per view), × closes it. Timeline Channel Set menu: pick, Create from selected channels, Add/Remove Selected, Rename, Delete, Show Animated. Row labels' right-click menu gains Add to Channel Set; User Channels rows now pick and key like other rows.
- Provisional: rows reorder by their ⋯ menu (no drag-to-reorder yet); the panel shows in the active view only; LXO channel-set import waits for the format (CP-90).

## Schematic workspaces and layout (2026-11-24, branch claude/project-thread-mna1o9)

- Workspaces (BEHAVIOR-SCHEMATIC): the Schematic shows one saved page at a time (`Rig::workspaces`, native v67), with commands `operandi.rig.workspaceAdd/Rename/Clear/Remove`, `schematicPlace/Remove/Arrange`, `schematicFit` and the `operandi.rig.workspaces` query. Old scenes show every rig node on an unsaved "Workspace" page; pieces on no page show on "Unplaced". Off-page wires are ringed ports; double-click brings the other end over.
- Arrange (`schematic_layout.rs`): layered by data flow, crossing-cut ordering, modifiers level with what they drive; used only for nodes with no place and by Arrange (hand-moved nodes stay). Expression nodes show their formula. Zoom about the pointer, middle/Alt pan, Fit.
- LXO import reads schematic pages (`lxo/schematic_import.rs`): workspace names and order, node places (6 pt per unit), sub-assemblies flattened; backdrops, notes and elbows counted in the report. Provisional SC-Q10..Q12.
## 2026-11-25 — Tool tab cycling (branch claude/project-thread-rjq1h8)

- Ctrl+Tab opens the next tool tab, Ctrl+Shift+Tab the previous one (the real Control key on a Mac), wrapping at the ends, in strip order; only the strip's tabs count, and one tab or no strip does nothing. Each press switches at once, repeats step one tab each, the active tool stays, fields are confirmed first (as a click). Focus on a control in the old tab's drawer goes back to the viewport; elsewhere it stays. Works with a text or number field focused; a modal dialog, menu, pop-up or pie keeps the keys, and a mouse drag ignores them. Plain Tab is unchanged. Input Map actions "Next Tool Tab" / "Previous Tool Tab" (rebindable, shown in the tab tooltips); egui's Ctrl+Tab focus stepping and the Layout Switcher's default key are gone (tool_tab_cycle.rs). Provisional: TTC-Q01..Q06. GUI check: docs/TOOL_TAB_CYCLING_GUI_QA.md.
