//! Select tab tools (modelling parity batch 4) against the probe cases
//! (SELECTION-TOOLS SL-xx, selection-cases.json). Meshes are rebuilt here
//! from the cases' points and polygons.
use glam::DVec3;
use operandi_core::select_tools::*;
use operandi_core::*;
use std::collections::BTreeSet;

fn mesh(points: &[[f64; 3]], polygons: &[Vec<u32>]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|p| DVec3::from_array(*p)).collect(),
        faces: polygons
            .iter()
            .map(|f| Face {
                vertices: f.clone(),
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect::<Vec<_>>()
            .into(),
        ..Default::default()
    }
}

/// The probes' 5 x 5 quad grid: points row by row (6 per row), polygon
/// `i` = row-major quad.
fn grid() -> Mesh {
    let mut p = Vec::new();
    for y in 0..6 {
        for x in 0..6 {
            p.push([x as f64, y as f64, 0.]);
        }
    }
    let mut f = Vec::new();
    for y in 0..5u32 {
        for x in 0..5u32 {
            let i = y * 6 + x;
            f.push(vec![i, i + 1, i + 7, i + 6]);
        }
    }
    mesh(&p, &f)
}

fn editor(m: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.mode = mode;
    e.undo.clear();
    e
}

fn run(e: &mut Editor, r: SelectRequest) {
    e.dispatch(Command::SelectTool(r)).unwrap();
}

fn verts(e: &Editor) -> BTreeSet<u32> {
    e.scene.selection.vertices.iter().copied().collect()
}
fn edges(e: &Editor) -> BTreeSet<(u32, u32)> {
    e.scene
        .selection
        .edges
        .iter()
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect()
}
fn faces(e: &Editor) -> BTreeSet<u32> {
    e.scene.selection.faces.iter().copied().collect()
}

// ---- Shortest Path (SL-01..02) ----

#[test]
fn sl_sp1_vertices_go_along_the_row_then_up_the_column() {
    let mut e = editor(grid(), Mode::Vertex);
    e.scene.selection.vertices = vec![0, 35];
    run(&mut e, SelectRequest::ShortestPath);
    // The reference's exact route (SL-02).
    assert_eq!(
        verts(&e),
        [0, 1, 2, 3, 4, 5, 11, 17, 23, 29, 35].into_iter().collect()
    );
    // One undo step that changes only the selection.
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.undo.last().unwrap().label, "Shortest Path");
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(verts(&e), [0, 35].into_iter().collect());
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(verts(&e).len(), 11);
}

#[test]
fn sl_sp2_edges() {
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1), (34, 35)];
    run(&mut e, SelectRequest::ShortestPath);
    let want: BTreeSet<(u32, u32)> = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 10),
        (10, 16),
        (16, 22),
        (22, 28),
        (28, 34),
        (34, 35),
    ]
    .into_iter()
    .collect();
    assert_eq!(edges(&e), want);
}

#[test]
fn sl_sp3_polygons() {
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![0, 24];
    run(&mut e, SelectRequest::ShortestPath);
    assert_eq!(
        faces(&e),
        [0, 1, 2, 3, 4, 9, 14, 19, 24].into_iter().collect()
    );
}

#[test]
fn sl_sp4_same_row() {
    let mut e = editor(grid(), Mode::Vertex);
    e.scene.selection.vertices = vec![0, 5];
    run(&mut e, SelectRequest::ShortestPath);
    assert_eq!(verts(&e), (0..6).collect());
}

#[test]
fn shortest_path_refuses_one_element_or_separate_pieces() {
    let mut e = editor(grid(), Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    assert!(
        e.dispatch(Command::SelectTool(SelectRequest::ShortestPath))
            .is_err()
    );
    // Two separate quads.
    let two = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [5., 0., 0.],
            [6., 0., 0.],
            [6., 1., 0.],
            [5., 1., 0.],
        ],
        &[vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
    );
    let mut e = editor(two, Mode::Vertex);
    e.scene.selection.vertices = vec![0, 6];
    assert!(
        e.dispatch(Command::SelectTool(SelectRequest::ShortestPath))
            .is_err()
    );
    assert_eq!(verts(&e), [0, 6].into_iter().collect());
    assert!(e.undo.is_empty());
}

// ---- Sharp (SL-10..12) ----

const STRIP: [[f64; 3]; 12] = [
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 1.0],
    [1.939693, 0.34202, 0.0],
    [1.939693, 0.34202, 1.0],
    [2.362311, 1.248328, 0.0],
    [2.362311, 1.248328, 1.0],
    [1.456003, 1.670946, 0.0],
    [1.456003, 1.670946, 1.0],
    [1.798023, 0.731254, 0.0],
    [1.798023, 0.731254, 1.0],
];
fn strip() -> Mesh {
    mesh(
        &STRIP,
        &[
            vec![0, 2, 3, 1],
            vec![2, 4, 5, 3],
            vec![4, 6, 7, 5],
            vec![6, 8, 9, 7],
            vec![8, 10, 11, 9],
        ],
    )
}

#[test]
fn sl_sh1_and_sh2_sharp_edges_by_normal_angle() {
    let mut e = editor(strip(), Mode::Edge);
    run(
        &mut e,
        SelectRequest::Sharp {
            min: SHARP_MIN,
            max: SHARP_MAX,
        },
    );
    assert_eq!(edges(&e), [(6, 7), (8, 9)].into_iter().collect());
    let mut e = editor(strip(), Mode::Edge);
    run(
        &mut e,
        SelectRequest::Sharp {
            min: 30.,
            max: 100.,
        },
    );
    assert_eq!(edges(&e), [(4, 5), (6, 7)].into_iter().collect());
    // SH3: Min above Max is refused.
    assert!(
        e.dispatch(Command::SelectTool(SelectRequest::Sharp {
            min: 100.,
            max: 30.
        }))
        .is_err()
    );
}

// ---- Colinear Vertices (SL-30) ----

fn outline() -> Mesh {
    mesh(
        &[
            [0.0, 0.0, 0.0],
            [0.5, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0005, 0.0, 0.5],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        &[vec![0, 1, 2, 3, 4, 5]],
    )
}

#[test]
fn sl_co1_co2_co3_colinear_points_within_distance() {
    for (distance, want) in [
        (COLINEAR_DISTANCE, vec![1]),
        (0.001, vec![1, 3]),
        (0.0004, vec![1]),
    ] {
        let mut e = editor(outline(), Mode::Vertex);
        run(&mut e, SelectRequest::ColinearVertices { distance });
        assert_eq!(verts(&e), want.into_iter().collect(), "{distance}");
    }
}

// ---- Diagonal and Coplanar Edges (SL-20..22, SL-40..44) ----

fn tri_grid(lift: bool) -> Mesh {
    let mut p = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            p.push([x as f64, y as f64, 0.]);
        }
    }
    if lift {
        p[8][2] = 0.4;
    }
    let f = [
        [0, 1, 4],
        [0, 4, 3],
        [1, 2, 5],
        [1, 5, 4],
        [3, 4, 7],
        [3, 7, 6],
        [4, 5, 8],
        [4, 8, 7],
    ]
    .map(|t| t.to_vec());
    mesh(&p, &f)
}

#[test]
fn sl_di1_one_diagonal_per_quad_on_a_flat_grid() {
    let mut e = editor(tri_grid(false), Mode::Edge);
    run(
        &mut e,
        SelectRequest::Diagonal(DetriangulateOptions::default()),
    );
    assert_eq!(
        edges(&e),
        [(0, 4), (1, 5), (3, 7), (4, 8)].into_iter().collect()
    );
}

#[test]
fn sl_di2_any_valid_pairing_with_a_bent_quad() {
    let m = tri_grid(true);
    let mut e = editor(m.clone(), Mode::Edge);
    run(
        &mut e,
        SelectRequest::Diagonal(DetriangulateOptions::default()),
    );
    // SL-42: each chosen edge joins two triangles, and no triangle twice.
    let ef = m.edge_faces();
    let mut used = BTreeSet::new();
    assert!(!edges(&e).is_empty());
    for e in edges(&e) {
        let f = &ef[&e];
        assert_eq!(f.len(), 2);
        for &t in f {
            assert!(used.insert(t), "triangle {t} used twice");
        }
    }
}

#[test]
fn sl_ce1_ce2_coplanar_edges() {
    let mut e = editor(tri_grid(false), Mode::Edge);
    run(
        &mut e,
        SelectRequest::CoplanarEdges {
            flatness: COPLANAR_FLATNESS,
        },
    );
    assert_eq!(
        edges(&e),
        [
            (0, 4),
            (1, 4),
            (1, 5),
            (3, 4),
            (3, 7),
            (4, 5),
            (4, 7),
            (4, 8)
        ]
        .into_iter()
        .collect()
    );
    let mut e = editor(tri_grid(true), Mode::Edge);
    run(
        &mut e,
        SelectRequest::CoplanarEdges {
            flatness: COPLANAR_FLATNESS,
        },
    );
    assert_eq!(
        edges(&e),
        [(0, 4), (1, 4), (1, 5), (3, 4), (3, 7)]
            .into_iter()
            .collect()
    );
}

// ---- Coplanar Polygons (SL-50..53) ----

fn cp_strip() -> Mesh {
    mesh(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [2.0, 0.0, 1.0],
            [3.0, 0.0, 1.0],
            [3.99863, 0.052336, 0.0],
            [3.99863, 0.052336, 1.0],
            [-0.984808, 0.173648, 0.0],
            [-0.984808, 0.173648, 1.0],
            [0.0, 0.5, 2.0],
            [1.0, 0.5, 2.0],
            [1.0, 0.5, 3.0],
            [0.0, 0.5, 3.0],
            [0.0, 0.05, 2.0],
            [1.0, 0.05, 2.0],
            [1.0, 0.05, 3.0],
            [0.0, 0.05, 3.0],
        ],
        &[
            vec![0, 1, 5, 4],
            vec![1, 2, 6, 5],
            vec![2, 3, 7, 6],
            vec![3, 8, 9, 7],
            vec![10, 0, 4, 11],
            vec![12, 13, 14, 15],
            vec![16, 17, 18, 19],
        ],
    )
}

#[test]
fn sl_cp1_cp2_cp3_coplanar_polygons() {
    for (connectivity, range, want) in [
        (true, 1., vec![0, 1, 2, 3]),
        (false, 1., vec![0, 1, 2, 3, 5, 6]),
        (false, 0.1, vec![0, 1, 2, 3, 6]),
    ] {
        let mut e = editor(cp_strip(), Mode::Polygon);
        e.scene.selection.faces = vec![1];
        run(
            &mut e,
            SelectRequest::CoplanarPolygons {
                start: vec![1],
                options: CoplanarPolygonOptions {
                    angle: 5.,
                    connectivity,
                    range,
                },
            },
        );
        assert_eq!(
            faces(&e),
            want.into_iter().collect(),
            "{connectivity} {range}"
        );
    }
    // CP4: no start polygon.
    let mut e = editor(cp_strip(), Mode::Polygon);
    assert!(
        e.dispatch(Command::SelectTool(SelectRequest::CoplanarPolygons {
            start: vec![],
            options: Default::default()
        }))
        .is_err()
    );
}

// ---- Conditional Loop (SL-60..62) ----

#[test]
fn sl_cl1_conditional_loop_from_one_edge() {
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(13, 14)];
    run(&mut e, SelectRequest::ConditionalLoop(Default::default()));
    assert_eq!(
        edges(&e),
        [(12, 13), (13, 14), (14, 15), (15, 16), (16, 17)]
            .into_iter()
            .collect()
    );
}

#[test]
fn conditional_loop_stop_rules() {
    // Max Loop caps the count.
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(13, 14)];
    run(
        &mut e,
        SelectRequest::ConditionalLoop(ConditionalLoopOptions {
            max_loop: 3,
            ..Default::default()
        }),
    );
    assert_eq!(edges(&e).len(), 3);
    // Length Difference: a stretched last column stops the loop.
    let mut m = grid();
    for y in 0..6 {
        m.vertices[y * 6 + 5].x = 20.;
    }
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = vec![(13, 14)];
    run(
        &mut e,
        SelectRequest::ConditionalLoop(ConditionalLoopOptions {
            stop_length: true,
            ..Default::default()
        }),
    );
    assert!(!edges(&e).contains(&(16, 17)));
    assert!(edges(&e).contains(&(15, 16)));
    // Sharp Angles: a row bent 90° at point 15 stops there.
    let mut m = grid();
    for y in 0..6 {
        for x in 4..6 {
            let v = &mut m.vertices[y * 6 + x];
            v.z = v.x - 3.;
            v.x = 3.;
        }
    }
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = vec![(13, 14)];
    run(&mut e, SelectRequest::ConditionalLoop(Default::default()));
    assert!(edges(&e).contains(&(14, 15)));
    assert!(!edges(&e).contains(&(15, 16)));
    // Selection: crossing two selected edges stops it.
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(15, 16), (16, 17), (12, 13)];
    run(
        &mut e,
        SelectRequest::ConditionalLoop(ConditionalLoopOptions {
            stop_selection: true,
            selection_count: 1,
            ..Default::default()
        }),
    );
    // Seed (12, 13) grows right until it reaches selected (15, 16).
    assert!(edges(&e).contains(&(13, 14)));
    assert!(edges(&e).contains(&(14, 15)));
}

// ---- Shared rules ----

#[test]
fn select_tools_follow_symmetry_and_refuse_item_mode() {
    let mut m = grid();
    for v in &mut m.vertices {
        v.x -= 2.5;
    }
    let mut e = editor(m, Mode::Vertex);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.scene.selection.vertices = vec![0, 2];
    run(&mut e, SelectRequest::ShortestPath);
    // 0, 1, 2 and their mirrors 5, 4, 3.
    assert_eq!(verts(&e), (0..6).collect());
    e.scene.selection.mode = Mode::Item;
    assert!(
        e.dispatch(Command::SelectTool(SelectRequest::Sharp {
            min: SHARP_MIN,
            max: SHARP_MAX
        }))
        .is_err()
    );
}

// ---- SL-64 Conditional Loop stop rules (probe 2026-11-17) ----

/// An `nx` x `ny` quad grid in XY, points row by row.
fn quad_grid(nx: u32, ny: u32) -> Mesh {
    let mut p = Vec::new();
    for y in 0..=ny {
        for x in 0..=nx {
            p.push([x as f64, y as f64, 0.]);
        }
    }
    let w = nx + 1;
    let mut f = Vec::new();
    for y in 0..ny {
        for x in 0..nx {
            let i = y * w + x;
            f.push(vec![i, i + 1, i + w + 1, i + w]);
        }
    }
    mesh(&p, &f)
}

fn cloop(m: Mesh, seed: Vec<(u32, u32)>, o: ConditionalLoopOptions) -> BTreeSet<(u32, u32)> {
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = seed;
    run(&mut e, SelectRequest::ConditionalLoop(o));
    edges(&e)
}

#[test]
fn sl64_a_middle_edge_runs_the_whole_row() {
    // 6 x 2 grid: the middle row's points are 7..13.
    let got = cloop(quad_grid(6, 2), vec![(9, 10)], Default::default());
    assert_eq!(got, (7..13).map(|i| (i, i + 1)).collect());
}

#[test]
fn sl64_a_closed_ring_closes_on_itself() {
    // An 8-sided tube, two rings of 8 points (0..8 bottom, 8..16 top) and a
    // middle ring (16..24).
    let mut p = Vec::new();
    for h in [0., 2., 1.] {
        for k in 0..8 {
            let a = k as f64 * std::f64::consts::TAU / 8.;
            p.push([a.cos(), h, a.sin()]);
        }
    }
    let mut f = Vec::new();
    for k in 0..8u32 {
        let n = (k + 1) % 8;
        f.push(vec![k, n, 16 + n, 16 + k]);
        f.push(vec![16 + k, 16 + n, 8 + n, 8 + k]);
    }
    let got = cloop(mesh(&p, &f), vec![(16, 17)], Default::default());
    assert_eq!(got.len(), 8, "{got:?}");
    assert!(got.iter().all(|&(a, b)| a >= 16 && b >= 16));
}

#[test]
fn sl64_max_loop_1_adds_nothing() {
    let got = cloop(
        quad_grid(6, 2),
        vec![(9, 10)],
        ConditionalLoopOptions {
            max_loop: 1,
            ..Default::default()
        },
    );
    assert_eq!(got, [(9, 10)].into_iter().collect());
}

#[test]
fn sl64_sharp_angles_stop_at_a_90_degree_fold() {
    // A 6 x 2 strip folded 90° along the column x = 3.
    let mut m = quad_grid(6, 2);
    for v in &mut m.vertices {
        if v.x > 3. {
            *v = DVec3::new(3., v.y, v.x - 3.);
        }
    }
    let on = cloop(m.clone(), vec![(7, 8)], Default::default());
    assert_eq!(on.len(), 3, "stops at the fold: {on:?}");
    let off = cloop(
        m,
        vec![(7, 8)],
        ConditionalLoopOptions {
            stop_sharp: false,
            ..Default::default()
        },
    );
    assert_eq!(off.len(), 6, "carries on over the fold: {off:?}");
}

#[test]
fn sl64_stop_at_selection_count_1_keeps_the_other_selected_edge() {
    // Two edges in the middle row; the last selected (8, 9) grows.
    let got = cloop(
        quad_grid(6, 2),
        vec![(11, 12), (8, 9)],
        ConditionalLoopOptions {
            selection_count: 1,
            ..Default::default()
        },
    );
    // Towards (11, 12) it stops on reaching it (kept); the other way it
    // runs to the border.
    let want: BTreeSet<(u32, u32)> = [(7, 8), (8, 9), (9, 10), (10, 11), (11, 12)]
        .into_iter()
        .collect();
    assert_eq!(got, want);
}
