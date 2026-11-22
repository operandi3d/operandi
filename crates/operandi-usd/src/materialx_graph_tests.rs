//! MaterialX graph layers through USD (materials slice 2): exported as a
//! UsdShade MaterialX network and imported back as the same graph.
use operandi_core::interchange::ExportFormat;
use operandi_core::material::DEFAULT_MATERIAL;
use operandi_core::mesh::Mesh;
use operandi_core::{Scene, materialx};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-usd-mxg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A noise-driven roughness with a picture for the base colour, in a
/// nodegraph: not simple, so it is kept as a graph.
fn material(dir: &std::path::Path) -> String {
    image::RgbImage::from_pixel(2, 2, image::Rgb([200, 40, 40]))
        .save(dir.join("red.png"))
        .unwrap();
    r#"<?xml version="1.0"?>
<materialx version="1.39">
  <nodegraph name="NG">
    <image name="col" type="color3">
      <input name="file" type="filename" value="red.png" colorspace="srgb_texture" />
    </image>
    <output name="base" type="color3" nodename="col" />
  </nodegraph>
  <noise3d name="grain" type="float">
    <input name="amplitude" type="float" value="0.2" />
    <input name="pivot" type="float" value="0.5" />
  </noise3d>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodegraph="NG" output="base" />
    <input name="specular_roughness" type="float" nodename="grain" />
  </open_pbr_surface>
  <surfacematerial name="Grain" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#
        .to_string()
}

fn scene(dir: &std::path::Path) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    let doc = materialx::doc::parse(&material(dir)).unwrap();
    let (preset, _) = materialx::import::preset(&doc, None, dir, &|_| Some((2, 2))).unwrap();
    operandi_core::preset::add_material_group(&mut scene, &preset.scene, DEFAULT_MATERIAL).unwrap();
    scene
}

#[test]
fn a_graph_layer_exports_as_a_materialx_network_and_imports_as_a_graph() {
    let dir = scratch("rt");
    let scene = scene(&dir);
    let out = dir.join("out").join("graph.usda");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "token outputs:mtlx:surface.connect",
        "uniform token info:id = \"ND_noise3d_float\"",
        "uniform token info:id = \"ND_image_color3\"",
        "./graph_textures/red.png",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }
    assert!(
        out.with_file_name("graph_textures")
            .join("red.png")
            .is_file()
    );
    let back = crate::import(&out, &Default::default()).unwrap();
    let graph = back
        .scene
        .shader_tree
        .node_graphs
        .graphs
        .iter()
        .find(|g| g.materialx.is_some())
        .expect("imported as a MaterialX graph");
    let flat = graph.flatten(&Default::default()).unwrap();
    for want in ["noise_texture", "image_texture", "principled_bsdf"] {
        assert!(flat.nodes.iter().any(|n| n.cycles == want), "{want}");
    }
}
