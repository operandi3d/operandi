"""Batch 20: 3D painting (painting builds only): the Paint tab's tools,
tips, Nozzle and Save Brush Preset, the four inks, the image commands,
the Shader Tree's paint-target badge, the Images list's Photoshop layer
marker and the Paint layout.

Original symbols drawn flat, in the cube's tones with ink outlines, as
the list and UI icons are. The cue colour is cyan ("new"): the paint being
laid down, always as a cyan line on an ink backing or a light dot in an
ink ring, so it reads on the dark theme as well as the light one. The
X/Y/Z red, green and blue appear only where colour itself is the subject
(colour textures, random ink, the palette).
"""

from math import cos, pi, sin

from .conventions import ACCENT, DETAIL, EDGE, Point
from .create_icons import backed, backed_arrow, knot, sampled
from .primitives import Canvas

TAU = 2 * pi


# ------------------------------------------------------------- helpers

def circle(c: Point, r: float, k: int = 40, squash: float = 1.0) -> list[Point]:
    return [(c[0] + r * cos(TAU * i / k), c[1] + r * squash * sin(TAU * i / k)) for i in range(k)]


def rect(x0: float, y0: float, x1: float, y1: float) -> list[Point]:
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def card(x0: float, y0: float, x1: float, y1: float, c: float = 1.6) -> list[Point]:
    """A sheet with its corners trimmed, so square pictures keep the
    canvas corners clear."""
    return [(x0 + c, y0), (x1 - c, y0), (x1, y0 + c), (x1, y1 - c), (x1 - c, y1),
            (x0 + c, y1), (x0, y1 - c), (x0, y0 + c)]


def turned(points: list[Point], about: Point, degrees: float) -> list[Point]:
    a = degrees * pi / 180
    c, s = cos(a), sin(a)
    return [(about[0] + (x - about[0]) * c - (y - about[1]) * s,
             about[1] + (x - about[0]) * s + (y - about[1]) * c) for x, y in points]


def solid(canvas: Canvas, points: list[Point], color: str, width: float = EDGE):
    """A filled shape with an ink outline."""
    canvas.face(points, color)
    canvas.edge(points, width=width, closed=True)


def stroke_mark(canvas: Canvas, controls: list[Point]):
    """The paint a tool lays: a cyan curve on an ink backing."""
    backed(canvas, sampled(controls, 24))


def sheet(canvas: Canvas, x0: float, y0: float, x1: float, y1: float, color: str = "top"):
    solid(canvas, rect(x0, y0, x1, y1), color)


def plus(canvas: Canvas, c: Point, r: float = 3.2):
    backed(canvas, [(c[0] - r, c[1]), (c[0] + r, c[1])])
    backed(canvas, [(c[0], c[1] - r), (c[0], c[1] + r)])


def bottle(canvas: Canvas):
    """An ink bottle: a squat body, a neck and a cap."""
    body = [(7, 13), (25, 13), (27, 16), (27, 28), (5, 28), (5, 16)]
    solid(canvas, body, "top")
    solid(canvas, rect(11, 8, 21, 13), "left")
    solid(canvas, rect(9.5, 4, 22.5, 8), "right")


# ------------------------------------------------------------- tools

def airbrush(canvas: Canvas):
    """Airbrush: a spray can whose nozzle sends a fan of cyan dots."""
    solid(canvas, rect(4, 11, 13, 29), "top")
    solid(canvas, rect(5.5, 7, 11.5, 11), "left")
    solid(canvas, rect(7, 4.5, 10, 7), "right")
    canvas.edge([(4, 16), (13, 16)], width=DETAIL)
    for x, y in ((17, 6), (20, 9), (23, 5.5), (19, 13), (24, 11), (27, 8), (22, 15), (27, 14)):
        knot(canvas, (x, y), 1.5, "new")


def paintbrush(canvas: Canvas):
    """Paintbrush: a brush laid on a slant over its cyan stroke."""
    stroke_mark(canvas, [(3, 28), (9, 23), (13, 29), (20, 24)])
    handle = turned(rect(15, 2.5, 19, 15), (17, 15), 35)
    ferrule = turned(rect(14.5, 15, 19.5, 19), (17, 15), 35)
    tip = turned([(14.5, 19), (19.5, 19), (17, 25)], (17, 15), 35)
    solid(canvas, handle, "right")
    solid(canvas, ferrule, "top")
    solid(canvas, tip, "left")


def eraser(canvas: Canvas):
    """Eraser: a two-tone block rubbing out a cyan stroke (the stroke
    stops where it has passed)."""
    stroke_mark(canvas, [(2, 27), (5, 24), (8, 27), (11, 25)])
    block = turned(rect(10, 10, 28, 19), (19, 14.5), -35)
    sleeve = turned(rect(19, 10, 28, 19), (19, 14.5), -35)
    solid(canvas, block, "top")
    solid(canvas, sleeve, "left")


def smudge(canvas: Canvas):
    """Smudge: a round blot pulled out into a smear along an arrow."""
    blot = circle((9, 18), 5.5)
    canvas.face([(9, 12.5), (22, 15), (22, 21), (9, 23.5)], "left")
    solid(canvas, blot, "left")
    canvas.edge([(9, 12.5), (22, 15)], width=EDGE)
    canvas.edge([(9, 23.5), (22, 21)], width=EDGE)
    for y in (16, 18, 20):
        canvas.edge([(12, y), (21, y)], "right", DETAIL)
    backed_arrow(canvas, (15, 7), (28, 9))


def blur(canvas: Canvas):
    """Blur: a drop with soft rings spreading round it."""
    for r, tone in ((13, "right"), (10.5, "left")):
        canvas.face(circle((16, 19), r, squash=0.55), tone)
    drop = sampled([(16, 3), (24, 15), (22, 24), (16, 24)], 12) + \
        sampled([(16, 24), (10, 24), (8, 15), (16, 3)], 12)[1:]
    solid(canvas, drop, "top")
    knot(canvas, (13.5, 17), 1.6, "new")


def sharpen(canvas: Canvas):
    """Sharp: a faceted diamond with a crisp cyan edge."""
    top = [(5, 12), (11, 5), (21, 5), (27, 12)]
    solid(canvas, [(5, 12), (11, 5), (21, 5), (27, 12)], "top")
    solid(canvas, [(5, 12), (16, 28), (16, 12)], "left")
    solid(canvas, [(16, 12), (16, 28), (27, 12)], "right")
    backed(canvas, top)


def clone(canvas: Canvas):
    """Clone: a rubber stamp pressing a copy of a cyan mark next to the
    source mark."""
    solid(canvas, rect(13, 3, 19, 11), "right")
    solid(canvas, rect(8, 11, 24, 16), "left")
    solid(canvas, rect(6, 16, 26, 19), "top")
    knot(canvas, (6, 25), 2.4, "new")
    knot(canvas, (16, 25), 2.4, "new")
    backed(canvas, [(9, 25), (13, 25)])


def line(canvas: Canvas):
    """Line: a straight cyan stroke between two end points."""
    backed(canvas, [(5, 26), (27, 6)])
    knot(canvas, (5, 26), 2.4)
    knot(canvas, (27, 6), 2.4)


def fill(canvas: Canvas):
    """Fill: a tipped bucket pouring a cyan drop."""
    bucket = turned([(7, 9), (21, 9), (19, 24), (9, 24)], (14, 16), -30)
    solid(canvas, bucket, "top")
    rim = turned([(7, 9), (21, 9)], (14, 16), -30)
    canvas.edge(rim, width=ACCENT)
    stroke_mark(canvas, [rim[1], (25, 6), (28, 14), (26, 22)])
    knot(canvas, (26, 26), 2.2, "new")


def linear_gradient(canvas: Canvas):
    """Linear Gradient: a square stepping from light to dark, an arrow
    across it."""
    canvas.face(card(4, 4, 28, 28), "top")
    canvas.face([(12, 4), (20, 4), (20, 28), (12, 28)], "left")
    canvas.face([(20, 4), (26.4, 4), (28, 5.6), (28, 26.4), (26.4, 28), (20, 28)], "right")
    canvas.edge(card(4, 4, 28, 28), width=EDGE, closed=True)
    backed_arrow(canvas, (7, 16), (25, 16))


def radial_gradient(canvas: Canvas):
    """Radial Gradient: rings from a light centre to a dark rim, an arrow
    out from the centre."""
    for r, tone in ((12.5, "right"), (9, "left"), (5, "top")):
        canvas.face(circle((16, 16), r), tone)
    canvas.edge(circle((16, 16), 12.5), width=EDGE, closed=True)
    backed_arrow(canvas, (16, 16), (25, 9))
    knot(canvas, (16, 16), 2.0)


def lasso(canvas: Canvas):
    """Lasso: a loose cyan loop with its tail and end knot."""
    loop = sampled([(16, 4), (29, 4), (29, 18), (16, 18)], 16) + \
        sampled([(16, 18), (3, 18), (3, 4), (16, 4)], 16)[1:]
    canvas.face(loop, "top")
    backed(canvas, loop, closed=True)
    tail = sampled([(10, 17), (8, 22), (13, 24), (9, 29)], 12)
    backed(canvas, tail)
    knot(canvas, (9, 29), 1.9)


# ------------------------------------------------------------- tips

def tip_smooth(canvas: Canvas):
    """Smooth tip: a dab fading softly from a light core."""
    for r, tone in ((13, "right"), (10, "left"), (6.5, "top")):
        canvas.face(circle((16, 16), r), tone)
    canvas.edge(circle((16, 16), 13), "left", DETAIL, closed=True)
    knot(canvas, (16, 16), 2.0, "new")


def tip_hard(canvas: Canvas):
    """Hard tip: an even dab with a crisp edge."""
    solid(canvas, circle((16, 16), 12.5), "top", ACCENT)
    knot(canvas, (16, 16), 2.0, "new")


def tip_procedural(canvas: Canvas):
    """Procedural tip: a dab broken up by a noise pattern."""
    solid(canvas, circle((16, 16), 12.5), "top")
    for x, y, r in ((11, 11, 2.6), (19, 9, 1.8), (21, 17, 3.0), (12, 20, 2.2),
                    (17, 23, 1.6), (8, 15, 1.3), (24, 12, 1.2), (16, 15, 1.4)):
        canvas.face(circle((x, y), r, 16), "right")


def tip_sphere(canvas: Canvas):
    """Sphere tip: a shaded ball with a highlight."""
    canvas.face(circle((16, 16), 12.5), "right")
    canvas.face(circle((14, 14), 9.5), "left")
    canvas.face(circle((12.5, 12.5), 5.5), "top")
    canvas.edge(circle((16, 16), 12.5), width=EDGE, closed=True)
    knot(canvas, (11, 11), 1.6, "top")


def tip_texture(canvas: Canvas):
    """Texture tip: a dab filled with a checker picture."""
    disc = circle((16, 16), 12.5)
    canvas.face(disc, "top")
    for i in range(4):
        for j in range(4):
            if (i + j) % 2:
                x, y = 6 + 5 * i, 6 + 5 * j
                square = [(max(x, 4.5), max(y, 4.5)), (min(x + 5, 27.5), max(y, 4.5)),
                          (min(x + 5, 27.5), min(y + 5, 27.5)), (max(x, 4.5), min(y + 5, 27.5))]
                inside = [p for p in square if (p[0] - 16) ** 2 + (p[1] - 16) ** 2 <= 12.2 ** 2]
                if len(inside) == 4:
                    canvas.face(square, "right")
    canvas.edge(disc, width=EDGE, closed=True)


def tip_stamp(canvas: Canvas):
    """Stamp tip: one picture card pressed down once, with its corners."""
    solid(canvas, rect(5, 5, 27, 27), "top")
    star = []
    for i in range(10):
        r = 9 if i % 2 == 0 else 4
        a = -pi / 2 + i * pi / 5
        star.append((16 + r * cos(a), 16.5 + r * sin(a)))
    solid(canvas, star, "left")
    backed(canvas, rect(5, 5, 27, 27), closed=True)


def tip_text(canvas: Canvas):
    """Text tip: a capital T."""
    t = [(5, 4), (27, 4), (27, 10), (19.5, 10), (19.5, 28), (12.5, 28), (12.5, 10), (5, 10)]
    solid(canvas, t, "top", ACCENT)
    backed(canvas, [(9, 31), (23, 31)])


# ------------------------------------------------------------- nozzle, preset

def nozzle(canvas: Canvas):
    """Tablet Nozzle: a pen with pressure lines where its tip meets the
    surface."""
    pen = turned([(14, 2), (19, 2), (19, 20), (16.5, 25), (14, 20)], (16.5, 14), 30)
    solid(canvas, pen, "top")
    grip = turned(rect(14, 12, 19, 17), (16.5, 14), 30)
    solid(canvas, grip, "right")
    tip = pen[3]
    for k, r in enumerate((3.5, 6.5)):
        arc = [(tip[0] + r * cos(a), tip[1] + 0.5 * r * sin(a))
               for a in [pi * (0.15 + 0.7 * i / 10) for i in range(11)]]
        backed(canvas, arc)


def save_preset(canvas: Canvas):
    """Save Brush Preset: a soft dab going down into a tray."""
    for r, tone in ((7.5, "left"), (4.5, "top")):
        canvas.face(circle((16, 9), r), tone)
    canvas.edge(circle((16, 9), 7.5), width=EDGE, closed=True)
    tray = [(4, 21), (9, 21), (11, 24), (21, 24), (23, 21), (28, 21), (28, 29), (4, 29)]
    solid(canvas, tray, "right")
    backed_arrow(canvas, (16, 13), (16, 24))


# ------------------------------------------------------------- inks

def ink_image(canvas: Canvas):
    """Image Ink: an ink bottle whose label is a little picture."""
    bottle(canvas)
    solid(canvas, rect(9, 16, 23, 25), "left", DETAIL)
    canvas.face([(9, 25), (14, 19), (17, 22), (19, 20), (23, 25)], "right")
    knot(canvas, (20, 18.5), 1.3, "new")


def ink_parametric(canvas: Canvas):
    """Parametric Ink: an ink bottle with a ramp from light to dark."""
    bottle(canvas)
    canvas.face([(9, 25), (23, 25), (23, 17)], "right")
    canvas.edge([(9, 25), (23, 17)], width=EDGE)
    backed(canvas, sampled([(9, 25), (14, 25), (18, 20), (23, 17)], 12))


def ink_random(canvas: Canvas):
    """Random Ink: an ink bottle with dots of different colours."""
    bottle(canvas)
    for (x, y), tone in (((11, 18), "x"), ((17, 23), "y"), ((21, 18), "z"), ((14, 24), "new")):
        knot(canvas, (x, y), 2.3, tone)


def ink_projection(canvas: Canvas):
    """Projection Ink: an ink bottle with rays thrown out of it."""
    bottle(canvas)
    solid(canvas, rect(9, 17, 23, 25), "left", DETAIL)
    for x in (11, 16, 21):
        backed(canvas, [(16, 21), (x + (x - 16) * 0.6, 17)])
    for start, end in (((9.5, 4), (2, -4)), ((16, 4), (16, -5)), ((22.5, 4), (30, -4))):
        backed(canvas, [start, end])
    knot(canvas, (16, 21), 2.0, "new")


# ------------------------------------------------------------- badges

def paint_target(canvas: Canvas):
    """The paint target: a checker texture with a brush dab on it."""
    canvas.face(card(4, 4, 28, 28), "top")
    canvas.face([(16, 4), (26.4, 4), (28, 5.6), (28, 16), (16, 16)], "left")
    canvas.face([(4, 16), (16, 16), (16, 28), (5.6, 28), (4, 26.4)], "left")
    canvas.edge(card(4, 4, 28, 28), width=EDGE, closed=True)
    stroke_mark(canvas, [(8, 22), (12, 14), (19, 22), (24, 10)])


def psd_layers(canvas: Canvas):
    """Photoshop layers: three sheets stacked and stepped."""
    for k, tone in enumerate(("right", "left", "top")):
        d = 5 * k
        solid(canvas, card(4 + d, 4 + d, 18 + d, 18 + d), tone)


# ------------------------------------------------------------- image commands

def add_color_texture(canvas: Canvas):
    """Add Color Texture: a picture in red, green and blue bands, and a
    plus."""
    canvas.face([(4.6, 3), (9, 3), (9, 21), (4.6, 21), (3, 19.4), (3, 4.6)], "x")
    canvas.face(rect(9, 3, 15, 21), "y")
    canvas.face([(15, 3), (19.4, 3), (21, 4.6), (21, 19.4), (19.4, 21), (15, 21)], "z")
    canvas.edge(card(3, 3, 21, 21), width=EDGE, closed=True)
    plus(canvas, (24, 24))


def add_mask_texture(canvas: Canvas):
    """Add Mask Texture: a picture split into light and dark, and a plus."""
    canvas.face(card(3, 3, 21, 21), "top")
    canvas.face([(4.6, 21), (21, 4.6), (21, 19.4), (19.4, 21)], "right")
    canvas.edge([(4.6, 21), (21, 4.6)], width=EDGE)
    canvas.edge(card(3, 3, 21, 21), width=EDGE, closed=True)
    plus(canvas, (24, 24))


def add_displacement_texture(canvas: Canvas):
    """Add Displacement Texture: a picture with a raised bump in profile,
    and a plus."""
    canvas.face(card(3, 3, 21, 21), "top")
    bump = sampled([(3, 17), (9, 17), (10, 7), (12, 7)], 10) + \
        sampled([(12, 7), (14, 7), (15, 17), (21, 17)], 10)[1:]
    canvas.face(bump + [(21, 19.4), (19.4, 21), (4.6, 21), (3, 19.4)], "left")
    canvas.edge(bump, width=EDGE)
    canvas.edge(card(3, 3, 21, 21), width=EDGE, closed=True)
    plus(canvas, (24, 24))


def image_invert(canvas: Canvas):
    """Invert Image: a picture half light, half dark, and a turning
    arrow."""
    canvas.face(rect(4, 8, 28, 28), "top")
    canvas.face([(16, 8), (28, 8), (28, 28), (16, 28)], "right")
    canvas.edge([(16, 8), (16, 28)], width=EDGE)
    canvas.edge(rect(4, 8, 28, 28), width=EDGE, closed=True)
    backed(canvas, sampled([(9, 6), (11, 1), (21, 1), (23, 6)], 12))
    backed_arrow(canvas, (22.2, 4.5), (23.2, 7.2))


def image_clear(canvas: Canvas):
    """Clear Image: an empty picture frame swept clean."""
    canvas.face(card(4, 4, 28, 28), "top")
    canvas.edge(card(4, 4, 28, 28), width=ACCENT, closed=True)
    # A cyan X: everything in it goes.
    backed(canvas, [(10, 10), (22, 22)])
    backed(canvas, [(22, 10), (10, 22)])


def image_set_resolution(canvas: Canvas):
    """Set Resolution: a picture with arrows out of two corners."""
    sheet(canvas, 9, 9, 23, 23)
    for x in (12, 16, 20):
        canvas.edge([(x, 9), (x, 23)], "left", DETAIL)
        canvas.edge([(9, x), (23, x)], "left", DETAIL)
    canvas.edge(rect(9, 9, 23, 23), width=EDGE, closed=True)
    backed_arrow(canvas, (23, 23), (29.5, 29.5))
    backed_arrow(canvas, (9, 9), (2.5, 2.5))


def image_expand_seams(canvas: Canvas):
    """Expand UV Seams: a UV island with its colour bled out past the
    edge."""
    island = [(9, 7), (22, 9), (24, 20), (14, 25), (7, 18)]
    grown = [(6, 4), (25, 6.5), (28, 22), (14, 29), (3, 19)]
    backed(canvas, grown, closed=True)
    solid(canvas, island, "left")


def image_flatten(canvas: Canvas):
    """Flatten Layers: two sheets and an arrow down to one."""
    solid(canvas, card(3, 3, 14, 12), "right")
    solid(canvas, card(6, 6, 17, 15), "top")
    backed_arrow(canvas, (20, 9), (25, 17))
    solid(canvas, card(16, 20, 29, 29), "left")


# ------------------------------------------------------------- layout

def layout_paint(canvas: Canvas):
    """The Paint layout: a painter's palette with colour dots."""
    outline = sampled([(16, 3), (30, 3), (31, 25), (18, 27)], 16) + \
        sampled([(18, 27), (12, 28), (10, 22), (6, 25)], 10)[1:] + \
        sampled([(6, 25), (1, 22), (2, 3), (16, 3)], 14)[1:]
    solid(canvas, outline, "top")
    canvas.face(circle((11, 19), 2.6), "left")
    canvas.edge(circle((11, 19), 2.6), width=DETAIL, closed=True)
    for (x, y), tone in (((9, 10), "x"), ((16, 8), "y"), ((23, 10), "z"), ((24, 18), "new")):
        knot(canvas, (x, y), 2.4, tone)


PAINT_ICONS = {
    "paint-airbrush": (airbrush, "Airbrush", "A spray can sending a fan of cyan dots."),
    "paint-paintbrush": (paintbrush, "Paintbrush", "A slanted paintbrush over its cyan stroke."),
    "paint-eraser": (eraser, "Eraser", "A two-tone eraser block where a cyan stroke stops."),
    "paint-smudge": (smudge, "Smudge", "A round blot smeared out along a cyan arrow."),
    "paint-blur": (blur, "Blur", "A light drop with soft rings spreading round it."),
    "paint-sharpen": (sharpen, "Sharp", "A faceted diamond with a crisp cyan top edge."),
    "paint-clone": (clone, "Clone", "A rubber stamp above a cyan mark and its copy."),
    "paint-line": (line, "Line", "A straight cyan stroke between two end points."),
    "paint-fill": (fill, "Fill", "A tipped bucket pouring a cyan drop."),
    "paint-linear-gradient": (linear_gradient, "Linear Gradient",
                              "A square stepping from light to dark under a cyan arrow."),
    "paint-radial-gradient": (radial_gradient, "Radial Gradient",
                              "Rings from a light centre to a dark rim with a cyan arrow outward."),
    "paint-lasso": (lasso, "Lasso", "A loose cyan loop with a tail and end knot."),
    "tip-smooth": (tip_smooth, "Smooth Tip", "A round dab fading softly from a light core."),
    "tip-hard": (tip_hard, "Hard Tip", "An even round dab with a crisp edge."),
    "tip-procedural": (tip_procedural, "Procedural Tip", "A round dab broken up by noise spots."),
    "tip-sphere": (tip_sphere, "Sphere Tip", "A shaded ball with a highlight."),
    "tip-texture": (tip_texture, "Texture Tip", "A round dab filled with a checker picture."),
    "tip-stamp": (tip_stamp, "Stamp Tip", "A square card with a star, outlined in cyan."),
    "tip-text": (tip_text, "Text Tip", "A capital T over a cyan base line."),
    "paint-nozzle": (nozzle, "Tablet Nozzle", "A pen with cyan pressure lines at its tip."),
    "paint-save-preset": (save_preset, "Save Brush Preset", "A soft dab going down a cyan arrow into a tray."),
    "ink-image": (ink_image, "Image Ink", "An ink bottle whose label is a little picture."),
    "ink-parametric": (ink_parametric, "Parametric Ink", "An ink bottle with a ramp from light to dark."),
    "ink-random": (ink_random, "Random Ink", "An ink bottle with red, green, blue and cyan dots."),
    "ink-projection": (ink_projection, "Projection Ink",
                       "An ink bottle with a picture label, cyan rays fanning up out of it."),
    "paint-target": (paint_target, "Paint Target", "A checker texture with a cyan brush stroke on it."),
    "psd-layers": (psd_layers, "Photoshop Layers", "Three stepped sheets stacked."),
    "add-color-texture": (add_color_texture, "Add Color Texture",
                          "A picture in red, green and blue bands with a cyan plus."),
    "add-mask-texture": (add_mask_texture, "Add Mask Texture",
                         "A picture split light and dark grey with a cyan plus."),
    "add-displacement-texture": (add_displacement_texture, "Add Displacement Texture",
                                 "A picture with a raised bump in profile and a cyan plus."),
    "image-invert": (image_invert, "Invert Image", "A half light, half dark picture under a cyan turning arrow."),
    "image-clear": (image_clear, "Clear Image", "A blank picture frame with a cyan cross."),
    "image-set-resolution": (image_set_resolution, "Set Resolution",
                             "A gridded picture with cyan arrows out of two corners."),
    "image-expand-seams": (image_expand_seams, "Expand UV Seams",
                           "A UV island with a cyan outline grown past its edge."),
    "image-flatten": (image_flatten, "Flatten Layers", "Two stacked sheets and a cyan arrow down to one."),
    "layout-paint": (layout_paint, "Paint Layout", "A painter's palette with coloured dots."),
}
