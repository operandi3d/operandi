//! Topology Pen v2 modes (BEHAVIOR-TOPOLOGY-PEN TP-10..TP-30,
//! 2026-11-20). Original test scenes: flat grids over a flat background.
use glam::DVec3;
use operandi_core::topology_pen::*;
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

/// An `nx` × `nz` grid of 1 m quads at height `y`, facing +Y.
fn grid(nx: u32, nz: u32, y: f64) -> Mesh {
    let mut vertices = Vec::new();
    for i in 0..=nx {
        for j in 0..=nz {
            vertices.push(DVec3::new(i as f64, y, j as f64));
        }
    }
    let at = |i: u32, j: u32| i * (nz + 1) + j;
    let mut faces = Vec::new();
    for i in 0..nx {
        for j in 0..nz {
            faces.push(face(vec![
                at(i, j),
                at(i, j + 1),
                at(i + 1, j + 1),
                at(i + 1, j),
            ]));
        }
    }
    let m = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    assert!(m.normal(0).y > 0.9, "grid faces +Y");
    m
}

/// A big flat background at y = 0 (its own item), then `active`.
fn scene_with(active: Mesh) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut back = grid(1, 1, 0.);
    for p in &mut back.vertices {
        *p = (*p - DVec3::new(0.5, 0., 0.5)) * 40.;
    }
    e.scene.active_mut().unwrap().mesh = back.into();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = active.into();
    let id = e.scene.active().unwrap().id;
    e.scene.selection.mode = Mode::Vertex;
    e.history.undo.clear();
    (e, id)
}

fn mesh(e: &Editor) -> Mesh {
    (*e.scene.active().unwrap().mesh).clone()
}

fn pen(e: &mut Editor, edit: PenEdit) -> Result<()> {
    e.dispatch(Command::TopologyPen(Box::new(edit)))
}

#[test]
fn tp10_modes_and_defaults() {
    let labels: Vec<_> = PenMode::ALL.iter().map(|m| m.label()).collect();
    assert_eq!(
        labels,
        [
            "Move",
            "Duplicate",
            "Remove",
            "Split",
            "Add Loop",
            "Point",
            "Fill",
            "Smoothing"
        ]
    );
    let o = TopologyPenOptions::default();
    assert_eq!(o.mode, PenMode::Move);
    assert!(!o.edge_loop && !o.keep_vertices && !o.split_middle);
    assert!(o.quad_only && o.lock_corner && !o.lock_boundary);
    assert_eq!(
        (o.fill_range, o.strength, o.iterations, o.radius),
        (1.5, 1., 1, 0.)
    );
    assert!(o.show_vertex && o.show_edge && !o.inner_snap && !o.copy_uvs);
    assert!(!o.edge_slide && !o.backface && !o.raycast_inactive);
}

#[test]
fn tp10_options_stick_within_the_session() {
    let (mut e, _) = scene_with(grid(1, 1, 0.1));
    e.dispatch(Command::ActivatePreset("mesh.topology".into()))
        .unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.topology_pen.mode = PenMode::Fill;
    p.tweak_options.range = 0.5; // the falloff is edited in Tweak's options
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::ActivatePreset("mesh.topology".into()))
        .unwrap();
    let p = &e.session.live.as_ref().unwrap().params;
    assert_eq!(p.topology_pen.mode, PenMode::Fill);
    assert_eq!(p.tweak_options.range, 0.5, "the falloff drives Move");
}

#[test]
fn tp12_remove_polygon_edge_and_vertex() {
    let (mut e, id) = scene_with(grid(2, 2, 0.1));
    // Polygon: deleted.
    pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Polygon(0),
            edge_loop: false,
            keep_vertices: false,
        },
    )
    .unwrap();
    assert_eq!(mesh(&e).faces.len(), 3);
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    // Interior edge (1,0)-(1,1) = points 3-4: its two polygons join.
    pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Edge(3, 4),
            edge_loop: false,
            keep_vertices: true,
        },
    )
    .unwrap();
    assert_eq!(mesh(&e).faces.len(), 3);
    e.dispatch(Command::Undo).unwrap();
    // Interior vertex (the centre, point 4): the four quads become one,
    // and the border midpoints its edges left in a straight line go too
    // (issue #5), leaving the four corners.
    pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Vertex(4),
            edge_loop: false,
            keep_vertices: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1);
    assert_eq!(m.vertices.len(), 4);
    e.dispatch(Command::Undo).unwrap();
    // Border edges are never removed, and a refusal is no undo step.
    let steps = e.history.undo.len();
    let err = pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Edge(0, 1),
            edge_loop: false,
            keep_vertices: false,
        },
    )
    .unwrap_err();
    assert!(err.contains("border"), "{err}");
    assert_eq!(e.history.undo.len(), steps);
    assert_eq!(mesh(&e).faces.len(), 4);
}

#[test]
fn tp12_remove_a_loop_with_and_without_keep_vertices() {
    // 3 × 3 grid: the loop along x = 1 crosses three interior edges.
    let (mut e, id) = scene_with(grid(3, 3, 0.1));
    let a = 4; // (1, 0)
    let b = 5; // (1, 1)
    assert_eq!(pen_edge_loop(&e.scene, id, a, b).len(), 3);
    pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Edge(a, b),
            edge_loop: true,
            keep_vertices: true,
        },
    )
    .unwrap();
    let kept = mesh(&e);
    assert_eq!(kept.faces.len(), 6);
    assert_eq!(kept.vertices.len(), 16, "Keep Vertices keeps the ends");
    e.dispatch(Command::Undo).unwrap();
    pen(
        &mut e,
        PenEdit::Remove {
            item: id,
            element: TweakElement::Edge(a, b),
            edge_loop: true,
            keep_vertices: false,
        },
    )
    .unwrap();
    let cleaned = mesh(&e);
    assert_eq!(cleaned.faces.len(), 6);
    assert!(
        cleaned.vertices.len() < 16,
        "points left in only two edges go"
    );
}

#[test]
fn tp13_split_cuts_between_two_points_of_one_polygon() {
    let (mut e, id) = scene_with(grid(2, 1, 0.1));
    // Diagonal of the first quad: points 0 and 3.
    pen(
        &mut e,
        PenEdit::Split {
            item: id,
            a: 0,
            b: 3,
        },
    )
    .unwrap();
    assert_eq!(mesh(&e).faces.len(), 3);
    assert_eq!(e.history.undo.len(), 1);
    // Already an edge: nothing, and why.
    let err = pen(
        &mut e,
        PenEdit::Split {
            item: id,
            a: 0,
            b: 1,
        },
    )
    .unwrap_err();
    assert!(err.contains("already"), "{err}");
    // Points of different polygons: nothing.
    let err = pen(
        &mut e,
        PenEdit::Split {
            item: id,
            a: 0,
            b: 5,
        },
    )
    .unwrap_err();
    assert!(err.contains("same polygon"), "{err}");
    assert_eq!(e.history.undo.len(), 1);
}

#[test]
fn tp11_duplicate_a_border_vertex_makes_a_triangle_on_the_background() {
    let (mut e, id) = scene_with(grid(1, 1, 0.1));
    // Corner (1, 1) is point 3; dragging toward -Z picks neighbour (1, 0).
    pen(
        &mut e,
        PenEdit::DuplicateVertex {
            item: id,
            v: 3,
            offset: DVec3::new(0.6, 0., -0.4),
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.faces[1].vertices.len(), 3);
    assert!(m.faces[1].vertices.contains(&2), "partner (1, 0)");
    assert!(m.vertices[4].y.abs() < 1e-9, "on the background");
    assert!(m.normal(1).y > 0., "faces like its neighbour");
    assert_eq!(e.history.undo.len(), 1);
}

#[test]
fn tp11_duplicate_refuses_an_interior_edge_and_pulls_border_loops() {
    let (mut e, id) = scene_with(grid(2, 1, 0.1));
    // (1,0)-(1,1) = points 2-3 is interior.
    let err = pen(
        &mut e,
        PenEdit::DuplicateLoop {
            item: id,
            a: 2,
            b: 3,
            offset: DVec3::X,
        },
    )
    .unwrap_err();
    assert_eq!(err, "Duplicate needs a border edge");
    assert!(e.history.undo.is_empty());
    // The whole border (six edges) comes out as one row.
    pen(
        &mut e,
        PenEdit::DuplicateLoop {
            item: id,
            a: 0,
            b: 1,
            offset: DVec3::new(0., 0.2, 0.),
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 2 + 6);
    assert_eq!(m.vertices.len(), 6 + 6, "new points shared along the row");
    for v in 6..12 {
        assert!(m.vertices[v].y.abs() < 1e-9, "lands on the background");
    }
    assert_eq!(e.history.undo.len(), 1);
}

#[test]
fn tp15_point_makes_a_one_point_polygon() {
    let (mut e, _) = scene_with(grid(1, 1, 0.1));
    pen(
        &mut e,
        PenEdit::Point {
            world: DVec3::new(3., 0., 3.),
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.faces[1].vertices.len(), 1);
    e.scene.selection.mode = Mode::Item;
    let err = pen(&mut e, PenEdit::Point { world: DVec3::ZERO }).unwrap_err();
    assert!(err.contains("component mode"), "{err}");
}

#[test]
fn tp16_fill_makes_a_quad_from_four_points_and_drops_their_single_point_polygons() {
    let (mut e, id) = scene_with(Mesh::default());
    for (x, z) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
        pen(
            &mut e,
            PenEdit::Point {
                world: DVec3::new(x, 0., z),
            },
        )
        .unwrap();
    }
    let fill = |e: &mut Editor, quad_only| {
        pen(
            e,
            PenEdit::Fill {
                item: id,
                click: DVec3::new(0.5, 0., 0.45),
                toward_viewer: DVec3::Y,
                range: 1.5,
                quad_only,
            },
        )
    };
    let steps = e.history.undo.len();
    fill(&mut e, true).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1, "one quad, no one-point polygons left");
    assert_eq!(m.faces[0].vertices.len(), 4);
    assert!(m.normal(0).y > 0.9, "faces the viewer");
    assert_eq!(e.history.undo.len(), steps + 1);
    // Clicking on the new polygon fills nothing.
    assert!(fill(&mut e, true).is_err());
}

#[test]
fn tp16_quad_only_off_allows_a_triangle() {
    let (mut e, id) = scene_with(Mesh::default());
    for (x, z) in [(0., 0.), (1., 0.), (0., 1.)] {
        pen(
            &mut e,
            PenEdit::Point {
                world: DVec3::new(x, 0., z),
            },
        )
        .unwrap();
    }
    let fill = |e: &mut Editor, quad_only| {
        pen(
            e,
            PenEdit::Fill {
                item: id,
                click: DVec3::new(0.3, 0., 0.3),
                toward_viewer: DVec3::Y,
                range: 1.5,
                quad_only,
            },
        )
    };
    assert!(fill(&mut e, true).unwrap_err().contains("Quad Only"));
    fill(&mut e, false).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1);
    assert_eq!(m.faces[0].vertices.len(), 3);
    assert!(m.normal(0).y > 0.9);
}

#[test]
fn tp20_ctrl_click_quad_faces_the_viewer() {
    let (mut e, _) = scene_with(Mesh::default());
    pen(
        &mut e,
        PenEdit::Quad {
            center: DVec3::new(2., 0., 2.),
            right: DVec3::X,
            up: DVec3::NEG_Z,
            side: 0.5,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1);
    assert!(m.normal(0).y > 0.9, "facing +Y: {:?}", m.normal(0));
    let xs: Vec<f64> = m.vertices.iter().map(|p| p.x).collect();
    let width =
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!((width - 0.5).abs() < 1e-9);
}

#[test]
fn tp17_smoothing_relaxes_and_respects_the_locks() {
    let mut m = grid(2, 2, 0.);
    m.vertices[4].x += 0.4; // the centre, pushed off
    m.vertices[0].x -= 0.3; // a corner, pushed off
    let all = falloff_weights(&m, DVec3::ZERO, 0., FalloffShape::Linear);
    assert_eq!(all.len(), 9, "Radius 0: the whole mesh");
    let moved = smooth_points(&m, &all, 1., 1, false, true, None);
    let centre = moved.iter().find(|(v, _)| *v == 4).unwrap().1;
    assert!(
        (centre.x - 1.).abs() < 0.2,
        "back toward the middle: {centre}"
    );
    assert!(moved.iter().all(|(v, _)| *v != 0), "Lock Corner");
    let locked = smooth_points(&m, &all, 1., 1, true, false, None);
    let border = [0, 1, 2, 3, 5, 6, 7, 8];
    assert!(
        locked.iter().all(|(v, _)| !border.contains(v)),
        "Lock Boundary"
    );
    let free = smooth_points(&m, &all, 1., 1, false, false, None);
    assert!(
        free.iter().any(|(v, _)| *v == 0),
        "corner free without the lock"
    );
    // A radius limits it: only points near the centre.
    let near = falloff_weights(&m, DVec3::new(1., 0., 1.), 0.6, FalloffShape::Linear);
    assert_eq!(near.len(), 1);
}

#[test]
fn tp17_smoothing_along_a_loop_keeps_its_ends() {
    let mut m = grid(3, 1, 0.);
    m.vertices[2].z += 0.3; // (1, 0) pushed sideways along the border
    let lp = vec![(0, 2), (2, 4), (4, 6)];
    let w: Vec<(u32, f64)> = [0, 2, 4, 6].iter().map(|&v| (v, 1.)).collect();
    let moved = smooth_points(&m, &w, 1., 1, false, true, Some(&lp));
    assert!(moved.iter().all(|(v, _)| *v != 0 && *v != 6), "ends stay");
    let p = moved.iter().find(|(v, _)| *v == 2).unwrap().1;
    assert!(p.z < 0.3, "slides back toward its loop neighbours");
}

#[test]
fn tp30_weld_merges_and_drops_collapsed_polygons() {
    // Two quads side by side, not sharing points.
    let mut m = grid(1, 1, 0.1);
    let mut right = grid(1, 1, 0.1);
    for p in &mut right.vertices {
        p.x += 1.;
    }
    let base = m.vertices.len() as u32;
    m.vertices.extend(right.vertices.iter().copied());
    let mut faces = m.faces.to_vec();
    faces.push(face(
        right.faces[0].vertices.iter().map(|v| v + base).collect(),
    ));
    m.faces = faces.into();
    let (mut e, id) = scene_with(m);
    pen(
        &mut e,
        PenEdit::Weld {
            item: id,
            pairs: vec![(4, 2), (5, 3)],
            moves: Vec::new(),
            copy_uvs: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 6);
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.edges().len(), 7, "the seam is one shared edge");
    assert_eq!(e.history.undo.len(), 1);
    // Welding two points of one quad: it becomes a triangle.
    pen(
        &mut e,
        PenEdit::Weld {
            item: id,
            pairs: vec![(1, 0)],
            moves: vec![(1, DVec3::new(0., 0.1, 0.))],
            copy_uvs: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert!(m.faces.iter().any(|f| f.vertices.len() == 3));
}

#[test]
fn tp_move_points_is_one_step_and_skips_nothing_moved() {
    let (mut e, id) = scene_with(grid(1, 1, 0.1));
    pen(
        &mut e,
        PenEdit::MovePoints {
            item: id,
            points: vec![(0, DVec3::new(-0.2, 0.1, 0.))],
            label: "Topology Pen Smoothing".into(),
        },
    )
    .unwrap();
    assert_eq!(mesh(&e).vertices[0].x, -0.2);
    assert_eq!(e.history.undo.len(), 1);
    assert!(
        pen(
            &mut e,
            PenEdit::MovePoints {
                item: id,
                points: vec![(0, DVec3::new(-0.2, 0.1, 0.))],
                label: "x".into(),
            },
        )
        .is_err()
    );
    assert_eq!(e.history.undo.len(), 1, "no change, no undo step");
}

#[test]
fn tp_border_loop_walks_the_open_border() {
    let m = grid(2, 2, 0.);
    assert_eq!(border_loop(&m, 0, 1).len(), 8);
    assert!(is_border_edge(&m, 0, 1));
    assert!(!is_border_edge(&m, 3, 4));
    assert_eq!(duplicate_partner(&m, 0, DVec3::X), Some(3));
    assert_eq!(duplicate_partner(&m, 0, DVec3::Z), Some(1));
}

#[test]
fn edge_counts_follow_line_spans_while_pen_fill_counts_surface_sides() {
    let line = |closed| PolygonRepresentation::Polyline {
        closed,
        curve: curves::CurveKind::Line,
    };
    let mut m = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::Z, DVec3::X + DVec3::Z, DVec3::X],
        faces: vec![
            face(vec![0]),
            face(vec![0, 1]),
            face(vec![0, 1, 2]),
            face(vec![0, 1, 2]),
            face(vec![0, 1, 2, 3]),
        ]
        .into(),
        ..Default::default()
    };
    m.attributes.representations = vec![
        line(false),
        line(false),
        line(false),
        line(true),
        PolygonRepresentation::Face,
    ];
    m.validate().unwrap();
    let spans = [
        vec![],
        vec![(0, 1)],
        vec![(0, 1), (1, 2)],
        vec![(0, 1), (1, 2), (2, 0)],
        vec![(0, 1), (1, 2), (2, 3), (3, 0)],
    ];
    for (element, spans) in spans.into_iter().enumerate() {
        assert_eq!(m.element_spans(element).collect::<Vec<_>>(), spans);
    }
    let counts = [
        ((0, 1), 4),
        ((0, 2), 1),
        ((0, 3), 1),
        ((1, 2), 3),
        ((2, 3), 1),
    ];
    assert_eq!(*m.edge_counts(), counts.into_iter().collect());
    assert_eq!(m.edge_faces().get(&(0, 1)).unwrap(), &[4]);
    assert!(!m.edge_faces().contains_key(&(0, 2)));

    // Two lines occupying one side do not occupy either surface incidence.
    m.faces = vec![
        face(vec![0, 1]),
        face(vec![0, 1]),
        face(vec![2]),
        face(vec![3]),
    ]
    .into();
    m.attributes.representations = vec![line(false); 4];
    let (mut e, item) = scene_with(m);
    pen(
        &mut e,
        PenEdit::Fill {
            item,
            click: DVec3::new(0.5, 0., 0.5),
            toward_viewer: DVec3::Y,
            range: 2.,
            quad_only: true,
        },
    )
    .unwrap();
    assert_eq!(mesh(&e).edge_faces().get(&(0, 1)).unwrap(), &[2]);
}
