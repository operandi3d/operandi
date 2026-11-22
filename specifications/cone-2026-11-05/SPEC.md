# Cone primitive — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Creates a cone in the active mesh: a ring of base points, optional
intermediate rings, and a single apex point. Options set its size, position,
axis, number of sides and segments, how the base and apex are filled,
polygon type, base edge sharpness for subdivision, a cross-section bulge,
and an optional UV map.

## Inputs

None. All probes applied it numerically into an empty mesh layer. The tool
does not use or change the selection (nothing selected after, in every
case).

## Options

Defaults are the values after a tool reset (D0, CN01).

| Option | Type | Default | Effect |
|---|---|---|---|
| Position X/Y/Z | distance | 0, 0, 0 | Centre of the cone's bounding box. |
| Size X/Y/Z | distance | 0.5, 0.75, 0.5 | Half-extents. Along the axis: half the height. Across: the base radii. |
| Min X/Y/Z, Max X/Y/Z | distance | −0.5, −0.75, −0.5 / 0.5, 0.75, 0.5 | Bounding box. Always Position ∓ Size (CN09). |
| Sides | integer | 24 | Points per ring. |
| Segments | integer | 1 | Steps from base to apex. |
| Axis | X / Y / Z | Y | Direction from base to apex. |
| Top Cap | None / Single Polygon / Quad Grid / Radial | Single Polygon | Fill at the apex end (see Caps). |
| Bottom Cap | same | Single Polygon | Fill of the base. |
| Polygon Type | Face / Subdivs / Catmull-Clark | Face | Type of every polygon created. |
| Edge Weight | percent | 100 % | Subdivision weight on the base rim edges; only with Subdivs or Catmull-Clark. |
| Bulge | float | 0 | Reshapes the ring cross-section (see Bulge). |
| Make UVs | boolean | on | Create the UV map "Texture". |
| Distortion Compensation | boolean | off | UVs keep the real proportions instead of circles. |
| Gaps | percent | 0 % | UV spacing; not probed. |
| Handle Mode | Resize from Center / Resize / Move All | Resize | Handle behavior; not probed. |
| Show Corner | boolean | off | Show bounding-box corner handles; not probed. |
| (flip) | boolean | off | A tool option with no panel name in the guide; not probed. |

The guide gives 12 as the Segments default. The tool reset gives 1, which
is what we observed (Q-CN-01).

## Result

### Shape (Axis Y; X and Z below)

With Position P, Size (sx, sy, sz), Sides n, Segments m:
- **Base.** The ring sits at y = Py − sy. It is an ellipse with radii sx
  (X) and sz (Z).
- **Apex.** A single point at (Px, Py + sy, Pz).
- **Rings.** m − 1 intermediate rings, evenly spaced in height. Their radii
  shrink linearly to zero at the apex.
  - m = 3, base radius 0.5: rings at y −0.75, −0.25, 0.25 with radius 0.5,
    0.333333, 0.166667 (CN02).
- **Ring order.** Point k of a ring (k = 0 … n−1) is at angle
  θ = k·360°/n. Point 0 is on the −Z side and the order runs toward −X:
  (Px − sx·sin θ, y, Pz − sz·cos θ). Example: n 24, point 1 =
  (−0.12941, −0.75, −0.482963).
- **Point order in the mesh.** The base ring, then each higher ring, then
  the apex last.
- **Polygons and facing.** All outward:
  - Single Polygon bottom cap: one n-gon.
  - Each segment band: n quads.
  - Apex band: n triangles.
  - Counts: n 24, m 1 → 25 points, 25 polygons (CN01). n 8, m 3 → 25
    points: 1 octagon, 16 quads, 8 triangles (CN02).

### Axis X and Z (CN04a/b, n 6)

The height runs along the chosen axis, from −Size to +Size of that axis.
The base radii are the sizes of the other two axes.
- **Axis X:** point 0 on the −Y side, order toward −Z. Base at x = −0.5,
  apex at (0.5, 0, 0), base radii 0.75 (Y) and 0.5 (Z).
- **Axis Z:** point 0 on the −X side, order toward −Y. Base at z = −0.5,
  apex at (0, 0, 0.5), base radii 0.5 (X) and 0.75 (Y).

So point 0 always lies on the negative side of the axis that follows the
cone's axis in X→Y→Z→X order, and the ring runs toward the negative side of
the remaining axis.

### Negative and zero height (earlier probes)

- **Size along the axis negative** (cone-sizeYneg: sy −0.75): the cone
  flips. Base ring at y = +0.75, apex at −0.75, polygons still face
  outward.
- **Size along the axis zero** (cone-sizeY0, PR3-cone-y0-*): only the base
  polygon is made, one n-gon. This happens with every Bottom Cap setting,
  even None, Quad Grid and Radial (n 5 and 12 checked).

### Caps

- **None** (CN05a): base left open. n 8, m 2: 17 points, 16 polygons.
- **Single Polygon:** one n-gon over the base ring.
- **Radial** (PR3-cone-y-0.75-sides5/12-radial): a centre point plus a fan
  of n triangles.
- **Quad Grid:** the base is filled with quads:
  - n 8 (cone-8-quads, CN05c): a second ring of 8 points at half radius
    plus a centre point. 8 quads between the rings, 4 quads inside.
  - n 5 (cone-5-quads): an inner ring of 5 at half radius, filled with 5
    ring quads, 1 triangle and 1 quad.
  - n 12 (PR3-cone-y-0.75-sides12-quads): the inner ring sits at radius
    0.26438 (not half), plus 4 interior points; 21 quads in the cap.
  - The general layout rule is Q-CN-02. The cases hold exact meshes.
- **Top Cap.** The top of a cone is a single point, so None and Single
  Polygon both give the apex fan of triangles (CN05a, CN05b). With Quad
  Grid on top and n 8 (CN05c), the apex triangles are merged in pairs into
  4 quads. The cone is then all quads: 26 points, 24 polygons. Top Radial
  and odd n with top Quad Grid were not probed (Q-CN-03).

### Bulge (CN03a/b, n 8, m 4)

Bulge changes the shape of each ring, not the profile; ring heights and
axis radii are unchanged.
- Points on the ring's main axes (0°, 90° …) stay where they were.
- Diagonal points move:
  - Bulge +0.5: out to 0.378929 per coordinate on the base ring (radius
    0.5), squarer.
  - Bulge −0.5: in to 0.314980 per coordinate, pointier.
- Both values put the 45° point exactly on |x/r|^p + |z/r|^p = 1 with
  p = 2 + Bulge. The same holds on every ring, scaled.
- Only 45° points were observed (n 8). Whether other angles follow the same
  curve, and how, is Q-CN-04.

### Polygon type and edge weight (CN06a–d, n 8, m 2)

- Polygon Type sets the type of every polygon: Subdivs or Catmull-Clark.
- With either subdivision type, the base rim edges carry the subdivision
  edge weight. These are the 8 edges between the base ring points, shared
  by the bottom cap and the first band. The weight is 1.0 at 100 % and 0.5
  at 50 %. Every other edge reads 0.
- With Face (CN01), all edges read 0 even though Edge Weight is 100 %.
- A subdivision weight map named "Subdivision" exists after every Cone
  apply, including Face.

### UVs (Make UVs on; CN01–CN09)

The map is named "Texture"; UVs are per polygon corner.

**With a bottom cap**, two discs of radius 0.25:
- **Cap disc:** centre (0.75, 0.75).
- **Side disc:** centre (0.25, 0.75), apex at the centre. Ring j (j = 0 at
  the base) sits at radius 0.25·(m − j)/m.
- **Angles:** on the side disc, ring point k is at 180°/n + k·360°/n,
  counter-clockwise from +U. On the cap disc, base point k is at the mirror
  angle −(180°/n + k·360°/n).
  - n 6: side point 0 at 30° → (0.466506, 0.875).
  - n 6: cap point 0 at −30° → (0.966506, 0.625).
  - Checked at n 6, 8 and 24.

**Bottom cap None** (CN05a): only the side disc, centre (0.5, 0.5), radius
0.5.

**Unequal radii:**
- Compensation off: the discs stay circles. CN08b (sizes 0.5/0.75/0.25)
  has the same UVs as CN01, and Axis X/Z with radii 0.75/0.5 likewise
  (CN04a/b).
- Compensation on (CN08): the layout changes. The cap becomes a narrow
  ellipse in u 0.501–0.723, v 0.555–0.998. The side fills u 0.002–0.498,
  v 0.004–0.996 with the apex at (0.25, 0.5). Exact values are in the
  case; the rule is Q-CN-05.

**Other:**
- Bulge changes the UVs (CN03a/b); values in the cases (Q-CN-04).
- Quad Grid caps have their own UV layout (CN05c, PR3 quad cases).
- Position and Size don't change the UVs (CN09 matches CN01).

## Selection after

Nothing selected (all cases). The mode is unchanged.

## Refusals

None observed.

## Undo

Not probed (Q-CN-06).

## Acceptance cases

In `cases.json`:
- CN01 defaults (with UVs)
- CN02 sides/segments
- CN03a/b bulge ±0.5
- CN04a/b axis X/Z
- CN05a/b/c caps None / Single / Quad Grid (top and bottom)
- CN06a–d polygon type and edge weight
- CN07 UVs n 6 m 2
- CN08/CN08b compensation on/off
- CN09 size and position
- Earlier probes: cone-8, cone-sizeYneg, cone-sizeY0, cone-5-quads,
  cone-8-quads, PR3-cone-y-0.75-sides5-radial, -sides12-radial,
  -sides12-quads, -sides5-single, PR3-cone-y0-sides5-radial.

## Open questions

See `QUESTIONS.md`.

## Open questions and decisions

- **Q-CN-01** The guide gives Segments default 12; a tool reset gives 1.
  Does a fresh Modo (preferences cleared) start at 1 too? Which should
  Operandi use?
- **Q-CN-02** Quad Grid cap layout rule for any number of sides: inner ring
  radius (half for n 5 and 8, 0.26438 for n 12), interior points, and
  triangles for odd n. Needs a sweep of n.
- **Q-CN-03** Top Cap Radial, and top Quad Grid with odd n: what happens to
  the apex fan?
- **Q-CN-04** Bulge: the 45° points fit |x|^p + |z|^p = 1 with p = 2 +
  Bulge. Where do points at other angles go (same direction from the
  centre, or a different parametrisation)? What range does Bulge accept?
  How do UVs follow bulge?
- **Q-CN-05** Distortion Compensation: the rule behind the CN08 layout
  (disc aspect, scale, placement); also with Bottom Cap None.
- **Q-CN-06** Undo, handles, Handle Mode, Show Corner, and interactive
  creation (click-drag) were not probed.
- **Q-CN-07** Gaps: how it shrinks or separates the UV discs.
- **Q-CN-08** The unnamed "flip" option: what does it do?
- **Q-CN-09** Edge Weight on the Quad Grid and Radial caps, and on the top
  when Top Cap is Quad Grid: which edges get the weight?
- **Q-CN-10** Minimum Sides and Segments (1, 2, 3) and what happens below.
- **Q-CN-11** Material and part of the new polygons. The probes show
  Default only; is the current default material used?
- **Q-CN-12** Size zero across the axis (radius 0) and negative radius.

Polygon type names in the cases: face = Face, subdiv = Subdivision, psubdiv = Catmull-Clark, spatch = Spline Patch, curve = Curve, line = Polyline/line, bspline = B-Spline, onePoint = one-point polygon.
