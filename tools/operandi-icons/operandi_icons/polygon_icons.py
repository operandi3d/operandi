"""Batch 2: Polygon tab tools, plus Scale and Transform (with Move/Rotate).

Same conventions as icons.py: unit-cube model coordinates, the shared
projection, yellow-orange = selected, cyan = new geometry, red/green/blue =
X/Y/Z. Each recipe centers its own geometry with solids.fitted.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, GIZMO, UNIT, Projection, Vec3
from .primitives import Canvas
from .solids import fitted


def box(lo: Vec3, hi: Vec3, top="top", left="left", right="right"):
    """The three visible faces of an axis-aligned box: +Z (top), +Y (screen
    left) and +X (screen right), in painter order."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    return [([(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], top),
            ([(x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)], left),
            ([(x1, y0, z1), (x1, y1, z1), (x1, y1, z0), (x1, y0, z0)], right)]


def corners(faces) -> list[Vec3]:
    return [p for points, _ in faces for p in points]


def lerp(a: Vec3, b: Vec3, t: float) -> Vec3:
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def chamfer_edit(canvas: Canvas):
    """A bevel strip along the top/right edge, refined into rounded rows."""
    amount, rows = 0.3, 3
    faces = [
        ([(0, 0, 1), (1 - amount, 0, 1), (1 - amount, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1 - amount, 1, 1), (1, 1, 1 - amount), (1, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1 - amount), (1, 1, 1 - amount), (1, 1, 0), (1, 0, 0)], "right"),
    ]
    # Rounded profile: points on a quarter circle around the old corner edge.
    def profile(y, k):
        angle = pi / 2 * k / rows
        return (1 - amount + amount * sin(angle), y, 1 - amount + amount * cos(angle))
    for k in range(rows):
        faces.append(([profile(0, k), profile(0, k + 1), profile(1, k + 1), profile(1, k)],
                      "new"))
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)


def poly_haul(canvas: Canvas):
    """The selected top face pushed along its normal."""
    faces = box((0, 0, 0), (1, 1, 0.75), top="selected")
    projection = fitted(corners(faces) + [(0.5, 0.5, 1.45)])
    canvas.mesh(projection, faces)
    canvas.arrow(projection((0.5, 0.5, 0.75)), projection((0.5, 0.5, 1.45)), "ink")


def sketch_extrude(canvas: Canvas):
    """A selected square profile and the curved cyan path it sweeps along."""
    profile = [(0, 0, 0), (1.2, 0, 0), (1.2, 1.2, 0), (0, 1.2, 0)]
    # Up from the profile, then curving away to the right.
    path = [(0.6, 0.6, 0), (0.6, 0.6, 1.1), (1.3, 0.3, 1.5), (2.3, -0.4, 1.35)]
    projection = fitted(profile + path, fit=24)
    canvas.highlight_face(projection, profile)
    canvas.cubic_arrow([projection(p) for p in path], "new")


def smooth_shift(canvas: Canvas):
    """Top and right faces pushed out together; the new walls are cyan."""
    d = 0.32
    faces = box((0, 0, 0), (1, 1, 1))
    shifted = [
        ([(0, 0, 1 + d), (1, 0, 1 + d), (1, 1, 1 + d), (0, 1, 1 + d)], "selected"),
        ([(0, 1, 1), (1, 1, 1), (1, 1, 1 + d), (0, 1, 1 + d)], "new"),
        ([(1, 1, 1), (1 + d, 1, 1), (1 + d, 1, 0), (1, 1, 0)], "new"),
        ([(1, 1, 1 + d), (1 + d, 1, 1), (1 + d, 0, 1), (1, 0, 1 + d)], "new"),
        ([(1 + d, 0, 1), (1 + d, 1, 1), (1 + d, 1, 0), (1 + d, 0, 0)], "selected"),
    ]
    projection = fitted(corners(faces + shifted))
    canvas.mesh(projection, [faces[1]] + shifted)


def thicken(canvas: Canvas):
    """A single surface given thickness: a thin plate with cyan new sides."""
    faces = box((0, 0, 0.42), (1.2, 1.2, 0.68), top="top", left="new", right="new")
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)


def pen(canvas: Canvas):
    """A flat polygon drawn point by point; the newest edge is cyan."""
    points = [(0, 0.4, 0), (1.3, 0, 0), (2.2, 1.0, 0), (1.7, 2.2, 0), (0.3, 1.9, 0)]
    projection = fitted(points, fit=24)
    canvas.face([projection(p) for p in points], "top")
    canvas.edge([projection(p) for p in points[:-1]])
    canvas.edge([projection(points[-1]), projection(points[0])])
    canvas.edge([projection(points[-2]), projection(points[-1])], "new", ACCENT)
    for p in points:
        canvas.vertex(projection(p), 1.35)


def spike(canvas: Canvas):
    """A cyan pyramid rising from the cube's top face."""
    apex = (0.5, 0.5, 1.75)
    faces = box((0, 0, 0), (1, 1, 1))
    top = [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)]
    spikes = [([top[i], top[(i + 1) % 4], apex], "new") for i in (3, 1, 2)]
    projection = fitted(corners(faces) + [apex])
    canvas.mesh(projection, faces[1:] + [(top, "top")] + spikes)


def bridge(canvas: Canvas):
    """Two blocks whose facing sides are joined by a cyan bridge."""
    left = box((0, 0, 0), (0.7, 0.7, 0.7))
    right = box((1.8, 0, 0), (2.5, 0.7, 0.7))
    span = box((0.7, 0.15, 0.2), (1.8, 0.55, 0.55), top="new", left="new", right="new")[:2]
    projection = fitted(corners(left + right + span))
    canvas.mesh(projection, left + span + right)


def polygon_reduction(canvas: Canvas):
    """A dense grid on the left side, a coarse one on the right."""
    projection = Projection()
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    n = 4
    for i in range(1, n):
        t = i / n
        canvas.edge([projection((t, 1, 1)), projection((t, 1, 0))], width=DETAIL)
        canvas.edge([projection((0, 1, t)), projection((1, 1, t))], width=DETAIL)
    canvas.edge([projection((1, 0, 1)), projection((1, 1, 0))], "new", ACCENT)


def triple(canvas: Canvas):
    """Every visible face split into triangles by a cyan diagonal."""
    projection = Projection()
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    for a, b in (((0, 0, 1), (1, 1, 1)), ((0, 1, 1), (1, 1, 0)), ((1, 0, 1), (1, 1, 0))):
        canvas.edge([projection(a), projection(b)], "new", ACCENT)


def script_action(canvas: Canvas):
    """A page with a folded corner and a </> code mark."""
    page = [(8, 3.5), (20, 3.5), (25, 8.5), (25, 28.5), (8, 28.5)]
    canvas.face(page, "top")
    canvas.edge(page, closed=True)
    canvas.face([(20, 3.5), (20, 8.5), (25, 8.5)], "left")
    canvas.edge([(20, 3.5), (20, 8.5), (25, 8.5)])
    canvas.edge([(14, 13.5), (11, 17), (14, 20.5)], "new", ACCENT)
    canvas.edge([(19, 13.5), (22, 17), (19, 20.5)], "new", ACCENT)
    canvas.edge([(17.6, 12.6), (15.4, 21.4)], "new", ACCENT)


def _axes(canvas: Canvas, projection: Projection, origin: Vec3, extent: float):
    ends = {"x": (origin[0] + extent, origin[1], origin[2]),
            "y": (origin[0], origin[1] + extent, origin[2]),
            "z": (origin[0], origin[1], origin[2] + extent)}
    return projection(origin), {k: projection(v) for k, v in ends.items()}


def scale(canvas: Canvas):
    """Axis lines from a cube corner ending in square handles."""
    projection = Projection(UNIT * 0.98 / 1.2)
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    origin, ends = _axes(canvas, projection, (1, 1, 1), 27 / 21.5)
    for color, end in ends.items():
        canvas.edge([origin, end], color, GIZMO)
        h = 1.6
        canvas.face([(end[0] - h, end[1] - h), (end[0] + h, end[1] - h),
                     (end[0] + h, end[1] + h), (end[0] - h, end[1] + h)], color)
    canvas.vertex(origin)


def transform(canvas: Canvas):
    """Move arrows inside a rotation ring: move, rotate and scale together."""
    canvas.guide((16, 16), 12.6, DETAIL)
    projection = Projection(UNIT * 0.62, (16, 17.5))
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)), width=DETAIL)
    origin, ends = _axes(canvas, projection, (1, 1, 1), 1.25)
    for color, end in ends.items():
        canvas.arrow(origin, end, color, length=2.4, half_width=1.3, width=GIZMO)
    canvas.vertex(origin, 0.6)


def twist_bevel(canvas: Canvas):
    """A cyan tube of rings rising from the selected top face, tapering,
    twisting and curling to one side, with its cap selected."""
    base = box((0, 0, 0), (1, 1, 0.42))
    centre, rings, length = (0.5, 0.5, 0.42), 3, 1.25
    bend, twist = 55 * pi / 180, 45 * pi / 180

    def ring(k):
        t = k / rings
        a = bend * t
        radius = length / bend
        # The spine curls from +Z toward +Y (up and left on screen), so
        # the cap still faces the viewer.
        cx = centre[0]
        cy = centre[1] + radius * (1 - cos(a))
        cz = centre[2] + radius * sin(a)
        half = 0.3 * (1 - 0.3 * t)
        points = []
        for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            x, y = sx * half, sy * half
            # Twist about the spine, then tilt the ring square to it.
            x, y = x * cos(twist * t) - y * sin(twist * t), x * sin(twist * t) + y * cos(twist * t)
            y, z = y * cos(a), -y * sin(a)
            points.append((cx + x, cy + y, cz + z))
        return points

    def facing(points):
        # Faces toward the viewer (+X, +Y, +Z) are drawn.
        a, b, c = points[0], points[1], points[2]
        u = [b[i] - a[i] for i in range(3)]
        v = [c[i] - a[i] for i in range(3)]
        n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        return sum(n) > 0

    def depth(points):
        return sum(sum(p) for p in points) / len(points)

    tube = []
    for k in range(rings):
        low, high = ring(k), ring(k + 1)
        for i in range(4):
            wall = [low[i], low[(i + 1) % 4], high[(i + 1) % 4], high[i]]
            if facing(wall):
                tube.append((wall, "new"))
    tube.sort(key=lambda face: depth(face[0]))
    cap = ring(rings)
    points = corners(base) + corners(tube) + cap
    projection = fitted(points)
    canvas.mesh(projection, base)
    # Painter order, each face with its own outline, so a wall behind
    # another never shows its edges through it.
    for face, colour in tube + [(cap, "selected")]:
        projected = [projection(p) for p in face]
        canvas.face(projected, colour)
        canvas.edge(projected, closed=True)


POLYGON_ICONS = {
    "chamfer-edit": (chamfer_edit, "Chamfer Edit", "A bevel strip refined into cyan rounded rows."),
    "poly-haul": (poly_haul, "Poly Haul", "Selected top face with an arrow pushing it outward."),
    "sketch-extrude": (sketch_extrude, "Sketch Extrude", "Selected square profile and a cyan curved path to sweep along."),
    "smooth-shift": (smooth_shift, "Smooth Shift", "Top and side faces pushed out together, joined by cyan walls."),
    "thicken": (thicken, "Thicken", "A thin plate whose new sides are cyan."),
    "pen": (pen, "Pen", "A flat polygon with its points; the newest edge is cyan."),
    "spike": (spike, "Spike", "A cyan pyramid rising from the cube's top face."),
    "twist-bevel": (twist_bevel, "Twist Bevel", "A cyan tube of rings rising from the top face, tapering, twisting and curling, with its cap selected."),
    "bridge": (bridge, "Bridge", "Two blocks joined by a cyan bridge."),
    "poly-reduct": (polygon_reduction, "Polygon Reduction", "Dense grid on one side, a single cyan diagonal on the other."),
    "triple": (triple, "Triple", "Cube faces split into triangles by cyan diagonals."),
    "script-action": (script_action, "Script Action", "A page with a folded corner and a cyan code mark."),
    "scale": (scale, "Scale", "Red, green and blue axes ending in square handles."),
    "transform": (transform, "Transform", "Axis arrows on a small cube inside a rotation ring."),
}
