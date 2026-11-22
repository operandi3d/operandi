# PolyScale Offset / PolyScale Offset Detach — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Rules

1. **Dialog:** one value, titled "Enter Inset Amount", opens at **0**.
2. **PolyScale Offset (B1–B3):** each selected polygon shrinks in its own
   plane: every edge moves inward by the value (0.1 on a unit cube top →
   corners at ±0.4). A negative value grows it (−0.1 → ±0.6). **No new
   points or polygons:** the polygon keeps its points, so the neighbouring
   side polygons slant to follow. The polygon stays selected. Point and
   polygon order may be renumbered (B1).
3. **Several faces (B2):** top and front both shrink; their shared points
   move in both planes (the shared corner points end at (±0.4, 0.5, 0.5)).
4. **PolyScale Offset Detach (B4):** the same shrink, but the polygon is
   rebuilt on **four new points** (8 → 12 points, still 6 polygons). The
   original corner points stay with the side polygons, so the box is left
   open and the smaller polygon floats in the same plane, unconnected.
   Selection is cleared afterwards.
5. **Not observed:** n-gons, non-planar faces, values above half the face
   width.

## Open questions and decisions

- **Q-PO-01** Values large enough to collapse or flip the face.
- **Q-PO-02** Detach with two adjacent faces: one detached piece or two.
- **Q-PO-03** Why Offset keeps the selection and Detach clears it (one case each).
