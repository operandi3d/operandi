//! The Node View as a docked panel (node_material_ui.rs, panel_docking.rs):
//! Show as Nodes docks it under the views, it saves with the layout and
//! reopens after a restart, and it follows the Shader Tree's selection.
use super::*;
use crate::layouts_tests::{data_root, frames, launch};
use crate::panel_registry::PanelKind;
use egui_tiles::{Container, LinearDir, Tile};
use operandi_core::shader_tree::{NewLayer, NodeId, NodeKind, ShaderTreeEdit};

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

fn node_views(a: &Operandi) -> usize {
    a.views
        .dock_tabs
        .values()
        .filter(|s| s.kind == "Node View")
        .count()
}

#[test]
fn show_as_nodes_docks_the_node_view_under_the_views_once() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    assert!(!a.registry_panel_open(PanelKind::NodeView));
    let group = add_group(&mut a);
    a.execute(&format!("operandi.nodeMaterial.showGraph layer:{group}"));
    frames(&mut a, &ctx, 2);
    let pane = a
        .registry_dock_pane(PanelKind::NodeView)
        .unwrap_or_else(|| panic!("docked: {}", a.status));
    assert_eq!(a.graph_window.group, Some(group));
    assert!(a.dock_tab_visible(PanelKind::NodeView));
    // Under the views: a stacked split with the view area above it.
    let tree = &a.views.main;
    let slot = a.views.dock_slot(pane).unwrap();
    let parent = tree.tiles.parent_of(slot).unwrap();
    let Some(Tile::Container(Container::Linear(split))) = tree.tiles.get(parent) else {
        panic!("the Node View isn't in a split");
    };
    assert_eq!(split.dir, LinearDir::Vertical);
    let at = split.children.iter().position(|&c| c == slot).unwrap();
    assert!(at > 0, "something sits above it");
    let above = split.children[at - 1];
    let views_above = a
        .views
        .views()
        .into_iter()
        .filter_map(|k| a.views.view_pane(k))
        .filter_map(|p| tree.tiles.find_pane(&p))
        .all(|mut t| {
            loop {
                if t == above {
                    break true;
                }
                match tree.tiles.parent_of(t) {
                    Some(p) => t = p,
                    None => break false,
                }
            }
        });
    assert!(views_above, "every view is above the Node View");
    // Again: the same panel comes to the front, no second copy.
    a.execute(&format!("operandi.nodeMaterial.showGraph layer:{group}"));
    a.execute("operandi.panel.show kind:{Node View}");
    frames(&mut a, &ctx, 2);
    assert_eq!(node_views(&a), 1);
}

/// It's a layout panel: saved with the layout, back after a restart.
#[test]
fn the_docked_node_view_is_saved_with_the_layout() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    a.execute("operandi.panel.show kind:{Node View}");
    frames(&mut a, &ctx, 3);
    assert!(a.registry_panel_open(PanelKind::NodeView), "{}", a.status);
    let doc = a.capture_layout().unwrap();
    assert!(
        serde_json::to_string(&doc).unwrap().contains("Node View"),
        "the layout file names it"
    );
    drop(a);
    let ctx = Context::default();
    let mut b = launch(&root, &ctx);
    frames(&mut b, &ctx, 2);
    assert!(b.registry_panel_open(PanelKind::NodeView));
    assert_eq!(node_views(&b), 1);
    // Empty until a material is picked: no crash, nothing shown.
    assert_eq!(b.graph_window.group, None);
}

/// A new Shader Tree selection moves the view to its material; an
/// explicit Show as Nodes wins until the selection changes again.
#[test]
fn the_node_view_follows_the_selected_material() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    let first = add_group(&mut a);
    let second = add_group(&mut a);
    let inside = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(second))
        .map_or(second, |n| n.id);
    a.shading.selected = vec![inside];
    a.node_view_follow_selection();
    assert_eq!(
        a.graph_window.group,
        Some(second),
        "a layer picks its group"
    );
    // Show as Nodes on the other group, selection unchanged: it stays.
    a.open_group_nodes(first);
    a.node_view_follow_selection();
    assert_eq!(a.graph_window.group, Some(first));
    // Selecting the group itself follows again.
    a.shading.selected = vec![second];
    a.node_view_follow_selection();
    assert_eq!(a.graph_window.group, Some(second));
    // Nothing selected keeps the last material on screen.
    a.shading.selected.clear();
    a.node_view_follow_selection();
    assert_eq!(a.graph_window.group, Some(second));
    frames(&mut a, &ctx, 1);
}
