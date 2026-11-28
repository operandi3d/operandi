//! Selection falloff right-drags (Steps and radius) and the falloff drawn on
//! screen: its handles, shape and per-point weight tint.
use super::*;

impl Operandi {
    /// Falloff handles in world space: Linear Start (0) and End (1); Radial
    /// Center (0) and one Size handle per axis (1–3). Jason-specified
    /// on-screen editing (2026-11-02); the reference's pointer behaviour
    /// wasn't measured, so the layout is provisional.
    /// A right-drag on a Selection falloff: one Step per
    /// `FALLOFF_STEP_PIXELS` moved right from the press, fewer to the left,
    /// never below 1. Vertical movement is ignored.
    pub(crate) fn falloff_steps_drag(&mut self, layer: usize, start: u32, from: Pos2, to: Pos2) {
        let moved = ((to.x - from.x) / FALLOFF_STEP_PIXELS).trunc() as i64;
        self.set_falloff_steps(layer, start as i64 + moved);
    }
    /// FO-46 / FO-Q04 default: a right-click that didn't drag adds one Step.
    pub(crate) fn falloff_steps_release(&mut self, layer: usize, start: u32, from: Pos2, to: Pos2) {
        if from.distance(to) < FALLOFF_CLICK_PIXELS {
            self.set_falloff_steps(layer, start as i64 + 1);
        }
    }
    /// FO-Q05: start a Soft Selection Radius drag, measured at the
    /// selection's centre (or the view's target).
    pub(super) fn begin_falloff_radius(
        &mut self,
        layer: usize,
        start: f64,
        _pos: Pos2,
        r: Rect,
    ) -> Option<input::GestureKind> {
        let center = self.transform_handle_center();
        let metres_per_pixel = self.camera.metres_per_pixel(center, r)?;
        self.status = format!(
            "Soft Selection Radius: {} · drag right for more, left for less",
            crate::form::Unit::Distance.format(start)
        );
        Some(input::GestureKind::FalloffRadius {
            layer,
            start,
            metres_per_pixel,
        })
    }
    pub(crate) fn falloff_radius_drag(
        &mut self,
        layer: usize,
        start: f64,
        metres_per_pixel: f64,
        from: Pos2,
        to: Pos2,
    ) {
        let radius = (start + (to.x - from.x) as f64 * metres_per_pixel).max(0.);
        falloff_panel::set_soft_selection_radius(self, layer, radius);
        self.status = format!(
            "Soft Selection Radius: {} · drag right for more, left for less",
            crate::form::Unit::Distance.format(radius)
        );
    }
    pub(super) fn set_falloff_steps(&mut self, layer: usize, steps: i64) {
        let Some((at, f)) = falloff_panel::selection_layer(&self.editor) else {
            return;
        };
        let steps = steps.clamp(1, operandi_core::falloff::MAX_SELECTION_STEPS as i64) as u32;
        if at != layer || steps == f.steps {
            return;
        }
        falloff_panel::set_selection_steps(self, layer, steps);
        self.status =
            format!("Selection falloff Steps: {steps} · drag right for more, left for fewer");
    }
    /// The falloff on screen: its handles, the Linear line or Radial
    /// outline, and each point of the active mesh tinted by its weight.
    pub(super) fn draw_falloff(&self, painter: &egui::Painter, r: Rect) {
        // CS-01: nothing is drawn for an armed tool.
        if self.editor.is_armed() {
            return;
        }
        let (Some(field), Some(stack)) =
            (self.editor.spatial_falloff, self.editor.active_falloff())
        else {
            return;
        };
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        // FO-06: View ▸ Show Falloffs turns the point tints off.
        if self.show_falloffs
            && let Some(item) = self.editor.scene.active()
            && item.mesh.vertices.len() <= 20_000
        {
            // The whole stack, Vertex Map included; tints show 0…1.
            // Point weights come from the tool's starting mesh while it
            // edits, as the tool reads them (worked out once per drag).
            let base = self
                .editor
                .session
                .live
                .as_ref()
                .filter(|l| l.base.item(item.id).is_some())
                .map_or(&self.editor.scene, |l| &l.base);
            let prepared = stack.prepare(base, item.id).unwrap_or_default();
            let inputs = prepared.inputs();
            // FO-47/48 Show Vertex and Show Edge on the first layer.
            let (show_vertex, show_edge) = match field {
                Falloff::SoftSelection(f) => (f.show_vertex, f.show_edge),
                Falloff::Coplanar(f) => (f.show_vertex, f.show_edge),
                _ => (true, false),
            };
            let weights: Vec<f32> = item
                .mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let p = self.editor.scene.world_point(item.id, v);
                    (stack.weight_at_local(p, v, i as u32, inputs) as f32).clamp(0., 1.)
                })
                .collect();
            if show_edge {
                for (a, b) in item.mesh.edges() {
                    let w = weights[a as usize].min(weights[b as usize]);
                    if w > 0.
                        && let (Some(qa), Some(qb)) = (
                            project(
                                self.editor
                                    .scene
                                    .world_point(item.id, item.mesh.vertices[a as usize]),
                            ),
                            project(
                                self.editor
                                    .scene
                                    .world_point(item.id, item.mesh.vertices[b as usize]),
                            ),
                        )
                    {
                        painter.line_segment(
                            [qa, qb],
                            Stroke::new(
                                1.5_f32,
                                palette::falloff_weight(w).gamma_multiply(0.25 + 0.75 * w),
                            ),
                        );
                    }
                }
            }
            for (i, &v) in item.mesh.vertices.iter().enumerate() {
                if !show_vertex {
                    break;
                }
                let p = self.editor.scene.world_point(item.id, v);
                let w = weights[i];
                if w > 0.
                    && let Some(q) = project(p)
                {
                    painter.circle_filled(
                        q,
                        2. + 2. * w,
                        palette::falloff_weight(w).gamma_multiply(0.25 + 0.75 * w),
                    );
                }
            }
        }
        self.draw_falloff_shape(painter, r, field);
    }
}
