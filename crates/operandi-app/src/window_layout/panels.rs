use super::*;
use crate::unavailable::Gate;

/// The kinds of Tool Properties panel, each a declared field list
/// (`tool_form::ToolFields`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolPanel {
    Utility,
    /// The active sculpt brush (sculpt_ui.rs).
    Sculpt,
    Draw,
    Pen,
    SolidSketch,
    Reduction,
    Bridge,
    Spike,
    Ellipsoid,
    Shape,
    Cone,
    Surface,
    Primitive,
    Live,
    EdgeInsert,
    None,
}

impl Operandi {
    pub(super) fn number(
        &mut self,
        ui: &mut Ui,
        key: &str,
        label: &str,
        value: &mut f64,
        unit: &str,
        (min, max): (f64, f64),
    ) -> bool {
        let initial = workspace::number_text(*value, unit);
        self.confirmed_fields
            .entry(key.into())
            .or_insert_with(|| initial.clone());
        let scrubbing = self.scrub.as_ref().is_some_and(|s| s.key == key);
        // UIC-56 (MN-06b): an X/Y/Z group keyed `{base}0..2` has one link
        // button, on its first row.
        let link = self.drawing_link.take().and_then(|axis| {
            key.strip_suffix(["0", "1", "2"][axis.min(2)])
                .map(|base| (base.to_owned(), axis))
        });
        let link_mode = link
            .as_ref()
            .map(|(base, _)| self.link_modes.get(base).copied().unwrap_or_default());
        let mut link_clicked = None;
        let text = self.fields.entry(key.into()).or_insert(initial);
        let mut handle = None;
        let response = option_row::row(ui, label, |ui| {
            // One cell the width a plain field had: the text field, then
            // the scrub handle inside its right end (UIC-53/113). Sized like the plain field
            // so rows keep their height.
            let size = [ui.available_width().max(52.), option_row::ROW_HEIGHT];
            let response = ui.add_sized(size, |ui: &mut Ui| {
                let (handle_rect, field_rect) = scrub::split(ui.max_rect());
                handle = Some(scrub::handle(ui, handle_rect, key, scrubbing));
                // Every row of a group gives up the button's width so the
                // three fields line up; only the first row draws it.
                let field_rect = match (&link, link_mode) {
                    (Some((_, axis)), Some(mode)) => {
                        let (link_rect, rest) = crate::link_mode::LinkMode::split(field_rect);
                        if *axis == 0 {
                            link_clicked = Some(crate::link_mode::button(ui, link_rect, mode));
                        }
                        rest
                    }
                    _ => field_rect,
                };
                ui.put(
                    field_rect,
                    // UIC-90: the value reads from the left, unit after it.
                    TextEdit::singleline(text)
                        .horizontal_align(Align::LEFT)
                        .id_salt(key),
                )
            });
            // Min/max labels (an approved change to UIC-90); the row grows.
            if let Some(limits) = crate::form::range_labels::shown(min, max) {
                let cell = response.rect;
                crate::form::range_labels::paint(ui, cell, Some(*value), limits, |v| {
                    workspace::number_text(v, unit)
                });
                crate::form::range_labels::reserve(ui, cell);
            }
            response
        })
        .inner;
        let handle = handle.expect("drawn with its field");
        if let (Some((base, _)), Some(button), Some(mode)) = (&link, link_clicked, link_mode) {
            self.field_rects.insert(format!("{base}-link"), button.rect);
            if button.clicked() {
                self.link_modes.insert(base.clone(), mode.next());
            }
        }
        self.field_rects.insert(key.into(), response.rect);
        self.text_editors.insert(response.id);
        // Tab and Shift+Tab move between these fields (UIC-51).
        if let Some(text) = self.fields.get(key) {
            crate::numeric_tab::register(ui, &response, Some(text));
        }
        if response.has_focus() {
            self.field_focus = Some(response.id);
            self.field_focus_words = false;
        }
        if self.modal_owned_input || self.has_modal() {
            return false;
        }
        if self.menu_owned_input || egui::Popup::is_any_open(ui.ctx()) {
            // Blur may confirm a value, but the popup owns Enter/Escape and focus.
            if response.lost_focus() {
                self.confirm_fields();
            }
            return false;
        }
        if self.scrub_field(ui, &handle, key, label, *value, unit) {
            return false;
        }
        if self.field_focus == Some(response.id) && self.field_errors.contains_key(key) {
            response.request_focus();
        }
        let editing = response.has_focus() || response.lost_focus();
        if editing && ui.input(|i| i.key_pressed(Key::Escape)) {
            if let Some(text) = self.confirmed_fields.get(key) {
                self.fields.insert(key.into(), text.clone());
            }
            // Escape is an explicit correction: revalidate the restored values
            // so a rejected geometry preview cannot leave a stale stage error.
            let restored = self.confirm_fields();
            response.surrender_focus();
            if restored {
                self.status = format!("{label}: restored confirmed value");
            }
            return false;
        }
        let enter = editing && ui.input(|i| i.key_pressed(Key::Enter));
        // UIC-57: how Enter is pressed picks the link for this one entry;
        // the other two rows take their share before the form confirms.
        if enter && let (Some((base, axis)), Some(mode)) = (&link, link_mode) {
            let mode = crate::link_mode::LinkMode::for_enter(ui.input(|i| i.modifiers), mode);
            self.spread_link(key, base, *axis, unit, mode);
        }
        // A blur caused by clicking a control in this same frame keeps that
        // control's feedback once the draft confirms; a failed confirmation
        // still reports its field error, and later frames update normally.
        let action_status = (!enter && response.lost_focus())
            .then(|| self.same_frame_action_status())
            .flatten();
        if (enter || response.lost_focus()) && !self.confirm_fields() {
            response.request_focus();
        } else if enter {
            response.surrender_focus();
        } else if let Some(status) = action_status {
            self.status = status;
        }
        if let Some(error) = self.field_errors.get(key) {
            ui.painter().rect_stroke(
                response.rect,
                1.,
                Stroke::new(1.0f32, theme::error()),
                StrokeKind::Inside,
            );
            response.on_hover_text(error);
        }
        false
    }
    /// Write the other two rows of `base`'s X/Y/Z group into the draft as
    /// `mode` spreads the typed value of row `axis` (UIC-56/57). Rows whose
    /// confirmed text doesn't read as a number are left alone, and so is
    /// everything when the typed text doesn't; the form's own checks then
    /// confirm or refuse the lot. Whole-number groups round.
    fn spread_link(
        &mut self,
        key: &str,
        base: &str,
        axis: usize,
        unit: &str,
        mode: crate::link_mode::LinkMode,
    ) {
        if mode == crate::link_mode::LinkMode::Independent {
            return;
        }
        let tool = self.editor.session.live.as_ref().map(|l| l.tool);
        let kind = scrub::Kind::for_field(key, unit, tool);
        let parse = |t: &String| numeric::parse_quantity(t, kind.quantity()).ok();
        let Some(typed) = self.fields.get(key).and_then(parse) else {
            return;
        };
        let mut old = [0.; 3];
        for (i, slot) in old.iter_mut().enumerate() {
            let row = format!("{base}{i}");
            match self.confirmed_fields.get(&row).and_then(parse) {
                Some(v) => *slot = v,
                None => return,
            }
        }
        let new = mode.apply(old, axis, typed);
        for (i, v) in new.into_iter().enumerate() {
            if i == axis {
                continue;
            }
            let v = if kind == scrub::Kind::Count {
                v.round()
            } else {
                v
            };
            self.fields
                .insert(format!("{base}{i}"), workspace::number_text(v, unit));
        }
    }
    /// Drive a drag on `key`'s scrub handle: write each new value into the
    /// field and confirm it like typed text. Returns true while this field
    /// is being scrubbed (including the release frame).
    fn scrub_field(
        &mut self,
        ui: &mut Ui,
        handle: &Response,
        key: &str,
        label: &str,
        value: f64,
        unit: &str,
    ) -> bool {
        let tool = self.editor.session.live.as_ref().map(|l| l.tool);
        // UIC-53: a click on a whole-number field's arrows steps it by one,
        // as its own undo step.
        let kind = scrub::Kind::for_field(key, unit, tool);
        if let Some(step) = scrub::click_step(handle, kind, ui.input(|i| i.modifiers)) {
            let current = self
                .confirmed_fields
                .get(key)
                .and_then(|t| numeric::parse_quantity(t, kind.quantity()).ok())
                .unwrap_or(value);
            self.fields
                .insert(key.into(), workspace::number_text(current + step, unit));
            if !self.confirm_fields() {
                // Refused (below a minimum): put the shown value back.
                if let Some(text) = self.confirmed_fields.get(key).cloned() {
                    self.fields.insert(key.into(), text);
                }
                self.field_errors.remove(key);
            }
            self.editor.end_tweak();
            return true;
        }
        if handle.drag_started() {
            let start_text = self
                .confirmed_fields
                .get(key)
                .cloned()
                .unwrap_or_else(|| workspace::number_text(value, unit));
            let kind = scrub::Kind::for_field(key, unit, tool);
            self.scrub = Some(scrub::Scrub::new(key, kind, value, start_text));
            // The whole scrub is one undo step, like a viewport drag
            // (`hold_pointer`); `drag_stopped` below ends it.
            self.editor.session.tweak_held = true;
        }
        let Some(scrub) = self.scrub.as_mut().filter(|s| s.key == key) else {
            return false;
        };
        // egui ends a drag itself when Escape is pressed, so look for the
        // key on the release frame too, and keep it from reaching the
        // tool (where it could drop or cancel it).
        if (handle.dragged() || handle.drag_stopped())
            && !scrub.cancelled
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            scrub.cancelled = true;
            let text = scrub.start_text.clone();
            self.fields.insert(key.into(), text);
            if self.confirm_fields() {
                self.status = format!("{label}: restored");
            }
        }
        let scrub = self.scrub.as_mut().unwrap();
        if handle.dragged() && !scrub.cancelled {
            ui.ctx().request_repaint();
            let dx = handle.drag_delta().x;
            let modifiers = ui.input(|i| i.modifiers);
            let kind = scrub.kind;
            let change = scrub.advance(dx, modifiers);
            if dx != 0. && change != scrub.accepted {
                let candidate = scrub.start + change;
                self.fields
                    .insert(key.into(), workspace::number_text(candidate, unit));
                if self.confirm_fields() {
                    // A tool may clamp what it takes (Bevel Segments at 1);
                    // follow the value it shows so dragging back responds
                    // at once.
                    let shown = self
                        .confirmed_fields
                        .get(key)
                        .and_then(|t| numeric::parse_quantity(t, kind.quantity()).ok());
                    if let Some(scrub) = self.scrub.as_mut() {
                        scrub.accept(change, shown.map_or(change, |v| v - scrub.start));
                    }
                } else {
                    // Stop at the last value the tool took; its message
                    // stays in the status bar.
                    let back = self.confirmed_fields.get(key).cloned().unwrap_or_else(|| {
                        let s = self.scrub.as_ref().unwrap();
                        workspace::number_text(s.start + s.accepted, unit)
                    });
                    self.fields.insert(key.into(), back);
                    self.field_errors.remove(key);
                    if let Some(scrub) = self.scrub.as_mut() {
                        scrub.refuse();
                    }
                }
            }
        }
        if handle.drag_stopped() {
            let scrub = self.scrub.take().unwrap();
            self.editor.end_tweak();
            // Move/Rotate/Scale/Transform record each drag as its own undo
            // step, as their viewport handles do; other tools' sessions are
            // already one step.
            if !scrub.cancelled && scrub.accepted != 0. && tool.is_some_and(|t| t.is_transform()) {
                self.run(Command::CommitTransformStep);
                self.clear_fields();
            }
        }
        true
    }
    /// The Tool Properties header: the name of the tool whose properties
    /// show below it, or "Tool Properties" when no tool is on. Follows the
    /// same order as `parameters`, so the header names what the panel shows.
    pub(super) fn tool_properties_title(&self) -> &'static str {
        if let Some(utility) = &self.utility {
            return utility.label();
        }
        if let Some(brush) = self.sculpt.brush {
            return sculpt_ui::title(brush);
        }
        #[cfg(feature = "painting")]
        if let Some(tool) = self.painting.brush.tool {
            return tool.label();
        }
        if let Some(draw) = &self.draw {
            return draw.kind.label();
        }
        let e = &self.editor;
        if e.session.pen_draft.is_some() {
            return "Pen";
        }
        if e.session.solid_sketch.is_some() {
            return "Solid Sketch";
        }
        if e.session.reduction.is_some() {
            return "Polygon Reduction";
        }
        if e.session.bridge.is_some() {
            return "Bridge";
        }
        if e.session.spike.is_some() {
            return "Spike";
        }
        if e.session.ellipsoid.is_some() {
            return "Ellipsoid";
        }
        if let Some(shape) = &e.session.shape {
            return shape.requested.kind().label();
        }
        if e.session.cone.is_some() {
            return "Cone";
        }
        if let Some(live) = &e.session.live {
            return live.tool.label();
        }
        if let Some(tool) = self.edge_click {
            return tool.label();
        }
        "Tool Properties"
    }

    /// Which panel Tool Properties shows: one order for drawing, confirming
    /// and the header.
    pub(super) fn tool_panel_kind(&self) -> ToolPanel {
        let e = &self.editor;
        if self.utility.is_some() {
            ToolPanel::Utility
        } else if self.sculpt.brush.is_some() {
            ToolPanel::Sculpt
        } else if self.draw.is_some() {
            ToolPanel::Draw
        } else if e.session.pen_draft.is_some() {
            ToolPanel::Pen
        } else if e.session.solid_sketch.is_some() {
            ToolPanel::SolidSketch
        } else if e.session.reduction.is_some() {
            ToolPanel::Reduction
        } else if e.session.bridge.is_some() {
            ToolPanel::Bridge
        } else if e.session.spike.is_some() {
            ToolPanel::Spike
        } else if e.session.ellipsoid.is_some() {
            ToolPanel::Ellipsoid
        } else if e.session.shape.is_some() {
            ToolPanel::Shape
        } else if e.session.cone.is_some() {
            ToolPanel::Cone
        } else if let Some(live) = &e.session.live {
            if live.tool.is_smooth_shift() {
                ToolPanel::Surface
            } else if live.tool.is_primitive() {
                ToolPanel::Primitive
            } else {
                ToolPanel::Live
            }
        } else if self.edge_click.is_some() {
            ToolPanel::EdgeInsert
        } else {
            ToolPanel::None
        }
    }

    /// Tool Properties: the active tool's declared field list.
    pub(super) fn parameters(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = crate::form::layout::panel::ROW_GAP;
        self.tool_parameters(ui);
        // FO-10/16: one section per falloff layer under the tool's rows (or
        // alone with no tool), before the constraint and snapping rows.
        if self.editor.spatial_falloff.is_some() {
            self.tool_panel::<falloff_panel::FalloffFields>(ui);
        }
        // Constrain to Background is part of the tool setup: its rows sit
        // under the tool's while it is on.
        if self.editor.background_constraint.is_some() {
            ui.separator();
            self.tool_panel::<background_ui::BackgroundFields>(ui);
        }
    }
    pub(super) fn tool_parameters(&mut self, ui: &mut Ui) {
        #[cfg(feature = "painting")]
        if self.painting.brush.tool.is_some() || self.painting.brush.lasso_mode {
            self.paint_properties(ui);
            return;
        }
        match self.tool_panel_kind() {
            ToolPanel::Utility => self.tool_panel::<create_utility::UtilityFields>(ui),
            ToolPanel::Sculpt => {
                self.tool_panel::<sculpt_ui::SculptFields>(ui);
                // Painting builds: tips, nozzle, inks and the mask (PNT-151).
                #[cfg(feature = "painting")]
                self.sculpt_painting_properties(ui);
            }
            ToolPanel::Draw => self.tool_panel::<draw_tools::DrawFields>(ui),
            ToolPanel::Pen => self.tool_panel::<pen_interactive_ui::PenFields>(ui),
            ToolPanel::SolidSketch => self.tool_panel::<solid_sketch_ui::SolidSketchFields>(ui),
            ToolPanel::Reduction => self.tool_panel::<reduction_panel::ReductionFields>(ui),
            ToolPanel::Bridge => self.tool_panel::<bridge_panel::BridgeFields>(ui),
            ToolPanel::Spike => self.tool_panel::<spike_panel::SpikeFields>(ui),
            ToolPanel::Ellipsoid => self.tool_panel::<ellipsoid_panel::EllipsoidFields>(ui),
            ToolPanel::Shape => self.tool_panel::<shape_panel::ShapeFields>(ui),
            ToolPanel::Cone => self.tool_panel::<cone_panel::ConeFields>(ui),
            ToolPanel::Surface => self.tool_panel::<smooth_shift_panel::SurfaceFields>(ui),
            ToolPanel::Primitive => self.tool_panel::<primitive_panel::PrimitiveFields>(ui),
            ToolPanel::Live => self.tool_panel::<live_fields::LiveFields>(ui),
            ToolPanel::EdgeInsert => self.tool_panel::<edge_panel::EdgeInsertFields>(ui),
            ToolPanel::None => {
                ui.weak("Select a tool to edit its properties.");
                if ui
                    .button("Apply Last Cube")
                    .on_hover_text("Apply retained Cube properties once without arming a tool")
                    .clicked()
                {
                    self.run(Command::Apply);
                }
            }
        }
    }

    /// Confirms the pending field text of every form Tool Properties shows
    /// (Enter, leaving a field, a scrub, before a Drop): the active tool's,
    /// then the falloff sections and Constrain to Background rows drawn
    /// under it (`parameters`). False while a field is refused. GitHub #16:
    /// only the tool's rows used to be confirmed, so a typed or scrubbed
    /// falloff value (Noise Scale) never reached the falloff.
    pub(super) fn confirm_tool_fields(&mut self) -> bool {
        let tool = self.confirm_tool_rows();
        let falloff = self.editor.spatial_falloff.is_none()
            || self.confirm_form::<falloff_panel::FalloffFields>();
        let background = self.editor.background_constraint.is_none()
            || self.confirm_form::<background_ui::BackgroundFields>();
        tool && falloff && background
    }
    fn confirm_tool_rows(&mut self) -> bool {
        match self.tool_panel_kind() {
            ToolPanel::Utility => self.confirm_form::<create_utility::UtilityFields>(),
            ToolPanel::Sculpt => self.confirm_form::<sculpt_ui::SculptFields>(),
            ToolPanel::Draw => self.confirm_form::<draw_tools::DrawFields>(),
            ToolPanel::Pen => self.confirm_form::<pen_interactive_ui::PenFields>(),
            ToolPanel::SolidSketch => self.confirm_form::<solid_sketch_ui::SolidSketchFields>(),
            ToolPanel::Reduction => self.confirm_form::<reduction_panel::ReductionFields>(),
            ToolPanel::Bridge => self.confirm_form::<bridge_panel::BridgeFields>(),
            ToolPanel::Spike => self.confirm_form::<spike_panel::SpikeFields>(),
            ToolPanel::Ellipsoid => self.confirm_form::<ellipsoid_panel::EllipsoidFields>(),
            ToolPanel::Shape => self.confirm_form::<shape_panel::ShapeFields>(),
            ToolPanel::Cone => self.confirm_form::<cone_panel::ConeFields>(),
            ToolPanel::Surface => self.confirm_form::<smooth_shift_panel::SurfaceFields>(),
            ToolPanel::Primitive => self.confirm_form::<primitive_panel::PrimitiveFields>(),
            ToolPanel::Live => self.confirm_form::<live_fields::LiveFields>(),
            ToolPanel::EdgeInsert => self.confirm_form::<edge_panel::EdgeInsertFields>(),
            ToolPanel::None => true,
        }
    }

    pub(super) fn toolbar(&mut self, ctx: &Context) {
        // Jason, 2026-11-22: on Windows and Linux the menu bar (File…Help)
        // has its own row at the very top of the window, with the tool set
        // bar below it (UIC-11: under the menu bar sits the layout bar).
        // Shown first, so it sits above "top". It stays when the tool set
        // bar is collapsed (UIC-16): a Windows menu bar doesn't hide.
        if !self.menus_in_system_bar() {
            let panel = TopBottomPanel::top("menu-bar").show(ctx, |ui| {
                ui.horizontal(|ui| self.in_window_menu_bar(ui, ctx));
            });
            #[cfg(test)]
            {
                self.layouts.menu_bar_rect = Some(panel.response.rect);
            }
            #[cfg(not(test))]
            let _ = panel;
        }
        // Collapsed with nothing else in it, the bar is just the thin line,
        // as thin as the side columns' (Jason, 2026-11-19: the panel's own
        // margins left a wider empty band).
        let bare = self.layouts.top_collapsed && self.layouts.mode_bar_bottom;
        let panel = TopBottomPanel::top("top");
        let panel = if bare {
            panel
                .frame(Frame::NONE.fill(theme::chrome::background()))
                .exact_height(collapse_line::TOP_AREA)
        } else {
            panel
        };
        panel.show(ctx, |ui| {
            // UIC-16 (Jason 2026-11-18): clicking the logo collapses this
            // row to a thin line; a double-click on the line, or a drag
            // down, brings it back (collapse_line.rs).
            if self.layouts.top_collapsed {
                self.layouts.logo_rect = None;
                #[cfg(test)]
                self.layouts.tool_set_rects.clear();
                let (response, reopen) = collapse_line::show(ui, collapse_line::Bar::Top);
                self.layouts.strip_rect = Some(response.rect);
                if reopen.is_some() {
                    self.execute("operandi.bar.collapse bar:top on:false");
                }
            } else {
                self.layouts.strip_rect = None;
                self.top_bar_row(ui);
            }
            if !self.layouts.mode_bar_bottom {
                self.mode_bar(ui);
            }
        });
        // Jason, 2026-11-19: the mode bar docks at the top or the bottom
        // of the window only (it is too wide for a side); the shipped
        // Model layout puts it at the bottom, above the command line.
        // In a dock-tree layout the bottom bar shares the view area's last
        // row with the message line (views.rs); older layouts use a
        // full-width bottom row.
        if self.layouts.mode_bar_bottom && !self.layouts.mixed_dock {
            TopBottomPanel::bottom("mode-bar").show(ctx, |ui| self.mode_bar(ui));
        }
    }

    /// The mode bar: Selection modes, Action Center, Falloff, Symmetry,
    /// constraints, snapping, Work Plane, Drop Action, Render and Preview.
    /// Its grip moves it between the window's top and bottom.
    pub(super) fn mode_bar(&mut self, ui: &mut Ui) {
        let row = ui
            .horizontal(|ui| {
                use button_variants::ButtonId;
                // MB-01..15: each mode button swaps to its variants while
                // Shift, Ctrl or Alt is held (button_variants.rs).
                for button in [ButtonId::Vertices, ButtonId::Edges, ButtonId::Polygons] {
                    let response = self.component_button(ui, button);
                    self.variant_click(ui, button, &response);
                }
                // Note 04: Materials (polygon tag) mode and its tag type.
                ui.separator();
                self.tag_mode_controls(ui);
                ui.separator();
                self.context_bar(ui);
                ui.separator();
                // TK-70: the timeline's clock button.
                self.timeline_toggle_button(ui);
            })
            .response
            .rect;
        self.mode_bar_grip(ui, row);
    }

    /// UIC-71/72: the mode bar uses the hover grip and shortcut, with its
    /// existing top/bottom docking zones.
    fn mode_bar_grip(&mut self, ui: &Ui, row: Rect) {
        let id = Id::new("mode-bar-grip");
        #[cfg(test)]
        {
            self.layouts.mode_bar_rect = Some(row);
        }
        if !self.panel_drags_locked()
            && self.layouts.mode_bar_drag.is_none()
            && !self.panel_docking_in_progress()
            && let Some(press) = panel_dot::grip_start(ui, id, row, Some(panel_dot::grip_rect(row)))
        {
            ui.ctx().set_dragged_id(id);
            self.layouts.mode_bar_drag = Some(press.button);
        }
        // UI build mode: the bar's pad, like every panel's.
        if self.dots.build_mode {
            let dragging = self.layouts.mode_bar_drag.is_some();
            let busy = self.panel_docking_in_progress();
            let response = panel_dot::build_pad_area(ui, id, row, row, dragging, busy);
            if !dragging
                && !self.panel_docking_in_progress()
                && let Some(button) = [PointerButton::Primary, PointerButton::Middle]
                    .into_iter()
                    .find(|&b| response.drag_started_by(b))
            {
                self.layouts.mode_bar_drag = Some(button);
            }
            response.on_hover_text("Drag to dock this bar at the top or bottom");
        }
        panel_dot::grip_overlay(
            ui,
            id,
            row,
            panel_dot::grip_rect(row).center(),
            theme::weak_text(),
        );
        if let Some(button) = self.layouts.mode_bar_drag {
            let (at, down, released) = ui.input(|i| {
                (
                    i.pointer.latest_pos(),
                    i.focused && i.pointer.button_down(button),
                    i.pointer.button_released(button),
                )
            });
            if !down {
                self.layouts.mode_bar_drag = None;
                if released && let Some(at) = at {
                    self.dock_mode_bar(at.y > ui.ctx().content_rect().center().y);
                }
                return;
            }
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            ui.ctx().request_repaint();
            // UIC-72 dock guide: the top and bottom arrows only, with the
            // landing strip; release on either half of the window docks
            // the bar there.
            let screen = ui.ctx().content_rect();
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                Id::new("mode-bar-dock-zones"),
            ));
            let bottom = at.is_some_and(|p| p.y > screen.center().y);
            panel_dot::paint_mode_bar_guide(
                &painter,
                screen,
                screen.top() + 24.,
                row.height(),
                bottom,
            );
        }
        panel_dot::grip_menu(ui, id.with("menu"), row, |ui| {
            self.menu_owned_input = true;
            for (bottom, label) in [(false, "Dock at Top"), (true, "Dock at Bottom")] {
                let why = crate::unavailable::unless(
                    bottom != self.layouts.mode_bar_bottom,
                    "The mode bar is already docked here",
                );
                if ui.gated(why, Button::new(label)).clicked() {
                    self.dock_mode_bar(bottom);
                    ui.close();
                }
            }
        });
    }

    /// Dock the mode bar at the bottom (`true`) or top of the window; the
    /// current layout keeps it.
    pub(crate) fn dock_mode_bar(&mut self, bottom: bool) {
        if self.layouts.mode_bar_bottom != bottom {
            self.layouts.mode_bar_bottom = bottom;
            self.status = if bottom {
                "Mode bar docked at the bottom".into()
            } else {
                "Mode bar docked at the top".into()
            };
        }
    }
    /// D-08: the fixed tool set bar. The owner approved moving the set
    /// controls here (UIC-197); layout switches still run layout.restore.
    fn top_bar_row(&mut self, ui: &mut Ui) {
        #[cfg(test)]
        self.layouts.tool_set_rects.clear();
        ui.horizontal(|ui| {
            let logo = branding::identity(ui);
            self.layouts.logo_rect = Some(logo.rect);
            if logo.clicked() {
                self.execute("operandi.bar.collapse bar:top on:true");
            }
            ui.separator();
            for &set in toolbox::ToolSet::ALL {
                let line = format!("operandi.toolbox.set {}", set.label());
                let response = ui.add(Button::selectable(
                    self.commands
                        .state(self, &line)
                        .is_some_and(|state| state.on),
                    set.label(),
                ));
                #[cfg(test)]
                self.layouts.tool_set_rects.push((set, response.rect));
                if response.clicked() {
                    self.execute(&line);
                }
            }
            ui.separator();
            // Save, Undo and Redo stay on this row; the menus have their
            // own row above it (`toolbar`).
            if !self.menus_in_system_bar() {
                if ui.button("Save").clicked() {
                    self.save_current();
                }
                if self.history_button(ui, false).clicked() {
                    self.run(Command::Undo);
                }
                if self.history_button(ui, true).clicked() {
                    self.run(Command::Redo);
                }
                ui.separator();
            }
            // The scene's name and its unsaved mark are in the window title
            // (main.rs), so the bar doesn't repeat them (Jason, 2026-11-19).
        });
    }
    fn history_button(&self, ui: &mut Ui, redo: bool) -> Response {
        let action = if redo { "Redo" } else { "Undo" };
        let query = self.editor.history_eligibility(if redo {
            HistoryAction::Redo
        } else {
            HistoryAction::Undo
        });
        let available = query.is_available();
        let display = query
            .label
            .map(|label| format!("{action} {label}{}", self.shortcut(action)))
            .unwrap_or_else(|| format!("{action}{}", self.shortcut(action)));
        let disabled_reason = match &query.availability {
            HistoryAvailability::Refused(reason) => reason.to_string(),
            HistoryAvailability::EmptyHistory => "No history available".into(),
            HistoryAvailability::Available => String::new(),
        };
        ui.gated(
            crate::unavailable::unless(available, &disabled_reason),
            if available {
                Button::new(display)
            } else {
                Button::new(RichText::new(display).color(theme::disabled_text()))
                    .fill(theme::panel())
                    .stroke(Stroke::new(0.5f32, theme::border()))
            },
        )
    }
    pub(super) fn sidebars(&mut self, ctx: &Context) {
        if self.layouts.mixed_dock {
            if self.layouts.left_collapsed && !self.is_floating(floating::Panel::Toolbox) {
                self.collapsed_side(ctx, collapse_line::Bar::Left);
            }
            if self.layouts.right_collapsed {
                self.collapsed_side(ctx, collapse_line::Bar::Right);
            } else if [floating::Panel::Items, floating::Panel::Properties]
                .into_iter()
                .all(|panel| {
                    self.is_floating(panel)
                        || self.views.main.tiles.find_pane(&panel.pane()).is_none()
                })
            {
                self.command_history_strip(ctx);
            }
            return;
        }
        // A layout just opened sets the side columns' widths (layouts.rs).
        self.apply_layout_side(ctx);
        let width = ctx.content_rect().width();
        self.apply_side_reopen(ctx);
        if self.is_floating(floating::Panel::Toolbox) {
            self.right_sidebar(ctx);
            return;
        }
        if self.layouts.left_collapsed {
            self.collapsed_side(ctx, collapse_line::Bar::Left);
            self.right_sidebar(ctx);
            return;
        }
        SidePanel::left("tools")
            .resizable(false)
            .default_width(layout::LEFT_DEFAULT)
            .width_range(layout::LEFT_MIN..=layout::left_max(width))
            .frame(
                Frame::side_top_panel(&ctx.style())
                    .fill(theme::chrome::background())
                    .inner_margin(4.),
            )
            .show(ctx, |ui| {
                self.toolbox(ui);
            });
        self.right_sidebar(ctx);
    }
    /// A collapsed side bar: its line at the window edge (Jason
    /// 2026-11-18). A double-click reopens it at its old width; a drag out
    /// reopens it where the drag lets go, or at the old width when that is
    /// under the bar's minimum.
    pub(super) fn collapsed_side(&mut self, ctx: &Context, bar: collapse_line::Bar) {
        let panel = match bar {
            collapse_line::Bar::Left => SidePanel::left("tools-collapsed"),
            _ => SidePanel::right("scene-collapsed"),
        };
        panel
            .resizable(false)
            .exact_width(collapse_line::SIDE_AREA)
            .frame(Frame::NONE.fill(theme::chrome::background()))
            .show(ctx, |ui| {
                let (_, reopen) = collapse_line::show(ui, bar);
                let side = bar.name();
                match reopen {
                    Some(collapse_line::Reopen::OldSize) => {
                        self.execute(&format!("operandi.bar.collapse bar:{side} on:false"));
                    }
                    Some(collapse_line::Reopen::Width(w)) => {
                        self.execute(&format!(
                            "operandi.bar.collapse bar:{side} on:false width:{w:.0}"
                        ));
                    }
                    None => {}
                }
            });
    }
    /// The width a drag-out asked for, clamped to the bar's range; under
    /// its minimum, the old width stays.
    fn apply_side_reopen(&mut self, ctx: &Context) {
        let Some((bar, width)) = self.layouts.side_reopen.take() else {
            return;
        };
        let window = ctx.content_rect().width();
        let (id, min, max) = match bar {
            collapse_line::Bar::Left => ("tools", layout::LEFT_MIN, layout::left_max(window)),
            _ => (
                "scene",
                layout::RIGHT_MIN,
                layout::right_max(window, self.docked_left_width(ctx)),
            ),
        };
        if width < min {
            return;
        }
        let width = width.min(max);
        let mut rect = egui::containers::panel::PanelState::load(ctx, Id::new(id))
            .map_or(Rect::from_min_size(Pos2::ZERO, vec2(width, 600.)), |s| {
                s.rect
            });
        if bar == collapse_line::Bar::Left {
            rect.max.x = rect.min.x + width;
        } else {
            rect.min.x = rect.max.x - width;
        }
        ctx.data_mut(|d| {
            d.insert_persisted(Id::new(id), egui::containers::panel::PanelState { rect })
        });
    }
    pub(super) fn dialogs(&mut self, ctx: &Context) {
        if self.import_dialogs(ctx) {
            return;
        }
        self.layout_reset_dialog(ctx);
        self.layout_dialog(ctx);
        self.switcher_window(ctx);
        if self.has_modal() {
            self.modal_owned_input = true;
            self.editor_owned_input = true;
        }
        if let Some(warning) = self.recovery_warning.clone() {
            // An acknowledgement (UIC-97 frame, OK alone): there is nothing
            // to cancel, the journal can't be read.
            let message = format!("The recovery journal could not be read. {warning}");
            if self.show_notice(ctx, "Recover Scene", &message) != form::Outcome::Open {
                self.recovery_warning = None;
            }
            return;
        }
        if self.recovery.is_some() {
            let more = self.recovery_more.len();
            let message = if more == 0 {
                format!(
                    "The recovery journal holds unsaved work. Recover it? Journal: {}",
                    self.environment.paths().recovery().display()
                )
            } else {
                format!(
                    "Recovery journals hold {} unsaved scenes; OK opens each in its own tab. \
                     Journal: {}",
                    more + 1,
                    self.environment.paths().recovery().display()
                )
            };
            // UIC-97 frame. OK recovers; Discard deletes the journals.
            // Cancel (or Escape) keeps the journals on disk and starts a new
            // scene; they are offered again at the next launch.
            match self.show_confirm(ctx, "Recover Scene", &message, &["Discard"]) {
                form::Outcome::Ok => self.recover_journals(),
                form::Outcome::Extra(_) => self.discard_journals(),
                form::Outcome::Cancel => self.keep_journals(),
                form::Outcome::Open | form::Outcome::No => {}
            }
            return;
        }

        if self.scenes.confirm_close_all {
            self.close_all_confirmation(ctx);
            return;
        }
        if self.scenes.confirm_delete_children {
            self.delete_children_confirmation(ctx);
            return;
        }
        if self.scenes.confirm_last_mesh.is_some() {
            self.last_mesh_confirmation(ctx);
            return;
        }
        if self.shading.confirm_delete.is_some() {
            self.shading_delete_confirmation(ctx);
            return;
        }
        if self.file_dialog.is_none()
            && let Some(action) = self.pending
        {
            // UIC-97: OK saves first, Cancel keeps everything as it is,
            // Discard goes on without saving.
            let (title, doing) = match action {
                FileAction::Close => ("Quit".to_string(), "quitting".to_string()),
                FileAction::CloseScene => ("Close Scene".to_string(), "closing it".to_string()),
                other => (format!("{other:?}"), format!("{other:?}")),
            };
            let message = format!(
                "Save {} before {doing}? It has unsaved changes or a live tool.",
                self.scene_label()
            );
            match self.show_confirm(ctx, &title, &message, &["Discard"]) {
                form::Outcome::Ok => self.prompt_save(action),
                form::Outcome::Extra(_) => {
                    self.pending = None;
                    // Quit keeps the discarded changes in memory until
                    // every scene is decided, so a later Cancel loses
                    // nothing (continue_quit).
                    self.do_file(action);
                }
                form::Outcome::Cancel => {
                    self.pending = None;
                    self.queued_open = None;
                    self.scenes.closing_all = false;
                }
                form::Outcome::Open | form::Outcome::No => {}
            }
            ctx.request_repaint();
            return;
        }
        if let Some(mut dialog) = self.file_dialog.take() {
            let export = match dialog.action {
                FileAction::Export(format) => Some(format),
                _ => None,
            };
            let save = matches!(dialog.action, FileAction::SaveAs) || export.is_some();
            // UIC-81: the command's name; the path's help is the row's
            // tooltip (UIC-82).
            let (title, help) = match export {
                Some(format) => (
                    format!("Export {}", format.label()),
                    format!(".{} file · absolute file path", format.extension()),
                ),
                None if save => (
                    "Save As".to_string(),
                    "Operandi scene (.opscene) · absolute file path".to_string(),
                ),
                None => (
                    "Open".to_string(),
                    "Operandi scene (.opscene), Modo Scene (.lxo, any case), OBJ, STL, glTF, \
                     GLB or VideoScape GEO · absolute file path. Files are checked before \
                     import; unsupported content is refused."
                        .to_string(),
                ),
            };
            // The form lives in egui's memory while the dialog is open.
            let id = Id::new(("file-dialog-form", format!("{:?}", dialog.action)));
            let frame = ctx.cumulative_frame_nr();
            let mut form = ctx
                .data_mut(|d| d.get_temp::<(form::Dialog, u64)>(id))
                .filter(|(_, shown)| shown + 1 >= frame)
                .map(|(form, _)| form)
                .unwrap_or_else(|| {
                    let rows = vec![
                        form::Row::text("path", "Path", &dialog.path)
                            .fresh()
                            .tip(help.clone()),
                    ];
                    form::Dialog::with_defaults(&title, rows).focus("path")
                });
            let outcome = self.show_form(ctx, &mut form);
            let typed = form.values.text("path").to_owned();
            if typed != dialog.path {
                dialog.path = typed;
                dialog.overwrite = false;
                form.error = None;
            }
            let mut keep = true;
            match outcome {
                form::Outcome::Ok => {
                    let p = PathBuf::from(&dialog.path);
                    if !p.is_absolute() {
                        self.status = "Enter an absolute file path".into();
                    } else if save {
                        if p.exists() && !dialog.overwrite {
                            // Rows never appear (UIC-92): the question goes
                            // to the status line and OK's tooltip; OK
                            // again replaces the file.
                            dialog.overwrite = true;
                            self.status = "File exists: OK again replaces it".into();
                            form.error = Some(self.status.clone());
                        } else if let Some(format) = export {
                            keep = !self.export_to(p, format);
                        } else if self.save(p) {
                            keep = false;
                            if let Some(action) = self.pending.take() {
                                self.do_file(action);
                            }
                        }
                    } else if matches!(dialog.action, FileAction::Import) {
                        // PG-07: into the current scene.
                        self.import_into_current(
                            &p,
                            crate::import_into::ImportKinds::default(),
                            None,
                        );
                        keep = false;
                    } else {
                        self.open(p);
                        keep = false;
                    }
                }
                form::Outcome::Cancel => {
                    keep = false;
                    self.pending = None;
                    self.queued_open = None;
                }
                _ => {}
            }
            if keep {
                ctx.data_mut(|d| d.insert_temp(id, (form, frame)));
                self.file_dialog = Some(dialog);
            } else {
                ctx.data_mut(|d| d.remove::<(form::Dialog, u64)>(id));
                ctx.request_repaint();
            }
            return;
        }
        if self.show_bindings && self.pending.is_none() && self.file_dialog.is_none() {
            self.input_map_window(ctx);
        }
    }
}

#[cfg(test)]
mod history_widget_tests {
    use super::*;
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.editor.scene = serde_json::from_str(include_str!(
            "../../../../docs/i2/expectations/graphs/G0.json"
        ))
        .unwrap();
        a.editor.synchronize_identities().unwrap();
        a.editor
            .dispatch(Command::Rename("renamed".into()))
            .unwrap();
        a
    }
    fn context() -> Context {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx
    }
    fn text_shapes(shape: &egui::Shape, text: &mut Vec<(String, Pos2)>) {
        match shape {
            egui::Shape::Text(s) => {
                text.push((s.galley.text().to_owned(), s.pos + s.galley.size() * 0.5))
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    text_shapes(shape, text);
                }
            }
            _ => {}
        }
    }
    fn texts(output: &egui::FullOutput) -> Vec<(String, Pos2)> {
        let mut text = vec![];
        for shape in &output.shapes {
            text_shapes(&shape.shape, &mut text);
        }
        text
    }
    fn widget(
        a: &Operandi,
        ctx: &Context,
        redo: bool,
        time: f64,
        events: Vec<Event>,
    ) -> (Response, Vec<(String, Pos2)>) {
        let mut response = None;
        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1800., 600.))),
                focused: true,
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    response = Some(a.history_button(ui, redo));
                });
            },
        );
        (response.unwrap(), texts(&output))
    }
    fn toolbar(
        a: &mut Operandi,
        ctx: &Context,
        time: f64,
        events: Vec<Event>,
    ) -> Vec<(String, Pos2)> {
        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1800., 600.))),
                focused: true,
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| a.toolbar(ctx),
        );
        texts(&output)
    }
    fn local() -> Operandi {
        let mut a = app();
        a.editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        a.editor
            .dispatch(Command::ConfigureExtrude(Parameters {
                extrude_mode: ExtrudeMode::Shift,
                extrude_uvs: ExtrudeUvs::None,
                ..Default::default()
            }))
            .unwrap();
        a.editor.dispatch(Command::BeginExtrude).unwrap();
        a
    }
    /// Jason, 2026-11-22 (UIC-11, UIC-16): on Windows and Linux the menu
    /// bar is its own row above the tool set bar, and stays when that bar
    /// is collapsed to its thin line.
    #[test]
    fn in_window_menu_bar_has_its_own_row_above_the_tool_sets() {
        let mut a = app();
        a.menus_in_system_bar = false;
        let ctx = context();
        let texts = toolbar(&mut a, &ctx, 0., vec![]);
        let menu = a.layouts.menu_bar_rect.expect("menu bar row shown");
        let file = texts
            .iter()
            .find(|(t, _)| t == "File")
            .expect("File menu")
            .1;
        assert!(menu.contains(file), "File sits in the menu row");
        let logo = a.layouts.logo_rect.expect("logo shown");
        let (_, model) = a.layouts.tool_set_rects[0];
        assert!(
            menu.bottom() <= logo.top(),
            "menu {menu:?} above logo {logo:?}"
        );
        assert!(
            menu.bottom() <= model.top(),
            "menu {menu:?} above {model:?}"
        );
        // Save/Undo/Redo stay on the tool set row.
        let save = texts.iter().find(|(t, _)| t == "Save").expect("Save").1;
        assert!(save.y > menu.bottom() && save.y < logo.bottom());

        // Collapsed by the logo (UIC-16): the menu row stays, the thin
        // line sits under it.
        for mode_bar_bottom in [false, true] {
            a.layouts.top_collapsed = true;
            a.layouts.mode_bar_bottom = mode_bar_bottom;
            a.layouts.menu_bar_rect = None;
            let texts = toolbar(&mut a, &ctx, 1., vec![]);
            let menu = a.layouts.menu_bar_rect.expect("menu row while collapsed");
            assert!(texts.iter().any(|(t, _)| t == "File"));
            assert!(a.layouts.logo_rect.is_none());
            let line = a.layouts.strip_rect.expect("collapsed line");
            assert!(menu.bottom() <= line.top(), "menu {menu:?} above {line:?}");
        }

        // macOS: the menus are in the system bar, so no in-window row.
        let mut mac = app();
        mac.menus_in_system_bar = true;
        let texts = toolbar(&mut mac, &context(), 0., vec![]);
        assert!(mac.layouts.menu_bar_rect.is_none());
        assert!(!texts.iter().any(|(t, _)| t == "File"));
    }
    #[test]
    fn hw_c01_armed_surface_widget_agrees_with_authoritative_undo() {
        let mut a = app();
        a.editor.dispatch(Command::Activate(Tool::Bevel)).unwrap();
        assert!(
            a.editor
                .history_eligibility(HistoryAction::Undo)
                .is_available()
        );
        let (response, _) = widget(&a, &context(), false, 0., vec![]);
        println!(
            "HW-C01 fixed: widget={}, core=Available",
            response.enabled()
        );
        assert!(response.enabled());
    }
    #[test]
    fn hw_c02_numeric_widget_agrees_with_authoritative_history() {
        for cone in [false, true] {
            let mut a = app();
            a.editor
                .dispatch(if cone {
                    Command::ArmCone
                } else {
                    Command::ArmEllipsoid
                })
                .unwrap();
            // Undo now runs mid-tool for the primitives (MTU-Q4).
            assert!(
                a.editor
                    .history_eligibility(HistoryAction::Undo)
                    .is_available()
            );
            let (response, _) = widget(&a, &context(), false, 0., vec![]);
            println!(
                "HW-C02 fixed: Cone={cone}, widget={}, core=Available",
                response.enabled()
            );
            assert!(response.enabled());
        }
    }
    #[test]
    fn hw_c03_toolbar_and_fallback_edit_use_local_labels_and_remapping() {
        let mut a = local();
        a.bindings.insert("Undo".into(), "Ctrl+U".into());
        assert_eq!(
            a.editor.history_eligibility(HistoryAction::Undo).label,
            Some("Begin Extrude")
        );
        let ctx = context();
        let first = toolbar(&mut a, &ctx, 0., vec![]);
        let expected = "Undo Begin Extrude  [Ctrl+U] ";
        println!(
            "HW-C03 toolbar labels: {:?}",
            first
                .iter()
                .filter(|(t, _)| t.starts_with("Undo"))
                .collect::<Vec<_>>()
        );
        assert!(first.iter().any(|(text, _)| text == expected));
        // Headless test process never installs AppKit. Exercise the actual
        // fallback Edit caller by opening its egui menu, without OS changes.
        assert!(!native_menu::installed());
        let edit = first.iter().find(|(text, _)| text == "Edit").unwrap().1;
        toolbar(
            &mut a,
            &ctx,
            0.1,
            vec![
                Event::PointerMoved(edit),
                crate::tests::pointer(edit, true, Modifiers::NONE),
            ],
        );
        toolbar(
            &mut a,
            &ctx,
            0.2,
            vec![crate::tests::pointer(edit, false, Modifiers::NONE)],
        );
        assert!(
            egui::Popup::is_any_open(&ctx),
            "actual Edit popup must open"
        );
        let menu = toolbar(&mut a, &ctx, 0.4, vec![]);
        println!(
            "HW-C03 fallback labels: {:?}",
            menu.iter()
                .filter(|(t, _)| t.starts_with("Undo"))
                .collect::<Vec<_>>()
        );
        assert!(
            menu.iter().filter(|(text, _)| text == expected).count() >= 2,
            "toolbar and fallback popup must each paint the label: {menu:?}"
        );
    }
    fn hovered_text(a: &Operandi) -> Vec<(String, Pos2)> {
        let ctx = context();
        let (response, _) = widget(a, &ctx, false, 0., vec![]);
        assert!(!response.enabled());
        widget(
            a,
            &ctx,
            false,
            0.1,
            vec![Event::PointerMoved(response.rect.center())],
        );
        widget(a, &ctx, false, 2., vec![]);
        widget(a, &ctx, false, 2.2, vec![]).1
    }
    #[test]
    fn hw_c04_empty_and_refused_widgets_show_distinct_real_tooltips() {
        let mut a = app();
        let (_, ordinary) = widget(&a, &context(), false, 0., vec![]);
        // The label carries the input-map chord (Cmd+Z by default).
        let label = format!("Undo Rename Mesh{}", a.shortcut("Undo"));
        assert!(ordinary.iter().any(|(t, _)| t == &label));
        a.editor.history.undo.clear();
        assert!(
            hovered_text(&a)
                .iter()
                .any(|(t, _)| t == "No history available")
        );
        // A tool that still refuses Undo while active.
        a.editor.dispatch(Command::ArmReduction).unwrap();
        let expected = match a
            .editor
            .history_eligibility(HistoryAction::Undo)
            .availability
        {
            HistoryAvailability::Refused(reason) => reason.to_string(),
            other => panic!("expected numeric refusal: {other:?}"),
        };
        let text = hovered_text(&a);
        assert!(
            text.iter().any(|(t, _)| t == &expected),
            "actual tooltip text: {text:?}"
        );
    }
    #[test]
    fn hw_c05_widget_queries_are_pure_and_stale_enabled_state_is_not_a_permit() {
        let mut a = app();
        let before = a.editor.clone();
        let marks = (
            a.editor.identities.next_id().unwrap(),
            a.editor.i2.serial.next().unwrap(),
        );
        let ctx = context();
        for time in [0., 0.1, 0.2] {
            assert!(widget(&a, &ctx, false, time, vec![]).0.enabled());
        }
        assert!(a.editor == before);
        assert_eq!(
            marks,
            (
                a.editor.identities.next_id().unwrap(),
                a.editor.i2.serial.next().unwrap()
            )
        );
        // State changed after the visible Available widget. run still routes to
        // the same execution guard and cannot consume the old availability.
        // (Move now keeps running through Undo, so Polygon Reduction is
        // the refusing owner here.)
        a.editor.dispatch(Command::ArmReduction).unwrap();
        let protected = a.editor.clone();
        a.run(Command::Undo);
        assert_eq!(a.status, "Drop or Cancel Polygon Reduction before Undo");
        assert!(a.editor == protected);
        assert!(!widget(&a, &ctx, false, 0.3, vec![]).0.enabled());

        let mut a = local();
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.extent = DVec3::Z * 0.5;
        a.editor.dispatch(Command::Preview(p)).unwrap();
        let (_, text) = widget(&a, &context(), false, 0., vec![]);
        let label = format!("Undo Move Extrude{}", a.shortcut("Undo"));
        assert!(text.iter().any(|(t, _)| t == &label));
        a.run(Command::Undo);
        let (_, text) = widget(&a, &context(), true, 0., vec![]);
        let label = format!("Redo Move Extrude{}", a.shortcut("Redo"));
        assert!(text.iter().any(|(t, _)| t == &label));
    }
}

#[cfg(test)]
mod mode_availability_tests {
    use super::*;
    fn text(shape: &egui::Shape, out: &mut Vec<(String, Pos2)>) {
        match shape {
            egui::Shape::Text(t) => {
                out.push((t.galley.text().to_owned(), t.pos + t.galley.size() * 0.5))
            }
            egui::Shape::Vec(v) => {
                for s in v {
                    text(s, out)
                }
            }
            _ => {}
        }
    }
    fn frame(
        a: &mut Operandi,
        ctx: &Context,
        time: f64,
        events: Vec<Event>,
    ) -> Vec<(String, Pos2)> {
        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1800., 600.))),
                focused: true,
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| a.toolbar(ctx),
        );
        let mut out = vec![];
        for s in output.shapes {
            text(&s.shape, &mut out)
        }
        out
    }
    #[test]
    fn mode_availability_actual_fallback_select_entry_characterization() {
        for tool in ["cone", "ellipsoid", "spike", "smooth shift", "thicken"] {
            let mut a = Operandi::empty();
            a.editor.scene = serde_json::from_str(include_str!(
                "../../../../docs/i2/expectations/graphs/G0.json"
            ))
            .unwrap();
            a.execute(tool);
            a.status = "before click".into();
            a.bindings.insert("Edges".into(), "Ctrl+2".into());
            let ctx = Context::default();
            theme::install(&ctx);
            let first = frame(&mut a, &ctx, 0., vec![]);
            let select = first.iter().find(|(t, _)| t == "Select").unwrap().1;
            frame(
                &mut a,
                &ctx,
                0.1,
                vec![
                    Event::PointerMoved(select),
                    crate::tests::pointer(select, true, Modifiers::NONE),
                ],
            );
            frame(
                &mut a,
                &ctx,
                0.2,
                vec![crate::tests::pointer(select, false, Modifiers::NONE)],
            );
            assert!(egui::Popup::is_any_open(&ctx));
            frame(&mut a, &ctx, 0.4, vec![]);
            let menu = frame(&mut a, &ctx, 0.45, vec![]);
            println!(
                "settled Mode menu {tool}: {:?}",
                menu.iter()
                    .filter(|(t, _)| t.starts_with("Vertices") || t.starts_with("Edges"))
                    .collect::<Vec<_>>()
            );
            let edge = menu
                .iter()
                .rev()
                .find(|(t, _)| t == "Edges  [Ctrl+2] ")
                .unwrap()
                .1;
            frame(
                &mut a,
                &ctx,
                0.5,
                vec![
                    Event::PointerMoved(edge),
                    crate::tests::pointer(edge, true, Modifiers::NONE),
                ],
            );
            frame(
                &mut a,
                &ctx,
                0.6,
                vec![crate::tests::pointer(edge, false, Modifiers::NONE)],
            );
            frame(&mut a, &ctx, 0.8, vec![]);
            println!(
                "Mode fallback {tool}: status={}; mode={:?}",
                a.status, a.editor.scene.selection.mode
            );
            // Every open tool is dropped first (Jason, 2026-11-05).
            assert_eq!(a.editor.scene.selection.mode, Mode::Edge, "{tool}");
            assert!(!a.editor.session.has_active_task(), "{tool}");
            assert_eq!(a.status, "Ready", "{tool}");
        }
    }
}

impl Operandi {
    /// The Unsaved-changes prompt's Save: save (asking where if untitled),
    /// then carry on with `action`. Cancelling keeps the prompt.
    pub(super) fn prompt_save(&mut self, action: FileAction) {
        if let Some(p) = self.path.clone() {
            if self.save(p) {
                self.pending = None;
                self.do_file(action);
            }
            return;
        }
        let pending = self.pending.take();
        self.request_file(FileAction::SaveAs);
        if self.file_dialog.is_none() && self.path.is_some() && !self.dirty() {
            // The native panel saved: carry on with the action.
            self.do_file(action);
        } else {
            // The typed dialog is still open, or the panel was cancelled.
            self.pending = pending;
        }
    }
}
