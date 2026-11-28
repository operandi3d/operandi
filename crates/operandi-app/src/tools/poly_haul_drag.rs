//! Poly Haul drags per mode (spec poly-haul-modes-2026-11-06). Offset,
//! Extrude and Scale act on the selection wherever the drag starts; Bevel,
//! Split, Tear, Slide and Remove act on the element under the pointer when
//! the drag starts. Drag gains are provisional (only the result shapes were
//! specified).
use super::*;
use operandi_core::{HaulTarget, PolyHaulMode};

/// What a Poly Haul drag started on.
#[derive(Clone)]
pub(super) struct HaulDrag {
    pub mode: PolyHaulMode,
    pub start_offset: f64,
    pub target: HaulTarget,
    /// Split: the picked edge's ends on screen.
    pub edge: Option<(Pos2, Pos2)>,
    /// Tear and Slide: the picked point (world), for the drag plane.
    pub through: DVec3,
    pub camera: Camera,
}

impl Operandi {
    pub(super) fn begin_poly_haul(&mut self, l: &Live, pos: Pos2, r: Rect) -> input::GestureKind {
        let mode = l.params.poly_haul_mode;
        let mut drag = HaulDrag {
            mode,
            start_offset: l.params.poly_haul_offset,
            target: HaulTarget::None,
            edge: None,
            through: DVec3::ZERO,
            camera: self.camera.clone(),
        };
        if mode.uses_selection() {
            if l.base.selection.faces.is_empty() {
                self.status = "Poly Haul: select polygons to haul".into();
                return input::GestureKind::Blocked;
            }
            return input::GestureKind::PolyHaul(Box::new(drag));
        }
        let Some(item) = self.editor.scene.active() else {
            return input::GestureKind::Blocked;
        };
        let Ok(world) = self.editor.scene.world_matrix(item.id) else {
            return input::GestureKind::Blocked;
        };
        let face_under = self
            .pick_surface()
            .pick_in(
                &self.camera,
                pos,
                r,
                crate::picking::limit(&self.pick_only()),
            )
            .filter(|h| h.0 == item.id)
            .map(|h| h.1);
        match mode {
            PolyHaulMode::Bevel | PolyHaulMode::Split => {
                let Some((a, b, _)) = self.pick_edge_parameter(pos, r) else {
                    self.status = "Poly Haul: start the drag on an edge".into();
                    return input::GestureKind::Blocked;
                };
                let screen = |v: u32| {
                    self.camera
                        .project(world.transform_point3(item.mesh.vertices[v as usize]), r)
                        .map(|(p, _)| p)
                };
                drag.edge = screen(a).zip(screen(b));
                drag.target = HaulTarget::Edge(a, b);
            }
            PolyHaulMode::Remove => {
                let Some(face) = face_under else {
                    self.status = "Poly Haul: click a polygon to remove".into();
                    return input::GestureKind::Blocked;
                };
                drag.target = HaulTarget::Polygon(face);
                self.preview_haul(&drag, Default::default());
                return input::GestureKind::Blocked;
            }
            PolyHaulMode::Tear | PolyHaulMode::Slide => {
                let Some(point) = self.pick_vertex(pos, r) else {
                    self.status = "Poly Haul: start the drag on a point".into();
                    return input::GestureKind::Blocked;
                };
                // Tear's polygon: the one under the pointer when it has the
                // point, else the first that does (Q-PH-07 open).
                let polygon = face_under
                    .filter(|&f| item.mesh.faces[f as usize].vertices.contains(&point))
                    .or_else(|| {
                        item.mesh
                            .faces
                            .iter()
                            .position(|f| f.vertices.contains(&point))
                            .map(|f| f as u32)
                    });
                let Some(polygon) = polygon else {
                    self.status = "Poly Haul: that point is on no polygon".into();
                    return input::GestureKind::Blocked;
                };
                drag.through = world.transform_point3(item.mesh.vertices[point as usize]);
                drag.target = HaulTarget::Point { point, polygon };
            }
            _ => {}
        }
        input::GestureKind::PolyHaul(Box::new(drag))
    }

    /// One frame of a Poly Haul drag.
    pub(super) fn poly_haul_drag(&mut self, drag: &HaulDrag, start: Pos2, pos: Pos2, r: Rect) {
        let gain = self.camera.distance / (r.height() as f64 * 0.9);
        let mut amount = self
            .editor
            .session
            .live
            .as_ref()
            .map(|l| l.params.poly_haul_amount)
            .unwrap_or_default();
        let mut offset = drag.start_offset;
        match drag.mode {
            // Up pulls out, down pushes in.
            PolyHaulMode::Offset | PolyHaulMode::Extrude => {
                offset += (start.y - pos.y) as f64 * gain;
            }
            PolyHaulMode::Bevel => amount.inset = start.distance(pos) as f64 * gain * 0.5,
            // Right grows, left shrinks.
            PolyHaulMode::Scale => {
                amount.scale = (1. + (pos.x - start.x) as f64 / (r.width() as f64 * 0.25)).max(0.)
            }
            PolyHaulMode::Split => {
                if let Some((a, b)) = drag.edge {
                    let d = b - a;
                    amount.position =
                        ((pos - a).dot(d) / d.length_sq().max(1e-6)).clamp(0., 1.) as f64;
                }
            }
            PolyHaulMode::Tear | PolyHaulMode::Slide => {
                let normal = drag.camera.basis().3;
                let at = |p| Self::sketch_plane_point(&drag.camera, r, p, drag.through, normal);
                if let (Some(a), Some(b)) = (at(start), at(pos)) {
                    amount.vector = b - a;
                }
            }
            PolyHaulMode::Remove => return,
        }
        let mut drag = drag.clone();
        drag.start_offset = offset;
        self.preview_haul(&drag, amount);
    }

    fn preview_haul(&mut self, drag: &HaulDrag, amount: operandi_core::HaulAmount) {
        let Some(live) = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::PolyHaul)
        else {
            return;
        };
        let mut params = live.params.clone();
        params.poly_haul_offset = drag.start_offset;
        params.poly_haul_target = drag.target;
        params.poly_haul_amount = amount;
        if params != live.params {
            self.run(Command::Preview(params));
            self.clear_fields();
        }
    }
}
