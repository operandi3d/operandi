# Chamfer Edit — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Chamfer Edit reworks an existing chamfer or bevel: it widens or narrows
it and changes how many segments it has and how round it is. Input in the
probes: a box corner, made of a top face, a chamfer strip and a side face,
extruded along Z, with the chamfer strip polygon(s) selected.

## Settings and defaults (after Reset)

| Setting | Default | Values |
|---|---|---|
| Offset | 0 | distance |
| Mode | Inset | Inset, Width |
| Segments | 1 | ≥ 1 |
| Depth | 100 % | percent, negative allowed |
| Reverse Direction / Reverse Depth | off / off | profile flips |
| Source Segments | 1 | ≥ 1 |
| Absolute Size | off | on/off |
| Swap Side | off | on/off; only allowed when Source Segments is 1 |

## Rules

1. **Defaults change nothing** (CE01, CE20).
2. **Offset, Inset mode, relative:** the chamfer's two border edges slide
   along the neighbouring faces so the inset grows by Offset. The inset is
   the distance along each face from the virtual sharp corner. Inset 0.2 +
   0.1 → 0.3 (CE02); 0.5 − 0.1 → 0.4 (CE10).
3. **Offset, Width mode, relative:** the chamfer's width (the distance
   between its border edges) grows by Offset. Width 0.282843 + 0.1 puts the
   borders at 0.270711 from the corner (CE03); 0.707107 + 0.2 at 0.641421
   (CE11).
4. **Absolute Size on:** Offset is the inset itself, not a change. Offset
   0.1 → inset 0.1 (CE07). Offset 0 collapses the chamfer into a sharp
   corner: the strip is removed and the faces meet at the corner (CE08:
   3 → 2 polygons).
5. **Segments:** the strip is rebuilt with that many segments between the
   (possibly moved) borders (CE04: 1 → 3, 8 → 12 points, 3 → 5 polygons).
   **Depth 100 %** bows them outward into a rounded profile; 0 % keeps them
   on the flat chamfer plane; −100 % bows them inward by the same amount
   (CE04, CE05, CE06). Modo's points are neither on a circular arc nor
   evenly spaced. **Decided for Operandi (Q-CE-01):** points evenly spaced
   in angle on the true arc tangent to both faces, with Depth scaling the
   bulge; compare with CE04–CE06 within a tolerance.
6. **Source Segments** must equal the number of segments across the
   selected chamfer. Matching (2 → 4) rebuilds it cleanly (CE12); 2 → 1
   gives one flat strip (CE14). A mismatch (1 given, 2 selected) produces
   n-gon neighbours and a broken result in Modo (CE13). **Decided for
   Operandi (Q-CE-02):** refuse the edit on a mismatch, and keep border
   points on their faces (CE15 is not copied).
7. **Swap Side** takes the chamfer as running across the strip the other
   way: Offset then moves the strip's other two edges (CE09: the ends moved
   out by 0.1 along Z).
8. **Reverse Direction** without a profile has no effect (CE19 = CE04).
9. **A flat strip** works too: on a flat 3×3 grid with the middle column
   selected, Offset 0.1 moves the column's borders out by 0.1 and Segments 2
   splits it down the middle (CE18).
10. **Selection all three faces, or nothing selected,** rebuilt each face
    across Z into three segments (CE16, CE17). Reference only (Q-CE-03).
11. **Selection after:** kept when no polygons were added or removed
    (CE02, CE09–CE11); cleared when the strip was rebuilt.
12. **Not observed:** handle drags, profiles, falloff, chamfers made by
    Modo's own Edge Chamfer/Bevel (inputs here were built by hand), the
    guide's "select the two outer polygons" workflow, and CE15 (Source
    Segments 2 with Offset 0.1 moved points off their faces; treated as
    bug-like, Q-CE-02).

## Open questions and decisions

- **Q-CE-01** (decided below) Round profile: Modo's segment points (CE04–CE06) are
  neither on a circular arc nor evenly spaced. Sane rule for Operandi:
  points evenly spaced in angle on the arc tangent to both faces (Depth
  scales the bulge; 0 % = flat, evenly spaced), compared within a
  tolerance. Alternative: fit Modo's curve exactly from more cases.
- **Q-CE-02** (decided below) Mismatched Source Segments (CE13) and CE15 give broken
  geometry. Sane rule: refuse the edit when the selection doesn't
  match Source Segments, and for CE15 keep border points on their faces.
- **Q-CE-03** What counts as the chamfer when faces next to it are also
  selected, or nothing is (CE16, CE17), and the guide's "two outer
  polygons" selection.
- **Q-CE-04** Profiles, Reverse Depth, falloff and handle drags (pointer
  input; hands-on).

## Decided (2026-11-05)

- **Q-CE-01 → decided:** segment points evenly spaced in angle on the true
  circular arc tangent to both faces; Depth scales the bulge (100 % = the
  arc, 0 % = flat and evenly spaced, −100 % = mirrored inward). Compare with
  Modo's CE04–CE06 within a tolerance.
- **Q-CE-02 → decided:** refuse the edit when the selected chamfer doesn't
  match Source Segments (CE13), and keep border points on their faces
  (CE15 is not copied).
