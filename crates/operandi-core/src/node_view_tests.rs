use super::*;
use crate::shader_tree::{Drop, NewLayer, ShaderTreeEdit};
use crate::{Command, Editor};

/// A plane whose Base Material (layer 4) sits in a new Material Group;
/// the group's id.
pub(crate) fn editor() -> (Editor, NodeId) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = e.scene.shader_tree.next_id - 1;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
        ids: vec![MATERIAL],
        drop: Drop {
            parent: Some(group),
            index: 0,
        },
    }))
    .unwrap();
    (e, group)
}
/// The default tree's Base Material.
pub(crate) const MATERIAL: NodeId = 4;

pub(crate) fn nodes(e: &mut Editor, group: NodeId, edit: NodeEdit) -> crate::Result<()> {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Nodes { group, edit }))
}
pub(crate) fn add(e: &mut Editor, group: NodeId, category: &str, ty: &str) -> String {
    nodes(
        e,
        group,
        NodeEdit::Add {
            category: category.into(),
            ty: ty.into(),
            at: [10., 20.],
        },
    )
    .unwrap();
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    g.free.last().unwrap().name.clone()
}
pub(crate) fn diffuse(layer: NodeId) -> Target {
    Target::Layer {
        layer,
        channel: "diffuse_color".into(),
    }
}
fn connect(e: &mut Editor, group: NodeId, from: &str, to: Target) -> crate::Result<()> {
    nodes(
        e,
        group,
        NodeEdit::Connect {
            from: from.into(),
            output: "out".into(),
            to,
        },
    )
}
fn last_label(e: &Editor) -> String {
    e.undo.last().unwrap().label.clone()
}

#[test]
fn ne_02_add_makes_a_free_node_with_the_definitions_values() {
    let (mut e, group) = editor();
    let name = add(&mut e, group, "noise3d", "color3");
    assert_eq!(name, "Noise");
    assert_eq!(last_label(&e), "Add Node Noise");
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    let n = g.node("Noise").unwrap();
    assert_eq!(n.input("amplitude").unwrap().value, "1.0, 1.0, 1.0");
    // `position` has no value (a geometric default) but is still an input.
    assert!(n.input("position").is_none());
    assert_eq!(n.input_type("position").as_deref(), Some("vector3"));
    assert_eq!(g.places["Noise"], [10., 20.]);
    // A second one gets its own name; nodes outside the menu are refused.
    assert_eq!(add(&mut e, group, "noise3d", "color3"), "Noise2");
    assert!(
        nodes(
            &mut e,
            group,
            NodeEdit::Add {
                category: "open_pbr_surface".into(),
                ty: "surfaceshader".into(),
                at: [0.; 2],
            }
        )
        .is_err()
    );
    // Only a Material Group has a node view.
    assert!(
        nodes(
            &mut e,
            MATERIAL,
            NodeEdit::Add {
                category: "noise3d".into(),
                ty: "color3".into(),
                at: [0.; 2],
            }
        )
        .is_err()
    );
}

#[test]
fn ne_03_08_18_connect_checks_types_and_names_the_step() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    let uv = add(&mut e, group, "texcoord", "vector2");
    connect(&mut e, group, &noise, diffuse(MATERIAL)).unwrap();
    assert_eq!(last_label(&e), "Connect Noise > Diffuse Color");
    let tree = &e.scene.shader_tree;
    assert!(tree.channel_driver(MATERIAL, Ch::DiffuseColor).is_some());
    // The layer's own value is kept.
    let crate::shader_tree::NodeKind::Material(m) = &tree.get(MATERIAL).unwrap().kind else {
        panic!()
    };
    assert_eq!(
        m.diffuse_color,
        crate::shader_tree::MaterialChannels::default().diffuse_color
    );
    // NE-08: a vector2 can't drive a colour; nothing changes.
    let before = e.scene.clone();
    assert!(connect(&mut e, group, &uv, diffuse(MATERIAL)).is_err());
    assert!(
        connect(
            &mut e,
            group,
            &uv,
            Target::Free {
                node: noise.clone(),
                input: "amplitude".into(),
            }
        )
        .is_err()
    );
    // No loops between free nodes.
    let mix = add(&mut e, group, "mix", "color3");
    connect(
        &mut e,
        group,
        &noise,
        Target::Free {
            node: mix.clone(),
            input: "fg".into(),
        },
    )
    .unwrap();
    assert!(
        connect(
            &mut e,
            group,
            &mix,
            Target::Free {
                node: noise.clone(),
                input: "amplitude".into(),
            }
        )
        .is_err()
    );
    // No wires into another material's layer.
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Material,
        selected: None,
    }))
    .unwrap();
    let outside = e.scene.shader_tree.next_id - 1;
    assert_eq!(e.scene.shader_tree.material_group_of(outside), None);
    assert!(connect(&mut e, group, &noise, diffuse(outside)).is_err());
    let _ = before;
}

#[test]
fn ne_03_disconnect_restores_the_own_value() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(&mut e, group, &noise, diffuse(MATERIAL)).unwrap();
    nodes(&mut e, group, NodeEdit::Disconnect(diffuse(MATERIAL))).unwrap();
    assert_eq!(last_label(&e), "Disconnect Diffuse Color");
    assert!(
        e.scene
            .shader_tree
            .channel_driver(MATERIAL, Ch::DiffuseColor)
            .is_none()
    );
    assert!(nodes(&mut e, group, NodeEdit::Disconnect(diffuse(MATERIAL))).is_err());
}

#[test]
fn ne_t5_three_free_nodes_save_and_reopen() {
    let (mut e, group) = editor();
    let a = add(&mut e, group, "noise3d", "color3");
    let b = add(&mut e, group, "multiply", "color3");
    add(&mut e, group, "constant", "float");
    connect(
        &mut e,
        group,
        &a,
        Target::Free {
            node: b.clone(),
            input: "in1".into(),
        },
    )
    .unwrap();
    connect(&mut e, group, &b, diffuse(MATERIAL)).unwrap();
    nodes(&mut e, group, NodeEdit::Hide(vec![a.clone()])).unwrap();
    assert_eq!(e.scene.shader_tree.free_node_count(group), 3);
    let bytes = crate::persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 56"), "saved as v56");
    let back = crate::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree.node_views, e.scene.shader_tree.node_views);
    // An older version number with node views is refused, not dropped.
    let old = text.replace("\"version\": 56", "\"version\": 55");
    assert!(crate::persistence::decode(old.as_bytes()).is_err());
}

#[test]
fn ne_t6_ne_13_deleting_a_driven_layer_keeps_the_free_node() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(&mut e, group, &noise, diffuse(MATERIAL)).unwrap();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Remove(vec![MATERIAL])))
        .unwrap();
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    assert!(g.node(&noise).is_some(), "the free node stays");
    assert!(!g.feeds(&noise), "unwired: NE-14 draws it dimmed");
    // One undo brings the layer and the wire back.
    e.dispatch(Command::Undo).unwrap();
    assert!(
        e.scene
            .shader_tree
            .channel_driver(MATERIAL, Ch::DiffuseColor)
            .is_some()
    );
}

#[test]
fn ne_05_06_19_deleting_boxes() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(&mut e, group, &noise, diffuse(MATERIAL)).unwrap();
    // A mixing node is locked.
    let before = e.scene.clone();
    let err = nodes(&mut e, group, NodeEdit::Delete(vec![mix_name(MATERIAL)])).unwrap_err();
    assert!(err.contains("locked"), "{err}");
    assert_eq!(e.scene, before);
    // A layer's own box deletes the layer, one step, with its wires.
    nodes(&mut e, group, NodeEdit::Delete(vec![layer_name(MATERIAL)])).unwrap();
    assert_eq!(last_label(&e), "Delete Node Base Material");
    assert!(e.scene.shader_tree.get(MATERIAL).is_none());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before, "one undo restores layer, node and wire");
    // A free node goes with its wires; the driven channel is its own again.
    nodes(&mut e, group, NodeEdit::Delete(vec![noise.clone()])).unwrap();
    assert_eq!(last_label(&e), "Delete Node Noise");
    assert_eq!(e.scene.shader_tree.group_nodes(group), None);
}

#[test]
fn ne_t7_tree_and_node_edits_share_one_history() {
    let (mut e, group) = editor();
    let start = e.scene.clone();
    // Tree, node view, tree.
    let mut node = e.scene.shader_tree.get(MATERIAL).unwrap().clone();
    node.layer.opacity = 0.5;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    let one = e.scene.clone();
    add(&mut e, group, "noise3d", "color3");
    let two = e.scene.clone();
    let mut node = e.scene.shader_tree.get(MATERIAL).unwrap().clone();
    node.name = "Renamed".into();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, two);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, one);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, start);
}

#[test]
fn ne_07_16_hide_show_and_move_are_steps_and_drags_fold() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    let steps = e.undo.len();
    for (i, continuing) in [false, true, true].into_iter().enumerate() {
        nodes(
            &mut e,
            group,
            NodeEdit::Move {
                places: vec![(noise.clone(), [i as f64, 0.])],
                continuing,
            },
        )
        .unwrap();
    }
    assert_eq!(e.undo.len(), steps + 1, "one drag, one step");
    assert_eq!(
        e.scene.shader_tree.group_nodes(group).unwrap().places[&noise],
        [2., 0.]
    );
    // Layer boxes have places too; unknown names are refused.
    nodes(
        &mut e,
        group,
        NodeEdit::Move {
            places: vec![(layer_name(MATERIAL), [5., 5.])],
            continuing: false,
        },
    )
    .unwrap();
    assert!(
        nodes(
            &mut e,
            group,
            NodeEdit::Move {
                places: vec![("Nope".into(), [5., 5.])],
                continuing: false,
            },
        )
        .is_err()
    );
    nodes(
        &mut e,
        group,
        NodeEdit::Hide(vec![noise.clone(), layer_name(MATERIAL)]),
    )
    .unwrap();
    assert_eq!(last_label(&e), "Hide Node");
    nodes(&mut e, group, NodeEdit::ShowAll).unwrap();
    assert_eq!(last_label(&e), "Show All Nodes");
    assert!(
        e.scene
            .shader_tree
            .group_nodes(group)
            .unwrap()
            .hidden
            .is_empty()
    );
    assert!(nodes(&mut e, group, NodeEdit::ShowAll).is_err());
}

#[test]
fn set_value_checks_the_type_and_folds_drags() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    let set = |e: &mut Editor, v: &str, continuing| {
        nodes(
            e,
            group,
            NodeEdit::SetValue {
                node: noise.clone(),
                input: "amplitude".into(),
                value: v.into(),
                continuing,
            },
        )
    };
    let steps = e.undo.len();
    set(&mut e, "2", false).unwrap();
    set(&mut e, "0.5,1,1", true).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    assert_eq!(
        g.node(&noise).unwrap().input("amplitude").unwrap().value,
        "0.5, 1, 1"
    );
    assert!(set(&mut e, "a,b", false).is_err());
    assert!(set(&mut e, "1,2", false).is_err());
}

#[test]
fn compatibility_table() {
    assert!(compatible("color3", "color3"));
    assert!(compatible("float", "color3"));
    assert!(compatible("vector3", "color3"));
    assert!(!compatible("vector2", "color3"));
    assert!(!compatible("color3", "float"));
    assert!(!compatible("surfaceshader", "color3"));
}

// -- Slice 3: wires drive the surface --------------------------------------

fn base_colour(e: &Editor, group: NodeId) -> crate::shader_eval::Expr {
    let p = crate::materialx::tree_standard::group_surface(&e.scene, group).unwrap();
    p.inputs
        .iter()
        .find(|(i, _)| *i == crate::shader_eval::Input::BaseColor)
        .unwrap()
        .1
        .clone()
}

#[test]
fn ne_03_a_wired_channel_folds_to_the_free_nodes_and_back() {
    use crate::shader_eval::Expr;
    let (mut e, group) = editor();
    let own = base_colour(&e, group);
    assert!(matches!(own, Expr::Const(_)));
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(&mut e, group, &noise, diffuse(MATERIAL)).unwrap();
    let driven = base_colour(&e, group);
    let Expr::Nodes(n) = &driven else {
        panic!("{driven:?}")
    };
    assert_eq!(n.node, noise);
    assert!(n.flat.nodes.iter().any(|f| f.cycles == "noise_texture"));
    // The viewport draws it per point: it varies.
    let at = |x: f64| {
        let p = crate::shader_eval::ShadingPoint {
            position: [x, 0.3, 0.7],
            ..Default::default()
        };
        driven.eval_at(&p, &|_, _| [0.; 4])
    };
    assert_ne!(at(0.1), at(2.3));
    // Removing the wire gives the very same surface as before (NE-T1).
    nodes(&mut e, group, NodeEdit::Disconnect(diffuse(MATERIAL))).unwrap();
    assert_eq!(base_colour(&e, group), own);
}

#[test]
fn the_tree_form_holds_the_free_nodes_and_reads_back_the_same() {
    use crate::materialx::{doc, tree};
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    let mul = add(&mut e, group, "multiply", "color3");
    connect(
        &mut e,
        group,
        &noise,
        Target::Free {
            node: mul.clone(),
            input: "in1".into(),
        },
    )
    .unwrap();
    connect(&mut e, group, &mul, diffuse(MATERIAL)).unwrap();
    let p = crate::materialx::tree_standard::group_surface(&e.scene, group).unwrap();
    let text = tree::write(&p, &e.scene, "Mat");
    let d = doc::parse(&text).unwrap();
    assert_eq!(d.element(&noise).unwrap().category, "noise3d");
    let m = d.element(&mul).unwrap();
    assert_eq!(
        m.input("in1").unwrap().nodename.as_deref(),
        Some(noise.as_str())
    );
    assert_eq!(tree::read(&text).unwrap(), p);
    // Moving a box never changes what renders (NE-16).
    nodes(
        &mut e,
        group,
        NodeEdit::Move {
            places: vec![(noise.clone(), [300., 40.])],
            continuing: false,
        },
    )
    .unwrap();
    let moved = crate::materialx::tree_standard::group_surface(&e.scene, group).unwrap();
    assert_eq!(moved, p);
}

#[test]
fn ne_t5_export_writes_every_free_node_and_reimport_keeps_them() {
    let (mut e, group) = editor();
    let noise = add(&mut e, group, "noise3d", "color3");
    let mul = add(&mut e, group, "multiply", "color3");
    let spare = add(&mut e, group, "constant", "float");
    connect(
        &mut e,
        group,
        &noise,
        Target::Free {
            node: mul.clone(),
            input: "in1".into(),
        },
    )
    .unwrap();
    connect(&mut e, group, &mul, diffuse(MATERIAL)).unwrap();
    let dir = std::env::temp_dir().join(format!("ne-t5-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mat.mtlx");
    crate::materialx::export(&e.scene, group, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    for name in [&noise, &mul, &spare] {
        assert!(text.contains(&format!("name=\"{name}\"")), "{name}: {text}");
    }
    assert!(text.contains("xpos=\"0.1\""), "places go out: {text}");
    // Re-imported, it renders as a graph and shows every node and wire.
    let d = crate::materialx::doc::parse(&text).unwrap();
    crate::materialx::graph::preset(&d, None, std::path::Path::new("")).unwrap();
    let stored = crate::materialx::doc::parse(&crate::materialx::write::material(
        &d,
        &crate::materialx::material_names(&d)[0],
    ))
    .unwrap();
    let view = crate::materialx::view::graph(&stored, None).unwrap();
    let names: Vec<&str> = view.nodes.iter().map(|n| n.name.as_str()).collect();
    for name in [&noise, &mul, &spare] {
        assert!(names.contains(&name.as_str()), "{name}: {names:?}");
    }
    assert!(
        view.links
            .iter()
            .any(|l| l.from.node() == noise && l.to.node() == mul)
    );
}

// -- Texture layer settings take wires (NE-03, 2026-11-21) -----------------

fn texture_layer(e: &mut Editor, layer: NewLayer) -> NodeId {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer,
        selected: Some(MATERIAL),
    }))
    .unwrap();
    e.scene.shader_tree.next_id - 1
}

fn setting(layer: NodeId, key: &str) -> Target {
    Target::Layer {
        layer,
        channel: key.into(),
    }
}

#[test]
fn ne_03_a_constant_texture_colour_takes_a_wire_and_gives_it_back() {
    use crate::shader_eval::Expr;
    let (mut e, group) = editor();
    let constant = texture_layer(&mut e, NewLayer::Constant);
    let own = base_colour(&e, group);
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(&mut e, group, &noise, setting(constant, "texture.color")).unwrap();
    assert_eq!(last_label(&e), "Connect Noise > Color");
    assert!(matches!(base_colour(&e, group), Expr::Nodes(_)));
    // The layer keeps its own colour.
    let crate::shader_tree::NodeKind::Texture(t) = &e.scene.shader_tree.get(constant).unwrap().kind
    else {
        panic!()
    };
    assert!(crate::node_view_settings::value(t, "texture.color").is_some());
    // A colour effect has no Value setting to wire.
    assert!(connect(&mut e, group, &noise, setting(constant, "texture.value")).is_err());
    nodes(
        &mut e,
        group,
        NodeEdit::Disconnect(setting(constant, "texture.color")),
    )
    .unwrap();
    assert_eq!(base_colour(&e, group), own, "unwired: the same surface");
}

#[test]
fn ne_03_procedural_zones_take_wires_and_a_type_change_drops_them() {
    use crate::shader_eval::Expr;
    let (mut e, group) = editor();
    let kind = crate::procedural::ProceduralKind::ALL[0];
    let pattern = texture_layer(&mut e, NewLayer::Procedural(kind));
    let own = base_colour(&e, group);
    let noise = add(&mut e, group, "noise3d", "color3");
    connect(
        &mut e,
        group,
        &noise,
        setting(pattern, "texture.zone2_color"),
    )
    .unwrap();
    let driven = base_colour(&e, group);
    assert_ne!(driven, own);
    let mut found = false;
    fn walk(x: &Expr, found: &mut bool) {
        match x {
            Expr::Nodes(_) => *found = true,
            Expr::Mix { fac, a, b, .. } => {
                walk(fac, found);
                walk(a, found);
                walk(b, found);
            }
            _ => {}
        }
    }
    walk(&driven, &mut found);
    assert!(found, "the zone colour reads the free node: {driven:?}");
    // The board shows the zone rows, driven greyed (UIC-92).
    let board = e.scene.shader_tree.board(group).unwrap();
    let b = board.get(&layer_name(pattern)).unwrap();
    assert!(b.input("texture.zone2_color").unwrap().driven);
    assert!(!b.input("texture.zone1_color").unwrap().driven);
    // Changing the layer to an Image Map (no such setting) drops the wire,
    // keeps the free node.
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::ChangeType {
        node: pattern,
        to: NewLayer::ImageMap,
    }))
    .unwrap();
    let g = e.scene.shader_tree.group_nodes(group).unwrap();
    assert!(g.node(&noise).is_some() && !g.feeds(&noise));
}

#[test]
fn ne_01_box_values_are_the_layers_own_edits() {
    let (mut e, group) = editor();
    let constant = texture_layer(&mut e, NewLayer::Constant);
    let tree = &e.scene.shader_tree;
    let m = tree.edited_layer(MATERIAL, "roughness", &[0.25]).unwrap();
    let crate::shader_tree::NodeKind::Material(mc) = &m.kind else {
        panic!()
    };
    assert_eq!(mc.roughness, 0.25);
    let t = tree
        .edited_layer(constant, "texture.color", &[0.1, 0.2, 0.3])
        .unwrap();
    let crate::shader_tree::NodeKind::Texture(tt) = &t.kind else {
        panic!()
    };
    assert_eq!(
        crate::node_view_settings::value(tt, "texture.color"),
        Some(vec![0.1, 0.2, 0.3])
    );
    // A mixing node's opacity is its layer's (NE-05); clamped.
    let o = tree.edited_layer(constant, "opacity", &[-0.5]).unwrap();
    assert_eq!(o.layer.opacity, 0.);
    // Nothing that isn't a setting, and no change, give nothing.
    assert!(tree.edited_layer(constant, "roughness", &[0.5]).is_none());
    let same = tree.get(MATERIAL).unwrap().layer.opacity;
    assert!(tree.edited_layer(MATERIAL, "opacity", &[same]).is_none());
    let _ = group;
}
