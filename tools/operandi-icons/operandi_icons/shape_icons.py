"""Batch 6: the remaining Create-panel shapes (Torus, Capsule) and the Draw
tools (Arc, Tube). Original drawings in the primitives' style: the cube's
three tones lit from the upper left, ink outlines, no inner ink on round
bodies. Arc and Tube show what they make along points, so they carry the
cyan "new geometry" cue on their control points."""

from math import atan2, cos, hypot, pi, sin

from .conventions import ACCENT, DETAIL, SELECT, SELECT_BACKING, Point, Projection, Vec3
from .primitives import Canvas
from . import solids

TAU = 2 * pi


def _keyhole(outer: list[Point], inner: list[Point]) -> list[Point]:
    """One polygon for a ring: the outer loop, then the inner loop the other
    way round, joined by a zero-width slit. Under the default nonzero fill
    the inner region winds to zero, so the hole stays transparent."""
    start = min(range(len(inner)), key=lambda i: hypot(inner[i][0] - outer[0][0],
                                                       inner[i][1] - outer[0][1]))
    loop = inner[start:] + inner[:start]
    return outer + [outer[0]] + [loop[0]] + loop[::-1] + [loop[0]]


def _ccw(points: list[Point]) -> list[Point]:
    area = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(points, points[1:] + points[:1]))
    return points if area > 0 else points[::-1]


def torus_points(center: Vec3, major: float, minor: float,
                 around: int = 160, tube: int = 64) -> list[Vec3]:
    cx, cy, cz = center
    return [(cx + (major + minor * cos(v)) * cos(u),
             cy + (major + minor * cos(v)) * sin(u),
             cz + minor * sin(v))
            for u in (TAU * i / around for i in range(around))
            for v in (TAU * j / tube for j in range(tube))]


# Light from the upper left and a little toward the viewer, in model space.
_LIGHT = (-0.35, 0.25, 1.0)


def torus(canvas: Canvas):
    """A ring lying flat, lit from the upper left: base tone on the body, a
    lighter band over the top of the ring, a highlight inside it, the hole
    left clear.

    A torus hides parts of itself, so the tone regions come from a depth
    buffer: sample the surface densely, keep the nearest sample in each fine
    screen cell, then trace each region around the ring's projected center
    (every tone region here is a ring, crossed once by any ray from the
    center). Only the traced outlines reach the SVG."""
    center, major, minor = (0.5, 0.5, 0.5), 0.66, 0.27
    projection = solids.fitted(torus_points(center, major, minor, 48, 16))
    mx, my = projection(center)
    cell, bins = 0.1, 144
    lx, ly, lz = _LIGHT
    ln = (lx * lx + ly * ly + lz * lz) ** 0.5
    nearest: dict[tuple[int, int], tuple[float, int]] = {}
    around, tube = 900, 360
    for i in range(around):
        u = TAU * i / around
        for j in range(tube):
            v = TAU * j / tube
            normal = (cos(v) * cos(u), cos(v) * sin(u), sin(v))
            if sum(normal) <= 0:  # faces away from the viewer at (1, 1, 1)
                continue
            point = (center[0] + (major + minor * cos(v)) * cos(u),
                     center[1] + (major + minor * cos(v)) * sin(u),
                     center[2] + minor * sin(v))
            x, y = projection(point)
            key = (round(x / cell), round(y / cell))
            depth = sum(point)
            if key not in nearest or depth > nearest[key][0]:
                lit = (normal[0] * lx + normal[1] * ly + normal[2] * lz) / ln
                nearest[key] = (depth, 2 if lit > 0.86 else 1 if lit > 0.45 else 0)

    def trace(level: int):
        inner: list[float | None] = [None] * bins
        outer: list[float | None] = [None] * bins
        for (kx, ky), (_, tone) in nearest.items():
            if tone < level:
                continue
            dx, dy = kx * cell - mx, ky * cell - my
            b = int((atan2(dy, dx) % TAU) / TAU * bins) % bins
            r = hypot(dx, dy)
            inner[b] = r if inner[b] is None else min(inner[b], r)
            outer[b] = r if outer[b] is None else max(outer[b], r)
        def loop(radii, grow):
            out = []
            for b, r in enumerate(radii):
                if r is None:
                    continue
                # Average neighbours so cell steps don't show as jaggies.
                near = [radii[(b + k) % bins] for k in (-2, -1, 0, 1, 2)]
                r = sum(n for n in near if n is not None) / sum(n is not None for n in near)
                a = TAU * (b + 0.5) / bins
                out.append((mx + (r + grow) * cos(a), my + (r + grow) * sin(a)))
            return out
        return _ccw(loop(outer, cell / 2)), _ccw(loop(inner, -cell / 2))

    outline, hole = trace(0)
    canvas.face(_keyhole(outline, hole), "right")
    for level, tone in ((1, "left"), (2, "top")):
        outer, inner = trace(level)
        canvas.face(_keyhole(outer, inner), tone)
    canvas.edge(outline, closed=True)
    canvas.edge(hole, closed=True)


def capsule_points(center: Vec3, radius: float, length: float) -> list[Vec3]:
    """An upright cylinder with hemispherical ends; `length` is end to end."""
    cx, cy, cz = center
    half = length / 2 - radius
    points = []
    for u in (TAU * i / solids.SAMPLES for i in range(solids.SAMPLES)):
        for j in range(17):
            v = pi * (j / 16 - 0.5)
            z = cz + radius * sin(v) + (half if v >= 0 else -half)
            points.append((cx + radius * cos(u) * cos(v), cy + radius * sin(u) * cos(v), z))
    return points


def capsule(canvas: Canvas):
    """Shaded like the sphere (nested tones, no inner ink), so it reads as
    one rounded body rather than a cylinder with lids."""
    center, radius, length = (0.5, 0.5, 0.5), 0.46, 1.62
    points = capsule_points(center, radius, length)
    projection = solids.fitted(points)
    outline = solids.hull([projection(p) for p in points])
    xs, ys = [p[0] for p in outline], [p[1] for p in outline]
    cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    w, h = (max(xs) - min(xs)) / 2, (max(ys) - min(ys)) / 2
    canvas.face(outline, "right")
    # Narrow the lighter zones more across than along, so the highlight is
    # a long stripe down the body, as light falls on a rounded rod.
    for tone, (sx, sy), (fx, fy) in (("left", (0.78, 0.88), (-0.16, -0.08)),
                                     ("top", (0.36, 0.62), (-0.36, -0.2))):
        canvas.face([(cx + fx * w + (x - cx) * sx, cy + fy * h + (y - cy) * sy)
                     for x, y in outline], tone)
    canvas.edge(outline, closed=True)


def arc(canvas: Canvas):
    """A quarter arc of edges on the ground plane, its vertices dotted, with
    faint radius guides back to the center it was drawn around. The arc
    sits at the right of its circle, where the ground plane's
    foreshortening squashes it least. The arc is cyan on an ink backing
    and its points are light dots in ink rings, so it reads on the dark
    theme too (it was faint there, 2026-11-05)."""
    projection = Projection()
    radius, steps = 1.2, 6
    points = [projection((radius * cos(a), radius * sin(a), 0.0))
              for a in (-pi / 2 + pi / 2 * i / steps for i in range(steps + 1))]
    middle = projection((0.0, 0.0, 0.0))
    canvas.edge([points[0], middle, points[-1]], "ink", SELECT_BACKING)
    canvas.edge([points[0], middle, points[-1]], "left", DETAIL)
    canvas.edge(points, "ink", SELECT_BACKING)
    canvas.edge(points, "new", SELECT)
    for p in points[1:-1]:
        canvas.vertex(p, 1.25)
        canvas.vertex(p, 0.6, "top")
    for p in (points[0], points[-1]):
        canvas.vertex(p, 1.7)
        canvas.vertex(p, 1.0, "top")
    canvas.vertex(middle, 1.25)
    canvas.vertex(middle, 0.6, "left")


def _offset(path: list[Point], distance: float) -> list[Point]:
    """`path` moved sideways by `distance` (screen space, left of travel)."""
    out = []
    for i, (x, y) in enumerate(path):
        a, b = path[max(i - 1, 0)], path[min(i + 1, len(path) - 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        n = hypot(dx, dy)
        out.append((x + dy / n * distance, y - dx / n * distance))
    return out


def _ellipse(center: Point, rx: float, ry: float, angle: float, samples: int = 32):
    c, s = cos(angle), sin(angle)
    return [(center[0] + rx * cos(t) * c - ry * sin(t) * s,
             center[1] + rx * cos(t) * s + ry * sin(t) * c)
            for t in (TAU * i / samples for i in range(samples))]


def _bezier(p: list[Point], t: float) -> Point:
    a, b, c, d = p
    m = 1 - t
    return tuple(m ** 3 * a[k] + 3 * m * m * t * b[k] + 3 * m * t * t * c[k] + t ** 3 * d[k]
                 for k in (0, 1))


def tube(canvas: Canvas):
    """A capped pipe swept through three cyan control points, lit along its
    length like the cylinder, its near cap facing the viewer."""
    # Screen-space S from the near end (lower left) up to the far end.
    controls = [(6.0, 25.0), (20.0, 27.0), (10.0, 6.0), (26.0, 7.0)]
    path = [_bezier(controls, i / 64) for i in range(65)]
    width = 3.2
    # Far end: round the pipe off with the back half of its cap.
    end, before = path[-1], path[-2]
    heading = atan2(end[1] - before[1], end[0] - before[0])
    far = [p for p in _ellipse(end, width * 0.45, width, heading, 48)
           if (p[0] - end[0]) * cos(heading) + (p[1] - end[1]) * sin(heading) > 0]
    far.sort(key=lambda p: atan2(p[1] - end[1], p[0] - end[0]) - heading)
    left, right = _offset(path, width), _offset(path, -width)
    body = left + far[::-1] + right[::-1]
    canvas.face(body, "right")
    # Lighter stripes on the upper side of the pipe, where the light hits.
    canvas.face(_offset(path, width * 0.92) + _offset(path, width * 0.02)[::-1], "left")
    canvas.face(_offset(path, width * 0.78) + _offset(path, width * 0.42)[::-1], "top")
    canvas.edge(body, closed=True)
    start, after = path[0], path[1]
    heading = atan2(after[1] - start[1], after[0] - start[0])
    cap = _ellipse(start, width * 0.5, width, heading)
    canvas.face(cap, "top")
    canvas.edge(cap, closed=True)
    for t in (0.3, 0.5, 0.7):
        canvas.vertex(_bezier(controls, t), 0.85, "new")


SHAPE_ICONS = {
    "torus": (torus, "Torus", "Flat ring with a clear hole, lit from the upper left."),
    "capsule": (capsule, "Capsule", "Upright rounded rod shaded with nested tones."),
    "arc": (arc, "Arc", "Cyan quarter arc of edges with dotted vertices around a center."),
    "tube": (tube, "Tube", "Pipe swept through cyan control points, its open end facing out."),
}
