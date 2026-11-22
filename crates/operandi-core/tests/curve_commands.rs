//! Curve commands and 2D curve shapes (CURVE-COMMANDS-2D-SHAPES v1,
//! CC-xx): acceptance criteria from CURVE_COMMANDS_ACCEPTANCE.md.
//!
//! MC1–MC3 Make Open / Closed Curve; MF1, MF3 Make Curve Fill; AW1, AW2
//! Align Winding; SH1–SH6 the six shapes as MeshOps.
use glam::{DVec2, DVec3};
use operandi_core::curve_commands::{self, WindingOptions};
use operandi_core::curve_ops::{self, KAPPA, Shape, ShapeType};
use operandi_core::*;

fn points(ps: &[DVec3]) -> Mesh {
    Mesh {
        vertices: ps.to_vec(),
        ..Default::default()
    }
}

fn square(c: DVec3) -> [DVec3; 4] {
    [
        c + DVec3::new(-0.5, 0., -0.5),
        c + DVec3::new(0.5, 0., -0.5),
        c + DVec3::new(0.5, 0., 0.5),
        c + DVec3::new(-0.5, 0., 0.5),
    ]
}

fn rep(m: &Mesh, f: usize) -> PolygonRepresentation {
    m.attributes.representations[f]
}

#[test]
fn mc1_mc2_curves_follow_the_selection_order() {
    let m = points(&square(DVec3::ZERO));
    let (out, f) = curve_commands::make_curve(&m, &[2, 0, 3, 1], false).unwrap();
    assert_eq!(out.faces[f as usize].vertices, vec![2, 0, 3, 1]);
    assert!(matches!(
        rep(&out, f as usize),
        PolygonRepresentation::Polyline {
            closed: false,
            curve: CurveKind::Curve { .. }
        }
    ));
    assert_eq!(out.vertices, m.vertices, "points stay where they are");
    let (out, f) = curve_commands::make_curve(&m, &[2, 0, 3, 1], true).unwrap();
    assert!(matches!(
        rep(&out, f as usize),
        PolygonRepresentation::Polyline { closed: true, .. }
    ));
}

#[test]
fn mc2_the_command_selects_the_new_curve_in_polygon_mode() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = points(&square(DVec3::ZERO)).into();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![2, 0, 3, 1];
    e.dispatch(Command::MakeCurve { closed: true }).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.scene.selection.faces, vec![0]);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
}

#[test]
fn mc3_fewer_than_two_points_is_refused() {
    let m = points(&square(DVec3::ZERO));
    let err = curve_commands::make_curve(&m, &[1], true).unwrap_err();
    assert_eq!(err, curve_commands::TOO_FEW_POINTS);
}

fn closed_curve(m: &Mesh, ids: Vec<u32>, curve: CurveKind) -> Mesh {
    let mut m = m.clone();
    m.faces.push(Face {
        vertices: ids,
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    let n = m.faces.len();
    m.attributes
        .representations
        .resize(n, PolygonRepresentation::Face);
    m.attributes.representations[n - 1] = PolygonRepresentation::Polyline {
        closed: true,
        curve,
    };
    m.validate().unwrap();
    m
}

#[test]
fn mf1_fill_adds_a_polygon_on_the_same_points_and_keeps_the_curve() {
    let curve = CurveKind::Curve {
        start_control: false,
        end_control: false,
    };
    let m = closed_curve(&points(&square(DVec3::ZERO)), vec![0, 1, 2, 3], curve);
    let (out, fills) = curve_commands::make_curve_fill(&m, &[0]).unwrap();
    assert_eq!(out.faces.len(), 2);
    assert!(out.is_polyline(0), "the curve stays");
    assert_eq!(out.faces[fills[0] as usize].vertices, vec![0, 1, 2, 3]);
    assert!(!out.is_polyline(fills[0] as usize));
}

#[test]
fn mf2_a_bezier_circle_fills_along_its_outline() {
    let circle = curve_ops::shape_mesh(
        Shape::Circle,
        ShapeType::Bezier,
        DVec2::ZERO,
        DVec2::splat(0.5),
        1,
    );
    let (out, fills) = curve_commands::make_curve_fill(&circle, &[0]).unwrap();
    let fill = &out.faces[fills[0] as usize];
    assert_eq!(fill.vertices.len(), 4 * curve_commands::FILL_STEPS);
    for &v in &fill.vertices {
        assert!((out.vertices[v as usize].length() - 0.5).abs() < 1e-3);
    }
}

#[test]
fn mf3_open_curves_are_refused() {
    let m = points(&square(DVec3::ZERO));
    let (open, f) = curve_commands::make_curve(&m, &[0, 1, 2], false).unwrap();
    assert!(curve_commands::make_curve_fill(&open, &[f]).is_err());
}

/// Counter-clockwise seen from +Y: the area normal points up.
fn faces_up(m: &Mesh, f: usize) -> bool {
    let p: Vec<DVec3> = m.faces[f]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    let n = (0..p.len()).fold(DVec3::ZERO, |a, i| a + p[i].cross(p[(i + 1) % p.len()]));
    n.y > 0.
}

#[test]
fn aw1_aw2_two_squares_end_up_wound_the_same_way() {
    let mut ps = square(DVec3::ZERO).to_vec();
    ps.extend(square(DVec3::new(3., 0., 0.)));
    let line = CurveKind::Line;
    // The first runs clockwise from +Y, the second counter-clockwise.
    let m = closed_curve(&points(&ps), vec![0, 1, 2, 3], line);
    let m = closed_curve(&m, vec![4, 7, 6, 5], line);
    let a = faces_up(&m, 0);
    assert_ne!(a, faces_up(&m, 1));
    let (out, _) = curve_commands::align_winding(&m, &[0, 1], WindingOptions::default()).unwrap();
    assert!(faces_up(&out, 0) && faces_up(&out, 1));
    // The one that already matched keeps its points.
    let matching = if faces_up(&m, 0) { 0 } else { 1 };
    assert_eq!(out.faces[matching].vertices, m.faces[matching].vertices);
    let (out, _) = curve_commands::align_winding(
        &m,
        &[0, 1],
        WindingOptions {
            reverse: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!faces_up(&out, 0) && !faces_up(&out, 1));
}

#[test]
fn aw2_merge_curves_joins_pieces_that_share_end_points() {
    let m = points(&square(DVec3::ZERO));
    let (m, _) = curve_commands::make_curve(&m, &[0, 1, 2], false).unwrap();
    let (m, _) = curve_commands::make_curve(&m, &[2, 3, 0], false).unwrap();
    let (out, curves) =
        curve_commands::align_winding(&m, &[0, 1], WindingOptions::default()).unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(curves, vec![0]);
    assert!(matches!(
        rep(&out, 0),
        PolygonRepresentation::Polyline { closed: true, .. }
    ));
    assert!(faces_up(&out, 0));
}

#[test]
fn aw_bezier_reversal_keeps_its_shape() {
    let c = curve_ops::shape_mesh(
        Shape::Circle,
        ShapeType::Bezier,
        DVec2::ZERO,
        DVec2::splat(0.5),
        1,
    );
    let up = faces_up(&c, 0);
    let (out, _) = curve_commands::align_winding(
        &c,
        &[0],
        WindingOptions {
            reverse: up,
            ..Default::default()
        },
    )
    .unwrap();
    let draw = |m: &Mesh| {
        let p: Vec<DVec3> = m.faces[0]
            .vertices
            .iter()
            .map(|&v| m.vertices[v as usize])
            .collect();
        curves::sample(&p, CurveKind::Bezier, true, 16)
    };
    // Every drawn point is still on the circle.
    for q in draw(&out) {
        assert!((q.length() - 0.5).abs() < 2e-3, "{q}");
    }
    assert_ne!(out.faces[0].vertices, c.faces[0].vertices);
}

fn run_op(id: &str, values: serde_json::Value) -> Mesh {
    let def = meshop_types::op_def(id).unwrap();
    let mut v = def.defaults();
    if let (Some(v), Some(over)) = (v.as_object_mut(), values.as_object()) {
        for (k, x) in over {
            v.insert(k.clone(), x.clone());
        }
    }
    let input = Mesh::default();
    let ctx = meshop_types::OpContext {
        input: &input,
        op: 1,
        selection: None,
        links: &[],
        time: Default::default(),
    };
    def.run(&ctx, &v).unwrap()
}

fn ranges(m: &Mesh) -> (DVec3, DVec3) {
    let lo = m
        .vertices
        .iter()
        .fold(DVec3::splat(f64::INFINITY), |a, v| a.min(*v));
    let hi = m
        .vertices
        .iter()
        .fold(DVec3::splat(f64::NEG_INFINITY), |a, v| a.max(*v));
    (lo, hi)
}

#[test]
fn sh1_sh2_shapes_are_ops_with_full_size_spans() {
    let ids = [
        "curve.circle.item",
        "curve.ellipse.item",
        "curve.rectangle.item",
        "curve.diamond.item",
        "curve.nsided.item",
        "curve.star.item",
    ];
    for id in ids {
        let def = meshop_types::op_def(id).unwrap();
        assert_eq!(def.folder(), "Create › Curves", "{id}");
        assert_eq!(def.groups(), vec![("curve".into(), "Curve".into())], "{id}");
        let d = def.defaults();
        assert_eq!(d["axis"], 1, "{id}");
        assert_eq!(d["type"], 0, "{id}");
    }
    let rect = run_op("curve.rectangle.item", serde_json::json!({}));
    let (lo, hi) = ranges(&rect);
    assert!((lo - DVec3::new(-0.5, 0., -0.5)).length() < 1e-12);
    assert!((hi - DVec3::new(0.5, 0., 0.5)).length() < 1e-12);
    let e = run_op("curve.ellipse.item", serde_json::json!({}));
    let (lo, hi) = ranges(&e);
    assert!((hi - DVec3::new(0.5, 0., 0.25)).length() < 1e-12, "{hi}");
    assert!((lo + DVec3::new(0.5, 0., 0.25)).length() < 1e-12, "{lo}");
    // The circle's handles stick out past the radius; its anchors don't.
    let c = run_op("curve.circle.item", serde_json::json!({}));
    for k in [0usize, 3, 6, 9] {
        assert!((c.vertices[k].length() - 0.5).abs() < 1e-12);
    }
}

#[test]
fn sh3_bezier_layouts_and_point_counts() {
    let count = |id: &str| run_op(id, serde_json::json!({})).vertices.len();
    assert_eq!(count("curve.circle.item"), 12);
    assert_eq!(count("curve.ellipse.item"), 12);
    assert_eq!(count("curve.rectangle.item"), 12);
    assert_eq!(count("curve.diamond.item"), 12);
    assert_eq!(count("curve.nsided.item"), 24);
    assert_eq!(count("curve.star.item"), 30);
    // Circle handles at 0.552 × the radius (0.2761) along the tangent.
    let c = run_op("curve.circle.item", serde_json::json!({}));
    assert!((c.vertices[0] - DVec3::new(0., 0., 0.5)).length() < 1e-12);
    assert!((c.vertices[1] - DVec3::new(0.5 * KAPPA, 0., 0.5)).length() < 1e-12);
    assert!((0.5 * KAPPA - 0.2761).abs() < 1e-4);
    // Straight sides: both handles at the side's midpoint.
    let r = run_op("curve.rectangle.item", serde_json::json!({}));
    assert_eq!(r.vertices[1], r.vertices[2]);
    assert!((r.vertices[1] - (r.vertices[0] + r.vertices[3]) / 2.).length() < 1e-12);
}

#[test]
fn sh4_star_points() {
    let s = run_op("curve.star.item", serde_json::json!({ "type": 2 }));
    assert_eq!(s.vertices.len(), 10);
    assert!((s.vertices[0] - DVec3::new(0., 0., 0.5)).length() < 1e-12);
    assert!(
        (s.vertices[1] - DVec3::new(0.1469, 0., 0.2023)).length() < 1e-4,
        "{}",
        s.vertices[1]
    );
    assert!(
        (s.vertices[2] - DVec3::new(0.4755, 0., 0.1545)).length() < 1e-4,
        "{}",
        s.vertices[2]
    );
    let n = run_op("curve.nsided.item", serde_json::json!({ "type": 2 }));
    assert!((n.vertices[0] - DVec3::new(0., 0., 0.5)).length() < 1e-12);
}

#[test]
fn sh5_spline_and_polyline_types_and_fallback() {
    let c = run_op("curve.circle.item", serde_json::json!({ "type": 3 }));
    assert_eq!(c.vertices.len(), 16);
    assert!(c.vertices.iter().all(|v| (v.length() - 0.5).abs() < 1e-12));
    assert!(matches!(
        c.attributes.representations[0],
        PolygonRepresentation::Polyline {
            closed: true,
            curve: CurveKind::Curve { .. }
        }
    ));
    assert_eq!(
        run_op("curve.rectangle.item", serde_json::json!({ "type": 2 }))
            .vertices
            .len(),
        4
    );
    // A circle doesn't offer Polyline, a rectangle not Spline: Bezier.
    assert_eq!(
        run_op("curve.circle.item", serde_json::json!({ "type": 2 }))
            .vertices
            .len(),
        12
    );
    assert_eq!(
        run_op("curve.rectangle.item", serde_json::json!({ "type": 3 }))
            .vertices
            .len(),
        12
    );
}

#[test]
fn sh6_axis_and_position_place_the_plane() {
    let r = run_op(
        "curve.rectangle.item",
        serde_json::json!({ "pos.X": 2.0, "pos.Y": 3.0, "axis": 2 }),
    );
    let (lo, hi) = ranges(&r);
    assert!((lo - DVec3::new(1.5, 2.5, 0.)).length() < 1e-12, "{lo}");
    assert!((hi - DVec3::new(2.5, 3.5, 0.)).length() < 1e-12, "{hi}");
}

#[test]
fn path_ops_are_in_the_table_and_run() {
    for id in ["curve.extrude.item", "curve.clone.item", "pen.clone.item"] {
        assert!(meshop_types::op_def(id).is_some(), "{id}");
    }
    // An empty path changes nothing.
    let out = run_op("curve.clone.item", serde_json::json!({}));
    assert!(out.faces.is_empty());
}
