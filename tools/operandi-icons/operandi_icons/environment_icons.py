"""Batch 13: the Environment item.

The Environment is the world around the scene, not an object in it, so the
icon shows a view out to a horizon: sky above, ground below running away
to it. It is flat (no shading), so it can't be read as a sphere mesh, and it
has no rays or glow, so it can't be read as a light. Blue is the sky's own
colour here; the icon has no motion, so it doesn't read as the Z axis.
"""

from math import cos, pi, sin

from .conventions import DETAIL, EDGE
from .primitives import Canvas

TAU = 2 * pi
C, R, HORIZON = (16.0, 16.0), 13.0, 16.5


def _arc(a0: float, a1: float, steps: int = 40):
    return [(C[0] + R * cos(a0 + (a1 - a0) * i / steps), C[1] + R * sin(a0 + (a1 - a0) * i / steps))
            for i in range(steps + 1)]


def environment(canvas: Canvas):
    """A round window onto a horizon: blue sky over grey ground, the ground
    lines running away to a point on the horizon."""
    import math
    dy = HORIZON - C[1]
    # Angles (screen y down): the horizon meets the circle at angles a, pi - a.
    a = math.asin(dy / R)
    sky = _arc(pi - a, TAU + a)          # over the top
    ground = _arc(a, pi - a)             # under the bottom
    canvas.face(sky, "z")
    canvas.face(ground, "left")
    vanish = (C[0], HORIZON)
    # Ground lines fanning from the vanishing point to the rim.
    for t in (0.06, 0.28, 0.5, 0.72, 0.94):
        ang = a + (pi - 2 * a) * t
        rim = (C[0] + R * cos(ang), C[1] + R * sin(ang))
        canvas.edge([vanish, rim], "right", EDGE)
    # One cross line on the ground, for depth.
    for y in (HORIZON + 5.0,):
        x = (R ** 2 - (y - C[1]) ** 2) ** 0.5
        canvas.edge([(C[0] - x, y), (C[0] + x, y)], "right", DETAIL)
    x = (R ** 2 - dy ** 2) ** 0.5
    canvas.edge([(C[0] - x, HORIZON), (C[0] + x, HORIZON)], width=EDGE)
    canvas.edge(_arc(0, TAU, 96)[:-1], width=EDGE, closed=True)


ENVIRONMENT_ICONS = {
    "environment": (environment, "Environment",
                    "A round window onto a horizon: blue sky over grey ground running away to it."),
}
