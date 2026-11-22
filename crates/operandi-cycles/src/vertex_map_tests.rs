//! Shader tree remaining, stage 3 in Cycles: Vertex Map and Variation
//! textures read per-corner attributes.
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::shader_tree::{
    Layer, Node, TextureEffect, TextureLayer, TextureSource, TwoZone,
};
use operandi_core::vertex_map_texture::{Variation, VariationSource, VertexMapKind, VertexMapRef};

/// A glowing square (Luminous Color, intensity 1) facing the camera, a
/// layer on its Luminous Color, and every point's weight `w` in map
/// "Paint" (no map with `None`).
fn patch(source: TextureSource, w: Option<f64>) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    if let Some(w) = w {
        let weights = item
            .mesh
            .attributes
            .point_weights
            .entry("Paint".into())
            .or_default();
        for p in 0..vertices.len() as u32 {
            weights.insert(p, w);
        }
    }
    let tree = &mut scene.shader_tree;
    if let Some(NodeKind::Material(m)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_intensity = 1.;
        m.luminous_color = [0.2, 0.4, 0.6];
    }
    let id = tree.next_id;
    tree.next_id += 1;
    tree.nodes.insert(
        3,
        Node {
            extras: Default::default(),
            id,
            parent: None,
            name: "Layer".into(),
            layer: Layer::default(),
            kind: NodeKind::Texture(TextureLayer {
                source,
                effect: TextureEffect::LuminousColor,
                locator: None,
                legacy_projection: None,
                legacy_axis: None,
                ignore_scale_group: false,
                node_input: None,
            }),
        },
    );
    tree.validate().unwrap();
    scene
}

fn weight() -> TextureSource {
    TextureSource::VertexMap {
        map: Some(VertexMapRef {
            name: "Paint".into(),
            kind: VertexMapKind::Weight,
        }),
        zones: TwoZone {
            zone1_color: [1., 0., 0.],
            zone2_color: [0., 0., 1.],
            ..Default::default()
        },
    }
}

#[test]
fn meshes_carry_the_attributes_their_shader_reads() {
    let scene = patch(weight(), Some(0.25));
    let meshes = scene_meshes(&scene).unwrap();
    let m = &meshes[0];
    let sh = m.shader.as_ref().unwrap();
    assert_eq!(sh.attributes, vec!["weight:Paint".to_string()]);
    assert_eq!(m.attributes.len(), m.triangles.len() * 3);
    assert!(m.attributes.iter().all(|a| *a == [0.25, 0.25, 0.25, 1.0]));
    assert!(sh.is_valid());
    // Variation by item: the same number on every corner of one item.
    let scene = patch(
        TextureSource::Variation(Box::new(Variation {
            source: VariationSource::Item,
            color: operandi_core::gradient::Curve {
                keys: vec![
                    operandi_core::gradient::Key {
                        at: 0.,
                        out: [0.; 4],
                        shape: operandi_core::gradient::KeyShape::Linear,
                    },
                    operandi_core::gradient::Key {
                        at: 1.,
                        out: [1.; 4],
                        shape: operandi_core::gradient::KeyShape::Linear,
                    },
                ],
            },
            ..Default::default()
        })),
        None,
    );
    let m = &scene_meshes(&scene).unwrap()[0];
    let first = m.attributes[0];
    assert!(first[3] == 1.0 && m.attributes.iter().all(|a| *a == first));
}

#[cfg(feature = "cycles")]
#[test]
fn weight_maps_render_between_the_zones() {
    let centre = |scene: &Scene| {
        let eye = DVec3::new(0.0, 0.0, 5.5);
        let mut settings = Settings::studio(
            8,
            8,
            View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.05),
        );
        settings.sun_strength = 0.0;
        settings.world[0].intensity = 0.0;
        settings.samples = 16;
        settings.gpu = false;
        settings.denoise = false;
        let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
        let p = (4 * 8 + 4) * 4;
        [img.rgba[p], img.rgba[p + 1], img.rgba[p + 2]]
    };
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 0.04);
    let got = centre(&patch(weight(), Some(0.0)));
    assert!(near(got, [1., 0., 0.]), "weight 0: {got:?}");
    let got = centre(&patch(weight(), Some(1.0)));
    assert!(near(got, [0., 0., 1.]), "weight 1: {got:?}");
    let got = centre(&patch(weight(), Some(0.5)));
    assert!(near(got, [0.5, 0., 0.5]), "weight 0.5: {got:?}");
    // No such map on this mesh: the layer does nothing.
    let got = centre(&patch(weight(), None));
    assert!(near(got, [0.2, 0.4, 0.6]), "no map: {got:?}");
}
