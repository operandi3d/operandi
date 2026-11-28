//! Interactive Pen panel options: Type, Flip, Close and Select New, with the
//! numeric Pen's measured meanings (PEN-N02/N03).
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![2];
    e.synchronize_identities().unwrap();
    e
}
const SQUARE: [DVec3; 4] = [
    DVec3::new(3., 0., 0.),
    DVec3::new(4., 0., 0.),
    DVec3::new(4., 1., 0.),
    DVec3::new(3., 1., 0.),
];
fn draw(e: &mut Editor, options: PenOptions, points: &[DVec3]) {
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(options)).unwrap();
    for &p in points {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
}
fn options(output: PenType) -> PenOptions {
    PenOptions {
        output,
        ..Default::default()
    }
}

#[test]
fn flip_reverses_the_new_polygon() {
    for flip in [false, true] {
        let mut e = editor();
        draw(
            &mut e,
            PenOptions {
                flip,
                ..options(PenType::Polygon)
            },
            &SQUARE,
        );
        e.dispatch(Command::Drop).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 7);
        let z = m.normal(6).z;
        assert!(if flip { z < -0.99 } else { z > 0.99 }, "flip {flip}: {z}");
        assert_eq!(e.scene.selection.faces, vec![6]);
    }
}

#[test]
fn polyline_open_and_closed_and_its_two_point_minimum() {
    for close in [false, true] {
        let mut e = editor();
        draw(
            &mut e,
            PenOptions {
                close,
                ..options(PenType::Polyline)
            },
            &SQUARE[..1],
        );
        // One point is not a line: Drop discards it (draw-tools Pen, seen
        // 2026-11-05; it used to be refused with the draft kept).
        let scene = e.scene.clone();
        e.dispatch(Command::Drop).unwrap();
        assert!(
            e.session.pen_draft.is_none() && e.scene == scene,
            "one point is not a line"
        );
        draw(
            &mut e,
            PenOptions {
                close,
                ..options(PenType::Polyline)
            },
            &SQUARE[..1],
        );
        e.dispatch(Command::PenCapture(SQUARE[1])).unwrap();
        e.dispatch(Command::PenCapture(SQUARE[2])).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 7);
        assert_eq!(m.faces[6].vertices, vec![8, 9, 10]);
        assert_eq!(
            m.attributes.representations[6],
            PolygonRepresentation::Polyline {
                closed: close,
                curve: Default::default()
            }
        );
        assert_eq!(e.scene.selection.faces, vec![6]);
        let after = e.scene.clone();
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, after);
    }
}

#[test]
fn vertices_add_points_only_and_select_them() {
    let mut e = editor();
    draw(&mut e, options(PenType::Vertices), &SQUARE[..2]);
    e.dispatch(Command::Drop).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!((m.vertices.len(), m.faces.len()), (10, 6));
    assert_eq!(m.vertices[9], SQUARE[1]);
    assert!(e.scene.selection.faces.is_empty());
    assert_eq!(e.scene.selection.vertices, vec![8, 9]);
}

#[test]
fn select_new_off_keeps_the_selection() {
    let mut e = editor();
    let before = e.scene.selection.clone();
    draw(
        &mut e,
        PenOptions {
            select_new: false,
            ..options(PenType::Polygon)
        },
        &SQUARE,
    );
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 7);
    assert_eq!(e.scene.selection, before);
}

#[test]
fn invalid_combinations_refuse_and_options_carry_to_the_next_session() {
    let mut e = editor();
    e.dispatch(Command::PenStart).unwrap();
    let draft = e.session.pen_draft.clone();
    // Flip and Close no longer depend on the type (pen-2026-11-05: Close
    // does nothing on a polygon); out-of-range values still refuse.
    for bad in [
        PenOptions {
            segments: 0,
            ..options(PenType::Polygon)
        },
        PenOptions {
            inset: -1.,
            ..options(PenType::Polygon)
        },
        PenOptions {
            wall: WallMode::Inner,
            offset: 0.1,
            ..options(PenType::Polyline)
        },
    ] {
        assert!(e.dispatch(Command::PenOptions(bad)).is_err(), "{bad:?}");
        assert_eq!(e.session.pen_draft, draft);
    }
    // Options need a Pen session.
    e.dispatch(Command::Cancel).unwrap();
    assert!(
        e.dispatch(Command::PenOptions(PenOptions::default()))
            .is_err()
    );
    let polyline = PenOptions {
        close: true,
        ..options(PenType::Polyline)
    };
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(polyline)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::PenStart).unwrap();
    assert_eq!(e.session.pen_draft.as_ref().unwrap().options(), polyline);
}
