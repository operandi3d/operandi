//! ME-C01..C13 Edge command rows: one focused geometry oracle per command,
//! plus editor history and refusals.
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
fn mesh(points: &[[f64; 3]], faces: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|&p| DVec3::from_array(p)).collect(),
        faces: faces.iter().map(|f| face(f.to_vec())).collect(),
        ..Default::default()
    }
}
/// Two quads side by side in z = 0, sharing edge 1–4.
fn strip() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
        ],
        &[&[0, 1, 4, 3], &[1, 2, 5, 4]],
    )
}
fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}
fn run(m: &Mesh, edges: &[(u32, u32)], c: EdgeCommand) -> (Mesh, Vec<(u32, u32)>) {
    let r = edge_command_geometry(m, edges, c).unwrap_or_else(|e| panic!("{c:?}: {e}"));
    r.0.validate().unwrap();
    r
}
fn top_edge(m: &Mesh) -> (u32, u32) {
    m.edges()
        .into_iter()
        .find(|&(a, b)| {
            let (p, q) = (m.vertices[a as usize], m.vertices[b as usize]);
            p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
        })
        .unwrap()
}

#[test]
fn subdivide_adds_a_middle_point_to_every_polygon_on_the_edge() {
    let c = cube();
    let e = top_edge(&c);
    let (out, sel) = run(&c, &[e], EdgeCommand::Subdivide);
    assert_eq!(out.vertices.len(), 9);
    let mid = out.vertices[8];
    assert!(mid.distance((c.vertices[e.0 as usize] + c.vertices[e.1 as usize]) * 0.5) < 1e-12);
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.contains(&8)).count(),
        2
    );
    assert_eq!(sel.len(), 2);
}

#[test]
fn collapse_heals_without_a_hole() {
    let c = cube();
    let (out, _) = run(&c, &[top_edge(&c)], EdgeCommand::Collapse);
    assert_eq!(out.vertices.len(), 7);
    assert_eq!(out.faces.len(), 6);
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        2
    );
}

#[test]
fn remove_merges_the_two_polygons_and_keeps_points() {
    let (out, _) = run(&strip(), &[(1, 4)], EdgeCommand::Remove);
    assert_eq!(out.vertices.len(), 6);
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 6);
    // CMD-E-02: a border edge's points are removed from their polygons.
    let (border, _) = edge_command_geometry(&strip(), &[(0, 1)], EdgeCommand::Remove).unwrap();
    assert_eq!(border.vertices.len(), 4);
    assert_eq!(border.faces.len(), 1);
    assert_eq!(border.faces[0].vertices.len(), 3);
}

#[test]
fn spin_turns_the_shared_diagonal() {
    let m = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let (out, sel) = run(&m, &[(0, 2)], EdgeCommand::Spin);
    assert_eq!(sel, vec![(1, 3)]);
    let edges = out.edges();
    assert!(edges.contains(&(1, 3)) && !edges.contains(&(0, 2)));
    assert!(out.faces.iter().all(|f| f.vertices.len() == 3));
    assert!((0..2).all(|f| out.normal(f).z > 0.));
}

#[test]
fn split_separates_polygons_along_the_edge() {
    let (out, _) = run(&strip(), &[(1, 4)], EdgeCommand::Split);
    assert_eq!(out.vertices.len(), 8);
    let a: std::collections::BTreeSet<u32> = out.faces[0].vertices.iter().copied().collect();
    let b: std::collections::BTreeSet<u32> = out.faces[1].vertices.iter().copied().collect();
    assert!(a.is_disjoint(&b));
}

#[test]
fn join_moves_the_first_edge_onto_the_second_or_averages() {
    // Two separate quads with a 0.2 gap between x = 1 and x = 1.2.
    let m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [1.2, 0., 0.],
            [2., 0., 0.],
            [2., 1., 0.],
            [1.2, 1., 0.],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    );
    let (out, sel) = run(&m, &[(1, 2), (4, 7)], EdgeCommand::Join);
    assert_eq!(out.vertices.len(), 6);
    assert_eq!(sel.len(), 1);
    let x = out.vertices[sel[0].0 as usize].x;
    assert!((x - 1.2).abs() < 1e-12);
    let (avg, sel) = run(&m, &[(1, 2), (4, 7)], EdgeCommand::JoinAveraged);
    assert!((avg.vertices[sel[0].0 as usize].x - 1.1).abs() < 1e-12);
    assert!(edge_command_geometry(&m, &[(1, 2)], EdgeCommand::Join).is_err());
}

#[test]
fn grow_quads_matches_the_preceding_row() {
    let m = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2, 3]],
    );
    let (out, sel) = run(&m, &[(1, 2)], EdgeCommand::GrowQuads);
    assert_eq!(out.faces.len(), 2);
    assert!(out.normal(1).z > 0.);
    let (a, b) = sel[0];
    for v in [a, b] {
        assert!((out.vertices[v as usize].x - 2.).abs() < 1e-12);
    }
    assert!(edge_command_geometry(&cube(), &[top_edge(&cube())], EdgeCommand::GrowQuads).is_err());
}

#[test]
fn fill_quads_closes_an_even_border_facing_out() {
    let mut c = cube();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap();
    let ring = c.faces.remove(top).vertices;
    let loop_edges: Vec<(u32, u32)> = (0..4).map(|i| (ring[i], ring[(i + 1) % 4])).collect();
    let (out, _) = run(&c, &loop_edges, EdgeCommand::FillQuads);
    assert_eq!(out.faces.len(), 6);
    assert!(out.normal(5).y > 0.9);
    // Odd borders are refused.
    let tri = mesh(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[&[0, 1, 2]]);
    assert!(
        edge_command_geometry(&tri, &[(0, 1), (1, 2), (2, 0)], EdgeCommand::FillQuads).is_err()
    );
}

/// A flat grid one cell wider than a `w`×`h` hole all round, the hole's
/// lower-left corner at (1, 1); its border loop as selected, starting at
/// that corner and running counter-clockwise (`ccw`) or clockwise.
fn holed_grid(w: u32, h: u32, ccw: bool) -> (Mesh, Vec<(u32, u32)>) {
    let (cols, rows) = (w + 3, h + 3);
    let id = |x: u32, y: u32| y * cols + x;
    let points: Vec<[f64; 3]> = (0..rows)
        .flat_map(|y| (0..cols).map(move |x| [x as f64, y as f64, 0.]))
        .collect();
    let faces: Vec<Vec<u32>> = (0..rows - 1)
        .flat_map(|y| (0..cols - 1).map(move |x| (x, y)))
        .filter(|&(x, y)| !(1..1 + w).contains(&x) || !(1..1 + h).contains(&y))
        .map(|(x, y)| vec![id(x, y), id(x + 1, y), id(x + 1, y + 1), id(x, y + 1)])
        .collect();
    let refs: Vec<&[u32]> = faces.iter().map(|f| &f[..]).collect();
    let mut ring: Vec<u32> = (1..=w).map(|x| id(x, 1)).collect();
    ring.extend((1..=h).map(|y| id(w + 1, y)));
    ring.extend((2..=w + 1).rev().map(|x| id(x, h + 1)));
    ring.extend((2..=h + 1).rev().map(|y| id(1, y)));
    if !ccw {
        ring[1..].reverse();
    }
    let n = ring.len();
    let edges = (0..n).map(|i| (ring[i], ring[(i + 1) % n])).collect();
    (mesh(&points, &refs), edges)
}

/// The new polygons as sorted corner positions (x, y), with their areas.
fn filled(w: u32, h: u32, ccw: bool) -> Vec<(Vec<(i64, i64)>, f64)> {
    let (m, edges) = holed_grid(w, h, ccw);
    let (out, _) = run(&m, &edges, EdgeCommand::FillQuads);
    let mut polys: Vec<_> = (m.faces.len()..out.faces.len())
        .map(|f| {
            assert!(out.normal(f).z > 0., "faces out");
            let v: Vec<DVec3> = out.faces[f]
                .vertices
                .iter()
                .map(|&i| out.vertices[i as usize])
                .collect();
            let area = (0..v.len())
                .map(|i| v[i].x * v[(i + 1) % v.len()].y - v[(i + 1) % v.len()].x * v[i].y)
                .sum::<f64>()
                / 2.;
            let mut c: Vec<(i64, i64)> = v.iter().map(|p| (p.x as i64, p.y as i64)).collect();
            c.sort();
            (c, area)
        })
        .collect();
    polys.sort_by(|a, b| a.0.cmp(&b.0));
    polys
}

/// FQ-Q01, measured with the border selected counter-clockwise from the
/// hole's lower-left corner: a 2×1 hole gets two squares split by the
/// middle rung; a 3×1 hole a square at the start end, then a diagonal from
/// the far end's start-side corner (4, 1) to the top of that rung (2, 2).
#[test]
fn fill_quads_two_and_three_by_one_holes() {
    let shapes = |w, ccw| -> Vec<Vec<(i64, i64)>> {
        filled(w, 1, ccw).into_iter().map(|(c, _)| c).collect()
    };
    assert_eq!(
        shapes(2, true),
        [
            vec![(1, 1), (1, 2), (2, 1), (2, 2)],
            vec![(2, 1), (2, 2), (3, 1), (3, 2)],
        ]
    );
    assert_eq!(
        shapes(3, true),
        [
            vec![(1, 1), (1, 2), (2, 1), (2, 2)],
            vec![(2, 1), (2, 2), (3, 1), (4, 1)],
            vec![(2, 2), (3, 2), (4, 1), (4, 2)],
        ]
    );
    // Selected the other way round (up the short side first; not
    // measured), the pieces still cover the hole once.
    let cw = filled(3, 1, false);
    assert!((cw.iter().map(|(_, a)| a).sum::<f64>() - 3.).abs() < 1e-9);
    assert!(cw.iter().all(|&(_, a)| a > 0.));
}

/// Other plain rectangles keep the ladder of squares; a 2×2 hole fans from
/// the middle of one side (UVN-T-07).
#[test]
fn fill_quads_other_rectangles_keep_their_squares() {
    for (w, h) in [(1, 1), (4, 1), (1, 4), (5, 1)] {
        let quads = filled(w, h, true);
        assert_eq!(quads.len() as u32, w * h, "{w}×{h}");
        assert!(quads.iter().all(|&(_, a)| (a - 1.).abs() < 1e-9), "{w}×{h}");
    }
    let fan = filled(2, 2, true);
    assert_eq!(fan.len(), 3);
    let shared = (3, 2);
    assert!(fan.iter().all(|(c, _)| c.contains(&shared)), "{fan:?}");
}

#[test]
fn edges_to_curves_makes_an_ordered_polyline() {
    let (out, sel) = run(&strip(), &[(0, 1), (1, 2)], EdgeCommand::ToCurves);
    assert_eq!(out.faces.len(), 3);
    assert!(out.is_polyline(2));
    assert_eq!(out.faces[2].vertices.len(), 3);
    let xs: Vec<f64> = out.faces[2]
        .vertices
        .iter()
        .map(|&v| out.vertices[v as usize].x)
        .collect();
    assert!(xs == vec![0., 1., 2.] || xs == vec![2., 1., 0.]);
    assert_eq!(sel.len(), 2);
}

#[test]
fn unbevel_restores_a_sharp_edge_from_a_bevel_ring() {
    let c = cube();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap() as u32;
    let r = bevel_geometry(&c, &[top], 0.2, 0.2, false, &BevelOptions::default(), None).unwrap();
    let m = r.mesh;
    let cap = &m.faces[r.selected[0] as usize].vertices;
    let mut loops: Vec<(u32, u32)> = (0..4).map(|i| (cap[i], cap[(i + 1) % 4])).collect();
    // The walls' top edges (the outer loop).
    for f in 0..m.faces.len() {
        if m.normal(f).y.abs() < 1e-9 {
            let vs = &m.faces[f].vertices;
            for i in 0..vs.len() {
                let (a, b) = (vs[i], vs[(i + 1) % vs.len()]);
                if m.vertices[a as usize].y > 0.4 && m.vertices[b as usize].y > 0.4 {
                    loops.push((a, b));
                }
            }
        }
    }
    let (out, sharp) = run(&m, &loops, EdgeCommand::Unbevel);
    assert_eq!(out.vertices.len(), 8);
    assert_eq!(out.faces.len(), 6);
    assert_eq!(sharp.len(), 4);
    // Converged corners: the cap's plane meets the walls' planes.
    for v in &out.vertices {
        assert!(
            (v.x.abs() - 0.5).abs() < 1e-9 && (v.z.abs() - 0.5).abs() < 1e-9,
            "{v}"
        );
        assert!((v.y - 0.7).abs() < 1e-9 || (v.y + 0.5).abs() < 1e-9, "{v}");
    }
}

#[test]
fn split_normals_hardens_the_edge_in_the_existing_map() {
    // Two quads folded 90° along edge 1–4.
    let mut m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 0., -1.],
            [0., 1., 0.],
            [1., 1., 0.],
            [1., 1., -1.],
        ],
        &[&[0, 1, 4, 3], &[1, 2, 5, 4]],
    );
    assert!(edge_command_geometry(&m, &[(1, 4)], EdgeCommand::SplitNormals).is_err());
    let soft = DVec3::new(1., 0., 1.).normalize();
    m.attributes
        .corner_normals
        .insert("vert_normals".into(), vec![vec![Some(soft); 4]; 2]);
    let (out, _) = run(&m, &[(1, 4)], EdgeCommand::SplitNormals);
    let rows = &out.attributes.corner_normals["vert_normals"];
    assert_eq!(rows[0][1], Some(m.normal(0)));
    assert_eq!(rows[1][0], Some(m.normal(1)));
    assert_eq!(rows[0][0], Some(soft));
    assert_eq!(out.vertices, m.vertices);
}

#[test]
fn edge_commands_through_the_editor_are_single_undo_steps() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = strip().into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(1, 4)];
    let before = e.scene.clone();
    e.dispatch(Command::EdgeCommand(EdgeCommand::Subdivide))
        .unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Edge Subdivide");
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 7);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    // Polygon mode and empty selections are refused without changes.
    e.scene.selection.mode = Mode::Polygon;
    assert!(
        e.dispatch(Command::EdgeCommand(EdgeCommand::Remove))
            .is_err()
    );
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges.clear();
    assert!(
        e.dispatch(Command::EdgeCommand(EdgeCommand::Remove))
            .is_err()
    );
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    // Set Position works on edge endpoints.
    e.scene.selection.edges = vec![(1, 4)];
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value: 0.5,
        expected: None,
    }))
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        (m.vertices[1].z, m.vertices[4].z, m.vertices[0].z),
        (0.5, 0.5, 0.)
    );
}

#[test]
fn bridge_joins_two_border_chains_and_two_loops() {
    // Two quads facing up, 1 apart in x: bridge their facing borders.
    let m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [3., 1., 0.],
            [2., 1., 0.],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    );
    let out = edge_bridge_geometry(&m, &[(1, 2), (4, 7)], 0, 0).unwrap().0;
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 3);
    assert!(out.normal(2).z > 0.9);
    // Segments counts rows; 0 and 1 both give one (bridge-2026-11-05).
    let one_row = edge_bridge_geometry(&m, &[(1, 2), (4, 7)], 1, 0).unwrap().0;
    assert_eq!(one_row, out);
    let two_rows = edge_bridge_geometry(&m, &[(1, 2), (4, 7)], 2, 0).unwrap().0;
    two_rows.validate().unwrap();
    assert_eq!((two_rows.faces.len(), two_rows.vertices.len()), (4, 10));
    // Open-top box and a lid one unit above it, facing up: bridge the loops
    // into a closed box twice as tall.
    let mut c = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap();
    let rim = c.faces.remove(top).vertices;
    let base = c.vertices.len() as u32;
    for &v in &rim {
        c.vertices.push(c.vertices[v as usize] + DVec3::Y);
    }
    c.faces.push(face((base..base + 4).collect()));
    c.validate().unwrap();
    assert!(c.normal(5).y > 0.9, "lid faces up");
    let mut loops: Vec<(u32, u32)> = (0..4).map(|i| (rim[i], rim[(i + 1) % 4])).collect();
    loops.extend((0..4).map(|i| (base + i, base + (i + 1) % 4)));
    let closed = edge_bridge_geometry(&c, &loops, 0, 0).unwrap().0;
    closed.validate().unwrap();
    assert_eq!(closed.faces.len(), 10);
    // Each wall joins a rim point straight up to the lid point above it.
    for f in 6..10 {
        assert!(closed.normal(f).y.abs() < 1e-9, "{}", closed.normal(f));
    }
    assert!(edge_bridge_geometry(&m, &[(1, 2)], 0, 0).is_err());
}
