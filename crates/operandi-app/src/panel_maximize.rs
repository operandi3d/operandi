//! Maximise any docked panel (MN-08): the Maximise keys (Numpad 0, Alt+0)
//! pressed with the pointer over a docked panel that isn't a view (Items,
//! Properties, a sidebar tab such as Shader Tree or Groups, Tool
//! Properties, the Toolbox) make it fill its group; the key again over the
//! same panel puts the group back. Views keep their own Maximise.
//!
//! - The pointer decides the target, not the keyboard focus (MN-08.2); a
//!   focused text field takes the key as typing (the keyboard handler
//!   gives text fields every key first).
//! - "Its group" is the split the panel (or the tab group holding it) sits
//!   in: its neighbours there hide, the rest of the window stays.
//! - Outside the dock tree (the shipped layouts' side columns), Items and
//!   Properties share the right column and the tool drawer and Tool
//!   Properties the left one; Maximise fills the column with one of them.
//! - Provisional: per session, not saved with the layout (views' Maximise
//!   is).
use super::Operandi;
use eframe::egui::Pos2;
use egui_tiles::{Container, Tile, TileId, Tree};
use operandi_layout::Pane;

/// The tile that stands for `pane` in its group: its tab group when it
/// is one tab of several, else its own tile.
fn group_member(tree: &Tree<Pane>, pane: Pane) -> Option<TileId> {
    let id = tree.tiles.find_pane(&pane)?;
    Some(match tree.tiles.parent_of(id) {
        Some(parent)
            if matches!(
                tree.tiles.get(parent),
                Some(Tile::Container(Container::Tabs(_)))
            ) =>
        {
            parent
        }
        _ => id,
    })
}

/// The tiles that hide while `pane` is maximised: everything else in its
/// group.
pub(crate) fn siblings(tree: &Tree<Pane>, pane: Pane) -> Vec<TileId> {
    let Some(member) = group_member(tree, pane) else {
        return Vec::new();
    };
    let Some(parent) = tree.tiles.parent_of(member) else {
        return Vec::new();
    };
    match tree.tiles.get(parent) {
        Some(Tile::Container(c)) => c.children().copied().filter(|&c| c != member).collect(),
        _ => Vec::new(),
    }
}

/// A docked panel that Maximise can act on (a view has its own).
fn maximisable(pane: Pane) -> bool {
    pane.view_key().is_none() && !matches!(pane, Pane::Uv(_) | Pane::Missing(_))
}

impl Operandi {
    /// The docked panel (not a view) under `pos` in the main window.
    pub(crate) fn dock_panel_at(&self, pos: Pos2) -> Option<Pane> {
        if !self.layouts.mixed_dock {
            // The side columns outside the dock tree (`side_panel_rects`).
            return self
                .side_panel_rects
                .iter()
                .find(|(_, r)| r.contains(pos))
                .map(|(&p, _)| p);
        }
        let tree = &self.views.main;
        tree.tiles
            .iter()
            .filter_map(|(&id, tile)| match tile {
                Tile::Pane(pane) if maximisable(*pane) && tree.tiles.is_visible(id) => {
                    let rect = tree.tiles.rect(id)?;
                    rect.contains(pos).then_some((*pane, rect.area()))
                }
                _ => None,
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(pane, _)| pane)
    }

    /// MN-08.1: maximise `pane` in its group, or put the group back when it
    /// already is. Layout, not scene: never an undo step.
    pub(crate) fn toggle_panel_maximized(&mut self, pane: Pane) {
        let title = self.views.title_of(pane);
        if self.panel_maximized == Some(pane) {
            self.panel_maximized = None;
            self.status = format!("{title}: panel layout restored");
            return;
        }
        let shares = if self.layouts.mixed_dock {
            !siblings(&self.views.main, pane).is_empty()
        } else {
            // Side columns: Items with Properties, the tool drawer with
            // Tool Properties.
            let partner = match pane {
                Pane::Items => Pane::Properties,
                Pane::Properties => Pane::Items,
                Pane::Toolbox => Pane::ToolProperties,
                _ => Pane::Toolbox,
            };
            self.side_panel_rects.contains_key(&partner)
        };
        if !shares {
            self.status = format!("{title} is alone in its group; there is nothing to maximise");
            return;
        }
        self.panel_maximized = Some(pane);
        self.status = format!("{title} maximised");
    }
}
