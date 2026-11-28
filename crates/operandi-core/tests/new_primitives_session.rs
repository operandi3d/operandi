//! Torus, Capsule, Arc and Tube through the editor: arm, preview, Apply,
//! Drop, Cancel, Reset, remembered settings, undo/redo, save/reopen.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e
}
fn counts(e: &Editor) -> (usize, usize) {
    let m = &e.scene.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}
/// Arming draws nothing (Jason, 2026-11-05); place the remembered shape the
/// way a panel edit or viewport drag would.
fn place(e: &mut Editor) {
    let p = e.session.shape.as_ref().unwrap().requested.clone();
    e.dispatch(Command::ConfigureShape(p)).unwrap();
}
fn tube_points() -> Vec<DVec3> {
    vec![DVec3::ZERO, DVec3::new(1., 1., 0.), DVec3::new(2., 0., 0.)]
}

#[test]
fn arming_previews_and_drop_appends_one_undo_step() {
    let mut e = editor();
    let base = e.scene.clone();
    e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
    assert_eq!(
        e.session.active_task(),
        Some(ActiveTask::Shape(ShapeKind::Torus))
    );
    assert!(
        e.shape_preview_scene().unwrap().is_none(),
        "nothing until drawn"
    );
    place(&mut e);
    let preview = e.shape_preview_scene().unwrap().unwrap();
    assert_eq!(preview.active().unwrap().mesh.vertices.len(), 288);
    // The preview isn't committed yet.
    assert_eq!(e.scene, base);
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.shape.is_none());
    assert_eq!(counts(&e), (288, 288));
    assert_eq!(e.history.undo.last().unwrap().label, "Torus");
    // Nothing is selected afterwards (every NPR oracle).
    let s = &e.scene.selection;
    assert!(s.vertices.is_empty() && s.edges.is_empty() && s.faces.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(counts(&e), (0, 0));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(counts(&e), (288, 288));
}

#[test]
fn cancel_leaves_the_scene_as_it_was() {
    let mut e = editor();
    let base = e.scene.clone();
    let history = e.history.undo.len();
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, base);
    assert_eq!(e.history.undo.len(), history);
    assert!(!e.session.has_active_task());
}

#[test]
fn needs_a_mesh() {
    let mut e = Editor::default();
    let err = e.dispatch(Command::ArmShape(ShapeKind::Arc)).unwrap_err();
    assert!(err.contains("New Mesh"), "{err}");
    assert!(e.session.shape.is_none());
}

#[test]
fn invalid_values_are_refused_until_corrected() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    let bad = CapsuleParameters {
        radius: DVec3::new(0., 0.75, 0.25),
        ..Default::default()
    };
    assert!(
        e.dispatch(Command::ConfigureShape(ShapeParameters::Capsule(bad)))
            .is_err()
    );
    let s = e.session.shape.as_ref().unwrap();
    assert!(s.error.is_some() && s.preview.is_none());
    assert!(e.dispatch(Command::Apply).is_err());
    assert!(e.dispatch(Command::Drop).is_err());
    e.dispatch(Command::ConfigureShape(ShapeParameters::Capsule(
        Default::default(),
    )))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e), (242, 264));
}

#[test]
fn apply_keeps_the_tool_for_another_shape() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Arc)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&e), (17, 1));
    assert!(e.session.shape.is_some());
    let moved = ArcParameters {
        center: DVec3::new(3., 0., 0.),
        ..Default::default()
    };
    e.dispatch(Command::ConfigureShape(ShapeParameters::Arc(moved)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&e), (34, 2));
    assert_eq!(
        e.history.undo.iter().filter(|c| c.label == "Arc").count(),
        2
    );
    // Drop after Apply with nothing new previewed adds nothing.
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e), (34, 2));
}

#[test]
fn settings_are_remembered_and_reset_restores_them() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
    let small = TorusParameters {
        sides: 8,
        segments: 4,
        ..Default::default()
    };
    e.dispatch(Command::ConfigureShape(ShapeParameters::Torus(
        small.clone(),
    )))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
    assert_eq!(
        e.session.shape.as_ref().unwrap().requested,
        ShapeParameters::Torus(small)
    );
    e.dispatch(Command::ResetShape).unwrap();
    assert_eq!(
        e.session.shape.as_ref().unwrap().requested,
        ShapeParameters::Torus(Default::default())
    );
    assert_eq!(
        e.presets.get::<operandi_core::ShapePreset>().torus,
        TorusParameters::default()
    );
}

#[test]
fn tube_waits_for_points() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Tube)).unwrap();
    assert!(e.session.shape.as_ref().unwrap().preview.is_none());
    assert!(e.dispatch(Command::Apply).is_err() || counts(&e) == (0, 0));
    let tube = TubeParameters {
        points: tube_points(),
        sides: 4,
        segments: 2,
        radius: 0.25,
        make_uvs: false,
        ..Default::default()
    };
    e.dispatch(Command::ConfigureShape(ShapeParameters::Tube(tube)))
        .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e), (20, 18));
}

#[test]
fn another_tool_drops_the_shown_shape_first() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
    place(&mut e);
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    assert_eq!(counts(&e), (288, 288));
    place(&mut e);
    assert_eq!(
        e.session.active_task(),
        Some(ActiveTask::Shape(ShapeKind::Capsule))
    );
    assert_eq!(e.session.open_task_slots(), 1);
    // The same tool's button drops it.
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    assert!(e.session.shape.is_none());
    assert_eq!(counts(&e), (288 + 242, 288 + 264));
}

#[test]
fn selection_waits_for_the_tool() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Arc)).unwrap();
    assert!(e.dispatch(Command::ClearSelection).is_err());
    assert!(e.session.shape.is_some());
}

#[test]
fn saved_and_reopened_shapes_match() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    let grid = CapsuleParameters {
        ends: CapsuleEnds::QuadGrid,
        ..Default::default()
    };
    e.dispatch(Command::ConfigureShape(ShapeParameters::Capsule(grid)))
        .unwrap();
    let path = std::env::temp_dir().join(format!("operandi-npr-{}.opscene", std::process::id()));
    // Save drops the tool first, committing the preview.
    e.save_committed(&path).unwrap();
    let reopened = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(
        reopened.active().unwrap().mesh.vertices,
        e.scene.active().unwrap().mesh.vertices
    );
    assert_eq!(
        reopened.active().unwrap().mesh.faces,
        e.scene.active().unwrap().mesh.faces
    );
}

/// Settings beyond the oracles still make valid, closed meshes.
#[test]
fn settings_sweep_makes_valid_meshes() {
    for axis in 0..3 {
        for sides in [3, 5, 8, 12, 16, 24] {
            for end_segments in 1..4 {
                for ends in [CapsuleEnds::Polar, CapsuleEnds::QuadGrid] {
                    let p = CapsuleParameters {
                        axis,
                        sides,
                        end_segments,
                        ends,
                        segments: 2,
                        ..Default::default()
                    };
                    let m = capsule_geometry(&p).unwrap();
                    assert!(!m.edges().is_empty());
                    // Closed: every edge used twice.
                    assert_eq!(
                        m.edges().len() * 2,
                        m.faces.iter().map(|f| f.vertices.len()).sum::<usize>(),
                        "{p:?}"
                    );
                    if ends == CapsuleEnds::QuadGrid && sides.is_multiple_of(4) {
                        assert!(m.faces.iter().all(|f| f.vertices.len() == 4), "{p:?}");
                    }
                }
            }
            let t = TorusParameters {
                axis,
                sides,
                segments: sides,
                bulge_top: 0.7,
                bulge_side: -0.5,
                ..Default::default()
            };
            assert_eq!(
                torus_geometry(&t).unwrap().faces.len() as u32,
                sides * sides
            );
        }
    }
    for closed in [false, true] {
        let tube = TubeParameters {
            points: vec![
                DVec3::ZERO,
                DVec3::new(1., 0.5, 0.3),
                DVec3::new(1., 2., -1.),
                DVec3::new(-1., 1., 0.),
            ],
            closed,
            twist: 45.,
            by_length: true,
            ..Default::default()
        };
        tube_geometry(&tube).unwrap();
    }
    let arc = ArcParameters {
        angle: 180.,
        segments: 1,
        ..Default::default()
    };
    // A one-segment half disc is flat: kept as a degenerate record.
    assert_eq!(arc_geometry(&arc).unwrap().faces.len(), 1);
}

#[test]
fn arc_center_start_end_links() {
    let mut a = ArcParameters::default();
    a.set_angle(270.);
    assert_eq!(a.angle, 180.);
    a.set_angle(90.);
    a.set_center(DVec3::new(1., 2., 3.));
    assert!((a.start() - a.center).length() - 1. < 1e-12);
    a.radius = 2.;
    assert!(((a.end() - a.center).length() - 2.).abs() < 1e-12);
    let end = a.center + DVec3::new(0., 0., 2.);
    a.set_end(end);
    assert!((a.end() - end).length() < 1e-9);
}
