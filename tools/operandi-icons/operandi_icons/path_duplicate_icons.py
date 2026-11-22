"""Batch 20: the Duplicate tab's path tools (N23, Jason 2026-11-17).

Each Extrude icon is Sketch Extrude's picture (a selected square profile
and the cyan path it sweeps along) and each Clone icon is Clone's (a source
cube and copies with cyan tops) laid along a path. The path's kind is the
one cue that tells the variants apart:

- Curve: a smooth curve with knots on it.
- Bezier: a smooth curve with a tangent handle at each end.
- B-Spline: a smooth curve inside its control polygon, the knots off the
  curve.
- Pen: straight segments between knots.

Scatter Clone drops copies at loose spots on a flat patch.
"""

from .conventions import DETAIL, Point, Vec3
from .create_icons import backed, knot
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted


def _bezier(points: list[Point], steps: int = 24) -> list[Point]:
    (ax, ay), (bx, by), (cx, cy), (dx, dy) = points
    out = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        out.append((u * u * u * ax + 3 * u * u * t * bx + 3 * u * t * t * cx + t * t * t * dx,
                    u * u * u * ay + 3 * u * u * t * by + 3 * u * t * t * cy + t * t * t * dy))
    return out


def _path(canvas: Canvas, kind: str, control: list[Point]):
    """Draw a path of `kind` from four screen-space control points."""
    if kind == "pen":
        backed(canvas, control)
        for p in control:
            knot(canvas, p, 1.7)
        return
    curve = _bezier(control)
    if kind == "bspline":
        canvas.edge(control, "left", DETAIL)
        backed(canvas, curve)
        for p in control:
            knot(canvas, p, 1.5)
    elif kind == "bezier":
        backed(canvas, curve)
        canvas.edge([control[0], control[1]], "left", DETAIL)
        canvas.edge([control[3], control[2]], "left", DETAIL)
        for p in (control[1], control[2]):
            knot(canvas, p, 1.5, "new")
        for p in (control[0], control[3]):
            knot(canvas, p, 1.7)
    else:
        backed(canvas, curve)
        for p in (curve[0], curve[len(curve) // 2], curve[-1]):
            knot(canvas, p, 1.7)


# The path in model space, rising from the profile and curving away.
EXTRUDE_PATH: list[Vec3] = [(0.6, 0.6, 0.15), (0.6, 0.6, 1.25), (1.4, 0.2, 1.6), (2.4, -0.5, 1.35)]


def _extrude(canvas: Canvas, kind: str):
    profile = [(0, 0, 0), (1.2, 0, 0), (1.2, 1.2, 0), (0, 1.2, 0)]
    projection = fitted(profile + EXTRUDE_PATH, fit=24)
    canvas.highlight_face(projection, profile)
    _path(canvas, kind, [projection(p) for p in EXTRUDE_PATH])


# The clone path on the ground: an S from the front left up to the back
# right on screen, so the copies climb across the icon.
CLONE_PATH: list[Vec3] = [(1.0, 3.0, 0.0), (-0.5, 1.5, 0.0), (2.5, 0.5, 0.0), (1.0, -1.0, 0.0)]


def _on_path(t: float) -> Vec3:
    (ax, ay, _), (bx, by, _), (cx, cy, _), (dx, dy, _) = CLONE_PATH
    u = 1 - t
    return (u * u * u * ax + 3 * u * u * t * bx + 3 * u * t * t * cx + t * t * t * dx,
            u * u * u * ay + 3 * u * u * t * by + 3 * u * t * t * cy + t * t * t * dy, 0.0)


def _clone(canvas: Canvas, kind: str):
    size = 0.7
    boxes = []
    for i, t in enumerate((0.0, 0.5, 1.0)):
        x, y, _ = _on_path(t)
        o = (x - size / 2, y - size / 2, 0.0)
        faces = box(o, (o[0] + size, o[1] + size, size), top="top" if i == 0 else "new")
        boxes.append((x + y, faces))
    projection = fitted([p for _, f in boxes for p in corners(f)] + CLONE_PATH, fit=26)
    _path(canvas, kind, [projection(p) for p in CLONE_PATH])
    for _, faces in sorted(boxes, key=lambda b: b[0]):
        canvas.mesh(projection, faces)


def scatter_clone(canvas: Canvas):
    """Scatter Clone: copies dropped at loose spots on a flat patch."""
    size = 0.6
    patch = [(0, 0, 0), (2.8, 0, 0), (2.8, 2.8, 0), (0, 2.8, 0)]
    spots = [(0.2, 2.0), (1.7, 1.9), (2.0, 0.3), (0.9, 0.9)]
    boxes = []
    for i, (x, y) in enumerate(spots):
        faces = box((x, y, 0.0), (x + size, y + size, size), top="top" if i == 3 else "new")
        boxes.append((x + y, faces))
    projection = fitted(patch + [p for _, f in boxes for p in corners(f)], fit=26)
    canvas.face([projection(p) for p in patch], "left")
    canvas.edge([projection(p) for p in patch], "ink", DETAIL, closed=True)
    for _, faces in sorted(boxes, key=lambda b: b[0]):
        canvas.mesh(projection, faces)


def _make(fn, kind):
    def recipe(canvas: Canvas):
        fn(canvas, kind)
    return recipe


PATH_DUPLICATE_ICONS = {
    "curve-extrude": (_make(_extrude, "curve"), "Curve Extrude",
                      "Selected square profile and a cyan curve with knots on it to sweep along."),
    "bezier-extrude": (_make(_extrude, "bezier"), "Bezier Extrude",
                       "Selected square profile and a cyan Bezier curve with a tangent handle at each end."),
    "bspline-extrude": (_make(_extrude, "bspline"), "B-Spline Extrude",
                        "Selected square profile and a cyan B-spline inside its control polygon."),
    "pen-extrude": (_make(_extrude, "pen"), "Pen Extrude",
                    "Selected square profile and a cyan path of straight segments between knots."),
    "curve-clone": (_make(_clone, "curve"), "Curve Clone",
                    "A cube and copies with cyan tops along a cyan curve with knots on it."),
    "bezier-clone": (_make(_clone, "bezier"), "Bezier Clone",
                     "A cube and copies with cyan tops along a cyan Bezier curve with end handles."),
    "bspline-clone": (_make(_clone, "bspline"), "B-Spline Clone",
                      "A cube and copies with cyan tops along a cyan B-spline in its control polygon."),
    "pen-clone": (_make(_clone, "pen"), "Pen Clone",
                  "A cube and copies with cyan tops along straight cyan segments between knots."),
    "scatter-clone": (scatter_clone, "Scatter Clone",
                      "A cube and copies with cyan tops dropped at loose spots on a flat patch."),
}
