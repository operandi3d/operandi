//! #37: move arrows and radius squares on a drawn Cone, Ellipsoid, Torus,
//! Capsule and Text.
use super::tests::box_app;
use super::*;
use crate::session_handles::SessionShape;
use operandi_core::shapes::ShapeParameters;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// Shapes 0..4: Cone, Ellipsoid, Torus, Capsule, drawn solid (Ctrl drag)
/// in the default view.
fn drawn(which: usize) -> Operandi {
    let mut a = box_app();
    a.run(match which {
        0 => Command::ArmCone,
        1 => Command::ArmEllipsoid,
        2 => Command::ArmShape(ShapeKind::Torus),
        _ => Command::ArmShape(ShapeKind::Capsule),
    });
    let r = rect();
    let plane = a.actual_plane();
    let from = a.camera.project(DVec3::new(2., 0., 0.), r).unwrap().0;
    let to = a.camera.project(DVec3::new(3., 0., 1.), r).unwrap().0;
    if which < 2 {
        let cone = which == 0;
        let input::GestureKind::SolidDraw { anchor, .. } = a.begin_solid_draw(cone, from, r) else {
            panic!("no draw: {}", a.status);
        };
        a.solid_draw(cone, plane, anchor, to, r, true);
    } else {
        let input::GestureKind::Numeric(drag) = a.begin_numeric_drag(from, r) else {
            panic!("no draw: {}", a.status);
        };
        a.numeric_drag(&drag, from, to, r, true);
    }
    a
}

/// Drags handle `id` along its axis by `pixels` on screen.
fn drag(a: &mut Operandi, id: usize, pixels: f32) {
    let r = rect();
    let (_, _, end, _) = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == id)
        .unwrap_or_else(|| panic!("handle {id} missing: {:?}", a.handles(r)));
    let base = a.session_shape().unwrap();
    let axis = if id < 3 { id } else { (id - 3) / 2 };
    let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
    let o = a.camera.project(base.position, r).unwrap().0;
    let u = a.camera.project(base.position + direction, r).unwrap().0;
    let step = (u - o).normalized() * pixels;
    let Some(input::GestureKind::SessionHandle { id: hit, base }) = a.begin_session_handle(end, r)
    else {
        panic!("handle {id} didn't grab: {}", a.status);
    };
    assert_eq!(hit, id);
    a.session_handle_drag(id, base, end, end + step, r);
}

#[test]
fn drawn_solids_show_move_arrows_and_radius_squares() {
    for which in 0..4 {
        let a = drawn(which);
        let ids: Vec<usize> = a.handles(rect()).iter().map(|h| h.0).collect();
        // Top view: Y is end-on, so its arrow is left out.
        for id in [0, 2] {
            assert!(ids.contains(&id), "shape {which} move arrow {id}: {ids:?}");
        }
        // The squares on the axes seen side-on (the default view sees all).
        assert!(
            ids.iter().filter(|&&id| id >= 3).count() >= 4,
            "shape {which}: {ids:?}"
        );
    }
}

#[test]
fn a_move_arrow_moves_the_shape_and_keeps_its_size() {
    for which in 0..4 {
        let mut a = drawn(which);
        let before = a.session_shape().unwrap();
        drag(&mut a, 0, 60.);
        let after = a.session_shape().unwrap();
        assert!(
            after.position.x > before.position.x + 0.05,
            "shape {which}: {before:?} -> {after:?} {}",
            a.status
        );
        assert_eq!(after.position.y, before.position.y, "shape {which}");
        assert_eq!(after.radius, before.radius, "shape {which}");
    }
}

#[test]
fn a_radius_square_sizes_from_the_centre_or_the_far_side() {
    for which in 0..4 {
        let mut a = drawn(which);
        let before = a.session_shape().unwrap();
        // +X square: 3 + axis 0 * 2 + 1.
        drag(&mut a, 4, 60.);
        let after = a.session_shape().unwrap();
        let (b, c) = (before.radius.unwrap().x, after.radius.unwrap().x);
        assert!(
            c > b + 0.02,
            "shape {which} +X grew: {b} -> {c} {}",
            a.status
        );
        if before.from_center {
            assert_eq!(after.position, before.position, "shape {which}");
        } else {
            // The -X side stays where it was.
            let far = |s: SessionShape| s.position.x - s.radius.unwrap().x;
            assert!((far(after) - far(before)).abs() < 1e-9, "shape {which}");
        }
        assert!(
            a.editor
                .session
                .cone
                .as_ref()
                .is_none_or(|s| s.preview.is_some())
        );
    }
}

#[test]
fn a_flat_base_keeps_only_its_height_handle() {
    let mut a = box_app();
    a.run(Command::ArmEllipsoid);
    let r = rect();
    let plane = a.actual_plane();
    let from = a.camera.project(DVec3::new(2., 0., 0.), r).unwrap().0;
    let to = a.camera.project(DVec3::new(3., 0., 1.), r).unwrap().0;
    let input::GestureKind::SolidDraw { anchor, .. } = a.begin_solid_draw(false, from, r) else {
        panic!("no draw");
    };
    a.solid_draw(false, plane, anchor, to, r, false);
    assert!(a.session_height_handle(r).is_some());
    assert!(a.handles(r).is_empty());
}

#[test]
fn the_torus_squares_set_its_radius_like_the_panel() {
    let mut a = drawn(2);
    let torus = |a: &Operandi| match &a.editor.session.shape.as_ref().unwrap().requested {
        ShapeParameters::Torus(t) => t.clone(),
        _ => unreachable!(),
    };
    let before = torus(&a);
    drag(&mut a, 3 + 1, 40.);
    let after = torus(&a);
    // Across the axis: Cross Section grows and Hole Size is kept.
    assert!(after.cross_section > before.cross_section);
    assert!((after.hole() - before.hole()).abs() < 1e-9);
}

#[test]
fn text_gets_move_arrows_only() {
    let mut a = box_app();
    a.run(Command::ArmShape(ShapeKind::Text));
    let Some(ShapeParameters::Text(mut t)) =
        a.editor.session.shape.as_ref().map(|s| s.requested.clone())
    else {
        panic!("no text session");
    };
    t.text = "A".into();
    a.run(Command::ConfigureShape(ShapeParameters::Text(t)));
    if a.editor.session.shape.as_ref().unwrap().preview.is_none() {
        // No font in this build: nothing drawn, so no handles either.
        assert!(a.handles(rect()).is_empty());
        return;
    }
    let ids: Vec<usize> = a.handles(rect()).iter().map(|h| h.0).collect();
    // Top view: X and Z arrows (Y is end-on), and no squares.
    assert_eq!(ids, vec![0, 2], "{}", a.status);
    let before = a.session_shape().unwrap().position;
    drag(&mut a, 0, 60.);
    assert!(a.session_shape().unwrap().position.x > before.x);
}

/// #42: Arc and Teapot had no handles at all once drawn. Like Text they
/// get the move arrows (their sizes are single numbers on the panel).
#[test]
fn arc_and_teapot_get_move_arrows() {
    for kind in [ShapeKind::Arc, ShapeKind::Teapot] {
        let mut a = box_app();
        a.run(Command::ArmShape(kind));
        let p = a.editor.session.shape.as_ref().unwrap().requested.clone();
        // Drawn: configured once, as a drag or a panel edit does.
        a.run(Command::ConfigureShape(p));
        assert!(
            a.editor.session.shape.as_ref().unwrap().preview.is_some(),
            "{kind:?} drawn: {}",
            a.status
        );
        let ids: Vec<usize> = a.handles(rect()).iter().map(|h| h.0).collect();
        // Top view: X and Z arrows (Y is end-on), and no squares.
        assert_eq!(ids, vec![0, 2], "{kind:?}: {}", a.status);
        let before = a.session_shape().unwrap().position;
        drag(&mut a, 0, 60.);
        let after = a.session_shape().unwrap().position;
        assert!(after.x > before.x, "{kind:?} moved along X");
        assert!((after.z - before.z).abs() < 1e-9, "{kind:?} only along X");
    }
}
