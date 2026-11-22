# Subdivision surfaces (Subdivision and Catmull-Clark)

**Status: v3 (partial).** v2: Modo's Subdivision rule found for all levels and open boundaries (SDV-04, SDV-09, SDV-10). v3: Face + smooth meshes (SDV-13), Freeze keeps UVs (SDV-15). Observed in Modo 17.1v1 on 2026-11-03 by
scripted probes (convert, set level, freeze, read back to six decimals).
REA-assisted: no.

## Covered

- `BEHAVIOR-SUBDIVISION.md`: the two smooth polygon kinds; Tab and
  Shift+Tab toggles; per-mesh levels and defaults; Catmull-Clark rules
  (exact, including triangles, n-gons, open boundaries and the three
  Boundary Rules); edge weights as semi-sharp creases (sharpness = 10 ×
  weight); Modo's own Subdivision (one Catmull-Clark step, then biquadratic
  patches sampled L × L); Freeze.
- `ORACLES.json`: 57 cases with cage, settings and every frozen point.

## Suggested build order

Both kinds are now fully specified without weights. Edge weights are
specified for Catmull-Clark only; for Subdivision, ignore them for now and
mark that provisional (SDV-Q02).

## Not covered (QUESTIONS.md)

Subdivision edge weights;
vertex weights; UV smoothing and Linear UVs; mixed Face/smooth meshes;
render levels and adaptive subdivision; multiresolution; display (cage,
OpenSubdiv draw); undo.
