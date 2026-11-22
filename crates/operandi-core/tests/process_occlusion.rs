//! Shader tree remaining, stage 4: Process layer and Occlusion texture.
use operandi_core::process_occlusion::{Occlusion, OcclusionType, Process};
use operandi_core::shader_eval::{Ch, Context, Expr};
use operandi_core::shader_tree::{
    Layer, Node, NodeId, NodeKind, ShaderTree, TextureEffect, TextureLayer, TextureSource,
};

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

fn process(f: impl FnOnce(&mut Process)) -> TextureSource {
    let mut p = Process::default();
    f(&mut p);
    TextureSource::Process(Box::new(p))
}

fn colour(tree: &ShaderTree) -> [f64; 3] {
    tree.evaluate(&ctx())
        .get(Ch::DiffuseColor)
        .constant()
        .expect("a constant")
}

fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
}

fn blue_patch() -> ShaderTree {
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        TextureSource::Constant {
            color: [0.25, 0.5, 0.75],
            value: 1.,
        },
    );
    tree
}

#[test]
fn process_adjusts_the_layer_directly_below() {
    let mut tree = blue_patch();
    on_top(&mut tree, TextureEffect::DiffuseColor, process(|_| {}));
    assert!(near(colour(&tree), [0.25, 0.5, 0.75]), "starting values");
    let mut tree = blue_patch();
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        process(|p| p.hue = 0.25),
    );
    assert!(near(colour(&tree), [0.4375, 0.25, 0.75]));
    // Its own opacity mixes adjusted and unadjusted.
    let mut tree = blue_patch();
    let id = on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        process(|p| p.value = 0.25),
    );
    tree.nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .layer
        .opacity = 0.5;
    assert!(near(colour(&tree), [0.28125, 0.5625, 0.84375]));
}

#[test]
fn process_with_nothing_below_does_nothing() {
    // PQ-3: directly above a material, or above a texture of another
    // effect, it changes nothing.
    let mut tree = ShaderTree::default();
    let before = tree.evaluate(&ctx()).channels;
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        process(|p| p.value = 0.5),
    );
    assert_eq!(tree.evaluate(&ctx()).channels, before);
    let mut tree = blue_patch();
    let before = tree.evaluate(&ctx()).channels;
    on_top(
        &mut tree,
        TextureEffect::Roughness,
        process(|p| p.value = 0.5),
    );
    assert_eq!(tree.evaluate(&ctx()).channels, before);
}

#[test]
fn process_over_an_image_builds_hsv_and_ramps() {
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        TextureSource::ImageMap {
            image: Some(3),
            settings: Box::default(),
        },
    );
    on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        process(|p| {
            p.bias = 0.7;
            p.hue_shift = 90.;
        }),
    );
    let e = format!("{:?}", tree.evaluate(&ctx()).get(Ch::DiffuseColor));
    assert!(e.contains("Hsv") && e.contains("Ramp"), "{e}");
}

#[test]
fn occlusion_is_a_two_zone_over_openness() {
    let mut tree = ShaderTree::default();
    let id = on_top(
        &mut tree,
        TextureEffect::DiffuseColor,
        TextureSource::Occlusion(Box::default()),
    );
    let Expr::Ramp { input, stops, .. } = tree.evaluate(&ctx()).get(Ch::DiffuseColor).clone()
    else {
        panic!()
    };
    assert_eq!(
        *input,
        Expr::Occlusion {
            rays: 24,
            distance: 0.2,
            same_surface: false,
            inside: false,
            toward: operandi_core::shader_eval::AoToward::Normal,
        }
    );
    assert_eq!(stops[0], [0., 0., 0., 1.]);
    // Thickness fires inside; the others render as Uniform (PQ-4).
    for (kind, inside) in [(OcclusionType::Thickness, true), (OcclusionType::Up, false)] {
        for n in &mut tree.nodes {
            if n.id == id
                && let NodeKind::Texture(t) = &mut n.kind
                && let TextureSource::Occlusion(o) = &mut t.source
            {
                o.kind = kind;
            }
        }
        let e = format!("{:?}", tree.evaluate(&ctx()).get(Ch::DiffuseColor));
        assert!(e.contains(&format!("inside: {inside}")), "{e}");
    }
    // Rays out of range are refused.
    let mut bad = tree.clone();
    for n in &mut bad.nodes {
        if let NodeKind::Texture(t) = &mut n.kind
            && let TextureSource::Occlusion(o) = &mut t.source
        {
            o.rays = 0;
        }
    }
    assert!(bad.validate().is_err());
}

#[test]
fn process_and_occlusion_save() {
    let mut scene = operandi_core::Scene::default();
    on_top(
        &mut scene.shader_tree,
        TextureEffect::DiffuseColor,
        TextureSource::Occlusion(Box::new(Occlusion {
            kind: OcclusionType::Thickness,
            rays: 64,
            ..Default::default()
        })),
    );
    on_top(
        &mut scene.shader_tree,
        TextureEffect::DiffuseColor,
        process(|p| p.saturation = -1.),
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

/// GR-Q10: XYZ Distance to Locator is the largest of |x|, |y|, |z| (the
/// world origin's frame without a locator); Locator Incidence the angle
/// to the locator; Thickness reads the inside rays; none is stored only.
#[test]
fn new_gradient_inputs_evaluate() {
    use operandi_core::gradient::{Gradient, GradientInput};
    use operandi_core::shader_eval::ShadingPoint;
    for input in [
        GradientInput::XyzDistanceToLocator,
        GradientInput::LocatorIncidence,
        GradientInput::Thickness,
        GradientInput::BumplessIncident,
    ] {
        assert!(!input.is_stored_only(), "{input:?}");
    }
    let value_at = |input, p: [f64; 3], n: [f64; 3]| {
        let mut tree = ShaderTree::default();
        let mut g = Gradient {
            input,
            ..Default::default()
        };
        // Value = the input itself over 0..10.
        g.color = operandi_core::gradient::Curve {
            keys: vec![
                operandi_core::gradient::Key {
                    at: 0.,
                    out: [0., 0., 0., 1.],
                    shape: operandi_core::gradient::KeyShape::Linear,
                },
                operandi_core::gradient::Key {
                    at: 180.,
                    out: [180., 180., 180., 1.],
                    shape: operandi_core::gradient::KeyShape::Linear,
                },
            ],
        };
        on_top(
            &mut tree,
            TextureEffect::DiffuseColor,
            TextureSource::Gradient(Box::new(g)),
        );
        tree.evaluate(&ctx()).get(Ch::DiffuseColor).eval_at(
            &ShadingPoint {
                position: p,
                uv: [0.; 2],
                normal: n,
            },
            &|_, _| [0.; 4],
        )[0]
    };
    let d = value_at(
        GradientInput::XyzDistanceToLocator,
        [0.3, -0.7, 0.2],
        [0., 1., 0.],
    );
    assert!((d - 0.7).abs() < 0.01, "{d}");
    let a = value_at(GradientInput::LocatorIncidence, [0., 2., 0.], [0., -1., 0.]);
    assert!(a.abs() < 0.5, "facing the origin: {a}");
    let a = value_at(GradientInput::LocatorIncidence, [0., 2., 0.], [1., 0., 0.]);
    assert!((a - 90.).abs() < 0.5, "side-on: {a}");
}
