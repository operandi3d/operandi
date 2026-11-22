//! Freeze v2 (specifications/freeze-2026-11-10, FRZ-01..10): curves freeze
//! as Faces, Polylines or 2 Point Polygons, reusing their on-curve points;
//! the selection limits what freezes; Tessellation off freezes nothing.
//! Cases FRZ-C01..C13 from cases.json.
use glam::DVec3;
use operandi_core::curves::CurveKind;
use operandi_core::freeze::{FreezeCurves, FreezeOptions, NOTHING_TO_FREEZE};
use operandi_core::subdivision::{SubdivisionSettings, Toggle, has_smooth};
use operandi_core::*;

const TOL: f64 = 1e-5;

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

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

const CURVE: CurveKind = CurveKind::Curve {
    start_control: false,
    end_control: false,
};
const STRAIGHT_OPEN: PolygonRepresentation = PolygonRepresentation::Polyline {
    closed: false,
    curve: CurveKind::Line,
};

/// Elements over shared points: (kind, closed, point indices); kind None
/// is a Face.
fn mesh(points: Vec<DVec3>, elements: &[(Option<CurveKind>, bool, Vec<u32>)]) -> Mesh {
    let mut m = Mesh {
        vertices: points,
        ..Default::default()
    };
    for (kind, closed, v) in elements {
        m.faces.push(face(v.clone()));
        m.attributes.representations.push(match kind {
            None => PolygonRepresentation::Face,
            Some(curve) => PolygonRepresentation::Polyline {
                closed: *closed,
                curve: *curve,
            },
        });
    }
    m.validate().unwrap();
    m
}

fn curve_mesh(points: Vec<DVec3>, kind: CurveKind, closed: bool) -> Mesh {
    let n = points.len() as u32;
    mesh(points, &[(Some(kind), closed, (0..n).collect())])
}

fn c01_points() -> Vec<DVec3> {
    vec![p(0., 0., 0.), p(1., 1., 0.), p(2., 0., 0.), p(3., 1., 0.)]
}

fn c01() -> Mesh {
    curve_mesh(c01_points(), CurveKind::Bezier, false)
}

fn options(curves_as: FreezeCurves) -> FreezeOptions {
    FreezeOptions {
        curves_as,
        ..Default::default()
    }
}

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = mode;
    e
}

fn frozen(mesh: Mesh, o: FreezeOptions) -> Mesh {
    let mut e = editor_with(mesh, Mode::Item);
    e.dispatch(Command::Freeze(o)).unwrap();
    e.scene.active().unwrap().mesh.to_owned_value()
}

fn rep(m: &Mesh, f: usize) -> PolygonRepresentation {
    m.attributes
        .representations
        .get(f)
        .copied()
        .unwrap_or(PolygonRepresentation::Face)
}

fn curves_left(m: &Mesh) -> usize {
    (0..m.faces.len())
        .filter(|&f| operandi_core::freeze::is_curve(m, f))
        .count()
}

fn chain(m: &Mesh, f: usize) -> Vec<DVec3> {
    m.faces[f]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect()
}

fn has_point(m: &Mesh, q: DVec3) -> bool {
    m.vertices.iter().any(|v| (*v - q).length() < TOL)
}

fn near(a: DVec3, b: DVec3) -> bool {
    (a - b).length() < TOL
}

#[test]
fn frz_c01_faces_keep_anchors_and_drop_handles() {
    let m = frozen(c01(), options(FreezeCurves::Faces));
    assert_eq!(m.faces.len(), 1);
    assert_eq!(rep(&m, 0), PolygonRepresentation::Face);
    assert_eq!(m.faces[0].vertices.len(), 37);
    assert_eq!(m.vertices.len(), 37);
    assert!(has_point(&m, p(0., 0., 0.)) && has_point(&m, p(3., 1., 0.)));
    assert!(!has_point(&m, p(1., 1., 0.)) && !has_point(&m, p(2., 0., 0.)));
    assert_eq!(curves_left(&m), 0);
    assert_eq!(m.faces[0].material, "Default");
}

#[test]
fn frz_c02_polylines() {
    let m = frozen(c01(), options(FreezeCurves::Polylines));
    assert_eq!(m.faces.len(), 1);
    assert_eq!(rep(&m, 0), STRAIGHT_OPEN);
    assert_eq!((m.faces[0].vertices.len(), m.vertices.len()), (37, 37));
}

#[test]
fn frz_c03_two_point_polygons() {
    let m = frozen(c01(), options(FreezeCurves::TwoPointPolygons));
    assert_eq!((m.faces.len(), m.vertices.len()), (36, 37));
    for (f, face) in m.faces.iter().enumerate() {
        assert_eq!(face.vertices.len(), 2);
        assert_eq!(rep(&m, f), STRAIGHT_OPEN);
        if f > 0 {
            assert_eq!(m.faces[f - 1].vertices[1], face.vertices[0]);
        }
    }
}

#[test]
fn frz_c04_bezier_samples_in_equal_parameter_steps() {
    let m = frozen(c01(), options(FreezeCurves::Polylines));
    let s = chain(&m, 0);
    for (i, q) in [
        (0, p(0., 0., 0.)),
        (1, p(0.083333, 0.078789, 0.)),
        (18, p(1.5, 0.5, 0.)),
        (35, p(2.916667, 0.921211, 0.)),
        (36, p(3., 1., 0.)),
    ] {
        assert!(near(s[i], q), "sample {i}: {}", s[i]);
    }
}

#[test]
fn frz_c05_curve_keeps_all_its_points_as_samples() {
    let m = frozen(
        curve_mesh(c01_points(), CURVE, false),
        options(FreezeCurves::Polylines),
    );
    assert_eq!(m.faces.len(), 1);
    assert_eq!((m.faces[0].vertices.len(), m.vertices.len()), (109, 109));
    let s = chain(&m, 0);
    for (i, q) in c01_points().into_iter().enumerate() {
        assert!(near(s[36 * i], q), "{i}");
    }
    // The span-1 midpoint (0.5, 0.702254, 0): PF-09's end tangent.
    assert!((s[18].x - 0.5).abs() < TOL);
    assert!((s[18].y - 0.702254).abs() < TOL, "{}", s[18]);
}

#[test]
fn frz_c06_straight_curve_still_gets_36_steps_a_span() {
    let points = vec![p(0., 0., 0.), p(1., 0., 0.), p(2., 0., 0.), p(3., 0., 0.)];
    let m = frozen(
        curve_mesh(points, CURVE, false),
        options(FreezeCurves::Faces),
    );
    assert_eq!(m.faces.len(), 1);
    assert_eq!(rep(&m, 0), PolygonRepresentation::Face);
    assert_eq!((m.faces[0].vertices.len(), m.vertices.len()), (109, 109));
}

#[test]
fn frz_c07_shared_start_point_stays_shared() {
    let mut points = c01_points();
    points.extend([p(-1., 0., 0.), p(-1., -1., 0.)]);
    let m = frozen(
        mesh(
            points,
            &[
                (Some(CurveKind::Bezier), false, vec![0, 1, 2, 3]),
                (None, false, vec![0, 4, 5]),
            ],
        ),
        options(FreezeCurves::Faces),
    );
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.vertices.len(), 39);
    let triangle = m.faces.iter().position(|f| f.vertices.len() == 3).unwrap();
    let frozen_face = 1 - triangle;
    assert_eq!(m.faces[frozen_face].vertices.len(), 37);
    let t = chain(&m, triangle);
    assert!(near(t[0], p(0., 0., 0.)) && near(t[1], p(-1., 0., 0.)) && near(t[2], p(-1., -1., 0.)));
    // The shared point is the same point in both polygons.
    assert!(
        m.faces[frozen_face]
            .vertices
            .contains(&m.faces[triangle].vertices[0])
    );
}

fn two_beziers() -> Mesh {
    let mut points = c01_points();
    points.extend(c01_points().iter().map(|&q| q + p(0., 3., 0.)));
    mesh(
        points,
        &[
            (Some(CurveKind::Bezier), false, vec![0, 1, 2, 3]),
            (Some(CurveKind::Bezier), false, vec![4, 5, 6, 7]),
        ],
    )
}

#[test]
fn frz_c08_polygon_mode_freezes_only_the_selection() {
    let mut e = editor_with(two_beziers(), Mode::Polygon);
    e.scene.selection.faces = vec![1];
    e.dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), 41);
    assert_eq!(m.faces.len(), 2);
    assert_eq!(curves_left(m), 1);
    let lower = (0..2)
        .find(|&f| operandi_core::freeze::is_curve(m, f))
        .unwrap();
    assert_eq!(chain(m, lower), c01_points());
    assert_eq!(m.faces[1 - lower].vertices.len(), 37);
    assert!(e.scene.selection.faces.is_empty());
}

#[test]
fn frz_c09_item_mode_freezes_all_vertex_mode_nothing() {
    let m = frozen(two_beziers(), options(FreezeCurves::Faces));
    assert_eq!((m.faces.len(), m.vertices.len()), (2, 74));
    assert_eq!(curves_left(&m), 0);

    let input = two_beziers();
    let mut e = editor_with(input.clone(), Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    let steps = e.undo.len();
    let err = e
        .dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap_err();
    assert!(err.starts_with(NOTHING_TO_FREEZE), "{err}");
    assert_eq!(e.undo.len(), steps);
    assert_eq!(*e.scene.active().unwrap().mesh, input);
    assert_eq!(e.scene.selection.vertices, vec![0]);
}

/// A unit cube of 6 subdivision quads plus the FRZ-C01 Bezier moved +3 in X.
fn mixed() -> Mesh {
    let mut m = subdivision::toggle(
        &Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap(),
        &[],
        Toggle::Subdivision,
    )
    .unwrap();
    let base = m.vertices.len() as u32;
    m.vertices
        .extend(c01_points().iter().map(|&q| q + p(3., 0., 0.)));
    m.faces.push(face((base..base + 4).collect()));
    m.attributes
        .representations
        .push(PolygonRepresentation::Polyline {
            closed: false,
            curve: CurveKind::Bezier,
        });
    m.validate().unwrap();
    m
}

#[test]
fn frz_c10_tessellation_off_changes_nothing() {
    for input in [c01(), mixed()] {
        let mut e = editor_with(input.clone(), Mode::Item);
        let steps = e.undo.len();
        let err = e
            .dispatch(Command::Freeze(FreezeOptions {
                tessellation: false,
                ..options(FreezeCurves::Faces)
            }))
            .unwrap_err();
        assert!(err.starts_with(NOTHING_TO_FREEZE), "{err}");
        assert_eq!(e.undo.len(), steps);
        assert_eq!(*e.scene.active().unwrap().mesh, input);
    }
}

#[test]
fn frz_c11_mixed_mesh_freezes_in_one_step() {
    let cube = subdivision::freeze(
        &subdivision::toggle(
            &Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap(),
            &[],
            Toggle::Subdivision,
        )
        .unwrap(),
        &SubdivisionSettings::default(),
    )
    .unwrap();
    let mut e = editor_with(mixed(), Mode::Item);
    let steps = e.undo.len();
    e.dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Freeze");
    let m = &e.scene.active().unwrap().mesh;
    assert!(!has_smooth(m));
    assert_eq!(curves_left(m), 0);
    assert_eq!(m.faces.len(), cube.faces.len() + 1);
    assert_eq!(m.faces.iter().filter(|f| f.vertices.len() == 37).count(), 1);
    assert_eq!(m.vertices.len(), cube.vertices.len() + 37);
    // One undo restores the cage and the curve.
    let input = mixed();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*e.scene.active().unwrap().mesh, input);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(curves_left(&e.scene.active().unwrap().mesh), 0);
}

#[test]
fn frz_c12_nothing_to_freeze() {
    let m = mesh(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(0., 1., 0.),
            p(3., 0., 0.),
            p(4., 1., 0.),
            p(5., 0., 0.),
        ],
        &[
            (None, false, vec![0, 1, 2, 3]),
            (Some(CurveKind::Line), false, vec![4, 5, 6]),
        ],
    );
    let mut e = editor_with(m.clone(), Mode::Item);
    let steps = e.undo.len();
    let err = e
        .dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap_err();
    assert!(err.starts_with(NOTHING_TO_FREEZE), "{err}");
    assert_eq!(e.undo.len(), steps);
    assert_eq!(*e.scene.active().unwrap().mesh, m);
}

fn turn(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    (b - a)
        .normalize()
        .dot((c - b).normalize())
        .clamp(-1., 1.)
        .acos()
        .to_degrees()
}

#[test]
fn frz_c13_adaptive_spans_follow_the_refinement_angle() {
    let adaptive = |degrees| FreezeOptions {
        curves_as: FreezeCurves::Polylines,
        fixed_spans: false,
        refinement_degrees: degrees,
        ..Default::default()
    };
    let straight = vec![p(0., 0., 0.), p(1., 0., 0.), p(2., 0., 0.), p(3., 0., 0.)];
    let m = frozen(curve_mesh(straight, CurveKind::Bezier, false), adaptive(5.));
    assert_eq!(m.faces[0].vertices.len(), 2);
    let segments = |degrees| frozen(c01(), adaptive(degrees)).faces[0].vertices.len() - 1;
    let (fine, coarse) = (segments(2.), segments(20.));
    assert!(fine > coarse && coarse >= 2, "{fine} {coarse}");
    for degrees in [2., 5., 20.] {
        let s = chain(&frozen(c01(), adaptive(degrees)), 0);
        assert_eq!(s[0], p(0., 0., 0.));
        assert_eq!(*s.last().unwrap(), p(3., 1., 0.));
        for w in s.windows(3) {
            assert!(turn(w[0], w[1], w[2]) <= degrees + 1e-5, "{degrees}");
        }
    }
}

#[test]
fn closed_bezier_keeps_its_anchors() {
    // FRZ-Q3 build guess: the closing span is sampled too; anchors kept.
    let points = vec![
        p(0., 0., 0.),
        p(1., 1., 0.),
        p(2., 1., 0.),
        p(3., 0., 0.),
        p(2., -1., 0.),
        p(1., -1., 0.),
    ];
    let m = frozen(
        curve_mesh(points, CurveKind::Bezier, true),
        options(FreezeCurves::Polylines),
    );
    assert_eq!((m.faces[0].vertices.len(), m.vertices.len()), (72, 72));
    assert_eq!(
        rep(&m, 0),
        PolygonRepresentation::Polyline {
            closed: true,
            curve: CurveKind::Line
        }
    );
    assert!(has_point(&m, p(0., 0., 0.)) && has_point(&m, p(3., 0., 0.)));
    assert!(!has_point(&m, p(1., 1., 0.)));
}

#[test]
fn polygon_mode_selection_limits_subdivision_too() {
    // FRZ-01: only the selected curve freezes in a mixed mesh; the cube
    // stays a cage when it isn't selected.
    let mut e = editor_with(mixed(), Mode::Polygon);
    e.scene.selection.faces = vec![6];
    e.dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 7);
    assert_eq!(
        (0..7)
            .filter(|&f| subdivision::is_smooth_polygon(m, f))
            .count(),
        6
    );
    assert_eq!(curves_left(m), 0);
    // Two cube polygons selected: they freeze, the other four stay smooth.
    let mut e = editor_with(mixed(), Mode::Polygon);
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::Freeze(Default::default())).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        (0..m.faces.len())
            .filter(|&f| subdivision::is_smooth_polygon(m, f))
            .count(),
        4
    );
    assert_eq!(curves_left(m), 1);
    m.validate().unwrap();
}

#[test]
fn start_control_end_is_not_sampled() {
    let kind = CurveKind::Curve {
        start_control: true,
        end_control: false,
    };
    let m = frozen(
        curve_mesh(c01_points(), kind, false),
        options(FreezeCurves::Polylines),
    );
    let s = chain(&m, 0);
    assert_eq!(s.len(), 2 * 36 + 1);
    assert!(near(s[0], p(1., 1., 0.)));
    assert_eq!(m.vertices.len(), 73);
}

#[test]
fn mixed_mesh_keeps_lines_through_the_surface_freeze() {
    for (form, lines) in [
        (FreezeCurves::Polylines, 1),
        (FreezeCurves::TwoPointPolygons, 36),
    ] {
        let m = frozen(mixed(), options(form));
        let count = (0..m.faces.len()).filter(|&f| m.is_polyline(f)).count();
        assert_eq!(count, lines);
        assert!(!has_smooth(&m));
    }
}

#[test]
fn frozen_curves_survive_save_and_reopen() {
    let mut e = editor_with(mixed(), Mode::Item);
    e.dispatch(Command::Freeze(options(FreezeCurves::Faces)))
        .unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
}
