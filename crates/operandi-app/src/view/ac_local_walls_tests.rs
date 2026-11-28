//! Jason's report (2026-11-28), dragged: a cube's two opposite walls
//! selected under Local, Rotate, a drag on the red ring (the ring about
//! the direction the walls share). Both walls must turn the same way, as
//! the typed case in core's `ac_local_opposite_walls.rs`.
use super::tests::{draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

/// A 1 m cube with the walls facing −`last` and `last` selected in that
/// order, seen in `view`.
fn app(ctx: &Context, view: View, last: DVec3) -> (Operandi, [Vec<usize>; 2]) {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    a.camera.view = view;
    a.camera.target = DVec3::ZERO;
    a.camera.distance = 6.;
    a.run(Command::Mode(Mode::Polygon));
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let facing = |out: DVec3| {
        (0..mesh.faces.len())
            .find(|&f| mesh.normal(f).normalize().distance(out) < 1e-9)
            .unwrap()
    };
    let walls = [facing(-last), facing(last)];
    a.editor.scene.selection.faces = walls.iter().map(|&f| f as u32).collect();
    assert!(
        a.run_ok(Command::ActionContext(ActionContext::LOCAL)),
        "{}",
        a.status
    );
    draw(&mut a, ctx, SIZE, vec![]);
    let points = walls.map(|f| {
        mesh.faces[f]
            .vertices
            .iter()
            .map(|&v| v as usize)
            .collect::<Vec<_>>()
    });
    (a, points)
}

/// How far `points` turned about `axis`, about their own middle (degrees).
fn turned(before: &[DVec3], after: &[DVec3], points: &[usize], axis: DVec3) -> f64 {
    let middle =
        |all: &[DVec3]| points.iter().map(|&v| all[v]).sum::<DVec3>() / points.len() as f64;
    let flat = |v: DVec3| (v - axis * v.dot(axis)).normalize();
    let (from, to) = (
        flat(before[points[0]] - middle(before)),
        flat(after[points[0]] - middle(after)),
    );
    from.cross(to).dot(axis).atan2(from.dot(to)).to_degrees()
}

#[test]
fn a_red_ring_drag_turns_opposite_walls_the_same_way() {
    for (last, shared, view) in [
        (DVec3::X, DVec3::Z, View::Front),
        (-DVec3::X, DVec3::Z, View::Front),
        (DVec3::Z, DVec3::X, View::Right),
        (DVec3::Y, DVec3::X, View::Right),
    ] {
        let ctx = Context::default();
        let (mut a, walls) = app(&ctx, view, last);
        a.activate(Tool::Rotate);
        draw(&mut a, &ctx, SIZE, vec![]);
        let r = a.viewport_rect;
        let handle = a.editor.handle_frame().expect("a handle frame");
        assert!(
            handle.center.distance(last * 0.5) < 1e-9,
            "on the last wall"
        );
        assert!(a.handle_axis(0).distance(shared) < 1e-9, "red is shared");
        let centre = a.camera.project(handle.center, r).unwrap().0;
        // A point on the red ring (handle 0), found by looking for it.
        let at = (30..160)
            .flat_map(|radius| (0..24).map(move |k| (radius as f32, k as f32 * 0.2618)))
            .map(|(radius, angle)| centre + vec2(angle.cos(), angle.sin()) * radius)
            .find(|&p| a.handle_hit(p, r) == Some(0))
            .expect("the red ring on screen");
        let out = (at - centre).normalized();
        let along = vec2(-out.y, out.x) * 70.;
        let before = a.editor.scene.active().unwrap().mesh.vertices.to_vec();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
        draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
        for step in 1..=4 {
            let p = at + along * (step as f32 / 4.);
            draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(p)]);
        }
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at + along, false, Modifiers::NONE)],
        );
        let after = a.editor.scene.active().unwrap().mesh.vertices.to_vec();
        let (first, second) = (
            turned(&before, &after, &walls[0], shared),
            turned(&before, &after, &walls[1], shared),
        );
        assert!(
            first.abs() > 2.,
            "the drag turned the walls: {first} ({})",
            a.status
        );
        assert!(
            (first - second).abs() < 1e-6,
            "both walls the same way about {shared}: {first} and {second}"
        );
    }
}
