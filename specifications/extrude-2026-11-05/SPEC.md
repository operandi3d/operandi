# Edge Extrude and Vertex Extrude — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

Polygon Extrude is specified in `specs/polygon-extrude-2026-11-02` (modes,
odd input, non-manifold input). Nothing new about Polygon Extrude was
observed in this round, so nothing is added here.

Fixtures used below (all in the XY plane, facing +Z):
- **Strip**: 2×1 quads, points x ∈ {−1, 0, 1}, y = ±0.5.
- **Grid**: 2×2 quads, points x, y ∈ {−1, 0, 1}.
- **Cube**: unit cube centred on the origin, outward faces.

## Shared options

Both tools have the same four options (D0). Values after reset:

| Option | Type | Default | Effect |
|---|---|---|---|
| Extrude | distance | 0 | How far the edge or vertex moves along its normal. Negative moves it the other way. |
| Width | distance | 0 | How far the new edges sit from the original edge or vertex, measured along the neighbouring polygon edges. |
| Use Material | boolean | off | Tag the new polygons with Material Name. |
| Material Name | string | empty | Material tag for the new polygons. |

With the defaults the tools change nothing (EE01, VE01). **Width 0 changes
nothing even with a non-zero Extrude** (EE02, VE06). Width is what creates
geometry.

Handles, live preview and undo were not probed (Q-EX-01, Q-EX-02).

---

## Edge Extrude

### Summary

Raises each selected edge into a ridge. The edge is replaced by two new
edges, one slid into each neighbouring polygon by Width. The original edge
position becomes the ridge, moved by Extrude, and two new quads join the
ridge to the slid edges. The neighbouring polygons shrink but stay in their
own planes.

### Inputs

- Edge selection mode.
- Selected edges. An edge with two polygons (interior) is the main case.
- An edge with one polygon (border) gives a different, degenerate result
  (below).
- **Nothing selected: every edge in the mesh is processed** (EE07),
  interior and border edges alike.

### Result: interior edge (EE03, EE06, EE08, EE09)

For a selected edge between polygons A and B, with Width w and Extrude e:
- **Slid edges.** In A and in B, the edge is replaced by a copy slid into
  that polygon. Each end of the copy sits on the polygon's other edge at
  that corner, at distance w from the corner.
  - Strip middle edge, w 0.05: slid copies at x = ±0.05 (EE03).
  - Cube top-front edge, w 0.05: copies at z = 0.45 on the top face and at
    y = 0.45 on the front face (EE06).
- **Ridge.** The original edge moves by e along the normalized average of
  A's and B's normals.
  - Strip, e 0.1: ridge at z = 0.1.
  - Cube edge: ridge at y = z = 0.570711.
  - e −0.1: ridge at z = −0.1 (EE09).
- **New polygons.** Two quads, each joining one slid copy to the ridge.
  They face the same side as A and B (outward for the cube).
- **Edge ends.** What happens at each end vertex:
  - Other polygons meet there (cube corner): each of those polygons gains
    the slid end points in place of the original corner, so a quad becomes
    a 5-gon. One triangle closes the gap between the two slid ends and the
    ridge end. The triangle lies in the plane of the polygon it borders
    (EE06: 4 new points, 2 ridge quads + 2 end triangles; the two side
    faces become 5-gons).
  - The end lies on the mesh border: no triangle. The slid ends lie on the
    border edges, and the original border vertex is no longer used (EE03,
    EE05).
- **Connected selected edges in a line** (EE05: two grid edges meeting at
  the centre) form one continuous ridge with a shared middle ridge vertex.
  The slid copies meet at points w along the crossing edge: (0, ±0.05).
  Result: 15 points, 8 polygons.
- **Material.** With Use Material on, only the two ridge quads per edge get
  the tag (EE08). Shrunken neighbours keep theirs. Otherwise new polygons
  carry Default.

### Result: border edge (EE04)

Selecting a strip border edge (one polygon) with e 0.1, w 0.05 creates no
polygons and no ridge; Extrude has no visible effect.
- The edge's polygon is cut back by w: its border edge moves to y = −0.45.
- The other polygon at that edge's inner end gets a notch: its corner
  (0, −0.5) is replaced by (0, −0.45) and (0.05, −0.5), making it a 5-gon.
- The strip between y = −0.5 and −0.45 is left open.

This looks unintended. Copy it exactly or refuse? (Q-EX-03.)

### Result: nothing selected (EE07)

On the Grid every edge is processed:
- The four interior edges form a raised cross: 5 ridge vertices at
  z = 0.1, 8 ridge quads.
- Border edges get the border-edge treatment, so the four polygons become
  irregular 5-gons.
- Result: 25 points, 12 polygons. The exact mesh is in the case.

### Selection after

- Edge mode kept. Selected: the ridge edges (EE03: the one ridge edge;
  EE05: both; EE07: all four). Their vertices also read back as selected.
- Border-edge case (EE04): nothing selected.

---

## Vertex Extrude

### Summary

Raises each selected vertex into a small pyramid. On every edge leaving the
vertex a new point is made at distance Width. The vertex moves by Extrude
along its normal, and each neighbouring polygon gets a triangle joining its
two new points to the moved vertex.

### Inputs

- Vertex selection mode.
- Selected vertices. **Nothing selected: every vertex is processed** (VE08).

### Result (VE02–VE05, VE07, VE09)

For a selected vertex with Width w and Extrude e:
- **New points.** One on every edge at the vertex, at distance w from it.
- **Neighbour polygons.** Each one replaces its corner at the vertex with
  the two new points on its two edges there, gaining one side (grid quads
  become 5-gons).
- **Apex.** The vertex moves by e along the normalized average of its
  polygons' normals.
  - Grid centre: (0, 0, 0.1).
  - Cube corner: ±0.557735 (VE05).
- **New polygons.** One triangle per neighbouring polygon, joining that
  polygon's two new points to the apex. They face the same side as the
  surface.
- **Counts:**
  - Grid centre (4 polygons): 4 new points, 4 triangles (VE02).
  - Cube corner (3 polygons): 3 new points, 3 triangles (VE05).
- **Vertex on the mesh border** (VE03, border midpoint of the grid, 2
  polygons):
  - New points on both border edges and on the interior edge (3).
  - 2 triangles, apex at (0, −1, 0.1).
  - The border side is left open.
- **Vertex with only one polygon** (VE04, grid corner):
  - No apex and no triangles.
  - The corner is cut off: the polygon gets (−1, −0.9) and (−0.9, −1) in
    place of (−1, −1).
  - Extrude has no visible effect.
- **Two adjacent vertices** (VE07): each gets its own pyramid. The edge
  between them gets a new point at w from each end. Result: 16 points, 10
  polygons.
- **Material.** With Use Material on, only the pyramid triangles get the
  tag (VE09).

### Selection after

- Vertex mode kept. Selected: the moved apex vertices (VE02: 1; VE07: 2;
  VE08: 5).
- Single-polygon corner (VE04): nothing selected.

---

## Refusals

None observed: every probe applied without a message. Mesh tools refuse to
activate on an empty layer (D0).

## Acceptance cases

All in `cases.json`; compare geometry, not indices.

- EE01 defaults, no change; EE02 Width 0, no change
- EE03 strip middle edge; EE09 same with negative Extrude
- EE04 border edge (degenerate)
- EE05 two edges in a line
- EE06 cube edge (end triangles, 5-gon sides)
- EE07 nothing selected
- EE08 Use Material
- VE01 defaults, no change; VE06 Width 0, no change
- VE02 grid centre; VE03 border vertex; VE04 corner (cut only)
- VE05 cube corner; VE07 two vertices; VE08 nothing selected
- VE09 Use Material

## Open questions

See `QUESTIONS.md`.

## Open questions and decisions

- **Q-EX-01** Handles and interaction: which handle drives Extrude and
  Width, drag scaling, live preview, repeat clicks.
- **Q-EX-02** Undo: one step per apply? Selection restored?
- **Q-EX-03** Border edges (EE04) and single-polygon vertices (VE04)
  produce a cut with no new polygons, and for border edges an open sliver
  and a notch in the neighbour. Should Operandi reproduce this exactly or
  treat it as a quirk? (Edge Extrude part decided below; it affects EE07/VE08 too.)
- **Q-EX-04** Width larger than the neighbouring edges (slid points would
  pass the next vertex): clamped, overlapping, or refused?
- **Q-EX-05** Edge Extrude on edges that meet at an angle (an L of two
  selected edges, or three or more edges at one vertex): how do the slid
  copies meet?
- **Q-EX-06** Edges or vertices shared by more than two polygons
  (non-manifold).
- **Q-EX-07** Do new polygons inherit polygon type (Subdivs, Catmull-Clark),
  part, UVs and weight maps from their neighbours when Use Material is off?
  The probes only show the Default material on plain faces.
- **Q-EX-08** Use Material with an empty Material Name.
- **Q-EX-09** Vertex Extrude when the neighbouring polygons are not planar
  or the vertex normal is undefined (opposite polygons).
- **Q-EX-10** The Vertex Extrude guide page says three new vertices are
  created around the vertex; we observed one per edge (4 on a grid
  centre). Confirm on a vertex with three edges (cube corner: 3, matches).
- **Q-EX-11** Ranges and clamping of Extrude and Width (negative Width?).

## Decided (2026-11-05)

- **Q-EX-03 → decided (Edge Extrude, border edges):** build a sane ridge:
  border edges extrude into proper new polygons with no open sliver and
  no notch in the neighbour. Modo's EE04 result is not a target; EE07
  likewise where it shows the quirk. Vertex Extrude on single-polygon
  vertices (VE04/VE08) is not covered by this decision and stays open.
