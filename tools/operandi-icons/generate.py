#!/usr/bin/env python3
"""Generate independent icons, validate them, and optionally build review sheets."""

import argparse
from pathlib import Path

from operandi_icons.icons import ICONS, drawing
from operandi_icons.validation import validate_svg

HERE = Path(__file__).resolve().parent


def generate(output: Path, *, svg_only: bool = False, only: list[str] | None = None):
    names = only or list(ICONS)
    unknown = [name for name in names if name not in ICONS]
    if unknown:
        raise SystemExit(f"Unknown icon(s): {', '.join(unknown)}")
    sources = {}
    destination = output / "svg"
    destination.mkdir(parents=True, exist_ok=True)
    for name in names:
        source = drawing(name).tostring() + "\n"
        validate_svg(source, check_bounds=not svg_only)
        (destination / f"{name}.svg").write_text(source, encoding="utf-8")
        sources[name] = source.encode("utf-8")
    if not svg_only:
        from operandi_icons.previews import generate_previews
        generate_previews(output, sources, HERE / "reference" / "svg")
    return sources


DEPRECATION = (
    "warning: the flat icon generator is deprecated (Jason, 2026-11-20). New\n"
    "icons are rendered with the icon studio: see tools/icon-studio/README.md.\n"
    "This tool only keeps the existing SVGs.")
APP_ICONS = HERE.parent.parent / "crates" / "operandi-app" / "assets" / "icons"


def new_icons(names):
    """The icons that aren't in the app yet: new ones, which belong in the
    icon studio instead."""
    return [name for name in names if not (APP_ICONS / f"{name}.svg").is_file()]


if __name__ == "__main__":
    import sys
    print(DEPRECATION, file=sys.stderr)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=HERE / "generated")
    parser.add_argument("--svg-only", action="store_true",
                        help="Only svgwrite required; skip raster bounds checks and previews")
    parser.add_argument("--only", help="Comma-separated icon names (default: all)")
    args = parser.parse_args()
    only = args.only.split(",") if args.only else None
    sources = generate(args.output, svg_only=args.svg_only, only=only)
    print(f"Generated {len(sources)} icons in {args.output.resolve()}")
    fresh = new_icons(sources)
    if fresh:
        print("warning: not in the app yet, so new: " + ", ".join(fresh)
              + "\nRender new icons with tools/icon-studio instead.", file=sys.stderr)
