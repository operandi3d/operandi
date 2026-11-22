"""Batch 11: the Edit tab's path and axis Slice tools (Pen, Curve and
Primitive Slice; Polygon Axis Slice, Dicer, Julienne).

Same conventions as the batch 8 Slice icons: cyan = the cut (new edges),
ink outlines, the cube's three tones. Each cut lies on the block's top face,
so one switch shows the set either tilted (the usual three-quarter view, the
toolbox majority) or flat (the top face seen face-on, as the Subdivide icons
try on branch subdivide-tools):

- Pen Slice: a straight-sided path with a cyan point at each click.
- Curve Slice: one smooth cyan curve through a few clicked points.
- Primitive Slice: a cyan circle placed on the face, its centre marked.
- Polygon Axis Slice: evenly spaced straight cuts along X and Y, a grid.
- Dicer / Julienne: the cube cut into closed pieces and pulled apart, in a
  grid of blocks (Dicer) or in slabs (Julienne); cut faces outlined cyan.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, Point, Vec3
from .create_icons import knot
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted
from .vertex_icons import new_point

TAU = 2 * pi

# "tilted" or "flat". Jason picks one for the whole Edit tab; switching here
# and regenerating changes all six icons (no other code reads it).
VIEW = "tilted"


def _view(points: list[Vec3], view: str | None = None):
    """A model -> canvas mapping for these points, centred on the canvas.
    Flat looks straight down Z: model x to the right, y down."""
    if (view or VIEW) == "flat":
        return lambda p: (4 + 24 * p[0], 4 + 24 * p[1])
    return fitted(points)


def _slab(canvas: Canvas, project, lo: Vec3, hi: Vec3, edge: str = "ink"):
    """One box: its three visible faces tilted, only its top flat."""
    faces = box(lo, hi)
    if (VIEW) == "flat":
        faces = faces[:1]
    for points, color in faces:
        flat = [project(p) for p in points]
        canvas.face(flat, color)
        canvas.edge(flat, edge, ACCENT if edge == "new" else EDGE, closed=True)


def _cube(canvas: Canvas, height: float = 1.0):
    project = _view(corners(box((0, 0, 0), (1, 1, height))))
    _slab(canvas, project, (0, 0, 0), (1, 1, height))
    return project


# The path slices draw on a low block, so its top (where the cut lies)
# fills the icon; tilted, the sides still say it is a solid.
LOW = 0.38


def _top(points_xy: list[tuple[float, float]]) -> list[Vec3]:
    return [(x, y, LOW) for x, y in points_xy]


# ------------------------------------------------------------- path slices

def pen_slice(canvas: Canvas):
    """Pen Slice: a straight-sided path clicked across the top face; each
    click is a cyan point."""
    project = _cube(canvas, LOW)
    path = [project(p) for p in _top([(0.14, 0.18), (0.66, 0.24), (0.34, 0.76), (0.86, 0.82)])]
    canvas.edge(path, "new", ACCENT)
    for p in path:
        new_point(canvas, p, 1.4)


def _bezier(a, b, c, d, samples: int = 24) -> list[tuple[float, float]]:
    out = []
    for i in range(samples + 1):
        t = i / samples
        u = 1 - t
        out.append(tuple(u ** 3 * a[k] + 3 * u * u * t * b[k] + 3 * u * t * t * c[k] + t ** 3 * d[k]
                         for k in range(2)))
    return out


def curve_slice(canvas: Canvas):
    """Curve Slice: one smooth cyan S-curve across the top face; the points
    it was drawn through stand on it as light knots."""
    project = _cube(canvas, LOW)
    xy = _bezier((0.12, 0.2), (0.95, 0.1), (0.05, 0.9), (0.88, 0.8))
    curve = [project(p) for p in _top(xy)]
    canvas.edge(curve, "new", ACCENT)
    for i in (0, 12, 24):
        knot(canvas, curve[i], 1.6, "new")


def primitive_slice(canvas: Canvas):
    """Primitive Slice: a cyan circle placed on the top face, with its
    centre and the drag that sized it."""
    project = _cube(canvas, LOW)
    r = 0.36
    ring = [project((0.5 + r * cos(a), 0.5 + r * sin(a), LOW))
            for a in (TAU * i / 48 for i in range(48))]
    canvas.edge(ring, "new", ACCENT, closed=True)
    centre = project((0.5, 0.5, LOW))
    rim = project((0.5 + r * cos(pi / 4), 0.5 + r * sin(pi / 4), LOW))
    canvas.edge([centre, rim], "new", DETAIL)
    new_point(canvas, centre, 1.3)


# ------------------------------------------------------------- axis slices

def axis_slice(canvas: Canvas):
    """Polygon Axis Slice: evenly spaced straight cuts along X and Y over
    the top face, carried down the sides."""
    project = _cube(canvas)
    for t in (1 / 3, 2 / 3):
        # Along X: a cut at y = t over the top and down the +X side.
        canvas.edge([project(p) for p in [(0, t, 1), (1, t, 1), (1, t, 0)]], "new", ACCENT)
        # Along Y: a cut at x = t over the top and down the +Y side.
        canvas.edge([project(p) for p in [(t, 0, 1), (t, 1, 1), (t, 1, 0)]], "new", ACCENT)


def _pieces(canvas: Canvas, boxes: list[tuple[Vec3, Vec3]]):
    """Closed pieces pulled apart, drawn back to front (the viewer looks
    down -(1, 1, 1), so a larger x + y is nearer); cut faces in cyan."""
    project = _view([p for lo, hi in boxes for p in corners(box(lo, hi))])
    for lo, hi in sorted(boxes, key=lambda b: b[0][0] + b[0][1]):
        _slab(canvas, project, lo, hi, "new")
    return project


def dicer(canvas: Canvas):
    """Dicer: the cube cut into a 2 x 2 grid of closed blocks, pulled
    apart."""
    gap, s = 0.22, 0.5
    _pieces(canvas, [((i * (s + gap), j * (s + gap), 0), (i * (s + gap) + s, j * (s + gap) + s, 1))
                     for i in (0, 1) for j in (0, 1)])


def julienne(canvas: Canvas):
    """Julienne: the cube cut into three closed slabs along X, pulled
    apart."""
    gap, s = 0.22, 1 / 3
    _pieces(canvas, [((i * (s + gap), 0, 0), (i * (s + gap) + s, 1, 1)) for i in range(3)])


PATH_SLICE_ICONS = {
    "pen-slice": (pen_slice, "Pen Slice", "A straight-sided cyan path clicked across the block's top, a point at each click."),
    "curve-slice": (curve_slice, "Curve Slice", "One smooth cyan curve across the block's top through a few knots."),
    "primitive-slice": (primitive_slice, "Primitive Slice", "A cyan circle placed on the block's top, its centre marked."),
    "axis-slice": (axis_slice, "Polygon Axis Slice", "Evenly spaced cyan cuts along X and Y across the cube."),
    "dicer": (dicer, "Dicer", "The cube cut into four closed blocks, pulled apart."),
    "julienne": (julienne, "Julienne", "The cube cut into three closed slabs, pulled apart."),
}
