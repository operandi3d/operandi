//! Shader tree remaining, stage 5: group Scale, Apply to Sub-Group and the
//! newer Material settings (group extras v1).
use operandi_core::shader_eval::{Ch, Context, Expr, Input};
use operandi_core::shader_tree::{
    GroupMask, GroupScale, Layer, MaterialChannels, NewLayer, Node, NodeId, NodeKind, ShaderTree,
    ShaderTreeEdit, TagMask, TextureEffect, TextureLayer, TextureSource,
};

fn ctx(material: &'static str) -> Context<'static> {
    Context {
        item: 1,
        ancestors: &[],
        material,
        part: None,
        sets: &[],
        locators: &[],
    }
}

fn insert(tree: &mut ShaderTree, parent: Option<NodeId>, kind: NodeKind) -> NodeId {
    let id = tree.next_id;
    tree.next_id += 1;
    let at = match parent {
        Some(p) => tree.nodes.iter().position(|n| n.id == p).unwrap() + 1,
        None => tree.nodes.iter().position(|n| n.id == 3).unwrap() + 1,
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

fn group(scale: f64) -> NodeKind {
    NodeKind::Group(GroupMask {
        scale: GroupScale {
            scale,
            ..Default::default()
        },
        ..Default::default()
    })
}

fn image() -> NodeKind {
    NodeKind::Texture(TextureLayer {
        source: TextureSource::ImageMap {
            image: Some(1),
            settings: Box::default(),
        },
        effect: TextureEffect::DiffuseColor,
        locator: None,
        legacy_projection: None,
        legacy_axis: None,
        ignore_scale_group: false,
        node_input: None,
    })
}

fn distance(tree: &ShaderTree) -> f64 {
    tree.evaluate(&ctx("Default"))
        .get(Ch::DisplacementDistance)
        .constant()
        .unwrap()[0]
}

#[test]
fn scale_block_starting_values() {
    let s = GroupScale::default();
    assert!(
        s.uv && s.size
            && s.bump_amplitude
            && s.absorption_distance
            && s.displacement_distance
            && s.subsurface_distance
    );
    assert_eq!(s.scale, 1.);
}

#[test]
fn group_scale_resizes_distances_and_nests() {
    let mut tree = ShaderTree::default();
    let base = distance(&tree);
    let outer = insert(&mut tree, None, group(0.5));
    // The material is outside the group: its surfaces in the group still
    // take the group's Scale.
    let lone = insert(&mut tree, Some(outer), image());
    let _ = lone;
    assert!((distance(&tree) - base * 0.5).abs() < 1e-12);
    // Nested Scales multiply (XQ-2).
    insert(&mut tree, Some(outer), group(0.5));
    assert!((distance(&tree) - base * 0.25).abs() < 1e-12);
    // A switched-off distance is left alone.
    for n in &mut tree.nodes {
        if let NodeKind::Group(g) = &mut n.kind {
            g.scale.displacement_distance = false;
        }
    }
    assert!((distance(&tree) - base).abs() < 1e-12);
}

#[test]
fn group_scale_resizes_textures_unless_ignored() {
    use operandi_core::shader_tree::Projection;
    use operandi_core::texture_locator::{LocatorFrame, TextureLocator};
    let mut tree = ShaderTree::default();
    let g = insert(&mut tree, None, group(0.5));
    let id = insert(&mut tree, Some(g), image());
    for n in &mut tree.nodes {
        if n.id == id
            && let NodeKind::Texture(t) = &mut n.kind
        {
            t.locator = Some(99);
        }
    }
    let frames = [LocatorFrame::unplaced(
        99,
        TextureLocator::new(Projection::UvMap),
    )];
    let c = Context {
        locators: &frames,
        ..ctx("Default")
    };
    let e = format!("{:?}", tree.evaluate(&c).get(Ch::DiffuseColor));
    // UV repeat ÷ Scale: 50 % shows the pattern at half size (XQ-1).
    assert!(e.contains("repeat: [2.0, 2.0]"), "{e}");
    for n in &mut tree.nodes {
        if n.id == id
            && let NodeKind::Texture(t) = &mut n.kind
        {
            t.ignore_scale_group = true;
        }
    }
    let e = format!("{:?}", tree.evaluate(&c).get(Ch::DiffuseColor));
    assert!(e.contains("repeat: [1.0, 1.0]"), "{e}");
}

#[test]
fn apply_to_sub_group_limits_an_untagged_group() {
    let mut tree = ShaderTree::default();
    let outer = insert(
        &mut tree,
        None,
        NodeKind::Group(GroupMask {
            apply_to_sub_group: true,
            ..Default::default()
        }),
    );
    insert(
        &mut tree,
        Some(outer),
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
    // A tagged group inside it, for material "Red".
    let red = tree.nodes.iter().position(|n| n.id == outer).unwrap() + 2;
    let id = tree.next_id;
    tree.next_id += 1;
    tree.nodes.insert(
        red,
        Node {
            extras: Default::default(),
            id,
            parent: Some(outer),
            name: "Red".into(),
            layer: Layer::default(),
            kind: NodeKind::Group(GroupMask {
                tag: TagMask::Name("Red".into()),
                ..Default::default()
            }),
        },
    );
    tree.validate().unwrap();
    let colour = |m| {
        tree.evaluate(&ctx(m))
            .get(Ch::DiffuseColor)
            .constant()
            .unwrap()
    };
    assert_eq!(colour("Red"), [1., 0., 0.]);
    assert_ne!(colour("Default"), [1., 0., 0.]);
    // Off: an untagged group is a folder.
    for n in &mut tree.nodes {
        if n.id == outer
            && let NodeKind::Group(g) = &mut n.kind
        {
            g.apply_to_sub_group = false;
        }
    }
    let colour = |m| {
        tree.evaluate(&ctx(m))
            .get(Ch::DiffuseColor)
            .constant()
            .unwrap()
    };
    assert_eq!(colour("Default"), [1., 0., 0.]);
}

fn base_material(tree: &mut ShaderTree) -> &mut MaterialChannels {
    match &mut tree.nodes.iter_mut().find(|n| n.id == 4).unwrap().kind {
        NodeKind::Material(m) => m,
        _ => panic!(),
    }
}

fn input(tree: &ShaderTree, which: Input) -> Option<Expr> {
    tree.evaluate(&ctx("Default"))
        .principled()
        .inputs
        .into_iter()
        .find(|(i, _)| *i == which)
        .map(|(_, e)| e)
}

#[test]
fn material_extras_reach_the_renderer() {
    let mut tree = ShaderTree::default();
    // Untouched: no anisotropy input, IOR 1.5 with Specular Amount.
    assert_eq!(input(&tree, Input::SubsurfaceAnisotropy), None);
    assert_eq!(input(&tree, Input::Ior), Some(Expr::scalar(1.5)));
    {
        let m = base_material(&mut tree);
        m.more.use_refractive_index = true;
        m.refractive_index = 2.4;
        m.more.subsurface_phase = 0.75;
        m.more.rounded_edge_width = 0.01;
    }
    assert_eq!(input(&tree, Input::Ior), Some(Expr::scalar(2.4)));
    assert_eq!(
        input(&tree, Input::SpecularIorLevel),
        Some(Expr::scalar(0.5))
    );
    assert_eq!(
        input(&tree, Input::SubsurfaceAnisotropy),
        Some(Expr::scalar(0.5))
    );
    let p = tree.evaluate(&ctx("Default")).principled();
    assert_eq!(p.rounded_edge, Some(0.01));
    assert_eq!(p.absorption, None);
    // Absorption on coloured glass: Beer's law.
    {
        let m = base_material(&mut tree);
        m.transparent_amount = 1.;
        m.refractive_index = 1.5;
        m.transparent_color = [0.5, 1., 1.];
        m.absorption_distance = 0.1;
    }
    let p = tree.evaluate(&ctx("Default")).principled();
    let (colour, density) = p.absorption.unwrap();
    assert!((density - (2f64).ln() / 0.1).abs() < 1e-9);
    assert!(colour[0].abs() < 1e-9 && (colour[1] - 1.).abs() < 1e-9);
}

#[test]
fn apply_scale_bakes_and_resets_in_one_step() {
    let mut e = operandi_core::Editor::default();
    e.dispatch(operandi_core::Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let g = e.scene.shader_tree.next_id - 1;
    e.dispatch(operandi_core::Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Material,
        selected: Some(g),
    }))
    .unwrap();
    let m = e.scene.shader_tree.next_id - 1;
    let mut node = e.scene.shader_tree.get(m).unwrap().clone();
    if let NodeKind::Material(mat) = &mut node.kind {
        mat.displacement_distance = 0.02;
    }
    e.dispatch(operandi_core::Command::ShaderTree(
        ShaderTreeEdit::Replace {
            node,
            continuing: false,
        },
    ))
    .unwrap();
    let mut node = e.scene.shader_tree.get(g).unwrap().clone();
    if let NodeKind::Group(gm) = &mut node.kind {
        gm.scale.scale = 0.5;
    }
    e.dispatch(operandi_core::Command::ShaderTree(
        ShaderTreeEdit::Replace {
            node,
            continuing: false,
        },
    ))
    .unwrap();
    let before = e.scene.shader_tree.clone();
    e.dispatch(operandi_core::Command::ShaderTree(
        ShaderTreeEdit::ApplyScale { group: g },
    ))
    .unwrap();
    let tree = &e.scene.shader_tree;
    match (&tree.get(g).unwrap().kind, &tree.get(m).unwrap().kind) {
        (NodeKind::Group(gm), NodeKind::Material(mat)) => {
            assert_eq!(gm.scale.scale, 1.);
            assert!((mat.displacement_distance - 0.01).abs() < 1e-12);
        }
        _ => panic!(),
    }
    e.dispatch(operandi_core::Command::Undo).unwrap();
    assert_eq!(e.scene.shader_tree, before);
    // Nothing to apply at 100 %.
    let mut e2 = operandi_core::Editor::default();
    e2.dispatch(operandi_core::Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let g2 = e2.scene.shader_tree.next_id - 1;
    assert!(
        e2.dispatch(operandi_core::Command::ShaderTree(
            ShaderTreeEdit::ApplyScale { group: g2 }
        ))
        .is_err()
    );
}

#[test]
fn extras_save_only_when_used() {
    let mut scene = operandi_core::Scene::default();
    // A pre-OpenPBR material, so the older version is what is tested.
    scene.shader_tree.legacy_openpbr();
    let text = String::from_utf8(operandi_core::persistence::encode(&scene).unwrap()).unwrap();
    assert!(!text.contains("rounded_edge_width") && !text.contains("\"scale\": {"));
    base_material(&mut scene.shader_tree).more.dispersion = 0.3;
    let bytes = operandi_core::persistence::encode(&scene).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"version\": 26")
    );
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree.get(4), scene.shader_tree.get(4));
}
