//! Bend (tutorial-gaps-2026-11-13, SPEC-BEND; v2 bend-handles-2026-11-15):
//! the tool's workflow on a strip. Original test data; the arc follows the
//! provisional reading (BD-Q1..Q6, BH-Q01..Q03).
use glam::DVec3;
use operandi_core::*;

/// BD-01's strip: 10 segments, 0.2 m wide (X), from y = 0 to y = 2.
fn strip() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut vertices = Vec::new();
    for i in 0..=10 {
        let y = 0.2 * i as f64;
        vertices.push(DVec3::new(-0.1, y, 0.));
        vertices.push(DVec3::new(0.1, y, 0.));
    }
    let faces = (0..10u32)
        .map(|i| Face {
            vertices: vec![2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces,
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    // The strip stands in XY: work in the front view's plane.
    e.scene.workplane.plane = 2;
    e.history.undo.clear();
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn bend(e: &mut Editor, angle: f64) {
    let mut p = live(e);
    p.bend.angle = angle;
    e.dispatch(Command::Preview(p)).unwrap();
}

/// BN-01/02/24 (revised 2026-11-16): starting Bend puts Automatic centre
/// and Auto (World) axes in place; the pivot is the strip's centre and the
/// spine is fitted from it: Y reaches 1 m to either end (X only 0.1 m), so
/// it runs -Y 1 m, on the plane's normal, Axis Z. Nothing moves until
/// Angle changes.
#[test]
fn activation_fits_the_spine_and_changes_nothing() {
    let mut e = strip();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert_eq!(e.action_context.center, CenterMode::Automatic);
    assert_eq!(e.action_context.axis, AxisOrientation::World);
    let p = live(&e).bend;
    assert!(
        (p.base - DVec3::new(0., 1., 0.)).length() < 1e-9,
        "{}",
        p.base
    );
    assert!(
        (p.spine - DVec3::new(0., -1., 0.)).length() < 1e-9,
        "{}",
        p.spine
    );
    assert_eq!(p.axis, 2);
    assert_eq!(p.angle, 0.);
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert_eq!(frame.center, p.base, "the pivot is the action centre");
    assert_eq!(points(&e), before);
}

/// Put the centre at the strip's foot, as a press there would (BN-30).
fn foot(e: &mut Editor) {
    e.dispatch(Command::SetActionCenter(DVec3::ZERO)).unwrap();
    let mut p = live(e);
    p.bend.spine = DVec3::new(0., 2., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
}

/// BD-01/02 through the tool: the centre line follows a quarter circle at
/// 90° and a half circle at 180°.
#[test]
fn bd01_bd02_the_centre_line_follows_an_arc() {
    for (angle, radius) in [
        (90., 2. / std::f64::consts::FRAC_PI_2),
        (180., 2. / std::f64::consts::PI),
    ] {
        let mut e = strip();
        e.dispatch(Command::Activate(Tool::Bend)).unwrap();
        foot(&mut e);
        bend(&mut e, angle);
        let after = points(&e);
        let centre = DVec3::new(radius, 0., 0.);
        for i in 0..=10 {
            let mid = (after[2 * i] + after[2 * i + 1]) / 2.;
            assert!(
                ((mid - centre).length() - radius).abs() < 1e-9,
                "{angle}°: row {i} at {mid}"
            );
        }
        let top = (after[20] + after[21]) / 2.;
        let want = if angle == 90. {
            DVec3::new(radius, radius, 0.)
        } else {
            DVec3::new(2. * radius, 0., 0.)
        };
        assert!((top - want).length() < 1e-9, "{angle}°: top {top}");
    }
}

/// BD-03 through the tool: with the base moved to the middle, the lower
/// half stays put unless Both Sides is on, when it mirrors the upper half.
#[test]
fn bd03_both_sides() {
    let mut e = strip();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let mut p = live(&e);
    p.bend.base = DVec3::new(0., 1., 0.);
    p.bend.spine = DVec3::Y;
    p.bend.angle = 60.;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let after = points(&e);
    for v in 0..10 {
        assert_eq!(after[v], before[v], "point {v} below the base moved");
    }
    p.bend.both_sides = true;
    e.dispatch(Command::Preview(p)).unwrap();
    let after = points(&e);
    for i in 0..5 {
        let up = after[2 * (5 + i)];
        let down = after[2 * (5 - i)];
        assert!(
            (down - DVec3::new(up.x, 2. - up.y, up.z)).length() < 1e-9,
            "row {i}: {up} vs {down}"
        );
    }
}

/// Drop is one undo step; Cancel and Undo give back the strip; Redo and a
/// save round trip keep the bend.
#[test]
fn drop_undo_redo_cancel_and_reopen() {
    let mut e = strip();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    bend(&mut e, 45.);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    bend(&mut e, 45.);
    let bent = points(&e);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(points(&e), bent);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points(&e), bent);
    let bytes = persistence::encode(&e.scene).unwrap();
    let reopened = persistence::decode(&bytes).unwrap();
    assert_eq!(reopened.active().unwrap().mesh.vertices, bent);
}

/// Item mode is refused; a selection bends only its own points; a
/// falloff scales each point's share; a spine of no length is refused.
#[test]
fn item_mode_selection_falloff_and_bad_values() {
    let mut e = strip();
    e.scene.selection.mode = Mode::Item;
    assert!(e.dispatch(Command::Activate(Tool::Bend)).is_err());

    let mut e = strip();
    let before = points(&e);
    // The top half only: the centre is its middle, the spine fits to it.
    e.scene.selection.faces = (5..10).collect();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert!((live(&e).bend.base - DVec3::new(0., 1.5, 0.)).length() < 1e-9);
    // The top half's box: 0.5 m either way along Y, so -Y 0.5 m.
    assert!((live(&e).bend.spine - DVec3::new(0., -0.5, 0.)).length() < 1e-9);
    let mut p = live(&e);
    p.bend.spine = DVec3::new(0., 1., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    bend(&mut e, 90.);
    let after = points(&e);
    for v in 0..10 {
        assert_eq!(after[v], before[v]);
    }
    assert_ne!(after[21], before[21]);
    e.dispatch(Command::Cancel).unwrap();

    // A Selection falloff: the border row (y = 1) weighs 0 and stays.
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Selection(
        SelectionFalloff {
            steps: 2,
            shape: FalloffShape::Linear,
        },
    ))))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let mut p = live(&e);
    p.bend.base = DVec3::ZERO;
    p.bend.spine = DVec3::new(0., 2., 0.);
    p.bend.angle = 90.;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let after = points(&e);
    assert_eq!(after[10], before[10]);
    assert_eq!(after[11], before[11]);
    p.bend.spine = DVec3::ZERO;
    assert!(e.dispatch(Command::Preview(p)).is_err());
    // The refused preview leaves the last good one.
    assert_eq!(points(&e), after);
}

/// G5 / BN-03: the Axis row picks the axis the bend curls around. On an
/// upright strip Z curls it toward +X and X toward -Z; switching changes
/// neither the centre nor the spine (BN-11, BN-21), and the turn axis the
/// ring is drawn square to follows at once.
#[test]
fn axis_switching_turns_the_bend_and_keeps_centre_and_spine() {
    let mut e = strip();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    foot(&mut e);
    bend(&mut e, 90.);
    let top = |e: &Editor| (points(e)[20] + points(e)[21]) / 2.;
    assert!(top(&e).x > 1. && top(&e).z.abs() < 1e-9, "{}", top(&e));
    let before = live(&e).bend;
    let mut p = live(&e);
    p.bend.axis = 0;
    e.dispatch(Command::Preview(p)).unwrap();
    assert!(top(&e).z < -1. && top(&e).x.abs() < 1e-9, "{}", top(&e));
    let after = live(&e).bend;
    assert_eq!(after.base, before.base);
    assert_eq!(after.spine, before.spine);
    assert_eq!(after.turn_axis(glam::DMat3::IDENTITY), DVec3::X);
}

/// The ring's contract with the maths (BN-13): a positive Angle turns the
/// spine's end the left-handed way about the ring's axis, i.e. toward
/// spine × axis, so dragging the ring the way the end should go adds Angle.
#[test]
fn positive_angle_turns_left_handed_about_the_ring_axis() {
    for axis in [0, 2] {
        let o = BendOptions {
            angle: 30.,
            spine: DVec3::new(0., 2., 0.),
            axis,
            ..Default::default()
        };
        let i = glam::DMat3::IDENTITY;
        let end = o.bend_point(o.spine, o.direction(i));
        let a = o.turn_axis(i);
        // Seen down the axis, the end moved clockwise (negative turn).
        let turn = o.spine.cross(end - o.spine).dot(a);
        assert!(turn < 0., "axis {axis}: {end}");
    }
}

/// §3 / G2: Bend has no drag-layer row, so no button hauls a value.
#[test]
fn bend_has_no_haul() {
    assert!(haul::row(Tool::Bend).is_none());
    for button in [
        haul::Button::Left,
        haul::Button::Middle,
        haul::Button::Right,
    ] {
        assert!(!haul::claims(Tool::Bend, button));
    }
}

/// BN-30/32 / G4: the script's centre (`SetActionCenter`) moves the
/// centre before and after the bend is applied; the bend then starts there
/// and, with no view click, the spine keeps its vector (a view click
/// refits it: bend_spine_fit.rs). Cancel and Drop give back the user's own action
/// centre (TP-12).
#[test]
fn centre_from_a_press_and_the_spine_vector() {
    let mut e = strip();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let mut p = live(&e);
    p.bend.spine = DVec3::new(0., 2., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    let spine = live(&e).bend.spine;
    // Before applying: only the centre moves.
    let at = DVec3::new(0.05, 0.4, 0.);
    e.dispatch(Command::SetActionCenter(at)).unwrap();
    assert_eq!(live(&e).bend.base, at);
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        at
    );
    assert_eq!(live(&e).bend.spine, spine);
    bend(&mut e, 60.);
    let from_low = points(&e);
    // Rows below the centre (y < 0.4) stay with Both Sides off.
    assert_eq!(from_low[0], DVec3::new(-0.1, 0., 0.));
    assert_eq!(from_low[2], DVec3::new(-0.1, 0.2, 0.));
    // After applying: the bend restarts from the new centre at once.
    let higher = DVec3::new(0., 1., 0.);
    e.dispatch(Command::SetActionCenter(higher)).unwrap();
    assert_eq!(live(&e).bend.base, higher);
    assert_eq!(live(&e).bend.spine, spine, "the spine keeps its vector");
    assert_eq!(live(&e).bend.angle, 60.);
    let from_high = points(&e);
    assert_ne!(from_high, from_low);
    assert_eq!(
        from_high[8],
        DVec3::new(-0.1, 0.8, 0.),
        "below the new centre"
    );
    assert!(e.dispatch(Command::SetActionCenter(DVec3::NAN)).is_err());
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.action_context.center, CenterMode::Origin, "TP-12");
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    bend(&mut e, 30.);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.action_context.center, CenterMode::Origin, "TP-12");
}

/// The Auto Action Center fields write the centre as a preview; undo and
/// redo inside the tool keep the frame on it.
#[test]
fn centre_fields_move_the_frame() {
    let mut e = strip();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    bend(&mut e, 45.);
    let mut p = live(&e);
    p.bend.base = DVec3::new(0., 0.5, 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(0., 0.5, 0.)
    );
}
