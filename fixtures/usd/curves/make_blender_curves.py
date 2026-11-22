# Builds fixtures/usd/curves/blender_curves.usda in Blender: a Bezier
# circle, a NURBS path and a poly line, exported as USD curves (USD-IQ24).
#
#   /Applications/Blender.app/Contents/MacOS/Blender -b --factory-startup \
#       --python fixtures/usd/curves/make_blender_curves.py
#
# Made with Blender 5.2.2, from scratch.
import os

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.environ.get("CURVES_OUT") or os.path.join(HERE, "blender_curves.usda")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.curve.primitive_bezier_circle_add(radius=1, location=(0, 0, 0))
bpy.context.object.name = "Circle"
bpy.ops.curve.primitive_nurbs_path_add(location=(0, 3, 0))
bpy.context.object.name = "Path"
data = bpy.data.curves.new("Poly", "CURVE")
spline = data.splines.new("POLY")
spline.points.add(3)
for i, p in enumerate([(0, 0, 0, 1), (1, 0, 0, 1), (1, 1, 0, 1), (2, 1, 1, 1)]):
    spline.points[i].co = p
ob = bpy.data.objects.new("Poly", data)
ob.location = (0, -3, 0)
bpy.context.scene.collection.objects.link(ob)
bpy.ops.wm.usd_export(filepath=OUT, export_curves=True) if "export_curves" in {
    p.identifier for p in bpy.ops.wm.usd_export.get_rna_type().properties
} else bpy.ops.wm.usd_export(filepath=OUT)
