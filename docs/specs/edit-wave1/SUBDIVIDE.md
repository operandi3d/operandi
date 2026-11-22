# Subdivide group and Center: behaviour spec v1 (wave 1)

Provenance: the reference application's user guide (Subdivide, UnSubdivide, Center pages) for names and intent; its plain-text UI configuration for the button order, which method each button runs and the shortcuts; Jason's screenshots of the tab; and about 75 scripted black-box cases run on 2026-11-08 (small cubes, grids, a triangle, a pentagon, a pyramid; exact positions read back to six decimals). Oracle data: `cases.json`. REA-assisted: no.

Read with: `subdivision-2026-11-03` (the subdivision-surface rules and Freeze), `edge-weights-2026-11-05`, `mesh-commands-2026-11-02` (Edge Subdivide), `toolbox-panels-2026-11-06/EDIT.md` (the rest of the tab).

Status tags: **Observed** (measured), **Docs** (from the guide only), **Provisional** (Operandi's choice until measured).

## 1. Panel layout

**SUB-01 (Observed in screenshots).** Under the Center row, a section titled **Subdivide** with five buttons, top to bottom: **SDS Subdivide**, **Faceted**, **Smooth**, **Options...**, **UnSubdivide**. A divider follows, then a **Freeze...** button, then the Booleans & Drill section. (CORRECTED 2026-11-08: these buttons have icons; Operandi draws its own.)

**SUB-02.** Tooltips, in our own words: SDS Subdivide "Subdivide the selected polygons once and smooth them (Catmull-Clark). Alt+click: twice"; Faceted "Split the selected polygons without changing the shape"; Smooth "Split the selected polygons and round them off, keeping sharp edges"; Options... "Subdivide with all options (Shift+D)"; UnSubdivide "Undo a Catmull-Clark subdivision by merging quads back".

**SUB-03 Shortcuts (corrected 2026-11-08).** D and Shift+D depend on the selection mode.

- **D**: Edge mode = Edge Subdivide (as now). Vertex/Polygon mode = SDS Subdivide (Catmull-Clark). Item mode = SDS Subdivide on all polygons of the selected mesh items.
- **Shift+D**: Vertex/Edge/Polygon mode = open the Subdivide Options dialog. Item mode = Instance (a linked instance of each selected item, with its children). Operandi has no instances yet, so in Item mode Shift+D shows a status message like "Instance isn't available yet" and does nothing else.
- The Options... button keeps "Shift+D" in its label/tooltip (UIC-04, UIC-22) even though the key does something else in Item mode.
- Not this build (log for the keymap): Ctrl+D duplicate with children, Ctrl+Alt+D duplicate the item alone, Shift+Alt+D instance the item alone.

**SUB-03a Instance (future job, open question only).** Shares the source mesh, has its own transforms, starts at the source's position/rotation/scale/parent, takes shading from the source, shows italic in the Item List with its own icon; only the new instances are selected afterwards; one undo step.

## 2. One command, four methods

All four buttons and the dialog run one command, **Subdivide Polygons**, with a **Method**:

| Button | Method it runs | What it does |
|---|---|---|
| SDS Subdivide | **Catmull-Clark** | One Catmull-Clark step (SUB-10) |
| Faceted | **Faceted** | Splits, nothing moves (SUB-20) |
| Smooth | **Smooth** | Splits, new points pushed out (SUB-21) |
| (dialog only) | **SDS Subdivide** | Catmull-Clark step that keeps sharp edges (SUB-11) |

**SUB-04 Naming trap (Observed).** The toolbox button labelled "SDS Subdivide" runs the method the dialog lists as **Catmull-Clark**, not the dialog's method called "SDS Subdivide". Operandi copies this exactly: same labels, same mapping.

**SUB-05 Buttons reuse the dialog's other settings (Observed).** A button sets only the Method. Every other option (Fractal, Max Smoothing Angle, Linear UVs, Boundary Rules, Background Constraint, Ignore Edge Weights) keeps the value last used, whether set in the dialog or by an earlier run. Settings persist for the session (Provisional: also across restarts, like other tool settings in Operandi).

**SUB-06 Alt+click on SDS Subdivide (Docs/config).** Runs the Catmull-Clark method twice in a row. Two applications give exactly two Catmull-Clark steps (Observed). Provisional: one undo step for the pair.

**SUB-07 Undo.** Each apply is one undo step.

## 3. Which polygons

**SUB-08 (Observed).** In Polygon mode the selected polygons are subdivided; with no polygons selected, every polygon of the active mesh is.

**SUB-09 (Observed).** In Vertex or Edge mode with elements selected, the command does nothing and reports "No polygons selected" (Operandi: status message). Provisional: Vertex/Edge mode with nothing selected also does nothing.

## 4. Catmull-Clark (the SDS Subdivide button)

**SUB-10 (Observed, exact).** One standard Catmull-Clark refinement step:

- every polygon with *n* sides becomes *n* quads around a new centre point (quad -> 4, triangle -> 3, pentagon -> 5);
- centre point = average of the polygon's corners;
- interior edge point = average of the edge's two ends and the two neighbouring centre points;
- interior original point of valence *n* moves to (F + 2R + (n - 3)P) / n (F = average of the surrounding centre points, R = average of the surrounding edge midpoints, P = old position);
- open-boundary edge point = edge midpoint; boundary point = 3/4 P + 1/8 (sum of its two boundary neighbours), unless Boundary Rules say otherwise (SUB-12).

This matches all cube, grid, triangle, pentagon and mixed-pyramid cases to six decimals. It is **not** the same as "turn on subdivision level 1 and Freeze" (that adds the patch-sampling step of SDV rules). The Max Smoothing Angle is ignored by this method (Observed).

**SUB-11 SDS Subdivide method (dialog only; Observed).** The same step, but an edge whose two faces meet at more than **Max Smoothing Angle** is treated as a crease: its edge point is the plain midpoint; an original point on exactly two crease edges uses 3/4 P + 1/8 (sum of the two crease neighbours); a point on three or more crease edges stays put. With the angle at or above every edge's angle it equals SUB-10; a cube at 89 degrees is split without moving (all edges creased), at 91 degrees it equals SUB-10. Boundary Rules are ignored by this method (always the boundary rule of SUB-10). For non-planar polygons the angle seems to be measured from the faces near the edge rather than from an averaged polygon normal (one case; SUB-Q4).

**SUB-12 Boundary Rules (Catmull-Clark method; Observed).** "Valence" counts the edges at the point in the whole mesh.

- **Smooth All**: SUB-10 as written.
- **Crease All**: a boundary point with only two edges (an open corner) stays put; other boundary points as SUB-10.
- **Crease Edges**: identical to Smooth All in every case measured (SUB-Q3).

**SUB-13 Edge weights.** The Catmull-Clark method honours subdivision edge weights unless **Ignore Subdivision Edge Weights** is on (Docs/config; not measured: SUB-Q2). Provisional: weighted edges follow the existing edge-weight rules (EW-xx) for one step.

## 5. Partial selections

**SUB-14 (Observed).** Only the selected polygons are split. They are subdivided as if they were a separate patch whose outline is an open boundary (the Boundary Rule applies to it, with valence counted in the whole mesh). Consequences:

- outline points do move (by the boundary rule) and the unselected neighbours that share them move with them;
- each new outline edge point is inserted into the neighbouring unselected polygon too, so a neighbour sharing one edge gains one side (a quad becomes a five-sided polygon); no other change to neighbours.

Example (3x3 grid, middle quad only): 16 points / 9 polygons become 21 / 12, the four edge-sharing neighbours become pentagons, the corner neighbours are untouched.

**SUB-15 Selection after (Observed).** With polygons selected, exactly the new polygons are selected afterwards. With nothing selected, nothing is selected afterwards.

**SUB-16 Point order (Observed, for oracle matching only).** Original points keep their indices; then new centre points and edge points are appended. Operandi need not match the order; the oracles compare by position.

## 6. Faceted and Smooth

**SUB-20 Faceted (Observed).** No original point moves. Quad -> 4 quads (edge midpoints + centre); **triangle -> 4 triangles** (edge midpoints, one middle triangle); polygon with 5+ sides -> *n* quads around its centre point. Same partial-selection and selection rules as SUB-14/15 (outline points don't move).

**SUB-21 Smooth (Observed in part).** Same topology as Faceted. Original points never move. New edge and centre points are pushed outward to round the surface, except across edges sharper than Max Smoothing Angle, where they stay on the flat polygon (cube at 89 degrees: identical to Faceted). The exact push is not yet known (SUB-Q1). Reference values for a unit cube with the angle above 90 degrees: edge midpoint moves out by sqrt(6)/24 along each of the two face normals (from (-0.5, 0, -0.5) to (-0.602062, 0, -0.602062)); face centre moves out by sqrt(3)/6 (to 0.788675 from the middle). Provisional for Operandi until measured: place new points on a curve through the neighbouring points using the corners' smoothed normals, matching these two cube values.

## 7. Options dialog

**SUB-30 (Docs/config).** Opened by Options... or Shift+D. Fields:

| Field | Values | Notes |
|---|---|---|
| Subdivision Method | Faceted, Smooth, SDS Subdivide, Catmull-Clark | |
| Fractal | percent | random offset added after subdividing (SUB-31) |
| Max Smoothing Angle | angle | used by Smooth and SDS Subdivide |
| Linear UVs | on/off | SUB-32 |
| Boundary Rules | Smooth All, Crease All, Crease Edges | Catmull-Clark only (SUB-12) |
| Background Constraint | on/off | wrap the result onto a visible background mesh; greyed until Operandi has background meshes |
| Ignore Subdivision Edge Weights | on/off | Catmull-Clark only |

Starting values (Provisional): Method Catmull-Clark, Fractal 0%, Max Smoothing Angle 89.5 degrees, Linear UVs off, Boundary Rules Smooth All, Background Constraint off, Ignore Edge Weights off.

**SUB-31 Fractal (Observed in part).** Adds a random offset to points after the method runs; a second run on the same input gives different results (no fixed seed). Size grows with the percentage (at 50% a unit cube's points moved by up to several units). Exact distribution and which points move: SUB-Q5. Provisional: offset each new point by a random vector up to Fractal% x the polygon's size.

**SUB-32 Linear UVs (Observed).** On: new UVs are a straight linear split of each polygon's UVs; original corners keep their UVs. Off: UVs are smoothed along with the surface (a corner UV of (0, 0) moved to (0.0417, 0.0417) on an open grid, matching the boundary rule on UVs); the exact rule isn't clean in the data (SUB-Q6). Provisional: apply the same Catmull-Clark step to UVs. Faceted never changes the UV layout.

## 8. UnSubdivide

**SUB-40 (corrected 2026-11-08).** Clicking UnSubdivide opens its options dialog first (the label is "UnSubdivide...", UIC-03). OK runs it, Cancel does nothing, the values are remembered (like Subdivide's). No Shift+click variant. Fields, top to bottom:

1. Iteration: whole number, min 0, starts at 1. 0 does nothing.
2. Flat: on/off, starts off. On = merge quads without moving points.
3. UV Corner Hint: on/off, starts on.
4. Material Corner Hint: on/off, starts on.
5. Part Corner Hint: starts off.
6. Smoothing Group Corner Hint: starts off.
7. Selection Set Corner Hint: starts off.
8. Selection Set Name: pick or type a polygon selection set; greyed unless 7 is on.

Hints are tie-breakers for picking the original groups of four quads; with all hints off, a clean Catmull-Clark mesh still comes back exactly.

**SUB-40a.** In Polygon mode it works on the selected polygons, or the whole mesh if none are selected. Only selected quads merge. Unselected polygons keep their topology, but shared points move with the merged quads (Flat off). Provisional: the merged polygons are selected afterwards. Vertex/Edge mode with a selection: do nothing and show "No polygons selected" (same as Subdivide, SUB-09). Unchanged: one undo step, exact inverse per iteration, a non-quad in scope = do nothing and say so (SUB-44).

**SUB-40b (Jason, 2026-11-08).** On the outline of the merged region, an old edge-midpoint point that an unselected neighbour still uses is kept, and the merged polygon includes it in its outline: a merged polygon becomes an n-gon (5 to 8 sides) wherever it borders unselected polygons and stays a quad where all its sides are inside the selection. Neighbours keep their topology unchanged; no T-junctions, no cracks. Flat off: each kept midpoint goes to the middle of its two restored corners (the inverse of the boundary edge rule), so the edge stays straight and the neighbour moves with it; outline corners use the boundary inverse. Flat on: nothing moves. Midpoints no unselected polygon uses are removed. With nothing selected (whole mesh) nothing changes from SUB-41.

**SUB-41 (Observed).** Each iteration finds groups of four quads around a centre point, merges each group back into one quad and (Flat off) moves the corners back to where they were before a Catmull-Clark step. A cube subdivided once comes back exactly; subdivided twice, 2 iterations bring back the cube (to 1e-6) and 1 iteration brings back the once-subdivided cube. **Iterations 0 does nothing.**

**SUB-42 Flat (Observed).** On: quads are merged but no point moves (the merged cube keeps the shrunken corner positions).

**SUB-43 (Observed).** Run on a mesh that wasn't Catmull-Clark subdivided (e.g. a Faceted cube), Flat off still applies the inverse step, so the shape grows (a unit cube's corners went to +/-1.15). Not an error.

**SUB-44 (Observed).** If the polygons it would work on include anything other than quads, it does nothing and reports "UnSubdivide only rebuilds quads".

## 9. Center (rides along in wave 1)

Four small label-only buttons in one row: **X**, **Y**, **Z**, **All**; Alt+click X/Y/Z = **XY**, **YZ**, **ZX**. No dialog.

**CEN-01 (Observed).** Moves the selected elements so the centre of their bounding box sits at 0 on the chosen world axes; other axes untouched. Bounding-box centre, not the average of points (points at x = 0,0,1,1,4 end up at -2...2).

**CEN-02 (Observed).** One point: it goes to 0. One edge: its midpoint goes to 0 and only its two points move. One polygon: its points move. Nothing selected: the whole mesh.

**CEN-03 (Observed).** Works in world space: in a mesh item moved to X = 10 the geometry is moved so its world-space centre is at 0 (12 units in that case, not 2).

**CEN-04 (Docs).** Meant for components; in Item mode Operandi does nothing and shows a status message (Provisional). One undo step.

## 10. Freeze... and Slide (wiring only)

**SUB-50 (corrected 2026-11-08).** Freeze... opens a dialog. OK runs it, Cancel does nothing, values are remembered. Fields, top to bottom:

1. Freeze Curves as: dropdown (2 Point Polygons / Faces / Polylines), starts at Faces.
2. Make Holes: checkbox, off.
3. Drill Axis: X/Y/Z choice, greyed while Make Holes is off.
4. Freeze Displacement: on.
5. Freeze Deformation: on.
6. Tessellation: on. On = freeze subdivision at display level (Operandi's existing Freeze); off keeps the cage.
7. Subdivide Curves with Fixed Spans: on.
8. Curve Refinement Angle: 5 degrees, greyed while Fixed Spans is on.
9. Save Deformation to Morph: off.
10. Morph Map Name: text, empty, greyed while 9 is off.

For now Tessellation drives the existing subdivision Freeze. Operandi has no curve freezing, so the curve fields, Displacement, Deformation, Morph and Make Holes/Drill Axis are shown with their starting values, disabled but readable, with the tooltip "Not available yet" (UIC-04).

**SUB-51.** The large Slide button at the top of the tab starts Edge Slide in Edge mode and Vertex Slide in Vertex mode; greyed in Polygon and Item mode until Polygon Slide exists.

## 11. Open questions

- SUB-Q1 Smooth: exact rule for the new points' positions.
- SUB-Q2 Catmull-Clark with edge weights, and Ignore Edge Weights.
- SUB-Q3 Crease Edges: when it differs from Smooth All.
- SUB-Q4 SDS Subdivide angle test on non-planar polygons.
- SUB-Q5 Fractal distribution.
- SUB-Q6 Linear UVs off: exact UV rule.
- SUB-Q7 First-run values of every option.
