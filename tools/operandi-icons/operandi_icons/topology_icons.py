"""Topology Pen (spec TP-01): a flat patch of quads being grown point by
point, the way a retopology layer is drawn. Original drawing in the
primitives' style."""

from .conventions import ACCENT, Vec3
from .primitives import Canvas
from .solids import fitted


def topology_pen(canvas: Canvas):
    """A flat strip of quads; a new cyan quad is pulled out from its end
    edge, and its two new points ride on the pointer."""
    rows = (0.0, 1.0)
    columns = (0.0, 1.0, 2.0)
    pulled = [(2.95, -0.1, 0.0), (2.95, 1.05, 0.0)]
    points: list[Vec3] = [(x, y, 0.0) for x in columns for y in rows] + pulled
    projection = fitted(points, fit=24)
    p = lambda q: projection(q)  # noqa: E731
    quads = [[(a, 0.0, 0.0), (b, 0.0, 0.0), (b, 1.0, 0.0), (a, 1.0, 0.0)]
             for a, b in zip(columns, columns[1:])]
    for quad in quads:
        canvas.face([p(q) for q in quad], "top")
    new = [(2.0, 0.0, 0.0), pulled[0], pulled[1], (2.0, 1.0, 0.0)]
    canvas.face([p(q) for q in new], "new")
    for quad in quads:
        canvas.edge([p(q) for q in quad], closed=True)
    canvas.edge([p(q) for q in new], "ink", ACCENT, closed=True)
    for q in [(x, y, 0.0) for x in columns for y in rows]:
        canvas.vertex(p(q), 1.2)
    # The two new points: light dots in ink rings, they read on dark too.
    for q in pulled:
        canvas.vertex(p(q), 1.9)
        canvas.vertex(p(q), 1.15, "top")


TOPOLOGY_ICONS = {
    "topology-pen": (topology_pen, "Topology Pen",
                     "A flat strip of quads; a new cyan quad is pulled out from its end edge."),
}
