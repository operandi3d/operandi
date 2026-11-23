"""Batch 19: tree-row type icons (Shader Tree, Items, Groups, Mesh Stack, UV
maps, preset lists), in place of single-letter badges (Jason, 2026-11-16).

Flat glyphs drawn for small rows (about 14 px): one bold silhouette per
type, palette colours only, the shared stroke widths. Original Operandi
designs.
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, Point
from .primitives import Canvas


def circle(center: Point, radius: float, samples: int = 40) -> list[Point]:
    cx, cy = center
    return [(cx + radius * cos(2 * pi * i / samples), cy + radius * sin(2 * pi * i / samples))
            for i in range(samples)]


def rounded(x0: float, y0: float, x1: float, y1: float, r: float, samples: int = 6) -> list[Point]:
    """A rounded rectangle as a polygon."""
    out: list[Point] = []
    for cx, cy, a0 in ((x1 - r, y0 + r, -pi / 2), (x1 - r, y1 - r, 0.),
                       (x0 + r, y1 - r, pi / 2), (x0 + r, y0 + r, pi)):
        for i in range(samples + 1):
            a = a0 + (pi / 2) * i / samples
            out.append((cx + r * cos(a), cy + r * sin(a)))
    return out


def folder_shape() -> list[Point]:
    return [(4, 9), (4, 7), (5, 6), (12, 6), (14.5, 9), (27, 9), (28, 10), (28, 25), (27, 26),
            (5, 26), (4, 25)]


def output(canvas: Canvas):
    """Render output: a picture frame with an arrow leaving it."""
    frame = rounded(3.5, 7, 21, 25, 2)
    canvas.face(frame, "right")
    canvas.face([(7, 22), (11.5, 14), (14.5, 18.5), (16.5, 16), (19, 22)], "top")
    canvas.edge(frame, closed=True, width=ACCENT)
    canvas.arrow((17, 16), (28.5, 16), "new")


def shader(canvas: Canvas):
    """Shader: a disc lit on one side and dark on the other."""
    disc = circle((16, 16), 11.5)
    canvas.face(disc, "right")
    lit = [(16 + 11.5 * cos(a), 16 + 11.5 * sin(a))
           for a in (pi * 0.75 + pi * i / 30 for i in range(31))]
    canvas.face(lit, "top")
    canvas.edge(disc, closed=True, width=ACCENT)


def material(canvas: Canvas):
    """Material: a glossy ball with a highlight."""
    ball = circle((16, 16.5), 11.5)
    canvas.face(ball, "left")
    canvas.face(circle((14.5, 18), 8.5), "top")
    canvas.face(circle((11.5, 12), 3.2), "selected")
    canvas.edge(ball, closed=True, width=ACCENT)


def group(canvas: Canvas):
    """Group: three layers stacked."""
    for dy, tone in ((9, "right"), (4.5, "left"), (0, "top")):
        layer = [(16, 5 + dy), (28, 10.5 + dy), (16, 16 + dy), (4, 10.5 + dy)]
        canvas.face(layer, tone)
        canvas.edge(layer, closed=True, width=EDGE)


def texture(canvas: Canvas):
    """Texture layer: a square of four checks."""
    canvas.face([(6, 6), (16, 6), (16, 16), (6, 16)], "top")
    canvas.face([(16, 6), (26, 6), (26, 16), (16, 16)], "right")
    canvas.face([(6, 16), (16, 16), (16, 26), (6, 26)], "right")
    canvas.face([(16, 16), (26, 16), (26, 26), (16, 26)], "top")
    canvas.edge(rounded(6, 6, 26, 26, 2.5), closed=True, width=ACCENT)


def folder(canvas: Canvas):
    """Folder."""
    shape = folder_shape()
    canvas.face(shape, "left")
    canvas.face([(4, 12), (28, 12), (28, 25), (27, 26), (5, 26), (4, 25)], "top")
    canvas.edge(shape, closed=True, width=ACCENT)


def reference(canvas: Canvas):
    """Referenced folder: a folder with a cyan link arrow."""
    folder(canvas)
    canvas.arrow((9, 23), (21, 15), "new")


def node_material(canvas: Canvas):
    """Node material: three nodes joined by links."""
    canvas.edge([(10, 9), (16, 9), (16, 16), (21, 16)], "new", ACCENT)
    canvas.edge([(10, 23), (16, 23), (16, 16)], "new", ACCENT)
    for x0, y0 in ((3, 5), (3, 19), (19, 12)):
        box = rounded(x0, y0, x0 + 10, y0 + 8, 1.6)
        canvas.face(box, "top")
        canvas.edge(box, closed=True, width=EDGE)


def light(canvas: Canvas):
    """Light: a bulb glowing yellow."""
    bulb = [(16 + 8.5 * cos(a), 13 + 8.5 * sin(a))
            for a in (pi * 0.8 - (pi * 1.6) * i / 32 for i in range(33))]
    shape = bulb + [(12, 23)]
    shape = [(12, 23)] + [(16 + 8.5 * cos(a), 13 + 8.5 * sin(a))
                          for a in (pi * 0.7 + (pi * 1.6) * i / 32 for i in range(33))] + [(20, 23)]
    canvas.face(shape, "selected")
    canvas.edge(shape, closed=True, width=ACCENT)
    canvas.face([(12, 23.5), (20, 23.5), (20, 27), (12, 27)], "left")
    canvas.edge([(12, 23.5), (20, 23.5), (20, 27), (12, 27)], closed=True, width=EDGE)
    canvas.edge([(12, 25.3), (20, 25.3)], width=DETAIL)


def environment(canvas: Canvas):
    """Environment: a sky dome over the horizon."""
    dome = [(16 + 12.5 * cos(a), 21 - 12.5 * sin(a)) for a in (pi * i / 30 for i in range(31))]
    canvas.face(dome, "z")
    canvas.face(circle((21, 14), 2.6), "selected")
    canvas.face([(3.5, 21), (28.5, 21), (28.5, 26), (3.5, 26)], "y")
    canvas.edge(dome + [(3.5, 21)], closed=True, width=ACCENT)
    canvas.edge([(3.5, 21), (28.5, 21), (28.5, 26), (3.5, 26)], closed=True, width=EDGE)


def mesh(canvas: Canvas):
    """Mesh item: a small cube."""
    top = [(16, 4.5), (27, 10), (16, 15.5), (5, 10)]
    left = [(5, 10), (16, 15.5), (16, 27.5), (5, 22)]
    right = [(16, 15.5), (27, 10), (27, 22), (16, 27.5)]
    for poly, tone in ((top, "top"), (left, "left"), (right, "right")):
        canvas.face(poly, tone)
    for poly in (top, left, right):
        canvas.edge(poly, closed=True, width=EDGE)


def uv_map(canvas: Canvas):
    """UV map: a square of UV space with a cyan island on it."""
    canvas.face(rounded(6, 6, 26, 26, 2.5), "top")
    for t in (12.3, 19.7):
        canvas.edge([(t, 6), (t, 26)], "left", DETAIL)
        canvas.edge([(6, t), (26, t)], "left", DETAIL)
    island = [(9, 22), (14, 9), (23, 13), (21, 23)]
    canvas.face(island, "new")
    canvas.edge(island, closed=True, width=EDGE)
    canvas.edge(rounded(6, 6, 26, 26, 2.5), closed=True, width=ACCENT)


def effect(canvas: Canvas):
    """Effect (FX): a four-pointed sparkle and a small one."""
    def star(cx, cy, big, small):
        pts = []
        for i in range(8):
            r = big if i % 2 == 0 else small
            a = -pi / 2 + pi * i / 4
            pts.append((cx + r * cos(a), cy + r * sin(a)))
        return pts
    s1 = star(14, 17, 11, 3.2)
    s2 = star(24.5, 8, 5, 1.6)
    canvas.face(s1, "selected")
    canvas.edge(s1, closed=True, width=ACCENT)
    canvas.face(s2, "selected")
    canvas.edge(s2, closed=True, width=EDGE)


def _almond(x0: float, x1: float, half: float, steps: int = 24) -> list[Point]:
    """An almond (eye shape) from x0 to x1, `half` tall above and below 16."""
    w = x1 - x0
    upper = [(x0 + w * t, 16 - half * sin(pi * t)) for t in (i / steps for i in range(steps + 1))]
    lower = [(x1 - w * t, 16 + half * sin(pi * t)) for t in (i / steps for i in range(1, steps))]
    return upper + lower


def _ring(outer: list[Point], inner: list[Point]) -> list[Point]:
    """A solid band between two closed outlines, as one polygon (the inner
    one walked backwards, joined by a seam)."""
    return outer + [outer[0], inner[0]] + list(reversed(inner[1:])) + [inner[0]]


def eye_mono(canvas: Canvas):
    """List visibility, dark part (N16b): a bold almond band and a pupil with
    a glint hole, drawn light so the app tints it; eye-mono-light shows
    through the band's inside and the glint."""
    outer = _almond(2.5, 29.5, 11.5)
    inner = _almond(6.5, 25.5, 7.2)
    canvas.face(_ring(outer, inner), "top")
    pupil = circle((16, 16), 5.2)
    glint = circle((17.8, 14.2), 1.7, 16)
    canvas.face(_ring(pupil, glint), "top")


def eye_closed_mono(canvas: Canvas):
    """List visibility off, single tone (N19): a closed lid, a bold arc
    curving down with short lashes below it; tinted by the app."""
    steps = 24
    outer = [(2.5 + 27 * t, 13 + 9 * sin(pi * t)) for t in (i / steps for i in range(steps + 1))]
    inner = [(6 + 20 * t, 13 + 5.6 * sin(pi * t)) for t in (i / steps for i in range(steps + 1))]
    canvas.face(outer + list(reversed(inner)), "top")
    for t in (0.2, 0.4, 0.6, 0.8):
        x, y = 2.5 + 27 * t, 13 + 9 * sin(pi * t)
        dx = (t - 0.5) * 6
        lash = [(x - 1.3, y - 0.5), (x + 1.3, y - 0.5), (x + 1.1 + dx, y + 4.5), (x - 1.1 + dx, y + 4.5)]
        canvas.face(lash, "top")


def eye_mono_light(canvas: Canvas):
    """List visibility, light part (N16b): the white of the eye, drawn under
    eye-mono so the eye reads in two tones on any grey."""
    canvas.face(_almond(5.5, 26.5, 8.2), "top")


def mesh_edit_mono(canvas: Canvas):
    """Editable (active) mesh, single tone (N18): a bold square cage split
    into four cells, with a solid point at each corner; drawn light so the
    app tints it (orange when the mesh is active)."""
    outer = rounded(5, 5, 27, 27, 1.5)
    inner = rounded(8, 8, 24, 24, 0.5)
    canvas.face(outer + [outer[0], inner[0]] + list(reversed(inner[1:])) + [inner[0]], "top")
    canvas.face([(14.5, 8), (17.5, 8), (17.5, 24), (14.5, 24)], "top")
    canvas.face([(8, 14.5), (24, 14.5), (24, 17.5), (8, 17.5)], "top")
    for x, y in ((6.5, 6.5), (25.5, 6.5), (6.5, 25.5), (25.5, 25.5), (16, 16)):
        canvas.face(circle((x, y), 3.2, 20), "top")


def disclosure_mono(canvas: Canvas):
    """List disclosure, single tone: a solid down-pointing triangle, drawn
    light so the app can tint it."""
    tri = [(7, 10), (25, 10), (16, 23)]
    canvas.face(tri, "top")
    canvas.edge(tri, "top", ACCENT, closed=True)


TREE_ICONS = {
    "eye-mono": (eye_mono, "Visible (list)", "A bold almond band and a pupil with a glint hole; the dark part of the list eye, tinted by the app."),
    "eye-closed-mono": (eye_closed_mono, "Hidden (list)", "A closed lid with short lashes; tinted by the app."),
    "eye-mono-light": (eye_mono_light, "Visible (list, light)", "The white of the list eye, drawn under eye-mono."),
    "mesh-edit-mono": (mesh_edit_mono, "Editable Mesh (list)", "A bold square cage of four cells with corner points; tinted by the app."),
    "disclosure-mono": (disclosure_mono, "Disclosure (list)", "A single-tone down-pointing triangle, tinted by the app."),
    "tree-output": (output, "Render Output", "A picture frame with an arrow leaving it: a render output layer."),
    "tree-shader": (shader, "Shader", "A disc lit on one side: a shader layer."),
    "tree-material": (material, "Material", "A glossy ball with a highlight: a material layer."),
    "tree-group": (group, "Group", "Three stacked layers: a group."),
    "tree-texture": (texture, "Texture", "A square of four checks: a texture layer."),
    "tree-folder": (folder, "Folder", "A folder."),
    "tree-reference": (reference, "Reference", "A folder with a cyan link arrow: a referenced folder."),
    "tree-node-material": (node_material, "Node Material", "Three nodes joined by cyan links."),
    "tree-light": (light, "Light", "A yellow light bulb."),
    "tree-environment": (environment, "Environment", "A blue sky dome over a green horizon with a sun."),
    "tree-mesh": (mesh, "Mesh", "A small cube: a mesh item."),
    "tree-uv-map": (uv_map, "UV Map", "A square of UV space with a cyan island."),
    "tree-effect": (effect, "Effect", "A yellow sparkle: an effect."),
}
