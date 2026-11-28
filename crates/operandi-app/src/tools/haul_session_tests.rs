//! Right-drag Sides and Segments on the create tools that run their own
//! numeric session (GitHub #13): Cone, Ellipsoid, Torus, Capsule, Arc and
//! Tube, as Sphere and Cylinder already do through the drag layer (UIC-76).
//! Sideways is Sides (Arc: Segments), up is Segments; right and up are
//! more; the drag is one undo step.
use super::tests::draw;
use super::*;
use operandi_core::haul::{self, Session, feel};

const SIZE: Vec2 = vec2(1440., 900.);

fn press(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    }
}

fn right_drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![press(from, true)]);
    for k in 1..=8 {
        let at = from + (to - from) * (k as f32 / 8.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    }
    draw(a, ctx, SIZE, vec![press(to, false)]);
}

fn arm(a: &mut Operandi, session: Session) {
    a.run(match session {
        Session::Cone => Command::ArmCone,
        Session::Ellipsoid => Command::ArmEllipsoid,
        Session::Torus => Command::ArmShape(ShapeKind::Torus),
        Session::Capsule => Command::ArmShape(ShapeKind::Capsule),
        Session::Arc => Command::ArmShape(ShapeKind::Arc),
        Session::Tube => Command::ArmShape(ShapeKind::Tube),
    });
}

fn app(ctx: &Context, session: Session) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    arm(&mut a, session);
    for _ in 0..2 {
        draw(&mut a, ctx, SIZE, vec![]);
    }
    a
}

#[test]
fn every_session_row_names_a_number_field_on_its_panel() {
    let ctx = Context::default();
    for session in Session::ALL {
        let row = haul::session_row(session).expect("every session has a row");
        let a = app(&ctx, session);
        for axis in [haul::Axis::Sideways, haul::Axis::Up] {
            if let Some(key) = row.right.get(axis) {
                assert!(
                    a.haul_session_value(key).is_some(),
                    "{session:?} {axis:?}: no number field {key}"
                );
            }
        }
    }
}

#[test]
fn right_drag_changes_sides_and_segments_in_whole_steps() {
    let ctx = Context::default();
    let px = feel::WHOLE_STEP_PX;
    for session in Session::ALL {
        let mut a = app(&ctx, session);
        let row = haul::session_row(session).unwrap();
        let from = a.viewport_rect.center() + vec2(150., 120.);
        let sideways = row.right.sideways.clone().unwrap();
        let before = a.haul_session_value(&sideways).unwrap();
        right_drag(&mut a, &ctx, from, from + vec2(2. * px + 3., 0.));
        assert_eq!(
            a.haul_session_value(&sideways),
            Some(before + 2.),
            "{session:?}: {}",
            a.status
        );
        a.run(Command::Undo);
        assert_eq!(
            a.haul_session_value(&sideways),
            Some(before),
            "{session:?}: one undo step"
        );
        if let Some(up) = row.right.up.clone() {
            let before = a.haul_session_value(&up).unwrap();
            right_drag(&mut a, &ctx, from, from - vec2(0., px + 3.));
            assert_eq!(
                a.haul_session_value(&up),
                Some(before + 1.),
                "{session:?}: {}",
                a.status
            );
        }
    }
}

#[test]
fn a_live_tool_keeps_its_own_row() {
    let ctx = Context::default();
    let mut a = app(&ctx, Session::Cone);
    a.run(Command::Cancel);
    assert!(a.editor.session.cone.is_none());
    assert_eq!(a.haul_session_value("cone_sides"), None);
}
