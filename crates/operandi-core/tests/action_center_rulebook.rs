//! The action-centers rulebook (2026-11-21) for the modes the Local file
//! doesn't cover: Selection and Selection Border's axes (A-SEL, AC-RULES
//! v6, in place of AC-T20/T21), AC-T22 Selection Center
//! Auto Axis's World axes, AC-T40 Screen's axes.
use glam::{DMat3, DVec3};
use operandi_core::*;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// One quad P tilted 45° about X (normal along (0, 1, -1)), box centre
/// (0.5, 0.5, 0.5).
fn tilted(context: ActionContext) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 1., 1.),
            DVec3::new(1., 1., 1.),
            DVec3::new(1., 0., 0.),
        ],
        faces: vec![quad([0, 1, 2, 3])].into(),
        ..Default::default()
    }
    .into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::ActionContext(context)).unwrap();
    e
}

fn frame(e: &mut Editor, tool: Tool) -> ActionFrame {
    e.dispatch(Command::Activate(tool)).unwrap();
    let f = e.session.live.as_ref().unwrap().action_frame.unwrap();
    e.dispatch(Command::Cancel).unwrap();
    f
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!((a - b).length() < 1e-9, "{what}: {a:?} != {b:?}");
}

fn normal() -> DVec3 {
    DVec3::new(0., 1., -1.).normalize()
}

#[test]
fn ac_t22_auto_axis_keeps_world_axes_on_a_tilted_polygon() {
    let mut e = tilted(ActionContext {
        center: CenterMode::SelectionAutoAxis,
        axis: AxisOrientation::World,
    });
    let f = frame(&mut e, Tool::Rotate);
    close(f.center, DVec3::splat(0.5), "box centre");
    assert_eq!(f.basis, DMat3::IDENTITY, "World axes, not tilted");
}

/// A-SEL (AC-RULES v6; it replaces AC-T20/T21's "normal on Y"): the
/// polygon is 1 wide and √2 long up its slope, so X runs up the slope (its
/// longest direction), Z is its normal (its thinnest) and Y is what is
/// left, world X.
#[test]
fn ac_t20_t21_selection_and_border_follow_the_selections_shape() {
    for center in [CenterMode::Selection, CenterMode::SelectionBorder] {
        let mut e = tilted(ActionContext {
            center,
            axis: AxisOrientation::Selection,
        });
        let f = frame(&mut e, Tool::Rotate);
        let slope = DVec3::new(0., 1., 1.).normalize();
        close(f.basis.x_axis, slope, &format!("{center:?} handle X"));
        close(f.basis.y_axis, DVec3::X, &format!("{center:?} handle Y"));
        close(f.basis.z_axis, normal(), &format!("{center:?} handle Z"));
    }
}

#[test]
fn ac_t40_screen_moves_along_the_views_axes() {
    // A view turned 30° about Y: right, up and toward the viewer.
    let turn = DMat3::from_rotation_y(30f64.to_radians());
    let view = PickedFrame::from_basis(turn);
    let mut e = tilted(ActionContext {
        center: CenterMode::Screen(PickedPoint::new(DVec3::splat(0.5))),
        axis: AxisOrientation::Screen(view),
    });
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(1., 0., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = &e.scene.active().unwrap().mesh.vertices;
    for (b, a) in before.iter().zip(after) {
        close(*a, *b + turn.x_axis, "moved along the screen's right");
    }
    assert_eq!(AxisOrientation::Screen(view).label(), "Screen");
}
