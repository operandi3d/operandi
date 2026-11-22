#!/usr/bin/env python3
"""Draw the labelled UI map for the manual (docs/manual/ui-map.*).

Usage:
    python3 docs/manual/tools/make_ui_map.py <screenshot-2880.png> [out_dir]

The screenshot is the default window at 1920x1080 logical points captured at
1.5x (2880x1620 pixels). Callout targets below are in logical points, the same
units as the capture's .json rects (viewport [244, 52, 1376, 1004]).

Writes, in out_dir (default: docs/manual):
    ui-map-screenshot.png  copy of the screenshot the SVG references
    ui-map.svg             screenshot + numbered callouts (vector)
    ui-map.png             the same, rasterised with Pillow

Needs Pillow for the PNG (the icon generator's venv has it:
tools/operandi-icons/.venv/bin/python). Without Pillow it still writes the SVG.
"""

import shutil
import sys
from pathlib import Path

SCALE = 1.5          # pixels per logical point in the screenshot
MARGIN = 60          # logical points of margin around the screenshot
SHOT_W, SHOT_H = 1920, 1080

ACCENT = "#ff9f1c"
OUTLINE = "#1a1a1a"
BACKGROUND = "#17181b"
RADIUS = 14          # callout circle radius, logical points
FONT_SIZE = 17

# Margin lanes (logical points, relative to the screenshot's top-left).
TOP = -MARGIN / 2
BOTTOM = SHOT_H + MARGIN / 2
LEFT = -MARGIN / 2
RIGHT = SHOT_W + MARGIN / 2

# (number, name, label position, target point, optional span underline
#  as (x0, x1, y)). Numbers run top-left to bottom-right; labels sit in the
# margins or in empty viewport space so they never cover a control.
CALLOUTS = [
    (1, "Logo", (18, TOP), (19, 13), None),
    (2, "Layout bar", (144, TOP), (144, 25), (102, 212, 25)),
    (3, "Menu bar", (489, TOP), (489, 25), (227, 805, 25)),
    (4, "Save, Undo and Redo", (853, TOP), (853, 25), (818, 1095, 25)),
    (5, "Selection modes", (LEFT, 39), (6, 39), (4, 398, 51)),
    (6, "Modelling aids", (1131, TOP), (1127, 51), (412, 1127, 51)),
    (7, "Render and Preview", (1294, TOP), (1290, 51), (1135, 1290, 51)),
    (8, "Panel-group tabs", (1765, TOP), (1765, 79), (1625, 1915, 79)),
    (9, "Transform tools", (LEFT, 75), (8, 75), None),
    (10, "Toolbox", (LEFT, 160), (8, 130), None),
    (11, "Tool Tabs", (290, 235), (234, 250), None),
    (12, "View dot", (275, 112), (254, 65), None),
    (13, "View header", (433, 112), (433, 77), (262, 560, 77)),
    (14, "Navigation buttons", (1560, 112), (1560, 77), (1512, 1614, 77)),
    (15, "Items list", (RIGHT, 150), (1916, 150), None),
    (16, "3D view", (1430, 230), (1380, 270), None),
    (17, "Tool Properties", (LEFT, 735), (8, 718), None),
    (18, "Properties", (RIGHT, 650), (1912, 650), None),
    (19, "Work plane grid", (1500, 700), (1420, 778), None),
    (20, "Axis marker", (LEFT, 1000), (285, 1000), None),
    (21, "Readout", (1580, BOTTOM), (1580, 1050), None),
    (22, "Message line", (258, BOTTOM), (258, 1075), None),
    (23, "Command entry", (1760, BOTTOM), (1760, 1077), None),
]


def px(v):
    """Logical point (relative to the screenshot) -> canvas pixel."""
    return (v + MARGIN) * SCALE


def canvas_size():
    return int(px(SHOT_W + MARGIN)), int(px(SHOT_H + MARGIN))


def write_svg(path):
    w, h = canvas_size()
    r = RADIUS * SCALE
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'xmlns:xlink="http://www.w3.org/1999/xlink" '
        f'width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
        f'<rect width="{w}" height="{h}" fill="{BACKGROUND}"/>',
        f'<image href="ui-map-screenshot.png" xlink:href="ui-map-screenshot.png" '
        f'x="{px(0)}" y="{px(0)}" width="{SHOT_W * SCALE}" height="{SHOT_H * SCALE}"/>',
    ]
    for num, name, (lx, ly), (tx, ty), span in CALLOUTS:
        g = [f'<g id="callout-{num}"><title>{num}. {name}</title>']
        if span:
            x0, x1, sy = span
            g.append(
                f'<line x1="{px(x0)}" y1="{px(sy)}" x2="{px(x1)}" y2="{px(sy)}" '
                f'stroke="{ACCENT}" stroke-width="3" stroke-linecap="round"/>'
            )
        g.append(
            f'<line x1="{px(lx)}" y1="{px(ly)}" x2="{px(tx)}" y2="{px(ty)}" '
            f'stroke="{OUTLINE}" stroke-width="4.5"/>'
            f'<line x1="{px(lx)}" y1="{px(ly)}" x2="{px(tx)}" y2="{px(ty)}" '
            f'stroke="{ACCENT}" stroke-width="2"/>'
            f'<circle cx="{px(tx)}" cy="{px(ty)}" r="5" fill="{ACCENT}" '
            f'stroke="{OUTLINE}" stroke-width="1.5"/>'
            f'<circle cx="{px(lx)}" cy="{px(ly)}" r="{r}" fill="{ACCENT}" '
            f'stroke="{OUTLINE}" stroke-width="1.5"/>'
            f'<text x="{px(lx)}" y="{px(ly)}" dy="0.35em" text-anchor="middle" '
            f'font-family="Helvetica, Arial, sans-serif" font-weight="bold" '
            f'font-size="{FONT_SIZE * SCALE}" fill="#ffffff">{num}</text></g>'
        )
        out.append("".join(g))
    out.append("</svg>")
    path.write_text("\n".join(out) + "\n", encoding="utf-8")


def find_font(size):
    from PIL import ImageFont

    for f in (
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
    ):
        if Path(f).exists():
            return ImageFont.truetype(f, size)
    return ImageFont.load_default()


def write_png(shot_path, path):
    from PIL import Image, ImageDraw

    w, h = canvas_size()
    base = Image.new("RGBA", (w, h), BACKGROUND)
    shot = Image.open(shot_path).convert("RGBA")
    base.paste(shot, (int(px(0)), int(px(0))))

    ss = 3  # supersample the overlay for smooth circles and lines
    over = Image.new("RGBA", (w * ss, h * ss), (0, 0, 0, 0))
    d = ImageDraw.Draw(over)
    font = find_font(int(FONT_SIZE * SCALE * ss))

    def p(x, y):
        return px(x) * ss, px(y) * ss

    for num, _name, (lx, ly), (tx, ty), span in CALLOUTS:
        if span:
            x0, x1, sy = span
            d.line([p(x0, sy), p(x1, sy)], fill=ACCENT, width=3 * ss)
        a, b = p(lx, ly), p(tx, ty)
        d.line([a, b], fill=OUTLINE, width=int(4.5 * ss))
        d.line([a, b], fill=ACCENT, width=2 * ss)
        dot = 5 * ss
        d.ellipse([b[0] - dot, b[1] - dot, b[0] + dot, b[1] + dot],
                  fill=ACCENT, outline=OUTLINE, width=int(1.5 * ss))
        r = RADIUS * SCALE * ss
        d.ellipse([a[0] - r, a[1] - r, a[0] + r, a[1] + r],
                  fill=ACCENT, outline=OUTLINE, width=int(1.5 * ss))
        d.text(a, str(num), font=font, fill="#ffffff", anchor="mm")

    over = over.resize((w, h), Image.LANCZOS)
    Image.alpha_composite(base, over).convert("RGB").save(path, optimize=True)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    shot = Path(sys.argv[1])
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parents[1]
    out_dir.mkdir(parents=True, exist_ok=True)
    if shot.resolve() != (out_dir / "ui-map-screenshot.png").resolve():
        shutil.copyfile(shot, out_dir / "ui-map-screenshot.png")
    write_svg(out_dir / "ui-map.svg")
    try:
        write_png(shot, out_dir / "ui-map.png")
    except ImportError:
        print("Pillow not found: wrote the SVG only", file=sys.stderr)
    print(f"wrote {out_dir}/ui-map.svg and ui-map.png")


if __name__ == "__main__":
    main()
