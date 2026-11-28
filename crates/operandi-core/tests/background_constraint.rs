//! Constrain to Background (SPEC-BACKGROUND-CONSTRAINT v1): the numeric
//! acceptance cases BG-01..04 plus the tutorial's Pen and Element Move
//! steps, through the editor.
use glam::DVec3;
use operandi_core::background_constraint::{
    BackgroundConstraint, ConstraintView, GeometryConstraint,
};
use operandi_core::*;

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
fn mesh(points: Vec<DVec3>, polygons: Vec<Vec<u32>>) -> Mesh {
    Mesh {
        vertices: points,
        faces: polygons.into_iter().map(face).collect(),
        ..Default::default()
    }
}
/// A 2 m quad at z = 0 facing +Z.
fn quad() -> Mesh {
    mesh(
        vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        vec![vec![0, 1, 2, 3]],
    )
}
/// A unit UV sphere at the origin, poles on ±Z, facing out.
fn sphere() -> Mesh {
    let (rings, segs) = (12u32, 16u32);
    let mut v = vec![DVec3::Z];
    for r in 1..rings {
        let t = std::f64::consts::PI * r as f64 / rings as f64;
        for s in 0..segs {
            let a = std::f64::consts::TAU * s as f64 / segs as f64;
            v.push(DVec3::new(t.sin() * a.cos(), t.sin() * a.sin(), t.cos()));
        }
    }
    v.push(-DVec3::Z);
    let ring = |r: u32, s: u32| 1 + (r - 1) * segs + s % segs;
    let south = v.len() as u32 - 1;
    let mut f = Vec::new();
    for s in 0..segs {
        f.push(vec![0, ring(1, s), ring(1, s + 1)]);
        f.push(vec![south, ring(rings - 1, s + 1), ring(rings - 1, s)]);
    }
    for r in 1..rings - 1 {
        for s in 0..segs {
            f.push(vec![
                ring(r, s),
                ring(r + 1, s),
                ring(r + 1, s + 1),
                ring(r, s + 1),
            ]);
        }
    }
    mesh(v, f)
}
/// A background mesh item and a foreground item holding `points` (one
/// triangle), the foreground active, Vertex mode. Returns (editor, fg id).
fn scene(background: Mesh, points: [DVec3; 3]) -> (Editor, u64, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = background.into();
    let bg = e.scene.active().unwrap().id;
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(points.to_vec(), vec![vec![0, 1, 2]]).into();
    let fg = e.scene.active().unwrap().id;
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    (e, bg, fg)
}
fn pick(e: &mut Editor, item: u64, v: u32) {
    e.dispatch(Command::Select {
        item,
        hit: Component::Vertex(v),
        replace: true,
        remove: false,
    })
    .unwrap();
}
fn point(e: &Editor, id: u64, v: usize) -> DVec3 {
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    e.scene
        .world_matrix(id)
        .unwrap()
        .transform_point3(item.mesh.vertices[v])
}
fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-9, "{a} != {b}");
}
fn on(e: &mut Editor, c: BackgroundConstraint) {
    e.dispatch(Command::BackgroundConstraint(Some(c))).unwrap();
}
fn move_by(e: &mut Editor, delta: DVec3) {
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
}
const TRI: [DVec3; 3] = [
    DVec3::new(0.3, 0.2, 5.),
    DVec3::new(5., 5., 5.),
    DVec3::new(6., 5., 5.),
];

#[test]
fn bg01_screen_axis_move_lands_on_the_quad() {
    let (mut e, _, fg) = scene(quad(), TRI);
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    e.set_constraint_view(ConstraintView::default());
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 0.));
    // Points the tool doesn't move are left alone.
    close(point(&e, fg, 1), TRI[1]);
    e.dispatch(Command::Drop).unwrap();
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 0.));
    // One undo step puts it back.
    e.dispatch(Command::Undo).unwrap();
    close(point(&e, fg, 0), TRI[0]);
}

#[test]
fn bg02_offset_rests_off_the_surface() {
    let (mut e, _, fg) = scene(quad(), TRI);
    pick(&mut e, fg, 0);
    on(
        &mut e,
        BackgroundConstraint {
            offset: 0.05,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 0.05));
}

#[test]
fn bg03_a_miss_stays_where_move_put_it() {
    let (mut e, _, fg) = scene(quad(), TRI);
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    move_by(&mut e, DVec3::new(2.7, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(3., 0.2, 5.));
}

#[test]
fn bg04_point_mode_holds_to_the_nearest_surface() {
    let (mut e, _, fg) = scene(
        sphere(),
        [
            DVec3::new(0., 0., 2.),
            DVec3::new(5., 5., 5.),
            DVec3::new(6., 5., 5.),
        ],
    );
    pick(&mut e, fg, 0);
    on(
        &mut e,
        BackgroundConstraint {
            mode: GeometryConstraint::Point,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(0., 0., 1.));
    close(point(&e, fg, 0), DVec3::new(0., 0., 1.));
}

#[test]
fn turning_it_on_keeps_the_tool_and_reapplies() {
    let (mut e, _, fg) = scene(quad(), TRI);
    pick(&mut e, fg, 0);
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 5.));
    on(&mut e, BackgroundConstraint::default());
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Move));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 0.));
    // Off again: the live tool moves the point freely.
    e.dispatch(Command::BackgroundConstraint(None)).unwrap();
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Move));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 5.));
    // The options come back next time.
    assert_eq!(
        e.background_constraint_preset,
        BackgroundConstraint::default()
    );
}

#[test]
fn mode_off_and_item_mode_do_nothing() {
    let (mut e, _, fg) = scene(quad(), TRI);
    pick(&mut e, fg, 0);
    on(
        &mut e,
        BackgroundConstraint {
            mode: GeometryConstraint::Off,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 5.));
    e.dispatch(Command::Drop).unwrap();
    on(&mut e, BackgroundConstraint::default());
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    // The item moved, its points kept their height.
    close(point(&e, fg, 0), DVec3::new(0.5, 0.2, 5.));
}

#[test]
fn element_move_slides_over_the_background() {
    let (mut e, _, fg) = scene(
        sphere(),
        [
            DVec3::new(0., 0., 1.),
            DVec3::new(5., 5., 5.),
            DVec3::new(6., 5., 5.),
        ],
    );
    on(&mut e, BackgroundConstraint::default());
    // A perspective view in front of the sphere.
    e.set_constraint_view(ConstraintView {
        forward: -DVec3::Z,
        eye: Some(DVec3::new(0., 0., 10.)),
    });
    e.dispatch(Command::Activate(Tool::Tweak)).unwrap();
    for step in 1..=5 {
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.tweak = Some(TweakGrab {
            item: fg,
            element: TweakElement::Vertex(0),
        });
        p.delta = DVec3::new(0.1 * step as f64, 0., 0.);
        e.dispatch(Command::Preview(p)).unwrap();
        let q = point(&e, fg, 0);
        // On the sphere's front (its polygons sit just inside the unit
        // sphere) and never left behind.
        assert!(q.length() <= 1. + 1e-9 && q.length() > 0.97, "{q}");
        assert!(q.z > 0. && q.x > 0.05 * step as f64, "{q}");
    }
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn pen_points_land_on_the_background() {
    let (mut e, _, fg) = scene(quad(), TRI);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    on(&mut e, BackgroundConstraint::default());
    e.set_constraint_view(ConstraintView::default());
    e.dispatch(Command::PenStart).unwrap();
    // Drawn on a work plane above the quad, in the Front view.
    for p in [
        DVec3::new(0., 0., 2.),
        DVec3::new(0.5, 0., 2.),
        DVec3::new(0.5, 0.5, 2.),
    ] {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    assert!(
        e.session
            .pen_draft
            .as_ref()
            .unwrap()
            .points()
            .iter()
            .all(|p| p.z.abs() < 1e-12)
    );
    // A dragged point stays on it too.
    e.dispatch(Command::PenRefine(DVec3::new(0.6, 0.6, 3.)))
        .unwrap();
    close(
        e.session.pen_draft.as_ref().unwrap().points()[2],
        DVec3::new(0.6, 0.6, 0.),
    );
    e.dispatch(Command::Drop).unwrap();
    let item = e.scene.items.iter().find(|i| i.id == fg).unwrap();
    assert_eq!(item.mesh.vertices.len(), 6);
    assert!(item.mesh.vertices[3..].iter().all(|p| p.z.abs() < 1e-12));
}

#[test]
fn hidden_and_active_meshes_are_not_background() {
    let (mut e, bg, fg) = scene(quad(), TRI);
    // Both selected: no background left.
    e.dispatch(Command::SelectItems(vec![fg, bg])).unwrap();
    assert!(background_constraint::background_ids(&e.scene).is_empty());
    e.dispatch(Command::SelectItems(vec![fg])).unwrap();
    assert_eq!(background_constraint::background_ids(&e.scene), vec![bg]);
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == bg)
        .unwrap()
        .visible = false;
    assert!(background_constraint::background_ids(&e.scene).is_empty());
}

#[test]
fn bad_options_are_refused_and_change_nothing() {
    let (mut e, _, _) = scene(quad(), TRI);
    let before = e.background_constraint;
    assert!(
        e.dispatch(Command::BackgroundConstraint(Some(BackgroundConstraint {
            offset: f64::NAN,
            ..Default::default()
        })))
        .is_err()
    );
    assert_eq!(e.background_constraint, before);
}

#[test]
fn vector_mode_follows_the_point_s_motion() {
    // A wall at x = 1 facing −X; the point moves along +X past it.
    let wall = mesh(
        vec![
            DVec3::new(1., -1., -1.),
            DVec3::new(1., -1., 1.),
            DVec3::new(1., 1., 1.),
            DVec3::new(1., 1., -1.),
        ],
        vec![vec![0, 1, 2, 3]],
    );
    let (mut e, _, fg) = scene(
        wall,
        [DVec3::ZERO, DVec3::new(5., 5., 5.), DVec3::new(6., 5., 5.)],
    );
    pick(&mut e, fg, 0);
    on(
        &mut e,
        BackgroundConstraint {
            mode: GeometryConstraint::Vector,
            double_sided: true,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(1.5, 0.2, 0.));
    let q = point(&e, fg, 0);
    assert!((q.x - 1.).abs() < 1e-9, "{q}");
}

/// Opt-in cost check: a Move of 1,600 points over a 32k-triangle
/// background, per frame after the first (the background is cached).
#[test]
#[ignore]
fn drag_frame_cost_probe() {
    frame_cost(None);
}

/// A Catmull-Clark background at level 3: 16k cage quads, about 4.2M
/// smooth triangles. The first frame builds the background; the rest only
/// pay for rays.
#[test]
#[ignore]
fn subd_drag_frame_cost_probe() {
    frame_cost(Some(3));
}

fn frame_cost(subd_level: Option<u8>) {
    let (rings, segs) = (128u32, 128u32);
    let mut v = Vec::new();
    let mut f = Vec::new();
    for r in 0..=rings {
        for s in 0..=segs {
            let (x, y) = (
                s as f64 / segs as f64 * 4. - 2.,
                r as f64 / rings as f64 * 4. - 2.,
            );
            v.push(DVec3::new(x, y, (x * 2.).sin() * (y * 3.).cos() * 0.3));
        }
    }
    let at = |r: u32, s: u32| r * (segs + 1) + s;
    for r in 0..rings {
        for s in 0..segs {
            f.push(vec![at(r, s), at(r, s + 1), at(r + 1, s + 1), at(r + 1, s)]);
        }
    }
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut background = mesh(v, f);
    if let Some(level) = subd_level {
        background.attributes.representations =
            vec![PolygonRepresentation::CatmullClark; background.faces.len()];
        e.scene.active_mut().unwrap().subdivision = Some(subdivision::SubdivisionSettings {
            catmull_clark_level: level,
            ..Default::default()
        });
    }
    e.scene.active_mut().unwrap().mesh = background.into();
    e.dispatch(Command::NewMesh).unwrap();
    let mut pts = Vec::new();
    let mut polys = Vec::new();
    for i in 0..40u32 {
        for j in 0..40u32 {
            pts.push(DVec3::new(i as f64 * 0.09 - 1.8, j as f64 * 0.09 - 1.8, 2.));
            if i < 39 && j < 39 {
                polys.push(vec![
                    i * 40 + j,
                    (i + 1) * 40 + j,
                    (i + 1) * 40 + j + 1,
                    i * 40 + j + 1,
                ]);
            }
        }
    }
    e.scene.active_mut().unwrap().mesh = mesh(pts, polys).into();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    on(&mut e, BackgroundConstraint::default());
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut frames = Vec::new();
    for k in 0..30 {
        let t = std::time::Instant::now();
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::new(0.001 * k as f64, 0., 0.),
            ..e.session.live.as_ref().unwrap().params.clone()
        }))
        .unwrap();
        frames.push(t.elapsed().as_secs_f64() * 1e3);
    }
    let first = frames[0];
    frames.sort_by(f64::total_cmp);
    println!(
        "background constraint drag frame ({subd_level:?}): first {first:.0} ms, median {:.2} ms, max of rest {:.2} ms",
        frames[15], frames[28]
    );
}

#[test]
fn target_item_limits_the_background_to_one_item() {
    let (mut e, low, fg) = scene(quad(), TRI);
    // A second background quad above the first, nearer the viewer.
    e.dispatch(Command::NewMesh).unwrap();
    let mut high = quad();
    for v in &mut high.vertices {
        v.z = 2.;
    }
    e.scene.active_mut().unwrap().mesh = high.into();
    e.dispatch(Command::SelectItems(vec![fg])).unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    move_by(&mut e, DVec3::new(0.1, 0., 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 2.));
    on(
        &mut e,
        BackgroundConstraint {
            target: Some(low),
            ..Default::default()
        },
    );
    close(point(&e, fg, 0), DVec3::new(0.4, 0.2, 0.));
}

#[test]
fn make_quads_partners_land_on_the_background() {
    let (mut e, _, fg) = scene(quad(), TRI);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    on(&mut e, BackgroundConstraint::default());
    e.set_constraint_view(ConstraintView::default());
    e.dispatch(Command::PenStart).unwrap();
    let mut o = e.session.pen_draft.as_ref().unwrap().options();
    o.make_quads = true;
    e.dispatch(Command::PenOptions(o)).unwrap();
    // Starting edge drawn at two heights; the next click's partner is
    // offset by that edge, then pushed onto the quad.
    e.dispatch(Command::PenCapture(DVec3::new(0., 0., 2.)))
        .unwrap();
    e.dispatch(Command::PenCapture(DVec3::new(0., 0.3, 3.)))
        .unwrap();
    e.dispatch(Command::PenCapture(DVec3::new(0.3, 0., 2.)))
        .unwrap();
    e.dispatch(Command::PenCaptureEnd(DVec3::new(0.6, 0.1, 4.)))
        .unwrap();
    let pts = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    assert_eq!(pts.len(), 5);
    assert!(pts.iter().all(|p| p.z.abs() < 1e-12), "{pts:?}");
    close(pts[3], DVec3::new(0.3, 0.3, 0.));
    e.dispatch(Command::Drop).unwrap();
    let item = e.scene.items.iter().find(|i| i.id == fg).unwrap();
    assert!(item.mesh.vertices[3..].iter().all(|p| p.z.abs() < 1e-12));
}

#[test]
fn make_quads_round_an_eye_keeps_its_partners_across_the_strip() {
    // Retopology round a curved eye: a strip drawn from the front round the
    // pole of a sphere, the starting rung pointing out from the pole. Each
    // guessed partner turns with the strip, lies on the sphere and stays on
    // the outer side; every quad faces out of the sphere without folding.
    // Both on plain polygons and on the smooth surface of a SubD eye.
    let mut subd = sphere();
    subd.attributes.representations = vec![PolygonRepresentation::Subdivs; subd.faces.len()];
    for eye in [sphere(), subd] {
        round_an_eye(eye);
    }
}
fn round_an_eye(eye: Mesh) {
    let (mut e, _, fg) = scene(eye, TRI);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    on(&mut e, BackgroundConstraint::default());
    e.set_constraint_view(ConstraintView::default());
    e.dispatch(Command::PenStart).unwrap();
    let mut o = e.session.pen_draft.as_ref().unwrap().options();
    o.make_quads = true;
    e.dispatch(Command::PenOptions(o)).unwrap();
    let at = |r: f64, degrees: f64| {
        let t = degrees.to_radians();
        DVec3::new(r * t.cos(), r * t.sin(), 3.)
    };
    e.dispatch(Command::PenCapture(at(0.4, 0.))).unwrap();
    e.dispatch(Command::PenCapture(at(0.6, 0.))).unwrap();
    for k in 1..=16 {
        e.dispatch(Command::PenCapture(at(0.4, 20. * k as f64)))
            .unwrap();
    }
    let pts = e.session.pen_draft.as_ref().unwrap().points().to_vec();
    let sphere_radius = |p: DVec3| p.length();
    for k in 1..pts.len() / 2 {
        let (l, r) = (pts[2 * k], pts[2 * k + 1]);
        // On the faceted unit sphere (between its flats and the sphere).
        for p in [l, r] {
            assert!(
                (0.85..=1. + 1e-9).contains(&sphere_radius(p)),
                "{p} is off the eye"
            );
        }
        // Out from the pole, as the starting rung was: within one click's
        // turn (the first guess can't know the curve yet) plus the facets.
        let out = DVec3::new(l.x, l.y, 0.).normalize();
        let rung = DVec3::new(r.x - l.x, r.y - l.y, 0.);
        assert!(
            rung.normalize().dot(out) > 25f64.to_radians().cos(),
            "pair {k}: {l} → {r} points along or into the strip"
        );
        assert!(rung.length() > 0.1, "pair {k} collapsed");
    }
    e.dispatch(Command::Drop).unwrap();
    let item = e.scene.items.iter().find(|i| i.id == fg).unwrap();
    let m = &item.mesh;
    for f in 1..m.faces.len() {
        let c: Vec<DVec3> = m.faces[f]
            .vertices
            .iter()
            .map(|&v| m.vertices[v as usize])
            .collect();
        let centre = c.iter().sum::<DVec3>() / c.len() as f64;
        let n = m.normal(f);
        let facing = n.dot(centre.normalize());
        assert!(facing.abs() > 0.8, "polygon {f} stands on edge: {c:?}");
        for i in 0..c.len() {
            let (a, b, d) = (c[i], c[(i + 1) % c.len()], c[(i + 2) % c.len()]);
            let turn = (b - a).cross(d - b).dot(centre.normalize()) * facing.signum();
            assert!(turn > 0., "polygon {f} folds at corner {i}: {c:?}");
        }
    }
}

/// A 2 m cube at the origin, facing out; every polygon `kind`.
fn cube(kind: PolygonRepresentation) -> Mesh {
    let mut m = mesh(
        vec![
            DVec3::new(-1., -1., -1.),
            DVec3::new(1., -1., -1.),
            DVec3::new(1., 1., -1.),
            DVec3::new(-1., 1., -1.),
            DVec3::new(-1., -1., 1.),
            DVec3::new(1., -1., 1.),
            DVec3::new(1., 1., 1.),
            DVec3::new(-1., 1., 1.),
        ],
        vec![
            vec![4, 5, 6, 7],
            vec![0, 3, 2, 1],
            vec![1, 2, 6, 5],
            vec![0, 4, 7, 3],
            vec![3, 7, 6, 2],
            vec![0, 1, 5, 4],
        ],
    );
    m.attributes.representations = vec![kind; 6];
    m
}
/// The height where a ray straight down −Z at (x, y) first meets the
/// displayed smooth surface of `m` (brute force over its pieces, fanned
/// as drawn), counting only pieces that face +Z.
fn smooth_top(m: &Mesh, settings: &subdivision::SubdivisionSettings, x: f64, y: f64) -> f64 {
    let (surface, _) = subdivision::display_surface(m, settings).unwrap();
    let mut best = f64::NEG_INFINITY;
    for f in &surface.faces {
        let v: Vec<DVec3> = f
            .vertices
            .iter()
            .map(|&i| surface.vertices[i as usize])
            .collect();
        for i in 1..v.len() - 1 {
            let (a, b, c) = (v[0], v[i], v[i + 1]);
            let n = (b - a).cross(c - a);
            if n.z <= 0. {
                continue;
            }
            // Barycentric coordinates of (x, y) in the triangle's top view.
            let d = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
            let u = ((x - a.x) * (c.y - a.y) - (c.x - a.x) * (y - a.y)) / d;
            let w = ((b.x - a.x) * (y - a.y) - (x - a.x) * (b.y - a.y)) / d;
            if u >= -1e-12 && w >= -1e-12 && u + w <= 1. + 1e-12 {
                best = best.max(a.z + u * (b.z - a.z) + w * (c.z - a.z));
            }
        }
    }
    best
}
fn background_item(e: &mut Editor, bg: u64) -> &mut Item {
    e.scene.items.iter_mut().find(|i| i.id == bg).unwrap()
}

/// BG-Q5 (answered 2026-11-15): a SubD background is its smooth surface.
/// Near a corner the rounded surface sits well inside the cage.
#[test]
fn subd_background_lands_on_the_smooth_surface_not_the_cage() {
    for kind in [
        PolygonRepresentation::Subdivs,
        PolygonRepresentation::CatmullClark,
    ] {
        let (mut e, bg, fg) = scene(cube(kind), TRI);
        pick(&mut e, fg, 0);
        on(&mut e, BackgroundConstraint::default());
        e.set_constraint_view(ConstraintView::default());
        move_by(&mut e, DVec3::new(0.1, 0.2, 0.));
        let p = point(&e, fg, 0);
        let item = background_item(&mut e, bg);
        let settings = item.subdivision.clone().unwrap_or_default();
        let want = smooth_top(&item.mesh, &settings, 0.4, 0.4);
        assert!(1. - p.z > 0.05, "{kind:?}: {p} is on the cage");
        assert!(
            (p - DVec3::new(0.4, 0.4, want)).length() < 1e-9,
            "{kind:?}: {p} vs {want}"
        );
    }
}

/// The same cube with SubD off is still the cage.
#[test]
fn face_background_still_lands_on_the_cage() {
    let (mut e, _, fg) = scene(cube(PolygonRepresentation::Face), TRI);
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    move_by(&mut e, DVec3::new(0.1, 0.2, 0.));
    close(point(&e, fg, 0), DVec3::new(0.4, 0.4, 1.));
}

/// Offset follows the smooth surface's normal, and Point mode finds the
/// nearest smooth point.
#[test]
fn subd_offset_and_point_follow_the_smooth_surface() {
    let (mut e, _, fg) = scene(cube(PolygonRepresentation::CatmullClark), TRI);
    pick(&mut e, fg, 0);
    on(
        &mut e,
        BackgroundConstraint {
            offset: 0.1,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(0.1, 0.2, 0.));
    let with_offset = point(&e, fg, 0);
    e.dispatch(Command::Cancel).unwrap();
    on(&mut e, BackgroundConstraint::default());
    move_by(&mut e, DVec3::new(0.1, 0.2, 0.));
    let on_surface = point(&e, fg, 0);
    let step = with_offset - on_surface;
    // A tenth of a metre, tilted off +Z toward the corner it rounds.
    assert!((step.length() - 0.1).abs() < 1e-9, "{step}");
    assert!(step.x > 1e-3 && step.y > 1e-3 && step.z > 0.05, "{step}");
    // Point: the nearest smooth point to a spot just outside the corner
    // is well inside the cage corner.
    e.dispatch(Command::Cancel).unwrap();
    on(
        &mut e,
        BackgroundConstraint {
            mode: GeometryConstraint::Point,
            ..Default::default()
        },
    );
    move_by(&mut e, DVec3::new(0.9, 0.9, -3.8));
    let p = point(&e, fg, 0);
    assert!((p - DVec3::ONE).length() > 0.2, "{p}");
    assert!(p.max_element() < 1., "{p}");
}

/// BG-Q1 holds on the smooth surface: pieces of a hidden cage polygon
/// don't count.
#[test]
fn hidden_cage_polygons_hide_their_smooth_pieces() {
    let (mut e, bg, fg) = scene(cube(PolygonRepresentation::CatmullClark), TRI);
    background_item(&mut e, bg).elements.hidden.faces.insert(0);
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    move_by(&mut e, DVec3::new(-0.3, -0.2, 0.));
    // Through the hidden top: only the bottom, facing away, is left.
    close(point(&e, fg, 0), DVec3::new(0., 0., 5.));
}

/// The cached background is rebuilt when SubD is toggled or the level
/// changes, and reused otherwise.
#[test]
fn cache_rebuilds_on_subd_toggle_and_level_change() {
    let (mut e, bg, _) = scene(cube(PolygonRepresentation::Subdivs), TRI);
    let cache = background_constraint::BackgroundCache::default();
    let c = BackgroundConstraint::default();
    let view = ConstraintView::default();
    let at = DVec3::new(0.4, 0.4, 5.);
    let height = |e: &Editor| {
        cache
            .get(&e.scene, None)
            .unwrap()
            .constrain(&c, &view, None, at)
            .unwrap()
            .z
    };
    let level2 = height(&e);
    let _ = height(&e);
    assert_eq!(cache.builds(), 1);
    // SubD off: the cage.
    background_item(&mut e, bg).mesh = cube(PolygonRepresentation::Face).into();
    assert!((height(&e) - 1.).abs() < 1e-12);
    assert_eq!(cache.builds(), 2);
    // Back on, at level 3.
    let item = background_item(&mut e, bg);
    item.mesh = cube(PolygonRepresentation::Subdivs).into();
    item.subdivision = Some(subdivision::SubdivisionSettings {
        subdivision_level: 3,
        ..Default::default()
    });
    let settings = item.subdivision.clone().unwrap();
    let want = smooth_top(&item.mesh, &settings, 0.4, 0.4);
    let level3 = height(&e);
    assert_eq!(cache.builds(), 3);
    assert!((level3 - want).abs() < 1e-9, "{level3} vs {want}");
    assert!((level3 - level2).abs() > 1e-6, "{level3} vs {level2}");
    let _ = height(&e);
    assert_eq!(cache.builds(), 3);
}

/// MC-07 acceptance: a SubD cube with a 1 m cage at display level 2; a
/// point moved over the centre of the front face (Screen Axis, front view)
/// lands on the smooth surface, nearer the centre than the cage's 0.5 m.
#[test]
fn mc07_front_centre_lands_on_the_level_2_surface() {
    let mut half = cube(PolygonRepresentation::Subdivs);
    for v in half.vertices.iter_mut() {
        *v *= 0.5;
    }
    let (mut e, bg, fg) = scene(half, TRI);
    pick(&mut e, fg, 0);
    on(&mut e, BackgroundConstraint::default());
    e.set_constraint_view(ConstraintView::default());
    move_by(&mut e, DVec3::new(-0.3, -0.2, 0.));
    let p = point(&e, fg, 0);
    let item = background_item(&mut e, bg);
    let want = smooth_top(&item.mesh, &Default::default(), 0., 0.);
    assert!(p.z < 0.5 - 1e-3, "{p}");
    assert!(
        (p - DVec3::new(0., 0., want)).length() < 1e-4,
        "{p} vs {want}"
    );
}
