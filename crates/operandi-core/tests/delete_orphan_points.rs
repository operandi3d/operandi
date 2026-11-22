//! Forward Delete (`delete`) leaves no orphan points (Jason, 2026-11-15):
//! a point the delete leaves with nothing attached goes too, in Edge and
//! Polygon mode, with Symmetry, as one undo step. Points that were already
//! unused stay, and Backspace (Edge Remove / Remove) still keeps points.
#![allow(clippy::field_reassign_with_default)] // meshes built field by field
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

fn line(closed: bool) -> PolygonRepresentation {
    PolygonRepresentation::Polyline {
        closed,
        curve: Default::default(),
    }
}

/// A lone quad (points 0..4) and an open two-point line (points 4, 5): the
/// stray edge with no polygons.
fn quad_and_stray_edge() -> Mesh {
    let mut m = Mesh::default();
    m.vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(3., 0., 0.),
        DVec3::new(4., 0., 0.),
    ];
    m.faces = vec![face(vec![0, 1, 2, 3]), face(vec![4, 5])].into();
    m.attributes.representations = vec![PolygonRepresentation::Face, line(false)];
    m
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = mode;
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn unused(m: &Mesh) -> usize {
    m.used_points().iter().filter(|u| !**u).count()
}

/// Run `command` as one undo step; undo puts the scene back exactly.
fn run_one_step(e: &mut Editor, command: &str, label: &str) {
    let start = e.scene.clone();
    let steps = e.undo.len();
    command_exec::run(e, command, &Map::new()).unwrap();
    assert_eq!(e.undo.len(), steps + 1, "{command} is one undo step");
    assert_eq!(e.undo.last().unwrap().label, label);
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, start);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

#[test]
fn deleting_a_stray_edge_takes_its_points() {
    let mut e = editor(quad_and_stray_edge(), Mode::Edge);
    e.scene.selection.edges = vec![(4, 5)];
    run_one_step(&mut e, "delete", "Delete");
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 4, "both end points went");
    assert_eq!(m.faces.len(), 1);
    assert_eq!(
        m.faces[0].vertices,
        vec![0, 1, 2, 3],
        "the quad is untouched"
    );
    assert_eq!(unused(m), 0);
}

#[test]
fn deleting_a_lone_polygons_edge_leaves_no_points() {
    // The edge's points leave the quad, which goes under three corners;
    // its other two corners had nothing else and go too.
    let mut e = editor(quad_and_stray_edge(), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    run_one_step(&mut e, "delete", "Delete");
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 2, "only the stray line's points stay");
    assert_eq!(m.faces.len(), 1, "the untouched line stays");
    assert!(m.is_polyline(0));
    assert_eq!(unused(m), 0);
}

#[test]
fn deleting_a_lone_triangles_edge_leaves_no_points() {
    let mut m = Mesh::default();
    m.vertices = vec![DVec3::ZERO, DVec3::X, DVec3::Y];
    m.faces = vec![face(vec![0, 1, 2])].into();
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    run_one_step(&mut e, "delete", "Delete");
    assert!(mesh(&e).vertices.is_empty());
    assert!(mesh(&e).faces.is_empty());
}

#[test]
fn a_cut_line_splits_or_opens() {
    // Open line 0-1-2-3: deleting the middle span leaves 0-1 and 2-3.
    let mut m = Mesh::default();
    m.vertices = (0..4).map(|i| DVec3::new(i as f64, 0., 0.)).collect();
    m.faces = vec![face(vec![0, 1, 2, 3])].into();
    m.attributes.representations = vec![line(false)];
    let mut e = editor(m.clone(), Mode::Edge);
    e.scene.selection.edges = vec![(1, 2)];
    run_one_step(&mut e, "delete", "Delete");
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), 4);
    let lines: Vec<_> = out.faces.iter().map(|f| f.vertices.clone()).collect();
    assert_eq!(lines, vec![vec![0, 1], vec![2, 3]]);
    // The end span: the line shortens and its end point goes.
    let mut e = editor(m.clone(), Mode::Edge);
    e.scene.selection.edges = vec![(2, 3)];
    command_exec::run(&mut e, "delete", &Map::new()).unwrap();
    assert_eq!(mesh(&e).vertices.len(), 3);
    assert_eq!(mesh(&e).faces[0].vertices, vec![0, 1, 2]);
    // A closed line opens at the cut, keeping every point.
    m.attributes.representations = vec![line(true)];
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = vec![(0, 3)];
    command_exec::run(&mut e, "delete", &Map::new()).unwrap();
    let out = mesh(&e);
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices, vec![0, 1, 2, 3]);
    assert_eq!(out.attributes.representations[0], line(false));
}

#[test]
fn points_that_were_already_unused_stay() {
    let mut m = quad_and_stray_edge();
    m.vertices.push(DVec3::new(9., 9., 9.));
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = vec![(4, 5)];
    command_exec::run(&mut e, "delete", &Map::new()).unwrap();
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), 5);
    assert_eq!(out.vertices[4], DVec3::new(9., 9., 9.));
}

#[test]
fn backspace_remove_still_keeps_points() {
    // Edge Remove (CMD-E-01) and polygon Remove keep points; only forward
    // Delete cleans up.
    let mut e = editor(quad_and_stray_edge(), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    command_exec::run(&mut e, "edge.remove", &Map::new()).unwrap();
    assert_eq!(unused(mesh(&e)), 2);
    let mut e = editor(quad_and_stray_edge(), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    command_exec::run(&mut e, "poly.remove", &Map::new()).unwrap();
    assert_eq!(unused(mesh(&e)), 4);
}

#[test]
fn polygon_delete_takes_a_lone_polygons_points() {
    let mut e = editor(quad_and_stray_edge(), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run_one_step(&mut e, "delete", "Delete");
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 2);
    assert_eq!(unused(m), 0);
}

#[test]
fn vertex_remove_keeps_lines_it_does_not_touch() {
    // Regression: Remove dropped every two-point line in the mesh.
    let mut e = editor(quad_and_stray_edge(), Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    command_exec::run(&mut e, "vert.remove", &Map::new()).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 2);
    assert!(m.is_polyline(1));
}

#[test]
fn symmetry_cleans_both_sides_in_one_step() {
    // Mirrored stray edges and lone triangles either side of X = 0.
    let mut m = Mesh::default();
    m.vertices = vec![
        DVec3::new(2., 0., 0.),
        DVec3::new(3., 0., 0.),
        DVec3::new(-2., 0., 0.),
        DVec3::new(-3., 0., 0.),
        DVec3::new(2., 2., 0.),
        DVec3::new(3., 2., 0.),
        DVec3::new(2., 3., 0.),
        DVec3::new(-2., 2., 0.),
        DVec3::new(-3., 2., 0.),
        DVec3::new(-2., 3., 0.),
    ];
    m.faces = vec![
        face(vec![0, 1]),
        face(vec![2, 3]),
        face(vec![4, 5, 6]),
        face(vec![7, 9, 8]),
    ]
    .into();
    m.attributes.representations = vec![
        line(false),
        line(false),
        PolygonRepresentation::Face,
        PolygonRepresentation::Face,
    ];
    let mut e = editor(m, Mode::Edge);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.scene.selection.edges = vec![(0, 1), (4, 5)];
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 4, "mirror partners selected");
    run_one_step(&mut e, "delete", "Delete");
    assert!(mesh(&e).vertices.is_empty());
    assert!(mesh(&e).faces.is_empty());
}
