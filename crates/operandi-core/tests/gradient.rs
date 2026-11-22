//! Shader tree remaining, stage 2: the Gradient layer and Driver A–D
//! (gradient family v1 §1–2).
use operandi_core::gradient::{Curve, Gradient, GradientInput, Key, KeyShape};
use operandi_core::shader_eval::{Ch, Context, Expr, ShadingPoint};
use operandi_core::shader_tree::{
    BlendMode, Layer, Node, NodeId, NodeKind, ShaderTree, TextureEffect, TextureLayer,
    TextureSource,
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

/// Puts a layer directly below the Base Shader (so above everything
/// already added there).
fn on_top(tree: &mut ShaderTree, kind: NodeKind) -> NodeId {
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
            kind,
        },
    );
    tree.validate().unwrap();
    id
}

fn texture(effect: TextureEffect, source: TextureSource) -> NodeKind {
    NodeKind::Texture(TextureLayer {
        source,
        effect,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

fn constant(effect: TextureEffect, value: f64) -> NodeKind {
    texture(
        effect,
        TextureSource::Constant {
            color: [value; 3],
            value,
        },
    )
}

fn linear(points: &[(f64, f64)]) -> Curve<f64> {
    Curve {
        keys: points
            .iter()
            .map(|&(at, out)| Key {
                at,
                out,
                shape: KeyShape::Linear,
            })
            .collect(),
    }
}

fn gradient(input: GradientInput, value: Curve<f64>) -> TextureSource {
    TextureSource::Gradient(Box::new(Gradient {
        input,
        value,
        ..Default::default()
    }))
}

fn value(tree: &ShaderTree, ch: Ch) -> [f64; 3] {
    tree.evaluate(&ctx())
        .get(ch)
        .eval_at(&ShadingPoint::default(), &|_, _| [0.; 4])
}

#[test]
fn a_new_gradient_is_flat_white() {
    // [measured G-20] Diffuse Color, Incidence, one white key.
    let mut tree = ShaderTree::default();
    let id = tree
        .add(operandi_core::shader_tree::NewLayer::Gradient, Some(4))
        .unwrap();
    let NodeKind::Texture(t) = &tree.get(id).unwrap().kind else {
        panic!()
    };
    assert_eq!(t.effect, TextureEffect::DiffuseColor);
    assert!(matches!(&t.source, TextureSource::Gradient(g) if g.input == GradientInput::Incidence));
    assert_eq!(
        tree.evaluate(&ctx()).get(Ch::DiffuseColor),
        &Expr::Const([1.; 3])
    );
}

#[test]
fn multiply_by_a_zero_key_makes_it_glossy() {
    // [hint] Roughness, Multiply, one key at 0 → fully glossy.
    let mut tree = ShaderTree::default();
    let id = on_top(
        &mut tree,
        texture(
            TextureEffect::Roughness,
            gradient(GradientInput::Incidence, Curve::single(0.)),
        ),
    );
    tree.nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .layer
        .blend = BlendMode::Multiply;
    assert_eq!(tree.evaluate(&ctx()).get(Ch::Roughness), &Expr::scalar(0.));
}

#[test]
fn texture_value_reads_the_folded_channel_below() {
    let mut tree = ShaderTree::default();
    on_top(&mut tree, constant(TextureEffect::DiffuseAmount, 0.25));
    on_top(
        &mut tree,
        texture(
            TextureEffect::DiffuseAmount,
            gradient(GradientInput::TextureValue, linear(&[(0., 1.), (1., 0.)])),
        ),
    );
    assert!((value(&tree, Ch::DiffuseAmount)[0] - 0.75).abs() < 1e-9);
    // Invert gives 1 − output.
    let id = tree.nodes[tree.nodes.iter().position(|n| n.id == 3).unwrap() + 1].id;
    tree.nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .layer
        .invert = true;
    assert!((value(&tree, Ch::DiffuseAmount)[0] - 0.25).abs() < 1e-9);
}

#[test]
fn drivers_fold_hidden_and_feed_gradients() {
    let mut tree = ShaderTree::default();
    let before = tree.evaluate(&ctx());
    on_top(&mut tree, constant(TextureEffect::DriverB, 0.3));
    // A driver layer leaves every surface channel alone.
    let with_driver = tree.evaluate(&ctx());
    for ch in Ch::ALL {
        if ch != Ch::DriverB {
            assert_eq!(with_driver.get(ch), before.get(ch), "{ch:?}");
        }
    }
    assert_eq!(with_driver.get(Ch::DriverB), &Expr::scalar(0.3));
    // A Gradient reading Driver B remaps it.
    on_top(
        &mut tree,
        texture(
            TextureEffect::SpecularAmount,
            gradient(GradientInput::DriverB, linear(&[(0., 0.), (1., 2.)])),
        ),
    );
    assert!((value(&tree, Ch::SpecularAmount)[0] - 0.6).abs() < 1e-9);
    // Driver A has nothing folded into it: 0.
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        texture(
            TextureEffect::SpecularAmount,
            gradient(GradientInput::DriverA, linear(&[(0., 0.2), (1., 1.)])),
        ),
    );
    assert!((value(&tree, Ch::SpecularAmount)[0] - 0.2).abs() < 1e-9);
}

#[test]
fn geometry_inputs_make_a_ramp_over_the_key_span() {
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        texture(
            TextureEffect::Roughness,
            gradient(GradientInput::Incidence, linear(&[(0., 0.1), (90., 0.9)])),
        ),
    );
    match tree.evaluate(&ctx()).get(Ch::Roughness) {
        Expr::Ramp { lo, hi, stops, .. } => {
            assert_eq!((*lo, *hi), (0., 90.));
            assert!((stops[0][0] - 0.1).abs() < 1e-9);
            assert!((stops[stops.len() - 1][0] - 0.9).abs() < 1e-9);
        }
        other => panic!("expected a ramp, got {other:?}"),
    }
    // Distance to Locator without a locator: metres from the world origin.
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        texture(
            TextureEffect::Roughness,
            gradient(
                GradientInput::DistanceToLocator,
                linear(&[(0., 0.), (10., 1.)]),
            ),
        ),
    );
    let r = tree.evaluate(&ctx()).get(Ch::Roughness).eval_at(
        &ShadingPoint {
            position: [3., 4., 0.],
            ..Default::default()
        },
        &|_, _| [0.; 4],
    );
    assert!((r[0] - 0.5).abs() < 0.02, "{r:?}");
}

#[test]
fn colour_effects_read_the_color_curve_and_its_alpha() {
    let mut tree = ShaderTree::default();
    on_top(&mut tree, constant(TextureEffect::DiffuseColor, 0.));
    on_top(&mut tree, constant(TextureEffect::DriverC, 0.5));
    let g = Gradient {
        input: GradientInput::DriverC,
        color: Curve {
            keys: vec![
                Key {
                    at: 0.,
                    out: [1., 0., 0., 0.],
                    shape: KeyShape::Linear,
                },
                Key {
                    at: 1.,
                    out: [1., 0., 0., 1.],
                    shape: KeyShape::Linear,
                },
            ],
        },
        ..Default::default()
    };
    on_top(
        &mut tree,
        texture(
            TextureEffect::DiffuseColor,
            TextureSource::Gradient(Box::new(g)),
        ),
    );
    // Red at alpha 0.5 over black: half red.
    let c = value(&tree, Ch::DiffuseColor);
    assert!((c[0] - 0.5).abs() < 1e-9 && c[1].abs() < 1e-9, "{c:?}");
}

#[test]
fn stored_only_inputs_read_zero() {
    let mut tree = ShaderTree::default();
    on_top(
        &mut tree,
        texture(
            TextureEffect::Roughness,
            gradient(GradientInput::GroupMask, linear(&[(0., 0.3), (1., 1.)])),
        ),
    );
    assert_eq!(tree.evaluate(&ctx()).get(Ch::Roughness), &Expr::scalar(0.3));
    assert!(GradientInput::GroupMask.is_stored_only());
    // Thickness renders since GR-Q10.
    assert!(!GradientInput::Thickness.is_stored_only());
    assert!(!GradientInput::ScreenX.is_stored_only());
    assert!(!GradientInput::Slope.is_stored_only());
}

#[test]
fn gradients_save_and_reopen() {
    let mut scene = operandi_core::Scene::default();
    on_top(
        &mut scene.shader_tree,
        texture(
            TextureEffect::Roughness,
            gradient(
                GradientInput::Slope,
                Curve {
                    keys: vec![
                        Key {
                            at: 0.,
                            out: 0.,
                            shape: KeyShape::Stepped,
                        },
                        Key {
                            at: 45.,
                            out: 1.,
                            shape: KeyShape::Smooth,
                        },
                    ],
                },
            ),
        ),
    );
    // A pre-OpenPBR material, so the older version is what is tested.
    scene.shader_tree.legacy_openpbr();
    let bytes = operandi_core::persistence::encode(&scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 26"));
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    // Reading gives the layer a locator; the gradient itself is unchanged.
    let source = |tree: &ShaderTree| match &tree.get(5).unwrap().kind {
        NodeKind::Texture(t) => t.source.clone(),
        _ => panic!(),
    };
    assert_eq!(source(&back.shader_tree), source(&scene.shader_tree));
    let old = text.replace("\"version\": 26", "\"version\": 25");
    assert!(operandi_core::persistence::decode(old.as_bytes()).is_err());
    // Keys out of order are refused.
    let mut bad = scene.shader_tree.clone();
    for n in &mut bad.nodes {
        if let NodeKind::Texture(TextureLayer {
            source: TextureSource::Gradient(g),
            ..
        }) = &mut n.kind
        {
            g.value.keys.reverse();
        }
    }
    assert!(bad.validate().is_err());
}

#[test]
fn gradient_edits_undo_as_one_step() {
    use operandi_core::shader_tree::{NewLayer, ShaderTreeEdit};
    let mut e = operandi_core::Editor::default();
    e.dispatch(operandi_core::Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Gradient,
        selected: Some(4),
    }))
    .unwrap();
    let id = e.scene.shader_tree.next_id - 1;
    let before = e.scene.shader_tree.clone();
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    if let NodeKind::Texture(TextureLayer {
        source: TextureSource::Gradient(g),
        ..
    }) = &mut node.kind
    {
        g.input = GradientInput::DistanceToCamera;
        g.color.keys.push(Key {
            at: 10.,
            out: [0., 0., 0., 1.],
            shape: KeyShape::Linear,
        });
    }
    e.dispatch(operandi_core::Command::ShaderTree(
        ShaderTreeEdit::Replace {
            node,
            continuing: false,
        },
    ))
    .unwrap();
    assert_ne!(e.scene.shader_tree, before);
    e.dispatch(operandi_core::Command::Undo).unwrap();
    assert_eq!(e.scene.shader_tree, before);
}
