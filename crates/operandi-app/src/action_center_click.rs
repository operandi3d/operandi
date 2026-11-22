//! Element and Screen action centers (spec action-center-scale-2026-11-06
//! F4, F6): choosing one waits for a viewport click, which fixes the centre.
use super::*;
use glam::DMat3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CenterClick {
    Element,
    Screen,
}

impl Operandi {
    /// Wait for the click that places an Element or Screen centre.
    pub(super) fn await_center_click(&mut self, kind: CenterClick) {
        self.center_click = Some(kind);
        self.status = match kind {
            CenterClick::Element => "Element: click a polygon, edge or point to centre on it",
            CenterClick::Screen => "Screen: click in the viewport to place the centre",
        }
        .into();
    }

    /// The click. Element (AC-30/31): the polygon, edge or point under
    /// the pointer; a polygon gives its centre and its normal as the
    /// handle's Y, an edge its middle, the normal of the polygons around
    /// it and its own direction as X, a point itself and its normal. The
    /// selection stays as it was. Screen (AC-12, AC-40): the clicked spot
    /// on the camera-facing plane through the selection's box centre, not
    /// snapped, with the view's axes.
    pub(super) fn place_action_center(&mut self, kind: CenterClick, pos: Pos2, r: Rect) {
        self.center_click = None;
        // A Snaps and Precision preset: any element, axes on its normal.
        if kind == CenterClick::Element && self.snap_preset.is_some() {
            self.place_preset_center(pos, r);
            return;
        }
        let context = match kind {
            CenterClick::Element => self.element_center(pos, r),
            CenterClick::Screen => self.screen_center(pos, r),
        };
        let Some(context) = context else {
            self.status = "Nothing under the pointer to centre on".into();
            return;
        };
        self.run(Command::ActionContext(context));
    }

    fn element_center(&self, pos: Pos2, r: Rect) -> Option<ActionContext> {
        use operandi_core::TweakElement as E;
        let pick = self.tweak_pick_in(pos, r, operandi_core::TweakMode::AutoCenter)?;
        let item = self.editor.scene.item(pick.grab.item)?;
        let world = self.editor.scene.world_matrix(item.id).ok()?;
        let points = pick.grab.element.points(&item.mesh).unwrap_or_default();
        let at = points
            .iter()
            .map(|&v| world.transform_point3(item.mesh.vertices[v as usize]))
            .sum::<DVec3>()
            / points.len().max(1) as f64;
        let at = if at.is_finite() { at } else { pick.at };
        let up = world
            .transform_vector3(crate::snaps_panel::element_normal(
                &item.mesh,
                pick.grab.element,
            ))
            .normalize_or_zero();
        let along = match pick.grab.element {
            E::Edge(a, b) => world
                .transform_vector3(item.mesh.vertices[b as usize] - item.mesh.vertices[a as usize]),
            _ => DVec3::ZERO,
        };
        Some(ActionContext {
            center: CenterMode::Element(operandi_core::PickedPoint::new(at)),
            axis: element_axes(up, along).map_or(AxisOrientation::World, |b| {
                AxisOrientation::Element(operandi_core::PickedFrame::from_basis(b))
            }),
        })
    }

    fn screen_center(&self, pos: Pos2, r: Rect) -> Option<ActionContext> {
        let scene = &self.editor.scene;
        let through = scene
            .active()
            .zip(operandi_core::transform_vertices(scene).ok())
            .and_then(|(item, moved)| {
                let world = scene.world_matrix(item.id).ok()?;
                let c = operandi_core::box_centre(
                    moved
                        .iter()
                        .map(|&v| world.transform_point3(item.mesh.vertices[v as usize])),
                );
                c.is_finite().then_some(c)
            })
            .unwrap_or(self.camera.target);
        let (_, right, up, forward) = self.camera.basis();
        let point = Self::sketch_plane_point(&self.camera, r, pos, through, forward)?;
        Some(ActionContext {
            center: CenterMode::Screen(operandi_core::PickedPoint::new(point)),
            axis: AxisOrientation::Screen(operandi_core::PickedFrame::from_basis(
                DMat3::from_cols(right, up, -forward),
            )),
        })
    }
}

/// AC-31: Y along `up` (the element's normal); X along `along` (an edge's
/// direction) squared up against it, else the world X (Z when the normal
/// is along X) laid square to it; Z = X × Y. `None` without a normal.
pub(super) fn element_axes(up: DVec3, along: DVec3) -> Option<DMat3> {
    if up == DVec3::ZERO || !up.is_finite() {
        return None;
    }
    let square = |v: DVec3| (v - up * v.dot(up)).normalize_or_zero();
    let mut x = square(along);
    if x == DVec3::ZERO {
        x = square(DVec3::X);
    }
    if x == DVec3::ZERO {
        x = square(DVec3::Z);
    }
    (x != DVec3::ZERO).then(|| DMat3::from_cols(x, up, x.cross(up)))
}

#[cfg(test)]
mod tests {
    use super::element_axes;
    use glam::DVec3;

    fn close(a: DVec3, b: DVec3) {
        assert!((a - b).length() < 1e-9, "{a:?} != {b:?}");
    }

    /// AC-T31: a polygon's normal is the handle's Y.
    #[test]
    fn element_axes_put_y_on_the_normal() {
        let n = DVec3::new(0., 1., -1.).normalize();
        let b = element_axes(n, DVec3::ZERO).unwrap();
        close(b.y_axis, n);
        close(b.x_axis, DVec3::X);
        close(b.z_axis, b.x_axis.cross(b.y_axis));
    }

    /// AC-31: an edge's direction is the second axis (X).
    #[test]
    fn element_axes_take_an_edges_direction_as_x() {
        let b = element_axes(DVec3::Y, DVec3::new(1., 0., 1.)).unwrap();
        close(b.x_axis, DVec3::new(1., 0., 1.).normalize());
        close(b.y_axis, DVec3::Y);
        assert!(element_axes(DVec3::ZERO, DVec3::X).is_none());
    }
}
