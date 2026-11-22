# Imports a USD file into an empty Blender scene and prints, as one JSON
# line starting "READBACK", what came in: objects by type, polygons, the
# world box (Blender's Z-up), the hierarchy depth, UV maps, creased edges,
# materials and their image textures. Used to check Operandi's USD export
# (docs/BEHAVIOR-USD-IMPORT.md, U6):
#
#   Blender -b --factory-startup --python fixtures/usd/blender_readback.py -- file.usdc
import json
import sys

import bpy
from mathutils import Vector

path = sys.argv[sys.argv.index("--") + 1]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.usd_import(filepath=path)
deps = bpy.context.evaluated_depsgraph_get()
out = {"objects": {}, "polygons": 0, "uv_maps": {}, "creased_edges": 0, "materials": {}}
lo, hi = Vector((1e9,) * 3), Vector((-1e9,) * 3)
depth = 0
for ob in bpy.context.scene.objects:
    out["objects"][ob.type] = out["objects"].get(ob.type, 0) + 1
    d, p = 0, ob.parent
    while p:
        d, p = d + 1, p.parent
    depth = max(depth, d)
    if ob.type != "MESH":
        continue
    me = ob.data
    out["polygons"] += len(me.polygons)
    out["uv_maps"][ob.name] = [uv.name for uv in me.uv_layers]
    crease = me.attributes.get("crease_edge")
    if crease:
        out["creased_edges"] += sum(1 for c in crease.data if c.value > 0)
    for v in me.vertices:
        w = ob.matrix_world @ v.co
        lo = Vector(map(min, lo, w))
        hi = Vector(map(max, hi, w))
    for slot in ob.material_slots:
        m = slot.material
        if m and m.name not in out["materials"]:
            images = []
            if m.node_tree:
                images = sorted(
                    n.image.name for n in m.node_tree.nodes if n.type == "TEX_IMAGE" and n.image
                )
            out["materials"][m.name] = images
out["depth"] = depth
out["box"] = [[round(x, 4) for x in lo], [round(x, 4) for x in hi]]
out["lights"] = sorted(
    (ob.data.type, round(ob.data.energy, 2)) for ob in bpy.context.scene.objects if ob.type == "LIGHT"
)
out["cameras"] = [round(ob.data.lens, 2) for ob in bpy.context.scene.objects if ob.type == "CAMERA"]
print("READBACK " + json.dumps(out))
