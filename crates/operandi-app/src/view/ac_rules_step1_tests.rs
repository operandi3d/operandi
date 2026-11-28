//! Action Center rule sheet v6, step 1, seen from the app: the handle's
//! arrows are the pivot service's frame (A-SEL under Symmetry), Element is
//! in force as soon as it is chosen, and Polygon Bevel's arrows under
//! Selection axes. The rules themselves are tested in core
//! (`ac_rules_step1.rs`).
use super::*;

fn quad(v: [u32; 4]) -> operandi_core::Face {
    operandi_core::Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn turn() -> glam::DQuat {
    glam::DQuat::from_rotation_y(0.35) * glam::DQuat::from_rotation_z(0.2)
}

/// A box 3 long, 1 thick and 2 wide, turned and set on the + side of the X
/// plane (points 0..8, polygons 0..6, the last one its inner end), and its
/// mirror image (points + 8, polygons + 6).
fn prongs() -> operandi_core::Mesh {
    let mut m = operandi_core::Mesh::default();
    for along in [0., 3.] {
        for (thick, wide) in [(0., 0.), (1., 0.), (1., 2.), (0., 2.)] {
            m.vertices
                .push(DVec3::new(4., 1., 0.5) + turn() * DVec3::new(along, thick, wide));
        }
    }
    for side in 0..4u32 {
        let (a, b) = (side, (side + 1) % 4);
        m.faces.push(quad([a, b, b + 4, a + 4]));
    }
    m.faces.push(quad([4, 5, 6, 7]));
    m.faces.push(quad([3, 2, 1, 0]));
    for i in 0..8 {
        let p = m.vertices[i];
        m.vertices.push(DVec3::new(-p.x, p.y, p.z));
    }
    for f in 0..6 {
        let mut v: Vec<u32> = m.faces[f].vertices.iter().map(|v| v + 8).collect();
        v.reverse();
        m.faces.push(quad([v[0], v[1], v[2], v[3]]));
    }
    m
}

fn app(mesh: operandi_core::Mesh, faces: Vec<u32>) -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = faces;
    a
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}

/// The screenshot: Selection Border with Symmetry X on box polygons. The
/// arrows the view draws are the frame's columns, each along one of the +
/// box's own directions, and the handle sits on the + side's border.
#[test]
fn selection_border_with_symmetry_draws_arrows_square_to_the_faces() {
    // Everything but the two inner ends: each box's border is its inner
    // ring.
    let mut a = app(prongs(), vec![0, 1, 2, 3, 4, 6, 7, 8, 9, 10]);
    a.run(Command::Symmetry(operandi_core::Symmetry::x()));
    a.run(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionBorder,
        axis: AxisOrientation::Selection,
    }));
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        a.activate(tool);
        let frame = a.editor.live.as_ref().unwrap().action_frame.unwrap();
        for axis in 0..3 {
            let arrow = a.handle_axis(axis);
            close(
                arrow,
                frame.basis.col(axis),
                "the arrow is the frame's axis",
            );
            let best = [DVec3::X, DVec3::Y, DVec3::Z]
                .into_iter()
                .map(|d| (turn() * d).dot(arrow).abs())
                .fold(0., f64::max);
            assert!(best > 1. - 1e-9, "{tool:?} arrow {axis} is tilted: {arrow}");
        }
        close(a.handle_axis(0), turn() * DVec3::X, "X along the box");
        close(
            a.transform_handle_center(),
            DVec3::new(4., 1., 0.5) + turn() * DVec3::new(0., 0.5, 1.),
            "on the + side's border ring",
        );
        a.run(Command::Cancel);
    }
}

/// C-ELEM / A-ELEM: choosing Element puts it in force at once, at the
/// world origin with World axes, whatever was on before. (What presses do
/// after that is in `ac_press_tests.rs`.)
#[test]
fn choosing_element_is_the_world_origin_until_something_is_picked() {
    let mut a = app(prongs(), vec![0, 1]);
    a.run(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionBorder,
        axis: AxisOrientation::Selection,
    }));
    a.choose_element_center();
    assert_eq!(a.editor.action_context, ActionContext::ELEMENT_UNPICKED);
    a.activate(Tool::Rotate);
    assert_eq!(a.transform_handle_center(), DVec3::ZERO);
    for (axis, world) in [DVec3::X, DVec3::Y, DVec3::Z].into_iter().enumerate() {
        close(a.handle_axis(axis), world, "World axes");
    }
    // Choosing it again keeps the last pick.
    let picked = ActionContext::ELEMENT_UNPICKED.picked(operandi_core::ActionFrame {
        center: DVec3::new(1., 2., 3.),
        basis: operandi_core::element_axes(DVec3::X, DVec3::ZERO),
    });
    a.run(Command::Cancel);
    a.run(Command::ActionContext(picked));
    a.choose_element_center();
    assert_eq!(a.editor.action_context, picked);
}

/// Polygon Bevel under Selection axes: the Shift arrow is the selection's
/// average normal (A-SEL no longer carries the normal on Y), and the Inset
/// arrow lies square to it along the selection's long direction.
#[test]
fn bevel_arrows_under_selection_axes_follow_the_normal() {
    // The top of the + box: 3 long, 2 wide, facing the box's own +Y.
    let mut a = app(prongs(), vec![1]);
    a.run(Command::ActionContext(ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::Selection,
    }));
    a.activate(Tool::Bevel);
    assert_eq!(a.editor.live.as_ref().map(|l| l.tool), Some(Tool::Bevel));
    let normal = {
        let item = a.editor.scene.active().unwrap();
        item.mesh.normal(1)
    };
    close(a.handle_axis(0), normal, "Shift along the normal");
    let inset = a.handle_axis(1);
    assert!(inset.dot(normal).abs() < 1e-9, "Inset square to the normal");
    close(inset, turn() * DVec3::X, "Inset along the polygon's length");
}
