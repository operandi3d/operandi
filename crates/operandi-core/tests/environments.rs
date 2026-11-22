//! Environment items (specifications: environments-2026-11-09, "ENV"):
//! the new-scene default, the colour-from-direction maths, several
//! environments, edits as undo steps, duplicate, save/reopen and old files.
use glam::DVec3;
use operandi_core::environment::*;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn envs(e: &Editor) -> Vec<&Item> {
    e.scene
        .items
        .iter()
        .filter(|i| i.environment.is_some())
        .collect()
}

fn env(e: &Editor, id: u64) -> &Environment {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .environment
        .as_deref()
        .unwrap()
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

fn up(degrees: f64) -> DVec3 {
    let r = degrees.to_radians();
    DVec3::new(r.cos(), r.sin(), 0.0)
}

/// ENV §1-§3: one "Environment" holding "Environment Material", 4 Color
/// Gradient defaults; Intensity 1 and every ray type on.
#[test]
fn new_scene_has_the_default_environment() {
    let e = new_scene();
    let all = envs(&e);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "Environment");
    let env = all[0].environment.as_ref().unwrap();
    assert_eq!(env.material.name, "Environment Material");
    assert!(env.layers.is_empty(), "no texture layers");
    assert_eq!(env.intensity, 1.0);
    assert!(RayKind::ALL.iter().all(|&k| env.visible_to(k)));
    assert!(env.lpe_label.is_empty());
    let layer = &env.material.layer;
    assert!(layer.enable && !layer.invert && layer.opacity == 1.0);
    let m = &env.material.settings;
    assert_eq!(m.kind, EnvironmentType::FourColor);
    assert_eq!(
        (m.zenith, m.sky, m.ground, m.nadir),
        ([0.9; 3], [0.34; 3], [0.34; 3], [0.03; 3])
    );
    assert_eq!((m.sky_exponent, m.ground_exponent), (4.0, 4.0));
    assert_eq!(m.daylight, Daylight::default());
    assert_eq!(m.daylight.sun_light, None);
    assert_eq!(m.fog.fog_type, EnvironmentFogType::None);
    assert_eq!((m.fog.start, m.fog.end, m.fog.density), (0.0, 10.0, 0.1));
    // Not a mesh: nothing to draw or export.
    assert!(!all[0].is_mesh());
}

/// ENV §4's worked check and each type's formula.
#[test]
fn colour_from_direction() {
    let mut m = EnvironmentMaterial::default();
    assert!(close(m.color(DVec3::Y), [0.9; 3]));
    assert!(close(m.color(up(1e-9)), [0.34; 3]));
    assert!(close(m.color(up(-1e-9)), [0.34; 3]));
    assert!(close(m.color(-DVec3::Y), [0.03; 3]));
    let thirty = m.color(up(30.0))[0];
    assert!((thirty - 0.865).abs() < 1e-9, "{thirty}");
    // Below: lerp(nadir, ground, (1 - |s|)^4).
    let below = m.color(up(-30.0))[0];
    assert!((below - (0.03 + (0.34 - 0.03) * 0.5f64.powi(4))).abs() < 1e-9);
    // Hard edge at the horizon (sky above, ground below).
    m.sky = [1.0, 0.0, 0.0];
    m.ground = [0.0, 0.0, 1.0];
    assert!(close(m.color(up(1e-9)), [1.0, 0.0, 0.0]));
    assert!(close(m.color(up(-1e-9)), [0.0, 0.0, 1.0]));
    // Exponent 1: linear in s.
    m.sky_exponent = 1.0;
    assert!(close(m.color(up(30.0)), [0.95, 0.45, 0.45]));

    let m = EnvironmentMaterial {
        kind: EnvironmentType::Constant,
        ..Default::default()
    };
    assert!(close(m.color(-DVec3::Y), [0.9; 3]));
    let m = EnvironmentMaterial {
        kind: EnvironmentType::TwoColor,
        ..Default::default()
    };
    assert!(
        close(m.color(DVec3::X), [(0.03 + 0.9) / 2.0; 3]),
        "smooth, no edge"
    );
    assert!(close(m.color(-DVec3::Y), [0.03; 3]));
    let m = EnvironmentMaterial {
        kind: EnvironmentType::Overcast,
        ..Default::default()
    };
    assert!(close(m.color(DVec3::Y), [0.9; 3]));
    assert!(close(m.color(up(1e-12)), [0.3; 3]), "horizon is a third");
    assert!(close(m.color(up(-45.0)), [0.03; 3]), "nadir below, flat");
}

/// ENV §6: each ray type sees the newest environment visible to it;
/// Intensity scales only its own; none is black.
#[test]
fn newest_visible_environment_wins() {
    let mut e = new_scene();
    let old = envs(&e)[0].id;
    e.dispatch(Command::AddItem(item_list::AddItemKind::Environment))
        .unwrap();
    let new = e.scene.selection.item.unwrap();
    let names: Vec<_> = envs(&e).iter().map(|i| i.name.clone()).collect();
    assert_eq!(names, ["Environment", "Environment (2)"]);
    assert_eq!(env(&e, new).material.name, "Environment Material (2)");
    let set =
        |e: &mut Editor, item, edit| e.dispatch(Command::EditEnvironment { item, edit }).unwrap();
    set(
        &mut e,
        old,
        EnvironmentEdit::Color(GradientColor::Zenith, [1.0, 0.0, 0.0]),
    );
    set(
        &mut e,
        old,
        EnvironmentEdit::Kind(EnvironmentType::Constant),
    );
    set(
        &mut e,
        new,
        EnvironmentEdit::Color(GradientColor::Zenith, [0.0, 1.0, 0.0]),
    );
    set(
        &mut e,
        new,
        EnvironmentEdit::Kind(EnvironmentType::Constant),
    );
    let camera = |e: &Editor| e.scene.environment_radiance(RayKind::Camera, DVec3::Y);
    assert_eq!(camera(&e), [0.0, 1.0, 0.0], "both visible: the newer");
    set(
        &mut e,
        new,
        EnvironmentEdit::Visible(RayKind::Camera, false),
    );
    assert_eq!(camera(&e), [1.0, 0.0, 0.0], "newer hidden: the older");
    assert_eq!(
        e.scene.environment_for(RayKind::Reflection).unwrap().id,
        new
    );
    set(&mut e, new, EnvironmentEdit::Visible(RayKind::Camera, true));
    set(
        &mut e,
        old,
        EnvironmentEdit::Visible(RayKind::Camera, false),
    );
    assert_eq!(camera(&e), [0.0, 1.0, 0.0], "older hidden: the newer");
    set(&mut e, new, EnvironmentEdit::Intensity(0.5));
    assert_eq!(camera(&e), [0.0, 0.5, 0.0], "Intensity halves its own");
    e.scene.items.retain(|i| i.environment.is_none());
    assert_eq!(camera(&e), [0.0; 3], "no environment is black");
}

/// ENVIRONMENT-PLACEMENT §2: an environment has no transform. Every edit
/// that would give it one is refused and changes nothing; a rotation
/// stored anyway is ignored by the colour a ray sees.
#[test]
fn environments_have_no_transform() {
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    let before = e.scene.clone();
    // The Rotation Order channel isn't there, and a transform edit is
    // refused.
    assert!(channels::set(&mut e, channels::Target::Item(id), "rotOrder", "ZXY").is_err());
    let mut moved = scene::Transform::default();
    moved.translation.x = 1.0;
    moved.rotation.y = 1.0;
    let r = e.dispatch(Command::ItemTransform {
        item: id,
        transform: moved,
    });
    assert!(r.is_err());
    assert_eq!(e.scene, before, "nothing changed");
    // Parenting either way is refused.
    let mesh = e
        .scene
        .items
        .iter()
        .find(|i| i.camera.is_none() && i.light.is_none() && i.environment.is_none())
        .unwrap()
        .id;
    for (item, parent) in [(mesh, id), (id, mesh)] {
        let r = e.dispatch(Command::SetParent {
            item,
            parent: Some(parent),
            keep_position: false,
        });
        assert!(r.is_err());
        assert_eq!(e.scene, before);
    }
    // Several selected items: the environment isn't a transform target.
    e.dispatch(Command::SelectItems(vec![mesh, id])).unwrap();
    assert_eq!(e.scene.item_transform_targets(), Some(vec![mesh]));
    // A stored rotation (bypassing the checks) is ignored: straight up
    // still sees the zenith.
    let item = e.scene.items.iter_mut().find(|i| i.id == id).unwrap();
    item.transform.rotation = DVec3::new(std::f64::consts::PI, 0.0, 0.0);
    let c = e.scene.environment_radiance(RayKind::Camera, DVec3::Y);
    assert!(close(c, [0.90; 3]), "{c:?}");
}

/// ENVIRONMENT-PLACEMENT §2: a scene saved by an earlier build with an
/// environment transform, parent and child still opens: the transform and
/// parent are dropped and the child is unparented where it stands.
#[test]
fn old_scene_with_an_environment_transform_loads() {
    let e = new_scene();
    let env_id = envs(&e)[0].id;
    let mesh = e
        .scene
        .items
        .iter()
        .find(|i| i.camera.is_none() && i.light.is_none() && i.environment.is_none())
        .unwrap()
        .id;
    let camera = e
        .scene
        .items
        .iter()
        .find(|i| i.camera.is_some())
        .unwrap()
        .id;
    let mut doc: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&e.scene).unwrap()).unwrap();
    for item in doc["scene"]["items"].as_array_mut().unwrap() {
        let id = item["id"].as_u64().unwrap();
        if id == env_id {
            item["transform"]["translation"] = serde_json::json!([0.0, 2.0, 0.0]);
            item["transform"]["rotation"] = serde_json::json!([0.0, 1.0, 0.0]);
            item["parent"] = serde_json::json!(camera);
        } else if id == mesh {
            item["parent"] = serde_json::json!(env_id);
        }
    }
    // Where the mesh stood in the old file.
    let mut old = e.scene.clone();
    for item in &mut old.items {
        if item.id == env_id {
            item.transform.translation.y = 2.0;
            item.transform.rotation.y = 1.0;
            item.parent = Some(camera);
        } else if item.id == mesh {
            item.parent = Some(env_id);
        }
    }
    let stood = old.world_matrix(mesh).unwrap();
    let opened = persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
    let env = opened.items.iter().find(|i| i.id == env_id).unwrap();
    assert_eq!(env.transform, scene::Transform::default());
    assert_eq!(env.parent, None);
    let child = opened.items.iter().find(|i| i.id == mesh).unwrap();
    assert_eq!(child.parent, None);
    // Kept where it stood.
    let world = opened.world_matrix(mesh).unwrap();
    assert!(world.abs_diff_eq(stood, 1e-9), "{world:?} vs {stood:?}");
    // Re-saved, it round-trips unchanged.
    let again = persistence::decode(&persistence::encode(&opened).unwrap()).unwrap();
    assert_eq!(again, opened);
}

/// ENVIRONMENT-PLACEMENT §1: no environment rows in the Item List under any
/// filter, and the list's Select patterns never pick one.
#[test]
fn item_list_never_shows_environments() {
    use item_list::*;
    let mut e = new_scene();
    e.dispatch(Command::AddItem(AddItemKind::Environment))
        .unwrap();
    let env_ids: Vec<u64> = envs(&e).iter().map(|i| i.id).collect();
    assert_eq!(env_ids.len(), 2);
    // The new one is selected; Only Selected must still show nothing of it.
    assert!(env_ids.contains(&e.scene.selection.item.unwrap()));
    let filters = [
        ListFilter::default(),
        ListFilter {
            flatten: true,
            ..Default::default()
        },
        ListFilter {
            sort_by_type: true,
            sort_by_name: true,
            ..Default::default()
        },
        ListFilter {
            only_selected: true,
            ..Default::default()
        },
        ListFilter {
            text: Some("Env".into()),
            ..Default::default()
        },
        ListFilter {
            text: Some("Env".into()),
            text_flat: true,
            ..Default::default()
        },
    ];
    for filter in filters {
        let rows = list_rows(&e.scene, &filter, &Default::default());
        assert!(
            rows.iter().all(|r| !env_ids.contains(&r.id)),
            "{filter:?}: {rows:?}"
        );
    }
    let plain = list_rows(&e.scene, &ListFilter::default(), &Default::default());
    assert_eq!(
        plain.len(),
        e.scene.items.len() - 2,
        "every other item shows"
    );
    // Select > * (every type ticked) picks every item but the environments.
    let matches = e.scene.pattern_matches(&PatternQuery::default());
    assert!(matches.iter().all(|id| !env_ids.contains(id)));
    assert_eq!(matches.len(), e.scene.items.len() - 2);
}

/// ENVIRONMENT-PLACEMENT §3.2: a hidden environment (its eye off) is seen
/// by no ray; the next one down is.
#[test]
fn hidden_environment_is_skipped() {
    let mut e = new_scene();
    let old = envs(&e)[0].id;
    e.dispatch(Command::AddItem(item_list::AddItemKind::Environment))
        .unwrap();
    let new = e.scene.selection.item.unwrap();
    e.dispatch(Command::EditEnvironment {
        item: new,
        edit: EnvironmentEdit::Color(GradientColor::Zenith, [1.0, 0.0, 0.0]),
    })
    .unwrap();
    let top = |e: &Editor| e.scene.environment_for(RayKind::Camera).map(|i| i.id);
    assert_eq!(top(&e), Some(new));
    e.dispatch(Command::Visibility(new)).unwrap();
    assert_eq!(top(&e), Some(old));
    e.dispatch(Command::Visibility(old)).unwrap();
    assert_eq!(top(&e), None);
    assert_eq!(
        e.scene.environment_radiance(RayKind::Camera, DVec3::Y),
        [0.0; 3]
    );
}

/// ENV §11: every edit is one undo step named after its property; a bad
/// value changes nothing.
#[test]
fn edits_are_named_undo_steps() {
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    let before = e.scene.clone();
    let steps = e.undo.len();
    let edits = [
        EnvironmentEdit::Intensity(2.0),
        EnvironmentEdit::Visible(RayKind::Refraction, false),
        EnvironmentEdit::LpeLabel("sky".into()),
        EnvironmentEdit::Opacity(0.5),
        EnvironmentEdit::Kind(EnvironmentType::Overcast),
        EnvironmentEdit::SkyExponent(1.0),
        EnvironmentEdit::FogType(EnvironmentFogType::Linear),
        EnvironmentEdit::Haze(3.0),
    ];
    for edit in edits.clone() {
        let label = edit.label();
        e.dispatch(Command::EditEnvironment { item: id, edit })
            .unwrap();
        assert_eq!(e.undo.last().unwrap().label, label);
    }
    assert_eq!(e.undo.len(), steps + edits.len());
    for bad in [
        EnvironmentEdit::Opacity(2.0),
        EnvironmentEdit::Intensity(f64::NAN),
        EnvironmentEdit::SkyExponent(-1.0),
        EnvironmentEdit::MaterialName(" ".into()),
    ] {
        let now = e.scene.clone();
        assert!(
            e.dispatch(Command::EditEnvironment {
                item: id,
                edit: bad
            })
            .is_err()
        );
        assert_eq!(e.scene, now);
    }
    // Sun Light takes only a directional light.
    let light = e.scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
    let mesh = e.scene.items[0].id;
    e.dispatch(Command::EditEnvironment {
        item: id,
        edit: EnvironmentEdit::SunLight(Some(light)),
    })
    .unwrap();
    assert!(
        e.dispatch(Command::EditEnvironment {
            item: id,
            edit: EnvironmentEdit::SunLight(Some(mesh)),
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::EditEnvironment {
            item: mesh,
            edit: EnvironmentEdit::Intensity(1.0),
        })
        .is_err()
    );
    for _ in 0..edits.len() + 1 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert_eq!(e.scene.items, before.items);
}

/// `item.channel` on an environment (UIC-163/165): its item channels, its
/// Environment Material's (`layer:<id>`) and a texture layer's, set as one
/// undo step and read back.
#[test]
fn channels_set_and_read_back() {
    use channels::{Target, get, set};
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    let item = Target::Item(id);
    let material = Target::Layer(env(&e, id).material.id);
    for (target, name, value, back) in [
        (item, "intensity", "2", "2"),
        (item, "visCamera", "off", "false"),
        (item, "visRefraction", "false", "false"),
        (item, "lpeLabel", "key", "key"),
        (material, "enable", "false", "false"),
        (material, "opacity", "50%", "0.5"),
        (material, "blend", "Multiply", "Multiply"),
        (material, "type", "Overcast", "Overcast"),
        (material, "type", "1", "2 Color Gradient"),
        (material, "zenColor", "1,0.5,0", "1,0.5,0"),
        (material, "nadColor.b", "0.25", "0.25"),
        (material, "skyExp", "2", "2"),
        (material, "discSize", "150%", "1.5"),
        (material, "clampSky", "false", "false"),
        (material, "fogType", "Layered", "Layered"),
        (material, "fogEnd", "20m", "20"),
        (material, "fogFalloff", "0.25", "0.25"),
    ] {
        let steps = e.undo.len();
        set(&mut e, target, name, value).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(e.undo.len(), steps + 1, "{name}: one undo step");
        assert_eq!(get(&e.scene, target, name).unwrap(), back, "{name}");
    }
    layer_edit(
        &mut e,
        id,
        EnvironmentLayerEdit::Add {
            layer: shader_tree::NewLayer::Constant,
            at: LayerPlace::Top,
        },
    )
    .unwrap();
    let layer = Target::Layer(env(&e, id).layers[0].id);
    for (name, value, back) in [
        ("color", "0,1,0", "0,1,0"),
        ("opacity", "0.5", "0.5"),
        ("effect", "Driver A", "Driver A"),
        ("enable", "false", "false"),
    ] {
        set(&mut e, layer, name, value).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(get(&e.scene, layer, name).unwrap(), back, "{name}");
    }
    let before = e.scene.clone();
    assert!(
        set(&mut e, item, "zenColor", "1,1,1").is_err(),
        "a material channel"
    );
    assert!(
        set(&mut e, material, "intensity", "1").is_err(),
        "an item channel"
    );
    assert!(set(&mut e, material, "opacity", "2").is_err());
    assert!(set(&mut e, layer, "image", "nope").is_err());
    assert_eq!(e.scene, before);
}

/// ENV §6: Create and Duplicate are one step each; a duplicate copies the
/// settings with its own layer ids and material name; Delete removes it.
#[test]
fn create_duplicate_delete() {
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    e.dispatch(Command::EditEnvironment {
        item: id,
        edit: EnvironmentEdit::Intensity(3.0),
    })
    .unwrap();
    e.scene.select_item(Some(id));
    let steps = e.undo.len();
    e.dispatch(Command::DuplicateItem).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let copy = e.scene.selection.item.unwrap();
    assert_ne!(copy, id);
    let (a, b) = (env(&e, id), env(&e, copy));
    assert_eq!(b.intensity, 3.0);
    assert_ne!(a.material.id, b.material.id);
    assert_eq!(b.material.name, "Environment Material (2)");
    assert_eq!(
        e.scene.environments().next().unwrap().id,
        copy,
        "the copy is newest"
    );
    e.dispatch(Command::DeleteItem).unwrap();
    assert_eq!(envs(&e).len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(envs(&e).len(), 2);
    e.dispatch(Command::AddItem(item_list::AddItemKind::Environment))
        .unwrap();
    assert_eq!(envs(&e).len(), 3);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(envs(&e).len(), 2);
}

/// ENV §11: everything saves and reopens; files from before version 33
/// with a camera or light get the default environment; a scene saved
/// with none keeps none.
#[test]
fn save_reopen_and_old_files() {
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    for edit in [
        EnvironmentEdit::Intensity(2.0),
        EnvironmentEdit::Kind(EnvironmentType::Physical),
        EnvironmentEdit::DiscInScatter(0.25),
        EnvironmentEdit::FogEnd(20.0),
        EnvironmentEdit::LpeLabel("sky".into()),
    ] {
        e.dispatch(Command::EditEnvironment { item: id, edit })
            .unwrap();
    }
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 33"));
    assert_eq!(persistence::decode(text.as_bytes()).unwrap(), e.scene);
    let old = text.replacen("\"version\": 33", "\"version\": 32", 1);
    assert!(
        persistence::decode(old.as_bytes()).is_err(),
        "32 can't hold environments"
    );

    // An older file with a light: the default environment is added.
    let mut legacy = new_scene();
    legacy.scene.items.retain(|i| i.environment.is_none());
    let text = String::from_utf8(persistence::encode(&legacy.scene).unwrap()).unwrap();
    assert!(
        text.contains("\"version\": 33"),
        "no environment on purpose is version 33"
    );
    assert!(
        persistence::decode(text.as_bytes())
            .unwrap()
            .environments()
            .next()
            .is_none()
    );
    let old = text.replacen("\"version\": 33", "\"version\": 16", 1);
    let opened = persistence::decode(old.as_bytes()).unwrap();
    let added: Vec<_> = opened.environments().collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].name, "Environment");
    assert_eq!(
        added[0]
            .environment
            .as_ref()
            .unwrap()
            .material
            .settings
            .as_ref(),
        &EnvironmentMaterial::default()
    );

    // A bare scene without lights is unchanged, at its old version.
    let mut bare = Scene::default();
    bare.new_mesh().unwrap();
    let text = String::from_utf8(persistence::encode(&bare).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 33"));
    assert_eq!(persistence::decode(text.as_bytes()).unwrap(), bare);
}

fn layer_edit(e: &mut Editor, item: u64, edit: EnvironmentLayerEdit) -> Result<()> {
    e.dispatch(Command::EditEnvironmentLayers { item, edit })
}

fn locator_of(e: &Editor, node: &shader_tree::Node) -> Item {
    let shader_tree::NodeKind::Texture(t) = &node.kind else {
        panic!("not a texture")
    };
    e.scene
        .items
        .iter()
        .find(|i| Some(i.id) == t.locator)
        .unwrap()
        .clone()
}

/// ENV §5: texture layers over the Environment Material, Effect
/// Environment Color, with their locators (Solid / Spherical Y); the
/// measured case: green Constant at 50 % Normal over red is (0.5, 0.5, 0).
#[test]
fn layers_blend_over_the_environment_material() {
    use shader_tree::{NewLayer, NodeKind, Projection, TextureEffect, TextureSource};
    let mut e = new_scene();
    let id = envs(&e)[0].id;
    e.dispatch(Command::EditEnvironment {
        item: id,
        edit: EnvironmentEdit::Kind(EnvironmentType::Constant),
    })
    .unwrap();
    e.dispatch(Command::EditEnvironment {
        item: id,
        edit: EnvironmentEdit::Color(GradientColor::Zenith, [1.0, 0.0, 0.0]),
    })
    .unwrap();
    let steps = e.undo.len();
    layer_edit(
        &mut e,
        id,
        EnvironmentLayerEdit::Add {
            layer: NewLayer::Constant,
            at: LayerPlace::Top,
        },
    )
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let mut node = env(&e, id).layers[0].clone();
    assert_eq!(node.name, "Constant");
    let NodeKind::Texture(t) = &mut node.kind else {
        panic!()
    };
    assert_eq!(t.effect, TextureEffect::EnvironmentColor);
    let locator = locator_of(&e, &env(&e, id).layers[0]);
    assert_eq!(locator.name, "Constant (Texture)");
    assert_eq!(
        locator.texture_locator().unwrap().projection,
        Projection::Solid
    );
    t.source = TextureSource::Constant {
        color: [0.0, 1.0, 0.0],
        value: 1.0,
    };
    node.layer.opacity = 0.5;
    layer_edit(
        &mut e,
        id,
        EnvironmentLayerEdit::Replace(Box::new(node.clone())),
    )
    .unwrap();
    let c = e.scene.environment_radiance(RayKind::Camera, DVec3::Y);
    assert!(close(c, [0.5, 0.5, 0.0]), "{c:?}");
    // Add: 100 % white over it.
    let mut add = node.clone();
    add.layer.blend = shader_tree::BlendMode::Add;
    add.layer.opacity = 1.0;
    layer_edit(&mut e, id, EnvironmentLayerEdit::Replace(Box::new(add))).unwrap();
    let c = e.scene.environment_radiance(RayKind::Camera, DVec3::Y);
    assert!(close(c, [1.0, 1.0, 0.0]), "{c:?}");

    // An Image Map lands above the selected layer, Spherical about Y.
    layer_edit(
        &mut e,
        id,
        EnvironmentLayerEdit::Add {
            layer: NewLayer::ImageMap,
            at: LayerPlace::Top,
        },
    )
    .unwrap();
    let image = env(&e, id).layers[0].clone();
    assert_eq!(image.name, "Image Map");
    let l = locator_of(&e, &image);
    let l = l.texture_locator().unwrap();
    assert_eq!((l.projection, l.axis), (Projection::Spherical, 1));
    layer_edit(
        &mut e,
        id,
        EnvironmentLayerEdit::Add {
            layer: NewLayer::Constant,
            at: LayerPlace::Above(node.id),
        },
    )
    .unwrap();
    let names: Vec<_> = env(&e, id).layers.iter().map(|n| n.name.clone()).collect();
    assert_eq!(names, ["Image Map", "Constant (2)", "Constant"]);

    // Not under an environment: materials, groups; a surface effect.
    for layer in [NewLayer::Material, NewLayer::Group, NewLayer::Shader] {
        assert!(
            layer_edit(
                &mut e,
                id,
                EnvironmentLayerEdit::Add {
                    layer,
                    at: LayerPlace::Top
                }
            )
            .is_err()
        );
    }
    let mut wrong = image.clone();
    if let NodeKind::Texture(t) = &mut wrong.kind {
        t.effect = TextureEffect::DiffuseColor;
    }
    assert!(layer_edit(&mut e, id, EnvironmentLayerEdit::Replace(Box::new(wrong))).is_err());

    // Saved and reopened.
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);

    // Deleting a layer deletes its locator.
    let gone = l_id(&e, &image);
    layer_edit(&mut e, id, EnvironmentLayerEdit::Remove(vec![image.id])).unwrap();
    assert!(e.scene.items.iter().all(|i| i.id != gone));
    // Deleting the environment deletes the rest of its locators.
    let rest = e.scene.environment_locators(&[id]);
    assert_eq!(rest.len(), 2);
    e.scene.select_item(Some(id));
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(e.scene.items.iter().all(|i| !rest.contains(&i.id)));
    e.dispatch(Command::Undo).unwrap();
    assert!(
        rest.iter()
            .all(|r| e.scene.items.iter().any(|i| i.id == *r))
    );
}

fn l_id(e: &Editor, node: &shader_tree::Node) -> u64 {
    locator_of(e, node).id
}

/// ENVIRONMENT-PLACEMENT §1: Item and Assembly presets work on Item List
/// items; a selected environment stays out (an Environment preset holds it).
#[test]
fn item_presets_leave_environments_out() {
    use preset::PresetSource;
    let mut e = new_scene();
    let env_id = envs(&e)[0].id;
    e.dispatch(Command::SelectItem(Some(env_id))).unwrap();
    for source in [PresetSource::Items, PresetSource::Assembly] {
        assert!(
            source.build(&e.scene).is_err(),
            "only an environment selected"
        );
    }
    let all: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    e.dispatch(Command::SelectItems(all)).unwrap();
    for source in [PresetSource::Items, PresetSource::Assembly] {
        let built = source.build(&e.scene).unwrap();
        assert!(
            built.preset.scene.environments().next().is_none(),
            "{source:?}"
        );
    }
    let built = PresetSource::Environment {
        lights: true,
        meshes: false,
        cameras: false,
    }
    .build(&e.scene)
    .unwrap();
    assert_eq!(built.preset.scene.environments().count(), 1);
}
