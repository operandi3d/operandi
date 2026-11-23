//! A scene's evaluation graph (EVAL-GRAPH-PROPOSAL step 2): channels read
//! Time only when they change over time, a Mesh Stack's shown mesh reads
//! its links, and the structure fingerprint changes only with structure.
use operandi_core::eval_build::{build, structure};
use operandi_core::eval_graph::NodeKey;
use operandi_core::meshops_editor::OpRef;
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
    let op = e.scene.item(b).unwrap().mesh_ops.as_ref().unwrap().ops[0].id;
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
