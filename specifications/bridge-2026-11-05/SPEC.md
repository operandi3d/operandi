# Bridge — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Bridge joins two separate selections (two polygons, or two edge runs) with a
band of new quads, optionally with rings of new points in between. By
default the two source polygons are removed, leaving an open tube between
the two holes.

## Inputs

- **Polygon mode**: two polygons (or clusters) that share no edge. Each
  boundary loop is connected to the other.
- **Edge mode**: two edge runs that share no point.
- Nothing selected, or only one polygon selected: nothing happens, no
  message (BR18, BR19).

## Options

Defaults are the values after a tool reset (D0, BR01).

| Option (attr) | Default | Effect observed |
|---|---|---|
| Segments (`segments`) | 0 | Number of polygon rows across the bridge. 0 behaves like 1 (BR01 = BR02). n rows add (n−1) rings of points. |
| Mode (`mode`) | Curve | Linear / Curve / Smooth: path of the in-between rings (below). |
| Tension (`tension`) | 100% | Curve mode tangent length (below). |
| Twist (`twist`) | 0 | Shifts which point of the second loop pairs with each point of the first, by one step per unit (BR07). |
| Remove Polygons (`remove`) | on | Off: the two source polygons stay and stay selected (BR08). |
| Flip Polygons (`flip`) | off | Reverses the corner order of every new polygon (BR09). |
| Reverse Direction (`orient`) | off | Changes the pairing of the two loops (BR12; see Q-BR-03). |
| `reversex`, `reversey` | off | No effect seen on two parallel quads (BR10, BR11). |
| `inset` | 0 | No effect seen at 0.1 on two parallel quads (BR13). |
| Auto Connection (`connect`) | on | No effect seen in BR14 (applies to edges within one surface per the guide; not probed). |
| `uvs` | none | Not probed. |
| `aspect` on, `autoStep` off, `steps` 10, `continuous` on, `guideCurve` off, `closeLoop` off, `array` empty | — | Profile/guide-curve options; not probed. |

## Geometry (polygon mode, BR02–BR06)

Input: quad A `[0,1,2,3]` at y = −1 facing −Y, quad B `[7,6,5,4]` at
y = +1 facing +Y (points 4–7 directly above 0–3).

- Pairing: each corner of A connects to the corner of B straight above it
  (0↔4, 1↔5, 2↔6, 3↔7). See Q-BR-02 for the general rule.
- One new quad per edge of A, corner order `[a_next, a, b, b_next]` for A's
  edge a→a_next and the paired B points, e.g. `[1,0,4,5]`. With A and B
  removed, the band's normals point outward (away from the tube axis).
- Polygon order: one quad per edge of A, in A's corner order, the first for
  edge 0→1 (BR02): `[1,0,4,5] [2,1,5,6] [3,2,6,7] [0,3,7,4]`.
  Removed source polygons leave no slot (new polygons start at 0 when the
  mesh had only A and B).
- Segments n: rings of points are inserted, one ring per inner step, in
  ring order A→B; each ring has one point per corner, in A's corner order;
  rows of quads follow the same pattern ring by ring (BR03).
- Ring positions:
  - **Linear**: evenly spaced on the straight line between paired points
    (t = k/n).
  - **Curve**: a cubic Hermite curve between paired points with end
    tangents along the bridge direction of length tension × chord. Tension
    100% gives even spacing on parallel facing quads (BR04 = BR03); 50%
    gives rings at y = ±0.407407 for n = 3 (BR05), which matches the Hermite
    curve with tangent = 0.5 × chord.
  - **Smooth**: on parallel facing quads, even spacing like Linear (BR06).
    How it differs on a curved path is open (Q-BR-01).
- Quad to triangle (BR15): four new polygons: three quads and one triangle
  `[2,6,3]` where two corners of the quad pair with the same triangle corner.

## Edge mode (BR16, BR17)

Two border edges of a 2×1 strip, 0–3 (left) and 2–5 (right):
- Segments 1: one new quad `[3,5,2,0]` joining the two edges directly
  (it spans across the strip, overlapping it).
- Segments 2: one ring of two new points at the midpoints, (0, 0.5, 0) and
  (0, −0.5, 0). They coincide with existing points 4 and 1 but are not
  merged. Two quads: `[3,6,7,0]`, `[6,5,2,7]`.
- The two source edges stay selected; mode stays Edge.

## Selection after

Polygon mode with Remove on: nothing selected, mode stays Polygon. Remove
off: the source polygons stay selected. New polygons are never selected in
these cases.

## Open questions and decisions

- **Q-BR-01** Smooth mode: on parallel facing quads it equals Linear. What
  path does it take when the two ends are not facing (offset or angled)?
  Needs an angled pair.
- **Q-BR-02** Pairing rule in general: nearest point, or loop order from
  some start corner? Needs rotated and unequal loops.
- **Q-BR-03** Reverse Direction (`orient`): BR12 paired 0↔4, 1↔7 (reversed
  loop). What is the rule, and when does it help?
- **Q-BR-04** `reversex`, `reversey`, `inset`, `aspect`, `steps`,
  `autoStep`, `continuous`, `guideCurve`, `closeLoop`, `array`, `uvs`: no
  effect, or not probed. Likely profile and guide-curve controls.
- **Q-BR-05** Auto Connection: needs two edge runs inside one surface.
- **Q-BR-06** Clusters of several polygons on each side; unequal cluster
  outlines (e.g. 4 vs 6 boundary points).
- **Q-BR-07** Interactive segment drag in the viewport; undo; UVs on new
  points (see `new-point-uvs-2026-11-02` for one Bridge UV case).
- **Q-BR-08** Material/part of new polygons (not read).
