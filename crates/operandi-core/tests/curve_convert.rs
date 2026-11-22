//! Hands-on curve commands (CURVE-COMMANDS-2D-SHAPES, 2026-11-17):
//! CC-50..54 conversions, CC-40..43 offsets, CC-60..64 Bezier Tangent, and
//! Pen Extrude's Corner Scale (CX-31).
use glam::{DVec2, DVec3};
use operandi_core::curve_convert::{
    self, CurveType, TangentOperation, TangentOptions, TangentSide,
};
use operandi_core::curve_ops::{Shape, ShapeType, shape_mesh};
use operandi_core::*;

fn closed_curve(points: &[DVec3], curve: CurveKind) -> Mesh {
    let mut m = Mesh {
        vertices: points.to_vec(),
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: (0..points.len() as u32).collect(),
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: true,
        curve,
    }];
    m
}

fn square() -> Vec<DVec3> {
    vec![
        DVec3::ZERO,
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 0., 1.),
        DVec3::new(0., 0., 1.),
    ]
}
fn curve_kind() -> CurveKind {
    CurveKind::Curve {
        start_control: false,
        end_control: false,
    }
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

#[test]
fn cc52_curve_to_bezier_is_exact_with_sixth_handles() {
    let p =
        curve_convert::convert_points(&square(), curve_kind(), true, CurveType::Bezier).unwrap();
    assert_eq!(p.len(), 12);
    // Knot 0 at (0,0,0): handles ± (next − prev) / 6 = ±(1,0,−1)/6.
    assert!(
        (p[1] - DVec3::new(1. / 6., 0., -1. / 6.)).length() < 1e-9,
        "{}",
        p[1]
    );
    assert!(
        (p[11] - DVec3::new(-1. / 6., 0., 1. / 6.)).length() < 1e-9,
        "{}",
        p[11]
    );
    // The Bezier overlaps the curve.
    let a = curves::sample(&square(), curve_kind(), true, 16);
    let b = curves::sample(&p, CurveKind::Bezier, true, 16);
    for (x, y) in a.iter().zip(&b) {
        assert!(x.distance(*y) < 1e-9);
    }
}

#[test]
fn cc53_bezier_to_b_spline_uses_the_handles_and_the_first_knot() {
    let bez =
        curve_convert::convert_points(&square(), curve_kind(), true, CurveType::Bezier).unwrap();
    let bs =
        curve_convert::convert_points(&bez, CurveKind::Bezier, true, CurveType::BSpline).unwrap();
    assert_eq!(bs.len(), 9);
    assert_eq!(bs[0], bez[0]);
    // Inside the Bezier, not on it.
    let mid = curves::sample(&bs, CurveKind::BSpline, true, 8);
    assert!(mid.iter().all(|q| q.x > -0.2 && q.x < 1.2));
}

#[test]
fn cc51_cc54_conversion_makes_a_new_item_and_keeps_the_source() {
    let mut e = editor(closed_curve(&square(), curve_kind()), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    let source = e.scene.active().unwrap().id;
    let before = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::ConvertCurves {
        from: CurveType::Curve,
        to: CurveType::Bezier,
    })
    .unwrap();
    let new = e.scene.active().unwrap();
    assert_ne!(new.id, source);
    assert_eq!(new.name, "Converted_Bezier");
    assert_eq!(new.mesh.vertices.len(), 12);
    assert_eq!(
        e.scene.item(source).unwrap().mesh,
        before,
        "the source stays"
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items.iter().all(|i| i.name != "Converted_Bezier"));
    // CC-54: no Bezier curve selected → refused.
    e.dispatch(Command::SelectItem(Some(source))).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    assert!(
        e.dispatch(Command::ConvertCurves {
            from: CurveType::Bezier,
            to: CurveType::Curve,
        })
        .is_err()
    );
    // Every pair converts.
    let m = closed_curve(&square(), curve_kind());
    for from in [CurveType::Curve, CurveType::Bezier, CurveType::BSpline] {
        for to in [CurveType::Curve, CurveType::Bezier, CurveType::BSpline] {
            if from == to {
                continue;
            }
            let src = curve_convert::convert_mesh(&m, &[0], CurveType::Curve, from)
                .unwrap_or_else(|_| m.clone());
            let out = curve_convert::convert_mesh(&src, &[0], from, to).unwrap();
            assert!(out.validate().is_ok(), "{from:?} → {to:?}");
        }
    }
}

#[test]
fn cc41_cc42_closed_offset_moves_corners_in_and_trio_both_ways() {
    let m = closed_curve(&square(), curve_kind());
    let (out, new) = curve_convert::offset_curves(&m, &[0], true, 0.1, false).unwrap();
    assert_eq!(out.faces.len(), 2, "the original stays");
    let ring: Vec<DVec3> = out.faces[new[0] as usize]
        .vertices
        .iter()
        .map(|&v| out.vertices[v as usize])
        .collect();
    let want = [(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9)];
    for (p, (x, z)) in ring.iter().zip(want) {
        assert!((p.x - x).abs() < 1e-9 && (p.z - z).abs() < 1e-9, "{p}");
    }
    // The other winding still goes inward.
    let mut rev = square();
    rev.reverse();
    let (out, new) =
        curve_convert::offset_curves(&closed_curve(&rev, curve_kind()), &[0], true, 0.1, false)
            .unwrap();
    assert!(
        out.faces[new[0] as usize]
            .vertices
            .iter()
            .all(|&v| (0.05..0.95).contains(&out.vertices[v as usize].x))
    );
    let (out, new) = curve_convert::offset_curves(&m, &[0], true, 0.1, true).unwrap();
    assert_eq!(new.len(), 2);
    assert_eq!(out.faces.len(), 3);
    let first_outer = out.vertices[out.faces[new[1] as usize].vertices[0] as usize];
    assert!(
        (first_outer - DVec3::new(-0.1, 0., -0.1)).length() < 1e-9,
        "{first_outer}"
    );
    // Open buttons ignore closed curves.
    assert!(curve_convert::offset_curves(&m, &[0], false, 0.1, false).is_err());
}

/// Handles of knot 0 of the converted square: (in, out).
fn knot0(m: &Mesh) -> (DVec3, DVec3) {
    let v = &m.faces[0].vertices;
    (
        m.vertices[v[v.len() - 1] as usize],
        m.vertices[v[1] as usize],
    )
}

#[test]
fn cc61_cc64_bezier_tangent_operations() {
    let bez =
        curve_convert::convert_points(&square(), curve_kind(), true, CurveType::Bezier).unwrap();
    let m = closed_curve(&bez, CurveKind::Bezier);
    let run = |o: TangentOptions| curve_convert::bezier_tangent(&m, &[0], &o).unwrap().0;
    // Sharp: Out aims at the next knot, In at the previous one.
    let (i, o) = knot0(&run(TangentOptions::default()));
    assert!(o.normalize().distance(DVec3::X) < 1e-9, "{o}");
    assert!(i.normalize().distance(DVec3::Z) < 1e-9, "{i}");
    // Make the handles uneven and bent, then Smooth / Uniform.
    let mut bent = m.clone();
    let v = bent.faces[0].vertices.clone();
    bent.vertices[v[1] as usize] = DVec3::new(0.3, 0., 0.);
    bent.vertices[v[v.len() - 1] as usize] = DVec3::new(0., 0., 0.1);
    let smooth = curve_convert::bezier_tangent(
        &bent,
        &[0],
        &TangentOptions {
            operation: TangentOperation::Smooth,
            ..Default::default()
        },
    )
    .unwrap()
    .0;
    let (i, o) = knot0(&smooth);
    assert!(i.normalize().dot(o.normalize()) < -0.999999, "one line");
    assert!((i.length() - 0.1).abs() < 1e-9 && (o.length() - 0.3).abs() < 1e-9);
    let uniform = curve_convert::bezier_tangent(
        &bent,
        &[0],
        &TangentOptions {
            operation: TangentOperation::Uniform,
            ..Default::default()
        },
    )
    .unwrap()
    .0;
    let (i, o) = knot0(&uniform);
    assert!((i.length() - 0.2).abs() < 1e-9 && (o.length() - 0.2).abs() < 1e-9);
    // Out only, Set Length.
    let out_only = curve_convert::bezier_tangent(
        &bent,
        &[0],
        &TangentOptions {
            operation: TangentOperation::Smooth,
            tangent: TangentSide::Out,
            set_length: true,
            length: 1.,
        },
    )
    .unwrap()
    .0;
    let (i, o) = knot0(&out_only);
    assert_eq!(i, DVec3::new(0., 0., 0.1), "In unchanged");
    assert!((o.length() - 1.).abs() < 1e-9);
    // Not a knot: refused.
    assert!(curve_convert::bezier_tangent(&m, &[1], &TangentOptions::default()).is_err());
}

#[test]
fn bezier_tangent_command_is_one_step_in_vertex_mode() {
    let c = shape_mesh(
        Shape::Circle,
        ShapeType::Bezier,
        DVec2::ZERO,
        DVec2::splat(0.5),
        1,
    );
    let mut e = editor(c.clone(), Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    let steps = e.undo.len();
    e.dispatch(Command::BezierTangent(TangentOptions::default()))
        .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_ne!(e.scene.active().unwrap().mesh, c);
}

#[test]
fn cx31_pen_extrude_corner_scale_widens_across_the_bend() {
    use operandi_core::path_tools::{self, PathKind, PathTool};
    // A 0.2 quad standing across the path (normal +Y), path up then right.
    let mut q = Mesh {
        vertices: vec![
            DVec3::new(-0.1, 0., -0.1),
            DVec3::new(-0.1, 0., 0.1),
            DVec3::new(0.1, 0., 0.1),
            DVec3::new(0.1, 0., -0.1),
        ],
        ..Default::default()
    };
    q.faces.push(Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    let mut t = PathTool::default();
    t.curve.kind = PathKind::Pen;
    t.curve.points = vec![DVec3::ZERO, DVec3::new(0., 1., 0.), DVec3::new(1., 1., 0.)];
    t.sweep.cap_start = false;
    t.sweep.invert = true;
    let width = |t: &PathTool| {
        let r = path_tools::sweep_geometry(&q, &[0], t).unwrap();
        let ring = &r.mesh.vertices[4..8];
        let xs = ring.iter().map(|v| v.x);
        let ys = ring.iter().map(|v| v.y);
        let span = |it: &mut dyn Iterator<Item = f64>| {
            let v: Vec<f64> = it.collect();
            v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
        };
        (
            span(&mut xs.clone()).max(span(&mut ys.clone())),
            ring.to_vec(),
        )
    };
    let (wide, _) = width(&t);
    t.corner_scale = false;
    let (plain, _) = width(&t);
    assert!(
        (wide / plain - std::f64::consts::SQRT_2).abs() < 1e-6,
        "{wide} {plain}"
    );
}

#[test]
fn cc43_open_offset_is_mitred_to_the_left_and_trio_does_both_sides() {
    let pts = [
        DVec3::ZERO,
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 0., 1.),
        DVec3::new(2., 0., 1.),
    ];
    let mut m = closed_curve(&pts, curve_kind());
    m.attributes.representations[0] = PolygonRepresentation::Polyline {
        closed: false,
        curve: curve_kind(),
    };
    let got = |m: &Mesh, f: u32| -> Vec<DVec3> {
        m.faces[f as usize]
            .vertices
            .iter()
            .map(|&v| m.vertices[v as usize])
            .collect()
    };
    let near = |a: &[DVec3], b: &[DVec3]| a.iter().zip(b).all(|(x, y)| x.distance(*y) < 1e-9);
    let (out, new) = curve_convert::offset_curves(&m, &[0], false, 0.1, false).unwrap();
    assert_eq!(out.faces.len(), 2, "a new curve, the original kept");
    assert!(matches!(
        out.attributes.representations[new[0] as usize],
        PolygonRepresentation::Polyline { closed: false, .. }
    ));
    let left = [
        DVec3::new(0., 0., 0.1),
        DVec3::new(0.9, 0., 0.1),
        DVec3::new(0.9, 0., 1.1),
        DVec3::new(2., 0., 1.1),
    ];
    assert!(near(&got(&out, new[0]), &left), "{:?}", got(&out, new[0]));
    let (out, new) = curve_convert::offset_curves(&m, &[0], false, 0.1, true).unwrap();
    assert_eq!(out.faces.len(), 3);
    let right = [
        DVec3::new(0., 0., -0.1),
        DVec3::new(1.1, 0., -0.1),
        DVec3::new(1.1, 0., 0.9),
        DVec3::new(2., 0., 0.9),
    ];
    assert!(near(&got(&out, new[1]), &right), "{:?}", got(&out, new[1]));
}
