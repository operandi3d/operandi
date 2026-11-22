//! MeshOps suite batch 3: selection operations (MO-50..53) and Select by
//! Previous Operation groups (PG-xx).
use operandi_core::meshop_selection::{Blend, SelKind, SelOp};
use operandi_core::meshops::{OpStatus, Runtime};
use operandi_core::meshops_editor::{OpRef, SelOpEdit};
use operandi_core::meshops_items::BOOLEAN_OP;
use operandi_core::{Command, Editor, Mesh, persistence};
use serde_json::{Value, json};

fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let id = e.scene.items[0].id;
    (e, id)
}
fn add(e: &mut Editor, item: u64, type_id: &str, values: Value) -> OpRef {
    e.scene.select_item(Some(item));
    e.dispatch(Command::MeshOpCreate(type_id.into())).unwrap();
    let op = OpRef {
        item,
        op: e
            .scene
            .item(item)
            .unwrap()
            .mesh_ops
            .as_ref()
            .unwrap()
            .ops
            .last()
            .unwrap()
            .id,
    };
    let values = values.as_object().unwrap().clone();
    if !values.is_empty() {
        e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    }
    op
}
fn selop(e: &mut Editor, op: OpRef, s: SelOp) -> operandi_core::Result<()> {
    e.dispatch(Command::MeshOpSelOp {
        op,
        edit: SelOpEdit::Add(s),
    })
}
fn previous(source: OpRef, name: &str) -> SelOp {
    SelOp::new(SelKind::Previous {
        source: source.op,
        name: name.into(),
    })
}
fn result(e: &Editor, item: u64) -> operandi_core::meshops::Evaluated {
    e.meshops
        .current(&e.scene, e.scene.item(item).unwrap())
        .unwrap()
}
fn faces(e: &Editor, item: u64) -> usize {
    result(e, item).mesh.faces.len()
}
const BEVEL: &str = "poly.bevel.item";
const TRI: &str = "pmodel.triangulate.item";

#[test]
fn pg_03_a_cube_s_groups_pick_by_short_id_case_sensitive() {
    for (name, expect) in [
        ("capA", 6 + 4),   // the top bevelled: 4 side polygons more
        ("capB", 6 + 4),   // the bottom
        ("caps", 6 + 8),   // both
        ("sides", 6 + 16), // the four sides
        ("all", 6 + 24),   // every polygon
        ("Top", 6),        // a label selects nothing (PG-03)
        ("Caps", 6),       // case-sensitive
        ("bogus", 6),      // unknown: nothing, no error (PG-04)
    ] {
        let (mut e, id) = editor();
        let cube = add(&mut e, id, "prim.cube.item", json!({}));
        let bevel = add(
            &mut e,
            id,
            BEVEL,
            json!({ "inset": 0.1, "shift": 0.1, "group": false }),
        );
        selop(&mut e, bevel, previous(cube, name)).unwrap();
        let r = result(&e, id);
        assert_eq!(r.statuses, [OpStatus::Ok, OpStatus::Ok], "{name}");
        assert_eq!(r.mesh.faces.len(), expect, "{name}");
    }
}

#[test]
fn groups_never_leak_into_the_result() {
    let (mut e, id) = editor();
    let cube = add(&mut e, id, "prim.cube.item", json!({}));
    let bevel = add(&mut e, id, BEVEL, json!({ "inset": 0.1 }));
    selop(&mut e, bevel, previous(cube, "caps")).unwrap();
    let r = result(&e, id);
    assert!(
        r.mesh.attributes.polygon_sets.is_empty(),
        "{:?}",
        r.mesh.attributes.polygon_sets.keys()
    );
    // Freeze bakes the same clean mesh.
    e.dispatch(Command::MeshOpFreeze {
        op: bevel,
        duplicate: false,
    })
    .unwrap();
    assert!(
        e.scene
            .item(id)
            .unwrap()
            .mesh
            .attributes
            .polygon_sets
            .is_empty()
    );
}

#[test]
fn groups_follow_the_polygons_through_ops_in_between() {
    let (mut e, id) = editor();
    let cube = add(&mut e, id, "prim.cube.item", json!({}));
    // Triangulating the sides first renumbers everything.
    let tri = add(&mut e, id, TRI, json!({}));
    selop(&mut e, tri, previous(cube, "sides")).unwrap();
    assert_eq!(faces(&e, id), 2 + 8);
    let bevel = add(
        &mut e,
        id,
        BEVEL,
        json!({ "inset": 0.1, "shift": 0.1, "group": false }),
    );
    selop(&mut e, bevel, previous(cube, "capA")).unwrap();
    // Only the top quad bevels: its 4 new side polygons.
    assert_eq!(faces(&e, id), 10 + 4);
}

#[test]
fn array_copies_are_clone1_to_clone_n() {
    let (mut e, id) = editor();
    add(&mut e, id, "prim.cube.item", json!({}));
    let array = add(
        &mut e,
        id,
        "array.clone.item",
        json!({ "numX": 4, "numY": 1, "numZ": 1, "offX": 2.0 }),
    );
    let tri = add(&mut e, id, TRI, json!({}));
    for (name, expect) in [
        ("clone2", 24 + 6),
        ("clone", 24 + 18),
        ("clone0", 24),
        ("clone4", 24),
    ] {
        let s = previous(array, name);
        e.dispatch(Command::MeshOpSelOp {
            op: tri,
            edit: SelOpEdit::Remove(0),
        })
        .ok();
        selop(&mut e, tri, s).unwrap();
        assert_eq!(faces(&e, id), expect, "{name}");
    }
}

#[test]
fn boolean_groups_geometry_and_driver() {
    let (mut e, a) = editor();
    e.scene.item_mut(a).unwrap().mesh =
        operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::splat(2.))
            .unwrap()
            .into();
    e.scene.new_mesh().unwrap();
    let b = e.scene.items[1].id;
    e.scene.item_mut(b).unwrap().mesh =
        operandi_core::cube_geometry(glam::DVec3::ONE, glam::DVec3::splat(2.))
            .unwrap()
            .into();
    let boolean = add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": b }));
    let tri = add(&mut e, a, TRI, json!({}));
    selop(&mut e, tri, previous(boolean, "driver")).unwrap();
    // 9 after Subtract; the 3 driver quads become 6 triangles.
    assert_eq!(faces(&e, a), 12);
    let s = SelOp::new(SelKind::Previous {
        source: boolean.op,
        name: "geometry".into(),
    });
    e.dispatch(Command::MeshOpSelOp {
        op: tri,
        edit: SelOpEdit::Set(0, s),
    })
    .unwrap();
    // 6 from A: 3 whole quads and 3 L-shaped hexagons.
    assert!(faces(&e, a) > 12);
}

#[test]
fn range_pattern_invert_and_blend_modes() {
    let (mut e, id) = editor();
    add(&mut e, id, "prim.cube.item", json!({}));
    let tri = add(&mut e, id, TRI, json!({}));
    // Pattern 1,0: polygons 0, 2, 4.
    selop(
        &mut e,
        tri,
        SelOp::new(SelKind::Pattern {
            pattern: "1,0".into(),
        }),
    )
    .unwrap();
    assert_eq!(faces(&e, id), 3 + 6);
    // + Range 1..=1 added: 0, 1, 2, 4.
    let mut range = SelOp::new(SelKind::Range {
        start: 1,
        end: 1,
        step: 1,
    });
    range.blend = Blend::Add;
    selop(&mut e, tri, range.clone()).unwrap();
    assert_eq!(faces(&e, id), 2 + 8);
    // Range 0..=5 step 2, subtracted: 1 only.
    let mut sub = SelOp::new(SelKind::Range {
        start: 0,
        end: 5,
        step: 2,
    });
    sub.blend = Blend::Subtract;
    selop(&mut e, tri, sub).unwrap();
    assert_eq!(faces(&e, id), 5 + 2);
    // Inverted Index 1, multiplied: nothing left, the op does nothing.
    let mut mul = SelOp::new(SelKind::Index { index: "1".into() });
    mul.invert = true;
    mul.blend = Blend::Multiply;
    selop(&mut e, tri, mul).unwrap();
    assert_eq!(result(&e, id).statuses[1], OpStatus::Ok);
    assert_eq!(faces(&e, id), 6);
    // Turning that one off brings polygon 1 back.
    e.dispatch(Command::MeshOpSelOp {
        op: tri,
        edit: SelOpEdit::Set(3, {
            let mut m = SelOp::new(SelKind::Index { index: "1".into() });
            m.enable = false;
            m
        }),
    })
    .unwrap();
    assert_eq!(faces(&e, id), 5 + 2);
}

#[test]
fn previous_must_name_an_op_lower_in_the_stack() {
    let (mut e, id) = editor();
    let tri = add(&mut e, id, TRI, json!({}));
    let cube = add(&mut e, id, "prim.cube.item", json!({}));
    let before = e.scene.clone();
    assert!(selop(&mut e, tri, previous(cube, "caps")).is_err());
    assert_eq!(e.scene, before);
    // A primitive has no Selection input.
    assert!(selop(&mut e, cube, previous(tri, "x")).is_err());
}

#[test]
fn scripts_add_edit_and_remove_selection_ops() {
    let (mut e, id) = editor();
    let cube = add(&mut e, id, "prim.cube.item", json!({}));
    let tri = add(&mut e, id, TRI, json!({}));
    let run = |e: &mut Editor, name: &str, a: Value| {
        operandi_core::command_exec::run(e, name, a.as_object().unwrap())
    };
    run(
        &mut e,
        "selop.create",
        json!({ "type": "previous.selop", "item": tri.text() }),
    )
    .unwrap();
    run(
        &mut e,
        "selop.edit",
        json!({ "item": tri.text(), "index": 0, "values": { "source": cube.op, "name": "caps" } }),
    )
    .unwrap();
    assert_eq!(faces(&e, id), 4 + 4);
    assert!(
        run(
            &mut e,
            "selop.edit",
            json!({ "item": tri.text(), "index": 0, "values": { "nope": 1 } })
        )
        .is_err()
    );
    run(
        &mut e,
        "selop.edit",
        json!({ "item": tri.text(), "index": 0, "remove": true }),
    )
    .unwrap();
    assert_eq!(faces(&e, id), 12);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(faces(&e, id), 8);
}

#[test]
fn selection_ops_save_as_version_50_and_reopen_identical() {
    let (mut e, id) = editor();
    let cube = add(&mut e, id, "prim.cube.item", json!({}));
    let bevel = add(&mut e, id, BEVEL, json!({ "inset": 0.1 }));
    selop(&mut e, bevel, previous(cube, "capA")).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let doc: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["version"], 50);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    let theirs = Runtime::default()
        .current(&back, back.item(id).unwrap())
        .unwrap();
    assert_eq!(*theirs.mesh, *result(&e, id).mesh);
    let _ = Mesh::default();
}
