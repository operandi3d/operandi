"""Batch 19: Solid Sketch.

Solid Sketch roughs out a soft, blobby body from a tree of clicked nodes, so
the icon shows exactly that: a lumpy solid whose thick and thin parts follow
a little branching chain of nodes inside it. The body is shaded like the
sphere (nested tones lit from the upper left, no inner ink), so it reads as
one round solid. The node chain is the cyan "new geometry" cue, drawn like
Arc's points (light dots in ink rings, on an ink-backed line) so it reads on
both themes. The branch is what tells it apart from Tube, which is one pipe
of even width through its points.
"""

from math import hypot

from .conventions import SELECT, SELECT_BACKING, Point
from .primitives import Canvas

# Node centres and radii (canvas pixels), and the links of the tree. The root
# is the big lower-left lump; the chain thins towards the upper right and one
# branch leaves the middle node.
NODES: list[tuple[Point, float]] = [
    ((10.5, 20.5), 7.6),   # 0 root
    ((18.5, 13.5), 5.6),   # 1 middle
    ((25.0, 7.2), 4.2),    # 2 tip
    ((24.0, 21.6), 4.0),   # 3 branch
]
LINKS = [(0, 1), (1, 2), (1, 3)]
BLEND = 2.8  # how softly the lumps flow into each other (smooth-min width)


def _segment(p: Point, a: tuple[Point, float], b: tuple[Point, float]) -> float:
    """Signed distance to a cone-ish capsule whose radius runs from a's to b's."""
    (ax, ay), ra = a
    (bx, by), rb = b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((p[0] - ax) * dx + (p[1] - ay) * dy) / (dx * dx + dy * dy)))
    return hypot(p[0] - ax - dx * t, p[1] - ay - dy * t) - (ra + (rb - ra) * t)


def _smin(a: float, b: float, k: float = BLEND) -> float:
    h = max(k - abs(a - b), 0.0) / k
    return min(a, b) - h * h * k * 0.25


def field(p: Point) -> float:
    """Negative inside the body."""
    d = None
    for i, j in LINKS:
        s = _segment(p, NODES[i], NODES[j])
        d = s if d is None else _smin(d, s)
    return d


def _contours(f, step: float = 0.2) -> list[list[Point]]:
    """Closed loops where f = 0 (marching squares), longest first."""
    n = int(32 / step) + 1
    grid = [[f((i * step, j * step)) for i in range(n)] for j in range(n)]
    # Crossings are keyed by the grid edge they sit on, so the two cells
    # sharing an edge always agree on the point.
    points: dict[tuple, Point] = {}
    links: dict[tuple, list[tuple]] = {}

    def crossing(c0, c1):
        key = (c0, c1) if c0 < c1 else (c1, c0)
        if key not in points:
            (i0, j0), (i1, j1) = key
            a, b = grid[j0][i0], grid[j1][i1]
            t = a / (a - b)
            points[key] = ((i0 + (i1 - i0) * t) * step, (j0 + (j1 - j0) * t) * step)
        return key

    for j in range(n - 1):
        for i in range(n - 1):
            corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)]
            inside = [grid[cj][ci] < 0 for ci, cj in corners]
            keys = [crossing(corners[k], corners[(k + 1) % 4])
                    for k in range(4) if inside[k] != inside[(k + 1) % 4]]
            # Four crossings (a saddle) don't occur for these smooth shapes.
            for a, b in zip(keys[::2], keys[1::2]):
                links.setdefault(a, []).append(b)
                links.setdefault(b, []).append(a)
    loops, seen = [], set()
    for start in links:
        if start in seen:
            continue
        loop, previous, current = [], None, start
        while current not in seen:
            seen.add(current)
            loop.append(points[current])
            options = [q for q in links[current] if q != previous]
            previous, current = current, options[0]
        if len(loop) > 8:
            loops.append(loop)
    loops.sort(key=len, reverse=True)
    return [_thin(loop) for loop in loops]


def _thin(loop: list[Point], spacing: float = 0.55) -> list[Point]:
    """Keep points about `spacing` px apart, so the SVG stays small."""
    out = [loop[0]]
    for p in loop[1:]:
        if hypot(p[0] - out[-1][0], p[1] - out[-1][1]) >= spacing:
            out.append(p)
    return out


def solid_sketch(canvas: Canvas):
    """A soft lumpy body grown around a branching cyan chain of nodes."""
    outline = _contours(field)[0]
    canvas.face(outline, "right")
    # Lighter zones: the body shrunk and slid towards the light (upper left),
    # kept inside the outline; each lump gets its own highlight.
    for tone, inset, (sx, sy) in (("left", 1.6, (1.0, 1.1)), ("top", 3.9, (2.0, 2.3))):
        zone = lambda p, inset=inset, sx=sx, sy=sy: max(  # noqa: E731
            field((p[0] + sx, p[1] + sy)) + inset, field(p) + 0.6)
        for loop in _contours(zone):
            canvas.face(loop, tone)
    canvas.edge(outline, closed=True)
    # The node tree: an ink-backed cyan line, and light dots in ink rings.
    for i, j in LINKS:
        line = [NODES[i][0], NODES[j][0]]
        canvas.edge(line, "ink", SELECT_BACKING)
    for i, j in LINKS:
        canvas.edge([NODES[i][0], NODES[j][0]], "new", SELECT)
    for index, (centre, _) in enumerate(NODES):
        outer, inner = (1.9, 1.15) if index == 0 else (1.5, 0.8)
        canvas.vertex(centre, outer)
        canvas.vertex(centre, inner, "new")


SOLID_SKETCH_ICONS = {
    "solid-sketch": (solid_sketch, "Solid Sketch",
                     "A soft lumpy body grown around a branching cyan chain of nodes."),
}
