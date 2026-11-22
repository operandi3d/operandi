//! Painting stage 6 (PNT-215): painting in the UV view. With a paint tool
//! on, the paint target's image shows flat behind the UV map, and a left
//! press and drag paints straight into it in image space (core
//! `PaintStroke::begin_uv`): dab sizes are the UV view's screen points,
//! with no projection and no hidden-surface test. Fill and the gradients
//! work there too. Alt keeps navigating. Painting builds only.
use super::*;

impl Operandi {
    /// The UV view's input while a paint tool is on; true when it took it.
    pub(crate) fn uv_paint_input(&mut self, ui: &Ui, r: Rect, uv: &crate::uv_view::UvView) -> bool {
        if self.painting.brush.tool.is_none() {
            return false;
        }
        let (mods, pressed, down, released, origin, hover) = ui.input(|i| {
            (
                i.modifiers,
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.press_origin(),
                i.pointer.hover_pos(),
            )
        });
        if self.painting.brush.uv_live {
            if let Some(p) = hover
                && down
            {
                self.uv_paint_drag(p);
                ui.ctx().request_repaint();
            }
            if released || !down {
                self.finish_paint_stroke();
            }
            return true;
        }
        if mods.alt {
            return false;
        }
        // Ctrl+click sets Clone's source here too (PNT-95).
        if pressed && let Some(p) = origin.filter(|p| r.contains(*p)) {
            if self.painting.brush.tool == Some(operandi_core::paint_stroke::PaintTool::Clone)
                && (mods.ctrl || mods.command && !mods.mac_cmd)
            {
                let b = &mut self.painting.brush;
                b.clone_source = Some(p);
                b.clone_offset = None;
                self.status = "Clone source set: paint where the copy should go".into();
                return true;
            }
            let view = operandi_core::paint_stroke::UvScreen {
                screen_center: glam::DVec2::new(f64::from(r.center().x), f64::from(r.center().y)),
                uv_center: uv.center,
                zoom: uv.zoom,
                size: glam::DVec2::new(f64::from(r.width()), f64::from(r.height())),
            };
            self.uv_paint_press(p, view);
            ui.ctx().request_repaint();
            return true;
        }
        // With a paint tool the UV selection tools stay out of the
        // press's frames, its release (a click) included.
        hover.is_some_and(|p| r.contains(p)) && (down || released)
    }

    /// The paint target's image over the 0..1 tile.
    pub(crate) fn paint_uv_backdrop(&self, painter: &egui::Painter, tile: Rect) {
        if self.painting.brush.tool.is_none() {
            return;
        }
        let Some(image) = self.current_image() else {
            return;
        };
        let Some(px) = self.editor.scene.paint.buffers.get(&image) else {
            return;
        };
        let t = crate::painting_viewer::texture_of(painter.ctx(), "uv-paint-backdrop", px);
        painter.image(
            t.id(),
            tile,
            Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
            egui::Color32::WHITE,
        );
    }
}
