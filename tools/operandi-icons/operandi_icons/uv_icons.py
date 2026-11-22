"""Batch 22: the UV tab (UV tools finish, 2026-11-18).

UV icons are flat, as the UV view is: the 0-1 tile is a light square with
an ink outline and a faint grid (the UV Map item's tile), UV islands on it
are cyan, selected UVs yellow-orange. U is red and V green, as X and Y are
(the UV view's own handles use teal and yellow, but icons keep the house
palette). Each tool adds one cue: an arrow, a line the UVs meet, a net.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, GIZMO, Point
from .create_icons import backed, backed_arrow, knot
from .primitives import Canvas
from .tree_icons import rounded

TAU = 2 * pi
LO, HI = 5.0, 27.0


def tile(canvas: Canvas, x0: float = LO, y0: float = LO, x1: float = HI, y1: float = HI,
         grid: bool = True):
    """The UV tile: a light square (rounded corners, so the canvas corners
    stay clear) with a faint grid and an ink outline."""
    square = rounded(x0, y0, x1, y1, 2.0)
    canvas.face(square, "top")
    if grid:
        for k in (1, 2):
            t = k / 3
            x = x0 + (x1 - x0) * t
            y = y0 + (y1 - y0) * t
            canvas.edge([(x, y0), (x, y1)], "left", DETAIL)
            canvas.edge([(x0, y), (x1, y)], "left", DETAIL)
    canvas.edge(square, closed=True, width=ACCENT)


def island(canvas: Canvas, points: list[Point], color: str = "new"):
    canvas.face(points, color)
    canvas.edge(points, closed=True, width=EDGE)


def rect(x0: float, y0: float, x1: float, y1: float) -> list[Point]:
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def turned(points: list[Point], centre: Point, angle: float) -> list[Point]:
    c, s = cos(angle), sin(angle)
    return [(centre[0] + (x - centre[0]) * c - (y - centre[1]) * s,
             centre[1] + (x - centre[0]) * s + (y - centre[1]) * c) for x, y in points]


def two_way(canvas: Canvas, a: Point, b: Point, color: str):
    """A double-headed arrow."""
    canvas.arrow(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), b, color, width=GIZMO)
    canvas.arrow(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), a, color, width=GIZMO)


def small_cube(canvas: Canvas, x: float, y: float, s: float):
    """A small isometric cube with its top's back corner at (x, y)."""
    h = s * 0.5
    top = [(x, y), (x + s, y + h), (x, y + 2 * h), (x - s, y + h)]
    left = [(x - s, y + h), (x, y + 2 * h), (x, y + 2 * h + s), (x - s, y + h + s)]
    right = [(x, y + 2 * h), (x + s, y + h), (x + s, y + h + s), (x, y + 2 * h + s)]
    for poly, tone in ((top, "top"), (left, "left"), (right, "right")):
        canvas.face(poly, tone)
    for poly in (top, left, right):
        canvas.edge(poly, closed=True, width=EDGE)


def net(canvas: Canvas, x: float, y: float, s: float):
    """A cube's net: four squares in a row, one above and one below the
    second."""
    for i in range(4):
        island(canvas, rect(x + i * s, y + s, x + (i + 1) * s, y + 2 * s))
    island(canvas, rect(x + s, y, x + 2 * s, y + s))
    island(canvas, rect(x + s, y + 2 * s, x + 2 * s, y + 3 * s))


# ------------------------------------------------------------- Create

def uv_project(canvas: Canvas):
    """Project: a cube above the tile casts its shape down onto it as a
    cyan island."""
    tile(canvas, 11, 13, 29, 29)
    island(canvas, [(15, 18), (23, 16.5), (25.5, 22.5), (18, 25)])
    small_cube(canvas, 8, 2, 6)
    backed_arrow(canvas, (12, 15.5), (16.5, 20))


def uv_project_view(canvas: Canvas):
    """Project from View: a viewfinder's corners round a cyan island on
    the tile, as the view sees it."""
    tile(canvas)
    island(canvas, [(11, 20), (16, 11), (21, 20)])
    for (x, y, dx, dy) in ((8, 8, 1, 1), (24, 8, -1, 1), (8, 24, 1, -1), (24, 24, -1, -1)):
        backed(canvas, [(x, y + 4 * dy), (x, y), (x + 4 * dx, y)], "selected")


def uv_unwrap(canvas: Canvas):
    """Unwrap: a cube's net laid flat in cyan on the tile."""
    tile(canvas, grid=False)
    net(canvas, 7, 9.5, 4.5)


def _wave(canvas: Canvas, x0: float, x1: float, y: float):
    """Relax's cue: a blue swept arrow over the island."""
    canvas.cubic_arrow([(x0, y), (x0 + (x1 - x0) * 0.35, y - 4), (x0 + (x1 - x0) * 0.65, y + 4),
                        (x1, y)], "z", width=GIZMO)


def uv_relax(canvas: Canvas):
    """Relax: a cyan island with even, rounded sides and a blue smoothing
    sweep over it."""
    tile(canvas)
    blob = [(16 + 8 * cos(a) + 0.6 * cos(3 * a), 19 + 5.5 * sin(a)) for a in (TAU * i / 24 for i in range(24))]
    island(canvas, blob)
    _wave(canvas, 8, 25, 10)


def uv_pelt(canvas: Canvas):
    """Unwrap & Relax: the cube's net with Relax's blue sweep."""
    tile(canvas, grid=False)
    net(canvas, 8, 13, 3.8)
    _wave(canvas, 8, 25, 9)


def uv_pack(canvas: Canvas):
    """Pack UVs: four islands of different sizes packed tight in the tile."""
    tile(canvas, grid=False)
    for r in (rect(7, 7, 17, 17), rect(18, 7, 25, 13), rect(18, 14, 25, 25), rect(7, 18, 17, 25)):
        island(canvas, r)


def uv_atlas(canvas: Canvas):
    """Atlas Unwrap: many small charts scattered over the tile."""
    tile(canvas, grid=False)
    charts = [
        [(7, 7), (13, 7), (12, 12), (7, 12)],
        [(15, 7), (25, 7), (25, 11), (16, 12)],
        [(7, 14), (11, 14), (11, 25), (7, 25)],
        [(13, 14), (19, 14), (19, 19), (13, 20)],
        [(21, 13), (25, 13), (25, 19), (21, 18)],
        [(13, 22), (25, 21), (25, 25), (13, 25)],
    ]
    for c in charts:
        island(canvas, c)


def uv_rectangle(canvas: Canvas):
    """UV Rectangle: an island straightened into a rectangle with an even
    grid inside."""
    tile(canvas, grid=False)
    x0, y0, x1, y1 = 8, 10, 24, 22
    island(canvas, rect(x0, y0, x1, y1))
    for k in (1, 2):
        x = x0 + (x1 - x0) * k / 3
        canvas.edge([(x, y0), (x, y1)], "ink", DETAIL)
    y = (y0 + y1) / 2
    canvas.edge([(x0, y), (x1, y)], "ink", DETAIL)


# ------------------------------------------------------------- Align and Pack

def uv_fit(canvas: Canvas):
    """Fit UVs: a small island with arrows out to the tile's corners."""
    tile(canvas, grid=False)
    island(canvas, rect(13, 13, 19, 19))
    for (a, b) in (((12, 12), (7, 7)), ((20, 12), (25, 7)), ((12, 20), (7, 25)), ((20, 20), (25, 25))):
        canvas.arrow(a, b, "ink", width=ACCENT)


def _flip(canvas: Canvas, horizontal: bool):
    tile(canvas, grid=False)
    half = [(16, 8), (16, 19), (10, 19), (12.5, 8)]
    other = [(16, 8), (16, 19), (22, 19), (19.5, 8)]
    if not horizontal:
        half = [(y, x) for x, y in half]
        other = [(y, x) for x, y in other]
    island(canvas, half)
    island(canvas, other, "selected")
    if horizontal:
        two_way(canvas, (8.5, 23), (23.5, 23), "x")
    else:
        two_way(canvas, (23, 8.5), (23, 23.5), "y")


def uv_flip_u(canvas: Canvas):
    """Flip U: an island turned left to right about its own middle (its two
    halves cyan and yellow-orange), a red double arrow across."""
    _flip(canvas, True)


def uv_flip_v(canvas: Canvas):
    """Flip V: the same, top to bottom, with a green double arrow."""
    _flip(canvas, False)


def _mirror(canvas: Canvas, horizontal: bool):
    tile(canvas, grid=False)
    a = [(7, 11), (13, 9), (13, 22), (9, 22)]
    b = [(32 - x, y) for x, y in a]
    line = [(16, 6), (16, 26)]
    if not horizontal:
        a = [(y, x) for x, y in a]
        b = [(y, x) for x, y in b]
        line = [(y, x) for x, y in line]
    island(canvas, a)
    island(canvas, b, "selected")
    canvas.edge(line, "x" if horizontal else "y", GIZMO)


def uv_mirror_u(canvas: Canvas):
    """Mirror U: an island and its mirror image across a red line."""
    _mirror(canvas, True)


def uv_mirror_v(canvas: Canvas):
    """Mirror V: the same across a green line."""
    _mirror(canvas, False)


def _align(canvas: Canvas, side: str):
    """UV points flattened onto one side, with the arrow pointing there."""
    tile(canvas, grid=False)
    dots = [(10, 0), (16, 0), (22, 0)]
    color = "y" if side in ("up", "down") else "x"
    if side == "up":
        line, pts, arrow = [(7, 9), (25, 9)], [(x, 9) for x, _ in dots], ((16, 22), (16, 13))
    elif side == "down":
        line, pts, arrow = [(7, 23), (25, 23)], [(x, 23) for x, _ in dots], ((16, 10), (16, 19))
    elif side == "left":
        line, pts, arrow = [(9, 7), (9, 25)], [(9, x) for x, _ in dots], ((22, 16), (13, 16))
    else:
        line, pts, arrow = [(23, 7), (23, 25)], [(23, x) for x, _ in dots], ((10, 16), (19, 16))
    canvas.edge(line, color, GIZMO)
    for p in pts:
        knot(canvas, p, 1.9, "selected")
    canvas.arrow(*arrow, color, width=GIZMO)


def uv_align_up(canvas: Canvas):
    """Align Up: three UV points set on one top line, a green arrow up."""
    _align(canvas, "up")


def uv_align_down(canvas: Canvas):
    """Align Down: the points on one bottom line, a green arrow down."""
    _align(canvas, "down")


def uv_align_left(canvas: Canvas):
    """Align Left: the points on one left line, a red arrow left."""
    _align(canvas, "left")


def uv_align_right(canvas: Canvas):
    """Align Right: the points on one right line, a red arrow right."""
    _align(canvas, "right")


def uv_quick_align(canvas: Canvas):
    """Quick Align: a tilted island turned so its yellow edge lies along a
    red U line, a blue turn arrow."""
    tile(canvas, grid=False)
    body = turned(rect(9, 14, 20, 20), (14.5, 20), -0.32)
    island(canvas, body)
    backed(canvas, [body[3], body[2]], "selected")
    canvas.edge([(7, 24.5), (25, 24.5)], "x", GIZMO)
    canvas.cubic_arrow([(24.5, 12), (26.5, 15), (26.5, 19), (24.5, 22)], "z", width=GIZMO)


def uv_island_align(canvas: Canvas):
    """Island Align: three islands of different widths with their left
    sides on one red line."""
    tile(canvas, grid=False)
    for (w, y) in ((12, 8), (6, 14), (9, 20)):
        island(canvas, rect(10, y, 10 + w, y + 4.5))
    canvas.edge([(10, 6), (10, 26)], "x", GIZMO)


def uv_orient(canvas: Canvas):
    """Orient Pieces: a turned island's ghost outline and the island set
    square, a blue turn arrow between them."""
    tile(canvas, grid=False)
    ghost = turned(rect(9, 12, 23, 20), (16, 16), 0.5)
    canvas.edge(ghost, "left", ACCENT, closed=True)
    island(canvas, rect(9, 12.5, 23, 19.5))
    canvas.cubic_arrow([(22, 7.5), (25, 8.5), (26.5, 10.5), (26, 13)], "z", width=GIZMO)


def uv_to_origin(canvas: Canvas):
    """To Origin: an island in the tile's upper right with a cyan arrow to
    the yellow-orange dot on the tile's lower-left corner."""
    tile(canvas)
    island(canvas, rect(17, 8, 24, 15))
    backed_arrow(canvas, (16, 16), (9, 23))
    knot(canvas, (5, 27), 2.3, "selected")


def uv_texel_density(canvas: Canvas):
    """Texel Density: two islands with checks of the same size, one large
    and one small, so the checks match across them."""
    tile(canvas, grid=False)
    for (x0, y0, n) in ((7, 7, 3), (19, 17, 2)):
        s = 3.0
        for i in range(n):
            for j in range(n):
                canvas.face(rect(x0 + i * s, y0 + j * s, x0 + (i + 1) * s, y0 + (j + 1) * s),
                            "new" if (i + j) % 2 == 0 else "ink")
        canvas.edge(rect(x0, y0, x0 + n * s, y0 + n * s), closed=True, width=EDGE)


# ------------------------------------------------------------- Split and Sew

def uv_split(canvas: Canvas):
    """Split: an island cut in two along a yellow seam, the halves pushed
    apart by cyan arrows."""
    tile(canvas, grid=False)
    island(canvas, rect(7, 9, 14, 23))
    island(canvas, rect(18, 9, 25, 23))
    backed(canvas, [(14, 9), (14, 23)], "selected")
    backed(canvas, [(18, 9), (18, 23)], "selected")
    canvas.arrow((11, 16), (8.5, 16), "ink", width=ACCENT)
    canvas.arrow((21, 16), (23.5, 16), "ink", width=ACCENT)


def uv_sew(canvas: Canvas):
    """Sew: two islands joined along a yellow seam, stitched across it."""
    tile(canvas, grid=False)
    island(canvas, rect(7, 9, 16, 23))
    island(canvas, rect(16, 9, 25, 23))
    backed(canvas, [(16, 9), (16, 23)], "selected")
    for y in (12, 16, 20):
        canvas.edge([(14, y - 1.2), (18, y + 1.2)], "ink", ACCENT)


def uv_move_sew(canvas: Canvas):
    """Move and Sew: an island carried by a cyan arrow to meet the yellow
    seam of another."""
    tile(canvas, grid=False)
    island(canvas, rect(7, 13, 15, 25))
    backed(canvas, [(15, 13), (15, 25)], "selected")
    island(canvas, turned(rect(18, 6, 25, 14), (21.5, 10), 0.35))
    backed_arrow(canvas, (20, 15), (17, 19.5))


def uv_select_border(canvas: Canvas):
    """Select Border: an island's whole outline selected."""
    tile(canvas)
    shape = [(8, 21), (11, 9), (21, 8), (24, 17), (17, 24)]
    canvas.face(shape, "new")
    backed(canvas, shape, "selected", closed=True)


UV_ICONS = {
    "uv-project": (uv_project, "Project", "A cube above the UV tile casts a cyan island down onto it."),
    "uv-project-view": (uv_project_view, "Project from View", "Viewfinder corners round a cyan island on the UV tile."),
    "uv-unwrap": (uv_unwrap, "Unwrap", "A cube's net laid flat in cyan on the UV tile."),
    "uv-relax": (uv_relax, "Relax", "A rounded cyan island on the UV tile under a blue smoothing sweep."),
    "uv-pelt": (uv_pelt, "Unwrap & Relax", "A cube's net on the UV tile under Relax's blue sweep."),
    "uv-pack": (uv_pack, "Pack UVs", "Four cyan islands of different sizes packed tight in the UV tile."),
    "uv-atlas": (uv_atlas, "Atlas Unwrap", "Many small cyan charts scattered over the UV tile."),
    "uv-rectangle": (uv_rectangle, "UV Rectangle", "A cyan rectangle with an even grid inside, on the UV tile."),
    "uv-fit": (uv_fit, "Fit UVs", "A small cyan island with arrows out to the UV tile's corners."),
    "uv-flip-u": (uv_flip_u, "Flip U", "An island flipped left to right about its middle, a red double arrow across."),
    "uv-flip-v": (uv_flip_v, "Flip V", "An island flipped top to bottom about its middle, a green double arrow."),
    "uv-mirror-u": (uv_mirror_u, "Mirror U", "An island and its mirror image across a red line."),
    "uv-mirror-v": (uv_mirror_v, "Mirror V", "An island and its mirror image across a green line."),
    "uv-align-up": (uv_align_up, "Align Up", "Three selected UV points on one top line, a green arrow up."),
    "uv-align-down": (uv_align_down, "Align Down", "Three selected UV points on one bottom line, a green arrow down."),
    "uv-align-left": (uv_align_left, "Align Left", "Three selected UV points on one left line, a red arrow left."),
    "uv-align-right": (uv_align_right, "Align Right", "Three selected UV points on one right line, a red arrow right."),
    "uv-quick-align": (uv_quick_align, "Quick Align", "A tilted island turned so its selected edge lies along a red line."),
    "uv-island-align": (uv_island_align, "Island Align", "Three islands with their left sides on one red line."),
    "uv-orient": (uv_orient, "Orient Pieces", "A turned island's ghost and the island set square, with a blue turn arrow."),
    "uv-to-origin": (uv_to_origin, "To Origin", "An island with a cyan arrow to the dot on the UV tile's lower-left corner."),
    "uv-texel-density": (uv_texel_density, "Texel Density", "A large and a small island with checks of the same size."),
    "uv-split": (uv_split, "Split UVs", "An island cut along a selected seam, its halves pushed apart."),
    "uv-sew": (uv_sew, "Sew UVs", "Two islands joined along a selected seam, stitched across it."),
    "uv-move-sew": (uv_move_sew, "Move and Sew", "An island carried by a cyan arrow to meet another's selected seam."),
    "uv-select-border": (uv_select_border, "Select Border", "A cyan island with its whole outline selected."),
}
