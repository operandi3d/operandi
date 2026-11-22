use crate::node_graph::{self, StoredGraph};
use crate::shader_eval::{Context, Expr};
use crate::shader_tree::{NewLayer, NodeKind, ShaderTreeEdit, TextureSource};
use crate::{Command, Editor};

const NOISE_RAMP: &str = include_str!("../../../fixtures/node-graphs/noise_ramp.json");

fn graphs() -> Vec<StoredGraph> {
    node_graph::import(NOISE_RAMP, None).unwrap()
}

/// A plane with the Noise Ramp graph imported; the new layer's id.
fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::ImportNodeGraphs {
        graphs: graphs(),
        images: vec![],
        selected: None,
    }))
    .unwrap();
    let id = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::NodeMaterial(_)))
        .unwrap()
        .id;
    (e, id)
}

fn surface(e: &Editor) -> crate::shader_eval::Principled {
    let item = e.scene.selection.item.unwrap();
    let frames = e.scene.locator_frames(item);
    let ctx = Context {
        item,
        ancestors: &[],
        material: crate::material::DEFAULT_MATERIAL,
        part: None,
        sets: &[],
        locators: &frames,
    };
    e.scene.render_shader_tree().evaluate(&ctx).principled()
}

fn set_opacity(e: &mut Editor, id: u64, opacity: f64, continuing: bool) {
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    node.layer.opacity = opacity;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing,
    }))
    .unwrap();
}

#[test]
fn import_adds_the_graph_and_a_node_material_layer_as_one_step() {
    let (mut e, id) = editor();
    let tree = &e.scene.shader_tree;
    assert_eq!(tree.node_graphs.graphs.len(), 1);
    let layer = tree.get(id).unwrap();
    assert_eq!(layer.name, "Noise Ramp");
    assert_eq!(layer.type_label(), "Node Material");
    assert_eq!(tree.effect_label(layer), "Noise Ramp");
    let n = surface(&e).node.expect("the graph reaches the plane");
    assert_eq!(n.coverage, Expr::scalar(1.));
    assert_eq!(n.values["Tint"], vec![1., 1., 1.]);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.shader_tree.node_graphs.is_empty());
    assert!(surface(&e).node.is_none());
}

#[test]
fn nm_t2_half_opacity_mixes_and_one_undo_restores_full() {
    let (mut e, id) = editor();
    // An opacity drag: several continuing edits, one undo step.
    set_opacity(&mut e, id, 0.8, false);
    set_opacity(&mut e, id, 0.6, true);
    set_opacity(&mut e, id, 0.5, true);
    assert_eq!(surface(&e).node.unwrap().coverage, Expr::scalar(0.5));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(surface(&e).node.unwrap().coverage, Expr::scalar(1.));
    // At 0 % nothing of the graph shows: the layers render alone.
    set_opacity(&mut e, id, 0., false);
    assert!(surface(&e).node.is_none());
}

#[test]
fn nm_t3_a_texture_above_drives_an_exposed_input() {
    let (mut e, id) = editor();
    assert_eq!(
        e.scene
            .shader_tree
            .node_inputs_for(e.scene.shader_tree.nodes[0].id)
            .iter()
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>(),
        ["Tint", "Roughness"]
    );
    // A Constant layer added above the Node Material, driving Tint red.
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Constant,
        selected: Some(id),
    }))
    .unwrap();
    let tex = e.scene.shader_tree.next_id - 1;
    let mut node = e.scene.shader_tree.get(tex).unwrap().clone();
    if let NodeKind::Texture(t) = &mut node.kind {
        t.node_input = Some(Box::new("Tint".into()));
        t.source = TextureSource::Constant {
            color: [1., 0., 0.],
            value: 1.,
        };
    }
    node.layer.opacity = 0.5;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    // The layer must sit above the Node Material for the order to count.
    let order: Vec<_> = e.scene.shader_tree.nodes.iter().map(|n| n.id).collect();
    let (ti, ni) = (
        order.iter().position(|&x| x == tex).unwrap(),
        order.iter().position(|&x| x == id).unwrap(),
    );
    assert!(ti < ni, "texture above the node material");
    let n = surface(&e).node.unwrap();
    let tint = n.driver("Tint").expect("Tint is driven");
    // Half way from the default white to red.
    assert_eq!(tint.constant(), Some([1., 0.5, 0.5]));
    assert!(n.driver("Roughness").is_none());
    // The Effect column names the input.
    let tree = &e.scene.shader_tree;
    assert_eq!(tree.effect_label(tree.get(tex).unwrap()), "Tint");
}

#[test]
fn nm_t4_an_unsupported_graph_is_refused_and_the_scene_is_unchanged() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let before = e.scene.clone();
    let text = NOISE_RAMP.replace("ShaderNodeValToRGB", "ShaderNodeHueSaturation");
    let err = node_graph::import(&text, None).unwrap_err();
    assert!(
        err.contains("Color Ramp") && err.contains("ShaderNodeHueSaturation"),
        "{err}"
    );
    assert_eq!(e.scene, before);
}

#[test]
fn shared_defaults_follow_unless_a_layer_overrides() {
    let (mut e, a) = editor();
    let graph = e.scene.shader_tree.node_graphs.graphs[0].id;
    // A second layer on the same graph (NM-04).
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::NodeMaterial,
        selected: None,
    }))
    .unwrap();
    let b = e.scene.shader_tree.next_id - 1;
    let mut node = e.scene.shader_tree.get(b).unwrap().clone();
    if let NodeKind::NodeMaterial(m) = &mut node.kind {
        m.graph = Some(graph);
        m.overrides.insert("Roughness".into(), vec![0.9]);
    }
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetNodeGraphDefault {
        graph,
        input: "Roughness".into(),
        value: vec![0.2],
        continuing: false,
    }))
    .unwrap();
    let tree = &e.scene.shader_tree;
    let g = tree.node_graphs.get(graph).unwrap();
    let rough = g.exposed("Roughness").unwrap();
    let value = |id| match &tree.get(id).unwrap().kind {
        NodeKind::NodeMaterial(m) => crate::shader_tree::ShaderTree::exposed_value(m, rough),
        _ => unreachable!(),
    };
    assert_eq!(value(a), vec![0.2]);
    assert_eq!(value(b), vec![0.9]);
    // Out of range is refused.
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetNodeGraphDefault {
            graph,
            input: "Roughness".into(),
            value: vec![2.],
            continuing: false,
        }))
        .is_err()
    );
}

#[test]
fn non_normal_blend_and_dangling_graphs_are_refused() {
    let (mut e, id) = editor();
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    node.layer.blend = crate::shader_tree::BlendMode::Multiply;
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false
        }))
        .is_err()
    );
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    if let NodeKind::NodeMaterial(m) = &mut node.kind {
        m.graph = Some(99);
    }
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false
        }))
        .is_err()
    );
}

#[test]
fn nm_t5_save_and_reopen_keeps_graphs_values_and_overrides() {
    let (mut e, id) = editor();
    let mut node = e.scene.shader_tree.get(id).unwrap().clone();
    if let NodeKind::NodeMaterial(m) = &mut node.kind {
        m.overrides.insert("Tint".into(), vec![0.2, 0.4, 0.6]);
    }
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
    .unwrap();
    // A pre-OpenPBR material, so the older version is what is tested.
    e.scene.shader_tree.legacy_openpbr();
    let bytes = crate::persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 38"));
    // The graph is stored in the shared format's shape.
    assert!(text.contains("\"type\": \"ShaderNodeTexNoise\""));
    assert!(text.contains("\"node_graphs\""));
    let back = crate::persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    // An older version number can't carry them.
    let old = text.replace("\"version\": 38", "\"version\": 37");
    assert!(crate::persistence::decode(old.as_bytes()).is_err());
}

#[test]
fn nm_t5_scenes_without_node_materials_save_as_before() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let text = String::from_utf8(crate::persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!text.contains("node_graphs"));
    assert!(!text.contains("\"version\": 38"));
}

#[test]
fn unknown_node_types_load_kept_and_flagged_and_dont_render() {
    let (e, _) = editor();
    let text = String::from_utf8(crate::persistence::encode(&e.scene).unwrap()).unwrap();
    // A later build's node type in the saved graph.
    let text = text.replacen(
        "\"type\": \"ShaderNodeTexNoise\"",
        "\"type\": \"ShaderNodeTexFuture\"",
        1,
    );
    let scene = crate::persistence::decode(text.as_bytes()).unwrap();
    let g = &scene.shader_tree.node_graphs.graphs[0];
    assert_eq!(
        g.unsupported(),
        vec![(
            "Noise Texture".to_string(),
            "ShaderNodeTexFuture".to_string()
        )]
    );
    // Saved again, it is still there.
    let again = String::from_utf8(crate::persistence::encode(&scene).unwrap()).unwrap();
    assert!(again.contains("ShaderNodeTexFuture"));
    let e2 = Editor {
        scene,
        ..Default::default()
    };
    assert!(surface(&e2).node.is_none());
}

#[test]
fn copy_paste_and_presets_bring_the_graph() {
    let (e, id) = editor();
    let clip = e.scene.copy_shader_layers(&[id]);
    assert_eq!(clip.graphs.len(), 1);
    // Into a scene without it.
    let mut other = Editor::default();
    other.scene.new_mesh().unwrap();
    other
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Paste {
            clip: Box::new(clip.clone()),
            selected: None,
        }))
        .unwrap();
    assert_eq!(other.scene.shader_tree.node_graphs.graphs.len(), 1);
    assert!(surface(&other).node.is_some());
    // Pasting again shares the identical graph.
    other
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Paste {
            clip: Box::new(clip),
            selected: None,
        }))
        .unwrap();
    assert_eq!(other.scene.shader_tree.node_graphs.graphs.len(), 1);
}

#[test]
fn nm_17_a_material_preset_embeds_the_graph() {
    let (mut e, id) = editor();
    // Put the Node Material in a Material Group and save that group.
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = e.scene.shader_tree.next_id - 1;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
        ids: vec![id],
        drop: crate::shader_tree::Drop {
            parent: Some(group),
            index: 0,
        },
    }))
    .unwrap();
    let built = crate::preset::PresetSource::Material(group)
        .build(&e.scene)
        .unwrap();
    assert!(built.preset.has_nodes());
    // Through a file and into another scene's group.
    let dir = std::env::temp_dir().join(format!("nm17-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("noise.oppreset");
    crate::persistence::save_preset(&path, &built.preset).unwrap();
    let preset = crate::persistence::load_preset(&path).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let mut other = Editor::default();
    other.scene.new_mesh().unwrap();
    other
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Group,
            selected: None,
        }))
        .unwrap();
    let target = other.scene.shader_tree.next_id - 1;
    other.load_preset(preset, Some(target)).unwrap();
    assert_eq!(other.scene.shader_tree.node_graphs.graphs.len(), 1);
    assert!(surface(&other).node.is_some());
}

#[test]
fn nm_40_the_viewport_colour_is_the_plain_base_colour_or_grey() {
    let (e, _) = editor();
    let n = surface(&e).node.unwrap();
    // Noise Ramp feeds Base Color through links: grey.
    assert_eq!(e.scene.shader_tree.node_viewport_colour(&n), [0.5; 3]);
    let text = r#"{"format":"operandi-node-graph","format_version":1,"materials":[{
      "name":"Plain","nodes":[
        {"name":"P","type":"ShaderNodeBsdfPrincipled","inputs":{"Base Color":{"type":"RGBA","value":[0.2,0.3,0.4,1]}}},
        {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
      "links":[{"from":["P","BSDF"],"to":["Material Output","Surface"]}]}]}"#;
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::ImportNodeGraphs {
        graphs: node_graph::import(text, None).unwrap(),
        images: vec![],
        selected: None,
    }))
    .unwrap();
    let n = surface(&e).node.unwrap();
    assert_eq!(
        e.scene.shader_tree.node_viewport_colour(&n),
        [0.2, 0.3, 0.4]
    );
}
