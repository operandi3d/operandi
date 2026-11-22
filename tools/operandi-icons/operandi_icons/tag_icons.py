"""Batch 9: Polygon panel script commands (Scale Offset, Scale Offset Detach,
Flatten Ngons) and the polygon tag types of the tag selection mode button
(Material, Part, Smoothing Group, Selection Set).

Same conventions: yellow-orange = selected, cyan = new geometry. The tag type
icons all show a cube with a selected region, each with its own type cue, so
the mode button reads as a selection mode next to Vertices/Edges/Polygons.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, SELECT, SELECT_BACKING, Projection
from .polygon_icons import box, corners
from .primitives import Canvas
from .solids import ellipsoid, ellipsoid_points, fitted


def _frustum(t: float, h: float, top="selected"):
    """A block whose top face is shrunk by `t` on every side: the walls lean
    in to follow it, nothing new is added."""
    a, b = t, 1 - t
    top_face = [(a, a, h), (b, a, h), (b, b, h), (a, b, h)]
    return [
        (top_face, top),
        ([(a, b, h), (b, b, h), (1, 1, 0), (0, 1, 0)], "left"),
        ([(b, a, h), (b, b, h), (1, 1, 0), (1, 0, 0)], "right"),
    ]


def scale_offset(canvas: Canvas):
    """The selected top face shrunk in its own plane; the sides lean in to
    follow it (no new polygons). Arrows run from where its outer corners
    were to where they are now."""
    t, h = 0.2, 0.85
    faces = _frustum(t, h)
    was = [(0, 1, h), (1, 0, h)]
    projection = fitted(corners(faces) + was)
    canvas.mesh(projection, faces)
    for (x, y, z) in was:
        target = (t if x == 0 else 1 - t, t if y == 0 else 1 - t, z)
        start, end = projection((x, y, z)), projection(target)
        # Selected-colour arrows on an ink backing read on both themes.
        canvas.arrow(start, end, "ink", length=1.3, half_width=0.9, width=SELECT_BACKING)
        canvas.arrow(start, end, "selected", length=1.3, half_width=0.9, width=SELECT)


def scale_offset_detach(canvas: Canvas):
    """A smaller selected face lifted off the cube's top: the inset polygon
    on its own new (cyan) points, no longer joined to its neighbours."""
    cube = box((0, 0, 0), (1, 1, 0.7))
    t, lift = 0.22, 1.05
    a, b = t, 1 - t
    floating = [(a, a, lift), (b, a, lift), (b, b, lift), (a, b, lift)]
    projection = fitted(corners(cube) + floating)
    canvas.mesh(projection, cube)
    # Where it came from: the hole outline on the cube's top.
    canvas.edge([projection((a, a, 0.7)), projection((b, a, 0.7)), projection((b, b, 0.7)),
                 projection((a, b, 0.7))], closed=True, width=DETAIL)
    canvas.highlight_face(projection, floating)
    for p in floating[1:]:
        canvas.vertex(projection(p), 1.55, "ink")
        canvas.vertex(projection(p), 0.95, "new")


def flatten_ngons(canvas: Canvas):
    """A five-sided polygon laid flat; its one raised corner (where it was,
    joined by thin lines) comes down onto the plane along a cyan arrow."""
    flat = [(0, 0.35, 0), (1.25, 0, 0), (2.05, 0.95, 0), (1.55, 2.05, 0), (0.15, 1.75, 0)]
    raised = (2.05, 0.95, 1.15)
    projection = fitted(flat + [raised], fit=26)
    pts = [projection(p) for p in flat]
    ghost = projection(raised)
    canvas.face(pts, "selected")
    canvas.edge(pts, closed=True)
    canvas.edge([pts[1], ghost, pts[3]], "left", ACCENT)
    canvas.arrow(ghost, (pts[2][0], pts[2][1] - 1.4), "new",
                 length=2.2, half_width=1.3, width=ACCENT)
    canvas.vertex(ghost, 1.2, "ink")


# Tag type icons. One cube, one projection, so the four read as a family.
CUBE = box((0, 0, 0), (1, 1, 1))


def _cube_projection() -> Projection:
    return fitted(corners(CUBE), fit=27)


def tag_material(canvas: Canvas):
    """Material: a shaded material ball in front of a cube whose top and
    right faces (one material) are selected."""
    faces = box((0, 0, 0), (1, 1, 1), top="selected", right="selected")
    projection = fitted(corners(faces) + [(-0.35, 1.55, -0.1)], fit=27)
    canvas.mesh(projection, faces)
    center, radius = (0.05, 1.25, 0.2), (0.48, 0.48, 0.48)
    ellipsoid(canvas, projection, center, radius)


def tag_part(canvas: Canvas):
    """Part: the cube split into two blocks with a gap; the right one (a
    part) is selected."""
    gap = 0.18
    left = box((0, 0, 0), (0.48, 1, 1))
    right = box((0.48 + gap, 0, 0), (1 + gap, 1, 1), top="selected", left="selected",
                right="selected")
    projection = fitted(corners(left + right), fit=27)
    canvas.mesh(projection, left)
    canvas.mesh(projection, right)


def tag_smoothing(canvas: Canvas):
    """Smoothing Group: the cube's front edge rounded into a smooth
    selected strip, its rows drawn in fine lines."""
    a, rows = 0.42, 4
    faces = [
        ([(0, 0, 1), (1, 0, 1), (1, 1 - a, 1), (1 - a, 1, 1), (0, 1, 1)], "top"),
        ([(0, 1, 1), (1 - a, 1, 1), (1 - a, 1, 0), (0, 1, 0)], "left"),
        ([(1, 0, 1), (1, 1 - a, 1), (1, 1 - a, 0), (1, 0, 0)], "right"),
    ]
    projection = fitted(corners(faces), fit=27)

    def arc(k, z):
        angle = pi / 2 * k / rows
        return (1 - a + a * sin(angle), 1 - a + a * cos(angle), z)

    strip = [arc(k, 1) for k in range(rows + 1)] + [arc(k, 0) for k in range(rows, -1, -1)]
    canvas.mesh(projection, faces)
    canvas.face([projection(p) for p in strip], "selected")
    for k in range(1, rows):
        canvas.edge([projection(arc(k, 1)), projection(arc(k, 0))], width=DETAIL)
    canvas.edge([projection(p) for p in strip], closed=True)


def tag_selection_set(canvas: Canvas):
    """Selection Set: a cube with a saved selection (top and right faces)
    and a cyan label tag tied on at its lower left."""
    faces = box((0, 0, 0), (1, 1, 1), top="selected", right="selected")
    projection = fitted(corners(faces), fit=24)
    shift = Projection(projection.unit, (projection.origin[0] + 2.5, projection.origin[1] - 1.5))
    canvas.mesh(shift, faces)
    # A label tag in screen space, pointing at the cube's left face.
    x, y = shift((0.55, 1, 0.3))
    w, h, n = 12.5, 8.0, 3.4
    tag = [(x - w, y), (x - n, y), (x, y + h / 2), (x - n, y + h), (x - w, y + h)]
    tag = [(px + 1.0, py + 1.0) for px, py in tag]
    canvas.face(tag, "new")
    canvas.edge(tag, closed=True)
    canvas.vertex((tag[1][0] - 0.3, tag[0][1] + h / 2), 1.2, "ink")


TAG_ICONS = {
    "scale-offset": (scale_offset, "Scale Offset", "A cube's selected top face shrunk in its plane, the sides leaning in to follow, with inward arrows."),
    "scale-offset-detach": (scale_offset_detach, "Scale Offset (Detach)", "A smaller selected face lifted off the cube's top on new cyan points."),
    "flatten-ngons": (flatten_ngons, "Flatten Ngons", "A selected five-sided polygon; its raised corner comes down onto the plane along a cyan arrow."),
    "tag-material": (tag_material, "Material", "A shaded material ball beside a cube with two selected faces."),
    "tag-part": (tag_part, "Part", "A cube split into two blocks; the right block is selected."),
    "tag-smoothing-group": (tag_smoothing, "Smoothing Group", "A cube whose front edge is rounded into a smooth selected strip."),
    "tag-selection-set": (tag_selection_set, "Selection Set", "A cube with selected faces and a cyan label tag."),
}
