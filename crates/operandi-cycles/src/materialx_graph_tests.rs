//! MaterialX graphs in Cycles (materials architecture, slice 1): a
//! material the shader tree can't show is kept as MaterialX and rendered
//! through `materialx::cycles` and the general node list.
// The render helpers are only used with the library (`--features cycles`).
#![cfg_attr(not(feature = "cycles"), allow(dead_code, unused_imports))]
use super::*;
use operandi_core::materialx;
use operandi_core::mesh::Mesh;

/// Glows by a noise between red and blue: base and specular off, so only
/// the emission shows. Not simple (noise, mix), so it imports as a graph.
const NOISE_GLOW: &str = r#"<?xml version="1.0"?>
<materialx version="1.39" colorspace="lin_rec709">
  <position name="pos" type="vector3" />
  <multiply name="dense" type="vector3">
    <input name="in1" type="vector3" nodename="pos" />
    <input name="in2" type="float" value="6" />
  </multiply>
  <noise3d name="noise" type="float">
    <input name="position" type="vector3" nodename="dense" />
    <input name="amplitude" type="float" value="0.5" />
    <input name="pivot" type="float" value="0.5" />
  </noise3d>
  <clamp name="t" type="float">
    <input name="in" type="float" nodename="noise" />
  </clamp>
  <mix name="glow" type="color3">
    <input name="fg" type="color3" value="1, 0, 0" />
    <input name="bg" type="color3" value="0, 0, 1" />
    <input name="mix" type="float" nodename="t" />
  </mix>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_weight" type="float" value="0" />
    <input name="specular_weight" type="float" value="0" />
    <input name="emission_luminance" type="float" value="1" />
    <input name="emission_color" type="color3" nodename="glow" />
  </open_pbr_surface>
  <surfacematerial name="NoiseGlow" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#;

/// A square at z = 0.5 facing the camera with `mtlx` on top of its shader
/// tree, as `.mtlx` import puts it.
fn square(mtlx: &str) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    let doc = materialx::doc::parse(mtlx).unwrap();
    let (preset, _) =
        materialx::import::preset(&doc, None, std::path::Path::new(""), &|_| None).unwrap();
    let graphs = preset.scene.shader_tree.node_graphs.graphs.clone();
    assert!(graphs[0].materialx.is_some(), "imported as a graph");
    scene.shader_tree.import_node_graphs(graphs, None).unwrap();
    scene.shader_tree.validate().unwrap();
    scene
}

#[test]
fn a_materialx_graph_compiles_to_the_general_node_list() {
    let scene = square(NOISE_GLOW);
    let sh = scene_meshes(&scene).unwrap()[0].shader.clone().unwrap();
    let g = sh.node_graph.as_ref().expect("the graph is compiled");
    let kinds: Vec<_> = g.graph.nodes.iter().map(|n| n.cycles).collect();
    for want in ["noise_texture", "mix_color", "principled_bsdf", "output"] {
        assert!(kinds.contains(&want), "{want} in {kinds:?}");
    }
    assert!(sh.is_valid());
}

#[cfg(feature = "cycles")]
#[test]
fn a_materialx_noise_graph_renders_its_colours() {
    let scene = square(NOISE_GLOW);
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        24,
        24,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.15),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
    let px: Vec<[f32; 3]> = img.rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect();
    let inner: Vec<[f32; 3]> = (6..18)
        .flat_map(|y| (6..18).map(move |x| (x, y)))
        .map(|(x, y)| px[y * 24 + x])
        .collect();
    // Every colour lies between blue and red: no green, red + blue = 1.
    for p in &inner {
        assert!(p[1] < 0.03, "green in {p:?}");
        assert!((p[0] + p[2] - 1.0).abs() < 0.1, "off the mix: {p:?}");
    }
    let reds: Vec<f32> = inner.iter().map(|p| p[0]).collect();
    let (lo, hi) = reds
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &r| (a.min(r), b.max(r)));
    assert!(hi - lo > 0.05, "flat noise: {lo}..{hi}");
}

/// Regression (QA 2026-11-20): an import with no layer selected went to
/// the very top of the tree, above the Base Shader, where it didn't
/// surface the polygons and the render showed the plain clay.
#[test]
fn an_import_with_nothing_selected_reaches_the_render() {
    let mut e = operandi_core::Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let doc = materialx::doc::parse(NOISE_GLOW).unwrap();
    let (preset, _) =
        materialx::import::preset(&doc, None, std::path::Path::new(""), &|_| None).unwrap();
    e.dispatch(operandi_core::Command::InsertMaterialPreset {
        preset: Box::new(preset),
        beside: None,
    })
    .unwrap();
    let meshes = scene_meshes(&e.scene).unwrap();
    let sh = meshes[0].shader.as_ref().expect("not the clay");
    assert!(sh.node_graph.is_some(), "the graph reaches the render");
}
