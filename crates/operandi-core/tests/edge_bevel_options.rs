//! Edge Bevel / Edge Chamfer options (specifications/edge-bevel-chamfer-
//! 2026-11-02): one test per option, plus corner patches, Edge End, and the
//! editor lifecycle (retained options, undo, cancel, save/reopen).
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;

fn cube() -> Mesh {
    // The spec's Box (README): faces -Y, +Y, -Z, +X, +Z, -X.
    let v = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let f = [
        [0, 1, 2, 3],
        [7, 6, 5, 4],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [3, 7, 4, 0],
    ];
    Mesh {
        vertices: v.iter().map(|&p| DVec3::from_array(p)).collect(),
        faces: f
            .iter()
            .map(|r| Face {
                vertices: r.to_vec(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
}

fn settings(width: f64, rows: u32, chamfer: bool, options: EdgeBevelOptions) -> EdgeBevelSettings {
    EdgeBevelSettings {
        width,
        rows,
        depth: 100.,
        chamfer,
        options,
    }
}

fn has_point(m: &Mesh, p: [f64; 3]) -> bool {
    let p = DVec3::from_array(p);
    m.faces
        .iter()
        .flat_map(|f| &f.vertices)
        .any(|&v| m.vertices[v as usize].distance(p) < 1e-5)
}

#[cfg(operandi_specs)]
fn oracle(id: &str) -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/edge-bevel-chamfer-2026-11-02/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap()
        .clone()
}

fn input_mesh(o: &Value) -> Mesh {
    let n = |v: &Value| v.as_f64().unwrap();
    Mesh {
        vertices: o["input"]["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| DVec3::new(n(&p[0]), n(&p[1]), n(&p[2])))
            .collect(),
        faces: o["input"]["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: p
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap() as u32)
                    .collect(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
}

/// Polygon sizes inside the corner cube X, Y, Z > 0.35 (EBC-B3-04).
fn corner_patch(m: &Mesh) -> Vec<usize> {
    let mut sizes: Vec<usize> = m
        .faces
        .iter()
        .filter(|f| {
            f.vertices
                .iter()
                .all(|&v| m.vertices[v as usize].min_element() > 0.35)
        })
        .map(|f| f.vertices.len())
        .collect();
    sizes.sort();
    sizes
}

const CORNER3: [(u32, u32); 3] = [(6, 7), (5, 6), (2, 6)];

#[test]
fn width_mode_reads_the_width_across_the_corner() {
    // EBC-P-04: 0.1 Width on the 90° box edge moves each side 0.141421
    // (Edge Bevel) or 0.070711 (Edge Chamfer) along its face.
    let width = EdgeBevelOptions {
        mode: EdgeBevelMode::Width,
        ..Default::default()
    };
    for (chamfer, side) in [(false, 0.141421), (true, 0.070711)] {
        let out = edge_bevel_with(
            &cube(),
            &[(6, 7)],
            &settings(0.1, 1, chamfer, width.clone()),
            None,
        )
        .unwrap()
        .mesh;
        assert!(has_point(&out, [0.5, 0.5 - side, 0.5]), "{chamfer}");
        assert!(has_point(&out, [0.5, 0.5, 0.5 - side]), "{chamfer}");
        // Inset mode moves 0.1.
        let inset = edge_bevel_with(
            &cube(),
            &[(6, 7)],
            &settings(0.1, 1, chamfer, Default::default()),
            None,
        )
        .unwrap()
        .mesh;
        assert!(has_point(&inset, [0.5, 0.4, 0.5]));
    }
}

#[test]
fn square_and_sharp_keep_the_original_corner() {
    // EBC-P-02 / EBC-B3-08: two flat segments through the original edge,
    // whatever the level; Edge Chamfer closes each end with a triangle and
    // selects the original edge.
    for shape in [EdgeBevelShape::Square, EdgeBevelShape::Sharp] {
        for rows in [2, 4, 6] {
            let o = EdgeBevelOptions {
                shape,
                ..Default::default()
            };
            let r =
                edge_bevel_with(&cube(), &[(6, 7)], &settings(0.1, rows, false, o), None).unwrap();
            assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (12, 8));
            assert!(has_point(&r.mesh, [0.5, 0.5, 0.5]));
        }
    }
    let o = EdgeBevelOptions {
        shape: EdgeBevelShape::Square,
        ..Default::default()
    };
    let r = edge_bevel_with(&cube(), &[(6, 7)], &settings(0.1, 3, true, o.clone()), None).unwrap();
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (12, 10));
    let triangles = r
        .mesh
        .faces
        .iter()
        .filter(|f| f.vertices.len() == 3)
        .count();
    assert_eq!(triangles, 2);
    let [(a, b)] = r.selected_after[..] else {
        panic!("{:?}", r.selected_after)
    };
    let (pa, pb) = (r.mesh.vertices[a as usize], r.mesh.vertices[b as usize]);
    assert!(pa.y == 0.5 && pa.z == 0.5 && pb.y == 0.5 && pb.z == 0.5);
    // EBS-03 (EBC-Q12 v1): one segment keeps the corner too, and Depth
    // doesn't flatten it.
    for chamfer in [false, true] {
        let one = settings(0.1, 1, chamfer, o.clone());
        let r = edge_bevel_with(&cube(), &[(6, 7)], &one, None).unwrap();
        let faces = if chamfer { 10 } else { 8 };
        assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (12, faces));
        assert!(has_point(&r.mesh, [0.5, 0.5, 0.5]));
        let shallow = EdgeBevelSettings { depth: 0., ..one };
        let r = edge_bevel_with(&cube(), &[(6, 7)], &shallow, None).unwrap();
        assert!(has_point(&r.mesh, [0.5, 0.5, 0.5]));
    }
}

#[test]
fn corner_patches_follow_the_measured_layouts() {
    // EBC-B3-04: 3 / 4 / 6 rows at a box corner.
    let case = |rows: u32, chamfer: bool| {
        edge_bevel_with(
            &cube(),
            &CORNER3,
            &settings(0.1, rows, chamfer, Default::default()),
            None,
        )
        .unwrap()
        .mesh
    };
    let three = case(3, true);
    assert_eq!((three.vertices.len(), three.faces.len()), (28, 22));
    assert_eq!(corner_patch(&three), [[3].as_slice(), &[4; 6]].concat());
    for (rows, chamfer, counts, quads) in [
        (2, true, (20, 15), 3),
        (4, true, (38, 30), 12),
        (6, true, (62, 51), 27),
        (6, false, (62, 51), 27),
    ] {
        let m = case(rows, chamfer);
        assert_eq!((m.vertices.len(), m.faces.len()), counts, "{rows}");
        assert_eq!(corner_patch(&m), vec![4; quads], "{rows}");
        // The even-row centre point (EBC-B3-04).
        assert!(has_point(&m, [0.460948; 3]), "{rows}");
    }
}

#[test]
fn even_patch_centre_uv_is_the_corners_mean() {
    // EBC-B3-05: with U = X/3, V = Y/3 the centre's UV is (0.144444,
    // 0.144444) in every polygon around it, not its displaced position's.
    let mut m = cube();
    let uvs: Vec<Vec<Option<DVec2>>> = m
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    let p = m.vertices[v as usize];
                    Some(DVec2::new(p.x / 3., p.y / 3.))
                })
                .collect()
        })
        .collect();
    m.uv_maps.insert("Texture".into(), uvs);
    for (rows, chamfer) in [(2, true), (4, true), (6, false)] {
        let out = edge_bevel_with(
            &m,
            &CORNER3,
            &settings(0.1, rows, chamfer, Default::default()),
            None,
        )
        .unwrap()
        .mesh;
        let map = &out.uv_maps["Texture"];
        let mut seen = 0;
        for (fi, f) in out.faces.iter().enumerate() {
            for (c, &v) in f.vertices.iter().enumerate() {
                if out.vertices[v as usize].distance(DVec3::splat(0.460948)) < 1e-5 {
                    let uv = map[fi][c].unwrap();
                    assert!(uv.distance(DVec2::splat(0.144444)) < 1e-5, "{rows}: {uv}");
                    seen += 1;
                }
            }
        }
        assert!(seen >= 3, "{rows}");
    }
}

#[test]
#[cfg(operandi_specs)]
fn edges_ending_on_a_border_leave_it_open() {
    // EBC-B3-06: G3 edge 1–5 and row 4–5–6–7.
    let o = oracle("EBC-O-E3-06-one-bevel");
    let g3 = input_mesh(&o);
    let r = edge_bevel_with(
        &g3,
        &[(1, 5)],
        &settings(0.1, 1, false, Default::default()),
        None,
    )
    .unwrap();
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (19, 11));
    assert!(has_point(&r.mesh, [0.9, 0., 0.]) && has_point(&r.mesh, [1.1, 0., 0.]));
    let row = [(4, 5), (5, 6), (6, 7)];
    for chamfer in [false, true] {
        let r = edge_bevel_with(
            &g3,
            &row,
            &settings(0.1, 1, chamfer, Default::default()),
            None,
        )
        .unwrap();
        assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (20, 12));
    }
    // Edge Chamfer keeps the interior end point; Operandi joins the
    // reference's zero-area end triangles into the strip (logged departure).
    let r = edge_bevel_with(
        &g3,
        &[(1, 5)],
        &settings(0.1, 1, true, Default::default()),
        None,
    )
    .unwrap();
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (19, 10));
    assert!(has_point(&r.mesh, [1., 1., 0.]));
    assert!(r.mesh.faces.iter().any(|f| {
        f.vertices.len() == 5
            && f.vertices
                .iter()
                .any(|&v| r.mesh.vertices[v as usize] == DVec3::new(1., 1., 0.))
    }));
}

#[test]
#[cfg(operandi_specs)]
fn edge_end_triangle_fan_slides_the_end_and_fans_it() {
    // EBC-B3-07: planar T, Segments 3.
    let o = oracle("EBC-O-E3-07-triangleFan");
    let g3 = input_mesh(&o);
    let t = [(4, 5), (5, 6), (5, 9)];
    let fan = EdgeBevelOptions {
        edge_end: EdgeEnd::TriangleFan,
        ..Default::default()
    };
    let r = edge_bevel_with(&g3, &t, &settings(0.1, 3, true, fan), None).unwrap();
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (38, 31));
    assert!(has_point(&r.mesh, [2.1, 1., 0.]) && has_point(&r.mesh, [1., 2.1, 0.]));
    let triangles = r
        .mesh
        .faces
        .iter()
        .filter(|f| f.vertices.len() == 3)
        .count();
    assert_eq!(triangles, 7);
    // Profile keeps the end points where they were.
    let r = edge_bevel_with(&g3, &t, &settings(0.1, 3, true, Default::default()), None).unwrap();
    assert!(has_point(&r.mesh, [2., 1., 0.]) && !has_point(&r.mesh, [2.1, 1., 0.]));
    r.mesh.validate().unwrap();
}

#[test]
fn use_material_names_only_the_new_polygons() {
    let o = EdgeBevelOptions {
        use_material: true,
        material: "Trim".into(),
        ..Default::default()
    };
    for chamfer in [false, true] {
        let out = edge_bevel_with(
            &cube(),
            &CORNER3,
            &settings(0.1, 2, chamfer, o.clone()),
            None,
        )
        .unwrap()
        .mesh;
        let named = out.faces.iter().filter(|f| f.material == "Trim").count();
        // 3 strips × 2 rows + 3 corner quads; the six sides keep theirs.
        assert_eq!(named, 9);
        assert!(out.faces[..6].iter().all(|f| f.material.is_empty()));
    }
    let empty = EdgeBevelOptions {
        use_material: true,
        material: " ".into(),
        ..Default::default()
    };
    assert!(edge_bevel_with(&cube(), &[(6, 7)], &settings(0.1, 1, false, empty), None).is_err());
}

#[test]
fn parallel_off_moves_the_offset_along_the_sides() {
    // A slanted side: Parallel keeps the new edge 0.1 from the old one; off,
    // the point moves 0.1 along the side edge, like Edge Bevel's Inset.
    let mut m = cube();
    m.vertices[2].x = 1.;
    m.vertices[6].x = 1.;
    let edge = [(6, 7)];
    let bevel = edge_bevel_with(
        &m,
        &edge,
        &settings(0.1, 1, false, Default::default()),
        None,
    )
    .unwrap();
    let parallel =
        edge_bevel_with(&m, &edge, &settings(0.1, 1, true, Default::default()), None).unwrap();
    let off = EdgeBevelOptions {
        parallel: false,
        ..Default::default()
    };
    let along = edge_bevel_with(&m, &edge, &settings(0.1, 1, true, off), None).unwrap();
    assert_eq!(along.mesh.vertices, bevel.mesh.vertices);
    assert_ne!(along.mesh.vertices, parallel.mesh.vertices);
}

#[test]
fn stop_at_edge_and_auto_weld() {
    // A box whose front face is a trapezoid (side edges 1 m at +X, 2 m at
    // −X) and whose top is 3 m deep. An Offset of 1.25 passes the short side
    // edge: with Stop At Edge alone that's refused (the polygons would
    // fold); with Auto Weld the point stops on the far corner and merges
    // with it; with Stop At Edge off it runs past.
    let mut m = cube();
    m.vertices[3].y = -1.5;
    m.vertices[4].z = -2.5;
    m.vertices[5].z = -2.5;
    m.validate().unwrap();
    let edge = [(6, 7)];
    let err = edge_bevel_with(
        &m,
        &edge,
        &settings(1.25, 1, true, Default::default()),
        None,
    )
    .unwrap_err();
    assert!(err.contains("Auto Weld"), "{err}");
    let past = EdgeBevelOptions {
        stop_at_edge: false,
        ..Default::default()
    };
    let off = edge_bevel_with(&m, &edge, &settings(1.25, 1, true, past), None).unwrap();
    assert!(has_point(&off.mesh, [0.5, -0.75, 0.5]));
    let weld = EdgeBevelOptions {
        auto_weld: true,
        ..Default::default()
    };
    let welded = edge_bevel_with(&m, &edge, &settings(1.25, 1, true, weld.clone()), None).unwrap();
    assert_eq!(welded.mesh.vertices.len(), 9);
    assert!(has_point(&welded.mesh, [0.5, -0.5, 0.5]));
    welded.mesh.validate().unwrap();
    // EBC-P-05: nothing to weld at a normal Offset.
    let plain = edge_bevel_with(&cube(), &edge, &settings(0.1, 1, true, weld), None).unwrap();
    assert_eq!(plain.mesh.vertices.len(), 10);
}

#[test]
fn options_are_kept_between_uses_and_drop_undo_cancel_and_save_agree() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube().into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(6, 7)];
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::EdgeChamfer)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    assert_eq!(p.edge_bevel, EdgeBevelOptions::default());
    p.chamfer.offset = 0.1;
    p.chamfer.segments = 3;
    p.edge_bevel.shape = EdgeBevelShape::Square;
    p.edge_bevel.use_material = true;
    p.edge_bevel.material = "Trim".into();
    e.dispatch(Command::Preview(p.clone())).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = e.scene.clone();
    // Square Edge Chamfer selects the original edge.
    assert_eq!(after.selection.edges.len(), 1);
    let mesh = &after.active().unwrap().mesh;
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (12, 10));
    // Save and reopen keep the geometry, materials and selection.
    let reopened = persistence::decode(&persistence::encode(&after).unwrap()).unwrap();
    assert_eq!(reopened.active().unwrap().mesh.faces, mesh.faces);
    assert_eq!(reopened.selection.edges, after.selection.edges);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    // Next use: options and Segments kept, width back to zero.
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Activate(Tool::EdgeChamfer)).unwrap();
    let again = e.session.live.as_ref().unwrap().params.clone();
    assert_eq!(again.edge_bevel, p.edge_bevel);
    assert_eq!(again.chamfer.segments, 3);
    assert_eq!(again.chamfer.offset, 0.);
    // Cancel leaves the scene as it was.
    let mut q = again.clone();
    q.chamfer.offset = 0.2;
    e.dispatch(Command::Preview(q)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
    // Edge Bevel keeps its own options.
    e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
    assert_eq!(
        e.session.live.as_ref().unwrap().params.edge_bevel,
        EdgeBevelOptions::default()
    );
}

#[test]
fn invalid_requests_change_nothing() {
    let c = cube();
    let s = settings(0.1, 1, false, Default::default());
    assert!(edge_bevel_with(&c, &[], &s, None).is_err());
    assert!(edge_bevel_with(&c, &[(6, 99)], &s, None).is_err());
    assert!(
        edge_bevel_with(
            &c,
            &[(6, 7)],
            &settings(-1., 1, false, Default::default()),
            None
        )
        .is_err()
    );
    assert!(
        edge_bevel_with(
            &c,
            &[(6, 7)],
            &settings(0.1, 0, false, Default::default()),
            None
        )
        .is_err()
    );
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = c.into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    // TA-06 (tool-activation v1): with nothing selected Edge Bevel works on
    // every edge, so it turns on (it used to be refused); switching on
    // changes nothing.
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn auto_weld_heals_a_collapsed_side() {
    // Offset past both 1 m side edges: without Auto Weld the two sides
    // collapse to nothing and it's refused; with it, they're removed.
    let edge = [(6, 7)];
    assert!(
        edge_bevel_with(
            &cube(),
            &edge,
            &settings(1.2, 1, true, Default::default()),
            None
        )
        .is_err()
    );
    let weld = EdgeBevelOptions {
        auto_weld: true,
        ..Default::default()
    };
    let r = edge_bevel_with(&cube(), &edge, &settings(1.2, 1, true, weld), None).unwrap();
    r.mesh.validate().unwrap();
    assert_eq!(r.mesh.vertices.len(), 6);
}

#[test]
fn an_end_at_a_bend_keeps_its_point() {
    // Two quads on top, two in front, folded along the edge f0–f1–f2.
    // Beveling f1–l1 would slide f1 back along f1–b1 and fold a triangle
    // across the bend, so the end point stays put and the strip's two new
    // edges close back onto it (N08; it used to be refused, EBC-Q11). On a
    // flat grid the same end slides (EBC-B3-06).
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    let face = |v: [u32; 4]| Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    // b0 b1 b2 (back top), f0 f1 f2 (fold), l0 l1 l2 (front bottom).
    let fold = Mesh {
        vertices: vec![
            p(0., 1., -1.),
            p(1., 1., -1.),
            p(2., 1., -1.),
            p(0., 1., 0.),
            p(1., 1., 0.),
            p(2., 1., 0.),
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(2., 0., 0.),
        ],
        faces: vec![
            face([0, 1, 4, 3]),
            face([1, 2, 5, 4]),
            face([3, 4, 7, 6]),
            face([4, 5, 8, 7]),
        ]
        .into(),
        ..Default::default()
    };
    fold.validate().unwrap();
    for (chamfer, edge_end) in [(false, EdgeEnd::default()), (true, EdgeEnd::TriangleFan)] {
        let options = EdgeBevelOptions {
            edge_end,
            ..Default::default()
        };
        let r =
            edge_bevel_with(&fold, &[(4, 7)], &settings(0.1, 1, chamfer, options), None).unwrap();
        r.mesh.validate().unwrap();
        // f1 is kept where it was, and still used.
        assert!(has_point(&r.mesh, [1., 1., 0.]));
        assert!(r.mesh.faces.len() > fold.faces.len());
    }
    // N05: a gentle fold (30°, as on a curved organic surface) bevels and
    // stays valid; only the sharp one is refused.
    let mut gentle = fold.clone();
    for v in &mut gentle.vertices {
        if v.y < 0.5 {
            // The front quads tilt to 30° below the top's plane.
            let a = 30f64.to_radians();
            *v = DVec3::new(v.x, 1. - a.sin(), a.cos());
        }
    }
    for chamfer in [false, true] {
        let r = edge_bevel_with(
            &gentle,
            &[(4, 7)],
            &settings(0.1, 1, chamfer, Default::default()),
            None,
        )
        .unwrap();
        r.mesh.validate().unwrap();
    }
    // Flattened, the same selection works.
    let mut flat = fold.clone();
    for v in &mut flat.vertices {
        if v.y < 0.5 {
            *v = DVec3::new(v.x, 1., 1.);
        }
    }
    assert!(
        edge_bevel_with(
            &flat,
            &[(4, 7)],
            &settings(0.1, 1, false, Default::default()),
            None
        )
        .is_ok()
    );
}

/// N08: a strip down the middle of a surface folded sharply at both of its
/// ends (as on a face's centre line from brow to lip) bevels, both end
/// points kept.
#[test]
fn a_strip_between_two_sharp_folds_bevels() {
    let p = |x: f64, y: usize| match y {
        0 => DVec3::new(x, 1., 1.),
        4 => DVec3::new(x, 3., 1.),
        y => DVec3::new(x, y as f64, 0.),
    };
    let mut mesh = Mesh::default();
    for y in 0..5 {
        for x in 0..4 {
            mesh.vertices.push(p(x as f64, y));
        }
    }
    for y in 0..4u32 {
        for x in 0..3u32 {
            let i = y * 4 + x;
            mesh.faces.push(Face {
                vertices: vec![i, i + 1, i + 5, i + 4],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh.validate().unwrap();
    for (chamfer, rows) in [(false, 1), (true, 1), (false, 3)] {
        let r = edge_bevel_with(
            &mesh,
            &[(5, 9), (9, 13)],
            &settings(0.2, rows, chamfer, Default::default()),
            None,
        )
        .unwrap();
        r.mesh.validate().unwrap();
        // Both ends stay where they were; the middle point split into the
        // strip's two sides.
        assert!(has_point(&r.mesh, [1., 1., 0.]));
        assert!(has_point(&r.mesh, [1., 3., 0.]));
        assert!(!has_point(&r.mesh, [1., 2., 0.]));
    }
}

#[test]
fn square_corner_meets_at_the_original_point() {
    // Three Square strips at a box corner: three quads meeting at the
    // original corner, which stays where it was (Jason's QA, 2026-11-03).
    for chamfer in [false, true] {
        let o = EdgeBevelOptions {
            shape: EdgeBevelShape::Square,
            ..Default::default()
        };
        let m = edge_bevel_with(&cube(), &CORNER3, &settings(0.1, 2, chamfer, o), None)
            .unwrap()
            .mesh;
        m.validate().unwrap();
        assert!(has_point(&m, [0.5, 0.5, 0.5]));
        assert!(corner_patch(&m).iter().filter(|&&n| n == 4).count() >= 3);
        // The box keeps its silhouette: no point outside it.
        assert!(
            m.vertices
                .iter()
                .all(|p| p.abs().max_element() <= 0.5 + 1e-12)
        );
    }
}
