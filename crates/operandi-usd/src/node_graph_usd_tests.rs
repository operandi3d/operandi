//! Material node graphs through USD (usd-gaps-3): a MaterialX graph
//! layer's nodegraphs, interfaces, multi-output nodes, spare nodes and
//! places, a Material Group's free nodes, and a Blender Node Material's
//! graph go out as UsdShade networks and come back as the same graph.
use operandi_core::interchange::ExportFormat;
use operandi_core::material::DEFAULT_MATERIAL;
use operandi_core::materialx::doc::{self, Document, Element, Port};
use operandi_core::mesh::Mesh;
use operandi_core::node_view::{NodeEdit, Target};
use operandi_core::shader_tree::{Drop, NewLayer, NodeKind, ShaderTreeEdit, TagMask, TagType};
use operandi_core::{Command, Editor, Scene, materialx};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-usd-ngu-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn boxed(scene: &mut Scene) {
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
}

/// The MaterialX document of the scene's first MaterialX graph layer.
fn stored(scene: &Scene) -> Document {
    let g = scene
        .shader_tree
        .node_graphs
        .graphs
        .iter()
        .find_map(|g| g.materialx.as_ref())
        .expect("a MaterialX graph layer");
    g.document().unwrap()
}

/// Numbers compared as f32, pictures by file name: what a round trip may
/// spell differently.
fn norm_value(p: &Port) -> Option<String> {
    let v = p.value.as_ref()?;
    if p.ty == "filename" {
        return Path::new(v)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
    }
    let nums: Option<Vec<f32>> = v.split(',').map(|x| x.trim().parse().ok()).collect();
    Some(match nums {
        Some(n) => format!("{n:?}"),
        None => v.clone(),
    })
}

fn norm_port(p: &Port) -> String {
    format!(
        "{} {} v={:?} node={:?} graph={:?} out={:?} iface={:?}",
        p.name,
        p.ty,
        norm_value(p),
        p.nodename,
        p.nodegraph,
        p.output,
        p.interfacename
    )
}

/// An element as comparable text: category, name, type, place, inputs
/// and outputs, children; sorted.
fn norm(e: &Element) -> String {
    let mut inputs: Vec<String> = e.inputs.iter().map(norm_port).collect();
    inputs.sort();
    let mut outputs: Vec<String> = e.outputs.iter().map(norm_port).collect();
    outputs.sort();
    let mut children: Vec<String> = e.children.iter().map(norm).collect();
    children.sort();
    let at = |k: &str| e.attrs.get(k).and_then(|v| v.parse::<f32>().ok());
    format!(
        "{} {} {} at={:?},{:?}\n  in {inputs:?}\n  out {outputs:?}\n  [{}]",
        e.category,
        e.name,
        e.ty,
        at("xpos"),
        at("ypos"),
        children.join("\n")
    )
}

/// The graph's nodes and nodegraphs (the material itself named apart).
fn graph_of(d: &Document) -> Vec<String> {
    let mut out: Vec<String> = d
        .elements
        .iter()
        .filter(|e| e.category != "surfacematerial")
        .map(norm)
        .collect();
    out.sort();
    out
}

const GRAPH: &str = r#"<?xml version="1.0"?>
<materialx version="1.39" colorspace="lin_rec709">
  <nodegraph name="NG" xpos="4" ypos="-1.5">
    <input name="tint" type="color3" value="0.5, 0.25, 1" />
    <input name="gain" type="float" value="0.8" />
    <image name="col" type="color3" xpos="1" ypos="2">
      <input name="file" type="filename" value="red.png" colorspace="srgb_texture" />
    </image>
    <multiply name="m" type="color3">
      <input name="in1" type="color3" nodename="col" />
      <input name="in2" type="color3" interfacename="tint" />
    </multiply>
    <output name="base" type="color3" nodename="m" />
    <output name="amount" type="float" interfacename="gain" />
  </nodegraph>
  <separate3 name="split" type="multioutput" xpos="-3" ypos="0.5">
    <input name="in" type="color3" value="0.2, 0.9, 0.4" />
    <output name="outr" type="float" />
    <output name="outg" type="float" />
    <output name="outb" type="float" />
  </separate3>
  <noise3d name="grain" type="float">
    <input name="amplitude" type="float" value="0.2" />
    <input name="pivot" type="float" value="0.5" />
  </noise3d>
  <constant name="spare" type="float" xpos="7" ypos="8">
    <input name="value" type="float" value="0.125" />
  </constant>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodegraph="NG" output="base" />
    <input name="base_weight" type="float" nodegraph="NG" output="amount" />
    <input name="base_metalness" type="float" nodename="split" output="outg" />
    <input name="specular_roughness" type="float" nodename="grain" />
  </open_pbr_surface>
  <surfacematerial name="Grain" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#;

/// NodeGraphs with interface inputs (a value and a pass-through), a
/// multi-output node read by name, a node that feeds nothing and places
/// all come back as the same MaterialX graph.
#[test]
fn a_materialx_graph_comes_back_node_for_node() {
    let dir = scratch("mx");
    image::RgbImage::from_pixel(2, 2, image::Rgb([200, 40, 40]))
        .save(dir.join("red.png"))
        .unwrap();
    let mut scene = Scene::default();
    boxed(&mut scene);
    let d = doc::parse(GRAPH).unwrap();
    let (preset, _) = materialx::import::preset(&d, None, &dir, &|_| Some((2, 2))).unwrap();
    operandi_core::preset::add_material_group(&mut scene, &preset.scene, DEFAULT_MATERIAL).unwrap();
    let before = stored(&scene);

    let out = dir.join("out").join("graph.usda");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "def NodeGraph \"NG\"",
        "color3f inputs:tint = (0.5, 0.25, 1)",
        "inputs:in2.connect = </root/_materials/",
        "NG.inputs:tint>",
        "float outputs:amount.connect = </root/_materials/Default/NG.inputs:gain>",
        "float outputs:outg",
        "uniform token info:id = \"ND_separate3_color3\"",
        "uniform token info:id = \"ND_constant_float\"",
        "uniform float2 ui:nodegraph:node:pos = (7, 8)",
        "bool materialxGraph = 1",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }

    let back = crate::import(&out, &Default::default()).unwrap();
    let notes = back.notes.join("\n");
    assert!(!notes.contains("as written"), "{notes}");
    let after = stored(&back.scene);
    assert_eq!(graph_of(&after), graph_of(&before), "{notes}");
    // And it still renders as the same Cycles nodes.
    let flat = |s: &Scene| {
        let g = s
            .shader_tree
            .node_graphs
            .graphs
            .iter()
            .find(|g| g.materialx.is_some())
            .unwrap();
        let mut n: Vec<&str> = g
            .flatten(&Default::default())
            .unwrap()
            .nodes
            .iter()
            .map(|n| n.cycles)
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        n.sort();
        n.into_iter().map(str::to_owned).collect::<Vec<_>>()
    };
    assert_eq!(flat(&back.scene), flat(&scene));
    // Other programs see the same network resolved: Blender reads it.
    blender_loads(&out);
}

/// Blender 5.2.2, when installed, opens the file headless.
fn blender_loads(file: &Path) {
    let blender = Path::new("/Applications/Blender.app/Contents/MacOS/Blender");
    if !blender.is_file() {
        eprintln!("Blender not installed; skipped loading {}", file.display());
        return;
    }
    let script = format!(
        "import bpy, sys\nbpy.ops.wm.usd_import(filepath={:?})\nprint('OPERANDI_MATERIALS', len(bpy.data.materials), 'OBJECTS', len(bpy.data.objects))\n",
        file.to_string_lossy()
    );
    let out = std::process::Command::new(blender)
        .args([
            "--background",
            "--factory-startup",
            "--python-expr",
            &script,
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("OPERANDI_MATERIALS"),
        "Blender didn't load {}:\n{text}\n{}",
        file.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The material group the default polygons use, with the Base Material
/// in it (as the node editor's tests set it up).
fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    boxed(&mut e.scene);
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = e.scene.shader_tree.next_id - 1;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
        ids: vec![MATERIAL],
        drop: Drop {
            parent: Some(group),
            index: 0,
        },
    }))
    .unwrap();
    let node = e
        .scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == group)
        .unwrap();
    if let NodeKind::Group(g) = &mut node.kind {
        g.tag_type = TagType::Material;
        g.tag = TagMask::Name(DEFAULT_MATERIAL.into());
    }
    (e, group)
}
const MATERIAL: u64 = 4;

fn nodes(e: &mut Editor, group: u64, edit: NodeEdit) {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Nodes { group, edit }))
        .unwrap();
}
fn add(e: &mut Editor, group: u64, category: &str, ty: &str, at: [f64; 2]) -> String {
    nodes(
        e,
        group,
        NodeEdit::Add {
            category: category.into(),
            ty: ty.into(),
            at,
        },
    );
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    g.free.last().unwrap().name.clone()
}

/// A Material Group's free nodes (node editing): every one goes out,
/// wired or not, with its values and place, and comes back as nodes of
/// the imported graph with the same wires.
#[test]
fn free_nodes_go_out_and_come_back() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3", [100., 50.]);
    let mul = add(&mut e, group, "multiply", "color3", [300., 50.]);
    let spare = add(&mut e, group, "constant", "float", [100., 400.]);
    nodes(
        &mut e,
        group,
        NodeEdit::Connect {
            from: noise.clone(),
            output: "out".into(),
            to: Target::Free {
                node: mul.clone(),
                input: "in1".into(),
            },
        },
    );
    nodes(
        &mut e,
        group,
        NodeEdit::Connect {
            from: mul.clone(),
            output: "out".into(),
            to: Target::Layer {
                layer: MATERIAL,
                channel: "diffuse_color".into(),
            },
        },
    );
    let dir = scratch("free");
    let out = dir.join("free.usda");
    let report = crate::export(&out, &e.scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for name in [&noise, &mul, &spare] {
        assert!(
            text.contains(&format!("def Shader \"{name}\"")),
            "{name}:\n{text}"
        );
    }
    assert!(text.contains("ui:nodegraph:node:pos = (1, 0.5)"), "{text}");
    assert!(
        !report.notes.join(" ").contains("constants and images only"),
        "{:?}",
        report.notes
    );

    let back = crate::import(&out, &Default::default()).unwrap();
    let d = stored(&back.scene);
    let before = &e.scene.shader_tree.group_nodes(group).unwrap().free;
    for f in before {
        let el = d
            .element(&f.name)
            .unwrap_or_else(|| panic!("{} came back:\n{:#?}", f.name, back.notes));
        assert_eq!(el.category, f.category, "{}", f.name);
        assert_eq!(el.ty, f.ty, "{}", f.name);
    }
    assert_eq!(
        d.element(&mul)
            .unwrap()
            .input("in1")
            .unwrap()
            .nodename
            .as_deref(),
        Some(noise.as_str())
    );
    let at = |n: &str| {
        let el = d.element(n).unwrap();
        (el.attrs.get("xpos").cloned(), el.attrs.get("ypos").cloned())
    };
    assert_eq!(at(&spare), (Some("1".into()), Some("4".into())));
    // The multiply still drives the surface.
    let view = materialx::view::graph(&d, None).unwrap();
    assert!(view.links.iter().any(|l| l.from.node() == mul));
}

/// A Blender graph: what maps (Principled, picture, Math, RGB, Mix)
/// goes out as MaterialX nodes and comes back as the same Cycles nodes
/// and values; what doesn't (Noise) is named.
#[test]
fn a_blender_graph_goes_out_as_the_materialx_it_maps_to() {
    let dir = scratch("blender");
    image::RgbImage::from_pixel(2, 2, image::Rgb([10, 200, 40]))
        .save(dir.join("leaf.png"))
        .unwrap();
    let graph = serde_json::json!({
        "format": "operandi-node-graph",
        "format_version": 1,
        "materials": [{
            "name": DEFAULT_MATERIAL,
            "output": "Material Output",
            "nodes": [
                {"name": "Material Output", "type": "ShaderNodeOutputMaterial"},
                {"name": "Principled BSDF", "type": "ShaderNodeBsdfPrincipled",
                 "inputs": {"Metallic": {"type": "VALUE", "value": 0.25},
                            "IOR": {"type": "VALUE", "value": 1.45}}},
                {"name": "Image Texture", "type": "ShaderNodeTexImage",
                 "image": {"path": dir.join("leaf.png").to_string_lossy(), "colorspace": "sRGB"}},
                {"name": "Tint", "type": "ShaderNodeRGB",
                 "outputs": {"Color": {"value": [0.9, 0.8, 0.1, 1.0]}}},
                {"name": "Mix", "type": "ShaderNodeMixRGB", "props": {"blend_type": "MULTIPLY"},
                 "inputs": {"Fac": {"type": "VALUE", "value": 0.75}}},
                {"name": "Math", "type": "ShaderNodeMath", "props": {"operation": "MULTIPLY"},
                 "inputs": {"Value_001": {"type": "VALUE", "value": 0.5}}},
                {"name": "Noise Texture", "type": "ShaderNodeTexNoise"}
            ],
            "links": [
                {"from": ["Principled BSDF", "BSDF"], "to": ["Material Output", "Surface"]},
                {"from": ["Image Texture", "Color"], "to": ["Mix", "Color1"]},
                {"from": ["Tint", "Color"], "to": ["Mix", "Color2"]},
                {"from": ["Mix", "Color"], "to": ["Principled BSDF", "Base Color"]},
                {"from": ["Image Texture", "Alpha"], "to": ["Math", "Value"]},
                {"from": ["Math", "Value"], "to": ["Principled BSDF", "Roughness"]},
                {"from": ["Noise Texture", "Fac"], "to": ["Principled BSDF", "Coat Weight"]}
            ]
        }]
    });
    let sidecar = dir.join("leaf_graphs.json");
    std::fs::write(&sidecar, graph.to_string()).unwrap();
    let mut scene = Scene::default();
    boxed(&mut scene);
    let notes = crate::import_graphs::add(
        &mut scene,
        &sidecar,
        &[DEFAULT_MATERIAL.to_string()].into_iter().collect(),
    )
    .unwrap();
    assert!(
        notes.iter().any(|n| n.contains("Node Material layers")),
        "{notes:?}"
    );
    let source = scene
        .shader_tree
        .node_graphs
        .graphs
        .iter()
        .find(|g| g.materialx.is_none())
        .expect("a Blender graph")
        .flatten(&Default::default())
        .unwrap();

    let out = dir.join("leaf.usda");
    let report = crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "uniform token info:id = \"ND_open_pbr_surface_surfaceshader\"",
        "uniform token info:id = \"ND_image_color4\"",
        "uniform token info:id = \"ND_multiply_float\"",
        "token outputs:mtlx:surface.connect",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }
    let notes = report.notes.join("\n");
    assert!(
        notes.contains("Noise Texture (noise_texture) has no MaterialX node"),
        "{notes}"
    );

    let back = crate::import(&out, &Default::default()).unwrap();
    let again = back
        .scene
        .shader_tree
        .node_graphs
        .graphs
        .iter()
        .find(|g| g.materialx.is_some())
        .expect("imported as a MaterialX graph")
        .flatten(&Default::default())
        .unwrap();
    let principled = |f: &operandi_core::node_graph::FlatGraph| {
        let (i, _) = f.feeding("Surface").unwrap();
        assert_eq!(f.nodes[i].cycles, "principled_bsdf");
        i
    };
    let (a, b) = (principled(&source), principled(&again));
    let number = |f: &operandi_core::node_graph::FlatGraph, n: usize, s: &str| match f.value(n, s) {
        Some(operandi_core::node_graph::FlatValue::Float(x)) => *x,
        other => panic!("{s}: {other:?}"),
    };
    for socket in ["Metallic", "IOR"] {
        assert!(
            (number(&source, a, socket) - number(&again, b, socket)).abs() < 1e-5,
            "{socket}"
        );
    }
    for socket in ["Base Color", "Roughness"] {
        assert!(again.is_linked(b, socket), "{socket} is driven");
    }
    for cycles in ["image_texture", "math"] {
        assert!(again.nodes.iter().any(|n| n.cycles == cycles), "{cycles}");
    }
    blender_loads(&out);
}
