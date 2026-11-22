//! Create panel Edit group: Cut, Copy, Paste, Paste Tool (CLP/PST).
use glam::DVec3;
use operandi_core::*;

fn cube_editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.synchronize_identities().unwrap();
    e.undo.clear();
    e
}
fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

#[test]
fn copy_then_paste_adds_a_selected_copy_in_place_as_one_step() {
    let mut e = cube_editor();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    let undo = e.undo.len();
    e.copy_selection().unwrap();
    assert_eq!(e.undo.len(), undo, "Copy isn't an undo step");
    e.dispatch(Command::PasteSelection).unwrap();
    let m = mesh(&e);
    assert_eq!((m.vertices.len(), m.faces.len()), (12, 7), "not welded");
    assert_eq!(e.scene.selection.faces, vec![6]);
    let copy: Vec<DVec3> = m.faces[6]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    let original: Vec<DVec3> = m.faces[0]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    assert_eq!(copy, original);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 6);
}

#[test]
fn cut_polygon_keeps_shared_points_and_undo_brings_it_back() {
    let mut e = cube_editor();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![2];
    e.dispatch(Command::CutSelection).unwrap();
    assert_eq!((mesh(&e).vertices.len(), mesh(&e).faces.len()), (8, 5));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 6);
    // Paste with the cut piece puts it back where it was.
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::PasteSelection).unwrap();
    assert_eq!(mesh(&e).faces.len(), 6);
}

#[test]
fn cut_vertex_takes_the_corner_and_edges_paste_as_lines() {
    let mut e = cube_editor();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0];
    e.dispatch(Command::CutSelection).unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 7);
    assert_eq!(m.faces.iter().filter(|f| f.vertices.len() == 3).count(), 3);
    // Pasted vertices are loose points, selected.
    e.dispatch(Command::PasteSelection).unwrap();
    assert_eq!(mesh(&e).vertices.len(), 8);
    assert_eq!(e.scene.selection.vertices, vec![7]);

    let mut e = cube_editor();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    let (a, b) = {
        let f = &mesh(&e).faces[0].vertices;
        (f[0].min(f[1]), f[0].max(f[1]))
    };
    e.scene.selection.edges = vec![(a, b)];
    e.copy_selection().unwrap();
    e.dispatch(Command::PasteSelection).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 7);
    assert!(m.is_polyline(6));
    assert_eq!(m.faces[6].vertices.len(), 2);
    assert_eq!(e.scene.selection.edges.len(), 1);
}

#[test]
fn paste_keeps_world_position_in_a_moved_mesh() {
    let mut e = cube_editor();
    e.scene.active_mut().unwrap().transform.translation = DVec3::X;
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    e.copy_selection().unwrap();
    // Into a new mesh at the origin.
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::PasteSelection).unwrap();
    let m = mesh(&e);
    let x: f64 = m.vertices.iter().map(|v| v.x).sum::<f64>() / m.vertices.len() as f64;
    let centre = cube_editor().scene.active().unwrap().mesh.center(0).x + 1.;
    assert!((x - centre).abs() < 1e-9, "{x} vs {centre}");
}

#[test]
fn nothing_to_copy_or_paste_is_refused_without_change() {
    let mut e = cube_editor();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    assert!(e.copy_selection().is_err());
    assert!(e.dispatch(Command::PasteSelection).is_err());
    assert_eq!(mesh(&e).faces.len(), 6);
}

#[test]
fn paste_tool_centres_scales_and_biases_in_world_space() {
    let mut e = cube_editor();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(1., 0., 0.);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = (0..6).collect();
    e.copy_selection().unwrap();
    e.dispatch(Command::PasteAt(PastePlacement {
        center: DVec3::new(0., 5., 0.),
        size: DVec3::splat(2.),
        bias: DVec3::new(0., 0., 1.),
    }))
    .unwrap();
    let item = e.scene.active().unwrap();
    let world: Vec<DVec3> = item.mesh.vertices[8..]
        .iter()
        .map(|v| e.scene.world_point(item.id, *v))
        .collect();
    let (lo, hi) = world.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)),
    );
    // PST-Q05 decided: centred on the world Center, not Modo's offset.
    assert!(((lo + hi) / 2. - DVec3::new(0., 5., 1.)).length() < 1e-9);
    assert!((hi - lo - DVec3::splat(2.)).length() < 1e-9);
}
