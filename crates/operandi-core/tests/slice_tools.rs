//! Tier 1A Slice tools (specifications/toolbox-panels-2026-11-06, EDIT.md
//! "Slice section", STARTING-VALUES-1A-1B.md 1A). Acceptance criteria:
//!
//! 1. Loop Slice cuts Count loops across the ring of a selected edge (or of
//!    two neighbouring selected polygons) at the slices' Positions, with
//!    Free/Uniform/Symmetry placement; no ring changes nothing.
//! 2. Slice Selected keeps the cut inside the selected polygons; Slice N-gon
//!    continues through even n-gons; Preserve Curvature follows the surface.
//! 3. Edge Slice connects points placed edge to edge (and through inside
//!    points), closes on its first point, adds points only with Split
//!    Polygons off, and refuses a point that isn't next to the last one.
//! 4. Polygon Slice cuts the selected polygons (all when none) with a plane
//!    through the line, perpendicular to the work plane; Infinite off keeps
//!    to the line's extent; Split/Cap/Gap separate and close the pieces;
//!    Fast Slice waits for the release.
//! 5. Each tweak is its own undo step inside the tool; Drop makes one step
//!    named after the tool; Cancel leaves the scene as it was.
//! 6. The result saves and reopens unchanged.
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

/// A closed unit cube centred at the origin.
fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}
fn params(e: &Editor) -> Parameters {
    e.live.as_ref().unwrap().params.clone()
}
fn preview(e: &mut Editor, p: Parameters) {
    e.dispatch(Command::Preview(p)).unwrap();
}
/// One held gesture: every preview joins one tweak.
fn gesture(e: &mut Editor, p: Parameters) {
    e.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
}
fn has_point(m: &Mesh, p: DVec3) -> bool {
    m.vertices.iter().any(|v| v.distance(p) < 1e-9)
}

#[test]
fn starting_values_match_the_spec() {
    let p = SliceParameters::default();
    let l = &p.loop_slice;
    assert_eq!(
        (l.edit, l.mode, l.current, l.positions.clone()),
        (LoopSliceEdit::Move, LoopSliceMode::Free, 0, vec![0.5])
    );
    assert_eq!((l.length, l.slider_x, l.slider_y), (200., 20., 50.));
    assert!(!l.slice_selected && !l.keep_quads && !l.slice_ngon && !l.preserve_curvature);
    assert!(l.select_new && l.profile.keep_aspect && l.tension == 1.);
    assert!(!l.split && !l.cap_sections && l.gap == 0. && l.profile.shape.is_none());
    let s = &p.edge_slice;
    assert!(!s.split_middle && s.split_polygons && s.inside && s.always_raycast && !s.multi_slice);
    assert_eq!((s.show, s.snap, s.offset), (SliceShow::Position, 0.005, 0.));
    let c = &p.polygon_slice;
    assert_eq!(
        (c.start, c.end),
        (DVec3::new(-1., 0., 0.), DVec3::new(1., 0., 0.))
    );
    assert!(c.angle_snap && c.angle == 45. && !c.fast_slice && !c.split && !c.cap_sections);
    assert!(!c.infinite && c.gap == 0. && c.offset_side == OffsetSide::Center);
}

#[test]
fn loop_slice_cuts_the_selected_edges_ring_and_drop_is_one_step() {
    // A 3x3 grid; edge (0,0)-(1,0) runs along X, its ring crosses the
    // column of polygons i = 0 along Z.
    let mut e = editor(grid(3), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    let before = e.scene.clone();
    let steps = e.undo.len();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    assert_eq!(params(&e).slice.loop_slice.rings, vec![(0, 1)]);
    // The slice shows at once, at 50 %.
    assert_eq!(mesh(&e).faces.len(), 12);
    for j in 0..=3 {
        assert!(has_point(mesh(&e), DVec3::new(0.5, 0., j as f64)));
    }
    // Select New Polygon (Edge mode): the new loop's edges.
    assert_eq!(e.scene.selection.edges.len(), 3);
    // Three slices, Uniform.
    let mut p = params(&e);
    p.slice.loop_slice.mode = LoopSliceMode::Uniform;
    p.slice.loop_slice.set_count(3);
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 9 + 3 * 3);
    for x in [0.25, 0.5, 0.75] {
        assert!(has_point(mesh(&e), DVec3::new(x, 0., 2.)));
    }
    // Mid-tool undo takes back the Count change and keeps the tool.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 12);
    assert_eq!(e.live.as_ref().map(|l| l.tool), Some(Tool::LoopSlice));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Loop Slice");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
}

#[test]
fn loop_slice_from_two_neighbouring_polygons_and_symmetry() {
    let mut e = editor(grid(3), Mode::Polygon);
    // Polygons (0,0) and (0,1) share the edge from (0,1) to (1,1).
    e.scene.selection.faces = vec![0, 3];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    assert_eq!(params(&e).slice.loop_slice.rings, vec![(4, 5)]);
    let mut p = params(&e);
    p.slice.loop_slice.mode = LoopSliceMode::Symmetry;
    p.slice.loop_slice.set_count(2);
    p.slice.loop_slice.set_position(0, 0.2);
    preview(&mut e, p);
    assert!(has_point(mesh(&e), DVec3::new(0.2, 0., 1.)));
    assert!(has_point(mesh(&e), DVec3::new(0.8, 0., 1.)));
    // Select New Polygon (Polygon mode): the cut polygons' pieces.
    assert_eq!(e.scene.selection.faces.len(), 9);
}

#[test]
fn loop_slice_without_a_ring_changes_nothing_and_cancel_restores() {
    let mut e = editor(grid(2), Mode::Edge);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let p = params(&e);
    preview(&mut e, p);
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    // A clicked edge (the viewport sets the ring) cuts; Cancel undoes it.
    let mut p = params(&e);
    p.slice.loop_slice.rings = vec![(0, 1)];
    preview(&mut e, p);
    assert_ne!(mesh(&e).faces.len(), 4);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.live.is_none());
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
}

#[test]
fn slice_selected_keeps_inside_the_selection_and_ngons_continue_when_asked() {
    let mut e = editor(grid(3), Mode::Polygon);
    e.scene.selection.faces = vec![0, 3];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let mut p = params(&e);
    p.slice.loop_slice.slice_selected = true;
    preview(&mut e, p);
    // Only polygons (0,0) and (0,1) are cut; (0,2) takes a point only.
    assert_eq!(mesh(&e).faces.len(), 11);
    assert_eq!(mesh(&e).faces[6].vertices.len(), 5);
    e.dispatch(Command::Cancel).unwrap();

    // A hexagon between two quads: off ends the ring there, on runs
    // through it to the opposite side.
    let mut m = Mesh::default();
    for p in [
        [0., 0.],
        [1., 0.],
        [0., 1.],
        [1., 1.],
        [-0.5, 1.5],
        [1.5, 1.5],
        [0., 2.],
        [1., 2.],
        [0., 3.],
        [1., 3.],
    ] {
        m.vertices.push(DVec3::new(p[0], 0., p[1]));
    }
    m.faces.push(face(vec![0, 2, 3, 1]));
    m.faces.push(face(vec![2, 4, 6, 7, 5, 3]));
    m.faces.push(face(vec![6, 8, 9, 7]));
    for ngon in [false, true] {
        let mut e = editor(m.clone(), Mode::Edge);
        e.scene.selection.edges = vec![(0, 1)];
        e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
        let mut p = params(&e);
        p.slice.loop_slice.slice_ngon = ngon;
        preview(&mut e, p);
        assert_eq!(mesh(&e).faces.len(), if ngon { 6 } else { 4 });
        assert_eq!(has_point(mesh(&e), DVec3::new(0.5, 0., 3.)), ngon);
    }
}

#[test]
fn preserve_curvature_follows_a_bent_ring() {
    // A strip bent up at both ends. The middle edge (1,0)-(2,0) runs along
    // X; its neighbours turn up, so with Preserve Curvature the slice point
    // dips below the straight edge, and without it sits on it.
    let mut m = Mesh::default();
    for (x, y) in [(0., 1.), (1., 0.), (2., 0.), (3., 1.)] {
        m.vertices.push(DVec3::new(x, y, 0.));
        m.vertices.push(DVec3::new(x, y, 1.));
    }
    for i in 0..3u32 {
        m.faces
            .push(face(vec![2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2]));
    }
    for curved in [false, true] {
        let mut e = editor(m.clone(), Mode::Edge);
        e.scene.selection.edges = vec![(2, 4)];
        e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
        let mut p = params(&e);
        p.slice.loop_slice.preserve_curvature = curved;
        preview(&mut e, p);
        let new = &mesh(&e).vertices[8..];
        assert_eq!(new.len(), 2);
        for v in new {
            assert!((v.x - 1.5).abs() < 1e-9);
            assert_eq!(v.y < -1e-3, curved, "{v:?}");
        }
    }
}

#[test]
fn edge_slice_cuts_from_edge_to_edge_and_closes() {
    let mut e = editor(grid(2), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::EdgeSlice)).unwrap();
    // Across polygon (0,0): bottom edge (0,0)-(1,0) to top edge (0,1)-(1,1)
    // then on through polygon (0,1) to its top edge.
    let mut p = params(&e);
    p.slice.edge_slice.points = vec![
        SlicePoint::Edge { a: 0, b: 1, t: 0.5 },
        SlicePoint::Edge { a: 3, b: 4, t: 0.5 },
    ];
    gesture(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 5);
    p.slice
        .edge_slice
        .points
        .push(SlicePoint::Edge { a: 6, b: 7, t: 0.5 });
    gesture(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 6);
    // A point not next to the last one is refused; the scene stays.
    let shown = e.scene.clone();
    let mut bad = p.clone();
    bad.slice
        .edge_slice
        .points
        .push(SlicePoint::Edge { a: 1, b: 2, t: 0.5 });
    assert!(e.dispatch(Command::Preview(bad)).is_err());
    assert_eq!(e.scene, shown);
    // Split Polygons off: points only.
    let mut off = p.clone();
    off.slice.edge_slice.split_polygons = false;
    preview(&mut e, off);
    assert_eq!(mesh(&e).faces.len(), 4);
    assert_eq!(mesh(&e).vertices.len(), 9 + 3);
    e.dispatch(Command::Cancel).unwrap();

    // A closed loop around the middle point of a 2x2 grid: four points on
    // the four inner edges, closed on the first.
    let mut e = editor(grid(2), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::EdgeSlice)).unwrap();
    let mut p = params(&e);
    p.slice.edge_slice.points = vec![
        SlicePoint::Edge { a: 1, b: 4, t: 0.5 },
        SlicePoint::Edge { a: 4, b: 5, t: 0.5 },
        SlicePoint::Edge { a: 4, b: 7, t: 0.5 },
        SlicePoint::Edge { a: 3, b: 4, t: 0.5 },
    ];
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 7);
    p.slice.edge_slice.closed = true;
    preview(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 8);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Edge Slice");
}

#[test]
fn edge_slice_inside_points_cut_into_a_polygon() {
    let mut e = editor(grid(1), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::EdgeSlice)).unwrap();
    let mut p = params(&e);
    p.slice.edge_slice.points = vec![
        SlicePoint::Edge { a: 0, b: 1, t: 0.5 },
        SlicePoint::Inside {
            face: 0,
            at: DVec3::new(0.3, 0., 0.5),
        },
    ];
    // Until the path leaves the polygon, the inside point changes nothing.
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 1);
    p.slice
        .edge_slice
        .points
        .push(SlicePoint::Edge { a: 2, b: 3, t: 0.5 });
    preview(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 2);
    assert!(has_point(mesh(&e), DVec3::new(0.3, 0., 0.5)));
}

fn cut_at_z(z: f64, from: f64, to: f64) -> PlaneCut {
    PlaneCut {
        start: DVec3::new(from, 0., z),
        end: DVec3::new(to, 0., z),
        up: DVec3::Y,
    }
}

#[test]
fn polygon_slice_cuts_with_a_plane_and_infinite_off_keeps_to_the_line() {
    let mut e = editor(grid(4), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PolygonSlice)).unwrap();
    let mut p = params(&e);
    // The line only covers x = 0..2 at z = 1.5: two polygons of the row.
    p.slice.polygon_slice.cuts = vec![cut_at_z(1.5, 0.2, 1.8)];
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 16 + 2);
    p.slice.polygon_slice.infinite = true;
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 16 + 4);
    for x in 0..=4 {
        assert!(has_point(mesh(&e), DVec3::new(x as f64, 0., 1.5)));
    }
    e.dispatch(Command::Cancel).unwrap();
    // Selected polygons only.
    let mut e = editor(grid(4), Mode::Polygon);
    e.scene.selection.faces = vec![4, 5];
    e.dispatch(Command::Activate(Tool::PolygonSlice)).unwrap();
    let mut p = params(&e);
    p.slice.polygon_slice.cuts = vec![cut_at_z(1.5, -1., 5.)];
    p.slice.polygon_slice.infinite = true;
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 18);
    // A second (Shift) cut through the pieces of the first.
    p.slice.polygon_slice.cuts.push(PlaneCut {
        start: DVec3::new(0.5, 0., -1.),
        end: DVec3::new(0.5, 0., 5.),
        up: DVec3::Y,
    });
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 20);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Polygon Slice");
}

#[test]
fn polygon_slice_split_cap_and_gap() {
    let mut e = editor(cube(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PolygonSlice)).unwrap();
    let mut p = params(&e);
    p.slice.polygon_slice.cuts = vec![cut_at_z(0., -2., 2.)];
    p.slice.polygon_slice.infinite = true;
    preview(&mut e, p.clone());
    // Four side polygons crossed: 6 + 4 polygons, 8 + 4 points.
    assert_eq!((mesh(&e).faces.len(), mesh(&e).vertices.len()), (10, 12));
    p.slice.polygon_slice.split = true;
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).vertices.len(), 16);
    assert!(!mesh(&e).is_closed());
    p.slice.polygon_slice.cap_sections = true;
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 12);
    assert!(mesh(&e).is_closed());
    // Gap 0.2 centred: each side's cut moves 0.1 away from the plane.
    p.slice.polygon_slice.gap = 0.2;
    preview(&mut e, p.clone());
    let zs: Vec<f64> = mesh(&e).vertices[8..].iter().map(|v| v.z).collect();
    assert!(zs.iter().all(|z| (z.abs() - 0.1).abs() < 1e-9), "{zs:?}");
    p.slice.polygon_slice.offset_side = OffsetSide::Above;
    preview(&mut e, p);
    let zs: Vec<f64> = mesh(&e).vertices[8..].iter().map(|v| v.z).collect();
    assert!(
        zs.iter().all(|z| z.abs() < 1e-9 || (z - 0.2).abs() < 1e-9),
        "{zs:?}"
    );
}

#[test]
fn fast_slice_waits_for_the_release() {
    let mut e = editor(grid(2), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PolygonSlice)).unwrap();
    let mut p = params(&e);
    p.slice.polygon_slice.fast_slice = true;
    p.slice.polygon_slice.infinite = true;
    p.slice.polygon_slice.cuts = vec![cut_at_z(0.5, -1., 3.)];
    p.slice.polygon_slice.held = true;
    preview(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 4);
    p.slice.polygon_slice.held = false;
    preview(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 6);
}

#[test]
fn options_carry_to_the_next_session_but_cuts_do_not() {
    let mut e = editor(grid(2), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let mut p = params(&e);
    p.slice.loop_slice.set_count(2);
    p.slice.loop_slice.slice_ngon = true;
    preview(&mut e, p);
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.edges.clear();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let o = params(&e).slice.loop_slice;
    assert_eq!((o.count(), o.slice_ngon, o.rings.len()), (2, true, 0));
}

#[test]
fn sliced_mesh_saves_and_reopens() {
    let mut e = editor(grid(2), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let p = params(&e);
    preview(&mut e, p);
    e.dispatch(Command::Drop).unwrap();
    let dir = std::env::temp_dir().join(format!("operandi-slice-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sliced.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    assert_eq!(back.active().unwrap().mesh, e.scene.active().unwrap().mesh);
    let _ = std::fs::remove_dir_all(&dir);
}
