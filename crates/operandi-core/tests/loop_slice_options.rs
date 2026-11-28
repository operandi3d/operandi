//! Loop Slice options (specifications/loop-slice-2026-11-15,
//! BEHAVIOR-LOOP-SLICE.md sections 4 and 7). Acceptance criteria:
//!
//! 1. On the measured fixture (3x3 grid of 1 m squares, the two bottom-left
//!    squares selected, ring = bottom row) Count 1 cuts one loop; Mode
//!    Symmetry with Count 2 at 30 % cuts at 30 % and 70 %; slices keep the
//!    order they were made in.
//! 2. Slice Selected cuts only the selected squares; Keep Quads changes
//!    nothing there, and splits a selected n-gon the ring ends in into
//!    quads (and only with Slice Selected on).
//! 3. Split doubles the slice's points with the same polygon count; Gap
//!    opens two loops centred on the slice; Cap Sections closes a closed
//!    ring's two sides and adds nothing on an open strip.
//! 4. A profile decides the loops (Count and Position set aside), scaled
//!    to the ring's width, pushed into the surface; Keep Aspect off lets
//!    Inset set the depth; Reverse Direction and Reverse Inset flip it.
//! 5. Undo/redo, cancel and save/reopen agree with the preview.
use glam::DVec3;
use operandi_core::*;

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

/// `n` x `n` unit quads in XZ (y = 0), point (i, j) at index j*(n+1)+i,
/// polygon (i, j) at index j*n+i.
fn grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(face(vec![a, a + n + 1, a + n + 2, a + 1]));
        }
    }
    mesh
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

/// The measured fixture with Loop Slice on: the two bottom-left squares
/// selected, so the ring is the bottom row (cut across Z).
fn fixture() -> Editor {
    let mut e = editor(grid(3), Mode::Polygon);
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    assert_eq!(params(&e).slice.loop_slice.rings, vec![(1, 5)]);
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn set(e: &mut Editor, change: impl FnOnce(&mut LoopSlice)) {
    let mut p = params(e);
    change(&mut p.slice.loop_slice);
    e.dispatch(Command::Preview(p)).unwrap();
}
fn counts(e: &Editor) -> (usize, usize) {
    (mesh(e).vertices.len(), mesh(e).faces.len())
}
/// The distinct z of the new points (index 16 on), rounded to 1e-6.
fn new_rows(e: &Editor) -> Vec<f64> {
    let mut z: Vec<f64> = mesh(e).vertices[16..]
        .iter()
        .map(|v| (v.z * 1e6).round() / 1e6)
        .collect();
    z.sort_by(f64::total_cmp);
    z.dedup();
    z
}

#[test]
fn count_one_cuts_one_loop_and_symmetry_places_30_and_70() {
    let mut e = fixture();
    assert_eq!(counts(&e), (20, 12));
    assert!(mesh(&e).faces.iter().all(|f| f.vertices.len() == 4));
    assert_eq!(new_rows(&e), vec![0.5]);
    set(&mut e, |o| {
        o.mode = LoopSliceMode::Symmetry;
        o.set_count(2);
        o.set_position(0, 0.3);
    });
    assert_eq!(new_rows(&e), vec![0.3, 0.7]);
}

#[test]
fn slices_keep_the_order_they_were_made_in() {
    let mut o = LoopSlice::default();
    o.add(0.8);
    o.add(0.2);
    assert_eq!(o.positions, vec![0.5, 0.8, 0.2]);
    assert_eq!(o.current, 2);
    // Moving one past another doesn't renumber them either.
    o.set_position(0, 0.9);
    assert_eq!(o.positions, vec![0.9, 0.8, 0.2]);
    assert_eq!(o.current, 0);
}

#[test]
fn slice_selected_cuts_the_two_squares_and_keep_quads_changes_nothing_here() {
    let mut e = fixture();
    set(&mut e, |o| o.slice_selected = true);
    assert_eq!(counts(&e), (19, 11));
    assert_eq!(mesh(&e).faces[2].vertices.len(), 5);
    let without = mesh(&e).clone();
    set(&mut e, |o| o.keep_quads = true);
    assert_eq!(*mesh(&e), without);
}

/// Two quads with a pentagon on top, along Z; the ring of edge (2, 3)
/// runs up the quads and stops at the pentagon (odd sides).
fn quads_and_pentagon() -> Mesh {
    let mut m = Mesh::default();
    for (x, z) in [
        (0., 0.),
        (1., 0.),
        (0., 1.),
        (1., 1.),
        (0., 2.),
        (1., 2.),
        (1.3, 2.6),
        (0.5, 3.),
        (-0.3, 2.6),
    ] {
        m.vertices.push(DVec3::new(x, 0., z));
    }
    m.faces.push(face(vec![0, 2, 3, 1]));
    m.faces.push(face(vec![2, 4, 5, 3]));
    m.faces.push(face(vec![5, 4, 8, 7, 6]));
    m
}

#[test]
fn keep_quads_splits_a_selected_ngon_the_ring_ends_in() {
    let run = |slice_selected: bool, keep_quads: bool| {
        let mut e = editor(quads_and_pentagon(), Mode::Polygon);
        e.scene.selection.faces = vec![0, 1, 2];
        e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
        set(&mut e, |o| {
            o.slice_selected = slice_selected;
            o.keep_quads = keep_quads;
        });
        let sides: Vec<usize> = mesh(&e).faces.iter().map(|f| f.vertices.len()).collect();
        e.dispatch(Command::Drop).unwrap();
        sides
    };
    // Without Keep Quads the pentagon takes the point: a hexagon.
    let plain = run(true, false);
    assert_eq!(plain.len(), 5);
    assert!(plain.contains(&6));
    // With it, the hexagon is cut into two quads.
    let quads = run(true, true);
    assert_eq!(quads.len(), 6);
    assert!(quads.iter().all(|&n| n == 4), "{quads:?}");
    // Keep Quads needs Slice Selected.
    assert_eq!(run(false, true), plain);
}

#[test]
fn split_doubles_the_points_and_gap_centres_two_loops() {
    let mut e = fixture();
    set(&mut e, |o| o.split = true);
    assert_eq!(counts(&e), (24, 12));
    assert_eq!(new_rows(&e), vec![0.5]);
    // The two sides no longer share an edge across the cut.
    let edges = mesh(&e).edge_counts();
    assert!(edges.values().all(|&n| n <= 2));
    let bottom = mesh(&e)
        .faces
        .iter()
        .filter(|f| {
            f.vertices
                .iter()
                .all(|&v| mesh(&e).vertices[v as usize].z <= 0.5)
        })
        .count();
    assert_eq!(bottom, 3);
    // Cap Sections: nothing closed to cap on an open strip.
    set(&mut e, |o| o.cap_sections = true);
    assert_eq!(counts(&e), (24, 12));
    // Gap 0.1: loops at 45 % and 55 %.
    set(&mut e, |o| o.gap = 0.1);
    assert_eq!(counts(&e), (24, 12));
    assert_eq!(new_rows(&e), vec![0.45, 0.55]);
    // Saves and reopens unchanged.
    e.dispatch(Command::Drop).unwrap();
    let path =
        std::env::temp_dir().join(format!("loop-slice-split-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(back.active().unwrap().mesh, e.scene.active().unwrap().mesh);
}

#[test]
fn split_with_caps_closes_both_sides_of_a_closed_ring() {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let mut e = editor(cube, Mode::Edge);
    // Any edge: its ring runs round four sides of the cube.
    let edge = mesh(&e).edges()[0];
    e.scene.selection.edges = vec![edge];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    assert_eq!(counts(&e), (12, 10));
    set(&mut e, |o| o.split = true);
    assert_eq!(counts(&e), (16, 10));
    set(&mut e, |o| o.cap_sections = true);
    assert_eq!(counts(&e), (16, 12));
    // Two closed shells, every edge used twice.
    assert!(mesh(&e).edge_counts().values().all(|&n| n == 2));
    // Undo inside the tool takes the caps back; redo restores them.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(counts(&e), (16, 10));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(counts(&e), (16, 12));
}

#[test]
fn a_profile_decides_the_loops_scaled_to_the_ring_and_pushed_in() {
    let mut e = fixture();
    set(&mut e, |o| o.profile.shape = Some(ProfileShape::TSlot));
    // Eight profile points: eight loops of four points, 3 x 8 new quads.
    assert_eq!(counts(&e), (48, 33));
    assert_eq!(params(&e).slice.loop_slice.count(), 1);
    // Scaled to the 1 m ring and centred: mouth 40-60 %, undercut 20-80 %.
    assert_eq!(new_rows(&e), vec![0.2, 0.4, 0.6, 0.8]);
    // Pushed into the surface, against the polygons' normal, by the
    // shape's own depths (Keep Aspect on: 0.15 and 0.4 of the width).
    let up = mesh(&e).normal(0).y.signum();
    let mut depths: Vec<f64> = mesh(&e).vertices[16..]
        .iter()
        .map(|v| (v.y * up * 1e6).round() / 1e6)
        .collect();
    depths.sort_by(f64::total_cmp);
    depths.dedup();
    assert_eq!(depths, vec![-0.4, -0.15, 0.]);
    // Position and Inset change nothing while Keep Aspect is on.
    let with_profile = mesh(&e).clone();
    set(&mut e, |o| {
        o.set_position(0, 0.3);
        o.inset = 0.25;
    });
    assert_eq!(*mesh(&e), with_profile);
    // Keep Aspect off: Inset is the deepest point.
    set(&mut e, |o| o.profile.keep_aspect = false);
    let deepest = mesh(&e).vertices[16..]
        .iter()
        .map(|v| v.y * up)
        .fold(0., f64::min);
    assert!((deepest + 0.25).abs() < 1e-9, "{deepest}");
    // Reverse Inset: a ridge instead.
    set(&mut e, |o| o.profile.reverse_inset = true);
    let highest = mesh(&e).vertices[16..]
        .iter()
        .map(|v| v.y * up)
        .fold(0., f64::max);
    assert!((highest - 0.25).abs() < 1e-9, "{highest}");
    // Cancel leaves the grid as it was.
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(counts(&e), (16, 9));
}

#[test]
fn reverse_direction_reads_the_profile_from_the_other_side() {
    let deep_at = |reverse: bool| {
        let mut e = fixture();
        set(&mut e, |o| {
            o.profile.shape = Some(ProfileShape::SawNotch);
            o.profile.reverse_direction = reverse;
        });
        let up = mesh(&e).normal(0).y.signum();
        let deepest = mesh(&e).vertices[16..]
            .iter()
            .min_by(|a, b| (a.y * up).total_cmp(&(b.y * up)))
            .copied()
            .unwrap();
        (deepest.z * 1e6).round() / 1e6
    };
    assert_eq!(deep_at(false), 0.7);
    assert_eq!(deep_at(true), 0.3);
}

#[test]
fn split_closes_where_slice_selected_stops_inside_the_mesh() {
    let mut e = fixture();
    set(&mut e, |o| {
        o.slice_selected = true;
        o.split = true;
    });
    // Two points on the open edge and on the edge between the squares,
    // one where the cut stops at the third square (it keeps one point).
    assert_eq!(counts(&e), (21, 11));
    assert!(mesh(&e).edge_counts().values().all(|&n| n <= 2));
}
