# Detriangulate (v1, 2026-11-08)

Behaviour-only spec. Not a Duplicate tool: it lives on the **Polygon**
tab and in **Geometry > Polygon**. Tags as in README.md.

## 1. What it does

- DT-01 CONFIRMED (docs). Turns **pairs of triangles back into quads** by
  removing the edge they share, where the two triangles are flat enough
  and the quad they'd make is a decent shape. It's for cleaning up meshes
  that came in triangulated (imports, booleans, scans).
- DT-02 [OP]. In the reference it only shows up after another tool; in
  Operandi it's an ordinary command, always available.

## 2. How it runs

- DT-10 [OP]. A **command with an options dialog**, built with the shared
  dialog (bible section 8, UIC-80..99): the dialog opens, the user sets
  values, OK applies. Values are remembered for next time (UIC-95).
- DT-11 [OP]. Works on the **selected polygons** of the active mesh(es)
  in Polygon mode; with nothing selected, or in another mode, on the
  whole mesh. Only triangles are considered; quads and larger polygons
  are never changed.

## 3. Options

| Row | Kind | Start value | Notes |
|---|---|---|---|
| Flatness | number, percent | **20 %** CONFIRMED (probe) | DT-21 |
| Max Angle | number, degrees | **170°** CONFIRMED (probe) | DT-22 |
| Keep Material Boundaries | checkbox | **on** | DT-23 |
| Keep Part Boundaries | checkbox | **on** | |
| Keep Smoothing Boundaries | checkbox | **on** | |
| Keep Selection Set Boundaries | checkbox | **off** CONFIRMED (probe) | |
| Keep UV Seams | checkbox | **on** | |

## 4. Rules

- DT-20 CONFIRMED (docs). Two triangles that share an edge become one quad
  (their four corners, in the triangles' own winding) only if every
  check below passes.
- DT-21 OPEN (DQ-11). **Flat enough.** Provisional: fold height (distance
  of the fourth corner from the plane through the other three) is no more
  than Flatness × the shared edge's length. 0 % = only perfectly flat
  pairs.
- DT-22 CONFIRMED (docs: "maximum angle for the corners of a quad"). Every
  corner of the new quad is **below Max Angle**. This keeps out quads
  that are really triangles with an extra point (a corner near 180°).
  [OP] The quad must also be convex.
- DT-23 CONFIRMED (docs). With a **Keep … Boundaries** option on, a shared
  edge isn't removed if the two triangles differ in that respect (other
  material, other part, other smoothing group, one in a selection set
  and one not, or a UV seam along the edge).
- DT-24 [OP]. Each triangle joins at most one other. When a triangle could
  pair with more than one neighbour, the **flattest, most square** pairing
  wins first (pairs taken best first), so a regular triangulated grid
  comes back as the original quad grid.
- DT-25 [OP]. Surviving points, UVs, vertex maps, material and part are
  kept. Point count never changes; only edges and polygons go.
- DT-26 [OP]. **Selection after:** in Polygon mode, the new quads plus any
  triangles left over are selected.
- DT-27 [OP]. One undo step. If nothing qualifies, nothing changes and the
  status line says "No triangle pairs met the settings."

## 5. Edge cases to test

- DT-30 A 4×4 quad grid triangulated along one diagonal: back to 16 quads.
- DT-31 Same, triangulated in alternating diagonals: back to 16 quads.
- DT-32 A cube's 12 triangles: 6 quads. The pair across a cube edge
  (90° fold) is never joined.
- DT-33 Two triangles with different materials, Keep Material on: stay
  triangles; off: become a quad.
- DT-34 A thin sliver pair whose quad would have a 175° corner: stays.
