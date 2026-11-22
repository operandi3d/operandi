//! Closing a registry panel (UIC-156, UIC-157): the × and the Close Panel
//! entry on a docked registry panel's header, the Node View's toggle and
//! its floating window's close button all come through here (Jason,
//! 2026-11-21: "once the node editor opens there doesn't seem to be a way
//! to close it").
//!
//! - A panel docked on its own (a dock tab in the layout's tree, in the
//!   main window or a floating one) leaves the layout; Window ▸ Panels
//!   brings it back where panels first open.
//! - A tab in a sidebar group leaves the group, unless it is the group's
//!   only tab (the group would be left empty, as UIC-156's Close Tab).
//! - A maximised panel's group comes back first (panel_maximize.rs).
//! - Closing is layout, not scene: never an undo step. What the panel
//!   shows stays with the app (the Node View's material, pan and zoom), so
//!   reopening shows the same thing.
//!
//! Panels ask while they draw, when the dock tree is out of `views`, so a
//! close is queued (`DotState::close_panels`) and done once the frame's
//! windows have drawn.
use super::Operandi;
use crate::floating::Panel;
use crate::panel_registry::{self, PanelKind};
use eframe::egui::{self, Align2, Key, Modifiers, Rect, Sense, Ui, pos2, vec2};
use operandi_layout::Pane;

/// The × button's square, at the right end of the panel's header.
const CLOSE_SIZE: f32 = 18.;

impl Operandi {
    /// Close `kind` after this frame's panels drew.
    pub(crate) fn request_close_panel(&mut self, kind: PanelKind) {
        if !self.dots.close_panels.contains(&kind) {
            self.dots.close_panels.push(kind);
        }
    }

    /// The closes panels asked for while they drew.
    pub(crate) fn close_panel_requests(&mut self) {
        for kind in std::mem::take(&mut self.dots.close_panels) {
            if let Err(e) = self.close_registry_panel(kind) {
                self.status = e;
            }
        }
    }

    /// Close `kind` now (the dock tree must be in `views`, as it is between
    /// frames). Nothing to close is not an error.
    pub(crate) fn close_registry_panel(&mut self, kind: PanelKind) -> Result<(), String> {
        let name = panel_registry::panel_type(kind).name;
        if let Some(pane) = self.registry_dock_pane(kind) {
            if self.panel_maximized == Some(pane) {
                self.panel_maximized = None;
            }
            let mut next = self.views.clone();
            if next.dock_slot(pane).is_some()
                || next.main.tiles.find_pane(&pane).is_some()
                || next.window_of(pane).is_some()
            {
                next.remove(pane)?;
            }
            if let Pane::DockTab(key) = pane {
                next.dock_tabs.remove(&key);
                if self.dots.active_side == Some(crate::panel_dot::Dot::Tab(key)) {
                    self.dots.active_side = None;
                }
            }
            self.install_dock_tree(next);
            self.status = format!("{name} closed");
            return Ok(());
        }
        for row in [Panel::Items, Panel::Properties] {
            let group = self.sidebar_group_mut(row);
            if !group.contains(kind) {
                continue;
            }
            if group.tabs.len() < 2 {
                return Err(format!("{name} is its group's only tab"));
            }
            group.remove(kind);
            if self.panel_maximized == Some(row.pane()) {
                self.panel_maximized = None;
            }
            self.status = format!("{name} closed");
            return Ok(());
        }
        Ok(())
    }

    /// `kind` is open and can be seen: its dock tab is the one in front, or
    /// it is the front tab of a sidebar group that shows.
    pub(crate) fn registry_panel_in_front(&self, kind: PanelKind) -> bool {
        self.dock_tab_visible(kind)
            || [Panel::Items, Panel::Properties].into_iter().any(|row| {
                self.sidebar_group(row).front() == Some(kind) && self.dock_group_visible(row)
            })
    }

    /// A floating window's close button was pressed: the Node View in it
    /// closes with it (the window was its place). Returns whether the
    /// window is gone, so nothing else in it needs a home.
    pub(crate) fn close_panels_with_window(&mut self, window: u32) -> bool {
        let Some(pane) = self.registry_dock_pane(PanelKind::NodeView) else {
            return false;
        };
        if self.views.window_of(pane) != Some(window) {
            return false;
        }
        if let Err(e) = self.close_registry_panel(PanelKind::NodeView) {
            self.status = e;
        }
        !self.views.windows.iter().any(|w| w.id == window)
    }

    /// A docked registry panel's header controls (`header` is its name
    /// strip, `area` the whole panel): the × and the right-click Close
    /// Panel (UIC-156; the × is Jason's, 2026-11-21, so a big editor like
    /// the Node View has a close you can see), and Esc over the panel
    /// while it is maximised puts its group back (UIC-70: keys act on the
    /// panel under the pointer).
    pub(crate) fn registry_tab_close(
        &mut self,
        ui: &mut Ui,
        kind: PanelKind,
        pane: Pane,
        header: Rect,
        area: Rect,
    ) {
        if matches!(kind, PanelKind::Missing(_)) {
            return;
        }
        let name = panel_registry::panel_type(kind).name;
        let id = egui::Id::new(("registry-tab-close", pane));
        let strip = ui.interact(header, id.with("header"), Sense::click());
        strip.context_menu(|ui| {
            self.menu_owned_input = true;
            if ui.button("Close Panel").clicked() {
                self.request_close_panel(kind);
                ui.close();
            }
        });
        let x = Rect::from_center_size(
            pos2(header.right() - CLOSE_SIZE / 2. - 2., header.center().y),
            vec2(CLOSE_SIZE, CLOSE_SIZE),
        );
        let key = if kind == PanelKind::NodeView {
            self.shortcut("Node View")
        } else {
            String::new()
        };
        let button = ui
            .interact(x, id.with("x"), Sense::click())
            .on_hover_text(format!("Close {name}{key}"));
        if button.hovered() {
            ui.painter()
                .rect_filled(x, 3., crate::theme::workspace::tab());
        }
        ui.painter().text(
            x.center(),
            Align2::CENTER_CENTER,
            "×",
            crate::typography::standard_font(),
            if button.hovered() {
                crate::theme::text()
            } else {
                crate::theme::disabled_text()
            },
        );
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(("panel-close-test", pane)), x));
        if button.clicked() {
            self.request_close_panel(kind);
        }
        let over = ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| area.contains(p));
        if self.panel_maximized == Some(pane)
            && over
            && ui.ctx().memory(|m| m.focused().is_none())
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.toggle_panel_maximized(pane);
        }
    }
}
