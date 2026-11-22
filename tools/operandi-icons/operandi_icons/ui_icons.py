"""Batch 5: workspace symbols (selection mode, Scenes list, context bar).

Mostly flat shapes in the same palette and stroke widths. Curves are sampled
polygons, as in solids.py.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, Point
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import ellipsoid, ellipsoid_points, fitted


def circle(center: Point, radius: float, samples: int = 40) -> list[Point]:
    cx, cy = center
    return [(cx + radius * cos(2 * pi * i / samples), cy + radius * sin(2 * pi * i / samples))
            for i in range(samples)]


def polygon_mode(canvas: Canvas):
    """The Polygon selection mode: a cube with its top face selected."""
    faces = box((0, 0, 0), (1, 1, 1), top="selected")
    canvas.mesh(fitted(corners(faces)), faces)


def eye(canvas: Canvas):
    """Visibility: an almond eye with an iris and pupil."""
    upper = [(4 + 24 * t, 16 - 8.5 * sin(pi * t)) for t in (i / 20 for i in range(21))]
    lower = [(28 - 24 * t, 16 + 8.5 * sin(pi * t)) for t in (i / 20 for i in range(1, 20))]
    outline = upper + lower
    canvas.face(outline, "top")
    canvas.face(circle((16, 16), 5.6), "left")
    canvas.face(circle((16, 16), 2.6), "ink")
    canvas.edge(outline, closed=True, width=ACCENT)


def disclosure(canvas: Canvas):
    """Expand/collapse: a down-pointing triangle."""
    tri = [(7, 10), (25, 10), (16, 23)]
    canvas.face(tri, "left")
    canvas.edge(tri, closed=True, width=ACCENT)


def action_center(canvas: Canvas):
    """Action Center: a ring with a crosshair and its center point."""
    canvas.edge(circle((16, 16), 9.5), closed=True, width=ACCENT)
    for a, b in (((16, 3), (16, 10)), ((16, 22), (16, 29)), ((3, 16), (10, 16)), ((22, 16), (29, 16))):
        canvas.edge([a, b], width=ACCENT)
    canvas.vertex((16, 16), 2.6, "ink")
    canvas.vertex((16, 16), 1.8, "new")


def falloff(canvas: Canvas):
    """Falloff: nested discs fading from full effect (light) outward."""
    for radius, tone in ((12.5, "right"), (8.6, "left"), (4.6, "top")):
        canvas.face(circle((16, 16), radius), tone)
    canvas.edge(circle((16, 16), 12.5), closed=True, width=EDGE)


def symmetry(canvas: Canvas):
    """Symmetry: two mirrored shapes either side of a cyan mirror line."""
    left = [(13.5, 7), (13.5, 25), (4, 20), (4, 12)]
    right = [(18.5, 7), (18.5, 25), (28, 20), (28, 12)]
    canvas.face(left, "left")
    canvas.face(right, "right")
    canvas.edge(left, closed=True)
    canvas.edge(right, closed=True)
    canvas.edge([(16, 3.5), (16, 28.5)], "new", ACCENT)


def snapping(canvas: Canvas):
    """Snapping: a magnet."""
    # U shape: outer arc from left leg to right leg, then back along the inner arc.
    arc_out = [(16 - 10 * cos(pi * t), 18 + 10 * sin(pi * t)) for t in (i / 16 for i in range(17))]
    arc_in = [(16 + 4.6 * cos(pi * t), 18 + 4.6 * sin(pi * t)) for t in (i / 16 for i in range(17))]
    shape = [(6, 5)] + arc_out + [(26, 5), (20.6, 5)] + arc_in + [(11.4, 5)]
    canvas.face(shape, "left")
    canvas.face([(6, 5), (11.4, 5), (11.4, 10), (6, 10)], "x")
    canvas.face([(20.6, 5), (26, 5), (26, 10), (20.6, 10)], "z")
    canvas.edge(shape, closed=True)
    canvas.edge([(6, 10), (11.4, 10)])
    canvas.edge([(20.6, 10), (26, 10)])


def workplane(canvas: Canvas):
    """Work Plane: a gridded plane with its red and green axes."""
    plane = [(0, 0, 0), (2, 0, 0), (2, 2, 0), (0, 2, 0)]
    projection = fitted(plane)
    canvas.face([projection(p) for p in plane], "top")
    for t in (1.0,):  # one line each way: a 2x2 grid stays legible at 24 px
        canvas.edge([projection((t, 0, 0)), projection((t, 2, 0))], width=DETAIL)
        canvas.edge([projection((0, t, 0)), projection((2, t, 0))], width=DETAIL)
    canvas.edge([projection(p) for p in plane], closed=True)
    canvas.edge([projection((0, 2, 0)), projection((2, 2, 0))], "x", ACCENT)
    canvas.edge([projection((2, 0, 0)), projection((2, 2, 0))], "y", ACCENT)


def scene(canvas: Canvas):
    """Scene: a cube standing on a ground plane, the Items list's root row."""
    ground = [(-0.5, -0.5, 0), (1.5, -0.5, 0), (1.5, 1.5, 0), (-0.5, 1.5, 0)]
    projection = fitted(ground + [(0, 0, 1), (1, 1, 1)])
    canvas.face([projection(p) for p in ground], "top")
    canvas.edge([projection(p) for p in ground], closed=True, width=DETAIL)
    a, b, c, d = (0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)
    e, f, g = (1, 1, 0), (1, 0, 0), (0, 1, 0)
    canvas.mesh(projection, [([a, b, c, d], "top"), ([d, c, e, g], "left"),
                             ([b, c, e, f], "right")])


def bug(canvas: Canvas):
    """Report Bug: a beetle seen from above (body, head, six legs, antennae).

    The body and head use the light `top` tone so the app can tint them with
    its Report Bug theme colour (a multiply tint leaves the ink dark)."""
    # Legs first, so the body covers their inner ends. Legs and antennae
    # are a mid grey so they read on both dark and light panels.
    for y, spread in ((14.5, -2.5), (19, 0), (23.5, 2.5)):
        canvas.edge([(16, y), (6, y + spread)], "left", ACCENT)
        canvas.edge([(16, y), (26, y + spread)], "left", ACCENT)
    # Antennae from the head.
    canvas.edge([(14.4, 9.4), (11.5, 4.5)], "left", ACCENT)
    canvas.edge([(17.6, 9.4), (20.5, 4.5)], "left", ACCENT)
    head = circle((16, 10.8), 3.4, 32)
    body = [(16 + 7 * cos(2 * pi * i / 48), 19.5 + 8.5 * sin(2 * pi * i / 48))
            for i in range(48)]
    canvas.face(head, "top")
    canvas.edge(head, closed=True)
    canvas.face(body, "top")
    canvas.edge(body, closed=True)
    # Wing-case seam and two spots.
    canvas.edge([(16, 11.2), (16, 27.8)], width=DETAIL)
    canvas.vertex((12.6, 17.6), 1.3, "ink")
    canvas.vertex((19.4, 21.6), 1.3, "ink")



def render(canvas: Canvas):
    """Render: the Sphere icon's shaded ball hung in a picture frame, the
    finished picture of the scene. The frame is a flat band in the mid tone
    between an inked outer and inner edge; the ball keeps the Sphere icon's
    nested tones and ink."""
    outer = [(2.5, 5.0), (29.5, 5.0), (29.5, 27.0), (2.5, 27.0)]
    inner = [(5.3, 7.8), (26.7, 7.8), (26.7, 24.2), (5.3, 24.2)]
    band = outer + [outer[0], inner[0]] + inner[::-1] + [inner[0]]
    canvas.face(band, "left")
    canvas.edge(outer, closed=True, width=ACCENT)
    canvas.edge(inner, closed=True)
    centre, radii = (0.5, 0.5, 0.5), (0.74, 0.74, 0.74)
    ellipsoid(canvas, fitted(ellipsoid_points(centre, radii), fit=14.0), centre, radii)


UI_ICONS = {
    "polygon": (polygon_mode, "Polygon", "Cube with its top face selected (Polygon selection mode)."),
    "eye": (eye, "Visible", "An eye: the item is shown."),
    "disclosure": (disclosure, "Disclosure", "A down-pointing triangle for expanding a section."),
    "action-center": (action_center, "Action Center", "A ring with a crosshair and its center point."),
    "falloff": (falloff, "Falloff", "Nested discs fading from light at the center to dark outside."),
    "symmetry": (symmetry, "Symmetry", "Two mirrored shapes either side of a cyan mirror line."),
    "snapping": (snapping, "Snapping", "A magnet."),
    "workplane": (workplane, "Work Plane", "A gridded plane with its red and green axes."),
    "scene": (scene, "Scene", "A cube standing on a ground plane."),
    "bug": (bug, "Report Bug", "A beetle seen from above: report a bug."),
    "render": (render, "Render", "A shaded sphere in a rectangular picture frame: render the scene."),
}
