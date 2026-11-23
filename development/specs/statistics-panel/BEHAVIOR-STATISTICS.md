# Statistics panel — behaviour spec + answers

Received from the coordinator 2026-11-23 (behaviour only; clean room).
Tags: [ref] = reference behaviour; [LW] = optional extra; [OP] = Operandi's own choice.

## Answers to spec questions

- Q1 shortcut: none; offer W only if W is free in the keymap (report which).
- Q2 + on a row switches selection mode automatically: yes.
- Q3 hidden elements left out of counts: yes.
- Q4 cleanup checks in their own "Checks" group: yes.
- Q5 "Not flat" tolerance: same flatness setting the Flatten tools use; a preference if none exists.
- Q6 show "Concave" instead of "Convex".
- Q7 Items by type: only types present in the scene.
- Q8 Paste on a selection set: add members.
- Q9 per-element Info page: NOT in this build.
- Q10 Filter row: BUILD it in v1.
- Q11 Hide/Unhide in row menu: yes.

## 1. What it is

A panel that counts the geometry you are working on, sorted into useful
groups (by polygon size, by how many edges meet at a vertex, by part,
material, selection set and so on), and lets you select or deselect a whole
group with one click. Both a "how heavy is this" readout and a cleanup tool:
find triangles, n-gons, stray vertices, open borders, non-planar polygons,
and select them instantly.

## 2. Opening it

- A normal panel, registered in the panel-type registry under the "info"
  category (UIC-156/157), so it can be added as a tab, torn off (UIC-71), or
  opened as a popover (UIC-61).
- A menu entry under the View/Windows menu opens it as a floating window
  (about 400 x 600).
- Default shortcut: see Q1.

## 3. Layout

Tree list (UIC-131) with columns, left to right:

| Column | Meaning |
|---|---|
| Add "+" | click: add every element of this row to the selection |
| Remove "−" | click: remove every element of this row from the selection |
| Name | group name, disclosure arrow if it has sub-rows |
| Count "#" | how many elements are in this group |
| Sel | how many of them are currently selected |

- + and − are narrow state columns at the left (UIC-135). Count and Sel are
  value columns at the right.
- Heading-only rows (e.g. "By polygon size") have no +/− and no numbers.
- Rows with a count of 0 still show +/− (clicking does nothing).
- A whole group that does not apply right now (e.g. Items, when nothing to
  count) is drawn dimmed (UIC-141).
- A named category with nothing in it (no selection sets yet) shows no
  disclosure arrow.

## 4. The tree — four top-level groups

Every row with a count is clickable with +/−.

### 4.1 Vertices (top row: count of all vertices)
- By edge count: 0, 1, 2, 3, 4, more than 4 (vertices with exactly that many
  edges attached; 0 = stray/loose; 2 = mid-edge-run; >4 = poles).
- By polygon count: 0, 1, 2, 3, 4, more than 4 (vertices used by exactly that
  many polygons).
- Border > In a straight line: vertices whose two edges are in line (angle
  under 0.01°), so the vertex could be removed.
- By vertex map: one sub-group per map kind Operandi has (weight/subdivision
  weight, UV, morph, etc.), each listing its maps; count = vertices with a
  value in that map.
- By selection set: one row per named vertex selection set.

### 4.2 Edges (top row: count of all edges)
- By polygon count: 1, 2, 3, 4, more than 4 (1 = open border; 2 = normal;
  3+ = non-manifold).
- Border: Part (edges between polygons in different parts); Material
  (between different materials); Geometry (open border, one polygon); UV (UV
  seams); Flat (edge between two coplanar polygons, removable without
  changing the shape).
- By vertex map: edge-weight maps (subdivision edge weight).
- By selection set: one row per named edge selection set.

### 4.3 Polygons (top row: count of all polygons)
- By type: one row per polygon type Operandi actually supports (faces, SubD,
  curves/kinds, patches, etc.). Only list types Operandi has.
- Checks (own group, Q4): Single point (one vertex); Not flat (non-planar
  beyond tolerance, Q5); Overlapping (exact duplicates of another polygon's
  vertex positions); Flat with a neighbour (coplanar with at least one
  neighbour; merge candidates); Sharp angle (any corner under 1°, slivers);
  Concave (has an inward-pointing corner, Q6).
- By vertex count: 1, 2, 3 (triangles), 4 (quads), more than 4 (n-gons). The
  most-used rows.
- Part: one row per part name. Material: one row per material tag used. By
  selection set: one row per polygon selection set. Smoothing groups: one row
  per smoothing group (if Operandi has them).
- Drawn (GPU): Active and Background: polygons actually drawn in the viewport
  for active meshes and background meshes, after subdivision. Read-only, no
  +/−. Reuse the "GL:" readout count.

### 4.4 Items (top row: count of items)
- By type: one row per item type present (Q7). +/− add/remove those items
  to/from the item selection.
- By selection set / group: one row per item group.
- Count = items in scene; Sel = selected items.

## 5. What gets counted
- Geometry rows count the active meshes (selected mesh items, or every mesh
  in component mode if nothing is selected — match how the 3D view decides
  what is editable). Several active meshes: totals across all.
- Hidden elements: not counted (Q3).
- Background meshes count only in Drawn > Background.

## 6. Selecting from the panel
- + adds the row's elements to the current selection; − removes them. No
  modifiers needed. Each is one undo step.
- + on a top-level row (Vertices/Edges/Polygons) selects all of that type; −
  deselects all.
- Selection mode: + switches to the row's mode automatically (Q2). − in a
  different mode just removes those elements from that mode's stored
  selection, no switch.
- Combining: + on "4" then + on "Part: Wheel" = union. Intersection comes
  from the Filter row (section 8).
- Clicking a row's name selects the row in the list (UIC-30), which the
  context menu acts on. It does not touch the 3D selection.

## 7. Row context menu (right-click, UIC-144)
On rows for named things (parts, materials, selection sets, vertex maps,
smoothing groups):
- Rename: renames it everywhere; also slow-double-click rename in place
  (UIC-39). One undo step.
- Delete: removes that tag/set/map from all elements (elements stay).
- Copy / Paste (selection sets and vertex maps): Copy remembers
  membership/values; Paste writes them into the right-clicked row (sets: add
  members, Q8; maps: overwrite values).
- Clear (vertex maps): sets every value to zero/empty, keeps the map.
- Hide and Unhide the row's elements. Select only (replace selection with
  this row).
On generic rows (by count, by type, checks): Select only, Add, Remove, Hide,
Unhide.

## 8. Filter row (build in v1)
Tool row (UIC-133) at the top: dropdowns "Part [(none) ▾]", "Material
[(none) ▾]", "Set [(none) ▾]". When one is set, every count and every +/− is
limited to elements in it. So Material: Glass + "+" on "4" = select the quads
of the glass only.

## 9. Updating
- Live: counts and Sel update after every edit, undo, redo, selection change
  and active-mesh change. ALL rows update (not only the clicked row).
- Heavy scenes: counting must never stall the viewport. Computed off the UI
  thread, at most a few times a second while dragging a tool, and once when
  the tool stops. Only expanded groups computed in full; collapsed groups
  just their top count. Expensive checks (not flat, overlapping, flat with
  neighbour, sharp angle) only when their group is expanded; cell shows "…"
  while computing. Target: 5M-poly mesh, whole panel expanded, under 300 ms
  to refresh, viewport stays interactive.
- Expanded/collapsed state of each group remembered with the layout.

## 10. Edge cases
- No active mesh: geometry groups show 0 and are dimmed; Items still works.
- Empty named category: row present, no arrow (UIC-145 spirit).
- Element in several sets: counted once per set row.
- Polygon with 1 or 2 vertices: counted under vertex count 1 / 2, and for 1
  under Single point.
- Counts use thousands separators (8,832), like the GL overlay.

## 11. Acceptance checks
Test mesh: one quad, two triangles, one five-sided polygon, four loose
vertices (15 vertices, 4 polygons, 13 edges).
1. Vertices 15, Edges 13, Polygons 4. By vertex count: 3 → 2, 4 → 1, >4 → 1.
2. Vertices by edge count: 0 → 4.
3. Edges by polygon count: 1 → 11, 2 → 2. Border > Geometry 11, Flat 2.
4. + on "3" selects exactly the two triangles; Sel shows 2. + on "4" adds the
   quad (3 selected). − on "3" leaves only the quad.
5. + on a vertex row while in polygon mode switches to vertex mode and
   selects them.
6. Rename a part from the panel: the name changes on its polygons; one undo
   step reverts it.
7. Filter: Material = M, + on "4" selects only quads with M.
8. 5M-poly mesh: dragging a vertex keeps the viewport at full speed with the
   panel open and fully expanded.
