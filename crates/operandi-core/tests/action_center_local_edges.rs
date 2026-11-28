//! Action Center Local on separate selected edges, points and polygons
//! (the "Model the Mug" tutorial, 2026-11-21; specifications from the
//! action-centers rulebook AC-73..75, tests AC-T73a/b/c, AC-T74, AC-T75).
//! Each edge is its own cluster: the tool axis nearest it is turned onto
//! it, so a Scale along that axis lengthens it along itself about its own
//! middle.
use glam::DVec3;
use operandi_core::*;

const SIDES: u32 = 8;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An open cylinder of radius 1 with four rings at y = 0, 1, 2, 3 (point
/// `ring * SIDES + side`), so the middle row's vertical edges run from
/// y = 1 to y = 2.
fn mug(center: CenterMode) -> Editor {
    tapered_mug(center, 0.)
}

/// The same cylinder widening by `taper` per ring, so its side edges lean
/// outward (a real mug's wall).
fn tapered_mug(center: CenterMode, taper: f64) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut vertices = Vec::new();
    for ring in 0..4 {
        for side in 0..SIDES {
            let a = side as f64 / SIDES as f64 * std::f64::consts::TAU;
            let r = 1. + taper * ring as f64;
            vertices.push(DVec3::new(r * a.cos(), ring as f64, r * a.sin()));
        }
    }
    let mut faces = Vec::new();
    for ring in 0..3 {
        for side in 0..SIDES {
            let (a, b) = (ring * SIDES + side, ring * SIDES + (side + 1) % SIDES);
            faces.push(quad([a, b, b + SIDES, a + SIDES]));
        }
    }
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
    .into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    // Side 0 sits at x = +1, side 4 at x = −1 (both on the XY plane).
    e.scene.selection.edges = vec![(SIDES, 2 * SIDES), (SIDES + 4, 2 * SIDES + 4)];
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        // Local by name is Local's centre with Local's axes.
        axis: if center == CenterMode::Local {
            AxisOrientation::Local
        } else {
            AxisOrientation::World
        },
    }))
    .unwrap();
    e
}

fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}

/// Activates `tool`, previews `delta` (the live result) and returns the
/// points during the preview; the caller drops or cancels.
fn preview(e: &mut Editor, tool: Tool, delta: DVec3) -> Vec<DVec3> {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = delta;
    e.dispatch(Command::Preview(p)).unwrap();
    points(e)
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!((a - b).length() < 1e-9, "{what}: {a:?} != {b:?}");
}

#[test]
fn local_scale_y_lengthens_each_edge_about_its_own_centre() {
    let mut e = mug(CenterMode::Local);
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::new(1., 1.8, 1.));
    e.dispatch(Command::Drop).unwrap();
    let after = points(&e);
    for side in [0, 4] {
        let (lo, hi) = ((SIDES + side) as usize, (2 * SIDES + side) as usize);
        let c = (before[lo] + before[hi]) / 2.;
        for (v, what) in [(lo, "bottom"), (hi, "top")] {
            let want = c + (before[v] - c) * DVec3::new(1., 1.8, 1.);
            close(during[v], want, &format!("preview side {side} {what}"));
            close(after[v], want, &format!("dropped side {side} {what}"));
        }
        let length = (after[hi] - after[lo]).length();
        assert!(
            (length - 1.8).abs() < 1e-9,
            "side {side} edge length {length}"
        );
    }
    // Nothing else moved.
    for (i, (&p, &q)) in before.iter().zip(&after).enumerate() {
        if ![8, 16, 12, 20].contains(&i) {
            close(q, p, &format!("unselected point {i}"));
        }
    }
}

/// Move and Rotate use the same frames: each edge moves along the handle's
/// own axes and turns about its own centre.
#[test]
fn local_move_and_rotate_act_on_each_edge_in_the_tool_axes() {
    let mut e = mug(CenterMode::Local);
    let before = points(&e);
    let moved = preview(&mut e, Tool::Move, DVec3::new(0.5, 0., 0.));
    e.dispatch(Command::Cancel).unwrap();
    for v in [8, 16, 12, 20] {
        close(
            moved[v],
            before[v] + DVec3::new(0.5, 0., 0.),
            &format!("moved {v}"),
        );
    }
    close(points(&e)[8], before[8], "cancel restores");
    let turned = preview(&mut e, Tool::Rotate, DVec3::new(0., 0., 90.));
    e.dispatch(Command::Drop).unwrap();
    for (lo, hi) in [(8, 16), (12, 20)] {
        let c = (before[lo] + before[hi]) / 2.;
        // A quarter turn about Z lays each vertical edge flat about its own
        // middle: the bottom end goes to +X of it, the top end to −X.
        close(
            turned[lo],
            c + DVec3::new(0.5, 0., 0.),
            &format!("turned {lo}"),
        );
        close(
            turned[hi],
            c + DVec3::new(-0.5, 0., 0.),
            &format!("turned {hi}"),
        );
    }
    // Undo puts the edges back.
    e.dispatch(Command::Undo).unwrap();
    for v in [8, 16, 12, 20] {
        close(points(&e)[v], before[v], &format!("undone {v}"));
    }
}

/// Scale along X, which the line has no size in, leaves each edge where it
/// is (it scales about its own centre, not the selection's).
#[test]
fn local_scale_x_keeps_each_edge_in_place() {
    let mut e = mug(CenterMode::Local);
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::new(2., 1., 1.));
    e.dispatch(Command::Drop).unwrap();
    for v in [8, 16, 12, 20] {
        close(during[v], before[v], &format!("point {v}"));
    }
}

/// AC-T73a: two leaning side edges of the tapered tube, Scale Y 200%:
/// each doubles along its own slant about its own middle; a Y rotation
/// spins each about itself, so it stays put.
#[test]
fn ac_t73a_scale_y_stretches_leaning_edges_along_themselves() {
    let mut e = tapered_mug(CenterMode::Local, -0.13);
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::new(1., 2., 1.));
    e.dispatch(Command::Drop).unwrap();
    for (lo, hi) in [(8, 16), (12, 20)] {
        let c = (before[lo] + before[hi]) / 2.;
        for v in [lo, hi] {
            close(during[v], c + (before[v] - c) * 2., &format!("point {v}"));
        }
        let (old, new) = (before[hi] - before[lo], during[hi] - during[lo]);
        assert!(
            (new.length() - old.length() * 2.).abs() < 1e-9,
            "edge {lo}-{hi} length"
        );
        assert!(
            new.normalize().dot(old.normalize()) > 1. - 1e-12,
            "slant kept"
        );
    }
    let spun = preview(&mut e, Tool::Rotate, DVec3::new(0., 40., 0.));
    e.dispatch(Command::Cancel).unwrap();
    let now = points(&e);
    for v in [8, 16, 12, 20] {
        close(spun[v], now[v], &format!("spun {v}"));
    }
}

/// AC-T73b: the same edges, Scale X 200% (across the lines): nothing
/// changes.
#[test]
fn ac_t73b_scale_across_the_edges_does_nothing() {
    let mut e = tapered_mug(CenterMode::Local, -0.13);
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::new(2., 1., 2.));
    for v in [8, 16, 12, 20] {
        close(during[v], before[v], &format!("point {v}"));
    }
}

/// AC-T73c: a level rim edge running mostly along X turns the tool's X onto
/// itself: Scale X 200% doubles it along itself about its middle, and Scale
/// Y does nothing to it.
#[test]
fn ac_t73c_level_edge_stretches_on_the_nearest_axis() {
    let mut e = mug(CenterMode::Local);
    let (a, b) = ((SIDES + 2) as usize, (SIDES + 3) as usize);
    e.scene.selection.edges = vec![(a as u32, b as u32)];
    let before = points(&e);
    let along = (before[b] - before[a]).normalize();
    assert!(
        along.x.abs() > along.z.abs(),
        "fixture edge is mostly along X"
    );
    let flat = preview(&mut e, Tool::Scale, DVec3::new(1., 2., 1.));
    e.dispatch(Command::Cancel).unwrap();
    close(flat[a], before[a], "Scale Y leaves it");
    let during = preview(&mut e, Tool::Scale, DVec3::new(2., 1., 1.));
    let c = (before[a] + before[b]) / 2.;
    close(during[a], c + (before[a] - c) * 2., "end a");
    close(during[b], c + (before[b] - c) * 2., "end b");
}

/// AC-T74: two lone points under Local: Scale moves nothing.
#[test]
fn ac_t74_scale_leaves_lone_points() {
    let mut e = mug(CenterMode::Local);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![8, 12];
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::new(2., 3., 0.5));
    for v in [8, 12] {
        close(during[v], before[v], &format!("point {v}"));
    }
}

/// AC-T75: the one Local handle sits on the last selected polygon's
/// cluster, at its centre, with its frame: pick a tilted polygon then a level one and the handle
/// is level; the other way round, it tilts with the tilted one.
#[test]
fn ac_t75_local_handle_follows_the_last_selected_polygon() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices: vec![
            // P: tilted 45° (normal along (0, 1, -1)).
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 1., 1.),
            DVec3::new(1., 1., 1.),
            DVec3::new(1., 0., 0.),
            // F: level (normal +Y), 5 m away.
            DVec3::new(5., 0., 0.),
            DVec3::new(5., 0., 1.),
            DVec3::new(6., 0., 1.),
            DVec3::new(6., 0., 0.),
        ],
        faces: vec![quad([0, 1, 2, 3]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
    .into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Local,
        axis: AxisOrientation::Local,
    }))
    .unwrap();
    // The pivot service's handle: the frame of the cluster holding the
    // last-selected element.
    let handle = |e: &Editor| {
        e.pivot_request(&e.scene, Tool::Move)
            .resolve()
            .unwrap()
            .handle
    };
    let handle_z = |e: &Editor| handle(e).basis.z_axis;
    let handle_at = |e: &Editor| handle(e).center;
    e.scene.selection.faces = vec![0, 1];
    close(handle_z(&e), DVec3::Y, "F last: level handle");
    // The handle sits on F's centre, not between the two (Jason 11-21).
    close(handle_at(&e), DVec3::new(5.5, 0., 0.5), "F last: on F");
    e.scene.selection.faces = vec![1, 0];
    close(handle_at(&e), DVec3::splat(0.5), "P last: on P");
    let tilted = handle_z(&e);
    let normal = DVec3::new(0., 1., -1.).normalize();
    assert!(
        (tilted.dot(normal).abs() - 1.).abs() < 1e-9,
        "P last: handle Z along P's normal, got {tilted:?}"
    );
    // Not Local: no special handle frame.
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    let request = e.pivot_request(&e.scene, Tool::Move);
    assert_eq!(request.resolve().unwrap().handle, request.frame().unwrap());
}

/// AC-03: two polygons on the wall with an unselected row between them are
/// two clusters (the edges across that row don't join them), so each scales
/// about its own centre.
#[test]
fn ac_t03_polygons_a_row_apart_are_separate_clusters() {
    let mut e = mug(CenterMode::Local);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    // Row 0 and row 2 of side 1.
    e.scene.selection.faces = vec![1, 2 * SIDES + 1];
    let before = points(&e);
    let during = preview(&mut e, Tool::Scale, DVec3::splat(0.5));
    for f in [1usize, 2 * SIDES as usize + 1] {
        let face = &e.scene.active().unwrap().mesh.faces[f].vertices;
        let (lo, hi) = face
            .iter()
            .fold((DVec3::INFINITY, DVec3::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(before[v as usize]), hi.max(before[v as usize]))
            });
        let c = (lo + hi) / 2.;
        let middle: DVec3 =
            face.iter().map(|&v| during[v as usize]).sum::<DVec3>() / face.len() as f64;
        let was: DVec3 =
            face.iter().map(|&v| before[v as usize]).sum::<DVec3>() / face.len() as f64;
        // Scaled about its own box centre: its middle moves halfway there.
        close(middle, c + (was - c) * 0.5, &format!("polygon {f}"));
    }
}
