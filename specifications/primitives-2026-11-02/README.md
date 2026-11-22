# Primitives: Cube, Sphere, Cylinder, Cone, Ellipsoid (TOOL_GAPS section 3)

**Status: v2 draft (partial); existing handed-off IDs retained.** Observed in Modo 17.1v1 on 2026-11-02 by applying each
primitive tool numerically into an empty mesh. REA-assisted: no.

## Covered

- `BEHAVIOR-DEFAULTS.md`: every option's reset value and accepted values.
- `COUNTS.md`: point/polygon counts for 32 cases.
- `BEHAVIOR-CUBE.md`, `BEHAVIOR-ROUND.md` (Sphere, Cylinder, Cone,
  Ellipsoid): rules read from the cases.
- `ORACLES.json`: all 32 cases with full points, polygons in order, and the
  UV of every polygon corner ("Texture" map).

## Not covered (QUESTIONS.md)

Viewport drawing and handles (PM-Q01–Q04, BOX-Q04/Q05/Q08/Q09: pointer
input), primitives created into a non-empty or transformed mesh, undo.

## Batch 3 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), with scripted numeric cases and exact six-decimal mesh/UV readback. REA-assisted: no.

BEHAVIOR-BATCH-3.md adds 21 oracles. Covered: Quad Ball sizes 0.5/1/2, Cube Bulge 0.2 at Radius 0.1/0.2, and sixteen zero/negative Cone cases at 5/12 sides and four Bottom Cap types.

Not covered: Quad Ball subdivision-limit meaning, broader Bulge formula, independently varied Top Cap, pointer behavior and undo. One final cone case is crash-adjacent with a complete pre-crash readback. Earlier scope text describes the prior pass; QUESTIONS.md lists remaining gaps.
