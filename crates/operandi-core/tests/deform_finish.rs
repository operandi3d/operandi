//! Deform finish (modelling parity batch 1): Jitter, Quantize, Radial Align,
//! Linear Align and Relax Mesh against the probe cases (DEFORM-PROBES
//! DP-xx, deform-cases.json). Case numbers are the probe's; meshes are
//! rebuilt here from their points.
use glam::DVec3;
use operandi_core::deform_finish::*;
use operandi_core::*;
use std::collections::BTreeSet;

fn mesh(points: &[[f64; 3]], polygons: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|p| DVec3::from_array(*p)).collect(),
        faces: polygons
            .iter()
            .map(|f| Face {
                vertices: f.to_vec(),
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

fn assert_points(got: &Mesh, want: &[[f64; 3]], tolerance: f64, case: &str) {
    assert_eq!(got.vertices.len(), want.len(), "{case}");
    for (i, (g, w)) in got.vertices.iter().zip(want).enumerate() {
        let w = DVec3::from_array(*w);
        assert!(
            g.distance(w) <= tolerance * 3f64.sqrt(),
            "{case} point {i}: got {g:?}, want {w:?}"
        );
    }
}

fn all(m: &Mesh) -> BTreeSet<u32> {
    (0..m.vertices.len() as u32).collect()
}

// ---- Quantize (DP-05) ----

const QZ_IN: [[f64; 3]; 4] = [
    [0.13, 0.41, -0.07],
    [0.62, -0.38, 0.88],
    [-0.9, 0.12, 0.26],
    [1.37, 0.74, -1.12],
];

#[test]
fn dp_qz_1_quantize_snaps_to_the_nearest_step_and_zero_leaves_an_axis() {
    let m = mesh(&QZ_IN, &[&[0, 1, 2, 3]]);
    let o = QuantizeOptions {
        step: DVec3::new(0.25, 0.25, 0.),
    };
    let r = quantize_geometry(&m, &all(&m), &o, glam::DMat4::IDENTITY, None).unwrap();
    assert_points(
        &r,
        &[
            [0.25, 0.5, -0.07],
            [0.5, -0.5, 0.88],
            [-1.0, 0.0, 0.26],
            [1.25, 0.75, -1.12],
        ],
        1e-5,
        "DP-QZ-1",
    );
}

#[test]
fn dp_qz_2_quantize_per_axis_steps() {
    let m = mesh(&QZ_IN, &[&[0, 1, 2, 3]]);
    let o = QuantizeOptions {
        step: DVec3::new(0.3, 0.5, 1.0),
    };
    let r = quantize_geometry(&m, &all(&m), &o, glam::DMat4::IDENTITY, None).unwrap();
    assert_points(
        &r,
        &[
            [0.0, 0.5, 0.0],
            [0.6, -0.5, 1.0],
            [-0.9, 0.0, 0.0],
            [1.5, 0.5, -1.0],
        ],
        1e-5,
        "DP-QZ-2",
    );
}

#[test]
fn quantize_measures_from_the_world_origin_and_blends_by_weight() {
    let m = mesh(&[[0.1, 0., 0.]], &[]);
    // The item sits at x = 0.3: world 0.4 snaps to 0.5, local 0.2.
    let world = glam::DMat4::from_translation(DVec3::new(0.3, 0., 0.));
    let o = QuantizeOptions {
        step: DVec3::new(0.5, 0., 0.),
    };
    let r = quantize_geometry(&m, &all(&m), &o, world, None).unwrap();
    assert!((r.vertices[0].x - 0.2).abs() < 1e-12);
    let half = quantize_geometry(&m, &all(&m), &o, world, Some(&[0.5])).unwrap();
    assert!((half.vertices[0].x - 0.15).abs() < 1e-12);
    assert!(
        quantize_geometry(
            &m,
            &all(&m),
            &QuantizeOptions {
                step: DVec3::new(-1., 0., 0.)
            },
            world,
            None
        )
        .is_err()
    );
}

// ---- Jitter (DP-01..04) ----

fn grid(n: u32) -> Mesh {
    let mut pts = Vec::new();
    for z in 0..=n {
        for x in 0..=n {
            pts.push([x as f64, 0., z as f64]);
        }
    }
    let at = |x: u32, z: u32| z * (n + 1) + x;
    let faces: Vec<Vec<u32>> = (0..n)
        .flat_map(|z| {
            (0..n).map(move |x| vec![at(x, z), at(x, z + 1), at(x + 1, z + 1), at(x + 1, z)])
        })
        .collect();
    let refs: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
    mesh(&pts, &refs)
}

#[test]
fn dp_01_jitter_stays_within_plus_minus_range_and_range_zero_is_no_change() {
    let m = grid(4);
    let o = JitterOptions {
        range: DVec3::new(0.1, 0.2, 0.3),
        ..Default::default()
    };
    let r = jitter_geometry(&m, &all(&m), &o, None).unwrap();
    let mut most = DVec3::ZERO;
    for (a, b) in m.vertices.iter().zip(&r.vertices) {
        let d = (*b - *a).abs();
        assert!(d.x <= 0.1 && d.y <= 0.2 && d.z <= 0.3, "{d:?}");
        most = most.max(d);
    }
    // 25 points use most of the range.
    assert!(most.x > 0.07 && most.y > 0.14 && most.z > 0.21, "{most:?}");
    let zero = jitter_geometry(&m, &all(&m), &JitterOptions::default(), None).unwrap();
    assert_eq!(zero.vertices, m.vertices);
    assert_eq!(r.faces, m.faces);
}

#[test]
fn dp_02_same_seed_same_result_and_another_seed_differs() {
    let m = grid(3);
    let o = JitterOptions {
        range: DVec3::splat(0.2),
        seed: 666,
        ..Default::default()
    };
    let a = jitter_geometry(&m, &all(&m), &o, None).unwrap();
    let b = jitter_geometry(&m, &all(&m), &o, None).unwrap();
    assert_eq!(a.vertices, b.vertices);
    let c = jitter_geometry(&m, &all(&m), &JitterOptions { seed: 667, ..o }, None).unwrap();
    assert_ne!(a.vertices, c.vertices);
}

#[test]
fn dp_03_switching_an_axis_off_leaves_the_others_unchanged() {
    let m = grid(3);
    let on = JitterOptions {
        range: DVec3::splat(0.2),
        ..Default::default()
    };
    let off = JitterOptions {
        enable: [true, true, false],
        ..on
    };
    let a = jitter_geometry(&m, &all(&m), &on, None).unwrap();
    let b = jitter_geometry(&m, &all(&m), &off, None).unwrap();
    for ((p, q), base) in a.vertices.iter().zip(&b.vertices).zip(&m.vertices) {
        assert_eq!(p.x, q.x);
        assert_eq!(p.y, q.y);
        assert_eq!(q.z, base.z);
    }
}

#[test]
fn dp_04_rigid_translate_moves_each_piece_as_a_block() {
    // Two separate quads.
    let m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 0., 1.],
            [0., 0., 1.],
            [5., 0., 0.],
            [6., 0., 0.],
            [6., 0., 1.],
            [5., 0., 1.],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    );
    let o = JitterOptions {
        range: DVec3::splat(0.5),
        rigid_translate: true,
        ..Default::default()
    };
    let r = jitter_geometry(&m, &all(&m), &o, None).unwrap();
    let d = |v: usize| r.vertices[v] - m.vertices[v];
    for v in 1..4 {
        assert!(d(v).distance(d(0)) < 1e-12);
    }
    for v in 5..8 {
        assert!(d(v).distance(d(4)) < 1e-12);
    }
    assert!(d(0).distance(d(4)) > 1e-6, "pieces get different offsets");
    // Rigid Rotate keeps each piece's shape.
    let rot = jitter_geometry(
        &m,
        &all(&m),
        &JitterOptions {
            rigid_translate: false,
            rigid_rotate: true,
            ..o
        },
        None,
    )
    .unwrap();
    assert!((rot.vertices[0].distance(rot.vertices[2]) - 2f64.sqrt()).abs() < 1e-9);
    assert_ne!(rot.vertices, m.vertices);
}

#[test]
fn jitter_scales_by_falloff_weight() {
    let m = grid(2);
    let o = JitterOptions {
        range: DVec3::splat(0.3),
        ..Default::default()
    };
    let w = vec![0.; 9];
    let r = jitter_geometry(&m, &all(&m), &o, Some(&w)).unwrap();
    assert_eq!(r.vertices, m.vertices);
}

// ---- Radial Align (DP-10..15) ----

const RA_IN: [[f64; 3]; 8] = [
    [1.441871, 0.0, 0.146371],
    [1.071188, 0.0, 0.651396],
    [0.585347, 0.0, 0.618034],
    [-0.761108, 0.0, 0.458559],
    [-0.585868, 0.0, -0.328672],
    [-0.387914, 0.0, -0.52],
    [0.479654, 0.0, -1.412829],
    [0.937433, 0.0, -0.597685],
];
const RA_EDGES: [(u32, u32); 8] = [
    (0, 1),
    (0, 7),
    (1, 2),
    (2, 3),
    (3, 4),
    (4, 5),
    (5, 6),
    (6, 7),
];

fn radial_case(o: RadialAlignOptions) -> Mesh {
    let m = mesh(&RA_IN, &[&[0, 1, 2, 3, 4, 5, 6, 7]]);
    let runs = runs_from_edges(&RA_EDGES);
    assert_eq!(runs.len(), 1);
    assert!(runs[0].closed);
    radial_align_geometry(&m, &runs, &[], &o, None).unwrap()
}

#[test]
fn dp_ra_1_circle_spaces_the_loop_evenly_on_the_measured_circle() {
    let r = radial_case(RadialAlignOptions::default());
    assert_points(
        &r,
        &[
            [1.350128, 0.0, -0.020434],
            [0.983889, 0.0, 0.658407],
            [0.244906, 0.0, 0.879449],
            [-0.433935, 0.0, 0.51321],
            [-0.654978, 0.0, -0.225773],
            [-0.288738, 0.0, -0.904614],
            [0.450245, 0.0, -1.125656],
            [1.129086, 0.0, -0.759417],
        ],
        1e-4,
        "DP-RA-1",
    );
}

#[test]
fn dp_ra_2_n_sided_puts_points_on_the_corners_and_sides() {
    let r = radial_case(RadialAlignOptions {
        mode: RadialMode::NSided,
        sides: 5,
        ..Default::default()
    });
    assert_points(
        &r,
        &[
            [0.8538, 0.0, 0.748327],
            [0.264514, 0.0, 0.687979],
            [-0.324772, 0.0, 0.627631],
            [-0.449477, 0.0, 0.048539],
            [-0.574182, 0.0, -0.530554],
            [-0.061969, 0.0, -0.828105],
            [0.450245, 0.0, -1.125656],
            [1.332786, 0.0, -0.335265],
        ],
        1e-4,
        "DP-RA-2",
    );
}

#[test]
fn dp_ra_3_weight_half_lands_halfway() {
    let r = radial_case(RadialAlignOptions {
        weight: 0.5,
        ..Default::default()
    });
    assert_points(
        &r,
        &[
            [1.395999, 0.0, 0.062968],
            [1.027539, 0.0, 0.654902],
            [0.415126, 0.0, 0.748742],
            [-0.597521, 0.0, 0.485884],
            [-0.620422, 0.0, -0.277223],
            [-0.338326, 0.0, -0.712307],
            [0.464949, 0.0, -1.269243],
            [1.033259, 0.0, -0.678551],
        ],
        1e-4,
        "DP-RA-3",
    );
    let none = radial_case(RadialAlignOptions {
        weight: 0.,
        ..Default::default()
    });
    assert_points(&none, &RA_IN, 1e-12, "weight 0");
}

#[test]
fn radial_align_polygon_mode_aligns_the_border_and_relaxes_inside() {
    let mut m = grid(4);
    m.vertices[12].y = 0.4; // the middle point, bumped
    let mut sel = Selection {
        mode: Mode::Polygon,
        ..Default::default()
    };
    sel.faces = (0..16).collect();
    let (runs, inner) = align_runs(&m, &sel);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].points.len(), 16);
    assert_eq!(inner.len(), 9);
    let r = radial_align_geometry(&m, &runs, &inner, &RadialAlignOptions::default(), None).unwrap();
    let c = DVec3::new(2., 0., 2.);
    let radius: Vec<f64> = runs[0]
        .points
        .iter()
        .map(|&v| r.vertices[v as usize].distance(c))
        .collect();
    for d in &radius {
        assert!((d - radius[0]).abs() < 1e-9, "border on one circle");
    }
    // Flatten puts the bump back on the plane.
    assert!(r.vertices[12].y.abs() < 1e-9);
    assert!(
        radial_align_geometry(&m, &[], &[], &RadialAlignOptions::default(), None).is_err(),
        "nothing to align"
    );
}

// ---- Linear Align (DP-16..18) ----

const LA_IN: [[f64; 3]; 10] = [
    [0.0, 0.0, 0.0],
    [0.7, 0.3, 0.0],
    [1.1, -0.2, 0.0],
    [2.4, 0.25, 0.0],
    [3.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.7, 0.3, 1.0],
    [1.1, -0.2, 1.0],
    [2.4, 0.25, 1.0],
    [3.0, 0.0, 1.0],
];

fn linear_case(uniform: bool) -> Mesh {
    let m = mesh(
        &LA_IN,
        &[&[0, 1, 6, 5], &[1, 2, 7, 6], &[2, 3, 8, 7], &[3, 4, 9, 8]],
    );
    let runs = runs_from_edges(&[(0, 1), (1, 2), (2, 3), (3, 4)]);
    let o = LinearAlignOptions {
        uniform,
        ..Default::default()
    };
    linear_align_geometry(&m, &runs, &o, None).unwrap()
}

#[test]
fn dp_la_1_uniform_spaces_points_evenly_between_the_ends() {
    let mut want = LA_IN;
    for (i, p) in want.iter_mut().take(5).enumerate() {
        *p = [0.75 * i as f64, 0., 0.];
    }
    assert_points(&linear_case(true), &want, 1e-5, "DP-LA-1");
}

#[test]
fn dp_la_2_uniform_off_projects_onto_the_line() {
    let mut want = LA_IN;
    for p in want.iter_mut().take(5) {
        p[1] = 0.;
    }
    assert_points(&linear_case(false), &want, 1e-5, "DP-LA-2");
}

#[test]
fn linear_align_refuses_closed_loops_and_resizes_runs() {
    let m = mesh(&RA_IN, &[&[0, 1, 2, 3, 4, 5, 6, 7]]);
    let loops = runs_from_edges(&RA_EDGES);
    assert!(linear_align_geometry(&m, &loops, &LinearAlignOptions::default(), None).is_err());
    let line = mesh(&[[0., 0., 0.], [1., 0.3, 0.], [2., 0., 0.]], &[]);
    let runs = runs_from_edges(&[(0, 1), (1, 2)]);
    let o = LinearAlignOptions {
        align_length: true,
        edge_length: 4.,
        ..Default::default()
    };
    let r = linear_align_geometry(&line, &runs, &o, None).unwrap();
    assert!(r.vertices[2].distance(DVec3::new(4., 0., 0.)) < 1e-12);
    assert!(r.vertices[1].distance(DVec3::new(2., 0., 0.)) < 1e-12);
    // Screen Align keeps depth along the axis.
    let deep = mesh(&[[0., 0., 0.], [1., 0.3, 0.5], [2., 0., 0.]], &[]);
    let o = LinearAlignOptions {
        screen_align: true,
        axis: DVec3::Z,
        ..Default::default()
    };
    let r = linear_align_geometry(&deep, &runs, &o, None).unwrap();
    assert!(r.vertices[1].distance(DVec3::new(1., 0., 0.5)) < 1e-12);
}

// ---- Relax Mesh (DP-20..22) ----

const RM_IN: [[f64; 3]; 25] = [
    [-1.0, -1.0, 0.011745],
    [-0.5, -1.0, 0.083021],
    [0.0, -1.0, 0.0],
    [0.5, -1.0, -0.083021],
    [1.0, -1.0, -0.011745],
    [-1.0, -0.5, -0.015249],
    [-0.356161, -0.39469, -0.10779],
    [-0.149624, -0.549938, 0.0],
    [0.636395, -0.525295, 0.10779],
    [1.0, -0.5, 0.015249],
    [-1.0, 0.0, -0.028224],
    [-0.447383, -0.096137, -0.199499],
    [0.0, 0.12, 0.0],
    [0.447383, -0.096137, 0.199499],
    [1.0, 0.0, 0.028224],
    [-1.0, 0.5, -0.015249],
    [-0.636395, 0.474705, -0.10779],
    [0.149624, 0.450062, 0.0],
    [0.356161, 0.60531, 0.10779],
    [1.0, 0.5, 0.015249],
    [-1.0, 1.0, 0.011745],
    [-0.5, 1.0, 0.083021],
    [0.0, 1.0, 0.0],
    [0.5, 1.0, -0.083021],
    [1.0, 1.0, -0.011745],
];

fn relax_case(constrain: bool) -> Mesh {
    let at = |x: u32, y: u32| y * 5 + x;
    let faces: Vec<Vec<u32>> = (0..4)
        .flat_map(|y| {
            (0..4).map(move |x| vec![at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)])
        })
        .collect();
    let refs: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
    let m = mesh(&RM_IN, &refs);
    relax_mesh_geometry(
        &m,
        &all(&m),
        &RelaxMeshOptions {
            iterations: 5,
            constrain,
        },
    )
    .unwrap()
}

#[test]
fn dp_rm_1_constrain_off_is_five_gentle_passes() {
    let mut want = RM_IN;
    let inner: [(usize, [f64; 3]); 9] = [
        (6, [-0.460012, -0.476754, -0.047282]),
        (7, [-0.026119, -0.50406, 0.002404]),
        (8, [0.530035, -0.518322, 0.04791]),
        (11, [-0.483605, -0.01908, -0.092669]),
        (12, [0.001827, 0.020226, 0.004014]),
        (13, [0.477898, -0.018868, 0.093757]),
        (16, [-0.528208, 0.482648, -0.043896]),
        (17, [0.026721, 0.496415, 0.002618]),
        (18, [0.45666, 0.52672, 0.044632]),
    ];
    for (i, p) in inner {
        want[i] = p;
    }
    // DP-20: the border never moves (exactly).
    let r = relax_case(false);
    for v in [0, 1, 2, 3, 4, 5, 9, 10, 14, 15, 19, 20, 21, 22, 23, 24] {
        assert_eq!(r.vertices[v], DVec3::from_array(RM_IN[v]));
    }
    assert_points(&r, &want, 0.02, "DP-RM-1");
}

#[test]
fn dp_rm_2_constrain_on_settles_into_an_even_grid() {
    let mut want = RM_IN;
    let inner: [(usize, [f64; 3]); 9] = [
        (6, [-0.501163, -0.499708, 0.021138]),
        (7, [-0.001274, -0.499639, 0.003903]),
        (8, [0.499346, -0.499807, -0.01538]),
        (11, [-0.501258, 0.000367, 0.005489]),
        (12, [-0.0013, 0.000388, 0.003919]),
        (13, [0.499345, 0.000197, 0.000348]),
        (16, [-0.500646, 0.500196, 0.019299]),
        (17, [-0.000653, 0.500197, 0.001962]),
        (18, [0.499673, 0.500099, -0.016365]),
    ];
    for (i, p) in inner {
        want[i] = p;
    }
    assert_points(&relax_case(true), &want, 0.02, "DP-RM-2");
}

// ---- Editor workflow: activate, preview, undo, cancel, symmetry ----

fn grid_editor(n: u32) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    // Centred on the origin so Symmetry X pairs every point.
    let mut m = grid(n);
    for v in &mut m.vertices {
        *v -= DVec3::new(n as f64 / 2., 0., n as f64 / 2.);
    }
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.mode = Mode::Vertex;
    e.history.undo.clear();
    e
}
fn pts(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}

#[test]
fn jitter_tool_previews_commits_one_step_and_cancel_restores() {
    let mut e = grid_editor(4);
    let before = pts(&e);
    e.dispatch(Command::Activate(Tool::Jitter)).unwrap();
    // DP starting values: Range 0 changes nothing; the button's Seed 666.
    assert_eq!(live(&e).jitter.seed, JITTER_BUTTON_SEED);
    assert_eq!(pts(&e), before);
    let mut p = live(&e);
    p.jitter.range = DVec3::splat(0.2);
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let jittered = pts(&e);
    assert_ne!(jittered, before);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Jitter");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(pts(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(pts(&e), jittered);
    // Cancel leaves the mesh as it was.
    e.dispatch(Command::Activate(Tool::Jitter)).unwrap();
    let mut p = live(&e);
    p.jitter.range = DVec3::splat(0.5);
    p.jitter.seed = 1;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(pts(&e), jittered);
}

#[test]
fn jitter_with_symmetry_mirrors_the_other_side() {
    let mut e = grid_editor(4);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Activate(Tool::Jitter)).unwrap();
    let mut p = live(&e);
    p.jitter.range = DVec3::splat(0.1);
    e.dispatch(Command::Preview(p)).unwrap();
    let base = grid_editor(4);
    let (b, a) = (pts(&base), pts(&e));
    for (i, q) in b.iter().enumerate() {
        if q.x.abs() < 1e-9 {
            continue;
        }
        let j = b
            .iter()
            .position(|o| (o.x + q.x).abs() < 1e-9 && (o.z - q.z).abs() < 1e-9)
            .unwrap();
        let mirrored = DVec3::new(-a[j].x, a[j].y, a[j].z);
        assert!(a[i].distance(mirrored) < 1e-6, "point {i}");
    }
}

#[test]
fn quantize_and_align_tools_refuse_item_mode_and_empty_runs() {
    let mut e = grid_editor(2);
    e.scene.selection.mode = Mode::Item;
    assert!(e.dispatch(Command::Activate(Tool::Quantize)).is_err());
    e.scene.selection.mode = Mode::Edge;
    // No edges selected: nothing to align.
    assert!(e.dispatch(Command::Activate(Tool::LinearAlign)).is_err());
    assert!(e.session.live.is_none());
    e.scene.selection.edges = vec![(0, 1), (1, 2)];
    e.dispatch(Command::Activate(Tool::LinearAlign)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn radial_align_tool_makes_a_polygon_border_round_in_one_step() {
    let mut e = grid_editor(4);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = (0..16).collect();
    e.dispatch(Command::Activate(Tool::RadialAlign)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 1);
    let m = &e.scene.active().unwrap().mesh;
    let corner = m.vertices[0].length();
    let side = m.vertices[2].length();
    assert!((corner - side).abs() < 1e-9, "border points on one circle");
}

#[test]
fn relax_mesh_is_one_step_keeps_borders_and_refuses_item_mode() {
    let mut e = grid_editor(4);
    let before = pts(&e);
    let mut bumped = before.clone();
    bumped[12].y = 0.5;
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: bumped.clone(),
        ..(*e.scene.active().unwrap().mesh).clone()
    }
    .into();
    e.dispatch(Command::RelaxMesh(RelaxMeshOptions::default()))
        .unwrap();
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Relax Mesh");
    let after = pts(&e);
    assert!(after[12].y < 0.5);
    assert_eq!(after[0], bumped[0]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(pts(&e), bumped);
    e.scene.selection.mode = Mode::Item;
    assert!(
        e.dispatch(Command::RelaxMesh(RelaxMeshOptions::default()))
            .is_err()
    );
    assert!(
        e.dispatch(Command::RelaxMesh(RelaxMeshOptions {
            iterations: 0,
            constrain: false
        }))
        .is_err()
    );
}

// ---- Preset rows ----

#[test]
fn dp_vx_1_vortex_turns_each_point_by_weight_times_angle() {
    // The probe's 7 x 7 grid in XZ, cylinder falloff along Y, rotation
    // about Z through the origin.
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut points = Vec::new();
    for z in 0..7 {
        for x in 0..7 {
            points.push([-1.5 + 0.5 * x as f64, 0., -1.5 + 0.5 * z as f64]);
        }
    }
    e.scene.active_mut().unwrap().mesh = mesh(&points, &[]).into();
    e.scene.selection.mode = Mode::Vertex;
    e.dispatch(Command::ActivatePreset("xfrm.vortex".into()))
        .unwrap();
    assert_eq!(e.live_tool_label(), Some("Vortex"));
    let Some(Falloff::Cylinder(c)) = e.spatial_falloff else {
        panic!("Vortex uses a Cylinder falloff");
    };
    assert_eq!(c.axis, 1, "axis Y");
    // The probe's falloff: centre 0, size 1.
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Cylinder(
        CylinderFalloff {
            center: DVec3::ZERO,
            size: DVec3::ONE,
            ..c
        },
    ))))
    .unwrap();
    e.dispatch(Command::ToolClick {
        point: DVec3::ZERO,
        plane: 2,
    })
    .ok();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 0., 90.),
        ..live(&e)
    }))
    .unwrap();
    let got = pts(&e);
    // DP-VX-1 points that move: (∓0.5, 0, ∓0.5) and (∓0.5, 0, 0) rows.
    let want = [
        ([-0.5, 0., -0.5], [-0.448009, -0.222008, -0.5]),
        ([0.5, 0., -0.5], [0.448009, 0.222008, -0.5]),
        ([-0.5, 0., 0.], [-0.353553, -0.353553, 0.]),
        ([0.5, 0., 0.], [0.353553, 0.353553, 0.]),
        ([-1., 0., 0.], [-1., 0., 0.]),
        ([0., 0., 0.], [0., 0., 0.]),
    ];
    for (from, to) in want {
        let i = points.iter().position(|p| *p == from).unwrap();
        assert!(
            got[i].distance(DVec3::from_array(to)) < 1e-5,
            "{from:?}: got {}, want {to:?}",
            got[i]
        );
    }
    e.dispatch(Command::Drop).unwrap();
    assert!(
        e.spatial_falloff.is_none(),
        "the preset's falloff goes back"
    );
}

#[test]
fn soft_selection_soft_drag_and_element_push_rows_put_their_layers_in_place() {
    for (id, tool, check) in [
        ("SoftSelectionMove", Tool::Move, 0),
        ("SoftSelectionRotate", Tool::Rotate, 0),
        ("SoftSelectionScale", Tool::Scale, 0),
        ("SoftSelectionTransform", Tool::Transform, 0),
        ("xfrm.softDrag", Tool::Move, 1),
        ("ElementPush", Tool::Push, 2),
    ] {
        let mut e = grid_editor(2);
        e.scene.selection.vertices = vec![4];
        e.dispatch(Command::ActivatePreset(id.into())).unwrap();
        // Armed rows wait for the click (CS-01); either way the layers are on.
        if let Some(l) = &e.session.live {
            assert_eq!(l.tool, tool, "{id}");
        }
        match (check, e.spatial_falloff) {
            (0, Some(Falloff::SoftSelection(s))) => {
                assert_eq!(s, SoftSelectionFalloff::default(), "{id}");
                assert_eq!(e.action_context.center, CenterMode::Selection, "{id}");
            }
            (1, Some(Falloff::Screen(s))) => assert_eq!(s.size, 20.),
            (2, Some(Falloff::Element(_))) => {}
            (_, f) => panic!("{id}: {f:?}"),
        }
        e.dispatch(Command::Drop).unwrap();
        assert!(e.spatial_falloff.is_none(), "{id} gives the falloff back");
        assert_ne!(e.action_context.center, CenterMode::Selection, "{id}");
    }
}

#[test]
fn soft_selection_move_moves_the_selection_rigidly_and_fades_beyond() {
    let mut e = grid_editor(4);
    e.scene.selection.vertices = vec![12];
    e.dispatch(Command::ActivatePreset("SoftSelectionMove".into()))
        .unwrap();
    e.dispatch(Command::SpatialFalloff(Some(Falloff::SoftSelection(
        SoftSelectionFalloff {
            radius: 1.5,
            ..Default::default()
        },
    ))))
    .unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 1., 0.),
        ..live(&e)
    }))
    .unwrap();
    let p = pts(&e);
    assert!((p[12].y - 1.).abs() < 1e-9, "selection moves fully");
    assert!(p[13].y > 0. && p[13].y < 1., "neighbour fades");
    assert_eq!(p[0].y, 0., "far corner stays");
}

// ---- Mesh Stack ----

#[test]
fn the_deform_ops_run_the_tools_functions() {
    use operandi_core::mesh_stack_deform::*;
    use operandi_core::meshop_types::{OpContext, op_def};
    let m = grid(3);
    let ctx = OpContext {
        input: &m,
        op: 1,
        selection: None,
        element: Default::default(),
        links: &[],
        time: Default::default(),
        notes: Default::default(),
        falloff: None,
    };
    let def = op_def(JITTER_OP).expect("Jitter is in the op table");
    // TYPE-IDS: the op's Seed starts at 137.
    assert_eq!(def.defaults()["seed"], 137);
    let mut v = def.defaults();
    v["rangeX"] = 0.2.into();
    let r = def.run(&ctx, &v).unwrap();
    let direct = jitter_geometry(
        &m,
        &all(&m),
        &JitterOptions {
            range: DVec3::new(0.2, 0., 0.),
            seed: 137,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(r.vertices, direct.vertices);
    for id in [QUANTIZE_OP, RADIAL_ALIGN_OP, LINEAR_ALIGN_OP, RELAX_MESH_OP] {
        let def = op_def(id).unwrap_or_else(|| panic!("{id} in the table"));
        let out = def.run(&ctx, &def.defaults());
        // Linear Align on a closed border refuses; the rest run.
        assert_eq!(out.is_ok(), id != LINEAR_ALIGN_OP, "{id}: {out:?}");
    }
}

// ---- Hands-on batch (2026-11-17): Element Push, Soft Selection ----

#[test]
fn element_push_moves_a_polygon_flat_along_its_normal_and_range_spreads_it() {
    let mut e = grid_editor(4);
    e.scene.selection.mode = Mode::Polygon;
    e.dispatch(Command::ActivatePreset("ElementPush".into()))
        .unwrap();
    let item = e.scene.active().unwrap().id;
    // The middle-ish polygon 5 (grid lies in XZ facing ±Y).
    e.dispatch(Command::FalloffElementPick(Some(TweakGrab {
        item,
        element: TweakElement::Polygon(5),
    })))
    .unwrap();
    let base = pts(&e);
    let normal = e.scene.active().unwrap().mesh.normal(5);
    e.dispatch(Command::Preview(Parameters {
        shift: 0.5,
        ..live(&e)
    }))
    .unwrap();
    let after = pts(&e);
    let corners = e.scene.active().unwrap().mesh.faces[5].vertices.clone();
    for (i, (a, b)) in base.iter().zip(&after).enumerate() {
        if corners.contains(&(i as u32)) {
            assert!(
                (*b - *a - normal * 0.5).length() < 1e-9,
                "corner {i} moves flat"
            );
        } else {
            assert_eq!(a, b, "Range 0: only the polygon moves");
        }
    }
    // Range 1.5: neighbours follow along the same direction, less.
    let Some(Falloff::Element(f)) = e.spatial_falloff else {
        panic!("Element falloff");
    };
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Element(
        ElementFalloff { range: 1.5, ..f },
    ))))
    .unwrap();
    e.dispatch(Command::Preview(Parameters {
        shift: 0.5,
        ..live(&e)
    }))
    .unwrap();
    let spread = pts(&e);
    let moved: Vec<usize> = (0..base.len())
        .filter(|&i| !corners.contains(&(i as u32)) && spread[i] != base[i])
        .collect();
    assert!(!moved.is_empty());
    for i in moved {
        let d = spread[i] - base[i];
        assert!(d.normalize().dot(normal).abs() > 0.999, "along the normal");
        assert!(d.length() < 0.5);
    }
}

#[test]
fn soft_selection_connectivity_keeps_disconnected_parts_still() {
    // Two strips 0.2 apart; select one point near the gap.
    let mut points = Vec::new();
    for z in [0., 1.2] {
        for x in 0..3 {
            points.push([x as f64, 0., z]);
            points.push([x as f64, 0., z + 1.]);
        }
    }
    let f = |o: u32| vec![o, o + 1, o + 3, o + 2];
    let quads = [f(0), f(2), f(6), f(8)];
    let m = mesh(&points, &[&quads[0], &quads[1], &quads[2], &quads[3]]);
    for (connectivity, other_moves) in [(true, false), (false, true)] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = m.clone().into();
        e.scene.selection.mode = Mode::Vertex;
        e.scene.selection.vertices = vec![3]; // (1, 0, 1), next to the gap
        e.dispatch(Command::ActivatePreset("SoftSelectionMove".into()))
            .unwrap();
        // The handle's axes follow the selection (hands-on).
        assert_eq!(e.action_context.axis, AxisOrientation::Selection);
        e.dispatch(Command::SpatialFalloff(Some(Falloff::SoftSelection(
            SoftSelectionFalloff {
                radius: 1.,
                connectivity,
                ..Default::default()
            },
        ))))
        .unwrap();
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::new(0., 1., 0.),
            ..live(&e)
        }))
        .unwrap();
        let p = pts(&e);
        assert!((p[3].y - 1.).abs() < 1e-9);
        // Point 8 is (1, 0, 1.2): 0.2 away in a straight line.
        assert_eq!(p[8].y > 1e-9, other_moves, "connectivity {connectivity}");
        if other_moves {
            assert!((p[8].y - 0.8).abs() < 1e-9, "1 − d/R");
        }
    }
}
