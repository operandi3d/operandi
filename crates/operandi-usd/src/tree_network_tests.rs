//! Tree materials through USD as their standard MaterialX form
//! (materials stage 3): the network holds only standard nodes onto an
//! open_pbr_surface, and USD import reads it back as a graph Cycles can
//! render.
use crate::materialx_out::{NetValue, tree_network};
use operandi_core::Scene;
use operandi_core::shader_eval::{Expr, ImageLookup, Input, Mix, Principled};
use operandi_core::shader_tree::Filtering;

fn plain(inputs: Vec<(Input, Expr)>) -> Principled {
    Principled {
        inputs,
        shader: Default::default(),
        bump: None,
        normal_map: None,
        displacement: None,
        coat_bump: 0.,
        see_through: None,
        rounded_edge: None,
        absorption: None,
        scatter: None,
        thin_walled: false,
        glass_principled: false,
        node: None,
        values: Vec::new(),
    }
}

#[test]
fn a_tree_surface_is_a_flat_network_of_standard_nodes() {
    let mut scene = Scene::default();
    scene.images.push(operandi_core::backdrop::SceneImage {
        id: 3,
        name: "wood".into(),
        path: "/pictures/wood.png".into(),
        width: 2,
        height: 2,
    });
    let image = Expr::ImageColor(Box::new(ImageLookup::uv(3, Filtering::Bilinear)));
    let p = plain(vec![
        (
            Input::BaseColor,
            Expr::Mix {
                mode: Mix::Multiply,
                fac: Box::new(Expr::scalar(0.8)),
                a: Box::new(Expr::Const([0.9, 0.7, 0.5])),
                b: Box::new(image.clone()),
            },
        ),
        (Input::Roughness, Expr::Luma(Box::new(image))),
    ]);
    let (net, notes) = tree_network(&scene, &p, "Wood", None).unwrap();
    // The same picture drives a colour and a number (roughness).
    assert_eq!(
        notes,
        ["pictures used as both colours and numbers are read as colours"]
    );
    assert_eq!(net.terminal, "surface");
    for n in &net.nodes {
        assert!(
            n.id.starts_with("ND_") && !n.id.contains("operandi"),
            "{}: {}",
            n.name,
            n.id
        );
    }
    let ids: Vec<&str> = net.nodes.iter().map(|n| n.id.as_str()).collect();
    for want in [
        "ND_image_color3",
        "ND_texcoord_vector2",
        "ND_mix_color3",
        "ND_luminance_color3",
        "ND_open_pbr_surface_surfaceshader",
    ] {
        assert!(ids.contains(&want), "{want} in {ids:?}");
    }
    // The picture is the scene image (copied like Preview Surface's), read
    // once by both uses.
    let images: Vec<_> = net
        .nodes
        .iter()
        .filter(|n| n.id == "ND_image_color3")
        .collect();
    assert_eq!(images.len(), 1, "{ids:?}");
    let file = images[0].inputs.iter().find(|i| i.name == "file").unwrap();
    assert_eq!(file.value, NetValue::Image(3));
    // It drives the base colour, so it is read as sRGB.
    assert_eq!(file.colorspace.as_deref(), Some("srgb_texture"));
    // Each implementation's nodes are named after the node using them.
    assert!(net.nodes.iter().any(|n| n.name.starts_with("n")));
}

#[cfg(feature = "usd")]
#[test]
fn a_tree_material_exports_as_standard_materialx_and_imports_as_a_graph() {
    use operandi_core::interchange::ExportFormat;
    use operandi_core::material::DEFAULT_MATERIAL;
    use operandi_core::materialx;
    use operandi_core::mesh::Mesh;
    let dir = std::env::temp_dir().join(format!("operandi-usd-tree-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    image::RgbImage::from_pixel(2, 2, image::Rgb([40, 200, 40]))
        .save(dir.join("green.png"))
        .unwrap();
    // A simple .mtlx imports as Material and Image Map layers, not a graph.
    let mtlx = r#"<?xml version="1.0"?>
<materialx version="1.39">
  <image name="col" type="color3">
    <input name="file" type="filename" value="green.png" colorspace="srgb_texture" />
  </image>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="col" />
    <input name="specular_roughness" type="float" value="0.35" />
  </open_pbr_surface>
  <surfacematerial name="Leaf" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#;
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    let doc = materialx::doc::parse(mtlx).unwrap();
    let (preset, _) = materialx::import::preset(&doc, None, &dir, &|_| Some((2, 2))).unwrap();
    assert!(
        preset.scene.shader_tree.node_graphs.graphs.is_empty(),
        "a simple material is tree layers"
    );
    operandi_core::preset::add_material_group(&mut scene, &preset.scene, DEFAULT_MATERIAL).unwrap();
    // A Screen blend, which the simpler writer (MX-05) can't write, sends
    // the material through its standard MaterialX form.
    let picture = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| matches!(n.kind, operandi_core::shader_tree::NodeKind::Texture(_)))
        .expect("an Image Map layer");
    picture.layer.blend = operandi_core::shader_tree::BlendMode::Screen;
    picture.layer.opacity = 0.6;
    let simple = crate::materialx_out::network(&scene, DEFAULT_MATERIAL)
        .unwrap()
        .unwrap();
    assert!(!simple.1.is_empty(), "MX-05 leaves the blend out");
    let out = dir.join("out").join("leaf.usda");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let report = crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "token outputs:mtlx:surface.connect",
        "uniform token info:id = \"ND_open_pbr_surface_surfaceshader\"",
        "uniform token info:id = \"ND_image_color3\"",
        "uniform token info:id = \"ND_texcoord_vector2\"",
        "./leaf_textures/green.png",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }
    assert!(!text.contains("operandi_"), "{text}");
    assert!(
        !report
            .notes
            .iter()
            .any(|n| n.contains("MaterialX networks")),
        "{:?}",
        report.notes
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
    for want in ["image_texture", "principled_bsdf"] {
        assert!(flat.nodes.iter().any(|n| n.cycles == want), "{want}");
    }
}
