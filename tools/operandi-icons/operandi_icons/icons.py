"""Operation geometry, expressed as faces and edges of a shared unit cube."""

from .conventions import CENTER, DETAIL, GIZMO, Projection, UNIT
from .primitives import FILL, Canvas, fit_to_canvas
from . import solids

# Back top, right top, front top, left top; their lower counterparts.
A, B, C, D = (0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)
E, F, G = (1, 1, 0), (1, 0, 0), (0, 1, 0)
TOP, LEFT, RIGHT = [A, B, C, D], [D, C, E, G], [B, C, E, F]


def cube(canvas: Canvas, projection: Projection = Projection(), width=None):
    kwargs = {} if width is None else {"width": width}
    canvas.mesh(projection, [(TOP, "top"), (LEFT, "left"), (RIGHT, "right")],
                **kwargs)


def extrude(canvas: Canvas):
    distance = 0.55
    unit = UNIT * 0.9
    # Center the elongated bounding box, retaining the approved optical fit.
    shift = Projection(unit)((distance / 2, 0, 0))
    projection = Projection(unit, (2 * CENTER[0] - shift[0],
                                   2 * CENTER[1] - shift[1]))
    b, c, e, f = [(x + distance, y, z) for x, y, z in (B, C, E, F)]
    canvas.mesh(projection, [(TOP, "top"), (LEFT, "left"),
                            ([B, b, c, C], "top"),
                            ([C, c, e, E], "left"),
                            ([b, c, e, f], "selected")])


def bevel(canvas: Canvas):
    amount = 0.23
    # Chamfer the top/right edge along Y, introducing a visible cyan strip.
    upper_back, upper_front = (1 - amount, 0, 1), (1 - amount, 1, 1)
    lower_back, lower_front = (1, 0, 1 - amount), (1, 1, 1 - amount)
    canvas.mesh(Projection(), [
        ([A, upper_back, upper_front, D], "top"),
        ([D, upper_front, lower_front, E, G], "left"),
        ([lower_back, lower_front, E, F], "right"),
        ([upper_back, lower_back, lower_front, upper_front], "new"),
    ])


def inset(canvas: Canvas):
    projection = Projection()
    cube(canvas, projection)
    margin = 0.23
    inner = [(margin, margin, 1), (1 - margin, margin, 1),
             (1 - margin, 1 - margin, 1), (margin, 1 - margin, 1)]
    canvas.highlight_face(projection, inner)
    for outer, inside in zip(TOP, inner):
        canvas.edge([projection(outer), projection(inside)])


def loop_cut(canvas: Canvas):
    projection = Projection()
    cube(canvas, projection)
    canvas.highlight_edge(projection, [(0, 1, 0.5), (1, 1, 0.5), (1, 0, 0.5)])


def move(canvas: Canvas):
    projection = Projection(UNIT * 0.98 / 1.2)
    cube(canvas, projection)
    origin = projection(C)
    extent = 27 / 21.5
    for color, endpoint in [("x", (1 + extent, 1, 1)),
                            ("y", (1, 1 + extent, 1)),
                            ("z", (1, 1, 1 + extent))]:
        canvas.arrow(origin, projection(endpoint), color,
                     length=2.7, half_width=1.45, width=GIZMO)
    canvas.vertex(origin)


def rotate(canvas: Canvas):
    radius = 12.1
    canvas.guide(CENTER, radius, DETAIL)
    cube(canvas, Projection(UNIT * 1.1 * 0.58 / 1.2), DETAIL)
    # Approved cubic silhouettes, parameterized relative to the guide circle.
    # These are screen-space overlays, not a second projection for mesh geometry.
    curves = {
        "x": [(-12, 12.5), (-22, -3.5), (-15, -21.5), (-1, -21.5)],
        "y": [(6, -17.5), (19, -6.5), (18, 7.5), (7, 16.5)],
        "z": [(-20, 2.5), (-12, 12.5), (10, 12.5), (20, 0.5)],
    }
    for color, offsets in curves.items():
        points = [(CENTER[0] + x / 22 * radius, CENTER[1] + y / 22 * radius)
                  for x, y in offsets]
        canvas.cubic_arrow(points, color, length=2.6, half_width=1.4, width=GIZMO)


# Primitives. Each solid is centered on the unit cube's center, (0.5, 0.5,
# 0.5), so all five share one optical center; sizes are matched by eye to
# the cube's silhouette.
MIDDLE = (0.5, 0.5, 0.5)


def primitive_cube(canvas: Canvas):
    cube(canvas)


def sphere(canvas: Canvas):
    radii = (0.74, 0.74, 0.74)
    projection = solids.fitted(solids.ellipsoid_points(MIDDLE, radii))
    solids.ellipsoid(canvas, projection, MIDDLE, radii)


def ellipsoid(canvas: Canvas):
    radii = (0.86, 0.86, 0.46)
    projection = solids.fitted(solids.ellipsoid_points(MIDDLE, radii))
    solids.ellipsoid(canvas, projection, MIDDLE, radii)


def cylinder(canvas: Canvas):
    radius, height = 0.62, 1.2
    base = (0.5, 0.5, 0.5 - height / 2)
    rings = solids.ring(base, radius) + solids.ring((0.5, 0.5, base[2] + height), radius)
    solids.cylinder(canvas, solids.fitted(rings), base, radius, height)


def cone(canvas: Canvas):
    radius, height = 0.66, 1.45
    base = (0.5, 0.5, 0.5 - height / 2)
    points = solids.ring(base, radius) + [(0.5, 0.5, base[2] + height)]
    solids.cone(canvas, solids.fitted(points), base, radius, height)


ICONS = {
    "cube": (primitive_cube, "Cube", "Isometric cube with light top and shaded sides."),
    "sphere": (sphere, "Sphere", "Sphere shaded with nested tones, lit from the upper left."),
    "ellipsoid": (ellipsoid, "Ellipsoid", "Flattened sphere shaded with nested tones, lit from the upper left."),
    "cylinder": (cylinder, "Cylinder", "Upright cylinder with a light top cap."),
    "cone": (cone, "Cone", "Upright cone with shaded left and right sides."),
    "extrude": (extrude, "Extrude", "Orange end face extended from an isometric cube."),
    "bevel": (bevel, "Bevel", "Cyan chamfer strip along the cube's upper right edge."),
    "inset": (inset, "Inset", "Orange inner top face joined to the original corners."),
    "loop-cut": (loop_cut, "Loop Cut", "Cyan edge loop across the cube's visible sides."),
    "move": (move, "Move", "Red X, green Y and blue Z arrows from a common cube vertex."),
    "rotate": (rotate, "Rotate", "Three colored curved arrows around a miniature cube."),
}


# Batches are kept in their own modules and merged into the one registry.
from .polygon_icons import POLYGON_ICONS  # noqa: E402
from .edge_icons import EDGE_ICONS  # noqa: E402
from .vertex_icons import VERTEX_ICONS  # noqa: E402
from .ui_icons import UI_ICONS  # noqa: E402
from .shape_icons import SHAPE_ICONS  # noqa: E402
from .create_icons import CREATE_ICONS  # noqa: E402
from .deform_icons import DEFORM_ICONS  # noqa: E402
from .tag_icons import TAG_ICONS  # noqa: E402
from .path_slice_icons import PATH_SLICE_ICONS  # noqa: E402
from .subdivide_icons import SUBDIVIDE_ICONS  # noqa: E402
from .sculpt_duplicate_icons import DUPLICATE_ICONS, SCULPT_ICONS  # noqa: E402
from .environment_icons import ENVIRONMENT_ICONS  # noqa: E402
from .snaps_icons import SNAPS_ICONS  # noqa: E402
from .sculpt_toolbox_icons import SCULPT_TOOLBOX_ICONS  # noqa: E402
from .boolean_icons import BOOLEAN_ICONS  # noqa: E402
from .vertex_map_icons import VERTEX_MAP_ICONS  # noqa: E402
from .solid_sketch_icons import SOLID_SKETCH_ICONS  # noqa: E402
from .tree_icons import TREE_ICONS  # noqa: E402
from .path_duplicate_icons import PATH_DUPLICATE_ICONS  # noqa: E402
from .uv_icons import UV_ICONS  # noqa: E402
from .paint_icons import PAINT_ICONS  # noqa: E402
from .topology_icons import TOPOLOGY_ICONS  # noqa: E402

for batch in (POLYGON_ICONS, EDGE_ICONS, VERTEX_ICONS, UI_ICONS, SHAPE_ICONS, CREATE_ICONS,
              DEFORM_ICONS, TAG_ICONS, SUBDIVIDE_ICONS, PATH_SLICE_ICONS, SCULPT_ICONS,
              DUPLICATE_ICONS, ENVIRONMENT_ICONS, SNAPS_ICONS, SCULPT_TOOLBOX_ICONS,
              BOOLEAN_ICONS, VERTEX_MAP_ICONS, SOLID_SKETCH_ICONS, TREE_ICONS,
              PATH_DUPLICATE_ICONS, UV_ICONS, PAINT_ICONS, TOPOLOGY_ICONS):
    ICONS.update(batch)


# Primitives are whole solids with no gizmo or cue around them, so at full
# fill they look heavier than the tool icons beside them (Jason, 2026-11-01).
FILLS = {name: 26.0 for name in ("cube", "sphere", "ellipsoid", "cylinder", "cone",
                                    "torus", "capsule", "tube")}


def drawing(name: str):
    builder, label, description = ICONS[name]
    return fit_to_canvas(builder, label, description, FILLS.get(name, FILL)).drawing
