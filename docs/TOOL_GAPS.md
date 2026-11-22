# Tool gap inventory

**Planning list:** `specifications/tool-matrix-2026-11-05/` (every Model
toolbox button and its spec) replaces the legacy milestone matrix cited
below. Plan new tools from it; this file stays the list of gaps in tools
already built.

Built 2026-11-01 from the code (refusals, greyed-out options, provisional
notes) and the open questions already logged. It lists what each tool does **not**
do yet. Everything not listed here is built and tested.

**How to use it.** Take one section to the Research side as a brief. Ask for
observable behavior: setup, action and result, with exact numbers (points,
faces, UVs) where geometry changes, plus defaults, option combinations and
what the reference refuses. When the spec comes back as a `spec:` commit,
the tool's next stage is built against it and the section is updated.

**Kinds of gap**

- **Missing**: an option, mode or gesture that doesn't exist yet (often a
  greyed-out control).
- **Basic limit**: the tool works only in the simple case and refuses the rest
  with a message.
- **Provisional**: built on Operandi's own reading, waiting for confirmation.
- **Spelling**: a command name chosen by Operandi (marked **P** in the M2/M3
  acceptance packets), not yet confirmed.

Question IDs (SQ-…, BV-Q…, EX-Q…) point into `docs/SPECIFICATION_QUESTIONS.md`
and the archived reports it links to; they already describe the exact setup.

## Suggested order

1. **Shared systems** (section 1). Symmetry, Snapping, Falloff and Action
   Center change how almost every tool behaves; specifying them before tool
   parity avoids reopening tools later.
2. **The tools you use most**, one per round: likely Bevel, Extrude, Edge
   Bevel/Chamfer, Loop and the transform handles.
3. **Everything else**, roughly in toolbox order.

---

## 1. Shared systems (affect many tools)

| Area | Gap | Kind | Questions |
|---|---|---|---|
| Symmetry | Built 2026-11-02 (Off/X/Y/Z + Offset; mirrored Move/Rotate/Scale; Select Symmetrical; Cube/Sphere/Cylinder doubling; Symmetrize). Topology, Arbitrary and Work Plane options, Ellipsoid/Cone and the handle on the picked side followed 2026-11-03. Missing: UV symmetry. | Provisional | SYM-Q01–Q11 |
| Snapping | No snapping; context bar entry is disabled. | Missing | SQ-007 |
| Falloff | Built 2026-11-02: Linear (shapes, Custom, Symmetric, Auto Size, Reverse) and Radial, on Move/Rotate/Scale, Bevel, Edge Bevel/Chamfer, Vertex Bevel, Vertex Extrude, Edge Slide and Spike, with draggable handles. Missing: Edge Extrude, Bridge, Smooth Shift, Selection/Soft Selection/Element and other types, Mix modes. | Provisional | FAL-Q01–Q09 |
| Action Center | Selection/Origin/Automatic and the dominant-axis rule work; Local and other centers, axis presets, automatic placement/haul and item contexts are missing or provisional. | Missing / Provisional | SQ-003, VAC-Q01–05, Q18-04 |
| Mesh Constraint | Not available. | Missing | SQ-007 |
| Work Plane frames | Set Position (Polygon/Edge/Vertex) works in local coordinates only; Work Plane and World Space are greyed out. Rotated construction planes are unsupported. | Missing | SQ-007 |
| Viewport handles | Off-handle hauling, modifier changes mid-drag and handle layouts are provisional or refused for most tools; Bridge and Spike have no viewport dragging (fields only); Cone and Ellipsoid draw by dragging since 2026-11-03. | Missing / Provisional | PM-Q02, M17-Q01–05, SA-Q01, EA-Q01 |
| Numeric fields | No mini-sliders, step clicks, drag sensitivity, linked XYZ fields or per-property presets. | Missing | SQ-006, PAR-Q02, M16-UI-Q04 |
| Tool States / presets | Named tool states exist for Bevel and Smooth Shift only; others show "not available yet". | Missing | EX-Q05, PT-Q13 |
| UVs on new points | Edge/Vertex tools, Grow/Fill Quads, Bridge and Chamfer Edit interpolate UVs by Operandi's own rule. | Provisional | SQ-UV-01 |
| Other corner data | Corner weights, colors, normals and typed corner maps are refused by most tools that add points. Polygon Reduction refuses UVs too (RED-003). | Basic limit | SQ-UV-01, EX-Q03, BV-Q04 |
| Subdivision surfaces | Polygon type is stored, but there's no evaluated subdivision display or picking. | Missing | EX-Q06, EA-Q05, SQ-009 |
| Default shortcuts | All tool keys start unassigned until the reference key map is specified. | Missing | SQ-005, SQ-M11-001 |
| Cmd+click menu | Agreed (Jason, 2026-11-01) but not built; contents need a spec per mode. | Missing | SEL-Q-M3 |
| Command spellings | Many M2/M3 command names are Operandi's own (`tool.set edge.crease`, `vert.weld`, …). | Spelling | M2/M3 acceptance packets |
| Modifier-clicks on rows | Scale Offset, Flatten Ngons, Make, Spin Quad, Unify, Split, Quadruple and Align refuse modifier-clicks. | Missing | PM-Q03 |

## 2. Selection

| Gap | Kind | Questions |
|---|---|---|
| Select Through and Previous/Next Loop are not available (the measured Previous/Next cases don't give a consistent rule yet). | Missing | SQ-009, SEL-LP-02..04 |
| Fill refuses areas that reach an open border or don't split the mesh. | Basic limit | SEL-Q-M2 |
| Selecting a row by clicking a second polygon (vs Fill on Shift+Cmd click). | Missing | SEL-Q-M4 |
| Select Between now follows the measured behaviour (2026-11-02): one route on a tie, points supported, last pair of 3+ picks. Rectangle order and the tie rule's basis are inferred. | Provisional | SEL-Q-B1, SEL-Q-B2 |
| Lasso visibility (hidden parts skipped, edges/polygons like points). | Provisional | SEL-U03 |
| Multi-item selection and editing (SEL-IT-04/05), item lasso, ranges. Parent/Unparent with Keep Position built 2026-11-02. | Missing | SQ-008, SQ-M11-004, M16-UI-Q02 |
| SubD cage picking, loop/ring edge cases. | Missing | SQ-009 |

## 3. Primitives

| Tool | Gap | Kind | Questions |
|---|---|---|---|
| All | UV layouts are Operandi's own; Distortion Compensation and Gaps are greyed out (except Cone/Ellipsoid, which have them). | Provisional / Missing | SP-Q13, CYL-Q16 |
| All | Ctrl-drag proportions, Shift-drag "draw another", kept settings and Reset follow Jason's decisions, not confirmed reference rules. | Provisional | PM-Q01, PM-Q02, BOX-Q05, BOX-Q09 |
| All | Unit-click targeting and Ctrl+Shift-click; crossed bounds (dragging past zero) refused. | Missing / Provisional | PM-Q03, PM-Q04, BOX-Q08 |
| All | Corner handles (Show Corner) and off-handle editing are unavailable. | Missing | BOX-Q04, SP-Q01/Q04, CYL-Q01/Q03 |
| Cube | Rounding (Radius, Radius Segments, Sharp, rounding Axis) and Bulge are greyed out. | Missing | BOX-Q10, BOX-Q14 |
| Sphere | Only the Globe mode; Quad Ball / Tessellated modes, Subdivision Level, other polygon types and Axis X/Z are greyed out. | Missing | SP-Q09, SP-Q10 |
| Cylinder | Only single-polygon caps and Face polygons; Edge Weight, other caps and Axis X/Z are greyed out. | Missing | CYL-Q02, CYL-Q12 |
| Cone | Bulge omitted on purpose (CA-D01); odd-sided quad-grid placement, some signs and zero sizes uncharacterized; no viewport dragging or Mini Properties. | Missing / Basic limit | CA-Q01–04, CA-Q08 |
| Ellipsoid | Three handle modes, corners, clone and reactivation; compensated UVs at zero/negative size; symmetry beyond one tested case; no viewport dragging. | Missing | EA-Q01–05 |

## 4. Polygon tools

| Tool | Gap | Kind | Questions / plan "later" |
|---|---|---|---|
| Pen | Interactive Pen has Type Polygon/Vertices/Polyline, Flip, Close and Select New (2026-11-03); selection after Drop and point minimums are Operandi's (PEN-Q-I01). Missing (greyed): other types, walls, Merge, Make Quads, Show overlays, Make UVs/Project To, Tool States. | Missing / Provisional | PEN-Q-I01 |
| Bevel | Grouped non-coplanar faces match the measured geometry (2026-11-02); Auto Weld collapse and empty selection follow the reference. Still: polygon-weight providers, adaptive profiles, sparse maps; collapsing insets without Auto Weld and zero-size walls refused (Jason's choice); index order differs. | Missing / Basic limit | BV-Q01, BV-Q02, BV-Q04, M17-Q04, PB-Q03..Q06 |
| Extrude | Measured 2026-11-02: Auto/Thick/Shift, non-planar, bowtie, two orientations, segments, zero shift and rotated items match the reference's geometry. Missing: merging points for an in-plane shift (PE-O-02), non-manifold input, empty selection in the app (PE-O-03); corner-only Weight/RGBA; Tool States. | Basic limit | EX-Q02–06, PE-Q01–03 |
| Inset / Shift | Built 2026-11-06 as live Polygon tools (modeling popup, Geometry > Polygon menu and command line; no toolbox buttons). Inset follows its spec; Shift is Operandi's reading. Missing: falloff, symmetry, Tool States; Inset's open Q-IN-02..05. | Provisional | Q-IN-02..05, TA1-Q08, TA1-Q09 |
| Chamfer Edit | Only quad strips with Source Segments 1; no Width or Absolute Size modes, profiles, side swapping or rounded sources; strips must have two clear outer polygons. | Basic limit / Missing | plan "later" |
| Smooth Shift | Pose-shape maps and one named-set case refused; angle weighting, corner options, curved patches. | Basic limit / Missing | Q-SS-01–12 |
| Thicken | Complex junctions, weighting/scale combinations, self-intersection cases. | Missing | Q-SS-01–12 |
| Spike | Hauling, modifiers and repeat; broader targets and transformed items; sparse non-UV maps. | Missing | SA-Q01–05 |
| Poly Haul | Haul quantities and defaults are Operandi's own; secondary haul modes and modifiers missing. | Provisional / Missing | plan "later" |
| Sketch Extrude | Only a fixed section: no Align to Path, Uniform, Scale, Spin, Profiles or Make UVs; no Edit Path, Delete Knot or Uniform Spans modes. | Missing | plan "later" |
| Bridge | Only two equal-count ordered sections; Auto Connection, Continuous/Close Bridges, closed chains, guide curves, curve/profile sampling and UV generation are greyed out. Chain matching tried against the measured cases 2026-11-03: no consistent rule (BRI-Q-CH01). | Missing | BRI-Q-CH01 |
| Polygon Reduction | Whole, planar meshes only (every polygon selected); Boundary/Material Border weights and Legacy Mode greyed out; UVs refused (RED-003). | Basic limit | plan "later" |
| Make | Only `poly.make auto`; other Make modes and modifier variants. | Missing | plan "later" |
| Set Position | Local coordinates only (Work Plane, World Space greyed out). | Missing | section 1 |
| Split | Successive cuts across complex regions. | Missing | plan "later" |
| Triple | Compound, self-intersecting and broader nonplanar polygons. | Basic limit | plan "later" |
| Quadruple | General region quadrangulation, ambiguous corners. | Basic limit | plan "later" |
| Spin Quad | Mixed polygon sizes. | Basic limit | plan "later" |
| Flip | Polygons with authored normals refused; curve-direction branches. | Basic limit | plan "later" |
| Align | Authored normals/tangents refused; disconnected or nonorientable regions. | Basic limit | plan "later" |
| Merge | Holes, complex and nonplanar regions. | Basic limit | plan "later" |
| Remove | Additional cleanup variants. | Missing | plan "later" |
| Collapse | Alternative collapse modes, complex regions. | Missing | plan "later" |
| Unify | Conflicting attributes, broader duplicate types. | Basic limit | plan "later" |
| Scale Offset | Detach and modifier/dropdown variants. | Missing | plan "later" |
| Flatten Ngons | Conflicting neighbor constraints, complex surfaces. | Basic limit | plan "later" |
| Set Material / Set Part | Basic material settings built (dialog); shader tree, material library and LXO material items missing. | Partial | MAT-Q01..Q09 |

## 5. Edge tools

| Tool | Gap | Kind | Questions / plan "later" |
|---|---|---|---|
| Edge Bevel / Edge Chamfer | Built 2026-11-03: Inset/Width, Round/Square/Sharp, Round Level/Segments, Depth, Use Material, Parallel, Stop At Edge, Auto Weld, Edge End; junctions, border ends, corner patches. Missing: Mitering, Coplanar, Flatness, Sharp, Sharp Corner, Boundary Extend, Boundary Coplanar, Reverse X/Y (greyed out, unmeasured). Provisional: patch inner points, Chamfer Width at 60°, the unmeasured options' readings. B runs Edge Bevel in Edge mode (KEY-01). | Missing / Provisional | EBC-Q02, Q03, Q08, Q10 |
| Edge Extrude | Only separate interior edges; general configurations and mapping. | Basic limit | plan "later" |
| Extend | Open boundary edges only; complex handles, local offsets, propagation, UV modes. | Basic limit / Missing | plan "later" |
| Edge Bridge | Curved and unequal-count bridges, automatic connections, guide curves. | Missing | M2 packet |
| Slide | Built 2026-11-06: Stop Negative Slide, Preserve Curvature/Tension, Slip UVs. Missing: Duplicate (greyed), Mode Linear/Radial as one tool, Direction, Use Handle Vector, Snapping, Relax, Projection; complex junctions, propagated regions. Loop Slide / Merge Vertices fixed on for edges. | Provisional / Missing | TA1-Q04..Q07 |
| Edge Relax | Simple strips only; propagation, custom falloff, added points, convergence. | Basic limit / Missing | M2 packet |
| Add Loop / Add Point | Built 2026-11-06: Split at the Middle, Both Sides, Absolute Distance / Offset from End / Distance 1–2, Preserve Curvature/Tension, Show Position/Distance. Missing: Keep Quads, Split, Cap Sections, Gap (greyed); branches, n-gon centre cuts. | Provisional / Missing | TA1-Q01..Q03 |
| Edge Weight (was Edge Crease) | Built 2026-11-05 with evaluated subdivision (edge-weights-2026-11-05); new edges start at 0 %, split edges keep the weight. | Built | EW-09 |
| Grow Quads | Background constraints, advanced smoothing. | Missing | plan "later" |
| Fill Quads | Partial borders, corner detection, grid mode. | Missing | M2 packet |
| Split | Gap and cap variants. | Missing | M2 packet |
| Split Normals | Intermediate hardness levels; normal-map creation and editing. | Missing | M2 packet |
| Spin / Join / Join Averaged / Collapse / Remove | Quad/ngon spin; unequal or ambiguous pairs; region collapse; boundary and nonplanar removal. | Basic limit | plan "later" |
| Edges To Curves | Branches, spline/Bezier conversion, curve editing. | Missing | plan "later" |
| Unbevel | Flat and Square modes; complex strip inference. | Missing | M2 packet |
| Edge Subdivide | Percent/Segments options, advanced patterns, difficult junctions. | Missing | M2 packet |

## 6. Vertex tools

| Tool | Gap | Kind | Questions / plan "later" |
|---|---|---|---|
| Create | Projection, snapping, other creation modes. | Missing | plan "later" |
| Vertex Bevel | Round Level above 0 refused; curves, complex valence, material overrides. | Basic limit / Missing | M3 packet |
| Vertex Extrude | Complex valence, more contexts and maps. | Missing | plan "later" |
| Merge Tool | Complex clusters, degenerate-output handling. | Missing | plan "later" |
| Weld | Whole behavior is Operandi's reading (no guide page); multiple targets, projection. | Provisional | M3 packet |
| Tear | Whole behavior is Operandi's reading (set by Jason, 2026-10-31); alternate modes. | Provisional | M3 packet |
| Vertex Slide | Separate "Vertex Slide" action beside Edge Slide (the guide describes one adaptive Slide); branching paths, propagation. | Provisional / Missing | M3 packet |
| Split Edge | Middle point on each edge between two selected points: Operandi's reading. | Provisional | M3 packet |
| Sync Index Order | Two-step capture (source, then target): Operandi's reading. | Provisional | M3 packet |
| Show Vertices | Draws points in Edge/Polygon modes too: Operandi's reading. | Provisional | M3 packet |
| Merge (command) | "Keep 1-Vertex Polygons" greyed out; other range modes. | Missing | M3 packet |
| Make Polygon | Other polygon types, curves, mixed-neighbor Auto. | Missing | plan "later" |
| Align / Reorder Index / Collapse / Remove / Remove Edge / Join | Planar constraints, complex groups and valence, boundary/nonplanar cases. | Basic limit | plan "later" |
| Vertex-only dialogs | Whether Vertex-tab dialogs should open outside Vertex mode. | Provisional | M3 packet F3–F5 |

## 7. Item transforms (Move, Rotate, Scale, Transform)

| Gap | Kind | Questions |
|---|---|---|
| Positive scale only: no negative/mirror scale, no percent entry (Jason, 2026-11-01: not for now). | Missing | SQ-004, SQ-M11-005 |
| Clone and haul, full center/axis presets, rotated item composition. | Missing | SQ-004 |
| Changing rotation order is unavailable. | Missing | M16-UI-Q04 |
| Transform's combined handles follow an approved provisional design. | Provisional | PAR-Q05 |

## 8. Workspace

| Gap | Kind | Questions |
|---|---|---|
| Exact panel sizes, theme and typography values. | Provisional | M16-UI-Q01, FONT-Q01–03 |
| Context bar popup contents and gestures. | Missing | M16-UI-Q03, PAR-Q03 |
| Scenes list columns (Edit/Ref actions), inheritance, multi-item. | Missing | M16-UI-Q02, PAR-Q04 |
| Toolbox tabs on the left, below the shared rows: Jason's deliberate departure from the reference. | Decided | — |
