"""Checks on serialized production SVGs, independent of the drawing primitives."""

from copy import deepcopy
from io import BytesIO
from math import isfinite
import re
import xml.etree.ElementTree as ET

from .conventions import PALETTE, STROKES

SVG = "http://www.w3.org/2000/svg"
NUMBER = re.compile(r"[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?")
CUBIC = re.compile(r"M\s*" + r"[\s,]+".join([NUMBER.pattern] * 2)
                   + r"\s*C\s*" + r"[\s,]+".join([NUMBER.pattern] * 6) + r"\s*")
SHAPES = {"polygon", "polyline", "path", "circle"}


def validate_svg(source: str, *, check_bounds: bool = True):
    """Raise ValueError on a broken convention; bounds include actual stroked ink."""
    try:
        root = ET.fromstring(source)
    except ET.ParseError as error:
        raise ValueError("Invalid SVG XML") from error
    if root.tag != f"{{{SVG}}}svg":
        raise ValueError("Expected an SVG namespace root")
    if root.get("viewBox") != "0 0 32 32":
        raise ValueError("Expected a 32×32 viewBox")
    if (root.get("width"), root.get("height")) != ("32", "32"):
        raise ValueError("Expected 32×32 default display dimensions")
    for tag in ("title", "desc"):
        element = root.find(f"{{{SVG}}}{tag}")
        if element is None or not (element.text or "").strip():
            raise ValueError(f"Missing accessible {tag}")
    shapes = 0
    for element in root.iter():
        tag = element.tag.removeprefix(f"{{{SVG}}}")
        if tag not in SHAPES | {"svg", "title", "desc", "defs"}:
            raise ValueError(f"Unexpected SVG element: {tag}")
        if any(attr in element.attrib for attr in
               ("style", "transform", "opacity", "fill-opacity", "stroke-opacity")):
            raise ValueError("Styles, transforms and opacity are not production conventions")
        for attr in element.attrib:
            if attr.startswith("on") or "href" in attr:
                raise ValueError("External/active SVG content is forbidden")
        if tag not in SHAPES:
            continue
        shapes += 1
        for attribute in ("fill", "stroke"):
            color = element.get(attribute, "none")
            if color != "none" and color not in PALETTE.values():
                raise ValueError(f"Unapproved {attribute}: {color}")
        if "fill" not in element.attrib:
            raise ValueError("Every shape needs an explicit fill")
        if element.get("stroke", "none") != "none":
            if float(element.get("stroke-width", "0")) not in STROKES:
                raise ValueError("Inconsistent stroke width")
            if any(element.get(attr) != "round" for attr in
                   ("stroke-linecap", "stroke-linejoin")):
                raise ValueError("Expected round caps and joins")
        if tag in {"polygon", "polyline"}:
            raw = element.get("points", "")
            numbers = NUMBER.findall(raw)
            if NUMBER.sub("", raw).strip(" ,\t\n") or len(numbers) % 2:
                raise ValueError("Invalid point list")
            if len(numbers) < (6 if tag == "polygon" else 4):
                raise ValueError("Too few polygon/polyline points")
        elif tag == "path":
            raw = element.get("d", "")
            numbers = NUMBER.findall(raw)
            if not CUBIC.fullmatch(raw):
                raise ValueError("Only one explicit M/C cubic is supported")
        else:
            raw = " ".join(element.get(key, "") for key in ("cx", "cy", "r"))
            numbers = NUMBER.findall(raw)
            if len(numbers) != 3 or float(numbers[-1]) <= 0:
                raise ValueError("Invalid circle")
        if not all(isfinite(float(value)) for value in numbers):
            raise ValueError("Nonfinite geometry")
        coordinates = [float(value) for value in numbers]
        # Conservative control-hull check also catches distant geometry that
        # would lie entirely outside the expanded raster used below.
        if tag == "circle":
            x, y, radius = coordinates
            coordinates = [x - radius, y - radius, x + radius, y + radius]
        if any(value < 0.5 or value > 31.5 for value in coordinates):
            raise ValueError("Geometry exceeds safe canvas bounds")
    if not shapes:
        raise ValueError("Empty icon")
    if check_bounds:
        validate_bounds(root)


def validate_bounds(root: ET.Element):
    # Render an expanded viewBox so clipping at the original canvas cannot hide
    # out-of-bounds paths or rounded caps. 8 samples per final pixel includes AA.
    import cairosvg
    from PIL import Image

    padded = deepcopy(root)
    padded.set("viewBox", "-4 -4 40 40")
    padded.set("width", "40")
    padded.set("height", "40")
    png = cairosvg.svg2png(bytestring=ET.tostring(padded),
                          output_width=320, output_height=320)
    alpha = Image.open(BytesIO(png)).convert("RGBA").getchannel("A")
    box = alpha.getbbox()
    if box is None:
        raise ValueError("Icon has no visible ink")
    # Require a half-pixel minimum clear margin inside the original canvas.
    if box[0] < 36 or box[1] < 36 or box[2] > 284 or box[3] > 284:
        raise ValueError(f"Drawing/stroke exceeds safe canvas bounds: {box}")
    # A normal render must also be transparent at every corner.
    normal = Image.open(BytesIO(cairosvg.svg2png(bytestring=ET.tostring(root))))
    normal = normal.convert("RGBA")
    if any(normal.getpixel(p)[3] for p in ((0, 0), (31, 0), (0, 31), (31, 31))):
        raise ValueError("Expected transparent canvas corners")
