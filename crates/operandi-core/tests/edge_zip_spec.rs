//! Edge Zip against specifications/missing-tools-g1 (G1-SMALL-COMMANDS
//! v1, MT3, cases EZ-A..F). Shapes are compared as positions.
#[path = "support/small_commands.rs"]
mod small_commands;

use operandi_core::edge_zip::NEEDS_QUADS;
use operandi_core::*;
use small_commands::*;

fn zip(m: Mesh, edges: &[(u32, u32)]) -> Editor {
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = edges.to_vec();
    e.dispatch(Command::EdgeZip).unwrap();
    e
}

/// Rows y..y+1 of two quads two wide: x 0..2 and 2..4 (EZ-A, EZ-D).
fn two_wide(m: &Mesh) {
    assert_eq!(m.vertices.len(), 15);
    assert_eq!(m.faces.len(), 8);
    for y in 0..4 {
        let (y0, y1) = (f64::from(y), f64::from(y + 1));
        has_polygons(
            m,
            &[
                &[[0., y0], [2., y0], [2., y1], [0., y1]],
                &[[2., y0], [4., y0], [4., y1], [2., y1]],
            ],
        )
        .unwrap();
    }
}

#[test]
fn ez_a_a_full_column_zips_away_its_neighbours() {
    let source = grid(4);
    let mut e = zip(source.clone(), &[(2, 7), (7, 12), (12, 17), (17, 22)]);
    two_wide(active(&e));
    assert_eq!(e.scene.selection.mode, Mode::Edge);
    assert!(e.scene.selection.edges.is_empty(), "MT3-06");
    undo_redo_reopen(&mut e, &source, "Edge Zip");
}

#[test]
fn ez_b_one_inner_edge_merges_the_end_triangles_back_into_quads() {
    let e = zip(grid(4), &[(7, 12)]);
    let m = active(&e);
    assert_eq!(m.vertices.len(), 21);
    assert_eq!(m.faces.len(), 12);
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4), "all quads");
    has_polygons(
        m,
        &[
            &[[0., 1.], [2., 1.], [2., 2.], [0., 2.]],
            &[[2., 1.], [4., 1.], [4., 2.], [2., 2.]],
            &[[0., 0.], [1., 0.], [2., 1.], [0., 1.]],
            &[[3., 0.], [4., 0.], [4., 1.], [2., 1.]],
            &[[2., 1.], [1., 0.], [2., 0.], [3., 0.]],
            &[[0., 2.], [2., 2.], [1., 3.], [0., 3.]],
            &[[2., 2.], [4., 2.], [4., 3.], [3., 3.]],
            &[[2., 2.], [3., 3.], [2., 3.], [1., 3.]],
            &[[0., 3.], [1., 3.], [1., 4.], [0., 4.]],
            &[[1., 3.], [2., 3.], [2., 4.], [1., 4.]],
            &[[2., 3.], [3., 3.], [3., 4.], [2., 4.]],
            &[[3., 3.], [4., 3.], [4., 4.], [3., 4.]],
        ],
    )
    .unwrap();
}

#[test]
fn ez_c_a_full_row_zips_into_quads_two_tall() {
    let e = zip(grid(4), &[(10, 11), (11, 12), (12, 13), (13, 14)]);
    let m = active(&e);
    assert_eq!(m.vertices.len(), 15);
    assert_eq!(m.faces.len(), 8);
    for x in 0..4 {
        let (x0, x1) = (f64::from(x), f64::from(x + 1));
        has_polygons(
            m,
            &[
                &[[x0, 0.], [x1, 0.], [x1, 2.], [x0, 2.]],
                &[[x0, 2.], [x1, 2.], [x1, 4.], [x0, 4.]],
            ],
        )
        .unwrap();
    }
}

#[test]
fn ez_d_uneven_columns_weld_at_the_average_of_the_group() {
    // Columns x = 0, 1, 1.5, 3.5, 4; the x = 1.5 column (i = 2) selected.
    let m = grid_x(&[0., 1., 1.5, 3.5, 4.], 4);
    let e = zip(m, &[(2, 7), (7, 12), (12, 17), (17, 22)]);
    two_wide(active(&e));
}

#[test]
fn ez_e_two_edges_one_quad_apart_chain_into_one_group_per_row() {
    let e = zip(grid(4), &[(6, 11), (8, 13)]);
    let m = active(&e);
    assert_eq!(m.faces.len(), 6);
    has_polygons(
        m,
        &[
            &[[2., 1.], [0., 0.], [1., 0.], [2., 0.], [3., 0.], [4., 0.]],
            &[[2., 2.], [4., 3.], [3., 3.], [2., 3.], [1., 3.], [0., 3.]],
            &[[0., 3.], [1., 3.], [1., 4.], [0., 4.]],
            &[[3., 3.], [4., 3.], [4., 4.], [3., 4.]],
        ],
    )
    .unwrap();
}

fn refused(m: Mesh, mode: Mode, edges: &[(u32, u32)]) {
    let mut e = editor(m.clone(), mode);
    e.scene.selection.edges = edges.to_vec();
    let steps = e.undo.len();
    let err = e.dispatch(Command::EdgeZip).unwrap_err();
    assert!(err.starts_with(NEEDS_QUADS), "{err}");
    assert_eq!(active(&e), &m);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn ez_f_an_open_border_edge_is_refused() {
    // Q-EZ-1 (provisional): refused instead of welded.
    refused(grid(4), Mode::Edge, &[(0, 1)]);
}

#[test]
fn edges_beside_a_triangle_or_nothing_selected_are_refused() {
    // Q-EZ-2: quad 5 of G4 split into two triangles.
    let mut m = grid(4);
    m.faces[5] = small_commands::face(vec![6, 7, 12]);
    m.faces.push(small_commands::face(vec![6, 12, 11]));
    refused(m, Mode::Edge, &[(7, 12)]);
    refused(grid(4), Mode::Edge, &[]);
    refused(grid(4), Mode::Polygon, &[(7, 12)]);
}
