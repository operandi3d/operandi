//! Milestone 3 Vertex panel: one focused geometry oracle per row, editor
//! lifecycle/history/refusals, and saved demonstration scenes.
//!
//! Scenes live in `docs/m3-vertex-tools/scenes`; regenerate them with
//! `OPERANDI_WRITE_M3_DEMO=1 cargo test -p operandi-core --test all m3_vertex_tools::`.
use glam::DVec3;
use operandi_core::*;
use std::path::PathBuf;

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
fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}
/// 3×3 quads in z = 0 facing +z; point `r * 4 + c` sits at (c, r, 0).
/// Points 5, 6, 9, 10 are interior (four polygons each).
fn grid() -> Mesh {
    let mut m = Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64, (i / 4) as f64, 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3 {
        for c in 0..3 {
            let i = r * 4 + c;
            m.faces.push(face(vec![i, i + 1, i + 5, i + 4]));
        }
    }
    m.validate().unwrap();
    m
}
/// Two separate quads whose facing sides coincide (points 1=4, 2=7).
fn seam(gap: f64) -> Mesh {
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.);
    let m = Mesh {
        vertices: vec![
            p(0., 0.),
            p(1., 0.),
            p(1., 1.),
            p(0., 1.),
            p(1. + gap, 0.),
            p(2., 0.),
            p(2., 1.),
            p(1. + gap, 1.),
        ],
        faces: vec![face(vec![0, 1, 2, 3]), face(vec![4, 5, 6, 7])].into(),
        ..Default::default()
    };
    m.validate().unwrap();
    m
}
fn run(m: &Mesh, sel: &[u32], c: VertexCommand) -> (Mesh, Vec<u32>) {
    let r = vertex_command_geometry(m, sel, c).unwrap_or_else(|e| panic!("{c:?}: {e}"));
    r.0.validate().unwrap();
    r
}
fn uses(m: &Mesh, v: u32) -> usize {
    m.faces.iter().filter(|f| f.vertices.contains(&v)).count()
}
fn near(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < 1e-9
}

#[test]
fn split_gives_each_polygon_its_own_coincident_point() {
    let (out, sel) = run(&grid(), &[5], VertexCommand::Split);
    assert_eq!(out.vertices.len(), 19);
    assert_eq!(out.faces.len(), 9);
    assert_eq!(uses(&out, 5), 1);
    assert_eq!(sel.len(), 4);
    for &v in &sel {
        assert!(near(out.vertices[v as usize], DVec3::new(1., 1., 0.)));
        assert_eq!(uses(&out, v), 1);
    }
    // A point on one polygon only has nothing to split.
    assert!(vertex_command_geometry(&grid(), &[0], VertexCommand::Split).is_err());
}

#[test]
fn split_edge_adds_a_middle_point_shared_by_both_polygons() {
    let (out, sel) = run(&grid(), &[5, 6], VertexCommand::SplitEdge);
    assert_eq!(out.vertices.len(), 17);
    assert!(near(out.vertices[16], DVec3::new(1.5, 1., 0.)));
    assert_eq!(uses(&out, 16), 2);
    assert_eq!(sel, vec![5, 6, 16]);
    assert!(vertex_command_geometry(&grid(), &[5, 10], VertexCommand::SplitEdge).is_err());
}

#[test]
fn join_welds_to_the_last_point_and_join_averaged_to_the_middle() {
    let (out, sel) = run(
        &grid(),
        &[6, 5],
        VertexCommand::Join {
            average: false,
            options: MergeOptions::default(),
        },
    );
    assert_eq!(out.vertices.len(), 15);
    assert!(near(out.vertices[sel[0] as usize], DVec3::new(1., 1., 0.)));
    // The two quads on the joined edge become triangles.
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        2
    );
    let (out, sel) = run(
        &grid(),
        &[6, 5],
        VertexCommand::Join {
            average: true,
            options: MergeOptions::default(),
        },
    );
    assert!(near(out.vertices[sel[0] as usize], DVec3::new(1.5, 1., 0.)));
    assert!(
        vertex_command_geometry(
            &grid(),
            &[5],
            VertexCommand::Join {
                average: false,
                options: MergeOptions::default()
            }
        )
        .is_err()
    );
}

#[test]
fn merge_automatic_and_fixed_ranges() {
    let all: Vec<u32> = (0..8).collect();
    let (out, _) = run(
        &seam(0.),
        &all,
        VertexCommand::Merge {
            distance: 0.,
            options: MergeOptions::default(),
        },
    );
    assert_eq!(out.vertices.len(), 6);
    assert_eq!(out.edges().len(), 7, "the two quads now share one edge");
    // Automatic ignores a small gap; Fixed closes it at the later point.
    let (out, _) = run(
        &seam(0.01),
        &all,
        VertexCommand::Merge {
            distance: 0.,
            options: MergeOptions::default(),
        },
    );
    assert_eq!(out.vertices.len(), 8);
    let (out, _) = run(
        &seam(0.01),
        &all,
        VertexCommand::Merge {
            distance: 0.02,
            options: MergeOptions::default(),
        },
    );
    assert_eq!(out.vertices.len(), 6);
    assert!(
        out.vertices
            .iter()
            .any(|p| near(*p, DVec3::new(1.005, 0., 0.)))
    );
    // VX-M01: each group lands at its average.
}

#[test]
fn collapse_merges_the_fan_into_one_polygon() {
    // CMD-B3-02: the point and its edge neighbours merge at their mean; the
    // four polygons around it go.
    let (out, sel) = run(&grid(), &[5], VertexCommand::Collapse);
    assert_eq!((out.vertices.len(), out.faces.len()), (12, 5));
    assert!(near(out.vertices[sel[0] as usize], DVec3::new(1., 1., 0.)));
    // A border point (G3 point 1 with neighbours 0, 2, 5): 13 points, 7
    // polygons, merged at (1, 0.25).
    let (out, sel) = run(&grid(), &[1], VertexCommand::Collapse);
    assert_eq!((out.vertices.len(), out.faces.len()), (13, 7));
    assert!(near(
        out.vertices[sel[0] as usize],
        DVec3::new(1., 0.25, 0.)
    ));
}

#[test]
fn collapse_of_a_cube_corner_leaves_a_closed_three_quad_solid() {
    // CMD-B3-02 on a valence-3 corner (a reported "pyramid", 2026-11-25):
    // the corner and its three edge neighbours become one point at their
    // mean, the three quads around the corner go, and the three far quads
    // each take the merged point: 5 points, 3 quads, still closed. That is
    // the measured rule (the valence-3 fixture goes 13/9 -> 10/6), not a bug.
    let cube = cube();
    // The lowest corner; its neighbours are one unit along each axis.
    let low = cube.vertices.iter().copied().reduce(DVec3::min).unwrap();
    let corner = cube.vertices.iter().position(|p| near(*p, low)).unwrap() as u32;
    let (out, sel) = run(&cube, &[corner], VertexCommand::Collapse);
    assert_eq!((out.vertices.len(), out.faces.len()), (5, 3));
    assert!(out.faces.iter().all(|f| f.vertices.len() == 4));
    assert!(near(
        out.vertices[sel[0] as usize],
        low + DVec3::splat(0.25)
    ));
    assert!(out.edge_counts().values().all(|&n| n == 2), "closed");
}

#[test]
fn remove_deletes_points_and_their_polygons_and_remove_edge_dissolves() {
    // CMD-V-04: the polygons keep their other corners (quads become triangles).
    let (out, sel) = run(&grid(), &[5], VertexCommand::Remove);
    assert_eq!((out.vertices.len(), out.faces.len()), (15, 9));
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        4
    );
    assert!(sel.is_empty());
    let (out, _) = run(&grid(), &[5, 6], VertexCommand::RemoveEdge);
    assert_eq!((out.vertices.len(), out.faces.len()), (16, 8));
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 6).count(),
        1
    );
}

#[test]
fn align_projects_or_spaces_along_the_first_to_last_line() {
    let mut m = grid();
    m.vertices[5] = DVec3::new(1.2, 1.3, 0.);
    m.vertices[6] = DVec3::new(2., 0.8, 0.);
    let all = [true; 3];
    let (out, _) = run(
        &m,
        &[4, 5, 6, 7],
        VertexCommand::Align {
            axes: all,
            uniform: false,
        },
    );
    assert!(near(out.vertices[5], DVec3::new(1.2, 1., 0.)));
    let (out, _) = run(
        &m,
        &[4, 5, 6, 7],
        VertexCommand::Align {
            axes: all,
            uniform: true,
        },
    );
    assert!(near(out.vertices[5], DVec3::new(1., 1., 0.)));
    // Y only: X stays.
    let (out, _) = run(
        &m,
        &[4, 5, 6, 7],
        VertexCommand::Align {
            axes: [false, true, false],
            uniform: true,
        },
    );
    assert!(near(out.vertices[5], DVec3::new(1.2, 1., 0.)));
    assert!(
        vertex_command_geometry(
            &m,
            &[4, 7],
            VertexCommand::Align {
                axes: all,
                uniform: false
            }
        )
        .is_err()
    );
}

/// Each polygon as a cycle of positions, for shape comparisons.
fn shape(m: &Mesh) -> Vec<Vec<[i64; 3]>> {
    m.faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    (m.vertices[v as usize] * 1e6)
                        .round()
                        .as_i64vec3()
                        .to_array()
                })
                .collect()
        })
        .collect()
}

#[test]
fn reorder_index_modes_keep_the_shape() {
    let c = cube();
    for mode in [ReorderMode::Swap, ReorderMode::Top, ReorderMode::Bottom] {
        let (out, sel) = run(&c, &[6, 2, 4], VertexCommand::Reorder(mode));
        assert_eq!(shape(&out), shape(&c), "{mode:?}");
        for (k, &v) in [6u32, 2, 4].iter().enumerate() {
            assert_eq!(out.vertices[sel[k] as usize], c.vertices[v as usize]);
        }
        match mode {
            ReorderMode::Swap => assert_eq!(sel, vec![2, 4, 6]),
            ReorderMode::Top => assert_eq!(sel, vec![0, 1, 2]),
            ReorderMode::Bottom => assert_eq!(sel, vec![5, 6, 7]),
        }
    }
}

#[test]
fn sync_index_order_matches_a_reordered_copy() {
    let source = cube();
    let (target, _) = run(
        &source,
        &[6, 2, 4, 7],
        VertexCommand::Reorder(ReorderMode::Top),
    );
    assert_ne!(target.vertices, source.vertices);
    let (synced, moved) = sync_index_order(&source, [0, 1], &target, [0, 1]).unwrap();
    assert!(moved > 0);
    assert_eq!(synced.vertices, source.vertices);
    assert_eq!(shape(&synced), shape(&target));
    // Different topology is refused.
    assert!(sync_index_order(&source, [0, 1], &grid(), [0, 1]).is_err());
}

#[test]
fn make_polygon_follows_the_selection_order() {
    let mut m = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
        ..Default::default()
    };
    let (out, f) = make_polygon_from_points(&m, &[0, 1, 2, 3]).unwrap();
    assert!(near(out.normal(f as usize), DVec3::Z));
    let (out, f) = make_polygon_from_points(&m, &[3, 2, 1, 0]).unwrap();
    assert!(near(out.normal(f as usize), -DVec3::Z));
    // Two points make a line (make-split-2026-11-05, MP05).
    let (out, f) = make_polygon_from_points(&m, &[0, 1]).unwrap();
    assert!(out.is_polyline(f as usize));
    // Collinear points make the flat polygon asked for (rule 1).
    m.vertices[2] = DVec3::new(2., 0., 0.);
    let (out, f) = make_polygon_from_points(&m, &[0, 1, 2]).unwrap();
    assert!(out.attributes.degenerate_faces.contains(&f));
}

#[test]
fn make_polygon_fills_a_hole_whatever_the_selection_order() {
    // Jason's GUI test (2026-10-31): the reversed order and a lasso-style
    // order are rewound to match the neighbours instead of refused.
    let mut holed = grid();
    holed.faces.remove(4);
    holed.validate().unwrap();
    for order in [[5, 6, 10, 9], [9, 10, 6, 5], [5, 6, 9, 10], [10, 5, 9, 6]] {
        let (out, f) =
            make_polygon_from_points(&holed, &order).unwrap_or_else(|e| panic!("{order:?}: {e}"));
        out.validate().unwrap();
        assert!(near(out.normal(f as usize), DVec3::Z), "{order:?}");
    }
    // Points inside the surface would reuse edges in every order.
    assert!(make_polygon_from_points(&grid(), &[5, 6, 10, 9]).is_err());
}

#[test]
fn vertex_bevel_replaces_a_corner_with_a_polygon() {
    let c = cube();
    let (out, sel) = vertex_bevel_geometry(&c, &[0], 0.2).unwrap();
    out.validate().unwrap();
    assert_eq!((out.vertices.len(), out.faces.len()), (10, 7));
    assert_eq!(sel.len(), 3);
    let corner = c.vertices[0];
    for &v in &sel {
        assert!((out.vertices[v as usize].distance(corner) - 0.2).abs() < 1e-9);
    }
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 5).count(),
        3
    );
    assert!(out.is_closed());
    // VX-B05: an Inset past the edges stops at their far ends; VX-B03: a
    // corner point of one polygon is cut off.
    let (long, sel) = vertex_bevel_geometry(&c, &[0], 1.).unwrap();
    for &v in &sel {
        assert!((long.vertices[v as usize].distance(corner) - 1.).abs() < 1e-9);
    }
    let (cut, _) = vertex_bevel_geometry(&grid(), &[0], 0.2).unwrap();
    assert_eq!((cut.vertices.len(), cut.faces.len()), (17, 9));
}

#[test]
fn vertex_extrude_raises_a_point_inside_a_ring_of_triangles() {
    let (out, sel) = vertex_extrude_geometry(&grid(), &[5], 0.5, 0.25).unwrap();
    out.validate().unwrap();
    assert_eq!((out.vertices.len(), out.faces.len()), (20, 13));
    assert_eq!(sel, vec![5]);
    assert!(near(out.vertices[5], DVec3::new(1., 1., 0.5)));
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        4
    );
    // Width 0 changes nothing (extrude-2026-11-05, VE06).
    assert_eq!(
        vertex_extrude_geometry(&grid(), &[5], 0.5, 0.).unwrap().0,
        grid()
    );
}

#[test]
fn tear_moves_the_selected_polygons_corner_away() {
    // Jason's GUI test (2026-10-31): Tear needs a polygon and a point; the
    // selected polygon's corner at that point is torn off and moved.
    let offset = DVec3::new(-0.2, -0.2, 0.);
    let (out, sel) = vertex_tear_geometry(&grid(), &[5], &[4], offset).unwrap();
    out.validate().unwrap();
    assert_eq!(out.vertices.len(), 17);
    assert!(near(
        out.vertices[sel[0] as usize],
        DVec3::new(0.8, 0.8, 0.)
    ));
    // The selected polygon took the new point, whatever the drag direction.
    assert!(out.faces[4].vertices.contains(&sel[0]));
    assert_eq!(uses(&out, 5), 3);
    // Two selected polygons at the point share one torn copy.
    let (out, sel) = vertex_tear_geometry(&grid(), &[5], &[4, 3], offset).unwrap();
    assert_eq!(out.vertices.len(), 17);
    assert_eq!(uses(&out, sel[0]), 2);
    // A point without a selected polygon, a polygon without a selected
    // point, or a point whose polygons are all selected is refused.
    assert!(vertex_tear_geometry(&grid(), &[5], &[], offset).is_err());
    assert!(vertex_tear_geometry(&grid(), &[0], &[4], offset).is_err());
    assert!(vertex_tear_geometry(&grid(), &[5], &[0, 1, 3, 4], offset).is_err());
}

#[test]
fn slide_runs_along_the_best_aligned_edge() {
    let out = vertex_slide_geometry(&grid(), &[5], DVec3::X, 0.5).unwrap();
    assert!(near(out.vertices[5], DVec3::new(1.5, 1., 0.)));
    let out = vertex_slide_geometry(&grid(), &[5], DVec3::X, -1.).unwrap();
    assert!(near(out.vertices[5], DVec3::new(0., 1., 0.)));
    assert_eq!(out.faces, grid().faces);
}

#[test]
fn weld_moves_sources_onto_the_target() {
    let (out, sel) = weld_points(&grid(), &[5], 6).unwrap();
    assert_eq!(out.vertices.len(), 15);
    assert!(near(out.vertices[sel[0] as usize], DVec3::new(2., 1., 0.)));
    assert!(weld_points(&grid(), &[6], 6).is_err());
}

fn editor(m: Mesh, points: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = points;
    e
}

#[test]
fn editor_rows_are_single_undo_steps_and_refusals_change_nothing() {
    let mut e = editor(grid(), vec![5]);
    let before = e.scene.clone();
    e.dispatch(Command::VertexCommand(VertexCommand::Remove))
        .unwrap();
    assert_eq!(e.history.undo.last().unwrap().label, "Remove");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    let history = e.history.undo.len();
    // Wrong mode, empty selection and an ineligible point are refused.
    e.scene.selection.mode = Mode::Edge;
    assert!(
        e.dispatch(Command::VertexCommand(VertexCommand::Split))
            .is_err()
    );
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices.clear();
    assert!(
        e.dispatch(Command::VertexCommand(VertexCommand::Split))
            .is_err()
    );
    e.scene.selection.vertices = vec![0];
    assert!(
        e.dispatch(Command::VertexCommand(VertexCommand::Split))
            .is_err()
    );
    // A corner point is a valid Vertex Bevel now (VX-B03); with no point
    // selected it is still refused.
    e.scene.selection.vertices.clear();
    assert!(e.dispatch(Command::Activate(Tool::VertexBevel)).is_err());
    assert_eq!(e.history.undo.len(), history);
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    // Set Position works on selected points in Vertex mode.
    e.scene.selection.vertices = vec![5, 6];
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value: 0.5,
        expected: None,
    }))
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        (m.vertices[5].z, m.vertices[6].z, m.vertices[9].z),
        (0.5, 0.5, 0.)
    );
}

#[test]
fn live_vertex_tools_preview_cancel_and_drop() {
    for tool in [
        Tool::VertexBevel,
        Tool::VertexExtrude,
        Tool::VertexMerge,
        Tool::VertexTear,
        Tool::VertexSlide,
    ] {
        let mut e = editor(grid(), vec![5]);
        if tool == Tool::VertexTear {
            // Tear also needs the polygon whose corner is torn.
            e.scene.selection.faces = vec![4];
        }
        let before = e.scene.clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.inset = 0.25;
        p.shift = 0.5;
        p.delta = DVec3::new(0.2, 0.2, 0.);
        if tool == Tool::VertexBevel || tool == Tool::VertexMerge {
            // Bevel needs an interior closed fan; Merge needs a neighbour.
            e.scene.selection.vertices = vec![5];
        }
        if tool == Tool::VertexMerge {
            e.dispatch(Command::Cancel).unwrap();
            e.scene.selection.vertices = vec![5, 6];
            e.dispatch(Command::Activate(tool)).unwrap();
            p.shift = 1.;
        }
        if tool == Tool::VertexBevel {
            // A grid point has four polygons: a valid interior corner.
            p.inset = 0.25;
        }
        e.dispatch(Command::Preview(p.clone()))
            .unwrap_or_else(|err| panic!("{tool:?}: {err}"));
        assert_ne!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{tool:?}"
        );
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{tool:?} cancel"
        );
        e.scene.selection = before.selection.clone();
        if tool == Tool::VertexMerge {
            e.scene.selection.vertices = vec![5, 6];
        }
        let history = e.history.undo.len();
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(
            e.history.undo.len(),
            history + 1,
            "{tool:?} drop is one step"
        );
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{tool:?} undo"
        );
    }
}

#[test]
fn create_tool_adds_clicked_points_and_selects_them_in_order() {
    let mut e = editor(Mesh::default(), vec![]);
    e.dispatch(Command::Activate(Tool::VertexCreate)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.sketch_path = vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.)];
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 3);
    assert_eq!(e.scene.selection.vertices, vec![0, 1, 2]);
    // Make Polygon (P) from those points.
    e.dispatch(Command::MakePolygon).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Make Polygon");
}

enum Act {
    Tool(Tool, Box<dyn Fn(&mut Parameters)>),
    Command(Box<Command>),
}
type Row = (&'static str, Mesh, Vec<u32>, Act);

fn vc(c: VertexCommand) -> Act {
    Act::Command(Box::new(Command::VertexCommand(c)))
}

fn rows() -> Vec<Row> {
    let mut bent = grid();
    bent.vertices[5].y = 1.3;
    bent.vertices[6].y = 0.8;
    let points = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
        ..Default::default()
    };
    let all8: Vec<u32> = (0..8).collect();
    vec![
        (
            "MV-T01-create",
            Mesh::default(),
            vec![],
            Act::Tool(
                Tool::VertexCreate,
                Box::new(|p| p.sketch_path = vec![DVec3::ZERO, DVec3::X, DVec3::Y]),
            ),
        ),
        (
            "MV-T02-vertex-bevel",
            cube(),
            vec![0],
            Act::Tool(Tool::VertexBevel, Box::new(|p| p.inset = 0.2)),
        ),
        (
            "MV-T03-vertex-extrude",
            grid(),
            vec![5],
            Act::Tool(
                Tool::VertexExtrude,
                Box::new(|p| {
                    p.shift = 0.5;
                    p.inset = 0.25;
                }),
            ),
        ),
        (
            "MV-T04-merge-tool",
            seam(0.01),
            all8.clone(),
            Act::Tool(Tool::VertexMerge, Box::new(|p| p.shift = 0.02)),
        ),
        (
            "MV-T05-weld",
            grid(),
            vec![5],
            Act::Command(Box::new(Command::WeldVertices {
                sources: vec![5],
                target: 6,
            })),
        ),
        (
            "MV-T06-tear",
            grid(),
            vec![5],
            Act::Tool(
                Tool::VertexTear,
                Box::new(|p| p.delta = DVec3::new(0.2, 0.2, 0.)),
            ),
        ),
        (
            "MV-T07-slide",
            grid(),
            vec![5],
            Act::Tool(Tool::VertexSlide, Box::new(|p| p.shift = 0.5)),
        ),
        (
            "MV-C01-new-vertex",
            grid(),
            vec![],
            Act::Command(Box::new(Command::NewVertex(DVec3::new(1.5, 1.5, 1.)))),
        ),
        (
            "MV-C02-make-polygon",
            points,
            vec![0, 1, 2, 3],
            Act::Command(Box::new(Command::MakePolygon)),
        ),
        ("MV-C04-split", grid(), vec![5], vc(VertexCommand::Split)),
        (
            "MV-C05-split-edge",
            grid(),
            vec![5, 6],
            vc(VertexCommand::SplitEdge),
        ),
        (
            "MV-C06-split-polygon",
            grid(),
            vec![5, 10],
            Act::Command(Box::new(Command::SplitPolygon)),
        ),
        (
            "MV-C07-set-position",
            grid(),
            vec![5, 6],
            Act::Command(Box::new(Command::SetPosition(PositionAssignment {
                space: PositionSpace::Local,
                axis: PositionAxis::Z,
                value: 0.5,
                expected: None,
            }))),
        ),
        (
            "MV-C08-align",
            bent,
            vec![4, 5, 6, 7],
            vc(VertexCommand::Align {
                axes: [true; 3],
                uniform: false,
            }),
        ),
        (
            "MV-C09-merge",
            seam(0.),
            all8,
            vc(VertexCommand::Merge {
                distance: 0.,
                options: MergeOptions::default(),
            }),
        ),
        (
            "MV-C10-join",
            grid(),
            vec![6, 5],
            vc(VertexCommand::Join {
                average: false,
                options: MergeOptions::default(),
            }),
        ),
        (
            "MV-C11-join-averaged",
            grid(),
            vec![6, 5],
            vc(VertexCommand::Join {
                average: true,
                options: MergeOptions::default(),
            }),
        ),
        (
            "MV-C12-collapse",
            grid(),
            vec![5],
            vc(VertexCommand::Collapse),
        ),
        ("MV-C13-remove", grid(), vec![5], vc(VertexCommand::Remove)),
        (
            "MV-C14-remove-edge",
            grid(),
            vec![5, 6],
            vc(VertexCommand::RemoveEdge),
        ),
        (
            "MV-C15-reorder-index",
            cube(),
            vec![6, 2, 4],
            vc(VertexCommand::Reorder(ReorderMode::Top)),
        ),
    ]
}

fn act(e: &mut Editor, act: &Act) -> Result<()> {
    match act {
        Act::Tool(tool, set) => {
            e.dispatch(Command::Activate(*tool))?;
            let mut p = e
                .session
                .live
                .as_ref()
                .ok_or("no live tool")?
                .params
                .clone();
            set(&mut p);
            e.dispatch(Command::Preview(p))?;
            e.dispatch(Command::Drop)
        }
        Act::Command(c) => e.dispatch((**c).clone()),
    }
}

fn check_scene(dir: &std::path::Path, write: bool, name: &str, before: &Scene, after: &Scene) {
    for (suffix, scene) in [("before", before), ("after", after)] {
        let path = dir.join(format!("{name}.{suffix}.opscene"));
        if write {
            persistence::save(&path, scene).unwrap();
        }
        let saved =
            persistence::load(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        for (a, b) in saved.items.iter().zip(&scene.items) {
            assert_eq!(
                a.mesh,
                b.mesh,
                "{} is stale; regenerate (see module docs)",
                path.display()
            );
        }
    }
}

/// Every M3 row except the display toggle (MV-C03, app-only) has a saved
/// before/after pair produced through the Editor, with one-step undo/redo.
#[test]
fn m3_demo_scenes_cover_every_row_and_are_current() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/m3-vertex-tools/scenes");
    let write = std::env::var_os("OPERANDI_WRITE_M3_DEMO").is_some();
    if write {
        std::fs::create_dir_all(&dir).unwrap();
    }
    let rows = rows();
    assert_eq!(rows.len(), 21, "one scene pair per row, plus Sync below");
    for (name, m, points, a) in rows {
        let mut e = editor(m, points);
        if name == "MV-T06-tear" {
            // Tear also needs the polygon whose corner is torn.
            e.scene.selection.faces = vec![4];
        }
        let before = e.scene.clone();
        act(&mut e, &a).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_ne!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{name} changed nothing"
        );
        e.scene.validate().unwrap();
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{name} undo"
        );
        e.dispatch(Command::Redo).unwrap();
        let after = e.scene.clone();
        check_scene(&dir, write, name, &before, &after);
    }
    // MV-C16: two meshes; the second is a reordered copy of the first.
    let mut e = editor(cube(), vec![]);
    let source = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let (target, _) = run(
        &cube(),
        &[6, 2, 4, 7],
        VertexCommand::Reorder(ReorderMode::Top),
    );
    e.scene.active_mut().unwrap().mesh = target.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 1];
    let before = e.scene.clone();
    e.dispatch(Command::SyncVertexOrder {
        source_item: source,
        source_faces: [0, 1],
    })
    .unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, cube().vertices);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    let after = e.scene.clone();
    check_scene(&dir, write, "MV-C16-sync-index-order", &before, &after);
}
