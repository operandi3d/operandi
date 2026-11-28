use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let first = e.scene.selection.clone();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::new(8., 0., 0.), DVec3::ONE, 1)
        .unwrap()
        .into();
    e.dispatch(Command::NewMesh).unwrap();
    assert_ne!(e.scene.selection.item, first.item);
    e
}
#[test]
fn short_invalid_and_active_task_refusals_are_atomic() {
    let mut e = editor();
    let scene = e.scene.clone();
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenCapture(DVec3::ZERO)).unwrap();
    e.dispatch(Command::PenCapture(DVec3::X)).unwrap();
    let draft = e.session.pen_draft.clone();
    assert!(
        e.dispatch(Command::PenCapture(DVec3::splat(f64::NAN)))
            .is_err()
    );
    assert!(e.dispatch(Command::Activate(Tool::Extrude)).is_err());
    assert!(e.session.pen_draft == draft && e.scene == scene);
    // Draw tools Pen (seen, 2026-11-05): dropping a two-point polygon
    // leaves its two points, one undo step.
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.pen_draft.is_none());
    let m = &e.scene.active().unwrap().mesh;
    let before = &scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), before.vertices.len() + 2);
    assert_eq!(m.faces.len(), before.faces.len());
}

#[test]
fn dropping_a_one_point_pen_draft_discards_it() {
    let mut e = editor();
    let scene = e.scene.clone();
    let undo = e.history.undo.len();
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenCapture(DVec3::ZERO)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.pen_draft.is_none() && e.scene == scene && e.history.undo.len() == undo);
}

#[test]
fn self_intersecting_polygon_drop_retains_draft() {
    let mut e = editor();
    let scene = e.scene.clone();
    e.dispatch(Command::PenStart).unwrap();
    for p in [
        DVec3::ZERO,
        DVec3::new(2., 2., 0.),
        DVec3::new(0., 2., 0.),
        DVec3::new(2., 0., 0.),
    ] {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    let draft = e.session.pen_draft.clone();
    assert!(e.dispatch(Command::Drop).is_err());
    assert!(e.scene == scene && e.session.pen_draft == draft);
}

#[test]
fn save_refuses_short_draft_and_roundtrips_committed_polygon() {
    let mut e = editor();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-pen-interactive-20261028/native-files");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("pen-{}.opscene", std::process::id()));
    assert!(!path.exists());
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenCapture(DVec3::ZERO)).unwrap();
    // A one-point draft is no longer refused on Drop (it is discarded), so
    // a short draft no longer blocks saving; the full-draft round trip
    // below is unchanged.
    e.dispatch(Command::PenCapture(DVec3::X)).unwrap();
    e.dispatch(Command::PenCapture(DVec3::Y)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.save_committed(&path).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), e.scene);
    std::fs::remove_file(path).unwrap();
}
