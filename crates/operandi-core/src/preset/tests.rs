//! Stage 0 acceptance (PRESET-BROWSER.md PB-T1..T4) and the file rules.
use super::*;
use crate::camera_light::LightKind;
use crate::shader_tree::{NewLayer, TextureSource};
use crate::{Command, Mesh, ShaderTreeEdit, Transform, persistence};
use glam::DVec3;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-preset-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add(e: &mut Editor, layer: NewLayer, selected: Option<NodeId>) -> NodeId {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add { layer, selected }))
        .unwrap();
    e.scene.shader_tree.next_id - 1
}

/// A Material Group holding a Material and two texture layers, with
/// settings off their defaults.
fn group_with_two_layers(e: &mut Editor) -> NodeId {
    let group = add(e, NewLayer::Group, None);
    add(e, NewLayer::Material, Some(group));
    for (k, value) in [(0, 0.25), (1, 0.75)] {
        let id = add(e, NewLayer::Constant, Some(group));
        let mut node = e.scene.shader_tree.get(id).unwrap().clone();
        node.layer.opacity = 0.3 + k as f64 * 0.2;
        if let NodeKind::Texture(t) = &mut node.kind {
            t.source = TextureSource::Constant {
                color: [value, 0.5, 1. - value],
                value,
            };
        }
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
    }
    group
}

/// The layers inside `group`, without ids, parents or locator ids, plus
/// each texture's locator settings: what "settings identical" compares.
fn contents(scene: &Scene, group: NodeId) -> Vec<(String, String, String)> {
    let tree = &scene.shader_tree;
    tree.nodes
        .iter()
        .filter(|n| n.id != group && tree.is_within(n.id, group))
        .map(|n| {
            let mut kind = n.kind.clone();
            let mut locator = String::new();
            if let NodeKind::Texture(t) = &mut kind
                && let Some(l) = t.locator.take()
            {
                locator = format!("{:?}", scene.texture_locator(l).expect("locator exists"));
            }
            (format!("{:?}", n.layer), format!("{kind:?}"), locator)
        })
        .collect()
}

fn load(e: &mut Editor, path: &std::path::Path, group: Option<NodeId>) -> Result<()> {
    let preset = persistence::load_preset(path)?;
    e.dispatch(Command::LoadPreset {
        preset: Box::new(preset),
        group,
    })
}

fn mesh_editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e
}

fn add_light(e: &mut Editor, kind: LightKind, at: DVec3) -> u64 {
    e.scene
        .add_light(
            crate::camera_light::LightSettings::new(kind),
            Transform {
                translation: at,
                ..Default::default()
            },
        )
        .unwrap()
}

#[test]
fn pb_t1_material_group_with_two_layers_round_trips() {
    let dir = scratch("t1");
    let path = dir.join("Two Layers.oppreset");
    let mut source = Editor::default();
    let group = group_with_two_layers(&mut source);
    let before = source.scene.clone();
    let notes = source
        .save_preset(&path, PresetSource::Material(group))
        .unwrap();
    assert!(notes.is_empty());
    // Saving changes nothing (PB-35).
    assert_eq!(source.scene, before);
    assert_eq!(
        persistence::load_preset(&path).unwrap().header.kind,
        PresetKind::Material
    );

    let mut target = Editor::default();
    let into = add(&mut target, NewLayer::Group, None);
    // Something already inside the group is replaced.
    add(&mut target, NewLayer::Constant, Some(into));
    let undo_before = target.undo.len();
    let scene_before = target.scene.clone();
    load(&mut target, &path, Some(into)).unwrap();
    assert_eq!(target.undo.len(), undo_before + 1, "one undo step");
    assert_eq!(
        contents(&target.scene, into),
        contents(&source.scene, group)
    );
    let textures = target
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
        .count();
    assert_eq!(textures, 2);
    // The replaced layer's locator went with it; the two new ones exist.
    let locators = target
        .scene
        .items
        .iter()
        .filter(|i| i.texture_locator().is_some())
        .count();
    assert_eq!(locators, 2);
    target.dispatch(Command::Undo).unwrap();
    assert_eq!(target.scene, scene_before);
}

#[test]
fn pb_q3_material_load_without_a_group_changes_nothing() {
    let dir = scratch("q3");
    let path = dir.join("m.oppreset");
    let mut source = Editor::default();
    let group = group_with_two_layers(&mut source);
    source
        .save_preset(&path, PresetSource::Material(group))
        .unwrap();
    let mut target = Editor::default();
    let before = target.scene.clone();
    let error = load(&mut target, &path, None).unwrap_err();
    assert!(error.contains("material group"), "{error}");
    // The Base Material is not a material group either.
    assert!(load(&mut target, &path, Some(4)).is_err());
    assert_eq!(target.scene, before);
    assert!(target.undo.is_empty());
}

#[test]
fn a_single_material_loads_into_the_group() {
    let dir = scratch("material");
    let path = dir.join("base.oppreset");
    let mut source = Editor::default();
    // The default tree's Base Material (id 4), made shiny.
    let mut node = source.scene.shader_tree.get(4).unwrap().clone();
    if let NodeKind::Material(m) = &mut node.kind {
        m.specular_amount = 0.9;
    }
    source
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
    source
        .save_preset(&path, PresetSource::Material(4))
        .unwrap();
    let mut target = Editor::default();
    let into = add(&mut target, NewLayer::Group, None);
    load(&mut target, &path, Some(into)).unwrap();
    let inside: Vec<&Node> = target.scene.shader_tree.children(Some(into)).collect();
    assert_eq!(inside.len(), 1);
    assert!(matches!(&inside[0].kind, NodeKind::Material(m) if m.specular_amount == 0.9));
    // Texture layers can't be saved as material presets.
    let texture = add(&mut source, NewLayer::Constant, None);
    assert!(
        source
            .save_preset(&dir.join("t.oppreset"), PresetSource::Material(texture))
            .is_err()
    );
}

#[test]
fn pb_t2_mesh_preset_drops_the_item_transform() {
    let dir = scratch("t2");
    let path = dir.join("cube.oppreset");
    let mut source = mesh_editor();
    source.scene.active_mut().unwrap().transform.translation.x = 5.;
    let built = PresetSource::Items.build(&source.scene).unwrap();
    // PB-44: the save says so.
    assert_eq!(built.notes.len(), 1);
    assert!(built.notes[0].contains("origin"));
    source.save_preset(&path, PresetSource::Items).unwrap();

    let mut target = Editor::default();
    load(&mut target, &path, None).unwrap();
    assert_eq!(target.undo.len(), 1);
    let item = target.scene.active().expect("the loaded mesh is selected");
    assert_eq!(item.transform, Transform::default());
    let mesh = &source.scene.items[0].mesh;
    assert_eq!(item.mesh.vertices, mesh.vertices);
    assert_eq!(item.mesh.faces.len(), mesh.faces.len());
    // Loading again adds another copy with its own name.
    load(&mut target, &path, None).unwrap();
    assert_eq!(target.scene.items.len(), 2);
    assert_ne!(target.scene.items[0].name, target.scene.items[1].name);
}

#[test]
fn mesh_preset_brings_its_material_group_when_the_scene_lacks_one() {
    let dir = scratch("tagged");
    let path = dir.join("red cube.oppreset");
    let mut source = mesh_editor();
    let red = add(&mut source, NewLayer::Group, None);
    let mut node = source.scene.shader_tree.get(red).unwrap().clone();
    if let NodeKind::Group(g) = &mut node.kind {
        g.tag = TagMask::Name("Red".into());
        g.item = ItemMask::Item(source.scene.items[0].id);
    }
    source
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
    add(&mut source, NewLayer::Material, Some(red));
    let item = source.scene.active_mut().unwrap();
    let mut mesh = (*item.mesh).clone();
    for f in &mut mesh.faces {
        f.material = "Red".into();
    }
    item.mesh = mesh.into();
    source.save_preset(&path, PresetSource::Items).unwrap();
    let saved = persistence::load_preset(&path).unwrap();
    // The item mask named a source item: dropped (PB-Q7).
    assert!(
        saved
            .scene
            .shader_tree
            .nodes
            .iter()
            .all(|n| !matches!(&n.kind, NodeKind::Group(g) if g.item != ItemMask::All))
    );

    let mut target = Editor::default();
    load(&mut target, &path, None).unwrap();
    assert!(target.scene.shader_tree.material_for_tag("Red").is_some());
    let groups = |s: &Scene| {
        s.shader_tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Group(_)))
            .count()
    };
    assert_eq!(groups(&target.scene), 1);
    // A second load finds the group there and adds no other.
    load(&mut target, &path, None).unwrap();
    assert_eq!(groups(&target.scene), 1);
}

#[test]
fn assembly_keeps_the_hierarchy() {
    let dir = scratch("assembly");
    let mut source = mesh_editor();
    let parent = source.scene.items[0].id;
    source.scene.items[0].transform.translation.y = 2.;
    let light = add_light(&mut source, LightKind::Point, DVec3::X);
    source.scene.item_mut(light).unwrap().parent = Some(parent);
    source.scene.select_item(Some(parent));
    let path = dir.join("rig.oppreset");
    source.save_preset(&path, PresetSource::Assembly).unwrap();
    let mut target = Editor::default();
    load(&mut target, &path, None).unwrap();
    assert_eq!(target.scene.items.len(), 2);
    let root = target.scene.active().unwrap();
    assert_eq!(
        root.transform.translation.y, 2.,
        "assemblies keep transforms"
    );
    let root = root.id;
    let child = target
        .scene
        .items
        .iter()
        .find(|i| i.light.is_some())
        .unwrap();
    assert_eq!(child.parent, Some(root));
    // One undo removes the whole assembly.
    target.dispatch(Command::Undo).unwrap();
    assert!(target.scene.items.is_empty());
    // Nothing selected: nothing to save.
    let mut empty = mesh_editor();
    empty.scene.select_item(None);
    assert!(
        empty
            .save_preset(&dir.join("x.oppreset"), PresetSource::Items)
            .is_err()
    );
}

#[test]
/// PB-T3 as the environment preset spec now has it (EP §Apply, Replace):
/// the preset's lights come in; a hand-made light stays.
fn pb_t3_environment_replaces_every_light_in_one_step() {
    let dir = scratch("t3");
    let path = dir.join("studio.oppreset");
    let mut studio = mesh_editor();
    add_light(&mut studio, LightKind::Area, DVec3::new(3., 4., 0.));
    add_light(&mut studio, LightKind::Spot, DVec3::new(-3., 4., 0.));
    studio
        .save_preset(
            &path,
            PresetSource::Environment {
                lights: true,
                meshes: false,
                cameras: false,
            },
        )
        .unwrap();
    let saved = persistence::load_preset(&path).unwrap();
    assert_eq!(saved.header.kind, PresetKind::Environment);
    assert_eq!(saved.scene.items.len(), 2, "lights only");

    let mut other = mesh_editor();
    add_light(&mut other, LightKind::Directional, DVec3::ZERO);
    let before = other.scene.clone();
    let steps = other.undo.len();
    load(&mut other, &path, None).unwrap();
    let kinds: Vec<LightKind> = other
        .scene
        .items
        .iter()
        .filter_map(|i| i.light.as_ref().map(|l| l.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![LightKind::Directional, LightKind::Area, LightKind::Spot]
    );
    let area = other
        .scene
        .items
        .iter()
        .find(|i| i.light.as_ref().is_some_and(|l| l.kind == LightKind::Area))
        .unwrap();
    assert_eq!(area.transform.translation, DVec3::new(3., 4., 0.));
    assert!(
        other.scene.items.iter().any(|i| i.light.is_none()),
        "meshes stay"
    );
    assert_eq!(other.undo.len(), steps + 1);
    other.dispatch(Command::Undo).unwrap();
    assert_eq!(other.scene, before);
    // No lights, no environment preset.
    assert!(
        mesh_editor()
            .save_preset(
                &dir.join("none.oppreset"),
                PresetSource::Environment {
                    lights: true,
                    meshes: false,
                    cameras: false,
                }
            )
            .is_err()
    );
}

#[test]
fn render_settings_replace_outputs_and_add_the_camera() {
    let dir = scratch("render");
    let path = dir.join("render.oppreset");
    let mut source = Editor::default();
    let mut alpha = source.scene.shader_tree.get(1).unwrap().clone();
    alpha.layer.opacity = 0.5;
    source
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node: alpha,
            continuing: false,
        }))
        .unwrap();
    source.dispatch(Command::NewCamera).unwrap();
    source.scene.render.samples = 128;
    // Final Color Output (id 2) with its own settings.
    source.scene.shader_tree.outputs.insert(
        2,
        crate::render_outputs::OutputSettings {
            gamma: 1.5,
            ..Default::default()
        },
    );
    source
        .save_preset(&path, PresetSource::RenderSettings)
        .unwrap();
    let mut target = Editor::default();
    target.dispatch(Command::NewCamera).unwrap();
    let steps = target.undo.len();
    load(&mut target, &path, None).unwrap();
    let outputs: Vec<&Node> = target
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::RenderOutput(_)))
        .collect();
    assert_eq!(target.scene.render.samples, 128);
    let gammas: Vec<f64> = target
        .scene
        .shader_tree
        .outputs
        .values()
        .map(|o| o.gamma)
        .collect();
    assert_eq!(gammas, vec![1.5], "output settings follow their layer");
    assert_eq!(outputs.len(), 2, "replaced, not added");
    assert!(outputs.iter().any(|n| n.layer.opacity == 0.5));
    let cameras = target
        .scene
        .items
        .iter()
        .filter(|i| i.camera.is_some())
        .count();
    assert_eq!(cameras, 2, "the preset's camera is added");
    assert_eq!(target.undo.len(), steps + 1);
}

#[test]
fn pb_t4_a_copied_preset_loads_and_files_stay_apart() {
    let dir = scratch("t4");
    let path = dir.join("cube.oppreset");
    let source = mesh_editor();
    source.save_preset(&path, PresetSource::Items).unwrap();
    // Copied somewhere else (another machine): it carries everything.
    let elsewhere = scratch("t4-copy").join("renamed cube.OPPRESET");
    std::fs::copy(&path, &elsewhere).unwrap();
    let preset = persistence::load_preset(&elsewhere).unwrap();
    // PB-52: the thumbnail slot exists; Stage 0 leaves it empty.
    assert_eq!(preset.header.thumbnail, None);
    let mut target = Editor::default();
    load(&mut target, &elsewhere, None).unwrap();
    assert_eq!(target.scene.items.len(), 1);

    // A preset isn't a scene: plain Open refuses it cleanly.
    let error = persistence::load(&path).unwrap_err();
    assert!(error.contains("Load Preset"), "{error}");
    // A scene isn't a preset: Load Preset refuses it.
    let scene = dir.join("scene.opscene");
    persistence::save(&scene, &source.scene).unwrap();
    let error = persistence::load_preset(&scene).unwrap_err();
    assert!(error.contains("not a preset"), "{error}");
    // Scene files are as before: no preset header is written.
    let text = std::fs::read_to_string(&scene).unwrap();
    assert!(!text.contains("\"preset\""));
    assert_eq!(persistence::load(&scene).unwrap(), source.scene);
    // A preset needs its own extension.
    assert!(
        source
            .save_preset(&dir.join("cube.opscene"), PresetSource::Items)
            .is_err()
    );
}

#[test]
fn a_refused_load_leaves_the_scene_and_history_alone() {
    let dir = scratch("refuse");
    let path = dir.join("env.oppreset");
    let mut source = mesh_editor();
    add_light(&mut source, LightKind::Point, DVec3::ZERO);
    source
        .save_preset(
            &path,
            PresetSource::Environment {
                lights: true,
                meshes: false,
                cameras: false,
            },
        )
        .unwrap();
    // A live tool must be dropped first.
    let mut target = mesh_editor();
    target
        .dispatch(Command::Activate(crate::Tool::Move))
        .unwrap();
    let before = target.scene.clone();
    let undo = target.undo.len();
    assert!(load(&mut target, &path, None).is_err());
    assert_eq!(target.scene, before);
    assert_eq!(target.undo.len(), undo);
    // A broken file is an error, not a panic.
    std::fs::write(dir.join("bad.oppreset"), b"{nope").unwrap();
    assert!(persistence::load_preset(&dir.join("bad.oppreset")).is_err());
}

#[test]
fn kind_names_round_trip() {
    for kind in PresetKind::ALL {
        assert_eq!(PresetKind::from_script_name(kind.script_name()), Some(kind));
    }
}

// ---------------------------------------------------------------- Stage 1

#[test]
fn the_browser_reads_a_header_without_the_scene_and_keeps_thumbnails() {
    let dir = scratch("header");
    let mut e = Editor::default();
    let group = group_with_two_layers(&mut e);
    let path = dir.join("Rust.oppreset");
    e.save_preset(&path, PresetSource::Material(group)).unwrap();
    let header = persistence::read_preset_header(&path).unwrap();
    assert_eq!(header.kind, PresetKind::Material);
    assert_eq!(header.thumbnail, None);
    // A thumbnail goes into the file and comes back; the preset still loads.
    persistence::set_preset_thumbnail(&path, Some(b"\x89PNG fake")).unwrap();
    let header = persistence::read_preset_header(&path).unwrap();
    assert_eq!(
        persistence::preset_thumbnail(&header).as_deref(),
        Some(&b"\x89PNG fake"[..])
    );
    assert_eq!(persistence::load_preset(&path).unwrap().header, header);
    // A scene file is not a preset.
    let scene = dir.join("scene.opscene");
    persistence::save(&scene, &e.scene).unwrap();
    assert!(persistence::read_preset_header(&scene).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cd_t3_an_hdr_becomes_the_environment_image_in_one_step() {
    use crate::shader_tree::TextureSource;
    let mut e = Editor::default();
    e.scene.ensure_environment().unwrap();
    let before = e.scene.clone();
    let undo = e.undo.len();
    e.dispatch(Command::SetEnvironmentImage {
        path: "/hdri/studio_4k.exr".into(),
        width: 64,
        height: 32,
        item: None,
    })
    .unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    let image_of = |scene: &Scene| {
        let env = scene
            .environments()
            .next()
            .unwrap()
            .environment
            .as_ref()
            .unwrap();
        env.layers.iter().find_map(|n| match &n.kind {
            NodeKind::Texture(t) => match &t.source {
                TextureSource::ImageMap { image, .. } => *image,
                _ => None,
            },
            _ => None,
        })
    };
    let first = image_of(&e.scene).unwrap();
    assert!(
        e.scene
            .images
            .iter()
            .any(|i| i.id == first && i.path.ends_with("studio_4k.exr"))
    );
    // A second picture replaces the first rather than stacking.
    e.dispatch(Command::SetEnvironmentImage {
        path: "/hdri/sunset.hdr".into(),
        width: 64,
        height: 32,
        item: None,
    })
    .unwrap();
    let env = e
        .scene
        .environments()
        .next()
        .unwrap()
        .environment
        .clone()
        .unwrap();
    assert_eq!(env.layers.len(), 1);
    assert_ne!(image_of(&e.scene), Some(first));
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

/// The environment the tests below give a picture: the top one.
fn top_env(scene: &Scene) -> (u64, crate::environment::Environment) {
    let item = scene.environments().next().unwrap();
    (item.id, (**item.environment.as_ref().unwrap()).clone())
}

fn hdr(path: &str) -> Command {
    Command::SetEnvironmentImage {
        path: path.into(),
        width: 64,
        height: 32,
        item: None,
    }
}

#[test]
fn ep_t1_an_hdr_goes_on_the_top_environment_above_its_material() {
    use crate::environment::{EnvironmentEdit, RayKind};
    use crate::shader_tree::{TextureEffect, TextureSource};
    let mut e = Editor::default();
    e.scene.ensure_environment().unwrap();
    let older = top_env(&e.scene).0;
    let newer = e.scene.add_environment().unwrap();
    // The newest environment is hidden from the camera, so the camera sees
    // the older one; the picture still goes on the newest.
    e.dispatch(Command::EditEnvironment {
        item: newer,
        edit: EnvironmentEdit::Visible(RayKind::Camera, false),
    })
    .unwrap();
    assert_eq!(e.scene.environment_for(RayKind::Camera).unwrap().id, older);
    // A Constant layer already there: the image goes below it, directly
    // above the Environment Material.
    e.dispatch(Command::EditEnvironmentLayers {
        item: newer,
        edit: crate::environment::EnvironmentLayerEdit::Add {
            layer: NewLayer::Constant,
            at: crate::environment::LayerPlace::Top,
        },
    })
    .unwrap();
    let undo = e.undo.len();
    e.dispatch(hdr("/hdri/studio.hdr")).unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    let (id, env) = top_env(&e.scene);
    assert_eq!(id, newer);
    assert_eq!(env.layers.len(), 2);
    let image = env.layers.last().unwrap();
    assert_eq!(image.name, "studio");
    assert_eq!(image.layer.opacity, 1.0);
    let NodeKind::Texture(t) = &image.kind else {
        panic!("not a texture layer")
    };
    assert_eq!(t.effect, TextureEffect::EnvironmentColor);
    assert!(matches!(
        t.source,
        TextureSource::ImageMap { image: Some(_), .. }
    ));
    let locator = e
        .scene
        .items
        .iter()
        .find(|i| Some(i.id) == t.locator)
        .unwrap();
    let Some(crate::texture_locator::TextureItem::Locator(placement)) = locator.texture.as_ref()
    else {
        panic!("not a Texture Locator")
    };
    assert_eq!(
        placement.projection,
        crate::shader_tree::Projection::Spherical
    );
    assert_eq!(placement.axis, 1);
    assert!(e.scene.environment_image_layer(older).is_none());
}

#[test]
fn ep_t2_a_second_hdr_swaps_the_image_and_keeps_rotation_and_opacity() {
    let mut e = Editor::default();
    e.dispatch(hdr("/hdri/a.hdr")).unwrap();
    let (env_id, env) = top_env(&e.scene);
    let mut layer = env.layers[0].clone();
    layer.layer.opacity = 0.5;
    e.dispatch(Command::EditEnvironmentLayers {
        item: env_id,
        edit: crate::environment::EnvironmentLayerEdit::Replace(Box::new(layer)),
    })
    .unwrap();
    let (locator, _) = e.scene.environment_image_heading(env_id).unwrap();
    let mut t = e.scene.item(locator).unwrap().transform.clone();
    t.rotation.y = 30f64.to_radians();
    e.dispatch(Command::ItemTransform {
        item: locator,
        transform: t,
    })
    .unwrap();
    e.dispatch(hdr("/hdri/b.exr")).unwrap();
    let (_, env) = top_env(&e.scene);
    assert_eq!(env.layers.len(), 1);
    assert_eq!(env.layers[0].layer.opacity, 0.5);
    assert_eq!(env.layers[0].name, "b");
    assert!(
        e.scene
            .environment_image(env_id)
            .unwrap()
            .path
            .ends_with("b.exr")
    );
    let (same, heading) = e.scene.environment_image_heading(env_id).unwrap();
    assert_eq!(same, locator);
    assert!((heading - 30f64.to_radians()).abs() < 1e-12);
}

#[test]
fn a_named_environment_takes_the_image_and_a_wrong_item_is_refused() {
    let mut e = Editor::default();
    e.scene.ensure_environment().unwrap();
    let older = top_env(&e.scene).0;
    e.scene.add_environment().unwrap();
    e.dispatch(Command::SetEnvironmentImage {
        path: "/hdri/a.hdr".into(),
        width: 8,
        height: 4,
        item: Some(older),
    })
    .unwrap();
    assert!(e.scene.environment_image(older).is_some());
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::SetEnvironmentImage {
            path: "/hdri/a.hdr".into(),
            width: 8,
            height: 4,
            item: Some(u64::MAX - 7),
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
}

#[test]
fn the_view_draws_the_top_enabled_image_of_the_camera_environment() {
    use crate::environment::{EnvironmentEdit, RayKind};
    let mut e = Editor::default();
    assert!(e.scene.environment_sky(RayKind::Camera).is_none());
    e.dispatch(hdr("/hdri/a.hdr")).unwrap();
    let (env, _) = top_env(&e.scene);
    let sky = e.scene.environment_sky(RayKind::Camera).unwrap();
    assert!(sky.path.ends_with("a.hdr"));
    assert_eq!(sky.to_image, glam::DMat3::IDENTITY);
    // Rotation turns the frame; Intensity comes along.
    let (locator, _) = e.scene.environment_image_heading(env).unwrap();
    let mut t = e.scene.item(locator).unwrap().transform.clone();
    t.rotation.y = 1.0;
    e.dispatch(Command::ItemTransform {
        item: locator,
        transform: t,
    })
    .unwrap();
    e.dispatch(Command::EditEnvironment {
        item: env,
        edit: EnvironmentEdit::Intensity(2.0),
    })
    .unwrap();
    let sky = e.scene.environment_sky(RayKind::Camera).unwrap();
    assert!(
        (sky.to_image - glam::DMat3::from_rotation_y(-1.0)).abs_diff_eq(glam::DMat3::ZERO, 1e-9)
    );
    assert_eq!(sky.intensity, 2.0);
    // A disabled layer, or an environment hidden from the camera, draws none.
    let (_, environment) = top_env(&e.scene);
    let mut layer = environment.layers[0].clone();
    layer.layer.enable = false;
    e.dispatch(Command::EditEnvironmentLayers {
        item: env,
        edit: crate::environment::EnvironmentLayerEdit::Replace(Box::new(layer)),
    })
    .unwrap();
    assert!(e.scene.environment_sky(RayKind::Camera).is_none());
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::EditEnvironment {
        item: env,
        edit: EnvironmentEdit::Visible(RayKind::Camera, false),
    })
    .unwrap();
    assert!(e.scene.environment_sky(RayKind::Camera).is_none());
    assert!(e.scene.environment_sky(RayKind::Indirect).is_some());
}

#[test]
fn pb_t13_a_texture_set_loads_into_the_selected_group_in_one_step() {
    use super::library::{ResolutionChoice, texture_sets};
    let files = [
        PathBuf::from("/t/a_color.jpg"),
        PathBuf::from("/t/a_rough.jpg"),
    ];
    let refs: Vec<&std::path::Path> = files.iter().map(PathBuf::as_path).collect();
    let (preset, _) = texture_sets(&refs)[0]
        .material_preset(ResolutionChoice::Highest, &|_| Some((4, 4)))
        .unwrap();
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let before = e.scene.clone();
    // Nothing selected: refused, nothing changes.
    assert!(
        e.dispatch(Command::LoadPreset {
            preset: Box::new(preset.clone()),
            group: None
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
    e.dispatch(Command::LoadPreset {
        preset: Box::new(preset),
        group: Some(group),
    })
    .unwrap();
    let inside = e.scene.shader_tree.children(Some(group)).count();
    assert_eq!(inside, 3, "two image layers and the material");
    assert_eq!(e.scene.images.len(), 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn pb_110_a_material_dropped_on_a_tag_without_a_group_gets_one_in_one_step() {
    let mut source = Editor::default();
    let group = group_with_two_layers(&mut source);
    let preset = PresetSource::Material(group)
        .build(&source.scene)
        .unwrap()
        .preset;
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let item = e.scene.items[0].id;
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    mesh.faces
        .iter_mut()
        .for_each(|f| f.material = "Default".into());
    e.scene.items[0].mesh = mesh.into();
    let before = e.scene.clone();
    e.dispatch(Command::ApplyMaterialPreset {
        preset: Box::new(preset),
        item,
        faces: vec![0, 1],
    })
    .unwrap();
    // PB1-Q02 (2026-11-11): the polygons keep their tag; the tag gets a
    // group of its own, named after it.
    let mesh = &e.scene.items[0].mesh;
    let tag = mesh.faces[0].material.clone();
    assert_eq!(tag, "Default");
    assert!(mesh.faces.iter().all(|f| f.material == "Default"));
    let tree = e.scene.render_shader_tree();
    let g = tree
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, NodeKind::Group(m) if m.tag == TagMask::Name(tag.clone())))
        .unwrap();
    assert!(tree.children(Some(g.id)).count() >= 3);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

/// PB-110 (Jason, 2026-11-11): a second material dropped on the same
/// material group replaces the first instead of stacking a second tag and
/// group; one undo step brings the first back.
#[test]
fn pb_110_a_second_drop_on_a_material_group_replaces_the_first() {
    let mut source = Editor::default();
    let first = group_with_two_layers(&mut source);
    let a = PresetSource::Material(first)
        .build(&source.scene)
        .unwrap()
        .preset;
    let mut source = Editor::default();
    let plain = source.scene.shader_tree.add(NewLayer::Group, None).unwrap();
    source
        .scene
        .shader_tree
        .add(NewLayer::Material, Some(plain))
        .unwrap();
    let b = PresetSource::Material(plain)
        .build(&source.scene)
        .unwrap()
        .preset;
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let item = e.scene.items[0].id;
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    mesh.faces
        .iter_mut()
        .for_each(|f| f.material = "Default".into());
    e.scene.items[0].mesh = mesh.into();
    let drop = |e: &mut Editor, preset: &Preset| {
        e.dispatch(Command::ApplyMaterialPreset {
            preset: Box::new(preset.clone()),
            item,
            faces: vec![0, 1],
        })
        .unwrap();
    };
    drop(&mut e, &a);
    let tag = e.scene.items[0].mesh.faces[0].material.clone();
    let groups = |e: &Editor| {
        e.scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Group(_)))
            .map(|n| n.id)
            .collect::<Vec<_>>()
    };
    let after_a = groups(&e);
    let group = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, NodeKind::Group(m) if m.tag == TagMask::Name(tag.clone())))
        .unwrap()
        .id;
    assert_eq!(e.scene.shader_tree.children(Some(group)).count(), 3);
    let with_a = e.scene.clone();
    drop(&mut e, &b);
    assert_eq!(groups(&e), after_a, "no second group");
    assert_eq!(e.scene.items[0].mesh.faces[0].material, tag, "same tag");
    assert_eq!(e.scene.items[0].mesh.faces[2].material, "Default");
    let inside: Vec<_> = e.scene.shader_tree.children(Some(group)).collect();
    assert_eq!(inside.len(), 1, "only B's material");
    assert!(matches!(inside[0].kind, NodeKind::Material(_)));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, with_a, "one undo step brings A back");
}

/// PB-110 / PB1-Q02 (Jason, 2026-11-11): a material dropped on the table
/// top fills the TableTop group (its old material goes), makes no new
/// group, re-tags nothing, and leaves Table_Edge alone; one undo.
#[test]
fn pb_110_a_drop_fills_the_existing_group_of_the_tag() {
    let mut source = Editor::default();
    let layers = group_with_two_layers(&mut source);
    let preset = PresetSource::Material(layers)
        .build(&source.scene)
        .unwrap()
        .preset;
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let item = e.scene.items[0].id;
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    for (i, f) in mesh.faces.iter_mut().enumerate() {
        f.material = if i < 2 { "TableTop" } else { "Table_Edge" }.into();
    }
    e.scene.items[0].mesh = mesh.into();
    let mut group_for = |tag: &str| {
        let g = e.scene.shader_tree.add(NewLayer::Group, None).unwrap();
        let mut node = e.scene.shader_tree.get(g).unwrap().clone();
        node.name = tag.into();
        if let NodeKind::Group(m) = &mut node.kind {
            m.tag_type = crate::shader_tree::TagType::Material;
            m.tag = TagMask::Name(tag.into());
        }
        e.scene.shader_tree.replace(node).unwrap();
        e.scene
            .shader_tree
            .add(NewLayer::Material, Some(g))
            .unwrap();
        g
    };
    let top = group_for("TableTop");
    let edge = group_for("Table_Edge");
    e.undo.clear();
    let before = e.scene.clone();
    let groups = |e: &Editor| {
        e.scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Group(_)))
            .count()
    };
    let edge_before: Vec<_> = e.scene.shader_tree.children(Some(edge)).cloned().collect();
    e.dispatch(Command::ApplyMaterialPreset {
        preset: Box::new(preset),
        item,
        faces: vec![0, 1],
    })
    .unwrap();
    assert_eq!(
        groups(&e),
        groups(&Editor {
            scene: before.clone(),
            ..Editor::default()
        })
    );
    assert_eq!(
        e.scene.items[0].mesh.faces, before.items[0].mesh.faces,
        "no re-tagging"
    );
    assert_eq!(
        e.scene.shader_tree.children(Some(top)).count(),
        3,
        "the preset's layers only"
    );
    let edge_after: Vec<_> = e.scene.shader_tree.children(Some(edge)).cloned().collect();
    assert_eq!(edge_after, edge_before);
    assert_eq!(e.scene.shader_tree.get(top).unwrap().name, "TableTop");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

// ------------------------------------------------- Environment presets (EP)

fn env_preset(e: &Editor, path: &std::path::Path, lights: bool, copy: bool) -> Preset {
    e.save_environment_preset(
        path,
        crate::preset::EnvironmentPresetOptions {
            lights,
            meshes: false,
            cameras: false,
            copy_images: copy,
        },
    )
    .unwrap();
    persistence::load_preset(path).unwrap()
}

fn apply(e: &mut Editor, preset: &Preset, mode: crate::preset::ApplyMode, name: &str) {
    e.dispatch(Command::ApplyEnvironmentPreset {
        preset: Box::new(preset.clone()),
        mode,
        name: name.into(),
    })
    .unwrap();
}

fn env_names(scene: &Scene) -> Vec<String> {
    // Oldest first (item order).
    scene
        .items
        .iter()
        .filter(|i| i.environment.is_some())
        .map(|i| i.name.clone())
        .collect()
}

#[test]
fn ep_t3_two_environments_and_a_light_replace_the_default_in_one_step() {
    use crate::preset::ApplyMode;
    let dir = scratch("ep3");
    let path = dir.join("sky.openv");
    let mut source = Editor::default();
    source.scene.ensure_environment().unwrap();
    let first = top_env(&source.scene).0;
    source.dispatch(hdr("/hdri/a.hdr")).unwrap();
    let second = source.scene.add_environment().unwrap();
    let _ = (first, second);
    add_light(&mut source, LightKind::Area, DVec3::new(1., 2., 3.));
    let saved = env_preset(&source, &path, true, false);
    assert_eq!(saved.header.kind, PresetKind::Environment);
    assert_eq!(saved.scene.environments().count(), 2);
    assert!(saved.scene.items.iter().all(|i| i.camera.is_none()));
    // The image layer's locator and picture came along.
    let env = saved.scene.environments().last().unwrap().id;
    assert!(saved.scene.environment_image(env).is_some());
    assert!(saved.scene.environment_image_heading(env).is_some());

    let mut target = Editor::default();
    target.scene.ensure_environment().unwrap();
    let default_env = top_env(&target.scene).0;
    let steps = target.undo.len();
    let before = target.scene.clone();
    // Double-click is Replace.
    apply(&mut target, &saved, ApplyMode::Replace, "sky");
    assert_eq!(target.undo.len(), steps + 1);
    assert!(
        target.scene.items.iter().all(|i| i.id != default_env),
        "default env gone"
    );
    assert_eq!(
        env_names(&target.scene),
        env_names(&source.scene),
        "order kept"
    );
    let lights: Vec<&Item> = target
        .scene
        .items
        .iter()
        .filter(|i| i.light.is_some())
        .collect();
    assert_eq!(lights.len(), 1);
    // EP v0.2: it arrives inside one item group named after the preset.
    let group = target
        .scene
        .items
        .iter()
        .find(|i| Some(i.id) == lights[0].parent)
        .unwrap();
    assert_eq!(group.name, "sky");
    assert!(group.is_group_locator());
    assert_eq!(group.common.from_preset.as_deref(), Some("sky"));
    assert!(lights[0].common.from_preset.is_none());
    // The image survives with its own locator in this scene.
    let top = top_env(&target.scene).0;
    let image_env = target
        .scene
        .environments()
        .find(|i| target.scene.environment_image(i.id).is_some())
        .unwrap()
        .id;
    let (locator, _) = target.scene.environment_image_heading(image_env).unwrap();
    assert!(
        target
            .scene
            .items
            .iter()
            .any(|i| i.id == locator && i.texture_locator().is_some())
    );
    let _ = top;
    target.dispatch(Command::Undo).unwrap();
    assert_eq!(target.scene, before);
}

fn preset_groups(scene: &Scene) -> Vec<String> {
    scene
        .items
        .iter()
        .filter(|i| i.common.from_preset.is_some())
        .map(|i| i.name.clone())
        .collect()
}

#[test]
fn ep_t4_replace_removes_the_earlier_presets_group_and_keeps_a_hand_made_spot() {
    use crate::preset::ApplyMode;
    let dir = scratch("ep4");
    let mut a = Editor::default();
    a.scene.ensure_environment().unwrap();
    add_light(&mut a, LightKind::Area, DVec3::ZERO);
    let a = env_preset(&a, &dir.join("a.openv"), true, false);
    let mut b = Editor::default();
    b.scene.ensure_environment().unwrap();
    add_light(&mut b, LightKind::Point, DVec3::ZERO);
    let b = env_preset(&b, &dir.join("b.openv"), true, false);

    let mut e = Editor::default();
    apply(&mut e, &a, ApplyMode::Replace, "A");
    assert_eq!(preset_groups(&e.scene), ["A"]);
    add_light(&mut e, LightKind::Spot, DVec3::ZERO);
    let spot = e.scene.items.last().unwrap().id;
    // Renamed by the user: still found by its marker.
    let group = e
        .scene
        .items
        .iter()
        .find(|i| i.common.from_preset.is_some())
        .unwrap()
        .id;
    e.scene.item_mut(group).unwrap().name = "My Lights".into();
    apply(&mut e, &b, ApplyMode::Replace, "B");
    assert_eq!(preset_groups(&e.scene), ["B"], "A's group gone, B's in");
    let kinds: Vec<LightKind> = e
        .scene
        .items
        .iter()
        .filter_map(|i| i.light.as_ref().map(|l| l.kind))
        .collect();
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert!(kinds.contains(&LightKind::Spot) && kinds.contains(&LightKind::Point));
    assert!(e.scene.item(spot).is_some(), "the hand-made spot stays");
    assert_eq!(e.scene.environments().count(), 1);
}

#[test]
fn ep_t4b_applying_the_same_preset_twice_doesnt_pile_up() {
    use crate::preset::ApplyMode;
    let dir = scratch("ep4b");
    let mut source = Editor::default();
    source.dispatch(hdr("/hdri/sky.hdr")).unwrap();
    add_light(&mut source, LightKind::Area, DVec3::ZERO);
    let preset = env_preset(&source, &dir.join("sky.openv"), true, false);
    let mut e = Editor::default();
    let count = |e: &Editor| {
        (
            e.scene.items.len(),
            e.scene.environments().count(),
            e.scene.shader_tree.nodes.len(),
            e.scene.images.len(),
        )
    };
    apply(&mut e, &preset, ApplyMode::Replace, "sky");
    let once = count(&e);
    apply(&mut e, &preset, ApplyMode::Replace, "sky");
    assert_eq!(once, count(&e));
    assert_eq!(preset_groups(&e.scene), ["sky"]);
}

#[test]
fn replace_leaves_no_orphaned_environment_material_or_locators() {
    use crate::preset::ApplyMode;
    let dir = scratch("ep-orphans");
    let mut source = Editor::default();
    source.scene.ensure_environment().unwrap();
    let preset = env_preset(&source, &dir.join("plain.openv"), false, false);
    let mut e = Editor::default();
    // Two environments with image layers (two locators) to be replaced.
    e.dispatch(hdr("/hdri/a.hdr")).unwrap();
    let second = e.scene.add_environment().unwrap();
    e.dispatch(Command::SetEnvironmentImage {
        path: "/hdri/b.hdr".into(),
        width: 8,
        height: 4,
        item: Some(second),
    })
    .unwrap();
    let locators = |e: &Editor| {
        e.scene
            .items
            .iter()
            .filter(|i| i.texture_locator().is_some())
            .count()
    };
    assert_eq!(locators(&e), 2);
    apply(&mut e, &preset, ApplyMode::Replace, "plain");
    assert_eq!(e.scene.environments().count(), 1);
    assert_eq!(
        locators(&e),
        0,
        "the replaced layers' locators went with them"
    );
    let materials = e
        .scene
        .items
        .iter()
        .filter(|i| i.environment.is_some())
        .count();
    assert_eq!(materials, 1, "one environment, one Environment Material");
    e.scene.validate().unwrap();
}

#[test]
fn cameras_come_only_when_asked_and_land_in_the_group() {
    use crate::preset::{ApplyMode, EnvironmentPresetOptions};
    let dir = scratch("ep-cameras");
    let mut source = Editor::default();
    source.scene.ensure_environment().unwrap();
    source
        .dispatch(Command::AddItem(crate::item_list::AddItemKind::Camera))
        .unwrap();
    source
        .save_environment_preset(&dir.join("no.openv"), Default::default())
        .unwrap();
    let no = persistence::load_preset(&dir.join("no.openv")).unwrap();
    assert!(
        no.scene.items.iter().all(|i| i.camera.is_none()),
        "off by default"
    );
    source
        .save_environment_preset(
            &dir.join("yes.openv"),
            EnvironmentPresetOptions {
                cameras: true,
                ..Default::default()
            },
        )
        .unwrap();
    let yes = persistence::load_preset(&dir.join("yes.openv")).unwrap();
    assert_eq!(
        yes.scene
            .items
            .iter()
            .filter(|i| i.camera.is_some())
            .count(),
        1
    );
    assert!(
        yes.scene.render == Scene::default().render,
        "never render settings"
    );
    let mut e = Editor::default();
    apply(&mut e, &yes, ApplyMode::Add, "Shot");
    let camera = e
        .scene
        .items
        .iter()
        .rev()
        .find(|i| i.camera.is_some())
        .unwrap();
    let group = e
        .scene
        .items
        .iter()
        .find(|i| Some(i.id) == camera.parent)
        .unwrap();
    assert_eq!(group.name, "Shot");
}

#[test]
fn ep_t5_add_puts_the_new_environments_on_top_and_they_win() {
    use crate::environment::RayKind;
    use crate::preset::ApplyMode;
    let dir = scratch("ep5");
    let mut source = Editor::default();
    source.dispatch(hdr("/hdri/sky.hdr")).unwrap();
    let preset = env_preset(&source, &dir.join("sky.openv"), false, false);
    let mut e = Editor::default();
    e.scene.ensure_environment().unwrap();
    let old = top_env(&e.scene).0;
    let steps = e.undo.len();
    apply(&mut e, &preset, ApplyMode::Add, "sky");
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.scene.environments().count(), 2, "nothing replaced");
    let top = top_env(&e.scene).0;
    assert_ne!(top, old);
    assert_eq!(e.scene.environment_for(RayKind::Camera).unwrap().id, top);
    assert!(
        e.scene
            .environment_sky(RayKind::Camera)
            .unwrap()
            .path
            .ends_with("sky.hdr")
    );
    // Name clash: "Environment (2)".
    let names = env_names(&e.scene);
    assert_ne!(names[0], names[1]);
}

#[test]
fn ep_t6_copied_images_travel_and_missing_ones_load_with_a_warning() {
    use crate::preset::ApplyMode;
    let dir = scratch("ep6");
    let pictures = dir.join("pictures");
    std::fs::create_dir_all(&pictures).unwrap();
    let picture = pictures.join("sky.hdr");
    std::fs::write(&picture, b"#?RADIANCE stand-in").unwrap();
    let presets = dir.join("presets");
    std::fs::create_dir_all(&presets).unwrap();
    let mut source = Editor::default();
    source
        .dispatch(Command::SetEnvironmentImage {
            path: picture.to_string_lossy().into_owned(),
            width: 8,
            height: 4,
            item: None,
        })
        .unwrap();
    // Copy Images: the picture is copied beside the preset, named relative.
    source
        .save_environment_preset(
            &presets.join("copied.openv"),
            crate::preset::EnvironmentPresetOptions {
                lights: true,
                meshes: false,
                cameras: false,
                copy_images: true,
            },
        )
        .unwrap();
    assert!(presets.join("sky.hdr").exists());
    let text = std::fs::read_to_string(presets.join("copied.openv")).unwrap();
    assert!(text.contains("\"sky.hdr\""), "relative path stored");
    // The folder moves: it still loads clean.
    let moved = dir.join("moved");
    std::fs::rename(&presets, &moved).unwrap();
    let loaded = persistence::load_preset(&moved.join("copied.openv")).unwrap();
    assert!(crate::preset::missing_images(&loaded).is_empty());
    let mut e = Editor::default();
    apply(&mut e, &loaded, ApplyMode::Replace, "copied");
    let env = top_env(&e.scene).0;
    assert!(
        e.scene
            .environment_image(env)
            .unwrap()
            .path
            .starts_with(&*moved.to_string_lossy())
    );
    // Without copying, an image elsewhere is stored absolute; deleted, the
    // preset still loads, keeps the path and reports it once.
    source
        .save_environment_preset(
            &dir.join("absolute.openv"),
            crate::preset::EnvironmentPresetOptions {
                lights: true,
                meshes: false,
                cameras: false,
                copy_images: false,
            },
        )
        .unwrap();
    std::fs::remove_file(&picture).unwrap();
    let loaded = persistence::load_preset(&dir.join("absolute.openv")).unwrap();
    let missing = crate::preset::missing_images(&loaded);
    assert_eq!(missing, vec![picture.to_string_lossy().into_owned()]);
    let mut e = Editor::default();
    apply(&mut e, &loaded, ApplyMode::Replace, "absolute");
    let env = top_env(&e.scene).0;
    assert_eq!(
        e.scene.environment_image(env).unwrap().path,
        picture.to_string_lossy()
    );
}

#[test]
fn environment_presets_are_openv_and_older_files_still_read() {
    let dir = scratch("ep-ext");
    let mut e = Editor::default();
    e.scene.ensure_environment().unwrap();
    // Wrong extension for an environment preset: refused.
    assert!(
        e.save_environment_preset(
            &dir.join("x.json"),
            crate::preset::EnvironmentPresetOptions {
                lights: false,
                meshes: false,
                cameras: false,
                copy_images: false
            }
        )
        .is_err()
    );
    // .oppreset (what older builds wrote) is still accepted.
    e.save_environment_preset(
        &dir.join("old.oppreset"),
        crate::preset::EnvironmentPresetOptions {
            lights: false,
            meshes: false,
            cameras: false,
            copy_images: false,
        },
    )
    .unwrap();
    assert_eq!(
        library::classify(&dir.join("old.oppreset"), &|p| {
            persistence::read_preset_header(p).ok().map(|h| h.kind)
        }),
        library::EntryKind::Preset(PresetKind::Environment)
    );
    e.save_environment_preset(
        &dir.join("new.openv"),
        crate::preset::EnvironmentPresetOptions {
            lights: false,
            meshes: false,
            cameras: false,
            copy_images: false,
        },
    )
    .unwrap();
    assert_eq!(
        library::classify(&dir.join("new.openv"), &|p| {
            persistence::read_preset_header(p).ok().map(|h| h.kind)
        }),
        library::EntryKind::Preset(PresetKind::Environment)
    );
    // A scene without the marker stays at its old version; with it, v42,
    // and the marker round-trips.
    let mut plain = Editor::default();
    add_light(&mut plain, LightKind::Point, DVec3::ZERO);
    let text = String::from_utf8(persistence::encode(&plain.scene).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 42"));
    let id = plain.scene.items.last().unwrap().id;
    plain.scene.item_mut(id).unwrap().common.from_preset = Some("Studio".into());
    let bytes = persistence::encode(&plain.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 42"), "marker needs v42");
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(
        back.item(id).unwrap().common.from_preset.as_deref(),
        Some("Studio")
    );
}

/// A preset whose maps sit beside it keeps naming them relative to its
/// folder when it is rewritten after a load (a new thumbnail, a re-render,
/// a texture set saved with full paths), so the folder stays portable.
/// Pictures outside the folder keep their full paths.
#[test]
fn rewritten_presets_keep_beside_maps_relative() {
    let dir = scratch("relative-maps");
    let folder = dir.join("Wood");
    std::fs::create_dir_all(folder.join("maps")).unwrap();
    std::fs::write(folder.join("Wood_Color.jpg"), b"jpg").unwrap();
    std::fs::write(folder.join("maps/Wood_Rough.jpg"), b"jpg").unwrap();
    let outside = dir.join("Shared.jpg").to_string_lossy().into_owned();
    let mut e = Editor::default();
    let group = group_with_two_layers(&mut e);
    let mut preset = PresetSource::Material(group)
        .build(&e.scene)
        .unwrap()
        .preset;
    let image = |id: u64, path: &str| crate::backdrop::SceneImage {
        id,
        name: path.into(),
        path: path.into(),
        width: 4,
        height: 4,
    };
    preset.scene.images = vec![
        image(1, "Wood_Color.jpg"),
        image(2, "maps/Wood_Rough.jpg"),
        image(3, &outside),
    ];
    let path = folder.join("Wood.oppreset");
    persistence::save_preset(&path, &preset).unwrap();
    let stored = |path: &std::path::Path| -> Vec<String> {
        let text: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        text["scene"]["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["path"].as_str().unwrap().to_owned())
            .collect()
    };
    let expected = vec![
        "Wood_Color.jpg".to_owned(),
        "maps/Wood_Rough.jpg".to_owned(),
        outside.clone(),
    ];
    assert_eq!(stored(&path), expected);

    // Loading gives full paths; a new thumbnail must not write them back.
    let loaded = persistence::load_preset(&path).unwrap();
    assert!(
        loaded.scene.images[..2]
            .iter()
            .all(|i| std::path::Path::new(&i.path).is_absolute())
    );
    persistence::set_preset_thumbnail(&path, Some(b"png")).unwrap();
    assert_eq!(stored(&path), expected);

    // Saving the loaded (full-path) preset straight back, as Save Texture
    // Set as Preset does with its maps, also comes out relative.
    persistence::save_preset(&path, &loaded).unwrap();
    assert_eq!(stored(&path), expected);

    // The folder moved elsewhere still finds its maps.
    let moved = dir.join("Moved");
    std::fs::rename(&folder, &moved).unwrap();
    let again = persistence::load_preset(&moved.join("Wood.oppreset")).unwrap();
    assert!(
        again.scene.images[..2]
            .iter()
            .all(|i| std::path::Path::new(&i.path).is_file()
                && i.path.starts_with(moved.to_string_lossy().as_ref()))
    );
    assert_eq!(again.scene.images[2].path, outside);
    std::fs::remove_dir_all(&dir).unwrap();
}
