//! Tweak (Element Move), slice TWK-1, through real pointer and key events.
//! Spec: `specifications/tweak-2026-11-03` (v0, guide-only). The picking
//! priority and the drag plane are the provisional readings of TWK-05/09.
use super::tests::{draw, face, key, pointer};
use super::*;
use operandi_core::{TweakElement, TweakGrab};

const SIZE: Vec2 = vec2(1200., 800.);

/// The spec's fixture: a 4×4 grid of 1 m quads in the XY plane, here
/// centered on the origin (points at whole metres from −2 to 2).
fn grid_app(view: View) -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..5 {
        for i in 0..5 {
            mesh.vertices
                .push(DVec3::new(i as f64 - 2., j as f64 - 2., 0.));
        }
    }
    for j in 0..4u32 {
        for i in 0..4u32 {
            let a = j * 5 + i;
            mesh.faces.push(face(vec![a, a + 1, a + 6, a + 5]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = view;
    a.camera.target = DVec3::ZERO;
    a.camera.distance = 8.;
    if view == View::Perspective {
        a.camera.yaw = 0.35;
        a.camera.pitch = 0.3;
    }
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

/// Index of the grid point at (x, y).
fn point(x: i32, y: i32) -> u32 {
    ((y + 2) * 5 + x + 2) as u32
}

fn screen(a: &Operandi, world: DVec3) -> Pos2 {
    a.camera.project(world, a.viewport_rect).unwrap().0
}

fn world_point(a: &Operandi, v: u32) -> DVec3 {
    let item = a.editor.scene.active().unwrap();
    a.editor
        .scene
        .world_point(item.id, item.mesh.vertices[v as usize])
}

/// Hover, press, move in a few steps and release, as a hand would.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for k in 1..=4 {
        let p = from + (to - from) * (k as f32 / 4.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(to, false, Modifiers::NONE)]);
}

/// World metres per screen pixel in an orthographic view.
fn metres_per_pixel(a: &Operandi) -> f64 {
    a.camera.distance / (a.viewport_rect.height() as f64 * 0.9)
}

#[test]
fn t_starts_tweak_in_every_mode_and_leaves_the_selection_alone() {
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
        let (mut a, ctx) = grid_app(View::Front);
        a.run(Command::Mode(mode));
        a.editor.scene.selection.faces = vec![5];
        a.editor.scene.selection.vertices = vec![3];
        let before = a.editor.scene.clone();
        draw(&mut a, &ctx, SIZE, vec![key(Key::T)]);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Tweak),
            "{mode:?}: {}",
            a.status
        );
        assert!(
            a.editor.scene == before,
            "{mode:?}: activation changed the scene"
        );
        // T again (after letting go of it) drops it, like the other tools.
        let release = Event::Key {
            key: Key::T,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        draw(&mut a, &ctx, SIZE, vec![release]);
        draw(&mut a, &ctx, SIZE, vec![key(Key::T)]);
        assert!(a.editor.session.live.is_none(), "{mode:?}: {}", a.status);
    }
}

#[test]
fn auto_picks_a_point_then_an_edge_then_the_polygon() {
    let (mut a, _ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let r = a.viewport_rect;
    let item = a.editor.scene.active().unwrap().id;
    let grab = |element| TweakGrab { item, element };
    let at = |a: &Operandi, x: f64, y: f64| screen(a, DVec3::new(x, y, 0.));
    // A few pixels from a point: the point.
    let pick = a.tweak_pick(at(&a, 1., 1.) + vec2(5., -4.), r).unwrap();
    assert_eq!(pick.grab, grab(TweakElement::Vertex(point(1, 1))));
    assert_eq!(pick.at, DVec3::new(1., 1., 0.));
    // Halfway along an edge, a few pixels off it: the edge, grabbed at the
    // point on it nearest the pointer.
    let pick = a.tweak_pick(at(&a, 0.25, 1.) + vec2(0., 3.), r).unwrap();
    let TweakElement::Edge(p, q) = pick.grab.element else {
        panic!("{pick:?}");
    };
    assert_eq!((p.min(q), p.max(q)), (point(0, 1), point(1, 1)));
    assert!(
        (pick.at - DVec3::new(0.25, 1., 0.)).length() < 1e-6,
        "{pick:?}"
    );
    // Inside a polygon: the polygon, grabbed where the pointer meets it.
    let pick = a.tweak_pick(at(&a, 0.5, 0.5), r).unwrap();
    assert_eq!(pick.grab, grab(TweakElement::Polygon(2 * 4 + 2)));
    assert!(
        (pick.at - DVec3::new(0.5, 0.5, 0.)).length() < 1e-6,
        "{pick:?}"
    );
    // Off the mesh: nothing.
    assert_eq!(a.tweak_pick(at(&a, 3.5, 3.5), r), None);
}

#[test]
fn dragging_a_point_moves_only_it_and_keeps_it_under_the_pointer() {
    let (mut a, ctx) = grid_app(View::Front);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    a.activate(Tool::Tweak);
    let before = a.editor.scene.clone();
    let v = point(1, 1);
    let from = screen(&a, DVec3::new(1., 1., 0.));
    drag(&mut a, &ctx, from, from + vec2(100., -40.));
    let m = metres_per_pixel(&a);
    let expected = DVec3::new(1. + 100. * m, 1. + 40. * m, 0.);
    let after = &a.editor.scene;
    let (mb, ma) = (
        &before.active().unwrap().mesh,
        &after.active().unwrap().mesh,
    );
    assert!(
        (ma.vertices[v as usize] - expected).length() < 1e-6,
        "{:?} vs {expected:?} ({})",
        ma.vertices[v as usize],
        a.status
    );
    for i in 0..mb.vertices.len() {
        if i != v as usize {
            assert_eq!(ma.vertices[i], mb.vertices[i], "point {i} moved");
        }
    }
    // TWK-25 / IDs: same points, same polygons in the same order.
    assert_eq!(ma.vertices.len(), mb.vertices.len());
    assert_eq!(ma.faces, mb.faces);
    // TWK-07/22: the selection is untouched.
    assert_eq!(after.selection, before.selection);
    // TWK-24: one undo step (a tweak of the session, Jason 2026-11-04),
    // and the tool stays live.
    assert_eq!(a.editor.session.tweaks.undo_steps(), 1);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Tweak);
}

#[test]
fn edges_and_polygons_move_all_their_points_by_the_same_offset() {
    for (press, moved) in [
        // Middle of the edge (0,1)–(1,1).
        (DVec3::new(0.5, 1., 0.), vec![point(0, 1), point(1, 1)]),
        // Middle of the polygon with corners (0,0)–(1,1).
        (
            DVec3::new(0.5, 0.5, 0.),
            vec![point(0, 0), point(1, 0), point(1, 1), point(0, 1)],
        ),
    ] {
        let (mut a, ctx) = grid_app(View::Front);
        a.activate(Tool::Tweak);
        let before: Vec<DVec3> = a.editor.scene.active().unwrap().mesh.vertices.clone();
        let steps = a.editor.history.undo.len();
        let from = screen(&a, press);
        drag(&mut a, &ctx, from, from + vec2(-60., 30.));
        let m = metres_per_pixel(&a);
        let offset = DVec3::new(-60. * m, -30. * m, 0.);
        let after = &a.editor.scene.active().unwrap().mesh.vertices;
        for (i, (b, now)) in before.iter().zip(after).enumerate() {
            let expected = if moved.contains(&(i as u32)) {
                *b + offset
            } else {
                *b
            };
            assert!((*now - expected).length() < 1e-6, "{press:?} point {i}");
        }
        assert_eq!(a.editor.history.undo.len(), steps, "{press:?}");
        assert!(a.editor.session.tweaks.can_undo(), "{press:?}");
    }
}

#[test]
fn each_drag_is_its_own_undo_step_and_undo_keeps_the_tool() {
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let start = a.editor.scene.clone();
    let steps = a.editor.history.undo.len();
    let p1 = screen(&a, DVec3::new(-1., -1., 0.));
    drag(&mut a, &ctx, p1, p1 + vec2(50., 0.));
    let first = a.editor.scene.clone();
    let p2 = screen(&a, DVec3::new(2., 2., 0.));
    drag(&mut a, &ctx, p2, p2 + vec2(0., 50.));
    // Each drag is a tweak of the Tweak session (Jason, 2026-11-04).
    assert_eq!(a.editor.history.undo.len(), steps);
    assert_eq!(a.editor.session.tweaks.undo_steps(), 2);
    a.run(Command::Undo);
    assert!(a.editor.scene == first, "{}", a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Tweak)
    );
    a.run(Command::Undo);
    assert!(a.editor.scene == start);
    a.run(Command::Redo);
    assert!(a.editor.scene == first);
    // The tool still works after Undo/Redo, and Drop makes one step.
    drag(&mut a, &ctx, p2, p2 + vec2(0., 50.));
    assert_eq!(a.editor.session.tweaks.undo_steps(), 2);
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
}

#[test]
fn empty_space_and_clicks_without_moving_change_nothing() {
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let before = a.editor.scene.clone();
    let steps = a.editor.history.undo.len();
    // TWK-23: a press off the mesh does nothing (no camera move either).
    let camera = (a.camera.target, a.camera.distance);
    let off = screen(&a, DVec3::new(3.5, 3.5, 0.));
    drag(&mut a, &ctx, off, off + vec2(80., 80.));
    assert!(a.editor.scene == before);
    assert_eq!((a.camera.target, a.camera.distance), camera);
    // A click on a point without moving records nothing.
    let on = screen(&a, DVec3::new(0., 0., 0.));
    drag(&mut a, &ctx, on, on);
    assert!(a.editor.scene == before);
    assert_eq!(a.editor.history.undo.len(), steps);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Tweak);
}

#[test]
fn in_perspective_the_grabbed_point_stays_under_the_pointer() {
    let (mut a, ctx) = grid_app(View::Perspective);
    a.activate(Tool::Tweak);
    let v = point(1, 0);
    let from = screen(&a, world_point(&a, v));
    let to = from + vec2(70., -45.);
    let depth = |a: &Operandi, w: DVec3| a.camera.project(w, a.viewport_rect).unwrap().1;
    let depth_before = depth(&a, world_point(&a, v));
    drag(&mut a, &ctx, from, to);
    let now = world_point(&a, v);
    assert!(
        screen(&a, now).distance(to) < 0.01,
        "{:?} vs {to:?}",
        screen(&a, now)
    );
    // It keeps its depth from the camera (the view-facing plane).
    assert!((depth(&a, now) - depth_before).abs() < 1e-6);
}

#[test]
fn tweaks_a_moved_and_scaled_item_in_world_space() {
    let (mut a, ctx) = grid_app(View::Front);
    {
        let item = a.editor.scene.active_mut().unwrap();
        item.transform.translation = DVec3::new(0.5, -0.25, 0.);
        item.transform.scale = DVec3::new(0.5, 0.5, 1.);
    }
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::Tweak);
    let v = point(2, 2);
    let from = screen(&a, world_point(&a, v));
    let to = from + vec2(40., 25.);
    drag(&mut a, &ctx, from, to);
    assert!(screen(&a, world_point(&a, v)).distance(to) < 0.01);
    // In the item's own space the point moved twice as far (scale 0.5).
    let m = metres_per_pixel(&a);
    let local = a.editor.scene.active().unwrap().mesh.vertices[v as usize];
    assert!((local - DVec3::new(2. + 80. * m, 2. - 50. * m, 0.)).length() < 1e-6);
}

#[test]
fn hovering_with_tweak_changes_nothing() {
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let before = a.editor.clone();
    for p in [
        DVec3::new(1., 1., 0.),
        DVec3::new(0.5, 1., 0.),
        DVec3::new(0.5, 0.5, 0.),
    ] {
        let q = screen(&a, p);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(q)]);
    }
    assert!(a.editor == before);
}

#[test]
fn a_saved_input_map_that_already_uses_t_keeps_it() {
    let dir = std::env::temp_dir().join(format!("operandi-tweak-keys-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bindings.json");
    std::fs::write(&path, r#"{"Move":"T"}"#).unwrap();
    let bindings = input::load_bindings(&path);
    assert_eq!(bindings["Move"], "T");
    assert_eq!(bindings["Element Move"], "");
    std::fs::write(&path, r#"{"Move":"W"}"#).unwrap();
    assert_eq!(input::load_bindings(&path)["Element Move"], "T");
    // A map saved under the old name keeps its binding.
    std::fs::write(&path, r#"{"Tweak":"Shift+W"}"#).unwrap();
    let bindings = input::load_bindings(&path);
    assert_eq!(bindings["Element Move"], "Shift+W");
    assert!(!bindings.contains_key("Tweak"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// TWK-20 (provisional): with Symmetry X, the counterpart moves mirrored and
/// points on the plane stay on it.
#[test]
fn symmetry_moves_the_mirror_counterpart_too() {
    // (pressed at, points that move as dragged, points that move mirrored,
    // points on the plane)
    type Case = (DVec3, Vec<u32>, Vec<u32>, Vec<u32>);
    let cases: [Case; 3] = [
        (
            DVec3::new(1., 1., 0.),
            vec![point(1, 1)],
            vec![point(-1, 1)],
            vec![],
        ),
        (
            DVec3::new(1.5, 1., 0.),
            vec![point(1, 1), point(2, 1)],
            vec![point(-1, 1), point(-2, 1)],
            vec![],
        ),
        (
            DVec3::new(0.5, 0.5, 0.),
            vec![point(1, 0), point(1, 1)],
            vec![point(-1, 0), point(-1, 1)],
            vec![point(0, 0), point(0, 1)],
        ),
    ];
    for (press, dragged, mirrored, on_plane) in cases {
        let (mut a, ctx) = grid_app(View::Front);
        a.run(Command::Symmetry(operandi_core::Symmetry::x()));
        a.activate(Tool::Tweak);
        let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
        let steps = a.editor.history.undo.len();
        let from = screen(&a, press);
        drag(&mut a, &ctx, from, from + vec2(60., -30.));
        let m = metres_per_pixel(&a);
        let d = DVec3::new(60. * m, 30. * m, 0.);
        let after = &a.editor.scene.active().unwrap().mesh.vertices;
        for (i, (b, now)) in before.iter().zip(after).enumerate() {
            let i = i as u32;
            let expected = if dragged.contains(&i) {
                *b + d
            } else if mirrored.contains(&i) {
                *b + DVec3::new(-d.x, d.y, d.z)
            } else if on_plane.contains(&i) {
                *b + DVec3::new(0., d.y, d.z)
            } else {
                *b
            };
            assert!(
                (*now - expected).length() < 1e-6,
                "{press:?} point {i}: {now:?} vs {expected:?} ({})",
                a.status
            );
        }
        assert_eq!(a.editor.history.undo.len(), steps, "{press:?}");
        assert!(
            a.editor.session.tweaks.can_undo(),
            "{press:?}: one undo step"
        );
    }
}

#[test]
fn a_point_on_the_symmetry_plane_stays_on_it() {
    let (mut a, ctx) = grid_app(View::Front);
    a.run(Command::Symmetry(operandi_core::Symmetry::x()));
    a.activate(Tool::Tweak);
    let v = point(0, 1);
    let from = screen(&a, DVec3::new(0., 1., 0.));
    drag(&mut a, &ctx, from, from + vec2(80., 40.));
    let now = a.editor.scene.active().unwrap().mesh.vertices[v as usize];
    let m = metres_per_pixel(&a);
    assert!(
        (now - DVec3::new(0., 1. - 40. * m, 0.)).length() < 1e-6,
        "{now:?}"
    );
    // Its neighbours didn't move.
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices[point(1, 1) as usize], DVec3::new(1., 1., 0.));
    assert_eq!(
        mesh.vertices[point(-1, 1) as usize],
        DVec3::new(-1., 1., 0.)
    );
}

/// Sets Tweak's options the way the panel does (a Preview of the live
/// parameters with only the options changed).
fn set_options(a: &mut Operandi, f: impl FnOnce(&mut operandi_core::TweakOptions)) {
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    f(&mut p.tweak_options);
    a.run(Command::Preview(p));
}

/// Each point's move as a fraction of `offset` (which must have a non-zero
/// X), checking every move points the same way.
fn weights(before: &[DVec3], after: &[DVec3], offset: DVec3) -> Vec<f64> {
    before
        .iter()
        .zip(after)
        .map(|(b, now)| {
            let moved = *now - *b;
            let w = moved.x / offset.x;
            assert!(
                (moved - offset * w).length() < 1e-6,
                "{moved:?} vs {offset:?}"
            );
            w
        })
        .collect()
}

#[test]
fn range_spreads_a_drag_and_one_undo_restores_every_point() {
    // TWK-12 (TWK-O-T4-08-linear): Range 1.5, Linear, the centre point.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    set_options(&mut a, |o| o.range = 1.5);
    let before = a.editor.scene.clone();
    let start: Vec<DVec3> = before.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, DVec3::ZERO);
    drag(&mut a, &ctx, from, from + vec2(80., -20.));
    let m = metres_per_pixel(&a);
    let offset = DVec3::new(80. * m, 20. * m, 0.);
    let w = weights(
        &start,
        &a.editor.scene.active().unwrap().mesh.vertices,
        offset,
    );
    for x in -2i32..=2 {
        for y in -2i32..=2 {
            let expected = match x.abs() + y.abs() {
                0 => 1.,
                1 => 1. / 3.,
                2 if x != 0 && y != 0 => 1. - 2_f64.sqrt() / 1.5,
                _ => 0.,
            };
            let got = w[point(x, y) as usize];
            assert!(
                (got - expected).abs() < 1e-5,
                "({x},{y}): {got} vs {expected}"
            );
        }
    }
    // The options stay for the next drag; the drag is one undo step that
    // brings back every weighted point (TWK-21).
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .tweak_options
            .range,
        1.5
    );
    assert!(a.editor.session.tweaks.can_undo());
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, start);
    assert_eq!(a.editor.scene.selection, before.selection);
}

#[test]
fn mode_limits_what_a_press_can_grab() {
    let (mut a, _ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let r = a.viewport_rect;
    // Just inside quad 10, a few pixels from its corner (0,0).
    let near_corner = screen(&a, DVec3::ZERO) + vec2(4., -4.);
    let element = |a: &Operandi| a.tweak_pick(near_corner, r).map(|p| p.grab.element);
    assert_eq!(element(&a), Some(TweakElement::Vertex(point(0, 0))));
    set_options(&mut a, |o| o.mode = operandi_core::TweakMode::Polygon);
    assert_eq!(element(&a), Some(TweakElement::Polygon(10)));
    set_options(&mut a, |o| o.mode = operandi_core::TweakMode::Edge);
    assert!(matches!(element(&a), Some(TweakElement::Edge(..))));
    // Vertex mode in the middle of a polygon grabs nothing, and a press
    // there changes nothing.
    set_options(&mut a, |o| o.mode = operandi_core::TweakMode::Vertex);
    let middle = screen(&a, DVec3::new(0.5, 0.5, 0.));
    assert_eq!(a.tweak_pick(middle, r), None);
    // Polygon Center grabs at the polygon's center (TWK-09's handle spot).
    set_options(&mut a, |o| o.mode = operandi_core::TweakMode::PolygonCenter);
    let pick = a.tweak_pick(near_corner, r).unwrap();
    assert_eq!(pick.grab.element, TweakElement::Polygon(10));
    assert!(
        (pick.at - DVec3::new(0.5, 0.5, 0.)).length() < 1e-9,
        "{pick:?}"
    );
}

#[test]
fn edge_loops_drag_moves_the_whole_row() {
    // TWK-16 (TWK-O-T4-10-edge): the edge (0,1)–(1,1) moves the y = 1 row.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    set_options(&mut a, |o| {
        o.connect = operandi_core::TweakConnect::EdgeLoops
    });
    let start: Vec<DVec3> = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, DVec3::new(0.5, 1., 0.));
    drag(&mut a, &ctx, from, from + vec2(50., 0.));
    let offset = DVec3::new(50. * metres_per_pixel(&a), 0., 0.);
    let w = weights(
        &start,
        &a.editor.scene.active().unwrap().mesh.vertices,
        offset,
    );
    for (v, got) in w.iter().enumerate() {
        let expected = if (15..20).contains(&v) { 1. } else { 0. };
        // Screen points are f32: 1e-6 allows for the view's size (the
        // floating header gave the view its full height).
        assert!((got - expected).abs() < 1e-6, "point {v}: {got}");
    }
}

#[test]
fn a_typed_range_is_kept() {
    // Regression (Jason, 2026-11-04): Range lost the typed value.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.fields.insert("tweak_range".into(), "1.5 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .tweak_options
            .range,
        1.5
    );
    assert_eq!(a.fields["tweak_range"], a.confirmed_fields["tweak_range"]);
    // A negative entry clamps to 0 and shows 0.
    a.fields.insert("tweak_range".into(), "-2 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .tweak_options
            .range,
        0.
    );
    // It still works for the next drag.
    a.fields.insert("tweak_range".into(), "1.5 m".into());
    assert!(a.confirm_fields());
    let start: Vec<DVec3> = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, DVec3::ZERO);
    drag(&mut a, &ctx, from, from + vec2(60., 0.));
    let moved = a.editor.scene.active().unwrap().mesh.vertices[point(1, 0) as usize]
        - start[point(1, 0) as usize];
    assert!(moved.x > 0., "neighbour didn't follow");
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .tweak_options
            .range,
        1.5
    );
}

// Tweak Range resize by right-drag (Tweak Range spec v1, TWR-06..14).

fn right(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        pressed,
        modifiers,
        button: PointerButton::Secondary,
    }
}

fn range(a: &Operandi) -> f64 {
    a.editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .tweak_options
        .range
}

/// Right-press at `from`, move through `path` and release at its end.
fn right_drag(a: &mut Operandi, ctx: &Context, from: Pos2, path: &[Pos2], mods: Modifiers) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![right(from, true, mods)]);
    for &p in path {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    let end = *path.last().unwrap_or(&from);
    draw(a, ctx, SIZE, vec![right(end, false, mods)]);
}

fn range_sphere_shown(a: &Operandi) -> bool {
    matches!(
        a.navigation.as_ref().map(|g| &g.kind),
        Some(input::GestureKind::TweakRange { .. })
    )
}

#[test]
fn right_drag_grows_range_to_the_right_and_shrinks_it_to_the_left() {
    // TWR-06/07/09: horizontal travel only; the field follows live.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let from = screen(&a, DVec3::ZERO);
    let rate = viewport::tweak_view::TWEAK_RANGE_RATE * metres_per_pixel(&a);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(&mut a, &ctx, SIZE, vec![right(from, true, Modifiers::NONE)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(50., 0.))],
    );
    assert!((range(&a) - 50. * rate).abs() < 1e-9, "{}", range(&a));
    // The Range field shows the live value mid-drag.
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        a.fields.get("tweak_range").map(String::as_str),
        Some(workspace::number_text(range(&a), " m").as_str())
    );
    assert!(a.status.contains("Range"), "{}", a.status);
    // Vertical movement changes nothing.
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(50., 120.))],
    );
    assert!((range(&a) - 50. * rate).abs() < 1e-9);
    // Left shrinks it.
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(30., 120.))],
    );
    assert!((range(&a) - 30. * rate).abs() < 1e-9);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![right(from + vec2(30., 120.), false, Modifiers::NONE)],
    );
    assert!((range(&a) - 30. * rate).abs() < 1e-9);
    // Over empty space it works the same (TWR-06).
    let empty = a.viewport_rect.right_top() + vec2(-40., 40.);
    assert!(a.pick_face(empty, a.viewport_rect).is_none());
    right_drag(
        &mut a,
        &ctx,
        empty,
        &[empty + vec2(10., 0.)],
        Modifiers::NONE,
    );
    assert!((range(&a) - 40. * rate).abs() < 1e-9, "{}", range(&a));
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Tweak);
}

#[test]
fn right_drag_rate_follows_the_zoom() {
    // TWR-07: the same drag gives the same sphere size on screen.
    let mut on_screen = Vec::new();
    for distance in [4., 16.] {
        let (mut a, ctx) = grid_app(View::Front);
        a.camera.distance = distance;
        a.activate(Tool::Tweak);
        let from = screen(&a, DVec3::ZERO);
        right_drag(&mut a, &ctx, from, &[from + vec2(40., 0.)], Modifiers::NONE);
        on_screen.push(range(&a) / metres_per_pixel(&a));
    }
    assert!((on_screen[0] - on_screen[1]).abs() < 1e-6, "{on_screen:?}");
    assert!(
        (on_screen[0] - 40. * viewport::tweak_view::TWEAK_RANGE_RATE).abs() < 1e-6,
        "{on_screen:?}"
    );
    // In perspective the scale is taken at the surface under the pointer.
    let (mut a, ctx) = grid_app(View::Perspective);
    a.activate(Tool::Tweak);
    let from = screen(&a, DVec3::ZERO);
    let r = a.viewport_rect;
    let scale = a.camera.metres_per_pixel(DVec3::ZERO, r).unwrap();
    right_drag(&mut a, &ctx, from, &[from + vec2(40., 0.)], Modifiers::NONE);
    let expected = 40. * viewport::tweak_view::TWEAK_RANGE_RATE * scale;
    assert!(
        (range(&a) - expected).abs() < 1e-3 * expected,
        "{}",
        range(&a)
    );
}

#[test]
fn right_drag_stops_at_zero_and_grows_again_on_reversal() {
    // TWR-08.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    set_options(&mut a, |o| o.range = 0.25);
    let from = screen(&a, DVec3::ZERO);
    let rate = viewport::tweak_view::TWEAK_RANGE_RATE * metres_per_pixel(&a);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(&mut a, &ctx, SIZE, vec![right(from, true, Modifiers::NONE)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(-300., 0.))],
    );
    assert_eq!(range(&a), 0.);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(-290., 0.))],
    );
    assert!((range(&a) - 10. * rate).abs() < 1e-9, "{}", range(&a));
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![right(from + vec2(-290., 0.), false, Modifiers::NONE)],
    );
}

#[test]
fn right_drag_ignores_shift_ctrl_and_cmd() {
    // TWR-10.
    let mut results = Vec::new();
    for mods in [
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::CTRL,
        Modifiers::MAC_CMD,
    ] {
        let (mut a, ctx) = grid_app(View::Front);
        a.activate(Tool::Tweak);
        let from = screen(&a, DVec3::ZERO);
        let start = a.editor.scene.clone();
        right_drag(&mut a, &ctx, from, &[from + vec2(25., 0.)], mods);
        assert_eq!(a.editor.scene, start, "{mods:?} changed the scene");
        assert!(a.editor.scene.selection == start.selection);
        results.push(range(&a));
    }
    assert!(results[0] > 0.);
    assert!(results.iter().all(|&r| r == results[0]), "{results:?}");
}

#[test]
fn range_sphere_shows_only_while_the_right_button_is_held() {
    // TWR-11/12: centred where the drag started (the surface there).
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    set_options(&mut a, |o| o.range = 0.5);
    let from = screen(&a, DVec3::new(1., 1., 0.));
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(from)]);
    assert!(!range_sphere_shown(&a));
    draw(&mut a, &ctx, SIZE, vec![right(from, true, Modifiers::NONE)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(from + vec2(20., 30.))],
    );
    let Some(input::GestureKind::TweakRange { center, .. }) =
        a.navigation.as_ref().map(|g| &g.kind)
    else {
        panic!("no range drag");
    };
    assert!(
        (*center - DVec3::new(1., 1., 0.)).length() < 0.01,
        "{center:?}"
    );
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![right(from + vec2(20., 30.), false, Modifiers::NONE)],
    );
    assert!(!range_sphere_shown(&a));
    // A left drag shows no sphere either.
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    assert!(!range_sphere_shown(&a));
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(from, false, Modifiers::NONE)],
    );
}

#[test]
fn one_right_drag_is_one_undo_step_and_the_tool_stays() {
    // TWR-14: press to release is one step however many moves it had.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    set_options(&mut a, |o| o.range = 0.5);
    let steps = a.editor.session.tweaks.undo_steps();
    let from = screen(&a, DVec3::ZERO);
    let path: Vec<Pos2> = (1..=8).map(|k| from + vec2(10. * k as f32, 0.)).collect();
    right_drag(&mut a, &ctx, from, &path, Modifiers::NONE);
    let grown = range(&a);
    assert!(grown > 0.5);
    assert_eq!(a.editor.session.tweaks.undo_steps(), steps + 1);
    a.run(Command::Undo);
    assert_eq!(range(&a), 0.5);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Tweak);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.fields["tweak_range"], workspace::number_text(0.5, " m"));
    a.run(Command::Redo);
    assert_eq!(range(&a), grown);
    // The resized Range drives the next left-drag.
    let start: Vec<DVec3> = a.editor.scene.active().unwrap().mesh.vertices.clone();
    drag(&mut a, &ctx, from, from + vec2(40., 0.));
    let moved = a.editor.scene.active().unwrap().mesh.vertices[point(1, 0) as usize]
        - start[point(1, 0) as usize];
    assert!(moved.x > 0., "neighbour didn't follow");
}

#[test]
fn right_drag_leaves_the_mesh_alone() {
    // A Range drag only changes the option: the mesh stays the same shared
    // data, so nothing is rebuilt per frame (Fast drags).
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let from = screen(&a, DVec3::ZERO);
    right_drag(&mut a, &ctx, from, &[from + vec2(60., 0.)], Modifiers::NONE);
    assert!(range(&a) > 0.);
    assert!(operandi_core::Shared::same(
        mesh.shared(),
        a.editor.scene.active().unwrap().mesh.shared()
    ));
}

#[test]
fn status_bar_names_the_right_drag_and_rigid_blocks_it() {
    // TWR-13.
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    assert!(
        a.status
            .contains("Right Click and Drag: Adjust Falloff Range"),
        "{}",
        a.status
    );
    // Rigid doesn't use Range (its field is greyed), so the drag does nothing.
    set_options(&mut a, |o| {
        o.range = 0.5;
        o.connect = operandi_core::TweakConnect::Rigid
    });
    let from = screen(&a, DVec3::ZERO);
    right_drag(&mut a, &ctx, from, &[from + vec2(60., 0.)], Modifiers::NONE);
    assert_eq!(range(&a), 0.5);
    assert!(a.status.contains("Rigid"), "{}", a.status);
}

/// Jason 2026-11-14: Symmetry turns on and off while a tool is open, and
/// the open tool's next drag follows it (the tool chain: Symmetry is a layer
/// under every tool). Each drag stays one undo step.
#[test]
fn symmetry_toggled_with_tweak_open_applies_to_the_next_drag() {
    let (mut a, ctx) = grid_app(View::Front);
    a.activate(Tool::Tweak);
    a.execute("toggle symmetry");
    assert_eq!(
        a.editor.symmetry,
        operandi_core::Symmetry::x(),
        "{}",
        a.status
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Tweak)
    );
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, DVec3::new(1., 1., 0.));
    drag(&mut a, &ctx, from, from + vec2(60., -30.));
    let m = metres_per_pixel(&a);
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let moved = after[point(1, 1) as usize] - before[point(1, 1) as usize];
    let mirrored = after[point(-1, 1) as usize] - before[point(-1, 1) as usize];
    assert!(
        (moved - DVec3::new(60. * m, 30. * m, 0.)).length() < 1e-6,
        "{moved:?}"
    );
    assert!(
        (mirrored - DVec3::new(-60. * m, 30. * m, 0.)).length() < 1e-6,
        "{mirrored:?}"
    );

    // Off again with the tool still open: the next drag moves one point.
    a.execute("toggle symmetry");
    assert_eq!(a.editor.symmetry.axis, None, "{}", a.status);
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, after[point(1, 1) as usize]);
    drag(&mut a, &ctx, from, from + vec2(40., 0.));
    let now = &a.editor.scene.active().unwrap().mesh.vertices;
    assert!((now[point(1, 1) as usize] - before[point(1, 1) as usize]).length() > 1e-3);
    assert_eq!(now[point(-1, 1) as usize], before[point(-1, 1) as usize]);
}

/// Move open on a mirrored pair: Symmetry turned on mid-tool moves the
/// handle to the picked side and the next drag mirrors (SYM-TR-02/03);
/// turned off after the drag, the shown result follows at once. Neither
/// toggle is an undo step; the drag stays one.
#[test]
fn symmetry_toggled_with_move_open_applies_live() {
    let (mut a, ctx) = grid_app(View::Front);
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![point(1, 1), point(-1, 1)];
    a.activate(Tool::Move);
    let centre = |a: &Operandi| {
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center
    };
    assert!(centre(&a).x.abs() < 1e-9, "both sides: on the plane");
    let steps = a.editor.history.undo.len();
    a.execute("toggle symmetry");
    assert_eq!(
        a.editor.symmetry,
        operandi_core::Symmetry::x(),
        "{}",
        a.status
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move),
        "{}",
        a.status
    );
    assert!(
        (centre(&a).x.abs() - 1.).abs() < 1e-9,
        "picked side: {:?}",
        centre(&a)
    );
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let from = screen(&a, DVec3::new(1.5, -1.5, 0.));
    drag(&mut a, &ctx, from, from + vec2(60., -60.));
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let right = after[point(1, 1) as usize] - before[point(1, 1) as usize];
    let left = after[point(-1, 1) as usize] - before[point(-1, 1) as usize];
    assert!(right.x.abs() > 1e-3, "moved {right:?} ({})", a.status);
    assert!(
        (left - DVec3::new(-right.x, right.y, right.z)).length() < 1e-6,
        "{left:?} vs {right:?}"
    );

    // Off with Move still open: the finished drag stays mirrored and the
    // next drag moves both points the same way. (The drag left the handle
    // where it was pressed, so it stays there.)
    a.execute("toggle symmetry");
    assert_eq!(a.editor.symmetry.axis, None, "{}", a.status);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, after);
    let from = screen(&a, DVec3::new(1.5, -1.5, 0.));
    drag(&mut a, &ctx, from, from + vec2(60., 0.));
    let now = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let right = now[point(1, 1) as usize] - after[point(1, 1) as usize];
    let left = now[point(-1, 1) as usize] - after[point(-1, 1) as usize];
    assert!(right.length() > 1e-3, "{}", a.status);
    assert!((left - right).length() < 1e-6, "{left:?} vs {right:?}");
    // Each drag is its own step inside the tool; dropped, the session is one.
    assert_eq!(
        a.editor.session.tweaks.undo_steps(),
        2,
        "one tweak per drag, none per toggle"
    );
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
}

/// An open Cone preview is made twice, or once again, as Symmetry changes.
#[test]
fn symmetry_toggled_with_cone_open_redraws_the_preview() {
    let (mut a, _ctx) = grid_app(View::Front);
    a.execute("Cone");
    let mut p = a
        .editor
        .session
        .cone
        .as_ref()
        .expect("cone armed")
        .requested
        .clone();
    p.position = DVec3::new(4., 0., 0.);
    a.run(Command::ConfigureCone(p));
    let points = |a: &Operandi| {
        a.editor
            .session
            .cone
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .map(|m| m.vertices.len())
    };
    let one = points(&a).expect("preview");
    a.execute("toggle symmetry");
    assert_eq!(points(&a), Some(2 * one), "{}", a.status);
    a.execute("toggle symmetry");
    assert_eq!(points(&a), Some(one), "{}", a.status);
}

/// Where a piece of text was drawn this frame (its centre), to click it.
fn find_text(out: &egui::FullOutput, want: &str) -> Option<Pos2> {
    fn walk(shape: &egui::Shape, want: &str, found: &mut Option<Pos2>) {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == want => {
                *found = Some(t.pos + t.galley.rect.center().to_vec2())
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, want, found)),
            _ => {}
        }
    }
    let mut found = None;
    for c in &out.shapes {
        walk(&c.shape, want, &mut found);
    }
    found
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) -> egui::FullOutput {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![])
}

/// The Symmetry button's Toggle row and a bound shortcut both work with a
/// tool open, and the tool stays open.
#[test]
fn symmetry_button_and_shortcut_work_with_a_tool_open() {
    for tool in [Tool::Move, Tool::Rotate, Tool::Tweak, Tool::Bevel] {
        let (mut a, ctx) = grid_app(View::Front);
        a.run(Command::Mode(Mode::Polygon));
        a.run(Command::Selection(SelectionOperation::All));
        a.activate(tool);
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        let button = find_text(&out, "Symmetry: Off").expect("Symmetry button");
        let out = click(&mut a, &ctx, button);
        let toggle = find_text(&out, "Toggle Symmetry").expect("menu open");
        click(&mut a, &ctx, toggle);
        assert_eq!(a.editor.symmetry.axis, Some(0), "{tool:?}: {}", a.status);
        assert_eq!(a.editor.session.live.as_ref().map(|l| l.tool), Some(tool));

        let used: Vec<String> = a.bindings.values().cloned().collect();
        let free = ["F13", "F14", "F15", "F16", "F17", "F18"]
            .into_iter()
            .find(|k| !used.iter().any(|u| u == k))
            .unwrap();
        a.bindings.insert("Toggle Symmetry".into(), free.into());
        let c = a.viewport_rect.center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
        draw(&mut a, &ctx, SIZE, vec![key(Key::from_name(free).unwrap())]);
        assert_eq!(a.editor.symmetry.axis, None, "{tool:?}: {}", a.status);
        assert_eq!(a.editor.session.live.as_ref().map(|l| l.tool), Some(tool));
    }
}

/// Every session that isn't a live tool (Solid Sketch, Bridge, Reduction,
/// Spike, the numeric primitives) takes the toggle and stays open.
#[test]
fn symmetry_toggles_with_every_session_open() {
    for word in [
        "solid sketch",
        "bridge",
        "polygon reduction",
        "spike",
        "Cone",
        "Ellipsoid",
        "torus",
    ] {
        let mut a = super::tests::box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.run(Command::Selection(SelectionOperation::All));
        a.execute(word);
        let task = a.editor.session.active_task();
        assert!(task.is_some(), "{word}: {}", a.status);
        a.execute("toggle symmetry");
        assert_eq!(a.editor.symmetry.axis, Some(0), "{word}: {}", a.status);
        assert_eq!(a.editor.session.active_task(), task, "{word}");
    }
}
