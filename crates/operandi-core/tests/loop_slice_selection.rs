//! Loop Slice's Slice Selected follows the selection (spec section 8,
//! LS-72 and LS-75, measured 2026-11-20 on a flat 5x3 grid of 1 m
//! squares). Acceptance criteria:
//!
//! 1. Each connected group of selected polygons is cut on its own; an
//!    unselected polygon between two groups isn't cut and takes one point
//!    on each side it shares with a cut polygon. With Slice Selected off
//!    the whole ring is cut, gap included.
//! 2. An L turns its corner: the corner polygon is cut from the middle of
//!    its entry side to the middle of its exit side (a triangle and a
//!    five-sided rest); the ends run out to the border.
//! 3. A 2x2 block gets a closed loop round its centre: each polygon is cut
//!    corner to corner, nothing outside the block changes.
//! 4. A lone polygon is cut across its first side (point 0 to 1) and the
//!    opposite one; its neighbours there take one point each.
//! 5. Pick order (the ring the tool started with) doesn't matter.
//! 6. Two polygons meeting over a box's corner are both cut.
//! 7. Drop and undo agree with the preview.
use glam::DVec3;
use operandi_core::*;
use std::collections::BTreeMap;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// `nx` x `nz` 1 m squares in XZ (y = 0); point (i, j) at index
/// j*(nx+1)+i, polygon (i, j) at index j*nx+i.
fn grid(nx: u32, nz: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=nz {
        for i in 0..=nx {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..nz {
        for i in 0..nx {
            let a = j * (nx + 1) + i;
            mesh.faces
                .push(face(vec![a, a + nx + 1, a + nx + 2, a + 1]));
        }
    }
    mesh
}

/// Polygon (i, j) of the 5x3 grid.
fn at(i: u32, j: u32) -> u32 {
    j * 5 + i
}

fn editor(mesh: Mesh, selected: &[u32], slice_selected: bool) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.presets
        .get_mut::<operandi_core::slice::SlicePreset>()
        .loop_slice
        .slice_selected = slice_selected;
    e.scene.selection.faces = selected.to_vec();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

/// How many polygons have each number of sides.
fn sides(m: &Mesh) -> BTreeMap<usize, usize> {
    let mut out = BTreeMap::new();
    for f in &m.faces {
        *out.entry(f.vertices.len()).or_default() += 1;
    }
    out
}

fn has_point(m: &Mesh, x: f64, z: f64) -> bool {
    m.vertices
        .iter()
        .any(|v| v.distance(DVec3::new(x, 0., z)) < 1e-9)
}

fn ring_of(e: &Editor) -> Vec<(u32, u32)> {
    e.session
        .live
        .as_ref()
        .unwrap()
        .params
        .slice
        .loop_slice
        .rings
        .clone()
}

#[test]
fn groups_are_cut_on_their_own_and_the_gap_takes_a_point_each_side() {
    // Middle row: two pairs with the middle square unselected between.
    let pairs = [at(0, 1), at(1, 1), at(3, 1), at(4, 1)];
    let e = editor(grid(5, 3), &pairs, true);
    // 4 squares cut (+4); the gap square has a point on both sides.
    assert_eq!(sides(mesh(&e)), [(4, 18), (6, 1)].into());
    assert_eq!(mesh(&e).faces[at(2, 1) as usize].vertices.len(), 6);
    for x in [0., 1., 2., 3., 4., 5.] {
        assert!(has_point(mesh(&e), x, 1.5), "{x}");
    }
    // Select New Polygons: the cut polygons' pieces only.
    assert_eq!(e.scene.selection.faces.len(), 8);
    // Slice Selected off: the whole ring, gap included.
    let off = editor(grid(5, 3), &pairs, false);
    assert_eq!(sides(mesh(&off)), [(4, 20)].into());
}

#[test]
fn an_l_turns_its_corner() {
    // (0, 1) -> corner (1, 1) -> (1, 0).
    let l = [at(0, 1), at(1, 1), at(1, 0)];
    let e = editor(grid(5, 3), &l, true);
    // Ends: two quads each; corner: a triangle and a five-sided rest.
    assert_eq!(sides(mesh(&e)), [(3, 1), (4, 16), (5, 1)].into());
    for (x, z) in [(0., 1.5), (1., 1.5), (1.5, 1.), (1.5, 0.)] {
        assert!(has_point(mesh(&e), x, z), "({x}, {z})");
    }
    // Slice Selected off gives the same cut.
    let off = editor(grid(5, 3), &l, false);
    assert_eq!(sides(mesh(&off)), sides(mesh(&e)));
    // An L that ends inside the grid: on, the square beyond takes a
    // point; off, the cut runs on through it to the border.
    let inside = [at(1, 1), at(2, 1), at(2, 2)];
    let on = editor(grid(5, 3), &inside, true);
    assert_eq!(sides(mesh(&on)), [(3, 1), (4, 15), (5, 2)].into());
    let off = editor(grid(5, 3), &inside, false);
    assert_eq!(sides(mesh(&off)), [(3, 1), (4, 17), (5, 1)].into());
    assert!(has_point(mesh(&off), 0., 1.5));
}

#[test]
fn a_two_by_two_block_gets_a_closed_loop_round_its_centre() {
    let block = [at(1, 0), at(2, 0), at(1, 1), at(2, 1)];
    let e = editor(grid(5, 3), &block, true);
    // Four triangles at the centre, four five-sided rests, the other 11
    // squares unchanged.
    assert_eq!(sides(mesh(&e)), [(3, 4), (4, 11), (5, 4)].into());
    for (x, z) in [(2., 0.5), (2., 1.5), (1.5, 1.), (2.5, 1.)] {
        assert!(has_point(mesh(&e), x, z), "({x}, {z})");
    }
    assert_eq!(mesh(&e).vertices.len(), 24 + 4);
}

#[test]
fn a_lone_polygon_is_cut_across_its_first_side() {
    let e = editor(grid(5, 3), &[at(2, 1)], true);
    assert!(!ring_of(&e).is_empty(), "starts with a ring");
    // Side 0 -> 1 of square (2, 1) runs along Z at x = 2: the cut crosses
    // it and the side at x = 3, and the squares beyond take one point.
    assert!(has_point(mesh(&e), 2., 1.5) && has_point(mesh(&e), 3., 1.5));
    assert_eq!(sides(mesh(&e)), [(4, 14), (5, 2)].into());
}

#[test]
fn a_lone_polygon_with_slice_selected_off_cuts_round_its_ring() {
    // Forum report 2026-11-22: a lone selected polygon gave no cut with
    // Slice Selected off (the default). It names its first side's ring,
    // and the cut runs on round it (provisional, LS-Q19).
    let e = editor(grid(5, 3), &[at(2, 1)], false);
    assert!(!ring_of(&e).is_empty(), "starts with a ring");
    assert!((0..=5).all(|x| has_point(mesh(&e), x as f64, 1.5)));
    assert_eq!(sides(mesh(&e)), [(4, 20)].into());
}

#[test]
fn separate_lone_polygons_with_slice_selected_off_each_cut_their_ring() {
    let e = editor(grid(5, 3), &[at(0, 0), at(3, 2)], false);
    assert_eq!(ring_of(&e).len(), 2);
    for z in [0.5, 2.5] {
        assert!((0..=5).all(|x| has_point(mesh(&e), x as f64, z)), "row {z}");
    }
    assert_eq!(sides(mesh(&e)), [(4, 25)].into());
}

#[test]
fn pick_order_doesnt_matter() {
    let l = [at(0, 1), at(1, 1), at(1, 0)];
    let picked = editor(grid(5, 3), &l, true);
    let reversed = editor(grid(5, 3), &[at(1, 0), at(1, 1), at(0, 1)], true);
    assert_eq!(sides(mesh(&reversed)), sides(mesh(&picked)));
    // A ring clicked somewhere else changes nothing either.
    let mut clicked = editor(grid(5, 3), &l, true);
    let mut p = clicked.session.live.as_ref().unwrap().params.clone();
    p.slice.loop_slice.rings = vec![(16, 22)];
    clicked.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(sides(mesh(&clicked)), sides(mesh(&picked)));
}

#[test]
fn a_corner_pair_on_a_box_is_cut_on_both_sides_of_the_corner() {
    let boxed = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(4.), [4, 4, 4]).unwrap();
    let centre = |f: &Face| {
        f.vertices
            .iter()
            .map(|&i| boxed.vertices[i as usize])
            .sum::<DVec3>()
            / 4.
    };
    let pick = |c: DVec3| {
        boxed
            .faces
            .iter()
            .position(|f| centre(f).distance(c) < 1e-6)
            .unwrap() as u32
    };
    // The top polygon and the front one below it share the box's top front
    // edge.
    let pair = [
        pick(DVec3::new(0.5, 2., 1.5)),
        pick(DVec3::new(0.5, 1.5, 2.)),
    ];
    let e = editor(boxed.clone(), &pair, true);
    // Both cut (+2); the two polygons the cut ends against take a point.
    assert_eq!(sides(mesh(&e)), [(4, 96), (5, 2)].into());
}

#[test]
fn drop_and_undo_agree_with_the_preview() {
    let block = [at(1, 0), at(2, 0), at(1, 1), at(2, 1)];
    let mut e = editor(grid(5, 3), &block, true);
    let preview = mesh(&e).clone();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(*mesh(&e), preview);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), grid(5, 3));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(*mesh(&e), preview);
}

fn edge_editor(mesh: Mesh, edges: &[(u32, u32)], slice_selected: bool) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.presets
        .get_mut::<operandi_core::slice::SlicePreset>()
        .loop_slice
        .slice_selected = slice_selected;
    e.scene.selection.edges = edges.to_vec();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    e
}

/// GitHub #23 (provisional, LS-Q20): with edges selected, Slice Selected
/// cuts only through them instead of the whole ring.
#[test]
fn slice_selected_with_edges_cuts_only_through_them() {
    // The two sides of square (1, 1) the cut crosses: x = 1 and x = 2.
    let pair = [(7, 13), (8, 14)];
    let e = edge_editor(grid(5, 3), &pair, true);
    // Square (1, 1) is cut; its neighbours in the row take a point each.
    assert_eq!(sides(mesh(&e)), [(4, 14), (5, 2)].into());
    assert!(has_point(mesh(&e), 1., 1.5) && has_point(mesh(&e), 2., 1.5));
    assert!(!has_point(mesh(&e), 3., 1.5) && !has_point(mesh(&e), 0., 1.5));
    // Off: the whole row.
    let off = edge_editor(grid(5, 3), &pair, false);
    assert_eq!(sides(mesh(&off)), [(4, 20)].into());
    // A lone edge: just the squares either side of it.
    let lone = edge_editor(grid(5, 3), &[(7, 13)], true);
    assert_eq!(sides(mesh(&lone)), [(4, 16), (5, 1)].into());
    assert!(has_point(mesh(&lone), 0., 1.5) && !has_point(mesh(&lone), 3., 1.5));
}
