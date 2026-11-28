//! Snapping for the placement tools (service holdouts, 2026-11-20): Pen
//! Slice through the shared snap service and the Pen's direction locks
//! (SN2-34..37), primitives' placed corners and sides (SN2-41), Vertex
//! Create's points (VX-CR02), and the one Rotation Snap Angle rounding
//! (SN2-40).
use super::tests::{draw, pointer};
use super::*;

/// A 2 x 2 grid of unit quads in XZ (points on whole metres), seen from
/// the top.
fn grid_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            mesh.faces
                .push(super::tests::face(vec![v, v + 3, v + 4, v + 1]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.frame(false);
    a
}

const SIZE: Vec2 = vec2(1440., 900.);

fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

/// 8 screen points away from `p`, up and to the right.
fn near(a: &Operandi, p: DVec3) -> Pos2 {
    screen(a, p) + vec2(6., -5.)
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
}

fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for s in 1..=4 {
        let p = from + (to - from) * (s as f32 / 4.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(to, false, Modifiers::NONE)]);
}

fn live_params(a: &Operandi) -> Parameters {
    a.editor.session.live.as_ref().unwrap().params.clone()
}

fn close(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < 1e-9
}

#[test]
fn pen_slice_points_snap_to_a_vertex_and_stay_free_with_snapping_off() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::PenSlice);
    a.snapping.enabled = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let target = DVec3::new(1., 0., 1.);
    let at = near(&a, target);
    click(&mut a, &ctx, at);
    let points = live_params(&a).slice.pen_slice.points;
    assert!(close(points[0], target), "{points:?} ({})", a.status);
    // Off: the click lands where the pointer is.
    a.snapping.enabled = false;
    let at = near(&a, DVec3::new(2., 0., 2.));
    click(&mut a, &ctx, at);
    let points = live_params(&a).slice.pen_slice.points;
    assert!(!close(points[1], DVec3::new(2., 0., 2.)), "{points:?}");
}

#[test]
fn pen_slice_direction_locks_follow_the_snapping_ticks() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::PenSlice);
    a.snapping.enabled = true;
    a.snapping.global.vertex = false;
    a.snapping.global.world_axis = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = screen(&a, DVec3::new(0.5, 0., 0.5));
    click(&mut a, &ctx, at);
    // SNH-Q01: a plain click 10 px off world X stays where it is put.
    let off = screen(&a, DVec3::new(0.5, 0., 0.5)) + vec2(200., 10.);
    click(&mut a, &ctx, off);
    let points = live_params(&a).slice.pen_slice.points;
    assert_eq!(points.len(), 2);
    assert!((points[1].z - points[0].z).abs() > 0.01, "{points:?}");
    a.run(Command::Undo);
    // Press-drag to place: within Inner Range of the line it locks on.
    drag(&mut a, &ctx, off + vec2(0., 30.), off);
    let points = live_params(&a).slice.pen_slice.points;
    assert_eq!(points.len(), 2);
    assert!((points[1].z - points[0].z).abs() < 1e-9, "{points:?}");
    // 30 px off the line: free.
    a.run(Command::Undo);
    drag(&mut a, &ctx, off, off + vec2(0., 20.));
    let points = live_params(&a).slice.pen_slice.points;
    assert!((points[1].z - points[0].z).abs() > 0.01, "{points:?}");
    // World Axis unticked: a drag onto the line's neighbourhood stays free.
    a.snapping.global.world_axis = false;
    a.snapping.global.vertex = true;
    a.run(Command::Undo);
    drag(&mut a, &ctx, off + vec2(0., 30.), off);
    let points = live_params(&a).slice.pen_slice.points;
    assert!((points[1].z - points[0].z).abs() > 0.01, "{points:?}");
}

#[test]
fn a_cube_drawn_from_near_a_vertex_starts_and_ends_on_vertices() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::Box);
    a.snapping.enabled = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let (from, to) = (DVec3::new(1., 0., 1.), DVec3::new(2., 0., 2.));
    let (from, to) = (near(&a, from), near(&a, to));
    drag(&mut a, &ctx, from, to);
    let p = live_params(&a);
    assert!(
        close(
            p.position * DVec3::new(1., 0., 1.),
            DVec3::new(1.5, 0., 1.5)
        ),
        "{:?} ({})",
        p.position,
        a.status
    );
    assert!(
        (p.size.x - 1.).abs() < 1e-9 && (p.size.z - 1.).abs() < 1e-9,
        "{:?}",
        p.size
    );
}

#[test]
fn a_cube_side_haul_snaps_along_its_axis() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::Box);
    draw(&mut a, &ctx, SIZE, vec![]);
    // Draw a cube from (0.2, 0) to (0.8, 2) with snapping off: its +X
    // side's centre is at (0.8, 0, 1).
    let (from, to) = (
        screen(&a, DVec3::new(0.2, 0., 0.)),
        screen(&a, DVec3::new(0.8, 0., 2.)),
    );
    drag(&mut a, &ctx, from, to);
    a.snapping.enabled = true;
    // Haul that side to just short of the point at (1, 0, 1).
    let start = screen(&a, DVec3::new(0.8, 0., 1.)) + vec2(4., 0.);
    let end = screen(&a, DVec3::new(1., 0., 1.)) + vec2(-7., 0.);
    drag(&mut a, &ctx, start, end);
    let p = live_params(&a);
    let max_x = p.position.x + p.size.x / 2.;
    assert!(
        (max_x - 1.).abs() < 1e-9,
        "{:?} {:?} ({})",
        p.position,
        p.size,
        a.status
    );
}

#[test]
fn vertex_create_points_snap() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    a.run(Command::Mode(Mode::Vertex));
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::VertexCreate);
    a.snapping.enabled = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let target = DVec3::new(1., 0., 1.);
    let at = near(&a, target);
    click(&mut a, &ctx, at);
    let path = live_params(&a).sketch_path;
    assert!(close(path[0], target), "{path:?} ({})", a.status);
}

#[test]
fn rotation_snap_angle_rounding_guards_a_zero_step() {
    let mut h = crate::handle_preferences::HandlePreferences::default();
    assert_eq!(h.round_to_snap_angle(22.), 15.);
    h.rotation_snap = 10.;
    assert_eq!(h.round_to_snap_angle(-26.), -30.);
    h.rotation_snap = 0.;
    assert_eq!(h.round_to_snap_angle(22.5), 22.5);
}
