"""Batch 17: the Vertex Map Tools (spec vertex-map-tools, VMT-01): Paint,
Smooth, Scale Down, Erase and Set Value.

The same flat clay slab as the sculpt brushes, seen from the usual angle,
with the weight painted on its top in the weight shading's red (the
palette's X red; VMT-30 shows full weight red). Each tool changes how the
painted patch looks: a soft blob being laid down, a hard edge going soft,
a patch shrinking to a faint core, a patch with a bite taken out, and a
crisp flat disc. No relief: weight painting moves nothing.
"""

from math import cos, sin

from .conventions import DETAIL, EDGE
from .create_icons import backed, sampled
from .primitives import Canvas
from .sculpt_duplicate_icons import TAU, _slab


def _flat(canvas: Canvas, extra=()):
    """The flat slab; returns its projection (top spans 0..2 in x and y)."""
    return _slab(canvas, lambda x, y: 0.0, n=8, lines=1, extra=list(extra))


def _disc(projection, cx: float, cy: float, r: float, k: int = 40, squash: float = 1.0,
          wobble: float = 0.0):
    pts = []
    for i in range(k):
        a = TAU * i / k
        rr = r * (1 + wobble * sin(3 * a) * cos(2 * a))
        pts.append(projection((cx + rr * cos(a), cy + rr * squash * sin(a), 0.0)))
    return pts


def vmap_paint(canvas: Canvas):
    """Paint: a soft red blob, dense in the middle, with a cyan stroke
    showing it being painted on."""
    p = _flat(canvas, extra=[(1.9, 0.2, 0.0)])
    canvas.face(_disc(p, 0.95, 1.05, 0.62, wobble=0.12), "x")
    canvas.edge(_disc(p, 0.95, 1.05, 0.62, wobble=0.12), "x", DETAIL, closed=True)
    start = p((0.45, 1.5, 0.0))
    end = p((1.7, 0.35, 0.0))
    backed(canvas, sampled([start, (start[0] + 4, start[1] - 3), (end[0] - 4, end[1] + 1), end], 16))


def vmap_smooth(canvas: Canvas):
    """Smooth: a red patch whose hard edge (back) turns into soft steps
    (front)."""
    p = _flat(canvas)
    canvas.face(_disc(p, 1.0, 1.0, 0.75), "selected")
    canvas.face(_disc(p, 1.0, 1.0, 0.55), "x")
    canvas.edge(_disc(p, 1.0, 1.0, 0.75), "x", DETAIL, closed=True)


def vmap_scale_down(canvas: Canvas):
    """Scale Down: a faint outline of the old patch around a small red
    core, with arrows pulling inward."""
    p = _flat(canvas)
    canvas.edge(_disc(p, 1.0, 1.0, 0.8), "x", EDGE, closed=True)
    canvas.face(_disc(p, 1.0, 1.0, 0.32), "x")
    for a in (0.15, 0.4, 0.65, 0.9):
        t = TAU * a
        outer = p((1 + 0.72 * cos(t), 1 + 0.72 * sin(t), 0.0))
        inner = p((1 + 0.42 * cos(t), 1 + 0.42 * sin(t), 0.0))
        canvas.arrow(outer, inner, "new")


def vmap_erase(canvas: Canvas):
    """Erase: a red patch with a round bite cleared back to bare clay."""
    p = _flat(canvas)
    canvas.face(_disc(p, 0.85, 1.0, 0.72), "x")
    canvas.face(_disc(p, 1.35, 1.25, 0.45), "top")
    canvas.edge(_disc(p, 1.35, 1.25, 0.45), "new", EDGE, closed=True)


def vmap_set_value(canvas: Canvas):
    """Set Value: a crisp, flat, evenly filled disc, inked hard."""
    p = _flat(canvas)
    canvas.face(_disc(p, 1.0, 1.0, 0.7), "x")
    canvas.edge(_disc(p, 1.0, 1.0, 0.7), width=EDGE, closed=True)
    canvas.vertex(p((1.0, 1.0, 0.0)), 1.1, "top")


VERTEX_MAP_ICONS = {
    "vmap-paint": (vmap_paint, "Paint (Vertex Map)", "A clay slab with a soft red weight blob being painted along a cyan stroke."),
    "vmap-smooth": (vmap_smooth, "Smooth (Vertex Map)", "A red weight patch with a soft yellow rim."),
    "vmap-scale-down": (vmap_scale_down, "Scale Down (Vertex Map)", "A small red weight core inside the outline of a larger patch, with cyan arrows pointing in."),
    "vmap-erase": (vmap_erase, "Erase (Vertex Map)", "A red weight patch with a round bite cleared to bare clay, outlined in cyan."),
    "vmap-set-value": (vmap_set_value, "Set Value (Vertex Map)", "A crisp, evenly filled red disc with a hard ink edge."),
}
