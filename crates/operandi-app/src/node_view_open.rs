//! Window ▸ Node View and its key (Cmd+Shift+N): opens the node view of
//! the Material Group the Shader Tree selection is in, without going
//! through the layer's right-click Show as Nodes.
//!
//! Which group (provisional, no spec yet):
//! - the selected layer itself when it is a Material Group;
//! - otherwise the nearest Material Group it sits in (a layer, or a
//!   Folder, inside a group);
//! - with nothing suitable selected, the group the node view shows now,
//!   else the first Material Group in the Shader Tree;
//! - with no Material Group at all, nothing opens and the status line
//!   says what to select.
//!
//! Only `open_group_nodes` is called, so this works the same whether the
//! node view is a floating window or a docked panel.
//!
//! The command toggles (UIC-205's "opened and closed by the same toggle";
//! Jason, 2026-11-21: the node view had no way to close): with the Node
//! View open and in front it closes (panel_close.rs); maximised, the first
//! press puts its group back and the next closes it. Open behind another
//! tab, it comes to the front. Window ▸ Panels ▸ Node View and Layout ▸
//! Windows/Palettes ▸ Node View are ticked while it is open.
//!
//! Departs from UIC-184 (a key opens a panel as a popover, the Window
//! menu as a floating window): the node view has no popover form, so the
//! key and the menu open the same node view.
use crate::Operandi;
use operandi_core::shader_tree::{NodeId, NodeKind, ShaderTree};

/// The status line when there's no group to show.
pub(crate) const NOTHING_TO_SHOW: &str =
    "Node View: the scene has no Material Group; add one in the Shader Tree";

/// The Material Group `layer` is, or the nearest one it sits in.
pub(crate) fn enclosing_group(tree: &ShaderTree, layer: NodeId) -> Option<NodeId> {
    let mut at = tree.get(layer);
    while let Some(node) = at {
        if matches!(node.kind, NodeKind::Group(_)) {
            return Some(node.id);
        }
        at = node.parent.and_then(|p| tree.get(p));
    }
    None
}

impl Operandi {
    /// Window ▸ Node View / its key: close the Node View when it is open
    /// and in front, else open it (see the module notes).
    pub(crate) fn toggle_node_view(&mut self) {
        use crate::panel_registry::PanelKind;
        // A Show as Nodes this frame is opening it, not closing it.
        if self.graph_window.reveal || !self.registry_panel_in_front(PanelKind::NodeView) {
            // Closed (or hidden) with a material still kept and the
            // selection where it was: back as it was left, same material,
            // pan and zoom.
            let w = &self.graph_window;
            if (w.group.is_some() || w.graph.is_some())
                && self.shading.selected.last().copied() == w.followed
            {
                self.graph_window.reveal = true;
                return;
            }
            self.open_node_view();
            return;
        }
        if let Some(pane) = self.registry_dock_pane(PanelKind::NodeView)
            && self.panel_maximized == Some(pane)
        {
            self.toggle_panel_maximized(pane);
            return;
        }
        self.request_close_panel(PanelKind::NodeView);
    }

    /// Window ▸ Node View / its key: the node view of the selection's
    /// Material Group, else the one shown or the first, else a status
    /// message.
    pub(crate) fn open_node_view(&mut self) {
        let tree = &self.editor.scene.shader_tree;
        let group = self
            .shading
            .selected
            .last()
            .and_then(|&l| enclosing_group(tree, l))
            .or_else(|| {
                self.graph_window
                    .group
                    .filter(|&g| enclosing_group(tree, g) == Some(g))
            })
            .or_else(|| {
                tree.nodes
                    .iter()
                    .find(|n| matches!(n.kind, NodeKind::Group(_)))
                    .map(|n| n.id)
            });
        match group {
            Some(g) => self.open_group_nodes(g),
            None => self.status = NOTHING_TO_SHOW.into(),
        }
    }
}
