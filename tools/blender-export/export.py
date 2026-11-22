# SPDX-FileCopyrightText: 2026 Operandi contributors
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Operandi node graph exporter for Blender.
#
# This script imports bpy, so it is licensed GPL-2.0-or-later (see LICENSE.md
# in this folder). It is a separate tool: nothing here is linked into
# Operandi. It writes a plain JSON description of every material's node tree
# in the shared "operandi-node-graph" format, version 1 (NODE-GRAPH-JSON.md).
#
# Usage (headless):
#   blender -b file.blend -P export.py -- out.json \
#       [--licence CC0-1.0] [--url https://...] [--credit "ambientCG"]
#
# Packed images are unpacked next to the JSON (their entries say
# "packed": true). Image paths are written relative to the JSON when they
# can be.

import json
import os
import sys

import bpy

FORMAT = "operandi-node-graph"
FORMAT_VERSION = 1

# Node settings worth recording. Only those a node actually has are written.
PROPS = (
    "interpolation",
    "projection",
    "extension",
    "distribution",
    "subsurface_method",
    "vector_type",
    "blend_type",
    "data_type",
    "operation",
    "use_clamp",
    "clamp_factor",
    "clamp_result",
    "factor_mode",
    "mode",
    "space",
    "uv_map",
    "from_instancer",
    "target",
    "invert",
    "attribute_name",
    "attribute_type",
    "label",
    "noise_dimensions",
    "noise_type",
    "voronoi_dimensions",
    "feature",
    "musgrave_type",
    "wave_type",
    "bands_direction",
    "rings_direction",
    "wave_profile",
    "gradient_type",
    "normalize",
    "component",
    "falloff",
    "convert_from",
    "convert_to",
)


def parse_args():
    argv = sys.argv
    args = argv[argv.index("--") + 1 :] if "--" in argv else []
    if not args:
        raise SystemExit("usage: blender -b file.blend -P export.py -- out.json [--licence N] [--url U] [--credit C]")
    out = args[0]
    opts = {"licence": None, "url": None, "credit": None}
    i = 1
    while i < len(args):
        key = args[i].lstrip("-")
        if key not in opts or i + 1 >= len(args):
            raise SystemExit("unknown or incomplete option: " + args[i])
        opts[key] = args[i + 1]
        i += 2
    return out, opts


def plain(value):
    """A socket or property value as JSON."""
    if isinstance(value, (bool, int, float, str)) or value is None:
        if isinstance(value, float):
            return round(value, 7)
        return value
    if hasattr(value, "name") and hasattr(value, "bl_rna"):
        return value.name
    try:
        return [plain(v) for v in value]
    except TypeError:
        return str(value)


SHORT_TYPES = ("VALUE", "RGBA", "VECTOR", "INT", "BOOLEAN", "SHADER", "STRING")

# Socket idname prefix -> short type. Subtypes (NodeSocketFloatFactor,
# NodeSocketVectorXYZ...) match by prefix.
IDNAME_TYPES = (
    ("NodeSocketFloat", "VALUE"),
    ("NodeSocketColor", "RGBA"),
    ("NodeSocketVector", "VECTOR"),
    ("NodeSocketInt", "INT"),
    ("NodeSocketBool", "BOOLEAN"),
    ("NodeSocketShader", "SHADER"),
    ("NodeSocketString", "STRING"),
)


def short_type(idname):
    """A socket idname (NodeSocketFloatFactor) as its short type (VALUE)."""
    if idname in SHORT_TYPES:
        return idname
    for prefix, short in IDNAME_TYPES:
        if idname.startswith(prefix):
            return short
    return idname


def socket_type(socket):
    """Always a short type: VALUE/RGBA/VECTOR/INT/BOOLEAN/SHADER/STRING."""
    t = getattr(socket, "type", "")
    if t in SHORT_TYPES:
        return t
    return short_type(getattr(socket, "bl_idname", "") or t)


def socket_key(sockets, socket):
    """Name, plus identifier when the name repeats on that node."""
    same = [s for s in sockets if s.name == socket.name]
    if len(same) > 1:
        return [socket.name, socket.identifier]
    return [socket.name]


def image_entry(image, out_dir):
    packed = image.packed_file is not None
    if packed:
        name = bpy.path.clean_name(image.name)
        ext = os.path.splitext(image.filepath or "")[1] or ".png"
        path = os.path.join(out_dir, name + ext)
        image.filepath_raw = path
        image.save()
    else:
        path = bpy.path.abspath(image.filepath, library=image.library)
    path = os.path.realpath(path)
    # Relative only when the picture sits under the JSON's folder; a path
    # climbing out with ".." breaks across symlinked folders, so those stay
    # absolute.
    try:
        rel = os.path.relpath(path, os.path.realpath(out_dir))
        if not rel.startswith(".."):
            path = rel.replace(os.sep, "/")
    except ValueError:
        pass
    return {
        "path": path,
        "colorspace": image.colorspace_settings.name,
        "packed": packed,
    }


def ramp_entry(ramp):
    return {
        "interpolation": ramp.interpolation,
        "elements": [[round(e.position, 7), plain(e.color)] for e in ramp.elements],
    }


def node_entry(node, out_dir, groups_used):
    props = {}
    for p in PROPS:
        if hasattr(node, p) and p != "label":
            props[p] = plain(getattr(node, p))
    if node.label:
        props["label"] = node.label
    if node.bl_idname == "ShaderNodeValToRGB":
        props["ramp"] = ramp_entry(node.color_ramp)
    if node.bl_idname == "ShaderNodeGroup" and node.node_tree is not None:
        props["group"] = node.node_tree.name
        if node.node_tree.name not in groups_used:
            groups_used.append(node.node_tree.name)
    entry = {"name": node.name, "type": node.bl_idname, "props": props}
    if node.bl_idname in ("ShaderNodeTexImage", "ShaderNodeTexEnvironment"):
        entry["image"] = image_entry(node.image, out_dir) if node.image else None
    inputs = {}
    for s in node.inputs:
        if s.is_linked or not s.enabled:
            continue
        # A repeated name (Mix's A/B per data type) is keyed by identifier.
        key = s.identifier if len(socket_key(node.inputs, s)) > 1 else s.name
        item = {"type": socket_type(s)}
        if hasattr(s, "default_value"):
            item["value"] = plain(s.default_value)
        inputs[key] = item
    entry["inputs"] = inputs
    # RGB and Value nodes keep their number on the output socket.
    if node.bl_idname in ("ShaderNodeRGB", "ShaderNodeValue") and node.outputs:
        out = node.outputs[0]
        entry["outputs"] = {out.name: {"type": socket_type(out), "value": plain(out.default_value)}}
    return entry


def links_of(tree):
    out = []
    for link in tree.links:
        if not link.is_valid or getattr(link, "is_muted", False):
            continue
        a, b = link.from_node, link.to_node
        out.append(
            {
                "from": [a.name] + socket_key(a.outputs, link.from_socket),
                "to": [b.name] + socket_key(b.inputs, link.to_socket),
            }
        )
    return out


def interface_items(tree, in_out):
    items = []
    for item in tree.interface.items_tree:
        if getattr(item, "item_type", "") == "SOCKET" and item.in_out == in_out:
            entry = {"name": item.name}
            if item.identifier != item.name:
                entry["identifier"] = item.identifier
            entry["type"] = short_type(getattr(item, "socket_type", ""))
            entry["default"] = plain(getattr(item, "default_value", None))
            items.append(entry)
    return items


def tree_entry(tree, out_dir, groups_used):
    nodes = [
        node_entry(n, out_dir, groups_used)
        for n in tree.nodes
        if n.bl_idname != "NodeFrame"
    ]
    return nodes, links_of(tree)


def exposed_of(tree, nodes):
    """The group interface when the material is a single group node."""
    groups = [n for n in tree.nodes if n.bl_idname == "ShaderNodeGroup"]
    others = [n for n in tree.nodes if n.bl_idname not in ("ShaderNodeGroup", "ShaderNodeOutputMaterial", "NodeFrame")]
    if len(groups) != 1 or others or groups[0].node_tree is None:
        return []
    group = groups[0]
    out = []
    for s in group.inputs:
        entry = {"name": s.name}
        if s.identifier != s.name:
            entry["identifier"] = s.identifier
        entry.update(
            {
                "to": [group.name, s.name],
                "type": socket_type(s),
                "default": plain(getattr(s, "default_value", None)),
                "min": None,
                "max": None,
            }
        )
        out.append(entry)
    return out


def active_output(tree):
    outputs = [n for n in tree.nodes if n.bl_idname == "ShaderNodeOutputMaterial"]
    for n in outputs:
        if n.is_active_output:
            return n.name
    return outputs[0].name if outputs else None


def main():
    out, opts = parse_args()
    out = os.path.abspath(out)
    out_dir = os.path.dirname(out)
    os.makedirs(out_dir, exist_ok=True)
    licence = None
    if opts["licence"] or opts["url"] or opts["credit"]:
        licence = {"name": opts["licence"], "url": opts["url"], "credit": opts["credit"]}
    materials = []
    all_groups = []
    for mat in bpy.data.materials:
        if mat.node_tree is None or mat.users == 0:
            continue
        groups_used = []
        nodes, links = tree_entry(mat.node_tree, out_dir, groups_used)
        materials.append(
            {
                "name": mat.name,
                "output": active_output(mat.node_tree),
                "nodes": nodes,
                "links": links,
                "exposed": exposed_of(mat.node_tree, nodes),
                "groups_used": groups_used,
            }
        )
        for g in groups_used:
            if g not in all_groups:
                all_groups.append(g)
    # Groups inside groups.
    groups = []
    i = 0
    while i < len(all_groups):
        tree = bpy.data.node_groups[all_groups[i]]
        inner = []
        nodes, links = tree_entry(tree, out_dir, inner)
        for g in inner:
            if g not in all_groups:
                all_groups.append(g)
        groups.append(
            {
                "name": tree.name,
                "nodes": nodes,
                "links": links,
                "inputs": interface_items(tree, "INPUT"),
                "outputs": interface_items(tree, "OUTPUT"),
            }
        )
        i += 1
    doc = {
        "format": FORMAT,
        "format_version": FORMAT_VERSION,
        "source": {
            "app": "blender",
            "app_version": bpy.app.version_string,
            "file": os.path.basename(bpy.data.filepath),
        },
        "licence": licence,
        "materials": materials,
        "groups": groups,
    }
    with open(out, "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=1)
    print("wrote", out, len(materials), "materials")


main()
