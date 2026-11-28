//! A 3D view's pointer and key input: presses, drags and releases for tools,
//! handles, selection and navigation, routed to the editor's commands.
use super::*;

impl Operandi {
    pub(super) fn viewport_input(&mut self, ui: &Ui, response: &Response, r: Rect) {
        if self.show_plane
            || self.show_diagnostics
            || self.show_about
            || self.modal_owned_input
            || self.menu_owned_input
            || egui::Popup::is_any_open(ui.ctx())
            || self.file_dialog.is_some()
            || self.pending.is_some()
            || self.show_bindings
            || self.recovery.is_some()
            || self.recovery_warning.is_some()
            // A preset dragged out of the browser (PB-110) owns the
            // pointer: the views neither select nor run tools under it.
            || self.preset_browser.drag.is_some()
            // UI build mode: the views only move as panels (Jason,
            // 2026-11-20), they don't select, navigate or run tools.
            || self.dots.build_mode
        {
            self.end_gesture();
            self.drop_drag = None;
            return;
        }
        if !ui.input(|i| i.focused) {
            self.end_gesture();
            self.drop_drag = None;
            return;
        }
        // UIC-71: the frame and its controls take their own presses.
        let header = self.corner.header_hits.get(&self.drawing_view);
        let grip = self
            .dots
            .rects
            .get(&panel_dot::Dot::View(self.drawing_view));
        let drawing_falloff = self.falloff_draw_armed && self.screen_falloff_press().is_some();
        let events: Vec<Event> = ui
            .input(|i| i.events.clone())
            .into_iter()
            .filter(|event| {
                let Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } = *event
                else {
                    return true;
                };
                // N03: a popover's title bar must not drive the active tool.
                if self.overlay_rects.iter().any(|rect| rect.contains(pos)) {
                    return false;
                }
                // UIC-61/181: a press on a window drawn above this view (a
                // popover such as the Add browser, a floating panel, a
                // menu) is that window's. A panel drawn after this view
                // claims the pointer (`menu_owned_input`) too late for
                // this frame, so ask egui which layer is on top instead: in
                // Item mode the press deselected the mesh, which closed the
                // Mesh Stack's Add Operator browser (bug report #5).
                if over_window_above(ui, pos) {
                    return false;
                }
                let over_grip = grip.is_some_and(|rect| rect.contains(pos));
                if over_grip && button == PointerButton::Primary && drawing_falloff {
                    return true;
                }
                !over_grip
                    && !header.is_some_and(|hits| hits.iter().any(|rect| rect.contains(pos)))
                    && !(r.contains(pos) && panel_dot::grip_shortcut(button, modifiers))
            })
            .collect();
        // Screen Axis searches along the view the tool is used in: note it
        // on each press here (Constrain to Background).
        if self.editor.background_constraint.is_some()
            && events.iter().any(|e| {
                matches!(e, Event::PointerButton { pos, pressed: true, .. } if r.contains(*pos))
            })
        {
            self.note_constraint_view();
        }
        let press_time = ui.input(|i| i.time);
        let double_click_delay = ui.ctx().options(|o| o.input_options.max_double_click_delay);
        let badge = Rect::from_min_max(
            r.left_bottom() + vec2(0., -90.),
            r.left_bottom() + vec2(98., 0.),
        );
        // A release this view didn't see (outside it, or a path that
        // returned early) still ends the held tweak.
        if self.pointer_held
            && events
                .iter()
                .all(|e| !matches!(e, Event::PointerButton { .. }))
            && !ui.input(|i| i.pointer.any_down())
        {
            self.release_pointer();
        }
        // Free Cursor (the tester, 2026-11-25): during a drag the pointer
        // is put back in at the opposite edge when it reaches one, and the
        // drag reads on as if it had kept going.
        let mut events = events;
        if self.navigation.is_none() {
            self.cursor_wrap.reset();
        }
        let real_latest = crate::cursor_wrap::latest(&events);
        self.cursor_wrap.read_events(&mut events);
        // Whatever the Input Device (the tester, 2026-11-26): with a pen,
        // whose place can't be moved, the drag simply goes on unwrapped.
        if self.input_preferences.options.unrestricted_cursor
            && self.navigation.as_ref().is_some_and(|g| wraps(&g.kind))
            && ui.input(|i| i.pointer.any_down())
            && let Some(real) = real_latest
            && let Some(to) = self.cursor_wrap.put_back(real, r)
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorPosition(to));
        }
        // A real mouse reports several moves per frame. A move followed by
        // another before any button change is superseded: gestures that
        // read only the start and the latest position skip it, so a drag
        // runs its tool update once per frame instead of once per event
        // (on a large mesh the per-event updates outran the frame and the
        // drag fell behind further every frame).
        let superseded: Vec<bool> = (0..events.len())
            .map(|i| {
                matches!(events[i], Event::PointerMoved(_))
                    && events[i + 1..]
                        .iter()
                        .take_while(|e| !matches!(e, Event::PointerButton { .. }))
                        .any(|e| matches!(e, Event::PointerMoved(_)))
            })
            .collect();
        let pointer_down = ui.input(|i| i.pointer.any_down());
        for (index, event) in events.into_iter().enumerate() {
            match event {
                Event::PointerMoved(_)
                    if superseded[index]
                        && self
                            .navigation
                            .as_ref()
                            .is_some_and(|g| latest_position_only(&g.kind)) => {}
                // Alt+Shift+middle-click: the default field of view (view_fov.rs).
                Event::PointerButton {
                    pos,
                    button: PointerButton::Middle,
                    pressed: true,
                    modifiers,
                } if r.contains(pos) && self.is_fov_reset_press(modifiers) => {
                    self.reset_fov(press_time);
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } if r.contains(pos)
                    && matches!(button, PointerButton::Secondary | PointerButton::Middle) =>
                {
                    if !self.confirm_fields() {
                        continue;
                    }
                    // Every press to its release is one undo step, whatever
                    // gesture it starts below.
                    self.hold_pointer();
                    // HM-01: modifiers are decided at the press.
                    self.begin_constrain(modifiers);
                    // Blender- and Maya-style presets navigate with these buttons.
                    if let Some(nav) = self
                        .input_preferences
                        .navigation
                        .navigation(button, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Navigate(nav),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // IK-51: middle-drag (FK) and right-click (pins).
                    if self.pose_active()
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.pose_press(pos, r, button, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // Solid Sketch: right drag scales, middle click deletes.
                    if let Some(kind) = self.solid_sketch_press(pos, r, button, modifiers) {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // SC-22/23: with a sculpt brush, right-drag sizes it and
                    // middle-drag sets its strength; a right-click that
                    // doesn't move still opens the menu (on release).
                    // PNT-53: with a paint tool, right-drag sizes the tip;
                    // middle-drag does nothing.
                    #[cfg(feature = "painting")]
                    if self.painting.brush.tool.is_some() && input::navigation(modifiers).is_none()
                    {
                        let kind = if button == PointerButton::Secondary {
                            self.begin_paint_size(pos)
                        } else {
                            input::GestureKind::Blocked
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.sculpt.brush.is_some() && input::navigation(modifiers).is_none() {
                        let kind = if button == PointerButton::Secondary {
                            self.begin_sculpt_size(pos)
                        } else {
                            self.begin_sculpt_strength(pos)
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // MN-03.1: a drawn Cube, Sphere or Cylinder: a middle drag
                    // keeps it and starts a copy at the pointer with the same
                    // settings, which the drag then adjusts.
                    if button == PointerButton::Middle
                        && modifiers.is_none()
                        && self
                            .haul_tool(operandi_core::haul::Button::Middle)
                            .is_none()
                        && let Some(live) =
                            self.editor.live.clone().filter(|l| {
                                matches!(l.tool, Tool::Box | Tool::Sphere | Tool::Cylinder)
                            })
                    {
                        let kind = self.begin_primitive_copy(live, pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // HM-04: with a transform tool, a middle press repeats the
                    // previous drag's amounts and a drag goes on from there; a
                    // click without moving keeps the repeat. Navigation
                    // presets that claim the button win (above).
                    if button == PointerButton::Middle
                        && modifiers.is_none()
                        && let Some(tool) = self
                            .editor
                            .live
                            .as_ref()
                            .map(|l| l.tool)
                            .filter(|t| t.transforms_items())
                    {
                        // RPT-02/07: a typed amount still showing counts as
                        // an apply, so record it as its own step first and
                        // the middle-click repeats it (type 15, click: 30°).
                        if self.editor.transform_change_pending() {
                            self.run(Command::CommitTransformStep);
                            self.clear_fields();
                        }
                        // The repeat and the drag after it are one tweak
                        // (the press holds the pointer, above).
                        let kind = match self.editor.repeat_transform_params() {
                            Ok(params) => {
                                self.run(Command::Preview(params));
                                self.free_transform_drag(tool)
                            }
                            Err(e) => {
                                self.status = e;
                                input::GestureKind::Blocked
                            }
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.draw.is_some()
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.draw_press(
                            pos,
                            r,
                            modifiers,
                            if button == PointerButton::Secondary {
                                1
                            } else {
                                2
                            },
                        )
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // Pen walls: right-drag left/right sets Offset.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(draft) = &self.editor.pen_draft
                        && draft.options().wall != WallMode::Off
                    {
                        let at = draft
                            .points()
                            .last()
                            .copied()
                            .unwrap_or(self.workplane_center());
                        let metres_per_pixel = self.camera.metres_per_pixel(at, r).unwrap_or(0.01);
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::PenOffset {
                                start_offset: draft.options().offset,
                                metres_per_pixel,
                            },
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Secondary
                        && self.utility.is_some()
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.utility_press(pos, r, modifiers, true)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(live) = self.editor.live.as_ref()
                        && live.tool.is_primitive()
                        // Sphere and Cylinder right-drag through the drag
                        // layer below; Box keeps its own Segments drag.
                        && self.haul_tool(operandi_core::haul::Button::Right).is_none()
                    {
                        self.navigation = Some(input::Navigation {
                            kind: self.begin_primitive_alternate(live.clone(), self.actual_plane()),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // Duplicate tools: right-drag changes counts (CL-22,
                    // AR-22, RA-21); press to release is one step.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.duplicate_count_press(pos)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // SPEC-SELECTION-FALLOFF-FLEX §1, FO-46: with a
                    // Selection falloff in the stack under a live tool, a
                    // right-drag anywhere changes Steps instead of the
                    // tool's own right-drag, and a right-click adds one
                    // (provisional, SF-Q7, FO-Q04).
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some((layer, f)) = falloff_panel::selection_layer(&self.editor)
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.card().layers.falloff)
                    {
                        self.status = format!(
                            "Selection falloff Steps: {} · drag right for more, left for fewer",
                            f.steps
                        );
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::FalloffSteps {
                                layer,
                                start: f.steps,
                            },
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // FO-44: a right-drag resizes the Airbrush brush.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.begin_airbrush_size()
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // MN-03.2: a transform tool's right-drag, where its row in
                    // the hauls table has no right button, sets up the falloff
                    // in use: Linear from the press to the pointer, Radial and
                    // Cylinder around the press, Screen's circle.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.right_drag_falloff_setup(pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // FO-47 / FO-Q05 (provisional): with a Soft Selection
                    // falloff under a live tool, a right-drag changes
                    // Radius, right for more, left for less.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some((layer, f)) = falloff_panel::soft_selection_layer(&self.editor)
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.card().layers.falloff)
                        && let Some(kind) = self.begin_falloff_radius(layer, f.radius, pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // The shared drag layer (DL-01..09): after navigation,
                    // the falloff layer's own right-drag and the tool's
                    // other claims above, a right or middle drag drives the
                    // value the tool's row lists for that button. A middle
                    // click on Smooth Shift still clones it.
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && let Some(which) = crate::haul_view::button_of(button)
                        && let Some(kind) = {
                            let click = self.haul_click(which, modifiers);
                            self.begin_haul(which, pos, r, click)
                        }
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // TP-20: Topology Pen's right and middle gestures.
                    if matches!(button, PointerButton::Secondary | PointerButton::Middle)
                        && input::navigation(modifiers).is_none()
                        && self.topology_pen_open()
                    {
                        let kind = self.topology_pen_press(button, modifiers, pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // TWR-06/10: with Tweak, a right-drag anywhere resizes
                    // Range; Shift, Ctrl and Cmd change nothing. (Alt drags
                    // stay navigation, above.)
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool == Tool::Tweak)
                    {
                        // Press to release is one undo step (TWR-14).
                        self.navigation = Some(input::Navigation {
                            kind: self.begin_tweak_range(pos, r),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && let Some(kind) = self.slice_press(pos, r, modifiers, button)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.is_smooth_shift())
                    {
                        self.run(Command::CloneSmoothShift);
                        self.clear_fields();
                        continue;
                    }
                    // DPM-06/10 (D1): middle-click a Pen or Tube point
                    // deletes it in any mode; Shift also deletes every later
                    // point. A middle-click elsewhere does nothing.
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && !modifiers.ctrl
                        && !modifiers.mac_cmd
                        && (self.editor.pen_draft.is_some() || self.tube_active())
                    {
                        self.point_delete_press(pos, r, modifiers.shift);
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Blocked,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.editor.live.is_some()
                        || self.editor.pen_draft.is_some()
                        || input::navigation(modifiers).is_some()
                    {
                        self.status="This tool/navigation context owns non-primary hauling; selection lasso is unavailable".into();
                        continue;
                    }
                    if self.editor.scene.selection.mode == Mode::Item {
                        if button == PointerButton::Secondary && self.mouse_region_click(pos, r) {
                            continue;
                        }
                        self.status =
                            "Lasso can't select items yet; select them in the Scenes list".into();
                        continue;
                    }
                    self.run(Command::BeginSelection);
                    self.navigation = Some(input::Navigation {
                        kind: input::GestureKind::Lasso(vec![pos], button == PointerButton::Middle),
                        start: pos,
                        last: pos,
                        modifiers,
                        button,
                    });
                }
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } if r.contains(pos) => {
                    if self.plane_widget && badge.contains(pos) {
                        continue;
                    }
                    self.hold_pointer();
                    if !self.confirm_fields() {
                        if let Some(id) = self.field_focus {
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        continue;
                    }
                    self.release_field_for_view_press(ui.ctx());
                    if self.editor.solid_sketch.is_some()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) =
                            self.solid_sketch_press(pos, r, PointerButton::Primary, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    if self.draw.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.draw_press(pos, r, modifiers, 0)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // IK-50: the Pose tool takes the press first.
                    if self.pose_active()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) =
                            self.pose_press(pos, r, PointerButton::Primary, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // SK-10: the Skeleton tool takes a plain or Shift press;
                    // Alt stays navigation.
                    if self.skeleton_active()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.skeleton_press(pos, r, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // ACS-30: in a rig's Guide context, a press on a guide
                    // drags it (no tool active; Alt stays navigation).
                    if input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.rig_guide_press(pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // A Create panel utility (Paste Tool, Ruler, Align…) takes
                    // the press when it wants it.
                    if self.utility.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.utility_press(pos, r, modifiers, false)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // A sculpt brush takes a plain, Ctrl or Shift press (SC-60,
                    // SC-61); Alt stays navigation (SC-64) and Cmd keeps its
                    // menu.
                    // PNT-48: a paint tool takes a plain press; Alt stays
                    // navigation, Cmd keeps its menu.
                    #[cfg(feature = "painting")]
                    if (self.painting.brush.tool.is_some() || self.painting.brush.lasso_mode)
                        && input::navigation(modifiers).is_none()
                        && !(modifiers.mac_cmd
                            && self.painting.brush.tool
                                != Some(operandi_core::paint_stroke::PaintTool::Clone))
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.paint_press(pos, r, modifiers);
                        ui.ctx().request_repaint();
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    if self.sculpt.brush.is_some()
                        && input::navigation(modifiers).is_none()
                        && !modifiers.mac_cmd
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.sculpt_press(pos, r, modifiers);
                        ui.ctx().request_repaint();
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // Armed Edge panel tool (Add Loop / Add Point): a click on
                    // an edge adds one; Ctrl places it at the middle, Shift
                    // is an ordinary click (AL-PHY-02/03), and Command does
                    // nothing and opens no menu (AL-PHY-04).
                    if self.edge_click.is_some()
                        && modifiers.mac_cmd
                        && input::navigation(modifiers).is_none()
                    {
                        continue;
                    }
                    if self.edge_click.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let (taken, slide) = self.edge_click_at(pos, r, modifiers.ctrl);
                        if let Some(kind) = slide {
                            self.navigation = Some(input::Navigation {
                                kind,
                                start: pos,
                                last: pos,
                                modifiers,
                                button: PointerButton::Primary,
                            });
                        }
                        if taken {
                            continue;
                        }
                    }
                    // CS-02/15: the first press with an armed tool makes it
                    // live at the click, then goes on as that tool's own
                    // press (click_start_view.rs).
                    if self.editor.is_armed()
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && !self.start_armed_tool(pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Blocked,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // Duplicate tools take their presses (duplicate_view.rs).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.duplicate_press(pos, r, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // DL-Q04 (provisional): on a Mac trackpad Cmd+drag
                    // stands in for a middle drag and Cmd+Shift+drag for a
                    // right drag, for tools whose drag-control row lists
                    // that button. Handles and the tool's own on-screen
                    // controls still win (DL-01).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && let Some(which) = crate::haul_view::trackpad_button(modifiers)
                        && self.haul_owner(which).is_some()
                        && self.handle_hit(pos, r).is_none()
                        && !self.haul_on_tool_control(pos, r)
                        && let Some(kind) = {
                            let click = self.haul_click(which, modifiers);
                            self.begin_haul(which, pos, r, click)
                        }
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // Slice tools take their presses (slice_view.rs).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) =
                            self.slice_press(pos, r, modifiers, PointerButton::Primary)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // Weld (WD-01): the press picks a point and drags it.
                    if self.vertex_weld
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.weld_press(pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // A click adding a point with Create. A press on a
                    // handle is left to the handle gesture.
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool == Tool::VertexCreate)
                        && self.vertex_click_at(pos, r)
                    {
                        continue;
                    }
                    // Draw tools Pen: Shift-click ends this polygon and starts
                    // the next; it adds no point.
                    if self.editor.pen_draft.is_some()
                        && modifiers.shift_only()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        if self.run_ok(Command::Drop) {
                            self.run(Command::PenStart);
                            self.status = "Pen · new polygon · click its first point".into();
                        }
                        continue;
                    }
                    // Make Quads: Ctrl+click ends the strip with a triangle.
                    let strip_end = modifiers.ctrl
                        && !modifiers.shift
                        && !modifiers.alt
                        && !modifiers.mac_cmd
                        && self
                            .editor
                            .pen_draft
                            .as_ref()
                            .is_some_and(|d| d.options().strip())
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none();
                    if self.editor.pen_draft.is_some() && (modifiers.is_none() || strip_end) {
                        if self.editor_owned_input || self.text_editor_focused(ui.ctx()) {
                            self.status =
                                "Finish editing the field before capturing a Pen point".into();
                        } else if let Some(point) = self.pen_press(pos, r, strip_end) {
                            // A click adds a vertex; pressing on a
                            // vertex of this session picks it; dragging moves it.
                            self.navigation = Some(input::Navigation {
                                kind: input::GestureKind::PenDrag { point },
                                start: pos,
                                last: pos,
                                modifiers,
                                button: PointerButton::Primary,
                            });
                        }
                        continue;
                    }
                    // Drop Action (DA-4): a press on an item in Item mode
                    // with no tool may become a drop; it waits for release.
                    if self.drop_press(pos, r, modifiers) {
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // HM-01: modifiers are decided at the press.
                    self.begin_constrain(modifiers);
                    // HM-02: Shift-drag with a transform tool continues the
                    // drag just made (its values keep growing); a plain drag
                    // starts fresh.
                    let continues = modifiers.shift
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !modifiers.mac_cmd
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.transforms_items());
                    if continues {
                        self.run(Command::ContinueTransformStep);
                    }
                    // MN-03.3: with the Screen falloff on, Bevel and Twist
                    // Bevel follow the transform rule: a plain drag starts a
                    // fresh edit on the last one's result, Shift keeps
                    // adjusting the same edit.
                    let screen_rule = self.screen_fresh_rule();
                    if screen_rule
                        && modifiers.is_none()
                        && !self.falloff_draw_armed
                        && self.falloff_hit(pos, r).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.applied && self.editor.scene != l.base)
                    {
                        self.run(Command::Continue);
                        self.bevel_dragged = false;
                    }
                    // Soft Drag (DEFORM, FO-43): every press centres the
                    // screen circle on the pointer before the drag moves
                    // what is inside it.
                    if modifiers.is_none() {
                        self.soft_drag_press(pos, r);
                    }
                    let kind = if let Some(nav) = self
                        .input_preferences
                        .navigation
                        .navigation(PointerButton::Primary, modifiers)
                    {
                        input::GestureKind::Navigate(nav)
                    } else if modifiers.is_none() && self.uv_view_project_click(r) {
                        // UVT-19: Project from View projects from this view.
                        input::GestureKind::Blocked
                    } else if modifiers.is_none()
                        && let Some(grab) = self.meshop_handle_hit(pos, r)
                    {
                        // The selected primitive MeshOp's handles.
                        input::GestureKind::MeshOpHandle(Box::new(grab))
                    } else if let (Some(handle), Some(field)) =
                        (self.falloff_hit(pos, r), self.editor.spatial_falloff)
                    {
                        self.path_drag_base = Some(self.editor.falloff_path.clone());
                        input::GestureKind::Falloff(handle, field)
                    } else if let Some(f) = self.screen_falloff_press()
                        && let Some(stroke) = if matches!(f, Falloff::Airbrush(_)) {
                            // MN-03.4: Shift lowers, Ctrl (Cmd) erases.
                            falloff_screen_view::AirbrushStroke::for_press(modifiers)
                        } else {
                            modifiers
                                .is_none()
                                .then_some(falloff_screen_view::AirbrushStroke::Add)
                        }
                    {
                        // FO-43/52 draw, FO-44 paint.
                        self.begin_screen_falloff(f, pos, r, stroke)
                    } else if modifiers.is_none() && self.path_falloff_click(pos, r) {
                        // FO-53: the click added a Path point.
                        input::GestureKind::Blocked
                    } else if modifiers.is_none() && self.falloff_draw_ready() {
                        // FO-05: the press after choosing a geometric
                        // falloff draws it.
                        self.begin_falloff_draw(pos, r)
                    } else if self.preset_center_click && modifiers.is_none() {
                        // A Snaps and Precision preset's set-up click.
                        self.place_preset_center(pos, r);
                        input::GestureKind::Blocked
                    } else if let Some(grab) = self.item_handle_hit(pos, r)
                        && modifiers.is_none()
                    {
                        input::GestureKind::ItemHandle(grab)
                    } else if self.editor.pen_draft.is_some() {
                        self.status = "Pen capture uses an unmodified primary click".into();
                        input::GestureKind::Blocked
                    } else if let Some(height) = self.begin_session_height(pos, r) {
                        height
                    } else if let Some(handle) = self
                        .begin_session_handle(pos, r)
                        .filter(|_| modifiers.is_none())
                    {
                        // #37: the drawn shape's move arrows and radius squares.
                        handle
                    } else if self.editor.reduction.is_some()
                        || self.editor.bridge.is_some()
                        || self.editor.spike.is_some()
                        || self.editor.shape.is_some()
                    {
                        // A drag anywhere drives these numeric sessions too
                        // (Jason, 2026-11-04; DRAG-Q01; was RED-002, BRI-Q05,
                        // SA-Q01, NPR-Q01).
                        if modifiers.is_none() {
                            self.begin_numeric_drag(pos, r)
                        } else {
                            input::GestureKind::Blocked
                        }
                    } else if self.editor.ellipsoid.is_some() {
                        self.begin_solid_draw(false, pos, r)
                    } else if self.editor.cone.is_some() {
                        self.begin_solid_draw(true, pos, r)
                    } else if let Some(l) = self.editor.live.clone() {
                        if l.tool.is_primitive() {
                            self.begin_primitive(l, pos, r, modifiers)
                        } else if l.tool == Tool::Tweak && self.topology_pen_open() {
                            // TP-10/20: the pen's Mode and modifier gestures.
                            self.topology_pen_press(PointerButton::Primary, modifiers, pos, r)
                        } else if l.tool == Tool::Tweak {
                            // HM-61/71: Ctrl locks the drag to one axis.
                            let ctrl_only = modifiers.ctrl
                                && !modifiers.shift
                                && !modifiers.alt
                                && !modifiers.mac_cmd;
                            if modifiers.is_none() || ctrl_only {
                                self.begin_tweak(pos, r)
                            } else {
                                self.status = "Tweak: drag without modifier keys, or with Ctrl to keep to one axis".into();
                                input::GestureKind::Blocked
                            }
                        } else if l.tool == Tool::SketchExtrude && modifiers.is_none() {
                            let center = l
                                .base
                                .active()
                                .and_then(|item| {
                                    l.base
                                        .selection
                                        .faces
                                        .first()
                                        .map(|&face| item.mesh.center(face as usize))
                                })
                                .unwrap_or(DVec3::ZERO);
                            let camera = self.camera.clone();
                            let normal = camera.basis().3;
                            if let Some(anchor) =
                                Self::sketch_plane_point(&camera, r, pos, center, normal)
                            {
                                input::GestureKind::SketchStroke(input::SketchStroke {
                                    camera,
                                    rect: r,
                                    plane_center: center,
                                    plane_normal: normal,
                                    last_sample: pos,
                                    precision: l.params.sketch_precision,
                                    samples: vec![anchor],
                                    current: anchor,
                                    started: false,
                                })
                            } else {
                                self.status =
                                    "Sketch Extrude cannot initialize its camera-facing plane"
                                        .into();
                                input::GestureKind::Blocked
                            }
                        } else if l.tool == Tool::PolyHaul && modifiers.is_none() {
                            // Selection modes: a drag anywhere (Jason,
                            // 2026-11-04; DRAG-Q01). Element modes: the
                            // element under the pointer.
                            self.begin_poly_haul(&l, pos, r)
                        } else if l.tool.is_smooth_shift()
                            && modifiers.shift
                            && !modifiers.ctrl
                            && !modifiers.alt
                        {
                            self.run(Command::Continue);
                            if let Some(l) = &self.editor.live {
                                self.run(Command::Preview(l.params.clone()));
                            }
                            input::GestureKind::Blocked
                        } else if l.tool == Tool::Extrude && !l.applied && !self.editor.i2_active()
                        {
                            // First click creates the extrusion in place
                            // (zero distance); the gizmo appears afterwards.
                            if self.editor.i2_begin_source_supported() {
                                self.run(Command::BeginExtrude);
                            } else {
                                self.run(Command::StartExtrude);
                            }
                            input::GestureKind::Blocked
                        } else if l.tool.is_smooth_shift() && !l.applied {
                            self.run(Command::Preview(l.params));
                            input::GestureKind::Blocked
                        } else if modifiers.ctrl
                            && modifiers.shift
                            && !modifiers.alt
                            && l.tool.transforms_items()
                            && let Some(axis) = self.handle_hit(pos, r)
                        {
                            // IT-PHY-04: Ctrl+Shift on a handle duplicates the
                            // item and drags the copy, leaving the original.
                            // Provisional: a plain copy, not a linked instance.
                            // HM-06: in Vertex/Edge/Polygon mode it copies the
                            // whole selected polygons into the same mesh.
                            let tool = l.tool;
                            let component = self.editor.scene.selection.mode != Mode::Item;
                            self.run(Command::Drop);
                            let copied = if component {
                                self.run_ok(Command::DuplicateSelectedPolygons)
                            } else {
                                self.run_ok(Command::DuplicateItem)
                            };
                            let status = self.status.clone();
                            self.activate(tool);
                            let captured = if copied {
                                i2_route::capture(&mut self.editor, axis, false)
                            } else {
                                // Nothing to copy: the tool comes back and the
                                // refusal stays on the status line.
                                Err(status)
                            };
                            match captured {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(axis, params, tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if let Some(axis) = self.handle_hit(pos, r) {
                            // Jason, 2026-11-09: a second handle drag adjusts
                            // the same bevel while the tool stays on (Drop ends
                            // it); `BEVEL_REDRAG_STACKS` flips back to bevel-drag
                            // G2, where a drag after a released one commits that
                            // bevel and starts the next on its cap. Panel edits
                            // adjust the current bevel either way (TA-02).
                            if self.bevel_redrag_stacks
                                && !screen_rule
                                && l.tool.is_bevel()
                                && modifiers.is_none()
                                && std::mem::take(&mut self.bevel_dragged)
                                && self.editor.scene != l.base
                            {
                                self.run(Command::Continue);
                            }
                            // Jason, 2026-11-09: Shift+drag on a handle keeps
                            // this bevel and shapes a fresh one on its caps.
                            // MN-03.3: not with the Screen falloff on, where
                            // Shift keeps adjusting this one.
                            if l.tool.is_bevel() && modifiers.shift_only() && !screen_rule {
                                self.run(Command::Continue);
                            }
                            match i2_route::capture(&mut self.editor, axis, modifiers.is_none()) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(axis, params, l.tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::VertexBevel && modifiers.is_none() {
                            match i2_route::capture(
                                &mut self.editor,
                                VERTEX_BEVEL_FREE_HANDLE,
                                false,
                            ) {
                                Ok((params, _)) => {
                                    self.clear_fields();
                                    input::GestureKind::Handle(
                                        VERTEX_BEVEL_FREE_HANDLE,
                                        params,
                                        Tool::VertexBevel,
                                    )
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::Extrude && !modifiers.shift {
                            // Off the gizmo: move the extrusion freely in the
                            // view plane (user correction 2026-10-30).
                            match i2_route::capture(&mut self.editor, EXTRUDE_FREE_HANDLE, false) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(
                                        EXTRUDE_FREE_HANDLE,
                                        params,
                                        Tool::Extrude,
                                    )
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::EdgeExtend && modifiers.shift_only() {
                            // Spec: Shift+click keeps this extension and
                            // extends again from the new edges with the same
                            // values; a drag then shapes the new one like a
                            // plain drag anywhere.
                            if !self.run_ok(Command::Continue) {
                                input::GestureKind::Blocked
                            } else {
                                match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                    Ok((params, ticket)) => {
                                        self.i2_gesture_ticket = ticket;
                                        self.clear_fields();
                                        input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                    }
                                    Err(e) => {
                                        self.status = e;
                                        input::GestureKind::Blocked
                                    }
                                }
                            }
                        } else if l.tool.is_bevel() && modifiers.shift_only() && !screen_rule {
                            // Jason, 2026-11-09: Shift+click or Shift+drag
                            // keeps this bevel and starts a fresh one (values
                            // reset) on the current polygon selection; a drag
                            // then shapes it like a plain drag anywhere. The
                            // selection stays as it is (provisional, TA-Q02).
                            self.run(Command::Continue);
                            match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                            && !screen_rule
                            && modifiers.shift
                            && !modifiers.alt
                            && !modifiers.ctrl
                            && !modifiers.command
                            && !modifiers.mac_cmd
                        {
                            self.run(Command::Continue);
                            // No unreviewed off-handle displacement law is invented.
                            // Capture the press so release cannot select/repeat again.
                            input::GestureKind::Blocked
                        } else if l.tool.is_transform()
                            && (!modifiers.shift || continues)
                            && !modifiers.alt
                            && !modifiers.mac_cmd
                        // HM-10 / HM-20 / HM-32 / HM-40: Ctrl hauls are
                        // constrained (axis lock, snapped turns; Scale's
                        // haul is uniform either way).
                        {
                            // Off the handles (a press on a handle was taken
                            // above, so a handle always wins): the Action
                            // Center's rule says what the press does to the
                            // centre (AC-RULES v6, `ActionContext::press`),
                            // then the press drags the tool freely in the
                            // view plane. HM-02: a Shift haul adds to the
                            // continued drag where it is; the center stays.
                            use operandi_core::CenterPress;
                            if continues {
                                self.free_transform_drag(l.tool)
                            } else if self.element_falloff().is_some() {
                                // FO-45: a press picks the element under the
                                // pointer and the drag moves it.
                                self.begin_element_falloff_drag(l.tool, pos, r)
                            } else if self.editor.scene.selection.mode == Mode::Item {
                                // Items keep their presses until the rule
                                // sheet's Item-mode step (ACR-Q19): beside
                                // the items the center jumps to the press;
                                // on a selected item a moved center goes
                                // back (VAC-Q02); on another it stays.
                                match self.pick_component(pos, r) {
                                    None => self.place_center_press(l.tool, pos, r, true),
                                    Some((id, c)) if self.is_selected_component(id, c) => {
                                        self.run(Command::ResetActionCenter);
                                        self.free_transform_drag(l.tool)
                                    }
                                    Some(_) => self.free_transform_drag(l.tool),
                                }
                            } else {
                                match self.editor.action_context.press() {
                                    // A Snaps and Precision preset sets its
                                    // own centre with its one click.
                                    CenterPress::Pick if self.snap_preset.is_none() => {
                                        self.element_press(l.tool, pos, r)
                                    }
                                    CenterPress::Place { snapped } => {
                                        self.place_center_press(l.tool, pos, r, snapped)
                                    }
                                    // Selection, Selection Border, Origin,
                                    // Parent, Local and Pivot: the centre
                                    // stays where its rule puts it.
                                    _ => self.free_transform_drag(l.tool),
                                }
                            }
                        } else if l.tool == Tool::Bend
                            && !modifiers.shift
                            && !modifiers.alt
                            && !modifiers.mac_cmd
                        {
                            // Bend v2: the ring bends, the spine's end moves
                            // the spine, any other press moves the centre.
                            self.begin_bend(pos, r)
                        } else if (modifiers.is_none() || (screen_rule && modifiers.shift_only()))
                            && off_handle_drag(l.tool)
                        {
                            // Element Push (FO-45): with an Element falloff a
                            // press first picks the element under the
                            // pointer; nothing under it, nothing moves.
                            let picked = self.element_falloff().is_none() || {
                                match self.tweak_pick(pos, r) {
                                    Some(pick) => {
                                        self.run_ok(Command::FalloffElementPick(Some(pick.grab)))
                                    }
                                    None => {
                                        self.status =
                                            "Element falloff: press on a vertex, edge or polygon"
                                                .into();
                                        false
                                    }
                                }
                            };
                            // Off the handles: a drag anywhere drives the tool
                            // (Jason, 2026-11-04; DRAG-Q01).
                            if !picked {
                                input::GestureKind::Blocked
                            } else {
                                match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                    Ok((params, ticket)) => {
                                        self.i2_gesture_ticket = ticket;
                                        self.clear_fields();
                                        input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                    }
                                    Err(e) => {
                                        self.status = e;
                                        input::GestureKind::Blocked
                                    }
                                }
                            }
                        } else {
                            self.status="Drag an explicit handle. Overlapping handles require orbiting. Edit fields or Drop/Cancel to finish.".into();
                            input::GestureKind::Blocked
                        }
                    } else if let Some(kind) = self.fill_press(pos, r, modifiers) {
                        kind
                    } else if modeling_menu::wants_menu(modifiers) {
                        // Capture the press; the menu opens on release (as the
                        // selection context menu does), so the release doesn't
                        // count as a click outside it and close it again.
                        self.modeling_menu_pressed = true;
                        input::GestureKind::Blocked
                    } else if let Some(kind) = self.empty_expand_double_click(
                        pos,
                        r,
                        modifiers,
                        press_time,
                        double_click_delay,
                    ) {
                        kind
                    } else if let Some(kind) = self.polygon_connected_double_click(
                        pos,
                        r,
                        modifiers,
                        press_time,
                        double_click_delay,
                    ) {
                        kind
                    } else if let Some(kind) = self.edge_loop_double_click(
                        pos,
                        r,
                        modifiers,
                        press_time,
                        double_click_delay,
                    ) {
                        kind
                    } else {
                        // A possible first press of an Edge (loop) or
                        // Polygon (connected) double-click.
                        if matches!(self.editor.scene.selection.mode, Mode::Edge | Mode::Polygon) {
                            self.last_component_press = Some((press_time, pos));
                        }
                        self.run(Command::BeginSelection);
                        let hit = self.select_at(pos, r, modifiers);
                        // A possible first press of a Shift+double-click on
                        // empty space (Expand Selection, empty_expand.rs).
                        self.note_empty_shift_press(hit, modifiers, press_time, pos);
                        input::GestureKind::Select
                    };
                    ui.ctx().request_repaint();
                    self.navigation = Some(input::Navigation {
                        kind,
                        start: pos,
                        last: pos,
                        modifiers,
                        button: PointerButton::Primary,
                    });
                }
                Event::PointerMoved(pos) => {
                    if self.drop_drag.is_some() {
                        let _ = pos;
                        ui.ctx().request_repaint();
                    }
                    if let Some(mut g) = self.navigation.take() {
                        // A press that became a paint drag is no first click
                        // of a double-click, even if it comes back.
                        if matches!(g.kind, input::GestureKind::Select)
                            && pos.distance(g.start) > EDGE_DOUBLE_CLICK_DISTANCE
                        {
                            self.last_component_press = None;
                            self.last_empty_shift_press = None;
                        }
                        if let input::GestureKind::Draw(d) = &mut g.kind {
                            let start = g.start;
                            self.draw_drag(d, start, pos, r);
                        }
                        if let input::GestureKind::SolidSketch(d) = &mut g.kind {
                            let start = g.start;
                            self.solid_sketch_drag(d, start, pos, r);
                        }
                        if let input::GestureKind::Skeleton(d) = &g.kind {
                            let (d, start) = (d.clone(), g.start);
                            self.skeleton_drag(&d, start, pos, r);
                        }
                        if let input::GestureKind::Pose(d) = &g.kind {
                            let (d, start) = (d.clone(), g.start);
                            self.pose_drag(&d, start, pos, r);
                        }
                        if let input::GestureKind::PenDrag { point } = g.kind
                            && pos.distance(g.start) >= 1.
                        {
                            self.pen_drag(point, pos, r);
                        }
                        if matches!(g.kind, input::GestureKind::WeldDrag) {
                            self.weld_move(pos, r);
                        }
                        if let input::GestureKind::Haul(drag) = &mut g.kind {
                            self.haul_move(drag, pos);
                        }
                        if let input::GestureKind::PenOffset {
                            start_offset,
                            metres_per_pixel,
                        } = g.kind
                        {
                            self.pen_offset_drag(start_offset, metres_per_pixel, pos.x - g.start.x);
                        }
                        if let input::GestureKind::Lasso(path, _) = &mut g.kind
                            && path.last().is_none_or(|p| p.distance(pos) >= 1.)
                        {
                            if path.len() < 4096 {
                                path.push(pos);
                            } else {
                                self.status = "Lasso path budget is 4096 points".into();
                            }
                        }
                        if let input::GestureKind::ScreenFalloffDraw { start, path } = &mut g.kind {
                            let start = *start;
                            self.screen_falloff_drag(start, path, pos, r);
                        }
                        if let input::GestureKind::SketchStroke(stroke) = &mut g.kind {
                            self.sketch_stroke_move(stroke, pos, false);
                        } else {
                            self.gesture_move(&g, pos, r);
                        }
                        ui.ctx().request_repaint();
                        g.last = pos;
                        self.navigation = Some(g);
                    }
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    ..
                } => {
                    if std::mem::take(&mut self.modeling_menu_pressed) {
                        self.modeling_menu_requested = true;
                    }
                    if button == PointerButton::Primary && self.drop_drag.is_some() {
                        self.drop_release(pos, r);
                        ui.ctx().request_repaint();
                        continue;
                    }
                    if let Some(mut g) = self.navigation.take() {
                        if g.button != button {
                            self.navigation = Some(g);
                            continue;
                        }
                        // NAV-07 box zoom finishes on release.
                        if let input::GestureKind::Navigate(kind) = g.kind {
                            self.navigation_release(kind, g.start, pos, r);
                        }
                        if let input::GestureKind::Pose(d) = &g.kind {
                            let d = d.clone();
                            self.pose_release(&d);
                            self.release_pointer();
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if let input::GestureKind::Skeleton(d) = &g.kind {
                            let d = d.clone();
                            self.skeleton_release(&d);
                            self.release_pointer();
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if let input::GestureKind::SolidSketch(d) = &mut g.kind {
                            let start = g.start;
                            self.solid_sketch_drag(d, start, pos, r);
                            let d = d.clone();
                            self.solid_sketch_release(&d, start, pos);
                            self.release_pointer();
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if let input::GestureKind::PenDrag { .. }
                        | input::GestureKind::PenOffset { .. } = g.kind
                        {
                            self.i2_gesture_ticket = None;
                            self.snap_session = Default::default();
                            self.snap_marker = None;
                            self.snap_reticle = None;
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if matches!(g.kind, input::GestureKind::WeldDrag) {
                            self.weld_release(pos, g.start, r);
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if matches!(g.kind, input::GestureKind::Blocked)
                            && button == PointerButton::Primary
                            && g.modifiers.is_none()
                            && g.start.distance(pos) < 3.
                            && g.start.distance(g.last) < 3.
                            && self
                                .editor
                                .reduction
                                .as_ref()
                                .is_some_and(|r| !r.initialized())
                        {
                            let succeeded = self.run_ok(Command::InitializeReduction);
                            if succeeded {
                                self.status = self.reduction_status();
                            }
                        }
                        if let input::GestureKind::Lasso(path, middle) = &mut g.kind {
                            path.push(pos);
                            if g.start.distance(pos) < 1.
                                && path.iter().all(|p| p.distance(g.start) < 1.)
                            {
                                self.run(Command::EndSelection { cancel: true });
                                if !*middle && !self.mouse_region_click(g.start, r) {
                                    self.selection_context_requested = true;
                                }
                            } else {
                                self.finish_lasso(path, *middle, g.modifiers, r);
                            }
                        }
                        if let input::GestureKind::ScreenFalloffDraw { start, path } = &mut g.kind {
                            let start = *start;
                            self.screen_falloff_drag(start, path, pos, r);
                            self.screen_falloff_release();
                        }
                        if let input::GestureKind::SketchStroke(stroke) = &mut g.kind {
                            self.sketch_stroke_move(stroke, pos, true);
                        } else {
                            self.gesture_move(&g, pos, r);
                        }
                        ui.ctx().request_repaint();
                        // UIC-78: a plain click (no drag) finishes a numeric
                        // session that waits for Apply.
                        if let input::GestureKind::Numeric(drag) = &g.kind
                            && button == PointerButton::Primary
                            && g.modifiers.is_none()
                            && g.start.distance(pos) < click_finish::CLICK_SLOP
                            && g.start.distance(g.last) < click_finish::CLICK_SLOP
                        {
                            self.click_finish_numeric(drag);
                        }
                        if let input::GestureKind::Utility(drag) = &g.kind {
                            self.utility_release(drag, pos, r);
                        }
                        if let input::GestureKind::Slice(drag) = &g.kind {
                            self.slice_release(drag);
                        }
                        if let input::GestureKind::Duplicate(drag) = &g.kind {
                            self.path_tool_release(drag, r);
                        }
                        if let input::GestureKind::Haul(drag) = &mut g.kind {
                            self.haul_move(drag, pos);
                            let drag = drag.clone();
                            self.haul_release(&drag, pos, r);
                        }
                        if let input::GestureKind::Draw(d) = &mut g.kind {
                            let start = g.start;
                            self.draw_release(d, start, pos, r);
                        }
                        if matches!(g.kind, input::GestureKind::Select) {
                            self.run(Command::EndSelection { cancel: false });
                        }
                        // Each transform drag is its own undo step; the tool
                        // stays active (Jason, 2026-11-03).
                        if let input::GestureKind::Handle(_, _, tool) = g.kind
                            && tool.is_transform()
                        {
                            self.run(Command::CommitTransformStep);
                        }
                        if matches!(g.kind, input::GestureKind::Handle(..)) {
                            self.release_drag_readout(ui.ctx().input(|i| i.time));
                        }
                        if let input::GestureKind::Handle(id, _, _) = g.kind {
                            self.finish_precision_handle(id);
                        }
                        if let input::GestureKind::Handle(_, _, tool) = g.kind
                            && tool.is_bevel()
                        {
                            self.bevel_dragged = true;
                        }
                        // TWK-24: each Tweak drag is one undo step; a click
                        // that didn't move records nothing.
                        if let input::GestureKind::Tweak { pick, .. } = g.kind {
                            // TP-30: a pen drag with a weld waiting is one
                            // move-and-weld edit instead.
                            if !(self.topology_pen_open() && self.pen_tweak_release(pick)) {
                                self.run(Command::CommitTransformStep);
                            }
                        }
                        // SC-90: one stroke, one undo step.
                        match g.kind {
                            input::GestureKind::Sculpt => self.finish_sculpt_stroke(),
                            #[cfg(feature = "painting")]
                            input::GestureKind::Paint => self.finish_paint_stroke(),
                            #[cfg(feature = "painting")]
                            input::GestureKind::PaintSize { .. } => self.paint_size_release(),
                            input::GestureKind::SculptSize { .. } => {
                                self.sculpt_adjust_release(true, g.start, pos)
                            }
                            input::GestureKind::SculptStrength { .. } => {
                                self.sculpt_adjust_release(false, g.start, pos)
                            }
                            input::GestureKind::FalloffSteps { layer, start } => {
                                self.falloff_steps_release(layer, start, g.start, pos)
                            }
                            _ => {}
                        }
                        // The press is over: the next change is a new tweak.
                        self.release_pointer();
                        self.snap_session = Default::default();
                        self.snap_marker = None;
                        self.snap_reticle = None;
                        self.i2_gesture_ticket = None;
                    }
                }
                // A press still held when the pointer leaves the window
                // keeps going until its release, as egui's own drags do:
                // orbiting, a tool drag or a stroke that runs off the edge
                // doesn't stop dead (Windows tester, 2026-11-21). The OS
                // still sends the moves and the release to this window.
                // Losing focus still ends it, below.
                Event::PointerGone if pointer_down => {}
                Event::WindowFocused(false) | Event::PointerGone => {
                    self.cancel_drop();
                    // An abandoned Command-press must not open the menu later.
                    self.modeling_menu_pressed = false;
                    self.end_gesture();
                }
                _ => {}
            }
        }
        if response.hovered()
            && !self.editor_owned_input
            && !egui::Popup::is_any_open(ui.ctx())
            && !self.fov_wheel(ui)
        {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                self.wheel_zoom(response.hover_pos(), scroll, r);
            }
        }
    }
}

/// Gestures whose drag depends only on where it started and where the
/// pointer is now, so intermediate pointer moves in a frame can be skipped.
/// Paths (lasso, sketch, pen, draw), painting and navigation keep every
/// move.
/// Drags that go on past the view's edge with Free Cursor: navigation and
/// the value drags (tool handles, the drag layer, counts and sizes). Not
/// those that draw or pick under the pointer (selection, lasso, strokes,
/// drawing shapes, tweaking a point).
fn wraps(kind: &input::GestureKind) -> bool {
    use input::GestureKind as G;
    match kind {
        G::Navigate(k) => *k != input::NavigationKind::BoxZoom,
        G::Numeric(n) => matches!(
            n,
            input::NumericDrag::Spike(_)
                | input::NumericDrag::Bridge(_)
                | input::NumericDrag::Reduction(..)
        ),
        G::Handle(..)
        | G::ItemHandle(_)
        | G::PolyHaul(_)
        | G::Haul(_)
        | G::Falloff(..)
        | G::FalloffSteps { .. }
        | G::FalloffRadius { .. }
        | G::AirbrushSize { .. }
        | G::SculptSize { .. }
        | G::SculptStrength { .. }
        | G::PaintSize { .. }
        | G::Bend(_)
        | G::Duplicate(_) => true,
        _ => false,
    }
}

pub(super) fn latest_position_only(kind: &input::GestureKind) -> bool {
    matches!(
        kind,
        input::GestureKind::Handle(..)
            | input::GestureKind::ItemHandle(_)
            | input::GestureKind::Falloff(..)
            | input::GestureKind::FalloffDraw { .. }
            | input::GestureKind::Tweak { .. }
            | input::GestureKind::Numeric(_)
            | input::GestureKind::PolyHaul(_)
            | input::GestureKind::Haul(_)
            // A stroke lays its dabs along the straight line from its last
            // dab, so skipped moves cost nothing but their kinks.
            | input::GestureKind::Sculpt
            | input::GestureKind::SculptSize { .. }
            | input::GestureKind::SculptStrength { .. }
    ) || painting_fast(kind)
}

/// Whether `pos` is on a window (an egui area that takes the pointer) drawn
/// above the layer `ui` draws on: what the user clicks there is that window.
pub(super) fn over_window_above(ui: &Ui, pos: Pos2) -> bool {
    let own = ui.layer_id();
    ui.ctx()
        .layer_id_at(pos)
        .is_some_and(|top| top != own && top.order >= own.order)
}

/// Paint strokes skip moves like sculpt strokes (the dabs follow the
/// straight line from the last one).
#[cfg(feature = "painting")]
pub(super) fn painting_fast(kind: &input::GestureKind) -> bool {
    matches!(
        kind,
        input::GestureKind::Paint | input::GestureKind::PaintSize { .. }
    )
}

#[cfg(not(feature = "painting"))]
pub(super) fn painting_fast(_: &input::GestureKind) -> bool {
    false
}
