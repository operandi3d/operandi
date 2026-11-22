//! Item transform Reset + Freeze, stage 1 (spec freeze-transforms-2026-11-14,
//! BUILD-BRIEF-STAGE1 tests): Freeze bakes the chosen part into the points
//! and nothing moves, children and selected children included; mirrored
//! scale keeps faces pointing out; locked items are skipped; identity is no
//! step; one undo restores everything; the script commands run the same.
use glam::{DMat4, DVec3};
use operandi_core::camera_props::TransformReset;
use operandi_core::*;

fn deg(v: [f64; 3]) -> DVec3 {
    DVec3::from(v) * std::f64::consts::PI / 180.
}

fn transform(pos: [f64; 3], rot: [f64; 3], scale: [f64; 3]) -> scene::Transform {
    scene::Transform {
        translation: DVec3::from(pos),
        rotation: deg(rot),
        scale: DVec3::from(scale),
        ..Default::default()
    }
}

/// An empty scene's editor.
fn editor() -> Editor {
    let mut e = Editor::default();
    e.next_item_id = e.scene.next_id;
    e
}

/// A new 1 m cube mesh item centred at `centre` (local), under `parent`.
fn add_cube(e: &mut Editor, centre: DVec3, t: scene::Transform, parent: Option<u64>) -> u64 {
    e.scene.new_mesh().unwrap();
    e.next_item_id = e.scene.next_id;
    let item = e.scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(centre, DVec3::ONE, 0).unwrap().into();
    item.transform = t;
    item.parent = parent;
    item.id
}

fn select(e: &mut Editor, ids: &[u64]) {
    e.scene.selection.mode = Mode::Item;
    e.scene.selection.item = ids.first().copied();
    e.scene.selection.also = ids[1..].to_vec();
}

fn world_points(e: &Editor, id: u64) -> Vec<DVec3> {
    let w = e.scene.world_matrix(id).unwrap();
    e.scene
        .item(id)
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|p| w.transform_point3(*p))
        .collect()
}

fn same_points(a: &[DVec3], b: &[DVec3]) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!(p.abs_diff_eq(*q, 1e-9), "{p} vs {q}");
    }
}

fn same_matrix(a: DMat4, b: DMat4) {
    assert!(a.abs_diff_eq(b, 1e-9), "{a} vs {b}");
}

fn freeze(e: &mut Editor, ids: &[u64], part: TransformReset) -> Result<()> {
    e.dispatch(Command::FreezeTransform {
        items: ids.to_vec(),
        part,
    })
}

/// Every polygon's world normal points away from the mesh's world centre.
fn faces_point_out(e: &Editor, id: u64) -> bool {
    let pts = world_points(e, id);
    let centre = pts.iter().copied().sum::<DVec3>() / pts.len() as f64;
    e.scene.item(id).unwrap().mesh.faces.iter().all(|f| {
        let c: Vec<DVec3> = f.vertices.iter().map(|&v| pts[v as usize]).collect();
        let mut n = DVec3::ZERO;
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            n += DVec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        let mid = c.iter().copied().sum::<DVec3>() / c.len() as f64;
        n.dot(mid - centre) > 0.
    })
}

#[test]
fn freeze_each_part_keeps_parent_and_child_in_place() {
    for part in TransformReset::ALL {
        let mut e = editor();
        let parent = add_cube(
            &mut e,
            DVec3::new(0.5, 0., 0.),
            transform([1., 2., 3.], [20., 35., -10.], [2., 2., 2.]),
            None,
        );
        let child = add_cube(
            &mut e,
            DVec3::ZERO,
            transform([0., 1.5, 0.], [0., 0., 30.], [1., 1., 1.]),
            Some(parent),
        );
        let points = world_points(&e, parent);
        let child_world = e.scene.world_matrix(child).unwrap();
        select(&mut e, &[parent]);
        let before = e.scene.clone();
        freeze(&mut e, &[parent], part).unwrap();
        same_points(&world_points(&e, parent), &points);
        same_matrix(e.scene.world_matrix(child).unwrap(), child_world);
        let t = &e.scene.item(parent).unwrap().transform;
        assert_eq!(
            *t,
            part.apply(&before.item(parent).unwrap().transform),
            "{part:?}"
        );
        assert_eq!(e.undo.last().unwrap().label, part.step_label("Freeze"));
        // Undo restores geometry, channels and the child exactly.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.items, before.items, "{part:?}");
        e.dispatch(Command::Redo).unwrap();
        same_points(&world_points(&e, parent), &points);
    }
}

#[test]
fn parent_and_child_selected_both_freeze_and_nothing_moves() {
    let mut e = editor();
    let parent = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 0., 0.], [0., 45., 0.], [2., 2., 2.]),
        None,
    );
    let child = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0., 1., 0.], [10., 0., 0.], [1., 1., 1.]),
        Some(parent),
    );
    let (pp, cp) = (world_points(&e, parent), world_points(&e, child));
    // Child listed first: the parent still goes first.
    select(&mut e, &[child, parent]);
    let undo = e.undo.len();
    freeze(&mut e, &[child, parent], TransformReset::All).unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    same_points(&world_points(&e, parent), &pp);
    same_points(&world_points(&e, child), &cp);
    for id in [parent, child] {
        let t = &e.scene.item(id).unwrap().transform;
        assert!(t.translation.abs_diff_eq(DVec3::ZERO, 1e-12));
        assert!(t.rotation.abs_diff_eq(DVec3::ZERO, 1e-12));
        assert!(t.scale.abs_diff_eq(DVec3::ONE, 1e-12));
    }
}

#[test]
fn mirrored_cube_keeps_outward_faces_after_freeze_scale() {
    let mut e = editor();
    let id = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0.; 3], [0.; 3], [-1., 1., 1.]),
        None,
    );
    // Raw world winding is inside out; the negative determinant is what
    // draws it right side out until the bake.
    assert!(!faces_point_out(&e, id));
    let points = world_points(&e, id);
    freeze(&mut e, &[id], TransformReset::Scale).unwrap();
    assert_eq!(e.scene.item(id).unwrap().transform.scale, DVec3::ONE);
    same_points(&world_points(&e, id), &points);
    assert!(faces_point_out(&e, id), "winding reversed with the bake");
}

#[test]
fn offset_mesh_with_rotation_stays_put_for_rotation_and_all() {
    for part in [TransformReset::Rotation, TransformReset::All] {
        let mut e = editor();
        // The mesh sits 3 m off its item centre (the pivot).
        let id = add_cube(
            &mut e,
            DVec3::new(3., -1., 2.),
            transform([0.5, 0., -2.], [30., 60., 15.], [1., 2., 0.5]),
            None,
        );
        let points = world_points(&e, id);
        freeze(&mut e, &[id], part).unwrap();
        same_points(&world_points(&e, id), &points);
        let t = &e.scene.item(id).unwrap().transform;
        assert_eq!(t.rotation, DVec3::ZERO);
        if part == TransformReset::Rotation {
            // Rotation alone keeps position and the non-uniform scale.
            assert_eq!(t.translation, DVec3::new(0.5, 0., -2.));
            assert_eq!(t.scale, DVec3::new(1., 2., 0.5));
        }
    }
}

#[test]
fn rotation_freeze_that_would_shear_a_child_is_skipped_with_a_note() {
    let mut e = editor();
    let parent = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0.; 3], [0., 0., 30.], [1., 3., 1.]),
        None,
    );
    let _child = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0.; 3], [0., 0., 20.], [1.; 3]),
        Some(parent),
    );
    let before = e.scene.clone();
    let err = freeze(&mut e, &[parent], TransformReset::Rotation).unwrap_err();
    assert!(err.contains("shear"), "{err}");
    assert_eq!(e.scene, before);
}

#[test]
fn locked_items_are_skipped_with_one_note() {
    let mut e = editor();
    let a = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 0., 0.], [0.; 3], [2.; 3]),
        None,
    );
    let b = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0., 1., 0.], [0.; 3], [3.; 3]),
        None,
    );
    let c = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0., 0., 1.], [0.; 3], [4.; 3]),
        None,
    );
    e.dispatch(Command::EditItemCommon {
        items: vec![b, c],
        edit: item_common::ItemCommonEdit::Lock(item_common::Choice::Yes),
    })
    .unwrap();
    let locked_b = e.scene.item(b).unwrap().clone();
    freeze(&mut e, &[a, b, c], TransformReset::Scale).unwrap();
    assert_eq!(e.scene.item(a).unwrap().transform.scale, DVec3::ONE);
    assert_eq!(*e.scene.item(b).unwrap(), locked_b);
    assert_eq!(e.transform_note.as_deref(), Some("2 locked items skipped"));
    // Only locked items: refused.
    assert!(freeze(&mut e, &[b], TransformReset::Scale).is_err());
    // Reset skips them the same way.
    e.dispatch(Command::ResetTransform {
        items: vec![a, b],
        part: TransformReset::Position,
    })
    .unwrap();
    assert_eq!(e.transform_note.as_deref(), Some("1 locked item skipped"));
    assert_eq!(*e.scene.item(b).unwrap(), locked_b);
}

#[test]
fn identity_records_no_step_and_non_mesh_items_are_skipped() {
    let mut e = editor();
    let id = add_cube(&mut e, DVec3::ZERO, scene::Transform::default(), None);
    let undo = e.undo.len();
    for part in TransformReset::ALL {
        freeze(&mut e, &[id], part).unwrap();
        e.dispatch(Command::ResetTransform {
            items: vec![id],
            part,
        })
        .unwrap();
    }
    assert_eq!(e.undo.len(), undo, "nothing changed, no undo step");
    // A camera (no geometry) alone can't Freeze; mixed in, it's skipped.
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    let camera = e
        .scene
        .items
        .iter()
        .find(|i| i.camera.is_some())
        .unwrap()
        .id;
    let camera_t = e.scene.item(camera).unwrap().transform.clone();
    assert!(freeze(&mut e, &[camera], TransformReset::All).is_err());
    e.scene.item_mut(id).unwrap().transform.translation = DVec3::X;
    freeze(&mut e, &[camera, id], TransformReset::All).unwrap();
    assert_eq!(e.scene.item(camera).unwrap().transform, camera_t);
    assert_eq!(e.scene.item(id).unwrap().transform.translation, DVec3::ZERO);
}

#[test]
fn multi_select_freezes_each_item_and_one_undo_restores_all() {
    let mut e = editor();
    let a = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 0., 0.], [0., 90., 0.], [1.; 3]),
        None,
    );
    let b = add_cube(
        &mut e,
        DVec3::X,
        transform([0., 2., 0.], [45., 0., 0.], [2., 1., 1.]),
        None,
    );
    let (pa, pb) = (world_points(&e, a), world_points(&e, b));
    let before = e.scene.clone();
    freeze(&mut e, &[a, b], TransformReset::Rotation).unwrap();
    same_points(&world_points(&e, a), &pa);
    same_points(&world_points(&e, b), &pb);
    // Each about its own centre: positions are untouched.
    assert_eq!(e.scene.item(a).unwrap().transform.translation, DVec3::X);
    assert_eq!(
        e.scene.item(b).unwrap().transform.translation,
        DVec3::new(0., 2., 0.)
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
}

#[test]
fn normals_and_morphs_follow_the_bake() {
    let mut e = editor();
    let id = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0.; 3], [0., 0., 90.], [1.; 3]),
        None,
    );
    {
        let mesh = &mut e.scene.item_mut(id).unwrap().mesh;
        let n = mesh.faces.len();
        let rows: Vec<Vec<Option<DVec3>>> = mesh
            .faces
            .iter()
            .map(|f| vec![Some(DVec3::X); f.vertices.len()])
            .collect();
        assert_eq!(rows.len(), n);
        let a = &mut mesh.attributes;
        a.corner_normals.insert("Normals".into(), rows);
        a.relative_morphs
            .insert("Morph".into(), [(0u32, DVec3::X)].into_iter().collect());
    }
    freeze(&mut e, &[id], TransformReset::Rotation).unwrap();
    let a = &e.scene.item(id).unwrap().mesh.attributes;
    let n = a.corner_normals["Normals"][0][0].unwrap();
    assert!(n.abs_diff_eq(DVec3::Y, 1e-12), "{n}");
    let d = a.relative_morphs["Morph"][&0];
    assert!(d.abs_diff_eq(DVec3::Y, 1e-12), "{d}");
}

#[test]
fn mesh_ops_stay_live_on_the_frozen_base_mesh() {
    let mut e = editor();
    let id = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([2., 0., 0.], [0.; 3], [1.; 3]),
        None,
    );
    select(&mut e, &[id]);
    command_exec::run(&mut e, "mirror.clone.create", &Default::default()).unwrap();
    let ops = e.scene.item(id).unwrap().mesh_ops.clone();
    assert!(ops.is_some());
    freeze(&mut e, &[id], TransformReset::Position).unwrap();
    let item = e.scene.item(id).unwrap();
    assert_eq!(item.mesh_ops, ops, "the stack stays as it was");
    assert_eq!(item.transform.translation, DVec3::ZERO);
}

#[test]
fn mesh_light_prototype_keeps_its_world_placement() {
    // The nearest thing to an instance today: a Mesh light drawing a mesh
    // item through the prototype's own transform (FRZT-Q03).
    let mut e = editor();
    let proto = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0., 3., 0.], [0., 30., 0.], [2.; 3]),
        None,
    );
    let points = world_points(&e, proto);
    freeze(&mut e, &[proto], TransformReset::All).unwrap();
    same_points(&world_points(&e, proto), &points);
}

#[test]
fn script_commands_act_on_the_item_selection() {
    let mut e = editor();
    let a = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 2., 3.], [10., 20., 30.], [2.; 3]),
        None,
    );
    let b = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([4., 5., 6.], [0.; 3], [3.; 3]),
        None,
    );
    select(&mut e, &[a, b]);
    let points = world_points(&e, a);
    command_line::run_line(&mut e, "transform.freeze scale")
        .unwrap()
        .unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Freeze Scale");
    same_points(&world_points(&e, a), &points);
    assert_eq!(e.scene.item(b).unwrap().transform.scale, DVec3::ONE);
    // No argument means all.
    command_line::run_line(&mut e, "transform.freeze")
        .unwrap()
        .unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Freeze Transform");
    same_points(&world_points(&e, a), &points);
    command_line::run_line(&mut e, "transform.reset translation")
        .unwrap()
        .unwrap();
    assert!(
        command_line::run_line(&mut e, "transform.reset shear")
            .unwrap()
            .is_err()
    );
    // Reset after Freeze All is a no-op: no new step.
    let undo = e.undo.len();
    command_line::run_line(&mut e, "transform.reset")
        .unwrap()
        .unwrap();
    assert_eq!(e.undo.len(), undo);
}

#[test]
fn reset_moves_the_item_but_not_its_geometry_or_child_channels() {
    let mut e = editor();
    let parent = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 0., 0.], [0., 45., 0.], [2.; 3]),
        None,
    );
    let child = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([0., 1., 0.], [0.; 3], [1.; 3]),
        Some(parent),
    );
    let mesh = e.scene.item(parent).unwrap().mesh.clone();
    let child_t = e.scene.item(child).unwrap().transform.clone();
    e.dispatch(Command::ResetTransform {
        items: vec![parent],
        part: TransformReset::All,
    })
    .unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Reset Transform");
    let p = e.scene.item(parent).unwrap();
    assert_eq!(p.transform, scene::Transform::default());
    assert_eq!(p.mesh, mesh);
    assert_eq!(e.scene.item(child).unwrap().transform, child_t);
}

#[test]
fn rotation_order_is_never_changed() {
    let mut e = editor();
    let mut t = transform([1., 0., 0.], [10., 20., 30.], [1.; 3]);
    t.set_rotation_order(scene::RotationOrder::Zyx);
    let id = add_cube(&mut e, DVec3::ZERO, t, None);
    let points = world_points(&e, id);
    freeze(&mut e, &[id], TransformReset::All).unwrap();
    same_points(&world_points(&e, id), &points);
    assert_eq!(
        e.scene.item(id).unwrap().transform.rotation_order(),
        scene::RotationOrder::Zyx
    );
}

#[test]
fn frozen_scene_saves_and_reopens() {
    let mut e = editor();
    let id = add_cube(
        &mut e,
        DVec3::ZERO,
        transform([1., 2., 3.], [10., 20., 30.], [-1., 2., 1.]),
        None,
    );
    let points = world_points(&e, id);
    freeze(&mut e, &[id], TransformReset::All).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    let w = back.world_matrix(id).unwrap();
    let reopened: Vec<DVec3> = back
        .item(id)
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|p| w.transform_point3(*p))
        .collect();
    same_points(&reopened, &points);
}
