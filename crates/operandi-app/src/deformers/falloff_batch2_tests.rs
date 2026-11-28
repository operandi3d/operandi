//! Falloff batch 2 (falloff spec 2026-11-15): Soft Selection, Element and
//! Coplanar in Tool Properties and the viewport (FO-45, FO-47, FO-48).
use super::tests::{box_app, draw};
use super::*;
use crate::falloff_panel::{FalloffFields, FalloffP};
use crate::tool_form::{Item, ToolFields, ToolForm};
use operandi_core::{CoplanarFalloff, ElementFalloff, MixMode, SoftSelectionFalloff, TweakConnect};

const SIZE: Vec2 = vec2(1440., 900.);

fn app_with(tool: Tool) -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    a.activate(tool);
    assert_eq!(a.editor.session.live.as_ref().map(|l| l.tool), Some(tool));
    a
}

fn form(a: &Operandi) -> ToolForm<FalloffP> {
    let p = FalloffFields::params(a).expect("a falloff is active");
    FalloffFields::form(a, &p)
}

fn sections(f: &ToolForm<FalloffP>) -> Vec<(&'static str, Vec<String>)> {
    f.items
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

fn field<'a>(f: &'a ToolForm<FalloffP>, key: &str) -> &'a tool_form::Field<FalloffP> {
    f.fields()
        .into_iter()
        .find(|x| x.row.key == key)
        .unwrap_or_else(|| panic!("no field {key}"))
}

/// FO-45/47/48: each section lists the spec's rows in order, Mix Mode last;
/// Range greys under Rigid Connections; Show Vertex/Edge grey on an added
/// layer.
#[test]
fn sections_list_the_spec_rows() {
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::SoftSelection(
        SoftSelectionFalloff::default(),
    ))));
    a.run(Command::AddedFalloffs(vec![
        (
            Falloff::Element(ElementFalloff {
                connect: TweakConnect::Rigid,
                ..Default::default()
            }),
            MixMode::Max,
        ),
        (
            Falloff::Coplanar(CoplanarFalloff::default()),
            MixMode::Multiply,
        ),
    ]));
    let f = form(&a);
    let s = sections(&f);
    let shape = ["Shape", "In", "Out"];
    let mut soft = vec!["Radius", "Use Connectivity"];
    soft.extend(shape);
    soft.extend(["Show Vertex", "Show Edge", "Mix Mode"]);
    assert_eq!(s[0].0, "Soft Selection Falloff");
    assert_eq!(s[0].1, soft);
    let mut el = vec![
        "Element Mode",
        "Range",
        "Connected Elements",
        "Always Raycast",
    ];
    el.extend(shape);
    el.push("Mix Mode");
    assert_eq!(s[1].0, "Element Falloff");
    assert_eq!(s[1].1, el);
    let mut cop = vec!["Angle", "Use Connectivity"];
    cop.extend(shape);
    cop.extend(["Show Vertex", "Show Edge", "Mix Mode"]);
    assert_eq!(s[2].0, "Coplanar Falloff");
    assert_eq!(s[2].1, cop);
    assert!(field(&f, "falloff1_el_range").disabled_tip.is_some());
    assert!(field(&f, "falloff0_show_edge").disabled_tip.is_none());
    assert!(field(&f, "falloff2_show_edge").disabled_tip.is_some());
}

/// FO-Q05 default: a right-drag in the view changes Soft Selection's Radius
/// (right for more), and the tool stays.
#[test]
fn right_drag_changes_soft_selection_radius() {
    let ctx = Context::default();
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::SoftSelection(
        SoftSelectionFalloff::default(),
    ))));
    draw(&mut a, &ctx, SIZE, vec![]);
    let radius = |a: &Operandi| match a.editor.spatial_falloff {
        Some(Falloff::SoftSelection(s)) => s.radius,
        other => panic!("{other:?}"),
    };
    let from = a.viewport_rect.left_top() + vec2(80., 80.);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(&mut a, &ctx, SIZE, vec![button(from, true)]);
    for step in 1..=4 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(from + vec2(15. * step as f32, 0.))],
        );
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![button(from + vec2(60., 0.), false)],
    );
    assert!(radius(&a) > 0.1, "{} {}", radius(&a), a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

/// FO-45: with Move and an Element falloff, a press on the box picks the
/// element under the pointer and the drag moves only it; one undo step
/// brings the box back.
#[test]
fn element_press_drags_the_picked_element() {
    let ctx = Context::default();
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Element(
        ElementFalloff::default(),
    ))));
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let at = a.viewport_rect.center() + vec2(25., 25.); // off the Move handle
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![button(at, true)]);
    assert!(a.editor.falloff_element.is_some(), "{}", a.status);
    for step in 1..=4 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(at + vec2(0., -10. * step as f32))],
        );
    }
    draw(&mut a, &ctx, SIZE, vec![button(at + vec2(0., -40.), false)]);
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let moved = before.iter().zip(&after).filter(|(b, a)| b != a).count();
    assert!(
        moved > 0 && moved < before.len(),
        "moved {moved}: {}",
        a.status
    );
    a.run(Command::Drop);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
}
