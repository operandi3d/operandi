"""Batch 7: the Create panel v2 tools: Text, the Draw tools (Curve, Bezier
Curve, B-Spline, Sketch, Draw Patch) and the Edit, Merge, Align and Measure
groups. Original drawings in the primitives' style.

Dark-theme rule for this batch: no bare ink line ever sits on the background,
because ink vanishes on the dark theme (Arc, 2026-11-05). Lines that stand
alone are cyan (or yellow, or an axis colour) on a wider ink backing, the way
selected edges are drawn: the colour carries on dark, the backing on light.
Points that stand alone are light dots in an ink ring."""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, SELECT, SELECT_BACKING, Point, Projection, Vec3
from .primitives import Canvas
from .polygon_icons import box, corners, lerp
from .shape_icons import _bezier, _keyhole, _ccw, _ellipse
from .solids import fitted

TAU = 2 * pi


# ------------------------------------------------------------- helpers

def backed(canvas: Canvas, points: list[Point], color: str = "new", closed: bool = False):
    """A coloured line on an ink backing: reads on light and dark."""
    canvas.edge(points, "ink", SELECT_BACKING, closed)
    canvas.edge(points, color, SELECT, closed)


def knot(canvas: Canvas, point: Point, radius: float = 2.0, color: str = "top"):
    """A point that stands alone: a light dot in an ink ring."""
    canvas.vertex(point, radius, "ink")
    canvas.vertex(point, radius - 0.7, color)


def backed_arrow(canvas: Canvas, start: Point, end: Point, color: str = "new",
                 length: float = 2.6, half_width: float = 1.6):
    from math import hypot
    dx, dy = end[0] - start[0], end[1] - start[1]
    n = hypot(dx, dy)
    ux, uy = dx / n, dy / n
    base = (end[0] - ux * length, end[1] - uy * length)
    head = [(base[0] - uy * half_width, base[1] + ux * half_width), end,
            (base[0] + uy * half_width, base[1] - ux * half_width)]
    for width, ink in ((SELECT_BACKING, "ink"), (SELECT, color)):
        canvas.edge([start, end], ink, width)
        canvas.edge(head, ink, width)


def sampled(controls: list[Point], steps: int = 48) -> list[Point]:
    return [_bezier(controls, i / steps) for i in range(steps + 1)]


def bspline(controls: list[Point], steps: int = 12) -> list[Point]:
    """Uniform cubic B-spline, clamped by repeating the end points: a curve
    that is pulled toward, but (in the middle) does not touch, its cage."""
    pts = [controls[0]] * 2 + controls + [controls[-1]] * 2
    out = []
    for i in range(len(pts) - 3):
        p0, p1, p2, p3 = pts[i:i + 4]
        for s in range(steps + (1 if i == len(pts) - 4 else 0)):
            t = s / steps
            b = ((1 - t) ** 3 / 6, (3 * t ** 3 - 6 * t * t + 4) / 6,
                 (-3 * t ** 3 + 3 * t * t + 3 * t + 1) / 6, t ** 3 / 6)
            out.append(tuple(b[0] * p0[k] + b[1] * p1[k] + b[2] * p2[k] + b[3] * p3[k]
                             for k in (0, 1)))
    return out


# ------------------------------------------------------------- Draw row

def _wave(t: float) -> Point:
    return (3.0 + 26.0 * t, 16.0 - 8.0 * sin(TAU * 0.85 * t + 0.35))


def curve(canvas: Canvas):
    """A smooth cyan wave through four light points."""
    backed(canvas, [_wave(i / 64) for i in range(65)])
    for t in (0.0, 0.36, 0.66, 1.0):
        knot(canvas, _wave(t))


def bezier_curve(canvas: Canvas):
    """A cyan curve with one point's tangent handles pulled out: the handle
    bar is yellow (it is what you drag), with square-ish ink ends."""
    a, m, b = (4.0, 25.0), (16.0, 15.0), (28.0, 6.0)
    h1, h2 = (8.0, 9.0), (24.0, 21.0)
    path = sampled([a, (5.0, 16.0), h1, m], 24)[:-1] + sampled([m, h2, (27.0, 14.0), b], 24)
    backed(canvas, path)
    backed(canvas, [h1, h2], "selected")
    for h in (h1, h2):
        canvas.vertex(h, 1.45, "ink")
        canvas.vertex(h, 0.75, "selected")
    for p in (a, m, b):
        knot(canvas, p)


def b_spline(canvas: Canvas):
    """A smooth cyan arch inside its control cage: a thin light line joining
    light points that the curve is pulled toward but does not pass through."""
    cage = [(3.0, 27.0), (7.0, 5.0), (25.0, 5.0), (29.0, 27.0)]
    canvas.edge(cage, "ink", SELECT_BACKING)
    canvas.edge(cage, "left", DETAIL)
    backed(canvas, bspline(cage, 16))
    for p in cage:
        knot(canvas, p)


def sketch(canvas: Canvas):
    """A freehand cyan scribble: a wobbly line that loops twice."""
    points = []
    for i in range(161):
        t = i / 160
        # A line drifting left to right with two loops along it (a prolate
        # cycloid), plus a little hand tremor.
        angle = TAU * 2 * t
        x = 4 + 20 * t - 5.2 * sin(angle)
        y = 17 - 6.5 * cos(angle) * (0.75 + 0.25 * t) - 6 * t + 0.5 * sin(37 * t)
        points.append((x, y))
    backed(canvas, points)


def draw_patch(canvas: Canvas):
    """A curved cyan quad patch spanned between four inked boundary curves,
    with its inner grid in thin ink."""
    def surface(u, v):
        x, y = u * 2.0, v * 2.0
        z = 0.55 * sin(pi * u) * (0.6 + 0.4 * v) - 0.25 * v
        return (x, y, z)
    n = 4
    samples = [surface(i / 16, j / 16) for i in range(17) for j in range(17)]
    projection = fitted(samples)
    grid = lambda u, v: projection(surface(u, v))
    outline = ([grid(i / 16, 0) for i in range(17)] + [grid(1, j / 16) for j in range(1, 17)]
               + [grid(i / 16, 1) for i in range(15, -1, -1)]
               + [grid(0, j / 16) for j in range(15, 0, -1)])
    canvas.face(outline, "new")
    # A lighter band on the upper half where the surface bulges toward the light.
    band = ([grid(i / 16, 0.0) for i in range(4, 13)]
            + [grid(i / 16, 0.45) for i in range(12, 3, -1)])
    canvas.face(band, "top")
    for k in range(1, n):
        canvas.edge([grid(k / n, j / 16) for j in range(17)], width=DETAIL)
        canvas.edge([grid(i / 16, k / n) for i in range(17)], width=DETAIL)
    canvas.edge(outline, closed=True)
    for u, v in ((0, 0), (1, 0), (1, 1), (0, 1)):
        knot(canvas, grid(u, v), 1.4)


# ------------------------------------------------------------- Text

def text(canvas: Canvas):
    """An extruded capital T standing on the ground, in the cube's tones."""
    stem = box((0.3, 0.0, 0.0), (0.7, 0.4, 0.85))
    bar = box((-0.15, 0.0, 0.85), (1.15, 0.4, 1.3))
    projection = fitted(corners(stem + bar))
    # The bar's underside hides nothing we see, so stem then bar is painter order.
    canvas.mesh(projection, stem)
    canvas.mesh(projection, bar)


# ------------------------------------------------------------- Edit group

def _ring(center: Point, outer: float, inner: float, squash: float, angle: float):
    return _keyhole(_ccw(_ellipse(center, outer, outer * squash, angle, 40)),
                    _ccw(_ellipse(center, inner, inner * squash, angle, 40)))


def cut(canvas: Canvas):
    """Open scissors: two light blades crossing at a pivot, two finger rings."""
    from math import hypot
    pivot = (16.0, 14.5)
    for side in (1, -1):
        handle, tip = (16.0 - side * 5.0, 21.5), (16.0 + side * 11.0, 2.5)
        dx, dy = tip[0] - handle[0], tip[1] - handle[1]
        n = hypot(dx, dy)
        px, py = -dy / n, dx / n
        blade = [(handle[0] + px * 1.5, handle[1] + py * 1.5),
                 (pivot[0] + px * 2.1, pivot[1] + py * 2.1),
                 tip,
                 (pivot[0] - px * 1.2, pivot[1] - py * 1.2),
                 (handle[0] - px * 1.5, handle[1] - py * 1.5)]
        canvas.face(blade, "top" if side > 0 else "left")
        canvas.edge(blade, closed=True)
    for side in (-1, 1):
        center = (16.0 + side * 6.3, 25.4)
        tilt = -side * 0.5
        canvas.face(_ring(center, 5.0, 2.5, 0.8, tilt), "right")
        canvas.edge(_ellipse(center, 5.0, 4.0, tilt, 40), closed=True)
        canvas.edge(_ellipse(center, 2.5, 2.0, tilt, 40), closed=True)
    canvas.vertex(pivot, 1.3, "ink")
    canvas.vertex(pivot, 0.6, "selected")


def copy(canvas: Canvas):
    """A cube and its copy: the copy steps forward with a cyan top."""
    back = box((0.0, 0.0, 0.0), (1, 1, 1))
    front = box((0.75, -0.75, -0.55), (1.75, 0.25, 0.45), top="new")
    projection = fitted(corners(back + front))
    canvas.mesh(projection, back)
    canvas.mesh(projection, front)


def _clipboard(canvas: Canvas) -> None:
    board = [(5.0, 5.0), (27.0, 5.0), (27.0, 30.0), (5.0, 30.0)]
    canvas.face(board, "right")
    canvas.edge(board, closed=True)
    sheet = [(8.0, 8.5), (24.0, 8.5), (24.0, 27.0), (8.0, 27.0)]
    canvas.face(sheet, "top")
    canvas.edge(sheet, closed=True, width=DETAIL)
    clip = [(11.0, 2.0), (21.0, 2.0), (22.0, 7.5), (10.0, 7.5)]
    canvas.face(clip, "left")
    canvas.edge(clip, closed=True)


def paste(canvas: Canvas):
    """A clipboard with a small cube landing on its sheet."""
    _clipboard(canvas)
    projection = Projection(unit=7.2, origin=(16.0, 18.5))
    faces = box((-0.5, -0.5, -0.5), (0.5, 0.5, 0.5), top="new")
    canvas.mesh(projection, faces)


def paste_tool(canvas: Canvas):
    """Copies stamped where you click: two cubes on the ground and a third
    arriving over a yellow click target."""
    ground = [(-0.4, -0.4, 0), (2.9, -0.4, 0), (2.9, 2.9, 0), (-0.4, 2.9, 0)]
    a = box((1.7, -0.1, 0), (2.5, 0.7, 0.8))
    b = box((-0.1, 1.7, 0), (0.7, 2.5, 0.8))
    c = box((0.85, 0.85, 1.25), (1.65, 1.65, 2.05), top="new")
    projection = fitted(ground + corners(a + b + c))
    canvas.face([projection(p) for p in ground], "top")
    canvas.edge([projection(p) for p in ground], closed=True, width=DETAIL)
    target = projection((1.25, 1.25, 0))
    canvas.edge(_ellipse(target, 3.2, 1.6, 0, 32), "selected", ACCENT, closed=True)
    canvas.vertex(target, 0.8, "selected")
    canvas.mesh(projection, a)
    canvas.mesh(projection, b)
    canvas.mesh(projection, c)


# ------------------------------------------------------------- Merge group

def _pair(gap: float):
    left = box((0.0, 0.0, 0.0), (1.0, 1.0, 1.0))
    right = box((1.0 + gap, 0.0, 0.0), (2.0 + gap, 1.0, 1.0))
    return left, right


def merge(canvas: Canvas):
    """Two blocks pushed together into one: arrows meet at a cyan seam."""
    left, right = _pair(0.0)
    joined = box((0.0, 0.0, 0.0), (2.0, 1.0, 1.0))
    head = [(-0.5, 0.5, 2.0), (2.5, 0.5, 2.0)]
    projection = fitted(corners(joined) + head)
    canvas.mesh(projection, joined)
    seam = [projection(p) for p in ((1, 0, 1), (1, 1, 1), (1, 1, 0))]
    canvas.edge(seam, "new", ACCENT)
    mid = projection((1.0, 0.5, 2.0))
    for end in head:
        start = projection(end)
        stop = (mid[0] + (start[0] - mid[0]) * 0.22, mid[1] + (start[1] - mid[1]) * 0.22)
        backed_arrow(canvas, start, stop)


def unmerge(canvas: Canvas):
    """One block split into two pieces that move apart."""
    left, right = _pair(0.7)
    head = [(0.0, 0.5, 2.0), (2.7, 0.5, 2.0)]
    projection = fitted(corners(left + right) + head)
    canvas.mesh(projection, left)
    canvas.mesh(projection, right)
    mid = projection((1.35, 0.5, 2.0))
    for end in head:
        stop = projection(end)
        start = (mid[0] + (stop[0] - mid[0]) * 0.22, mid[1] + (stop[1] - mid[1]) * 0.22)
        backed_arrow(canvas, start, stop)


# ------------------------------------------------------------- Align group

def quick_align(canvas: Canvas):
    """A selected block hops onto a yellow target corner of a bigger one."""
    big = box((0.0, 0.0, 0.0), (1.6, 1.6, 0.9))
    small = box((-0.3, 2.3, 0.0), (0.4, 3.0, 0.7), top="selected")
    target = (1.6, 1.6, 0.9)
    projection = fitted(corners(big + small) + [(1.0, 2.0, 2.1)])
    canvas.mesh(projection, big)
    canvas.mesh(projection, small)
    start = projection((0.05, 2.65, 0.85))
    end = projection(target)
    top = projection((0.9, 2.2, 2.0))
    path = [start, (start[0] + 1, top[1]), (end[0] - 4, top[1] - 1), end]
    backed(canvas, sampled(path, 20)[:-3])
    tail = sampled(path, 20)[-4]
    backed_arrow(canvas, tail, end, length=2.4, half_width=1.5)
    knot(canvas, end, 1.5, "selected")


def ground_align(canvas: Canvas):
    """A block above the ground drops straight down onto it."""
    ground = [(-0.6, -0.6, 0), (1.6, -0.6, 0), (1.6, 1.6, 0), (-0.6, 1.6, 0)]
    block = box((0.0, 0.0, 1.05), (1.0, 1.0, 1.85), top="selected")
    projection = fitted(ground + corners(block))
    canvas.face([projection(p) for p in ground], "top")
    canvas.edge([projection(p) for p in ground], closed=True, width=DETAIL)
    # Where it will land, in cyan.
    footprint = [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)]
    canvas.face([projection(p) for p in footprint], "new")
    canvas.edge([projection(p) for p in footprint], closed=True, width=DETAIL)
    canvas.mesh(projection, block)
    for x, y in ((1.0, 0.0), (0.0, 1.0)):
        backed_arrow(canvas, projection((x, y, 0.95)), projection((x, y, 0.12)),
                     length=1.8, half_width=1.2)


# ------------------------------------------------------------- Measure group

def ruler(canvas: Canvas):
    """A long ruler bar with ink ticks on its top face."""
    length, width, depth = 3.0, 0.95, 0.2
    faces = box((0.0, 0.0, 0.0), (length, width, depth))
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)
    for i in range(1, 12):
        x = length * i / 12
        reach = 0.55 if i % 4 == 0 else 0.3
        canvas.edge([projection((x, width, depth)), projection((x, width - reach, depth))],
                    width=DETAIL if i % 4 else EDGE)


def protractor(canvas: Canvas):
    """A half-disc protractor with ticks and a cyan angle swept between two arms."""
    center, outer, inner = (16.0, 24.0), 14.5, 6.0
    arc = [(center[0] + outer * cos(pi + pi * i / 48), center[1] + outer * sin(pi + pi * i / 48))
           for i in range(49)]
    hole = [(center[0] + inner * cos(pi * i / 24), center[1] - inner * sin(pi * i / 24))
            for i in range(25)]
    body = arc + hole
    canvas.face(body, "top")
    canvas.edge(body, closed=True)
    for i in range(1, 12):
        a = pi + pi * i / 12
        r0 = outer - (2.6 if i % 3 == 0 else 1.5)
        canvas.edge([(center[0] + r0 * cos(a), center[1] + r0 * sin(a)),
                     (center[0] + (outer - 0.3) * cos(a), center[1] + (outer - 0.3) * sin(a))],
                    width=DETAIL)
    # The measured angle: a cyan wedge between the baseline and a raised arm.
    sweep = 0.30 * pi
    wedge = [center] + [(center[0] + 9.5 * cos(-a), center[1] + 9.5 * sin(-a))
                        for a in (sweep * i / 16 for i in range(17))]
    canvas.face(wedge, "new")
    arm = (center[0] + 15.5 * cos(-sweep), center[1] + 15.5 * sin(-sweep))
    backed(canvas, [center, arm], "selected")
    canvas.vertex(center, 1.2, "ink")


def dimensions(canvas: Canvas):
    """A block with red, green and blue size lines along its three sides."""
    faces = box((0.0, 0.0, 0.0), (1.0, 1.0, 1.0))
    off, d = 0.4, 0.32
    lines = {
        "x": [(0.0, 1.0 + off, 0.0), (1.0, 1.0 + off, 0.0)],
        "y": [(1.0 + off, 0.0, 0.0), (1.0 + off, 1.0, 0.0)],
        "z": [(-d, 1.0 + d, 0.0), (-d, 1.0 + d, 1.0)],
    }
    projection = fitted(corners(faces) + [p for pts in lines.values() for p in pts])
    canvas.mesh(projection, faces)
    for axis, (a, b) in lines.items():
        pa, pb = projection(a), projection(b)
        backed(canvas, [pa, pb], axis)
        # End ticks across the line.
        dx, dy = pb[0] - pa[0], pb[1] - pa[1]
        n = (dx * dx + dy * dy) ** 0.5
        nx, ny = -dy / n * 1.5, dx / n * 1.5
        for p in (pa, pb):
            backed(canvas, [(p[0] - nx, p[1] - ny), (p[0] + nx, p[1] + ny)], axis)


CREATE_ICONS = {
    "text": (text, "Text", "An extruded capital T in the cube's tones."),
    "curve": (curve, "Curve", "A smooth cyan curve through four light points."),
    "bezier-curve": (bezier_curve, "Bezier Curve", "A cyan curve with a yellow tangent handle on its middle point."),
    "b-spline": (b_spline, "B-Spline", "A cyan curve pulled toward a cage of light control points."),
    "sketch": (sketch, "Sketch", "A freehand cyan scribble."),
    "draw-patch": (draw_patch, "Draw Patch", "A curved cyan quad patch inside four boundary curves."),
    "cut": (cut, "Cut", "Open scissors."),
    "copy": (copy, "Copy", "A cube and a copy of it with a cyan top."),
    "paste": (paste, "Paste", "A clipboard with a cube on its sheet."),
    "paste-tool": (paste_tool, "Paste Tool", "Cubes stamped onto the ground, one arriving over a yellow target."),
    "merge": (merge, "Merge", "Two blocks joined at a cyan seam, arrows pointing inward."),
    "unmerge": (unmerge, "Unmerge", "A block split in two, arrows pointing apart."),
    "quick-align": (quick_align, "Quick Align", "A selected block hopping onto a target corner."),
    "ground-align": (ground_align, "Ground Align", "A selected block dropping onto its cyan footprint on the ground."),
    "ruler": (ruler, "Ruler", "A ruler bar with ticks."),
    "protractor": (protractor, "Protractor", "A half-disc protractor with a cyan angle and yellow arm."),
    "dimensions": (dimensions, "Dimensions", "A cube with red, green and blue size lines."),
}
