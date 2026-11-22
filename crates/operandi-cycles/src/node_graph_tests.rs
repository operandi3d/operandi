//! Nodes N1 in Cycles: Node Material graphs through the general node list
//! (NM-30..32, NM-T1..T3).
// The render helpers are only used with the library (`--features cycles`).
#![cfg_attr(not(feature = "cycles"), allow(dead_code, unused_imports))]
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::node_graph;
use operandi_core::shader_tree::{NewLayer, NodeKind, TextureSource};

/// Noise -> Color Ramp -> Emission, with an exposed Tint (a Mix Color
/// multiply after the ramp) and an exposed glow Strength.
const GLOW: &str = r#"{"format":"operandi-node-graph","format_version":1,"materials":[{
  "name":"Glow","nodes":[
    {"name":"Noise","type":"ShaderNodeTexNoise","props":{"noise_dimensions":"3D","noise_type":"FBM"},
     "inputs":{"Scale":{"type":"VALUE","value":6.0},"Detail":{"type":"VALUE","value":2.0}}},
    {"name":"Ramp","type":"ShaderNodeValToRGB","props":{"ramp":{"interpolation":"LINEAR",
      "elements":[[0.3,[0.0,0.0,1.0,1.0]],[0.7,[1.0,0.0,0.0,1.0]]]}},"inputs":{}},
    {"name":"Mix","type":"ShaderNodeMix","props":{"data_type":"RGBA","blend_type":"MULTIPLY"},
     "inputs":{"Factor_Float":{"type":"VALUE","value":1.0},"B_Color":{"type":"RGBA","value":[1,1,1,1]}}},
    {"name":"Emission","type":"ShaderNodeEmission","inputs":{"Strength":{"type":"VALUE","value":1.0}}},
    {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
  "links":[
    {"from":["Noise","Fac"],"to":["Ramp","Fac"]},
    {"from":["Ramp","Color"],"to":["Mix","A","A_Color"]},
    {"from":["Mix","Result","Result_Color"],"to":["Emission","Color"]},
    {"from":["Emission","Emission"],"to":["Material Output","Surface"]}],
  "exposed":[{"name":"Tint","to":["Mix","B","B_Color"],"type":"RGBA","default":[1,1,1,1]}]}]}"#;

/// One flat green glow.
const GREEN: &str = r#"{"format":"operandi-node-graph","format_version":1,"materials":[{
  "name":"Green","nodes":[
    {"name":"Emission","type":"ShaderNodeEmission","inputs":{
      "Color":{"type":"RGBA","value":[0,1,0,1]},"Strength":{"type":"VALUE","value":1.0}}},
    {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
  "links":[{"from":["Emission","Emission"],"to":["Material Output","Surface"]}],
  "exposed":[{"name":"Glow","to":["Emission","Color"],"type":"RGBA","default":[0,1,0,1]}]}]}"#;

/// A square at z = 0.5 facing the camera; the Base Material glows red
/// (diffuse black). `graph` is imported as a Node Material; its layer id.
fn square(graph: &str) -> (Scene, u64) {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    let tree = &mut scene.shader_tree;
    if let Some(NodeKind::Material(m)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_color = [1., 0., 0.];
        m.luminous_intensity = 1.;
    }
    let graphs = node_graph::import(graph, None).unwrap();
    let id = tree.import_node_graphs(graphs, None).unwrap()[0];
    tree.validate().unwrap();
    (scene, id)
}

fn picture(scene: &Scene, size: u32) -> Vec<[f32; 3]> {
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        size,
        size,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.15),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    img.rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect()
}

fn mean(px: &[[f32; 3]]) -> [f32; 3] {
    let mut m = [0.0; 3];
    for p in px {
        for k in 0..3 {
            m[k] += p[k] / px.len() as f32;
        }
    }
    m
}

fn set_opacity(scene: &mut Scene, id: u64, opacity: f64) {
    let mut node = scene.shader_tree.get(id).unwrap().clone();
    node.layer.opacity = opacity;
    scene.shader_tree.replace(node).unwrap();
}

fn shader_of(scene: &Scene) -> RenderShader {
    scene_meshes(scene).unwrap()[0].shader.clone().unwrap()
}

#[test]
fn a_node_material_compiles_to_a_general_node_list() {
    let (scene, _) = square(GLOW);
    let sh = shader_of(&scene);
    let g = sh.node_graph.as_ref().expect("the graph is compiled");
    let kinds: Vec<_> = g.graph.nodes.iter().map(|n| n.cycles).collect();
    assert_eq!(
        kinds,
        [
            "noise_texture",
            "rgb_ramp",
            "mix_color",
            "emission",
            "output"
        ]
    );
    assert_eq!(g.mix, -1, "full coverage needs no mix factor");
    assert!(sh.is_valid());
    assert!(sh.needs_uvs());
    assert!(!sh.displaces());
}

#[test]
fn layer_only_materials_compile_as_before() {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let meshes = scene_meshes(&scene).unwrap();
    assert!(!meshes.is_empty());
    assert!(
        meshes
            .iter()
            .all(|m| m.shader.as_ref().is_none_or(|s| s.node_graph.is_none()))
    );
}

#[test]
fn nm_32_an_exposed_value_changes_only_that_shader() {
    let (mut scene, id) = square(GREEN);
    let before = shader_of(&scene);
    let mut node = scene.shader_tree.get(id).unwrap().clone();
    if let NodeKind::NodeMaterial(m) = &mut node.kind {
        m.overrides.insert("Glow".into(), vec![0., 0., 1.]);
    }
    scene.shader_tree.replace(node).unwrap();
    let after = shader_of(&scene);
    assert_ne!(before, after);
    // Only the graph's value moved: same layered ops, same everything else.
    assert_eq!(before.ops, after.ops);
    assert_eq!(before.inputs, after.inputs);
}

#[cfg(feature = "cycles")]
#[test]
fn nm_t1_noise_through_a_ramp_renders_ramp_colours_that_vary() {
    let (scene, _) = square(GLOW);
    let px = picture(&scene, 24);
    // The middle of the picture is all square.
    let inner: Vec<[f32; 3]> = (6..18)
        .flat_map(|y| (6..18).map(move |x| (x, y)))
        .map(|(x, y)| px[y * 24 + x])
        .collect();
    // Every colour lies on the ramp from blue to red: no green, and red +
    // blue = 1 (within sampling noise).
    for p in &inner {
        assert!(p[1] < 0.03, "green in {p:?}");
        assert!((p[0] + p[2] - 1.0).abs() < 0.08, "off the ramp: {p:?}");
    }
    // The noise varies across the square.
    let reds: Vec<f32> = inner.iter().map(|p| p[0]).collect();
    let (lo, hi) = reds
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &r| (a.min(r), b.max(r)));
    assert!(hi - lo > 0.05, "flat noise: {lo}..{hi}");
}

#[cfg(feature = "cycles")]
#[test]
fn nm_t2_half_opacity_mixes_the_graph_over_the_layers() {
    let (mut scene, id) = square(GREEN);
    let full = mean(&picture(&scene, 8)[27..29]);
    assert!(full[1] > 0.9 && full[0] < 0.05, "full: {full:?}");
    set_opacity(&mut scene, id, 0.5);
    let half = mean(&picture(&scene, 8)[27..29]);
    assert!(
        (half[0] - 0.5).abs() < 0.06 && (half[1] - 0.5).abs() < 0.06,
        "half: {half:?}"
    );
    set_opacity(&mut scene, id, 0.0);
    let none = mean(&picture(&scene, 8)[27..29]);
    assert!(none[0] > 0.9 && none[1] < 0.05, "none: {none:?}");
}

#[cfg(feature = "cycles")]
#[test]
fn nm_t3_a_layer_above_drives_the_exposed_tint() {
    let (mut scene, id) = square(GREEN);
    // A Constant layer above the Node Material, driving Glow blue.
    let tex = scene.shader_tree.add(NewLayer::Constant, Some(id)).unwrap();
    let mut node = scene.shader_tree.get(tex).unwrap().clone();
    if let NodeKind::Texture(t) = &mut node.kind {
        t.node_input = Some(Box::new("Glow".into()));
        t.source = TextureSource::Constant {
            color: [0., 0., 1.],
            value: 1.,
        };
    }
    scene.shader_tree.replace(node).unwrap();
    let sh = shader_of(&scene);
    assert_eq!(sh.node_graph.as_ref().unwrap().drivers.len(), 1);
    let got = mean(&picture(&scene, 8)[27..29]);
    assert!(got[2] > 0.9 && got[1] < 0.05, "driven: {got:?}");
}
