# Flatten Ngons and PolyScale Offset — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Flatten Ngons

One-click command, no options.

1. **Only n-gons.** Polygons with five or more corners are flattened;
   triangles and quads are never changed, even when non-planar and
   selected (FN01, FN03, FN07).
2. **Whole mesh, selection ignored.** Every n-gon in the mesh is
   flattened whether it is selected or not: selecting only a quad still
   flattens the pentagon beside it (FN05), and nothing selected flattens
   all n-gons (FN09). Vertex mode behaves the same (FN08).
3. **Projection.** Each n-gon's points move straight onto one plane (every
   point moves along the plane's normal; the polygon keeps its shape seen
   along that normal). For the pentagon (0,0,0) (2,0,0) (2,1,0.3) (1,2,0)
   (0,1,0) the plane normal is (−0.042698, 0.104424, 0.993616) and the
   points move by +0.210769, +0.296164, −0.106345, +0.044619, +0.106345
   along it (FN02). Modo's plane is neither the Newell-normal plane through
   the centroid nor a plane through any three original points. **Decided
   for Operandi (Q-FL-01):** the least-squares plane through the centroid,
   compared with Modo within a tolerance.
4. **Shared points** move with the n-gon, so neighbouring polygons using
   them change shape too (no separate case yet; follows from rule 3).
5. Already planar n-gons don't move (by rule 3; FN06 is a quad).
6. **Selection after:** the polygon (or point) selection is cleared; the
   selection mode stays.

## PolyScale Offset / PolyScale Offset Detach (from the guide only)

7. PolyScale Offset asks for one value, **Inset**, then makes, inside each
   selected polygon, a smaller (or, for a negative value, larger) copy
   whose edges are moved by Inset, framed by the original. The guide shows
   it on cube faces.
8. PolyScale Offset Detach does the same and detaches the new inner
   polygon from the mesh so it can be moved away on its own.
9. Exact geometry, point sharing, selection after and the dialog's
   default are open (Q-FL-03); a hands-on probe is needed.

## Open questions and decisions

- **Q-FL-01** (decided below) Flatten plane: Modo's plane can't be derived from
  one case. Sane rule for Operandi: least-squares (best-fit) plane through
  the n-gon's centroid, points projected along its normal. Provisional
  under "sane over bug-like": use the sane rule and compare within a
  tolerance. More cases (regular hexagon with one raised point; a twisted
  hexagon) would help if a closer match is wanted.
- **Q-FL-02** Flatten when two n-gons share points and want different
  planes (order dependence).
- **Q-FL-03 (decision pending)** PolyScale Offset and Detach open a value
  dialog, which blocks scripted probing. A hands-on probe is planned. Until then, rules 7–9 are guide-level only.
- **Q-FL-04** Flatten in item mode / with several meshes selected.

## Decided (2026-11-05)

- **Q-FL-01 → decided:** least-squares (best-fit) plane through the n-gon's
  centroid, points projected along its normal; compare with Modo within a
  tolerance.
