//! Pointer gestures in the viewport: orbiting, panning and zooming the view,
//! whole-mesh drags, and moving and ending the gesture in progress.
use super::*;

impl Operandi {
    pub(crate) fn navigate(&mut self, kind: input::NavigationKind, delta: Vec2, r: Rect) {
        // The header's Zoom button heads for the surface at the view
        // centre, as the Zoom In/Out keys do (fix-stuck-zoom).
        if kind == input::NavigationKind::Zoom {
            let factor = self.input_preferences.options.drag_zoom_factor(delta);
            if factor != 1. && self.zoom_toward_surface(r.center(), r, factor) {
                return;
            }
        }
        // Buttons and keys have no pointer position: a trackball turn then
        // starts from the view's centre.
        self.navigate_from(kind, r.center(), r.center() + delta, r);
    }
    /// Navigation for a pointer move from `from` to `to`. Trackball Rotation
    /// (Preferences > Input > Remapping, or the view's own choice) makes
    /// Orbit a free trackball turn; Camera view keeps the two-axis orbit
    /// since a render camera has no roll.
    pub(crate) fn navigate_from(
        &mut self,
        kind: input::NavigationKind,
        from: Pos2,
        to: Pos2,
        r: Rect,
    ) {
        let delta = to - from;
        let kind = match kind {
            // NAV-03 look around: the eye stays put, so Orbit Selected and
            // Trackball Rotation don't apply. In Camera and Light view the
            // item turns in place once the drag is written back.
            input::NavigationKind::Look => {
                self.camera.look_around(delta.x as f64, delta.y as f64);
                return;
            }
            input::NavigationKind::Roll => {
                // NAV-02: the angle the pointer turns about the view centre.
                let (a, b) = (from - r.center(), to - r.center());
                if a.length() > 2. && b.length() > 2. {
                    let angle = (a.x * b.y - a.y * b.x).atan2(a.dot(b)) as f64;
                    self.camera.roll_by(-angle);
                }
                return;
            }
            // NAV-05: sideways motion ignored.
            input::NavigationKind::PanVertical => {
                let (_, _, up, _) = self.camera.basis();
                self.camera.target += up * delta.y as f64 * self.camera.distance
                    / (r.height() as f64 * self.camera.lens);
                return;
            }
            // NAV-07: nothing moves until release (`box_zoom`).
            input::NavigationKind::BoxZoom => return,
            k => k,
        };
        // Orbit Selected (Viewport Properties > Drawing and Control): the
        // eye turns about the selection's centre, not the view's.
        let pivot = (kind == input::NavigationKind::Orbit
            && !self.camera.looks_through()
            && self.display.settings.drawing.orbit_selected)
            .then(|| self.orbit_pivot())
            .flatten();
        let before = pivot.map(|_| self.camera.basis());
        self.orbit_or_move(kind, from, to, delta, r);
        if let (Some(pivot), Some(before)) = (pivot, before) {
            crate::orbit_selected::turn_about(&mut self.camera, before, pivot);
        }
    }
    /// `navigate_from` once Look, Roll, Pan Vertical and Box Zoom are dealt
    /// with: orbit, pan or zoom.
    pub(super) fn orbit_or_move(
        &mut self,
        kind: input::NavigationKind,
        from: Pos2,
        to: Pos2,
        delta: Vec2,
        r: Rect,
    ) {
        if kind == input::NavigationKind::Orbit
            && !self.camera.looks_through()
            && self
                .input_preferences
                .options
                .trackball_in(self.display.settings.drawing.trackball)
        {
            let speed = self.input_preferences.options.trackball_speed() as f64;
            self.camera.trackball(from, to, r, speed);
            return;
        }
        match kind {
            input::NavigationKind::Pan => {
                let (_, right, up, _) = self.camera.basis();
                self.camera.target += (-right * delta.x as f64 + up * delta.y as f64)
                    * self.camera.distance
                    / (r.height() as f64 * self.camera.lens);
            }
            input::NavigationKind::Zoom => {
                self.camera.distance = crate::camera::zoomed_distance(
                    self.camera.distance,
                    self.input_preferences.options.drag_zoom_factor(delta),
                )
            }
            input::NavigationKind::Orbit => {
                self.camera.unfix_for_orbit();
                // Free Orbit (the tester, 2026-11-25). Camera and Light view
                // keep the stop: a camera item has no roll to stand upside
                // down with.
                if self.input_preferences.options.unrestricted_rotation
                    && !self.camera.looks_through()
                {
                    self.camera.orbit_free_by(delta.x as f64, delta.y as f64);
                } else {
                    self.camera.orbit_by(delta.x as f64, delta.y as f64);
                }
            }
            // Handled above.
            input::NavigationKind::Look
            | input::NavigationKind::Roll
            | input::NavigationKind::PanVertical
            | input::NavigationKind::BoxZoom => {}
        }
    }
    /// A component Move, Rotate, Scale or Transform drag that moves every
    /// point of the active mesh by one affine map (nothing selected, or
    /// everything): the tool's base item and that map in the mesh's own
    /// coordinates. The viewport then draws the base mesh, whose GPU
    /// geometry and subdivision surface are already cached, through the
    /// map, instead of rebuilding the whole mesh every frame. `None` when
    /// any point moved differently (a partial selection, falloff, symmetry).
    pub(super) fn whole_mesh_drag(
        &self,
        scene: &operandi_core::Scene,
    ) -> Option<(operandi_core::Item, glam::DMat4)> {
        let live = self.editor.session.live.as_ref()?;
        if !live.tool.transforms_items() || scene.selection.mode == Mode::Item {
            return None;
        }
        let id = scene.selection.item?;
        let now = scene.item(id)?;
        let base = live.base.item(id)?;
        if operandi_core::Shared::same(now.mesh.shared(), base.mesh.shared())
            || now.transform != base.transform
            || now.parent != base.parent
            || now.mesh.faces.len() != base.mesh.faces.len()
        {
            return None;
        }
        let basis = self
            .whole_mesh_basis_memo
            .get(&live.base, || whole_mesh_basis(&base.mesh.vertices))?;
        let map = whole_mesh_map_from(&base.mesh.vertices, &now.mesh.vertices, basis)?;
        Some((base.clone(), map))
    }
    pub(super) fn gesture_move(&mut self, g: &input::Navigation, pos: Pos2, r: Rect) {
        match &g.kind {
            input::GestureKind::Falloff(handle, start) => {
                self.drag_falloff(*handle, *start, g.start, pos, r)
            }
            input::GestureKind::MeshOpHandle(grab) => {
                self.drag_meshop_handle(grab, g.start, pos, r)
            }
            input::GestureKind::FalloffDraw { origin } => self.falloff_draw_drag(*origin, pos, r),
            input::GestureKind::Airbrush(stroke) => self.airbrush_drag(pos, r, *stroke),
            input::GestureKind::AirbrushSize { layer, start } => {
                self.airbrush_size_drag(*layer, *start, g.start, pos)
            }
            input::GestureKind::ItemHandle(grab) => self.drag_item_handle(*grab, pos, r),
            input::GestureKind::SolidDraw {
                cone,
                plane,
                anchor,
            } => self.solid_draw(*cone, *plane, *anchor, pos, r, g.modifiers.ctrl),
            input::GestureKind::SessionHeight { base, axis } => {
                self.session_height(*base, *axis, g.start, pos, r)
            }
            input::GestureKind::SessionHandle { id, base } => {
                self.session_handle_drag(*id, *base, g.start, pos, r)
            }
            input::GestureKind::EdgeInsert { tool, a, b, ends } => {
                self.edge_insert_slide(*tool, *a, *b, *ends, pos)
            }
            input::GestureKind::Navigate(kind) => {
                self.navigate_gesture(*kind, g.start, g.last, pos, r)
            }
            input::GestureKind::PolyHaul(drag) => self.poly_haul_drag(drag, g.start, pos, r),
            input::GestureKind::Handle(axis, params, tool) => {
                self.update_handle(*axis, params.clone(), g.start, pos, *tool, r);
                self.note_drag_readout(*axis, *tool);
            }
            input::GestureKind::Tweak { pick, base } => {
                self.tweak_drag(*pick, base, g.start, pos, r)
            }
            input::GestureKind::Numeric(drag) => {
                self.numeric_drag(drag, g.start, pos, r, g.modifiers.ctrl)
            }
            input::GestureKind::Utility(drag) => self.utility_drag(drag, g.start, pos, r),
            input::GestureKind::Slice(drag) => self.slice_drag(drag, pos, r, g.modifiers),
            input::GestureKind::Sculpt => self.sculpt_drag(pos, r),
            #[cfg(feature = "painting")]
            input::GestureKind::Paint => self.paint_drag(pos, r),
            #[cfg(feature = "painting")]
            input::GestureKind::PaintSize { start } => self.paint_size_drag(*start, g.start, pos),
            input::GestureKind::SculptSize { start } => {
                self.sculpt_adjust_drag(true, *start, g.start, pos)
            }
            input::GestureKind::SculptStrength { start } => {
                self.sculpt_adjust_drag(false, *start, g.start, pos)
            }
            input::GestureKind::Duplicate(drag) => self.duplicate_drag(drag, pos, r, g.modifiers),
            input::GestureKind::TweakRange {
                metres_per_pixel, ..
            } => self.tweak_range_drag(*metres_per_pixel, g.last, pos),
            input::GestureKind::Bend(drag) => {
                self.bend_drag(drag, g.start, pos, r, g.modifiers.ctrl)
            }
            input::GestureKind::FalloffSteps { layer, start } => {
                self.falloff_steps_drag(*layer, *start, g.start, pos)
            }
            input::GestureKind::FalloffRadius {
                layer,
                start,
                metres_per_pixel,
            } => self.falloff_radius_drag(*layer, *start, *metres_per_pixel, g.start, pos),
            input::GestureKind::Primitive(gesture) => self.update_primitive(gesture, g, pos, r),
            input::GestureKind::Select => {
                if self.editor.scene.selection.mode == Mode::Item {
                    return;
                }
                let steps = g.last.distance(pos).ceil().clamp(1., 8192.) as usize;
                let surface = self.pick_surface();
                // Selecting never changes the meshes, so the picking surface
                // and the candidates hold for the whole move.
                let candidates = self.pick_candidates(g.last, pos, r);
                for step in 1..=steps {
                    let sample = g.last + (pos - g.last) * (step as f32 / steps as f32);
                    let mut mods = g.modifiers;
                    if !mods.ctrl {
                        mods.shift = true;
                    }
                    // Painting samples every pixel along the pointer's path;
                    // most land on a component already painted. Skip those
                    // instead of running a selection command each (that was
                    // most of a fast drag's frame on large meshes).
                    let hit = match self.editor.scene.selection.mode {
                        Mode::Vertex | Mode::Edge => {
                            self.pick_among(sample, r, &surface, &candidates)
                        }
                        _ => self.pick_component_on(sample, r, &surface),
                    };
                    if let Some((item, hit)) = hit
                        && self.paint_changes(item, hit, mods.ctrl)
                    {
                        self.click_select(Click {
                            hit: Some((item, hit)),
                            how: if mods.ctrl {
                                ClickHow::Remove
                            } else {
                                ClickHow::Add
                            },
                            over_skipped_mesh: false,
                        });
                        self.selection_status();
                    }
                }
            }
            _ => {}
        }
    }
    /// Ends whatever the viewport's press was doing when it can't go on:
    /// focus lost, a dialog or menu took the input, or the pointer left
    /// with nothing held (Astra P5-001). One place, so every exit does the
    /// same: a selection drag is cancelled, a sculpt or paint stroke keeps
    /// what it did as its own undo step, and the press is released. Before,
    /// a dialog or focus change only dropped the navigation, so the stroke
    /// stayed held and the next press merged into it.
    pub(crate) fn end_gesture(&mut self) {
        if self.navigation.is_none()
            && self.editor.selection_start.is_none()
            && !self.pointer_held
            && !self.app_stroke_held()
        {
            return;
        }
        if self.editor.selection_start.is_some() {
            self.run(Command::EndSelection { cancel: true });
        }
        self.navigation = None;
        // A stroke the pointer left keeps what it did.
        self.finish_sculpt_stroke();
        #[cfg(feature = "painting")]
        self.finish_paint_stroke();
        self.sculpt.adjust_at = None;
        self.release_pointer();
        let lost_capture = self.i2_gesture_ticket.take().is_some();
        if lost_capture {
            let _ = self.editor.identity_ticket();
        }
    }
    /// A sculpt or paint stroke is held: the app, not core, owns its
    /// tentative points in the scene.
    pub(crate) fn app_stroke_held(&self) -> bool {
        #[cfg(feature = "painting")]
        if self.painting.brush.live.is_some() {
            return true;
        }
        self.sculpt.stroke.is_some()
    }
}
