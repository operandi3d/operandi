"""Curved primitives (sphere, ellipsoid, cylinder, cone) in the cube's style.

Each solid is convex, so its outline is the convex hull of its projected
surface. It is shaded like the cube: the top (cap or upper half) in the top
tone, and the body split at the front by a vertical line into the left and
right tones. Round bodies (sphere, ellipsoid) use nested tones instead, so no
inked edge makes them look flat. Curves are sampled polygons; no SVG arcs.
"""

from math import cos, pi, sin

from .conventions import UNIT, Point, Projection, Vec3
from .primitives import Canvas

SAMPLES = 48


def ring(center: Vec3, radius: float, samples: int = SAMPLES) -> list[Vec3]:
    """A horizontal circle (constant Z) in model coordinates."""
    cx, cy, cz = center
    return [(cx + radius * cos(2 * pi * i / samples),
             cy + radius * sin(2 * pi * i / samples), cz) for i in range(samples)]


def hull(points: list[Point]) -> list[Point]:
    """Convex hull, counter-clockwise from the lowest-x point (monotone chain)."""
    pts = sorted(set(points))
    if len(pts) < 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def clip(polygon: list[Point], x: float, keep_left: bool) -> list[Point]:
    """Keep the part of a polygon left (or right) of the vertical line at x."""
    inside = (lambda p: p[0] <= x) if keep_left else (lambda p: p[0] >= x)
    out = []
    for i, current in enumerate(polygon):
        previous = polygon[i - 1]
        if inside(current) != inside(previous):
            t = (x - previous[0]) / (current[0] - previous[0])
            out.append((x, previous[1] + t * (current[1] - previous[1])))
        if inside(current):
            out.append(current)
    return out


def front_half(curve: list[Point]) -> list[Point]:
    """The lower (viewer-facing) half of a projected horizontal circle,
    from its rightmost to its leftmost point."""
    right = max(range(len(curve)), key=lambda i: curve[i][0])
    left = min(range(len(curve)), key=lambda i: curve[i][0])
    out, i = [], right
    # Walk the way that passes the lowest point (largest screen y).
    step = 1 if curve[(right + 1) % len(curve)][1] > curve[(right - 1) % len(curve)][1] else -1
    while True:
        out.append(curve[i])
        if i == left:
            return out
        i = (i + step) % len(curve)


def shaded(canvas: Canvas, outline: list[Point], top: list[Point] | None,
           top_curve: list[Point], axis_x: float):
    """Fill the body (outline minus the top region) in left/right tones split
    at axis_x, the top region in the top tone, then ink the outline, the
    front curve and the split line."""
    canvas.face(clip(outline, axis_x, True), "left")
    canvas.face(clip(outline, axis_x, False), "right")
    if top:
        canvas.face(top, "top")
    canvas.edge(outline, closed=True)
    if len(top_curve) > 1:
        canvas.edge(top_curve)
    # No ink line down the split: on a curved body it read as a seam
    # (Jason, 2026-11-01); the two tones meet on their own.


def ellipsoid(canvas: Canvas, projection: Projection, center: Vec3,
              radii: tuple[float, float, float]):
    """Sphere when the radii are equal. Shaded with nested tones lit from the
    upper left (dark base, mid ring, light highlight), the cube's lighting
    without hard edges: an inked boundary on a round body reads as a flat
    lid (Jason chose this "F2" look on 2026-11-01). Each lighter zone is a
    smaller copy of the outline, offset in proportion to its size."""
    outline = hull([projection(p) for p in ellipsoid_points(center, radii)])
    xs, ys = [p[0] for p in outline], [p[1] for p in outline]
    cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    w, h = (max(xs) - min(xs)) / 2, (max(ys) - min(ys)) / 2
    canvas.face(outline, "right")
    for tone, scale, (fx, fy) in (("left", 0.80, (-0.137, -0.137)),
                                  ("top", 0.45, (-0.256, -0.31))):
        canvas.face([(cx + fx * w + (x - cx) * scale, cy + fy * h + (y - cy) * scale)
                     for x, y in outline], tone)
    canvas.edge(outline, closed=True)


def cylinder(canvas: Canvas, projection: Projection, base: Vec3, radius: float,
             height: float):
    """Upright cylinder; its whole top cap is in the top tone."""
    bx, by, bz = base
    bottom = [projection(p) for p in ring(base, radius)]
    top = [projection(p) for p in ring((bx, by, bz + height), radius)]
    outline = hull(bottom + top)
    shaded(canvas, outline, None, front_half(top), projection(base)[0])
    canvas.face(top, "top")
    canvas.edge(top, closed=True)


def cone(canvas: Canvas, projection: Projection, base: Vec3, radius: float,
         height: float):
    """Upright cone; the body is split from the apex to the front of the base."""
    bx, by, bz = base
    apex = projection((bx, by, bz + height))
    bottom = [projection(p) for p in ring(base, radius)]
    outline = hull(bottom + [apex])
    shaded(canvas, outline, None, [apex], apex[0])


def fitted(points: list[Vec3], unit: float = UNIT, fit: float = 25.5) -> Projection:
    """A projection that centers these model points' screen bounds on the
    canvas, so shapes of different proportions share one optical center.
    Shapes wider or taller than `fit` pixels are scaled down to fit; line
    widths stay fixed, as everywhere else."""
    raw = [Projection(unit, (0.0, 0.0))(p) for p in points]
    xs, ys = [p[0] for p in raw], [p[1] for p in raw]
    extent = max(max(xs) - min(xs), max(ys) - min(ys))
    if extent > fit:
        unit *= fit / extent
        raw = [Projection(unit, (0.0, 0.0))(p) for p in points]
        xs, ys = [p[0] for p in raw], [p[1] for p in raw]
    return Projection(unit, (16 - (min(xs) + max(xs)) / 2, 16 - (min(ys) + max(ys)) / 2))


def ellipsoid_points(center: Vec3, radii: tuple[float, float, float]) -> list[Vec3]:
    cx, cy, cz = center
    rx, ry, rz = radii
    return [(cx + rx * cos(u) * cos(v), cy + ry * sin(u) * cos(v), cz + rz * sin(v))
            for u in (2 * pi * i / SAMPLES for i in range(SAMPLES))
            for v in (pi * (j / 16 - 0.5) for j in range(17))]
