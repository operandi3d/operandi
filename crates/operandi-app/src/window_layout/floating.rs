//! Side panels in their own windows (workspace layout, step 2;
//! docs/WORKSPACE_LAYOUT.md). The Toolbox, the Items list and Properties can
//! each float in an OS window outside the main one; closing that window, or
//! Window > Dock, puts the panel back in its side panel. Which panels float,
//! and their window sizes, are saved with the app's data and restored.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Panel {
    Toolbox,
    Items,
    Properties,
}

impl Panel {
    pub const ALL: [Self; 3] = [Self::Toolbox, Self::Items, Self::Properties];
    pub(super) fn pane(self) -> operandi_layout::Pane {
        match self {
            Self::Toolbox => operandi_layout::Pane::Toolbox,
            Self::Items => operandi_layout::Pane::Items,
            Self::Properties => operandi_layout::Pane::Properties,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Toolbox => "Toolbox",
            Self::Items => "Items",
            Self::Properties => "Properties",
        }
    }
}

/// The two kinds of dockable floating window. The original side palettes
/// still use their palette files (LAY-50/51); all other panels use the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum DockWindow {
    Sidebar(Panel),
    Tree(u32),
}

pub(super) struct WindowDrag {
    pub source: DockWindow,
    pub pointer: Pos2,
}

#[cfg(test)]
pub(super) fn dock_title_rect(ctx: &Context, source: DockWindow) -> Rect {
    ctx.data(|data| data.get_temp(Id::new(("dock-title-test", source))))
        .unwrap()
}

/// The size and place an OS window opens with. egui turns any change in a
/// viewport's builder into a resize or move command, so feeding a window the
/// size it reported last frame sends that stale size back to the OS on every
/// frame of a resize. Dragging the top edge then fights the command (the
/// window jitters), so a window keeps the size and place it opened with
/// until it closes; the live size is still read back and saved. A window
/// that isn't open yet (not in the integration's viewport list) takes the
/// current values.
pub(crate) fn opening_place(
    ctx: &Context,
    id: egui::ViewportId,
    size: [f32; 2],
    position: Option<[f32; 2]>,
) -> ([f32; 2], Option<[f32; 2]>) {
    let key = Id::new(("operandi-window-opening-place", id));
    let open = ctx.input(|i| i.raw.viewports.contains_key(&id));
    ctx.data_mut(|d| {
        if !open {
            d.insert_temp(key, (size, position));
        }
        *d.get_temp_mut_or_insert_with(key, || (size, position))
    })
}

/// How a floating OS window opens (`Operandi::os_window`).
pub(crate) struct OsWindow {
    pub id: egui::ViewportId,
    pub title: String,
    /// Inner size in points when it opens.
    pub size: [f32; 2],
    /// Its top-left on screen (points) when it opens; `None` leaves it to
    /// the system.
    pub position: Option<[f32; 2]>,
    pub min_size: [f32; 2],
}

/// What a floating window did this frame.
pub(crate) struct OsWindowShown {
    /// `false` once its close button was pressed.
    pub open: bool,
    /// Its inner size now, once the system reported it.
    pub size: Option<[f32; 2]>,
    /// Its top-left on screen now (the same frame `OsWindow::position`
    /// opens it at), once the system reported it.
    pub position: Option<[f32; 2]>,
}

impl Operandi {
    /// A side panel floats while its palette is open (layouts stage 6:
    /// the old floating panels are the toolbox, items and properties
    /// palettes, palettes.rs).
    pub(super) fn is_floating(&self, panel: Panel) -> bool {
        self.palettes
            .side_hash(panel)
            .is_some_and(|h| self.palettes.open.contains(h))
    }

    /// Float a panel in its own window, or dock it back: its palette opens
    /// or closes (LAY-50).
    pub(super) fn set_floating(&mut self, panel: Panel, float: bool) {
        if let Some(hash) = self.palettes.side_hash(panel).map(str::to_owned)
            && let Err(e) = self.set_palette_open(&hash, float)
        {
            self.status = e;
        }
    }

    /// Draw each floating panel in its own OS window.
    pub(super) fn floating_windows(&mut self, ctx: &Context) {
        let mut dock = None;
        if self.palettes.hidden {
            return;
        }
        let panels: Vec<(Panel, String)> = Panel::ALL
            .into_iter()
            .filter(|&p| self.is_floating(p) && self.views.window_of(p.pane()).is_none())
            .filter_map(|p| Some((p, self.palettes.side_hash(p)?.to_owned())))
            .collect();
        for (panel, hash) in panels {
            let place = self.palettes.place(&hash);
            let spec = OsWindow {
                id: egui::ViewportId::from_hash_of(("operandi-panel-window", panel)),
                title: format!("{} — {}", panel.label(), self.environment.title()),
                size: place.size,
                // Dragged out by its corner dot: it opens at the release
                // point; else where it was last (UIC-73).
                position: self
                    .dots
                    .side_positions
                    .get(&panel)
                    .copied()
                    .or(place.position),
                min_size: [200., 160.],
            };
            let shown = self.os_window(ctx, spec, |app, ctx| {
                CentralPanel::default()
                    .frame(
                        Frame::side_top_panel(&ctx.style())
                            .fill(if panel == Panel::Toolbox {
                                theme::chrome::background()
                            } else {
                                ctx.style().visuals.panel_fill
                            })
                            .inner_margin(4.),
                    )
                    .show(ctx, |ui| {
                        app.dock_window_title(ui, DockWindow::Sidebar(panel), panel.label());
                        let area = ui.available_rect_before_wrap();
                        match panel {
                            Panel::Toolbox => app.toolbox(ui),
                            Panel::Items => app.items_section(ui, area),
                            Panel::Properties => app.properties_section(ui, area),
                        }
                    });
            });
            let mut place = place;
            if let Some(new_size) = shown.size
                && new_size.iter().all(|v| *v >= 100.)
            {
                place.size = new_size;
            }
            if let Some(at) = shown.position {
                place.position = Some(at);
            }
            self.palettes.places.insert(hash, place);
            if !shown.open {
                dock = Some(panel);
            }
        }
        if let Some(panel) = dock {
            self.set_floating(panel, false);
        }
    }

    /// D-06 / UIC-72: use a client title strip so the same drag can
    /// highlight Operandi's docking targets across native viewports.
    pub(super) fn dock_window_title(&mut self, ui: &mut Ui, source: DockWindow, title: &str) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.), Sense::hover());
        let response = ui.interact(
            rect,
            ui.id().with(("dock-window-title", source)),
            Sense::drag(),
        );
        ui.painter()
            .rect_filled(rect, 0., theme::workspace::tab_strip());
        ui.painter().with_clip_rect(rect).text(
            rect.left_center() + vec2(8., 0.),
            Align2::LEFT_CENTER,
            title,
            crate::typography::standard_font(),
            theme::text(),
        );
        #[cfg(test)]
        ui.ctx()
            .data_mut(|data| data.insert_temp(Id::new(("dock-title-test", source)), rect));
        self.track_dock_title(&response, source);
        response.on_hover_text("Drag to dock this window");
    }

    /// The one way Operandi opens a floating OS window: palettes, floating
    /// side panels and torn-off views all come through here (Jason,
    /// 2026-11-10: floating panels float as their own OS window; VST-29:
    /// one kind of floating window, not two). The window opens at `size`
    /// and `position` and then keeps where the user puts it
    /// (`opening_place`); a field's Tab moves between fields (UIC-51); app
    /// shortcuts work while it has the keyboard (UIC-70, UIC-71: it behaves
    /// like the main window). `add` draws its contents. Where the platform
    /// can't open more windows (tests, some Linux setups) egui draws it
    /// inside the main one.
    pub(super) fn os_window(
        &mut self,
        ctx: &Context,
        spec: OsWindow,
        add: impl FnOnce(&mut Self, &Context),
    ) -> OsWindowShown {
        let (open_size, open_at) = opening_place(ctx, spec.id, spec.size, spec.position);
        let mut builder = egui::ViewportBuilder::default()
            .with_title(spec.title)
            .with_inner_size(open_size)
            .with_min_inner_size(spec.min_size)
            // UIC-189: above the main window, not above other apps
            // (floating_level.rs). Torn-off views too, so one can't get
            // lost behind it on a single screen (VST-Q05).
            .with_window_level(self.floating_window_level());
        if let Some(at) = open_at {
            builder = builder.with_position(at);
        }
        let mut shown = OsWindowShown {
            open: true,
            size: None,
            position: None,
        };
        let mut add = Some(add);
        ctx.show_viewport_immediate(spec.id, builder, |ctx, _class| {
            crate::numeric_tab::take_tab_in_window(ctx);
            if let Some(add) = add.take() {
                add(self, ctx);
            }
            shown.size = ctx
                .input(|i| i.viewport().inner_rect)
                .map(|r| [r.width(), r.height()]);
            // Drawn inside the main window (no native windows), it has no
            // place of its own.
            if !ctx.embed_viewports() {
                shown.position = ctx
                    .input(|i| i.viewport().outer_rect)
                    .map(|r| [r.left(), r.top()]);
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                shown.open = false;
            }
            if ctx.input(|i| i.viewport().focused == Some(true)) {
                self.keyboard(ctx);
            }
        });
        // What the window did shows in the main window too.
        ctx.request_repaint_of(egui::ViewportId::ROOT);
        shown
    }

    /// A small palette (Snaps, Absolute) in an OS window of its own, so it
    /// can sit outside the main window (Jason, 2026-11-10). It opens near
    /// the main window's top left at `size`, stays above the main window
    /// (UIC-189),
    /// and returns `false` once its close button is pressed. Used by every
    /// floating palette (Jason, 2026-11-10: "most floating panels should be
    /// the same moving forward").
    pub(super) fn palette_window<R>(
        &mut self,
        ctx: &Context,
        key: &str,
        title: &str,
        size: [f32; 2],
        at: Option<Pos2>,
        add: impl FnOnce(&mut Self, &mut Ui) -> R,
    ) -> (bool, Option<R>) {
        // Toggle Palettes hid it (LAY-53): it stays open, unseen.
        if self.palettes.hidden {
            return (true, None);
        }
        // `at` is a point in the calling window (Snapping Options opens under
        // the pointer); else near that window's top left.
        let near = ctx.input(|i| i.viewport().inner_rect).map(|r| match at {
            Some(p) if p.is_finite() => [r.left() + p.x, r.top() + p.y],
            _ => [r.left() + 60., r.top() + 120.],
        });
        let spec = OsWindow {
            id: egui::ViewportId::from_hash_of(("operandi-palette-window", key)),
            title: title.to_owned(),
            size,
            position: near,
            min_size: [160., 120.],
        };
        let mut out = None;
        let shown = self.os_window(ctx, spec, |app, ctx| {
            CentralPanel::default()
                .frame(
                    Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(6.),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        out = Some(add(app, ui));
                    });
                });
        });
        (shown.open, out)
    }

    /// Right-click menu for a docked panel group's header or one of its
    /// tabs: float the group; on a tab of a group with several, Close Tab
    /// (UIC-156). Returns the tab to close.
    pub(super) fn panel_header_menu(
        &mut self,
        response: &Response,
        panel: Panel,
        close: Option<crate::panel_registry::PanelKind>,
    ) -> Option<crate::panel_registry::PanelKind> {
        let mut closed = None;
        response.context_menu(|ui| {
            self.panel_dock_menu(ui, crate::panel_dot::Dot::Side(panel));
            if let Some(kind) = close
                && ui.button("Close Tab").clicked()
            {
                closed = Some(kind);
                ui.close();
            }
        });
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw};

    const SIZE: Vec2 = vec2(1440., 900.);

    /// Regression: resizing a floating window from its top edge jittered,
    /// because each frame fed the last-reported size back to the OS. While
    /// a window is open its builder keeps the size and place it opened with;
    /// once it closes, the next opening takes the saved values.
    #[test]
    fn open_windows_keep_their_opening_size_and_place() {
        let ctx = Context::default();
        let id = egui::ViewportId::from_hash_of("opening-place-test");
        let frame = |open: bool, size: [f32; 2], at: Option<[f32; 2]>| {
            let mut raw = egui::RawInput::default();
            if open {
                raw.viewports.insert(id, Default::default());
            }
            let mut got = None;
            let _ = ctx.run(raw, |ctx| got = Some(opening_place(ctx, id, size, at)));
            got.unwrap()
        };
        // Opening: the saved size and place.
        let first = ([320., 520.], Some([40., 60.]));
        assert_eq!(frame(false, first.0, first.1), first);
        // Open and being resized from the top: height and top change every
        // frame, but the builder isn't told, so egui sends no commands.
        assert_eq!(frame(true, [320., 560.], Some([40., 20.])), first);
        assert_eq!(frame(true, [320., 600.], None), first);
        // Closed and reopened: the latest saved values.
        let reopened = ([320., 600.], Some([40., -20.]));
        assert_eq!(frame(false, reopened.0, reopened.1), reopened);
    }

    #[test]
    fn floating_panels_leave_the_side_panels_and_come_back() {
        let mut a = box_app();
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let docked = a.viewport_rect;
        // The Toolbox floats: the view widens to the left.
        a.set_floating(Panel::Toolbox, true);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert!(
            a.viewport_rect.left() < docked.left() - 100.,
            "{:?}",
            a.viewport_rect
        );
        // Both right-hand panels float: the view widens to the right too, and
        // the command line stays in the main window.
        a.set_floating(Panel::Items, true);
        a.set_floating(Panel::Properties, true);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert!(a.viewport_rect.right() > docked.right() + 100.);
        // Only Items docked: it takes the whole right-hand panel.
        a.set_floating(Panel::Items, false);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert!(a.scene_rect.height() > 400., "{:?}", a.scene_rect);
        // Saved and loaded: the palettes reopen.
        a.write_palettes_state();
        a.palettes.open.clear();
        a.load_palettes();
        assert!(a.is_floating(Panel::Toolbox) && a.is_floating(Panel::Properties));
        assert!(!a.is_floating(Panel::Items));
        for panel in Panel::ALL {
            a.set_floating(panel, false);
        }
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert_eq!(a.viewport_rect, docked);
    }
}
