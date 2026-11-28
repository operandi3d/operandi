//! Jason's hands-on report (2026-11-28): a plain cube, Polygons, Symmetry
//! off, Local, its two opposite X walls selected. Dragging the ring whose
//! axis is the direction the walls share leaned them opposite ways (a
//! wedge); he expected both to lean the same way (a sheared box).
//!
//! The cause was Local's axes rule (AC-Q5): a wall's X went on its outward
//! normal with Y kept up, so the wall facing −X got the opposite Z of the
//! one facing +X. A-LOCAL (AC-RULES v6) puts Z on the normal and X along
//! the surface, pointed the same way on both walls.
use glam::{DMat3, DQuat, DVec3, dvec3};
use operandi_core::pivot_service::Role;
use operandi_core::*;

/// A 1 m cube at the origin, or a box of `size`.
fn editor(size: DVec3) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube_geometry(DVec3::ZERO, size).unwrap().into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext::LOCAL))
        .unwrap();
    e
}

/// The cube's polygon facing `normal`.
fn facing(e: &Editor, normal: DVec3) -> u32 {
    let mesh = &e.scene.active().unwrap().mesh;
    (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).normalize().distance(normal) < 1e-9)
        .unwrap() as u32
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}

fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.to_vec()
}

/// The three pairs of opposite walls: the outward normal of the first, and
/// the direction the pair shares that Local's X (the red ring) lies along.
const PAIRS: [(DVec3, DVec3); 3] = [
    (DVec3::X, DVec3::Z),
    (DVec3::Z, DVec3::X),
    (DVec3::Y, DVec3::X),
];

/// Each wall's frame: Z out of the wall, X the shared direction, pointed
/// the same way on both walls, Y = Z × X. The handle is the last-selected
/// wall's.
#[test]
fn opposite_walls_share_their_x_and_have_z_outward() {
    for (normal, shared) in PAIRS {
        for last in [normal, -normal] {
            let mut e = editor(DVec3::ONE);
            e.scene.selection.faces = vec![facing(&e, -last), facing(&e, last)];
            e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
            let pivots = e.live_pivots().unwrap().unwrap();
            assert_eq!(pivots.primary, None, "Symmetry is off");
            let groups = &pivots.items[0].groups;
            assert_eq!(groups.len(), 2, "two walls, two clusters");
            for group in groups {
                let out = (group.frame.center * 2.).normalize();
                close(group.frame.basis.z_axis, out, "Z out of the wall");
                close(group.frame.basis.x_axis, shared, "X the shared direction");
                close(group.frame.basis.y_axis, out.cross(shared), "Y = Z × X");
                assert!(group.targets.iter().all(|t| t.role == Role::Primary));
            }
            let handle = e.handle_frame().unwrap();
            close(handle.center, last * 0.5, "on the last-selected wall");
            close(handle.basis.z_axis, last, "with its frame");
        }
    }
}

/// The acceptance case, typed: Rotate X (the red ring) turns both walls
/// the same way about the direction they share, each about its own
/// middle. The cube becomes a sheared box: the two walls stay parallel.
/// Whichever wall was picked last.
#[test]
fn rotate_about_the_shared_direction_turns_both_walls_the_same_way() {
    for (normal, shared) in PAIRS {
        for last in [normal, -normal] {
            let mut e = editor(DVec3::ONE);
            let walls = [facing(&e, -last), facing(&e, last)];
            e.scene.selection.faces = walls.to_vec();
            let before = points(&e);
            e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.delta = dvec3(25., 0., 0.);
            e.dispatch(Command::Preview(p)).unwrap();
            let during = points(&e);
            e.dispatch(Command::Drop).unwrap();
            let after = points(&e);
            assert_eq!(during, after, "the preview is the dropped result");
            let turn = DQuat::from_axis_angle(shared, 25f64.to_radians());
            let mesh = e.scene.active().unwrap().mesh.clone();
            for (wall, out) in walls.into_iter().zip([-last, last]) {
                for &v in &mesh.faces[wall as usize].vertices {
                    let centre = out * 0.5;
                    close(
                        after[v as usize],
                        centre + turn * (before[v as usize] - centre),
                        &format!("wall facing {out}, point {v}"),
                    );
                }
            }
            // Sheared, not a wedge: the walls' normals still oppose.
            let (a, b) = (
                mesh.normal(walls[0] as usize).normalize(),
                mesh.normal(walls[1] as usize).normalize(),
            );
            close(a, -b, "the walls stay parallel");
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(points(&e), before, "undo");
            e.dispatch(Command::Redo).unwrap();
            assert_eq!(points(&e), after, "redo");
        }
    }
}

/// Walls that aren't square take the same axes: a tall wall facing X gets
/// its level direction for X (the item's Z), not its longer upright one.
#[test]
fn a_tall_wall_facing_x_takes_its_level_direction() {
    let mut e = editor(dvec3(1., 3., 2.));
    for out in [DVec3::X, -DVec3::X] {
        e.scene.selection.faces = vec![facing(&e, out)];
        let frame = e.pivot_request(&e.scene, Tool::Rotate).resolve().unwrap();
        close(frame.handle.basis.x_axis, DVec3::Z, "X level");
        close(frame.handle.basis.z_axis, out, "Z out");
    }
}

/// The frames of the everyday cases, so a change to them is seen: a floor
/// polygon, a wall facing Z, a slope.
#[test]
fn everyday_local_frames() {
    let mut e = editor(dvec3(2., 1., 1.));
    let axes = |e: &mut Editor, out: DVec3| {
        e.scene.selection.faces = vec![facing(e, out)];
        e.pivot_request(&e.scene, Tool::Move)
            .resolve()
            .unwrap()
            .handle
            .basis
    };
    // The top: Z up, X along the item's X (also its longer side).
    let top = axes(&mut e, DVec3::Y);
    close(top.x_axis, DVec3::X, "top X");
    close(top.y_axis, -DVec3::Z, "top Y");
    close(top.z_axis, DVec3::Y, "top Z");
    // The front wall: Z toward the viewer, X to the right, Y up.
    let front = axes(&mut e, DVec3::Z);
    assert_eq!(front, DMat3::IDENTITY);
    // The back wall: X still to +X, so Y points down.
    let back = axes(&mut e, -DVec3::Z);
    close(back.x_axis, DVec3::X, "back X");
    close(back.y_axis, -DVec3::Y, "back Y");
    close(back.z_axis, -DVec3::Z, "back Z");
}

/// Two mirror-image roof slopes that only meet at the ridge (two
/// clusters): their Y runs along the ridge the same way on both, so
/// Rotate Y turns both the same way; Z is each slope's own normal.
#[test]
fn mirror_image_slopes_turn_the_same_way_about_the_ridge() {
    let quad = |v: [u32; 4]| Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut e = editor(DVec3::ONE);
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            dvec3(-1., 0., 0.),
            dvec3(-1., 0., 3.),
            dvec3(0., 2., 3.),
            dvec3(0., 2., 0.),
            dvec3(1., 0., 0.),
            dvec3(0., 2., 0.),
            dvec3(0., 2., 3.),
            dvec3(1., 0., 3.),
        ],
        faces: vec![quad([0, 1, 2, 3]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
    .into();
    e.synchronize_identities().unwrap();
    e.scene.selection.faces = vec![0, 1];
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let pivots = e.live_pivots().unwrap().unwrap();
    let mesh = e.scene.active().unwrap().mesh.clone();
    for (group, face) in pivots.items[0].groups.iter().zip([0, 1]) {
        close(
            group.frame.basis.z_axis,
            mesh.normal(face).normalize(),
            "Z is the slope's normal",
        );
        close(group.frame.basis.y_axis, -DVec3::Z, "Y along the ridge");
    }
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(0., 20., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    let after = points(&e);
    let turn = DQuat::from_axis_angle(-DVec3::Z, 20f64.to_radians());
    for (centre, slope) in [(dvec3(-0.5, 1., 1.5), 0..4), (dvec3(0.5, 1., 1.5), 4..8)] {
        for v in slope {
            close(after[v], centre + turn * (before[v] - centre), "one turn");
        }
    }
}

/// With Symmetry X on, the same selection is a mirrored pair (S-09): the
/// − wall takes the mirror image, so here the walls do lean opposite ways.
#[test]
fn symmetry_x_still_mirrors_the_walls() {
    let mut e = editor(DVec3::ONE);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.scene.selection.faces = vec![facing(&e, -DVec3::X), facing(&e, DVec3::X)];
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(25., 0., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    for v in points(&e) {
        let twin = dvec3(-v.x, v.y, v.z);
        assert!(
            points(&e).iter().any(|w| w.distance(twin) < 1e-9),
            "{v} has its mirror image"
        );
    }
}
