//! Fix Black Dots against specifications/missing-tools-g1
//! (G1-SMALL-COMMANDS v1, MT2, cases FB-A..G). Shapes are compared as
//! positions, not point order.
#[path = "support/small_commands.rs"]
mod small_commands;

use glam::DVec3;
use operandi_core::fix_black_dots::NO_BLACK_DOTS;
use operandi_core::*;
use small_commands::*;

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

fn fb_a() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [2., 0., 0.],
            [2., 2., 0.],
            [0., 2., 0.],
            [1., 1., 0.],
        ],
        &[&[0, 1, 2, 4], &[0, 4, 2, 3]],
    )
}

/// G4 with quads 5 and 6 replaced by two polygons pinched through an
/// extra point 25 at (2.2, 1.5, 0).
fn fb_b() -> Mesh {
    let mut m = grid(4);
    m.vertices.push(p(2.2, 1.5, 0.));
    m.faces[5] = face(vec![6, 7, 25, 12, 11]);
    m.faces[6] = face(vec![7, 8, 13, 12, 25]);
    m
}

/// A unit cube whose edge (1,1,0)–(0,1,0) has an extra point 8 used by
/// both faces beside it.
fn fb_c() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
            [0.5, 1., 0.],
        ],
        &[
            &[0, 1, 2, 8, 3],
            &[8, 2, 6, 7, 3],
            &[4, 7, 6, 5],
            &[1, 5, 6, 2],
            &[0, 3, 7, 4],
            &[0, 4, 5, 1],
        ],
    )
}

#[test]
fn fb_a_two_polygons_through_a_middle_point_become_one() {
    let source = fb_a();
    let mut e = editor(source.clone(), Mode::Polygon);
    e.dispatch(Command::FixBlackDots).unwrap();
    let m = active(&e);
    assert_eq!(m.faces.len(), 1);
    assert_eq!(m.vertices.len(), 5, "point 4 is kept (Q-FB-1)");
    assert_eq!(used_positions(m).len(), 4, "point 4 is unused");
    let square: Vec<DVec3> = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]]
        .iter()
        .map(|q| p(q[0], q[1], 0.))
        .collect();
    let got = &polygons(m)[0];
    assert!(
        (0..4).any(|r| (0..4).all(|i| got[(i + r) % 4] == square[i])),
        "{got:?}"
    );
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    assert_eq!(
        selected_positions(&e),
        vec![p(0., 0., 0.), p(1., 1., 0.), p(2., 2., 0.)]
    );
    undo_redo_reopen(&mut e, &source, "Fix Black Dots");
}

#[test]
fn fb_b_a_pinched_pair_in_a_grid_merges_and_the_selection_does_not_limit_it() {
    for select_first in [false, true] {
        let mut e = editor(fb_b(), Mode::Polygon);
        if select_first {
            e.scene.selection.faces = vec![0];
        }
        e.dispatch(Command::FixBlackDots).unwrap();
        let m = active(&e);
        assert_eq!(m.faces.len(), 15);
        has_polygons(
            m,
            &[&[[1., 2.], [1., 1.], [2., 1.], [3., 1.], [3., 2.], [2., 2.]]],
        )
        .unwrap();
        // The other 14 quads are unchanged.
        let quads = polygons(m).iter().filter(|f| f.len() == 4).count();
        assert_eq!(quads, 14);
        assert_eq!(m.vertices.len(), 26, "point 25 is kept, unused");
        assert_eq!(used_positions(m).len(), 25);
        assert_eq!(
            selected_positions(&e),
            vec![p(2., 1., 0.), p(2., 2., 0.), p(2.2, 1.5, 0.)]
        );
    }
}

#[test]
fn fb_c_a_bent_pair_on_a_cube_becomes_one_six_sided_polygon() {
    let mut e = editor(fb_c(), Mode::Polygon);
    e.dispatch(Command::FixBlackDots).unwrap();
    let m = active(&e);
    assert_eq!(m.faces.len(), 5);
    let want: Vec<DVec3> = [
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [1., 1., 1.],
        [0., 1., 1.],
        [0., 1., 0.],
    ]
    .iter()
    .map(|q| DVec3::from_array(*q))
    .collect();
    assert!(
        polygons(m)
            .iter()
            .any(|f| f.len() == 6 && (0..6).any(|r| (0..6).all(|i| f[(i + r) % 6] == want[i]))),
        "Q-FB-2: one non-flat six-sided polygon"
    );
    assert_eq!(m.vertices.len(), 9, "point 8 kept");
    assert_eq!(
        selected_positions(&e),
        vec![p(0., 1., 0.), p(0.5, 1., 0.), p(1., 1., 0.)]
    );
}

fn refused(m: Mesh) {
    let mut e = editor(m.clone(), Mode::Polygon);
    let steps = e.undo.len();
    assert_eq!(
        e.dispatch(Command::FixBlackDots).unwrap_err(),
        NO_BLACK_DOTS
    );
    assert_eq!(active(&e), &m, "no change");
    assert_eq!(e.undo.len(), steps, "no undo step (MT2-04)");
}

#[test]
fn fb_d_a_clean_grid_has_no_black_dots() {
    refused(grid(4));
}

#[test]
fn fb_e_a_triangle_fan_has_no_black_dots() {
    let mut pts = vec![[0., 0., 0.]];
    for k in 0..5 {
        let a = f64::from(k) * std::f64::consts::TAU / 5.;
        pts.push([a.cos(), a.sin(), 0.]);
    }
    let tris: Vec<Vec<u32>> = (0..5).map(|k| vec![0, 1 + k, 1 + (k + 1) % 5]).collect();
    let tris: Vec<&[u32]> = tris.iter().map(Vec::as_slice).collect();
    refused(mesh(&pts, &tris));
}

#[test]
fn fb_f_quads_touching_at_one_point_have_no_black_dots() {
    refused(mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 1., 0.],
            [2., 2., 0.],
            [1., 2., 0.],
        ],
        &[&[0, 1, 2, 3], &[2, 4, 5, 6]],
    ));
}

#[test]
fn fb_g_a_polygon_listing_a_point_twice_is_left_alone() {
    let m = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2, 2, 3]],
    );
    assert!(
        fix_black_dots::fix_black_dots_geometry(&m)
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_command_line_runs_it() {
    let mut e = editor(fb_a(), Mode::Item);
    command_line::run_line(&mut e, "operandi.mesh.fixBlackDots")
        .unwrap()
        .unwrap();
    assert_eq!(active(&e).faces.len(), 1);
}
