"""Batch 3: Edge tab tools and the Edge selection-mode icon.

Same conventions: yellow-orange = selected (here, selected edges), cyan = new
geometry. Add Loop uses the original Loop Cut icon.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, SELECT, SELECT_BACKING, Projection
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted

# The cube's front vertical edge, nearest the viewer.
FRONT = [(1, 1, 1), (1, 1, 0)]


def selected_edge(canvas: Canvas, projection: Projection, points):
    """A selected edge: yellow-orange over an ink backing so it reads on
    light faces as well as against the background."""
    projected = [projection(p) for p in points]
    canvas.edge(projected, "ink", SELECT_BACKING)
    canvas.edge(projected, "selected", SELECT)


def edge_mode(canvas: Canvas):
    """The Edge selection mode: a cube with its visible edges selected."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))))
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    for a, b in (((0, 1, 1), (1, 1, 1)), ((1, 0, 1), (1, 1, 1)), FRONT):
        selected_edge(canvas, projection, [a, b])


def _rounded_front(canvas: Canvas, rows: int, amount: float):
    """The cube with its front vertical edge cut into `rows` cyan strips on
    a quarter-circle profile (one row = a flat chamfer)."""
    a = amount
    faces = [
        ([(0, 0, 1), (1, 0, 1), (1, 1 - a, 1), (1 - a, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1 - a, 1, 1), (1 - a, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1), (1, 1 - a, 1), (1, 1 - a, 0), (1, 0, 0)], "right"),
    ]
    def profile(k, z):
        angle = pi / 2 * k / rows
        return (1 - a + a * sin(angle), 1 - a + a * cos(angle), z)
    for k in range(rows):
        faces.append(([profile(k, 1), profile(k + 1, 1), profile(k + 1, 0), profile(k, 0)],
                      "new"))
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)


def edge_bevel(canvas: Canvas):
    _rounded_front(canvas, 3, 0.36)


def edge_chamfer(canvas: Canvas):
    _rounded_front(canvas, 1, 0.36)


def edge_extend(canvas: Canvas):
    """An open plane whose selected border edge is extended by a cyan row."""
    plane = [(0, 0, 0), (1.1, 0, 0), (1.1, 1.1, 0), (0, 1.1, 0)]
    row = [(1.1, 0, 0), (1.9, 0, 0.35), (1.9, 1.1, 0.35), (1.1, 1.1, 0)]
    projection = fitted(plane + row)
    canvas.mesh(projection, [(plane, "top"), (row, "new")])
    selected_edge(canvas, projection, [(1.9, 0, 0.35), (1.9, 1.1, 0.35)])


def edge_extrude(canvas: Canvas):
    """The top-right edge pulled out (up and to the right) into a cyan ridge."""
    d, w = 0.45, 0.3
    ridge_back, ridge_front = (1 + d, 0, 1 + d), (1 + d, 1, 1 + d)
    faces = [
        ([(0, 0, 1), (1 - w, 0, 1), (1 - w, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1 - w, 1, 1), (1, 1, 1 - w), (1, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1 - w), (1, 1, 1 - w), (1, 1, 0), (1, 0, 0)], "right"),
        ([(1 - w, 0, 1), ridge_back, ridge_front, (1 - w, 1, 1)], "new"),
        ([(1 - w, 1, 1), ridge_front, (1, 1, 1 - w)], "new"),
        ([ridge_back, (1, 0, 1 - w), (1, 1, 1 - w), ridge_front], "new"),
    ]
    projection = fitted(corners(faces))
    canvas.mesh(projection, faces)
    selected_edge(canvas, projection, [ridge_back, ridge_front])


def edge_slide(canvas: Canvas):
    """A selected edge on the top face with a double arrow along the face."""
    projection = Projection()
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    selected_edge(canvas, projection, [(0.5, 0, 1), (0.5, 1, 1)])
    a, b = projection((0.08, 0.5, 1)), projection((0.92, 0.5, 1))
    mid = projection((0.5, 0.5, 1))
    canvas.arrow(mid, a, "ink", length=2.1, half_width=1.05)
    canvas.arrow(mid, b, "ink", length=2.1, half_width=1.05)


def edge_relax(canvas: Canvas):
    """A jagged edge chain on a plane, smoothed into a cyan curve."""
    plane = [(0, 0, 0), (2, 0, 0), (2, 2, 0), (0, 2, 0)]
    projection = fitted(plane)
    canvas.mesh(projection, [(plane, "top")])
    xs = [0.2 + i * 0.32 for i in range(6)]
    jagged = [(x, 1.0 + (0.45 if i % 2 else -0.45), 0) for i, x in enumerate(xs)]
    canvas.edge([projection(p) for p in jagged], width=EDGE)
    smooth = [(0.2 + 1.6 * t, 1.0 + 0.18 * sin(2 * pi * t), 0) for t in
              (i / 16 for i in range(17))]
    canvas.edge([projection(p) for p in smooth], "new", ACCENT)


def edge_crease(canvas: Canvas):
    """A selected front edge kept sharp: a cyan angle mark across it."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))))
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    selected_edge(canvas, projection, FRONT)
    mark = [(1 - 0.5, 1, 0.5), (1, 1, 0.5), (1, 1 - 0.5, 0.5)]
    canvas.edge([projection(p) for p in mark], "new", ACCENT)


def add_point(canvas: Canvas):
    """A new cyan point splitting the cube's front top edge."""
    projection = fitted(corners(box((0, 0, 0), (1, 1, 1))))
    canvas.mesh(projection, box((0, 0, 0), (1, 1, 1)))
    canvas.vertex(projection((0.5, 1, 1)), 2.4, "ink")
    canvas.vertex(projection((0.5, 1, 1)), 1.6, "new")


EDGE_ICONS = {
    "edge": (edge_mode, "Edge", "Cube with its visible edges selected (Edge selection mode)."),
    "edge-bevel": (edge_bevel, "Edge Bevel", "Front edge rounded into cyan strips."),
    "edge-chamfer": (edge_chamfer, "Edge Chamfer", "Front edge cut into one flat cyan strip."),
    "edge-extend": (edge_extend, "Edge Extend", "An open plane extended past its selected border edge by a cyan row."),
    "edge-extrude": (edge_extrude, "Edge Extrude", "Top-right edge pulled out into a cyan ridge."),
    "edge-slide": (edge_slide, "Edge Slide", "Selected edge on the top face with a double arrow along the face."),
    "edge-relax": (edge_relax, "Edge Relax", "A jagged edge chain smoothed into a cyan curve."),
    "edge-crease": (edge_crease, "Edge Crease", "Selected front edge with a cyan sharpness mark."),
    "edge-add-point": (add_point, "Add Point", "A new cyan point on the cube's front top edge."),
}
