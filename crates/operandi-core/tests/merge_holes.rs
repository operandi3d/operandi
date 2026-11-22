//! Merge of a region with holes (CMD-P-05, CMD-O-merge-ring-with-hole): one
//! keyhole polygon that walks the outer border and each hole, joined by a
//! doubled bridge, so the bridge's two points appear twice. It must survive
//! undo, save and reopen, drawing, subdivision and every command.
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

/// A `w` × `h` grid of unit quads in XY, missing the quads in `holes`
/// (column, row). Point (x, y) is `y * (w + 1) + x`.
fn grid_with_holes(w: u32, h: u32, holes: &[(u32, u32)]) -> Mesh {
    let mut m = Mesh::default();
    for y in 0..=h {
        for x in 0..=w {
            m.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let p = |x: u32, y: u32| y * (w + 1) + x;
    let mut faces = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !holes.contains(&(x, y)) {
                faces.push(face(vec![
                    p(x, y),
                    p(x + 1, y),
                    p(x + 1, y + 1),
                    p(x, y + 1),
                ]));
            }
        }
    }
    m.faces = faces.into();
    m
}

fn merged(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let faces = mesh.faces.len() as u32;
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = (0..faces).collect();
    command_exec::run(&mut e, "poly.merge", &Map::new()).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

/// Twice the signed area in XY.
fn area2(m: &Mesh, f: usize) -> f64 {
    let v = &m.faces[f].vertices;
    (0..v.len())
        .map(|i| {
            let (a, b) = (
                m.vertices[v[i] as usize],
                m.vertices[v[(i + 1) % v.len()] as usize],
            );
            a.x * b.y - b.x * a.y
        })
        .sum()
}

/// The area the drawn triangles cover (all must face the polygon's way).
fn filled(m: &Mesh, f: usize) -> f64 {
    let v = &m.faces[f].vertices;
    m.surface_triangles(f)
        .unwrap()
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| m.vertices[v[i] as usize]);
            let z = (b - a).cross(c - a).z / 2.;
            assert!(z > 0., "a triangle faces the other way");
            z
        })
        .sum()
}

#[test]
fn a_ring_merges_into_the_measured_keyhole() {
    let e = merged(grid_with_holes(3, 3, &[(1, 1)]));
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1);
    assert_eq!(
        m.faces[0].vertices,
        vec![0, 1, 2, 3, 7, 11, 15, 14, 10, 6, 5, 9, 10, 14, 13, 12, 8, 4]
    );
    // The bridge 10-14 stays an edge; the other interior edges are gone.
    assert!(m.edges().contains(&(10, 14)));
    assert!(!m.edges().contains(&(1, 5)));
    assert!((area2(m, 0) - 16.).abs() < 1e-9, "9 - 1 squares, twice");
    assert!(
        (filled(m, 0) - 8.).abs() < 1e-9,
        "the triangles leave the hole open"
    );
    // Undo and redo.
    let mut e = e;
    let after = mesh(&e).clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), grid_with_holes(3, 3, &[(1, 1)]));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(*mesh(&e), after);
}

#[test]
fn two_holes_join_into_one_polygon() {
    let e = merged(grid_with_holes(5, 3, &[(1, 1), (3, 1)]));
    let m = mesh(&e);
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 1);
    // 15 - 2 squares; two bridges, each two repeated points.
    assert!((area2(m, 0) - 26.).abs() < 1e-9);
    assert_eq!(m.faces[0].vertices.len(), 16 + 4 + 4 + 4);
    assert!((filled(m, 0) - 13.).abs() < 1e-9);
}

#[test]
fn a_keyhole_survives_save_reopen_subdivision_and_every_command() {
    let base = merged(grid_with_holes(3, 3, &[(1, 1)]));
    let dir = std::env::temp_dir().join(format!("merge-holes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ring.opscene");
    persistence::save(&path, &base.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    assert!(back.items.iter().any(|i| i.mesh == *mesh(&base)));
    for format in interchange::SCRIPT_EXPORT_FORMATS {
        let file = dir.join(format!("ring.{}", format.extension()));
        if interchange::export(&file, &base.scene, format).is_ok()
            && interchange::IMPORT_EXTENSIONS.contains(&format.extension())
        {
            let _ = interchange::import(&file);
        }
    }
    std::fs::remove_dir_all(&dir).ok();
    let settings = subdivision::SubdivisionSettings::default();
    let smooth = subdivision::toggle(mesh(&base), &[0], subdivision::Toggle::Subdivision).unwrap();
    subdivision::subdivide(&smooth, &settings).unwrap();
    subdivision::display_surface(&smooth, &settings).unwrap();
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
