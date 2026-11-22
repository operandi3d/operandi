//! Diagonal Re-Direct against specifications/missing-tools-g1
//! (G1-SMALL-COMMANDS v1, MT4, cases DR-A..H). Shapes are compared as
//! positions.
#[path = "support/small_commands.rs"]
mod small_commands;

use glam::DVec3;
use operandi_core::diagonal_redirect::SELECT_TWO;
use operandi_core::*;
use small_commands::*;

fn redirect(m: Mesh, faces: &[u32]) -> Editor {
    let mut e = editor(m, Mode::Polygon);
    e.scene.selection.faces = faces.to_vec();
    e.dispatch(Command::DiagonalRedirect).unwrap();
    e
}

fn added(m: &Mesh, before: usize) -> Vec<DVec3> {
    m.vertices[before..].to_vec()
}

fn triangles(m: &Mesh) -> usize {
    m.faces.iter().filter(|f| f.vertices.len() == 3).count()
}

#[test]
fn dr_a_rebuilds_both_side_quads() {
    let source = grid(4);
    let mut e = redirect(source.clone(), &[5, 10]);
    let m = active(&e);
    let mut new = added(m, 25);
    new.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_eq!(
        new,
        vec![DVec3::new(1.5, 2.5, 0.), DVec3::new(2.5, 1.5, 0.)]
    );
    assert_eq!(m.faces.len(), 18);
    assert_eq!(triangles(m), 0);
    has_polygons(
        m,
        &[
            &[[1., 1.], [2., 1.], [2.5, 1.5], [2., 2.]],
            &[[3., 1.], [3., 2.], [2.5, 1.5], [2., 1.]],
            &[[2., 2.], [2.5, 1.5], [3., 2.], [3., 3.]],
            &[[1., 1.], [2., 2.], [1.5, 2.5], [1., 2.]],
            &[[1., 3.], [1., 2.], [1.5, 2.5], [2., 3.]],
            &[[2., 2.], [3., 3.], [2., 3.], [1.5, 2.5]],
        ],
    )
    .unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    assert_eq!(selected_positions(&e), vec![DVec3::new(2., 2., 0.)]);
    undo_redo_reopen(&mut e, &source, "Diagonal Re-Direct");
}

#[test]
fn dr_b_the_other_diagonal_is_the_mirror_image() {
    let e = redirect(grid(4), &[6, 9]);
    let m = active(&e);
    let mut new = added(m, 25);
    new.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_eq!(
        new,
        vec![DVec3::new(1.5, 1.5, 0.), DVec3::new(2.5, 2.5, 0.)]
    );
    assert_eq!(m.faces.len(), 18);
    assert_eq!(triangles(m), 0);
}

#[test]
fn dr_c_a_side_quad_on_the_border_is_left_with_its_triangles() {
    let e = redirect(grid(4), &[1, 6]);
    let m = active(&e);
    assert_eq!(added(m, 25), vec![DVec3::new(1.5, 1.5, 0.)]);
    assert_eq!(m.faces.len(), 18);
    has_polygons(
        m,
        &[
            &[[1., 0.], [2., 0.], [2., 1.]],
            &[[2., 1.], [3., 1.], [3., 2.]],
            &[[1., 0.], [2., 1.], [1.5, 1.5], [1., 1.]],
            &[[2., 1.], [3., 2.], [2., 2.], [1.5, 1.5]],
            &[[1., 2.], [1., 1.], [1.5, 1.5], [2., 2.]],
        ],
    )
    .unwrap();
    assert!(e.scene.selection.vertices.is_empty(), "MT4-07");
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
}

#[test]
fn dr_d_the_same_pattern_further_up() {
    let e = redirect(grid(4), &[6, 11]);
    let m = active(&e);
    assert_eq!(added(m, 25), vec![DVec3::new(2.5, 2.5, 0.)]);
    assert_eq!(m.faces.len(), 18);
    assert_eq!(triangles(m), 2);
}

#[test]
fn dr_e_both_side_quads_on_the_border_leave_four_triangles() {
    let e = redirect(grid(4), &[0, 5]);
    let m = active(&e);
    assert_eq!(m.vertices.len(), 25, "no new points");
    assert_eq!(m.faces.len(), 18);
    has_polygons(
        m,
        &[
            &[[0., 0.], [1., 0.], [1., 1.]],
            &[[0., 0.], [1., 1.], [0., 1.]],
            &[[1., 1.], [2., 1.], [2., 2.]],
            &[[1., 1.], [2., 2.], [1., 2.]],
        ],
    )
    .unwrap();
}

#[test]
fn dr_f_three_in_a_diagonal_row_treat_each_pair() {
    let e = redirect(grid(4), &[5, 10, 15]);
    let m = active(&e);
    assert_eq!(m.vertices.len(), 27);
    assert_eq!(m.faces.len(), 19);
    has_polygons(
        m,
        &[
            &[[1., 1.], [2., 1.], [2.5, 1.5], [2., 2.]],
            &[[2., 2.], [3., 3.], [2., 3.], [1.5, 2.5]],
            &[[3., 3.], [4., 3.], [4., 4.]],
            &[[3., 3.], [4., 4.], [3., 4.]],
        ],
    )
    .unwrap();
}

#[test]
fn dr_g_a_bigger_grid() {
    let e = redirect(grid(6), &[14, 21]);
    let m = active(&e);
    let mut new = added(m, 49);
    new.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_eq!(
        new,
        vec![DVec3::new(2.5, 3.5, 0.), DVec3::new(3.5, 2.5, 0.)]
    );
    assert_eq!(m.faces.len(), 38);
    assert_eq!(selected_positions(&e), vec![DVec3::new(3., 3., 0.)]);
}

fn refused(m: Mesh, mode: Mode, faces: &[u32]) {
    let mut e = editor(m.clone(), mode);
    e.scene.selection.faces = faces.to_vec();
    if mode == Mode::Edge {
        e.scene.selection.edges = vec![(6, 7)];
    }
    let steps = e.undo.len();
    assert_eq!(
        e.dispatch(Command::DiagonalRedirect).unwrap_err(),
        SELECT_TWO
    );
    assert_eq!(active(&e), &m);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn dr_h_anything_but_diagonal_quads_is_refused() {
    // Two triangles touching at a corner.
    let tris = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
            [2., 2., 0.],
        ],
        &[&[0, 1, 2], &[2, 3, 4]],
    );
    refused(tris, Mode::Polygon, &[0, 1]);
    // Two triangles sharing an edge.
    let pair = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    refused(pair, Mode::Polygon, &[0, 1]);
    // Two quads sharing an edge, an edge selected, nothing selected.
    refused(grid(4), Mode::Polygon, &[5, 6]);
    refused(grid(4), Mode::Edge, &[]);
    refused(grid(4), Mode::Polygon, &[]);
}

#[test]
fn q_dr_1_a_pole_at_the_corner_is_refused() {
    // G4 with quad 6 split in two: a side polygon that is not a quad.
    let mut m = grid(4);
    m.faces[6] = small_commands::face(vec![7, 8, 13]);
    m.faces.push(small_commands::face(vec![7, 13, 12]));
    refused(m, Mode::Polygon, &[5, 10]);
}
