//! Loop, Ring and Close Loop on dense meshes (imported scans and sculpts run
//! to tens of thousands of polygons). There is no component limit: the walks
//! build their incidence once and then read only what they touch. A
//! non-manifold point elsewhere on the mesh doesn't stop a loop away from it.
use glam::DVec3;
use operandi_core::*;

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

/// A torus of `rings` × `sides` quads: every point has four edges and every
/// loop closes.
fn torus(rings: u32, sides: u32) -> Mesh {
    let mut vertices = Vec::new();
    for r in 0..rings {
        let u = r as f64 / rings as f64 * std::f64::consts::TAU;
        for s in 0..sides {
            let v = s as f64 / sides as f64 * std::f64::consts::TAU;
            let radius = 3. + v.cos();
            vertices.push(DVec3::new(radius * u.cos(), v.sin(), radius * u.sin()));
        }
    }
    let at = |r: u32, s: u32| (r % rings) * sides + s % sides;
    let faces = (0..rings)
        .flat_map(|r| (0..sides).map(move |s| (r, s)))
        .map(|(r, s)| face(vec![at(r, s), at(r + 1, s), at(r + 1, s + 1), at(r, s + 1)]))
        .collect();
    Mesh {
        vertices,
        faces,
        ..Default::default()
    }
}

/// An open grid of `columns` × `rows` quads; polygon `row * columns + column`.
fn grid(columns: u32, rows: u32) -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..=rows {
        for x in 0..=columns {
            vertices.push(DVec3::new(x as f64, 0., y as f64));
        }
    }
    let at = |x: u32, y: u32| y * (columns + 1) + x;
    let faces = (0..rows)
        .flat_map(|y| (0..columns).map(move |x| (x, y)))
        .map(|(x, y)| face(vec![at(x, y), at(x, y + 1), at(x + 1, y + 1), at(x + 1, y)]))
        .collect();
    Mesh {
        vertices,
        faces,
        ..Default::default()
    }
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = mode;
    e
}

fn run(e: &mut Editor, op: SelectionOperation) {
    e.dispatch(Command::Selection(op)).unwrap();
}

fn sorted(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

#[test]
fn edge_loop_and_ring_go_all_the_way_round_a_65536_quad_torus() {
    let (rings, sides) = (256, 256);
    let mut e = editor(torus(rings, sides), Mode::Edge);
    // An edge running around the tube: its loop is one ring of the torus.
    e.scene.selection.edges = vec![sorted(0, 1)];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.edges.len(), sides as usize);
    assert!(
        e.scene
            .selection
            .edges
            .iter()
            .all(|&(a, b)| a < sides && b < sides)
    );

    e.scene.selection.edges = vec![sorted(0, 1)];
    run(&mut e, SelectionOperation::Ring);
    assert_eq!(e.scene.selection.edges.len(), rings as usize);
}

#[test]
fn polygon_and_vertex_loops_work_on_a_dense_torus() {
    let (rings, sides) = (256, 256);
    let mut e = editor(torus(rings, sides), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.faces.len(), rings as usize);

    e.scene.selection.faces = vec![0, 1];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.faces.len(), sides as usize);
    assert_eq!(&e.scene.selection.faces[..2], &[0, 1]);

    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0, 1];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.vertices.len(), sides as usize);
}

#[test]
fn close_loop_fills_the_smaller_side_of_a_60000_quad_grid() {
    let (columns, rows) = (300, 200);
    let mut e = editor(grid(columns, rows), Mode::Polygon);
    let row = 50;
    e.scene.selection.faces = vec![row * columns + 10, row * columns + 11];
    run(&mut e, SelectionOperation::CloseLoop);
    // The row band plus every row below it.
    let expected = (columns * (row + 1)) as usize;
    assert_eq!(e.scene.selection.faces.len(), expected);
    assert!(e.scene.selection.faces.iter().all(|&f| f / columns <= row));
}

#[test]
fn a_non_manifold_point_elsewhere_does_not_block_the_loop() {
    let (columns, rows) = (64, 64);
    let mut mesh = grid(columns, rows);
    // A quad hanging off the grid's corner point 0 by that point alone: the
    // polygons around point 0 no longer join up (a "bow tie").
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend([
        DVec3::new(-1., 0., 0.),
        DVec3::new(-1., 0., -1.),
        DVec3::new(0., 0., -1.),
    ]);
    mesh.faces.push(face(vec![0, base, base + 1, base + 2]));
    let mut e = editor(mesh, Mode::Edge);

    // A loop through the middle of the grid, away from the bow tie.
    let middle = 32 * (columns + 1);
    e.scene.selection.edges = vec![sorted(middle, middle + 1)];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.edges.len(), columns as usize);

    // The grid's first column is open border: its loop runs round the
    // border (PF-03) and ends at the bow-tie point at both ends instead of
    // refusing or turning onto the extra quad.
    e.scene.selection.edges = vec![sorted(columns + 1, 2 * (columns + 1))];
    run(&mut e, SelectionOperation::Loop);
    assert_eq!(e.scene.selection.edges.len(), 2 * (rows + columns) as usize);
    assert!(
        e.scene
            .selection
            .edges
            .iter()
            .all(|&(a, b)| a < base && b < base)
    );
}
