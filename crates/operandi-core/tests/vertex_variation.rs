//! Shader tree remaining, stage 3: Vertex Map Texture and Variation
//! Texture (gradient family v1 §3–4).
use operandi_core::gradient::{Curve, Key, KeyShape};
use operandi_core::shader_eval::{Ch, Context, Expr};
use operandi_core::shader_tree::{
    Layer, NewLayer, Node, NodeId, NodeKind, ShaderTree, TextureEffect, TextureLayer,
    TextureSource, TwoZone,
};
use operandi_core::vertex_map_texture::{Variation, VariationSource, VertexMapKind, VertexMapRef};

fn ctx() -> Context<'static> {
    Context {
        item: 1,
        ancestors: &[],
        material: "Default",
        part: None,
        sets: &[],
        locators: &[],
    }
}

fn on_top(tree: &mut ShaderTree, effect: TextureEffect, source: TextureSource) -> NodeId {
    let id = tree.next_id;
    tree.next_id += 1;
    let at = tree.nodes.iter().position(|n| n.id == 3).unwrap() + 1;
    tree.nodes.insert(
        at,
        Node {
            extras: Default::default(),
            id,
            parent: None,
            name: format!("Layer {id}"),
            layer: Layer::default(),
            kind: NodeKind::Texture(TextureLayer {
                source,
                effect,
                locator: None,
                legacy_projection: None,
                legacy_axis: None,
                ignore_scale_group: false,
                node_input: None,
            }),
        },
    );
    tree.validate().unwrap();
    id
}

fn map(kind: VertexMapKind) -> Option<VertexMapRef> {
    Some(VertexMapRef {
        name: "Paint".into(),
        kind,
    })
}

#[test]
fn starting_values_and_no_map() {
    let mut tree = ShaderTree::default();
    let before = tree.evaluate(&ctx());
    let id = tree.add(NewLayer::VertexMap, Some(4)).unwrap();
    match &tree.get(id).unwrap().kind {
        NodeKind::Texture(TextureLayer {
            source: TextureSource::VertexMap { map, zones },
            effect,
            ..
        }) => {
            assert_eq!(*map, None);
            assert_eq!(*zones, TwoZone::default());
            assert_eq!(*effect, TextureEffect::DiffuseColor);
            let z = TwoZone::default();
            assert_eq!(
                (z.zone1_value, z.zone1_color, z.zone2_value, z.zone2_color),
                (0., [0.; 3], 1., [1.; 3])
            );
        }
        other => panic!("{other:?}"),
    }
    // No map chosen: the layer does nothing.
    assert_eq!(tree.evaluate(&ctx()).channels, before.channels);
}

#[test]
fn a_weight_map_picks_between_the_zones() {
    let mut tree = ShaderTree::default();
    let id = on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        TextureSource::VertexMap {
            map: map(VertexMapKind::Weight),
            zones: TwoZone::default(),
        },
    );
    let s = tree.evaluate(&ctx());
    let Expr::Mix { fac, b, .. } = s.get(Ch::DiffuseColor) else {
        panic!("{:?}", s.get(Ch::DiffuseColor))
    };
    // Coverage: the map's presence (alpha) on this mesh.
    assert!(format!("{fac:?}").contains("weight:Paint"));
    let Expr::Ramp {
        input,
        stops,
        lo,
        hi,
        ..
    } = &**b
    else {
        panic!("{b:?}")
    };
    assert_eq!((*lo, *hi), (0., 1.));
    assert_eq!(stops[0], [0., 0., 0., 1.]);
    assert_eq!(stops[stops.len() - 1], [1., 1., 1., 1.]);
    assert_eq!(
        **input,
        Expr::channel(
            Expr::Attribute {
                name: "weight:Paint".into(),
                alpha: false
            },
            0
        )
    );
    // Invert flips the weight before the zones.
    tree.nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .layer
        .invert = true;
    let s = tree.evaluate(&ctx());
    assert!(format!("{:?}", s.get(Ch::DiffuseColor)).contains("Subtract"));
}

#[test]
fn a_colour_map_is_used_as_it_is() {
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        TextureSource::VertexMap {
            map: map(VertexMapKind::Color),
            zones: TwoZone::default(),
        },
    );
    let s = tree.evaluate(&ctx());
    let Expr::Mix { fac, b, .. } = s.get(Ch::DiffuseColor) else {
        panic!()
    };
    assert_eq!(
        **b,
        Expr::Attribute {
            name: "color:Paint".into(),
            alpha: false
        }
    );
    assert!(format!("{fac:?}").contains("alpha: true"));
}

#[test]
fn variation_reads_a_number_per_element() {
    // Particle (the start): r = 0 everywhere, so a flat curve value.
    let mut tree = ShaderTree::default();
    let id = on_top(
        &mut tree,
        TextureEffect::Roughness,
        TextureSource::Variation(Box::new(Variation {
            value: Curve {
                keys: vec![
                    Key {
                        at: 0.,
                        out: 0.2,
                        shape: KeyShape::Stepped,
                    },
                    Key {
                        at: 0.75,
                        out: 0.9,
                        shape: KeyShape::Stepped,
                    },
                ],
            },
            ..Default::default()
        })),
    );
    assert_eq!(tree.evaluate(&ctx()).get(Ch::Roughness), &Expr::scalar(0.2));
    // Item: a ramp over the item's number.
    for n in &mut tree.nodes {
        if n.id == id
            && let NodeKind::Texture(t) = &mut n.kind
            && let TextureSource::Variation(v) = &mut t.source
        {
            v.source = VariationSource::Item;
        }
    }
    let r = format!("{:?}", tree.evaluate(&ctx()).get(Ch::Roughness));
    assert!(
        r.contains("Ramp") && r.contains("operandi:variation-item"),
        "{r}"
    );
}

#[test]
fn vertex_maps_and_variations_save_and_list() {
    let mut scene = operandi_core::Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = operandi_core::mesh::Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    item.mesh
        .attributes
        .point_weights
        .entry("Paint".into())
        .or_default()
        .insert(0, 0.5);
    item.mesh
        .attributes
        .point_colors
        .entry("Tint".into())
        .or_default()
        .insert(0, glam::DVec4::ONE);
    assert_eq!(
        scene.vertex_maps(),
        vec![
            VertexMapRef {
                name: "Paint".into(),
                kind: VertexMapKind::Weight
            },
            VertexMapRef {
                name: "Tint".into(),
                kind: VertexMapKind::Color
            },
        ]
    );
    on_top(
        &mut scene.shader_tree,
        TextureEffect::DiffuseColor,
        TextureSource::VertexMap {
            map: map(VertexMapKind::Weight),
            zones: TwoZone {
                bias: 0.7,
                ..Default::default()
            },
        },
    );
    on_top(
        &mut scene.shader_tree,
        TextureEffect::LayerMask,
        TextureSource::Variation(Box::new(Variation {
            source: VariationSource::MeshPart,
            ..Default::default()
        })),
    );
    // A pre-OpenPBR material, so the older version is what is tested.
    scene.shader_tree.legacy_openpbr();
    let bytes = operandi_core::persistence::encode(&scene).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"version\": 26")
    );
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    let sources = |t: &ShaderTree| -> Vec<TextureSource> {
        t.nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Texture(t) => Some(t.source.clone()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(sources(&back.shader_tree), sources(&scene.shader_tree));
}
