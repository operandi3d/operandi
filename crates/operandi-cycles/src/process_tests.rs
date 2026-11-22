//! Shader tree remaining, stage 4 in Cycles: Process (HSV) and Occlusion.
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::gradient::{Curve, Gradient, GradientInput, Key, KeyShape};
use operandi_core::mesh::Mesh;
use operandi_core::process_occlusion::{Occlusion, Process};
use operandi_core::shader_tree::{Layer, Node, TextureEffect, TextureLayer, TextureSource};

/// A glowing square facing the camera with `layers` (bottom first) on its
/// Luminous Color.
fn patch(layers: Vec<TextureSource>) -> Scene {
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
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_intensity = 1.;
    }
    for source in layers {
        let id = tree.next_id;
        tree.next_id += 1;
        tree.nodes.insert(
            3,
            Node {
                extras: Default::default(),
                id,
                parent: None,
                name: format!("Layer {id}"),
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
    }
    tree.validate().unwrap();
    scene
}

fn centre(scene: &Scene) -> [f32; 3] {
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
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 0.04)
}

#[test]
fn process_hue_shift_renders_as_measured() {
    // A colour that only the renderer knows (a ramp over incidence), so
    // the Process runs as HSV nodes, not folded in advance.
    let c = [0.25, 0.5, 0.75, 1.];
    let key = |at| Key {
        at,
        out: c,
        shape: KeyShape::Linear,
    };
    let gradient = TextureSource::Gradient(Box::new(Gradient {
        input: GradientInput::Incidence,
        color: Curve {
            keys: vec![key(0.), key(90.)],
        },
        ..Default::default()
    }));
    let shift = TextureSource::Process(Box::new(Process {
        hue_shift: 90.,
        ..Default::default()
    }));
    let got = centre(&patch(vec![gradient, shift]));
    assert!(near(got, [0.75, 0.25, 0.75]), "{got:?}");
}

#[test]
fn an_open_patch_is_zone_two() {
    let got = centre(&patch(vec![TextureSource::Occlusion(Box::new(
        Occlusion::default(),
    ))]));
    assert!(near(got, [1.; 3]), "{got:?}");
}

/// PQ-5: Up fires its rays toward +Y. A roof above the square shuts
/// them in (dark) though nothing faces the square; Uniform, fired round
/// the square's own normal (+Z), stays open. On a flat open square,
/// Concavity & Convexity reads its middle (½).
#[test]
fn occlusion_types_fire_their_own_way() {
    use operandi_core::process_occlusion::OcclusionType;
    let with_roof = |kind| {
        let mut scene = patch(vec![TextureSource::Occlusion(Box::new(Occlusion {
            kind,
            distance: 5.,
            rays: 64,
            ..Default::default()
        }))]);
        scene.new_mesh().unwrap();
        scene.items.last_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(0., 1.5, 0.), DVec3::new(6., 0.2, 6.), 1)
                .unwrap()
                .into();
        scene
    };
    let open = centre(&with_roof(OcclusionType::Uniform));
    let up = centre(&with_roof(OcclusionType::Up));
    // Uniform's hemisphere grazes the roof too, so it isn't fully open.
    assert!(open[0] > 0.6, "uniform {open:?}");
    assert!(up[0] < open[0] - 0.4, "up {up:?} vs uniform {open:?}");
    let flat = centre(&patch(vec![TextureSource::Occlusion(Box::new(
        Occlusion {
            kind: OcclusionType::ConcavityConvexity,
            ..Default::default()
        },
    ))]));
    assert!(flat[0] > 0.1 && flat[0] < 0.6, "flat {flat:?}");
}
