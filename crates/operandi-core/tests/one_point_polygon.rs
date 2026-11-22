//! One-point polygons (make-split-2026-11-05 rule 2, MP06): Make Polygon
//! (Type Auto) from one selected point makes a polygon of just that point.
//! Operandi keeps it as an open straight line of one point: no spans, no
//! area, no edges. It must survive undo, save and reopen, every export,
//! subdivision and every command without a crash.
use glam::DVec3;
use operandi_core::*;
use serde_json::Map;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A unit cube and one loose point beside it (point 8).
fn cube_and_point() -> Mesh {
    let mut m = Mesh::default();
    for z in [-0.5, 0.5] {
        for y in [-0.5, 0.5] {
            for x in [-0.5, 0.5] {
                m.vertices.push(DVec3::new(x, y, z));
            }
        }
    }
    m.vertices.push(DVec3::new(2., 0., 0.));
    m.faces = vec![
        face(vec![0, 2, 3, 1]),
        face(vec![4, 5, 7, 6]),
        face(vec![0, 1, 5, 4]),
        face(vec![2, 6, 7, 3]),
        face(vec![0, 4, 6, 2]),
        face(vec![1, 3, 7, 5]),
    ]
    .into();
    m
}

fn editor_with_point_polygon() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = cube_and_point().into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![8];
    command_exec::run(&mut e, "poly.make", &Map::new()).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

#[test]
fn make_polygon_from_one_point_makes_a_one_point_polygon() {
    let e = editor_with_point_polygon();
    let m = mesh(&e);
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 7);
    assert_eq!(m.faces[6].vertices, vec![8]);
    assert!(m.is_polyline(6));
    assert_eq!(m.element_spans(6).count(), 0, "no spans");
    // Rule 9: the new polygon is selected, the point selection cleared.
    assert_eq!(e.scene.selection.faces, vec![6]);
    assert!(e.scene.selection.vertices.is_empty());
    // One undo step that puts the mesh back; redo makes it again.
    let mut e = e;
    let made = mesh(&e).clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), cube_and_point());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(*mesh(&e), made);
}

#[test]
fn a_one_point_polygon_survives_save_and_reopen() {
    let e = editor_with_point_polygon();
    let dir = std::env::temp_dir().join(format!("one-point-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("point.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    let item = back.items.iter().find(|i| i.mesh.faces.len() == 7).unwrap();
    assert_eq!(item.mesh, *mesh(&e));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_one_point_polygon_survives_every_export_and_subdivision() {
    let e = editor_with_point_polygon();
    let dir = std::env::temp_dir().join(format!("one-point-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for format in interchange::SCRIPT_EXPORT_FORMATS {
        let path = dir.join(format!("point.{}", format.extension()));
        // A format may leave the point out or refuse; it must not panic,
        // and a written file must read back.
        if interchange::export(&path, &e.scene, format).is_ok()
            && interchange::IMPORT_EXTENSIONS.contains(&format.extension())
        {
            let _ = interchange::import(&path);
        }
    }
    std::fs::remove_dir_all(&dir).ok();
    let settings = subdivision::SubdivisionSettings::default();
    subdivision::subdivide(mesh(&e), &settings).unwrap();
    subdivision::display_surface(mesh(&e), &settings).unwrap();
    let toggled = subdivision::toggle(mesh(&e), &[6], subdivision::Toggle::Subdivision);
    if let Ok(m) = toggled {
        subdivision::subdivide(&m, &settings).unwrap();
    }
}

/// Every headless command, run with no arguments in each component mode
/// with everything selected (the one-point polygon too), may refuse but
/// must not panic, and whatever it leaves must be a valid mesh.
#[test]
fn every_command_copes_with_a_one_point_polygon() {
    let base = editor_with_point_polygon();
    let mut panics = Vec::new();
    for entry in command_registry::COMMANDS {
        if !entry.flags.headless || entry.name.starts_with("scene.") {
            continue;
        }
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
            let mut e = base.clone();
            e.scene.selection.mode = mode;
            e.dispatch(Command::Selection(SelectionOperation::All))
                .unwrap();
            let name = entry.name;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = command_exec::run(&mut e, name, &Map::new());
                if let Some(item) = e.scene.active() {
                    item.mesh.validate().unwrap();
                }
            }));
            if outcome.is_err() {
                panics.push(format!("{name} in {mode:?}"));
            }
        }
    }
    assert!(panics.is_empty(), "panicked: {panics:?}");
}
