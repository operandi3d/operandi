//! Merge Instance against specifications/missing-tools-g3
//! (G3-TOPOLOGY-TAB v1, MT12, case MI-1), with Q-MI-1 decided as flip
//! (Jason, 2026-11-21) and Q-MI-2 provisional.
use glam::DVec3;
use operandi_core::merge_instance::{NEEDS_INSTANCE, ONE_INSTANCE};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// MI-1's grid: points (i, j, 0), i, j = 0..2, four quads.
fn grid() -> Mesh {
    let mut m = Mesh::default();
    for j in 0..3 {
        for i in 0..3 {
            m.vertices.push(DVec3::new(f64::from(i), f64::from(j), 0.));
        }
    }
    for q in [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]] {
        m.faces.push(face(q.to_vec()));
    }
    m
}

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid().into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e
}

/// Instance Mirror with its defaults (X axis, through the origin).
fn instance_mirror(e: &mut Editor) {
    e.dispatch(Command::Activate(Tool::Mirror)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Instance);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}

fn select_source(e: &mut Editor) {
    let source = e.scene.items[0].id;
    e.dispatch(Command::SelectItem(Some(source))).unwrap();
}

#[test]
fn mi_1_a_mirrored_instance_bakes_in_facing_the_same_way() {
    let mut e = editor();
    instance_mirror(&mut e);
    assert_eq!(e.scene.items.len(), 2);
    select_source(&mut e);
    let before = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::MergeInstance).unwrap();
    assert_eq!(e.scene.items.len(), 1, "the instance is deleted");
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), 18, "seam points not welded (MT12-03)");
    for (k, v) in m.vertices[9..].iter().enumerate() {
        let (i, j) = ((k % 3) as f64, (k / 3) as f64);
        assert!(
            v.abs_diff_eq(DVec3::new(-i, j, 0.), 1e-9),
            "point {} {v}",
            9 + k
        );
    }
    assert_eq!(m.faces.len(), 8);
    // The added polygons use the new points in MI-1's quads, turned round
    // so they face +Z like the original (Q-MI-1: flip).
    for (f, q) in [
        [9, 10, 13, 12],
        [10, 11, 14, 13],
        [12, 13, 16, 15],
        [13, 14, 17, 16],
    ]
    .iter()
    .enumerate()
    {
        let got = &m.faces[4 + f].vertices;
        let mut want: Vec<u32> = q.to_vec();
        want[1..].reverse();
        assert_eq!(got, &want);
        assert!(m.normal(4 + f).z > 0., "polygon {} faces +Z", 4 + f);
    }
    for f in 0..4 {
        assert!(m.normal(f).z > 0.);
    }
    // MT12-04: the source selected, Polygon mode, the added polygons.
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.scene.selection.faces, vec![4, 5, 6, 7]);
    // MT12-05: one undo step.
    assert_eq!(e.history.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items == before.items);
}

#[test]
fn the_instance_itself_or_a_mesh_without_one_is_refused() {
    let mut e = editor();
    assert_eq!(
        e.dispatch(Command::MergeInstance).unwrap_err(),
        NEEDS_INSTANCE
    );
    instance_mirror(&mut e);
    let instance = e.scene.items[1].id;
    e.dispatch(Command::SelectItem(Some(instance))).unwrap();
    let before = e.scene.items.clone();
    assert_eq!(
        e.dispatch(Command::MergeInstance).unwrap_err(),
        NEEDS_INSTANCE
    );
    assert!(e.scene.items == before, "no change");
}

#[test]
fn q_mi_2_two_instances_are_refused() {
    let mut e = editor();
    instance_mirror(&mut e);
    select_source(&mut e);
    e.dispatch(Command::InstanceItems).unwrap();
    select_source(&mut e);
    assert_eq!(
        e.dispatch(Command::MergeInstance).unwrap_err(),
        ONE_INSTANCE
    );
}

#[test]
fn an_unmirrored_instance_keeps_its_winding_and_the_result_reopens() {
    let mut e = editor();
    e.dispatch(Command::InstanceItems).unwrap();
    select_source(&mut e);
    e.dispatch(Command::MergeInstance).unwrap();
    let m: Mesh = (*e.scene.active().unwrap().mesh).clone();
    assert_eq!(m.faces[4].vertices, vec![9, 10, 13, 12]);
    let path = std::env::temp_dir().join(format!("merge-instance-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert!(*back.active().unwrap().mesh == m);
}
