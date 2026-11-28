//! Panel popovers (panel_popover.rs; the Panels pie, Jason 2026-11-28):
//! one panel on its own near the pointer, beside the docked one, closing
//! when the pointer rolls off unless pinned.
use super::*;
use crate::panel_popover::{PopPanel, line_panel};
use crate::panel_registry::PanelKind;
use crate::tests::{box_app, draw};

const SIZE: Vec2 = vec2(1400., 900.);

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
}

/// Every popover panel has a line that names it back, and unknown ones
/// only report.
#[test]
fn every_panel_has_a_line() {
    for panel in PopPanel::all() {
        assert_eq!(line_panel(&panel.line()), Some(panel), "{}", panel.line());
    }
    assert_eq!(
        PopPanel::from_id("shaderTree"),
        Some(PopPanel::Kind(PanelKind::ShaderTree))
    );
    assert_eq!(
        line_panel("operandi.panel.popover items"),
        Some(PopPanel::Kind(PanelKind::Items))
    );
    let mut a = box_app();
    a.execute("operandi.panel.popover panel:nothing");
    assert!(a.status.contains("no panel nothing"), "{}", a.status);
    assert!(a.panel_popover.is_none());
}

/// Items opens on its own, near the pointer, and the docked panels stay
/// where they are: nothing floats, no palette opens.
#[test]
fn items_pops_out_alone_beside_the_docked_panels() {
    let mut a = box_app();
    let ctx = Context::default();
    let at = pos2(600., 400.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(at)]);
    let docked = a.viewport_rect;
    a.execute("operandi.panel.popover panel:items");
    frame(&mut a, &ctx, vec![]);
    let rect = a
        .panel_popover
        .as_ref()
        .and_then(|p| p.rect)
        .expect("drawn");
    assert!(rect.contains(at), "the pointer starts inside: {rect:?}");
    assert!(!a.is_floating(crate::floating::Panel::Items));
    assert!(!a.palette_open("items"));
    assert_eq!(a.viewport_rect, docked, "the side column is untouched");
    // Only one popover: Properties replaces it, and draws beside the
    // docked Properties.
    a.execute("operandi.panel.popover panel:properties");
    assert!(a.panel_popover_open(PopPanel::Kind(PanelKind::Properties)));
    assert!(!a.panel_popover_open(PopPanel::Kind(PanelKind::Items)));
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(at + vec2(4., 4.))]);
    assert!(a.panel_popover.is_some());
}

/// Rolling off closes an unpinned one (once the pointer has been over
/// it); the circle pins it, and a pinned one stays.
#[test]
fn rolling_off_closes_it_unless_pinned() {
    let mut a = box_app();
    let ctx = Context::default();
    let at = pos2(600., 400.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(at)]);
    a.execute("operandi.panel.popover panel:toolbox");
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(at + vec2(4., 4.))]);
    assert!(a.panel_popover.is_some(), "still over it");
    let rect = a.panel_popover.as_ref().unwrap().rect.unwrap();
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(pos2(
            rect.right() + 60.,
            rect.center().y,
        ))],
    );
    assert!(a.panel_popover.is_none(), "rolled off");
    // Pinned, it stays.
    frame(&mut a, &ctx, vec![Event::PointerMoved(at)]);
    a.execute("operandi.panel.popover panel:toolbox");
    frame(&mut a, &ctx, vec![]);
    a.panel_popover.as_mut().unwrap().pinned = true;
    frame(&mut a, &ctx, vec![Event::PointerMoved(pos2(20., 880.))]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.panel_popover.is_some(), "pinned stays");
    // Esc closes an unpinned one; running its line again closes any.
    a.execute("operandi.panel.popover panel:toolbox");
    assert!(a.panel_popover.is_none());
}

/// The Timeline and Graph Editor pop out too, drawing their usual
/// contents in the popover.
#[test]
fn timeline_and_graph_editor_pop_out() {
    let mut a = box_app();
    let ctx = Context::default();
    frame(&mut a, &ctx, vec![Event::PointerMoved(pos2(500., 400.))]);
    for id in ["timeline", "graphEditor"] {
        a.execute(&format!("operandi.panel.popover panel:{id}"));
        frame(&mut a, &ctx, vec![]);
        let p = a.panel_popover.as_ref().expect(id);
        assert_eq!(p.panel.id(), id);
        assert!(
            p.rect.is_some_and(|r| r.width() > 600.),
            "{id}: {:?}",
            p.rect
        );
    }
    assert!(!a.timeline.open, "the docked timeline stays as it was");
    // With the docked timeline showing too.
    a.timeline.open = true;
    a.execute("operandi.panel.popover panel:timeline");
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.panel_popover_open(PopPanel::Timeline));
}
