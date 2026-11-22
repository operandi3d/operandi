//! Node editing in Cycles: a free MaterialX node wired into a material
//! channel renders through the surface's node list (NE-03, NE-T1).
// The render helpers are only used with the library (`--features cycles`).
#![cfg_attr(not(feature = "cycles"), allow(dead_code, unused_imports))]
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::node_view::{NodeEdit, Target};
use operandi_core::shader_tree::{Drop, NewLayer, NodeKind};

/// The Base Material (layer 4).
const MATERIAL: u64 = 4;

/// A square at z = 0.5 facing the camera, its Base Material (glowing red,
/// diffuse black) in a Material Group; the group's id.
fn square() -> (Scene, u64) {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    let tree = &mut scene.shader_tree;
    if let Some(NodeKind::Material(m)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == MATERIAL)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_color = [1., 0., 0.];
        m.luminous_intensity = 1.;
    }
    let group = tree.add(NewLayer::Group, None).unwrap();
    tree.move_to(
        &[MATERIAL],
        Drop {
            parent: Some(group),
            index: 0,
        },
    )
    .unwrap();
    tree.validate().unwrap();
    (scene, group)
}

fn luminous() -> Target {
    Target::Layer {
        layer: MATERIAL,
        channel: "luminous_color".into(),
    }
}

/// A Noise node wired into the glow.
fn wire_noise(scene: &mut Scene, group: u64) {
    let tree = &mut scene.shader_tree;
    tree.edit_nodes(
        group,
        NodeEdit::Add {
            category: "noise3d".into(),
            ty: "color3".into(),
            at: [0.; 2],
        },
    )
    .unwrap();
    tree.edit_nodes(
        group,
        NodeEdit::SetValue {
            node: "Noise".into(),
            input: "amplitude".into(),
            value: "4".into(),
            continuing: false,
        },
    )
    .unwrap();
    tree.edit_nodes(
        group,
        NodeEdit::Connect {
            from: "Noise".into(),
            output: "out".into(),
            to: luminous(),
        },
    )
    .unwrap();
}

fn shader_of(scene: &Scene) -> RenderShader {
    scene_meshes(scene).unwrap()[0].shader.clone().unwrap()
}

#[test]
fn ne_03_a_wired_channel_reads_the_free_nodes_output() {
    let (mut scene, group) = square();
    let before = shader_of(&scene);
    assert!(before.node_graph.is_none());
    wire_noise(&mut scene, group);
    let sh = shader_of(&scene);
    assert!(sh.is_valid());
    let g = sh
        .node_graph
        .as_ref()
        .expect("the free nodes join the node list");
    assert!(g.graph.nodes.iter().any(|n| n.cycles == "noise_texture"));
    assert!(g.graph.nodes.iter().all(|n| n.cycles != "output"));
    let node = sh.ops.iter().find(|o| o.op == ShaderOp::NODE).unwrap();
    assert!((node.a as usize) < g.graph.nodes.len());
    // Disconnected, the surface compiles exactly as before.
    scene
        .shader_tree
        .edit_nodes(group, NodeEdit::Disconnect(luminous()))
        .unwrap();
    assert_eq!(shader_of(&scene), before);
}

#[cfg(feature = "cycles")]
fn picture(scene: &Scene, size: u32) -> Vec<[f32; 3]> {
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        size,
        size,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.15),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    img.rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect()
}

#[cfg(feature = "cycles")]
#[test]
fn ne_t1_wiring_noise_changes_the_render_and_unwiring_restores_it() {
    let (mut scene, group) = square();
    let original = picture(&scene, 16);
    wire_noise(&mut scene, group);
    let wired = picture(&scene, 16);
    // The glow now follows the noise: it varies over the square and isn't
    // the flat red any more.
    let inner = |px: &[[f32; 3]]| -> Vec<[f32; 3]> {
        (4..12)
            .flat_map(|y| (4..12).map(move |x| (x, y)))
            .map(|(x, y)| px[y * 16 + x])
            .collect()
    };
    let greens: Vec<f32> = inner(&wired).iter().map(|p| p[1]).collect();
    let (lo, hi) = greens
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &g| (a.min(g), b.max(g)));
    assert!(hi - lo > 0.05, "flat: {lo}..{hi}");
    assert_ne!(inner(&wired), inner(&original));
    // Removing the wire: pixel-identical to the original.
    scene
        .shader_tree
        .edit_nodes(group, NodeEdit::Disconnect(luminous()))
        .unwrap();
    assert_eq!(picture(&scene, 16), original);
}

/// A Constant texture layer over the Base Material driving its glow
/// colour (red, as the material's own); its id.
fn glow_texture(scene: &mut Scene) -> u64 {
    use operandi_core::shader_tree::{TextureEffect, TextureSource};
    let tree = &mut scene.shader_tree;
    let id = tree.add(NewLayer::Constant, Some(MATERIAL)).unwrap();
    let mut node = tree.get(id).unwrap().clone();
    if let NodeKind::Texture(t) = &mut node.kind {
        t.effect = TextureEffect::LuminousColor;
        t.source = TextureSource::Constant {
            color: [1., 0., 0.],
            value: 1.,
        };
    }
    tree.replace(node).unwrap();
    id
}

fn wire_noise_into(scene: &mut Scene, group: u64, to: Target) {
    wire_noise(scene, group);
    let tree = &mut scene.shader_tree;
    tree.edit_nodes(group, NodeEdit::Disconnect(luminous()))
        .unwrap();
    tree.edit_nodes(
        group,
        NodeEdit::Connect {
            from: "Noise".into(),
            output: "out".into(),
            to,
        },
    )
    .unwrap();
}

#[test]
fn ne_03_a_wired_texture_setting_reads_the_free_nodes_output() {
    let (mut scene, group) = square();
    let texture = glow_texture(&mut scene);
    assert_eq!(scene.shader_tree.material_group_of(texture), Some(group));
    let before = shader_of(&scene);
    let to = Target::Layer {
        layer: texture,
        channel: "texture.color".into(),
    };
    wire_noise_into(&mut scene, group, to.clone());
    let sh = shader_of(&scene);
    assert!(sh.is_valid());
    assert!(sh.ops.iter().any(|o| o.op == ShaderOp::NODE));
    scene
        .shader_tree
        .edit_nodes(group, NodeEdit::Disconnect(to))
        .unwrap();
    assert_eq!(shader_of(&scene), before);
}

#[cfg(feature = "cycles")]
#[test]
fn ne_t1_texture_setting_wired_noise_changes_the_render_and_unwiring_restores_it() {
    let (mut scene, group) = square();
    let texture = glow_texture(&mut scene);
    let original = picture(&scene, 16);
    let to = Target::Layer {
        layer: texture,
        channel: "texture.color".into(),
    };
    wire_noise_into(&mut scene, group, to.clone());
    let wired = picture(&scene, 16);
    let greens: Vec<f32> = (4..12)
        .flat_map(|y| (4..12).map(move |x| y * 16 + x))
        .map(|i| wired[i][1])
        .collect();
    let (lo, hi) = greens
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &g| (a.min(g), b.max(g)));
    assert!(hi - lo > 0.05, "flat: {lo}..{hi}");
    scene
        .shader_tree
        .edit_nodes(group, NodeEdit::Disconnect(to))
        .unwrap();
    assert_eq!(
        picture(&scene, 16),
        original,
        "pixel-identical once unwired"
    );
}

// -- More texture layers take wires (NE-03): Gradient Input, Occlusion zones.

/// A texture layer over the Base Material driving its glow, set up by `f`;
/// its id.
fn glow_layer(
    scene: &mut Scene,
    new: NewLayer,
    f: impl FnOnce(&mut operandi_core::shader_tree::TextureSource),
) -> u64 {
    use operandi_core::shader_tree::TextureEffect;
    let tree = &mut scene.shader_tree;
    let id = tree.add(new, Some(MATERIAL)).unwrap();
    let mut node = tree.get(id).unwrap().clone();
    if let NodeKind::Texture(t) = &mut node.kind {
        t.effect = TextureEffect::LuminousColor;
        f(&mut t.source);
    }
    tree.replace(node).unwrap();
    id
}

/// A black-to-white Gradient driving the glow.
fn glow_gradient(scene: &mut Scene) -> u64 {
    use operandi_core::gradient::{Curve, Key};
    use operandi_core::shader_tree::TextureSource;
    glow_layer(scene, NewLayer::Gradient, |s| {
        if let TextureSource::Gradient(g) = s {
            g.color = Curve::single([0., 0., 0., 1.]);
            g.color.keys.push(Key {
                at: 1.,
                ..g.color.keys[0]
            });
            g.color.keys[1].out = [1., 1., 1., 1.];
        }
    })
}

/// A Noise of type `ty` (amplitude 4) wired into `to`.
fn wire_typed_noise(scene: &mut Scene, group: u64, ty: &str, to: Target) {
    let tree = &mut scene.shader_tree;
    tree.edit_nodes(
        group,
        NodeEdit::Add {
            category: "noise3d".into(),
            ty: ty.into(),
            at: [0.; 2],
        },
    )
    .unwrap();
    let name = tree
        .group_nodes(group)
        .unwrap()
        .free
        .last()
        .unwrap()
        .name
        .clone();
    tree.edit_nodes(
        group,
        NodeEdit::SetValue {
            node: name.clone(),
            input: "amplitude".into(),
            value: "4".into(),
            continuing: false,
        },
    )
    .unwrap();
    tree.edit_nodes(
        group,
        NodeEdit::Connect {
            from: name,
            output: "out".into(),
            to,
        },
    )
    .unwrap();
}

fn setting(layer: u64, key: &str) -> Target {
    Target::Layer {
        layer,
        channel: key.into(),
    }
}

#[test]
fn ne_03_gradient_input_and_occlusion_zone_wires_join_the_node_list() {
    for (new, ty, key) in [
        (NewLayer::Gradient, "float", "texture.gradient_input"),
        (NewLayer::Occlusion, "color3", "texture.zone2_color"),
    ] {
        let (mut scene, group) = square();
        let layer = if new == NewLayer::Gradient {
            glow_gradient(&mut scene)
        } else {
            glow_layer(&mut scene, new, |_| {})
        };
        let before = shader_of(&scene);
        wire_typed_noise(&mut scene, group, ty, setting(layer, key));
        let sh = shader_of(&scene);
        assert!(sh.is_valid(), "{key}");
        assert!(sh.ops.iter().any(|o| o.op == ShaderOp::NODE), "{key}");
        scene
            .shader_tree
            .edit_nodes(group, NodeEdit::Disconnect(setting(layer, key)))
            .unwrap();
        assert_eq!(shader_of(&scene), before, "{key}: unwired, as before");
    }
}

#[cfg(feature = "cycles")]
#[test]
fn ne_t1_gradient_input_and_occlusion_zone_wires_change_the_render() {
    for (new, ty, key) in [
        (NewLayer::Gradient, "float", "texture.gradient_input"),
        (NewLayer::Occlusion, "color3", "texture.zone2_color"),
    ] {
        let (mut scene, group) = square();
        let layer = if new == NewLayer::Gradient {
            glow_gradient(&mut scene)
        } else {
            glow_layer(&mut scene, new, |_| {})
        };
        let original = picture(&scene, 16);
        wire_typed_noise(&mut scene, group, ty, setting(layer, key));
        let wired = picture(&scene, 16);
        let greens: Vec<f32> = (4..12)
            .flat_map(|y| (4..12).map(move |x| y * 16 + x))
            .map(|i| wired[i][1])
            .collect();
        let (lo, hi) = greens
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &g| (a.min(g), b.max(g)));
        assert!(hi - lo > 0.05, "{key} flat: {lo}..{hi}");
        scene
            .shader_tree
            .edit_nodes(group, NodeEdit::Disconnect(setting(layer, key)))
            .unwrap();
        assert_eq!(
            picture(&scene, 16),
            original,
            "{key}: pixel-identical once unwired"
        );
    }
}
