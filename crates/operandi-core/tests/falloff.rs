//! Falloff measured cases (specifications/falloff-2026-11-01/ORACLES.json).
//! Weights were measured as the displacement of a 1 m numeric Move; the
//! fixtures are small inline meshes from the oracle file.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

#[cfg(operandi_specs)]
fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/falloff-2026-11-01/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
#[cfg(operandi_specs)]
fn oracle(id: &str) -> Value {
    oracles()
        .into_iter()
        .find(|o| o["id"] == id)
        .unwrap_or_else(|| panic!("missing oracle {id}"))
}
fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}
fn numbers(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}
fn indices(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
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
fn editor_with(points: Vec<DVec3>, polygons: Vec<Vec<u32>>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: points,
        faces: polygons.into_iter().map(face).collect(),
        ..Default::default()
    }
    .into();
    e
}
fn editor(o: &Value) -> Editor {
    let input = &o["input"];
    editor_with(
        input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(indices)
            .collect(),
    )
}
fn shape(s: &Value) -> FalloffShape {
    match s["shape"].as_str().unwrap_or("linear") {
        "easeIn" => FalloffShape::EaseIn,
        "easeOut" => FalloffShape::EaseOut,
        "smooth" => FalloffShape::Smooth,
        "custom" => FalloffShape::Custom {
            ease_in: s["in"].as_f64().unwrap(),
            ease_out: s["out"].as_f64().unwrap(),
        },
        _ => FalloffShape::Linear,
    }
}
fn falloff(s: &Value) -> Falloff {
    if s["falloff"] == "radial" {
        RadialFalloff {
            center: vec3(&s["center"]),
            size: vec3(&s["size"]),
            shape: shape(s),
            world: false,
        }
        .into()
    } else {
        LinearFalloff {
            start: vec3(&s["start"]),
            end: vec3(&s["end"]),
            shape: shape(s),
            world: false,
            symmetric: match s["symmetric"].as_str() {
                Some("start") => FalloffSymmetric::Start,
                Some("end") => FalloffSymmetric::End,
                _ => FalloffSymmetric::None,
            },
        }
        .into()
    }
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-4, "{a:?} != {b:?}");
}
/// Positions compared as sorted lists, for results whose point order is
/// Operandi's own.
fn same_points(mut got: Vec<DVec3>, want: &Value) {
    let mut want: Vec<DVec3> = want.as_array().unwrap().iter().map(vec3).collect();
    let key = |a: &DVec3, b: &DVec3| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.z.total_cmp(&b.z))
    };
    let round = |v: &mut Vec<DVec3>| {
        for p in v.iter_mut() {
            *p = (*p * 1e6).round() / 1e6;
        }
        v.sort_by(key);
    };
    round(&mut got);
    round(&mut want);
    assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
    for (a, b) in got.iter().zip(&want) {
        close(*a, *b);
    }
}
fn move_all(e: &mut Editor, delta: DVec3) {
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices.clear();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
}

/// FAL-L-01..04, FAL-R-01..03: every measured weight, read back as the
/// displacement of a 1 m Move of the whole mesh.
#[test]
#[cfg(operandi_specs)]
fn fal_weight_oracles_through_move() {
    let mut checked = 0;
    for o in oracles() {
        let s = &o["settings"];
        let Some(delta) = s.get("move").map(vec3) else {
            continue;
        };
        let x = &o["expected"];
        let (key, axis) = if x.get("y").is_some() {
            ("y", 1)
        } else if x.get("dy").is_some() {
            ("dy", 1)
        } else {
            ("dx", 0)
        };
        let mut e = editor(&o);
        let before = points(&e);
        e.dispatch(Command::SpatialFalloff(Some(falloff(s))))
            .unwrap();
        move_all(&mut e, delta);
        for ((a, b), w) in before.iter().zip(points(&e)).zip(numbers(&x[key])) {
            let d = b - *a;
            assert!(
                (d[axis] - w).abs() < 1e-4,
                "{}: {a:?} moved {d:?}, weight {w}",
                o["id"]
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 18, "every weight oracle ran");
}

/// FAL-L-05: swapping Start and End reverses the weights.
#[test]
fn fal_l05_swapped_start_end() {
    let f = LinearFalloff {
        start: DVec3::X,
        end: DVec3::ZERO,
        shape: FalloffShape::EaseIn,
        ..Default::default()
    };
    assert!((f.weight(DVec3::new(0.05, 0., 0.)) - 0.0975).abs() < 1e-12);
}

/// FAL-M-01/03: one falloff at a time; each type keeps its settings.
#[test]
fn fal_m01_m03_one_at_a_time_and_kept_settings() {
    let mut e = editor_with(vec![DVec3::ZERO], vec![]);
    let radial = RadialFalloff {
        center: DVec3::ONE,
        size: DVec3::splat(2.),
        shape: FalloffShape::Smooth,
        world: false,
    };
    let linear = LinearFalloff {
        end: DVec3::Y * 3.,
        ..Default::default()
    };
    e.dispatch(Command::SpatialFalloff(Some(radial.into())))
        .unwrap();
    e.dispatch(Command::SpatialFalloff(Some(linear.into())))
        .unwrap();
    assert_eq!(e.spatial_falloff, Some(Falloff::Linear(linear)));
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    assert_eq!(e.radial_falloff_preset, radial);
    assert_eq!(e.linear_falloff_preset, linear);
    assert_eq!(
        FalloffShape::CUSTOM.weight(0.5),
        FalloffShape::Custom {
            ease_in: 0.,
            ease_out: 1.
        }
        .weight(0.5)
    );
}

/// 7×7-point grid, x and y from -3 to 3; polygons covering x -1…2,
/// y -1…1 selected (FAL-M-04).
fn grid() -> Editor {
    let mut points = Vec::new();
    for j in 0..7 {
        for i in 0..7 {
            points.push(DVec3::new(i as f64 - 3., j as f64 - 3., 0.));
        }
    }
    let mut polygons = Vec::new();
    for j in 0..6 {
        for i in 0..6 {
            let a = j * 7 + i;
            polygons.push(vec![a, a + 1, a + 8, a + 7]);
        }
    }
    let mut e = editor_with(points, polygons);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    // Cells with x in -1…2 (columns 2..5) and y in -1…1 (rows 2..4).
    e.scene.selection.faces = (2..4)
        .flat_map(|j| (2..5).map(move |i| j * 6 + i))
        .collect();
    e
}

/// FAL-M-04/05: Linear Auto Size per axis; Z with no depth changes
/// nothing; Reverse swaps Start and End.
#[test]
fn fal_m04_m05_linear_auto_size_and_reverse() {
    let e = grid();
    let f = LinearFalloff::default();
    let x = f.auto_size(&e.scene, 0).unwrap();
    close(x.start, DVec3::new(-1., 0., 0.));
    close(x.end, DVec3::new(2., 0., 0.));
    let y = f.auto_size(&e.scene, 1).unwrap();
    close(y.start, DVec3::new(0.5, -1., 0.));
    close(y.end, DVec3::new(0.5, 1., 0.));
    assert_eq!(y.auto_size(&e.scene, 2).unwrap(), y);
    let r = y.reversed();
    close(r.start, DVec3::new(0.5, 1., 0.));
    close(r.end, DVec3::new(0.5, -1., 0.));
}

/// FAL-M-06: Radial Auto Size from the selection, or the whole mesh.
#[test]
fn fal_m06_radial_auto_size() {
    let mut e = grid();
    let r = RadialFalloff::default().auto_size(&e.scene).unwrap();
    close(r.center, DVec3::new(0.5, 0., 0.));
    close(r.size, DVec3::new(1.5, 1., 0.));
    e.scene.selection.faces.clear();
    let r = RadialFalloff::default().auto_size(&e.scene).unwrap();
    close(r.center, DVec3::ZERO);
    close(r.size, DVec3::new(3., 3., 0.));
}

fn strip_falloff() -> Falloff {
    LinearFalloff {
        start: DVec3::new(-1., 0., 0.),
        end: DVec3::X,
        ..Default::default()
    }
    .into()
}

/// FAL-T-01: Polygon Bevel scales Shift and Inset per corner by its weight;
/// counts don't change, even where the weight is 0.
#[test]
#[cfg(operandi_specs)]
fn fal_t01_polygon_bevel() {
    for (id, shift, inset) in [
        ("FAL-O-poly.bevel_inset0_shift1", 1., 0.),
        ("FAL-O-poly.bevel_inset0.2_shift0", 0., 0.2),
    ] {
        let o = oracle(id);
        let mut e = editor(&o);
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.scene.selection.faces = vec![0, 1];
        e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
        e.dispatch(Command::SpatialFalloff(Some(strip_falloff())))
            .unwrap();
        let p = Parameters {
            shift,
            inset,
            ..e.session.live.as_ref().unwrap().params.clone()
        };
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        let n = numbers(&o["expected"]["n"]);
        assert_eq!(
            (mesh.vertices.len(), mesh.faces.len()),
            (n[0] as usize, n[1] as usize),
            "{id}"
        );
        same_points(points(&e), &o["expected"]["points"]);
    }
}

/// FAL-T-02: Edge Bevel's width at each end is scaled by the weight there
/// (0.5: half the width). The measured case (FAL-O-09) bevels an edge whose
/// ends are on an open border, which Operandi's Edge Bevel doesn't support
/// yet; this checks the same rule on a closed box's top-front edge, where a
/// 0.2 bevel at weight 0.5 must equal an unweighted 0.1 bevel.
#[test]
fn fal_t02_edge_bevel_scaled_width() {
    let bevel = |width: f64, falloff: Option<Falloff>| {
        let mut e = editor_with(vec![], vec![]);
        e.scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
        let m = &e.scene.active().unwrap().mesh;
        let edge = m
            .edges()
            .into_iter()
            .find(|&(a, b)| {
                let (p, q) = (m.vertices[a as usize], m.vertices[b as usize]);
                p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
            })
            .unwrap();
        e.dispatch(Command::Mode(Mode::Edge)).unwrap();
        e.scene.selection.edges = vec![edge];
        e.dispatch(Command::SpatialFalloff(falloff)).unwrap();
        e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.chamfer.offset = width;
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        points(&e)
    };
    // Weight 0.5 at y = 0.5, where both ends of the edge lie.
    let half = LinearFalloff {
        start: DVec3::new(0., -0.5, 0.),
        end: DVec3::new(0., 1.5, 0.),
        ..Default::default()
    };
    let weighted = bevel(0.2, Some(half.into()));
    let plain = bevel(0.1, None);
    assert_eq!(weighted.len(), plain.len());
    for (a, b) in weighted.iter().zip(&plain) {
        close(*a, *b);
    }
    assert_ne!(bevel(0.2, None), plain);
}

/// FAL-T-03: Vertex Bevel's inset is scaled by the point's weight. The
/// measured case bevels boundary points, which Operandi's Vertex Bevel
/// doesn't support yet, so this checks the same rule on an interior point
/// of weight 0.5: cut points at 0.1 instead of 0.2.
#[test]
fn fal_t03_vertex_bevel_interior_point() {
    let mut points = Vec::new();
    for j in 0..3 {
        for i in 0..3 {
            points.push(DVec3::new(i as f64 - 1., j as f64 - 1., 0.));
        }
    }
    let polygons = vec![
        vec![0, 1, 4, 3],
        vec![1, 2, 5, 4],
        vec![3, 4, 7, 6],
        vec![4, 5, 8, 7],
    ];
    let mut e = editor_with(points, polygons);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![4];
    e.dispatch(Command::SpatialFalloff(Some(strip_falloff())))
        .unwrap();
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    let p = Parameters {
        inset: 0.2,
        ..e.session.live.as_ref().unwrap().params.clone()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    let cuts: Vec<DVec3> = mesh.vertices[mesh.vertices.len() - 4..].to_vec();
    for c in cuts {
        assert!((c.length() - 0.1).abs() < 1e-9, "{c:?}");
    }
}

/// FAL-T-04 is not built: Extrude's topology rule under a falloff is open
/// (FAL-Q04), so Extrude refuses an active falloff and changes nothing.
#[test]
#[cfg(operandi_specs)]
fn fal_t04_extrude_refuses_falloff() {
    let o = oracle("FAL-O-08");
    let mut e = editor(&o);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::SpatialFalloff(Some(strip_falloff())))
        .unwrap();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Activate(Tool::Extrude)).is_err());
    assert_eq!(e.scene, before);
}

/// Rotate and Scale take the weighted share too; Cancel and Undo restore
/// the scene; a live Move re-applies when the falloff changes.
#[test]
#[cfg(operandi_specs)]
fn weighted_transforms_cancel_undo_and_live_change() {
    let o = oracle("FAL-O-linear");
    let mut e = editor(&o);
    let original = e.scene.clone();
    // A ramp along X (the factory End is 0,1,0, FO-40).
    e.dispatch(Command::SpatialFalloff(Some(
        LinearFalloff {
            end: DVec3::X,
            ..Default::default()
        }
        .into(),
    )))
    .unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    let start = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::splat(3.),
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, start);

    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..Default::default()
    }))
    .unwrap();
    close(points(&e)[10], DVec3::new(0.5, 0.5, 0.));
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    close(points(&e)[10], DVec3::new(0.5, 1., 0.));
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active(), original.active());
}

/// FAL-B2-02/03/04 (FAL-O-F2-03-*, FAL-O-F2-04*): a weighted Rotate or Scale
/// moves each point to p + w·(T(p) − p); with X Symmetry the weight is read
/// at the point's positive-side position.
#[test]
#[cfg(operandi_specs)]
fn fal_b2_rotate_scale_blend_and_symmetry_sampling() {
    let mut checked = 0;
    for o in oracles() {
        let id = o["id"].as_str().unwrap();
        if !(id.starts_with("FAL-O-F2-03") || id.starts_with("FAL-O-F2-04")) {
            continue;
        }
        let s = &o["settings"];
        let mut e = editor(&o);
        if s["Symmetry"] == "X" {
            e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
        }
        e.dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::World,
        }))
        .unwrap();
        e.dispatch(Command::SpatialFalloff(Some(
            LinearFalloff {
                start: vec3(&s["Start m"]),
                end: vec3(&s["End m"]),
                ..Default::default()
            }
            .into(),
        )))
        .unwrap();
        e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
        e.scene.selection.vertices = indices(&o["selection"]["sel_vertices"]);
        let (tool, delta) = match s["tool"].as_str().unwrap() {
            "Rotate" => (
                Tool::Rotate,
                DVec3::new(0., 0., s["Rotation Z degrees"].as_f64().unwrap()),
            ),
            "Scale" => (Tool::Scale, DVec3::splat(s["Scale XYZ"].as_f64().unwrap())),
            _ => (Tool::Move, vec3(&s["Move m"])),
        };
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Preview(Parameters {
            delta,
            ..Default::default()
        }))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
        for (got, want) in points(&e)
            .iter()
            .zip(o["expected"]["points"].as_array().unwrap())
        {
            assert!(
                (*got - vec3(want)).length() < 1e-5,
                "{id}: {got:?} vs {want:?}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 6);
}

/// FAL-B3-04 (FAL-O-F3-edge-slide-x*): Edge Slide scales each point's slide
/// by its weight (−0.25, −0.125 and 0 at weights 1, 0.5 and 0).
#[test]
#[cfg(operandi_specs)]
fn fal_b3_04_edge_slide_scales_by_weight() {
    for x in [1, 2, 3] {
        let control = oracle(&format!("FAL-O-F3-edge-slide-x{x}-control"));
        let linear = oracle(&format!("FAL-O-F3-edge-slide-x{x}-linear"));
        let e = editor(&control);
        let mesh = e.scene.active().unwrap().mesh.to_owned_value();
        let edge = indices(&control["selection"]["sel_edges"][0]);
        let edges = [(edge[0].min(edge[1]), edge[0].max(edge[1]))];
        let want = |o: &Value| -> Vec<DVec3> {
            o["expected"]["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(vec3)
                .collect()
        };
        // Operandi's side sign: whichever reproduces the no-falloff control.
        let amount = [0.25, -0.25]
            .into_iter()
            .find(|&a| {
                let (m, _) = edge_slide_with(&mesh, &edges, a, false).unwrap();
                m.vertices
                    .iter()
                    .zip(want(&control))
                    .all(|(p, q)| p.distance(q) < 1e-5)
            })
            .expect("a sign reproduces the control");
        let field: Falloff = LinearFalloff {
            start: vec3(&linear["settings"]["Start m"]),
            end: vec3(&linear["settings"]["End m"]),
            ..Default::default()
        }
        .into();
        let weights: Vec<f64> = mesh.vertices.iter().map(|&v| field.weight(v)).collect();
        let (m, _) = edge_slide_weighted(&mesh, &edges, amount, false, Some(&weights)).unwrap();
        for (p, q) in m.vertices.iter().zip(want(&linear)) {
            close(*p, q);
        }
    }
}

/// FAL-B3-02 on an interior point (the measured case uses border points,
/// which Operandi's Vertex Extrude doesn't support): weight 0.5 halves the
/// Extrude and the Width; weight 0 leaves the point alone.
#[test]
fn fal_b3_02_vertex_extrude_scales_by_weight() {
    let mut points = Vec::new();
    for j in 0..3 {
        for i in 0..3 {
            points.push(DVec3::new(i as f64 - 1., j as f64 - 1., 0.));
        }
    }
    let polygons = vec![
        vec![0, 1, 4, 3],
        vec![1, 2, 5, 4],
        vec![3, 4, 7, 6],
        vec![4, 5, 8, 7],
    ];
    let e = editor_with(points, polygons);
    let mesh = e.scene.active().unwrap().mesh.to_owned_value();
    let half = vec![0.5; 9];
    let (out, _) =
        vertex_commands::vertex_extrude_weighted(&mesh, &[4], 1., 0.2, Some(&half)).unwrap();
    assert!((out.vertices[4].z.abs() - 0.5).abs() < 1e-9);
    let zero = vec![0.; 9];
    let (out, _) =
        vertex_commands::vertex_extrude_weighted(&mesh, &[4], 1., 0.2, Some(&zero)).unwrap();
    assert_eq!(out, mesh);
}

/// FAL-B3-03 (FAL-O-F3-spike-*): a weighted Spike's tip height is the mean
/// of its face's corner weights; faces of weight 0 stay quads.
#[test]
#[cfg(operandi_specs)]
fn fal_b3_03_spike_scales_by_mean_corner_weight() {
    for id in [
        "FAL-O-F3-spike-control",
        "FAL-O-F3-spike-linear",
        "FAL-O-F3-spike-centers-linear",
    ] {
        let o = oracle(id);
        let e = editor(&o);
        let mesh = e.scene.active().unwrap().mesh.to_owned_value();
        let s = &o["settings"];
        let weights: Option<Vec<f64>> = (s["falloff"] == "Linear").then(|| {
            let field: Falloff = LinearFalloff {
                start: vec3(&s["Start m"]),
                end: vec3(&s["End m"]),
                ..Default::default()
            }
            .into();
            mesh.vertices.iter().map(|&v| field.weight(v)).collect()
        });
        let result = spike::spike_geometry_weighted(&mesh, &[], 1., weights.as_deref()).unwrap();
        let x = &o["expected"];
        assert_eq!(
            result.mesh.faces.len(),
            x["polygons"].as_array().unwrap().len(),
            "{id}"
        );
        same_points(result.mesh.vertices.clone(), &x["points"]);
    }
}
