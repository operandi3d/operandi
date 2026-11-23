use super::*;
use crate::materialx::doc;

fn tx(text: &str) -> Translated {
    let d = doc::parse(text).unwrap();
    translate(&d, None, Path::new("/pictures")).unwrap_or_else(|r| panic!("{r}"))
}

fn wrap(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<materialx version="1.39">
{body}
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#
    )
}

fn node<'a>(g: &'a FlatGraph, cycles: &str) -> (usize, &'a FlatNode) {
    g.nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.cycles == cycles)
        .unwrap_or_else(|| panic!("no {cycles} in {g:#?}"))
}

fn value<'a>(n: &'a FlatNode, socket: &str) -> Option<&'a FlatValue> {
    n.values.iter().find(|(s, _)| s == socket).map(|(_, v)| v)
}

fn float(n: &FlatNode, socket: &str) -> f64 {
    match value(n, socket) {
        Some(FlatValue::Float(v)) => *v,
        other => panic!("{socket}: {other:?}"),
    }
}

fn vec3(n: &FlatNode, socket: &str) -> [f64; 3] {
    match value(n, socket) {
        Some(FlatValue::Vec3(v)) => *v,
        other => panic!("{socket}: {other:?}"),
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn constant_open_pbr_folds_onto_principled_values() {
    let t = tx(&wrap(
        r#"<open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" value="0.5, 0.25, 0.125" />
    <input name="base_weight" type="float" value="0.5" />
    <input name="specular_weight" type="float" value="0.8" />
    <input name="specular_roughness" type="float" value="0.4" />
    <input name="emission_luminance" type="float" value="10" />
  </open_pbr_surface>"#,
    ));
    let g = &t.graph;
    // A material of only values is a Principled BSDF and the output.
    assert_eq!(g.nodes.len(), 2, "{g:#?}");
    let (b, p) = node(g, "principled_bsdf");
    assert_eq!(g.feeding("Surface"), Some((b, "BSDF")));
    let c = vec3(p, "Base Color");
    assert!(close(c[0], 0.25) && close(c[1], 0.125) && close(c[2], 0.0625));
    assert!(close(float(p, "Specular IOR Level"), 0.4));
    assert!(close(float(p, "Roughness"), 0.4));
    assert!(close(
        float(p, "Emission Strength"),
        10. * openpbr::STRENGTH_PER_NIT
    ));
    // OpenPBR's own defaults, not Cycles': coat IOR 1.6.
    assert!(close(float(p, "Coat IOR"), 1.6));
    assert_eq!(t.name, "M");
}

#[test]
fn an_image_chain_becomes_image_texture_mapping_and_maths() {
    let t = tx(&wrap(
        r#"<texcoord name="uv" type="vector2" />
  <place2d name="place" type="vector2">
    <input name="texcoord" type="vector2" nodename="uv" />
    <input name="scale" type="vector2" value="0.5, 0.25" />
  </place2d>
  <image name="tex" type="color3">
    <input name="file" type="filename" value="bricks.png" colorspace="srgb_texture" />
    <input name="texcoord" type="vector2" nodename="place" />
  </image>
  <image name="rough" type="float">
    <input name="file" type="filename" value="rough.png" />
  </image>
  <multiply name="tint" type="color3">
    <input name="in1" type="color3" nodename="tex" />
    <input name="in2" type="color3" value="0.5, 0.5, 0.5" />
  </multiply>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="tint" />
    <input name="specular_roughness" type="float" nodename="rough" />
  </open_pbr_surface>"#,
    ));
    let g = &t.graph;
    let images: Vec<&FlatNode> = g
        .nodes
        .iter()
        .filter(|n| n.cycles == "image_texture")
        .collect();
    assert_eq!(images.len(), 2);
    let colour = images.iter().find(|n| n.name == "tex").unwrap();
    assert_eq!(
        value(colour, "Filename"),
        // The base folder joined the system's way (`\` on Windows).
        Some(&FlatValue::Text(
            Path::new("/pictures")
                .join("bricks.png")
                .to_string_lossy()
                .into_owned()
        ))
    );
    assert_eq!(
        value(colour, "Colorspace"),
        Some(&FlatValue::Text("__builtin_srgb".into()))
    );
    let rough = images.iter().find(|n| n.name == "rough").unwrap();
    assert_eq!(
        value(rough, "Colorspace"),
        Some(&FlatValue::Text("data".into()))
    );
    // place2d divides by its scale: a Mapping node scaling by 2 and 4.
    let (m, mapping) = node(g, "mapping");
    let s = vec3(mapping, "Scale");
    assert!(close(s[0], 2.) && close(s[1], 4.));
    assert!(g.links.iter().any(|l| l.to == m
        && l.to_socket == "Vector"
        && g.nodes[l.from].cycles == "texture_coordinate"
        && l.from_socket == "UV"));
    // The multiply is a Vector Math node into Base Color.
    let (b, _) = node(g, "principled_bsdf");
    let into_base = g
        .links
        .iter()
        .find(|l| l.to == b && l.to_socket == "Base Color")
        .unwrap();
    assert_eq!(g.nodes[into_base.from].cycles, "vector_math");
    // A float image reads its first channel.
    let into_rough = g
        .links
        .iter()
        .find(|l| l.to == b && l.to_socket == "Roughness")
        .unwrap();
    assert_eq!(g.nodes[into_rough.from].cycles, "separate_color");
    assert_eq!(into_rough.from_socket, "Red");
}

#[test]
fn nodegraphs_interfaces_and_normal_maps_resolve() {
    let t = tx(&wrap(
        r#"<nodegraph name="NG" >
    <input name="tiling" type="vector2" value="3, 3" />
    <tiledimage name="n" type="vector3">
      <input name="file" type="filename" value="normal.png" />
      <input name="uvtiling" type="vector2" interfacename="tiling" />
    </tiledimage>
    <normalmap name="nm" type="vector3">
      <input name="in" type="vector3" nodename="n" />
    </normalmap>
    <output name="normal_out" type="vector3" nodename="nm" />
  </nodegraph>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="geometry_normal" type="vector3" nodegraph="NG" output="normal_out" />
  </open_pbr_surface>"#,
    ));
    let g = &t.graph;
    let (nm, _) = node(g, "normal_map");
    let (b, _) = node(g, "principled_bsdf");
    assert!(
        g.links
            .iter()
            .any(|l| l.from == nm && l.to == b && l.to_socket == "Normal")
    );
    // The interface value reached the tiling.
    let (_, mapping) = node(g, "mapping");
    assert!(close(vec3(mapping, "Scale")[0], 3.));
}

#[test]
fn standard_surface_runs_through_the_published_translation() {
    let t = tx(&wrap(
        r#"<standard_surface name="s" type="surfaceshader">
    <input name="base" type="float" value="1" />
    <input name="base_color" type="color3" value="0.2, 0.4, 0.6" />
    <input name="metalness" type="float" value="1" />
    <input name="specular_roughness" type="float" value="0.25" />
  </standard_surface>"#,
    ));
    let g = &t.graph;
    // All values: folded to one Principled BSDF.
    assert_eq!(g.nodes.len(), 2, "{g:#?}");
    let (_, p) = node(g, "principled_bsdf");
    let c = vec3(p, "Base Color");
    assert!(
        close(c[0], 0.2) && close(c[1], 0.4) && close(c[2], 0.6),
        "{c:?}"
    );
    assert!(close(float(p, "Metallic"), 1.));
    assert!(close(float(p, "Roughness"), 0.25));
}

#[test]
fn procedural_maths_become_cycles_nodes() {
    let t = tx(&wrap(
        r#"<position name="pos" type="vector3" />
  <noise3d name="noise" type="float">
    <input name="position" type="vector3" nodename="pos" />
  </noise3d>
  <ifgreater name="pick" type="color3">
    <input name="value1" type="float" nodename="noise" />
    <input name="value2" type="float" value="0.2" />
    <input name="in1" type="color3" value="1, 0, 0" />
    <input name="in2" type="color3" value="0, 0, 1" />
  </ifgreater>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="pick" />
  </open_pbr_surface>"#,
    ));
    let g = &t.graph;
    node(g, "noise_texture");
    let (_, cmp) = node(g, "math");
    assert!(
        g.nodes.iter().any(|n| n.cycles == "math"
            && value(n, "Type") == Some(&FlatValue::Text("greater_than".into()))),
        "{cmp:?}"
    );
    let (_, mix) = node(g, "mix_color");
    let a = vec3(mix, "A");
    let b = vec3(mix, "B");
    assert_eq!((a, b), ([0., 0., 1.], [1., 0., 0.]));
    assert!(t.notes.iter().any(|n| n.contains("noise")));
}

#[test]
fn a_custom_node_expands_through_its_nodegraph() {
    let t = tx(&wrap(
        r#"<nodedef name="ND_double_color3" node="double">
    <input name="in" type="color3" value="0.1, 0.1, 0.1" />
    <output name="out" type="color3" />
  </nodedef>
  <nodegraph name="NG_double_color3" nodedef="ND_double_color3">
    <multiply name="m" type="color3">
      <input name="in1" type="color3" interfacename="in" />
      <input name="in2" type="float" value="2" />
    </multiply>
    <output name="out" type="color3" nodename="m" />
  </nodegraph>
  <double name="d" type="color3">
    <input name="in" type="color3" value="0.2, 0.3, 0.4" />
  </double>
  <double name="e" type="color3" />
  <mix name="both" type="color3">
    <input name="fg" type="color3" nodename="d" />
    <input name="bg" type="color3" nodename="e" />
    <input name="mix" type="float" value="0.5" />
  </mix>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="both" />
  </open_pbr_surface>"#,
    ));
    let (_, p) = node(&t.graph, "principled_bsdf");
    // (0.4, 0.6, 0.8) and the default doubled (0.2) mixed half and half.
    let c = vec3(p, "Base Color");
    assert!(
        close(c[0], 0.3) && close(c[1], 0.4) && close(c[2], 0.5),
        "{c:?}"
    );
}

#[test]
fn unknown_nodes_and_loops_refuse_naming_the_node() {
    let d = doc::parse(&wrap(
        r#"<hextiledimage name="hex" type="color3" />
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="hex" />
  </open_pbr_surface>"#,
    ))
    .unwrap();
    let r = translate(&d, None, Path::new("")).unwrap_err();
    assert_eq!(
        (r.node.as_str(), r.category.as_str()),
        ("hex", "hextiledimage")
    );
    let d = doc::parse(&wrap(
        r#"<add name="a" type="float">
    <input name="in1" type="float" nodename="b" />
  </add>
  <add name="b" type="float">
    <input name="in1" type="float" nodename="a" />
  </add>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="specular_roughness" type="float" nodename="a" />
  </open_pbr_surface>"#,
    ))
    .unwrap();
    let r = translate(&d, None, Path::new("")).unwrap_err();
    assert!(r.reason.contains("feeds itself"), "{r}");
}

#[test]
fn displacement_and_surface_mixes_reach_the_output() {
    let t = tx(r#"<?xml version="1.0"?>
<materialx version="1.39">
  <open_pbr_surface name="red" type="surfaceshader">
    <input name="base_color" type="color3" value="1, 0, 0" />
  </open_pbr_surface>
  <open_pbr_surface name="blue" type="surfaceshader">
    <input name="base_color" type="color3" value="0, 0, 1" />
  </open_pbr_surface>
  <image name="h" type="float">
    <input name="file" type="filename" value="height.exr" />
  </image>
  <mix name="s" type="surfaceshader">
    <input name="fg" type="surfaceshader" nodename="red" />
    <input name="bg" type="surfaceshader" nodename="blue" />
    <input name="mix" type="float" nodename="h" />
  </mix>
  <displacement name="d" type="displacementshader">
    <input name="displacement" type="float" nodename="h" />
    <input name="scale" type="float" value="2" />
  </displacement>
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
    <input name="displacementshader" type="displacementshader" nodename="d" />
  </surfacematerial>
</materialx>"#);
    let g = &t.graph;
    let (m, _) = node(g, "mix_closure");
    assert_eq!(g.feeding("Surface"), Some((m, "Closure")));
    let (d, disp) = node(g, "displacement");
    assert_eq!(g.feeding("Displacement"), Some((d, "Displacement")));
    assert!(close(
        float(disp, "Scale"),
        2. * openpbr::DEFAULT_LENGTH_IN_METRES
    ));
    // The height picture is read once, for both.
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.cycles == "image_texture")
            .count(),
        1
    );
}
