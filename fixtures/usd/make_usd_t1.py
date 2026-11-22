# Builds the USD-T1 test scene (docs/BEHAVIOR-USD-IMPORT.md) in Blender and
# exports it as usd_t1/usd_t1.usdc and usd_t1.usdz with the export settings
# the spec targets. Everything is made here from scratch (no outside assets).
#
#   /Applications/Blender.app/Contents/MacOS/Blender -b --factory-startup \
#       --python fixtures/usd/make_usd_t1.py
#
# Made with Blender 5.2.2. Pictures are tiny (32 x 32) to keep the repo
# small; their contents only need to be recognisable.
import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
# USD_T1_OUT writes elsewhere (the brightness check renders from a copy).
OUT = os.environ.get("USD_T1_OUT") or os.path.join(HERE, "usd_t1")
os.makedirs(OUT, exist_ok=True)

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
coll = scene.collection


def link(obj, collection=None):
    (collection or coll).objects.link(obj)
    return obj


def mesh_object(name, verts, faces, collection=None):
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    me.update()
    return link(bpy.data.objects.new(name, me), collection)


def box(cx, cy, cz, sx, sy, sz, start=0):
    """Eight points and six quads of a box (outward winding)."""
    v = []
    for z in (-1, 1):
        for y in (-1, 1):
            for x in (-1, 1):
                v.append((cx + x * sx, cy + y * sy, cz + z * sz))
    f = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    return v, [tuple(i + start for i in q) for q in f]


def cube(name, size=1.0, collection=None):
    v, f = box(0, 0, 0, size / 2, size / 2, size / 2)
    return mesh_object(name, v, f, collection)


def picture(name, pixels_of, srgb=True):
    """A 32 x 32 PNG saved beside the scene, loaded back as an image."""
    img = bpy.data.images.new(name, 32, 32, alpha=True)
    px = []
    for y in range(32):
        for x in range(32):
            px.extend(pixels_of(x, y))
    img.pixels = px
    path = os.path.join(OUT, name + ".png")
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    loaded = bpy.data.images.load(path, check_existing=False)
    loaded.colorspace_settings.name = "sRGB" if srgb else "Non-Color"
    return loaded


# --- 3-level hierarchy with a rotated parent ---
base = link(bpy.data.objects.new("Base", None))
base.location = (3, 0, 0)
base.rotation_euler = (0, 0, math.radians(30))
arm = link(bpy.data.objects.new("Arm", None))
arm.parent = base
arm.location = (0, 2, 0)
arm.rotation_euler = (math.radians(45), 0, 0)
hand = cube("Hand", 0.5)
hand.parent = arm
hand.location = (0, 0, 1)
hand.scale = (1, 2, 1)

# --- a chair, instanced x10 through a collection ---
chair_coll = bpy.data.collections.new("ChairCollection")
verts, faces = [], []
for part in [
    (0, 0, 0.5, 0.5, 0.5, 0.05),  # seat
    (-0.45, -0.45, 0.25, 0.05, 0.05, 0.25),
    (0.45, -0.45, 0.25, 0.05, 0.05, 0.25),
    (-0.45, 0.45, 0.25, 0.05, 0.05, 0.25),
    (0.45, 0.45, 0.25, 0.05, 0.05, 0.25),
    (0, 0.45, 1.0, 0.5, 0.05, 0.45),  # back
]:
    v, f = box(*part, start=len(verts))
    verts += v
    faces += f
mesh_object("Chair", verts, faces, chair_coll)
for i in range(10):
    e = link(bpy.data.objects.new(f"ChairInstance{i}", None))
    e.instance_type = "COLLECTION"
    e.instance_collection = chair_coll
    e.location = (-6 + i * 1.3, -4, 0)

# --- a cube with two UV sets and vertex colours, and the textured material ---
uvcube = cube("UVCube", 1.0)
uvcube.location = (0, 0, 0.5)
me = uvcube.data
me.uv_layers.new(name="UVMap")
second = me.uv_layers.new(name="UVMap2")
for loop in me.loops:
    # The second set: each corner at its point's x/y, so it differs from the first.
    co = me.vertices[loop.vertex_index].co
    second.data[loop.index].uv = (co.x + 0.5, co.y + 0.5)
colours = me.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
palette = [(1, 0, 0, 1), (0, 1, 0, 1), (0, 0, 1, 1), (1, 1, 0, 1)]
for i, c in enumerate(colours.data):
    c.color = palette[i % 4]

base_img = picture("base_color", lambda x, y: (x / 31, y / 31, 0.5, 1.0))
normal_img = picture("normal", lambda x, y: (0.5, 0.5, 1.0, 1.0), srgb=False)
rough_img = picture("roughness", lambda x, y: ((x + y) / 62,) * 3 + (1.0,), srgb=False)

painted = bpy.data.materials.new("Painted")
painted.use_nodes = True
nt = painted.node_tree
bsdf = nt.nodes["Principled BSDF"]
tex = nt.nodes.new("ShaderNodeTexImage")
tex.image = base_img
nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
rtex = nt.nodes.new("ShaderNodeTexImage")
rtex.image = rough_img
nt.links.new(rtex.outputs["Color"], bsdf.inputs["Roughness"])
ntex = nt.nodes.new("ShaderNodeTexImage")
ntex.image = normal_img
nmap = nt.nodes.new("ShaderNodeNormalMap")
nt.links.new(ntex.outputs["Color"], nmap.inputs["Color"])
nt.links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
me.materials.append(painted)

# --- a Sub-D mesh with creases ---
subd = cube("SubD", 1.0)
subd.location = (0, 3, 0.5)
mod = subd.modifiers.new("Subdivision", "SUBSURF")
mod.levels = 2
mod.render_levels = 2
crease = subd.data.attributes.new("crease_edge", "FLOAT", "EDGE")
for i, e in enumerate(subd.data.edges):
    crease.data[i].value = 1.0 if i < 4 else 0.0

# --- emission and glass ---
bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=8, radius=0.5, location=(-2, 0, 0.5))
emitter = bpy.context.active_object
emitter.name = "Emitter"
glow = bpy.data.materials.new("Glow")
glow.use_nodes = True
b = glow.node_tree.nodes["Principled BSDF"]
b.inputs["Emission Color"].default_value = (1.0, 0.5, 0.1, 1.0)
b.inputs["Emission Strength"].default_value = 5.0
emitter.data.materials.append(glow)

glass_obj = cube("GlassCube", 0.8)
glass_obj.location = (-2, 2, 0.4)
glass = bpy.data.materials.new("Glass")
glass.use_nodes = True
b = glass.node_tree.nodes["Principled BSDF"]
b.inputs["Transmission Weight"].default_value = 1.0
b.inputs["IOR"].default_value = 1.45
b.inputs["Roughness"].default_value = 0.0
glass_obj.data.materials.append(glass)

# --- camera, lights, world ---
cam_data = bpy.data.cameras.new("Camera")
cam_data.lens = 35
cam = link(bpy.data.objects.new("Camera", cam_data))
cam.location = (8, -8, 5)
cam.rotation_euler = (math.radians(65), 0, math.radians(45))
scene.camera = cam

area = bpy.data.lights.new("AreaLight", "AREA")
area.size = 2
area.energy = 200
link(bpy.data.objects.new("AreaLight", area)).location = (0, 0, 4)
point = bpy.data.lights.new("PointLight", "POINT")
point.energy = 100
link(bpy.data.objects.new("PointLight", point)).location = (2, -2, 3)
sun = bpy.data.lights.new("SunLight", "SUN")
sun.energy = 3
s = link(bpy.data.objects.new("SunLight", sun))
s.rotation_euler = (math.radians(40), 0, math.radians(20))

hdr = bpy.data.images.new("sky", 16, 8, float_buffer=True)
px = []
for y in range(8):
    for x in range(16):
        t = y / 7
        px.extend((0.3 + t, 0.5 + t, 1.0 + 2 * t, 1.0))
hdr.pixels = px
hdr_path = os.path.join(OUT, "sky.hdr")
hdr.filepath_raw = hdr_path
hdr.file_format = "HDR"
hdr.save()
world = bpy.data.worlds.new("World")
world.use_nodes = True
env = world.node_tree.nodes.new("ShaderNodeTexEnvironment")
env.image = bpy.data.images.load(hdr_path)
world.node_tree.links.new(env.outputs["Color"], world.node_tree.nodes["Background"].inputs["Color"])
scene.world = world

# --- export with the spec's settings ---
wanted = dict(
    selected_objects_only=False,
    export_animation=False,
    export_meshes=True,
    export_lights=True,
    export_cameras=True,
    export_materials=True,
    export_uvmaps=True,
    export_normals=True,
    export_mesh_colors=True,
    use_instancing=True,
    evaluation_mode="RENDER",
    generate_preview_surface=True,
    generate_materialx_network=True,
    export_textures=True,
    export_textures_mode="NEW",
    relative_paths=True,
    export_subdivision="BEST_MATCH",
    convert_world_material=True,
)
props = bpy.ops.wm.usd_export.get_rna_type().properties.keys()
args = {k: v for k, v in wanted.items() if k in props}
missing = sorted(set(wanted) - set(args))
if missing:
    print("usd_export has no", missing, file=sys.stderr)
for name in ("usd_t1.usdc", "usd_t1.usdz"):
    bpy.ops.wm.usd_export(filepath=os.path.join(OUT, name), **args)
print("Wrote", OUT)

# USD_T1_RENDER=<file.exr>: also render the scene with Cycles (CPU, no
# denoising, Standard view transform, linear EXR) for the brightness and
# framing check (usd_render_tests.rs).
render = os.environ.get("USD_T1_RENDER")
if render:
    r = scene.render
    r.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 256
    scene.cycles.use_denoising = False
    r.resolution_x, r.resolution_y, r.resolution_percentage = 480, 270, 100
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.view_settings.exposure = 0
    scene.view_settings.gamma = 1
    r.image_settings.file_format = "OPEN_EXR"
    r.image_settings.color_depth = "32"
    r.filepath = render
    bpy.ops.render.render(write_still=True)
    print("Rendered", render)
    # The same with the world dark, so only the lights light it.
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.0
    r.filepath = render.replace(".exr", "_lights.exr")
    bpy.ops.render.render(write_still=True)
    print("Rendered", r.filepath)
