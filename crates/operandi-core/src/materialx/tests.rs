//! MX-T1..T4 and the round trip (MX-06).
use super::*;
use crate::openpbr::Param;
use crate::preset::texture_set::{ResolutionChoice, texture_sets};
use crate::shader_tree::{
    BlendMode, ImageSettings, MaterialChannels, NewLayer, NodeKind, ShaderTreeEdit, TextureEffect,
    TextureSource,
};
use crate::{Command, Editor, Scene};
use std::path::{Path, PathBuf};

fn any_size(_: &Path) -> Option<(u32, u32)> {
    Some((8, 8))
}

fn from_text(text: &str, folder: &Path) -> crate::Result<(Preset, Vec<String>)> {
    import::preset(&doc::parse(text).unwrap(), None, folder, &any_size)
}

type LayerKey = (TextureEffect, String, BlendMode, u32, ImageSettings);

/// A preset's image layers, top first: effect, picture file name, blend,
/// opacity (in %) and adjustments.
fn layers(scene: &Scene) -> Vec<LayerKey> {
    scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => match &t.source {
                TextureSource::ImageMap {
                    image: Some(id),
                    settings,
                } => {
                    let pic = scene.images.iter().find(|i| i.id == *id).unwrap();
                    let file = Path::new(&pic.path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    Some((
                        t.effect,
                        file,
                        n.layer.blend,
                        (n.layer.opacity * 100.).round() as u32,
                        (**settings).clone(),
                    ))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn material(scene: &Scene) -> MaterialChannels {
    scene
        .shader_tree
        .nodes
        .iter()
        .find_map(|n| match &n.kind {
            NodeKind::Material(m) => Some((**m).clone()),
            _ => None,
        })
        .unwrap()
}

/// The shape ambientCG's .mtlx files have (tiledimage, normalmap,
/// displacement), written here for the test.
const BRICKS: &str = r#"<?xml version="1.0"?>
<materialx version="1.39" fileprefix="./">
  <open_pbr_surface name="Bricks_Surface" type="surfaceshader">
    <input nodename="Bricks_Color" name="base_color" type="color3" />
    <input nodename="NormalMap" name="geometry_normal" type="vector3" />
    <input nodename="Bricks_Roughness" name="specular_roughness" type="float" />
  </open_pbr_surface>
  <surfacematerial name="Bricks" type="material">
    <input nodename="Bricks_Surface" name="surfaceshader" type="surfaceshader" />
    <input nodename="displacement" name="displacementshader" type="displacementshader" />
  </surfacematerial>
  <tiledimage name="Bricks_Color" type="color3">
    <input colorspace="srgb_texture" name="file" type="filename" value="Bricks_1K-JPG_Color.jpg" />
    <input name="uvtiling" type="vector2" value="1.0, 1.0" />
  </tiledimage>
  <tiledimage name="Bricks_Displacement" type="float">
    <input name="file" type="filename" value="Bricks_1K-JPG_Displacement.jpg" />
    <input name="uvtiling" type="vector2" value="1.0, 1.0" />
  </tiledimage>
  <displacement name="displacement" type="displacementshader">
    <input nodename="Bricks_Displacement" name="displacement" type="float" />
    <input name="scale" type="float" value="1.0" />
  </displacement>
  <tiledimage name="Bricks_NormalGL" type="vector3">
    <input name="file" type="filename" value="Bricks_1K-JPG_NormalGL.jpg" />
    <input name="uvtiling" type="vector2" value="1.0, 1.0" />
  </tiledimage>
  <normalmap name="NormalMap" type="vector3">
    <input nodename="Bricks_NormalGL" name="in" type="vector3" />
    <input name="scale" type="float" value="1.0" />
  </normalmap>
  <tiledimage name="Bricks_Roughness" type="float">
    <input name="file" type="filename" value="Bricks_1K-JPG_Roughness.jpg" />
    <input name="uvtiling" type="vector2" value="1.0, 1.0" />
  </tiledimage>
</materialx>"#;

#[test]
fn mx_t1_an_ambientcg_file_gives_its_texture_sets_layers() {
    let folder = PathBuf::from("/content/Bricks");
    let (preset, notes) = from_text(BRICKS, &folder).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    let files: Vec<PathBuf> = [
        "Bricks_1K-JPG_Color.jpg",
        "Bricks_1K-JPG_Displacement.jpg",
        "Bricks_1K-JPG_NormalGL.jpg",
        "Bricks_1K-JPG_Roughness.jpg",
    ]
    .iter()
    .map(|f| folder.join(f))
    .collect();
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let sets = texture_sets(&refs);
    let (set_preset, _) = sets[0]
        .material_preset(ResolutionChoice::Highest, &any_size)
        .unwrap();
    assert_eq!(layers(&preset.scene), layers(&set_preset.scene));
    // Same displacement distance (1 cm) and a Material underneath.
    assert_eq!(
        material(&preset.scene).displacement_distance,
        material(&set_preset.scene).displacement_distance
    );
    assert_eq!(preset.header.kind, crate::preset::PresetKind::Material);
    let group = &preset.scene.shader_tree.nodes[0];
    assert_eq!(group.name, "Bricks");
}

#[test]
fn mx_q03_a_texture_sets_ao_joins_its_materialx_material() {
    let folder = PathBuf::from("/content/Bricks");
    let (mut preset, _) = from_text(BRICKS, &folder).unwrap();
    let files: Vec<PathBuf> = [
        "Bricks_1K-JPG_AmbientOcclusion.jpg",
        "Bricks_1K-JPG_Color.jpg",
        "Bricks_1K-JPG_Displacement.jpg",
        "Bricks_1K-JPG_NormalGL.jpg",
        "Bricks_1K-JPG_Roughness.jpg",
    ]
    .iter()
    .map(|f| folder.join(f))
    .collect();
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let set = &texture_sets(&refs)[0];
    let notes = set
        .add_ambient_occlusion(&mut preset, ResolutionChoice::Highest, &any_size)
        .unwrap();
    assert!(notes.is_empty());
    let (set_preset, _) = set
        .material_preset(ResolutionChoice::Highest, &any_size)
        .unwrap();
    // Now exactly the set's layers, the AO multiply on top.
    assert_eq!(layers(&preset.scene), layers(&set_preset.scene));
    assert_eq!(layers(&preset.scene)[0].2, BlendMode::Multiply);
    // Twice adds nothing more.
    set.add_ambient_occlusion(&mut preset, ResolutionChoice::Highest, &any_size)
        .unwrap();
    assert_eq!(layers(&preset.scene).len(), layers(&set_preset.scene).len());
}

/// A scene holding a material group loaded from `preset`.
fn editor_with(preset: Preset) -> (Editor, NodeId) {
    let mut e = Editor::default();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id);
    e.dispatch(Command::LoadPreset {
        preset: Box::new(preset),
        group,
    })
    .unwrap();
    (e, group.unwrap())
}

fn round_trip(e: &Editor, group: NodeId) -> (Preset, Vec<String>, String) {
    let folder = PathBuf::from("/content/Bricks");
    let (xml, losses) = export::material_x(&e.scene, group, &folder).unwrap();
    let (back, _) = from_text(&xml, &folder).unwrap();
    (back, losses, xml)
}

#[test]
fn mx_t2_a_texture_set_material_exports_and_reimports_the_same() {
    let folder = PathBuf::from("/content/Bricks");
    let files: Vec<PathBuf> = [
        "Bricks_4K_AmbientOcclusion.png",
        "Bricks_4K_Color.png",
        "Bricks_4K_Roughness.png",
        "Bricks_4K_Metalness.png",
        "Bricks_4K_NormalDX.png",
        "Bricks_4K_Displacement.png",
        "Bricks_4K_Opacity.png",
    ]
    .iter()
    .map(|f| folder.join(f))
    .collect();
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let (preset, _) = texture_sets(&refs)[0]
        .material_preset(ResolutionChoice::Highest, &any_size)
        .unwrap();
    let want_layers = layers(&preset.scene);
    let (mut e, group) = editor_with(preset);
    // Change the material too, so constants travel.
    let mat = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Material(_)))
        .unwrap()
        .id;
    let mut node = e.scene.shader_tree.get(mat).unwrap().clone();
    if let NodeKind::Material(m) = &mut node.kind {
        m.clearcoat_amount = 0.3;
        m.openpbr.coat_color = [0.9, 0.8, 0.7];
        m.luminous_intensity = 2.;
        m.luminous_color = [1., 0.5, 0.];
    }
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    let (back, losses, xml) = round_trip(&e, group);
    assert!(losses.is_empty(), "{losses:?}\n{xml}");
    assert_eq!(layers(&back.scene), want_layers, "{xml}");
    let m = material(&back.scene);
    assert!((m.clearcoat_amount - 0.3).abs() < 1e-6);
    assert_eq!(m.openpbr.coat_color, [0.9, 0.8, 0.7]);
    assert!((m.luminous_intensity - 2.).abs() < 1e-6);
    assert!((m.displacement_distance - 0.01).abs() < 1e-9);
}

#[test]
fn layer_opacity_repeat_and_scale_round_trip() {
    let folder = PathBuf::from("/content/Bricks");
    let files: Vec<PathBuf> = ["Wall_Color.png", "Wall_Roughness.png"]
        .iter()
        .map(|f| folder.join(f))
        .collect();
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let (preset, _) = texture_sets(&refs)[0]
        .material_preset(ResolutionChoice::Highest, &any_size)
        .unwrap();
    let (mut e, group) = editor_with(preset);
    // The colour layer at 60 % with brightness 0.5 and repeat 3 x 2.
    let colour = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(
            |n| matches!(&n.kind, NodeKind::Texture(t) if t.effect == TextureEffect::DiffuseColor),
        )
        .unwrap()
        .clone();
    let mut node = colour.clone();
    node.layer.opacity = 0.6;
    if let NodeKind::Texture(t) = &mut node.kind
        && let TextureSource::ImageMap { settings, .. } = &mut t.source
    {
        settings.brightness = 0.5;
    }
    let locator = match &colour.kind {
        NodeKind::Texture(t) => t.locator.unwrap(),
        _ => unreachable!(),
    };
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    if let Some(crate::texture_locator::TextureItem::Locator(l)) =
        &mut e.scene.item_mut(locator).unwrap().texture
    {
        l.repeat = [3., 2.];
    }
    let want = layers(&e.scene);
    let (back, losses, xml) = round_trip(&e, group);
    assert!(losses.is_empty(), "{losses:?}");
    assert_eq!(layers(&back.scene), want, "{xml}");
    let repeats: Vec<[f64; 2]> = back
        .scene
        .items
        .iter()
        .filter_map(|i| i.texture_locator().map(|l| l.repeat))
        .collect();
    assert!(
        repeats
            .iter()
            .any(|r| (r[0] - 3.).abs() < 1e-6 && (r[1] - 2.).abs() < 1e-6),
        "{repeats:?}\n{xml}"
    );
}

#[test]
fn exports_list_layers_they_cannot_write() {
    let (mut e, group) = editor_with(Preset {
        header: crate::preset::PresetHeader {
            kind: crate::preset::PresetKind::Material,
            thumbnail: None,
            source: None,
        },
        scene: Scene {
            shader_tree: crate::shader_tree::ShaderTree {
                node_views: Default::default(),
                nodes: vec![crate::shader_tree::Node {
                    id: 1,
                    parent: None,
                    name: "Plain".into(),
                    layer: Default::default(),
                    kind: NodeKind::Group(Default::default()),
                    extras: Default::default(),
                }],
                next_id: 2,
                outputs: Default::default(),
                node_graphs: Default::default(),
            },
            ..Scene::default()
        },
    });
    let g = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .rfind(|n| matches!(n.kind, NodeKind::Group(_)))
        .unwrap()
        .id;
    let _ = group;
    for layer in [NewLayer::Material, NewLayer::Constant] {
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer,
            selected: Some(g),
        }))
        .unwrap();
    }
    let (_, losses) = export::material_x(&e.scene, g, Path::new("/tmp")).unwrap();
    assert_eq!(losses.len(), 1, "the constant layer: {losses:?}");
    // The file itself is the surface's standard MaterialX form, which
    // keeps the constant layer, and reads back.
    let dir = std::env::temp_dir().join(format!("operandi-mx-std-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("plain.mtlx");
    let notes = super::export(&e.scene, g, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("<open_pbr_surface"), "{notes:?}\n{text}");
    // The constant layer folds into the surface's own values.
    assert!(!text.contains("<image"), "{text}");
    // Plain values read back as a Material layer.
    let (p, _) = from_text(&text, &dir).unwrap();
    assert!(
        p.scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Material(_)))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mx_t3_unsupported_nodes_refuse_the_whole_material() {
    // Not simple, but every node translates: kept whole as a MaterialX
    // graph (materials slice 1) instead of refused.
    let text = r#"<materialx version="1.39">
  <noise2d name="grain" type="float"><input name="amplitude" type="float" value="0.5"/></noise2d>
  <image name="col" type="color3"><input name="file" type="filename" value="c.png"/></image>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="col" />
    <input name="specular_roughness" type="float" nodename="grain" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#;
    let (p, notes) = from_text(text, Path::new("/x")).unwrap();
    let graphs = &p.scene.shader_tree.node_graphs.graphs;
    assert_eq!(graphs.len(), 1);
    let mx = graphs[0].materialx.as_ref().expect("a MaterialX graph");
    assert_eq!((mx.material.as_str(), mx.folder.as_str()), ("M", "/x"));
    assert!(mx.text.contains("noise2d") && mx.text.contains("c.png"));
    assert!(
        notes.iter().any(|n| n.contains("MaterialX graph")),
        "{notes:?}"
    );
    assert!(
        p.scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| matches!(&n.kind, NodeKind::NodeMaterial(m) if m.graph == Some(graphs[0].id)))
    );
    // A textured input Operandi has no layer for becomes a graph too.
    let coat = text.replace(
        "specular_roughness\" type=\"float\" nodename=\"grain\"",
        "coat_color\" type=\"color3\" nodename=\"col\"",
    );
    let (p, _) = from_text(&coat, Path::new("/x")).unwrap();
    assert!(
        p.scene.shader_tree.node_graphs.graphs[0]
            .materialx
            .is_some()
    );
    // A node with no translation still refuses the whole material.
    let unknown = text.replace("noise2d", "hextiledimage");
    let err = from_text(&unknown, Path::new("/x")).unwrap_err();
    assert!(
        err.starts_with(NEEDS_NODES) && err.contains("grain"),
        "{err}"
    );
    // Nothing reaches the scene: the import fails before any command.
    let e = Editor::default();
    let before = e.scene.clone();
    assert!(from_text(&unknown, Path::new("/x")).is_err());
    assert_eq!(e.scene, before);
}

#[test]
fn a_graph_material_loads_undoes_and_survives_save_and_reopen() {
    let text = r#"<materialx version="1.39">
  <noise3d name="grain" type="float" />
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" value="0.2, 0.4, 0.6" />
    <input name="specular_roughness" type="float" nodename="grain" />
  </open_pbr_surface>
  <surfacematerial name="Grain" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#;
    let (p, _) = from_text(text, Path::new("/x")).unwrap();
    let mut e = Editor::default();
    let before = e.scene.clone();
    e.dispatch(Command::InsertMaterialPreset {
        preset: Box::new(p),
        beside: None,
    })
    .unwrap();
    let graph = e
        .scene
        .shader_tree
        .node_graphs
        .graphs
        .last()
        .unwrap()
        .clone();
    assert!(graph.materialx.is_some() && graph.unsupported().is_empty());
    // It renders: a Principled BSDF fed by a noise.
    let flat = graph.flatten(&Default::default()).unwrap();
    assert!(flat.nodes.iter().any(|n| n.cycles == "noise_texture"));
    // The viewport shows its base colour (NM-40).
    let layer = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::NodeMaterial(_)))
        .unwrap()
        .id;
    let use_ = crate::shader_eval::NodeUse {
        layer,
        graph: graph.id,
        coverage: crate::shader_eval::Expr::scalar(1.),
        values: Default::default(),
        drivers: Vec::new(),
    };
    let c = e.scene.shader_tree.node_viewport_colour(&use_);
    assert!((c[1] - 0.4).abs() < 1e-9, "{c:?}");
    // Saved and reopened: the same MaterialX text, at native version 54.
    let bytes = crate::persistence::encode(&e.scene).unwrap();
    let saved = String::from_utf8(bytes.clone()).unwrap();
    assert!(saved.contains("\"version\": 54"), "{}", &saved[..200]);
    let back = crate::persistence::decode(&bytes).unwrap();
    assert_eq!(
        back.shader_tree.node_graphs,
        e.scene.shader_tree.node_graphs
    );
    // One undo step takes it all away; redo brings it back.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.shader_tree, before.shader_tree);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.shader_tree.node_graphs.graphs.last(), Some(&graph));
}

#[test]
fn mx_t4_standard_surface_and_gltf_pbr_import() {
    let std = r#"<materialx version="1.39">
  <image name="rough" type="float"><input name="file" type="filename" value="r.png"/></image>
  <standard_surface name="s" type="surfaceshader">
    <input name="base" type="float" value="0.5" />
    <input name="base_color" type="color3" value="0.8, 0.2, 0.1" />
    <input name="metalness" type="float" value="0.5" />
    <input name="specular_roughness" type="float" nodename="rough" />
    <input name="coat" type="float" value="0.5" />
    <input name="thin_film_thickness" type="float" value="400" />
  </standard_surface>
  <surfacematerial name="Std" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#;
    let (p, notes) = from_text(std, Path::new("/x")).unwrap();
    let m = material(&p.scene);
    assert_eq!(m.metallic, 0.5);
    // The published graph moves a metal's coat into its base colour.
    assert_eq!(m.clearcoat_amount, 0.);
    // base 0.5 folds into the colour (base_weight).
    assert!(
        (m.diffuse_color[0] - 0.4).abs() < 1e-9,
        "{:?}",
        m.diffuse_color
    );
    // Thin film: 400 nm is 0.4 um.
    assert!(m.openpbr.thin_film_weight == 1. && (m.openpbr.thin_film_thickness - 0.4).abs() < 1e-9);
    assert_eq!(layers(&p.scene)[0].0, TextureEffect::Roughness);
    assert!(notes.is_empty(), "{notes:?}");

    let gltf = r#"<materialx version="1.39">
  <image name="col" type="color3"><input name="file" type="filename" value="c.png" colorspace="srgb_texture"/></image>
  <image name="ao" type="float"><input name="file" type="filename" value="ao.png"/></image>
  <gltf_pbr name="g" type="surfaceshader">
    <input name="base_color" type="color3" nodename="col" />
    <input name="metallic" type="float" value="0" />
    <input name="roughness" type="float" value="0.35" />
    <input name="occlusion" type="float" nodename="ao" />
    <input name="emissive" type="color3" value="1, 0.5, 0" />
    <input name="emissive_strength" type="float" value="4" />
    <input name="transmission" type="float" value="0.5" />
    <input name="ior" type="float" value="1.45" />
  </gltf_pbr>
  <surfacematerial name="G" type="material"><input name="surfaceshader" type="surfaceshader" nodename="g"/></surfacematerial>
</materialx>"#;
    let (p, _) = from_text(gltf, Path::new("/x")).unwrap();
    let m = material(&p.scene);
    assert!((m.roughness - 0.35).abs() < 1e-9 && m.metallic == 0.);
    assert_eq!(m.luminous_color, [1., 0.5, 0.]);
    assert!((m.luminous_intensity - 4.).abs() < 1e-9);
    assert!((m.transparent_amount - 0.5).abs() < 1e-9 && m.refractive_index == 1.45);
    let l = layers(&p.scene);
    assert_eq!(
        (l[0].0, l[0].2),
        (TextureEffect::DiffuseColor, BlendMode::Multiply),
        "AO on top"
    );
    assert_eq!(l[1].0, TextureEffect::DiffuseColor);

    let usd = r#"<materialx version="1.39">
  <UsdUVTexture name="tex" type="multioutput"><input name="file" type="filename" value="n.png"/>
    <input name="scale" type="color4" value="2, 2, 2, 2"/><input name="bias" type="color4" value="-1, -1, -1, -1"/></UsdUVTexture>
  <UsdUVTexture name="rough" type="multioutput"><input name="file" type="filename" value="orm.png"/></UsdUVTexture>
  <UsdPreviewSurface name="u" type="surfaceshader">
    <input name="diffuseColor" type="color3" value="0.5, 0.5, 0.5" />
    <input name="roughness" type="float" nodename="rough" output="g" />
    <input name="normal" type="vector3" nodename="tex" output="rgb" />
  </UsdPreviewSurface>
  <surfacematerial name="U" type="material"><input name="surfaceshader" type="surfaceshader" nodename="u"/></surfacematerial>
</materialx>"#;
    let (p, _) = from_text(usd, Path::new("/x")).unwrap();
    let l = layers(&p.scene);
    assert!(
        l.iter().any(|x| x.0 == TextureEffect::Roughness
            && x.4.swizzle == crate::shader_tree::Swizzle::Green)
    );
    assert!(l.iter().any(|x| x.0 == TextureEffect::Normal));
    assert_eq!(material(&p.scene).diffuse_color, [0.5; 3]);
}

#[test]
fn colour_spaces_and_units_convert() {
    let text = r#"<materialx version="1.39" colorspace="acescg">
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" value="1, 1, 1" />
    <input name="emission_color" type="color3" value="1, 0, 0" />
    <input name="emission_luminance" type="float" value="2" />
    <input name="transmission_weight" type="float" value="1" />
    <input name="transmission_depth" type="float" value="5" />
    <input name="subsurface_radius" type="float" value="2" unit="millimeter" unittype="distance" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#;
    let (p, notes) = from_text(text, Path::new("/x")).unwrap();
    let m = material(&p.scene);
    assert!(m.diffuse_color.iter().all(|v| (v - 1.).abs() < 1e-3));
    // ACEScg red is out of Rec.709: clamped, with a note.
    assert!(m.luminous_color[0] == 1. && m.luminous_color[1] == 0.);
    assert!(notes.iter().any(|n| n.contains("clamped")), "{notes:?}");
    // 5 (cm, the default unit) and 2 mm.
    assert!((m.absorption_distance - 0.05).abs() < 1e-12);
    assert!((m.subsurface_distance - 0.002).abs() < 1e-12);
}

#[test]
fn openpbr_constants_round_trip_through_a_file() {
    let mut m = MaterialChannels {
        diffuse_color: [0.3, 0.4, 0.5],
        metallic: 0.25,
        roughness: 0.6,
        clearcoat_amount: 0.4,
        ..Default::default()
    };
    m.openpbr.coat_ior = 1.6;
    m.openpbr.thin_film_weight = 1.;
    m.openpbr.thin_film_thickness = 0.25;
    m.subsurface_amount = 0.3;
    m.subsurface_distance = 0.02;
    m.openpbr.subsurface_uses_diffuse = false;
    m.openpbr.subsurface_albedo = [0.9, 0.7, 0.6];
    m.diffuse_amount = 1.;
    m.refraction_roughness = m.roughness;
    m.bump_amplitude = 0.;
    m.displacement_distance = 0.;
    let (want, _) = crate::openpbr::to_material(&crate::openpbr::from_material(&m).0);
    let s = Scene {
        shader_tree: crate::shader_tree::ShaderTree {
            node_views: Default::default(),
            nodes: vec![
                crate::shader_tree::Node {
                    id: 1,
                    parent: None,
                    name: "Const Test".into(),
                    layer: Default::default(),
                    kind: NodeKind::Group(Default::default()),
                    extras: Default::default(),
                },
                crate::shader_tree::Node {
                    id: 2,
                    parent: Some(1),
                    name: "M".into(),
                    layer: Default::default(),
                    kind: NodeKind::Material(Box::new(m.clone())),
                    extras: Default::default(),
                },
            ],
            next_id: 3,
            outputs: Default::default(),
            node_graphs: Default::default(),
        },
        ..Scene::default()
    };
    let (xml, losses) = export::material_x(&s, 1, Path::new("/x")).unwrap();
    assert!(losses.is_empty(), "{losses:?}");
    assert!(xml.contains("name=\"Const_Test\""), "{xml}");
    let (back, _) = from_text(&xml, Path::new("/x")).unwrap();
    let mut got = material(&back.scene);
    got.bump_amplitude = want.bump_amplitude;
    got.displacement_distance = want.displacement_distance;
    let close = |a: f64, b: f64| (a - b).abs() < 1e-5;
    assert!(close(got.metallic, want.metallic) && close(got.roughness, want.roughness));
    assert!(close(got.openpbr.thin_film_thickness, 0.25) && close(got.openpbr.coat_ior, 1.6));
    assert!(close(got.subsurface_distance, 0.02));
    assert!((0..3).all(|i| close(
        got.openpbr.subsurface_albedo[i],
        want.openpbr.subsurface_albedo[i]
    )));
    let _ = Param::BaseColor;
}

/// The Test-Library's ambientCG .mtlx files (when that folder exists):
/// each imports, with its set's AO added (MX-Q03), the layers its own
/// texture set gives, except the normal map the .mtlx picks.
#[test]
#[ignore = "reads ~/Developer/Operandi-Content/Test-Library"]
fn mx_t1_every_ambientcg_file_in_the_test_library() {
    let root = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap()
        .join("Developer/Operandi-Content/Test-Library");
    let mut checked = 0;
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut files = Vec::new();
        for e in read.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                files.push(p);
            }
        }
        for mtlx in files
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "mtlx"))
        {
            let size = |p: &Path| p.exists().then_some((8, 8));
            let (mut preset, notes) = material_preset(mtlx, None, &size)
                .unwrap_or_else(|e| panic!("{}: {e}", mtlx.display()));
            let all: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
            if let Some(set) = texture_sets(&all).first() {
                set.add_ambient_occlusion(&mut preset, ResolutionChoice::Highest, &size)
                    .unwrap();
            }
            let got = layers(&preset.scene);
            assert!(!got.is_empty(), "{}: {notes:?}", mtlx.display());
            let pics: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
            if let Some(set) = texture_sets(&pics).first() {
                let (sp, _) = set
                    .material_preset(ResolutionChoice::Highest, &size)
                    .unwrap();
                let mut want: Vec<_> = layers(&sp.scene)
                    .into_iter()
                    .filter(|l| l.0 != TextureEffect::Normal)
                    .map(|l| l.0)
                    .collect();
                let mut have: Vec<_> = got
                    .iter()
                    .filter(|l| l.0 != TextureEffect::Normal)
                    .map(|l| l.0)
                    .collect();
                want.dedup();
                have.dedup();
                assert_eq!(have, want, "{}", mtlx.display());
            }
            checked += 1;
        }
    }
    assert!(checked > 0);
    println!("{checked} ambientCG files");
}

/// OPM-T2's import half: every OpenPBR example (set OPENPBR_EXAMPLES to
/// a checkout's examples/ folder) imports; refusals are listed.
#[test]
#[ignore = "reads OPENPBR_EXAMPLES"]
fn opm_t2_openpbr_examples_import() {
    let dir = PathBuf::from(std::env::var("OPENPBR_EXAMPLES").expect("OPENPBR_EXAMPLES"));
    let mut ok = 0;
    let mut notes_all = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "mtlx") {
            continue;
        }
        match material_preset(&p, None, &any_size) {
            Ok((_, notes)) => {
                ok += 1;
                for n in notes {
                    notes_all.push(format!("{}: {n}", p.file_name().unwrap().to_string_lossy()));
                }
            }
            Err(err) => notes_all.push(format!("REFUSED {}: {err}", p.display())),
        }
    }
    println!("{ok} imported\n{}", notes_all.join("\n"));
    assert!(ok > 0);
}
