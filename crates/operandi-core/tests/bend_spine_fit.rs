//! BN-24 revised, BN-30b (bend-handles-2026-11-15, 2026-11-16): every
//! centre click refits Bend's spine along the work-plane axis with the
//! larger far-side distance, and puts the Axis on the plane's normal.
//! Original test data: the spec's acceptance boxes.
use glam::DVec3;
use operandi_core::bend::{click_axis, fit_spine};
use operandi_core::*;

/// The front view's work plane (XY).
const FRONT: u8 = 2;
/// Looking down (XZ).
const GROUND: u8 = 0;

fn near(a: DVec3, b: DVec3) -> bool {
    (a - b).length() < 0.011
}

/// The acceptance box: x -1.4..1.4, y 0..2, z -0.5..0.5.
fn acceptance_box() -> (DVec3, DVec3) {
    (DVec3::new(-1.4, 0., -0.5), DVec3::new(1.4, 2., 0.5))
}

#[test]
fn bn24_front_view_acceptance_clicks() {
    let (lo, hi) = acceptance_box();
    let cases = [
        (DVec3::new(1.37, 0.95, 0.5), DVec3::new(-2.77, 0., 0.)),
        (DVec3::new(0.67, 1.86, 0.5), DVec3::new(-2.07, 0., 0.)),
        (DVec3::new(0.06, 1.88, 0.5), DVec3::new(0., -1.88, 0.)),
        // Off the box: the far side is the other end; the spec's figure
        // is 3.28 to its rounding.
        (DVec3::new(-1.89, 0.96, 0.), DVec3::new(3.28, 0., 0.)),
    ];
    for (click, want) in cases {
        let got = fit_spine(lo, hi, click, FRONT);
        assert!(near(got, want), "click {click:?}: {got:?}, want {want:?}");
    }
}

/// A 1 x 2 x 2.8 m box seen from the front: the long Z side points at the
/// viewer, so it never wins; the spine runs down Y, 1.08 m.
#[test]
fn bn24_never_along_the_view_axis() {
    let (lo, hi) = (DVec3::new(-0.5, -1., -1.4), DVec3::new(0.5, 1., 1.4));
    let got = fit_spine(lo, hi, DVec3::new(0.1, 0.08, 1.4), FRONT);
    assert!(near(got, DVec3::new(0., -1.08, 0.)), "{got:?}");
    assert_eq!(got.z, 0.);
    // Looking down, the candidates are X and Z, so Z wins there.
    let got = fit_spine(lo, hi, DVec3::new(0.1, 1., 0.2), GROUND);
    assert!(near(got, DVec3::new(0., 0., -1.6)), "{got:?}");
}

/// BN-30b: the Axis a click sets is the plane's normal, whatever it was.
#[test]
fn bn30b_axis_is_the_work_plane_normal() {
    assert_eq!(click_axis(FRONT, glam::DMat3::IDENTITY), 2);
    assert_eq!(click_axis(GROUND, glam::DMat3::IDENTITY), 1);
    assert_eq!(click_axis(1, glam::DMat3::IDENTITY), 0);
}

/// The acceptance box as a live mesh, Bend picked.
fn bend_on_box() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(0., 1., 0.), DVec3::new(2.8, 2., 1.), 0)
            .unwrap()
            .into();
    e.scene.selection.mode = Mode::Polygon;
    e.history.undo.clear();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    e
}
fn bend(e: &Editor) -> BendOptions {
    e.session.live.as_ref().unwrap().params.bend
}

/// BN-24 / CS-20: each click refits, the first and every later one, before
/// and after the bend has changed the mesh; the box is the mesh as Bend
/// found it. A click also resets the Axis (BN-30b); the X/Y/Z choice
/// stays until the next click.
#[test]
fn every_click_refits_the_spine_and_resets_the_axis() {
    let mut e = bend_on_box();
    let click = |e: &mut Editor, x: f64, y: f64| {
        e.dispatch(Command::ToolClick {
            point: DVec3::new(x, y, 0.5),
            plane: FRONT,
        })
        .unwrap();
    };
    click(&mut e, 1.37, 0.95);
    assert!(near(bend(&e).spine, DVec3::new(-2.77, 0., 0.)));
    assert_eq!(bend(&e).base, DVec3::new(1.37, 0.95, 0.5));
    assert_eq!(bend(&e).axis, 2);
    // The user picks X, then bends.
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.bend.axis = 0;
    p.bend.angle = 45.;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(bend(&e).axis, 0);
    // A later click on the bent mesh refits over the box as it was.
    click(&mut e, 0.06, 1.88);
    assert!(near(bend(&e).spine, DVec3::new(0., -1.88, 0.)));
    assert_eq!(bend(&e).axis, 2, "a click resets the Axis");
    assert_eq!(bend(&e).angle, 45., "Angle stays");
    // Bad input is refused and changes nothing.
    let before = bend(&e);
    assert!(
        e.dispatch(Command::ToolClick {
            point: DVec3::NAN,
            plane: FRONT
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::ToolClick {
            point: DVec3::ZERO,
            plane: 7
        })
        .is_err()
    );
    assert_eq!(bend(&e), before);
}

/// A selection is the target: its box, not the whole mesh's.
#[test]
fn the_selection_is_the_target() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(0., 1., 0.), DVec3::new(2.8, 2., 1.), 0)
            .unwrap()
            .into();
    e.scene.selection.mode = Mode::Polygon;
    // One face only: whichever it is, the fit uses its own box.
    e.scene.selection.faces = [0].into_iter().collect();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let (lo, hi) = falloff::selection_bounds(&e.scene).unwrap();
    let at = DVec3::new(0.2, 0.3, 0.);
    e.dispatch(Command::ToolClick {
        point: at,
        plane: FRONT,
    })
    .unwrap();
    assert!(near(bend(&e).spine, fit_spine(lo, hi, at, FRONT)));
}
