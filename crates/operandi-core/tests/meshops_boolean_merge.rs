//! MeshOps suite batch 2: the Boolean and Merge Meshes ops (BOOL-32,
//! BOP-xx, MM-xx), reading other items through links (SAFE-07..09).
//!
//! Boolean reference geometry as in BOOLEANS.md: target cube A from -1 to
//! 1 (the Base Mesh of the item with the stack), driver cube B from 0 to 2.
use glam::{DMat4, DVec3};
use operandi_core::meshops::{OpStatus, Runtime};
use operandi_core::meshops_editor::OpRef;
use operandi_core::meshops_items::{BOOLEAN_OP, MERGE_MESHES_OP, MergeMeshesOp};
use operandi_core::{BooleanOp, BooleanOptions, Command, DriveMesh, Editor, Mesh, persistence};
use serde_json::{Value, json};

fn cube(lo: f64, hi: f64) -> Mesh {
    let c = (lo + hi) / 2.;
    operandi_core::cube_geometry(DVec3::splat(c), DVec3::splat(hi - lo)).unwrap()
}
fn unit_cube_at(x: f64) -> Mesh {
    operandi_core::cube_geometry(DVec3::new(x, 0., 0.), DVec3::ONE).unwrap()
}
/// Target A (-1..1) and driver B (0..2), A selected.
fn scene() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let a = mesh(&mut e, cube(-1., 1.));
    let b = mesh(&mut e, cube(0., 2.));
    e.scene.select_item(Some(a));
    (e, a, b)
}
fn mesh(e: &mut Editor, m: Mesh) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().mesh = m.into();
    id
}
fn at(e: &mut Editor, id: u64, p: DVec3) {
    e.scene.item_mut(id).unwrap().transform.translation = p;
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
    set(e, op, values).unwrap();
    op
}
fn set(e: &mut Editor, op: OpRef, values: Value) -> operandi_core::Result<()> {
    let values = values.as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values })
}
fn result(e: &Editor, item: u64) -> operandi_core::meshops::Evaluated {
    e.meshops
        .current(&e.scene, e.scene.item(item).unwrap())
        .unwrap()
}
fn op_value(e: &Editor, op: OpRef, key: &str) -> bool {
    let stack = e.scene.item(op.item).unwrap().mesh_ops.as_ref().unwrap();
    let row = stack.ops.iter().find(|o| o.id == op.op).unwrap();
    row.kind.values()[key].as_bool().unwrap()
}
fn extent(m: &Mesh) -> (DVec3, DVec3) {
    let lo = m
        .vertices
        .iter()
        .fold(DVec3::splat(f64::MAX), |a, &p| a.min(p));
    let hi = m
        .vertices
        .iter()
        .fold(DVec3::splat(f64::MIN), |a, &p| a.max(p));
    (lo, hi)
}
fn close(a: DVec3, b: DVec3) -> bool {
    (a - b).abs().max_element() < 1e-9
}
/// The direct tool's result for the same inputs.
fn direct(mode: BooleanOp) -> Mesh {
    let (mut e, a, b) = scene();
    e.dispatch(Command::Boolean(BooleanOptions {
        op: mode,
        drive: DriveMesh::Item,
        item: Some(b),
        children: false,
    }))
    .unwrap();
    (*e.scene.item(a).unwrap().mesh).clone()
}

// ---- Boolean (BOP) ----

#[test]
fn bop_t01_to_t05_every_operation_matches_the_direct_tool() {
    for (mode, polygons) in [(0, 12), (1, 6), (2, 9), (3, 18), (4, 12)] {
        let (mut e, a, b) = scene();
        add(&mut e, a, BOOLEAN_OP, json!({ "mode": mode, "driver": b }));
        let r = result(&e, a);
        assert_eq!(r.statuses, [OpStatus::Ok], "mode {mode}");
        assert_eq!(r.mesh.faces.len(), polygons, "mode {mode}");
        let same = [
            BooleanOp::Union,
            BooleanOp::Intersect,
            BooleanOp::Subtract,
            BooleanOp::Add,
        ][mode.min(3) as usize % 4];
        let same = if mode == 4 { BooleanOp::Union } else { same };
        assert_eq!(
            *r.mesh,
            direct(same),
            "mode {mode}: one engine (tool family)"
        );
        // BOOL-08: the driver is left alone; the Base Mesh too (MO-05).
        assert_eq!(*e.scene.item(b).unwrap().mesh, cube(0., 2.));
        assert_eq!(*e.scene.item(a).unwrap().mesh, cube(-1., 1.));
    }
}

#[test]
fn bop_t06_to_t09_driver_placement_and_use_world_transform() {
    // T06: B's points at the origin (-0.5..0.5), item at (1,1,1).
    let mut e = Editor::default();
    let a = mesh(&mut e, cube(-1., 1.));
    let b = mesh(&mut e, cube(-0.5, 0.5));
    at(&mut e, b, DVec3::ONE);
    let op = add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": b }));
    assert_eq!(result(&e, a).mesh.faces.len(), 9);
    // T07: A moved up: the cut becomes a notch through an edge.
    at(&mut e, a, DVec3::new(0., 0., 1.));
    assert_eq!(result(&e, a).mesh.faces.len(), 10);
    // T08: Use World Transform off: as if A sat at the origin.
    set(&mut e, op, json!({ "useWorldXfrm": false })).unwrap();
    assert_eq!(result(&e, a).mesh.faces.len(), 9);
    // T09: a hidden driver still cuts (BOP-23, SAFE-09).
    set(&mut e, op, json!({ "useWorldXfrm": true })).unwrap();
    at(&mut e, a, DVec3::ZERO);
    e.scene.item_mut(b).unwrap().visible = false;
    assert_eq!(result(&e, a).mesh.faces.len(), 9);
}

#[test]
fn moving_the_driver_away_leaves_the_target_whole() {
    let (mut e, a, b) = scene();
    add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": b }));
    at(&mut e, b, DVec3::new(5., 0., 0.));
    assert_eq!(result(&e, a).mesh.faces.len(), 6, "BOOL-17");
}

#[test]
fn a_driver_with_its_own_stack_cuts_with_its_result() {
    let (mut e, a, b) = scene();
    add(
        &mut e,
        b,
        "poly.bevel.item",
        json!({ "inset": 0.1, "shift": 0.1 }),
    );
    add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": b }));
    let r = result(&e, a);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    assert!(r.mesh.faces.len() > 9, "the bevel's faces show in the cut");
}

#[test]
fn without_a_driver_or_with_an_open_one_the_op_fails_and_passes_through() {
    let (mut e, a, b) = scene();
    let op = add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2 }));
    let r = result(&e, a);
    assert!(matches!(&r.statuses[0], OpStatus::Failed(m) if m.contains("Driver Surface")));
    assert_eq!(*r.mesh, cube(-1., 1.));
    // An open driver is refused by the pre-flight (BOOL-19), not cut.
    let mut open = cube(0., 2.);
    open.faces.pop();
    e.scene.item_mut(b).unwrap().mesh = open.into();
    set(&mut e, op, json!({ "driver": b })).unwrap();
    let r = result(&e, a);
    assert!(
        matches!(&r.statuses[0], OpStatus::Failed(_)),
        "{:?}",
        r.statuses
    );
    assert_eq!(*r.mesh, cube(-1., 1.));
}

#[test]
fn a_group_driver_cuts_with_all_its_meshes() {
    let (mut e, a, b) = scene();
    let c = mesh(&mut e, cube(-2., 0.));
    let g = e.scene.add_group_locator().unwrap();
    e.scene.set_parent(b, Some(g), true).unwrap();
    e.scene.set_parent(c, Some(g), true).unwrap();
    add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": g }));
    let r = result(&e, a);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    assert!(r.mesh.faces.len() > 9, "two corners cut");
}

#[test]
fn bop_12_no_cut_with_its_own_mesh_or_through_a_loop() {
    let (mut e, a, b) = scene();
    let op = add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2 }));
    assert!(set(&mut e, op, json!({ "driver": a })).is_err());
    set(&mut e, op, json!({ "driver": b })).unwrap();
    let merge = add(&mut e, b, MERGE_MESHES_OP, json!({}));
    let err = set(&mut e, merge, json!({ "sources": [a] })).unwrap_err();
    assert!(err.contains("dependency loop"), "{err}");
}

// ---- Merge Meshes (MM) ----

#[test]
fn mm_t01_t02_t03_t11_placement() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let source = mesh(&mut e, unit_cube_at(0.));
    at(&mut e, source, DVec3::new(3., 0., 0.));
    at(&mut e, target, DVec3::new(0., 2., 0.));
    let op = add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    let check = |e: &Editor, lo: f64, hi: f64| {
        let r = result(e, target);
        assert_eq!(r.statuses, [OpStatus::Ok]);
        assert_eq!(r.mesh.vertices.len(), 8);
        let (l, h) = extent(&r.mesh);
        assert!(
            close(l, DVec3::new(lo, -0.5, -0.5)) && close(h, DVec3::new(hi, 0.5, 0.5)),
            "{l} {h}"
        );
    };
    // T01: placed by the source's world transform; the target's own
    // transform is not undone.
    check(&e, 2.5, 3.5);
    // T11: rotating the target changes nothing in its own space.
    e.scene.item_mut(target).unwrap().transform.rotation = DVec3::new(0., 0., 90f64.to_radians());
    check(&e, 2.5, 3.5);
    // T02: Source World Transform off: the source's own points.
    set(&mut e, op, json!({ "world": false })).unwrap();
    check(&e, -0.5, 0.5);
}

/// MOP-MM-M03 / M04: Use World Transforms off leaves the placement to
/// Source World Transform: placed with it on (M03), the source's own points
/// with it off (M04). One undo step each, and the value saves and reopens.
#[test]
fn mm_use_world_transforms_off_keeps_the_placement() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let source = mesh(&mut e, unit_cube_at(0.));
    at(&mut e, source, DVec3::new(3., 0., 0.));
    at(&mut e, target, DVec3::new(0., 2., 0.));
    let op = add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    let x_range = |e: &Editor| {
        let r = result(e, target);
        assert_eq!(r.statuses, [OpStatus::Ok]);
        let (l, h) = extent(&r.mesh);
        (l.x, h.x)
    };
    assert_eq!(x_range(&e), (2.5, 3.5), "default: placed");
    let undo = e.undo.len();
    set(&mut e, op, json!({ "useWorldXfrm": false })).unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    assert_eq!(x_range(&e), (2.5, 3.5), "M03");
    set(&mut e, op, json!({ "world": false })).unwrap();
    assert_eq!(x_range(&e), (-0.5, 0.5), "M04");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(x_range(&e), (2.5, 3.5), "undone");
    e.dispatch(Command::Undo).unwrap();
    assert!(op_value(&e, op, "useWorldXfrm"), "undone to on");
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(x_range(&e), (-0.5, 0.5), "redone");
    // Save and reopen keep it; a file without it reads as on.
    let dir = std::env::temp_dir().join(format!("operandi-mm-world-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mm.opscene");
    e.save_committed(&path).unwrap();
    let reopened = Editor {
        scene: persistence::load(&path).unwrap(),
        ..Default::default()
    };
    assert_eq!(x_range(&reopened), (-0.5, 0.5), "reopened");
    assert!(!op_value(&reopened, op, "useWorldXfrm"), "reopened off");
    let old: MergeMeshesOp =
        serde_json::from_value(json!({"sources": [], "hierarchy": false, "world": true,
            "index": "", "copyPtag": true, "copyMorph": true, "copyNormal": false,
            "copyUV": true, "copyWeight": true, "copyColor": true,
            "copySelectionSet": true, "copyOtherMaps": true, "face": true,
            "subdiv": true, "psubdiv": true, "curve": true, "bezier": true,
            "BSpline": true, "spatch": true, "text": true, "line": true,
            "curveFill": true, "filterIdent": "none"}))
        .unwrap();
    assert!(old.use_world);
}

#[test]
fn mm_t04_the_targets_own_geometry_is_kept() {
    let mut e = Editor::default();
    let target = mesh(&mut e, cube(-0.125, 0.125));
    let source = mesh(&mut e, unit_cube_at(0.));
    at(&mut e, source, DVec3::new(3., 0., 0.));
    add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    let r = result(&e, target);
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (16, 12));
}

#[test]
fn mm_t05_t06_t07_index_picks_sources_in_link_order() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let s: Vec<u64> = [3., 6., 9.]
        .iter()
        .map(|&x| mesh(&mut e, unit_cube_at(x)))
        .collect();
    let op = add(&mut e, target, MERGE_MESHES_OP, json!({ "sources": s }));
    assert_eq!(result(&e, target).mesh.faces.len(), 18, "MM-12: all three");
    for (index, lo, hi, n) in [
        ("1", 5.5, 6.5, 6),
        ("1...", 5.5, 9.5, 12),
        ("0,2", 2.5, 9.5, 12),
    ] {
        set(&mut e, op, json!({ "index": index })).unwrap();
        let r = result(&e, target);
        assert_eq!(r.mesh.faces.len(), n, "{index}");
        let (l, h) = extent(&r.mesh);
        assert!(
            (l.x - lo).abs() < 1e-9 && (h.x - hi).abs() < 1e-9,
            "{index}: {l} {h}"
        );
    }
    // Bad text fails the op and passes its input through.
    set(&mut e, op, json!({ "index": "x" })).unwrap();
    assert!(matches!(
        result(&e, target).statuses[0],
        OpStatus::Failed(_)
    ));
}

#[test]
fn mm_t08_merge_hierarchy_brings_child_meshes() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let source = mesh(&mut e, unit_cube_at(0.));
    let child = mesh(
        &mut e,
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::splat(0.5)).unwrap(),
    );
    e.scene.set_parent(child, Some(source), false).unwrap();
    at(&mut e, child, DVec3::new(0., 2., 0.));
    let op = add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    assert_eq!(result(&e, target).mesh.vertices.len(), 8);
    set(&mut e, op, json!({ "hierarchy": true })).unwrap();
    let r = result(&e, target);
    assert_eq!(r.mesh.vertices.len(), 16);
    let (_, h) = extent(&r.mesh);
    assert!(
        (h.y - 2.25).abs() < 1e-9,
        "the child at world y 1.75..2.25: {h}"
    );
}

#[test]
fn mm_t09_rotated_source() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let source = mesh(&mut e, unit_cube_at(0.5));
    e.scene.item_mut(source).unwrap().transform.rotation = DVec3::new(0., 90f64.to_radians(), 0.);
    add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    let (l, h) = extent(&result(&e, target).mesh);
    assert!(
        (l.z + 1.).abs() < 1e-9 && h.z.abs() < 1e-9,
        "z -1..0: {l} {h}"
    );
    let _ = DMat4::IDENTITY;
}

#[test]
fn mm_t10_faces_off_copies_no_faces() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let source = mesh(&mut e, unit_cube_at(0.));
    add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source], "face": false }),
    );
    let r = result(&e, target);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    assert!(r.mesh.faces.is_empty() && r.mesh.vertices.is_empty());
}

#[test]
fn mm_map_switches_drop_their_maps() {
    let mut e = Editor::default();
    let target = mesh(&mut e, Mesh::default());
    let mut m = unit_cube_at(0.);
    m.attributes.polygon_sets.insert("Top".into(), [0].into());
    let source = mesh(&mut e, m);
    let op = add(
        &mut e,
        target,
        MERGE_MESHES_OP,
        json!({ "sources": [source] }),
    );
    assert!(
        result(&e, target)
            .mesh
            .attributes
            .polygon_sets
            .contains_key("Top")
    );
    set(
        &mut e,
        op,
        json!({ "copySelectionSet": false, "copyUV": false }),
    )
    .unwrap();
    let r = result(&e, target);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    assert!(r.mesh.attributes.polygon_sets.is_empty());
    assert!(r.mesh.uv_maps.is_empty());
}

#[test]
fn mm_14_a_deleted_source_contributes_nothing_until_undo() {
    let (mut e, a, b) = scene();
    add(&mut e, a, MERGE_MESHES_OP, json!({ "sources": [b] }));
    assert_eq!(result(&e, a).mesh.faces.len(), 12);
    e.scene.select_item(Some(b));
    e.dispatch(Command::DeleteItem).unwrap();
    let r = result(&e, a);
    assert_eq!(r.statuses, [OpStatus::Ok], "not failed (MM-14)");
    assert_eq!(r.mesh.faces.len(), 6);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(result(&e, a).mesh.faces.len(), 12);
}

#[test]
fn merge_then_boolean_in_one_stack_saves_reopens_and_freezes() {
    let (mut e, a, b) = scene();
    let c = mesh(&mut e, cube(-4., -3.));
    add(&mut e, a, MERGE_MESHES_OP, json!({ "sources": [c] }));
    add(&mut e, a, BOOLEAN_OP, json!({ "mode": 2, "driver": b }));
    let ours = result(&e, a);
    assert!(
        ours.statuses.iter().all(|s| *s == OpStatus::Ok),
        "{:?}",
        ours.statuses
    );
    assert_eq!(ours.mesh.faces.len(), 15, "C's 6 + the cut A's 9");
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    let theirs = Runtime::default()
        .current(&back, back.item(a).unwrap())
        .unwrap();
    assert_eq!(*theirs.mesh, *ours.mesh);
    let top = OpRef {
        item: a,
        op: e.scene.item(a).unwrap().mesh_ops.as_ref().unwrap().ops[1].id,
    };
    e.dispatch(Command::MeshOpFreeze {
        op: top,
        duplicate: false,
    })
    .unwrap();
    assert!(e.scene.item(a).unwrap().mesh_ops.is_none());
    assert_eq!(*e.scene.item(a).unwrap().mesh, *ours.mesh);
}

#[test]
fn bop_10_the_other_selected_mesh_becomes_the_driver() {
    let (mut e, a, b) = scene();
    e.scene.select_item(Some(a));
    e.scene.selection.also.push(b);
    e.dispatch(Command::MeshOpCreate(BOOLEAN_OP.into()))
        .unwrap();
    let op = &e.scene.item(a).unwrap().mesh_ops.as_ref().unwrap().ops[0];
    assert_eq!(op.kind.values()["driver"], b);
    assert_eq!(result(&e, a).mesh.faces.len(), 12, "Union by default");
}

#[test]
fn bop_t10_solid_drill_tunnel_and_axis_drill_on_a_grid() {
    // A 2-unit cube cut by the 1-unit cube at (1,1,1): its corner faces.
    let mut e = Editor::default();
    let a = mesh(&mut e, cube(-1., 1.));
    let b = mesh(
        &mut e,
        operandi_core::cube_geometry(DVec3::ONE, DVec3::ONE).unwrap(),
    );
    add(&mut e, a, "solidDrill.meshop.item", json!({ "driver": b }));
    let r = result(&e, a);
    assert_eq!(r.statuses, [OpStatus::Ok]);
    assert_eq!((r.mesh.faces.len(), r.mesh.vertices.len()), (6, 13));
    // Axis Drill: the open top face of a cube, cut straight down (Y) by
    // a smaller cube's outline: a hole (Tunnel).
    let gone: Vec<u32> = (0..6).collect::<Vec<u32>>();
    let big = cube(-2., 2.);
    let top = (0..6u32)
        .find(|&f| {
            big.faces[f as usize]
                .vertices
                .iter()
                .all(|&v| big.vertices[v as usize].y == 2.)
        })
        .unwrap();
    let rest: Vec<u32> = gone.into_iter().filter(|&f| f != top).collect();
    let plate = operandi_core::polygon_remove::delete_geometry(&big, &rest, &[]).unwrap();
    let mut e = Editor::default();
    let t = mesh(&mut e, plate);
    let cutter = mesh(&mut e, cube(-1., 1.));
    add(
        &mut e,
        t,
        "axisDrill.meshop.item",
        json!({ "driver": cutter, "axis": 1 }),
    );
    let r = result(&e, t);
    assert_eq!(r.statuses, [OpStatus::Ok], "{:?}", r.statuses);
    assert!(r.mesh.faces.len() > 1, "the hole splits the plate");
}
