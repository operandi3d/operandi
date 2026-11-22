"""Batch 14: the Snaps and Precision panel.

Every icon is the shared cube with one cue. Snap targets are cyan points in
an ink ring (a point that stands alone, as in batch 7): the place a drag or
a scale lands. Moves are cyan arrows; rotations are blue (about Z) arcs;
measured sizes are axis-coloured dimension lines with end ticks.
"""

from math import cos, hypot, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, GIZMO, Point, Projection, Vec3
from .create_icons import backed, backed_arrow, knot
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted

TAU = 2 * pi


def _target(canvas: Canvas, point: Point, radius: float = 1.9):
    """A snap target: a cyan point in an ink ring."""
    knot(canvas, point, radius, "new")


def _dimension(canvas: Canvas, a: Point, b: Point, color: str, tick: float = 1.7,
               ticks: int = 0, width: float = GIZMO):
    """A dimension line from a to b with end ticks (and optional inner
    ruler ticks), across the line in screen space."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = hypot(dx, dy)
    nx, ny = -dy / n, dx / n
    canvas.edge([a, b], color, width)
    for p, t in ((a, tick), (b, tick)):
        canvas.edge([(p[0] - nx * t, p[1] - ny * t), (p[0] + nx * t, p[1] + ny * t)], color, width)
    for k in range(1, ticks + 1):
        f = k / (ticks + 1)
        p = (a[0] + dx * f, a[1] + dy * f)
        t = tick * 0.6
        canvas.edge([p, (p[0] + nx * t, p[1] + ny * t)], color, ACCENT)


def drag_snap(canvas: Canvas):
    """Drag Snap: one corner of the cube dragged off onto a snap target;
    only that point moves, the faces stretch to follow."""
    corner = (1, 0, 1)
    moved = (1.55, -0.35, 1.3)
    faces = box((0, 0, 0), (1, 1, 1))
    pulled = [([moved if p == corner else p for p in pts], c) for pts, c in faces]
    projection = fitted(corners(pulled) + [corner], fit=25)
    canvas.mesh(projection, pulled)
    # Where the corner was: its old edges, thin.
    for points, _ in faces:
        if corner in points:
            i = points.index(corner)
            canvas.edge([projection(points[i - 1]), projection(corner),
                         projection(points[(i + 1) % len(points)])], "ink", DETAIL)
    start, end = projection(corner), projection(moved)
    d = hypot(end[0] - start[0], end[1] - start[1])
    stop = (end[0] - (end[0] - start[0]) / d * 2.6, end[1] - (end[1] - start[1]) / d * 2.6)
    backed_arrow(canvas, start, stop, length=2.2, half_width=1.4)
    _target(canvas, end)


def drag_snap_rigid(canvas: Canvas):
    """Drag Snap Rigid: the whole cube carried along with its corner, which
    lands on a snap target; where it was is a thin outline."""
    size = 0.8
    shift = (1.05, -0.75, 0.35)
    ghost = box((0, 0, 0), (size, size, size))
    lo = shift
    hi = (shift[0] + size, shift[1] + size, shift[2] + size)
    solid = box(lo, hi)
    projection = fitted(corners(ghost + solid), fit=26)
    for points, _ in ghost:
        canvas.edge([projection(p) for p in points], "left", DETAIL, closed=True)
    canvas.mesh(projection, solid)
    start = projection((size, size, size))
    end = projection(hi)
    backed_arrow(canvas, start, (end[0] - 0.9 * (end[0] - start[0]) / hypot(end[0] - start[0], end[1] - start[1]) * 1.9,
                                 end[1] - 0.9 * (end[1] - start[1]) / hypot(end[0] - start[0], end[1] - start[1]) * 1.9),
                 length=2.2, half_width=1.4)
    _target(canvas, end)


def _turn(canvas: Canvas, projection: Projection, center: Vec3, r: float, a0: float, a1: float):
    """A blue arc about Z with the turn's start and end points as targets."""
    arc = [projection((center[0] + r * cos(a), center[1] + r * sin(a), center[2]))
           for a in (a0 + (a1 - a0) * i / 40 for i in range(41))]
    canvas.edge(arc, "z", GIZMO)
    _target(canvas, arc[0], 1.6)
    _target(canvas, arc[-1], 1.6)


def planar_rotate_snap(canvas: Canvas):
    """Planar Rotate Snap: the cube turned about one axis, from a start
    point to an end point."""
    cube = box((0, 0, 0), (1, 1, 1))
    r = 1.15
    ring = [(0.5 + r * cos(a), 0.5 + r * sin(a), 0.0) for a in (TAU * i / 24 for i in range(24))]
    projection = fitted(corners(cube) + ring, fit=25)
    canvas.mesh(projection, cube)
    _turn(canvas, projection, (0.5, 0.5, 0.0), r, -0.15 * pi, 0.85 * pi)


def planar_rotate_snap_axis(canvas: Canvas):
    """Planar Rotate Snap Axis: the same turn, about an axis picked first
    (the blue line, its foot a target)."""
    cube = box((0, 0, 0), (0.8, 0.8, 0.8))
    r = 0.95
    ring = [(0.4 + r * cos(a), 0.4 + r * sin(a), 0.0) for a in (TAU * i / 24 for i in range(24))]
    projection = fitted(corners(cube) + ring + [(0.4, 0.4, 1.7)], fit=26)
    canvas.edge([projection((0.4, 0.4, 0.0)), projection((0.4, 0.4, 1.7))], "z", GIZMO)
    canvas.mesh(projection, cube)
    canvas.edge([projection((0.4, 0.4, 0.8)), projection((0.4, 0.4, 1.7))], "z", GIZMO)
    _turn(canvas, projection, (0.4, 0.4, 0.0), r, -0.15 * pi, 0.85 * pi)
    canvas.vertex(projection((0.4, 0.4, 1.7)), 1.2, "z")


def scale_absolute(canvas: Canvas):
    """Scale Absolute: a small cube (thin outline) grown into a bigger one,
    scaled from a start point until its edge reaches an end point."""
    small = box((0, 0, 0), (0.6, 1, 1))
    big = box((0, 0, 0), (1.6, 1, 1))
    projection = fitted(corners(big) + [(1.6, -0.35, 0)], fit=27)
    canvas.mesh(projection, big)
    for points, _ in small[1:]:
        canvas.edge([projection(p) for p in points], "right", DETAIL, closed=True)
    a, b = projection((0.0, -0.3, 0.0)), projection((1.6, -0.3, 0.0))
    canvas.arrow(a, b, "x", length=2.4, half_width=1.4, width=GIZMO)
    _target(canvas, a, 1.6)
    _target(canvas, b, 1.6)


def size_absolute(canvas: Canvas):
    """Size Absolute: the cube's size set along one axis from a start
    point: a red ruler with ticks running from the target to a set length."""
    cube = box((0, 0, 0), (1.5, 1, 1))
    projection = fitted(corners(cube) + [(1.5, 1.4, 0), (0, 1.4, 0)], fit=27)
    canvas.mesh(projection, cube)
    a, b = projection((0.0, 1.35, 0.0)), projection((1.5, 1.35, 0.0))
    _dimension(canvas, a, b, "x", ticks=3)
    _target(canvas, a, 1.6)


def advanced_handles(canvas: Canvas):
    """Advanced Handles: the extra handles a tool can show, here an angle
    wheel with marks round the cube and a grip on it."""
    cube = box((0, 0, 0), (1, 1, 1))
    projection = fitted(corners(cube), fit=16)
    canvas.mesh(projection, cube, width=EDGE)
    c, r = (16.0, 16.0), 13.0
    a0, a1 = -1.05 * pi, 0.25 * pi
    arc = [(c[0] + r * cos(a), c[1] + r * sin(a)) for a in (a0 + (a1 - a0) * i / 48 for i in range(49))]
    canvas.edge(arc, "z", GIZMO)
    for k in range(9):
        a = a0 + (a1 - a0) * k / 8
        inner = r - (3.0 if k % 4 == 0 else 1.8)
        canvas.edge([(c[0] + inner * cos(a), c[1] + inner * sin(a)),
                     (c[0] + r * cos(a), c[1] + r * sin(a))], "z", ACCENT)
    grip = (c[0] + r * cos(-0.3 * pi), c[1] + r * sin(-0.3 * pi))
    knot(canvas, grip, 2.2, "selected")


def absolute_scaling(canvas: Canvas):
    """Absolute Scaling: the cube with its exact size measured on all
    three axes: red, green and blue dimension lines standing off its
    edges, like a drawing's measurements."""
    cube = box((0, 0, 0), (1, 1, 1))
    g = 0.26
    ends = [(-g, 1 + g, 0), (1, 1 + g, 0), (1 + g, 1, 0), (1 + g, 0, 0), (-g, 1 + g, 1)]
    projection = fitted(corners(cube) + ends, fit=27)
    canvas.mesh(projection, cube)
    _dimension(canvas, projection((0, 1 + g, 0)), projection((1, 1 + g, 0)), "x", tick=1.5, width=ACCENT)
    _dimension(canvas, projection((1 + g, 0, 0)), projection((1 + g, 1, 0)), "y", tick=1.5, width=ACCENT)
    _dimension(canvas, projection((-g, 1 + g, 0)), projection((-g, 1 + g, 1)), "z", tick=1.5, width=ACCENT)


SNAPS_ICONS = {
    "drag-snap": (drag_snap, "Drag Snap", "A cube corner dragged by a cyan arrow onto a snap target."),
    "drag-snap-rigid": (drag_snap_rigid, "Drag Snap Rigid", "A whole cube carried along until its corner lands on a snap target."),
    "planar-rotate-snap": (planar_rotate_snap, "Planar Rotate Snap", "A blue turn about Z from a start target to an end target."),
    "planar-rotate-snap-axis": (planar_rotate_snap_axis, "Planar Rotate Snap Axis", "A blue turn about a picked blue axis, between two targets."),
    "scale-absolute": (scale_absolute, "Scale Absolute", "A cube stretched along a red arrow from a start target to an end target."),
    "size-absolute": (size_absolute, "Size Absolute", "A cube with a red ruler set from a start target."),
    "advanced-handles": (advanced_handles, "Advanced Handles", "A blue angle wheel with marks and a grip round a small cube, and a value bar."),
    "absolute-scaling": (absolute_scaling, "Absolute Scaling", "A cube measured on all three axes by red, green and blue dimension lines."),
}
