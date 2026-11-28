//! Falloff batch 3 (falloff spec 2026-11-15): Cylinder in Tool Properties
//! and the view, per-axis handles (FO-40..42), drawing by click-drag
//! (FO-05) and Use World Transforms (FO-35).
use super::tests::{box_app, draw};
use super::*;
use crate::falloff_panel::{FalloffFields, FalloffP};
use crate::tool_form::{Item, ToolFields};
use crate::viewport::falloff_view::FalloffHandle;
use operandi_core::CylinderFalloff;

const SIZE: Vec2 = vec2(1440., 900.);

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

fn rows(a: &Operandi) -> Vec<(&'static str, Vec<String>)> {
    let p: FalloffP = FalloffFields::params(a).unwrap();
    FalloffFields::form(a, &p)
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Section { name, items, .. } => Some((
                *name,
                items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Field(f) => Some(f.row.label.clone()),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

/// FO-42 rows in order; Use World Transforms is a live checkbox on the
/// geometric falloffs (FO-35).
#[test]
fn cylinder_section_rows() {
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(
        CylinderFalloff::default().into(),
    )));
    let s = rows(&a);
    assert_eq!(s[0].0, "Cylinder Falloff");
    assert_eq!(
        s[0].1,
        [
            "Center X",
            "Y",
            "Z",
            "Size X",
            "Y",
            "Z",
            "Axis",
            "Use World Transforms",
            "Shape",
            "In",
            "Out",
            "Mix Mode"
        ]
    );
    let mut p = FalloffFields::params(&a).unwrap();
    if let Falloff::Cylinder(c) = &mut p.layers[0].0 {
        c.world = true;
    }
    FalloffFields::publish(&mut a, p).unwrap();
    assert!(a.editor.spatial_falloff.unwrap().uses_world());
}

/// FO-40: Linear has three per-axis handles at each end; dragging Start's
/// X handle moves Start along X only.
#[test]
fn per_axis_handle_moves_one_axis() {
    let mut a = box_app();
    let r = rect();
    let field = LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::Y,
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(field.into())));
    let handles = a.falloff_handles(r);
    assert_eq!(handles.len(), 2 + 6);
    let (i, &(at, _)) = handles
        .iter()
        .enumerate()
        .find(|(_, h)| h.1 == FalloffHandle::Axis(0, 0))
        .unwrap();
    let q = a.camera.project(at, r).unwrap().0;
    assert_eq!(a.falloff_hit(q, r), Some(i));
    a.drag_falloff(i, field.into(), q, q + vec2(40., -40.), r);
    let Some(Falloff::Linear(moved)) = a.editor.spatial_falloff else {
        panic!()
    };
    assert!(moved.start.x != 0.);
    assert_eq!((moved.start.y, moved.start.z), (0., 0.));
    assert_eq!(moved.end, field.end);
}

/// FO-35: with Use World Transforms off the handles sit where the
/// falloff's own-space positions land on the moved mesh.
#[test]
fn own_space_handles_follow_the_mesh() {
    let mut a = box_app();
    a.editor.scene.active_mut().unwrap().transform.translation = DVec3::new(3., 0., 0.);
    a.run(Command::SpatialFalloff(Some(
        RadialFalloff::default().into(),
    )));
    assert!((a.falloff_points()[0] - DVec3::new(3., 0., 0.)).length() < 1e-9);
    a.run(Command::SpatialFalloff(Some(
        RadialFalloff {
            world: true,
            ..Default::default()
        }
        .into(),
    )));
    assert_eq!(a.falloff_points()[0], DVec3::ZERO);
}

/// FO-05: choosing a geometric falloff arms drawing; the press sets the
/// origin, the drag the size, and it's then disarmed.
#[test]
fn press_and_drag_draws_the_falloff() {
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(
        RadialFalloff::default().into(),
    )));
    a.falloff_draw_armed = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = a.viewport_rect.left_top() + vec2(120., 120.);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![button(at, true)]);
    for step in 1..=4 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(at + vec2(20. * step as f32, 0.))],
        );
    }
    draw(&mut a, &ctx, SIZE, vec![button(at + vec2(80., 0.), false)]);
    let Some(Falloff::Radial(r)) = a.editor.spatial_falloff else {
        panic!()
    };
    assert_ne!(r.center, DVec3::ZERO, "{}", a.status);
    assert!(r.size.x > 0. && r.size.x == r.size.y && r.size.y == r.size.z);
    assert!(!a.falloff_draw_armed);
}

/// #65 (Leo): with snapping on, dragging a Linear falloff's End near a
/// box corner puts it exactly on the corner; Start stays where it was.
#[test]
fn falloff_points_snap_while_dragged() {
    let mut a = box_app();
    let r = rect();
    a.snapping.enabled = true;
    let field = LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::new(0., 3., 0.),
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(field.into())));
    let (i, &(at, _)) = a
        .falloff_handles(r)
        .iter()
        .enumerate()
        .find(|(_, h)| h.1 == FalloffHandle::Point(1))
        .unwrap();
    let from = a.camera.project(at, r).unwrap().0;
    // A box corner facing the eye.
    let (eye, ..) = a.camera.basis();
    let corner = DVec3::new(1., 1., eye.z.signum());
    let near = a.camera.project(corner, r).unwrap().0 + vec2(-3., 2.);
    a.drag_falloff(i, field.into(), from, near, r);
    let Some(operandi_core::Falloff::Linear(f)) = a.editor.spatial_falloff else {
        panic!("{}", a.status)
    };
    assert!((f.end - corner).length() < 1e-6, "{} ({})", f.end, a.status);
    assert_eq!(f.start, DVec3::ZERO);
}
