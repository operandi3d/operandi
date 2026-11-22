"""Batch 8: the Deform tab, the Edit tab's Slice tools, and Shift.

Same conventions: yellow-orange = selected, cyan = new geometry or the
tool's cue, red/green/blue = X/Y/Z. Most Deform tools reshape a mesh rather
than add to it, so their icons show the reshaped mesh itself: a bent bar, a
twisted column, a dented plane. `shaded` draws such meshes with the cube's
three tones, picked per facet from its normal. Cues that stand alone off the
mesh are backed lines (cyan on an ink backing) so they read on both themes,
as in batch 7.
"""

from math import atan2, cos, exp, hypot, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, GIZMO, Point, Projection, Vec3
from .create_icons import backed, backed_arrow, knot
from .edge_icons import selected_edge
from .polygon_icons import box, corners, lerp
from .primitives import Canvas
from .solids import fitted
from .vertex_icons import new_point, selected_point
from . import solids

TAU = 2 * pi
# The viewer looks along -(1, 1, 1): a facet faces it when its normal has a
# positive dot product with (1, 1, 1), and nearer facets have a larger sum.
VIEW = (1.0, 1.0, 1.0)

Facet = tuple[list[Vec3], str | None]


# ------------------------------------------------------------- helpers

def _sub(a: Vec3, b: Vec3) -> Vec3:
    return tuple(x - y for x, y in zip(a, b))


def _cross(a: Vec3, b: Vec3) -> Vec3:
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def _dot(a: Vec3, b: Vec3) -> float:
    return sum(x * y for x, y in zip(a, b))


def _mean(points: list[Vec3]) -> Vec3:
    return tuple(sum(p[k] for p in points) / len(points) for k in range(3))


def _normal(points: list[Vec3]) -> Vec3:
    """Newell's normal: robust for slightly non-planar quads."""
    n = [0.0, 0.0, 0.0]
    for a, b in zip(points, points[1:] + points[:1]):
        n[0] += (a[1] - b[1]) * (a[2] + b[2])
        n[1] += (a[2] - b[2]) * (a[0] + b[0])
        n[2] += (a[0] - b[0]) * (a[1] + b[1])
    return tuple(n)


def tone(normal: Vec3) -> str:
    """The cube tone for a facet facing this way: up is the light top,
    +Y the left side, +X the darker right side."""
    nx, ny, nz = normal
    if nz >= max(nx, ny) * 0.9:
        return "top"
    return "left" if ny >= nx else "right"


def shaded(canvas: Canvas, projection: Projection, facets: list[Facet],
           inside: Vec3 | None = None, width: float = DETAIL):
    """Fill and outline facets back to front. Facets turned away from the
    viewer are skipped; `inside` (a point inside a closed solid) decides
    which way each facet's normal points. A facet's color is its own, or the
    tone its normal picks."""
    visible = []
    for points, color in facets:
        normal = _normal(points)
        if inside is not None and _dot(normal, _sub(_mean(points), inside)) < 0:
            normal = tuple(-v for v in normal)
        elif inside is None and normal[2] < 0:
            normal = tuple(-v for v in normal)  # open sheets face up
        if _dot(normal, VIEW) <= 1e-9:
            continue
        visible.append((_dot(_mean(points), VIEW), points, color or tone(normal)))
    for _, points, color in sorted(visible, key=lambda v: v[0]):
        projected = [projection(p) for p in points]
        canvas.face(projected, color)
        canvas.edge(projected, width=width, closed=True)


def outline(canvas: Canvas, projection: Projection, points: list[Vec3]):
    """A heavier ink border round a sheet, the way the cube's edges read."""
    canvas.edge([projection(p) for p in points], width=EDGE, closed=True)


def sheet(height, n: int = 6, size: float = 2.0) -> list[Facet]:
    """An n × n grid sheet over [0, size]², lifted by height(x, y)."""
    step = size / n
    def at(i, j):
        x, y = i * step, j * step
        return (x, y, height(x, y))
    return [([at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)], None)
            for i in range(n) for j in range(n)]


def sheet_border(height, n: int = 6, size: float = 2.0) -> list[Vec3]:
    step = size / n
    side = [i * step for i in range(n)]
    ring = ([(x, 0) for x in side] + [(size, y) for y in side]
            + [(size - x, size) for x in side] + [(0, size - y) for y in side])
    return [(x, y, height(x, y)) for x, y in ring]


def loft(sections: list[list[Vec3]], colors: dict | None = None) -> list[Facet]:
    """Side quads between consecutive cross sections, plus both end caps.
    `colors` maps a facet index ("cap0", "cap1" or (level, side)) to a color."""
    colors = colors or {}
    facets = []
    for i, (a, b) in enumerate(zip(sections, sections[1:])):
        k = len(a)
        for j in range(k):
            facets.append(([a[j], a[(j + 1) % k], b[(j + 1) % k], b[j]], colors.get((i, j))))
    facets.append((sections[0], colors.get("cap0")))
    facets.append((sections[-1], colors.get("cap1")))
    return facets


def square(center: Vec3, half: float, z_axis: Vec3 = (0, 0, 1), angle: float = 0.0,
           half_y: float | None = None) -> list[Vec3]:
    """A square cross section around `center`, perpendicular to `z_axis`
    (which must be in the X–Z plane), turned by `angle`."""
    ax = z_axis
    u = (ax[2], 0.0, -ax[0])  # in the X–Z plane, perpendicular to the axis
    v = (0.0, 1.0, 0.0)
    hy = half if half_y is None else half_y
    out = []
    for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
        a, b = sx * half, sy * hy
        ca, sa = cos(angle), sin(angle)
        a, b = a * ca - b * sa, a * sa + b * ca
        out.append(tuple(center[k] + a * u[k] + b * v[k] for k in range(3)))
    return out


def column(radius_at, levels: int = 5, height: float = 2.0, angle_at=lambda t: 0.0,
           offset_at=lambda t: (0.0, 0.0)) -> list[list[Vec3]]:
    """Square sections stacked up the Z axis: each level's half-width,
    turn and sideways offset come from functions of t in [0, 1]."""
    out = []
    for i in range(levels + 1):
        t = i / levels
        dx, dy = offset_at(t)
        out.append(square((dx, dy, height * t), radius_at(t), angle=angle_at(t)))
    return out


def _all(facets: list[Facet]) -> list[Vec3]:
    return [p for points, _ in facets for p in points]


def bump(peak: float, spread: float, cx: float = 1.0, cy: float = 1.0):
    return lambda x, y: peak * exp(-((x - cx) ** 2 + (y - cy) ** 2) / spread ** 2)


def _grid_sheet(canvas: Canvas, height, n=6, extra: list[Vec3] = (), fit=25.5):
    facets = sheet(height, n)
    projection = fitted(_all(facets) + list(extra), fit=fit)
    shaded(canvas, projection, facets)
    outline(canvas, projection, sheet_border(height, n))
    return projection


# ------------------------------------------------------------- Move Tools

def element_move(canvas: Canvas):
    """Tweak: the cube's front corner grabbed and pulled straight off the
    mesh; nothing selected, the grabbed point is cyan."""
    # Pulled up and to the right on screen (not along the view direction).
    moved = (1.6, 0.95, 1.5)
    faces = [
        ([(0, 0, 1), (1, 0, 1), moved, (0, 1, 1)], "top"),
        ([(0, 1, 1), moved, (1, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1), moved, (1, 1, 0), (1, 0, 0)], "right"),
    ]
    projection = fitted(corners(faces), fit=23)
    canvas.mesh(projection, faces)
    new_point(canvas, projection(moved), 1.6)


def element_push(canvas: Canvas):
    """Element Push: one point of a sheet pushed in along its normal."""
    dent = lambda x, y: -0.75 * max(0.0, 1 - hypot(x - 1, y - 1) / 0.68)
    projection = _grid_sheet(canvas, dent, n=4, extra=[(1, 1, 1.0)], fit=24)
    backed_arrow(canvas, projection((1, 1, 1.05)), projection((1, 1, -0.6)),
                 length=2.4, half_width=1.5)


def flex(canvas: Canvas):
    """Flex: a bar flexed into an S, its sections staying square."""
    sections = []
    for i in range(9):
        t = i / 8
        x = 2.6 * t
        z = 0.5 * sin(TAU * t)
        slope = 0.5 * TAU * cos(TAU * t) / 2.6
        axis = (1 / hypot(1, slope), 0, slope / hypot(1, slope))
        sections.append(square((x, 0, z), 0.3, axis))
    _loft_icon(canvas, sections)


def _solid(canvas: Canvas, projection: Projection, sections, facets):
    """A lofted solid: each facet's outward side comes from its own section
    centers, so bends and twists still shade the right way."""
    centers = [_mean(s) for s in sections]
    visible = []
    k = len(sections[0])
    for index, (points, color) in enumerate(facets):
        level = index // k if index < k * (len(sections) - 1) else None
        if level is None:
            inside = centers[1] if index == len(facets) - 2 else centers[-2]
        else:
            inside = lerp(centers[level], centers[level + 1], 0.5)
        normal = _normal(points)
        if _dot(normal, _sub(_mean(points), inside)) < 0:
            normal = tuple(-v for v in normal)
        if _dot(normal, VIEW) <= 1e-9:
            continue
        visible.append((_dot(_mean(points), VIEW), points, color or tone(normal)))
    for _, points, color in sorted(visible, key=lambda v: v[0]):
        projected = [projection(p) for p in points]
        canvas.face(projected, color)
        canvas.edge(projected, width=DETAIL, closed=True)


def _loft_icon(canvas: Canvas, sections, colors=None, extra=(), fit=25.5):
    facets = loft(sections, colors)
    projection = fitted(_all(facets) + list(extra), fit=fit)
    _solid(canvas, projection, sections, facets)
    return projection


def soft_drag(canvas: Canvas):
    """Soft Drag: a sheet dragged up by its middle, the pull fading out;
    a free cyan stroke shows the drag."""
    lift = bump(0.95, 0.55)
    projection = _grid_sheet(canvas, lift, n=4, extra=[(1.9, 0.2, 1.6)])
    start = projection((1, 1, 0.95))
    end = projection((1.75, 0.35, 1.55))
    path = [start, (start[0] + 2.0, start[1] - 4.5), (end[0] - 4.0, end[1] + 1.5), end]
    from .create_icons import sampled
    curve = sampled(path, 16)
    backed(canvas, curve[:-2])
    backed_arrow(canvas, curve[-4], end, length=2.3, half_width=1.4)
    knot(canvas, start, 1.4, "new")


def soft_move(canvas: Canvas):
    """Soft Move: a sheet raised in the middle with a radial falloff,
    the Move gizmo's blue Z arrow at its peak."""
    lift = bump(0.9, 0.6)
    projection = _grid_sheet(canvas, lift, n=4, extra=[(1, 1, 2.0)])
    canvas.arrow(projection((1, 1, 0.9)), projection((1, 1, 2.0)), "z",
                 length=2.6, half_width=1.45, width=GIZMO)
    canvas.vertex(projection((1, 1, 0.9)), 0.9)


def shear(canvas: Canvas):
    """Shear: a block whose top slides along X while its base stays put."""
    sections = column(lambda t: 0.5, levels=4, height=1.6,
                      offset_at=lambda t: (0.85 * t, 0.0))
    projection = _loft_icon(canvas, sections, extra=[(2.0, 0, 1.6)])
    top = _mean(sections[-1])
    canvas.arrow(projection((top[0] - 0.3, -0.5, 1.6)), projection((top[0] + 0.7, -0.5, 1.6)),
                 "x", length=2.6, half_width=1.45, width=GIZMO)


def smooth(canvas: Canvas):
    """Smooth: a slab whose top is jagged at the back end and evened out
    at the front end."""
    tops = [0.95, 1.35, 0.85, 1.4, 0.9, 1.25, 1.12, 1.1, 1.1, 1.1]
    xs = [0.3 * i for i in range(len(tops))]
    depth = 0.9
    front = [(xs[0], depth, 0.0)] + [(x, depth, z) for x, z in zip(xs, tops)] + [(xs[-1], depth, 0.0)]
    facets = [(front, "left"),
              ([(xs[-1], 0, 0), (xs[-1], depth, 0), (xs[-1], depth, tops[-1]), (xs[-1], 0, tops[-1])], "right")]
    for (x0, z0), (x1, z1) in zip(zip(xs, tops), zip(xs[1:], tops[1:])):
        facets.append(([(x0, 0, z0), (x1, 0, z1), (x1, depth, z1), (x0, depth, z0)], None))
    projection = fitted(_all(facets), fit=26)
    shaded(canvas, projection, facets[2:])
    for points, color in facets[:2]:
        canvas.face([projection(p) for p in points], color)
        canvas.edge([projection(p) for p in points], width=EDGE, closed=True)
    # The smoothed run, in cyan along the front edge.
    canvas.edge([projection((x, depth, z)) for x, z in zip(xs[5:], tops[5:])], "new", ACCENT)


def jitter(canvas: Canvas):
    """Jitter: a sheet's points thrown off the grid at random."""
    offsets = [0.31, -0.22, 0.12, -0.35, 0.27, -0.05, 0.19, -0.29, 0.36, -0.14,
               0.08, -0.31, 0.24, -0.18, 0.33, -0.26, 0.15, -0.09, 0.28, -0.34,
               0.05, 0.21, -0.25, 0.3, -0.12]
    def height(x, y):
        i, j = round(x / 0.5), round(y / 0.5)
        return offsets[(i * 5 + j) % len(offsets)] if 0 < i < 4 and 0 < j < 4 else 0.0
    projection = _grid_sheet(canvas, height, n=4, fit=24)
    for i in (1, 2, 3):
        for j in (1, 2, 3):
            new_point(canvas, projection((i * 0.5, j * 0.5, height(i * 0.5, j * 0.5))), 1.0)


def quantize(canvas: Canvas):
    """Quantize: a slope snapped into even terraces, rising to the back."""
    steps = []
    for k in range(4):
        x0, x1 = 0.6 * k, 0.6 * (k + 1)
        steps.extend(box((x0, 0, 0), (x1, 1.4, 0.35 * (4 - k))))
    projection = fitted(corners(steps), fit=26)
    # Back to front along X: each nearer, lower step covers the one behind.
    for k in range(4):
        canvas.mesh(projection, steps[3 * k:3 * k + 3])


# Soft Selection: a flat sheet whose selection fades out from the middle,
# with the transform tool's gizmo at its center.

def _soft_selection(canvas: Canvas, extra=()):
    n = 4
    facets = sheet(lambda x, y: 0.0, n, 2.0)
    projection = fitted(_all(facets) + list(extra), fit=26)
    shaded(canvas, projection, facets)
    outline(canvas, projection, sheet_border(lambda x, y: 0.0, n))
    step = 2.0 / n
    middle = [(0.5, 0.5, 0), (1.5, 0.5, 0), (1.5, 1.5, 0), (0.5, 1.5, 0)]
    canvas.face([projection(p) for p in middle], "selected")
    canvas.edge([projection(p) for p in middle], width=DETAIL, closed=True)
    del step
    # The falloff: selected points shrink with distance from the middle.
    for x, y, r in ((0.5, 0.5, 1.3), (1.5, 0.5, 1.3), (1.5, 1.5, 1.3), (0.5, 1.5, 1.3),
                    (0.0, 1.0, 0.85), (2.0, 1.0, 0.85), (1.0, 0.0, 0.85), (1.0, 2.0, 0.85)):
        selected_point(canvas, projection((x, y, 0)), r)
    return projection


def soft_selection_move(canvas: Canvas):
    projection = _soft_selection(canvas, [(1, 1, 1.6)])
    o = projection((1, 1, 0))
    for color, end in (("x", (2.0, 1, 0)), ("y", (1, 2.0, 0)), ("z", (1, 1, 1.6))):
        canvas.arrow(o, projection(end), color, length=2.5, half_width=1.4, width=GIZMO)
    canvas.vertex(o, 0.9)


def soft_selection_rotate(canvas: Canvas):
    projection = _soft_selection(canvas, [(1, 1, 1.3)])
    c = projection((1, 1, 0))
    ring = [projection((1 + 0.85 * cos(a), 1 + 0.85 * sin(a), 0.75))
            for a in (TAU * (0.1 + 0.75 * i / 24) for i in range(25))]
    canvas.edge(ring[:-1], "z", GIZMO)
    canvas.arrowhead(ring[-1], (ring[-1][0] - ring[-2][0], ring[-1][1] - ring[-2][1]),
                     "z", 2.5, 1.4, GIZMO)
    canvas.edge([c, projection((1, 1, 0.75))], "ink", EDGE)
    canvas.vertex(c, 0.9)


def soft_selection_scale(canvas: Canvas):
    projection = _soft_selection(canvas, [(1, 1, 1.6)])
    o = projection((1, 1, 0))
    for color, end in (("x", (2.0, 1, 0)), ("y", (1, 2.0, 0)), ("z", (1, 1, 1.6))):
        e = projection(end)
        canvas.edge([o, e], color, GIZMO)
        h = 1.5
        canvas.face([(e[0] - h, e[1] - h), (e[0] + h, e[1] - h),
                     (e[0] + h, e[1] + h), (e[0] - h, e[1] + h)], color)
    canvas.vertex(o, 0.9)


def soft_selection_transform(canvas: Canvas):
    projection = _soft_selection(canvas, [(1, 1, 1.6)])
    o = projection((1, 1, 0))
    ring = [projection((1 + 0.85 * cos(a), 1 + 0.85 * sin(a), 0.0))
            for a in (TAU * i / 32 for i in range(33))]
    canvas.edge(ring, "z", ACCENT)
    for color, end in (("x", (1.75, 1, 0)), ("y", (1, 1.75, 0)), ("z", (1, 1, 1.5))):
        canvas.arrow(o, projection(end), color, length=2.3, half_width=1.3, width=GIZMO)
    canvas.vertex(o, 0.9)


# ------------------------------------------------------------- Rotate Tools

def twist(canvas: Canvas):
    """Twist: a square column wrung about its axis, more toward the top."""
    sections = column(lambda t: 0.55, levels=6, height=2.2, angle_at=lambda t: 1.35 * t)
    _loft_icon(canvas, sections, colors={"cap1": "top"})


def bend(canvas: Canvas):
    """Bend: a straight bar bent up through a quarter turn at one end."""
    sections = []
    radius = 0.9
    for i in range(9):
        t = i / 8
        if t <= 0.4:
            x, z, a = 2.0 * t / 0.4 * 0.6, 0.0, 0.0
        else:
            a = (t - 0.4) / 0.6 * pi / 2
            x, z = 1.2 + radius * sin(a), radius * (1 - cos(a))
        sections.append(square((x, 0, z), 0.32, (cos(a), 0, sin(a))))
    projection = _loft_icon(canvas, sections)
    del projection


def vortex(canvas: Canvas):
    """Vortex: a disc whose rings turn more the farther out they are,
    drawn as a cyan spiral."""
    disc = solids.ring((0, 0, 0), 1.0)
    projection = fitted(disc + [(0, 0, 0.4)], fit=26)
    canvas.face([projection(p) for p in disc], "top")
    canvas.edge([projection(p) for p in disc], width=EDGE, closed=True)
    for r in (0.33, 0.66):
        ring = solids.ring((0, 0, 0), r, 32)
        canvas.edge([projection(p) for p in ring], width=DETAIL, closed=True)
    spiral = [projection((r * cos(a), r * sin(a), 0))
              for r, a in ((0.05 + 0.9 * t, TAU * 1.6 * t) for t in (i / 64 for i in range(65)))]
    backed(canvas, spiral[:-2])
    backed_arrow(canvas, spiral[-4], spiral[-1], length=2.2, half_width=1.4)


def swirl(canvas: Canvas):
    """Swirl: a sheet's grid turned about its middle, most at the center."""
    n, size = 4, 2.0
    def turned(x, y):
        dx, dy = x - 1, y - 1
        r = hypot(dx, dy)
        a = 2.2 * max(0.0, 1 - r / 1.05) ** 1.5
        return (1 + dx * cos(a) - dy * sin(a), 1 + dx * sin(a) + dy * cos(a), 0.0)
    border = sheet_border(lambda x, y: 0.0, n, size)
    projection = fitted(border, fit=26)
    canvas.face([projection(p) for p in border], "top")
    for k in range(1, n):
        s = k * size / n
        for line in ([(s, t) for t in (i * size / 40 for i in range(41))],
                     [(t, s) for t in (i * size / 40 for i in range(41))]):
            canvas.edge([projection(turned(x, y)) for x, y in line], width=DETAIL)
    canvas.edge([projection(p) for p in border], width=EDGE, closed=True)
    arc = [projection((1 + 0.78 * cos(a), 1 + 0.78 * sin(a), 0.35))
           for a in (TAU * (0.6 + 0.55 * i / 20) for i in range(21))]
    backed(canvas, arc[:-1])
    backed_arrow(canvas, arc[-3], arc[-1], length=2.2, half_width=1.4)


# ------------------------------------------------------------- Scale Tools

def push(canvas: Canvas):
    """Push: a ball with its points pushed out along their normals."""
    radii = (0.7, 0.7, 0.7)
    center = (0.5, 0.5, 0.5)
    points = solids.ellipsoid_points(center, (1.25, 1.25, 1.25))
    projection = fitted(points, fit=26)
    solids.ellipsoid(canvas, projection, center, radii)
    c = projection(center)
    for angle in (-90, -18, 54, 126, 198):
        a = angle * pi / 180
        r0, r1 = 7.0, 12.4
        start = (c[0] + r0 * cos(a), c[1] + r0 * sin(a))
        end = (c[0] + r1 * cos(a), c[1] + r1 * sin(a))
        backed_arrow(canvas, start, end, length=2.2, half_width=1.4)


def sculpt(canvas: Canvas):
    """Sculpt: a brush ring on a sheet, raising the surface inside it."""
    lift = bump(0.55, 0.4)
    projection = _grid_sheet(canvas, lift, n=4)
    ring = [projection((1 + 0.62 * cos(a), 1 + 0.62 * sin(a), lift(1 + 0.62 * cos(a), 1 + 0.62 * sin(a)) + 0.02))
            for a in (TAU * i / 40 for i in range(40))]
    backed(canvas, ring, closed=True)
    knot(canvas, projection((1, 1, 0.55)), 1.3, "new")


def taper(canvas: Canvas):
    """Taper: a column scaled down toward its top, the base untouched."""
    sections = column(lambda t: 0.7 - 0.42 * t, levels=4, height=2.0)
    _loft_icon(canvas, sections, colors={"cap1": "top"})


def bulge(canvas: Canvas):
    """Bulge: a column swollen around its middle."""
    sections = column(lambda t: 0.4 + 0.38 * sin(pi * t), levels=6, height=2.0)
    _loft_icon(canvas, sections, colors={"cap1": "top"})


def flare(canvas: Canvas):
    """Flare: a column that opens out toward its top."""
    sections = column(lambda t: 0.3 + 0.55 * t ** 2.4, levels=6, height=2.0)
    _loft_icon(canvas, sections, colors={"cap1": "top"})


# ------------------------------------------------------------- Align Tools

def radial_align(canvas: Canvas):
    """Radial: scattered selected points drawn onto a cyan circle."""
    plane = [(0, 0, 0), (2, 0, 0), (2, 2, 0), (0, 2, 0)]
    projection = fitted(plane, fit=26)
    canvas.face([projection(p) for p in plane], "top")
    canvas.edge([projection(p) for p in plane], width=EDGE, closed=True)
    circle = [projection((1 + 0.72 * cos(a), 1 + 0.72 * sin(a), 0)) for a in (TAU * i / 40 for i in range(40))]
    backed(canvas, circle, closed=True)
    for angle, r in ((0.3, 0.32), (1.9, 0.38), (3.5, 0.3), (5.0, 0.4)):
        selected_point(canvas, projection((1 + 0.72 * cos(angle), 1 + 0.72 * sin(angle), 0)), 1.35)
        canvas.vertex(projection((1 + r * cos(angle + 0.4), 1 + r * sin(angle + 0.4), 0)), 0.75)


def linear_align(canvas: Canvas):
    """Linear: scattered selected points lined up on a cyan line."""
    plane = [(0, 0, 0), (2, 0, 0), (2, 2, 0), (0, 2, 0)]
    projection = fitted(plane, fit=26)
    canvas.face([projection(p) for p in plane], "top")
    canvas.edge([projection(p) for p in plane], width=EDGE, closed=True)
    backed(canvas, [projection((0.25, 1.75, 0)), projection((1.75, 0.25, 0))])
    for t, off in ((0.2, 0.35), (0.45, -0.4), (0.7, 0.3), (0.92, -0.25)):
        on = (0.25 + 1.5 * t, 1.75 - 1.5 * t, 0)
        canvas.vertex(projection((on[0] + off, on[1] + off, 0)), 0.75)
        selected_point(canvas, projection(on), 1.35)


# ------------------------------------------------------------- Edit: Slice

def loop_slice(canvas: Canvas):
    """Loop Slice: several evenly spaced cyan loops across the cube."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))))
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    for x in (0.25, 0.5, 0.75):
        canvas.edge([projection(p) for p in [(x, 0, 1), (x, 1, 1), (x, 1, 0)]], "new", ACCENT)


def edge_slice(canvas: Canvas):
    """Edge Slice: a cyan cut clicked from edge to edge across two faces,
    each click a point on an edge."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))), fit=23.5)
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    path = [(0.25, 0, 1), (0.7, 1, 1), (1, 1, 0.35), (1, 0.3, 0)]
    canvas.edge([projection(p) for p in path], "new", ACCENT)
    for p in path:
        new_point(canvas, projection(p), 1.25)


def polygon_slice(canvas: Canvas):
    """Polygon Slice: a straight drawn line cutting the cube clean through
    on a plane; the cut runs on as a backed line beyond the mesh."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))) + [(-0.25, 1.35, 1.25)], fit=25)
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    # The plane x + 0.6 z = 1.05 crosses the top and the right face's
    # neighbour (the front face); its trace on the visible faces:
    cut = [(0.45, 0, 1), (0.45, 1, 1), (1, 1, 0.08)]
    canvas.edge([projection(p) for p in cut], "new", ACCENT)
    a, b = projection((0.45, -0.45, 1)), projection((0.45, 1.45, 1))
    backed(canvas, [a, projection((0.45, -0.05, 1))])
    backed(canvas, [projection((0.45, 1.05, 1)), b])
    knot(canvas, a, 1.3, "new")
    knot(canvas, b, 1.3, "new")


# ------------------------------------------------------------- Polygon: Shift

def shift(canvas: Canvas):
    """Shift: the selected top and right faces moved out along their own
    normals on new points, leaving the box open where they were."""
    d = 0.42
    inner = [
        ([(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)], "left"),   # back wall
        ([(0, 0, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1)], "right"),  # left wall
        ([(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], "top"),    # floor
    ]
    front = [([(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)], "left")]
    moved = [
        ([(0, 0, 1 + d), (1, 0, 1 + d), (1, 1, 1 + d), (0, 1, 1 + d)], "selected"),
        ([(1 + d, 0, 1), (1 + d, 1, 1), (1 + d, 1, 0), (1 + d, 0, 0)], "selected"),
    ]
    projection = fitted(corners(inner + front + moved))
    for points, color in inner:
        canvas.face([projection(p) for p in points], color)
        canvas.edge([projection(p) for p in points], width=DETAIL, closed=True)
    canvas.mesh(projection, front)
    canvas.mesh(projection, moved)


DEFORM_ICONS = {
    "element-move": (element_move, "Element Move", "The cube's front corner pulled off the mesh; the grabbed point is cyan."),
    "element-push": (element_push, "Element Push", "One point of a sheet pushed in along its normal by a cyan arrow."),
    "flex": (flex, "Flex", "A bar flexed into an S-curve."),
    "soft-drag": (soft_drag, "Soft Drag", "A sheet dragged up by its middle along a free cyan stroke."),
    "soft-move": (soft_move, "Soft Move", "A sheet raised in the middle with a fading falloff, a blue Z arrow at its peak."),
    "shear": (shear, "Shear", "A block whose top slides along a red X arrow while its base stays."),
    "smooth": (smooth, "Smooth", "A sheet bumpy at the back and evened out at the front."),
    "jitter": (jitter, "Jitter", "A sheet's cyan points thrown off the grid at random heights."),
    "quantize": (quantize, "Quantize", "A slope snapped into even terraces."),
    "soft-selection-move": (soft_selection_move, "Soft Selection Move", "A selection fading out across a sheet, with move arrows."),
    "soft-selection-rotate": (soft_selection_rotate, "Soft Selection Rotate", "A selection fading out across a sheet, with a rotate ring."),
    "soft-selection-scale": (soft_selection_scale, "Soft Selection Scale", "A selection fading out across a sheet, with scale handles."),
    "soft-selection-transform": (soft_selection_transform, "Soft Selection Transform", "A selection fading out across a sheet, with move arrows inside a ring."),
    "twist": (twist, "Twist", "A square column wrung about its axis."),
    "bend": (bend, "Bend", "A straight bar bent up through a quarter turn."),
    "vortex": (vortex, "Vortex", "A disc with a cyan spiral turning outward."),
    "swirl": (swirl, "Swirl", "A sheet's grid turned most at its middle, with a cyan turning arrow."),
    "push": (push, "Push", "A ball with cyan arrows pushing out along its normals."),
    "sculpt": (sculpt, "Sculpt", "A cyan brush ring raising a sheet inside it."),
    "taper": (taper, "Taper", "A column narrowing toward its top."),
    "bulge": (bulge, "Bulge", "A column swollen around its middle."),
    "flare": (flare, "Flare", "A column opening out toward its top."),
    "radial-align": (radial_align, "Radial Align", "Scattered selected points placed on a cyan circle."),
    "linear-align": (linear_align, "Linear Align", "Scattered selected points placed on a cyan line."),
    "loop-slice": (loop_slice, "Loop Slice", "Three evenly spaced cyan loops across the cube."),
    "edge-slice": (edge_slice, "Edge Slice", "A cyan cut clicked from edge to edge across the cube."),
    "polygon-slice": (polygon_slice, "Polygon Slice", "A straight drawn line cutting the cube through on a plane."),
    "shift": (shift, "Shift", "Selected top and side faces moved out on their own, leaving the box open."),
}
