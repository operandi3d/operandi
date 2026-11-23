#!/usr/bin/env python3
"""Render the User Guide's pictures from docs/guide/shots.toml.

Usage:
    python3 tools/guide_shots.py [--no-build] [--only NAME] [--binary PATH]

Builds the QA app (cargo build --profile qa -p operandi-app --features
cycles,usd,qa) unless --no-build, then runs it once per shot with the
offscreen harness (OPERANDI_QA_OFFSCREEN=1) and a fresh, empty data folder,
so every picture shows the default Steel theme and layout. Pictures land in
docs/guide/src/images/ as PNG.

Shots with markers get numbered callouts drawn on them; that needs Pillow
(the icon generator's venv has it: tools/operandi-icons/.venv/bin/python).
Without Pillow the plain capture is kept and the script says so.

Only Operandi itself is captured, on the guide's own scenes in
docs/guide/scenes/ (original or CC0 content only).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GUIDE = ROOT / "docs" / "guide"
IMAGES = GUIDE / "src" / "images"

ACCENT = "#ff9f1c"
OUTLINE = "#1a1a1a"
BACKGROUND = "#17181b"
RADIUS = 14  # callout circle radius, logical points
FONT_SIZE = 17


def target_dir():
    meta = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    )
    return Path(json.loads(meta.stdout)["target_directory"])


def build():
    subprocess.run(
        ["cargo", "build", "--profile", "qa", "-p", "operandi-app",
         "--features", "cycles,usd,qa"],
        cwd=ROOT, check=True,
    )


def ppm_to_png(ppm, png):
    try:
        from PIL import Image
        Image.open(ppm).save(png, optimize=True)
    except ImportError:
        subprocess.run(["sips", "-s", "format", "png", str(ppm), "--out", str(png)],
                       check=True, capture_output=True)


def find_font(size):
    from PIL import ImageFont
    for f in ("/System/Library/Fonts/Supplemental/Arial Bold.ttf",
              "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
              "C:/Windows/Fonts/arialbd.ttf"):
        if Path(f).exists():
            return ImageFont.truetype(f, size)
    return ImageFont.load_default()


def draw_callouts(png, callouts, scale, margin=0):
    """Numbered markers: a circle with the number at `at`, a line to
    `target`, and an optional underline `span` = [x0, x1, y]. Logical points
    from the capture's top-left; `margin` points of border are added round
    the capture first, so markers can sit outside it."""
    from PIL import Image, ImageDraw
    shot = Image.open(png).convert("RGBA")
    m = int(margin * scale)
    base = Image.new("RGBA", (shot.width + 2 * m, shot.height + 2 * m), BACKGROUND)
    base.paste(shot, (m, m))
    ss = 2  # supersample for smooth circles
    over = Image.new("RGBA", (base.width * ss, base.height * ss), (0, 0, 0, 0))
    d = ImageDraw.Draw(over)
    font = find_font(int(FONT_SIZE * scale * ss))
    k = scale * ss

    def p(x, y):
        return (x + margin) * k, (y + margin) * k

    for c in callouts:
        a, b = p(*c["at"]), p(*c.get("target", c["at"]))
        if "span" in c:
            x0, x1, y = c["span"]
            d.line([p(x0, y), p(x1, y)], fill=ACCENT, width=int(3 * k))
        if a != b:
            d.line([a, b], fill=OUTLINE, width=int(4.5 * k))
            d.line([a, b], fill=ACCENT, width=int(2 * k))
            dot = 4 * k
            d.ellipse([b[0] - dot, b[1] - dot, b[0] + dot, b[1] + dot],
                      fill=ACCENT, outline=OUTLINE, width=int(1.5 * k))
        r = RADIUS * k
        d.ellipse([a[0] - r, a[1] - r, a[0] + r, a[1] + r],
                  fill=ACCENT, outline=OUTLINE, width=int(1.5 * k))
        d.text(a, str(c["n"]), font=font, fill="#000000", anchor="mm")
    over = over.resize(base.size, Image.LANCZOS)
    Image.alpha_composite(base, over).convert("RGB").save(png, optimize=True)


def shoot(binary, shot, defaults, keep):
    w, h = shot.get("size", defaults["size"])
    scale = shot.get("scale", defaults["scale"])
    out = IMAGES / shot["output"]
    work = Path(tempfile.mkdtemp(prefix="guide-shot-"))
    prefix = work / "capture"
    env = os.environ | {
        "OPERANDI_QA_OFFSCREEN": "1",
        "OPERANDI_QA_SIZE": f"{int(w * scale)}x{int(h * scale)}",
        "OPERANDI_QA_SCALE": str(scale),
        "OPERANDI_QA_CAPTURE": str(prefix),
        "OPERANDI_DEV_DATA": str(work / "data"),
        "OPERANDI_QA_TOOL": shot.get("tool", "None"),
    }
    if shot.get("selection", "none") == "none":
        env["OPERANDI_QA_KEEP_SELECTION"] = "1"
        env["OPERANDI_QA_CLEAR_SELECTION"] = "1"
    env.update({k: str(v) for k, v in shot.get("env", {}).items()})
    scene = GUIDE / shot["scene"]
    proc = subprocess.run([str(binary), str(scene)], env=env,
                          capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        print(f"{shot['output']}: FAILED\n{proc.stdout}{proc.stderr}", file=sys.stderr)
        return False
    IMAGES.mkdir(parents=True, exist_ok=True)
    ppm_to_png(Path(str(prefix) + ".ppm"), out)
    if keep:
        shutil.copyfile(str(prefix) + ".json", out.with_suffix(".json"))
    markers = shot.get("markers")
    if markers:
        try:
            draw_callouts(out, markers["list"], scale, markers.get("margin", 0))
        except ImportError:
            print(f"{shot['output']}: no Pillow, callouts not drawn", file=sys.stderr)
    shutil.rmtree(work, ignore_errors=True)
    print(f"{shot['output']}: ok")
    return True


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--only", help="render only the shot whose output contains this")
    ap.add_argument("--binary", help="use this QA binary instead of target/qa/operandi-app")
    ap.add_argument("--keep-json", action="store_true",
                    help="keep the harness's .json record beside each picture (for placing callouts)")
    args = ap.parse_args()
    shots = tomllib.loads((GUIDE / "shots.toml").read_text())
    if not args.no_build and not args.binary:
        build()
    binary = Path(args.binary) if args.binary else target_dir() / "qa" / "operandi-app"
    ok = True
    for shot in shots["shot"]:
        if args.only and args.only not in shot["output"]:
            continue
        ok &= shoot(binary, shot, shots["defaults"], args.keep_json)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
