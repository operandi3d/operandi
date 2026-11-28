//! GitHub #15 in the app: with a Cube, Sphere or Cylinder live, the
//! falloff's handles can be dragged and the falloff switched without the
//! tool dropping; the Falloff section starts folded and greys its rows with
//! a reason when no usable falloff is on (UIC-105/106, UIC-117).
use super::*;
use crate::tool_form::{Item, ToolFields};
use operandi_core::{LinearFalloff, SelectionFalloff};

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}
fn live_cylinder() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Cylinder);
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.radial = Some(operandi_core::RadialParameters {
        radius: DVec3::new(0.5, 0.75, 0.5),
        sides: 12,
        segments: 6,
    });
    p.shape_falloff.scale = 0.2;
    a.run(Command::Preview(p));
    a
}
fn falloff_section(a: &Operandi) -> (bool, Vec<bool>) {
    let p = crate::primitive_panel::PrimitiveFields::params(a).unwrap();
    let form = crate::primitive_panel::PrimitiveFields::form(a, &p);
    for item in &form.items {
        if let Item::Section {
            name: "Falloff",
            folded,
            items,
        } = item
        {
            let enabled = items
                .iter()
                .filter_map(|i| match i {
                    Item::Field(f) => Some(f.disabled_tip.is_none()),
                    _ => None,
                })
                .collect();
            return (*folded, enabled);
        }
    }
    panic!("no Falloff section");
}

#[test]
fn a_falloff_handle_drag_reshapes_the_live_cylinder_and_keeps_the_tool() {
    let mut a = live_cylinder();
    let r = rect();
    // Start above the shape: on the top radius square the tool's handle
    // would win the press (TP-19).
    let field = LinearFalloff {
        start: DVec3::new(0., 1.2, 0.),
        end: DVec3::new(0., -0.75, 0.),
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(field.into())));
    assert!(a.editor.session.live.is_some(), "{}", a.status);
    let shaped = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let handles = a.falloff_handles(r);
    let (at, _) = handles[0];
    let q = a.camera.project(at, r).unwrap().0;
    assert_eq!(
        a.falloff_hit(q, r),
        Some(0),
        "the falloff's Start is pickable"
    );
    a.drag_falloff(0, field.into(), q, q + vec2(0., -80.), r);
    assert!(a.editor.session.live.is_some(), "still the Cylinder tool");
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Cylinder);
    assert_ne!(
        a.editor.scene.active().unwrap().mesh.vertices,
        shaped,
        "the preview follows the falloff"
    );
}

#[test]
fn the_falloff_section_starts_folded_and_greys_without_a_usable_falloff() {
    let mut a = live_cylinder();
    let (folded, enabled) = falloff_section(&a);
    assert!(folded, "UIC-106: a rarely used section starts folded");
    assert!(enabled.iter().all(|on| !on), "no falloff: greyed");
    a.run(Command::SpatialFalloff(Some(
        LinearFalloff::default().into(),
    )));
    assert!(falloff_section(&a).1.iter().all(|on| *on));
    a.run(Command::SpatialFalloff(Some(
        operandi_core::Falloff::Selection(SelectionFalloff::default()),
    )));
    assert!(a.editor.session.live.is_some());
    assert!(
        falloff_section(&a).1.iter().all(|on| !on),
        "Selection: greyed"
    );
}

/// GitHub #35: the numeric Torus with a falloff on, its preview and its
/// Falloff section.
fn armed_torus() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::ArmShape(operandi_core::ShapeKind::Torus));
    let mut t = operandi_core::TorusParameters::default();
    t.falloff.scale = 0.2;
    a.run(Command::ConfigureShape(
        operandi_core::ShapeParameters::Torus(t),
    ));
    assert!(
        a.editor
            .session
            .shape
            .as_ref()
            .is_some_and(|s| s.preview.is_some()),
        "{}",
        a.status
    );
    a
}
fn torus_preview(a: &Operandi) -> Vec<DVec3> {
    a.editor
        .session
        .shape
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .vertices
        .clone()
}

#[test]
fn a_falloff_handle_drag_reshapes_the_open_torus_and_keeps_the_tool() {
    let mut a = armed_torus();
    let plain = torus_preview(&a);
    let r = rect();
    let field = LinearFalloff {
        start: DVec3::new(0., 1.2, 0.),
        end: DVec3::new(0., -0.75, 0.),
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(field.into())));
    assert!(a.editor.session.shape.is_some(), "{}", a.status);
    let shaped = torus_preview(&a);
    assert_ne!(shaped, plain, "the falloff shapes the preview");
    let (at, _) = a.falloff_handles(r)[0];
    let q = a.camera.project(at, r).unwrap().0;
    assert_eq!(
        a.falloff_hit(q, r),
        Some(0),
        "the falloff's Start is pickable"
    );
    a.drag_falloff(0, field.into(), q, q + vec2(0., -80.), r);
    assert!(a.editor.session.shape.is_some(), "still the Torus tool");
    assert_ne!(torus_preview(&a), shaped, "the preview follows the drag");
}

#[test]
fn the_torus_has_the_falloff_section_greyed_until_a_falloff_is_on() {
    use crate::shape_panel::ShapeFields;
    let mut a = armed_torus();
    let section = |a: &Operandi| {
        let p = ShapeFields::params(a).unwrap();
        ShapeFields::form(a, &p)
            .items
            .iter()
            .find_map(|i| match i {
                Item::Section {
                    name: "Falloff",
                    folded,
                    items,
                } => Some((
                    *folded,
                    items
                        .iter()
                        .filter_map(|i| match i {
                            Item::Field(f) => Some(f.disabled_tip.is_none()),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .expect("a Falloff section")
    };
    let (folded, enabled) = section(&a);
    assert!(folded);
    assert!(enabled.iter().all(|on| !on), "no falloff: greyed");
    a.run(Command::SpatialFalloff(Some(
        LinearFalloff::default().into(),
    )));
    // Every row but Spacing (a torus's rings can't move).
    assert_eq!(section(&a).1.iter().filter(|on| !**on).count(), 1);
}
