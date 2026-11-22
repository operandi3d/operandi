# Vertex Bevel: GUI checklist

Spec: vertex-tools-2026-11-14 v1.1 (VX-B01..B07). QA app, fresh scene.
Make a Plane (2 m, 3×3 segments) and a Box. Vertex mode, Vertex panel ▸
Bevel.

- **VBQ-01 Fields.** Tool Properties shows Inset, Use Material, Material
  Name (greyed until Use Material), Round Level 0.
- **VBQ-02 Flat cap.** Select one interior point of the plane, drag the
  Inset handle: a four-sided cap opens around the point.
- **VBQ-03 Round Level.** Set Round Level 2: the cap's sides curve inward
  as arcs and the cap becomes a grid of 16 quads, still flat. The four
  neighbouring polygons get the extra arc points. Scrub Round Level 0–4:
  the preview follows.
- **VBQ-04 Corner dome.** On the box, select a corner, Inset 0.2, Round
  Level 2: a rounded dome replaces the corner, inside the old corner.
- **VBQ-05 Border and corner points.** On the plane, a point on the edge
  gets a cap with a straight side along the border (rounded inner sides at
  Level 2); a plane corner is cut off with no cap.
- **VBQ-06 Clamp.** Type Inset 5: the cuts stop at the neighbouring
  points instead of refusing.
- **VBQ-07 Selection, material, undo.** Use Material on, name "Cap",
  Drop: only the cut points (one per edge) are selected; every cap quad has
  the Cap material. Cmd+Z: the plane comes back in one step. Esc during a
  preview leaves the mesh as it was.
- **VBQ-08 Save.** Bevel, save, reopen: the rounded cap is unchanged.
