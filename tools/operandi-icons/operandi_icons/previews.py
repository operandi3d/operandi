"""Review-only renders. Production drawing never imports this module."""

from io import BytesIO
from pathlib import Path

import cairosvg
from PIL import Image, ImageDraw, ImageFont
import svgwrite

from .icons import ICONS, drawing

BACKGROUND = {"light": "#F8FAFB", "dark": "#20262E"}
FOREGROUND = {"light": "#263442", "dark": "#DCE3E9"}
PREVIEW_SIZE = (1200, 850)


def raster(source: bytes, size: int) -> Image.Image:
    return Image.open(BytesIO(cairosvg.svg2png(
        bytestring=source, output_width=size, output_height=size))).convert("RGBA")


def text(draw, location, value, size, color, anchor=None):
    draw.text(location, value, font=ImageFont.load_default(size=size),
              fill=color, anchor=anchor)


COLUMNS = 6


def contact_sheet(output: Path, names: list[str]):
    """Enlarged vector sheet of `names`, in a grid that grows with the set."""
    rows = (len(names) + COLUMNS - 1) // COLUMNS
    width, height = 960, 150 + rows * 200
    sheet = svgwrite.Drawing(size=(width, height), viewBox=f"0 0 {width} {height}")
    sheet.set_desc(title="Operandi — programmatic icons",
                   desc="Generated original icons, enlarged for visual review.")
    sheet.add(sheet.rect((0, 0), (width, height), fill=BACKGROUND["light"]))
    def label(value, position, size, color="#263442", **kwargs):
        sheet.add(sheet.text(value, insert=position, font_size=size,
                             font_family="sans-serif", fill=color, **kwargs))
    label("OPERANDI / ICONS", (40, 54), 25, font_weight="bold")
    label("Programmatic SVG · visual review · transparent 32×32 masters", (40, 83), 15)
    for index, name in enumerate(names):
        title = ICONS[name][1]
        x, y = 90 + (index % COLUMNS) * 156, 170 + (index // COLUMNS) * 200
        group = sheet.g(transform=f"translate({x - 48},{y - 48}) scale(3)")
        for element in drawing(name).elements:
            if element.elementname in {"polygon", "polyline", "path", "circle"}:
                group.add(element)
        sheet.add(group)
        label(title, (x, y + 72), 17, text_anchor="middle")
    label("Yellow-orange: selected     Cyan: new geometry     Red / Green / Blue: X / Y / Z",
          (40, height - 24), 15)
    svg_path = output / "contact-sheet.svg"
    svg_path.write_text(sheet.tostring() + "\n", encoding="utf-8")
    cairosvg.svg2png(bytestring=svg_path.read_bytes(),
                    write_to=str(output / "contact-sheet.png"))


def review_sheets(output: Path, sources: dict):
    """Native 24/32 px and 4x nearest-neighbor detail for every icon, on
    light and dark backgrounds, in a grid that grows with the set."""
    names = list(sources)
    cell_w, cell_h = 190, 210
    rows = (len(names) + COLUMNS - 1) // COLUMNS
    size = (40 + COLUMNS * cell_w, 110 + rows * cell_h)
    for theme, background in BACKGROUND.items():
        sheet = Image.new("RGB", size, background)
        draw = ImageDraw.Draw(sheet)
        foreground = FOREGROUND[theme]
        text(draw, (32, 24), f"OPERANDI / {theme.upper()} REVIEW", 24, foreground)
        text(draw, (32, 60), "Native 24 px and 32 px, then 32 px at 4x nearest-neighbor",
             16, foreground)
        for index, name in enumerate(names):
            x = 20 + (index % COLUMNS) * cell_w
            y = 100 + (index // COLUMNS) * cell_h
            text(draw, (x + cell_w // 2, y + 8), ICONS[name][1], 15, foreground, "mm")
            small, large = raster(sources[name], 24), raster(sources[name], 32)
            sheet.paste(small, (x + 40, y + 24), small)
            sheet.paste(large, (x + 100, y + 20), large)
            enlarged = large.resize((128, 128), Image.Resampling.NEAREST)
            sheet.paste(enlarged, (x + (cell_w - 128) // 2, y + 64), enlarged)
        sheet.save(output / f"review-{theme}.png")


def comparisons(output: Path, sources: dict, reference: Path):
    """Approved-original versus generated, only for icons with an approved
    reference SVG (the original six)."""
    named = [name for name in sources if (reference / f"{name}.svg").is_file()]
    if not named:
        return
    for theme, background in BACKGROUND.items():
        sheet = Image.new("RGB", PREVIEW_SIZE, background)
        draw = ImageDraw.Draw(sheet)
        foreground = FOREGROUND[theme]
        text(draw, (32, 24), f"OPERANDI / {theme.upper()} COMPARISON", 24, foreground)
        text(draw, (32, 60), "A = approved    G = generated    Native pixels + 4x nearest-neighbor detail",
             16, foreground)
        for index, name in enumerate(named):
            title = ICONS[name][1]
            center = 230 + index * 175
            text(draw, (center, 110), title, 16, foreground, "mm")
            for delta, letter in ((-37, "A"), (37, "G")):
                text(draw, (center + delta, 138), letter, 13, foreground, "mm")
            for size, y in ((24, 180), (32, 470)):
                native_a = raster((reference / f"{name}.svg").read_bytes(), size)
                native_g = raster(sources[name], size)
                for delta, icon in ((-37, native_a), (37, native_g)):
                    sheet.paste(icon, (center + delta - size // 2, y), icon)
                # Enlarged A/G are stacked to avoid squeezing 128px details.
                for row, icon in enumerate((native_a, native_g)):
                    enlarged = icon.resize((size * 4, size * 4), Image.Resampling.NEAREST)
                    top = y + 53 + row * (size * 4 + 12)
                    sheet.paste(enlarged, (center - size * 2, top), enlarged)
        for size, y in ((24, 180), (32, 470)):
            text(draw, (32, y + 5), f"{size} px / native", 16, foreground)
            text(draw, (32, y + 65), "4x approved", 14, foreground)
            text(draw, (32, y + 65 + size * 4 + 12), "4x generated", 14, foreground)
            if size == 24:
                draw.line((32, 438, 1168, 438), fill=foreground, width=1)
        text(draw, (32, 825), "Open at 100% to judge native sizes. Artwork colors are identical on both backgrounds.",
             14, foreground)
        sheet.save(output / f"comparison-{theme}.png")


def generate_previews(output: Path, sources: dict, reference: Path):
    previews = output / "previews"
    previews.mkdir(parents=True, exist_ok=True)
    for size in (24, 32):
        destination = output / "png" / str(size)
        destination.mkdir(parents=True, exist_ok=True)
        for name, source in sources.items():
            raster(source, size).save(destination / f"{name}.png")
    contact_sheet(previews, list(sources))
    review_sheets(previews, sources)
    comparisons(previews, sources, reference)
