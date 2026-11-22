"""Serialized artwork contracts and failures, plus a real clean generation."""

from io import BytesIO
from math import hypot, sqrt
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from generate import generate
from operandi_icons.conventions import PALETTE, Projection, STROKES
from operandi_icons.icons import ICONS, drawing
from operandi_icons.previews import BACKGROUND, PREVIEW_SIZE, raster
from operandi_icons.primitives import Canvas
from operandi_icons.validation import SVG, validate_svg

# The originally approved six, plus the primitives batch. The full set grows
# batch by batch; these must always be present.
BASELINE = {"extrude", "bevel", "inset", "loop-cut", "move", "rotate"}
PRIMITIVES = {"cube", "sphere", "ellipsoid", "cylinder", "cone", "torus", "capsule"}
# Batch 2 (Polygon tab, plus Scale/Transform), each with its color cue.
POLYGON = {
    "chamfer-edit": ("new",), "poly-haul": ("selected",),
    "sketch-extrude": ("selected", "new"), "smooth-shift": ("selected", "new"),
    "thicken": ("new",), "pen": ("new",), "spike": ("new",), "bridge": ("new",),
    "poly-reduct": ("new",), "triple": ("new",), "script-action": ("new",),
    "scale": ("x", "y", "z"), "transform": ("x", "y", "z"),
}
# Batch 3 (Edge tab). Add Loop reuses the original Loop Cut icon.
EDGE = {
    "edge": ("selected",), "edge-bevel": ("new",), "edge-chamfer": ("new",),
    "edge-extend": ("new", "selected"), "edge-extrude": ("new", "selected"),
    "edge-slide": ("selected",), "edge-relax": ("new",),
    "edge-crease": ("selected", "new"), "edge-add-point": ("new",),
}
# Batch 4 (Vertex tab).
VERTEX = {
    "vertex": ("selected",), "vertex-create": ("new",), "vertex-bevel": ("new",),
    "vertex-extrude": ("new", "selected"), "vertex-merge": ("selected", "new"),
    "vertex-weld": ("selected",), "vertex-tear": ("selected",),
    "vertex-slide": ("selected",),
}
# Batch 5 (workspace symbols).
UI = {
    "polygon": ("selected",), "eye": ("ink",), "disclosure": ("left",),
    "action-center": ("new",), "falloff": ("top", "left", "right"),
    "symmetry": ("new",), "snapping": ("x", "z"), "workplane": ("x", "y"),
}


# Batch 6 (Draw tools): Arc and Tube mark what they make with the new-geometry cyan.
DRAW = {"arc": ("new",), "tube": ("new", "top", "left", "right")}

# Batch 8 (Deform tab, Edit tab Slice tools, Shift), each with its cue.
DEFORM = {
    "element-move": ("new",), "element-push": ("new",), "flex": ("top", "left", "right"),
    "soft-drag": ("new",), "soft-move": ("z",), "shear": ("x",), "smooth": ("new",),
    "jitter": ("new",), "quantize": ("top", "left", "right"),
    "soft-selection-move": ("selected", "x", "y", "z"),
    "soft-selection-rotate": ("selected", "z"),
    "soft-selection-scale": ("selected", "x", "y", "z"),
    "soft-selection-transform": ("selected", "x", "y", "z"),
    "twist": ("top", "left", "right"), "bend": ("top", "left", "right"),
    "vortex": ("new",), "swirl": ("new",), "push": ("new",), "sculpt": ("new",),
    "taper": ("top", "left", "right"), "bulge": ("top", "left", "right"),
    "flare": ("top", "left", "right"), "radial-align": ("selected", "new"),
    "linear-align": ("selected", "new"), "loop-slice": ("new",), "edge-slice": ("new",),
    "polygon-slice": ("new",), "shift": ("selected",),
}

# Batch 9 (Scale Offset, Scale Offset Detach, Flatten Ngons; tag types).
TAG = {
    "scale-offset": ("selected",), "scale-offset-detach": ("selected", "new"),
    "flatten-ngons": ("selected", "new"), "tag-material": ("selected",),
    "tag-part": ("selected",), "tag-smoothing-group": ("selected",),
    "tag-selection-set": ("selected", "new"),
}


# Batch 10 (Edit tab Subdivide group and Freeze), each with its cyan cue.
SUBDIVIDE = {
    "sds-subdivide": ("new",), "subdivide-faceted": ("new",), "subdivide-smooth": ("new",),
    "subdivide-options": ("new",), "unsubdivide": ("new",), "freeze": ("new",),
}


# Batch 11 (Edit tab path and axis Slice tools): the cut is cyan.
PATH_SLICE = {
    "pen-slice": ("new",), "curve-slice": ("new",), "primitive-slice": ("new",),
    "axis-slice": ("new",), "dicer": ("new",), "julienne": ("new",),
}


# Batch 12 (Sculpt brushes; Duplicate tab tools and Detriangulate). The
# brushes show the clay's own tones (only Move adds a cyan drag arrow); the
# copies' tops and the other cues are cyan.
SCULPT_DUPLICATE = {
    "sculpt-smooth": ("top", "left", "right"), "sculpt-push": ("top", "left", "right"),
    "sculpt-inflate": ("top", "left", "right"), "sculpt-move": ("new",),
    "radial-sweep": ("new",), "clone": ("new", "top"),
    "array": ("new", "top"), "radial-array": ("new", "top"), "mirror": ("new", "top"),
    "detriangulate": ("new",),
}


# Batch 13 (Environment item): blue sky over grey ground.
ENVIRONMENT = {"environment": ("z", "left")}

# Batch 14 (Snaps and Precision panel): cyan snap targets, axis-coloured cues.
SNAPS = {
    "drag-snap": ("new",), "drag-snap-rigid": ("new",), "planar-rotate-snap": ("new", "z"),
    "planar-rotate-snap-axis": ("new", "z"), "scale-absolute": ("new", "x"),
    "size-absolute": ("new", "x"), "advanced-handles": ("z", "selected"),
    "absolute-scaling": ("x", "y", "z"),
}


class IconTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.output = Path(cls.temp.name)
        cls.sources = generate(cls.output)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def root(self, name="inset"):
        return ET.fromstring(self.sources[name])

    def rejected(self, mutate, message):
        root = self.root()
        mutate(root)
        with self.assertRaisesRegex(ValueError, message):
            validate_svg(ET.tostring(root).decode())

    def test_every_registered_icon_is_an_independent_valid_master(self):
        self.assertLessEqual(BASELINE | PRIMITIVES | set(POLYGON) | set(EDGE) | set(VERTEX) | set(UI) | set(DRAW)
                             | set(DEFORM) | set(SUBDIVIDE) | set(PATH_SLICE) | set(SCULPT_DUPLICATE)
                             | set(ENVIRONMENT) | set(SNAPS),
                             set(ICONS))
        self.assertEqual({path.stem for path in (self.output / "svg").glob("*.svg")}, set(ICONS))
        for name, source in self.sources.items():
            with self.subTest(icon=name):
                validate_svg(source.decode())
                self.assertEqual((self.output / "svg" / f"{name}.svg").read_bytes(), source)

    def test_deterministic_masters(self):
        for name, source in self.sources.items():
            self.assertEqual(source, (drawing(name).tostring() + "\n").encode())

    def test_svg_generation_does_not_read_reference_art(self):
        with tempfile.TemporaryDirectory() as destination:
            with patch.object(Path, "read_bytes", side_effect=AssertionError("Reference read")):
                sources = generate(Path(destination), svg_only=True)
            self.assertEqual(sources, self.sources)

    def test_one_equal_length_isometric_projection(self):
        projection = Projection()
        origin = projection((0, 0, 0))
        axes = [tuple(b - a for a, b in zip(origin, projection(endpoint)))
                for endpoint in ((1, 0, 0), (0, 1, 0), (0, 0, 1))]
        for axis in axes:
            self.assertAlmostEqual(hypot(*axis), 12.9)
        for a, b in ((axes[0], axes[1]), (axes[1], axes[2]), (axes[0], axes[2])):
            self.assertAlmostEqual(sum(x * y for x, y in zip(a, b)) / 12.9**2, -0.5)
        self.assertAlmostEqual(axes[0][1] / axes[0][0], 1 / sqrt(3))

    def test_palette_and_stroke_system_are_used(self):
        # Each icon is checked against the palette by validate_svg; the
        # original six together exercise every color and stroke width.
        colors, widths = set(), set()
        for name, source in self.sources.items():
            if name not in BASELINE:
                continue
            for element in ET.fromstring(source).iter():
                for key in ("fill", "stroke"):
                    if element.get(key, "none") != "none":
                        colors.add(element.get(key))
                if element.get("stroke-width"):
                    widths.add(float(element.get("stroke-width")))
        self.assertEqual(colors, set(PALETTE.values()))
        self.assertEqual(widths, {0.96, 1.12, 0.72, 1.7})  # 1.7: gizmo axes
        self.assertLessEqual(widths, STROKES)

    def test_operation_cues(self):
        for name in ("extrude", "inset"):
            self.assertIn(PALETTE["selected"].encode(), self.sources[name])
        for name in ("bevel", "loop-cut"):
            self.assertIn(PALETTE["new"].encode(), self.sources[name])
        for name in ("move", "rotate"):
            for axis in ("x", "y", "z"):
                self.assertIn(PALETTE[axis].encode(), self.sources[name])
        self.assertEqual(len(self.root("rotate").findall(f"{{{SVG}}}path")), 3)

    def test_primitives_use_the_three_cube_tones(self):
        for name in PRIMITIVES:
            with self.subTest(icon=name):
                for tone in ("top", "left", "right", "ink"):
                    if name == "cone" and tone == "top":
                        continue  # a cone has no lit top
                    self.assertIn(PALETTE[tone].encode(), self.sources[name])
                for cue in ("selected", "new", "x", "y", "z"):
                    self.assertNotIn(PALETTE[cue].encode(), self.sources[name])

    def test_polygon_batch_cues(self):
        for name, cues in {**POLYGON, **EDGE, **VERTEX, **UI, **DRAW, **DEFORM, **TAG, **SUBDIVIDE,
                           **PATH_SLICE, **SCULPT_DUPLICATE, **ENVIRONMENT,
                           **SNAPS}.items():
            with self.subTest(icon=name):
                for cue in cues:
                    self.assertIn(PALETTE[cue].encode(), self.sources[name])

    def test_create_batch_reads_on_the_dark_theme(self):
        """Batch 7 rule (and Arc's fix): no icon is just ink lines, which
        vanish on the dark background. Count clearly light pixels at 32 px."""
        from operandi_icons.create_icons import CREATE_ICONS
        from operandi_icons.deform_icons import DEFORM_ICONS
        from operandi_icons.tag_icons import TAG_ICONS
        from operandi_icons.path_slice_icons import PATH_SLICE_ICONS
        for name in (list(CREATE_ICONS) + ["arc"] + list(DEFORM_ICONS) + list(TAG_ICONS)
                     + list(PATH_SLICE_ICONS) + list(SCULPT_DUPLICATE) + list(ENVIRONMENT) + list(SNAPS)):
            with self.subTest(name):
                image = raster(self.sources[name], 32).convert("RGBA")
                dark = Image.new("RGBA", image.size, BACKGROUND["dark"])
                dark.alpha_composite(image)
                light = sum(1 for p in dark.convert("L").getdata() if p > 90)
                self.assertGreater(light, 60)

    def test_paint_batch_reads_on_the_dark_theme_and_is_all_new(self):
        """Batch 20 (painting): every icon keeps enough light pixels on the
        dark background, and none repeats another icon's artwork."""
        from operandi_icons.paint_icons import PAINT_ICONS
        bodies = {}
        for name, source in self.sources.items():
            bodies.setdefault(source.split(b"</desc>", 1)[1], []).append(name)
        for name in PAINT_ICONS:
            with self.subTest(name):
                image = raster(self.sources[name], 32).convert("RGBA")
                dark = Image.new("RGBA", image.size, BACKGROUND["dark"])
                dark.alpha_composite(image)
                light = sum(1 for p in dark.convert("L").getdata() if p > 90)
                self.assertGreater(light, 60)
                self.assertEqual(bodies[self.sources[name].split(b"</desc>", 1)[1]], [name])

    def test_deform_batch_icons_are_all_different(self):
        """Every tool in the batch has its own artwork, and none repeats an
        existing icon."""
        from operandi_icons.deform_icons import DEFORM_ICONS
        bodies = {}
        for name, source in self.sources.items():
            body = source.split(b"</desc>", 1)[1]
            bodies.setdefault(body, []).append(name)
        from operandi_icons.tag_icons import TAG_ICONS
        from operandi_icons.path_slice_icons import PATH_SLICE_ICONS
        for name in (list(DEFORM_ICONS) + list(TAG_ICONS) + list(PATH_SLICE_ICONS)
                     + list(SCULPT_DUPLICATE) + list(ENVIRONMENT) + list(SNAPS)):
            body = self.sources[name].split(b"</desc>", 1)[1]
            self.assertEqual(bodies[body], [name])

    def test_torus_hole_is_clear(self):
        image = raster(self.sources["torus"], 128)
        self.assertEqual(image.getpixel((64, 64))[3], 0)  # the hole
        self.assertGreater(image.getpixel((64, 100))[3], 0)  # the front of the ring

    def test_only_generates_a_subset(self):
        with tempfile.TemporaryDirectory() as destination:
            sources = generate(Path(destination), svg_only=True, only=["cone", "cube"])
            self.assertEqual(set(sources), {"cone", "cube"})
            self.assertEqual({p.stem for p in (Path(destination) / "svg").glob("*.svg")},
                             {"cone", "cube"})
            with self.assertRaises(SystemExit):
                generate(Path(destination), svg_only=True, only=["no-such-icon"])

    def test_transparent_native_renders_and_preview_artifacts(self):
        for size in (24, 32):
            for name in ICONS:
                with self.subTest(icon=name, size=size):
                    path = self.output / "png" / str(size) / f"{name}.png"
                    with Image.open(path) as image:
                        self.assertEqual(image.size, (size, size))
                        self.assertEqual(image.mode, "RGBA")
                        self.assertIsNotNone(image.getchannel("A").getbbox())
                        self.assertEqual(image.getpixel((0, 0))[3], 0)
                    self.assertEqual(path.read_bytes(), self._png(self.sources[name], size))
        for theme, background in BACKGROUND.items():
            with Image.open(self.output / "previews" / f"comparison-{theme}.png") as image:
                self.assertEqual(image.size, PREVIEW_SIZE)
                self.assertEqual(image.getpixel((0, 0)),
                                 tuple(bytes.fromhex(background[1:])))
            self.assertTrue((self.output / "previews" / f"review-{theme}.png").is_file())
        for extension in ("svg", "png"):
            self.assertTrue((self.output / "previews" / f"contact-sheet.{extension}").is_file())

    @staticmethod
    def _png(source, size):
        output = BytesIO()
        raster(source, size).save(output, format="PNG")
        return output.getvalue()

    def test_rejects_wrong_canvas(self):
        self.rejected(lambda root: root.set("viewBox", "0 0 64 64"), "viewBox")

    def test_rejects_off_palette_color(self):
        self.rejected(lambda root: root.find(f"{{{SVG}}}polygon").set("fill", "#FFFFFF"),
                      "Unapproved fill")

    def test_rejects_wrong_stroke_width(self):
        self.rejected(lambda root: root.find(f"{{{SVG}}}polyline").set("stroke-width", "2"),
                      "stroke width")

    def test_rejects_external_content(self):
        self.rejected(lambda root: ET.SubElement(root, f"{{{SVG}}}image", href="external.png"),
                      "Unexpected SVG element")

    def test_rejects_far_geometry_even_outside_overscan(self):
        self.rejected(lambda root: root.find(f"{{{SVG}}}polygon").set(
            "points", "1000,1000 1010,1000 1010,1010"), "safe canvas bounds")

    def test_rejects_stroke_that_crosses_margin(self):
        self.rejected(lambda root: ET.SubElement(root, f"{{{SVG}}}polyline", {
            "points": "0.6,10 0.6,20", "fill": "none", "stroke": PALETTE["ink"],
            "stroke-width": "0.96", "stroke-linecap": "round", "stroke-linejoin": "round",
        }), "Drawing/stroke exceeds")

    def test_rejects_missing_accessible_title(self):
        self.rejected(lambda root: root.remove(root.find(f"{{{SVG}}}title")), "title")

    def test_rejects_malformed_xml(self):
        with self.assertRaisesRegex(ValueError, "Invalid SVG XML"):
            validate_svg("<svg>")

    def test_arrow_rejects_zero_tangent(self):
        with self.assertRaisesRegex(ValueError, "nonzero terminal tangent"):
            Canvas("Test", "Test").arrow((10, 10), (10, 10), "x")


if __name__ == "__main__":
    unittest.main()


class PathSliceViewSwitchTests(unittest.TestCase):
    def test_both_views_draw_every_icon(self):
        """The tilted/flat switch (VIEW) yields valid, different artwork."""
        from operandi_icons import path_slice_icons
        saved = path_slice_icons.VIEW
        try:
            out = {}
            for view in ("tilted", "flat"):
                path_slice_icons.VIEW = view
                out[view] = {name: drawing(name).tostring() for name in path_slice_icons.PATH_SLICE_ICONS}
                for source in out[view].values():
                    validate_svg(source)
        finally:
            path_slice_icons.VIEW = saved
        for name in out["flat"]:
            self.assertNotEqual(out["flat"][name], out["tilted"][name])
