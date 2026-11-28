//! Action Center rule sheet v6, step 3, seen from the app: the handle the
//! view draws and drags along is the pivot service's answer
//! (`Editor::handle_frame`), also for the mixing rules. The service itself
//! is tested in core (`ac_rules_step3.rs`).
use super::tests::{draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

fn quad(v: [u32; 4]) -> operandi_core::Face {
    operandi_core::Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

const SLOPE: DVec3 = DVec3::new(0., 0.6, 0.8);
const B_NORMAL: DVec3 = DVec3::new(0., 0.8, -0.6);
const B_CENTRE: DVec3 = DVec3::new(5.5, 0.45, 0.6);

/// Two separate quads: A flat on the ground (points 0..4), B running up a
/// slope (points 4..8), as in core's `ac_rules_step3.rs`.
fn two_clusters() -> operandi_core::Mesh {
    operandi_core::Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 0., 1.),
            DVec3::new(2., 0., 1.),
            DVec3::new(2., 0., 0.),
            DVec3::new(5., 0., 0.),
            DVec3::new(5., 0.9, 1.2),
            DVec3::new(6., 0.9, 1.2),
            DVec3::new(6., 0., 0.),
        ],
        faces: vec![quad([0, 1, 2, 3]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
}

fn app(ctx: &Context, context: ActionContext) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = two_clusters().into();
    a.camera.view = View::Front;
    a.camera.target = DVec3::new(3., 0.5, 0.);
    a.camera.distance = 16.;
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    assert!(a.run_ok(Command::ActionContext(context)), "{}", a.status);
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}

/// The arrows and the handle's place are the service's handle frame under
/// Local, under Local's centre with World axes, and under the Origin with
/// Local's axes.
#[test]
fn the_view_draws_the_services_handle() {
    let local_b = glam::DMat3::from_cols(DVec3::X, -SLOPE, B_NORMAL);
    for (context, center, basis) in [
        (ActionContext::LOCAL, B_CENTRE, local_b),
        (
            ActionContext {
                center: CenterMode::Local,
                axis: AxisOrientation::World,
            },
            B_CENTRE,
            glam::DMat3::IDENTITY,
        ),
        (
            ActionContext {
                center: CenterMode::Origin,
                axis: AxisOrientation::Local,
            },
            DVec3::ZERO,
            local_b,
        ),
    ] {
        let ctx = Context::default();
        let mut a = app(&ctx, context);
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
            a.activate(tool);
            let handle = a.editor.handle_frame().expect("a handle frame");
            assert_eq!(a.transform_handle_frame(), Some(handle));
            close(a.transform_handle_center(), center, "handle centre");
            for axis in 0..3 {
                close(a.handle_axis(axis), basis.col(axis), "arrow");
            }
            a.run(Command::Cancel);
        }
    }
}

/// A drag on the handle's Z arrow under the Origin with Local's axes moves
/// every selected point along that arrow, the last-selected cluster's
/// normal: the drag, the arrow and the preview use the one frame.
#[test]
fn an_arrow_drag_moves_along_the_services_axis() {
    let ctx = Context::default();
    let mut a = app(
        &ctx,
        ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::Local,
        },
    );
    a.activate(Tool::Move);
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    let (_, origin, end, label) = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 2)
        .expect("the Z arrow");
    let at = origin + (end - origin) * 0.7;
    assert_eq!(a.handle_hit(at, r), Some(2), "{label}");
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let along = (end - origin).normalized() * 60.;
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
    let after = &a.editor.scene.active().unwrap().mesh.vertices;
    let first = after[0] - before[0];
    assert!(first.length() > 0.1, "{first} ({})", a.status);
    close(first.normalize(), B_NORMAL, "along the cluster's normal");
    for v in 0..8 {
        close(after[v] - before[v], first, "every point alike");
    }
}
