# Mesh Cleanup — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Mesh Cleanup is one command with twelve on/off options (all **on** by
default, in this order): Remove Floating Vertices, Remove One Point
Polygons, Remove Two Point Polygons, Fix Duplicate Points in Polygons,
Remove Colinear Vertices, Fix Face Normal Vectors, Merge Vertices, Merge
Disco UVs, Unify Polygons, Force Unify, Remove Disco Weight Values, Fix
Gaps. Each option repairs one kind of fault; options that are off leave
that fault alone (MC21: all off changes nothing).

## Rules

1. **Remove Floating Vertices:** points used by no polygon are deleted
   (MC01); off keeps them (MC02). A point used only by a 1- or 2-point
   polygon is not floating (MC26).
2. **Remove One Point Polygons:** one-point polygons are deleted, and so is
   their point when nothing else uses it, even with Remove Floating
   Vertices off (MC03).
3. **Remove Two Point Polygons:** two-point polygons (type line) are
   deleted, and their points that nothing else uses (MC05).
4. **Fix Duplicate Points:** a corner listed twice in a row is dropped:
   `[0,1,1,2,3]` → `[0,1,2,3]` (MC07).
5. **Remove Colinear Vertices:** a point on a straight edge is removed from
   the polygon and deleted, but only when no other polygon uses it
   (MC08 removed; MC09, shared with a triangle, kept).
6. **Fix Face Normal Vectors:** a polygon whose last, first and second
   corners lie on a line has its corner list rotated so it starts
   elsewhere; points and facing don't change. `[0,1,2,3,4]` with 4, 0, 1
   colinear became `[2,3,4,0,1]` (MC10). Why it starts at corner 2 rather
   than 1 is open (Q-MC-03).
7. **Merge Vertices:** points at exactly the same position become one
   point. **Point order:** points that weren't merged keep their relative
   order at the front; each merged point is appended after them (MC11:
   `[0,1,2,3],[4,5,6,7]` with 1=4 and 2=7 → points `0,3,5,6,merged(1,4),
   merged(2,7)`, polygons `[0,4,5,1]`, `[4,2,3,5]`; MC34). Corners keep their
   order.
8. **Unify Polygons:** a polygon that uses the same points as an earlier
   one is deleted; the first stays. This holds for the same list rotated
   (MC13) and reversed, i.e. facing the other way (MC14). Duplicates with a
   different material tag are kept (MC15).
9. **Force Unify** only works together with Unify Polygons: with both on,
   duplicates are removed whatever their material (MC16, first one kept);
   Force Unify alone does nothing (MC17).
10. **Fix Gaps:** when a point lies on a polygon's edge and is used by the
    neighbouring polygons but not by that one (a T-junction), it is
    inserted into that polygon's corner list at the edge: `[0,1,2,3]` →
    `[0,1,2,6,3]` (MC18), `[0,1,2,4,3]` (MC19, MC20).
11. **Several faults at once** (all on, MC22): floating points, one- and
    two-point polygons and their points are all removed; a floating point
    sitting on a used point is removed as floating rather than merged, so
    the remaining points keep their order.
12. **Which part of the mesh:**
    - Polygon mode with nothing selected, Vertex mode with nothing
      selected, and Item mode: the whole mesh (MC01, MC33, MC25).
    - Polygon mode with polygons selected: only those polygons and the
      points they use. Unselected faults stay (MC23, MC27); selected
      degenerate polygons are removed (MC28).
    - Vertex mode with points selected, Edge mode with edges selected:
      nothing changed in four cases (MC24, MC29, MC30, MC32). Exception:
      with only a floating point selected, the whole mesh was cleaned, and
      the merge ran before the floating-point removal, so the merged point
      moved to the end (MC31). **Decided for Operandi:** a Vertex- or
      Edge-mode selection does nothing; MC31 is an accepted difference
      (Q-MC-01).
13. **Selection after:** selected polygons that were removed leave the
    selection; others stay selected; the mode doesn't change.
14. **Not observed:** Merge Disco UVs, Remove Disco Weight Values (UV and
    weight maps not probed); the merge distance (only exact matches were
    tested).

## Open questions and decisions

- **Q-MC-01** (decided below) Vertex/Edge-mode selections. Four cases change
  nothing; one (only a floating point selected) cleans the whole mesh.
  Sane rule for Operandi: a Vertex- or Edge-mode selection limits nothing
  and does nothing, matching four of five cases; accept MC31 as a known
  difference.
- **Q-MC-02** Merge Vertices tolerance: only exact matches were tested.
- **Q-MC-03** Fix Face Normal: which corner the rotated list starts at
  (one case, start moved by 2).
- **Q-MC-04** Merge Disco UVs, Remove Disco Weight Values: not probed.
- **Q-MC-05** Order of the repairs when they interact (MC22 vs MC31).

## Decided (2026-11-05)

- **Q-MC-01 → decided:** with something selected in Vertex or Edge mode,
  Mesh Cleanup does nothing. MC31 (only a floating point selected; Modo
  cleaned the whole mesh) is an accepted difference.

Polygon type names in the cases: face = Face, subdiv = Subdivision, psubdiv = Catmull-Clark, spatch = Spline Patch, curve = Curve, line = Polyline/line, bspline = B-Spline, onePoint = one-point polygon.
