# Builds the USD-T4 scene (docs/BEHAVIOR-USD-IMPORT.md, U4): a cube with a
# "Marble" material made of a Noise Texture through a Color Ramp (which
# neither USD Preview Surface nor the MaterialX export carries), and a
# sphere with a plain "Paint" material. Exports usd_t4/usd_t4.usdc with
# the spec's settings and saves usd_t4/usd_t4.blend for the graph export:
#
#   B=/Applications/Blender.app/Contents/MacOS/Blender
#   $B -b --factory-startup --python fixtures/usd/make_usd_t4.py
#   $B -b fixtures/usd/usd_t4/usd_t4.blend -P tools/blender-export/export.py \
#       -- fixtures/usd/usd_t4/usd_t4_graphs.json
#   rm fixtures/usd/usd_t4/usd_t4.blend
#
# Made with Blender 5.2.2.
import os

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "usd_t4")
os.makedirs(OUT, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)

bpy.ops.mesh.primitive_cube_add(size=1, location=(0, 0, 0.5))
cube = bpy.context.active_object
cube.name = "MarbleCube"
marble = bpy.data.materials.new("Marble")
marble.use_nodes = True
nt = marble.node_tree
noise = nt.nodes.new("ShaderNodeTexNoise")
noise.inputs["Scale"].default_value = 8.0
ramp = nt.nodes.new("ShaderNodeValToRGB")
ramp.color_ramp.elements[0].color = (0.05, 0.05, 0.08, 1)
ramp.color_ramp.elements[1].color = (0.9, 0.85, 0.8, 1)
nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
nt.links.new(ramp.outputs["Color"], nt.nodes["Principled BSDF"].inputs["Base Color"])
cube.data.materials.append(marble)

bpy.ops.mesh.primitive_uv_sphere_add(radius=0.4, location=(1.5, 0, 0.4))
sphere = bpy.context.active_object
sphere.name = "PaintBall"
paint = bpy.data.materials.new("Paint")
paint.use_nodes = True
paint.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.8, 0.1, 0.1, 1)
sphere.data.materials.append(paint)

bpy.ops.wm.save_as_mainfile(filepath=os.path.join(OUT, "usd_t4.blend"))
bpy.ops.wm.usd_export(
    filepath=os.path.join(OUT, "usd_t4.usdc"),
    export_materials=True,
    export_uvmaps=True,
    export_normals=True,
    use_instancing=True,
    evaluation_mode="RENDER",
    generate_preview_surface=True,
    generate_materialx_network=True,
    export_textures_mode="NEW",
    relative_paths=True,
)
print("Wrote", OUT)
