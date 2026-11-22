"""Batch 16: the rest of the Sculpt tab's brushes (spec sculpt-toolbox,
SCT-01). Same family as batch 12: a slab of clay seen from the usual
angle, its top showing the brush's effect. Only Smudge and Spin add a cyan
cue, for the motion that isn't visible in the shape alone.
"""

from math import atan2, cos, exp, hypot, sin

from .create_icons import backed, sampled
from .conventions import EDGE
from .primitives import Canvas
from .sculpt_duplicate_icons import TAU, _outline_raised, _rough, _slab


def _ramp(v: float) -> float:
    return min(1.0, max(0.0, v))


def sculpt_fold(canvas: Canvas):
    """Fold: a dent in the middle ringed by a raised rim."""
    def height(x, y):
        d = hypot(x - 1, y - 1)
        return -0.35 * exp(-(d / 0.32) ** 2) + 0.32 * exp(-((d - 0.62) / 0.16) ** 2)
    _slab(canvas, height, n=16, lines=5)


def sculpt_clay(canvas: Canvas):
    """Clay: a wide mound built up to a flat, inked top at its level."""
    level = 0.3
    def height(x, y):
        d = hypot(x - 1, y - 1)
        return level * _ramp((0.82 - d) / 0.2)
    projection = _slab(canvas, height, n=18, lines=3)
    _outline_raised(canvas, projection, height, floor=0.02)
    rim = [projection((1 + 0.62 * cos(a), 1 + 0.62 * sin(a), level)) for a in (TAU * k / 48 for k in range(49))]
    canvas.edge(rim, width=EDGE)


def sculpt_smudge(canvas: Canvas):
    """Smudge: a bump dragged into a long smear, with a cyan arrow for the
    drag."""
    def height(x, y):
        u, v = x - 0.45, y - 1.15
        head = exp(-((u - 1.0) / 0.28) ** 2 - (v / 0.28) ** 2)
        tail = exp(-(v / 0.2) ** 2) * _ramp(u / 1.0) * _ramp((1.2 - u) / 0.2)
        return 0.42 * max(head, 0.7 * tail)
    projection = _slab(canvas, height, n=18, lines=4, extra=[(1.9, 0.35, 0.7)])
    start = projection((0.45, 0.45, 0.5))
    end = projection((1.75, 0.45, 0.5))
    curve = sampled([start, (start[0] + 3.0, start[1] + 1.0), (end[0] - 3.0, end[1] - 1.0), end], 12)
    backed(canvas, curve)


def sculpt_carve(canvas: Canvas):
    """Carve: a sharp, narrow groove cut along a stroke."""
    path = [(0.2 + 1.6 * t, 1.0 + 0.4 * sin(t * TAU), 0.0) for t in (i / 40 for i in range(41))]
    def height(x, y):
        d = min(hypot(x - px, y - py) for px, py, _ in path)
        return -0.32 * _ramp(1 - d / 0.17) ** 1.5
    projection = _slab(canvas, height, n=18, lines=4)
    canvas.edge([projection((x, y, -0.32)) for x, y, _ in path], width=EDGE)


def sculpt_scar(canvas: Canvas):
    """Scar: a gash pinched between two raised lips, the cut inked."""
    line = lambda x: 1.0 + 0.18 * sin((x - 0.2) * 2.6)
    def height(x, y):
        d = abs(y - line(x))
        along = _ramp(1 - abs(x - 1) / 0.85)
        return along * (-0.3 * exp(-(d / 0.07) ** 2) + 0.3 * exp(-((d - 0.2) / 0.1) ** 2))
    projection = _slab(canvas, height, n=22, lines=3)
    xs = [0.3 + 1.4 * k / 30 for k in range(31)]
    canvas.edge([projection((x, line(x), height(x, line(x)))) for x in xs], width=EDGE)


def sculpt_flatten(canvas: Canvas):
    """Flatten: rough clay with a round patch planed flat, the cut inked."""
    cut, r = 0.2, 0.55
    def height(x, y):
        d = hypot(x - 1.0, y - 1.05)
        return min(_rough(x, y), cut + (_rough(x, y) - cut) * _ramp((d - r) / 0.12))
    projection = _slab(canvas, height, n=18, lines=4)
    rim = [projection((1.0 + r * cos(a), 1.05 + r * sin(a), min(cut, _rough(1.0 + r * cos(a), 1.05 + r * sin(a)))))
           for a in (TAU * k / 48 for k in range(49))]
    canvas.edge(rim, width=EDGE)


def sculpt_tangent_pinch(canvas: Canvas):
    """Tangent Pinch: the surface drawn into a sharp crease along a line,
    the crest inked."""
    crest = lambda x: 1.0 + 0.35 * (x - 1.0)
    def height(x, y):
        d = abs(y - crest(x))
        along = _ramp(1 - abs(x - 1) / 0.85)
        return 0.5 * along * _ramp(1 - d / 0.4) ** 3
    projection = _slab(canvas, height, n=20, lines=3)
    xs = [0.15 + 1.7 * k / 34 for k in range(35)]
    canvas.edge([projection((x, crest(x), height(x, crest(x)))) for x in xs], width=EDGE)


def sculpt_center_pinch(canvas: Canvas):
    """Center Pinch: the surface drawn into one sharp point."""
    def height(x, y):
        return 0.7 * _ramp(1 - hypot(x - 1, y - 1) / 0.7) ** 3
    projection = _slab(canvas, height, n=18, lines=5)
    _outline_raised(canvas, projection, height, floor=0.02)


def sculpt_emboss(canvas: Canvas):
    """Emboss: a crisp square patch stamped up from the surface."""
    def height(x, y):
        a = max(abs(x - 1) / 0.55, abs(y - 1) / 0.55)
        return 0.28 * _ramp((1 - a) / 0.12)
    projection = _slab(canvas, height, n=20, lines=3)
    _outline_raised(canvas, projection, height, floor=0.01)


def sculpt_attenuate(canvas: Canvas):
    """Attenuate: a row of bumps shrinking away to the plain surface."""
    bumps = ((0.45, 0.55, 0.6), (0.95, 1.0, 0.36), (1.45, 1.45, 0.16))
    def height(x, y):
        return sum(h * exp(-((x - bx) ** 2 + (y - by) ** 2) / 0.22 ** 2) for bx, by, h in bumps)
    projection = _slab(canvas, height, n=18, lines=4)
    for bx, by, h in bumps[:2]:
        f = lambda x, y, bx=bx, by=by, h=h: h * exp(-((x - bx) ** 2 + (y - by) ** 2) / 0.22 ** 2)
        _outline_raised(canvas, projection, f, floor=0.04)


def sculpt_spin(canvas: Canvas):
    """Spin: grid lines swirled round the brush centre, with a cyan turn
    arrow."""
    projection = _slab(canvas, lambda x, y: 0.0, n=8, lines=2, extra=[(1.0, 1.0, 0.5)])
    for arm in range(3):
        pts = []
        for k in range(25):
            t = k / 24
            a = arm * TAU / 3 + 2.4 * t
            r = 0.08 + 0.72 * t
            pts.append(projection((1 + r * cos(a), 1 + r * sin(a), 0.0)))
        canvas.edge(pts, width=EDGE)
    arc = [projection((1 + 0.95 * cos(a), 1 + 0.95 * sin(a), 0.2))
           for a in (TAU * (0.55 + 0.3 * k / 16) for k in range(17))]
    backed(canvas, arc)


SCULPT_TOOLBOX_ICONS = {
    "sculpt-fold": (sculpt_fold, "Fold Brush", "A clay slab with a dent ringed by a raised rim."),
    "sculpt-clay": (sculpt_clay, "Clay Brush", "A clay slab with a mound built up to a flat top."),
    "sculpt-smudge": (sculpt_smudge, "Smudge Brush", "A clay slab with a bump smeared along a cyan arrow."),
    "sculpt-carve": (sculpt_carve, "Carve Brush", "A clay slab with a sharp groove cut along a stroke."),
    "sculpt-scar": (sculpt_scar, "Scar Brush", "A clay slab with a pinched gash between raised lips."),
    "sculpt-flatten": (sculpt_flatten, "Flatten Brush", "Rough clay with a round patch planed flat."),
    "sculpt-tangent-pinch": (sculpt_tangent_pinch, "Tangent Pinch Brush",
                             "A clay slab drawn into a sharp crease along a line."),
    "sculpt-center-pinch": (sculpt_center_pinch, "Center Pinch Brush",
                            "A clay slab drawn into one sharp point."),
    "sculpt-emboss": (sculpt_emboss, "Emboss Brush", "A clay slab with a square patch stamped up."),
    "sculpt-attenuate": (sculpt_attenuate, "Attenuate Brush",
                         "A clay slab whose bumps fade away toward the plain surface."),
    "sculpt-spin": (sculpt_spin, "Spin Brush", "A clay slab with a swirl of ridges and a cyan turn arrow."),
}
