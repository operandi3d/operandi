//! MeshOps framework: the op type table kits add to (KIT-1, K7) and item
//! links between stacks (SAFE-07..09). A test op type registered here the
//! way a kit would register one, `test.gather.item`, appends its linked
//! items' meshes, placed in its own item's space.
use glam::DVec3;
use operandi_core::meshop_types::{
    Field, FieldKind, OpContext, Typed, TypedOp, field_links, op_def, register,
};
use operandi_core::meshops::{MISSING_INPUT, OpKind, OpStatus, Runtime, find_loop};
use operandi_core::meshops_editor::OpRef;
use operandi_core::{Command, Editor, Mesh, Result, persistence};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Once};

const GATHER: &str = "test.gather.item";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gather {
    sources: Vec<u64>,
}
impl TypedOp for Gather {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut out = ctx.input.clone();
        for s in ctx.sources()? {
            operandi_core::append_mesh(&mut out, s.local_mesh())?;
        }
        Ok(out)
    }
}

fn register_gather() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register(Arc::new(Typed::<Gather>::new(
            GATHER,
            "Gather",
            "Test",
            false,
            vec![Field::new("sources", "Source", FieldKind::Items)],
        )))
        .unwrap();
    });
}

/// An editor with mesh A (a Cube op, so it has a stack) and mesh B (plain,
/// empty), B selected.
fn two_meshes() -> (Editor, u64, u64) {
    register_gather();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let a = e.scene.items.last().unwrap().id;
    e.dispatch(Command::MeshOpCreate("prim.cube.item".into()))
        .unwrap();
    e.scene.new_mesh().unwrap();
    let b = e.scene.items.last().unwrap().id;
    e.scene.select_item(Some(b));
    (e, a, b)
}
fn top(e: &Editor, item: u64) -> OpRef {
    OpRef {
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
    }
}
fn link(e: &mut Editor, op: OpRef, ids: &[u64]) -> Result<()> {
    let mut v = serde_json::Map::new();
    v.insert("sources".into(), ids.to_vec().into());
    e.dispatch(Command::MeshOpSet { op, values: v })
}
fn gather_on(e: &mut Editor, item: u64, ids: &[u64]) -> OpRef {
    e.scene.select_item(Some(item));
    e.dispatch(Command::MeshOpCreate(GATHER.into())).unwrap();
    let op = top(e, item);
    link(e, op, ids).unwrap();
    op
}
fn result(e: &Editor, item: u64) -> operandi_core::meshops::Evaluated {
    e.meshops
        .current(&e.scene, e.scene.item(item).unwrap())
        .unwrap()
}

#[test]
fn a_kit_op_type_registers_and_runs_like_a_built_in() {
    register_gather();
    let def = op_def(GATHER).unwrap();
    assert_eq!(
        def.links(
            &serde_json::json!({ "sources": [3, 0, 5] }),
            &operandi_core::Scene::default()
        ),
        [3, 5]
    );
    assert_eq!(
        field_links(def.fields(), &serde_json::json!({ "sources": [] })),
        Vec::<u64>::new()
    );
    // Ids must be dotted and new; every value needs a form field (KIT-4).
    let again = register(Arc::new(Typed::<Gather>::new(
        GATHER,
        "Again",
        "Test",
        false,
        vec![Field::new("sources", "Source", FieldKind::Items)],
    )));
    assert!(again.is_err());
    let undotted = register(Arc::new(Typed::<Gather>::new(
        "gather",
        "G",
        "Test",
        false,
        vec![],
    )));
    assert!(undotted.is_err());
    let no_field = register(Arc::new(Typed::<Gather>::new(
        "test.nofield.item",
        "G",
        "Test",
        false,
        vec![],
    )));
    assert!(no_field.unwrap_err().contains("no form field"));
}

#[test]
fn a_link_reads_the_other_items_result_in_this_items_space() {
    let (mut e, a, b) = two_meshes();
    e.scene.item_mut(a).unwrap().transform.translation = DVec3::new(10., 0., 0.);
    e.scene.item_mut(b).unwrap().transform.translation = DVec3::new(4., 0., 0.);
    gather_on(&mut e, b, &[a]);
    let r = result(&e, b);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    // A's cube (from its own Cube op), 6 units to B's right in B's space.
    assert_eq!(r.mesh.faces.len(), 6);
    let x = r.mesh.vertices.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((x - 6.5).abs() < 1e-9, "{x}");
}

#[test]
fn editing_the_source_reruns_the_reader_and_nothing_else_does() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    result(&e, b);
    let runs = e.meshops.op_runs();
    result(&e, b);
    assert_eq!(e.meshops.op_runs(), runs, "cached");
    // A's cube gets bigger: A's op and B's gather re-run.
    let cube = top(&e, a);
    let mut v = serde_json::Map::new();
    v.insert("sizeX".into(), 3.0.into());
    e.dispatch(Command::MeshOpSet {
        op: cube,
        values: v,
    })
    .unwrap();
    let r = result(&e, b);
    assert_eq!(e.meshops.op_runs(), runs + 2);
    let x = r.mesh.vertices.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((x - 1.5).abs() < 1e-9, "{x}");
    // Moving A moves what B reads.
    e.scene.item_mut(a).unwrap().transform.translation.x = 1.;
    let x = result(&e, b)
        .mesh
        .vertices
        .iter()
        .map(|p| p.x)
        .fold(f64::MIN, f64::max);
    assert!((x - 2.5).abs() < 1e-9, "{x}");
}

#[test]
fn a_loop_is_refused_when_made_and_nothing_changes() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    // A reading B would close the loop A → B → A.
    e.scene.select_item(Some(a));
    e.dispatch(Command::MeshOpCreate(GATHER.into())).unwrap();
    let op = top(&e, a);
    let before = e.scene.clone();
    let err = link(&mut e, op, &[b]).unwrap_err();
    assert!(err.contains("dependency loop"), "{err}");
    assert_eq!(e.scene, before);
    // An op linking its own item is a loop too.
    assert!(link(&mut e, op, &[a]).is_err());
    assert!(find_loop(&e.scene).is_none());
}

#[test]
fn turning_on_an_op_that_closes_a_loop_is_refused() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    e.scene.select_item(Some(a));
    e.dispatch(Command::MeshOpCreate(GATHER.into())).unwrap();
    let op = top(&e, a);
    e.dispatch(Command::MeshOpEnable { op, on: false }).unwrap();
    // Off, it can't loop, so the link is allowed ...
    link(&mut e, op, &[b]).unwrap();
    // ... but turning it on is not.
    assert!(e.dispatch(Command::MeshOpEnable { op, on: true }).is_err());
}

#[test]
fn a_file_with_a_loop_opens_with_the_closing_op_turned_off() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    e.scene.select_item(Some(a));
    e.dispatch(Command::MeshOpCreate(GATHER.into())).unwrap();
    let op = top(&e, a);
    // A hand-edited file: force the loop past the editor.
    let stack = e.scene.item_mut(a).unwrap().mesh_ops.as_mut().unwrap();
    let i = stack.index_of(op.op).unwrap();
    stack.ops[i].kind = OpKind::from_raw(GATHER, 1, serde_json::json!({ "sources": [b] }));
    assert!(find_loop(&e.scene).is_some());
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert!(find_loop(&back).is_none());
    let off = back
        .items
        .iter()
        .flat_map(|i| i.mesh_ops.iter().flat_map(|s| &s.ops))
        .filter(|o| !o.enabled)
        .count();
    assert_eq!(off, 1);
    // Even unbroken, evaluation never hangs: the loop fails the op.
    let r = Runtime::default().current(&e.scene, e.scene.item(a).unwrap());
    assert!(r.is_some());
}

#[test]
fn a_deleted_source_fails_only_its_op_and_undo_brings_it_back() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    e.scene.select_item(Some(a));
    e.dispatch(Command::DeleteItem).unwrap();
    let r = result(&e, b);
    assert_eq!(r.statuses, [OpStatus::Failed(MISSING_INPUT.into())]);
    assert_eq!(r.mesh.faces.len(), 0, "passed through");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(result(&e, b).mesh.faces.len(), 6);
}

#[test]
fn a_hidden_source_still_evaluates() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    e.scene.item_mut(a).unwrap().visible = false;
    assert_eq!(result(&e, b).mesh.faces.len(), 6);
}

#[test]
fn links_save_and_reopen_to_the_same_result() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    let fresh = Runtime::default();
    let theirs = fresh.current(&back, back.item(b).unwrap()).unwrap();
    assert_eq!(*theirs.mesh, *result(&e, b).mesh);
}

#[test]
fn the_reader_waits_for_a_source_evaluating_in_the_background() {
    let (mut e, a, b) = two_meshes();
    gather_on(&mut e, b, &[a]);
    let runtime = Runtime::default();
    runtime.set_background(true, None);
    let item = e.scene.item(b).unwrap();
    // Ask as the viewport does, frame after frame, until it lands.
    let mut got = None;
    for _ in 0..2_000 {
        if let Some(r) = runtime.result(&e.scene, item)
            && r.current
        {
            got = Some(r);
            break;
        }
        runtime.wait();
    }
    let r = got.expect("the reader's result arrives");
    assert_eq!(
        r.statuses,
        [OpStatus::Ok],
        "no failure flashed while waiting"
    );
    assert_eq!(r.mesh.faces.len(), 6);
}

#[test]
fn an_op_of_an_unknown_type_runs_once_its_kit_registers() {
    const LATE: &str = "test.late.item";
    let json = format!(
        r#"{{"type":"{LATE}","version":1,"id":0,"enabled":true,"values":{{"sources":[]}}}}"#
    );
    let op: operandi_core::meshops::MeshOp = serde_json::from_str(&json).unwrap();
    assert!(matches!(op.kind, OpKind::Placeholder { .. }));
    let stack = operandi_core::meshops::MeshOpStack {
        ops: vec![op.clone()],
        next_id: 1,
    };
    let base = operandi_core::Shared::new(Mesh::default());
    let before = operandi_core::meshops::evaluate_stack(&base, &stack);
    assert!(matches!(before.statuses[0], OpStatus::Failed(_)));
    register(Arc::new(Typed::<Gather>::new(
        LATE,
        "Late",
        "Test",
        false,
        vec![Field::new("sources", "Source", FieldKind::Items)],
    )))
    .unwrap();
    let after = operandi_core::meshops::evaluate_stack(&base, &stack);
    assert_eq!(after.statuses[0], OpStatus::Ok);
    // Its saved form is untouched (SAFE-12).
    assert_eq!(
        serde_json::to_value(&op).unwrap(),
        serde_json::from_str::<serde_json::Value>(&json).unwrap()
    );
}

/// SAFE-16 for links: three meshes reading each other at random, with
/// loops attempted, items deleted and moved, eyes toggled, undo/redo, and a
/// background evaluator racing half the runs. Pass = no panic, refusals
/// leave the scene as it was, no loop ever gets in, and the saved scene
/// reopens to the same results.
#[test]
fn random_links_never_loop_panic_or_change_on_reopen() {
    register_gather();
    for seed in 1..=4u64 {
        let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut below = |n: usize| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % n.max(1) as u64) as usize
        };
        let mut e = Editor::default();
        if seed % 2 == 0 {
            e.meshops.set_background(true, None);
        }
        let mut ids = Vec::new();
        for _ in 0..3 {
            e.scene.new_mesh().unwrap();
            let id = e.scene.items.last().unwrap().id;
            e.dispatch(Command::MeshOpCreate("prim.cube.item".into()))
                .unwrap();
            ids.push(id);
        }
        for step in 0..80 {
            let live: Vec<u64> = e
                .scene
                .items
                .iter()
                .filter(|i| i.mesh_ops.is_some())
                .map(|i| i.id)
                .collect();
            if live.is_empty() {
                break;
            }
            let item = live[below(live.len())];
            let ops: Vec<u64> = e
                .scene
                .item(item)
                .unwrap()
                .mesh_ops
                .as_ref()
                .unwrap()
                .ops
                .iter()
                .map(|o| o.id)
                .collect();
            let op = OpRef {
                item,
                op: ops[below(ops.len())],
            };
            let cmd = match below(8) {
                0 | 1 => {
                    e.scene.select_item(Some(item));
                    Command::MeshOpCreate(GATHER.into())
                }
                2 | 3 => {
                    // Any id: live, deleted, itself, or one never made.
                    let mut v = serde_json::Map::new();
                    let pick = [ids[below(3)], ids[below(3)], 999][below(3)];
                    v.insert("sources".into(), vec![pick].into());
                    Command::MeshOpSet { op, values: v }
                }
                4 => Command::MeshOpEnable {
                    op,
                    on: below(2) == 0,
                },
                5 => {
                    e.scene.select_item(Some(item));
                    if below(3) == 0 {
                        Command::DeleteItem
                    } else {
                        e.scene.item_mut(item).unwrap().transform.translation.x += 1.;
                        continue;
                    }
                }
                6 => Command::Undo,
                _ => Command::Redo,
            };
            let before = e.scene.clone();
            if e.dispatch(cmd).is_err() {
                assert_eq!(e.scene, before, "seed {seed} step {step}");
            }
            assert!(find_loop(&e.scene).is_none(), "seed {seed} step {step}");
            for i in &e.scene.items {
                let _ = e.meshops.result(&e.scene, i);
            }
        }
        e.meshops.wait();
        let bytes = persistence::encode(&e.scene).unwrap();
        let back = persistence::decode(&bytes).unwrap();
        assert_eq!(back, e.scene, "seed {seed}");
        let fresh = Runtime::default();
        for (a, b) in e.scene.items.iter().zip(&back.items) {
            if a.mesh_ops.is_none() {
                continue;
            }
            let ours = e.meshops.current(&e.scene, a).unwrap();
            let theirs = fresh.current(&back, b).unwrap();
            assert_eq!(*ours.mesh, *theirs.mesh, "seed {seed}");
            assert_eq!(ours.statuses, theirs.statuses, "seed {seed}");
        }
    }
}
