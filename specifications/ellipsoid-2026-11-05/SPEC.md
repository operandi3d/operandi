# Ellipsoid primitive — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Options and defaults (after Reset)

| Option | Default | Notes |
|---|---|---|
| Position X/Y/Z | 0 | centre |
| Radius X/Y/Z | 0.5 m | half-extent per axis (a unit-wide ball) |
| Min/Max X/Y/Z | ∓0.5 m | bounding box, linked to Position/Radius (see the Unit Primitive spec, rules UC) |
| Sides | 24 | around the axis; minimum 3 |
| Segments | 12 | pole to pole; minimum 2 |
| Polygon Type | Face | Face, Subdivision, Catmull-Clark |
| Bulge Top | 0 | |
| Bulge Side | 0 | |
| Axis | Y | X, Y, Z |
| Make UVs | on | |
| Distortion Compensation | off | |
| Gaps | 0% | |
| Handle Mode | Resize from Center | handles only |
| Show Corner | off | handles only |

The tool remembers the last-used values between uses (Sides 8, Segments 4
and Compensation on were found before Reset); Reset restores the table.

## Geometry (axis Y)

1. **Counts.** `Sides × (Segments − 1) + 2` points and `Sides × Segments`
   polygons: a triangle fan at each pole, quads between (EL01: 266/288;
   EL02: 26/32; EL17 3/2: 5/6).
2. **Point order.** Point 0 is the bottom pole (−Y). Then the rings from
   bottom to top, each with `Sides` points. Ring point k sits at angle
   θ = 360° · k / Sides: x = −r·sin θ, z = −r·cos θ (starts at −Z, then
   towards −X). Last point is the top pole (+Y).
3. **Latitude.** Ring j (1 … Segments−1) is at φ = −90° + 180° · j / Segments:
   y = Ry·sin φ and the ring radius r = cos φ (scaled by Rx in x and Rz in
   z).
4. **Polygons, in order:** bottom fan `[0, k+1, k]` for each k (wrapping);
   then, ring by ring upward, quads `[a_k, a_k+1, b_k+1, b_k]` (a = lower
   ring, b = upper ring); then the top fan `[top, b_k, b_k+1]`. All face
   outward.
5. **Values below minimum are clamped:** Sides 2 / Segments 1 give 3 / 2
   (EL18 = EL17).
6. **Polygon Type** sets every polygon's type (EL03 Subdivision, EL04
   Catmull-Clark); geometry unchanged.
7. **Axis** rotates the layout by a cyclic swap of coordinates: for axis X,
   world (x, y, z) = local (y, z, x); for axis Z, world (x, y, z) = local
   (z, x, y), local being the axis-Y result (EL10, EL11, EL22).
8. **Position / Radius / Min / Max** place and scale the result; geometry
   always fills the bounding box (EL12, EL19, EL20).

## Bulge

9. Bulge reshapes the sphere into a superellipsoid with exponent
   e = 1 / (1 + Bulge / 2) (fits 0.5 → 0.8 and 1.0 → 2/3):
   - **Bulge Top** applies it to latitude: y = Ry·sgn(sin φ)·|sin φ|^e and
     ring radius |cos φ|^e (EL05, EL06).
   - **Bulge Side** applies it around the ring: x = −r·sgn(sin θ)·|sin θ|^e,
     z = −r·sgn(cos θ)·|cos θ|^e (EL07, EL08).
   - Both combine (EL09). Poles and axis-aligned ring points don't move.
   Example, Bulge Top 0.5, 8/4: ring 1 at y = −0.378929, radius 0.378929.

## UVs (Make UVs on)

10. **Layout.** One UV per polygon corner. Ring point k on ring j has
    (u, v) = (k / Sides, j / Segments); the seam point k = 0 uses u = 1 on
    the last column. Pole corners use v = 0 (bottom) / 1 (top) and u at the
    middle of their column, (k + 0.5) / Sides (EL02).
11. **Top cap quirk.** In the top fan `[top, b_k, b_k+1]` the two ring
    corners carry each other's u: b_k gets (k+1)/Sides and b_k+1 gets
    k/Sides (EL02 polygons 24–31). The bottom fan and the quads don't do
    this. **Decided for Operandi:** don't copy the swap; use u = k/Sides
    for b_k as in the bottom fan (Q-EL-01).
12. **Gaps** g shrinks the layout into [g, 1 − g] on both axes:
    u' = g + (1 − 2g)·u (EL14, 10%).
13. **Distortion Compensation** with unequal radii (1, 0.5, 0.25): column
    widths follow the lengths of the equator's edges, and the u range
    shrinks (here to 0…0.25, v stays 0…1) (EL13). The total width rule is
    open (Q-EL-02). With compensation off, radii don't affect UVs (EL12 =
    EL02).
14. Radius changes alone don't change the UVs (EL12).

## Options with no effect

15. **Flip** (not in the guide's list) leaves the result unchanged (EL16).
    The stored values "hole", "ring radius", "cross section" and "height"
    belong to other shapes; "hole" 0.5 changed nothing (EL21).

## Open questions and decisions

- **Q-EL-01** (decided below) Top-cap UV quirk (rule 11): match Modo's swapped u on
  the top fan, or use the sane layout (b_k at k/Sides like the bottom fan)?
  Provisional under the "sane over bug-like" rule: sane.
- **Q-EL-02** Distortion Compensation: how the total u (or v) range is
  chosen. One case only (radii 1/0.5/0.25 → u 0…0.25).
- **Q-EL-03** Negative or above-1 Bulge values; Bulge with Segments 2.
- **Q-EL-04** Make UVs off: confirm no UV values are written (the probe
  mesh had a UV map already, so EL15 can't tell).
- **Q-EL-05** UV map name and whether existing maps are reused.

## Decided (2026-11-05)

- **Q-EL-01 → decided:** use the clean layout for the top cap (ring corner
  b_k at u = k/Sides, as in the bottom fan); Modo's swapped u is not copied.

Polygon type names in the cases: face = Face, subdiv = Subdivision, psubdiv = Catmull-Clark, spatch = Spline Patch, curve = Curve, line = Polyline/line, bspline = B-Spline, onePoint = one-point polygon.
