//! LXP stage A item 1 (BEHAVIOR-LXP-MATERIALS §7.4): a Gradient on
//! Incidence drives every effect the Modo material presets use it on
//! (Diffuse Amount, Diffuse Color, Roughness, Specular Color and Group
//! Mask), all the way to the renderer's inputs and the viewport's per-pixel
//! program. Cycles takes the same principled inputs (`Surface::principled`),
//! and its Incidence render is checked in operandi-cycles `gradient_tests`.
use operandi_core::gradient::{Curve, Gradient, GradientInput, Key, KeyShape};
use operandi_core::shader_eval::{Ch, Context, Expr, Input};
use operandi_core::shader_tree::{
    GroupMask, Layer, Node, NodeId, NodeKind, ShaderTree, ShadingModel, TextureEffect,
    TextureLayer, TextureSource,
};
use operandi_core::shader_wgsl;

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

/// Adds a layer just below the Base Shader, or inside `parent` on top.
fn add(tree: &mut ShaderTree, parent: Option<NodeId>, kind: NodeKind) -> NodeId {
    let id = tree.next_id;
    tree.next_id += 1;
    let at = match parent {
        None => tree.nodes.iter().position(|n| n.id == 3).unwrap() + 1,
        Some(p) => tree.nodes.iter().position(|n| n.id == p).unwrap() + 1,
    };
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

fn incidence(effect: TextureEffect) -> NodeKind {
    let key = |at: f64, v: f64| Key {
        at,
        out: v,
        shape: KeyShape::Linear,
    };
    let ckey = |at: f64, c: [f64; 4]| Key {
        at,
        out: c,
        shape: KeyShape::Linear,
    };
    NodeKind::Texture(TextureLayer {
        source: TextureSource::Gradient(Box::new(Gradient {
            input: GradientInput::Incidence,
            value: Curve {
                keys: vec![key(0., 0.2), key(90., 0.9)],
            },
            color: Curve {
                keys: vec![ckey(0., [1., 0., 0., 1.]), ckey(90., [0., 0., 1., 1.])],
            },
        })),
        effect,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

/// Whether `e` reads the view: the viewport draws it per pixel then.
fn per_pixel(e: &Expr) -> bool {
    shader_wgsl::translate(e, &mut |_| None).is_some_and(|p| p.reads_view && p.per_pixel())
}

fn input(tree: &ShaderTree, which: Input) -> Expr {
    tree.evaluate(&ctx())
        .principled()
        .inputs
        .into_iter()
        .find(|(i, _)| *i == which)
        .map(|(_, e)| e)
        .unwrap()
}

fn physically_based(tree: &mut ShaderTree) {
    for n in &mut tree.nodes {
        if let NodeKind::Material(m) = &mut n.kind {
            m.shading_model = ShadingModel::PhysicallyBased;
        }
    }
}

#[test]
fn incidence_drives_each_material_effect_to_the_render() {
    for (effect, target) in [
        (TextureEffect::DiffuseColor, Input::BaseColor),
        (TextureEffect::DiffuseAmount, Input::BaseColor),
        (TextureEffect::Roughness, Input::Roughness),
        (TextureEffect::SpecularColor, Input::SpecularTint),
        (TextureEffect::SpecularAmount, Input::SpecularIorLevel),
    ] {
        let mut tree = ShaderTree::default();
        // Diffuse Amount multiplies the base colour in Physically Based,
        // the model a converted Modo material uses.
        physically_based(&mut tree);
        add(&mut tree, None, incidence(effect));
        let e = input(&tree, target);
        assert!(per_pixel(&e), "{effect:?} -> {target:?}: {e:?}");
    }
}

#[test]
fn incidence_group_mask_fades_the_group_with_the_angle() {
    let mut tree = ShaderTree::default();
    let group = add(&mut tree, None, NodeKind::Group(GroupMask::default()));
    // A red layer inside the group; its mask follows the view angle.
    add(
        &mut tree,
        Some(group),
        NodeKind::Texture(TextureLayer {
            source: TextureSource::Constant {
                color: [1., 0., 0.],
                value: 1.,
            },
            effect: TextureEffect::DiffuseColor,
            locator: None,
            legacy_projection: None,
            legacy_axis: None,
            ignore_scale_group: false,
            node_input: None,
        }),
    );
    add(&mut tree, Some(group), incidence(TextureEffect::GroupMask));
    let base = tree.evaluate(&ctx()).get(Ch::DiffuseColor).clone();
    assert!(per_pixel(&base), "{base:?}");
    assert!(per_pixel(&input(&tree, Input::BaseColor)));
}

#[test]
fn incidence_on_reflection_color_folds_into_its_channel() {
    // Reflection Color reaches no render input yet (ST-Q05, LXP-B01);
    // the channel itself still carries the gradient.
    let mut tree = ShaderTree::default();
    add(&mut tree, None, incidence(TextureEffect::ReflectionColor));
    let e = tree.evaluate(&ctx()).get(Ch::ReflectionColor).clone();
    assert!(per_pixel(&e), "{e:?}");
}

#[test]
fn constant_and_image_free_values_stay_on_the_cpu() {
    let p = shader_wgsl::translate(&Expr::scalar(0.4), &mut |_| None).unwrap();
    assert!(!p.per_pixel());
}

#[test]
fn a_noise_on_driver_a_colours_a_driver_a_gradient() {
    // LXP stage A item 2: the presets' "noise drives a gradient's colour"
    // (§7.4): Noise on Driver A, Gradient on Diffuse Color reading Driver A.
    use operandi_core::procedural::{Procedural, ProceduralKind};
    use operandi_core::shader_eval::ShadingPoint;
    let mut tree = ShaderTree::default();
    add(
        &mut tree,
        None,
        NodeKind::Texture(TextureLayer {
            source: TextureSource::Procedural(Box::new(Procedural::new(ProceduralKind::Noise))),
            effect: TextureEffect::DriverA,
            locator: None,
            legacy_projection: None,
            legacy_axis: None,
            ignore_scale_group: false,
            node_input: None,
        }),
    );
    let mut g = incidence(TextureEffect::DiffuseColor);
    if let NodeKind::Texture(t) = &mut g
        && let TextureSource::Gradient(g) = &mut t.source
    {
        g.input = GradientInput::DriverA;
        g.color.keys[1].at = 1.;
    }
    add(&mut tree, None, g);
    let surface = tree.evaluate(&ctx());
    let base = surface.get(Ch::DiffuseColor);
    assert!(base.varies_by_pattern(), "{base:?}");
    // Red where the noise is 0, blue where it is 1; somewhere between.
    let colours: Vec<[f64; 3]> = (0..40)
        .map(|k| {
            base.eval_at(
                &ShadingPoint {
                    // No locator: the noise reads the mesh UVs.
                    uv: [k as f64 * 0.137, k as f64 * 0.071],
                    ..Default::default()
                },
                &|_, _| [0.; 4],
            )
        })
        .collect();
    assert!(
        colours
            .iter()
            .all(|c| (c[0] + c[2] - 1.).abs() < 1e-6 && c[1].abs() < 1e-9)
    );
    assert!(
        colours.iter().any(|c| (c[0] - colours[0][0]).abs() > 0.05),
        "{colours:?} {base:?}"
    );
}
