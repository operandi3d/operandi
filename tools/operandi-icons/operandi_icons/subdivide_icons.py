"""Batch 10: the Edit tab's Subdivide group (SDS Subdivide, Faceted, Smooth,
Options..., UnSubdivide) and Freeze...

Same conventions: cyan = new geometry, ink outlines, the cube's tones. Each
Subdivide icon is one thin quad slab seen from the toolbox's usual angle
(tilted, Jason 2026-11-08), split
into four by cyan lines; the tile's outline says the method: rounded corners
for SDS Subdivide (Catmull-Clark rounds the shape), straight for Faceted
(nothing moves), sides bowed outward for Smooth (new points pushed out).
UnSubdivide is the inverse: the four pieces' lines fade and one cyan outline
holds them. Freeze is a cube carrying a cyan snowflake.
"""

from math import cos, copysign, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, Vec3
from .deform_icons import _normal, shaded
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted

SQUARE: list[Vec3] = [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)]


def _rounded_square(samples: int = 48, power: float = 4.0) -> list[Vec3]:
    """A squircle in the unit square: |x|^p + |y|^p = 1 around the centre."""
    points = []
    for i in range(samples):
        t = 2 * pi * i / samples
        c, s = cos(t), sin(t)
        x = copysign(abs(c) ** (2 / power), c)
        y = copysign(abs(s) ** (2 / power), s)
        points.append((0.5 + 0.5 * x, 0.5 + 0.5 * y, 0.0))
    return points


def _bowed_square(bow: float = 0.14, samples: int = 10) -> list[Vec3]:
    """The unit square with each side bowed outward by `bow` at its middle."""
    points = []
    for i, (a, b) in enumerate(zip(SQUARE, SQUARE[1:] + SQUARE[:1])):
        # Outward normal of side i (counter-clockwise square).
        normal = [(0, -1), (1, 0), (0, 1), (-1, 0)][i]
        for k in range(samples):
            t = k / samples
            out = bow * 4 * t * (1 - t)
            points.append((a[0] + (b[0] - a[0]) * t + normal[0] * out,
                           a[1] + (b[1] - a[1]) * t + normal[1] * out, 0.0))
    return points


CUBE_POINTS: list[Vec3] = [(x, y, z) for z in (0, 1) for y in (0, 1) for x in (0, 1)]
# Counter-clockwise seen from outside.
CUBE_FACES = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [1, 3, 7, 5], [0, 4, 6, 2]]
CENTER: Vec3 = (0.5, 0.5, 0.5)


def _avg(points) -> Vec3:
    points = list(points)
    return tuple(sum(p[k] for p in points) / len(points) for k in range(3))


def _split_cube(move_corners: bool, push: float = 0.0):
    """The cube split once into 24 quads: Catmull-Clark when `move_corners`
    (every point moves, the shape rounds), else new points only, pushed out
    along their faces' normals by `push` x (edge: sqrt(6)/24, centre:
    sqrt(3)/6, the Smooth rule's cube values)."""
    pts = CUBE_POINTS
    face_pt = [_avg(pts[i] for i in f) for f in CUBE_FACES]
    normal = [tuple(2 * (c - 0.5) for c in fp) for fp in face_pt]
    edges = {}
    for fi, f in enumerate(CUBE_FACES):
        for k in range(4):
            e = tuple(sorted((f[k], f[(k + 1) % 4])))
            edges.setdefault(e, []).append(fi)
    edge_pt = {}
    for e, fs in edges.items():
        mid = _avg((pts[e[0]], pts[e[1]]))
        if move_corners:
            edge_pt[e] = _avg([pts[e[0]], pts[e[1]], face_pt[fs[0]], face_pt[fs[1]]])
        else:
            n = tuple(normal[fs[0]][k] + normal[fs[1]][k] for k in range(3))
            edge_pt[e] = tuple(mid[k] + n[k] * push * 6 ** 0.5 / 24 for k in range(3))
    corners = list(pts)
    if move_corners:
        for v in range(8):
            fs = [face_pt[fi] for fi, f in enumerate(CUBE_FACES) if v in f]
            mids = [_avg((pts[e[0]], pts[e[1]])) for e in edges if v in e]
            F, R, P = _avg(fs), _avg(mids), pts[v]
            corners[v] = tuple((F[k] + 2 * R[k]) / 3 for k in range(3))
    else:
        face_pt = [tuple(fp[k] + normal[i][k] * push * 3 ** 0.5 / 6 for k in range(3))
                   for i, fp in enumerate(face_pt)]
    quads = []
    for fi, f in enumerate(CUBE_FACES):
        for k in range(4):
            a = tuple(sorted((f[k], f[(k + 1) % 4])))
            b = tuple(sorted((f[k - 1], f[k])))
            quads.append(([corners[f[k]], edge_pt[a], face_pt[fi], edge_pt[b]], None))
    return quads


def _solid_icon(canvas: Canvas, quads, extra: list[Vec3] = (), fit: float = 25.5):
    projection = fitted([p for q, _ in quads for p in q] + list(extra), fit=fit)
    shaded(canvas, projection, quads, inside=CENTER)
    return projection


def _face_cross(canvas: Canvas, projection, color="new", width=ACCENT):
    """Straight lines splitting the cube's three visible faces into four."""
    for a, b in [((0.5, 0, 1), (0.5, 1, 1)), ((0, 0.5, 1), (1, 0.5, 1)),
                 ((0.5, 1, 1), (0.5, 1, 0)), ((0, 1, 0.5), (1, 1, 0.5)),
                 ((1, 0.5, 1), (1, 0.5, 0)), ((1, 0, 0.5), (1, 1, 0.5))]:
        canvas.edge([projection(a), projection(b)], color, width)


def sds_subdivide(canvas: Canvas):
    """SDS Subdivide: the cube after one Catmull-Clark step, rounded, its
    new edges cyan."""
    quads = _split_cube(True)
    projection = _solid_icon(canvas, quads)
    _new_edges(canvas, projection, quads)


def _new_edges(canvas: Canvas, projection, quads):
    """Each visible quad's two inner sides (edge point to centre) in cyan."""
    for points, _ in quads:
        n = _normal(points)
        out = tuple(_avg(points)[k] - CENTER[k] for k in range(3))
        if sum(n[k] * out[k] for k in range(3)) < 0:
            n = tuple(-v for v in n)
        if sum(n) <= 1e-9:
            continue
        canvas.edge([projection(points[1]), projection(points[2]), projection(points[3])],
                    "new", ACCENT)


def faceted(canvas: Canvas):
    """Faceted: the cube's faces split straight into four; nothing moves."""
    faces = box((0, 0, 0), (1, 1, 1))
    projection = fitted(corners(faces), fit=25.5)
    canvas.mesh(projection, faces)
    _face_cross(canvas, projection)


def smooth_subdivide(canvas: Canvas):
    """Smooth: corners stay put (ink), the new points push out (cyan dots),
    so the faces bulge (the Smooth rule's cube)."""
    quads = _split_cube(False, push=1.0)
    projection = _solid_icon(canvas, quads)
    _new_edges(canvas, projection, quads)
    for points, _ in quads:
        n = _normal(points)
        out = tuple(_avg(points)[k] - CENTER[k] for k in range(3))
        if sum(n[k] * out[k] for k in range(3)) < 0:
            n = tuple(-v for v in n)
        if sum(n) > 1e-9:
            canvas.vertex(projection(points[1]), 1.2, "new")
            canvas.vertex(projection(points[0]), 1.0)


def subdivide_options(canvas: Canvas):
    """Options...: the SDS Subdivide cube with a row of three dots (the
    dialog) at its lower right."""
    quads = _split_cube(True)
    projection = fitted([p for q, _ in quads for p in q], fit=22)
    shaded(canvas, projection, quads, inside=CENTER)
    _new_edges(canvas, projection, quads)
    bottom = max(projection(p)[1] for q, _ in quads for p in q)
    right = max(projection(p)[0] for q, _ in quads for p in q)
    for k in range(3):
        canvas.vertex((right - 4.2 + k * 3.4, bottom + 2.6), 1.35, "new")


def unsubdivide(canvas: Canvas):
    """UnSubdivide: the split lines fade; each face is one quad again, its
    outline cyan."""
    faces = box((0, 0, 0), (1, 1, 1))
    projection = fitted(corners(faces), fit=25.5)
    canvas.mesh(projection, faces)
    _face_cross(canvas, projection, color="left", width=DETAIL)
    for points, _ in faces:
        canvas.edge([projection(p) for p in points], "new", ACCENT, closed=True)


def freeze(canvas: Canvas):
    """Freeze...: a cube with a cyan six-armed snowflake on its top."""
    faces = box((0, 0, 0), (1, 1, 1))
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)
    centre = (0.5, 0.5, 1.0)
    arm = 0.44
    for k in range(6):
        a = pi * k / 3
        direction = (cos(a), sin(a))
        tip = (centre[0] + arm * direction[0], centre[1] + arm * direction[1], 1.0)
        canvas.edge([projection(centre), projection(tip)], "new", ACCENT)
        if k % 2 == 0:
            # A small V near the tip of every other arm keeps it a snowflake.
            base = (centre[0] + 0.62 * arm * direction[0],
                    centre[1] + 0.62 * arm * direction[1], 1.0)
            for side in (-1, 1):
                b = a + side * pi / 4
                end = (base[0] + 0.14 * cos(b), base[1] + 0.14 * sin(b), 1.0)
                canvas.edge([projection(base), projection(end)], "new", ACCENT)


SUBDIVIDE_ICONS = {
    "sds-subdivide": (sds_subdivide, "SDS Subdivide",
                      "A rounded quad tile split into four by cyan lines."),
    "subdivide-faceted": (faceted, "Faceted",
                          "A straight quad tile split into four by cyan lines."),
    "subdivide-smooth": (smooth_subdivide, "Smooth",
                         "A quad tile with bowed-out sides split into four by cyan lines."),
    "subdivide-options": (subdivide_options, "Subdivide Options",
                          "A split quad tile with three dots for the options dialog."),
    "unsubdivide": (unsubdivide, "UnSubdivide",
                    "Four faded quads held by one cyan outline: merged back into one."),
    "freeze": (freeze, "Freeze", "A cube with a cyan snowflake on its top face."),
}
