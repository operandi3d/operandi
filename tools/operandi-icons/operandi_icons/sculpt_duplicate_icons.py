"""Batch 12: the Sculpt tab's brushes and the Duplicate tab's tools, plus
Detriangulate (Polygon tab).

Same conventions: yellow-orange = selected, cyan = new geometry or the
tool's cue. Every sculpt brush shares one picture: a slab of clay seen
from the usual angle, its top showing the brush's effect (rough becoming
calm, a low ridge along a stroke, a round swell, a patch pulled up). The
effect is the subject; only Move adds an overlay, its drag arrow.
Every Duplicate tool shows a source cube and its copies, the copies with
cyan tops (new geometry, as in Copy), laid out the way the tool places them.
"""

from math import cos, exp, hypot, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, SELECT, SELECT_BACKING, Point, Projection, Vec3
from .create_icons import backed, backed_arrow, knot, sampled
from .deform_icons import _all, _mean, _normal, outline, shaded, sheet, sheet_border, tone
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted, hull

TAU = 2 * pi


# ------------------------------------------------------------- sculpt

def _clay_tone(normal: Vec3) -> str:
    """Like the cube's tones, but steeper-sensitive, so even a gentle
    slope facing the viewer's left or right picks up a side tone."""
    nx, ny, nz = normal
    if nz >= 1.7 * max(nx, ny):
        return "top"
    return "left" if ny >= nx else "right"


def _slab(canvas: Canvas, height, n: int = 16, lines: int = 7, extra: list[Vec3] = (),
          fit: float = 28, size: float = 2.0, depth: float = 0.45):
    """A slab of clay: a sculpted top over two plain sides. The top is
    filled in the cube's tones and drawn with a few ink profile lines
    running across it, like a contour plot, so every bump and dip shows in
    the lines. Drawn back to front, strip by strip, so near rises hide the
    lines behind them. The four brush icons share it; only the top's shape
    changes."""
    step = size / n
    at = lambda i, j: (i * step, j * step, height(i * step, j * step))
    fine = [size * k / (2 * n) for k in range(2 * n + 1)]
    front = [(x, size, height(x, size)) for x in fine] + [(size, size, -depth), (0, size, -depth)]
    side = [(size, y, height(size, y)) for y in fine] + [(size, size, -depth), (size, 0, -depth)]
    top = [at(i, j) for i in range(n + 1) for j in range(n + 1)]
    projection = fitted(top + front + side + list(extra), fit=fit)
    for points, color in ((front, "left"), (side, "right")):
        canvas.face([projection(p) for p in points], color)
        canvas.edge([projection(p) for p in points], width=EDGE, closed=True)
    profile = lambda x: [projection((x, y, height(x, y))) for y in fine]
    every = n // (lines - 1) if lines > 1 else n
    # Flat clay in one fill; only the raised facets are drawn one by one.
    canvas.face([projection((x, y, 0.0)) for x, y in ((0, 0), (size, 0), (size, size), (0, size))], "top")
    canvas.edge(profile(0.0), width=EDGE)
    canvas.edge([projection((x, 0.0, height(x, 0.0))) for x in fine], width=EDGE)
    for i in range(n):
        for j in range(n):
            quad = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]
            if max(abs(p[2]) for p in quad) < 1e-3:
                continue
            nrm = _normal(quad)
            if nrm[2] < 0:
                nrm = tuple(-v for v in nrm)
            color = _clay_tone(nrm)
            projected = [projection(p) for p in quad]
            canvas.face(projected, color)
            canvas.edge(projected, color, DETAIL, closed=True)
        x = (i + 1) * step
        if (i + 1) % every == 0 and i + 1 < n:
            canvas.edge(profile(x), "right", DETAIL)
    # The front edges, heavy like the cube's.
    canvas.edge(profile(size), width=EDGE)
    canvas.edge([projection((x, size, height(x, size))) for x in fine], width=EDGE)
    return projection


def _rough(x: float, y: float) -> float:
    """Jagged peaks of uneven height, strongest at the back."""
    peaks = ((0.3, 0.3, 0.8), (0.95, 0.25, 0.55), (0.3, 1.05, 0.65), (0.35, 1.75, 0.75),
             (0.9, 0.95, 0.45), (1.0, 1.65, 0.4))
    return sum(h * max(0.0, 1 - hypot(x - px, y - py) / 0.36) for px, py, h in peaks)


def sculpt_smooth(canvas: Canvas):
    """Smooth brush: the top jagged at the back left, evened out and calm
    toward the front right, so the eye reads rough becoming tidy."""
    def height(x, y):
        calm = min(1.0, max(0.0, (1.25 - (x + 0.15 * y)) / 0.5))
        return _rough(x, y) * calm
    _slab(canvas, height, n=15, lines=6)


def sculpt_push(canvas: Canvas):
    """Push brush: a long, low, soft ridge raised along the path a stroke
    took across the top."""
    path = [(0.1 + 1.8 * t, 1.0 + 0.45 * sin(t * TAU), 0.0) for t in (i / 40 for i in range(41))]
    def height(x, y):
        d = min(hypot(x - px, y - py) for px, py, _ in path)
        return 0.38 * max(0.0, 1 - (d / 0.34) ** 2)
    projection = _slab(canvas, height, n=15, lines=6)
    # The crest, where the stroke went.
    canvas.edge([projection((x, y, 0.38)) for x, y, _ in path], width=EDGE)


def _outline_raised(canvas: Canvas, projection: Projection, height, size: float = 2.0,
                    k: int = 60, floor: float = 0.02):
    """Ink the outline of a single round rise, so its silhouette reads on
    both themes."""
    pts = [(size * i / k, size * j / k) for i in range(k + 1) for j in range(k + 1)]
    raised = [projection((x, y, height(x, y))) for x, y in pts if height(x, y) > floor]
    canvas.edge(hull(raised), width=EDGE, closed=True)


def sculpt_inflate(canvas: Canvas):
    """Inflate brush: a round, puffy swell blown up from inside, taller
    and fuller than Push's ridge."""
    rad = 0.8
    def height(x, y):
        d2 = ((x - 1) ** 2 + (y - 1) ** 2) / rad ** 2
        return 1.0 * max(0.0, 1 - d2) ** 0.5 if d2 < 1 else 0.0
    projection = _slab(canvas, height, n=14, lines=3)
    _outline_raised(canvas, projection, height, floor=0.0)


def sculpt_move(canvas: Canvas):
    """Move brush: a mostly flat top with one patch grabbed and pulled up
    and over; a cyan drag arrow (the only overlay in the family) shows the
    pull."""
    def height(x, y):
        return 0.75 * exp(-((x - 1.25) ** 2 + (y - 0.85) ** 2) / 0.38 ** 2)
    projection = _slab(canvas, height, n=14, lines=3, extra=[(1.9, 0.3, 1.6), (0.6, 1.4, 0.0)])
    _outline_raised(canvas, projection, height, floor=0.08)
    start = projection((0.75, 1.35, 0.02))
    end = projection((1.55, 0.55, 1.45))
    curve = sampled([start, (start[0] - 0.5, start[1] - 5.0), (end[0] - 5.0, end[1] - 0.5), end], 16)
    backed(canvas, curve[:-2])
    backed_arrow(canvas, curve[-4], end, length=2.6, half_width=1.6)
    knot(canvas, start, 1.5, "new")


# ------------------------------------------------------------- duplicate

def _cubes(canvas: Canvas, origins: list[Vec3], size: float, fit: float = 26,
           extra: list[Vec3] = ()):
    """The first cube is the source; the rest are copies with cyan tops.
    Cubes are drawn far to near so nearer ones overlap."""
    boxes = []
    for i, (x, y, z) in enumerate(origins):
        faces = box((x, y, z), (x + size, y + size, z + size), top="top" if i == 0 else "new")
        boxes.append((x + y + z, faces))
    projection = fitted([p for _, f in boxes for p in corners(f)] + list(extra), fit=fit)
    for _, faces in sorted(boxes, key=lambda b: b[0]):
        canvas.mesh(projection, faces)
    return projection


def clone(canvas: Canvas):
    """Clone: copies stepping away from the source in a straight line, a
    cyan arrow showing the step."""
    step = 1.05
    origins = [(i * step, 0.0, 0.0) for i in range(3)]
    projection = _cubes(canvas, origins, 0.62, extra=[(0.3, -0.5, 1.1), (2.7, -0.5, 1.1)])
    backed_arrow(canvas, projection((0.3, -0.35, 1.05)), projection((2.65, -0.35, 1.05)),
                 length=2.4, half_width=1.5)


def array(canvas: Canvas):
    """Array: copies set out in a grid of rows and columns."""
    step = 0.85
    origins = [(i * step, j * step, 0.0) for j in range(3) for i in range(3)]
    _cubes(canvas, origins, 0.55)


def radial_array(canvas: Canvas):
    """Radial Array: copies spaced evenly round a cyan circle."""
    r, size, count = 1.15, 0.5, 6
    origins = []
    for k in range(count):
        a = TAU * k / count + pi / 4
        origins.append((r * cos(a) - size / 2, r * sin(a) - size / 2, 0.0))
    # The source is the cube nearest the viewer, drawn plain.
    origins.sort(key=lambda o: -(o[0] + o[1]))
    circle = [(1.0 * r * cos(a), 1.0 * r * sin(a), 0.0) for a in (TAU * i / 48 for i in range(48))]
    projection = fitted([p for o in origins for p in corners(box(o, (o[0] + size, o[1] + size, size)))],
                        fit=26)
    backed(canvas, [projection(p) for p in circle], closed=True)
    boxes = []
    for i, (x, y, z) in enumerate(origins):
        faces = box((x, y, z), (x + size, y + size, z + size),
                    top="new" if i else "top")
        boxes.append((x + y, faces))
    for _, faces in sorted(boxes, key=lambda b: b[0]):
        canvas.mesh(projection, faces)
    knot(canvas, projection((0, 0, 0)), 1.3, "new")


def _step(x_near: float, x_far: float, top: str):
    """An L-shaped step: low next to the plane (x_near), tall at x_far.
    Returns its two boxes as face lists, far one first in painter order."""
    mid = (x_near + x_far) / 2
    lo_n, hi_n = sorted((x_near, mid))
    lo_f, hi_f = sorted((mid, x_far))
    low = box((lo_n, 0, 0), (hi_n, 0.9, 0.45), top=top)
    tall = box((lo_f, 0, 0), (hi_f, 0.9, 1.0), top=top)
    return (tall, low) if x_far < x_near else (low, tall)


def mirror(canvas: Canvas):
    """Mirror: a step and its mirror image either side of a cyan plane."""
    gap = 0.15
    source = _step(-gap, -1.1 - gap, "top")
    copy = _step(gap, 1.1 + gap, "new")
    plane = [(0, -0.3, -0.15), (0, 1.2, -0.15), (0, 1.2, 1.3), (0, -0.3, 1.3)]
    projection = fitted([p for b in source + copy for p in corners(b)] + plane, fit=27)
    for faces in source:
        canvas.mesh(projection, faces)
    backed(canvas, [projection(p) for p in plane], closed=True)
    for faces in copy:
        canvas.mesh(projection, faces)


def _round_sections(profile: list[tuple[float, float]], sides: int = 20) -> list[list[Vec3]]:
    return [[(r * cos(TAU * k / sides), r * sin(TAU * k / sides), z) for k in range(sides)]
            for r, z in profile]


def _facet_normal(points):
    n = _normal(points)
    c = _mean(points)
    if n[0] * c[0] + n[1] * c[1] < 0:
        n = tuple(-v for v in n)
    return n


def radial_sweep(canvas: Canvas):
    """Radial Sweep: a cyan profile line swept round an axis into a vase,
    a cyan arrow showing the turn."""
    profile = [(0.55, 0.0), (0.78, 0.35), (0.74, 0.75), (0.45, 1.1), (0.36, 1.4), (0.5, 1.7)]
    sections = _round_sections(profile)
    facets = []
    for a, b in zip(sections, sections[1:]):
        k = len(a)
        for j in range(k):
            facets.append(([a[j], a[(j + 1) % k], b[(j + 1) % k], b[j]], None))
    top = sections[-1]
    arc = [(1.12 * cos(a), 1.12 * sin(a), 0.55) for a in (TAU * (-0.1 + 0.42 * i / 30) for i in range(31))]
    projection = fitted(_all(facets) + arc + [(0, 0, 2.05), (0, 0, -0.3)], fit=26)
    # Filled without strip edges, like the round primitives, then one
    # heavier silhouette.
    for points, color in facets:
        n = _facet_normal(points)
        if n[0] + n[1] + n[2] > 1e-9:
            canvas.face([projection(p) for p in points], tone(n))
    # The open mouth, dark inside, and a heavier silhouette like the cube's.
    canvas.face([projection(p) for p in top], "right")
    canvas.edge([projection(p) for p in top], width=EDGE, closed=True)
    canvas.edge(hull([projection(p) for p in _all(facets)]), width=EDGE, closed=True)
    # The profile being swept, on the vase's front-left (+Y) side.
    backed(canvas, [projection((0.0, r + 0.02, z)) for r, z in profile])
    pts = [projection(p) for p in arc]
    backed(canvas, pts[:-1])
    backed_arrow(canvas, pts[-3], pts[-1], length=2.2, half_width=1.4)


def detriangulate(canvas: Canvas):
    """Detriangulate, Triple's opposite: the cube's faces were split into
    triangle pairs; the diagonals are fading out (thin, in a face tone) and
    each face is whole again, its quad outlined in cyan."""
    projection = Projection()
    faces = box((0, 0, 0), (1, 1, 1))
    canvas.mesh(projection, faces)
    for (a, b), color in ((((0, 0, 1), (1, 1, 1)), "left"), (((0, 1, 1), (1, 1, 0)), "right"),
                          (((1, 0, 1), (1, 1, 0)), "left")):
        canvas.edge([projection(a), projection(b)], color, DETAIL)
    for points, _ in faces:
        canvas.edge([projection(p) for p in points], "new", ACCENT, closed=True)


SCULPT_ICONS = {
    "sculpt-smooth": (sculpt_smooth, "Smooth Brush", "A clay slab whose top is jagged at the back and calm at the front."),
    "sculpt-push": (sculpt_push, "Push Brush", "A clay slab with a long, low ridge raised along a stroke."),
    "sculpt-inflate": (sculpt_inflate, "Inflate Brush", "A clay slab with a round, puffy swell."),
    "sculpt-move": (sculpt_move, "Move Brush", "A clay slab with a patch pulled up by a cyan drag arrow."),
}

DUPLICATE_ICONS = {
    "radial-sweep": (radial_sweep, "Radial Sweep", "A cyan profile swept round into a vase, a cyan arrow showing the turn."),
    "clone": (clone, "Clone", "A cube and two copies with cyan tops in a line, a cyan arrow showing the step."),
    "array": (array, "Array", "A cube and its copies with cyan tops laid out in a three by three grid."),
    "radial-array": (radial_array, "Radial Array", "A cube and its copies with cyan tops spaced round a cyan circle."),
    "mirror": (mirror, "Mirror", "A step block and its cyan-topped mirror image either side of a cyan plane."),
    "detriangulate": (detriangulate, "Detriangulate", "A cube whose triangle pairs join back into quads."),
}
