"""Batch 15: the Edit tab's Boolean... button and its four operations.

Every icon is the shared cube (the mesh being changed) and a ball sunk into
the middle of its top (the driver mesh). The ball keeps the sphere's nested
tones, so the two meshes stay apart at a glance; cyan is what the operation
makes or keeps that wasn't surface before: the stitched seam and buried part
(Add) and the faces that close the hole (Subtract). What an operation throws
away is a thin outline in the left tone, like the "where it was" outlines of
batch 14.

Boolean... shows the two meshes before anything happens: the ball a cyan
outline through the cube's top. Union fuses them into one solid (an ink
seam, nothing inside kept). Add keeps both whole and stitches them (a cyan
seam, the ball's buried part seen through the cube). Subtract scoops the
ball out (a cyan bowl, the ball a ghost). Intersect keeps only what is
inside both (the ball's buried part, flat on top; both meshes ghosts).

Each piece of the ball cut by the cube's top plane is convex, so its outline
is the convex hull of its projected points (as in solids.py).
"""

from math import cos, pi, sin, sqrt

from .conventions import ACCENT, DETAIL, Point, Projection, Vec3
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted, front_half, hull, ring

CUBE = box((0, 0, 0), (1, 1, 1))
CENTER: Vec3 = (0.5, 0.5, 0.86)
RADIUS = 0.5
TOP_Z = 1.0
SAMPLES = 64


def _sphere_points(steps: int = 48) -> list[Vec3]:
    cx, cy, cz = CENTER
    out = []
    for i in range(steps):
        u = 2 * pi * i / steps
        for j in range(1, steps // 2):
            v = pi * j / (steps // 2) - pi / 2
            out.append((cx + RADIUS * cos(u) * cos(v), cy + RADIUS * sin(u) * cos(v),
                        cz + RADIUS * sin(v)))
    return out


def _projection() -> Projection:
    """One projection for the whole set, so the cube is the same size and
    place in all five whether or not the ball sticks out above it."""
    return fitted(corners(CUBE) + [(CENTER[0], CENTER[1], CENTER[2] + RADIUS)], fit=26)


class _Ball:
    """The ball in screen space. The projection is orthographic, so the
    whole ball is a circle; the seam (where it crosses the cube's top) is
    a horizontal circle, an ellipse on screen."""

    def __init__(self, projection: Projection):
        self.c = projection(CENTER)
        self.r = RADIUS * projection.unit * sqrt(1.5)
        seam_radius = sqrt(RADIUS ** 2 - (TOP_Z - CENTER[2]) ** 2)
        self.seam = [projection(p) for p in ring((CENTER[0], CENTER[1], TOP_Z), seam_radius, SAMPLES)]
        points = _sphere_points()
        self.above = hull([projection(p) for p in points if p[2] >= TOP_Z] + self.seam)
        self.below = hull([projection(p) for p in points if p[2] <= TOP_Z] + self.seam)

    def circle(self) -> list[Point]:
        return [(self.c[0] + self.r * cos(a), self.c[1] + self.r * sin(a))
                for a in (2 * pi * i / SAMPLES for i in range(SAMPLES))]


def _clip_convex(subject: list[Point], window: list[Point]) -> list[Point]:
    """Sutherland-Hodgman: the part of `subject` inside the convex `window`
    (either winding)."""
    def side(a, b, p):
        return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])

    area = sum(side(window[0], window[i], window[i + 1]) for i in range(1, len(window) - 1))
    sign = 1 if area > 0 else -1
    out = subject
    for i, a in enumerate(window):
        b = window[(i + 1) % len(window)]
        if not out:
            break
        current, out = out, []
        for j, p in enumerate(current):
            q = current[j - 1]
            dp, dq = side(a, b, p), side(a, b, q)
            pin, qin = sign * dp >= 0, sign * dq >= 0
            if pin != qin:
                t = dq / (dq - dp)
                out.append((q[0] + (p[0] - q[0]) * t, q[1] + (p[1] - q[1]) * t))
            if pin:
                out.append(p)
    return out


def _ball_tones(canvas: Canvas, ball: _Ball, window: list[Point]):
    """The sphere's nested tones (solids.ellipsoid), limited to `window`."""
    outline = ball.circle()
    cx, cy = ball.c
    canvas.face(_clip_convex(outline, window), "right")
    for tone, scale, (fx, fy) in (("left", 0.80, (-0.137, -0.137)),
                                  ("top", 0.45, (-0.256, -0.31))):
        zone = [(cx + fx * ball.r + (x - cx) * scale, cy + fy * ball.r + (y - cy) * scale)
                for x, y in outline]
        part = _clip_convex(zone, window)
        if len(part) > 2:
            canvas.face(part, tone)


def _cap(canvas: Canvas, ball: _Ball, seam: str = "ink"):
    """The ball's part above the cube's top, shaded and inked; its lower
    edge is the seam, redrawn in cyan when `seam` asks for it."""
    _ball_tones(canvas, ball, ball.above)
    canvas.edge(ball.above, closed=True)
    if seam != "ink":
        canvas.edge(front_half(ball.seam), seam, ACCENT)


def _ghost(canvas: Canvas, points: list[Point], closed: bool = True):
    canvas.edge(points, "left", DETAIL, closed=closed)


def boolean(canvas: Canvas):
    """Boolean...: the cube and the ball before the operation, the ball a
    cyan outline through the cube's top, crossing it along the seam."""
    projection = _projection()
    ball = _Ball(projection)
    canvas.mesh(projection, CUBE)
    canvas.edge(ball.circle(), "new", ACCENT, closed=True)
    canvas.edge(ball.seam, "new", DETAIL, closed=True)


def boolean_union(canvas: Canvas):
    """Union: one solid, the cube with the ball's top fused on; the seam is
    an ordinary inked edge and nothing inside is kept."""
    projection = _projection()
    ball = _Ball(projection)
    canvas.mesh(projection, CUBE)
    _cap(canvas, ball)


def boolean_add(canvas: Canvas):
    """Add: both meshes kept whole and stitched where they cross: a cyan
    seam, and the ball's buried part seen through the cube."""
    projection = _projection()
    ball = _Ball(projection)
    canvas.mesh(projection, CUBE)
    canvas.edge(ball.circle(), "new", DETAIL, closed=True)
    _cap(canvas, ball, seam="new")


def boolean_subtract(canvas: Canvas):
    """Subtract: the ball scooped out of the cube's top, the bowl's new
    faces cyan; the ball itself is gone (a ghost above)."""
    projection = _projection()
    ball = _Ball(projection)
    canvas.mesh(projection, CUBE)
    canvas.face(ball.seam, "new")
    canvas.edge(ball.seam, closed=True)
    above = ball.above
    # The ghost is the cap's outline without the rim, which is already inked.
    rim_y = min(p[1] for p in front_half(ball.seam))
    _ghost(canvas, [p for p in above if p[1] <= rim_y], closed=False)


def boolean_intersect(canvas: Canvas):
    """Intersect: only what is inside both, the ball's buried part with the
    cube's top as its flat lid; the rest of both meshes are ghosts."""
    projection = _projection()
    ball = _Ball(projection)
    for points, _ in CUBE:
        _ghost(canvas, [projection(p) for p in points])
    _ghost(canvas, ball.above)
    _ball_tones(canvas, ball, ball.below)
    canvas.face(ball.seam, "top")
    canvas.edge(ball.below, closed=True)
    canvas.edge(ball.seam, closed=True)


BOOLEAN_ICONS = {
    "boolean": (boolean, "Boolean", "A cube with a cyan ball outline sunk through its top, crossing it along a thin seam."),
    "boolean-union": (boolean_union, "Boolean Union", "A cube with the top of a ball fused onto it as one solid."),
    "boolean-add": (boolean_add, "Boolean Add", "A cube and a ball both kept, stitched along a cyan seam; the buried part shows through in cyan."),
    "boolean-subtract": (boolean_subtract, "Boolean Subtract", "A cyan bowl scooped out of a cube's top under the ghost of the ball."),
    "boolean-intersect": (boolean_intersect, "Boolean Intersect", "Only the ball's part inside the cube is left, flat on top; both shapes are ghosts."),
}
