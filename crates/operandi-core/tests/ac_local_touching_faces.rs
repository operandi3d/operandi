//! Jason's hands-on report on the step 3 app (2026-11-27): a wedge with its
//! two sloped faces selected, left then right, Local, Rotate. The gizmo
//! sat on the left face, tilted to the left face; he expected it in the
//! middle of the right (last-selected) face, lined up with that face.
//!
//! What happens: the two faces share the ridge edge, so they are one Local
//! cluster (C-LOCAL). The handle sits at that cluster's box middle and
//! takes the cluster's frame, which is fitted to all six points, so the
//! longer left face dominates both. Selection order plays no part inside
//! one cluster.
use glam::{DVec3, dvec3};
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A roof-shaped prism, 2 long along Z. The left slope (polygon 0) is long
/// and shallow, the right slope (polygon 1) short and steep; they meet at
/// the ridge (points 2 and 3). Then the bottom and the two ends. It lies
/// across the X plane: the left slope on the − side, the right on the +.
fn wedge() -> Mesh {
    Mesh {
        vertices: vec![
            dvec3(-2., 0., 0.),
            dvec3(-2., 0., 2.),
            dvec3(0., 1., 0.),
            dvec3(0., 1., 2.),
            dvec3(1., 0., 0.),
            dvec3(1., 0., 2.),
        ],
        faces: vec![
            face(&[0, 1, 3, 2]),
            face(&[4, 2, 3, 5]),
            face(&[0, 4, 5, 1]),
            face(&[0, 2, 4]),
            face(&[1, 5, 3]),
        ]
        .into(),
        ..Default::default()
    }
}

/// The wedge's two slopes as separate polygons that only meet: the right
/// slope has its own ridge points (6 and 7), so they are two clusters.
fn unwelded_slopes() -> Mesh {
    let mut mesh = wedge();
    mesh.vertices.push(dvec3(0., 1., 0.));
    mesh.vertices.push(dvec3(0., 1., 2.));
    mesh.faces = vec![face(&[0, 1, 3, 2]), face(&[4, 6, 7, 5])].into();
    mesh
}

/// Rotate under Local with both slopes selected, left then right.
fn rotate(mesh: Mesh, symmetry: Symmetry) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext::LOCAL))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    e
}

/// The Rotate handle under Local with `faces` selected in that order.
fn handle(faces: Vec<u32>) -> ActionFrame {
    let mut e = rotate(wedge(), Symmetry::OFF);
    e.dispatch(Command::Cancel).unwrap();
    e.scene.selection.faces = faces;
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    e.handle_frame().unwrap()
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}

/// What the code does today, pinned so the report's numbers stay true:
/// one cluster, the handle at its box middle whichever face was picked
/// last, with axes fitted to the whole cluster (tilted toward the longer
/// left face, not along either face's normal).
#[test]
fn touching_faces_are_one_cluster_whatever_the_pick_order() {
    let (left_then_right, right_then_left) = (handle(vec![0, 1]), handle(vec![1, 0]));
    assert_eq!(left_then_right, right_then_left, "pick order plays no part");
    close(
        left_then_right.center,
        dvec3(-0.5, 0.5, 1.),
        "the cluster's box middle",
    );
    let mesh = wedge();
    let (left, right) = (mesh.normal(0), mesh.normal(1));
    let up = left_then_right.basis.z_axis;
    assert!(
        up.dot(left) > up.dot(right) + 0.1,
        "the frame leans to the left face: {up}"
    );
}

/// What Jason expects: the handle on the last-selected face, with that
/// face's own frame, even though it touches another selected face. Fails
/// today (the sheet's C-LOCAL / A-LOCAL say cluster centre and cluster
/// frame instead); which rule to build is Jason's call.
#[test]
#[ignore = "open decision (ACR-Q28): handle on the last-selected face, or on the cluster. Run with: cargo test --locked --workspace -- ac_local_touching_faces --ignored"]
fn the_handle_sits_on_the_last_selected_face() {
    let right_alone = handle(vec![1]);
    close(
        right_alone.center,
        dvec3(0.5, 0.5, 1.),
        "the right face's middle",
    );
    let got = handle(vec![0, 1]);
    close(got.center, right_alone.center, "handle centre");
    for axis in 0..3 {
        close(
            got.basis.col(axis),
            right_alone.basis.col(axis),
            "handle axis",
        );
    }
}

/// With Symmetry off nothing is mirrored (Jason's second report, "as though
/// symmetry is on"): every point the tool moves takes the values as
/// entered, whichever side of X it is on, and each cluster turns as one
/// rigid piece, the same way, about its own frame. Welded slopes are one
/// cluster; slopes that only meet are two.
#[test]
fn without_symmetry_no_point_is_mirrored() {
    use operandi_core::pivot_service::Role;
    for (mesh, clusters) in [(wedge(), 1), (unwelded_slopes(), 2)] {
        for angles in [dvec3(30., 0., 0.), dvec3(0., 30., 0.), dvec3(0., 0., 30.)] {
            let mut e = rotate(mesh.clone(), Symmetry::OFF);
            assert_eq!(e.active_symmetry().axis, None);
            let pivots = e.live_pivots().unwrap().unwrap();
            assert_eq!(pivots.primary, None);
            let groups = &pivots.items[0].groups;
            assert_eq!(groups.len(), clusters);
            let before = e.scene.active().unwrap().mesh.vertices.to_vec();
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.delta = angles;
            e.dispatch(Command::Preview(p)).unwrap();
            let after = &e.scene.active().unwrap().mesh.vertices;
            let turn = glam::DQuat::from_euler(
                glam::EulerRot::XYZ,
                angles.x.to_radians(),
                angles.y.to_radians(),
                angles.z.to_radians(),
            );
            for group in groups {
                let f = group.frame;
                for target in &group.targets {
                    assert_eq!((target.role, target.side), (Role::Primary, None));
                    let v = target.point as usize;
                    let local = f.basis.transpose() * (before[v] - f.center);
                    close(
                        after[v],
                        f.center + f.basis * (turn * local),
                        "the turn as entered",
                    );
                }
            }
        }
    }
}

/// For contrast, what Symmetry X does to the same selection: the + side
/// takes the values as entered and the − side the mirror image, so the
/// left slope turns the other way; the ridge, on the plane, stays on it.
/// This is S-03..S-05 at work, and what the report describes.
#[test]
fn with_symmetry_x_the_minus_side_turns_the_other_way() {
    use operandi_core::pivot_service::Role;
    let mut e = rotate(wedge(), Symmetry::x());
    let pivots = e.live_pivots().unwrap().unwrap();
    assert_eq!(pivots.primary, Some(Side::Positive));
    let role = |point: u32| {
        pivots.items[0].groups[0]
            .targets
            .iter()
            .find(|t| t.point == point)
            .unwrap()
            .role
    };
    assert_eq!([role(0), role(1)], [Role::Other; 2], "left eave");
    assert_eq!([role(2), role(3)], [Role::Seam; 2], "ridge");
    assert_eq!([role(4), role(5)], [Role::Primary; 2], "right eave");
    let before = e.scene.active().unwrap().mesh.vertices.to_vec();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    // The cluster's Y runs along the ridge, toward −Z.
    p.delta = dvec3(0., -30., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    let after = &e.scene.active().unwrap().mesh.vertices;
    // Seen along Z, the right eave goes up and so does the left one: a
    // mirror image, not one rigid turn (which would push the left down).
    assert!(after[4].y > before[4].y + 0.1, "right eave rises");
    assert!(after[0].y > before[0].y + 0.1, "left eave rises too");
    assert_eq!(after[2].x, 0., "the ridge stays on the plane");
}
