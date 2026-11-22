//! Curve Boolean (CURVE-BOOLEAN v1, CB-xx): the six acceptance cases on
//! A = square (0,0)–(2,2) (selected) and B = square (1,1)–(3,3) in XZ.
use glam::DVec3;
use operandi_core::curve_boolean::{self, CurveBooleanOp, CurveBooleanOptions};
use operandi_core::*;

fn xz(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 0., z)
}

/// A then B as closed polylines, both counter-clockwise in (x, z).
fn scene() -> Mesh {
    let a = [xz(0., 0.), xz(2., 0.), xz(2., 2.), xz(0., 2.)];
    let b = [xz(1., 1.), xz(3., 1.), xz(3., 3.), xz(1., 3.)];
    let mut m = Mesh::default();
    for ring in [a, b] {
        let first = m.vertices.len() as u32;
        m.vertices.extend(ring);
        m.faces.push(Face {
            vertices: (first..first + 4).collect(),
            uv: None,
            material: "M".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
        m.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: true,
                curve: CurveKind::Line,
            });
    }
    m.validate().unwrap();
    m
}

fn run(op: CurveBooleanOp, clip: bool, merge: bool) -> Mesh {
    curve_boolean::curve_boolean(
        &scene(),
        &[0],
        &CurveBooleanOptions {
            op,
            axis: 0,
            clip,
            merge,
        },
    )
    .unwrap()
}

/// The result curves other than B, as (x, z) lists with closure.
fn results(m: &Mesh) -> Vec<(Vec<(f64, f64)>, bool)> {
    (0..m.faces.len())
        .filter_map(|f| {
            let pts: Vec<(f64, f64)> = m.faces[f]
                .vertices
                .iter()
                .map(|&v| {
                    let p = m.vertices[v as usize];
                    ((p.x * 1e6).round() / 1e6, (p.z * 1e6).round() / 1e6)
                })
                .collect();
            let closed = matches!(
                m.attributes.representations[f],
                PolygonRepresentation::Polyline { closed: true, .. }
            );
            let b = vec![(1., 1.), (3., 1.), (3., 3.), (1., 3.)];
            (pts != b).then_some((pts, closed))
        })
        .collect()
}

/// Equal as loops (any start) or, open, exactly.
fn same(got: &[(f64, f64)], want: &[(f64, f64)], closed: bool) -> bool {
    if got.len() != want.len() {
        return false;
    }
    if !closed {
        return got == want;
    }
    (0..got.len()).any(|s| (0..got.len()).all(|i| got[(s + i) % got.len()] == want[i]))
}

fn check(m: &Mesh, want: &[(&[(f64, f64)], bool)]) {
    let got = results(m);
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (w, closed) in want {
        assert!(
            got.iter().any(|(g, c)| c == closed && same(g, w, *closed)),
            "want {w:?} in {got:?}"
        );
    }
    // CB-10: B is unchanged.
    let b = [xz(1., 1.), xz(3., 1.), xz(3., 3.), xz(1., 3.)];
    assert!((0..m.faces.len()).any(|f| {
        m.faces[f]
            .vertices
            .iter()
            .map(|&v| m.vertices[v as usize])
            .collect::<Vec<_>>()
            == b
    }));
}

#[test]
fn cb_t1_slice_inserts_the_crossings() {
    check(
        &run(CurveBooleanOp::Slice, false, true),
        &[(
            &[(0., 0.), (2., 0.), (2., 1.), (2., 2.), (1., 2.), (0., 2.)],
            true,
        )],
    );
}

#[test]
fn cb_t2_union() {
    check(
        &run(CurveBooleanOp::Union, false, true),
        &[(
            &[
                (1., 2.),
                (1., 3.),
                (3., 3.),
                (3., 1.),
                (2., 1.),
                (2., 0.),
                (0., 0.),
                (0., 2.),
            ],
            true,
        )],
    );
}

#[test]
fn cb_t3_intersect() {
    check(
        &run(CurveBooleanOp::Intersect, false, true),
        &[(&[(2., 1.), (1., 1.), (1., 2.), (2., 2.)], true)],
    );
}

#[test]
fn cb_t4_subtract() {
    check(
        &run(CurveBooleanOp::Subtract, false, true),
        &[(
            &[(1., 2.), (1., 1.), (2., 1.), (2., 0.), (0., 0.), (0., 2.)],
            true,
        )],
    );
}

#[test]
fn cb_t5_union_clip_keeps_the_selected_curves_open_piece() {
    check(
        &run(CurveBooleanOp::Union, true, true),
        &[(&[(2., 1.), (2., 0.), (0., 0.), (0., 2.), (1., 2.)], false)],
    );
}

#[test]
fn cb_t6_subtract_merge_off_leaves_two_open_pieces() {
    check(
        &run(CurveBooleanOp::Subtract, false, false),
        &[
            (&[(2., 1.), (2., 0.), (0., 0.), (0., 2.), (1., 2.)], false),
            (&[(1., 2.), (1., 1.), (2., 1.)], false),
        ],
    );
}

#[test]
fn cb_18_19_both_selected_union_and_one_undo_step() {
    let out = curve_boolean::curve_boolean(
        &scene(),
        &[0, 1],
        &CurveBooleanOptions {
            op: CurveBooleanOp::Union,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out.faces.len(), 1, "one outline");
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = scene().into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    let steps = e.undo.len();
    e.dispatch(Command::CurveBoolean(CurveBooleanOptions {
        op: CurveBooleanOp::Intersect,
        ..Default::default()
    }))
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert!(e.scene.selection.faces.is_empty(), "CB-19");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, scene());
    // Nothing selected: refused.
    e.scene.selection.faces.clear();
    assert!(
        e.dispatch(Command::CurveBoolean(CurveBooleanOptions::default()))
            .is_err()
    );
}
