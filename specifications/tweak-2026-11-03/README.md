# Tweak (Element Move), shortcut T

**Status: v1, partial.** 17 oracles, observed in Modo 17.1v1 on
2026-11-03 by desktop control, with point positions read back to six
decimals. Option names and intent come from the Modo help (Element Move,
Element Falloff). The key was verified by press in
`specifications/default-shortcuts-2026-11-01` (KEY-O-KEY-P06).
REA-assisted: no.

v1 replaces v0 (guide-only). Each rule is tagged **observed** (backed by
an oracle), **guide** (from the help, not measured) or **predicted** (our
best reading, build it but expect it to change).

## What it is

Tweak grabs one vertex, edge or polygon straight off the mesh and drags
it, with no need to select it first. A built-in falloff can spread the move
over nearby points. It moves points only: topology never changes.

## Files

- `BEHAVIOR-TWEAK.md`: rules TWK-01..
- `ORACLES.json`: 17 cases (inputs, settings, gesture, expected points).
- `QUESTIONS.md`: what is still unobserved.

## Reading the oracles: one big caveat

The pointer input used to drive Modo did not convert pixels to distance
normally: a 100 px drag (about 0.47 m at the recorded zoom) moved points
by 9–25 m. So the oracles **do not** say how far a drag moves anything.
What they do pin down is **which points move** and **how strongly each
one moves compared with the grabbed element** (its weight). To test
against an oracle, take the grabbed element's measured offset as given,
then check every other point's offset = weight × that offset.

## Build slices

1. **TWK-1:** T activates; Auto picking; press-drag moves the grabbed
   element (vertex 1 point, edge 2, polygon all its points); Range 0;
   selection untouched; one undo step per drag. (Picking and the drag plane
   are still predicted, the point sets are observed.)
2. **TWK-2:** Range > 0 with Linear and Smooth weights (observed), the
   other shapes (predicted), Connected Elements Ignore / Use Connectivity /
   Rigid (observed) and Edge Loops for an edge pick (observed).
3. **TWK-3:** click-to-show handle (placement observed), axis drags,
   Position X/Y/Z, right-drag Range, Symmetry, Snapping, Always Raycast
   (all still predicted).

## What Operandi has today (2026-11-03)

- No Tweak tool and no Element falloff type.
- The transform tools already support free drags on geometry, one undo step
  per drag, on the shared fast drag path. Tweak should reuse that path.
- Falloff shapes are in `specifications/falloff-2026-11-01`; Symmetry in
  `specifications/symmetry-2026-11-01`.
- Point and polygon IDs must stay stable on plain moves (project rule).
