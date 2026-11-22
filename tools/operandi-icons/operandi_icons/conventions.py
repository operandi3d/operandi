"""All artwork conventions, in final 32×32 canvas coordinates."""

from dataclasses import dataclass
from math import sqrt
from types import MappingProxyType

PALETTE = MappingProxyType({
    "ink": "#263442", "top": "#DCE3E9", "left": "#AAB4C0",
    "right": "#8795A5", "selected": "#FFD470", "new": "#56B4D3",
    "x": "#D85C58", "y": "#4EAA73", "z": "#4C8CD6",
})
EDGE = 0.96
ACCENT = 1.12
DETAIL = 0.72
# Selected edges: a yellow-orange line on a wider ink backing, so they read
# on light faces and against the background (added with the Edge batch).
SELECT = 1.5
SELECT_BACKING = 2.3
# Transform gizmos (Move, Rotate, Scale, Transform): bolder than other
# accents so the colored axes read at toolbar size (Jason, 2026-11-01).
GIZMO = 1.7
STROKES = frozenset((EDGE, ACCENT, DETAIL, SELECT, SELECT_BACKING, GIZMO))
CENTER = (16.0, 15.75)
UNIT = 12.9
Point = tuple[float, float]
Vec3 = tuple[float, float, float]


@dataclass(frozen=True)
class Projection:
    """Equal-length 3D axes, 120° apart; positive Z points up."""

    unit: float = UNIT
    origin: Point = CENTER

    def __call__(self, point: Vec3) -> Point:
        x, y, z = point
        return (self.origin[0] + self.unit * sqrt(3) / 2 * (x - y),
                self.origin[1] + self.unit * (0.5 * (x + y) - z))


def rounded(point: Point) -> Point:
    return tuple(round(value, 5) for value in point)
