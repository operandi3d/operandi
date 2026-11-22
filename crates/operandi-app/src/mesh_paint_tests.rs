//! Mesh Paint's placer in the app (missing-tools-g6 MP-03..08, MP-15,
//! MP-18): each paint mode driven through the viewport gesture, from the
//! Duplicate and Create tab buttons' commands.
use super::create_utility::Utility;
use super::tests::draw;
use super::*;
use operandi_core::mesh_paint::{MeshPaintSettings, PaintMode};

/// A unit cube (the background) and a 10 m plane facing up (the target,
/// selected), seen from the top.
fn app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::InsertUnit {
        tool: Tool::Box,
        destination: operandi_core::UnitDestination::NewMesh,
    });
    a.run(Command::InsertShapeUnit(
        operandi_core::UnitShape::Plane,
        operandi_core::UnitDestination::NewMesh,
    ));
    let mut plane: operandi_core::Mesh = (*a.editor.scene.active().unwrap().mesh).clone();
    for v in &mut plane.vertices {
        *v *= 10.;
    }
    a.editor.scene.active_mut().unwrap().mesh = plane.into();
    a.editor.synchronize_identities().unwrap();
    a.camera.view = View::Top;
    draw(&mut a, ctx, vec2(1440., 900.), vec![]);
    a
}

fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

/// One gesture from `from` to `to` with Mesh Paint on in `mode`.
fn paint(a: &mut Operandi, mode: PaintMode, from: Vec2, to: Vec2, mods: Modifiers) {
    if let Some(Utility::MeshPaint(s)) = &mut a.utility {
        s.mode = mode;
    }
    let r = a.viewport_rect;
    let (p, q) = (r.center() + from, r.center() + to);
    let Some(input::GestureKind::Utility(drag)) = a.utility_press(p, r, mods, false) else {
        panic!("Mesh Paint takes the press: {}", a.status);
    };
    for k in 1..=10 {
        a.utility_drag(&drag, p, p + (q - p) * (k as f32 / 10.), r);
    }
    a.utility_release(&drag, q, r);
}

#[test]
fn each_paint_mode_places_its_copies_as_one_step() {
    let ctx = Context::default();
    let mut a = app(&ctx);
    a.execute("Mesh Paint");
    assert!(
        matches!(a.utility, Some(Utility::MeshPaint(s)) if !s.instances),
        "{}",
        a.status
    );
    let cube = 6;
    for (mode, from, to, copies) in [
        // MP-03: one copy where the drag ends.
        (PaintMode::Slide, vec2(-50., 0.), vec2(50., 0.), 1),
        // MP-05: one copy at the click.
        (PaintMode::Drop, vec2(0., 0.), vec2(40., 0.), 1),
        (PaintMode::DropNoScale, vec2(0., 0.), vec2(0., 40.), 1),
        // MP-06..08: Linear Count 2, Array 2 × 2, Radial Count 6.
        (PaintMode::Linear, vec2(-100., 0.), vec2(100., 0.), 2),
        (PaintMode::Array, vec2(-100., -100.), vec2(100., 100.), 4),
        (PaintMode::Radial, vec2(0., 0.), vec2(100., 0.), 6),
    ] {
        let (before, steps) = (faces(&a), a.editor.undo.len());
        paint(&mut a, mode, from, to, Modifiers::NONE);
        assert_eq!(faces(&a) - before, copies * cube, "{mode:?}: {}", a.status);
        assert_eq!(a.editor.undo.len(), steps + 1, "{mode:?}: one step (MP-16)");
    }
}

#[test]
fn strokes_scatter_copies_inside_the_brush() {
    let ctx = Context::default();
    let mut a = app(&ctx);
    a.execute("Mesh Paint");
    let before = faces(&a);
    paint(
        &mut a,
        PaintMode::Strokes,
        vec2(-200., 0.),
        vec2(200., 0.),
        Modifiers::NONE,
    );
    let added = (faces(&a) - before) / 6;
    assert!(added > 1, "{}", a.status);
    // Every copy sits within the brush of the stroke's line (top view:
    // screen y is world -z; 20 px at this zoom is well under 1 m).
    let m = &a.editor.scene.active().unwrap().mesh;
    for v in &m.vertices[4..] {
        assert!(v.z.abs() < 2., "{v}");
    }
}

#[test]
fn mesh_paint_instance_makes_instance_items() {
    let ctx = Context::default();
    let mut a = app(&ctx);
    let cube = a.editor.scene.items[0].id;
    a.execute("Mesh Paint Instance");
    assert!(matches!(
        a.utility,
        Some(Utility::MeshPaint(MeshPaintSettings {
            instances: true,
            ..
        }))
    ));
    if let Some(Utility::MeshPaint(s)) = &mut a.utility {
        s.parent = operandi_core::duplicate_items::ParentMode::Root;
    }
    paint(
        &mut a,
        PaintMode::Linear,
        vec2(-100., 0.),
        vec2(100., 0.),
        Modifiers::NONE,
    );
    let copies: Vec<_> = a
        .editor
        .scene
        .items
        .iter()
        .filter(|i| i.instance_of == Some(cube))
        .collect();
    assert_eq!(copies.len(), 2, "{}", a.status);
    assert!(copies.iter().all(|i| i.parent == Some(cube)), "MP-15 Root");
    assert_eq!(faces(&a), 1, "the target stays as it was");
}

#[test]
fn without_a_background_it_says_so() {
    let ctx = Context::default();
    let mut a = app(&ctx);
    // Hide the cube: no visible background.
    a.editor.scene.items.remove(0);
    a.execute("Mesh Paint");
    let r = a.viewport_rect;
    a.utility_press(r.center(), r, Modifiers::NONE, false);
    assert_eq!(a.status, operandi_core::mesh_paint::NO_SOURCE);
}

/// MP-18: both tabs' Mesh Paint buttons wear the rendered icons, the Alt
/// variant its own.
#[test]
fn the_buttons_wear_their_rendered_icons() {
    use crate::form::command_form::{Control, Variant};
    let a = Operandi::empty();
    for form in ["toolbox.create", "toolbox.duplicate"] {
        fn find(controls: &[Control]) -> Option<&Control> {
            controls
                .iter()
                .find(|c| c.command.as_deref() == Some("Mesh Paint"))
        }
        let control = find(&a.forms.get(form).unwrap().controls).expect(form);
        assert_eq!(control.icon.as_deref(), Some("mesh-paint"), "{form}");
        assert!(icons::Icon::from_slug("mesh-paint").is_some());
        match &control.variants.alt {
            Some(Variant::Shown { icon, .. }) => {
                assert_eq!(icon, "mesh-paint-instance");
                assert!(icons::Icon::from_slug(icon).is_some());
            }
            other => panic!("{form}: {other:?}"),
        }
    }
}
