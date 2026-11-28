//! Pen Make Quads (SPEC-PEN-MAKE-QUADS): after a two-click starting edge,
//! each click adds a point and its partner and closes one more quad;
//! Ctrl+click ends the strip with a triangle.
//!
//! Acceptance criteria:
//! 1. PMQ-02: A, B, then P makes one quad A, B, P + (B − A), P.
//! 2. PMQ-03: a fourth click adds a second quad sharing an edge; the strip
//!    is one band whose ends are ordinary boundary edges (for Edge Join).
//! 3. PMQ-04: Ctrl+click closes the end with a triangle; the strip then
//!    takes no more points, and the draft is unchanged by the refusal.
//! 4. Undo while drawing takes back one click (a point pair, or the
//!    triangle's point); the dropped strip is one undo step; redo restores it.
//! 5. Cancel leaves the scene untouched; too few points follow the Pen's
//!    usual Drop rules.
//! 6. Make Quads is inert for non-polygon types, refused with Wall Mode, and
//!    can't change mid-draft.
//! 7. Dragging (refining) a strip point moves only that point; Flip and the
//!    viewer-facing reverse turn the whole strip without scrambling it.
//! 8. The strip survives save and reopen.
//! 9. PMQ-Q4: a strip that curves (either way, rungs on either side) turns
//!    its guessed partners with it: each rung keeps pointing across the
//!    strip, and the quads stay convex and face one way. The drag right
//!    after a click carries its partner; a later drag doesn't.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e
}
fn quads() -> PenOptions {
    PenOptions {
        make_quads: true,
        ..Default::default()
    }
}
fn start(e: &mut Editor, options: PenOptions) {
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(options)).unwrap();
}
fn click(e: &mut Editor, x: f64, y: f64) {
    e.dispatch(Command::PenCapture(DVec3::new(x, y, 0.)))
        .unwrap();
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
fn corners(m: &Mesh, f: usize) -> Vec<DVec3> {
    m.faces[f]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect()
}
/// Same cycle up to the starting corner.
fn same_cycle(got: &[DVec3], want: &[DVec3]) -> bool {
    got.len() == want.len()
        && (0..got.len())
            .any(|s| (0..got.len()).all(|i| got[(i + s) % got.len()].distance(want[i]) < 1e-12))
}
fn v(x: f64, y: f64) -> DVec3 {
    DVec3::new(x, y, 0.)
}

#[test]
fn pmq_02_three_clicks_make_one_quad_facing_the_strip_order() {
    let mut e = editor();
    start(&mut e, quads());
    click(&mut e, 0., 0.);
    click(&mut e, 1., 0.);
    click(&mut e, 0., 1.);
    let draft = e.session.pen_draft.clone().unwrap();
    assert_eq!(draft.points().len(), 4, "the click adds its partner");
    assert_eq!(draft.current(), Some(2), "the clicked point is current");
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 1);
    assert!(same_cycle(
        &corners(&m, 0),
        &[v(0., 0.), v(1., 0.), v(1., 1.), v(0., 1.)]
    ));
    assert!(m.normal(0).z > 0.99);
    assert_eq!(e.history.undo.last().unwrap().label, "Pen Quads");
}

#[test]
fn pmq_03_each_click_adds_a_quad_to_one_connected_band() {
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (0., 2.), (0., 3.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 3, "N + 2 clicks make N quads");
    assert_eq!(m.vertices.len(), 8);
    let shared = (v(0., 1.), v(1., 1.));
    let has = |f: usize, p: DVec3| corners(&m, f).iter().any(|q| q.distance(p) < 1e-12);
    assert!(has(0, shared.0) && has(0, shared.1) && has(1, shared.0) && has(1, shared.1));
    let counts = m.edge_counts();
    let inner = counts.values().filter(|&&c| c == 2).count();
    let border = counts.values().filter(|&&c| c == 1).count();
    assert_eq!((inner, border), (2, 8), "two shared edges, the rest border");
    assert!((0..3).all(|f| m.normal(f).z > 0.99), "consistent facing");
    m.validate().unwrap();
}

#[test]
fn pmq_04_ctrl_click_ends_the_strip_with_a_triangle() {
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (0., 2.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::PenCaptureEnd(v(0.5, 3.))).unwrap();
    let draft = e.session.pen_draft.clone().unwrap();
    assert!(draft.strip_ended());
    // No more points once the triangle closes the end.
    assert!(e.dispatch(Command::PenCapture(v(0., 4.))).is_err());
    assert!(e.dispatch(Command::PenCaptureEnd(v(0., 4.))).is_err());
    assert_eq!(e.session.pen_draft.as_ref(), Some(&draft));
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    let sides: Vec<_> = m.faces.iter().map(|f| f.vertices.len()).collect();
    assert_eq!(sides, [4, 4, 3]);
    assert!(same_cycle(
        &corners(&m, 2),
        &[v(0., 2.), v(1., 2.), v(0.5, 3.)]
    ));
    assert!(m.normal(2).z > 0.99);
}

#[test]
fn ctrl_click_needs_a_starting_edge_and_make_quads() {
    let mut e = editor();
    start(&mut e, quads());
    click(&mut e, 0., 0.);
    assert!(e.dispatch(Command::PenCaptureEnd(v(1., 0.))).is_err());
    click(&mut e, 1., 0.);
    // Straight after the starting edge it makes a lone triangle.
    e.dispatch(Command::PenCaptureEnd(v(0.5, 1.))).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e).faces.len(), 1);

    let mut e = editor();
    start(&mut e, PenOptions::default());
    click(&mut e, 0., 0.);
    click(&mut e, 1., 0.);
    assert!(e.dispatch(Command::PenCaptureEnd(v(0.5, 1.))).is_err());
}

#[test]
fn undo_takes_back_one_click_and_the_strip_is_one_undo_step() {
    let mut e = editor();
    let before = e.scene.clone();
    let undo = e.history.undo.len();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (0., 2.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::PenCaptureEnd(v(0.5, 3.))).unwrap();
    let count = |e: &Editor| e.session.pen_draft.as_ref().unwrap().points().len();
    assert_eq!(count(&e), 7);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(count(&e), 6, "the triangle's point");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(count(&e), 4, "a click and its partner");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(count(&e), 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(count(&e), 1, "starting-edge points go one by one");
    click(&mut e, 1., 0.);
    click(&mut e, 0., 1.);
    assert_eq!(e.scene, before, "nothing is published while drawing");
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), undo + 1);
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

#[test]
fn cancel_and_short_drafts_leave_the_scene_alone() {
    let mut e = editor();
    let before = e.scene.clone();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.session.pen_draft.is_none() && e.scene == before);
    // A starting edge alone leaves its two points, as for the Pen.
    start(&mut e, quads());
    click(&mut e, 0., 0.);
    click(&mut e, 1., 0.);
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!((m.vertices.len(), m.faces.len()), (2, 0));
}

#[test]
fn make_quads_applies_to_polygon_types_without_walls() {
    // Inert for Vertices: clicks are single points.
    let mut e = editor();
    let vertices = PenOptions {
        output: PenType::Vertices,
        ..quads()
    };
    assert!(!vertices.strip());
    start(&mut e, vertices);
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        click(&mut e, x, y);
    }
    assert_eq!(e.session.pen_draft.as_ref().unwrap().points().len(), 3);
    e.dispatch(Command::Cancel).unwrap();
    // Wall Mode together with Make Quads isn't specified (PMQ-Q1).
    let both = PenOptions {
        wall: WallMode::BothSides,
        offset: 0.1,
        ..quads()
    };
    assert!(both.validate().is_err());
    // Subdivision strips are Subdivision polygons.
    let subdivs = PenOptions {
        output: PenType::Subdivs,
        ..quads()
    };
    let m = pen_geometry(
        &[
            v(0., 0.),
            v(1., 0.),
            v(0., 1.),
            v(1., 1.),
            v(0., 2.),
            v(1., 2.),
        ],
        &subdivs,
    )
    .unwrap();
    assert_eq!(m.faces.len(), 2);
    assert_eq!(
        m.attributes.representations,
        vec![PolygonRepresentation::Subdivs; 2]
    );
}

#[test]
fn make_quads_cant_change_mid_draft() {
    let mut e = editor();
    start(&mut e, quads());
    click(&mut e, 0., 0.);
    let draft = e.session.pen_draft.clone();
    assert!(
        e.dispatch(Command::PenOptions(PenOptions::default()))
            .is_err()
    );
    // Neither can a Type change that switches the strip off.
    let polyline = PenOptions {
        output: PenType::Polyline,
        ..quads()
    };
    assert!(e.dispatch(Command::PenOptions(polyline)).is_err());
    assert_eq!(e.session.pen_draft, draft);
    // Other options still change.
    let flipped = PenOptions {
        flip: true,
        ..quads()
    };
    e.dispatch(Command::PenOptions(flipped)).unwrap();
}

#[test]
fn dragging_flip_and_reverse_keep_the_strip_whole() {
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        click(&mut e, x, y);
    }
    // Drag the partner point (index 3) out.
    e.dispatch(Command::PenSelect(3)).unwrap();
    e.dispatch(Command::PenRefine(v(2., 1.))).unwrap();
    assert_eq!(
        e.session.pen_draft.as_ref().unwrap().points(),
        &[v(0., 0.), v(1., 0.), v(0., 1.), v(2., 1.)]
    );
    // The viewer-facing reverse flips the polygons, not the point pairs, so
    // the next click still gets its partner on the same side, across the
    // strip by the last rung as dragged (PMQ-Q4).
    e.dispatch(Command::PenReverse).unwrap();
    click(&mut e, 0., 2.);
    assert_eq!(e.session.pen_draft.as_ref().unwrap().points()[5], v(2., 2.));
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert!(m.normal(0).z < -0.99 && m.normal(1).z < -0.99);
    assert!(same_cycle(
        &corners(&m, 0),
        &[v(0., 1.), v(2., 1.), v(1., 0.), v(0., 0.)]
    ));
    // Flip Polygon reverses the strip too.
    let mut e = editor();
    start(
        &mut e,
        PenOptions {
            flip: true,
            ..quads()
        },
    );
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::Drop).unwrap();
    assert!(mesh(&e).normal(0).z < -0.99);
}

#[test]
fn the_strip_survives_save_and_reopen() {
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (0., 2.)] {
        click(&mut e, x, y);
    }
    e.dispatch(Command::Drop).unwrap();
    let after = mesh(&e);
    let path = std::env::temp_dir().join(format!("pen-make-quads-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    let reopened = Mesh::clone(&back.active().unwrap().mesh);
    assert_eq!(reopened, after);
}

/// Draws a Make Quads strip round the unit circle: the starting edge from
/// (1, 0) out to radius `1 + width` (inward for a negative width), then a
/// click every `step` degrees (negative: clockwise) up to `sweep`.
fn arc_strip(width: f64, step: f64, sweep: f64) -> Editor {
    let mut e = editor();
    start(&mut e, quads());
    click(&mut e, 1., 0.);
    click(&mut e, 1. + width, 0.);
    let mut a = step;
    while a.abs() <= sweep.abs() + 1e-9 {
        let t = a.to_radians();
        click(&mut e, t.cos(), t.sin());
        a += step;
    }
    e
}
/// Each guessed partner lies across the strip from its click, within one
/// step of the circle's radius, and every quad is convex, facing one way.
fn check_arc(e: &mut Editor, width: f64, step: f64) {
    let points = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    let lag = (step.abs() + 1.).to_radians().cos();
    for k in 1..points.len() / 2 {
        let (l, r) = (points[2 * k], points[2 * k + 1]);
        let across = (r - l).normalize().dot(l.normalize() * width.signum());
        assert!(across > lag, "pair {k}: {l} → {r} isn't across the strip");
        assert!(
            ((r - l).length() - width.abs()).abs() < 1e-9,
            "pair {k} keeps the width"
        );
    }
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(e);
    let facing = m.normal(0).z.signum();
    for f in 0..m.faces.len() {
        assert!(m.normal(f).z * facing > 0.99, "polygon {f} folds over");
        let c = corners(&m, f);
        for i in 0..c.len() {
            let (a, b, d) = (c[i], c[(i + 1) % c.len()], c[(i + 2) % c.len()]);
            let turn = (b - a).cross(d - b).z * facing;
            assert!(turn > 1e-6, "polygon {f} isn't convex at corner {i}: {c:?}");
        }
    }
    m.validate().unwrap();
}

#[test]
fn a_strip_curving_left_turns_its_partners() {
    // Counter-clockwise, rungs on the outside (the strip's right).
    let mut e = arc_strip(0.3, 20., 300.);
    check_arc(&mut e, 0.3, 20.);
    // Rungs on the inside (the strip's left).
    let mut e = arc_strip(-0.3, 20., 300.);
    check_arc(&mut e, -0.3, 20.);
}

#[test]
fn a_strip_curving_right_turns_its_partners() {
    let mut e = arc_strip(0.3, -20., -300.);
    check_arc(&mut e, 0.3, -20.);
    let mut e = arc_strip(-0.3, -15., -330.);
    check_arc(&mut e, -0.3, -15.);
}

#[test]
fn a_strip_that_bends_back_and_forth_follows_each_bend() {
    // An S: left, then right; the rung turns back with the strip.
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [
        (0., 0.),
        (0., -0.4),
        (1., 0.),
        (2., 0.5),
        (3., 0.5),
        (4., 0.),
    ] {
        click(&mut e, x, y);
    }
    let points = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    for k in 1..points.len() / 2 {
        let rung = points[2 * k + 1] - points[2 * k];
        assert!((rung.length() - 0.4).abs() < 1e-9);
    }
    // At (2, 0.5) the strip has turned up by the step (1, 0.5)'s angle.
    let up = (0.5f64).atan2(1.);
    let rung = points[5] - points[4];
    close_to(rung, DVec3::new(up.sin(), -up.cos(), 0.) * 0.4);
    // Back level at (3, 0.5): the rung points straight down again.
    close_to(points[7] - points[6], DVec3::new(0., -0.4, 0.));
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    let facing = m.normal(0).z.signum();
    assert!((0..m.faces.len()).all(|f| m.normal(f).z * facing > 0.99));
}

fn close_to(a: DVec3, b: DVec3) {
    assert!(a.distance(b) < 1e-9, "{a} != {b}");
}

#[test]
fn the_drag_after_a_click_carries_its_partner() {
    let mut e = editor();
    start(&mut e, quads());
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        click(&mut e, x, y);
    }
    // Still the click's drag: the partner is guessed again from here.
    e.dispatch(Command::PenRefine(v(0., 2.))).unwrap();
    let points = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    assert_eq!(&points[2..], &[v(0., 2.), v(1., 2.)]);
    // One undo step for the click and its drag.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.session.pen_draft.as_ref().unwrap().points().len(), 2);
    // A turning click's drag turns the partner too.
    click(&mut e, 0., 1.);
    click(&mut e, -1., 1.);
    // Clicked at (-1, 1) the strip turns 90°: the rung (1, 0) turns to (0, 1).
    close_to(
        e.session.pen_draft.as_ref().unwrap().points()[5],
        v(-1., 2.),
    );
    // Dragged to (-1, 2) it turns only 45°.
    e.dispatch(Command::PenRefine(v(-1., 2.))).unwrap();
    let points = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    let half = 0.5f64.sqrt();
    let guessed = v(-1. + half, 2. + half);
    close_to(points[5], guessed);
    // A later drag of the same point leaves its partner alone (PMQ-Q4).
    e.dispatch(Command::PenSelect(4)).unwrap();
    e.dispatch(Command::PenRefine(v(-1., 3.))).unwrap();
    let after = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    assert_eq!(after[4], v(-1., 3.));
    close_to(after[5], guessed);
}
