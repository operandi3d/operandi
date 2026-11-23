//! SAFE-16 stress harness for Mesh Stack: random stacks of real ops with
//! random edits (reorder, eye, values including zero, huge, negative and
//! NaN through the script path, delete, freeze), undo/redo storms and
//! save/load round trips, with a background evaluator racing the edits.
//! Pass = no panic escapes, every command either works or refuses with
//! the scene unchanged, and a saved scene reopens to the identical result.
use operandi_core::mesh_stack::{Limits, Runtime, evaluate_stack_within, op_defs};

/// Small limits keep the run quick; over-limit results fail their op,
/// which the harness wants to see happen too.
const LIMITS: Limits = Limits {
    max_polygons: 20_000,
    max_points: 20_000,
};
use operandi_core::mesh_stack_editor::{IndexEdit, OpRef, OpRow};
use operandi_core::{Command, Editor, persistence};
use serde_json::{Value, json};

/// Small fixed generator: the harness is the same run on every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn wild_value(rng: &mut Rng, old: &Value) -> Value {
    match old {
        Value::Bool(b) => json!(!b),
        Value::Number(_) => match rng.below(9) {
            0 => json!(0),
            1 => json!(-1),
            2 => json!(1e12),
            3 => json!(0.25),
            4 => json!(2),
            5 => json!(7),
            // NaN can't be JSON; a script sends it as text.
            6 => json!("NaN"),
            7 => json!(-0.0001),
            _ => json!(3),
        },
        _ => json!("x"),
    }
}

fn ops_of(e: &Editor, item: u64) -> Vec<u64> {
    e.scene
        .items
        .iter()
        .find(|i| i.id == item)
        .and_then(|i| i.mesh_stack.as_ref())
        .map_or_else(Vec::new, |s| s.ops.iter().map(|o| o.id).collect())
}

/// Three seeds (one racing the background evaluator) in the normal run;
/// the other nine are the opt-in probe below (test speed, 2026-11-16).
#[test]
fn random_stacks_and_edits_never_panic_and_reopen_identical() {
    random_stacks_and_edits(1..=3);
}

/// The full stress set: `cargo test --locked --workspace --
/// mesh_stack_stress:: --ignored`.
#[test]
#[ignore = "stress probe: run with --ignored"]
fn random_stacks_and_edits_more_seeds() {
    random_stacks_and_edits(4..=12);
}

fn random_stacks_and_edits(seeds: std::ops::RangeInclusive<u64>) {
    for seed in seeds {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ seed);
        let mut e = Editor::default();
        e.mesh_stack.set_limits(LIMITS);
        // Half the runs race a background evaluator against the edits.
        if seed % 2 == 0 {
            e.mesh_stack.set_background(true, None);
        }
        e.scene.new_mesh().unwrap();
        // A Base Mesh with polygons, so direct tools have something to
        // (wrongly) edit.
        e.scene.items[0].mesh = operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
            .unwrap()
            .into();
        let item = e.scene.items[0].id;
        for step in 0..60 {
            let ops = ops_of(&e, item);
            let pick = |rng: &mut Rng| OpRef {
                item,
                op: ops[rng.below(ops.len())],
            };
            let cmd = match rng.below(if ops.is_empty() { 1 } else { 11 }) {
                0 | 1 => {
                    // Operandi's own types: kit types other tests register
                    // must not change this harness's random choices.
                    let types: Vec<_> = op_defs()
                        .into_iter()
                        .filter(|d| !d.id().starts_with("test."))
                        .collect();
                    Command::MeshOpCreate(types[rng.below(types.len())].id().into())
                }
                2 => Command::MeshOpMove {
                    op: pick(&mut rng),
                    position: rng.below(ops.len() + 2),
                },
                3 => Command::MeshOpEnable {
                    op: pick(&mut rng),
                    on: rng.below(2) == 0,
                },
                4 | 5 => {
                    let r = pick(&mut rng);
                    let op = &e.scene.items[0].mesh_stack.as_ref().unwrap().ops
                        [ops.iter().position(|&o| o == r.op).unwrap()];
                    let values = op.kind.values();
                    let Some(map) = values.as_object() else {
                        continue;
                    };
                    let keys: Vec<&String> = map.keys().collect();
                    if keys.is_empty() {
                        continue;
                    }
                    let key = keys[rng.below(keys.len())];
                    let mut changes = serde_json::Map::new();
                    changes.insert(key.clone(), wild_value(&mut rng, &map[key]));
                    Command::MeshOpSet {
                        op: r,
                        values: changes,
                    }
                }
                6 => Command::MeshOpDelete(vec![pick(&mut rng)]),
                7 => Command::Undo,
                8 => Command::Redo,
                9 => Command::MeshOpFreeze {
                    op: pick(&mut rng),
                    duplicate: rng.below(2) == 0,
                },
                _ => Command::MeshOpSelect(Some(OpRow {
                    item,
                    op: (rng.below(2) == 0).then(|| pick(&mut rng).op),
                })),
            };
            let before = e.scene.clone();
            if e.dispatch(cmd.clone()).is_err() {
                assert_eq!(
                    e.scene, before,
                    "seed {seed} step {step}: a refusal changed the scene"
                );
            }
            // A direct tool on the procedural mesh is refused (SAFE-05).
            // (A Base Mesh frozen down to no polygons has nothing to flip.)
            if step % 15 == 7
                && e.scene.items[0].mesh_stack.is_some()
                && e.base_edit.is_none()
                && !e.scene.items[0].mesh.faces.is_empty()
            {
                e.scene.selection.item = Some(item);
                let before = e.scene.clone();
                assert!(e.dispatch(Command::FlipPolygons).is_err());
                assert_eq!(e.scene, before);
            }
            // Ask for results as the app does every frame.
            for i in &e.scene.items {
                let _ = e.mesh_stack.result(&e.scene, i);
            }
            // Select by Index edits with nothing selected are refused.
            if step % 20 == 3 && !ops_of(&e, item).is_empty() {
                let op = OpRef {
                    item,
                    op: ops_of(&e, item)[0],
                };
                let _ = e.dispatch(Command::MeshOpIndex {
                    op,
                    edit: IndexEdit::Add,
                });
            }
        }
        e.mesh_stack.wait();
        // Save and reopen: same recipe, same result (SAFE-11, MO-91).
        let bytes = persistence::encode(&e.scene).unwrap();
        let back = persistence::decode(&bytes).unwrap();
        assert_eq!(back, e.scene, "seed {seed}");
        let fresh = Runtime::default();
        fresh.set_limits(LIMITS);
        for (a, b) in e.scene.items.iter().zip(&back.items) {
            if a.mesh_stack.is_none() {
                continue;
            }
            let ours = e.mesh_stack.current(&e.scene, a).unwrap();
            let theirs = fresh.current(&back, b).unwrap();
            assert_eq!(*ours.mesh, *theirs.mesh, "seed {seed}");
            assert_eq!(ours.statuses, theirs.statuses, "seed {seed}");
            let direct =
                evaluate_stack_within(b.mesh.shared(), b.mesh_stack.as_ref().unwrap(), LIMITS);
            assert_eq!(
                *direct.mesh, *ours.mesh,
                "seed {seed}: cache = from scratch"
            );
        }
    }
}

#[test]
fn a_hand_edited_file_with_bad_values_opens_and_fails_per_op() {
    // A file can bypass the panel (SAFE-10): values out of range load,
    // then fail only their op when it runs (SAFE-04, SAFE-17).
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(Command::MeshOpCreate("prim.sphere.item".into()))
        .unwrap();
    e.dispatch(Command::MeshOpCreate("array.clone.item".into()))
        .unwrap();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    let text = text.replacen("\"sides\": 24", "\"sides\": 0", 1);
    let text = text.replacen("\"numX\": 2", "\"numX\": 4000000", 1);
    let back = persistence::decode(text.as_bytes()).unwrap();
    let out = Runtime::default().current(&back, &back.items[0]).unwrap();
    assert!(
        matches!(&out.statuses[0], operandi_core::mesh_stack::OpStatus::Failed(r) if r.contains("Sides")),
        "{:?}",
        out.statuses
    );
    // Array on the empty result below has nothing to copy.
    assert!(
        out.mesh.faces.is_empty(),
        "the Sphere passed its empty input through"
    );
}
