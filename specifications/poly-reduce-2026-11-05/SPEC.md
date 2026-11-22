# Polygon Reduce — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Polygon Reduce removes edges until the mesh has a target
number of polygons, trying to keep the shape. The guide describes two
methods: a quadric-error method (default) and an older "Legacy" edge
method.

The exact output of a mesh-reduction algorithm is an implementation
choice. This spec fixes the contract (options, counts, when nothing
happens, what is kept); the cases are reference results, not oracles to
match point for point.

## Options

Defaults are the values after a tool reset (D0, RD01).

| Option (attr) | Default | Effect observed |
|---|---|---|
| Number (`number`) | 0 | Target polygon count for the mesh. Result has exactly this many polygons (RD02: 16 → 8; RD03: 16 → 4). |
| Boundary Weight (`boundaryWeight`) | 100% | How strongly open borders are kept. 0% on a flat grid changed which points survive, not the count (RD04 vs RD02). |
| Material Border Weight (`materialWeight`) | 100% | Not probed (single material). |
| Legacy Mode (`method`) | off | On: the older method; different result, same count (RD05, RD07). |

## Rules

1. **Count:** the result has exactly `number` polygons when `number` is
   between 1 and the current count (RD02, RD03, RD06, RD07).
2. **No-ops:** `number` 0 (the default), or `number` ≥ the current count,
   leave the mesh unchanged (RD01, RD09).
3. **Selection:** with 8 of 16 polygons selected and target 8, nothing
   changed (RD08). Whether the target then counts only the selection is
   open (Q-RD-01). With nothing selected the whole mesh is reduced.
4. **Shape kept:** on a grid with one raised point (height 1), both methods
   kept that peak point at (0, 0, 1) (RD06, RD07). On a flat grid the
   default method kept the outline corners.
5. **Mixed polygons:** results mix quads and triangles (RD02: 6 quads,
   2 triangles).
6. **Leftover points:** the default method leaves unused points in the
   mesh, some at duplicate positions (RD02: 17 points, two at (−2, −2, 0),
   not all used by polygons). Whether Operandi should clean these up is a
   decision (Q-RD-02).
7. **Selection after:** unchanged (the selected indices stay selected).

## Open questions and decisions

- **Q-RD-01** With a selection, is the target the count of selected
  polygons, and only those are reduced? Needs 8 selected with target 4.
- **Q-RD-02** (decided below) Leftover unused and duplicate points:
  copy Modo, or remove them? Recommendation: remove unused points; it
  doesn't change the visible result.
- **Q-RD-03** (decided below) Algorithm: implement a standard
  quadric-error reduction of our own and match the contract only, not
  Modo's exact meshes. Legacy Mode: needed at all?
- **Q-RD-04** Material Border Weight; UVs; interactive drag that changes
  Number; undo.

## Decided (2026-11-05)

- **Q-RD-02 → decided:** remove unused (and duplicate) points after
  reducing.
- **Q-RD-03 → decided:** Operandi uses its own reduction and must match
  the rules in SPEC.md (polygon-count rule, qualitative behaviour), not
  Modo's exact meshes. No Legacy Mode.
