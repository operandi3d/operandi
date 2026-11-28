//! Replicators (G8 Replica family, RP-1..RP-5). Acceptance criteria:
//!
//! 1. Replica Array 3 x 1 x 2 (X offset 2 m): a group, a hidden point
//!    cloud of 5 points and a Replicator; the original unchanged and still
//!    selected.
//! 2. Replica Radial Array 4: 3 points at the pivot, copies turned 90°,
//!    180°, 270° about Y.
//! 3. Random rotation repeats from the Seed, also after save and reopen.
//! 4. One undo removes the whole Replicator.
//! 5. Freeze Replicator: N − 1 instances in the group at the same places,
//!    the Replicator and cloud gone.
//! 6. Nothing to copy is refused with "Select an item to instance".
use glam::{DMat4, DVec3};
use operandi_core::evaluate::Purpose;
use operandi_core::replicator::{self, Replicator};
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().name = "proto".into();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(0.5), 0)
        .unwrap()
        .into();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.history.undo.clear();
    e
}

fn run(e: &mut Editor, tool: Tool, set: impl FnOnce(&mut DuplicateParameters)) -> Result<()> {
    e.dispatch(Command::Activate(tool))?;
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
    set(&mut p.duplicate);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    let r = e.dispatch(Command::Preview(p));
    e.end_tweak();
    e.dispatch(Command::Drop)?;
    r
}

fn rep_id(e: &Editor) -> u64 {
    e.scene
        .items
        .iter()
        .find(|i| i.replicator.is_some())
        .unwrap()
        .id
}

fn cloud(e: &Editor) -> &Item {
    let rep = e.scene.item(rep_id(e)).unwrap();
    e.scene
        .item(rep.replicator.as_ref().unwrap().point_source)
        .unwrap()
}

/// The copies the evaluated scene shows, as world matrices.
fn shown_copies(e: &Editor, purpose: Purpose) -> Vec<DMat4> {
    let s = e.evaluated(purpose);
    s.items
        .iter()
        .filter(|i| replicator::copy_replicator(i.id).is_some())
        .map(|i| s.world_matrix(i.id).unwrap())
        .collect()
}

#[test]
fn replica_array_makes_a_group_a_hidden_cloud_and_a_replicator() {
    let mut e = editor();
    let proto = e.scene.items[0].clone();
    run(&mut e, Tool::Array, |d| {
        d.array.count = [3, 1, 2];
        d.array.offset = DVec3::new(2., 1., 1.);
    })
    .unwrap();
    assert_eq!(e.scene.items.len(), 4, "the original and three new items");
    let names: Vec<&str> = e.scene.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "proto",
            "proto Instancer",
            "proto (Point Cloud)",
            "proto (Instancer)"
        ]
    );
    let group = &e.scene.items[1];
    assert!(group.is_group_locator() && group.parent.is_none());
    let c = cloud(&e);
    assert!(!c.visible, "the cloud is hidden");
    assert_eq!(c.parent, Some(group.id));
    assert!(c.mesh.faces.is_empty());
    let mut points = c.mesh.vertices.clone();
    points.sort_by(|a, b| (a.z, a.x).partial_cmp(&(b.z, b.x)).unwrap());
    let want = [
        DVec3::new(2., 0., 0.),
        DVec3::new(4., 0., 0.),
        DVec3::new(0., 0., 1.),
        DVec3::new(2., 0., 1.),
        DVec3::new(4., 0., 1.),
    ];
    assert_eq!(points.len(), 5);
    for (p, w) in points.iter().zip(want) {
        assert!(p.abs_diff_eq(w, 1e-9), "{p} vs {w}");
    }
    let rep = e.scene.item(rep_id(&e)).unwrap();
    assert_eq!(rep.parent, Some(group.id));
    assert_eq!(rep.replicator.as_ref().unwrap().prototype, proto.id);
    assert_eq!(e.scene.items[0], proto, "the original is unchanged");
    assert_eq!(e.scene.selection.item, Some(proto.id), "and stays selected");
    // The evaluated scene shows 5 copies; the document has no copy items.
    assert_eq!(shown_copies(&e, Purpose::Final).len(), 5);
}

#[test]
fn replica_radial_array_turns_each_copy() {
    let mut e = editor();
    run(&mut e, Tool::RadialArray, |d| d.radial.circle.count = 4).unwrap();
    let c = cloud(&e);
    assert_eq!(c.mesh.vertices.len(), 3);
    assert!(
        c.mesh.vertices.iter().all(|p| p.length() < 1e-9),
        "all at the pivot"
    );
    let mut angles: Vec<f64> = shown_copies(&e, Purpose::Final)
        .iter()
        .map(|w| {
            let x = w.transform_vector3(DVec3::X);
            // Turn about Y: X goes to (cos, 0, -sin).
            assert!(x.y.abs() < 1e-9);
            (-x.z).atan2(x.x).to_degrees().rem_euclid(360.)
        })
        .collect();
    angles.sort_by(f64::total_cmp);
    for (a, want) in angles.iter().zip([90., 180., 270.]) {
        assert!((a - want).abs() < 1e-6, "{angles:?}");
    }
}

#[test]
fn random_rotation_repeats_after_save_and_reopen() {
    let mut e = editor();
    run(&mut e, Tool::Array, |d| d.array.count = [4, 1, 1]).unwrap();
    let id = rep_id(&e);
    let mut settings = (**e.scene.item(id).unwrap().replicator.as_ref().unwrap()).clone();
    settings.random_rotation = DVec3::new(0., 180., 0.);
    settings.seed = 7;
    e.dispatch(Command::SetReplicator {
        item: id,
        settings: Box::new(settings),
    })
    .unwrap();
    let before = shown_copies(&e, Purpose::Final);
    assert!(
        before
            .iter()
            .any(|w| !w.transform_vector3(DVec3::X).abs_diff_eq(DVec3::X, 1e-6)),
        "the copies turn"
    );
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 58"));
    let back = Editor {
        scene: persistence::decode(&bytes).unwrap(),
        ..Default::default()
    };
    assert_eq!(back.scene, e.scene);
    let after = shown_copies(&back, Purpose::Final);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert!(a.abs_diff_eq(*b, 1e-12));
    }
}

#[test]
fn one_undo_removes_the_whole_replicator() {
    let mut e = editor();
    let start = e.scene.items.clone();
    run(&mut e, Tool::Array, |d| d.array.count = [3, 1, 1]).unwrap();
    let after = e.scene.clone();
    assert_eq!(e.scene.items.len(), 4);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, start);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

#[test]
fn freeze_replicator_makes_instances_in_the_group() {
    let mut e = editor();
    run(&mut e, Tool::Array, |d| {
        d.array.count = [3, 1, 2];
        d.array.offset = DVec3::new(2., 1., 1.);
    })
    .unwrap();
    let shown = shown_copies(&e, Purpose::Final);
    let proto = e.scene.items[0].id;
    let group = e.scene.items[1].id;
    e.scene.select_item(Some(rep_id(&e)));
    e.dispatch(Command::FreezeReplicator).unwrap();
    assert!(e.scene.items.iter().all(|i| i.replicator.is_none()));
    let instances: Vec<&Item> = e
        .scene
        .items
        .iter()
        .filter(|i| i.instance_of == Some(proto))
        .collect();
    assert_eq!(instances.len(), 5, "N − 1 instances for N = 6 copies");
    assert!(instances.iter().all(|i| i.parent == Some(group)));
    assert_eq!(e.scene.items.len(), 2 + 5, "original, group and instances");
    for (inst, want) in instances.iter().zip(&shown) {
        let w = e.scene.world_matrix(inst.id).unwrap();
        assert!(w.abs_diff_eq(*want, 1e-9));
    }
    assert_eq!(e.scene.selection.item, Some(group));
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items.iter().any(|i| i.replicator.is_some()));
}

#[test]
fn a_replica_tool_with_nothing_to_copy_is_refused() {
    let mut e = Editor::default();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    let r = e.dispatch(Command::Activate(Tool::Array));
    if r.is_ok() {
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
        p.duplicate.placed = true;
        let err = e.dispatch(Command::Preview(p)).unwrap_err();
        assert!(err.contains("Select an item to instance"), "{err}");
    }
    assert!(e.scene.items.iter().all(|i| i.replicator.is_none()));
}

#[test]
fn density_shares_and_settings_validate() {
    let mut e = editor();
    run(&mut e, Tool::Array, |d| d.array.count = [10, 1, 10]).unwrap();
    let id = rep_id(&e);
    let mut s: Replicator = (**e.scene.item(id).unwrap().replicator.as_ref().unwrap()).clone();
    s.density = 50.;
    s.display_density = 10.;
    e.dispatch(Command::SetReplicator {
        item: id,
        settings: Box::new(s.clone()),
    })
    .unwrap();
    let render = shown_copies(&e, Purpose::Final).len();
    let display = shown_copies(&e, Purpose::Display { base_edit: None }).len();
    assert!((30..70).contains(&render), "{render} of 99 render");
    assert!(display < 25, "{display} of 99 drawn");
    s.density = 150.;
    assert!(
        e.dispatch(Command::SetReplicator {
            item: id,
            settings: Box::new(s),
        })
        .is_err()
    );
}

/// RP-7 Mesh Paint Replica: one stroke makes one Replicator with a point
/// per dab, its prototype the background mesh; a second stroke a second.
#[test]
fn mesh_paint_replica_makes_one_replicator_per_stroke() {
    use operandi_core::mesh_paint::*;
    let mut e = Editor::default();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Box,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Plane,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    let cube = e.scene.items[0].id;
    let steps = e.history.undo.len();
    let stroke = |points: Vec<PaintPoint>| {
        Command::MeshPaint(Box::new(MeshPaintStroke {
            settings: MeshPaintSettings {
                replicas: true,
                mode: PaintMode::Drop,
                ..Default::default()
            },
            points,
            seed: 7,
            view_up: DVec3::Y,
            brush_world: 0.5,
        }))
    };
    let at = |x: f64| PaintPoint::at(DVec3::new(x, 0., 0.), DVec3::Y);
    e.dispatch(stroke(vec![at(0.1), at(0.2), at(0.3)])).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Mesh Paint Instancer");
    let reps: Vec<&Item> = e
        .scene
        .items
        .iter()
        .filter(|i| i.replicator.is_some())
        .collect();
    assert_eq!(reps.len(), 1);
    assert_eq!(reps[0].replicator.as_ref().unwrap().prototype, cube);
    let copies = shown_copies(&e, Purpose::Final);
    assert!(
        !copies.is_empty() && copies.len() <= 3,
        "{} copies",
        copies.len()
    );
    assert_eq!(cloud(&e).mesh.vertices.len(), copies.len());
    // The first copy sits on its dab, as Mesh Paint Instance places it.
    assert!(
        copies
            .iter()
            .any(|w| (w.transform_point3(DVec3::ZERO).y).abs() < 1e-9)
    );
    e.dispatch(stroke(vec![at(-0.2)])).unwrap();
    let count = e
        .scene
        .items
        .iter()
        .filter(|i| i.replicator.is_some())
        .count();
    assert_eq!(count, 2, "a second stroke, a second Instancer");
    let names: Vec<&str> = e.scene.items.iter().map(|i| i.name.as_str()).collect();
    assert!(
        names.iter().any(|n| n.ends_with("Instancer (2)")),
        "{names:?}"
    );
}

/// Hidden-limits A4: a Replicator past its copy limit says how many points
/// get no copy instead of dropping them silently.
#[test]
fn copies_left_out_counts_the_points_past_the_limit() {
    let mut e = editor();
    run(&mut e, Tool::Array, |d| d.array.count = [3, 1, 2]).unwrap();
    let rep = rep_id(&e);
    assert_eq!(replicator::copies_left_out(&e.scene, rep), 0);
    assert_eq!(
        replicator::copies_left_out(&e.scene, 999),
        0,
        "not an Instancer"
    );
}

/// Opt-in (about 400 MB): a cloud three points over the limit.
/// `cargo test --locked --workspace -- replicator::copies_left_out_past --ignored`
#[test]
#[ignore]
fn copies_left_out_past_the_real_limit() {
    let mut e = editor();
    run(&mut e, Tool::Array, |d| d.array.count = [3, 1, 2]).unwrap();
    let rep = rep_id(&e);
    let cloud_id = cloud(&e).id;
    let item = e.scene.items.iter_mut().find(|i| i.id == cloud_id).unwrap();
    item.mesh.vertices = vec![DVec3::ZERO; replicator::MAX_COPIES + 3];
    assert_eq!(replicator::copies_left_out(&e.scene, rep), 3);
}
