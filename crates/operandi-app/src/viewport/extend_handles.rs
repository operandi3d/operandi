//! Edge Extend's handles (SPEC-EDGE-EXTEND-ROTATE-SCALE v1): Transform's
//! layout of Move arrows, Rotate rings and Scale squares at the action
//! centre, which follows the Offset, each set shown when its Handles
//! option is on (Move and Scale by default), plus the Local Inset/Shift
//! arrows and the Plane circles (Move or Scale).
use super::*;

impl Operandi {
    /// The handles of the live Edge Extend, as `handles()` entries.
    pub(super) fn edge_extend_handles(
        &self,
        l: &Live,
        r: Rect,
    ) -> Vec<(usize, Pos2, Pos2, String)> {
        let shown = l.params.edge_extend.handles;
        let center = self.transform_handle_center();
        let Some((origin, _)) = self.camera.project(center, r) else {
            return Vec::new();
        };
        let zoom = self.handle_zoom_at(center);
        self.handle_zoom.set(zoom);
        let s = self.handle_preferences.scale * zoom;
        let mut out = Vec::new();
        for axis in 0..3 {
            let direction = self.handle_axis(axis);
            if let Some((_, arrow)) = self.handle_arrow(center, direction, 72. * s, r) {
                if shown.moves {
                    out.push((
                        axis,
                        origin,
                        origin + arrow,
                        format!("Offset {}", ["X", "Y", "Z"][axis]),
                    ));
                }
                if shown.scale {
                    out.push((
                        6 + axis,
                        origin + arrow * (30. / 72.),
                        origin + arrow * (44. / 72.),
                        format!("Scale {}", ["X", "Y", "Z"][axis]),
                    ));
                }
            }
            if shown.rotate
                && let Some(end) = self.rotation_ring(axis, r).last()
            {
                out.push((
                    3 + axis,
                    origin,
                    *end,
                    format!("Rotate {}", ["X", "Y", "Z"][axis]),
                ));
            }
        }
        if shown.local
            && let Some((inset, shift)) = self.extend_local_axes()
        {
            for (id, direction, label) in [
                (EXTEND_INSET_HANDLE, inset, "Inset"),
                (EXTEND_SHIFT_HANDLE, shift, "Shift"),
            ] {
                if let Some((tip, _)) = self
                    .camera
                    .project(center + direction * self.camera.distance * 0.1, r)
                    && tip.distance(origin) > 5.
                {
                    let end = origin + (tip - origin).normalized() * 56. * s;
                    out.push((id, origin, end, label.into()));
                }
            }
        }
        out.extend(self.plane_handles(l.tool, r));
        out
    }

    /// The Local handles' world directions, from the first selected edge's
    /// first neighbouring polygon in the extension's base: Inset outward
    /// in its plane, Shift along its normal (as the geometry uses them).
    pub(crate) fn extend_local_axes(&self) -> Option<(DVec3, DVec3)> {
        let l = self.editor.session.live.as_ref()?;
        let item = l.base.active()?;
        let world = l.base.world_matrix(item.id).ok()?;
        let &(a, b) = l.base.selection.edges.first()?;
        let mesh = &item.mesh;
        let (fi, f) = mesh.faces.iter().enumerate().find(|(fi, f)| {
            !mesh.is_polyline(*fi)
                && (0..f.vertices.len()).any(|i| {
                    let (x, y) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
                    (x, y) == (a, b) || (x, y) == (b, a)
                })
        })?;
        let w = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
        let corners: Vec<DVec3> = f.vertices.iter().map(|&v| w(v)).collect();
        let mut normal = DVec3::ZERO;
        for i in 0..corners.len() {
            normal += corners[i].cross(corners[(i + 1) % corners.len()]);
        }
        let normal = normal.normalize_or_zero();
        let centre = world.transform_point3(mesh.center(fi));
        let (pa, pb) = (w(a), w(b));
        let mut out = (pb - pa).cross(normal).normalize_or_zero();
        if out.dot((pa + pb) * 0.5 - centre) < 0. {
            out = -out;
        }
        (out != DVec3::ZERO && normal != DVec3::ZERO).then_some((out, normal))
    }

    /// One sample of a Local handle drag: the pointer's travel along the
    /// arrow on screen, in world units at the handle, adds to Inset or
    /// Shift.
    pub(super) fn update_extend_local_handle(
        &mut self,
        id: usize,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        r: Rect,
    ) {
        let Some((inset, shift)) = self.extend_local_axes() else {
            return;
        };
        let direction = if id == EXTEND_INSET_HANDLE {
            inset
        } else {
            shift
        };
        let (_, right, up, _) = self.camera.basis();
        let screen = vec2(direction.dot(right) as f32, -direction.dot(up) as f32);
        let pixels = (p - start).dot(screen) / screen.length_sq().max(0.001);
        let amount = pixels as f64 * self.camera.distance / (r.height() as f64 * 0.9);
        let mut params = base;
        if id == EXTEND_INSET_HANDLE {
            params.inset += amount;
        } else {
            params.shift += amount;
        }
        self.publish_handle_params(Tool::EdgeExtend, params);
    }
}
