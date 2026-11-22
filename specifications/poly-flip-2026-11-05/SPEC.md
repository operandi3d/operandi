# Flip Polygons — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Flip (no options) reverses the facing of polygons by
reversing their corner order. Points don't move.

## Rules

1. **Corner order:** the first corner stays first; the rest are reversed.
   `[0,1,3,2]` → `[0,2,3,1]` (FL01), `[0,1,2]` → `[0,2,1]` (FL02),
   `[0,1,2,3,4]` → `[0,4,3,2,1]` (FL03).
2. **Which polygons:** in Polygon mode, the selected polygons (FL04); with
   nothing selected, every polygon of the mesh (FL05). In Item mode, every
   polygon of the selected mesh (FL06).
3. **Vertex (and presumably Edge) mode:** refused with the message "Need to
   directly select one or more polygons."; nothing changes (FL07).
4. Polygon count, order and the selection are unchanged; mode unchanged.

## Open questions and decisions

- **Q-FL-01** Edge mode: same refusal as Vertex mode? (Not probed.)
- **Q-FL-02** UVs and vertex normals: are polygon-corner UVs reordered
  with the corners? (Not read.)
- **Q-FL-03** Undo; several mesh items selected in Item mode.
