"""Batch 4: Vertex tab tools and the Vertex selection-mode icon.

Same conventions: yellow-orange = selected (here, a selected point is a
yellow-orange dot with an ink ring), cyan = new geometry.
"""

from .conventions import ACCENT, Point, Projection
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import fitted

CUBE = box((0, 0, 0), (1, 1, 1))


def selected_point(canvas: Canvas, point: Point, radius: float = 1.7):
    canvas.vertex(point, radius + 0.7, "ink")
    canvas.vertex(point, radius, "selected")


def new_point(canvas: Canvas, point: Point, radius: float = 1.6):
    canvas.vertex(point, radius + 0.7, "ink")
    canvas.vertex(point, radius, "new")


def cube_projection() -> Projection:
    # Slightly smaller than full size, so corner dots stay inside the canvas.
    return fitted(corners(CUBE), fit=22.5)


def vertex_mode(canvas: Canvas):
    """The Vertex selection mode: a cube with its visible corners selected."""
    projection = cube_projection()
    canvas.mesh(projection, CUBE)
    for p in [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1), (0, 1, 0), (1, 1, 0), (1, 0, 0)]:
        selected_point(canvas, projection(p), 1.35)


def vertex_create(canvas: Canvas):
    """New cyan points placed on the work plane, with a plus mark."""
    plane = [(0, 0, 0), (1.6, 0, 0), (1.6, 1.6, 0), (0, 1.6, 0)]
    projection = fitted(plane, fit=24)
    canvas.mesh(projection, [(plane, "top")])
    for p in [(0.4, 0.45, 0), (1.15, 0.5, 0), (0.75, 1.2, 0)]:
        new_point(canvas, projection(p))
    cx, cy = projection((1.25, 1.25, 0))
    canvas.edge([(cx - 2.4, cy), (cx + 2.4, cy)], "ink", ACCENT)
    canvas.edge([(cx, cy - 2.4), (cx, cy + 2.4)], "ink", ACCENT)


def vertex_bevel(canvas: Canvas):
    """The cube's upper-right corner cut off by a cyan triangle."""
    a = 0.58
    p, q, r = (1 - a, 0, 1), (1, a, 1), (1, 0, 1 - a)
    faces = [
        ([(0, 0, 1), p, q, (1, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)], "left"),
        ([q, (1, 1, 1), (1, 1, 0), (1, 0, 0), r], "right"),
        ([p, r, q], "new"),
    ]
    canvas.mesh(cube_projection(), faces)


def vertex_extrude(canvas: Canvas):
    """The upper-right corner pulled out into a cyan point, now selected."""
    a, d = 0.34, 0.42
    p, q, r = (1 - a, 0, 1), (1, a, 1), (1, 0, 1 - a)
    tip = (1 + d, -d, 1 + d)
    faces = [
        ([(0, 0, 1), p, q, (1, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)], "left"),
        ([q, (1, 1, 1), (1, 1, 0), (1, 0, 0), r], "right"),
        ([p, q, tip], "new"), ([q, r, tip], "new"),
    ]
    projection = fitted(corners(faces), fit=23)
    canvas.mesh(projection, faces)
    selected_point(canvas, projection(tip), 1.4)


def vertex_merge(canvas: Canvas):
    """Three selected points drawn together into one."""
    plane = [(0, 0, 0), (1.6, 0, 0), (1.6, 1.6, 0), (0, 1.6, 0)]
    projection = fitted(plane, fit=24)
    canvas.mesh(projection, [(plane, "top")])
    target = projection((0.8, 0.8, 0))
    for p in [(0.2, 0.25, 0), (1.4, 0.45, 0), (0.6, 1.45, 0)]:
        start = projection(p)
        end = (target[0] + (start[0] - target[0]) * 0.32, target[1] + (start[1] - target[1]) * 0.32)
        canvas.arrow(start, end, "ink", length=1.7, half_width=0.9)
        selected_point(canvas, start, 1.3)
    new_point(canvas, target, 1.4)


def vertex_weld(canvas: Canvas):
    """A selected point moved onto another point (a corner) and joined."""
    projection = cube_projection()
    canvas.mesh(projection, CUBE)
    start, corner = projection((0.2, 0.75, 1)), projection((1, 0, 1))
    canvas.vertex(corner, 2.3, "ink")
    canvas.vertex(corner, 1.5, "top")
    canvas.arrow(start, (corner[0] - 2.6, corner[1] + 0.4), "ink", length=2.0, half_width=1.0)
    selected_point(canvas, start, 1.5)


def vertex_tear(canvas: Canvas):
    """The top face's upper-right corner torn away from the side and lifted."""
    lift = 0.42
    faces = [
        ([(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1), (1, 1, 1), (1, 1, 0), (1, 0, 0)], "right"),
        ([(0, 0, 1), (1, 0, 1 + lift), (1, 1, 1), (0, 1, 1)], "top"),
    ]
    projection = fitted(corners(faces), fit=23)
    canvas.mesh(projection, faces)
    selected_point(canvas, projection((1, 0, 1 + lift)), 1.5)


def vertex_slide(canvas: Canvas):
    """A selected point on the upper-right edge, with a two-way arrow along
    that edge drawn just inside the top face."""
    projection = cube_projection()
    canvas.mesh(projection, CUBE)
    a, b, m = projection((0.72, 0.1, 1)), projection((0.72, 0.9, 1)), projection((0.72, 0.5, 1))
    canvas.arrow(m, a, "ink", length=2.0, half_width=1.0)
    canvas.arrow(m, b, "ink", length=2.0, half_width=1.0)
    selected_point(canvas, projection((1, 0.5, 1)), 1.5)


VERTEX_ICONS = {
    "vertex": (vertex_mode, "Vertex", "Cube with its visible corners selected (Vertex selection mode)."),
    "vertex-create": (vertex_create, "Create", "New cyan points on the work plane, with a plus mark."),
    "vertex-bevel": (vertex_bevel, "Vertex Bevel", "Upper-right corner cut off by a cyan triangle."),
    "vertex-extrude": (vertex_extrude, "Vertex Extrude", "Upper-right corner pulled out into a cyan point."),
    "vertex-merge": (vertex_merge, "Merge Tool", "Three selected points drawn together into one."),
    "vertex-weld": (vertex_weld, "Weld", "A selected point moving onto the cube's corner point."),
    "vertex-tear": (vertex_tear, "Tear", "The top face's upper-right corner torn away and lifted."),
    "vertex-slide": (vertex_slide, "Vertex Slide", "A selected point on an edge with a two-way arrow along it."),
}
