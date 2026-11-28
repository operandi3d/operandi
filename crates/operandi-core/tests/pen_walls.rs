//! Pen Wall Mode (draw-tools Pen): the clicked path becomes a strip of quads
//! in the drawing plane.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e
}
const PATH: [DVec3; 3] = [
    DVec3::new(0., 0., 0.),
    DVec3::new(2., 0., 0.),
    DVec3::new(2., 0., 2.),
];
fn walls(wall: WallMode, offset: f64) -> PenOptions {
    PenOptions {
        wall,
        offset,
        ..Default::default()
    }
}

#[test]
fn both_sides_makes_a_mitred_strip() {
    let (points, quads) = pen_wall(&PATH, &walls(WallMode::BothSides, 0.5)).unwrap();
    assert_eq!((points.len(), quads.len()), (6, 2));
    // The bend's points sit on the mitre: 0.5 from both segments.
    let bend = [points[2], points[3]];
    for p in bend {
        let off_first = p.z.abs();
        let off_second = (p.x - 2.).abs();
        assert!(
            (off_first - 0.5).abs() < 1e-9 && (off_second - 0.5).abs() < 1e-9,
            "{p}"
        );
    }
    let mut m = Mesh {
        vertices: points,
        ..Default::default()
    };
    for q in quads {
        m.faces.push(Face {
            vertices: q,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    m.validate().unwrap();
    // Walls face like a polygon drawn through the same points
    // (pen-2026-11-05): this path turns clockwise seen from +Y.
    assert!(m.normal(0).y < -0.99 && m.normal(1).y < -0.99);
}

#[test]
fn inner_and_outer_put_the_strip_on_one_side() {
    let path = &PATH[..2];
    let (inner, _) = pen_wall(path, &walls(WallMode::Inner, 0.3)).unwrap();
    let (outer, _) = pen_wall(path, &walls(WallMode::Outer, 0.3)).unwrap();
    let zs = |p: &[DVec3]| p.iter().map(|p| p.z).collect::<Vec<_>>();
    // One edge stays on the path; the other is Offset away, on opposite sides.
    let (i, o) = (zs(&inner), zs(&outer));
    assert!(i.iter().any(|z| z.abs() < 1e-12) && o.iter().any(|z| z.abs() < 1e-12));
    let i_far = i.iter().copied().find(|z| z.abs() > 1e-6).unwrap();
    let o_far = o.iter().copied().find(|z| z.abs() > 1e-6).unwrap();
    assert!((i_far.abs() - 0.3).abs() < 1e-12 && (o_far.abs() - 0.3).abs() < 1e-12);
    assert!(i_far * o_far < 0.);
}

#[test]
fn drop_with_walls_adds_quads_in_one_undo_step() {
    let mut e = editor();
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(walls(WallMode::BothSides, 0.25)))
        .unwrap();
    // Two points are enough for a wall.
    for p in &PATH[..2] {
        e.dispatch(Command::PenCapture(*p)).unwrap();
    }
    e.dispatch(Command::Drop).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!((m.vertices.len(), m.faces.len()), (4, 1));
    assert_eq!(e.scene.selection.faces, vec![0]);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
}

#[test]
fn walls_follow_the_drawing_plane_and_flip() {
    for flip in [false, true] {
        let mut e = editor();
        e.dispatch(Command::PenStart).unwrap();
        e.dispatch(Command::PenOptions(PenOptions {
            plane_normal: DVec3::Z,
            flip,
            ..walls(WallMode::Outer, 1.)
        }))
        .unwrap();
        for p in [DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.)] {
            e.dispatch(Command::PenCapture(p)).unwrap();
        }
        e.dispatch(Command::Drop).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(m.faces.len(), 2);
        assert!(m.vertices.iter().all(|v| v.z.abs() < 1e-12));
        let z = m.normal(0).z;
        assert!(if flip { z < -0.99 } else { z > 0.99 }, "flip {flip}: {z}");
    }
}

#[test]
fn walls_need_polygon_type_and_an_offset() {
    let mut e = editor();
    e.dispatch(Command::PenStart).unwrap();
    let bad = PenOptions {
        output: PenType::Polyline,
        ..walls(WallMode::Inner, 1.)
    };
    assert!(e.dispatch(Command::PenOptions(bad)).is_err());
    e.dispatch(Command::PenOptions(walls(WallMode::Inner, 0.)))
        .unwrap();
    for p in &PATH[..2] {
        e.dispatch(Command::PenCapture(*p)).unwrap();
    }
    assert!(e.dispatch(Command::Drop).is_err());
    assert!(
        e.session.pen_draft.is_some(),
        "the draft stays to fix the Offset"
    );
}
