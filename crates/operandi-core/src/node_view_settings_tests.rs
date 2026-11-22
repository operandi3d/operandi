//! More texture layers take wires (NE-03): Image Map, Gradient, weight
//! Vertex Map Texture, Occlusion, and those used as masks.
use super::*;
use crate::node_view::tests::{MATERIAL, add, editor, nodes};
use crate::node_view::{NodeEdit, Target};
use crate::shader_tree::{NewLayer, NodeId, ShaderTreeEdit, TextureEffect};
use crate::vertex_map_texture::{VertexMapKind, VertexMapRef};
use crate::{Command, Editor};

fn base_colour(e: &Editor, group: NodeId) -> crate::shader_eval::Expr {
    let p = crate::materialx::tree_standard::group_surface(&e.scene, group).unwrap();
    p.inputs
        .iter()
        .find(|(i, _)| *i == crate::shader_eval::Input::BaseColor)
        .unwrap()
        .1
        .clone()
}

fn reads_free_nodes(e: &Editor, group: NodeId) -> bool {
    format!("{:?}", base_colour(e, group)).contains("Nodes(")
}

/// A texture layer above the Base Material, its source changed by `f`,
/// its effect `effect`.
fn layer(
    e: &mut Editor,
    new: NewLayer,
    effect: TextureEffect,
    f: impl FnOnce(&mut TextureSource),
) -> NodeId {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: new,
        selected: Some(MATERIAL),
    }))
    .unwrap();
    let id = e.scene.shader_tree.next_id - 1;
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    let NodeKind::Texture(t) = &mut node.kind else {
        panic!("not a texture layer")
    };
    t.effect = effect;
    f(&mut t.source);
    e.scene.shader_tree.replace(node).unwrap();
    id
}

fn to(layer: NodeId, key: &str) -> Target {
    Target::Layer {
        layer,
        channel: key.into(),
    }
}

fn connect(e: &mut Editor, group: NodeId, from: &str, target: Target) -> crate::Result<()> {
    nodes(
        e,
        group,
        NodeEdit::Connect {
            from: from.into(),
            output: "out".into(),
            to: target,
        },
    )
}

/// Wires `from` into `key`, checks the surface reads it in one undo step
/// named for the setting, then unwires and checks the surface is back.
fn wires_and_unwires(e: &mut Editor, group: NodeId, from: &str, layer: NodeId, key: &str) {
    let own = base_colour(e, group);
    let steps = e.undo.len();
    connect(e, group, from, to(layer, key)).unwrap();
    assert_eq!(e.undo.len(), steps + 1, "one undo step per wire");
    let label = label(key).unwrap();
    assert_eq!(
        e.undo.last().unwrap().label,
        format!("Connect {from} > {label}")
    );
    assert!(
        reads_free_nodes(e, group),
        "{key}: {:?}",
        base_colour(e, group)
    );
    // The board shows the row driven (NE-04).
    let board = e.scene.shader_tree.board(group).unwrap();
    let b = board.get(&crate::node_view::layer_name(layer)).unwrap();
    assert!(b.input(key).unwrap().driven, "{key}");
    // Save and reopen keeps the wire and the surface.
    let bytes = crate::persistence::encode(&e.scene).unwrap();
    let back = crate::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree.node_views, e.scene.shader_tree.node_views);
    assert_eq!(
        crate::materialx::tree_standard::group_surface(&back, group).unwrap(),
        crate::materialx::tree_standard::group_surface(&e.scene, group).unwrap()
    );
    // Undo takes the wire away again: the same surface as before.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        base_colour(e, group),
        own,
        "{key}: unwired, the same surface"
    );
}

#[test]
fn ne_03_an_image_maps_adjustments_take_wires() {
    let (mut e, group) = editor();
    let image = layer(
        &mut e,
        NewLayer::ImageMap,
        TextureEffect::DiffuseColor,
        |s| {
            if let TextureSource::ImageMap { image, .. } = s {
                *image = Some(1);
            }
        },
    );
    let noise = add(&mut e, group, "noise3d", "float");
    for key in ["texture.gamma", "texture.brightness", "texture.contrast"] {
        wires_and_unwires(&mut e, group, &noise, image, key);
    }
    // The picture is a file, not a value: no input for it. Low and High
    // Value only belong to scalar effects.
    for key in ["texture.image", "texture.low_value", "texture.color"] {
        assert!(
            connect(&mut e, group, &noise, to(image, key)).is_err(),
            "{key}"
        );
    }
    let NodeKind::Texture(t) = &e.scene.shader_tree.get(image).unwrap().kind else {
        panic!()
    };
    let mut scalar = t.clone();
    scalar.effect = TextureEffect::DiffuseAmount;
    let keys: Vec<&str> = settings(&scalar).into_iter().map(|s| s.0).collect();
    assert!(keys.contains(&"texture.low_value") && keys.contains(&"texture.high_value"));
}

#[test]
fn ne_03_a_gradients_input_takes_a_wire() {
    let (mut e, group) = editor();
    let gradient = layer(
        &mut e,
        NewLayer::Gradient,
        TextureEffect::DiffuseColor,
        |s| {
            if let TextureSource::Gradient(g) = s {
                // Two keys, so the looked-up number shows.
                g.color = crate::gradient::Curve::single([0., 0., 0., 1.]);
                g.color.keys.push(crate::gradient::Key {
                    at: 1.,
                    ..g.color.keys[0]
                });
                g.color.keys[1].out = [1., 1., 1., 1.];
            }
        },
    );
    let noise = add(&mut e, group, "noise3d", "float");
    wires_and_unwires(&mut e, group, &noise, gradient, "texture.gradient_input");
    // The Input has no number of its own: the box shows only a socket.
    let board = e.scene.shader_tree.board(group).unwrap();
    let b = board.get(&crate::node_view::layer_name(gradient)).unwrap();
    assert_eq!(
        b.input("texture.gradient_input").unwrap().value,
        crate::node_view_board::Value::None
    );
}

#[test]
fn ne_03_vertex_map_and_occlusion_zones_take_wires() {
    let (mut e, group) = editor();
    let weight = layer(
        &mut e,
        NewLayer::VertexMap,
        TextureEffect::DiffuseColor,
        |s| {
            if let TextureSource::VertexMap { map, .. } = s {
                *map = Some(VertexMapRef {
                    name: "Weight".into(),
                    kind: VertexMapKind::Weight,
                });
            }
        },
    );
    let noise = add(&mut e, group, "noise3d", "color3");
    wires_and_unwires(&mut e, group, &noise, weight, "texture.zone2_color");
    wires_and_unwires(&mut e, group, &noise, weight, "texture.zone1_color");
    let occlusion = layer(
        &mut e,
        NewLayer::Occlusion,
        TextureEffect::DiffuseColor,
        |_| {},
    );
    wires_and_unwires(&mut e, group, &noise, occlusion, "texture.zone1_color");
    // A colour vertex map's colours are the map: nothing to wire.
    let colour = layer(
        &mut e,
        NewLayer::VertexMap,
        TextureEffect::DiffuseColor,
        |s| {
            if let TextureSource::VertexMap { map, .. } = s {
                *map = Some(VertexMapRef {
                    name: "Colour".into(),
                    kind: VertexMapKind::Color,
                });
            }
        },
    );
    assert!(connect(&mut e, group, &noise, to(colour, "texture.zone2_color")).is_err());
}

#[test]
fn ne_03_a_mask_layers_settings_take_wires() {
    let (mut e, group) = editor();
    // New layers land on top: the mask first, then red over it.
    let mask = layer(
        &mut e,
        NewLayer::Occlusion,
        TextureEffect::LayerMask,
        |_| {},
    );
    layer(
        &mut e,
        NewLayer::Constant,
        TextureEffect::DiffuseColor,
        |s| {
            *s = TextureSource::Constant {
                color: [1., 0., 0.],
                value: 1.,
            }
        },
    );
    let noise = add(&mut e, group, "noise3d", "float");
    wires_and_unwires(&mut e, group, &noise, mask, "texture.zone2_value");
}

#[test]
fn zone_drives_match_the_own_ramp_when_the_wired_values_are_the_own() {
    // The factor-ramp form of a driven zone block gives the same numbers
    // as the colour ramp it replaces.
    let zones = crate::shader_tree::TwoZone {
        zone1_color: [0.2, 0.4, 0.6],
        zone2_color: [0.9, 0.1, 0.3],
        bias: 0.3,
        gain: 0.7,
        ..Default::default()
    };
    for t in [0., 0.13, 0.5, 0.77, 1.] {
        let (_, c, _) = zones.sample(t);
        let factor = crate::shader_tree::TwoZone {
            zone1_value: 0.,
            zone2_value: 1.,
            ..zones.clone()
        };
        let (w, _, _) = factor.sample(t);
        for (i, own) in c.iter().enumerate() {
            let (a, b) = (zones.zone1_color[i], zones.zone2_color[i]);
            assert!((a + (b - a) * w - own).abs() < 1e-12);
        }
    }
}
