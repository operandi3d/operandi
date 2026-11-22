"""Semantic drawing primitives; SVG serialization lives only here."""

from math import hypot

import svgwrite

from .conventions import ACCENT, EDGE, PALETTE, Point, Projection, Vec3, rounded


class Canvas:
    """Drawing surface. Every point passes through `place`, a uniform scale
    and shift that `fit_to_canvas` uses to make each icon fill the canvas;
    stroke widths and dot radii are style constants and are never scaled.
    While drawing, `extent` records each point with the ink it carries
    (half a stroke width, or a dot radius)."""

    def __init__(self, name: str, description: str,
                 scale: float = 1.0, shift: Point = (0.0, 0.0)):
        self.drawing = svgwrite.Drawing(size=(32, 32), viewBox="0 0 32 32")
        self.drawing.set_desc(title=f"Operandi — {name}", desc=description)
        self.scale, self.shift = scale, shift
        self.extent: list[tuple[float, float, float]] = []

    def place(self, point: Point, ink: float = 0.0) -> Point:
        x = point[0] * self.scale + self.shift[0]
        y = point[1] * self.scale + self.shift[1]
        self.extent.append((x, y, ink))
        return rounded((x, y))

    def face(self, points: list[Point], color: str):
        self.drawing.add(self.drawing.polygon(
            points=[self.place(p) for p in points], fill=PALETTE[color]))

    def stroke(self, element, color: str, width: float):
        element.update({"fill": "none", "stroke": PALETTE[color],
                        "stroke-width": width, "stroke-linecap": "round",
                        "stroke-linejoin": "round"})
        self.drawing.add(element)

    def edge(self, points: list[Point], color: str = "ink", width: float = EDGE,
             closed: bool = False):
        factory = self.drawing.polygon if closed else self.drawing.polyline
        self.stroke(factory(points=[self.place(p, width / 2) for p in points]), color, width)

    def vertex(self, point: Point, radius: float = 0.7, color: str = "ink"):
        self.drawing.add(self.drawing.circle(
            center=self.place(point, radius), r=radius, fill=PALETTE[color]))

    def guide(self, center: Point, radius: float, width: float):
        cx, cy = center
        for dx, dy in ((-radius, 0), (radius, 0), (0, -radius), (0, radius)):
            self.place((cx + dx, cy + dy), width / 2)
        self.stroke(self.drawing.circle(center=self.place(center),
                                        r=round(radius * self.scale, 5)), "left", width)

    def mesh(self, projection: Projection,
             faces: list[tuple[list[Vec3], str]], width: float = EDGE):
        """Fill in painter order, then draw each unique visible edge once."""
        edges = {}
        for points, color in faces:
            projected = [projection(p) for p in points]
            self.face(projected, color)
            for start, end in zip(points, points[1:] + points[:1]):
                key = tuple(sorted((start, end)))
                edges[key] = (projection(start), projection(end))
        for start, end in edges.values():
            self.edge([start, end], width=width)

    def highlight_face(self, projection: Projection, points: list[Vec3],
                       color: str = "selected"):
        projected = [projection(p) for p in points]
        self.face(projected, color)
        self.edge(projected, closed=True)

    def highlight_edge(self, projection: Projection, points: list[Vec3]):
        self.edge([projection(p) for p in points], "new", ACCENT)

    def arrowhead(self, tip: Point, tangent: Point, color: str,
                  length: float, half_width: float, width: float = ACCENT):
        magnitude = hypot(*tangent)
        if magnitude == 0:
            raise ValueError("An arrow needs a nonzero terminal tangent")
        ux, uy = (value / magnitude for value in tangent)
        base = (tip[0] - ux * length, tip[1] - uy * length)
        self.edge([(base[0] - uy * half_width, base[1] + ux * half_width),
                   tip,
                   (base[0] + uy * half_width, base[1] - ux * half_width)],
                  color, width)

    def arrow(self, start: Point, end: Point, color: str,
              length: float = 2.1, half_width: float = 1.05, width: float = ACCENT):
        self.edge([start, end], color, width)
        self.arrowhead(end, (end[0] - start[0], end[1] - start[1]),
                       color, length, half_width, width)

    def cubic_arrow(self, points: list[Point], color: str,
                    length: float = 1.925, half_width: float = 0.99,
                    width: float = ACCENT):
        """Cubic control points plus a terminal tangent-derived open arrowhead."""
        start, first, second, end = points
        path = self.drawing.path()
        path.push("M", *self.place(start, width / 2), "C", *self.place(first, width / 2),
                  *self.place(second, width / 2), *self.place(end, width / 2))
        self.stroke(path, color, width)
        self.arrowhead(end, (end[0] - second[0], end[1] - second[1]),
                       color, length, half_width, width)


# Every icon's ink fills the canvas to this size on its longer side, centered,
# leaving a 1 px margin (Jason, 2026-11-01: icons looked small and uneven).
FILL = 30.0


def fit_to_canvas(build, name: str, description: str, fill: float = FILL) -> "Canvas":
    """Draw once to measure, then again scaled and centered to fill `fill`."""
    probe = Canvas(name, description)
    build(probe)
    xs = [(x - ink, x + ink) for x, _, ink in probe.extent]
    ys = [(y - ink, y + ink) for _, y, ink in probe.extent]
    left, right = min(a for a, _ in xs), max(b for _, b in xs)
    top, bottom = min(a for a, _ in ys), max(b for _, b in ys)
    # Ink at the edges (strokes, dots) doesn't scale: solve for the scale at
    # which the scaled geometry plus that fixed ink spans FILL.
    pad_x = max(ink for x, _, ink in probe.extent if x - ink <= left + 1e-9 or x + ink >= right - 1e-9)
    pad_y = max(ink for _, y, ink in probe.extent if y - ink <= top + 1e-9 or y + ink >= bottom - 1e-9)
    geo_w, geo_h = right - left - 2 * pad_x, bottom - top - 2 * pad_y
    scale = min((fill - 2 * pad_x) / geo_w if geo_w > 0 else 1e9,
                (fill - 2 * pad_y) / geo_h if geo_h > 0 else 1e9)
    cx, cy = (left + right) / 2, (top + bottom) / 2
    canvas = Canvas(name, description, scale, (16 - cx * scale, 16 - cy * scale))
    build(canvas)
    return canvas
