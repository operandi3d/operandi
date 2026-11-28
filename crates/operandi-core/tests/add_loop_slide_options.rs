//! Tool-audit Tier 1: Add Loop / Add Point placement options and the new
//! Slide options (provisional readings, TA1-Q01..Q06).
use glam::{DVec2, DVec3};
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "M".into(),
        material_label: None,
        smooth: false,
        part: None,
    }
}

/// The 3×3 grid "G3" of the measured specs: 16 points, 9 quads, point
/// `4 * y + x` at (x, y, 0).
fn g3() -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let mut faces = Vec::new();
    for y in 0..3u32 {
        for x in 0..3u32 {
            let v = 4 * y + x;
            faces.push(face(&[v, v + 1, v + 5, v + 4]));
        }
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Edge;
    e.synchronize_identities().unwrap();
    e
}

fn xs_of_new_points(before: &Mesh, after: &Mesh) -> Vec<f64> {
    let mut xs: Vec<f64> = after.vertices[before.vertices.len()..]
        .iter()
        .map(|p| (p.x * 1e6).round() / 1e6)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    xs
}

fn loop_with(t: f64, options: EdgeInsertOptions) -> Result<(Mesh, Vec<(u32, u32)>)> {
    // Edge 1–2 runs from x = 1 to x = 2 on the bottom row.
    add_loop_with(&g3(), 1, 2, t, &options)
}

#[test]
fn default_options_match_plain_add_loop() {
    let plain = add_loop(&g3(), 1, 2, 0.25).unwrap();
    let with = loop_with(0.25, EdgeInsertOptions::default()).unwrap();
    assert!(plain == with);
}

#[test]
fn both_sides_cuts_two_mirrored_loops_in_one_undo_step() {
    let mut e = editor(g3());
    let options = EdgeInsertOptions {
        both_sides: true,
        ..Default::default()
    };
    e.dispatch(Command::AddEdgeLoopWith {
        a: 1,
        b: 2,
        t: 0.25,
        options,
    })
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    m.validate().unwrap();
    assert_eq!((m.vertices.len(), m.faces.len()), (24, 15));
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
    assert_eq!(xs_of_new_points(&g3(), &m), vec![1.25, 1.75]);
    assert_eq!(e.scene.selection.edges.len(), 6, "both loops selected");
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh == g3());
}

#[test]
fn both_sides_at_the_middle_makes_one_loop() {
    // AL-05: Both Sides at the midpoint gives the plain 20/12 result.
    let options = EdgeInsertOptions {
        both_sides: true,
        ..Default::default()
    };
    let (m, edges) = loop_with(0.5, options).unwrap();
    assert_eq!((m.vertices.len(), m.faces.len()), (20, 12));
    assert_eq!(edges.len(), 3);
}

#[test]
fn absolute_distance_measures_from_either_end_and_clamps() {
    let at = |d1: f64, from_end: bool| {
        let options = EdgeInsertOptions {
            absolute: true,
            from_end,
            distance1: d1,
            ..Default::default()
        };
        // Position is ignored while Absolute Distance is on.
        let (m, _) = loop_with(0.9, options).unwrap();
        xs_of_new_points(&g3(), &m)
    };
    assert_eq!(at(0.2, false), vec![1.2]);
    assert_eq!(at(0.2, true), vec![1.8]);
    assert_eq!(at(5., false), vec![1.99], "clamped to the edge");
    let options = EdgeInsertOptions {
        absolute: true,
        both_sides: true,
        distance1: 0.2,
        distance2: 0.3,
        ..Default::default()
    };
    let (m, _) = loop_with(0.5, options).unwrap();
    assert_eq!(xs_of_new_points(&g3(), &m), vec![1.2, 1.7]);
    m.validate().unwrap();
}

#[test]
fn add_point_takes_absolute_distance() {
    let options = EdgeInsertOptions {
        absolute: true,
        distance1: 0.3,
        ..Default::default()
    };
    let (m, p) = add_point_with(&g3(), 1, 2, 0.5, &options).unwrap();
    assert!(m.vertices[p as usize].distance(DVec3::new(1.3, 0., 0.)) < 1e-12);
}

#[test]
fn world_distances_follow_item_scale() {
    let mut e = editor(g3());
    let id = e.scene.active().unwrap().id;
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .transform
        .scale = DVec3::splat(2.);
    let options = EdgeInsertOptions {
        absolute: true,
        distance1: 0.5,
        ..Default::default()
    };
    e.dispatch(Command::AddEdgePointWith {
        a: 1,
        b: 2,
        t: 0.5,
        options,
    })
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert!(
        (m.vertices[16].x - 1.25).abs() < 1e-12,
        "0.5 m world = 0.25 local"
    );
}

/// Two rows of four points on a quarter circle of radius 1 (z = 0 and 1):
/// three quads whose arc edges bend.
fn arc_strip() -> Mesh {
    let mut vertices = Vec::new();
    for z in [0., 1.] {
        for k in 0..4 {
            let a = std::f64::consts::FRAC_PI_6 * k as f64;
            vertices.push(DVec3::new(a.cos(), a.sin(), z));
        }
    }
    Mesh {
        vertices,
        faces: vec![
            face(&[0, 1, 5, 4]),
            face(&[1, 2, 6, 5]),
            face(&[2, 3, 7, 6]),
        ]
        .into(),
        ..Default::default()
    }
}

#[test]
fn preserve_curvature_follows_the_arc() {
    let straight = add_loop(&arc_strip(), 1, 2, 0.5).unwrap().0;
    let options = EdgeInsertOptions {
        preserve_curvature: true,
        ..Default::default()
    };
    let curved = add_loop_with(&arc_strip(), 1, 2, 0.5, &options).unwrap().0;
    let radius = |m: &Mesh| m.vertices[8].truncate().length();
    assert!((radius(&straight) - (15f64).to_radians().cos()).abs() < 1e-9);
    assert!((radius(&curved) - 1.).abs() < 0.005, "{}", radius(&curved));
    // Tension 0: flat tangents keep the point on the chord.
    let flat = EdgeInsertOptions {
        preserve_curvature: true,
        tension: 0.,
        ..Default::default()
    };
    let chord = add_loop_with(&arc_strip(), 1, 2, 0.5, &flat).unwrap().0;
    assert!(chord.vertices[8].distance(straight.vertices[8]) < 1e-12);
    // On an even flat grid the curve is the straight cut.
    let on_grid = loop_with(0.3, options).unwrap().0;
    let plain = loop_with(0.3, EdgeInsertOptions::default()).unwrap().0;
    for (a, b) in on_grid.vertices.iter().zip(&plain.vertices) {
        assert!(a.distance(*b) < 1e-12);
    }
}

#[test]
fn invalid_options_are_refused_without_change() {
    let mut e = editor(g3());
    let before = e.clone();
    for options in [
        EdgeInsertOptions {
            tension: f64::NAN,
            ..Default::default()
        },
        EdgeInsertOptions {
            absolute: true,
            distance1: -1.,
            ..Default::default()
        },
    ] {
        assert!(
            e.dispatch(Command::AddEdgeLoopWith {
                a: 1,
                b: 2,
                t: 0.5,
                options,
            })
            .is_err()
        );
        assert!(e == before);
    }
}

// --- Slide -----------------------------------------------------------------

/// The x = 1 column of G3 (edges 1–5, 5–9, 9–13).
const COLUMN: [(u32, u32); 3] = [(1, 5), (5, 9), (9, 13)];

fn column_x(m: &Mesh) -> Vec<f64> {
    [1usize, 5, 9, 13]
        .iter()
        .map(|&v| m.vertices[v].x)
        .collect()
}

#[test]
fn stop_negative_slide_clamps_to_the_rail() {
    let extras = SlideExtras {
        stop_negative: true,
        ..Default::default()
    };
    let (m, _) = edge_slide_extra(&g3(), &COLUMN, -0.5, false, None, &extras).unwrap();
    assert!(m == g3(), "a negative slide does nothing");
    // 1.5 stops at the neighbour and merges there (ES-04 counts).
    let (m, _) = edge_slide_extra(&g3(), &COLUMN, 1.5, false, None, &extras).unwrap();
    assert_eq!((m.vertices.len(), m.faces.len()), (12, 6));
    // Without it the measured ES-05 overshoot stays.
    let (m, _) =
        edge_slide_extra(&g3(), &COLUMN, 1.5, false, None, &SlideExtras::default()).unwrap();
    assert_eq!(m.vertices.len(), 16);
    assert!(column_x(&m).iter().all(|&x| (x + 0.5).abs() < 1e-9));
}

#[test]
fn slide_preserve_curvature_is_straight_on_an_even_grid() {
    let extras = SlideExtras {
        preserve_curvature: true,
        ..Default::default()
    };
    let (a, _) = edge_slide_extra(&g3(), &COLUMN, 0.3, false, None, &extras).unwrap();
    let (b, _) = edge_slide_with(&g3(), &COLUMN, 0.3, false).unwrap();
    for (p, q) in a.vertices.iter().zip(&b.vertices) {
        assert!(p.distance(*q) < 1e-12);
    }
}

/// G3 with a planar UV map: u = x / 3, v = y / 3.
fn g3_uv() -> Mesh {
    let mut m = g3();
    let rows: Vec<Vec<Option<DVec2>>> = m
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| Some(m.vertices[v as usize].truncate() / 3.))
                .collect()
        })
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    m
}

#[test]
fn slip_uvs_keeps_the_texture_still() {
    let extras = SlideExtras {
        slip_uvs: true,
        ..Default::default()
    };
    for amount in [0.4, -0.4] {
        let (m, _) = edge_slide_extra(&g3_uv(), &COLUMN, amount, false, None, &extras).unwrap();
        let map = &m.uv_maps["Texture"];
        for (fi, f) in m.faces.iter().enumerate() {
            for (i, &v) in f.vertices.iter().enumerate() {
                let want = m.vertices[v as usize].truncate() / 3.;
                let got = map[fi][i].unwrap();
                assert!(
                    got.distance(want) < 1e-9,
                    "face {fi} corner {i}: {got} vs {want}"
                );
            }
        }
    }
    // Off: the UVs stay where they were, so the texture moves.
    let (m, _) =
        edge_slide_extra(&g3_uv(), &COLUMN, 0.4, false, None, &SlideExtras::default()).unwrap();
    assert!(m.uv_maps == g3_uv().uv_maps);
}

#[test]
fn vertex_slide_loop_slide_off_passes_the_edge_end() {
    use operandi_core::vertex_commands::{SlideOptions, vertex_slide_extra};
    let options = SlideOptions {
        by_distance: false,
        merge: false,
    };
    // Point 5 (1,1) slides toward +X (to point 6).
    assert!(
        vertex_slide_extra(&g3(), &[5], DVec3::X, 1.5, options, &SlideExtras::default()).is_err()
    );
    let off = SlideExtras {
        loop_slide: false,
        ..Default::default()
    };
    let (m, _) = vertex_slide_extra(&g3(), &[5], DVec3::X, 1.5, options, &off).unwrap();
    assert!(m.vertices[5].distance(DVec3::new(2.5, 1., 0.)) < 1e-12);
    let stop = SlideExtras {
        stop_negative: true,
        loop_slide: false,
        ..Default::default()
    };
    let (m, _) = vertex_slide_extra(&g3(), &[5], DVec3::X, -0.5, options, &stop).unwrap();
    assert!(m == g3());
    let (m, _) = vertex_slide_extra(&g3(), &[5], DVec3::X, 1.5, options, &stop).unwrap();
    assert!(m.vertices[5].distance(DVec3::new(2., 1., 0.)) < 1e-12);
}

#[test]
fn slide_tool_uses_the_extras_from_parameters() {
    let mut e = editor(g3());
    e.scene.selection.edges = COLUMN.to_vec();
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = -0.5;
    p.slide_extras.stop_negative = true;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(column_x(&e.scene.active().unwrap().mesh), vec![1.; 4]);
}
