# Builds fixtures/usd/foreign/blender_extras.usda in Blender: a cube with
# two vertex groups, two shape keys, a point colour, a corner colour, a
# float, an int and a vector attribute, exported with shape keys on.
# Checks USD-IQ23 (other programs' maps, sets and blend shapes on import).
#
#   /Applications/Blender.app/Contents/MacOS/Blender -b --factory-startup \
#       --python fixtures/usd/make_foreign_extras.py
#
# Made with Blender 5.2.2, from scratch. Blender writes vertex groups as
# float primvars per point (no customData), attributes as primvars of
# their type, and shape keys as UsdSkelBlendShapes bound to the mesh with
# offsets and pointIndices.
import os

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.environ.get("FOREIGN_OUT") or os.path.join(HERE, "foreign", "blender_extras.usda")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_cube_add(size=2)
ob = bpy.context.object
ob.name = "Cube"
me = ob.data
top = ob.vertex_groups.new(name="top")
side = ob.vertex_groups.new(name="side")
for v in me.vertices:
    if v.co.z > 0:
        top.add([v.index], 0.75, "REPLACE")
    if v.co.x > 0:
        side.add([v.index], 0.25 + 0.1 * v.index, "REPLACE")
ob.shape_key_add(name="Basis")
tall = ob.shape_key_add(name="Tall")
wide = ob.shape_key_add(name="Wide")
for v in tall.data:
    if v.co.z > 0:
        v.co.z += 1.0
for v in wide.data:
    if v.co.x > 0:
        v.co.x += 0.5
col = me.color_attributes.new("Col", "FLOAT_COLOR", "POINT")
for i, c in enumerate(col.data):
    c.color = (i / 8, 0.5, 1 - i / 8, 1)
corner = me.color_attributes.new("CornerCol", "FLOAT_COLOR", "CORNER")
for i, c in enumerate(corner.data):
    c.color = (i / 24, 0.25, 0.75, 1)
for i, d in enumerate(me.attributes.new("heat", "FLOAT", "POINT").data):
    d.value = i * 0.5
for i, d in enumerate(me.attributes.new("tag", "INT", "POINT").data):
    d.value = i * 3
for i, d in enumerate(me.attributes.new("dir", "FLOAT_VECTOR", "CORNER").data):
    d.vector = (i, 0, 1)
os.makedirs(os.path.dirname(OUT), exist_ok=True)
bpy.ops.wm.usd_export(filepath=OUT, export_shapekeys=True)
