use super::*;
use crate::materialx::cycles::translate;
use crate::node_graph::{FlatGraph, FlatNode, FlatValue};
use crate::procedural::{Procedural, ProceduralKind};
use crate::shader_eval::{Expr, ImageLookup, Mix, PatternLookup};
use crate::shader_tree::Filtering;
use crate::texture_locator::Tiling;
use std::path::Path;

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

fn principled(g: &FlatGraph) -> &FlatNode {
    g.nodes
        .iter()
        .find(|n| n.cycles == "principled_bsdf")
        .unwrap_or_else(|| panic!("no principled in {g:#?}"))
}

fn socket(n: &FlatNode, s: &str) -> Vec<f64> {
    match n.values.iter().find(|(k, _)| k == s).map(|(_, v)| v) {
        Some(FlatValue::Float(v)) => vec![*v],
        Some(FlatValue::Vec3(v)) => v.to_vec(),
        other => panic!("{s}: {other:?}"),
    }
}

fn translated(p: &Principled) -> (String, FlatGraph) {
    let (text, _) = document(p, &Scene::default(), "Test").unwrap();
    let d = doc::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    let t = translate(&d, None, Path::new("/")).unwrap_or_else(|r| panic!("{r}\n{text}"));
    (text, t.graph)
}

#[test]
fn constant_surfaces_land_on_the_same_principled_sockets() {
    let p = plain(vec![
        (Input::BaseColor, Expr::Const([0.6, 0.3, 0.1])),
        (Input::Metallic, Expr::scalar(0.25)),
        (Input::Roughness, Expr::scalar(0.4)),
        (Input::SpecularIorLevel, Expr::scalar(0.3)),
        (Input::SpecularTint, Expr::Const([1., 0.9, 0.8])),
        (Input::CoatWeight, Expr::scalar(0.5)),
        (Input::CoatRoughness, Expr::scalar(0.2)),
        (Input::EmissionColor, Expr::Const([1., 0.5, 0.])),
        (Input::EmissionStrength, Expr::scalar(3.)),
        (Input::SheenWeight, Expr::scalar(0.1)),
        (Input::Alpha, Expr::scalar(0.9)),
    ]);
    let (text, g) = translated(&p);
    assert!(
        text.contains("<open_pbr_surface name=\"surface\""),
        "{text}"
    );
    let n = principled(&g);
    let close = |s: &str, want: &[f64]| {
        let got = socket(n, s);
        assert!(
            got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-9),
            "{s}: {got:?} vs {want:?}"
        );
    };
    close("Base Color", &[0.6, 0.3, 0.1]);
    close("Metallic", &[0.25]);
    close("Roughness", &[0.4]);
    close("Specular IOR Level", &[0.3]);
    close("Specular Tint", &[1., 0.9, 0.8]);
    close("Coat Weight", &[0.5]);
    close("Coat Roughness", &[0.2]);
    close("Emission Color", &[1., 0.5, 0.]);
    close("Emission Strength", &[3.]);
    close("Sheen Weight", &[0.1]);
    close("Alpha", &[0.9]);
}

#[test]
fn textured_surfaces_carry_their_nodes_and_definitions() {
    let uv = Expr::MeshUv {
        map: None,
        repeat: [2., 2.],
        rotation: 15.,
    };
    let image = Expr::ImageColor(Box::new(ImageLookup {
        image: 3,
        filtering: Filtering::Bilinear,
        vector: Expr::Tile(Box::new(uv), [Tiling::Repeat, Tiling::Mirror]),
        extension: Tiling::Repeat,
    }));
    let mixed = Expr::Mix {
        mode: Mix::Overlay,
        fac: Box::new(Expr::scalar(0.7)),
        a: Box::new(Expr::Const([0.2, 0.4, 0.6])),
        b: Box::new(image.clone()),
    };
    let mut p = plain(vec![
        (Input::BaseColor, mixed),
        (Input::Roughness, Expr::Luma(Box::new(image))),
    ]);
    p.bump = Some(Expr::scalar(0.1));
    let (text, notes) = document(&p, &Scene::default(), "Wood").unwrap();
    let d = doc::parse(&text).unwrap();
    for cat in [
        "operandi_image",
        "operandi_tile_repeat_mirror",
        "operandi_texcoord",
        "operandi_mix_overlay",
        "operandi_luma",
    ] {
        assert!(d.element(&format!("ND_{cat}")).is_some(), "{cat}\n{text}");
        assert!(d.element(&format!("NG_{cat}")).is_some(), "{cat}\n{text}");
    }
    assert_eq!(
        notes,
        [
            "pictures used as both colours and numbers are read as colours",
            "bump heights"
        ]
    );
    // The picture drives the base colour, so it reads as sRGB.
    assert!(text.contains("colorspace=\"srgb_texture\""), "{text}");
    // The defs keep their uniform inputs.
    assert!(
        text.contains("name=\"addressmode\" type=\"string\" value=\"\" uniform=\"true\""),
        "{text}"
    );
}

#[test]
fn a_node_with_no_standard_version_refuses_naming_it() {
    let pattern = Expr::Pattern(Box::new(PatternLookup {
        pattern: Procedural::new(ProceduralKind::Noise).pattern,
        axis: 2,
        vector: Expr::DistanceTo([0.; 3]),
    }));
    let e = document(
        &plain(vec![(Input::BaseColor, pattern)]),
        &Scene::default(),
        "N",
    )
    .unwrap_err();
    assert!(e.contains("operandi_pattern"), "{e}");
}

#[test]
fn see_through_glass_and_normal_maps_reach_openpbr() {
    let mut p = plain(vec![(Input::Roughness, Expr::scalar(0.1))]);
    p.see_through = Some(crate::shader_eval::SeeThrough {
        kind: SeeThroughKind::Glass,
        amount: Expr::scalar(1.),
        color: Expr::Const([0.9, 1., 0.95]),
        ior: Expr::scalar(1.33),
        roughness: Expr::scalar(0.1),
    });
    p.thin_walled = true;
    p.normal_map = Some(Expr::Const([0.5, 0.5, 1.]));
    let (text, notes) = document(&p, &Scene::default(), "Glass").unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    for want in [
        "name=\"transmission_weight\" type=\"float\" value=\"1\"",
        "name=\"transmission_color\" type=\"color3\" value=\"0.9, 1, 0.95\"",
        "name=\"specular_ior\" type=\"float\" value=\"1.33\"",
        "name=\"geometry_thin_walled\" type=\"boolean\" value=\"true\"",
        "<normalmap ",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    doc::parse(&text).unwrap();
}
