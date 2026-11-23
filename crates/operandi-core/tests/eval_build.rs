//! A scene's evaluation graph (EVAL-GRAPH-PROPOSAL step 2): channels read
//! Time only when they change over time, a Mesh Stack's shown mesh reads
//! its links, and the structure fingerprint changes only with structure.
use operandi_core::eval_build::{build, structure};
use operandi_core::eval_graph::NodeKey;
use operandi_core::mesh_stack_editor::OpRef;
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

/// Two mesh items, the first keyed: pos.Y moving, pos.X flat.
fn keyed() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let a = e.scene.items.last().unwrap().id;
    e.scene.new_mesh().unwrap();
    let b = e.scene.items.last().unwrap().id;
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:10 value:5"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.X frame:0 value:1"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.X frame:10 value:1"),
    );
    (e, a, b)
}

#[test]
fn moving_channels_read_time_and_flat_ones_do_not() {
    let (e, a, _) = keyed();
    let g = build(&e.scene);
    let time = g.id(&NodeKey::Time).unwrap();
    let y = g.id(&NodeKey::Channel(a, "pos.Y".into())).unwrap();
    let x = g.id(&NodeKey::Channel(a, "pos.X".into())).unwrap();
    assert_eq!(g.inputs(y), [time]);
    assert!(g.inputs(x).is_empty(), "two equal keys never change");
    // Field ← Channel, Transform ← both fields.
    let fy = g.id(&NodeKey::Field(a, "pos.Y".into())).unwrap();
    assert_eq!(g.inputs(fy), [y]);
    let t = g.id(&NodeKey::Transform(a)).unwrap();
    assert_eq!(g.inputs(t).len(), 2);
    // A time change reaches pos.Y, its field and the transform only.
    let mut g = g;
    g.pull(t, &mut |_, _| operandi_core::eval_build::Value::Number(0.));
    g.set(time, operandi_core::eval_build::Value::Number(1.));
    let mut want = vec![y, fy, t];
    want.sort();
    assert_eq!(g.dirty_nodes(), want);
}

#[test]
fn a_mesh_stack_reads_its_links_and_their_transforms() {
    let (mut e, a, b) = keyed();
    e.scene.select_item(Some(b));
    e.dispatch(Command::MeshOpCreate("boolean.meshop.item".into()))
        .unwrap();
    let op = e.scene.item(b).unwrap().mesh_stack.as_ref().unwrap().ops[0].id;
    let mut values = serde_json::Map::new();
    values.insert("driver".into(), a.into());
    e.dispatch(Command::MeshOpSet {
        op: OpRef { item: b, op },
        values,
    })
    .unwrap();
    let g = build(&e.scene);
    let shown = g.id(&NodeKey::ShownMesh(b)).unwrap();
    let inputs: Vec<&NodeKey> = g.inputs(shown).iter().map(|&n| g.key(n)).collect();
    assert!(inputs.contains(&&NodeKey::Transform(b)), "{inputs:?}");
    assert!(inputs.contains(&&NodeKey::ShownMesh(a)), "{inputs:?}");
    assert!(inputs.contains(&&NodeKey::Transform(a)), "{inputs:?}");
    // Moving a (a key on its pos.Y) reaches b's shown mesh.
    let y = g.id(&NodeKey::Channel(a, "pos.Y".into())).unwrap();
    assert!(g.reaches(y, shown));
}

#[test]
fn the_structure_changes_with_channels_and_links_not_with_values() {
    let (mut e, a, _) = keyed();
    let s = structure(&e.scene);
    // A new value on a moving key: same structure.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:10 value:7"),
    );
    assert_eq!(structure(&e.scene), s);
    // pos.X starts moving: it now reads Time.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.X frame:10 value:3"),
    );
    let s2 = structure(&e.scene);
    assert_ne!(s2, s);
    // A new channel.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:rot.Z frame:0 value:0"),
    );
    assert_ne!(structure(&e.scene), s2);
}

#[test]
fn an_empty_scene_has_just_time() {
    let g = build(&Scene::default());
    assert_eq!(g.len(), 1);
    assert_eq!(g.key(g.id(&NodeKey::Time).unwrap()), &NodeKey::Time);
}

/// Step 5: each document keeps its own graph, rebuilt only when its
/// structure changes; the graph is no part of undo or editor equality.
#[test]
fn each_document_keeps_its_graph_and_it_is_not_document_data() {
    let (mut e, a, _) = keyed();
    let g1 = e.eval.graph(&e.scene);
    // A value edit keeps the cached graph (same Arc).
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:10 value:7"),
    );
    assert!(std::sync::Arc::ptr_eq(&g1, &e.eval.graph(&e.scene)));
    // A new channel rebuilds it.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:rot.Z frame:0 value:0"),
    );
    let g2 = e.eval.graph(&e.scene);
    assert!(!std::sync::Arc::ptr_eq(&g1, &g2));
    assert!(g2.id(&NodeKey::Channel(a, "rot.Z".into())).is_some());
    // Undo is about the scene; the graph follows the restored structure.
    e.dispatch(Command::Undo).unwrap();
    assert!(
        e.eval
            .graph(&e.scene)
            .id(&NodeKey::Channel(a, "rot.Z".into()))
            .is_none()
    );
    // Another scene tab has its own graph.
    let mut other = Editor::default();
    e.swap_document_state(&mut other);
    assert!(
        e.eval
            .graph(&e.scene)
            .id(&NodeKey::Channel(a, "pos.Y".into()))
            .is_none()
    );
    e.swap_document_state(&mut other);
    assert!(e.eval.cached_for(&e.scene));
    // Equality ignores it.
    let mut copy = e.clone();
    copy.eval = Default::default();
    assert!(copy == e);
}

#[test]
fn the_eval_graph_query_lists_nodes_and_edges() {
    let (e, a, _) = keyed();
    let text = operandi_core::command_exec::answer(&e, "operandi.eval.graph", &Default::default())
        .expect("a query")
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let nodes = v["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["kind"] == "time"));
    assert!(
        nodes
            .iter()
            .any(|n| n["kind"] == "channel" && n["item"] == a && n["name"] == "pos.Y")
    );
    assert!(!v["edges"].as_array().unwrap().is_empty());
    // As a typed line too, and one item's part.
    let mut e = e;
    let one = command_line::run_line(&mut e, &format!("operandi.eval.graph item:{a}"))
        .unwrap()
        .unwrap()
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&one).unwrap();
    assert!(
        v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["item"] == a)
    );
}

/// Rig pieces and parents are edges of the graph (Schematic data, RG-70).
#[test]
fn rig_pieces_and_parents_are_edges() {
    let (mut e, a, b) = keyed();
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{b} targets:{a}"),
    );
    let g = build(&e.scene);
    let ta = g.id(&NodeKey::Transform(a)).unwrap();
    let tb = g.id(&NodeKey::Transform(b)).unwrap();
    assert!(g.inputs(tb).contains(&ta));
    // a's keyed pos.Y reaches b through the constraint.
    let y = g.id(&NodeKey::Channel(a, "pos.Y".into())).unwrap();
    assert!(g.reaches(y, tb));
}
