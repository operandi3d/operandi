# Inset — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Inset makes a smaller copy of each selected polygon inside
it and joins the copy to the original outline with a ring of quads. One
option: Inset (distance).

## Inputs

Selected polygons. Nothing selected: every polygon of the mesh is inset
(IN10 = IN06).

## Options

| Option | Default | Effect |
|---|---|---|
| Inset (`inset`) | 0 m | Distance each edge moves inward, measured perpendicular to the edge in the polygon's plane. Negative moves outward (IN03). |

## Rules

1. **Each polygon on its own.** Adjacent selected polygons are inset
   separately; no new points are shared, and the shared edge gets a side
   quad on both sides (IN05, IN06).
2. **Edge offset, not scaling.** The inner polygon's edges are the original
   edges moved inward by Inset; its corners are where neighbouring moved
   edges meet (miter). Triangle legs 1,1: corners (0.1, 0.1),
   (0.758579, 0.1), (0.1, 0.758579) at 0.1 (IN08). A reflex corner moves
   outward from the notch, e.g. L-shape notch (1,1) → (0.9, 0.9) (IN09).
3. **No clamping.** An inset larger than half the width crosses over: a
   unit quad at 0.6 gives an inner quad at ±0.1 with its corners swapped
   (inverted) (IN04).
4. **Zero still builds.** Inset 0 (the default) creates the inner points on
   top of the originals and the side quads, all degenerate (IN01).
5. **Topology per polygon with n corners:** n new points, the inner
   polygon (same corner order as the original, using the new points), and
   n side quads. For original edge a→b with inner points a′, b′ the side
   quad is `[a, b, b′, a′]` (same facing as the original).
6. **Order.** New points: per polygon, in its corner order, polygons in
   selection order. The inner polygons take the slots of the selected
   originals, but in reverse order: with polygons 1 and 4 selected, slot 1
   holds polygon 4's inner copy and slot 4 holds polygon 1's (IN07, also
   IN05). Side quads are appended, per polygon, starting with the edge from
   corner 2 (as listed in the cases); see Q-IN-02.
7. **Selection after:** the inner polygons (the original slots) stay
   selected; mode stays Polygon.

## Open questions and decisions

- **Q-IN-01** Does Operandi need to match polygon slot order exactly (rule
  6: inner copies fill the original slots in reverse order)? If yes, more
  cases with three or more selected polygons would pin it.
- **Q-IN-02** Side-quad order within a polygon starts at a corner that
  isn't 0 (IN01: edges 3→2, 1→3, 0→1, 2→0 for corners [0,1,3,2]). The
  general rule needs a pentagon and a hexagon case.
- **Q-IN-03** Non-planar polygons: which plane is used?
- **Q-IN-04** Interactive use (drag), undo, falloff, symmetry; UVs and
  material of new polygons.
- **Q-IN-05** The guide's Inset is a single numeric field; the old
  "center" question (where the inset scales about) doesn't arise with an
  edge offset. Confirm no Group/Scale option exists in other Inset entries
  (e.g. Bevel's Inset).

## Decided (2026-11-05)

- **Q-IN-01 → decided:** Operandi need not match Modo's reversed polygon
  slot order (SPEC rule 6); any order is accepted, and cases compare
  polygons as a set.
