# Spikey (Spike) — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Spikey replaces each selected polygon with a fan of
triangles meeting at one new point, raised off the polygon along its
normal. One option: Spike Strength (`factor`).

## Inputs

Selected polygons. Nothing selected: every polygon of the mesh is spiked
(SP09).

## Options

| Option | Default | Effect |
|---|---|---|
| Spike Strength (`factor`) | 0.0 | Height of the spike as a fraction of the polygon's mean edge length. Negative pushes it inward (SP03). |

## Rules

1. **New point** per polygon: the average of its corners, moved along the
   polygon normal by `factor × mean edge length`.
   - Unit quad, 0.5 → height 0.5 (SP02). 2×1 rectangle, 0.5 → height 0.75
     (mean edge 1.5) (SP04). Right triangle with legs 1 → centre
     (1/3, 1/3), height 0.569036 = 0.5 × (2 + √2)/3 (SP05). Cube top, 1.0 →
     height 1 (SP08).
   - Non-planar quad (0,0,0) (1,0,0) (1,1,0.5) (0,1,0), 0.5 → (0.5, 0.5,
     0.654508): the centre plus 0.529508 straight up +Z, which is 0.5 × the
     mean edge length (SP06). Which normal is used for non-planar polygons
     is open (Q-SP-01).
   - Factor 0 (default) still builds the fan, with the new point flat in
     the polygon (SP01).
2. **Fan:** for each edge a→b of the original, in corner order, a triangle
   `[a, b, c]` (c = new point). Same facing as the original.
3. **Order:** unselected polygons keep their relative order and move to
   the front; the selected originals are removed and each polygon's fan is
   appended in selection order (SP07, SP08). New points are appended in the
   same order.
4. **Selection after:** with part of the mesh selected, all new triangles
   are selected (SP07, SP08). When the selected polygons were the whole
   mesh (SP01–SP06) or nothing was selected (SP09), nothing is selected
   after (Q-SP-02). Mode stays Polygon.

## Open questions and decisions

- **Q-SP-01** Normal for non-planar polygons: SP06 moved straight up +Z,
  which matches neither the Newell normal nor the first-corner cross
  product. Needs more non-planar cases (tilted the other way).
- **Q-SP-02** Selection after: new triangles were selected when part of
  the mesh was selected, but nothing when the whole mesh was selected.
  Confirm with a 2-polygon mesh, both selected.
- **Q-SP-03** Interactive drag (strength by mouse), undo, falloff,
  symmetry; UVs and material of the new triangles.
- **Q-SP-04** The procedural Spike mesh operation: same rule? (Operandi
  scope decision.)
