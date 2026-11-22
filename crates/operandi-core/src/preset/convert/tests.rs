use super::*;
use crate::node_graph_reader::GraphFile;
use crate::preset::texture_set::{MapKind, MapName, parse_map_name, texture_sets};
use crate::shader_eval::{Context, Input};
use crate::shader_tree::{Node, NodeKind, ShaderTree, TextureSource};
use std::path::{Path, PathBuf};

fn size(_: &Path) -> Option<(u32, u32)> {
    Some((4, 4))
}

fn set_of(files: &[&str]) -> TextureSet {
    let paths: Vec<PathBuf> = files
        .iter()
        .map(|f| PathBuf::from("/maps").join(f))
        .collect();
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    texture_sets(&refs).remove(0)
}

type LayerRow = (TextureEffect, String, ImageSettings, Layer);

/// The image layers of a preset, top first, with the file each reads.
fn layers(preset: &Preset) -> Vec<LayerRow> {
    let scene = &preset.scene;
    scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => match &t.source {
                TextureSource::ImageMap { image, settings } => {
                    let path = scene
                        .images
                        .iter()
                        .find(|i| Some(i.id) == *image)
                        .map(|i| i.path.clone())
                        .unwrap_or_default();
                    Some((
                        t.effect,
                        file_name(&path),
                        (**settings).clone(),
                        n.layer.clone(),
                    ))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn material(preset: &Preset) -> MaterialChannels {
    preset
        .scene
        .shader_tree
        .nodes
        .iter()
        .find_map(|n| match &n.kind {
            NodeKind::Material(m) => Some((**m).clone()),
            _ => None,
        })
        .expect("a material")
}

fn graph(json: &str) -> GraphMaterial {
    GraphFile::parse(json).unwrap().materials.remove(0)
}

/// A one-material graph: Principled into the output, plus `nodes` and
/// `links` (JSON array bodies).
fn simple_graph(principled_inputs: &str, nodes: &str, links: &str) -> GraphMaterial {
    let comma = |s: &str| {
        if s.trim().is_empty() {
            String::new()
        } else {
            format!(",{s}")
        }
    };
    graph(&format!(
        r#"{{"format":"operandi-node-graph","format_version":1,"licence":null,
        "materials":[{{"name":"Test","output":"Material Output",
        "nodes":[
          {{"name":"Material Output","type":"ShaderNodeOutputMaterial","props":{{}},"inputs":{{}}}},
          {{"name":"Principled BSDF","type":"ShaderNodeBsdfPrincipled","props":{{}},"inputs":{{{principled_inputs}}}}}
          {}
        ],
        "links":[{{"from":["Principled BSDF","BSDF"],"to":["Material Output","Surface"]}} {}],
        "exposed":[],"groups_used":[]}}],"groups":[]}}"#,
        comma(nodes),
        comma(links)
    ))
}

fn image(name: &str, file: &str, colorspace: &str) -> String {
    format!(
        r#"{{"name":"{name}","type":"ShaderNodeTexImage","props":{{"interpolation":"Linear","projection":"FLAT","extension":"REPEAT"}},
        "image":{{"path":"{file}","colorspace":"{colorspace}","packed":false}},"inputs":{{}}}}"#
    )
}

fn sample() -> GraphFile {
    GraphFile::parse(include_str!(
        "../../../../../tools/blender-export/samples/Metal049A_1K-JPG.json"
    ))
    .unwrap()
}

// ------------------------------------------------------------ suffixes

#[test]
fn suffixes_of_both_sources_are_recognised() {
    let kind = |stem: &str| match parse_map_name(stem) {
        Some(MapName::Map { kind, .. }) => Some(kind),
        _ => None,
    };
    // Poly Haven.
    assert_eq!(kind("plywood_diff_1k"), Some(MapKind::BaseColor));
    assert_eq!(kind("plywood_nor_gl_1k"), Some(MapKind::NormalGl));
    assert_eq!(kind("plywood_nor_dx_1k"), Some(MapKind::NormalDx));
    assert_eq!(kind("metal_plate_arm_1k"), Some(MapKind::Packed));
    assert_eq!(kind("plywood_rough_1k"), Some(MapKind::Roughness));
    assert_eq!(kind("plywood_disp_1k"), Some(MapKind::Height));
    // ambientCG.
    assert_eq!(kind("Metal049A_1K-JPG_Color"), Some(MapKind::BaseColor));
    assert_eq!(kind("Metal049A_1K-JPG_Metalness"), Some(MapKind::Metallic));
    assert_eq!(kind("Metal049A_1K-JPG_NormalDX"), Some(MapKind::NormalDx));
    assert_eq!(kind("Metal049A_1K-JPG_Displacement"), Some(MapKind::Height));
    assert_eq!(kind("Fence006_1K-JPG_Opacity"), Some(MapKind::Opacity));
    assert_eq!(kind("hessian_230_alpha_1k"), Some(MapKind::Opacity));
    // One base, so one set.
    let set = set_of(&[
        "Metal049A_1K-JPG_Color.jpg",
        "Metal049A_1K-JPG_Roughness.jpg",
        "Metal049A_1K-JPG_NormalGL.jpg",
        "Metal049A_1K-JPG_NormalDX.jpg",
    ]);
    assert_eq!(set.maps.len(), 4);
}

/// Regression: a Poly Haven paint mask became an inverted Dissolve layer and
/// made wood_peeling_paint_weathered see-through where the paint was missing.
#[test]
fn a_bare_mask_map_is_not_opacity() {
    assert_eq!(parse_map_name("wood_peeling_paint_weathered_mask_1k"), None);
    let set = set_of(&[
        "wood_peeling_paint_weathered_diff_1k.jpg",
        "wood_peeling_paint_weathered_rough_1k.jpg",
        "wood_peeling_paint_weathered_mask_1k.jpg",
    ]);
    assert_eq!(set.maps.len(), 2);
    assert!(set.maps.iter().all(|m| m.kind != MapKind::Opacity));
}

// ------------------------------------------------------------ colour spaces

#[test]
fn colour_maps_read_as_srgb_and_data_maps_as_numbers() {
    assert_eq!(colour_space(TextureEffect::DiffuseColor), ColourSpace::Srgb);
    assert_eq!(
        colour_space(TextureEffect::LuminousColor),
        ColourSpace::Srgb
    );
    for e in [
        TextureEffect::Roughness,
        TextureEffect::Metallic,
        TextureEffect::Normal,
        TextureEffect::Bump,
        TextureEffect::Displacement,
        TextureEffect::Dissolve,
    ] {
        assert_eq!(colour_space(e), ColourSpace::Linear, "{e:?}");
    }
    // The renderer side agrees (operandi-cycles reads by input).
    assert!(Input::BaseColor.is_colour());
    assert!(!Input::Roughness.is_colour() && !Input::Metallic.is_colour());
}

#[test]
fn a_colour_map_blender_read_as_data_is_noted() {
    let m = simple_graph(
        "",
        &image("Img", "wood_diff.jpg", "Non-Color"),
        r#"{"from":["Img","Color"],"to":["Principled BSDF","Base Color"]}"#,
    );
    let a = analyse(&m).unwrap();
    assert!(
        a.notes.iter().any(|n| n.contains("read as sRGB")),
        "{:?}",
        a.notes
    );
}

// ------------------------------------------------------------ DX normals

#[test]
fn a_directx_normal_has_its_green_flipped() {
    // Loader path (no graph): only a DX normal in the set.
    let set = set_of(&["bricks_diff_1k.jpg", "bricks_nor_dx_1k.jpg"]);
    let c = convert(&set, None, None, &size).unwrap();
    let normal = layers(&c.preset)
        .into_iter()
        .find(|l| l.0 == TextureEffect::Normal)
        .unwrap();
    assert!(normal.2.invert_green && !normal.2.invert_red);
    // OpenGL wins when both are there, unflipped.
    let set = set_of(&[
        "bricks_diff_1k.jpg",
        "bricks_nor_dx_1k.jpg",
        "bricks_nor_gl_1k.jpg",
    ]);
    let c = convert(&set, None, None, &size).unwrap();
    let normals: Vec<_> = layers(&c.preset)
        .into_iter()
        .filter(|l| l.0 == TextureEffect::Normal)
        .collect();
    assert_eq!(normals.len(), 1);
    assert_eq!(normals[0].1, "bricks_nor_gl_1k.jpg");
    assert!(!normals[0].2.invert_green);
    // Graph path: a DX map through a Normal Map node is flipped too.
    let m = simple_graph(
        "",
        &format!(
            r#"{},{{"name":"NM","type":"ShaderNodeNormalMap","props":{{"space":"TANGENT"}},"inputs":{{"Strength":{{"type":"VALUE","value":0.5}}}}}}"#,
            image("N", "bricks_nor_dx_1k.jpg", "Non-Color")
        ),
        r#"{"from":["N","Color"],"to":["NM","Color"]},{"from":["NM","Normal"],"to":["Principled BSDF","Normal"]}"#,
    );
    let c = convert(&set, Some((&m, Path::new("/maps"))), None, &size).unwrap();
    let l = &layers(&c.preset)[0];
    assert_eq!(l.0, TextureEffect::Normal);
    assert!(l.2.invert_green);
    assert_eq!(l.3.opacity, 0.5, "Normal Map Strength is the layer opacity");
}

// ------------------------------------------------------------ packed ARM

#[test]
fn a_packed_arm_map_splits_into_ao_roughness_and_metal() {
    let set = set_of(&["plate_diff_1k.jpg", "plate_arm_1k.jpg"]);
    let c = convert(&set, None, None, &size).unwrap();
    let ls = layers(&c.preset);
    let find = |e: TextureEffect, s: Swizzle| {
        ls.iter()
            .find(|l| l.0 == e && l.2.swizzle == s && l.1 == "plate_arm_1k.jpg")
    };
    let ao = find(TextureEffect::DiffuseColor, Swizzle::Red).expect("AO from red");
    assert_eq!(ao.3.blend, BlendMode::Multiply, "AO multiplies base colour");
    assert!(find(TextureEffect::Roughness, Swizzle::Green).is_some());
    assert!(find(TextureEffect::Metallic, Swizzle::Blue).is_some());
    // The AO multiply sits above the colour it darkens.
    let ao_at = ls
        .iter()
        .position(|l| l.3.blend == BlendMode::Multiply)
        .unwrap();
    let colour_at = ls.iter().position(|l| l.1 == "plate_diff_1k.jpg").unwrap();
    assert!(ao_at < colour_at);
    // The same split through a graph's Separate Color.
    let m = simple_graph(
        "",
        &format!(
            r#"{},{},{{"name":"Sep","type":"ShaderNodeSeparateColor","props":{{"mode":"RGB"}},"inputs":{{}}}},
            {{"name":"AO","type":"ShaderNodeMix","props":{{"data_type":"RGBA","blend_type":"MULTIPLY"}},"inputs":{{"Factor_Float":{{"type":"VALUE","value":0.8}}}}}}"#,
            image("C", "plate_diff_1k.jpg", "sRGB"),
            image("ARM", "plate_arm_1k.jpg", "Non-Color")
        ),
        r#"{"from":["ARM","Color"],"to":["Sep","Color"]},
           {"from":["Sep","Green"],"to":["Principled BSDF","Roughness"]},
           {"from":["Sep","Blue"],"to":["Principled BSDF","Metallic"]},
           {"from":["C","Color"],"to":["AO","A","A_Color"]},
           {"from":["Sep","Red"],"to":["AO","B","B_Color"]},
           {"from":["AO","Result","Result_Color"],"to":["Principled BSDF","Base Color"]}"#,
    );
    let c = convert(&set, Some((&m, Path::new("/maps"))), None, &size).unwrap();
    let ls = layers(&c.preset);
    let ao = &ls[0];
    assert_eq!(
        (ao.0, ao.2.swizzle, ao.3.blend),
        (
            TextureEffect::DiffuseColor,
            Swizzle::Red,
            BlendMode::Multiply
        )
    );
    assert!((ao.3.opacity - 0.8).abs() < 1e-9);
    assert_eq!(
        (ls[1].0, ls[1].2.swizzle),
        (TextureEffect::DiffuseColor, Swizzle::Rgba)
    );
    assert!(
        ls.iter()
            .any(|l| l.0 == TextureEffect::Roughness && l.2.swizzle == Swizzle::Green)
    );
    assert!(
        ls.iter()
            .any(|l| l.0 == TextureEffect::Metallic && l.2.swizzle == Swizzle::Blue)
    );
}

// ------------------------------------------------------------ Principled values

/// The Principled inputs our forward translation gives `m`.
fn forward(m: &MaterialChannels) -> Vec<(Input, [f64; 3])> {
    let mut tree = ShaderTree::default();
    for node in &mut tree.nodes {
        if let NodeKind::Material(x) = &mut node.kind {
            **x = m.clone();
        }
    }
    let ctx = Context {
        item: 0,
        ancestors: &[],
        material: crate::material::DEFAULT_MATERIAL,
        part: None,
        sets: &[],
        locators: &[],
    };
    tree.evaluate(&ctx)
        .principled()
        .inputs
        .into_iter()
        .filter_map(|(i, e)| e.constant().map(|c| (i, c)))
        .collect()
}

#[test]
fn principled_values_come_back_through_the_translation_in_reverse() {
    let m = simple_graph(
        r#""Base Color":{"type":"RGBA","value":[0.2,0.3,0.4,1.0]},
           "Metallic":{"type":"VALUE","value":0.7},
           "Roughness":{"type":"VALUE","value":0.35},
           "IOR":{"type":"VALUE","value":1.5},
           "Specular IOR Level":{"type":"VALUE","value":0.8},
           "Alpha":{"type":"VALUE","value":0.25},
           "Coat Weight":{"type":"VALUE","value":0.1},
           "Emission Color":{"type":"RGBA","value":[1.0,0.5,0.0,1.0]},
           "Emission Strength":{"type":"VALUE","value":2.0}"#,
        "",
        "",
    );
    let a = analyse(&m).unwrap();
    let mut notes = Vec::new();
    let ch = material_channels(&a.principled, &mut notes);
    assert_eq!(ch.diffuse_color, [0.2, 0.3, 0.4]);
    assert_eq!(
        ch.specular_amount, 0.8,
        "Specular IOR Level is Specular Amount"
    );
    assert!(!ch.more.use_refractive_index);
    assert!((ch.dissolve - 0.75).abs() < 1e-9, "Alpha = 1 − Dissolve");
    let back = forward(&ch);
    let get = |i: Input| back.iter().find(|(x, _)| *x == i).map(|(_, v)| *v).unwrap();
    assert_eq!(get(Input::BaseColor), [0.2, 0.3, 0.4]);
    assert_eq!(get(Input::Metallic)[0], 0.7);
    assert_eq!(get(Input::Roughness)[0], 0.35);
    assert_eq!(get(Input::Ior)[0], 1.5);
    assert_eq!(get(Input::SpecularIorLevel)[0], 0.8);
    assert!((get(Input::Alpha)[0] - 0.25).abs() < 1e-9);
    assert_eq!(get(Input::CoatWeight)[0], 0.1);
    assert_eq!(get(Input::EmissionColor), [1.0, 0.5, 0.0]);
    assert_eq!(get(Input::EmissionStrength)[0], 2.0);
}

#[test]
fn defaults_are_a_principled_bsdfs_and_another_ior_uses_the_refractive_index() {
    let ch = material_channels(&GraphNode::default(), &mut Vec::new());
    assert_eq!(ch.roughness, 0.5);
    assert_eq!(ch.specular_amount, 0.5);
    assert_eq!(ch.metallic, 0.);
    let m = simple_graph(r#""IOR":{"type":"VALUE","value":1.33}"#, "", "");
    let a = analyse(&m).unwrap();
    let ch = material_channels(&a.principled, &mut Vec::new());
    assert!(ch.more.use_refractive_index);
    assert_eq!(ch.refractive_index, 1.33);
    let back = forward(&ch);
    let ior = back.iter().find(|(i, _)| *i == Input::Ior).unwrap().1[0];
    assert_eq!(ior, 1.33);
}

#[test]
fn a_set_without_a_graph_gets_principled_defaults() {
    let set = set_of(&["wood_diff_1k.jpg", "wood_rough_1k.jpg", "wood_disp_1k.jpg"]);
    let c = convert(&set, None, None, &size).unwrap();
    let m = material(&c.preset);
    assert_eq!((m.roughness, m.specular_amount), (0.5, 0.5));
    assert_eq!(
        m.displacement_distance, 0.01,
        "the loader's height distance stays"
    );
    // Roughness isn't inverted: white is rough.
    let rough = layers(&c.preset)
        .into_iter()
        .find(|l| l.0 == TextureEffect::Roughness)
        .unwrap();
    assert!(!rough.2.invert_red && !rough.3.invert);
}

// ------------------------------------------------------------ simple or not

#[test]
fn the_real_ambientcg_sample_is_simple_and_converts() {
    let file = sample();
    assert_eq!(
        file.licence.as_ref().unwrap().name.as_deref(),
        Some("CC0-1.0")
    );
    let set = set_of(&[
        "Metal049A_1K-JPG_Color.jpg",
        "Metal049A_1K-JPG_Roughness.jpg",
        "Metal049A_1K-JPG_Metalness.jpg",
        "Metal049A_1K-JPG_NormalGL.jpg",
        "Metal049A_1K-JPG_NormalDX.jpg",
        "Metal049A_1K-JPG_Displacement.jpg",
    ]);
    let m = pick_material(&file.materials, &set).unwrap();
    assert_eq!(
        m.name, "Metal049A_1K-JPG_NORMAL",
        "the normal-mapped variant"
    );
    let c = convert(&set, Some((m, Path::new("/maps"))), None, &size).unwrap();
    let effects: Vec<TextureEffect> = layers(&c.preset).iter().map(|l| l.0).collect();
    assert_eq!(
        effects,
        [
            TextureEffect::DiffuseColor,
            TextureEffect::Roughness,
            TextureEffect::Metallic,
            TextureEffect::Normal
        ]
    );
    // Every layer shares one Texture Locator.
    let locators: std::collections::BTreeSet<_> = c
        .preset
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => t.locator,
            _ => None,
        })
        .collect();
    assert_eq!(locators.len(), 1);
    assert!(
        c.notes
            .iter()
            .any(|n| n.starts_with("Metal049A_1K-JPG_NormalDX.jpg isn't used")),
        "{:?}",
        c.notes
    );
    // The variant with displacement converts too.
    let d = file
        .materials
        .iter()
        .find(|m| m.name.ends_with("DISPLACEMENT"))
        .unwrap();
    let c = convert(&set, Some((d, Path::new("/maps"))), None, &size).unwrap();
    assert!(
        layers(&c.preset)
            .iter()
            .any(|l| l.0 == TextureEffect::Displacement)
    );
}

#[test]
fn a_graph_with_other_nodes_is_refused_whole() {
    let m = simple_graph(
        "",
        &format!(
            r#"{},{{"name":"Noise","type":"ShaderNodeTexNoise","props":{{}},"inputs":{{}}}}"#,
            image("C", "wood_diff_1k.jpg", "sRGB")
        ),
        r#"{"from":["C","Color"],"to":["Principled BSDF","Base Color"]},
           {"from":["Noise","Fac"],"to":["Principled BSDF","Roughness"]}"#,
    );
    let set = set_of(&["wood_diff_1k.jpg", "wood_rough_1k.jpg"]);
    let err = convert(&set, Some((&m, Path::new("/maps"))), None, &size).unwrap_err();
    let ConvertError::Refused(r) = &err else {
        panic!("{err}")
    };
    assert!(r.reason.contains("TexNoise"), "{r}");
    assert!(
        err.to_string()
            .contains("needs Node Material (not yet supported)")
    );
}

#[test]
fn shapes_layers_cant_express_are_refused() {
    let refused = |m: GraphMaterial| analyse(&m).unwrap_err().reason;
    // A Mix whose factor is driven.
    let r = refused(simple_graph(
        "",
        &format!(
            r#"{},{},{},{{"name":"Mix","type":"ShaderNodeMix","props":{{"data_type":"RGBA","blend_type":"MIX"}},"inputs":{{}}}}"#,
            image("A", "a_diff.jpg", "sRGB"),
            image("B", "b_diff.jpg", "sRGB"),
            image("F", "f_mask.jpg", "Non-Color")
        ),
        r#"{"from":["A","Color"],"to":["Mix","A","A_Color"]},
           {"from":["B","Color"],"to":["Mix","B","B_Color"]},
           {"from":["F","Color"],"to":["Mix","Factor","Factor_Float"]},
           {"from":["Mix","Result","Result_Color"],"to":["Principled BSDF","Base Color"]}"#,
    ));
    assert!(r.contains("factor"), "{r}");
    // A surface that isn't a Principled BSDF.
    let m = graph(
        r#"{"format":"operandi-node-graph","format_version":1,"materials":[{"name":"E","output":"Out",
        "nodes":[{"name":"Out","type":"ShaderNodeOutputMaterial"},{"name":"Em","type":"ShaderNodeEmission"}],
        "links":[{"from":["Em","Emission"],"to":["Out","Surface"]}]}]}"#,
    );
    assert!(refused(m).contains("not a Principled BSDF"));
    // Math other than multiply.
    let r = refused(simple_graph(
        "",
        &format!(
            r#"{},{{"name":"M","type":"ShaderNodeMath","props":{{"operation":"POWER"}},"inputs":{{"Value_001":{{"type":"VALUE","value":2.0}}}}}}"#,
            image("R", "r_rough.jpg", "Non-Color")
        ),
        r#"{"from":["R","Color"],"to":["M","Value","Value"]},{"from":["M","Value"],"to":["Principled BSDF","Roughness"]}"#,
    ));
    assert!(r.contains("POWER"), "{r}");
    // Images placed by two different mappings.
    let mapping = |name: &str, s: f64| {
        format!(
            r#"{{"name":"{name}","type":"ShaderNodeMapping","props":{{"vector_type":"POINT"}},"inputs":{{"Location":{{"type":"VECTOR","value":[0,0,0]}},"Rotation":{{"type":"VECTOR","value":[0,0,0]}},"Scale":{{"type":"VECTOR","value":[{s},{s},1]}}}}}}"#
        )
    };
    let r = refused(simple_graph(
        "",
        &format!(
            "{},{},{},{}",
            image("A", "a_diff.jpg", "sRGB"),
            image("R", "a_rough.jpg", "Non-Color"),
            mapping("M1", 2.),
            mapping("M2", 3.)
        ),
        r#"{"from":["M1","Vector"],"to":["A","Vector"]},{"from":["M2","Vector"],"to":["R","Vector"]},
           {"from":["A","Color"],"to":["Principled BSDF","Base Color"]},{"from":["R","Color"],"to":["Principled BSDF","Roughness"]}"#,
    ));
    assert!(r.contains("different mappings"), "{r}");
}

#[test]
fn unreached_nodes_are_ignored() {
    let m = simple_graph(
        "",
        &format!(
            r#"{},{{"name":"Stray","type":"ShaderNodeTexVoronoi","props":{{}},"inputs":{{}}}}"#,
            image("C", "w_diff.jpg", "sRGB")
        ),
        r#"{"from":["C","Color"],"to":["Principled BSDF","Base Color"]}"#,
    );
    assert!(analyse(&m).is_ok());
}

// ------------------------------------------------------------ mapping, bump, displacement

#[test]
fn mapping_becomes_the_locators_repeat_and_rotation() {
    let make = |kind: &str, scale: f64, z: f64| {
        simple_graph(
            "",
            &format!(
                r#"{},{{"name":"Map","type":"ShaderNodeMapping","props":{{"vector_type":"{kind}"}},"inputs":{{"Location":{{"type":"VECTOR","value":[0,0,0]}},"Rotation":{{"type":"VECTOR","value":[0,0,{z}]}},"Scale":{{"type":"VECTOR","value":[{scale},{scale},1]}}}}}},
                {{"name":"TC","type":"ShaderNodeTexCoord","props":{{}},"inputs":{{}}}}"#,
                image("C", "w_diff.jpg", "sRGB")
            ),
            r#"{"from":["TC","UV"],"to":["Map","Vector"]},{"from":["Map","Vector"],"to":["C","Vector"]},
               {"from":["C","Color"],"to":["Principled BSDF","Base Color"]}"#,
        )
    };
    let quarter = std::f64::consts::FRAC_PI_2;
    let a = analyse(&make("POINT", 4., quarter)).unwrap();
    assert_eq!(a.mapping.repeat, [4., 4.]);
    assert!((a.mapping.rotation - 90.).abs() < 1e-9);
    let a = analyse(&make("TEXTURE", 2., quarter)).unwrap();
    assert_eq!(a.mapping.repeat, [0.5, 0.5]);
    assert!((a.mapping.rotation + 90.).abs() < 1e-9);
    // The converted locator carries it.
    let set = set_of(&["w_diff.jpg", "w_rough.jpg"]);
    let c = convert(
        &set,
        Some((&make("POINT", 4., 0.), Path::new("/m"))),
        None,
        &size,
    )
    .unwrap();
    let loc = c
        .preset
        .scene
        .items
        .iter()
        .find_map(|i| i.texture_locator())
        .unwrap();
    assert_eq!(loc.repeat, [4., 4.]);
}

#[test]
fn bump_and_displacement_keep_their_strengths() {
    let m = graph(
        r#"{"format":"operandi-node-graph","format_version":1,"materials":[{"name":"B","output":"Out",
        "nodes":[{"name":"Out","type":"ShaderNodeOutputMaterial"},
          {"name":"P","type":"ShaderNodeBsdfPrincipled"},
          {"name":"H","type":"ShaderNodeTexImage","props":{"projection":"FLAT"},"image":{"path":"s_disp.jpg","colorspace":"Non-Color"}},
          {"name":"Bump","type":"ShaderNodeBump","props":{"invert":false},"inputs":{"Strength":{"type":"VALUE","value":0.5},"Distance":{"type":"VALUE","value":0.02}}},
          {"name":"D","type":"ShaderNodeDisplacement","props":{"space":"OBJECT"},"inputs":{"Midlevel":{"type":"VALUE","value":0.5},"Scale":{"type":"VALUE","value":0.1}}}],
        "links":[{"from":["P","BSDF"],"to":["Out","Surface"]},
          {"from":["H","Color"],"to":["Bump","Height"]},{"from":["Bump","Normal"],"to":["P","Normal"]},
          {"from":["H","Color"],"to":["D","Height"]},{"from":["D","Displacement"],"to":["Out","Displacement"]}]}]}"#,
    );
    let set = set_of(&["s_diff.jpg", "s_disp.jpg"]);
    let c = convert(&set, Some((&m, Path::new("/m"))), None, &size).unwrap();
    let ls = layers(&c.preset);
    let bump = ls
        .iter()
        .find(|l| l.0 == TextureEffect::Bump)
        .expect("height into Bump is a bump layer");
    assert_eq!(bump.1, "s_disp.jpg");
    let disp = ls
        .iter()
        .find(|l| l.0 == TextureEffect::Displacement)
        .unwrap();
    assert_eq!(
        (disp.2.low_value, disp.2.high_value),
        (-0.5, 0.5),
        "around the midlevel"
    );
    let mat = material(&c.preset);
    assert!(
        (mat.bump_amplitude - 0.01).abs() < 1e-12,
        "Distance × Strength"
    );
    assert_eq!(mat.displacement_distance, 0.1);
}

// ------------------------------------------------------------ metadata

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-convert-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn licence_and_credit_round_trip_and_older_presets_still_load() {
    let dir = temp_dir("meta");
    let set = set_of(&["wood_diff_1k.jpg", "wood_rough_1k.jpg"]);
    let credit = PresetCredit {
        licence: "CC0-1.0".into(),
        url: "https://polyhaven.com/a/wood".into(),
        credit: "Poly Haven".into(),
    };
    let mut c = convert(&set, None, Some(credit.clone()), &size).unwrap();
    // A pre-OpenPBR material, so the older version is what is tested.
    c.preset.scene.shader_tree.legacy_openpbr();
    let path = dir.join("wood.oppreset");
    crate::persistence::save_preset(&path, &c.preset).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"version\": 37"), "credits need version 37");
    let back = crate::persistence::load_preset(&path).unwrap();
    assert_eq!(back.header.source, Some(credit));
    // Without credits the file keeps an older version and loads.
    let plain = convert(&set, None, None, &size).unwrap();
    let old = dir.join("plain.oppreset");
    crate::persistence::save_preset(&old, &plain.preset).unwrap();
    let text = std::fs::read_to_string(&old).unwrap();
    assert!(!text.contains("\"version\": 37"));
    assert_eq!(
        crate::persistence::load_preset(&old).unwrap().header.source,
        None
    );
    // A credited preset claiming an older version is refused.
    let forged = dir.join("forged.oppreset");
    std::fs::write(
        &forged,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"version\": 37", "\"version\": 36"),
    )
    .unwrap();
    let err = crate::persistence::load_preset(&forged).unwrap_err();
    assert!(err.contains("version 37"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn relative_picture_paths_are_found_from_the_preset_folder() {
    let dir = temp_dir("relative");
    let set = set_of(&["wood_diff_1k.jpg", "wood_rough_1k.jpg"]);
    let mut c = convert(&set, None, None, &size).unwrap();
    for image in &mut c.preset.scene.images {
        image.path = format!("wood/{}", file_name(&image.path));
    }
    let path = dir.join("wood.oppreset");
    crate::persistence::save_preset(&path, &c.preset).unwrap();
    let back = crate::persistence::load_preset(&path).unwrap();
    for image in &back.scene.images {
        assert!(
            Path::new(&image.path).starts_with(dir.join("wood")),
            "{}",
            image.path
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_converted_preset_loads_into_a_material_group_as_one_step() {
    let set = set_of(&[
        "wood_diff_1k.jpg",
        "wood_rough_1k.jpg",
        "wood_nor_gl_1k.jpg",
    ]);
    let c = convert(&set, None, None, &size).unwrap();
    let mut e = crate::Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(crate::Command::ShaderTree(
        crate::shader_tree::ShaderTreeEdit::Add {
            layer: NewLayer::Group,
            selected: None,
        },
    ))
    .unwrap();
    let group = e
        .scene
        .render_shader_tree()
        .nodes
        .iter()
        .find(|n: &&Node| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id);
    let undo_before = e.undo.len();
    e.load_preset(c.preset, group).unwrap();
    assert_eq!(e.undo.len(), undo_before + 1);
    let tree = e.scene.render_shader_tree();
    let textures = tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
        .count();
    assert_eq!(textures, 3);
}

// ------------------------------------------------------------ format rules

#[test]
fn value_and_rgb_nodes_are_read_from_outputs_then_props() {
    // A Value node (outputs) scales a roughness map through Math Multiply.
    let m = simple_graph(
        "",
        &format!(
            r#"{},{{"name":"K","type":"ShaderNodeValue","props":{{}},"inputs":{{}},"outputs":{{"Value":{{"type":"VALUE","value":0.5}}}}}},
            {{"name":"M","type":"ShaderNodeMath","props":{{"operation":"MULTIPLY"}},"inputs":{{}}}}"#,
            image("R", "w_rough.jpg", "Non-Color")
        ),
        r#"{"from":["R","Color"],"to":["M","Value","Value"]},{"from":["K","Value"],"to":["M","Value","Value_001"]},
           {"from":["M","Value"],"to":["Principled BSDF","Roughness"]}"#,
    );
    let a = analyse(&m).unwrap();
    assert_eq!(a.layers[0].multiply, 0.5);
    // An RGB node (outputs) and an early export's props.value both read.
    for rgb in [
        r#""outputs":{"Color":{"type":"RGBA","value":[0.1,0.2,0.3,1.0]}}"#,
        r#""props":{"value":[0.1,0.2,0.3,1.0]}"#,
    ] {
        let m = simple_graph(
            "",
            &format!(r#"{{"name":"C","type":"ShaderNodeRGB",{rgb}}}"#),
            r#"{"from":["C","Color"],"to":["Principled BSDF","Base Color"]}"#,
        );
        let a = analyse(&m).unwrap();
        assert!(
            a.notes.iter().any(|n| n.contains("0.100, 0.200, 0.300")),
            "{:?}",
            a.notes
        );
    }
}

#[test]
fn group_interface_entries_read_with_optional_identifiers() {
    let file = GraphFile::parse(
        r#"{"format":"operandi-node-graph","format_version":1,"materials":[],
        "groups":[{"name":"Tint","nodes":[],"links":[],
          "inputs":[{"name":"Color","identifier":"Socket_1","type":"RGBA","default":[1,1,1,1]},
                    {"name":"Amount","type":"VALUE","default":0.5}],
          "outputs":[{"name":"Result","type":"RGBA","default":null}]}]}"#,
    )
    .unwrap();
    let g = &file.groups[0];
    assert_eq!(g.inputs[0].identifier.as_deref(), Some("Socket_1"));
    assert_eq!(g.inputs[1].identifier, None);
    assert_eq!(g.inputs[1].kind, "VALUE");
    assert_eq!(g.outputs[0].name, "Result");
}

#[test]
fn group_nodes_name_their_group_in_props_group_with_node_tree_fallback() {
    let node = |props: &str| -> crate::node_graph_reader::GraphNode {
        serde_json::from_str(&format!(
            r#"{{"name":"Group","type":"ShaderNodeGroup","props":{props}}}"#
        ))
        .unwrap()
    };
    assert_eq!(node(r#"{"group":"Tint"}"#).group_name(), Some("Tint"));
    assert_eq!(node(r#"{"node_tree":"Tint"}"#).group_name(), Some("Tint"));
    assert_eq!(
        node(r#"{"group":"Tint","node_tree":"Other"}"#).group_name(),
        Some("Tint")
    );
    assert_eq!(node("{}").group_name(), None);
}
