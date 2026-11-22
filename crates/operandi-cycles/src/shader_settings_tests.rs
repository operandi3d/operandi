//! Shader tree remaining, stage 1 in Cycles: cut-outs, Alpha Type
//! Constant, fog, shader Light Linking, Direct Illum Multiplier and
//! Receive Shadows.
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::shader_tree::{FogType, GroupMask, ItemMask, Layer, Node, NodeId, ShaderTree};
#[cfg(feature = "cycles")]
use operandi_core::shader_tree::{TextureEffect, TextureLayer, TextureSource};

fn shader_mut(tree: &mut ShaderTree, id: NodeId) -> &mut ShaderChannels {
    match &mut tree.nodes.iter_mut().find(|n| n.id == id).unwrap().kind {
        NodeKind::Shader(s) => s,
        _ => panic!("not a shader"),
    }
}

fn base_material(scene: &mut Scene) -> &mut operandi_core::shader_tree::MaterialChannels {
    match &mut scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .unwrap()
        .kind
    {
        NodeKind::Material(m) => m,
        _ => panic!("not a material"),
    }
}

fn insert(tree: &mut ShaderTree, at: usize, parent: Option<NodeId>, kind: NodeKind) -> NodeId {
    let id = tree.next_id;
    tree.next_id += 1;
    tree.nodes.insert(
        at,
        Node {
            extras: Default::default(),
            id,
            parent,
            name: format!("Layer {id}"),
            layer: Layer::default(),
            kind,
        },
    );
    tree.validate().unwrap();
    id
}

#[cfg(feature = "cycles")]
fn constant(effect: TextureEffect, value: f64) -> NodeKind {
    NodeKind::Texture(TextureLayer {
        source: TextureSource::Constant {
            color: [0.; 3],
            value,
        },
        effect,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

fn add_box(scene: &mut Scene, centre: DVec3, size: DVec3) -> u64 {
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(centre, size, 1).unwrap().into();
    item.id
}

/// An unlit patch: a single square at z = 0.5 facing the camera that glows
/// (0.8, 0.4, 0.2) and reflects nothing.
fn unlit_patch() -> Scene {
    let mut scene = Scene::default();
    add_box(&mut scene, DVec3::ZERO, DVec3::ONE);
    let mesh = &mut scene.items.last_mut().unwrap().mesh;
    let vertices = mesh.vertices.clone();
    mesh.faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    assert_eq!(mesh.faces.len(), 1);
    let m = base_material(&mut scene);
    m.diffuse_color = [0.; 3];
    m.specular_amount = 0.;
    m.luminous_color = [0.8, 0.4, 0.2];
    m.luminous_intensity = 1.;
    scene
}

#[cfg(feature = "cycles")]
fn centre(scene: &Scene, eye_z: f64, f: impl FnOnce(&mut Settings)) -> [f32; 4] {
    let eye = DVec3::new(0.0, 0.0, eye_z);
    let mut settings = Settings::studio(
        8,
        8,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.05),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 64;
    settings.gpu = false;
    settings.denoise = false;
    f(&mut settings);
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    let mut sum = [0.0f32; 4];
    for y in 3..5 {
        for x in 3..5 {
            for (k, s) in sum.iter_mut().enumerate() {
                *s += img.rgba[(y * 8 + x) * 4 + k] / 4.0;
            }
        }
    }
    sum
}

#[cfg(feature = "cycles")]
fn near(got: [f32; 4], want: [f32; 3], tol: f32) -> bool {
    (0..3).all(|k| (got[k] - want[k]).abs() < tol)
}

#[cfg(feature = "cycles")]
#[test]
fn dissolve_and_stencil_render_as_measured() {
    // [measured] Dissolve 25 % → (0.60, 0.30, 0.15); 75 % → (0.20, 0.10,
    // 0.05) over black.
    for (v, want) in [(0.25, [0.6, 0.3, 0.15]), (0.75, [0.2, 0.1, 0.05])] {
        let mut scene = unlit_patch();
        insert(
            &mut scene.shader_tree,
            3,
            None,
            constant(TextureEffect::Dissolve, v),
        );
        let got = centre(&scene, 6.0, |_| {});
        assert!(near(got, want, 0.03), "dissolve {v}: {got:?}");
    }
    // Stencil 49 % leaves it, 51 % removes the whole surface.
    let stencil = |v| {
        let mut scene = unlit_patch();
        insert(
            &mut scene.shader_tree,
            3,
            None,
            constant(TextureEffect::Stencil, v),
        );
        centre(&scene, 6.0, |_| {})
    };
    assert!(near(stencil(0.49), [0.8, 0.4, 0.2], 0.03));
    assert!(near(stencil(0.51), [0.0; 3], 0.01));
}

#[cfg(feature = "cycles")]
#[test]
fn fog_blends_toward_the_fog_colour() {
    // The box's front face is 5 m from the camera.
    let fogged = |kind: FogType| {
        let mut scene = unlit_patch();
        let s = shader_mut(&mut scene.shader_tree, 3);
        s.more.fog_type = kind;
        centre(&scene, 5.5, |_| {})
    };
    let mix = |f: f32| [0.8, 0.4, 0.2].map(|c| c + (0.5 - c) * f);
    let got = fogged(FogType::Linear);
    assert!(near(got, mix(0.5), 0.03), "linear: {got:?}");
    let got = fogged(FogType::Exponential);
    assert!(near(got, mix(0.393), 0.03), "exponential: {got:?}");
    // Layered at height 0 is Exponential (the face's centre is at y 0).
    let got = fogged(FogType::Layered);
    assert!(near(got, mix(0.393), 0.03), "layered: {got:?}");
    let got = fogged(FogType::None);
    assert!(near(got, [0.8, 0.4, 0.2], 0.03), "none: {got:?}");
}

#[cfg(feature = "cycles")]
#[test]
fn constant_alpha_keeps_the_surface_but_sets_alpha() {
    let mut scene = unlit_patch();
    let s = shader_mut(&mut scene.shader_tree, 3);
    s.alpha_type = operandi_core::shader_tree::AlphaType::Constant;
    s.alpha_value = 0.4;
    let got = centre(&scene, 6.0, |s| s.transparent = true);
    assert!((got[3] - 0.4).abs() < 0.03, "alpha {got:?}");
    // The colour is held out with it (premultiplied), not see-through.
    assert!(near(got, [0.32, 0.16, 0.08], 0.03), "colour {got:?}");
}

/// A lit wall facing the camera at z = 0, a point light up and to the
/// right, and a small box between them that shadows the wall's centre.
#[cfg(feature = "cycles")]
fn shadow_scene() -> (Scene, u64) {
    let mut scene = Scene::default();
    let wall = add_box(
        &mut scene,
        DVec3::new(0.0, 0.0, -0.1),
        DVec3::new(4.0, 4.0, 0.2),
    );
    add_box(&mut scene, DVec3::new(1.5, 0.0, 1.5), DVec3::splat(0.8));
    let mut light = LightSettings::new(LightKind::Point);
    light.intensity = 20.0;
    let id = scene
        .add_light(
            light,
            operandi_core::scene::Transform {
                translation: DVec3::new(3.0, 0.0, 3.0),
                ..Default::default()
            },
        )
        .unwrap();
    let _ = wall;
    // Not the studio clay, so every render uses the tree's own surface.
    base_material(&mut scene).diffuse_color = [0.7, 0.7, 0.7];
    (scene, id)
}

#[cfg(feature = "cycles")]
fn wall_centre(scene: &Scene) -> f32 {
    centre(scene, 8.0, |s| *s = s.clone().with_scene_lights(scene))[1]
}

#[cfg(feature = "cycles")]
#[test]
fn receive_shadows_off_lights_as_if_unblocked() {
    let (mut scene, _) = shadow_scene();
    let shadowed = wall_centre(&scene);
    shader_mut(&mut scene.shader_tree, 3).receive_shadows = false;
    let lit = wall_centre(&scene);
    assert!(lit > 0.02 && shadowed < 0.3 * lit, "{shadowed} {lit}");
}

#[cfg(feature = "cycles")]
#[test]
fn direct_multiplier_and_light_linking_render() {
    // No blocker for these: light the wall straight on.
    let (mut scene, light) = shadow_scene();
    scene
        .items
        .retain(|i| !(i.is_mesh() && i.mesh.vertices.iter().all(|v| v.z > 0.5)));
    let full = wall_centre(&scene);
    shader_mut(&mut scene.shader_tree, 3).more.direct_multiplier = 0.5;
    let half = wall_centre(&scene);
    assert!(full > 0.02, "lit: {full}");
    assert!((half / full - 0.5).abs() < 0.1, "{half} of {full}");
    // Exclude a group holding the light: no light on the wall.
    shader_mut(&mut scene.shader_tree, 3).more.direct_multiplier = 1.0;
    let group = scene.new_group(Some("Key"), &[light]).unwrap();
    shader_mut(&mut scene.shader_tree, 3).more.light_group = Some(group);
    assert!(wall_centre(&scene) < 0.1 * full);
    // Include: lit again.
    shader_mut(&mut scene.shader_tree, 3).more.link_mode = LinkMode::Include;
    assert!(wall_centre(&scene) > 0.8 * full);
}

#[test]
fn shader_rules_give_their_surfaces_sets_and_light_copies() {
    // Two boxes; a Material Group for the left one holds a Shader that
    // halves direct light and ignores shadows.
    let mut scene = Scene::default();
    let left = add_box(&mut scene, DVec3::new(-1.0, 0.0, 0.0), DVec3::ONE);
    let right = add_box(&mut scene, DVec3::new(1.0, 0.0, 0.0), DVec3::ONE);
    let mut light = LightSettings::new(LightKind::Point);
    light.intensity = 10.0;
    let lamp = scene
        .add_light(light, operandi_core::scene::Transform::default())
        .unwrap();
    // No shader rules and no linked lights: no sets at all.
    assert_eq!(link_sets(&scene), LinkSets::default());
    let tree = &mut scene.shader_tree;
    let group = insert(
        tree,
        0,
        None,
        NodeKind::Group(GroupMask {
            item: ItemMask::Item(left),
            ..Default::default()
        }),
    );
    let shader = insert(
        tree,
        1,
        Some(group),
        NodeKind::Shader(ShaderChannels::default()),
    );
    {
        let s = shader_mut(tree, shader);
        s.more.direct_multiplier = 0.5;
        s.receive_shadows = false;
    }
    let sets = link_sets(&scene);
    let rule = ShaderLightRule {
        link: None,
        direct: 0.5,
        receive_shadows: false,
    };
    let special = sets.set_for(left, &rule);
    assert_ne!(special, sets.set_for(right, &ShaderLightRule::default()));
    // The plain light no longer lights the left box's surfaces; a half
    // strength copy does, and nothing shadows that copy.
    let lights = scene_lights(&scene);
    assert_eq!(lights.len(), 2);
    let (plain, copy) = (&lights[0], &lights[1]);
    assert_eq!(plain.light_membership.unwrap() & (1 << special), 0);
    assert_eq!(copy.light_membership, Some(1 << special));
    assert_eq!(copy.shadow_membership, Some(0));
    assert!((copy.strength[0] - 0.5 * plain.strength[0]).abs() < 1e-6);
    // The meshes carry those sets.
    let meshes = scene_meshes(&scene).unwrap();
    assert!(meshes.iter().any(|m| m.light_set == special));
    assert!(meshes.iter().any(|m| m.light_set != special));
    let _ = lamp;
}

#[test]
fn constant_alpha_and_fog_reach_the_compiled_surface() {
    let mut scene = unlit_patch();
    {
        let s = shader_mut(&mut scene.shader_tree, 3);
        s.alpha_type = operandi_core::shader_tree::AlphaType::Constant;
        s.alpha_value = 0.25;
        s.more.fog_type = FogType::Underwater;
        s.more.fog_use_environment = true;
    }
    let meshes = scene_meshes(&scene).unwrap();
    let look = meshes[0].shader.as_ref().unwrap().look;
    assert_eq!(look.holdout, 0.75);
    let fog = look.fog.unwrap();
    assert_eq!((fog.kind, fog.use_environment), (4, true));
    // Use Environment Color only matters underwater.
    shader_mut(&mut scene.shader_tree, 3).more.fog_type = FogType::Linear;
    let meshes = scene_meshes(&scene).unwrap();
    let fog = meshes[0].shader.as_ref().unwrap().look.fog.unwrap();
    assert_eq!((fog.kind, fog.use_environment), (1, false));
}
