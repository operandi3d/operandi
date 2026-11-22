# New primitives: Torus, Capsule, Arc, Tube

**Status: v1 draft (partial).** Observed in Modo 17.1v1 on 2026-11-03 by
applying each primitive tool numerically into an empty mesh item and
reading the mesh back to six decimals. User guide pages Torus, Capsule,
Arc and Tube for names and intent. REA-assisted: no.

These are Create-panel primitives Operandi doesn't have yet. Text is
deferred (QUESTIONS.md NPR-Q10).

## Files

- `BEHAVIOR.md`: reset values, the rules read from the cases, and how the
  linked size options update each other.
- `ORACLES.json`: 36 cases (12 Torus, 8 Capsule, 9 Arc, 7 Tube), each with every
  point, every polygon's corner order, the selection afterwards, and the
  tool's option values after the settings were entered. The two default
  cases also carry the UV of every polygon corner ("Texture" map).
- `QUESTIONS.md`: what isn't covered yet.

## How the cases were made

Each case: a new scene with one empty mesh item at identity, Polygon
selection mode, the tool reset, then the listed settings entered in the
order given, then applied once. Several options are linked (changing one
updates others), so the order matters; `values_after_settings` records
the final values used. Tolerance 1e-5.

## Not covered

Viewport handles and click-to-create (all four tools), Text, polygon types
other than Face, UVs beyond the two defaults, Tube profiles, undo.
