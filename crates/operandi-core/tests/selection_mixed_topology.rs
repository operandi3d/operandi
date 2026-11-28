//! Selection commands on meshes that aren't all quads: the default Globe
//! sphere (24 sides, 12 segments, Y axis) has 240 quads and a fan of 24
//! triangles at each pole. Grow and Shrink work anywhere; Loop and Ring walk
//! the quads and stop at the triangles (provisional, SEL-Q-LOOP-01).
use operandi_core::*;

fn sphere() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 288);
    assert_eq!(
        mesh.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        48
    );
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

/// The quad just above the equator facing +X, and its neighbour around
/// the equator.
fn equator_quads(e: &Editor) -> (u32, u32) {
    let m = mesh(e);
    let row: Vec<usize> = (0..m.faces.len())
        .filter(|&f| {
            m.faces[f].vertices.len() == 4 && {
                let y = m.center(f).y;
                y > 0. && y < 0.1
            }
        })
        .collect();
    assert_eq!(row.len(), 24);
    let first = *row
        .iter()
        .max_by(|&&a, &&b| m.center(a).x.total_cmp(&m.center(b).x))
        .unwrap();
    let shares_edge = |a: usize, b: usize| {
        m.faces[a]
            .vertices
            .iter()
            .filter(|v| m.faces[b].vertices.contains(v))
            .count()
            == 2
    };
    let next = *row
        .iter()
        .find(|&&f| f != first && shares_edge(first, f))
        .unwrap();
    (first as u32, next as u32)
}

fn run(e: &mut Editor, op: SelectionOperation) {
    e.dispatch(Command::Selection(op)).unwrap();
}

#[test]
fn grow_and_shrink_work_on_a_sphere_with_triangle_poles() {
    let mut e = sphere();
    let (quad, _) = equator_quads(&e);
    e.scene.selection.faces = vec![quad];
    let sharing = {
        let mesh = &e.scene.active().unwrap().mesh;
        let own = mesh.faces[quad as usize].vertices.clone();
        mesh.faces
            .iter()
            .filter(|f| f.vertices.iter().any(|v| own.contains(v)))
            .count()
    };
    run(&mut e, SelectionOperation::Grow);
    // PF-01: the quad and every polygon sharing a point with it.
    assert_eq!(e.scene.selection.faces.len(), sharing);
    assert!(e.scene.selection.faces.contains(&quad));
    run(&mut e, SelectionOperation::Shrink);
    assert_eq!(e.scene.selection.faces, vec![quad]);

    // From a pole triangle: PF-01, every polygon sharing a point with it
    // (the whole pole fan and the quads on its base points).
    let m = mesh(&e);
    let triangle = (0..m.faces.len())
        .find(|&f| m.faces[f].vertices.len() == 3)
        .unwrap() as u32;
    let own = m.faces[triangle as usize].vertices.clone();
    let sharing = m
        .faces
        .iter()
        .filter(|f| f.vertices.iter().any(|v| own.contains(v)))
        .count();
    e.scene.selection.faces = vec![triangle];
    run(&mut e, SelectionOperation::Grow);
    assert_eq!(e.scene.selection.faces.len(), sharing);
    let m = mesh(&e);

    // Vertex mode: the pole point grows to its whole ring of 24 neighbours.
    let pole = (0..m.vertices.len() as u32)
        .max_by(|&a, &b| {
            m.vertices[a as usize]
                .y
                .total_cmp(&m.vertices[b as usize].y)
        })
        .unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![pole];
    run(&mut e, SelectionOperation::Grow);
    assert_eq!(e.scene.selection.vertices.len(), 25);

    // Edge mode: one equator edge grows by the six edges at its two ends.
    let m = mesh(&e);
    let edge = m
        .edges()
        .into_iter()
        .find(|&(a, b)| {
            m.vertices[a as usize].y.abs() < 1e-9 && m.vertices[b as usize].y.abs() < 1e-9
        })
        .unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge];
    run(&mut e, SelectionOperation::Grow);
    assert_eq!(e.scene.selection.edges.len(), 7);
}

#[test]
fn polygon_loop_goes_around_the_sphere() {
    let mut e = sphere();
    let (first, next) = equator_quads(&e);
    e.scene.selection.faces = vec![first, next];
    run(&mut e, SelectionOperation::Loop);
    // The whole band of quads around the equator.
    assert_eq!(e.scene.selection.faces.len(), 24);
    let m = mesh(&e);
    assert!(e.scene.selection.faces.iter().all(|&f| {
        let y = m.center(f as usize).y;
        m.faces[f as usize].vertices.len() == 4 && y > 0. && y < 0.1
    }));
    // One seed picks one of the two directions: around (24 quads) or
    // pole to pole (10 quads and the triangle at each end, where it stops).
    e.scene.selection.faces = vec![first];
    run(&mut e, SelectionOperation::Loop);
    let n = e.scene.selection.faces.len();
    assert!(n == 24 || n == 12, "{n}");
}

#[test]
fn edge_loop_and_ring_stop_at_the_poles() {
    let mut e = sphere();
    let m = mesh(&e);
    let on_equator = |v: u32| m.vertices[v as usize].y.abs() < 1e-9;
    let around = m
        .edges()
        .into_iter()
        .find(|&(a, b)| on_equator(a) && on_equator(b))
        .unwrap();
    let up = m
        .edges()
        .into_iter()
        .find(|&(a, b)| on_equator(a) != on_equator(b))
        .unwrap();
    e.scene.selection.mode = Mode::Edge;

    // Loop along the equator closes around the sphere.
    e.scene.selection.edges = vec![around];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.edges.len(), 24);
    // Loop along a meridian runs pole to pole and stops at the poles.
    e.scene.selection.edges = vec![up];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.edges.len(), 12);
    // Ring across the meridian edges goes around the sphere.
    e.scene.selection.edges = vec![up];
    run(&mut e, SelectionOperation::Ring);
    assert_eq!(e.scene.selection.edges.len(), 24);
    // Ring across the equator edge climbs the quad rows and stops where the
    // triangle fans begin: one edge on each of the 11 point rings.
    e.scene.selection.edges = vec![around];
    run(&mut e, SelectionOperation::Ring);
    assert_eq!(e.scene.selection.edges.len(), 11);
}

#[test]
fn edge_loop_that_cannot_continue_still_reports() {
    // On a plain cube every corner has three edges: nothing to add.
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let edge = mesh(&e).edges()[0];
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge];
    let before = e.scene.clone();
    let error = e
        .dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap_err();
    assert!(error.contains("can't continue straight"), "{error}");
    assert_eq!(e.scene, before);
}

#[test]
fn shrink_keeps_the_boundary_gate_open_when_exceptional_edges_balance_counts() {
    use glam::DVec3;
    use operandi_core::*;

    let vertices = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let tetra = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    for closed in [false, true] {
        let mut rings = tetra;
        if !closed {
            rings[3] = [0, 1, 2];
        }
        let mut mesh = Mesh {
            vertices: vertices.clone(),
            faces: rings
                .map(|v| Face {
                    vertices: v.into(),
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .to_vec()
                .into(),
            ..Default::default()
        };
        if !closed {
            mesh.attributes.extrude_edge_uses = [
                ("0:1".into(), vec![[0, 1, 0], [1, 0, 1], [3, 0, 1]]),
                ("0:2".into(), vec![[0, 0, 2], [2, 2, 0], [3, 2, 0]]),
            ]
            .into_iter()
            .collect();
        }
        mesh.validate().unwrap();
        let mut counts: Vec<_> = mesh.edge_faces().values().map(Vec::len).collect();
        counts.sort_unstable();
        assert_eq!(counts, if closed { [2; 6] } else { [1, 1, 2, 2, 3, 3] });
        assert_eq!(mesh.is_closed(), closed);
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = mesh.into();
        e.dispatch(Command::Mode(Mode::Edge)).unwrap();
        e.dispatch(Command::Selection(SelectionOperation::All))
            .unwrap();
        let shrink = e.dispatch(Command::Selection(SelectionOperation::Shrink));
        if closed {
            assert!(shrink.unwrap_err().contains("whole closed mesh"));
        } else {
            shrink.unwrap();
            assert_eq!(e.scene.selection.edges, [(0, 1), (0, 2), (0, 3), (1, 2)]);
        }
    }
}
