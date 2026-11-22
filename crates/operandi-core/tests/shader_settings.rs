//! Shader tree remaining, stage 1: cut-out effects (Dissolve, Stencil,
//! RGBA) and the Shader item's settings (control switches, fog formula,
//! starting values, saving).
use operandi_core::shader_eval::{Ch, Context, Expr, Input};
use operandi_core::shader_tree::{
    AlphaType, FogType, ImageAlpha, ImageSettings, Layer, Node, NodeId, NodeKind, ShaderChannels,
    ShaderTree, TextureEffect, TextureLayer, TextureSource,
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

/// Puts `node` directly above the Base Material (below the Base Shader).
fn above_material(tree: &mut ShaderTree, kind: NodeKind) -> NodeId {
    let id = tree.next_id;
    tree.next_id += 1;
    let at = tree.nodes.iter().position(|n| n.id == 4).unwrap();
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

fn constant(effect: TextureEffect, color: [f64; 3], value: f64) -> NodeKind {
    NodeKind::Texture(TextureLayer {
        source: TextureSource::Constant { color, value },
        effect,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

fn image(effect: TextureEffect, alpha: ImageAlpha) -> NodeKind {
    NodeKind::Texture(TextureLayer {
        source: TextureSource::ImageMap {
            image: Some(7),
            settings: Box::new(ImageSettings {
                alpha,
                ..Default::default()
            }),
        },
        effect,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

fn alpha_of(tree: &ShaderTree, rgba: [f64; 4]) -> f64 {
    let p = tree.evaluate(&ctx()).principled();
    let (_, a) = p.inputs.iter().find(|(i, _)| *i == Input::Alpha).unwrap();
    a.eval(&|_, _| rgba)[0]
}

fn shader_mut(tree: &mut ShaderTree, id: NodeId) -> &mut ShaderChannels {
    match &mut tree.nodes.iter_mut().find(|n| n.id == id).unwrap().kind {
        NodeKind::Shader(s) => s,
        _ => panic!("not a shader"),
    }
}

#[test]
fn dissolve_keeps_one_minus_the_value() {
    // [measured] 25 % keeps 0.75 of the surface, 75 % keeps 0.25.
    for (v, keep) in [(0.25, 0.75), (0.75, 0.25)] {
        let mut tree = ShaderTree::default();
        above_material(&mut tree, constant(TextureEffect::Dissolve, [0.; 3], v));
        assert!((alpha_of(&tree, [0.; 4]) - keep).abs() < 1e-9);
    }
}

#[test]
fn stencil_cuts_above_half_only() {
    // [measured] 0, 30, 49 % leave the surface; 51, 70, 100 % remove it.
    for (v, keep) in [
        (0., 1.),
        (0.3, 1.),
        (0.49, 1.),
        (0.5, 1.),
        (0.51, 0.),
        (0.7, 0.),
        (1., 0.),
    ] {
        let mut tree = ShaderTree::default();
        above_material(&mut tree, constant(TextureEffect::Stencil, [0.; 3], v));
        assert_eq!(alpha_of(&tree, [0.; 4]), keep, "stencil {v}");
    }
    // Stencil from an image's brightness: white cuts holes.
    let mut tree = ShaderTree::default();
    above_material(&mut tree, image(TextureEffect::Stencil, ImageAlpha::Ignore));
    assert_eq!(alpha_of(&tree, [1., 1., 1., 1.]), 0.);
    assert_eq!(alpha_of(&tree, [0.2, 0.2, 0.2, 1.]), 1.);
    // A material above it starts the channel again at 0.
    let mut tree = ShaderTree::default();
    above_material(&mut tree, constant(TextureEffect::Stencil, [0.; 3], 1.));
    let id = tree
        .add(operandi_core::shader_tree::NewLayer::Material, None)
        .unwrap();
    let at = tree.nodes.iter().position(|n| n.id == id).unwrap();
    let m = tree.nodes.remove(at);
    tree.nodes.insert(3, m);
    assert_eq!(alpha_of(&tree, [0.; 4]), 1.);
}

#[test]
fn rgba_sets_diffuse_and_cuts_low_alpha() {
    let mut tree = ShaderTree::default();
    above_material(&mut tree, image(TextureEffect::Rgba, ImageAlpha::Use));
    let s = tree.evaluate(&ctx());
    let diffuse = |rgba| s.get(Ch::DiffuseColor).eval(&|_, _| rgba);
    // Solid where alpha is 50 % or more, gone below.
    assert_eq!(alpha_of(&tree, [1., 0., 0., 0.49]), 0.);
    assert_eq!(alpha_of(&tree, [1., 0., 0., 0.5]), 1.);
    assert_eq!(alpha_of(&tree, [1., 0., 0., 1.]), 1.);
    // Full alpha: the image's colour is the Diffuse Color.
    let c = diffuse([1., 0., 0., 1.]);
    assert!((c[0] - 1.).abs() < 1e-9 && c[1].abs() < 1e-9);
    // Alpha Ignore: never cuts, plain Diffuse Color layer.
    let mut tree = ShaderTree::default();
    above_material(&mut tree, image(TextureEffect::Rgba, ImageAlpha::Ignore));
    assert_eq!(alpha_of(&tree, [1., 0., 0., 0.]), 1.);
    // A Constant (alpha 100 %) acts as a Diffuse Color layer.
    let mut tree = ShaderTree::default();
    above_material(&mut tree, constant(TextureEffect::Rgba, [0., 0., 1.], 1.));
    let s = tree.evaluate(&ctx());
    assert_eq!(s.get(Ch::DiffuseColor), &Expr::Const([0., 0., 1.]));
    assert_eq!(alpha_of(&tree, [0.; 4]), 1.);
    // A disabled RGBA layer does nothing.
    let mut tree = ShaderTree::default();
    let id = above_material(&mut tree, image(TextureEffect::Rgba, ImageAlpha::Use));
    tree.nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .layer
        .enable = false;
    assert_eq!(alpha_of(&tree, [1., 0., 0., 0.]), 1.);
}

#[test]
fn cut_outs_multiply() {
    // Visibility = (1 − dissolve) × (1 − stencil cut) × (1 − rgba cut).
    let mut tree = ShaderTree::default();
    above_material(&mut tree, constant(TextureEffect::Dissolve, [0.; 3], 0.5));
    above_material(&mut tree, constant(TextureEffect::Stencil, [0.; 3], 0.2));
    assert!((alpha_of(&tree, [0.; 4]) - 0.5).abs() < 1e-9);
    // The stencil channel is a constant here, so the alpha stays a number.
    let p = tree.evaluate(&ctx()).principled();
    let (_, a) = p.inputs.iter().find(|(i, _)| *i == Input::Alpha).unwrap();
    assert!(a.constant().is_some());
}

#[test]
fn starting_values_match_the_spec() {
    let s = ShaderChannels::default();
    let m = &s.more;
    assert!(
        m.compute_shading
            && m.control_quality
            && m.control_visibility
            && m.control_light_linking
            && m.control_fog
    );
    assert_eq!(
        (
            m.shading_rate,
            m.direct_multiplier,
            m.indirect_multiplier,
            m.incoming_indirect_saturation,
            m.outgoing_indirect_saturation
        ),
        (1., 1., 1., 1., 1.)
    );
    assert!(s.cast_shadows && s.receive_shadows && s.visible_to_camera && m.visible_to_occlusion);
    assert_eq!((s.alpha_type, s.alpha_value), (AlphaType::Opacity, 1.));
    assert_eq!(m.light_group, None);
    assert_eq!(m.link_mode, operandi_core::camera_light::LinkMode::Exclude);
    assert_eq!(m.fog_type, FogType::None);
    assert!(!m.fog_use_environment);
    assert_eq!(m.fog_color, [0.5; 3]);
    assert_eq!((m.fog_start, m.fog_end, m.fog_density), (0., 10., 0.1));
}

#[test]
fn fog_factor_matches_the_measured_points() {
    let mut s = ShaderChannels::default();
    assert_eq!(s.fog_factor(5., 0.), 0.);
    s.more.fog_type = FogType::Linear;
    assert!((s.fog_factor(5., 0.) - 0.5).abs() < 1e-9);
    assert!((s.fog_factor(2.5, 0.) - 0.25).abs() < 1e-9);
    assert_eq!(s.fog_factor(20., 0.), 1.);
    s.more.fog_type = FogType::Exponential;
    for (d, f) in [(0.5, 0.045), (2.5, 0.22), (5., 0.39)] {
        assert!((s.fog_factor(d, 0.) - f).abs() < 0.006, "d {d}");
    }
    s.more.fog_density = 0.3;
    assert!((s.fog_factor(5., 0.) - 0.77).abs() < 0.01);
    // Layered: Exponential at height 0, thinner higher up (SH-4).
    s.more.fog_type = FogType::Layered;
    assert!((s.fog_factor(5., 0.) - 0.77).abs() < 0.01);
    assert!(s.fog_factor(5., 2.) < s.fog_factor(5., 0.));
    assert_eq!(s.fog_factor(5., -3.), s.fog_factor(5., 0.));
}

#[test]
fn switched_off_sections_come_from_the_shader_below() {
    let mut tree = ShaderTree::default();
    // A second shader on top of the Base Shader.
    let top = tree
        .add(operandi_core::shader_tree::NewLayer::Shader, None)
        .unwrap();
    {
        let base = shader_mut(&mut tree, 3);
        base.visible_to_camera = false;
        base.more.fog_type = FogType::Exponential;
        base.more.direct_multiplier = 0.5;
    }
    {
        let s = shader_mut(&mut tree, top);
        s.cast_shadows = false;
        s.more.fog_type = FogType::Linear;
        s.more.direct_multiplier = 2.;
    }
    // Every switch on: the top shader wins every section.
    let won = tree.evaluate(&ctx()).shader;
    assert!(won.visible_to_camera && !won.cast_shadows);
    assert_eq!(won.more.fog_type, FogType::Linear);
    // Control Visibility off: visibility from the Base Shader; fog and
    // quality stay the top shader's.
    shader_mut(&mut tree, top).more.control_visibility = false;
    let won = tree.evaluate(&ctx()).shader;
    assert!(!won.visible_to_camera && won.cast_shadows);
    assert_eq!(won.more.fog_type, FogType::Linear);
    assert_eq!(won.more.direct_multiplier, 2.);
    shader_mut(&mut tree, top).more.control_fog = false;
    shader_mut(&mut tree, top).more.control_quality = false;
    let won = tree.evaluate(&ctx()).shader;
    assert_eq!(won.more.fog_type, FogType::Exponential);
    assert_eq!(won.more.direct_multiplier, 0.5);
    // With nothing below, a switched-off section has the starting values.
    let mut tree = ShaderTree::default();
    {
        let base = shader_mut(&mut tree, 3);
        base.visible_to_camera = false;
        base.more.control_visibility = false;
    }
    assert!(tree.evaluate(&ctx()).shader.visible_to_camera);
}

#[test]
fn shader_settings_save_and_reopen() {
    let mut scene = operandi_core::Scene::default();
    // Untouched settings save without the new block (older readers).
    let plain = String::from_utf8(operandi_core::persistence::encode(&scene).unwrap()).unwrap();
    assert!(!plain.contains("\"more\""));
    {
        let s = shader_mut(&mut scene.shader_tree, 3);
        s.more.fog_type = FogType::Underwater;
        s.more.fog_use_environment = true;
        s.more.direct_multiplier = 0.25;
        s.more.control_fog = false;
        s.more.light_group = Some(42);
        s.alpha_type = AlphaType::Constant;
        s.alpha_value = 0.4;
    }
    // A pre-OpenPBR material, so the older version is what is tested.
    scene.shader_tree.legacy_openpbr();
    let bytes = operandi_core::persistence::encode(&scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 21"));
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree, scene.shader_tree);
    // The settings need version 21: a file claiming 20 is refused.
    let old = text.replace("\"version\": 21", "\"version\": 20");
    assert!(operandi_core::persistence::decode(old.as_bytes()).is_err());
}

#[test]
fn shader_settings_undo_as_one_step() {
    use operandi_core::shader_tree::ShaderTreeEdit;
    let mut editor = operandi_core::Editor::default();
    let before = editor.scene.shader_tree.clone();
    let mut node = editor.scene.shader_tree.get(3).unwrap().clone();
    if let NodeKind::Shader(s) = &mut node.kind {
        s.more.fog_type = FogType::Linear;
        s.receive_shadows = false;
    }
    editor
        .dispatch(operandi_core::Command::ShaderTree(
            ShaderTreeEdit::Replace {
                node,
                continuing: false,
            },
        ))
        .unwrap();
    assert_ne!(editor.scene.shader_tree, before);
    editor.dispatch(operandi_core::Command::Undo).unwrap();
    assert_eq!(editor.scene.render_shader_tree().nodes, before.nodes);
    editor.dispatch(operandi_core::Command::Redo).unwrap();
    let won = editor.scene.shader_tree.evaluate(&ctx()).shader;
    assert_eq!(won.more.fog_type, FogType::Linear);
    assert!(!won.receive_shadows);
    // Out-of-range values are refused and leave the tree alone.
    let mut bad = editor.scene.shader_tree.get(3).unwrap().clone();
    if let NodeKind::Shader(s) = &mut bad.kind {
        s.more.fog_density = -1.;
    }
    let kept = editor.scene.shader_tree.clone();
    assert!(
        editor
            .dispatch(operandi_core::Command::ShaderTree(
                ShaderTreeEdit::Replace {
                    node: bad,
                    continuing: false,
                }
            ))
            .is_err()
    );
    assert_eq!(editor.scene.shader_tree, kept);
}

/// SH-2 (provisional): a Shader's Effect keeps only that part of the
/// surface's light. Diffuse drops reflections and emission; Specular keeps
/// the base colour only where metallic; Luminous keeps only the emission;
/// Full Shading changes nothing.
#[test]
fn shader_effect_keeps_only_its_part() {
    use operandi_core::shader_tree::ShaderEffect;
    let value = |effect: ShaderEffect, input: Input| {
        let mut tree = ShaderTree::default();
        above_material(&mut tree, constant(TextureEffect::Metallic, [0.; 3], 0.5));
        above_material(
            &mut tree,
            constant(TextureEffect::LuminousAmount, [0.; 3], 2.),
        );
        let base = tree
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Shader(_)))
            .unwrap()
            .id;
        shader_mut(&mut tree, base).effect = effect;
        let p = tree.evaluate(&ctx()).principled();
        let (_, e) = p.inputs.iter().find(|(i, _)| *i == input).unwrap();
        e.eval(&|_, _| [0.; 4])
    };
    assert_eq!(value(ShaderEffect::FullShading, Input::Metallic)[0], 0.5);
    assert_eq!(value(ShaderEffect::DiffuseShading, Input::Metallic)[0], 0.);
    assert_eq!(
        value(ShaderEffect::DiffuseShading, Input::EmissionStrength)[0],
        0.
    );
    assert_eq!(
        value(ShaderEffect::DiffuseShading, Input::SpecularIorLevel)[0],
        0.
    );
    let full = value(ShaderEffect::FullShading, Input::BaseColor);
    let spec = value(ShaderEffect::SpecularShading, Input::BaseColor);
    assert!((spec[0] - full[0] * 0.5).abs() < 1e-9, "{spec:?} {full:?}");
    assert_eq!(
        value(ShaderEffect::LuminousShading, Input::BaseColor),
        [0.; 3]
    );
    assert!(value(ShaderEffect::LuminousShading, Input::EmissionStrength)[0] > 0.);
    assert_eq!(
        value(ShaderEffect::FogShading, Input::EmissionStrength)[0],
        0.
    );
}
