//! Closing the Node View (panel_close.rs, node_view_open.rs): the ×,
//! the toggling command and menu tick, Esc out of the maximised view, and
//! its floating window's close button.
use super::*;
use crate::layouts_tests::{SIZE, data_root, frames, launch};
use crate::panel_dot::Dot;
use crate::panel_registry::PanelKind;
use crate::tests::draw;
use operandi_core::shader_tree::{NewLayer, NodeId, NodeKind, ShaderTreeEdit};
use operandi_layout::Pane;

fn add_group(a: &mut Operandi) -> NodeId {
    a.editor
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Group,
            selected: None,
        }))
        .unwrap();
    a.editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .rfind(|n| matches!(n.kind, NodeKind::Group(_)))
        .expect("a material group")
        .id
}

/// An app with the Node View docked, showing a fresh Material Group.
fn with_node_view() -> (Operandi, Context, NodeId) {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    let group = add_group(&mut a);
    a.execute(&format!("operandi.nodeMaterial.showGraph layer:{group}"));
    frames(&mut a, &ctx, 3);
    assert!(a.dock_tab_visible(PanelKind::NodeView), "{}", a.status);
    (a, ctx, group)
}

fn pane(a: &Operandi) -> Pane {
    a.registry_dock_pane(PanelKind::NodeView)
        .expect("the Node View is open")
}

fn close_button(a: &Operandi, ctx: &Context) -> Rect {
    ctx.data(|d| d.get_temp::<Rect>(Id::new(("panel-close-test", pane(a)))))
        .expect("the Node View drew its ×")
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![button(true)]);
    draw(a, ctx, SIZE, vec![button(false)]);
    frames(a, ctx, 2);
}

fn ticked(a: &Operandi) -> bool {
    a.line_state("operandi.window.nodeView")
        .is_some_and(|s| s.on)
}

#[test]
fn the_close_button_closes_the_node_view_and_reopening_shows_the_same_material() {
    let (mut a, ctx, group) = with_node_view();
    a.graph_window.zoom = 1.5;
    let x = close_button(&a, &ctx);
    click(&mut a, &ctx, x.center());
    assert!(!a.registry_panel_open(PanelKind::NodeView), "{}", a.status);
    assert!(!ticked(&a), "not ticked once closed");
    // What it showed stays with the app.
    assert_eq!(a.graph_window.group, Some(group));
    a.execute("Node View");
    frames(&mut a, &ctx, 3);
    assert!(a.dock_tab_visible(PanelKind::NodeView), "{}", a.status);
    assert_eq!(a.graph_window.group, Some(group));
    assert_eq!(a.graph_window.zoom, 1.5, "same pan and zoom");
}

#[test]
fn the_node_view_command_toggles_and_the_menus_tick_it() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    let group = add_group(&mut a);
    a.shading.selected = vec![group];
    assert!(!ticked(&a));
    a.execute("operandi.window.nodeView");
    frames(&mut a, &ctx, 3);
    assert!(a.dock_tab_visible(PanelKind::NodeView), "{}", a.status);
    assert!(ticked(&a), "Layout > Windows/Palettes ticks it");
    // Window > Panels ticks what is open (menu_forms.rs `panels`).
    assert!(a.registry_panel_open(PanelKind::NodeView));
    // Again: it closes.
    a.execute("operandi.window.nodeView");
    frames(&mut a, &ctx, 2);
    assert!(!a.registry_panel_open(PanelKind::NodeView), "{}", a.status);
    assert!(!ticked(&a));
    // And once more: back, on the same group.
    a.execute("Node View");
    frames(&mut a, &ctx, 3);
    assert!(a.dock_tab_visible(PanelKind::NodeView));
    assert_eq!(a.graph_window.group, Some(group));
}

#[test]
fn esc_over_the_maximised_node_view_puts_the_layout_back() {
    let (mut a, ctx, _) = with_node_view();
    let p = pane(&a);
    a.toggle_panel_maximized(p);
    frames(&mut a, &ctx, 2);
    assert_eq!(a.panel_maximized, Some(p), "{}", a.status);
    // The full view has its × too.
    ctx.data_mut(|d| d.remove::<Rect>(Id::new(("panel-close-test", p))));
    frames(&mut a, &ctx, 1);
    let x = close_button(&a, &ctx);
    let inside = a
        .dot_panel_rect(Dot::Tab(match p {
            Pane::DockTab(k) => k,
            _ => unreachable!(),
        }))
        .expect("drawn")
        .center();
    assert!(x.is_positive());
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(inside)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    frames(&mut a, &ctx, 1);
    assert_eq!(a.panel_maximized, None, "Esc restored the layout");
    assert!(a.dock_tab_visible(PanelKind::NodeView), "still open");
    // Maximised, the command first restores the layout, then closes.
    a.toggle_panel_maximized(p);
    frames(&mut a, &ctx, 1);
    a.execute("Node View");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.panel_maximized, None);
    assert!(a.dock_tab_visible(PanelKind::NodeView));
    a.execute("Node View");
    frames(&mut a, &ctx, 2);
    assert!(!a.registry_panel_open(PanelKind::NodeView));
}

#[test]
fn closing_the_node_views_floating_window_closes_the_panel() {
    let (mut a, ctx, group) = with_node_view();
    let p = pane(&a);
    let Pane::DockTab(key) = p else {
        panic!("a dock tab")
    };
    a.float_dock_grip(Dot::Tab(key), [600., 400.], Some([100., 100.]))
        .unwrap();
    frames(&mut a, &ctx, 2);
    let window = a.views.window_of(p).expect("floating in its own window");
    assert!(a.close_panels_with_window(window), "the window is gone");
    assert!(!a.registry_panel_open(PanelKind::NodeView));
    assert!(a.views.windows.iter().all(|w| w.id != window));
    frames(&mut a, &ctx, 1);
    // Reopens docked, on the same material.
    a.execute("Node View");
    frames(&mut a, &ctx, 3);
    assert!(a.dock_tab_visible(PanelKind::NodeView));
    assert_eq!(a.graph_window.group, Some(group));
}

#[test]
fn a_queued_close_waits_for_the_frame_and_a_groups_only_tab_stays() {
    let (mut a, ctx, _) = with_node_view();
    a.request_close_panel(PanelKind::NodeView);
    frames(&mut a, &ctx, 1);
    assert!(!a.registry_panel_open(PanelKind::NodeView));
    // A sidebar group's only tab stays (the group would be empty).
    let only = a.upper_frame.tabs.len() == 1;
    if let Some(&front) = a.upper_frame.tabs.first() {
        let r = a.close_registry_panel(front);
        assert_eq!(r.is_err(), only);
    }
}
